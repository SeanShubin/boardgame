//! Headless deterministic auto-resolution — the **par-solver substrate** (§8). With the optional
//! Clash module **off**, a §4 battle is a pure function of both sides' choices + seed (creatures are
//! deterministic), so a **greedy hero policy** plays it to an `Outcome`. Only the hero side needs a
//! policy; the foe side is the game's own creature AI. See §4.2 (deterministic base mode).

use std::collections::{HashMap, HashSet};

use contract::{Game, Outcome, PlayerId};

use crate::actor::{Actor, Token};
use crate::duel::Move;
use crate::game::{Action, Deckbound, battle_state_with};
use crate::ruleset::Ruleset;
use crate::state::{Phase, State};

/// Hard cap on decision steps, so a degenerate scenario (no one can damage anyone) returns rather
/// than spinning forever.
const MAX_STEPS: usize = 100_000;

/// Auto-resolve a PvE battle headlessly (Clash off → deterministic): the party (`heroes`) vs
/// `foes`. `Some(true)` = heroes win, `Some(false)` = heroes fall **or draw** (a draw is no different
/// from a loss in PvE), `None` = it never resolved (a degenerate stalemate — a balance/AI bug rather
/// than a silent result). Runs under the **analysis** [`Ruleset`] (bounded round horizon) so the
/// combat is finite, matching how the balance tooling sets up games (§0).
pub fn auto_resolve(heroes: Vec<Actor>, foes: Vec<Actor>, seed: u64) -> Option<bool> {
    auto_resolve_with(heroes, foes, seed, Ruleset::analysis())
}

/// As [`auto_resolve`], but with an explicit [`Ruleset`] (round/roster bounds).
pub fn auto_resolve_with(
    heroes: Vec<Actor>,
    foes: Vec<Actor>,
    seed: u64,
    ruleset: Ruleset,
) -> Option<bool> {
    let game = Deckbound;
    let mut state = battle_state_with(heroes, foes, false, seed, ruleset);
    for _ in 0..MAX_STEPS {
        if let Some(outcome) = game.outcome(&state) {
            return Some(matches!(outcome, Outcome::Win(PlayerId(0))));
        }
        let actions = game.legal_actions(&state);
        let action = greedy(&state, &actions);
        if game.apply(&mut state, &action).is_err() {
            return None;
        }
    }
    None
}

/// A moderately-greedy hero policy: commit melee to the Vanguard, hold and fight, strike the front,
/// or play a power if there's nothing to hit. Picks one action; called repeatedly. Public so the
/// campaign can suggest a combat move to the player.
pub fn greedy(state: &State, actions: &[Action]) -> Action {
    use Action::*;
    match state.phase {
        // §4 Marshal: each unit defaults to its stat-based intention (which is what the greedy
        // wants — melee fronts/flanks, ranged deals), so the greedy casts any beneficial `Standing` buff,
        // then advances. **Deploy resolves the whole engagement schedule** (targeting is the resolver's
        // job now, not an interactive choice).
        Phase::Marshal => best_play(state, actions).unwrap_or(Deploy),
        // Engage is transient — the round resolves inside Deploy, so this is never a resting choice.
        Phase::Engage => Deploy,
        // The Clash is off in the solver; if somehow reached, just strike.
        Phase::Clash => Play(Move::Strike),
        Phase::Menu(_) => ToMenu,
    }
}

/// The best `PlayCard` for the committing side — the highest-**scoring** playable card, so a member
/// spends its one-per-role play on its strongest option (and deeper cards get used), not the first it
/// happens to find. Scoring ranks **damage** (wins the race) over **amplification** (Empower/Haste —
/// indirect offense, race-positive) over proactive **debuffs**, with reactive heals last (a Mend at
/// Muster heals nobody — the solver shouldn't burn its play on it). Returns `None` if no card is
/// playable.
fn best_play(state: &State, actions: &[Action]) -> Option<Action> {
    let side = state.plan.committing;
    actions
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::PlayCard(i, idx) => state
                .s_pool(side)
                .get(i)
                .and_then(|act| act.actions.get(idx))
                .filter(|c| cast_still_useful(state.s_pool(side), i, c))
                .map(|c| (a, play_score(c))),
            _ => None,
        })
        .max_by_key(|&(_, score)| score)
        .filter(|&(_, score)| score > 0)
        .map(|(a, _)| a)
}

/// Would casting `card` from `pool[i]` still **do** anything (greedy policy)? A self-buff whose effect is
/// already in force — or an ally-buff with no eligible ally (a solo Wall's Cover/Thorns) — is a wasted
/// Tempo spend that, repeated, starves the caster's own strikes (a Wall that re-casts Cover every round
/// never attacks) and can loop forever (a self-Haste/Empower). So skip re-casting a buff whose effect the
/// caster already carries (placed Guard/Cover/Thorns token, standing Empower, held Lifeline), and skip an
/// ally-targeting buff (Cover / Thorns) when no other living ally exists to receive it. First casts and
/// all offensive / heal / self-or-ally cards that still land are unaffected. Used only by the greedy.
fn cast_still_useful(pool: &[Actor], i: usize, card: &crate::cards::Card) -> bool {
    use crate::actor::Token;
    use crate::cards::Effect::*;
    let caster = &pool[i];
    let has_other_ally = pool.iter().enumerate().any(|(j, a)| j != i && !a.is_down());
    card.effects.iter().all(|e| match e {
        // Self-buffs that don't usefully stack — skip if already in force.
        Empower { .. } => caster.might_bonus == 0,
        Lifeline => !caster.cannot_fall,
        // Tempo grants (Haste = ally, BankCadence = self) make *more* Tempo than they cost — re-casting
        // loops forever. Useful only while a recipient is below its refresh budget; once Tempo is topped
        // up the cast is wasted, so the greedy spends its Tempo acting instead. (Sanctuary's bundled
        // Haste is gated by its own `one_shot`, so it is exempt from this stall test.)
        Haste { .. } if !card.one_shot => pool
            .iter()
            .any(|a| !a.is_down() && a.tempo < a.eff_cadence() as i32),
        BankCadence { .. } if !card.one_shot => caster.tempo < caster.eff_cadence() as i32,
        Brace { .. } | Guard { .. } => !caster
            .tokens
            .iter()
            .any(|t| matches!(t, Token::Guard { .. })),
        // Cover always targets *other* allies — useless solo, and a no-op once one is placed.
        Cover => {
            has_other_ally
                && !caster
                    .tokens
                    .iter()
                    .any(|t| matches!(t, Token::Cover { .. }))
        }
        // Thorns wards the most-wounded ally (may be self); skip only once one is already in force.
        Thorns { .. } => !pool
            .iter()
            .any(|a| a.tokens.iter().any(|t| matches!(t, Token::Thorns { .. }))),
        // Reactive restores — useful only while someone is wounded; on a full-health party they heal
        // nobody, so re-casting just burns Tempo (and loops). Skip until there is damage to undo.
        Mend { .. } | Recover => pool
            .iter()
            .any(|a| !a.is_down() && a.defense.health.remaining() < a.defense.health.max()),
        // Everything else (Damage, debuffs, Charge, the one-shot bundles, etc.) always still applies.
        _ => true,
    })
}

/// A heuristic value for playing `card` now (greedy policy). Damage ≫ amplification ≫ proactive debuff
/// ≫ minor buff ≫ reactive heal. The magnitude terms give a mild preference for the deeper (stronger)
/// card of a track. Used only by the greedy resolver — not a rule.
fn play_score(card: &crate::cards::Card) -> i32 {
    use crate::cards::Effect::*;
    card.effects
        .iter()
        .map(|e| match e {
            Damage { power } => 100 + *power as i32,
            Haste { tempo } => 50 + *tempo as i32,
            Empower { might } => 50 + *might as i32,
            Slow { .. } | Confuse { .. } | Suppress { .. } | Stagger | Shove | Disarm | Rout => 40,
            // §10 token effects. Burn (DoT damage) and Charge (a damage setup) rank near offense;
            // proactive debuff tokens (Mark/Mire) with the other debuffs; Smoke/Silence as enablers.
            Burn { stacks, power } => 80 + (*stacks * *power) as i32,
            Charge { amount } => 60 + *amount as i32,
            Mark { .. } | Mire { .. } | Silence | Smoke | Pin => 40,
            // Sunder/Defang (Controller stat-drops). Sunder lowers the foe's per-phase wall — it is the
            // amp that lets the party crack a foe it can't out-burst, so rank it above the other debuffs
            // (a Sunder this Fray makes this round's strikes land). Defang softens incoming blows.
            Sunder { toughness } => 70 + *toughness as i32,
            Defang { might } => 45 + *might as i32,
            Guard { .. }
            | BankCadence { .. }
            | Ward
            | Lifeline
            | Brace { .. }
            | Cover
            | Thorns { .. } => 20,
            // Reactive: only worth it once someone is hurt — at Muster (full health) it is a
            // wasted play, so the greedy ranks it below acting.
            Mend { .. } | Recover => 5,
        })
        .sum()
}

// ===========================================================================
// The exact battle solver — perfect PvE combat play
// (`computability-and-balance.md` §10.7).
//
// Luck off (Clash off), so a §4 battle is **deterministic** and the creatures
// are a **fixed environment**: the foe AI runs *inside* `Game::apply`
// (`foe_fray` / `foe_volley`), and `legal_actions` only ever offers the
// committing side — always the heroes in PvE (`plan.committing == 0`). So a
// battle is a finite-horizon **single-agent** problem: every hero action leads
// to exactly one successor state, and "perfect play" is **exact backward
// induction** over the bounded `Ruleset::analysis()` horizon — no minimax, no
// evaluation heuristic (§0.4). This search *is* the §5 ground-truth resolver
// `P` and the strong policy the role-weight / encounter-suite measurements
// consume.
//
// The engine sequences a phase's commitments as single-action steps (the
// per-phase pile), so order-independence (§1.9) is collapsed by the
// **transposition table** rather than a separate set-enumerator. The only
// in-place cycles are the free Standoff position toggles; an on-stack set
// detects them (revisiting a Markov state can never improve a single-agent
// maximum).
//
// Phases (§10.7): A — `combat_actions` enumerator + `state_key` canonical hash;
// B — reachability (`winnable`); C — graded lexicographic `solve`. D (luck-on
// expectimax) is deferred — the ratified first cut is luck-off only.
// ===========================================================================

/// The lexicographic objective (§10.7): **win → fewer rounds → fewest characters
/// downed → most Health remaining**. Laid out so the derived `Ord`'s *maximum* is
/// the best line — `win` (`false < true`) dominates; among wins `neg_rounds` /
/// `neg_downed` make *fewer* better and `health` makes *more* better. For a
/// non-win (loss or round-cap draw) the round/downed tiebreakers are neutral
/// (every win beats every non-win on `win` alone); `health` carries a mild
/// "how close" gradient that is harmless to the win/loss verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value {
    pub win: bool,
    neg_rounds: i64,
    neg_downed: i64,
    health: i64,
}

impl Value {
    /// The fold identity / cycle-edge value: strictly below every reachable leaf,
    /// so the search prefers any real continuation over revisiting a state.
    const LOSS: Value = Value {
        win: false,
        neg_rounds: i64::MIN,
        neg_downed: i64::MIN,
        health: i64::MIN,
    };
}

/// The result of [`solve`]: the perfect-play value plus the witness line and perf
/// telemetry. `rounds` / `downed` are meaningful only when `win`.
#[derive(Clone, Debug)]
pub struct Solution {
    /// Is the battle winnable within the horizon under optimal play?
    pub win: bool,
    /// Battle-par: rounds to clear under the optimal line (0 if not a win).
    pub rounds: u32,
    /// Hero characters downed at the win leaf (0 if not a win).
    pub downed: u32,
    /// Total hero Health cards remaining at the leaf (the survival-margin tiebreak).
    pub health: u32,
    /// States expanded (perf telemetry / the §4 budget signal).
    pub nodes: u64,
    /// The node budget was hit — the result may be unreliable (raise the budget or
    /// shrink the encounter). Should not trip inside the analysis envelope.
    pub overflowed: bool,
    /// The optimal (perfect-play) witness line — the battle-par trace, also the
    /// readable transcript and the strong policy for role-weight.
    pub line: Vec<Action>,
}

/// Safety backstop on states expanded, so a mis-bounded encounter returns (with
/// `overflowed`) rather than spinning. The analysis envelope keeps real searches
/// far below this.
const MAX_NODES: u64 = 50_000_000;

// ---- Phase A: the canonical, hashable state key ----------------------------

/// The resolution-relevant mutable state of one Actor (the immutable Form / cards
/// / stats are constant across a search from a fixed roster, so they are omitted —
/// the index identifies the unit).
#[derive(Clone, PartialEq, Eq, Hash)]
struct ActorKey {
    remaining: u32,
    pile: u32,
    tempo: i32,
    might_bonus: u32,
    /// fallen·stunned·shoved·disarmed·routed·cannot_fall·free_slip_used bitset.
    flags: u8,
    /// Canonicalized (sorted) utility tokens.
    tokens: Vec<(u8, u32)>,
}

fn token_key(t: &Token) -> (u8, u32) {
    match t {
        Token::Guard { toughness } => (0, *toughness),
        Token::Cover { ally } => (1, *ally as u32),
        Token::Mark { finesse } => (2, *finesse),
        Token::Mire { cadence } => (3, *cadence),
        Token::Sunder { toughness } => (4, *toughness),
        Token::Defang { might } => (5, *might),
        Token::Burn { power } => (6, *power),
        Token::Thorns { power } => (7, *power),
        Token::Charge => (8, 0),
        Token::Smoke => (9, 0),
    }
}

fn actor_key(a: &Actor) -> ActorKey {
    let flags = (a.fallen as u8)
        | (a.stunned as u8) << 1
        | (a.shoved as u8) << 2
        | (a.disarmed as u8) << 3
        | (a.routed as u8) << 4
        | (a.cannot_fall as u8) << 5
        | (a.free_slip_used as u8) << 6;
    let mut tokens: Vec<(u8, u32)> = a.tokens.iter().map(token_key).collect();
    tokens.sort_unstable();
    ActorKey {
        remaining: a.defense.health.remaining(),
        pile: a.defense.health_pile(),
        tempo: a.tempo,
        might_bonus: a.might_bonus,
        flags,
        tokens,
    }
}

/// A canonical, hashable encoding of the combat state — the transposition key.
/// Captures everything that determines future resolution and legal moves:
/// round / phase, the per-actor mutable state, and the round plan (positions,
/// locks, pins, acted flags, the attacked-map, charges, deferred spells). The
/// attacked-map's inner lists and the token lists are **sorted** (membership-only,
/// §4.6 / §2.2 — so different orderings are the same state); charges and deferred
/// keep declaration order (their resolution may be order-sensitive).
#[derive(Clone, PartialEq, Eq, Hash)]
struct StateKey {
    round: u32,
    phase: u8,
    committing: u8,
    heroes: Vec<ActorKey>,
    creatures: Vec<ActorKey>,
    hero_intent: Vec<crate::actor::Intention>,
    foe_intent: Vec<crate::actor::Intention>,
    hero_group: Vec<usize>,
    foe_group: Vec<usize>,
    hero_acted: Vec<bool>,
    foe_acted: Vec<bool>,
    deferred: Vec<(u8, usize, String)>,
}

fn phase_tag(p: &Phase) -> u8 {
    match p {
        Phase::Menu(_) => 0,
        Phase::Marshal => 1,
        Phase::Engage => 2,
        Phase::Clash => 3,
    }
}

fn state_key(s: &State) -> StateKey {
    StateKey {
        round: s.round,
        phase: phase_tag(&s.phase),
        committing: s.plan.committing,
        heroes: s.heroes.iter().map(actor_key).collect(),
        creatures: s.creatures.iter().map(actor_key).collect(),
        hero_intent: s.plan.hero_intent.clone(),
        foe_intent: s.plan.foe_intent.clone(),
        hero_group: s.plan.hero_group.clone(),
        foe_group: s.plan.foe_group.clone(),
        hero_acted: s.plan.hero_acted.clone(),
        foe_acted: s.plan.foe_acted.clone(),
        deferred: s
            .plan
            .deferred
            .iter()
            .map(|d| (d.side, d.caster, d.card.name.clone()))
            .collect(),
    }
}

/// Phase A: the legal **combat** actions to branch on — `legal_actions` minus the
/// non-combat escape (`ToMenu`). In a luck-off PvE battle every remaining action
/// is a hero commitment (positions / casts / strikes / charges / pass / deploy).
fn combat_actions(game: &Deckbound, state: &State) -> Vec<Action> {
    game.legal_actions(state)
        .into_iter()
        .filter(|a| !matches!(a, Action::ToMenu))
        .collect()
}

/// An enemy unit's interchangeability signature: type (name), full mutable state,
/// and position/lock/pin/acted flags. (See [`target_class`].)
/// Phase E symmetry pruning. In the engagement-schedule model targeting is **not** an interactive choice
/// (the resolver picks targets, §4.6), so the only branching actions are intention declarations, Standing
/// casts, Pass, and Deploy — there is no `Target`/`Charge` fan-out to collapse. Pass through unchanged.
/// *(Follow-on: dedup symmetric units' identical intention sets to prune full-roster branching.)*
fn combat_actions_dedup(game: &Deckbound, state: &State) -> Vec<Action> {
    combat_actions(game, state)
}

/// Move-ordering: put the greedy policy's pick first, so a reachability search tends
/// to hit a winning line early (the §10.7 "greedy as move-ordering oracle" — speed
/// only, never correctness). No-op if greedy's choice was a pruned symmetric duplicate.
fn order_greedy_first(game: &Deckbound, state: &State, actions: &mut [Action]) {
    let pick = greedy(state, &combat_actions(game, state));
    if let Some(pos) = actions.iter().position(|a| *a == pick) {
        actions.swap(0, pos);
    }
}

// ---- Phases B & C: the memoized backward-induction search ------------------

/// The exact value of a terminal state (§10.7 objective). A hero win scores
/// `win = true` with the round/downed/health tiebreaks; anything else (party
/// fell, or round-cap draw) is a non-win.
fn leaf_value(state: &State) -> Value {
    let health: i64 = state
        .heroes
        .iter()
        .map(|h| h.defense.health.remaining() as i64)
        .sum();
    if matches!(state.outcome, Some(Outcome::Win(PlayerId(0)))) {
        let downed = state
            .heroes
            .iter()
            .filter(|h| h.is_down() || h.fallen)
            .count() as i64;
        Value {
            win: true,
            neg_rounds: -(state.round as i64),
            neg_downed: -downed,
            health,
        }
    } else {
        Value {
            win: false,
            neg_rounds: 0,
            neg_downed: 0,
            health,
        }
    }
}

/// The backward-induction search with a transposition table and on-stack cycle
/// detection.
struct Solver<'a> {
    game: &'a Deckbound,
    memo: HashMap<StateKey, Value>,
    stack: HashSet<StateKey>,
    nodes: u64,
    overflow: bool,
    max_nodes: u64,
}

impl Solver<'_> {
    /// The exact perfect-play value of `state` (max over hero actions; the foe is a
    /// fixed environment resolved inside `apply`).
    fn run(&mut self, state: &State) -> Value {
        if state.outcome.is_some() {
            return leaf_value(state);
        }
        let key = state_key(state);
        if let Some(v) = self.memo.get(&key) {
            return *v;
        }
        if self.nodes >= self.max_nodes {
            self.overflow = true;
            return Value::LOSS;
        }
        self.nodes += 1;
        // A state revisited while still on the stack is a no-progress cycle. Key the path-stack on the
        // MATERIAL state (round excluded): a round transition with **no state change** re-enters the
        // same material state with fewer rounds — a dominated loop that can never improve a single-agent
        // maximum — so prune it. This collapses non-terminating stalemates that would otherwise explore
        // all five rounds × the per-round branching (§0.4).
        let mut mkey = key.clone();
        mkey.round = 0;
        if !self.stack.insert(mkey.clone()) {
            return Value::LOSS;
        }
        let mut best = Value::LOSS;
        for action in combat_actions_dedup(self.game, state) {
            let mut child = state.clone();
            child.log.clear(); // keeps clones cheap; the log is outside the key.
            if self.game.apply(&mut child, &action).is_ok() {
                let v = self.run(&child);
                if v > best {
                    best = v;
                }
            }
        }
        self.stack.remove(&mkey);
        self.memo.insert(key, best);
        best
    }

    /// Reconstruct the optimal line by walking from the root, at each step taking the **highest-value
    /// action that leads to an as-yet-unvisited material state**. The unseen guard is essential: many
    /// actions are value-neutral (e.g. Standoff position toggles `SetVanguard(0)`↔`SetRearguard(0)`), and
    /// a plain greedy walk oscillates between them forever. Skipping revisited material states forces
    /// progress; an optimal **win** line never needs to revisit one (a revisit only wastes rounds), so
    /// this still extracts an optimal-value line.
    fn witness(&mut self, root: &State) -> Vec<Action> {
        let mat = |s: &State| {
            let mut k = state_key(s);
            k.round = 0; // material key: collapse the round so a no-progress revisit is caught
            k
        };
        let mut line = Vec::new();
        let mut cur = root.clone();
        let mut seen: HashSet<StateKey> = HashSet::new();
        seen.insert(mat(&cur));
        let mut guard = 0u32;
        while cur.outcome.is_none() && guard < 100_000 {
            guard += 1;
            // All applicable actions with their optimal-play value and resulting state.
            let mut cands: Vec<(Action, Value, State)> = Vec::new();
            for action in combat_actions_dedup(self.game, &cur) {
                let mut child = cur.clone();
                child.log.clear();
                if self.game.apply(&mut child, &action).is_ok() {
                    let v = self.run(&child);
                    cands.push((action, v, child));
                }
            }
            // Highest value first; among those, the first that reaches an unvisited material state.
            cands.sort_by_key(|c| std::cmp::Reverse(c.1));
            match cands
                .into_iter()
                .find(|(_, _, child)| !seen.contains(&mat(child)))
            {
                Some((action, _, child)) => {
                    seen.insert(mat(&child));
                    line.push(action);
                    cur = child;
                }
                None => break, // every move revisits a seen state — stop rather than loop
            }
        }
        line
    }
}

/// **Phase C — the exact battle solver.** Compute perfect PvE combat play for
/// `heroes` vs `foes` under `ruleset` (use [`Ruleset::analysis`] for the bounded,
/// exactly-searchable envelope). Returns the lexicographic value (§10.7), the
/// witness line, and perf telemetry. Luck-off / deterministic; the foe side is the
/// game's own fixed creature AI.
pub fn solve(heroes: Vec<Actor>, foes: Vec<Actor>, seed: u64, ruleset: Ruleset) -> Solution {
    solve_within(heroes, foes, seed, ruleset, MAX_NODES)
}

/// Budgeted graded solve — like [`solve`] but caps the search at `max_nodes`. Unlike
/// [`winnable_within`] (which short-circuits on the first win), the graded objective must explore
/// every line to prove a value is *optimal*, so even a winnable battle can exhaust a small budget;
/// when it does, `overflowed = true` flags the verdict as **budget-limited** (the returned value is a
/// lower bound on the true optimum — a deeper/better line may have been cut). The balance harness uses
/// a modest budget so a 5-hero graded solve never hangs, and reads `overflowed` as the §0.4
/// searchability-bound **needs-decision signal** (raise the budget, shrink the encounter, or accept it).
pub fn solve_within(
    heroes: Vec<Actor>,
    foes: Vec<Actor>,
    seed: u64,
    ruleset: Ruleset,
    max_nodes: u64,
) -> Solution {
    let game = Deckbound;
    let root = battle_state_with(heroes, foes, false, seed, ruleset);
    let mut solver = Solver {
        game: &game,
        memo: HashMap::new(),
        stack: HashSet::new(),
        nodes: 0,
        overflow: false,
        max_nodes,
    };
    let value = solver.run(&root);
    let line = solver.witness(&root);
    Solution {
        win: value.win,
        rounds: if value.win {
            (-value.neg_rounds) as u32
        } else {
            0
        },
        downed: if value.win {
            (-value.neg_downed) as u32
        } else {
            0
        },
        health: value.health.max(0) as u32,
        nodes: solver.nodes,
        overflowed: solver.overflow,
        line,
    }
}

/// A reachability search for the **boolean** objective: it short-circuits on the
/// first winning leaf (no need to find the *best* win), which — with greedy
/// move-ordering and symmetry pruning — makes "winnable?" far cheaper than the
/// graded [`solve`]. Memoizes winnability per canonical state.
struct Reach<'a> {
    game: &'a Deckbound,
    seen: HashMap<StateKey, bool>,
    stack: HashSet<StateKey>,
    nodes: u64,
    overflow: bool,
    max_nodes: u64,
}

impl Reach<'_> {
    fn win(&mut self, state: &State) -> bool {
        match state.outcome {
            Some(Outcome::Win(PlayerId(0))) => return true,
            Some(_) => return false, // party fell, or round-cap draw
            None => {}
        }
        let key = state_key(state);
        if let Some(v) = self.seen.get(&key) {
            return *v;
        }
        if self.nodes >= self.max_nodes {
            self.overflow = true;
            return false;
        }
        self.nodes += 1;
        // Material key (round excluded): a round transition with no state change re-enters the same
        // material state with fewer rounds — a dominated no-progress loop, not a new position. Keying
        // the path-stack on it prunes stalemates in ~1 round instead of exploring all five (§0.4).
        let mut mkey = key.clone();
        mkey.round = 0;
        if !self.stack.insert(mkey.clone()) {
            return false; // no-progress loop (same material state, round advanced)
        }
        let mut actions = combat_actions_dedup(self.game, state);
        order_greedy_first(self.game, state, &mut actions);
        let mut result = false;
        for action in actions {
            let mut child = state.clone();
            child.log.clear();
            if self.game.apply(&mut child, &action).is_ok() && self.win(&child) {
                result = true;
                break; // existential: one winning line suffices
            }
        }
        self.stack.remove(&mkey);
        self.seen.insert(key, result);
        result
    }
}

/// **Phase B — reachability.** Is this battle winnable within the horizon under
/// optimal play? The Spec §0.4 "winnable within the horizon?" boolean, under the
/// analysis envelope — the strong-policy answer the role-weight / encounter-suite
/// measurements key on. (Cheaper than [`solve`]: it stops at the first win.)
pub fn winnable(heroes: Vec<Actor>, foes: Vec<Actor>, seed: u64) -> bool {
    winnable_within(heroes, foes, seed, MAX_NODES).0
}

/// Budgeted reachability — like [`winnable`] but caps the search at `max_nodes`. Returns
/// `(winnable, overflowed)`. A **win short-circuits** (cheap regardless of budget — greedy
/// move-ordering finds a real win early), while **confirming a loss is exhaustive**, so a losing
/// battle that explores a large tree (e.g. a stalemate dragging to the round cap) hits the budget and
/// returns `(false, true)` — a §0.4 "not winnable within the bounded search" verdict. The balance
/// harness uses a modest budget so loss-confirmation never hangs; `overflowed = true` flags a
/// budget-limited verdict (don't read a budget-limited `false` as a *proven* loss).
pub fn winnable_within(
    heroes: Vec<Actor>,
    foes: Vec<Actor>,
    seed: u64,
    max_nodes: u64,
) -> (bool, bool) {
    winnable_within_rules(heroes, foes, seed, max_nodes, Ruleset::analysis())
}

/// As [`winnable_within`], but under an explicit [`Ruleset`] — so a simulation can run against a chosen
/// rule **subset** (disabled phases skipped, disabled behaviors not consulted) and key its result to it.
pub fn winnable_within_rules(
    heroes: Vec<Actor>,
    foes: Vec<Actor>,
    seed: u64,
    max_nodes: u64,
    ruleset: Ruleset,
) -> (bool, bool) {
    let game = Deckbound;
    let root = battle_state_with(heroes, foes, false, seed, ruleset);
    let mut reach = Reach {
        game: &game,
        seen: HashMap::new(),
        stack: HashSet::new(),
        nodes: 0,
        overflow: false,
        max_nodes,
    };
    let win = reach.win(&root);
    (win, reach.overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios::campaign;

    /// Diagnostic (run on demand): print win/lose for a clean-slate vs upgraded character against
    /// scaling foe counts, to calibrate encounter difficulty. `cargo test probe_power -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_power() {
        use crate::currency::Currency;
        use crate::encounter::{EncounterCard, RosterEntry};
        use crate::form::StatCard;
        use crate::scenarios::{build_character, build_encounter_foes, rewards_for};

        let enc = |creature: &str, count: u32| EncounterCard {
            name: "probe".into(),
            currency: Currency::Iron,
            strategy: "aggressor".into(),
            foes: vec![RosterEntry {
                creature: creature.into(),
                from_level: 1,
                base: count,
                growth: 0,
            }],
            scaling: StatCard::default(),
        };
        for k in 1..=8 {
            let foes = build_encounter_foes(&enc("Husk", k), 1);
            let bare = vec![build_character("Novice", &[])];
            let wall = vec![build_character("Novice", &rewards_for(Currency::Iron))];
            let b = auto_resolve(bare, foes.clone(), 1);
            let u = auto_resolve(wall, foes, 1);
            println!("Husk x{k}: bare={b:?}  Wall-kit={u:?}");
        }
    }

    #[test]
    fn tempo_refreshes_to_cadence() {
        // §3 tripwire: the Tempo pool's *count* is Cadence. A freshly built/refreshed actor holds
        // exactly Cadence-many Tempo cards. If this drifts, the Cadence·Finesse·Tempo identity is broken.
        use crate::scenarios::build_character;
        let a = build_character("Novice", &[]);
        assert_eq!(
            a.tempo, a.offense.cadence as i32,
            "a refreshed actor must hold Cadence-many Tempo cards"
        );
    }

    // (Removed `higher_finesse_crosses_an_equal_one_card_tie_is_held`: the static-ranks **crossing
    // contest** it tested was retired with the old charge-gauntlet model; the §4.6 Volley charge / flank
    // replaces it, and the evade contest is covered by `combat::evade_contest_strictly_exceeds_the_volley`.)

    // (Removed `a_holding_wall_plays_its_role_cards`: the gauntlet auto-resolves the Vanguard, so
    // there is no interactive Wall play window in v1 — a known limitation, see role-card-redesign.)

    #[test]
    fn auto_resolve_terminates_on_every_campaign_scenario() {
        // The greedy policy, Clash off, must drive every real scenario to a decisive result —
        // no stalemate, no error. (Win or loss is fine; *non-termination* is the bug we catch.)
        for s in campaign() {
            let (heroes, foes) = s.roster();
            assert!(
                auto_resolve(heroes, foes, 1).is_some(),
                "scenario {:?} did not resolve under the greedy policy",
                s.name
            );
        }
    }

    // ---- the exact battle solver (§10.7) ----

    use crate::currency::Currency;
    use crate::encounter::{EncounterCard, RosterEntry};
    use crate::form::StatCard;
    use crate::scenarios::{build_character, build_encounter_foes, rewards_for};

    /// A one-creature encounter card (mirrors `probe_power`'s helper).
    fn solo_encounter(creature: &str, count: u32) -> EncounterCard {
        EncounterCard {
            name: "toy".into(),
            currency: Currency::Iron,
            strategy: "aggressor".into(),
            foes: vec![RosterEntry {
                creature: creature.into(),
                from_level: 1,
                base: count,
                growth: 0,
            }],
            scaling: StatCard::default(),
        }
    }

    /// Diagnostic (run on demand): the exact solver's verdict, battle-par, and **node count**
    /// (the branching-factor reality check, §10.7) for each campaign scenario, beside the greedy
    /// result. `cargo test probe_solver -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_solver() {
        for s in campaign() {
            let (heroes, foes) = s.roster();
            let greedy = auto_resolve(heroes.clone(), foes.clone(), 1);
            let n = heroes.len() + foes.len();
            let win = winnable(heroes.clone(), foes.clone(), 1);
            println!("{:<24} units={n} greedy={greedy:?} winnable={win}", s.name);
            // The graded par search has no symmetry to exploit on distinct heroes, so it is
            // expensive on the largest hand-authored rosters; the boolean `winnable` above is the
            // scalable instrument. Run the graded solve only for in-envelope (small) encounters.
            if n <= 6 {
                let sol = solve(heroes, foes, 1, Ruleset::analysis());
                println!(
                    "    solve: win={} par={} downed={} health={} nodes={} overflow={}",
                    sol.win, sol.rounds, sol.downed, sol.health, sol.nodes, sol.overflowed
                );
            }
        }
    }

    #[test]
    fn solver_wins_a_trivially_winnable_battle() {
        // A fully-kitted Iron character vs a single weak Husk: the greedy already wins this
        // (see `probe_power`), so the exact solver — which is never worse — must find a win,
        // with a positive battle-par round count and the witness line non-empty.
        let hero = vec![build_character("Novice", &rewards_for(Currency::Iron))];
        let foes = build_encounter_foes(&solo_encounter("Husk", 1), 1);
        let sol = solve(hero, foes, 1, Ruleset::analysis());
        assert!(sol.win, "a kitted hero must beat one Husk");
        assert!(
            !sol.overflowed,
            "the search must stay within the node budget"
        );
        assert!(
            (1..=5).contains(&sol.rounds),
            "battle-par within the horizon"
        );
        assert!(!sol.line.is_empty(), "a winning line must be witnessed");
    }

    #[test]
    fn solver_reports_unwinnable_when_the_party_cannot_damage() {
        // A known-answer loss: a pacifist (Might 0, no role cards, no attack profile) can never
        // empty a foe's health pool, so no line wins within the horizon — the solver must say so,
        // exactly (no heuristic over-optimism). This is the lower-bound guard.
        use crate::actor::Attack;
        let mut pacifist = build_character("Novice", &[]);
        pacifist.offense.might = 0;
        pacifist.might_bonus = 0;
        pacifist.actions.clear();
        pacifist.attack = Attack::Neither;
        let foes = build_encounter_foes(&solo_encounter("Husk", 1), 1);
        assert!(
            !winnable(vec![pacifist], foes, 1),
            "a 0-damage party cannot win — the solver must not claim otherwise"
        );
    }

    #[test]
    fn optimal_is_never_worse_than_greedy() {
        // The defining invariant (§10.7 Phase B): the exact optimum dominates any fixed policy.
        // So on every (small) campaign scenario the greedy wins, perfect play must win too. We
        // bound roster size to keep the exhaustive search fast in CI.
        for s in campaign() {
            let (heroes, foes) = s.roster();
            if heroes.len() + foes.len() > 5 {
                continue;
            }
            if auto_resolve(heroes.clone(), foes.clone(), 1) == Some(true) {
                assert!(
                    winnable(heroes, foes, 1),
                    "scenario {:?}: greedy wins but the exact solver did not — optimal < greedy is impossible",
                    s.name
                );
            }
        }
    }
}
