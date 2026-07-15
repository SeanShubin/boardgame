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
//! option** - the act its instinct dictates ([`foe_act`]) - so a foe "declares" too, but the driver has nothing
//! to pick and auto-advances it. When the last body declares, [`apply`](Game::apply) resolves the whole round
//! ([`play_round`]) from the acts everyone committed.
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

use super::regions::{Act, Board, MAX_ROUNDS, Post, foe_act, legal_acts, play_round};
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

/// The regions combat as a [`Game`].
pub struct Combat;

impl Game for Combat {
    type State = State;
    type Choice = Choice;

    fn options(state: &State) -> Vec<Choice> {
        match state.order.get(state.next) {
            // A hero's real acts to choose among; a foe's single scripted act (its instinct's pick), so it
            // flows through the same loop but the driver has nothing to decide and auto-advances it.
            Some(&i) if state.board.units[i].side == Side::Party => legal_acts(&state.board, i)
                .into_iter()
                .map(Choice::Act)
                .collect(),
            Some(&i) => vec![Choice::Act(foe_act(&state.board, i).unwrap_or(Act::Hold))],
            None => Vec::new(),
        }
    }

    fn apply(state: &State, choice: &Choice) -> State {
        let mut s = state.clone();
        let Choice::Act(act) = choice;
        let unit = s.order[s.next];
        s.pending[unit] = Some(*act);
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
        .map(|i| s.pending[i].unwrap_or(Act::Hold))
        .collect();
    play_round(&mut s.board, &acts);

    s.round += 1;
    s.pending = vec![None; s.board.units.len()];
    s.next = s.next_undeclared(0).unwrap_or(s.order.len());
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
        assert_eq!(s.board().posts[0], Post::Front, "a melee body is front");
        assert_eq!(s.board().posts[1], Post::Back, "a ranged-only body is back");
        // The first decision is an action, not a placement: every option is an Act.
        let opts = Combat::options(&s);
        assert!(!opts.is_empty(), "the first hero has acts to choose from");
        for o in &opts {
            assert!(matches!(o, Choice::Act(_)), "an action, never a placement");
        }
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

/// A hashable digest of a position: per-unit `(health, fallen, post, intruder)`, the **canonicalized** regions
/// (so a relabelling is not a distinct position), the round, and the pending declarations + declare cursor (a
/// half-declared round is genuinely a different state than a fresh one). The intruder flag is in the key so two
/// positions that differ only by who is loose inside the enemy ranks stay distinct.
///
/// `tempo` and the damage pile are absent on purpose: both are re-derived at the round Reset, so they are only
/// ever meaningful *inside* [`play_round`], never at a state a search actually visits.
type Key = (
    Vec<(u32, bool, Post, bool)>,
    Vec<u8>,
    usize,
    u8,
    Vec<Option<Act>>,
);

fn key_of(s: &State) -> Key {
    let per: Vec<(u32, bool, Post, bool)> = s
        .board
        .units
        .iter()
        .enumerate()
        .map(|(i, u)| (u.health, u.fallen, s.board.posts[i], s.board.intruders[i]))
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
            .filter(|c| {
                !restrict || !matches!(c, Choice::Act(Act::Raid(..)) | Choice::Act(Act::Slip(..)))
            })
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
            full.iter().any(|c| matches!(c, Choice::Act(Act::Raid(..)))),
            "the full game offers a raid at the screened cannon"
        );
        let control = ClashOnly::options(&s);
        assert!(
            control
                .iter()
                .all(|c| !matches!(c, Choice::Act(Act::Raid(..)) | Choice::Act(Act::Slip(..)))),
            "but the clash-only control offers no slip of any kind"
        );
        assert!(
            !control.is_empty(),
            "and it is not empty - clashing and holding remain"
        );
    }
}
