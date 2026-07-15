//! **Tune the corners: the smallest warband that passes its assigned behavior** - over the `rules` crate.
//!
//! The four solos lock the creature stats (a solo is one creature), so the only corner lever is the
//! **composition**: which creatures, how many. That is also the low-numbers lever - it raises counts, not stats.
//! Compositions are walked smallest-first, so whatever comes back is the fewest bodies that teach the lesson.
//!
//! Each corner carries a [`Behavior`] - the lesson it is built to teach. This searches, per corner, for the
//! smallest warband that PASSES that behavior's necessity test (see [`behavior_passes`]). It drives
//! [`rules::combat`] through the generic [`Solver`]; it changes no files, it prints what to write.
//!
//! Run: `cargo run --release -p deckbound-board --example regions_tune_corners`

use std::time::Instant;

use deckbound_board::units::{beast, kit};
use deckbound_content::catalog::{self, Behavior, Creature};
use rules::combat::game::{ClashOnly, Combat, Scattered, State};
use rules::combat::resolve::Combatant;
use rules::core::{Game, Solvable, Solver, Verdict};

/// Stop doubling the node grant past this ceiling. A warband we cannot decisively settle within it is treated as
/// **not cleanly winnable** (and so it fails its behavior test), which keeps the search total and bounded - no
/// unbounded hang while the tuner scans hundreds of candidate warbands. Held well below the diagonal's 20M: the
/// tuner runs the solver on hundreds of candidates - and one keystone (the Storm, a 12-body horde) yields very
/// deep trees - so a low per-candidate ceiling is what keeps the whole scan bounded to a few minutes. A candidate
/// whose full-party win we cannot prove within it is simply skipped (the safe direction: a warband we cannot
/// decisively solve is not one to lean a lesson on). The cap hits the setup-branching `Combat` search, not the
/// forced-setup `Scattered` control, so lowering it turns unprovable warbands into rejections, not false passes.
///
/// NOTE: 30k, not the diagonal's 20M. The Storm corner (ScreenNecessary) scans many deep 12-body-horde warbands
/// whose nodes are individually expensive (a full round played over a 12-strong horde), so even 150k ran well
/// past 5 minutes. The cap is pulled down here to keep the whole scan bounded to a couple of minutes. A warband
/// whose full-party win needs more than 30k nodes to prove is skipped rather than waited on - the safe direction
/// for a gate, and noted so a proposal reads as "smallest we could decisively prove within the cap", not
/// "smallest that exists". (The three cheap corners settle far under this, so their proposals are unchanged.)
const GRANT_CAP: u64 = 30_000;

/// Can these heroes win, given their best formation, under game `G`? (The `Combat` game searches the formation
/// itself.) Ground out with an escalating grant up to [`GRANT_CAP`]; past the cap a still-`Evaluating` position
/// is called NOT winnable.
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

/// **Does this warband pass `behavior`?** Every behavior first requires the full party to win under `Combat`;
/// then its own necessity test (a sub-party wins/loses, or a control loses). Mirrors the diagonal's scorer.
fn behavior_passes(
    behavior: Behavior,
    kits: &[Combatant],
    melee: &[Combatant],
    ranged: &[Combatant],
    foes: &[Combatant],
) -> bool {
    if !winnable::<Combat>(kits, foes) {
        return false;
    }
    match behavior {
        Behavior::VanguardCarries => {
            winnable::<Combat>(melee, foes) && !winnable::<Combat>(ranged, foes)
        }
        Behavior::RearguardCarries => {
            winnable::<Combat>(ranged, foes) && !winnable::<Combat>(melee, foes)
        }
        Behavior::RaidNecessary => !winnable::<ClashOnly>(kits, foes),
        Behavior::ScreenNecessary => !winnable::<Scattered>(kits, foes),
        Behavior::CombinedArms => {
            !winnable::<Combat>(melee, foes)
                && !winnable::<Combat>(ranged, foes)
                && !winnable::<ClashOnly>(kits, foes)
        }
    }
}

/// Every warband worth trying, smallest first, keystone always present.
fn warbands(keystone: usize, n: usize, max_bodies: u32) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = Vec::new();
    fn walk(
        k: usize,
        n: usize,
        c: &mut Vec<u32>,
        keystone: usize,
        cap: u32,
        out: &mut Vec<Vec<u32>>,
    ) {
        if k == n {
            let total: u32 = c.iter().sum();
            if c[keystone] >= 1 && (2..=cap).contains(&total) {
                out.push(c.clone());
            }
            return;
        }
        for q in 0..=4u32 {
            c[k] = q;
            walk(k + 1, n, c, keystone, cap, out);
        }
        c[k] = 0;
    }
    walk(0, n, &mut vec![0; n], keystone, max_bodies, &mut out);
    out.sort_by_key(|c| (c.iter().sum::<u32>(), c.iter().filter(|&&q| q > 0).count()));
    out
}

fn main() {
    println!("regions_tune_corners - the smallest warband that passes its assigned behavior\n");
    let t0 = Instant::now();
    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    // The two sub-parties: melee-only (Raider, Bastion) and ranged-only (Marksman, Bombardier).
    let melee: Vec<Combatant> = kits.iter().filter(|k| k.melee).cloned().collect();
    let ranged: Vec<Combatant> = kits
        .iter()
        .filter(|k| k.ranged && !k.melee)
        .cloned()
        .collect();
    let creatures: Vec<&Creature> = catalog::CREATURES.iter().collect();

    let mut solved = 0;
    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let Some(behavior) = e.behavior else {
            continue;
        };
        let keystone = creatures
            .iter()
            .position(|c| c.name == e.keystone)
            .expect("keystone");
        println!(
            "{} - behavior {behavior:?} (keystone {})",
            e.location, e.keystone
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
        eprintln!("  [searching {} - {behavior:?}]", e.location);

        let mut found = None;
        let mut tried = 0;
        for counts in warbands(keystone, creatures.len(), 6) {
            let foes: Vec<Combatant> = creatures
                .iter()
                .zip(&counts)
                .flat_map(|(c, &q)| std::iter::repeat_n(beast(c), q as usize))
                .collect();
            tried += 1;
            if behavior_passes(behavior, &kits, &melee, &ranged, &foes) {
                found = Some((counts, foes.len()));
                break;
            }
        }
        match found {
            Some((counts, bodies)) => {
                solved += 1;
                let spec: Vec<String> = creatures
                    .iter()
                    .zip(counts.iter())
                    .filter(|&(_, &q)| q > 0)
                    .map(|(c, &q)| format!("(\"{}\", {q})", c.name))
                    .collect();
                println!(
                    "      FOUND after {tried} tries, {bodies} bodies:  foes: &[{}],",
                    spec.join(", ")
                );
            }
            None => println!("      NOTHING WORKS in {tried} warbands of up to 6 bodies."),
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }
    println!(
        "\n{solved}/4 corners solved   ({} ms)",
        t0.elapsed().as_millis()
    );
}
