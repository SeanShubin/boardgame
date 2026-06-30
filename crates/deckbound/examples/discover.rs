//! **Class-discovery sweep** — generate-and-test over the capability matrix to see whether a *balanced
//! RPS ecology* of classes emerges, and to **price the area capability**.
//!
//! A "class" is a generic, suit-free body: a **strike card** (melee/ranged × single/aoe, §4.3
//! capabilities-as-cards) plus a five-stat allocation. Its **role emerges** from range+stats. Capability
//! *budgeting*: **range is free** (melee↔ranged is a positional tradeoff, priced structurally by §4.2, not
//! by points), but **area costs `K` of the budget** — AoE hits the whole enemy group, unevadable, past the
//! bodyguard, so at equal stats it is *weakly dominant*; charging it `K` points (an AoE class gets `8−K`
//! stat points) restores a real tradeoff (better vs clusters, worse vs a lone tough wall).
//!
//! Combat is **NvN** (each class fielded as a grouped party of [`PARTY`]), on the **real engine** — AoE's
//! advantage only exists against a group, so a 1v1 sweep cannot price it. The sweep **scans `K`**: at each
//! cost it round-robins the field (both side-assignments, to cancel side bias) and reports whether AoE
//! still dominates (mean win-rate of the AoE cells vs the single cells) and whether a non-transitive RPS
//! cycle spanning distinct cells/roles exists. The smallest `K` that levels AoE↔single is the price.
//!
//! Run: `cargo run -p deckbound --example discover`

use deckbound::balance::compositions_k;
use deckbound::engagement::{ClassDef, Intention, Outcome, Stat5, Unit, battle, unit_from_class};

/// Total stat points a class gets (each stat ≥ 1) before any capability cost.
const BUDGET: u32 = 8;
/// Party size per side (NvN). AoE needs a group to matter, so this must be > 1.
const PARTY: usize = 3;
/// Per-matchup round cap.
const ROUNDS: u32 = 8;
/// AoE budget costs to scan (points removed from an AoE class's stat budget).
const COSTS: [u32; 4] = [0, 1, 2, 3];

struct Candidate {
    def: ClassDef,
    role: Intention,
    cell: &'static str,
    aoe: bool,
}

fn cell_name(ranged: bool, aoe: bool) -> &'static str {
    match (ranged, aoe) {
        (false, false) => "melee·single",
        (true, false) => "ranged·single",
        (false, true) => "melee·aoe",
        (true, true) => "ranged·aoe",
    }
}

fn role_tag(r: Intention) -> &'static str {
    match r {
        Intention::Vanguard => "Vanguard",
        Intention::Outrider => "Outrider",
        Intention::Rearguard => "Rearguard",
    }
}

/// Every candidate at AoE cost `k`: the 4 cells × all stat allocations of that cell's budget (each stat
/// ≥ 1). Range is free (budget `BUDGET`); area pays `k` (budget `BUDGET − k`).
fn candidates(k: u32) -> Vec<Candidate> {
    let cells = [(false, false), (true, false), (false, true), (true, true)];
    let mut out = Vec::new();
    for &(ranged, aoe) in &cells {
        let budget = BUDGET - if aoe { k } else { 0 };
        for c in compositions_k(budget - 5, 5) {
            let stats: Stat5 = (c[0] + 1, c[1] + 1, c[2] + 1, c[3] + 1, c[4] + 1);
            let (m, v, t, ca, f) = stats;
            let def = ClassDef {
                name: format!("{} M{m}V{v}T{t}C{ca}F{f}", cell_name(ranged, aoe)),
                ranged,
                aoe,
                stats,
            };
            let role = unit_from_class(&def, 0).intent;
            out.push(Candidate {
                def,
                role,
                cell: cell_name(ranged, aoe),
                aoe,
            });
        }
    }
    out
}

/// A grouped party of `PARTY` copies of a class on `side` — one shared group, so AoE hits all members and
/// single fire is walled by the front (the §4.5 bodyguard).
fn party(def: &ClassDef, side: u8) -> Vec<Unit> {
    (0..PARTY)
        .map(|_| {
            let mut u = unit_from_class(def, side);
            u.group = Some(0);
            u
        })
        .collect()
}

/// Net NvN result of `a` vs `b`, side bias cancelled by playing both side-assignments: `+1` a beats b,
/// `-1` b beats a, `0` even.
fn duel(a: &ClassDef, b: &ClassDef) -> i8 {
    let side_win = |x: &ClassDef, y: &ClassDef| -> i32 {
        match battle(party(x, 0), party(y, 1), ROUNDS) {
            Outcome::Win => 1,
            Outcome::Loss => -1,
            Outcome::Draw => 0,
        }
    };
    (side_win(a, b) - side_win(b, a)).signum() as i8
}

/// One AoE-cost setting's tournament: the full pairwise dominance matrix + per-candidate win counts.
struct Tournament {
    cands: Vec<Candidate>,
    beat: Vec<Vec<i8>>,
}

impl Tournament {
    fn run(k: u32) -> Tournament {
        let cands = candidates(k);
        let n = cands.len();
        let mut beat = vec![vec![0i8; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let r = duel(&cands[i].def, &cands[j].def);
                beat[i][j] = r;
                beat[j][i] = -r;
            }
        }
        Tournament { cands, beat }
    }
    fn wins(&self, i: usize) -> usize {
        self.beat[i].iter().filter(|&&x| x > 0).count()
    }
    fn losses(&self, i: usize) -> usize {
        self.beat[i].iter().filter(|&&x| x < 0).count()
    }
    /// Mean win-rate (wins / decisive-others) over candidates matching `pred` — the cell's strength.
    fn mean_winrate(&self, pred: impl Fn(&Candidate) -> bool) -> f64 {
        let rs: Vec<f64> = (0..self.cands.len())
            .filter(|&i| pred(&self.cands[i]))
            .map(|i| {
                let w = self.wins(i) as f64;
                let l = self.losses(i) as f64;
                if w + l == 0.0 { 0.5 } else { w / (w + l) }
            })
            .collect();
        if rs.is_empty() {
            0.0
        } else {
            rs.iter().sum::<f64>() / rs.len() as f64
        }
    }
    /// An RPS 3-cycle among viable candidates, maximizing distinct cells then roles. Returns the trio.
    fn rps_cycle(&self) -> Option<(usize, usize, usize, usize, usize)> {
        let n = self.cands.len();
        let viable: Vec<usize> = (0..n)
            .filter(|&i| self.wins(i) > 0 && self.losses(i) > 0)
            .collect();
        let mut best: Option<(usize, usize, usize, usize, usize)> = None;
        for &a in &viable {
            for &b in &viable {
                if self.beat[a][b] <= 0 {
                    continue;
                }
                for &c in &viable {
                    if self.beat[b][c] <= 0 || self.beat[c][a] <= 0 {
                        continue;
                    }
                    let mut cv = vec![self.cands[a].cell, self.cands[b].cell, self.cands[c].cell];
                    cv.sort_unstable();
                    cv.dedup();
                    let mut rv = vec![self.cands[a].role, self.cands[b].role, self.cands[c].role];
                    rv.sort_unstable();
                    rv.dedup();
                    let score = (cv.len(), rv.len());
                    if best.is_none_or(|(_, _, _, bc, br)| score > (bc, br)) {
                        best = Some((a, b, c, cv.len(), rv.len()));
                        if score == (3, 3) {
                            return best;
                        }
                    }
                }
            }
        }
        best
    }
}

fn main() {
    println!(
        "Class-discovery sweep — NvN ({PARTY}v{PARTY}, grouped) on the real engine; pricing the AoE capability.\n"
    );
    println!("AoE cost K (points removed from an AoE class's {BUDGET}-pt budget):");
    println!(
        "{:>3}  {:>5}  {:>9}  {:>10}  {:>11}  {:>5}",
        "K", "cands", "dominant", "AoE win%", "single win%", "cycle"
    );

    let mut tournaments = Vec::new();
    for &k in &COSTS {
        let t = Tournament::run(k);
        let n = t.cands.len();
        let dominant = (0..n).filter(|&i| t.wins(i) == n - 1).count();
        let aoe_wr = t.mean_winrate(|c| c.aoe) * 100.0;
        let single_wr = t.mean_winrate(|c| !c.aoe) * 100.0;
        let cyc = t
            .rps_cycle()
            .map(|(_, _, _, cells, roles)| format!("{cells}c/{roles}r"))
            .unwrap_or_else(|| "none".into());
        println!("{k:>3}  {n:>5}  {dominant:>9}  {aoe_wr:>7.0}%  {single_wr:>9.0}%  {cyc:>5}");
        tournaments.push((k, t, aoe_wr, single_wr));
    }

    // The price: smallest K where AoE no longer out-performs single (within 5 points) — i.e. area is paid
    // for, not free-and-dominant.
    let chosen = tournaments
        .iter()
        .find(|(_, _, aoe, single)| *aoe <= *single + 5.0)
        .or_else(|| tournaments.last());
    let Some((k, t, _, _)) = chosen else { return };
    println!("\nChosen AoE cost K = {k} (smallest where AoE win% ≤ single win% + 5). Details:\n");

    let n = t.cands.len();
    let dominant: Vec<usize> = (0..n).filter(|&i| t.wins(i) == n - 1).collect();
    let dead: Vec<usize> = (0..n).filter(|&i| t.losses(i) == n - 1).collect();
    println!("dominant (beat the whole field): {}", dominant.len());
    for &i in dominant.iter().take(6) {
        println!(
            "  !! {} [{}]",
            t.cands[i].def.name,
            role_tag(t.cands[i].role)
        );
    }
    println!("dead (lose to the whole field): {}", dead.len());

    if let Some((a, b, c, cells, roles)) = t.rps_cycle() {
        println!("\nRPS ecology ({cells} cells, {roles} roles) — no class is strictly best:");
        for &i in &[a, b, c] {
            let (m, v, tg, ca, f) = t.cands[i].def.stats;
            println!(
                "  {:<14} M{m}V{v}T{tg}C{ca}F{f} -> {}",
                t.cands[i].cell,
                role_tag(t.cands[i].role)
            );
        }
        println!(
            "  cycle: {} ▸ {} ▸ {} ▸ (back to first)",
            t.cands[a].cell, t.cands[b].cell, t.cands[c].cell
        );
    } else {
        println!("\nNo RPS cycle — the field is transitive (a strict pecking order).");
    }

    println!("\nPer-cell balanced exemplar (win-rate nearest 50%):");
    for &cell in &["melee·single", "ranged·single", "melee·aoe", "ranged·aoe"] {
        let pick = (0..n)
            .filter(|&i| t.cands[i].cell == cell)
            .min_by_key(|&i| {
                (
                    (t.wins(i) as i64 - t.losses(i) as i64).abs(),
                    -((t.wins(i) + t.losses(i)) as i64),
                )
            });
        if let Some(i) = pick {
            let (m, v, tg, ca, f) = t.cands[i].def.stats;
            println!(
                "  {cell:<14} M{m}V{v}T{tg}C{ca}F{f} -> {:<9} ({}W {}L of {})",
                role_tag(t.cands[i].role),
                t.wins(i),
                t.losses(i),
                n - 1
            );
        }
    }
}
