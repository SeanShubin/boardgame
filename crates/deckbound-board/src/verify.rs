//! **Balance + insight verification, shared.** The machinery the `regions_diagonal` example *prints* and the
//! `tests/diagonal.rs` gate *asserts* - kept here so the human diagnostic and the automated gate can never
//! disagree about who wins.
//!
//! Two questions per warband, and their pairing is the whole point:
//! - [`solver_wins`] - can the party force a win with **optimal** play? (the balance gate)
//! - [`greedy_wins`] - can it win with **greedy** play, no search? (the "without thinking" baseline)
//!
//! [`insight_class`] combines them: `T`rivial (greedy already wins), `I`nsight (only the solver wins - a real
//! read is needed), `X` impossible (neither). `greedy wins & solver loses` cannot happen (greedy is a legal line),
//! so the three are exhaustive.

use rules::combat::resolve::Combatant;
use rules::combat::step_game::{StepCombat, StepState, greedy_step_playout};
use rules::core::{Game, Outcome, Solvable, Solver, Verdict};

/// Stop doubling the node grant past this ceiling. A position we cannot decisively settle within it is treated as
/// **not cleanly winnable** - the safe direction for a balance gate (better to under-claim a win than to lean a
/// lesson on a search that never finished).
pub const GRANT_CAP: u64 = 20_000_000;

/// **Can these heroes force a win under game `G`?** `G` is [`StepCombat`] or a control like `StepClashOnly`. The
/// verdict is ground out with an escalating grant (doubling on `Evaluating`) up to [`GRANT_CAP`]; past the cap a
/// still undecided position is called NOT winnable.
pub fn solver_wins<G>(heroes: &[Combatant], foes: &[Combatant]) -> bool
where
    G: Solvable + Game<State = StepState>,
{
    let mut units: Vec<Combatant> = heroes.to_vec();
    units.extend_from_slice(foes);
    let s = StepState::new(units);

    let mut o = Solver::<G>::new();
    let mut grant = 1u64;
    loop {
        o.grant(grant);
        match o.verdict(&s) {
            Verdict::Winnable => return true,
            Verdict::Doomed => return false,
            Verdict::Evaluating => {
                if grant >= GRANT_CAP {
                    return false;
                }
                grant = grant.saturating_mul(2);
            }
        }
    }
}

/// **Can these heroes win playing GREEDILY?** Both sides run the scripted step policy
/// ([`greedy_step_playout`]) - no search, no insight. One deterministic playout (bounded by the draw cap), so it
/// is effectively free.
pub fn greedy_wins(heroes: &[Combatant], foes: &[Combatant]) -> bool {
    let mut units: Vec<Combatant> = heroes.to_vec();
    units.extend_from_slice(foes);
    matches!(greedy_step_playout(StepState::new(units)), Outcome::Win)
}

/// The insight class of these heroes vs these foes: `T`rivial (greedy wins), `I`nsight (only the solver wins), `X`
/// impossible (neither).
pub fn insight_class(heroes: &[Combatant], foes: &[Combatant]) -> char {
    match (
        solver_wins::<StepCombat>(heroes, foes),
        greedy_wins(heroes, foes),
    ) {
        (_, true) => 'T',
        (true, false) => 'I',
        (false, false) => 'X',
    }
}
