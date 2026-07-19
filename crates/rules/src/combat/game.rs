//! **The regions combat, as a [`Game`].** The state machine a runner, a solver, or an explorer drives - the
//! same rules as [`super::regions`], re-expressed through [`options`](Game::options) / [`apply`](Game::apply) /
//! [`outcome`](Game::outcome) so every consumer walks one machine.
//!
//! A whole fight is a walk through **declarations**, with no setup: there is **one region per side** - the party
//! stands on its ground (region 0), the foes on theirs (region 1), posts weapon-derived (melee front, ranged
//! back) and fixed at construction. Nobody ever chooses a partition, so the fight opens directly on round 1.
//!
//! **Declare** (each round, one choice per living body - heroes *and* foes): every body declares its [`Act`]
//! through the same loop. A hero's [`options`](Game::options) are its real choices; a **foe's are a single
//! option** - the act committed by the [`Decider`] this game plugs in for its foes (a deterministic [`Instinct`]
//! policy) - so a foe "declares" too, but the driver has nothing to pick and auto-advances it. When the last body
//! declares, [`apply`](Game::apply) resolves the whole round ([`play_round`]) from the acts everyone committed.
//!
//! Where a decision *comes from* lives behind [`Decider`], so nothing downstream (resolution, narration, the
//! solver) can tell a scripted foe from a random one or a human - the abstraction is the whole point.
//!
//! Everything flows through one system: a creature is not folded into `apply` as a hidden script, it takes its
//! turn like a hero, its turn just has one legal move. That keeps this a **single-agent reachability** machine
//! anyway - a foe multiplies the branching by exactly one - so the solver is unaffected (creatures are perfectly
//! predictable), while every consumer that walks the loop (a UI, an explorer) now *sees* the foe's declaration
//! instead of having to reconstruct it.
//!
//! Resolution is still *inside* `apply`, not a set of choices, because within a round nothing is a player
//! decision - the schedule is fixed and the slip answers are part of each [`Act`]. (In perfect-information PvE a
//! slip's answer declared up front is equivalent to one chosen on reveal, since the party already knows what the
//! scripted foes will commit - so folding it into the declaration loses nothing a solver could use.)

use std::collections::HashMap;

use super::regions::{Act, Board, MAX_ROUNDS, Rank, foe_act, greedy_act, legal_acts, play_round};
use super::resolve::{Combatant, Side};
use crate::core::{Game, Outcome};

/// A choice in the combat game: a body declares its act for the round. There is no setup - the formation is
/// fixed at construction (one region per side), so a fight opens on round 1's first declaration.
#[derive(Clone, Debug, PartialEq)]
pub enum Choice {
    /// A round: this body does `act`.
    Act(Act),
}

/// The whole position: the bodies, their formation, whose declaration is pending, and the round.
#[derive(Clone, Debug)]
pub struct State {
    board: Board,
    /// The **declaration order** every round: the party (seat order) then the foes. Every body that acts appears
    /// here once; the `next` cursor walks it, skipping the fallen and the already-declared.
    order: Vec<usize>,
    /// The declaration cursor: an index into `order`, the next living body that has not yet declared this round.
    /// A foe reaches this cursor like a hero - it just has one legal act.
    next: usize,
    /// Each unit's declared act this round (`None` until declared); reset each round.
    pending: Vec<Option<Act>>,
    round: usize,
}

impl State {
    /// Begin a fight at round 1, with **one region per side**: every party body on region 0 (its ground), every
    /// foe on region 1 (theirs). Posts are weapon-derived at construction, for heroes and foes alike (a
    /// ranged-only body stands back, everything else front). There is no setup phase - the fight opens directly
    /// on round 1's first declaration.
    pub fn new(units: Vec<Combatant>) -> State {
        let n = units.len();
        // One region per side: the party's formation faces the foes' formation, with no ground between them.
        let regions: Vec<u8> = (0..n)
            .map(|i| if units[i].side == Side::Party { 0 } else { 1 })
            .collect();
        let party: Vec<usize> = (0..n).filter(|&i| units[i].side == Side::Party).collect();
        let foes: Vec<usize> = (0..n).filter(|&i| units[i].side == Side::Foe).collect();
        // Everyone declares each round, party first then foes - the one loop the whole round flows through.
        let order: Vec<usize> = party.iter().chain(foes.iter()).copied().collect();
        let board = Board::new(units, regions);
        let mut s = State {
            board,
            order,
            next: 0,
            pending: vec![None; n],
            round: 1,
        };
        s.next = s.next_undeclared(0).unwrap_or(s.order.len());
        s
    }

    /// The next living body in the declaration order at or after `from` that has not declared, or `None` if all
    /// have. Walks heroes and foes alike - the cursor does not care which side is next.
    fn next_undeclared(&self, from: usize) -> Option<usize> {
        self.order[from..]
            .iter()
            .position(|&i| !self.board.units[i].fallen && self.pending[i].is_none())
            .map(|off| from + off)
    }

    /// Read-only view of the board, for a driver or renderer.
    pub fn board(&self) -> &Board {
        &self.board
    }
    pub fn round(&self) -> usize {
        self.round
    }

    /// The acts declared **so far this round**, indexed by unit (`None` = not yet declared, or a foe/fallen body).
    /// A renderer uses this to reconstruct the round it just resolved - who slipped where, and so who a slip
    /// contest would have caught - which the board alone cannot explain.
    pub fn pending(&self) -> &[Option<Act>] {
        &self.pending
    }

    /// The **body whose declaration is pending** right now - the body (hero or foe) declaring its act this round.
    /// `None` if nobody is deciding (a forced/terminal state). A UI names it so an action is never ambiguous about
    /// *which* body. (A foe reaching this cursor has a single option, so a driver auto-advances it without asking.)
    pub fn deciding(&self) -> Option<usize> {
        self.order.get(self.next).copied()
    }
}

/// **A Decider is where a body's committed act comes from** - the abstraction the rest of the machine stays blind
/// behind. The same three kinds share it: [`Instinct`] (deterministic, scripted), a weighted-random policy, and a
/// human at the UI. Resolution ([`play_round`]) and narration only ever see the committed [`Act`], never *which*
/// Decider produced it - so one body of code plays a scripted foe, a random one, or a human indistinguishably.
///
/// **Only a deterministic Decider can be a solver's environment.** The solver searches the party against a *fixed*
/// foe policy it **plugs in** (never one it inspects from the live game); a weighted or human foe is a distribution
/// or a real opponent, not a scripted environment, and cannot be searched single-agent. [`Decider::deterministic`]
/// is that gate - the solver may only plug in a Decider that answers `true`.
pub trait Decider {
    /// The act this body commits this round, or `None` if this policy **defers to an interactive driver** - a
    /// human at a UI supplies the act out of band (a click), so the loop polls again next frame. An *autonomous*
    /// policy always returns `Some`. `&mut self` lets a stateful policy (a seeded RNG) advance; a pure one ignores
    /// it.
    fn commit(&mut self, board: &Board, body: usize) -> Option<Act>;

    /// Whether this policy is reproducible - same board, same act, no hidden state. A solver may plug in a Decider
    /// as its foe environment **only** when this is `true`.
    fn deterministic(&self) -> bool;
}

/// **Instinct** - the deterministic scripted policy. A body commits the single act its instinct dictates
/// ([`foe_act`]). This is the policy the [`Combat`] game plugs in for its foes, and the one a solver searches
/// against: pure, reproducible, one act per board.
pub struct Instinct;

impl Decider for Instinct {
    fn commit(&mut self, board: &Board, body: usize) -> Option<Act> {
        Some(foe_act(board, body).unwrap_or(Act::Hold))
    }
    fn deterministic(&self) -> bool {
        true
    }
}

/// **Human** - the policy that never decides autonomously: [`commit`](Decider::commit) always returns `None`, so
/// an interactive driver supplies the act (a click). Not deterministic from the code's view - a person is free to
/// pick anything, so a solver can never plug a human in as its environment.
pub struct Human;

impl Decider for Human {
    fn commit(&mut self, _board: &Board, _body: usize) -> Option<Act> {
        None
    }
    fn deterministic(&self) -> bool {
        false
    }
}

/// **Random** - a stochastic policy, standing in for the whole *weighted-random* class: it commits a uniformly
/// random legal act. A seeded xorshift drives it, so a given seed replays identically, yet the choice is **not** a
/// pure function of the board (it advances RNG state) - so [`deterministic`](Decider::deterministic) is `false`
/// and a solver may not plug it in. The abstraction is deliberately blind to the *distribution* (uniform here, or
/// any weighting): that is exactly what makes instinct, random, and human interchangeable behind [`Decider`].
pub struct Random {
    state: u64,
}

impl Random {
    pub fn new(seed: u64) -> Self {
        Random { state: seed | 1 } // a nonzero state - xorshift is stuck at 0
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

impl Decider for Random {
    fn commit(&mut self, board: &Board, body: usize) -> Option<Act> {
        let opts = legal_acts(board, body);
        if opts.is_empty() {
            return Some(Act::Hold);
        }
        Some(opts[(self.next() % opts.len() as u64) as usize].clone())
    }
    fn deterministic(&self) -> bool {
        false
    }
}

/// The regions combat as a [`Game`].
pub struct Combat;

impl Game for Combat {
    type State = State;
    type Choice = Choice;

    fn options(state: &State) -> Vec<Choice> {
        match state.order.get(state.next) {
            // A hero's real acts to choose among; a foe's single scripted act, produced by the deterministic
            // policy this game plugs in ([`Instinct`]) - so it flows through the same loop, but the driver has
            // nothing to decide and auto-advances it. Swapping that policy is the only thing that changes a foe's
            // behaviour; nothing downstream can tell which Decider committed the act.
            Some(&i) if state.board.units[i].side == Side::Party => legal_acts(&state.board, i)
                .into_iter()
                .map(Choice::Act)
                .collect(),
            Some(&i) => vec![Choice::Act(
                Instinct.commit(&state.board, i).unwrap_or(Act::Hold),
            )],
            None => Vec::new(),
        }
    }

    fn apply(state: &State, choice: &Choice) -> State {
        let mut s = state.clone();
        let Choice::Act(act) = choice;
        let unit = s.order[s.next];
        s.pending[unit] = Some(act.clone());
        match s.next_undeclared(s.next + 1) {
            Some(n) => s.next = n,
            None => resolve_round(&mut s),
        }
        s
    }

    fn outcome(state: &State) -> Option<Outcome> {
        match state.board.outcome() {
            Some(true) => Some(Outcome::Win),
            Some(false) => Some(Outcome::Loss),
            None if state.round > MAX_ROUNDS => Some(Outcome::Draw),
            None => None,
        }
    }
}

/// The whole round resolves as one transition from the acts **everyone** committed - heroes and foes alike are in
/// `pending` now, so there is nothing left to script here. Afterwards the cursor resets to the next round's first
/// decision. (A body that somehow reached resolution undeclared defaults to [`Act::Hold`].)
fn resolve_round(s: &mut State) {
    let acts: Vec<Act> = (0..s.board.units.len())
        .map(|i| s.pending[i].clone().unwrap_or(Act::Hold))
        .collect();
    play_round(&mut s.board, &acts);

    s.round += 1;
    s.pending = vec![None; s.board.units.len()];
    s.next = s.next_undeclared(0).unwrap_or(s.order.len());
}

/// **Play a whole fight out under the greedy heuristic on BOTH sides** - every body (hero and foe) commits
/// [`greedy_act`], no search. This is the "can you win WITHOUT thinking?" baseline: a party driven by the same
/// one-ply disruption read the foes use. Deterministic and cheap (one playout, bounded by the 5-round draw cap).
/// Paired with the solver's verdict, the gap between them is exactly where player insight earns a win the greedy
/// line cannot.
pub fn greedy_playout(mut state: State) -> Outcome {
    loop {
        if let Some(o) = Combat::outcome(&state) {
            return o;
        }
        let i = state
            .deciding()
            .expect("a non-terminal state has a deciding body");
        let act = greedy_act(state.board(), i).unwrap_or(Act::Hold);
        state = Combat::apply(&state, &Choice::Act(act));
    }
}

// ---- the best-route scorer -----------------------------------------------------------------------------

/// A winning route's cost, ranked **lexicographically** in the player's stated priority: you must **win**
/// (an unwinnable route has no `Score` at all), then fewest **heroes downed**, then fewest **rounds taken**,
/// then fewest **hero Health cards flipped**. The derived `Ord` compares the fields in declaration order - which
/// IS that priority - so `min` over routes picks the best one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    /// Party bodies fallen at the win (first priority: keep everyone standing).
    pub downed: u32,
    /// Rounds taken to reach the win (second: win sooner).
    pub rounds: u32,
    /// Total party Health cards flipped over the route, measured against the fight-start Vitality passed to
    /// [`Scorer::new`] (third: take the least damage).
    pub hp_lost: u32,
}

/// **The best-route scorer.** Finds the lexicographically-minimal [`Score`] over *all* winning lines from a
/// position - the party plays to minimize it, the scripted foes are the environment. Unlike the winnable oracle
/// (which stops at the first win it finds), this must weigh **every** winning line, so it is heavier. It is
/// budgeted and resumable the same way, and until the search is exhausted its answer is a **provisional upper
/// bound** (`<=`) - more search can only find a cheaper line, never a costlier one.
///
/// `hp_lost` is measured against a fixed reference (the party's fight-start Vitality), so a `Score` is a pure
/// function of the terminal position and the memo is reusable across moves, exactly like the winnable memo.
pub struct Scorer {
    memo: HashMap<Key, Option<Score>>,
    /// The reference Health for `hp_lost` - the party's full Vitality, index-aligned with `units`. Fixed for the
    /// whole fight, so `Score`s stay comparable across positions and the memo stays valid across moves.
    start_hp: Vec<u32>,
    nodes: u64,
    walk: u64,
    budget: u64,
    aborted: bool,
}

impl Scorer {
    /// `start_hp` is the party's full Vitality (index-aligned with the board's `units`); foes' entries are
    /// ignored. `budget` bounds one walk (see [`Scorer::grant`]).
    pub fn new(start_hp: Vec<u32>, budget: u64) -> Self {
        Scorer {
            memo: HashMap::new(),
            start_hp,
            nodes: 0,
            walk: 0,
            budget,
            aborted: false,
        }
    }

    pub fn nodes(&self) -> u64 {
        self.nodes
    }
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    /// Allow the next walk `nodes` positions and clear the abort flag - the resumable frame tick. The memo
    /// survives, so settled subtrees are free and each grant pushes the frontier deeper.
    pub fn grant(&mut self, nodes: u64) {
        self.walk = 0;
        self.budget = nodes;
        self.aborted = false;
    }

    /// The cost of a **won** terminal position.
    fn score_of(&self, state: &State) -> Score {
        let b = state.board();
        let (mut downed, mut hp_lost) = (0u32, 0u32);
        for (i, u) in b.units.iter().enumerate() {
            if u.side != Side::Party {
                continue;
            }
            if u.fallen {
                downed += 1;
            }
            let start = self.start_hp.get(i).copied().unwrap_or(u.health);
            hp_lost += start.saturating_sub(u.health);
        }
        Score {
            downed,
            // round() has already advanced past the deciding round when the win is seen, so subtract it back.
            rounds: (state.round() as u32).saturating_sub(1),
            hp_lost,
        }
    }

    /// **The best [`Score`] achievable from `state`**, or `None` if no winning line was found (either genuinely
    /// unwinnable, or the budget ran out - [`Scorer::aborted`] tells them apart). While `aborted`, a `Some` is a
    /// provisional upper bound; when a walk completes without aborting, it is exact.
    pub fn best(&mut self, state: &State) -> Option<Score> {
        match Combat::outcome(state) {
            Some(Outcome::Win) => return Some(self.score_of(state)),
            Some(_) => return None, // a loss or a draw is not a winning route
            None => {}
        }
        if self.walk >= self.budget {
            self.aborted = true;
            return None;
        }
        let key = key_of(state);
        if let Some(&v) = self.memo.get(&key) {
            return v;
        }
        self.nodes += 1;
        self.walk += 1;

        // Each node judges its OWN subtree's completeness (stash the caller's abort flag), so a sibling's give-up
        // is never mistaken for this node being fully explored - the same rule that keeps the winnable oracle honest.
        let outer = self.aborted;
        self.aborted = false;

        let mut best: Option<Score> = None;
        for opt in Combat::options(state) {
            let child = Combat::apply(state, &opt);
            if let Some(s) = self.best(&child) {
                best = Some(best.map_or(s, |b| b.min(s)));
            }
        }

        let incomplete = self.aborted;
        self.aborted = outer || incomplete;
        // Cache only a COMPLETE subtree: a budget-truncated search may have missed a cheaper (or the only) line.
        if !incomplete {
            self.memo.insert(key, best);
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::run;

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    /// The Game plays a whole fight to a verdict, driven by the generic runner - rounds of declarations, with the
    /// foe folded into apply and never appearing in the options. No setup: it opens on round 1.
    #[test]
    fn the_runner_plays_a_whole_fight() {
        let s = State::new(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [1, 2, 1, 1, 1], true, false),
        ]);
        // Always take the first option (Clash the foe each round).
        let out = run::<Combat>(s, |_, _| 0);
        assert_eq!(out, Outcome::Win, "the stronger body wins the fight");
    }

    /// **Score orders by the stated priority** - downed, then rounds, then hp_lost (the derived `Ord`).
    #[test]
    fn score_orders_lexicographically() {
        let clean_slow = Score {
            downed: 0,
            rounds: 5,
            hp_lost: 9,
        };
        let bloody_fast = Score {
            downed: 1,
            rounds: 1,
            hp_lost: 0,
        };
        assert!(
            clean_slow < bloody_fast,
            "keeping everyone standing beats a fast, costly win"
        );
        let fewer_rounds = Score {
            downed: 0,
            rounds: 2,
            hp_lost: 9,
        };
        let less_hp = Score {
            downed: 0,
            rounds: 3,
            hp_lost: 0,
        };
        assert!(
            fewer_rounds < less_hp,
            "same downed: fewer rounds wins even at a higher hp cost"
        );
    }

    /// **The scorer finds a clean win and ranks it.** The Raider one-shots a weakling: won, nobody downed.
    #[test]
    fn the_scorer_scores_a_clean_win() {
        let s = State::new(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [1, 2, 1, 1, 1], true, false),
        ]);
        let start_hp: Vec<u32> = s.board().units.iter().map(|u| u.health).collect();
        let mut sc = Scorer::new(start_hp, u64::MAX);
        let best = sc.best(&s).expect("the Raider can win");
        assert!(!sc.aborted(), "an unbudgeted search settles");
        assert_eq!(best.downed, 0, "the Raider is never downed");
        assert!(best.rounds >= 1, "a win takes at least one round");
    }

    /// **An unwinnable fight has no Score.** A naked Marksman cannot crack a giant Wall - the scorer returns
    /// `None` (proven, not a budget give-up), never a bogus best.
    #[test]
    fn the_scorer_returns_none_when_doomed() {
        let s = State::new(vec![
            unit("Marksman", Side::Party, [1, 1, 1, 1, 1], false, true),
            unit("The Wall", Side::Foe, [9, 9, 9, 3, 3], true, false),
        ]);
        let start_hp: Vec<u32> = s.board().units.iter().map(|u| u.health).collect();
        let mut sc = Scorer::new(start_hp, u64::MAX);
        assert_eq!(sc.best(&s), None, "no winning route exists");
        assert!(!sc.aborted(), "and the search proved it, not gave up");
    }

    /// **A fight opens on round 1 with no setup.** The party stands on region 0, the foes on region 1, posts
    /// weapon-derived - and the first decision is a hero *choosing an action*, not a placement.
    #[test]
    fn new_starts_at_round_one_with_one_region_per_side() {
        let s = State::new(vec![
            unit("Sword", Side::Party, [5, 4, 1, 2, 2], true, false),
            unit("Bow", Side::Party, [5, 4, 1, 2, 2], false, true),
            unit("Foe", Side::Foe, [4, 4, 1, 2, 2], true, false),
        ]);
        assert_eq!(s.round(), 1, "the fight begins at round 1 - no setup");
        assert_eq!(s.board().regions[0], 0, "the party stands on region 0");
        assert_eq!(s.board().regions[1], 0, "all party bodies share region 0");
        assert_eq!(s.board().regions[2], 1, "the foes stand on region 1");
        // Posts are weapon-derived, exactly as before - front for melee, back for ranged-only.
        assert_eq!(s.board().ranks[0], Rank::Vanguard, "a melee body is front");
        assert_eq!(
            s.board().ranks[1],
            Rank::Rearguard,
            "a ranged-only body is back"
        );
        // The first decision is an action, not a placement: every option is an Act.
        let opts = Combat::options(&s);
        assert!(!opts.is_empty(), "the first hero has acts to choose from");
        for o in &opts {
            assert!(matches!(o, Choice::Act(_)), "an action, never a placement");
        }
    }

    /// **The play loop is decider-agnostic** - a body's act comes from its [`Decider`], and the loop that walks a
    /// fight never learns which kind it asked. Playing every body by [`Instinct`] reproduces the scripted fight;
    /// swapping the party to a [`Random`] policy still drives a legal fight to a verdict through the *same* loop.
    /// This is the whole point of the abstraction: instinct, random, and human are interchangeable here.
    #[test]
    fn any_decider_drives_the_same_loop() {
        // The autonomous play loop: while the deciding body's Decider commits (Some), apply and advance. (A Human
        // would return None here and hand off to an interactive driver; this headless loop uses only autonomous
        // policies.)
        fn play(mut s: State, deciders: &mut [Box<dyn Decider>]) -> Outcome {
            loop {
                if let Some(o) = Combat::outcome(&s) {
                    return o;
                }
                let i = s
                    .deciding()
                    .expect("a non-terminal state has a deciding body");
                let act = deciders[i]
                    .commit(s.board(), i)
                    .expect("an autonomous decider always commits");
                s = Combat::apply(&s, &Choice::Act(act));
            }
        }
        let board = || {
            State::new(vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
                unit("Weakling", Side::Foe, [1, 2, 1, 1, 1], true, false),
            ])
        };
        // Instinct is deterministic: the same all-Instinct fight twice reaches the same verdict.
        let mut d1: [Box<dyn Decider>; 2] = [Box::new(Instinct), Box::new(Instinct)];
        let mut d2: [Box<dyn Decider>; 2] = [Box::new(Instinct), Box::new(Instinct)];
        assert_eq!(
            play(board(), &mut d1),
            play(board(), &mut d2),
            "a deterministic policy replays identically"
        );
        // A Random party against an Instinct foe drives the SAME loop to a legal verdict, blind to the policy
        // (bounded by the 5-round draw cap, so it always terminates).
        let mut mixed: [Box<dyn Decider>; 2] = [Box::new(Random::new(1)), Box::new(Instinct)];
        let out = play(board(), &mut mixed);
        assert!(matches!(out, Outcome::Win | Outcome::Loss | Outcome::Draw));
    }

    /// Options are only ever the PARTY's - the foe is never offered a choice, because it is scripted inside
    /// apply.
    #[test]
    fn the_foe_never_appears_in_the_options() {
        let s = State::new(vec![
            unit("Hero", Side::Party, [5, 4, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [4, 4, 1, 2, 2], true, false),
        ]);
        // The fight opens on round 1's first declaration - the hero's, never the foe's.
        let opts = Combat::options(&s);
        assert!(!opts.is_empty(), "the hero has acts to choose from");
        // Every option is an Act declared by the hero; nothing here is a foe decision.
        for o in &opts {
            assert!(matches!(o, Choice::Act(_)));
        }
    }
}

// ---- searchability: the canonical key, and the "no slip" control ---------------------------------------

use super::regions::canonical;
use crate::core::Solvable;

/// A hashable digest of a position: per-unit `(health, fallen, rank)`, the **canonicalized** regions
/// (so a relabelling is not a distinct position), the round, and the pending declarations + declare cursor (a
/// half-declared round is genuinely a different state than a fresh one). The [`Rank`] carries the outrider state,
/// so two positions that differ only by who is loose inside the enemy ranks stay distinct.
///
/// `tempo` and the damage pile are absent on purpose: both are re-derived at the round Reset, so they are only
/// ever meaningful *inside* [`play_round`], never at a state a search actually visits.
type Key = (Vec<(u32, bool, Rank)>, Vec<u8>, usize, u8, Vec<Option<Act>>);

fn key_of(s: &State) -> Key {
    let per: Vec<(u32, bool, Rank)> = s
        .board
        .units
        .iter()
        .enumerate()
        .map(|(i, u)| (u.health, u.fallen, s.board.ranks[i]))
        .collect();
    (
        per,
        canonical(&s.board.regions),
        s.round,
        s.next as u8,
        s.pending.clone(),
    )
}

impl Solvable for Combat {
    type Key = Key;
    fn key(state: &State) -> Key {
        key_of(state)
    }
}

/// The **clash-only control**: the same game, but the party may never slip (no raid, no retreat, no regroup) -
/// it may only [`Act::Clash`] or [`Act::Hold`]. Wrapping `Combat` this way is the whole point of the generic
/// seam: a control is a five-line newtype that filters `options`, not a second copy of the rules.
///
/// It answers the experiment's question - *is slipping ever necessary?* - by search: if `Combat` is winnable
/// from a formation and `ClashOnly` is not, a slip was load-bearing there.
pub struct ClashOnly;

impl Game for ClashOnly {
    type State = State;
    type Choice = Choice;
    fn options(state: &State) -> Vec<Choice> {
        // Restrict the PARTY only. Now that foes declare through the same loop, a foe can reach the cursor with a
        // single scripted move that happens to be a raid - stripping it would strand the round with nothing to
        // declare. The control is about what the *party* may do, so it leaves the foes' one move alone.
        let restrict = state
            .deciding()
            .is_some_and(|i| state.board().units[i].side == Side::Party);
        Combat::options(state)
            .into_iter()
            .filter(|c| !restrict || !matches!(c, Choice::Act(Act::Cross(..))))
            .collect()
    }
    fn apply(state: &State, choice: &Choice) -> State {
        Combat::apply(state, choice)
    }
    fn outcome(state: &State) -> Option<Outcome> {
        Combat::outcome(state)
    }
}

impl Solvable for ClashOnly {
    type Key = Key;
    fn key(state: &State) -> Key {
        key_of(state)
    }
}

#[cfg(test)]
mod solve_tests {
    use super::*;
    use crate::core::{Solver, Verdict};

    fn u(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    fn settle<G: crate::core::Solvable>(s: &G::State) -> Verdict {
        let mut o = Solver::<G>::new();
        let mut grant = 1u64;
        loop {
            o.grant(grant);
            let v = o.verdict(s);
            if v != Verdict::Evaluating {
                return v;
            }
            grant = grant.saturating_mul(2);
        }
    }

    /// The generic solver, walking the generic Game, reaches the obvious verdict: a strong body beats a weak
    /// one. This is the whole point - the SAME `Solver` that would search any game searches combat, because the
    /// rules are behind the trait.
    #[test]
    fn the_generic_solver_calls_a_won_fight_winnable() {
        let s = State::new(vec![
            u("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            u("Weakling", Side::Foe, [1, 2, 1, 1, 1], true, false),
        ]);
        assert_eq!(settle::<Combat>(&s), Verdict::Winnable);
    }

    /// A hopeless fight is Doomed - and the solver proves it (exhausts the tree) rather than merely running out.
    #[test]
    fn a_hopeless_fight_is_doomed() {
        let s = State::new(vec![
            u("Gnat", Side::Party, [1, 1, 1, 1, 1], false, true),
            u("Titan", Side::Foe, [9, 9, 9, 3, 3], true, false),
        ]);
        assert_eq!(settle::<Combat>(&s), Verdict::Doomed);
    }

    /// **The clash-only control genuinely removes slipping** - it is a real restriction, not a no-op. This is
    /// what makes the control a control: the mechanism is tested here; the *balance* question it answers (is a
    /// raid ever necessary against a real encounter?) is a content property, proved with the catalog rather than
    /// a hand-authored board, because it depends on the scripted foe actually holding its formation.
    #[test]
    fn clash_only_never_offers_a_slip() {
        // A board with a screened back, so `Combat` WOULD offer a raid. The fight opens on round 1's first
        // declaration - the Raider's - with the foes' Wall (front) screening their Cannon (back) in region 1.
        let s = State::new(vec![
            u("Raider", Side::Party, [7, 6, 1, 3, 2], true, false),
            u("Wall", Side::Foe, [1, 6, 4, 1, 2], true, false),
            u("Cannon", Side::Foe, [4, 2, 1, 2, 2], false, true),
        ]);
        let full = Combat::options(&s);
        assert!(
            full.iter()
                .any(|c| matches!(c, Choice::Act(Act::Cross(Some(_), _, _)))),
            "the full game offers a raid (a cross with a target) at the screened cannon"
        );
        let control = ClashOnly::options(&s);
        assert!(
            control
                .iter()
                .all(|c| !matches!(c, Choice::Act(Act::Cross(..)))),
            "but the clash-only control offers no crossing of any kind"
        );
        assert!(
            !control.is_empty(),
            "and it is not empty - clashing and holding remain"
        );
    }
}
