//! **The balance diagonal, over the `rules` crate.** Does the encounter set still teach what it is meant to?
//!
//! This is the verification that the pure `rules` port reproduces the balance work: it drives
//! [`rules::combat`] (the regions model, behind the generic `Game`) via the generic [`Solver`], instead of the
//! old `deckbound_board::regions` copy.
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
use deckbound_content::catalog::{self, Behavior, Encounter};
use rules::combat::game::{ClashOnly, Combat, Score, Scorer, State};
use rules::combat::resolve::Combatant;
use rules::core::{Game, Solvable, Solver, Verdict};

/// Stop doubling the node grant past this ceiling. A position we cannot decisively settle within it is treated
/// as **not cleanly winnable** - a warband we cannot solve is not one to lean a lesson on. This makes the search
/// total and bounded (no unbounded hang), at the cost of calling a genuinely-winnable-but-enormous position a
/// loss, which is the safe direction for a balance gate.
const GRANT_CAP: u64 = 20_000_000;

fn foes_of(e: &Encounter) -> Vec<Combatant> {
    let mut out = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    out
}

/// **Can these heroes win under game `G`?** `G` is `Combat` or `ClashOnly` - a control is the same heroes under a
/// different Game. There is no setup: the party stands on region 0, the foes on region 1, and the solver searches
/// the rounds with one shared memo across the tree. The verdict is ground out with an escalating grant (doubling
/// on `Evaluating`) up to [`GRANT_CAP`]; past the cap a still-`Evaluating` position is called NOT winnable.
fn winnable<G>(heroes: &[Combatant], foes: &[Combatant]) -> bool
where
    G: Solvable + Game<State = State>,
{
    let mut units: Vec<Combatant> = heroes.to_vec();
    units.extend_from_slice(foes);
    let s = State::new(units);

    let mut o = Solver::<G>::new();
    let mut grant = 1u64;
    loop {
        o.grant(grant);
        match o.verdict(&s) {
            Verdict::Winnable => return true,
            Verdict::Doomed => return false,
            Verdict::Evaluating => {
                if grant >= GRANT_CAP {
                    eprintln!(
                        "  [cap] still Evaluating at {GRANT_CAP} nodes - treating as NOT winnable"
                    );
                    return false;
                }
                grant = grant.saturating_mul(2);
            }
        }
    }
}

/// **The best route these heroes can take to a win** under the priority order (win > fewest downed > fewest
/// rounds > least hp), or `None` if they cannot win. Informative only - it does **not** gate the diagonal, it just
/// says *how cleanly* the win comes. Budgeted with the same escalating grant as [`winnable`]; at the cap the
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
/// - `VanguardCarries`: melee-only wins, ranged-only loses.
/// - `RearguardCarries`: ranged-only wins, melee-only loses.
/// - `RaidNecessary`: the full party under `ClashOnly` loses (the raid was load-bearing).
/// - `CombinedArms`: melee-only loses, ranged-only loses, AND the full party under `ClashOnly` loses (the
///   whole toolkit - ranged, melee, and the raid - is load-bearing at once).
fn behavior_passes(
    behavior: Behavior,
    kits: &[Combatant],
    melee: &[Combatant],
    ranged: &[Combatant],
    foes: &[Combatant],
) -> Result<(), String> {
    if !winnable::<Combat>(kits, foes) {
        return Err("full party loses under Combat".to_string());
    }
    match behavior {
        Behavior::VanguardCarries => {
            if !winnable::<Combat>(melee, foes) {
                return Err("melee-only party loses (the vanguard should carry it)".to_string());
            }
            if winnable::<Combat>(ranged, foes) {
                return Err("ranged-only party wins (the vanguard is not load-bearing)".to_string());
            }
        }
        Behavior::RearguardCarries => {
            if !winnable::<Combat>(ranged, foes) {
                return Err("ranged-only party loses (the rearguard should carry it)".to_string());
            }
            if winnable::<Combat>(melee, foes) {
                return Err("melee-only party wins (the rearguard is not load-bearing)".to_string());
            }
        }
        Behavior::RaidNecessary => {
            if winnable::<ClashOnly>(kits, foes) {
                return Err(
                    "full party wins under ClashOnly (the raid is not necessary)".to_string(),
                );
            }
        }
        Behavior::CombinedArms => {
            if winnable::<Combat>(melee, foes) {
                return Err("melee-only party wins (ranged damage is not necessary)".to_string());
            }
            if winnable::<Combat>(ranged, foes) {
                return Err("ranged-only party wins (melee damage is not necessary)".to_string());
            }
            if winnable::<ClashOnly>(kits, foes) {
                return Err(
                    "full party wins under ClashOnly (the raid is not necessary)".to_string(),
                );
            }
        }
    }
    Ok(())
}

fn main() {
    let deep = std::env::args().any(|a| a == "scores" || a == "--scores");
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
            .filter(|(k, _)| winnable::<Combat>(std::slice::from_ref(*k), &foes))
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

    println!("CORNERS - each must pass its assigned behavior.\n");
    let mut corner_ok = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let foes = foes_of(e);
        let Some(behavior) = e.behavior else {
            println!("  {:<20} (no behavior assigned - skipped)", e.location);
            continue;
        };
        let verdict = match behavior_passes(behavior, &kits, &melee, &ranged, &foes) {
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
    println!("\n  {corner_ok}/4 corners.{note}");
    println!(
        "\nSCORE: {solo_ok}/4 solos, {corner_ok}/4 corners   ({} ms)",
        t0.elapsed().as_millis()
    );
}
