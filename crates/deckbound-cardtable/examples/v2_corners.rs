//! **Corner encounters — each relying heavily on one kit.** The four kits are the reach x spread diagonal
//! solution (see `v2_diagonal_search`). A *corner* for kit `x` is an encounter that:
//!   - the full 4-kit party WINS,
//!   - the party WITHOUT `x` LOSES (x is required),
//!   - `x` ALONE cannot solo (it's a corner, not an adjacent - the party is needed),
//!   - dropping any OTHER kit still wins (x is the *unique* linchpin).
//! Score is those 6 conditions (max 6). Coordinate descent over a 2-creature encounter, seeded from the kit's
//! own adjacent creature (scaled up) plus a minor. spec-4.6 mechanics.
//!
//! Run: `cargo run --release -p deckbound-cardtable --example v2_corners`

use deckbound::actor::Intention as Rank;
use deckbound_cardtable::combat::{Combatant, Side};
use deckbound_cardtable::solver::winnable;

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

/// The 6-condition score for a corner relying on kit `x`.
fn score(x: usize, enc: &[Foe]) -> u32 {
    let e = encounter(enc);
    let all = [0usize, 1, 2, 3];
    let without =
        |drop: usize| -> Vec<usize> { all.iter().copied().filter(|&i| i != drop).collect() };
    let mut s = 0;
    if wins(&all, &e) {
        s += 1; // full party wins
    }
    if !wins(&without(x), &e) {
        s += 1; // needs x
    }
    if !wins(&[x], &e) {
        s += 1; // x alone can't solo it
    }
    for y in 0..4 {
        if y != x && wins(&without(y), &e) {
            s += 1; // dropping any other kit still wins
        }
    }
    s
}

const STAT_LO: [u8; 5] = [1, 1, 1, 1, 1];
const STAT_HI: [u8; 5] = [6, 16, 12, 2, 4]; // Cadence <= 2, Vitality <= 16 to keep the solver quick
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
                let cur = if field == 0 {
                    enc[j].ranged
                } else {
                    enc[j].horde
                };
                let mut bb = cur;
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
        if score(x, &enc) == start {
            return enc;
        }
    }
}

/// Seed for corner `x`: the kit's own adjacent creature (scaled a bit) + a weak minor of a different type.
fn seed(x: usize) -> Vec<Foe> {
    let v = Rank::Vanguard;
    let adj = match x {
        0 => Foe {
            stats: [1, 8, 12, 1, 2],
            rank: v,
            ranged: false,
            horde: false,
        }, // scaled Wall
        1 => Foe {
            stats: [6, 8, 1, 2, 2],
            rank: v,
            ranged: false,
            horde: false,
        }, // scaled Duelist
        2 => Foe {
            stats: [1, 14, 1, 1, 1],
            rank: Rank::Rearguard,
            ranged: true,
            horde: true,
        }, // Swarm
        _ => Foe {
            stats: [3, 16, 1, 2, 1],
            rank: v,
            ranged: false,
            horde: true,
        }, // scaled Storm
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
    for x in 0..4 {
        let best = climb(x, seed(x));
        let sc = score(x, &best);
        println!("== corner relying on {} : {sc}/6 ==", KIT_NAMES[x]);
        report(x, &best);
        println!();
    }
}

fn rankc(r: Rank) -> &'static str {
    match r {
        Rank::Vanguard => "V",
        Rank::Outrider => "O",
        Rank::Rearguard => "R",
    }
}

fn report(x: usize, enc: &[Foe]) {
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
    let e = encounter(enc);
    let all = [0usize, 1, 2, 3];
    let without =
        |drop: usize| -> Vec<usize> { all.iter().copied().filter(|&i| i != drop).collect() };
    println!("  full party: {}", yn(wins(&all, &e)));
    println!("  {} alone: {}", KIT_NAMES[x], yn(wins(&[x], &e)));
    for y in 0..4 {
        println!("  without {}: {}", KIT_NAMES[y], yn(wins(&without(y), &e)));
    }
}

fn yn(b: bool) -> &'static str {
    if b { "WIN" } else { "lose" }
}
