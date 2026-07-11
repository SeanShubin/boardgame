//! **Automated co-tuning search: can the clean 4x4 identity diagonal hold with NO armor / NO riposte?**
//!
//! Each of the four creatures should be soloable by *exactly* its designated kit (Anvil->Executioner,
//! Swarm->Broadsider, Coil->Marksman, Mirage->Phantom) and by no other - a 16-cell identity matrix. By hand
//! we reach 15/16 (only Mirage->Phantom resists). This searches kit + creature stat lines (and each
//! creature's rank + reach) by coordinate descent with seeded random restarts, scoring by correct cells. If
//! it finds 16/16, current mechanics suffice; if it plateaus at 15/16 across many restarts, the evasion gate
//! is the one that needs a new mechanic (armor).
//!
//! Deterministic (seeded splitmix64) so the result reproduces. Run:
//!   `cargo run --release -p deckbound-cardtable --example v2_diagonal_search`

use deckbound::actor::Intention as Rank;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

// The 4 kits ARE the four reach x spread combos: Jab (melee single), Shot (ranged single), Sweep (melee
// area), Salvo (ranged area). Each should be uniquely useful along its hit type.
const KIT_NAMES: [&str; 4] = ["Jab", "Shot", "Sweep", "Salvo"];
/// The 4 creatures as the matching 2x2: {single, horde} x {safe-to-melee, punishes-melee}.
///  Wall   = single + safe    -> Jab   (concentrate to crack Toughness)
///  Duelist= single + punish  -> Shot   (crack it from range; melee trades and dies)
///  Swarm  = horde  + safe    -> Sweep  (area up front)
///  Storm  = horde  + punish  -> Salvo  (area from safety; melee gets torn up)
const FOE_NAMES: [&str; 4] = ["Wall", "Duelist", "Swarm", "Storm"];
/// Kit reach x spread `(melee, ranged, aoe)`: Jab, Shot, Sweep, Salvo.
const KIT_SHAPE: [(bool, bool, bool); 4] = [
    (true, false, false),
    (false, true, false),
    (true, false, true),
    (false, true, true),
];
/// Swarm and Storm are hordes (both want an area answer); the split between them is melee-danger.
const FOE_HORDE: [bool; 4] = [false, false, true, true];

const STAT_LO: [u8; 5] = [1, 2, 1, 1, 1]; // Might, Vitality, Toughness, Cadence, Finesse
const STAT_HI: [u8; 5] = [8, 24, 12, 3, 6]; // Cadence capped at 3 to keep the solver quick

#[derive(Clone)]
struct Design {
    kits: [[u8; 5]; 4],
    foes: [[u8; 5]; 4],
    foe_rank: [Rank; 4],
    foe_ranged: [bool; 4], // a non-horde creature is melee or ranged
}

fn kit_unit(d: &Design, i: usize) -> Combatant {
    let (melee, ranged, aoe) = KIT_SHAPE[i];
    Combatant::from_stats(
        KIT_NAMES[i],
        Side::Party,
        Rank::Vanguard,
        d.kits[i],
        0,
        melee,
        ranged,
    )
    .with_aoe(aoe)
}

fn foe_unit(d: &Design, j: usize) -> Combatant {
    let ranged = !FOE_HORDE[j] && d.foe_ranged[j];
    Combatant::from_stats(
        FOE_NAMES[j],
        Side::Foe,
        d.foe_rank[j],
        d.foes[j],
        0,
        !ranged,
        ranged,
    )
    .as_horde(FOE_HORDE[j])
}

/// One cell: does kit `i` solo creature `j`? (the solver picks the kit's rank).
fn beats(d: &Design, i: usize, j: usize) -> bool {
    winnable(
        std::slice::from_ref(&kit_unit(d, i)),
        std::slice::from_ref(&foe_unit(d, j)),
    )
}

/// Correct cells out of 16: a cell is right when `beats == (i == j)`.
fn score(d: &Design) -> u32 {
    let mut s = 0;
    for i in 0..4 {
        for j in 0..4 {
            if beats(d, i, j) == (i == j) {
                s += 1;
            }
        }
    }
    s
}

// ---- seeded RNG (splitmix64) for restart jitter; deterministic ------------------------------------
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn range(&mut self, lo: u8, hi: u8) -> u8 {
        lo + (self.next() % (hi - lo + 1) as u64) as u8
    }
}

const RANKS: [Rank; 3] = [Rank::Vanguard, Rank::Outrider, Rank::Rearguard];

/// Greedy coordinate descent: line-search each variable over its whole domain, keep the best; repeat sweeps
/// until a full sweep makes no gain. Returns the local optimum reached from `d`.
fn climb(mut d: Design) -> Design {
    loop {
        let start = score(&d);
        // Kit stats.
        for i in 0..4 {
            for s in 0..5 {
                let (mut best_v, mut best) = (d.kits[i][s], score(&d));
                for v in STAT_LO[s]..=STAT_HI[s] {
                    d.kits[i][s] = v;
                    let sc = score(&d);
                    if sc > best {
                        best = sc;
                        best_v = v;
                    }
                }
                d.kits[i][s] = best_v;
            }
        }
        // Foe stats.
        for j in 0..4 {
            for s in 0..5 {
                let (mut best_v, mut best) = (d.foes[j][s], score(&d));
                for v in STAT_LO[s]..=STAT_HI[s] {
                    d.foes[j][s] = v;
                    let sc = score(&d);
                    if sc > best {
                        best = sc;
                        best_v = v;
                    }
                }
                d.foes[j][s] = best_v;
            }
            // Foe rank + reach (skip the horde: Swarm stays an Outrider melee pack).
            if !FOE_HORDE[j] {
                let (mut best_r, mut best) = (d.foe_rank[j], score(&d));
                for r in RANKS {
                    d.foe_rank[j] = r;
                    let sc = score(&d);
                    if sc > best {
                        best = sc;
                        best_r = r;
                    }
                }
                d.foe_rank[j] = best_r;
                let (mut best_g, mut best) = (d.foe_ranged[j], score(&d));
                for g in [false, true] {
                    d.foe_ranged[j] = g;
                    let sc = score(&d);
                    if sc > best {
                        best = sc;
                        best_g = g;
                    }
                }
                d.foe_ranged[j] = best_g;
            }
        }
        if score(&d) == start {
            return d; // no improvement this sweep
        }
    }
}

fn seed_design() -> Design {
    // A reasonable start expressing the compensating-stats idea: Jab (worst combo) highest stats, Salvo
    // (best combo) lowest. The search co-tunes from here.
    Design {
        kits: [
            [7, 6, 1, 2, 2],
            [5, 4, 1, 2, 2],
            [4, 5, 2, 1, 2],
            [2, 3, 1, 1, 2],
        ],
        foes: [
            [1, 4, 9, 1, 2],
            [5, 5, 1, 2, 2],
            [1, 12, 1, 1, 1],
            [3, 12, 1, 2, 1],
        ],
        foe_rank: [
            Rank::Vanguard,
            Rank::Vanguard,
            Rank::Outrider,
            Rank::Outrider,
        ],
        foe_ranged: [false, false, false, false],
    }
}

fn jitter(base: &Design, rng: &mut Rng) -> Design {
    let mut d = base.clone();
    for i in 0..4 {
        for s in 0..5 {
            if rng.next() % 2 == 0 {
                d.kits[i][s] = rng.range(STAT_LO[s], STAT_HI[s]);
            }
        }
    }
    for j in 0..4 {
        for s in 0..5 {
            if rng.next() % 2 == 0 {
                d.foes[j][s] = rng.range(STAT_LO[s], STAT_HI[s]);
            }
        }
        if !FOE_HORDE[j] {
            d.foe_rank[j] = RANKS[(rng.next() % 3) as usize];
            d.foe_ranged[j] = rng.next() % 2 == 0;
        }
    }
    d
}

fn main() {
    let restarts: u32 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(60);

    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let mut best = seed_design();
    let mut best_score = score(&climb(best.clone()));
    best = climb(best);
    println!("seed (hand-tuned) climbs to {best_score}/16");

    for r in 0..restarts {
        // Restart 0 refines the seed; the rest jitter off the best so far.
        let start = if r == 0 {
            best.clone()
        } else {
            jitter(&best, &mut rng)
        };
        let cand = climb(start);
        let sc = score(&cand);
        if sc > best_score {
            best_score = sc;
            best = cand;
            println!("  restart {r}: new best {best_score}/16");
            if best_score == 16 {
                break;
            }
        }
    }

    println!("\n=== best found: {best_score}/16 ===\n");
    report(&best);
    println!(
        "\n{}",
        if best_score == 16 {
            "CLEAN IDENTITY DIAGONAL with NO armor / NO riposte - the mechanics suffice."
        } else {
            "Could not reach 16/16 - the remaining wrong cell(s) are where a mechanic is missing."
        }
    );
}

fn report(d: &Design) {
    print!("{:<14}", "kit \\ foe");
    for j in 0..4 {
        print!("{:>10}", FOE_NAMES[j]);
    }
    println!();
    for i in 0..4 {
        print!("{:<14}", KIT_NAMES[i]);
        for j in 0..4 {
            let w = beats(d, i, j);
            let ok = w == (i == j);
            print!(
                "{:>10}",
                format!(
                    "{}{}",
                    if w { "WIN" } else { "--" },
                    if ok { "" } else { "!" }
                )
            );
        }
        println!();
    }
    println!("\n(a trailing ! marks a wrong cell)\n");
    let rank = |r: Rank| match r {
        Rank::Vanguard => "V",
        Rank::Outrider => "O",
        Rank::Rearguard => "R",
    };
    println!("kits  [Might,Vit,Tough,Cad,Fin] + attack:");
    for i in 0..4 {
        let (m, r, a) = KIT_SHAPE[i];
        let shape = if a {
            "area"
        } else if r {
            "ranged"
        } else if m {
            "melee"
        } else {
            "?"
        };
        println!("  {:<12} {:?}  {shape}", KIT_NAMES[i], d.kits[i]);
    }
    println!("foes  [Might,Vit,Tough,Cad,Fin] + rank/reach:");
    for j in 0..4 {
        let reach = if FOE_HORDE[j] {
            "horde"
        } else if d.foe_ranged[j] {
            "ranged"
        } else {
            "melee"
        };
        println!(
            "  {:<12} {:?}  {}/{reach}",
            FOE_NAMES[j],
            d.foes[j],
            rank(d.foe_rank[j])
        );
    }
}
