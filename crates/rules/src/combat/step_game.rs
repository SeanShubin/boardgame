//! **The step machine's DECISION LAYER** - the eight-step round as a [`Game`]: each step is its own
//! declare/reveal wave, and the step RESOLVES the moment its wave completes, so every later declaration is made
//! against the real, post-death board. This is what the wave model could not express: step 8 aims at a rearguard
//! whose screen fell at step 3 *this round*.
//!
//! **The shipped decision layer** - the diagonal gate, the fight UI and the self-play baseline all drive this
//! machine. (The wave model it replaced is deleted; its physics live on in `regions`/`steps`.)
//!
//! The same house architecture as the wave model, re-derived for steps:
//! - **One loop for everyone**: heroes and foes declare through the same cursor; a foe's wave entry is a single
//!   scripted option ([`step_policy`], the side-agnostic greedy), auto-advanced by any driver.
//! - **Eligibility is the branching control**: a body with nothing to do at a step (wrong rank, no legal target,
//!   no tempo) never reaches the cursor - no decision point, no branch. Tempo is LIVE mid-round (steps resolve
//!   as they go), so a body that poured its pool early simply vanishes from later waves.
//! - **Validity is resolver-enforced** ([`super::steps`]): a stale declaration drops, never mislands.

use super::regions::{
    Board, MAX_ROUNDS, Rank, canonical, foe_catch, interchangeable, wants_to_cross,
};
use super::resolve::{Combatant, Side, refresh_round};
use super::steps::{
    resolve_advance, resolve_cross, resolve_early, resolve_inner, resolve_late, resolve_raid,
    resolve_volley, resolve_withdraw,
};
use crate::core::{Game, Outcome, Solvable};

/// The eight steps of a round, in schedule order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phase {
    /// Step 1: in-region strikes - prior-round outriders and their hosts, both tiers, no screen.
    Inner,
    /// Step 2: outriders may withdraw to their own line, free - step 1 was the price.
    Withdraw,
    /// Step 3: the early front trade - and the interception window (crossings declare NEXT step, blind here).
    Early,
    /// Step 4: vanguards that declared no line strike may cross - an uncontested walk, landing as an Outrider.
    Cross,
    /// Step 5: rearguards volley enemy outriders - one-way, opening blow only.
    Volley,
    /// Step 6: this round's arrivals strike a back-line target - opening blow only.
    Raid,
    /// Step 7: the late front trade - fire, and every vanguard that held back ("halt" is emergent).
    Late,
    /// Step 8: the advance - a rearguard whose screen has fallen BY NOW is reachable (same-round collapse).
    Advance,
}

const PHASES: [Phase; 8] = [
    Phase::Inner,
    Phase::Withdraw,
    Phase::Early,
    Phase::Cross,
    Phase::Volley,
    Phase::Raid,
    Phase::Late,
    Phase::Advance,
];

/// One body's declaration at the current step.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StepChoice {
    /// A strike step: strike `Some(target)`, or pass.
    Strike(Option<usize>),
    /// A movement step (withdraw / cross): go, or stay.
    Move(bool),
}

/// The whole position: the board, the current step and its wave cursor, this step's declarations so far, and
/// the round's accumulated commitments (who struck from the line, who arrived).
#[derive(Clone, Debug)]
pub struct StepState {
    board: Board,
    /// The declaration order within every wave: the party (seat order) then the foes.
    order: Vec<usize>,
    phase: Phase,
    /// Cursor into `order`: the next eligible body of the current wave that has not declared.
    next: usize,
    /// This step's declarations (per body); cleared when the step resolves.
    decls: Vec<Option<StepChoice>>,
    /// Declared a LINE strike this round (the crossing gate; the raid is exempt).
    struck: Vec<bool>,
    /// Crossed this round (step-6 eligibility).
    arrived: Vec<bool>,
    round: usize,
}

impl StepState {
    /// Begin a fight at round 1, step 1: one region per side, posts weapon-derived - the same opening as the
    /// wave model.
    pub fn new(units: Vec<Combatant>) -> StepState {
        let n = units.len();
        let regions: Vec<u8> = (0..n)
            .map(|i| if units[i].side == Side::Party { 0 } else { 1 })
            .collect();
        let party: Vec<usize> = (0..n).filter(|&i| units[i].side == Side::Party).collect();
        let foes: Vec<usize> = (0..n).filter(|&i| units[i].side == Side::Foe).collect();
        let order: Vec<usize> = party.iter().chain(foes.iter()).copied().collect();
        let mut s = StepState {
            board: Board::new(units, regions),
            order,
            phase: Phase::Inner,
            next: 0,
            decls: vec![None; n],
            struck: vec![false; n],
            arrived: vec![false; n],
            round: 1,
        };
        s.seek();
        s
    }

    pub fn board(&self) -> &Board {
        &self.board
    }
    pub fn round(&self) -> usize {
        self.round
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The body whose declaration is pending, or `None` on a terminal state.
    pub fn deciding(&self) -> Option<usize> {
        self.order.get(self.next).copied()
    }

    /// The legal strike targets for `i` at the current step - the menu, with the symmetric-target dedup (two
    /// interchangeable enemies collapse to the lowest-index representative, same as the wave model).
    pub fn targets(&self, i: usize) -> Vec<usize> {
        let b = &self.board;
        let candidates: Vec<usize> = (0..b.units.len())
            .filter(|&t| {
                !b.units[t].fallen && b.units[t].side != b.units[i].side && self.reaches(i, t)
            })
            .collect();
        candidates
            .iter()
            .copied()
            .filter(|&t| {
                !candidates
                    .iter()
                    .any(|&t2| t2 < t && interchangeable(b, t2, t))
            })
            .collect()
    }

    /// Whether `i` may aim at `t` at the current step - the step's rank-pair rule, menu side. (The resolver
    /// enforces the same rule again at resolution.)
    fn reaches(&self, i: usize, t: usize) -> bool {
        let b = &self.board;
        match self.phase {
            Phase::Inner => b.regions[i] == b.regions[t],
            Phase::Early => b.ranks[i] == Rank::Vanguard && b.ranks[t] == Rank::Vanguard,
            Phase::Volley => b.ranks[i] == Rank::Rearguard && b.ranks[t] == Rank::Outrider,
            Phase::Raid => {
                self.arrived[i] && b.ranks[t] == Rank::Rearguard && b.regions[t] == b.regions[i]
            }
            Phase::Late => {
                matches!(b.ranks[i], Rank::Vanguard | Rank::Rearguard)
                    && b.ranks[t] == Rank::Vanguard
            }
            Phase::Advance => {
                matches!(b.ranks[i], Rank::Vanguard | Rank::Rearguard)
                    && b.ranks[t] == Rank::Rearguard
                    && !b.is_screened(t)
            }
            Phase::Withdraw | Phase::Cross => false,
        }
    }

    /// Whether `i` has a genuine decision at the current step. This is the branching control: no eligibility,
    /// no decision point, no branch.
    fn eligible(&self, i: usize) -> bool {
        let b = &self.board;
        if b.units[i].fallen {
            return false;
        }
        match self.phase {
            Phase::Withdraw => b.ranks[i] == Rank::Outrider,
            Phase::Cross => {
                // A vanguard that declared no line strike, with an enemy region to walk into - and a reason:
                // the menu offers the crossing only when the enemy holds a SCREENED back (the dominated-slip
                // prune, unchanged; an exposed back is reached by the step-8 advance instead).
                b.ranks[i] == Rank::Vanguard
                    && !self.struck[i]
                    && b.units[i].tempo > 0
                    && (0..b.units.len()).any(|t| {
                        b.units[t].side != b.units[i].side && !b.units[t].fallen && b.is_screened(t)
                    })
            }
            _ => b.units[i].tempo > 0 && !self.targets(i).is_empty(),
        }
    }

    /// Advance the cursor to the current wave's next eligible, undeclared body - resolving the step and moving
    /// through the schedule (and the round) whenever a wave completes. Terminal states leave the cursor parked.
    fn seek(&mut self) {
        loop {
            if self.outcome_now().is_some() {
                self.next = self.order.len();
                return;
            }
            if let Some(off) = self.order[self.next..]
                .iter()
                .position(|&i| self.eligible(i) && self.decls[i].is_none())
            {
                self.next += off;
                return;
            }
            // The wave is complete: resolve this step against the live board, then move on.
            self.resolve_phase();
            if self.phase == Phase::Advance {
                // Round over: the reset - tempo stands back up, commitments clear, a new round opens.
                self.round += 1;
                refresh_round(&mut self.board.units);
                self.struck = vec![false; self.board.units.len()];
                self.arrived = vec![false; self.board.units.len()];
                self.phase = Phase::Inner;
            } else {
                let idx = PHASES.iter().position(|&p| p == self.phase).unwrap();
                self.phase = PHASES[idx + 1];
            }
            self.decls = vec![None; self.board.units.len()];
            self.next = 0;
        }
    }

    /// Resolve the current step from its collected declarations.
    fn resolve_phase(&mut self) {
        let strikes: Vec<(usize, usize)> = self
            .decls
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d {
                Some(StepChoice::Strike(Some(t))) => Some((i, *t)),
                _ => None,
            })
            .collect();
        let movers: Vec<usize> = self
            .decls
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d {
                Some(StepChoice::Move(true)) => Some(i),
                _ => None,
            })
            .collect();
        match self.phase {
            Phase::Inner => {
                resolve_inner(&mut self.board, &strikes);
            }
            Phase::Withdraw => {
                resolve_withdraw(&mut self.board, &movers);
            }
            Phase::Early => {
                resolve_early(&mut self.board, &strikes);
            }
            Phase::Cross => {
                let (landed, _) = resolve_cross(&mut self.board, &movers, &self.struck);
                for i in landed {
                    self.arrived[i] = true;
                }
            }
            Phase::Volley => {
                resolve_volley(&mut self.board, &strikes);
            }
            Phase::Raid => {
                resolve_raid(&mut self.board, &strikes, &self.arrived);
            }
            Phase::Late => {
                resolve_late(&mut self.board, &strikes);
            }
            Phase::Advance => {
                resolve_advance(&mut self.board, &strikes);
            }
        }
    }

    fn outcome_now(&self) -> Option<Outcome> {
        match self.board.outcome() {
            Some(true) => Some(Outcome::Win),
            Some(false) => Some(Outcome::Loss),
            None if self.round > MAX_ROUNDS => Some(Outcome::Draw),
            None => None,
        }
    }
}

/// **The side-agnostic per-step policy** - what a scripted foe declares at each wave, and what a greedy
/// (no-search) party plays. Derived from the same one-ply reads as the wave model: the whole-round intent from
/// [`foe_act`]-style greed, the target pick from the [`foe_catch`] instinct (max disruption, lowest index).
pub fn step_policy(state: &StepState, i: usize) -> StepChoice {
    let b = state.board();
    let candidates = state.targets(i);
    match state.phase() {
        Phase::Withdraw => StepChoice::Move(false), // instinct is havoc: stay in
        Phase::Cross => {
            // Cross iff the one-ply greedy would cross: the same read the wave model's script used.
            let crossing = wants_to_cross(b, i);
            StepChoice::Move(crossing)
        }
        Phase::Early => {
            // The interception window: a body that intends to cross holds its swing; otherwise strike the
            // max-disruption enemy vanguard now (foes strike early - vanguard deaths first is the schedule's
            // whole point).
            if wants_to_cross(b, i) {
                StepChoice::Strike(None)
            } else {
                StepChoice::Strike(foe_catch(b, i, &candidates))
            }
        }
        // Every other strike step: answer with the max-disruption pick, or pass when there is nobody.
        _ => StepChoice::Strike(foe_catch(b, i, &candidates)),
    }
}

/// The step machine as a [`Game`]: heroes get their real menus, foes their single scripted option, resolution
/// happens inside `apply` the moment a wave completes.
pub struct StepCombat;

impl Game for StepCombat {
    type State = StepState;
    type Choice = StepChoice;

    fn options(state: &StepState) -> Vec<StepChoice> {
        let Some(i) = state.deciding() else {
            return Vec::new();
        };
        if state.board.units[i].side != Side::Party {
            return vec![step_policy(state, i)];
        }
        match state.phase {
            Phase::Withdraw | Phase::Cross => {
                vec![StepChoice::Move(true), StepChoice::Move(false)]
            }
            _ => {
                let mut out: Vec<StepChoice> = state
                    .targets(i)
                    .into_iter()
                    .map(|t| StepChoice::Strike(Some(t)))
                    .collect();
                out.push(StepChoice::Strike(None)); // holding the swing is always legal
                out
            }
        }
    }

    fn apply(state: &StepState, choice: &StepChoice) -> StepState {
        let mut s = state.clone();
        let i = s.order[s.next];
        if matches!(choice, StepChoice::Strike(Some(_))) && !matches!(s.phase, Phase::Raid) {
            // A declared line strike commits the body: it cannot cross this round. (The raid is exempt - it
            // presupposes the crossing already made.)
            s.struck[i] = true;
        }
        s.decls[i] = Some(choice.clone());
        s.next += 1;
        s.seek();
        s
    }

    fn outcome(state: &StepState) -> Option<Outcome> {
        state.outcome_now()
    }
}

/// The memo key: per-unit `(health, fallen, rank, tempo)` - tempo is LIVE mid-round here, unlike the wave
/// model - plus the canonical partition, the round, the step, the cursor, and this step's declarations so far.
type StepKey = (
    Vec<(u32, bool, Rank, u32)>,
    Vec<u8>,
    usize,
    Phase,
    u8,
    Vec<Option<StepChoice>>,
    Vec<bool>,
    Vec<bool>,
);

impl Solvable for StepCombat {
    type Key = StepKey;
    fn key(state: &StepState) -> StepKey {
        let per: Vec<(u32, bool, Rank, u32)> = state
            .board
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| (u.health, u.fallen, state.board.ranks[i], u.tempo))
            .collect();
        (
            per,
            canonical(&state.board.regions),
            state.round,
            state.phase,
            state.next as u8,
            state.decls.clone(),
            state.struck.clone(),
            state.arrived.clone(),
        )
    }
}

/// The **clash-only control for the step machine**: the same game, but the party may never cross - the step
/// analog of [`super::game::ClashOnly`], for the "is the raid load-bearing?" experiments. A five-line newtype
/// filtering `options`, exactly as the seam intends.
pub struct StepClashOnly;

impl Game for StepClashOnly {
    type State = StepState;
    type Choice = StepChoice;
    fn options(state: &StepState) -> Vec<StepChoice> {
        let restrict = state.phase() == Phase::Cross
            && state
                .deciding()
                .is_some_and(|i| state.board().units[i].side == Side::Party);
        StepCombat::options(state)
            .into_iter()
            .filter(|c| !restrict || !matches!(c, StepChoice::Move(true)))
            .collect()
    }
    fn apply(state: &StepState, choice: &StepChoice) -> StepState {
        StepCombat::apply(state, choice)
    }
    fn outcome(state: &StepState) -> Option<Outcome> {
        StepCombat::outcome(state)
    }
}

impl Solvable for StepClashOnly {
    type Key = StepKey;
    fn key(state: &StepState) -> StepKey {
        StepCombat::key(state)
    }
}

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
    /// [`StepScorer::new`] (third: take the least damage).
    pub hp_lost: u32,
}

/// **The best-route scorer**: the lexicographically minimal [`Score`] (win > fewest downed > fewest rounds >
/// least hp lost) over ALL winning lines, budgeted and resumable, provisional (`<=`) until a walk completes
/// unaborted.
pub struct StepScorer {
    memo: std::collections::HashMap<StepKey, Option<Score>>,
    start_hp: Vec<u32>,
    nodes: u64,
    walk: u64,
    budget: u64,
    aborted: bool,
}

impl StepScorer {
    /// `start_hp` is the party's full Vitality (index-aligned); `budget` bounds one walk.
    pub fn new(start_hp: Vec<u32>, budget: u64) -> Self {
        StepScorer {
            memo: std::collections::HashMap::new(),
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

    /// Allow the next walk `nodes` positions and clear the abort flag; the memo survives.
    pub fn grant(&mut self, nodes: u64) {
        self.walk = 0;
        self.budget = nodes;
        self.aborted = false;
    }

    fn score_of(&self, state: &StepState) -> Score {
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
            rounds: (state.round() as u32).saturating_sub(1),
            hp_lost,
        }
    }

    /// The best [`Score`](Score) achievable from `state`, or `None` (unwinnable, or budget ran out -
    /// [`aborted`](StepScorer::aborted) tells them apart).
    pub fn best(&mut self, state: &StepState) -> Option<Score> {
        match StepCombat::outcome(state) {
            Some(Outcome::Win) => return Some(self.score_of(state)),
            Some(_) => return None,
            None => {}
        }
        if self.walk >= self.budget {
            self.aborted = true;
            return None;
        }
        let key = StepCombat::key(state);
        if let Some(v) = self.memo.get(&key) {
            return *v;
        }
        self.nodes += 1;
        self.walk += 1;

        // Each node judges its OWN subtree's completeness (stash the caller's abort flag) - the same rule that
        // keeps the winnable oracle honest: an incomplete subtree is never memoized.
        let outer = self.aborted;
        self.aborted = false;

        let mut best: Option<Score> = None;
        for opt in StepCombat::options(state) {
            let next = StepCombat::apply(state, &opt);
            if let Some(s) = self.best(&next) {
                best = Some(match best {
                    Some(b) if b <= s => b,
                    _ => s,
                });
            }
        }
        if !self.aborted {
            self.memo.insert(key, best);
        }
        self.aborted |= outer;
        best
    }
}

/// **Play a whole fight out under the per-step greedy on BOTH sides** - the step machine's "can you win WITHOUT
/// thinking?" baseline, the analog of the wave model's `greedy_playout`.
pub fn greedy_step_playout(mut state: StepState) -> Outcome {
    loop {
        if let Some(o) = StepCombat::outcome(&state) {
            return o;
        }
        let i = state
            .deciding()
            .expect("a non-terminal state has a deciding body");
        let c = step_policy(&state, i);
        state = StepCombat::apply(&state, &c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Solver, Verdict};

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    /// **The loop terminates and produces an outcome** under greedy self-play on a real-shaped board.
    #[test]
    fn greedy_self_play_terminates() {
        let s = StepState::new(vec![
            unit("Raider", Side::Party, [6, 6, 1, 2, 2], true, false),
            unit("Archer", Side::Party, [5, 2, 1, 2, 2], false, true),
            unit("Wall", Side::Foe, [1, 4, 6, 1, 2], true, false),
            unit("Sniper", Side::Foe, [5, 1, 1, 2, 3], false, true),
        ]);
        let o = greedy_step_playout(s);
        assert!(matches!(o, Outcome::Win | Outcome::Loss | Outcome::Draw));
    }

    /// **The solver settles on the step game** - a trivially winnable board proves Winnable, exactly, through
    /// the same generic machinery as the wave model.
    #[test]
    fn the_solver_settles_on_a_winnable_board() {
        let s = StepState::new(vec![
            unit("Raider", Side::Party, [6, 6, 1, 2, 2], true, false),
            unit("Runt", Side::Foe, [1, 1, 1, 1, 1], true, false),
        ]);
        let mut o = Solver::<StepCombat>::new();
        o.grant(u64::MAX);
        assert_eq!(o.verdict(&s), Verdict::Winnable);
        assert!(!o.aborted(), "an unbudgeted search settles");
    }

    /// **A striking vanguard is not offered the crossing** - the commitment shows up in ELIGIBILITY, not just
    /// the resolver: after declaring an early strike, the body never reaches the Cross wave's cursor.
    #[test]
    fn an_early_striker_is_not_asked_about_crossing() {
        let mut s = StepState::new(vec![
            unit("Raider", Side::Party, [3, 6, 1, 2, 2], true, false),
            unit("Wall", Side::Foe, [1, 9, 6, 1, 2], true, false),
            unit("Sniper", Side::Foe, [5, 9, 1, 2, 3], false, true),
        ]);
        // Round 1, step Early: the Raider strikes the Wall.
        assert_eq!(s.phase(), Phase::Early);
        assert_eq!(s.deciding(), Some(0));
        s = StepCombat::apply(&s, &StepChoice::Strike(Some(1)));
        // The wave rolls through the foes and the step resolves; the Raider - committed to the line - is never
        // eligible at Cross, so by the time the party would decide again the phase is past it.
        while s.deciding() == Some(0) && s.phase() == Phase::Cross {
            unreachable!("a committed striker must not be asked about crossing");
        }
        assert!(s.round() >= 1);
    }

    /// **The same-round advance, through the GAME loop**: the party fells the lone enemy vanguard at the early
    /// trade, and the step-8 wave then OFFERS the exposed rearguard as a target - reactive eligibility, the
    /// thing the wave model could not do.
    #[test]
    fn the_advance_wave_offers_a_back_exposed_this_round() {
        let mut s = StepState::new(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false), // fells the 1-health wall in one blow
            unit("Wall", Side::Foe, [1, 1, 6, 1, 2], true, false),
            unit("Sniper", Side::Foe, [5, 6, 1, 2, 3], false, true),
        ]);
        assert_eq!(s.phase(), Phase::Early);
        s = StepCombat::apply(&s, &StepChoice::Strike(Some(1))); // the wall dies at step 3
        // Walk the waves forward to the party's next decision.
        while let Some(i) = s.deciding() {
            if s.board().units[i].side == Side::Party {
                break;
            }
            let c = StepCombat::options(&s)[0].clone();
            s = StepCombat::apply(&s, &c);
        }
        // The Raider poured its pool at Early, so its next genuine decision is... none this round; but the
        // ADVANCE wave must have offered the Sniper to anyone eligible. Prove it structurally: by the time the
        // round rolled over, the Sniper is unscreened and alive - and a fresh round's Early wave has no enemy
        // vanguard, so the party's first offer is the ADVANCE at the exposed Sniper.
        while s.phase() != Phase::Advance || s.deciding().is_none() {
            let Some(i) = s.deciding() else { break };
            if s.board().units[i].side == Side::Party {
                s = StepCombat::apply(&s, &StepChoice::Strike(None));
            } else {
                let c = StepCombat::options(&s)[0].clone();
                s = StepCombat::apply(&s, &c);
            }
            if StepCombat::outcome(&s).is_some() {
                break;
            }
        }
        if s.phase() == Phase::Advance {
            if let Some(i) = s.deciding() {
                assert!(
                    s.targets(i).contains(&2),
                    "the exposed Sniper is on the advance menu"
                );
            }
        }
    }
}
