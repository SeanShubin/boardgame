//! **The decision policy** (§4.6) — *how the code chooses among legal moves*, cleaved out of the
//! mechanics resolver ([`crate::combat`]). The resolver applies the rules of the game; the policy
//! decides which target a unit aims at, whether a defender evades, whether it strikes back, and **when
//! an engagement stops cycling**. Swapping a human / scripted AI / solver swaps *this* module; the
//! mechanics in [`crate::combat`] do not change.
//!
//! This is the live mirror of the validated `engagement.rs` sim's policy half (`priorities` / `step_of`
//! / `choose_target` / `team_can_crack` / `should_avoid` + the cycling-to-exhaustion loop). It reads
//! `&State` (never mutates) and produces the [`Decision`]s the resolver consumes.
//!
//! ## The default policy (the predictable human stand-in)
//!
//! - **Role priority lists** (§4.6): a Vanguard targets `O→V→R`, an Outrider `R→V→O`, a Rearguard
//!   `V→O→R`. A unit holds its Tempo for a higher-priority engagement still to come rather than spend
//!   it on a lower one now (mirrors `priorities` + `step_of`).
//! - **Positive-effect rule, judged at the target** (§4.6): commit a strike only when it would flip a
//!   card — alone, *or* via focus-fire (the combined committed Might of the team that can reach this
//!   target, plus what is already banked in its pile, crosses its Toughness). Never a futile spend.
//! - **Back-access gate** (§4.6): a Rearguard is reachable only by an Outrider's raid, or once its own
//!   side's Vanguard has fallen (the dropped screen opens the back).
//! - **Evade**: full-cost only — a defender spends Tempo to dodge only a blow that would flip a card,
//!   and only if it can afford the bid.
//! - **Strike-back**: only against a melee attacker, only by a melee-capable defender, and only when it
//!   can crack the attacker (never burn Tempo on a blow that bounces off Toughness).

use crate::actor::{Actor, Intention, Range};
use crate::combat::base_strike;

/// The role priority list (§4.6): each role's ordered target preference. The Outrider alone puts the
/// back first (its raid); every other role puts it last (a mop-up through a broken line). Mirrors
/// `engagement::priorities`.
pub fn priorities(role: Intention) -> [Intention; 3] {
    use Intention::{Outrider as O, Rearguard as R, Vanguard as V};
    match role {
        V => [O, V, R], // screen the flankers → clash the front → breach the back
        O => [R, V, O], // raid the back → flank the front → hunt stragglers
        R => [V, O, R], // destroy the front → hunt flankers → finish the back
    }
}

/// Does `side`'s pool still field a **living Vanguard**? The back-access gate: a Rearguard is shielded
/// while its own side's Vanguard lives (§4.6). `def_int` is the per-unit intention of `def`.
pub fn vanguard_alive(def: &[Actor], def_int: &[Intention]) -> bool {
    def.iter()
        .enumerate()
        .any(|(j, u)| !u.is_down() && def_int.get(j) == Some(&Intention::Vanguard))
}

/// **Back-access gate** (§4.6): is a Rearguard target reachable by this attacker? Only by an
/// **Outrider**'s raid, or once the defending side has **no living Vanguard** (its screen fell). For
/// any non-Rearguard target this is always true. Mirrors the gate in `engagement::choose_target`.
pub fn can_reach(
    attacker_is_outrider: bool,
    tgt_role: Intention,
    def: &[Actor],
    def_int: &[Intention],
) -> bool {
    if tgt_role != Intention::Rearguard {
        return true;
    }
    attacker_is_outrider || !vanguard_alive(def, def_int)
}

/// Can `(atk_role → tgt_role)` resolve *somewhere* in the schedule? (Reach upper-bound for the
/// focus-fire estimate.) Mirrors `engagement::role_can_attack`.
fn role_can_attack(atk_role: Intention, tgt_role: Intention) -> bool {
    crate::combat::SCHEDULE
        .iter()
        .any(|pairs| pairs.iter().any(|&(a, t)| a == atk_role && t == tgt_role))
}

/// **Focus-fire test** (the positive-effect rule judged at the target, §4.6). A weak strike that cannot
/// flip a card *alone* is still worth committing if the **combined** Might the team can pile onto this
/// target — plus what is already banked in its pile — crosses its (effective) Toughness. Sums the
/// effective Might of living allies of `atk_side` that can reach this target's role. Mirrors
/// `engagement::team_can_crack`.
pub fn team_can_crack(
    atk_pool: &[Actor],
    atk_int: &[Intention],
    target: &Actor,
    tgt_role: Intention,
) -> bool {
    let tough = target.eff_toughness();
    let pile = target.defense.health_pile();
    let sum: u32 = atk_pool
        .iter()
        .enumerate()
        .filter(|(j, u)| {
            !u.is_down()
                && atk_int
                    .get(*j)
                    .is_some_and(|&r| role_can_attack(r, tgt_role))
        })
        .map(|(_, u)| u.eff_might())
        .sum();
    sum + pile >= tough
}

/// Pick the best target for `attacker` (index `_ai`, role `_atk_role`) among living enemies of
/// `tgt_role`: reachable (back-access gate), crackable (this strike alone given the banked pile, *or*
/// by the team's combined Might — focus-fire), lowest remaining Health first (finish kills). Mirrors
/// `engagement::choose_target`. Returns the chosen enemy's index in `def`.
#[allow(clippy::too_many_arguments)]
pub fn choose_target(
    attacker: &Actor,
    attacker_is_outrider: bool,
    atk_pool: &[Actor],
    atk_int: &[Intention],
    tgt_role: Intention,
    def: &[Actor],
    def_int: &[Intention],
) -> Option<usize> {
    if !can_reach(attacker_is_outrider, tgt_role, def, def_int) {
        return None;
    }
    let might = attacker.eff_might();
    def.iter()
        .enumerate()
        .filter(|(j, u)| !u.is_down() && def_int.get(*j) == Some(&tgt_role))
        .filter(|(_, u)| {
            might + u.defense.health_pile() >= u.eff_toughness()
                || team_can_crack(atk_pool, atk_int, u, tgt_role)
        })
        .min_by_key(|(j, u)| (u.defense.health.remaining(), *j))
        .map(|(j, _)| j)
}

/// **The governing target for `attacker` this engagement step**, by the role priority list timed
/// against the schedule (§4.6). Walk the priorities in order; the first one with a crackable target is
/// the goal. Strike it **iff its engagement is the current `step_idx`**; **hold** (return `None`,
/// committing nothing) if its window is still to come; **skip** to the next priority once its window
/// has passed. Mirrors the inner targeting loop of `engagement::run_round_logged`. Returns the chosen
/// target's **role** and its index in `def`, so the per-pair resolver can act only when the governing
/// role matches the pair it is resolving.
#[allow(clippy::too_many_arguments)]
pub fn governing_target(
    step_idx: usize,
    attacker: &Actor,
    atk_role: Intention,
    atk_pool: &[Actor],
    atk_int: &[Intention],
    def: &[Actor],
    def_int: &[Intention],
) -> Option<(Intention, usize)> {
    let is_outrider = atk_role == Intention::Outrider;
    for &role in &priorities(atk_role) {
        let Some(st) = step_of(atk_role, role) else {
            continue;
        };
        let pick = choose_target(attacker, is_outrider, atk_pool, atk_int, role, def, def_int);
        let Some(ti) = pick else {
            continue; // no crackable target of this role — try the next priority
        };
        if st == step_idx {
            return Some((role, ti)); // its window is now — strike
        }
        if st > step_idx {
            return None; // a higher priority's window is still to come — hold Tempo for it
        }
        // st < step_idx: this priority's window has passed — fall through to the next
    }
    None
}

/// The schedule step index in which the pair `(atk_role → tgt_role)` resolves, or `None` if it is never
/// a legal pair. Mirrors `engagement::step_of`.
pub fn step_of(atk_role: Intention, tgt_role: Intention) -> Option<usize> {
    crate::combat::SCHEDULE
        .iter()
        .position(|pairs| pairs.iter().any(|&(a, t)| a == atk_role && t == tgt_role))
}

/// Tempo a defender must commit to **avoid** one attack (§4.6 contest): `cards × Fd` must strictly
/// exceed `Fa`, so the minimum cards is `floor(Fa / Fd) + 1`. Mirrors `engagement::avoid_cost`.
pub fn avoid_cost(attacker_finesse: u32, defender_finesse: u32) -> i32 {
    (attacker_finesse / defender_finesse.max(1)) as i32 + 1
}

/// **Should the soaker evade this aimed blow?** Full-cost evade only (§4.6): avoid a blow that would
/// flip a card (Might ≥ effective Toughness — sub-threshold hits wipe harmlessly at the boundary), and
/// only if it can afford the bid. Mirrors `engagement::should_avoid`. (A *group* never slips — that is
/// the weakest-link rule, handled in the resolver; this is the lone-unit decision.)
pub fn should_avoid(defender: &Actor, might: u32, attacker_finesse: u32) -> bool {
    let bar = defender.eff_toughness();
    let cost = avoid_cost(attacker_finesse, defender.eff_finesse());
    might >= bar && defender.tempo >= cost
}

/// **Should `soaker` strike back at the melee attacker `atk`?** Only by a melee-capable defender, for
/// one Tempo, and only when it can crack the attacker (positive-effect). A corpse cannot react. Mirrors
/// the reflexive strike-back gate in `engagement::run_round_logged` (which counts the attacker's banked
/// pile toward the crack — focus-fire on the attacker).
pub fn should_strike_back(soaker: &Actor, atk: &Actor) -> bool {
    soaker.attack.has(Range::Melee)
        && !soaker.is_down()
        && !atk.is_down()
        && soaker.tempo >= 1
        && base_strike(soaker) + atk.defense.health_pile() >= atk.eff_toughness()
}
