//! **The regions combat, as a [`Game`].** The state machine a runner, a solver, or an explorer drives - the
//! same rules as [`super::regions`], re-expressed through [`options`](Game::options) / [`apply`](Game::apply) /
//! [`outcome`](Game::outcome) so every consumer walks one machine.
//!
//! A whole fight is a walk through these phases:
//!
//! 1. **Setup** (round 1, one choice per party hero, in seat order): each hero picks a **region** to stand in
//!    and a **post** (front or back). This is the secret formation. It is a sequence of ordinary choices, so a
//!    solver searches formations by walking the tree - and shares one memo across all of them, where an
//!    external formation loop would not.
//! 2. **Declare** (each round, one choice per living party hero): each hero picks its [`Act`]. The **foe folds
//!    in here** - when the last hero declares, [`apply`] scripts the foes ([`foe_acts`]) and resolves the whole
//!    round ([`play_round`]) as a single deterministic transition. So `options` only ever offers the party's
//!    decisions: this is a single-agent reachability machine, not a two-sided one.
//!
//! Resolution is *inside* `apply`, not a set of choices, because within a round nothing is a player decision -
//! the schedule is fixed and the slip answers are part of each [`Act`]. (In perfect-information PvE a slip's
//! answer declared up front is equivalent to one chosen on reveal, since the party already knows what the
//! scripted foes will commit - so folding it into the declaration loses nothing a solver could use.)

use super::regions::{Act, Board, MAX_ROUNDS, Post, foe_acts, legal_acts, play_round};
use super::resolve::{Combatant, Side};
use crate::core::{Game, Outcome};

/// A choice in the combat game: place a hero at setup, or declare its act in a round.
#[derive(Clone, Debug, PartialEq)]
pub enum Choice {
    /// Setup: stand this hero in `region`, posted `post`.
    Place { region: u8, post: Post },
    /// A round: this hero does `act`.
    Act(Act),
}

/// What decision is pending. The cursor is always a **party** unit index (the foes never choose).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Placing the party, one hero at a time, in seat order. `next` is the next unplaced party unit.
    Setup { next: usize },
    /// A round of combat. `next` is the next living party unit that has not yet declared.
    Declare { next: usize },
}

/// The whole position: the bodies, their formation-so-far, whose decision is pending, and the round.
#[derive(Clone, Debug)]
pub struct State {
    board: Board,
    /// Party unit indices, in the seat order they are placed and declare.
    party: Vec<usize>,
    /// The foes, pre-formed into their own region once the party finishes placing.
    foes: Vec<usize>,
    phase: Phase,
    /// Each unit's declared act this round (`None` until declared); reset each round.
    pending: Vec<Option<Act>>,
    round: usize,
    /// Set once, from the *scripted* foe formation, so it can be dropped in when setup ends.
    foe_posts: Vec<Post>,
}

impl State {
    /// Begin a fight: the party unplaced, the foes' formation fixed by script (shoot-only bodies at the back).
    ///
    /// The `Board` starts with every unit parked in region 0 as a placeholder; setup overwrites the party's
    /// region/post one hero at a time, and the foes' real placement drops in when the last hero is placed.
    pub fn new(units: Vec<Combatant>) -> State {
        let n = units.len();
        let party: Vec<usize> = (0..n).filter(|&i| units[i].side == Side::Party).collect();
        let foes: Vec<usize> = (0..n).filter(|&i| units[i].side == Side::Foe).collect();
        let foe_posts: Vec<Post> = foes
            .iter()
            .map(|&i| {
                if units[i].ranged && !units[i].melee {
                    Post::Back
                } else {
                    Post::Front
                }
            })
            .collect();
        let board = Board::new(units, vec![0; n], vec![Post::Front; n]);
        State {
            board,
            party,
            foes,
            phase: Phase::Setup { next: 0 },
            pending: vec![None; n],
            round: 0,
            foe_posts,
        }
    }

    /// The largest region id any already-placed party hero stands in (`None` before the first placement).
    fn max_party_region(&self, upto: usize) -> Option<u8> {
        self.party[..upto]
            .iter()
            .map(|&i| self.board.regions[i])
            .max()
    }

    /// The next living party unit at or after `from` that has not declared, or `None` if all have.
    fn next_undeclared(&self, from: usize) -> Option<usize> {
        self.party[from..]
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

    /// The **party unit whose decision is pending** right now - the hero being placed at setup, or the hero
    /// declaring its act this round. `None` if nobody is deciding (a forced/terminal state). A UI names it so
    /// "place region A" is never ambiguous about *which* hero.
    pub fn deciding(&self) -> Option<usize> {
        match self.phase {
            Phase::Setup { next } => self.party.get(next).copied(),
            Phase::Declare { next } => self.party.get(next).copied(),
        }
    }

    /// Whether the pending decision is a setup placement (vs a round action).
    pub fn placing(&self) -> bool {
        matches!(self.phase, Phase::Setup { .. })
    }
}

/// The regions combat as a [`Game`].
pub struct Combat;

impl Game for Combat {
    type State = State;
    type Choice = Choice;

    fn options(state: &State) -> Vec<Choice> {
        match state.phase {
            Phase::Setup { next } => {
                // Hero `party[next]` picks a region and a post. It may JOIN any region an earlier hero already
                // stands in, or OPEN the next fresh one (restricted growth - each partition offered once, never
                // a relabelling), and it may face front or back.
                let ceiling = state.max_party_region(next).map_or(0, |m| m + 1);
                let mut out = Vec::new();
                for region in 0..=ceiling {
                    for post in [Post::Front, Post::Back] {
                        out.push(Choice::Place { region, post });
                    }
                }
                out
            }
            Phase::Declare { next } => state
                .party
                .get(next)
                .map(|&i| {
                    legal_acts(&state.board, i)
                        .into_iter()
                        .map(Choice::Act)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn apply(state: &State, choice: &Choice) -> State {
        let mut s = state.clone();
        match (s.phase, choice) {
            (Phase::Setup { next }, Choice::Place { region, post }) => {
                let hero = s.party[next];
                s.board.regions[hero] = *region;
                s.board.posts[hero] = *post;
                match s.party.get(next + 1) {
                    Some(_) => s.phase = Phase::Setup { next: next + 1 },
                    None => {
                        // The party is formed. Drop the foes into a region of their own, past the party's, and
                        // begin the first round.
                        let foe_region = s.max_party_region(s.party.len()).map_or(0, |m| m + 1);
                        for (k, &f) in s.foes.iter().enumerate() {
                            s.board.regions[f] = foe_region;
                            s.board.posts[f] = s.foe_posts[k];
                        }
                        s.round = 1;
                        s.phase = Phase::Declare {
                            next: s.next_undeclared(0).unwrap_or(s.party.len()),
                        };
                    }
                }
            }
            (Phase::Declare { next }, Choice::Act(act)) => {
                let hero = s.party[next];
                s.pending[hero] = Some(*act);
                match s.next_undeclared(next + 1) {
                    Some(n) => s.phase = Phase::Declare { next: n },
                    None => resolve_round(&mut s),
                }
            }
            _ => debug_assert!(false, "choice does not match phase"),
        }
        s
    }

    fn outcome(state: &State) -> Option<Outcome> {
        // No verdict until the fight has actually begun.
        if matches!(state.phase, Phase::Setup { .. }) {
            return None;
        }
        match state.board.outcome() {
            Some(true) => Some(Outcome::Win),
            Some(false) => Some(Outcome::Loss),
            None if state.round > MAX_ROUNDS => Some(Outcome::Draw),
            None => None,
        }
    }
}

/// The whole round resolves as one transition: the party's declared acts, the foes scripted in, then
/// [`play_round`]. Afterwards the cursor resets to the next round's first decision.
fn resolve_round(s: &mut State) {
    let mut acts: Vec<Act> = (0..s.board.units.len())
        .map(|i| s.pending[i].unwrap_or(Act::Hold))
        .collect();
    for (i, a) in foe_acts(&s.board).into_iter().enumerate() {
        if let Some(a) = a {
            acts[i] = a;
        }
    }
    play_round(&mut s.board, &acts);

    s.round += 1;
    s.pending = vec![None; s.board.units.len()];
    s.phase = Phase::Declare {
        next: s.next_undeclared(0).unwrap_or(s.party.len()),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::run;

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    /// The Game plays a whole fight to a verdict, driven by the generic runner - setup, then rounds, with the
    /// foe folded into apply and never appearing in the options.
    #[test]
    fn the_runner_plays_a_whole_fight() {
        let s = State::new(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [1, 2, 1, 1, 1], true, false),
        ]);
        // Always take the first option (place front-of-region-0, then Clash the foe).
        let out = run::<Combat>(s, |_, _| 0);
        assert_eq!(out, Outcome::Win, "the stronger body wins the fight");
    }

    /// Options are only ever the PARTY's - the foe is never offered a choice, because it is scripted inside
    /// apply.
    #[test]
    fn the_foe_never_appears_in_the_options() {
        let mut s = State::new(vec![
            unit("Hero", Side::Party, [5, 4, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [4, 4, 1, 2, 2], true, false),
        ]);
        // Walk through setup (place the one hero), then check the first Declare state.
        while matches!(s.phase, Phase::Setup { .. }) {
            s = Combat::apply(&s, &Combat::options(&s)[0]);
        }
        let opts = Combat::options(&s);
        assert!(!opts.is_empty(), "the hero has acts to choose from");
        // Every option is an Act declared by the hero; nothing here is a foe decision.
        for o in &opts {
            assert!(matches!(o, Choice::Act(_)));
        }
    }

    /// Setup enumerates the party's formations: one hero => join-or-open x front/back. A single hero can only be
    /// in region 0, front or back - two placements, not a partition explosion.
    #[test]
    fn setup_offers_a_post_choice_even_for_a_lone_hero() {
        let s = State::new(vec![
            unit("Hero", Side::Party, [5, 4, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [4, 4, 1, 2, 2], true, false),
        ]);
        let opts = Combat::options(&s);
        assert_eq!(
            opts,
            vec![
                Choice::Place {
                    region: 0,
                    post: Post::Front
                },
                Choice::Place {
                    region: 0,
                    post: Post::Back
                },
            ]
        );
    }

    /// Two heroes: the second may join the first's region or open a new one - the partition search, as choices.
    #[test]
    fn setup_lets_a_second_hero_group_or_split() {
        let mut s = State::new(vec![
            unit("A", Side::Party, [5, 4, 1, 2, 2], true, false),
            unit("B", Side::Party, [5, 4, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [4, 4, 1, 2, 2], true, false),
        ]);
        s = Combat::apply(
            &s,
            &Choice::Place {
                region: 0,
                post: Post::Front,
            },
        ); // A in region 0
        let regions: Vec<u8> = Combat::options(&s)
            .into_iter()
            .filter_map(|c| match c {
                Choice::Place { region, .. } => Some(region),
                _ => None,
            })
            .collect();
        assert!(regions.contains(&0), "B may join A");
        assert!(regions.contains(&1), "B may split off");
        assert!(
            !regions.contains(&2),
            "but not open a region beyond the next - no relabellings"
        );
    }
}

// ---- searchability: the canonical key, and the "no slip" control ---------------------------------------

use super::regions::canonical;
use crate::core::Solvable;

/// A hashable digest of a position: per-unit `(health, fallen, post)`, the **canonicalized** regions (so a
/// relabelling is not a distinct position), the round, and the pending declarations + cursor (a half-declared
/// round is genuinely a different state than a fresh one).
///
/// `tempo` and the damage pile are absent on purpose: both are re-derived at the round Reset, so they are only
/// ever meaningful *inside* [`play_round`], never at a state a search actually visits.
type Key = (Vec<(u32, bool, Post)>, Vec<u8>, usize, u8, Vec<Option<Act>>);

fn key_of(s: &State) -> Key {
    let per: Vec<(u32, bool, Post)> = s
        .board
        .units
        .iter()
        .map(|u| (u.health, u.fallen, Post::Front)) // post filled below (units carry no post; the board does)
        .collect();
    // Fold the board's posts in (units and posts are index-aligned).
    let per: Vec<(u32, bool, Post)> = per
        .into_iter()
        .enumerate()
        .map(|(i, (h, f, _))| (h, f, s.board.posts[i]))
        .collect();
    let cursor = match s.phase {
        Phase::Setup { next } => 1 + next as u8, // setup cursors are distinct from declare (which starts at 0)
        Phase::Declare { next } => 128 + next as u8,
    };
    (
        per,
        canonical(&s.board.regions),
        s.round,
        cursor,
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
        Combat::options(state)
            .into_iter()
            .filter(|c| !matches!(c, Choice::Act(Act::Raid(..)) | Choice::Act(Act::Slip(..))))
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
        // A board with a screened back, so `Combat` WOULD offer a raid.
        let mut s = State::new(vec![
            u("Raider", Side::Party, [7, 6, 1, 3, 2], true, false),
            u("Wall", Side::Foe, [1, 6, 4, 1, 2], true, false),
            u("Cannon", Side::Foe, [4, 2, 1, 2, 2], false, true),
        ]);
        while matches!(s.phase, Phase::Setup { .. }) {
            s = Combat::apply(&s, &Combat::options(&s)[0]);
        }
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
