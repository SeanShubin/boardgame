//! **The balance diagonal, over the `rules` crate.** Does the encounter set still teach what it is meant to?
//!
//! Drives [`rules::combat::step_game`] - the eight-step round, behind the generic `Game` - via the generic
//! [`Solver`], through the same shared verify machinery the `tests/diagonal.rs` gate asserts.
//!
//! - Each **solo** must be soloable by EXACTLY ONE kit - the one its keystone is built to be weak to.
//! - Each **corner** must pass its assigned [`Behavior`] - the lesson it is built to teach, scored by search:
//!   the full party wins under `Combat`, AND the behavior's own necessity test holds (a sub-party wins/loses, or
//!   a control - clash-only / scattered - loses).
//!
//! Two reports from one tool:
//! - **fast gate** (default) - just the pass/fail, ~30 ms, run after every rule tweak to check nothing broke.
//! - **deep report** (`... regions_diagonal scores`) - adds each fight's **best route** (downed / rounds / hp,
//!   minimized by priority). Far slower - the scorer weighs every winning line - so it is opt-in.
//!
//! Run: `cargo run --release -p deckbound-board --example regions_diagonal [scores]`

use std::time::Instant;

use deckbound_board::units::{beast, kit};
use deckbound_board::verify::{GRANT_CAP, insight_class, solver_wins};
use deckbound_content::catalog::{self, Behavior, Encounter};
use rules::combat::resolve::Combatant;
use rules::combat::step_game::{
    Score, StepClashOnly as ClashOnly, StepCombat as Combat, StepScorer as Scorer,
    StepState as State,
};

fn foes_of(e: &Encounter) -> Vec<Combatant> {
    let mut out = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    out
}

/// **The best route these heroes can take to a win** under the priority order (win > fewest downed > fewest
/// rounds > least hp), or `None` if they cannot win. Informative only - it does **not** gate the diagonal, it just
/// says *how cleanly* the win comes. Budgeted with the same escalating grant as [`solver_wins`]; at the cap the
/// answer may be a provisional `<=` bound.
fn best_score(heroes: &[Combatant], foes: &[Combatant]) -> (Option<Score>, bool) {
    let mut units: Vec<Combatant> = heroes.to_vec();
    units.extend_from_slice(foes);
    let s = State::new(units);
    let start_hp: Vec<u32> = s.board().units.iter().map(|u| u.health).collect();

    let mut sc = Scorer::new(start_hp, 0);
    let mut grant = 1u64;
    loop {
        sc.grant(grant);
        let best = sc.best(&s);
        if !sc.aborted() {
            return (best, true); // exhausted - the score is exact
        }
        if grant >= GRANT_CAP {
            return (best, false); // provisional best at the cap (a `<=` upper bound)
        }
        grant = grant.saturating_mul(2);
    }
}

/// A best-route [`Score`] as a compact `Nd/Nr/Nhp` (downed / rounds / hp lost), prefixed `<=` when the search was
/// capped (a provisional bound), `-` for no win found, `?` for capped-before-any-win.
fn fmt_score((score, exact): (Option<Score>, bool)) -> String {
    match score {
        Some(s) => {
            let le = if exact { "" } else { "<=" };
            format!("{le}{}d/{}r/{}hp", s.downed, s.rounds, s.hp_lost)
        }
        None if exact => "-".to_string(),
        None => "?".to_string(),
    }
}

/// **Does this warband pass `behavior`?** `Ok(())` if it does; `Err(reason)` naming the first test that failed.
/// Every behavior first requires the full party to win under `Combat`; then its own necessity test:
///
/// - `Concentration`: single-target-only wins, area-only loses.
/// - `Range`: ranged-only wins, melee-only loses.
/// - `Sweep`: area-only wins, single-target-only loses.
/// - `Raid`: the full party under `ClashOnly` loses (the raid is load-bearing).
/// - `CombinedArms` (the capstone): melee-only, ranged-only, AND single-only all lose, AND `ClashOnly` loses -
///   so ranged, melee, an area strike, and the raid are ALL load-bearing at once.
///
/// The four corner strategies are orthogonal: each is a DIFFERENT control failing while its own strategy
/// suffices, so none subsumes another. The capstone is *meant* to subsume them (the graduation exam).
fn behavior_passes(
    behavior: Behavior,
    kits: &[Combatant],
    melee: &[Combatant],
    ranged: &[Combatant],
    single: &[Combatant],
    area: &[Combatant],
    foes: &[Combatant],
) -> Result<(), String> {
    if !solver_wins::<Combat>(kits, foes) {
        return Err("full party loses under Combat".to_string());
    }
    match behavior {
        Behavior::Concentration => {
            if !solver_wins::<Combat>(single, foes) {
                return Err("single-only party loses (concentration should carry it)".to_string());
            }
            if solver_wins::<Combat>(area, foes) {
                return Err("area-only party wins (concentration is not necessary)".to_string());
            }
        }
        Behavior::Range => {
            if !solver_wins::<Combat>(ranged, foes) {
                return Err("ranged-only party loses (range should carry it)".to_string());
            }
            if solver_wins::<Combat>(melee, foes) {
                return Err("melee-only party wins (range is not necessary)".to_string());
            }
        }
        Behavior::Sweep => {
            if !solver_wins::<Combat>(area, foes) {
                return Err("area-only party loses (a sweep should carry it)".to_string());
            }
            if solver_wins::<Combat>(single, foes) {
                return Err("single-only party wins (a sweep is not necessary)".to_string());
            }
        }
        Behavior::Raid => {
            if solver_wins::<ClashOnly>(kits, foes) {
                return Err(
                    "full party wins under ClashOnly (the raid is not necessary)".to_string(),
                );
            }
        }
        Behavior::CombinedArms => {
            if solver_wins::<Combat>(melee, foes) {
                return Err("melee-only party wins (ranged damage is not necessary)".to_string());
            }
            if solver_wins::<Combat>(ranged, foes) {
                return Err("ranged-only party wins (melee damage is not necessary)".to_string());
            }
            if solver_wins::<Combat>(single, foes) {
                return Err("single-only party wins (an area strike is not necessary)".to_string());
            }
            if solver_wins::<ClashOnly>(kits, foes) {
                return Err(
                    "full party wins under ClashOnly (the raid is not necessary)".to_string(),
                );
            }
        }
    }
    Ok(())
}

/// Print the `(solver, greedy)` insight class of each sub-party a corner leans on. A **win** sub-party ideally
/// reads `[I]` - the strategy is genuinely needed, because greedy play *without* it loses; a **lose** sub-party
/// ideally reads `[X]` - the wrong tool truly cannot win. The off-cases are the tuning signal: `[T]` on a win means
/// the corner is trivially winnable (it teaches nothing - greedy already wins); `[I]` on a lose means the wrong
/// tool can be *cheesed* with a non-obvious line though greedy fails (Greywater's old melee, before the Reaver).
fn insight_report(
    behavior: Behavior,
    kits: &[Combatant],
    melee: &[Combatant],
    ranged: &[Combatant],
    single: &[Combatant],
    area: &[Combatant],
    foes: &[Combatant],
) {
    let rows: Vec<(&str, &[Combatant], bool)> = match behavior {
        Behavior::Concentration => {
            vec![
                ("full", kits, true),
                ("single", single, true),
                ("area", area, false),
            ]
        }
        Behavior::Range => {
            vec![
                ("full", kits, true),
                ("ranged", ranged, true),
                ("melee", melee, false),
            ]
        }
        Behavior::Sweep => {
            vec![
                ("full", kits, true),
                ("area", area, true),
                ("single", single, false),
            ]
        }
        Behavior::Raid => vec![("full", kits, true)],
        Behavior::CombinedArms => vec![
            ("full", kits, true),
            ("melee", melee, false),
            ("ranged", ranged, false),
            ("single", single, false),
        ],
    };
    for (label, party, should_win) in rows {
        let c = insight_class(party, foes);
        let note = match (should_win, c) {
            (true, 'I') => "insight needed - good",
            (true, 'T') => "trivial - greedy already wins, lesson not forced",
            (true, _) => "IMPOSSIBLE - a sub-party that must win cannot!",
            (false, 'X') => "impossible - good",
            (false, 'I') => "cheesable - greedy loses, but an expert line wins",
            (false, _) => "TRIVIAL WIN - a control that must lose wins by greedy!",
        };
        let want = if should_win { "win " } else { "lose" };
        println!("    {label:<7} want {want} [{c}]  {note}");
    }
}

fn main() {
    let deep = std::env::args().any(|a| a == "scores" || a == "--scores");
    let insight = std::env::args().any(|a| a == "insight" || a == "--insight");
    println!("regions_diagonal - does the encounter set still teach what it is meant to?");
    println!(
        "(driving the pure `rules` crate through the generic Game + Solver{})\n",
        if deep {
            " -- deep report: +best routes, SLOW"
        } else {
            " -- fast gate"
        }
    );
    let t0 = Instant::now();
    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    let names: Vec<&str> = catalog::ROSTER.iter().map(|k| k.0).collect();
    // The two sub-parties: melee-only (Raider, Bastion) and ranged-only (Marksman, Bombardier).
    let melee: Vec<Combatant> = kits.iter().filter(|k| k.melee).cloned().collect();
    let ranged: Vec<Combatant> = kits
        .iter()
        .filter(|k| k.ranged && !k.melee)
        .cloned()
        .collect();
    // The single-target-only sub-party (no area strike): Raider + Marksman.
    let single: Vec<Combatant> = kits.iter().filter(|k| !k.aoe).cloned().collect();
    // The area sub-party (an area strike): Bastion + Bombardier.
    let area: Vec<Combatant> = kits.iter().filter(|k| k.aoe).cloned().collect();

    println!("SOLOS - each must be soloable by exactly ONE kit (its keystone's counter).\n");
    let mut solo_ok = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| !e.party) {
        let foes = foes_of(e);
        let want = catalog::creature(e.keystone)
            .map(catalog::creature_counter)
            .unwrap_or("");
        let winners: Vec<&str> = kits
            .iter()
            .zip(&names)
            .filter(|(k, _)| solver_wins::<Combat>(std::slice::from_ref(*k), &foes))
            .map(|(_, n)| *n)
            .collect();
        let verdict = if winners == vec![want] {
            solo_ok += 1;
            "OK".to_string()
        } else if winners.is_empty() {
            "TOO HARD - no kit solos it".to_string()
        } else if winners.len() > 1 {
            format!("TOO SOFT - {} kits solo it", winners.len())
        } else {
            format!("WRONG - {} solos it, want {want}", winners[0])
        };
        // In the deep report, also show how cleanly the intended counter kit solos it (best route, by priority).
        let score = if deep {
            let cs = kits
                .iter()
                .zip(&names)
                .find(|(_, n)| **n == want)
                .map(|(k, _)| best_score(std::slice::from_ref(k), &foes))
                .unwrap_or((None, true));
            format!("   {want} best {}", fmt_score(cs))
        } else {
            String::new()
        };
        println!(
            "  {:<20} {:<12} answer {want:<11} solos: {:<24} {verdict}{score}",
            e.location,
            e.keystone,
            format!("{winners:?}")
        );
    }
    let note = if deep {
        "   (best = downed/rounds/hp for the counter kit)"
    } else {
        ""
    };
    println!("\n  {solo_ok}/4 solos.{note}\n");

    println!("STRATEGY CORNERS + CAPSTONE - each must pass its assigned behavior.\n");
    let party_total = catalog::ENCOUNTERS.iter().filter(|e| e.party).count();
    let mut corner_ok = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = foes_of(e);
        let Some(behavior) = e.behavior else {
            println!("  {:<20} (no behavior assigned - skipped)", e.location);
            continue;
        };
        let verdict = match behavior_passes(behavior, &kits, &melee, &ranged, &single, &area, &foes)
        {
            Ok(()) => {
                corner_ok += 1;
                "OK".to_string()
            }
            Err(reason) => format!("FAIL - {reason}"),
        };
        // In the deep report, also show the full party's best route through the corner.
        let score = if deep {
            format!("   party best {}", fmt_score(best_score(&kits, &foes)))
        } else {
            String::new()
        };
        println!(
            "  {:<20} {:<18} {verdict}{score}",
            e.location,
            format!("{behavior:?}")
        );
    }
    let note = if deep {
        "   (best = downed/rounds/hp for the full party)"
    } else {
        ""
    };
    println!("\n  {corner_ok}/{party_total} party fights (4 corners + capstone).{note}");

    if insight {
        println!(
            "\nINSIGHT GRID - can each sub-party win with a SOLVER but not GREEDILY?\n  [I] insight needed (solver wins, greedy loses)   [T] trivial (greedy wins)   [X] impossible (neither)\n"
        );
        for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
            if let Some(behavior) = e.behavior {
                let foes = foes_of(e);
                println!("  {} ({behavior:?})", e.location);
                insight_report(behavior, &kits, &melee, &ranged, &single, &area, &foes);
            }
        }
    }

    println!(
        "\nSCORE: {solo_ok}/4 solos, {corner_ok}/{party_total} party fights   ({} ms)",
        t0.elapsed().as_millis()
    );
}
