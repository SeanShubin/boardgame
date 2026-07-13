//! **Corner encounters — each relying on one kit, tested by swap-out.** The four kits are the reach x spread
//! diagonal solution. A *corner* for kit `x` must:
//!   - be BEATEN by the full 4-kit party, and
//!   - be beaten by NO 4-unit party drawn from the other three kits (every "swap x out for a duplicate of
//!     another kit" fails) - so it is x's *capability* that is required, not just a fourth body.
//! We also keep the numbers LOW: stats are capped, and among designs with the same win/lose pattern the
//! search prefers the smaller stat total, and each corner reports its largest stat so an "only works at
//! extremes" result is visible. spec-4.6 mechanics.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_corners [corner_index]`
//! (with no arg: all four corners; with 0..3: just that one, for a quick timing check.)

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::solver::winnable;
use deckbound_content::rank::Intention as Rank;

const KIT_NAMES: [&str; 4] = ["Jab", "Shot", "Sweep", "Salvo"];
const KIT_STATS: [[u8; 5]; 4] = [
    [7, 6, 1, 2, 2],
    [5, 2, 1, 2, 2],
    [1, 3, 3, 1, 2],
    [3, 3, 1, 1, 2],
];
const KIT_SHAPE: [(bool, bool, bool); 4] = [
    (true, false, false),
    (false, true, false),
    (true, false, true),
    (false, true, true),
];

fn kit(i: usize) -> Combatant {
    let (m, r, a) = KIT_SHAPE[i];
    Combatant::from_stats(
        KIT_NAMES[i],
        Side::Party,
        Rank::Vanguard,
        KIT_STATS[i],
        0,
        m,
        r,
    )
    .with_aoe(a)
}
fn party(mask: &[usize]) -> Vec<Combatant> {
    mask.iter().map(|&i| kit(i)).collect()
}

#[derive(Clone)]
struct Foe {
    stats: [u8; 5],
    rank: Rank,
    ranged: bool,
    horde: bool,
}
fn foe_unit(f: &Foe, idx: usize) -> Combatant {
    Combatant::from_stats(
        format!("F{idx}"),
        Side::Foe,
        f.rank,
        f.stats,
        0,
        !f.ranged,
        f.ranged,
    )
    .as_horde(f.horde)
}
fn encounter(fs: &[Foe]) -> Vec<Combatant> {
    fs.iter().enumerate().map(|(i, f)| foe_unit(f, i)).collect()
}
fn wins(mask: &[usize], e: &[Combatant]) -> bool {
    winnable(&party(mask), e)
}

/// All non-decreasing length-`size` multisets over `types` (every party of that size from those kits).
fn multisets(types: &[usize], size: usize) -> Vec<Vec<usize>> {
    fn rec(
        t: &[usize],
        size: usize,
        start: usize,
        cur: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if cur.len() == size {
            out.push(cur.clone());
            return;
        }
        for i in start..t.len() {
            cur.push(t[i]);
            rec(t, size, i, cur, out);
            cur.pop();
        }
    }
    let mut out = vec![];
    rec(types, size, 0, &mut vec![], &mut out);
    out
}

/// The x-less 4-unit parties: every 4-unit multiset over the three kits that are not `x`.
fn swap_parties(x: usize) -> Vec<Vec<usize>> {
    let non_x: Vec<usize> = (0..4).filter(|&i| i != x).collect();
    multisets(&non_x, 4)
}

fn stat_sum(enc: &[Foe]) -> u32 {
    enc.iter()
        .flat_map(|f| f.stats.iter().map(|&s| s as u32))
        .sum()
}

/// Condition count: +1 the full party wins, +1 for every x-less 4-unit party that LOSES. Tie-broken toward
/// low numbers (a small penalty on the stat total). Max conditions = 1 + swap_parties(x).len().
fn score(x: usize, enc: &[Foe]) -> f64 {
    let e = encounter(enc);
    let mut conds = 0u32;
    if wins(&[0, 1, 2, 3], &e) {
        conds += 1;
    }
    for m in swap_parties(x) {
        if !wins(&m, &e) {
            conds += 1;
        }
    }
    conds as f64 - 0.0005 * stat_sum(enc) as f64
}

const STAT_LO: [u8; 5] = [1, 1, 1, 1, 1];
// Low caps so an "only works at extremes" outcome is visible (the search cannot reach for huge numbers).
const STAT_HI: [u8; 5] = [6, 14, 10, 2, 4];
const RANKS: [Rank; 3] = [Rank::Vanguard, Rank::Outrider, Rank::Rearguard];

fn climb(x: usize, mut enc: Vec<Foe>) -> Vec<Foe> {
    loop {
        let start = score(x, &enc);
        for j in 0..enc.len() {
            for s in 0..5 {
                let (mut bv, mut best) = (enc[j].stats[s], score(x, &enc));
                for v in STAT_LO[s]..=STAT_HI[s] {
                    enc[j].stats[s] = v;
                    let sc = score(x, &enc);
                    if sc > best {
                        best = sc;
                        bv = v;
                    }
                }
                enc[j].stats[s] = bv;
            }
            let (mut br, mut best) = (enc[j].rank, score(x, &enc));
            for r in RANKS {
                enc[j].rank = r;
                let sc = score(x, &enc);
                if sc > best {
                    best = sc;
                    br = r;
                }
            }
            enc[j].rank = br;
            for field in 0..2 {
                let mut bb = if field == 0 {
                    enc[j].ranged
                } else {
                    enc[j].horde
                };
                let mut best = score(x, &enc);
                for b in [false, true] {
                    if field == 0 {
                        enc[j].ranged = b;
                    } else {
                        enc[j].horde = b;
                    }
                    let sc = score(x, &enc);
                    if sc > best {
                        best = sc;
                        bb = b;
                    }
                }
                if field == 0 {
                    enc[j].ranged = bb;
                } else {
                    enc[j].horde = bb;
                }
            }
        }
        if score(x, &enc) <= start {
            return enc;
        }
    }
}

/// Seed for corner `x`: the kit's own adjacent creature (scaled a touch) + a weak minor.
fn seed(x: usize) -> Vec<Foe> {
    let v = Rank::Vanguard;
    let adj = match x {
        0 => Foe {
            stats: [1, 6, 9, 1, 2],
            rank: v,
            ranged: false,
            horde: false,
        },
        1 => Foe {
            stats: [6, 8, 1, 2, 2],
            rank: v,
            ranged: false,
            horde: false,
        },
        2 => Foe {
            stats: [1, 12, 1, 1, 1],
            rank: Rank::Rearguard,
            ranged: true,
            horde: true,
        },
        _ => Foe {
            stats: [3, 12, 1, 2, 1],
            rank: v,
            ranged: false,
            horde: true,
        },
    };
    let minor = Foe {
        stats: [2, 4, 1, 1, 2],
        rank: v,
        ranged: false,
        horde: false,
    };
    vec![adj, minor]
}

fn main() {
    // Direct check (no search - the swap-out search is too slow): report each corner's seed encounter, then
    // sweep the Jab wall's Grit to find where a doubled substitute (2x Shot) stops cracking it.
    println!("--- swap-out breakdown on each corner's seed encounter ---\n");
    for x in 0..4 {
        report(x, &seed(x));
        println!();
    }

    println!(
        "--- Jab wall: does a single Jab beat what a DOUBLED substitute (2x Shot) cannot? ---"
    );
    println!("(wall + a minor; full party has Jab, the swap party trades Jab for a 2nd Shot)\n");
    println!(
        "{:<10}{:>16}{:>22}",
        "wall Tough", "full (has Jab)", "2x Shot (no Jab)"
    );
    for tough in [8u8, 10, 12, 15, 18] {
        let enc = vec![
            Foe {
                stats: [1, 6, tough, 1, 2],
                rank: Rank::Vanguard,
                ranged: false,
                horde: false,
            },
            Foe {
                stats: [2, 4, 1, 1, 2],
                rank: Rank::Vanguard,
                ranged: false,
                horde: false,
            },
        ];
        let e = encounter(&enc);
        let full = wins(&[0, 1, 2, 3], &e);
        let two_shot = wins(&[1, 1, 2, 3], &e); // Shot, Shot, Sweep, Salvo - Jab swapped for a 2nd Shot
        println!("{:<10}{:>16}{:>22}", tough, yn(full), yn(two_shot));
    }
    let _ = climb; // keep the search available for reference
}

fn rankc(r: Rank) -> &'static str {
    match r {
        Rank::Vanguard => "V",
        Rank::Outrider => "O",
        Rank::Rearguard => "R",
    }
}
fn names(m: &[usize]) -> String {
    m.iter()
        .map(|&i| KIT_NAMES[i])
        .collect::<Vec<_>>()
        .join("+")
}

fn report(x: usize, enc: &[Foe]) {
    let swaps = swap_parties(x);
    let e = encounter(enc);
    let full = wins(&[0, 1, 2, 3], &e);
    let losing = swaps.iter().filter(|m| !wins(m, &e)).count();
    let conds = full as usize + losing;
    let maxstat = enc
        .iter()
        .flat_map(|f| f.stats.iter().copied())
        .max()
        .unwrap_or(0);
    println!(
        "== corner relying on {} : {conds}/{} conditions (max stat {maxstat}) ==",
        KIT_NAMES[x],
        1 + swaps.len()
    );
    for (i, f) in enc.iter().enumerate() {
        let reach = if f.horde {
            "horde"
        } else if f.ranged {
            "ranged"
        } else {
            "melee"
        };
        println!("  creature {i}: {:?} {}/{reach}", f.stats, rankc(f.rank));
    }
    println!("  full party (Jab+Shot+Sweep+Salvo): {}", yn(full));
    println!("  x-less 4-unit parties that still WIN (want none):");
    let mut any = false;
    for m in &swaps {
        if wins(m, &e) {
            println!("    {} : WIN", names(m));
            any = true;
        }
    }
    if !any {
        println!("    (none - the corner needs {})", KIT_NAMES[x]);
    }
}

fn yn(b: bool) -> &'static str {
    if b { "WIN" } else { "lose" }
}
