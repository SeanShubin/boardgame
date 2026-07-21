//! **The step machine's RESOLVER** - the eight-step round schedule, resolved from pre-supplied per-step
//! declarations. **Additive and inert**: nothing shipped calls it yet; it is stage A of the step-machine
//! restructure (see `needs-merge/round-sequence.md`, "IN FLIGHT: the step machine"). The decision layer - the
//! per-step declare/reveal waves in the `Game`, the foe scripts, the solver - is stage B; until it lands, a
//! [`StepScript`] is built by hand (tests) or by a driver.
//!
//! The eight steps, each declare -> reveal -> resolve, deaths closing at every step so **later targeting can
//! react to earlier deaths in the same round** - the design intent the wave model could not express (targets
//! declared up front could never aim at a rearguard whose screen fell mid-round):
//!
//! | # | who -> whom | resolution |
//! |---|---|---|
//! | 1 | O->RV, RV->O | in-region, both tiers, no screen, mutual; aoe sweeps the region; pours |
//! | 2 | O -> V | withdraw, free - step 1 was the price |
//! | 3 | V->V | the EARLY front trade / interception window; pours |
//! | 4 | V -> O | the crossing: only a vanguard that declared NO strike this round; uncontested walk |
//! | 5 | R->O | the volley: one-way, opening blow only |
//! | 6 | O->R | the raid: THIS round's arrivals only, opening blow only |
//! | 7 | RV->V | the LATE front trade: fire + held-back vanguards ("halt" is emergent); pours |
//! | 8 | RV->R | only vs a rearguard with NO living vanguard AT THIS STEP - the same-round advance; pours |
//!
//! **Tempo pin**: mutual melee steps (1, 3, 7, 8) pour the striker's remaining pool (the clash rule); the volley
//! (5) and the raid (6) land the opening blow only (the volley/raid rule). Contact bids auto-size; defense is the
//! automatic greedy dodge. A body may act in any step it can fund - all-in pours make one-strike-per-round
//! emergent for low-Cadence bodies, exactly the wave model's shape.
//!
//! **Validity is resolver-enforced, menu-independent** (the house rule): a declaration whose actor or target is
//! fallen, wrong-ranked *at resolution time*, or otherwise unreachable simply drops - it never mislands.

use super::regions::{
    Attack, Board, Rank, SubPhaseLog, close, dissolve, exchange, home_of, living,
};
use super::resolve::refresh_round;

/// **One round's declarations, per step** - what the decision layer (stage B) will collect wave by wave, and what
/// a test builds by hand. Every list is `(actor, target)` except the movement steps. Invalid entries drop at the
/// resolver; an absent body simply passes.
#[derive(Clone, Debug, Default)]
pub struct StepScript {
    /// Step 1: in-region strikes - an outrider at a host, a host at an intruding outrider.
    pub inner: Vec<Attack>,
    /// Step 2: outriders withdrawing to their own line.
    pub withdraw: Vec<usize>,
    /// Step 3: the early front trade - vanguard at enemy vanguard.
    pub early: Vec<Attack>,
    /// Step 4: the crossings - vanguards walking into the enemy region (must have declared NO strike).
    pub cross: Vec<usize>,
    /// Step 5: the volley - rearguard at enemy outrider.
    pub volley: Vec<Attack>,
    /// Step 6: the raid - a THIS-round arrival at an enemy rearguard.
    pub raid: Vec<Attack>,
    /// Step 7: the late front trade - vanguard OR rearguard at enemy vanguard.
    pub late: Vec<Attack>,
    /// Step 8: the advance - vanguard or rearguard at an enemy rearguard whose screen has fallen (checked at
    /// this step, so a collapse earlier this round opens it).
    pub advance: Vec<Attack>,
}

impl StepScript {
    /// Did `i` declare a LINE strike this round? The crossing gate: you cannot fight and run - a vanguard that
    /// declared any strike from the line (landed or not; the declaration is the commitment) may not cross. The
    /// **raid is exempt**: a raid declaration presupposes the crossing - it is the crossing's payoff, not a
    /// strike from the line it is leaving.
    fn declared_strike(&self, i: usize) -> bool {
        self.inner.iter().any(|&(a, _)| a == i)
            || self.early.iter().any(|&(a, _)| a == i)
            || self.volley.iter().any(|&(a, _)| a == i)
            || self.late.iter().any(|&(a, _)| a == i)
            || self.advance.iter().any(|&(a, _)| a == i)
    }
}

/// Resolve one exchange step: filter the declarations to the pairs valid RIGHT NOW (deaths from earlier steps
/// have closed), run the shared physics, close the step, tag it. An empty step logs nothing.
fn step(
    board: &mut Board,
    logs: &mut Vec<SubPhaseLog>,
    phase: &'static str,
    attacks: Vec<Attack>,
    pour: bool,
    sweep_whole: bool,
) {
    if attacks.is_empty() {
        return;
    }
    let before = living(board);
    let (hits, reaches) = exchange(board, &attacks, pour, sweep_whole);
    let mut lg = close(board, &before);
    lg.phase = phase;
    lg.hits = hits;
    lg.reaches = reaches;
    logs.push(lg);
}

/// The pairs of `list` valid at this moment: both alive, enemies, and passing `ok(actor, target)`.
fn valid(board: &Board, list: &[Attack], ok: impl Fn(usize, usize) -> bool) -> Vec<Attack> {
    list.iter()
        .copied()
        .filter(|&(a, t)| {
            !board.units[a].fallen
                && !board.units[t].fallen
                && board.units[a].side != board.units[t].side
                && board.units[a].tempo > 0
                && ok(a, t)
        })
        .collect()
}

/// **Play one whole round through the eight-step schedule.** Returns the per-step transcript, one
/// [`SubPhaseLog`] per step that did anything, with the step's own phase label.
pub fn play_steps(board: &mut Board, s: &StepScript) -> Vec<SubPhaseLog> {
    refresh_round(&mut board.units);
    let mut logs = Vec::new();

    // ---- STEP 1: the inner brawl - in-region, both tiers, no screen, mutual. --------------------------
    let inner = valid(board, &s.inner, |a, t| board.regions[a] == board.regions[t]);
    step(board, &mut logs, "Step 1: Inner", inner, true, true);
    // An outrider whose host formation was wiped here dissolves - it rejoins its own line at the boundary.
    dissolve(board);

    // ---- STEP 2: withdrawal - free; step 1 was the price. ---------------------------------------------
    let mut withdrew: Vec<usize> = Vec::new();
    for &i in &s.withdraw {
        if board.units[i].fallen || board.ranks[i] != Rank::Outrider {
            continue; // a corpse never leaves; a dissolved body is home already
        }
        board.regions[i] =
            home_of(board, board.units[i].side, board.regions[i]).unwrap_or(board.regions[i]);
        board.ranks[i] = Board::weapon_rank(&board.units[i]);
        withdrew.push(i);
    }
    if !withdrew.is_empty() || !logs.is_empty() {
        // Fold the boundary moves (dissolution + withdrawal) into the step-1 snapshot, or record them alone.
        if let Some(lg) = logs.last_mut() {
            lg.ranks = board.ranks.clone();
            lg.regions = board.regions.clone();
            lg.withdrew = withdrew;
        } else if !withdrew.is_empty() {
            let mut lg = SubPhaseLog {
                phase: "Step 2: Withdraw",
                health: board.units.iter().map(|u| u.health).collect(),
                tempo: board.units.iter().map(|u| u.tempo).collect(),
                ranks: board.ranks.clone(),
                regions: board.regions.clone(),
                withdrew,
                ..Default::default()
            };
            lg.fallen = Vec::new();
            logs.push(lg);
        }
    }

    // ---- STEP 3: the early front trade - the interception window. -------------------------------------
    let early = valid(board, &s.early, |a, t| {
        board.ranks[a] == Rank::Vanguard && board.ranks[t] == Rank::Vanguard
    });
    step(board, &mut logs, "Step 3: Early Trade", early, true, false);

    // ---- STEP 4: the crossings - an uncontested walk, gated on having declared no strike. --------------
    let mut landed: Vec<usize> = Vec::new();
    for &i in &s.cross {
        if board.units[i].fallen || board.ranks[i] != Rank::Vanguard || s.declared_strike(i) {
            continue; // dead men don't walk; you cannot fight and run
        }
        let Some(dest) = board
            .occupied()
            .into_iter()
            .find(|&r| r != board.regions[i])
        else {
            continue;
        };
        let into_enemy =
            board.owner(dest) != Some(board.units[i].side) && board.owner(dest).is_some();
        if !into_enemy {
            continue;
        }
        board.regions[i] = dest;
        board.ranks[i] = Rank::Outrider;
        landed.push(i);
    }
    if !landed.is_empty() {
        logs.push(SubPhaseLog {
            phase: "Step 4: Crossing",
            through: landed.clone(),
            health: board.units.iter().map(|u| u.health).collect(),
            tempo: board.units.iter().map(|u| u.tempo).collect(),
            ranks: board.ranks.clone(),
            regions: board.regions.clone(),
            ..Default::default()
        });
    }

    // ---- STEP 5: the volley - one-way, opening blow only. ----------------------------------------------
    let volley = valid(board, &s.volley, |a, t| {
        board.ranks[a] == Rank::Rearguard && board.ranks[t] == Rank::Outrider
    });
    step(board, &mut logs, "Step 5: Volley", volley, false, false);

    // ---- STEP 6: the raid - THIS round's arrivals strike a back-line target, opening blow only. --------
    let raid = valid(board, &s.raid, |a, t| {
        landed.contains(&a)
            && board.ranks[t] == Rank::Rearguard
            && board.regions[t] == board.regions[a]
    });
    step(board, &mut logs, "Step 6: Raid", raid, false, false);

    // ---- STEP 7: the late front trade - fire, and every vanguard that held back. -----------------------
    let late = valid(board, &s.late, |a, t| {
        matches!(board.ranks[a], Rank::Vanguard | Rank::Rearguard)
            && board.ranks[t] == Rank::Vanguard
    });
    step(board, &mut logs, "Step 7: Late Trade", late, true, false);

    // ---- STEP 8: the advance - only on a back whose screen has fallen BY NOW (same-round reactive). ----
    let advance = valid(board, &s.advance, |a, t| {
        matches!(board.ranks[a], Rank::Vanguard | Rank::Rearguard)
            && board.ranks[t] == Rank::Rearguard
            && !board.is_screened(t)
    });
    step(board, &mut logs, "Step 8: Advance", advance, true, false);

    logs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::resolve::{Combatant, Side};

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    /// Raider + Archer vs Wall + Sniper: the smallest board with both fronts and both backs.
    fn board() -> Board {
        Board::new(
            vec![
                unit("Raider", Side::Party, [6, 6, 1, 2, 2], true, false),
                unit("Archer", Side::Party, [5, 2, 1, 2, 2], false, true),
                unit("Wall", Side::Foe, [1, 4, 6, 1, 2], true, false),
                unit("Sniper", Side::Foe, [5, 1, 1, 2, 3], false, true),
            ],
            vec![0, 0, 1, 1],
        )
    }

    /// **An early kill prevents the crossing** - the whole point of step 3 preceding step 4: the interception
    /// is striking the body you predict will run, and a body felled at the early trade never walks.
    #[test]
    fn an_early_kill_prevents_the_crossing() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [6, 2, 1, 2, 2], true, false), // 2 health: killable early
                unit("Brute", Side::Foe, [5, 5, 1, 2, 2], true, false),
                unit("Sniper", Side::Foe, [5, 1, 1, 2, 3], false, true),
            ],
            vec![0, 1, 1],
        );
        let s = StepScript {
            early: vec![(1, 0)], // the Brute strikes the would-be runner, blind
            cross: vec![0],      // the Raider intended to walk
            raid: vec![(0, 2)],
            ..Default::default()
        };
        play_steps(&mut b, &s);
        assert!(b.units[0].fallen, "the early trade felled it");
        assert_eq!(b.regions[0], 0, "and a dead man never walked");
        assert!(!b.units[2].fallen, "so the Sniper was never raided");
    }

    /// **You cannot fight and run** - a vanguard that declared ANY strike this round cannot cross, even if the
    /// strike whiffed. The declaration is the commitment.
    #[test]
    fn a_striking_vanguard_cannot_cross() {
        let mut b = board();
        let s = StepScript {
            early: vec![(0, 2)], // the Raider swings at the Wall...
            cross: vec![0],      // ...and also tries to walk
            ..Default::default()
        };
        play_steps(&mut b, &s);
        assert_eq!(b.regions[0], 0, "the strike committed it to the line");
        assert_eq!(b.ranks[0], Rank::Vanguard);
    }

    /// **The same-round advance** - the collapse intent, end to end: the early trade fells the last enemy
    /// vanguard, and step 8 reaches the rearguard behind it THIS round. The wave model could never do this
    /// (targets were declared while the back was still screened).
    #[test]
    fn a_collapsed_front_is_advanced_upon_the_same_round() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false), // Might 7 crushes Grit 6 (health 1)
                // Finesse 3: its 2-card reach (bid 6) prices the Sniper's dodge out (cost 3 > tempo 2), so the
                // advance is a guaranteed contact - this test asserts the same-round REACH, cleanly.
                unit("Archer", Side::Party, [5, 2, 1, 2, 3], false, true),
                unit("Wall", Side::Foe, [1, 1, 6, 1, 2], true, false), // 1 health: collapses to one blow
                unit("Sniper", Side::Foe, [5, 1, 1, 2, 3], false, true),
            ],
            vec![0, 0, 1, 1],
        );
        let s = StepScript {
            early: vec![(0, 2)],   // fell the Wall at the early trade
            advance: vec![(1, 3)], // the Archer advances on the Sniper - screened at declaration, exposed by now
            ..Default::default()
        };
        play_steps(&mut b, &s);
        assert!(b.units[2].fallen, "the front collapsed at step 3");
        assert!(
            b.units[3].fallen,
            "and the back was advanced upon the SAME round"
        );
    }

    /// **Halt is emergent** - a would-be crosser that held back (declared nothing early) simply swings at the
    /// late trade instead. No posture enum, no special rule: the two windows do it.
    #[test]
    fn halting_is_swinging_at_the_late_trade() {
        let mut b = board();
        let wall = b.units[2].health;
        let s = StepScript {
            // The Raider declares nothing early (it was thinking about running), then swings late.
            late: vec![(0, 2)],
            ..Default::default()
        };
        play_steps(&mut b, &s);
        assert!(
            b.units[2].health < wall,
            "the held-back vanguard traded at step 7 with its whole pool"
        );
        assert_eq!(b.regions[0], 0, "and stayed home");
    }

    /// **The crossing is a walk, and the volley is its price** - the crosser lands as an outrider (step 4),
    /// the rearguard volleys it (step 5, one-way), and the survivor raids (step 6).
    #[test]
    fn cross_volley_raid_in_order() {
        let mut b = board();
        let s = StepScript {
            cross: vec![0],
            volley: vec![(3, 0)], // the Sniper volleys the fresh outrider
            raid: vec![(0, 3)],   // who then strikes it
            ..Default::default()
        };
        let logs = play_steps(&mut b, &s);
        assert_eq!(b.ranks[0], Rank::Outrider, "landed");
        assert_eq!(b.regions[0], 1);
        assert!(
            b.units[0].health < 6,
            "the volley hit it on the way in (one-way)"
        );
        assert!(b.units[3].fallen, "and the raid felled the Sniper");
        let phases: Vec<&str> = logs.iter().map(|l| l.phase).collect();
        assert_eq!(
            phases,
            vec!["Step 4: Crossing", "Step 5: Volley", "Step 6: Raid"],
            "the transcript carries the step labels in schedule order"
        );
    }

    /// **A prior-round outrider fights at step 1 and may withdraw at step 2** - the inner brawl then the free
    /// exit, unchanged from the wave model.
    #[test]
    fn inner_then_withdraw() {
        let mut b = board();
        b.regions[0] = 1;
        b.ranks[0] = Rank::Outrider; // the Raider is loose inside from a prior round
        let s = StepScript {
            inner: vec![(0, 2), (2, 0)], // it trades with the Wall...
            withdraw: vec![0],           // ...and walks out at the boundary
            ..Default::default()
        };
        play_steps(&mut b, &s);
        assert_eq!(b.regions[0], 0, "home");
        assert_eq!(
            b.ranks[0],
            Rank::Vanguard,
            "back in its line at weapon rank"
        );
    }
}
