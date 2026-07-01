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
//! The **4-cell roster** (one class per cell forming a directed cycle) is chosen for **balanced stat
//! investment**: for each stat, sum `(value − 1)` across the four; the pick **minimizes the spread** of
//! those five totals, so no stat is dead across the roster (an all-Might-1 roster has a Might total of 0).
//!
//! Run: `cargo run -p deckbound --example discover`

use deckbound::balance::compositions_k;
use deckbound::engagement::{ClassDef, Intention, Outcome, Stat5, Unit, battle, unit_from_class};

/// Single-class stat sum (an AoE class pays `K` off it). Each class spends this over 5 stats, each ≥ 1.
const BUDGET: u32 = 8;
/// Per-stat clamp — a stat is never **dumped to 0** or **spiked to 5+** (stays in [1,4]).
const STAT_LO: u32 = 1;
const STAT_HI: u32 = 4;

// Roster stat-investment balance (the selection constraint): for each stat, sum `(value − 1)` across the
// chosen archetypes; the five per-stat totals must sit in a narrow range. An all-Might-1 roster has a Might
// total of 0 (a dead stat) — we minimize the *spread* of these totals so every stat is used evenly.
/// Party size per side (NvN). AoE needs a group to matter, so this must be > 1.
const PARTY: usize = 3;
/// Per-matchup round cap.
const ROUNDS: u32 = 8;
/// AoE budget costs to scan (points removed from an AoE class's stat budget).
const COSTS: [u32; 4] = [0, 1, 2, 3];
/// The four capability cells, in a fixed order (used for bucketing + the 4-cell search).
const CELLS: [&str; 4] = ["melee·single", "ranged·single", "melee·aoe", "ranged·aoe"];

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
        for c in compositions_k(budget, 5) {
            if c.iter().any(|&x| !(STAT_LO..=STAT_HI).contains(&x)) {
                continue; // no stat dumped to 0 or spiked past 4
            }
            let stats: Stat5 = (c[0], c[1], c[2], c[3], c[4]);
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
    /// Candidate indices bucketed by cell, in `CELLS` order.
    fn buckets(&self) -> Vec<Vec<usize>> {
        CELLS
            .iter()
            .map(|&cn| {
                (0..self.cands.len())
                    .filter(|&i| self.cands[i].cell == cn)
                    .collect()
            })
            .collect()
    }

    /// Roster **stat-investment totals**: for each stat, `Σ (value − 1)` over the given classes. `0` for a
    /// stat means every member has it at 1 (a dead stat across the roster).
    fn aggregates(&self, roster: &[usize]) -> [i32; 5] {
        let mut a = [0i32; 5];
        for &i in roster {
            let (m, v, t, c, f) = self.cands[i].def.stats;
            a[0] += m as i32 - 1;
            a[1] += v as i32 - 1;
            a[2] += t as i32 - 1;
            a[3] += c as i32 - 1;
            a[4] += f as i32 - 1;
        }
        a
    }

    /// The **spread** of a roster's stat-investment totals (max − min) — small = every stat used evenly.
    fn spread(agg: [i32; 5]) -> i32 {
        agg.iter().max().unwrap() - agg.iter().min().unwrap()
    }

    /// A **4-cell cycle**: one class per cell (all four cells) forming a *directed Hamiltonian 4-cycle*
    /// `A ▸ B ▸ C ▸ D ▸ A` (every edge a strict beat). A Hamiltonian cycle ⟹ the quartet is strongly
    /// connected (Moon), i.e. no cell dominates or is dominated — every capability cell holds a niche.
    /// Among all such cycles, picks the one with the **most balanced stat investment** (smallest
    /// [`Self::spread`] of the per-stat `Σ(value−1)` totals — so no stat is dead across the roster), then
    /// the most distinct roles. Returns the four indices in cycle order.
    fn four_cell_cycle(&self) -> Option<[usize; 4]> {
        let b = self.buckets();
        if b.iter().any(|bk| bk.is_empty()) {
            return None;
        }
        // The 6 directed cyclic orders of the 4 cells (fix cell 0 first; permute the rest).
        const ORDERS: [[usize; 4]; 6] = [
            [0, 1, 2, 3],
            [0, 1, 3, 2],
            [0, 2, 1, 3],
            [0, 2, 3, 1],
            [0, 3, 1, 2],
            [0, 3, 2, 1],
        ];
        let mut best: Option<([usize; 4], i32, usize)> = None; // (cycle, spread, roles)
        for &r0 in &b[0] {
            for &r1 in &b[1] {
                for &r2 in &b[2] {
                    for &r3 in &b[3] {
                        let rep = [r0, r1, r2, r3]; // rep[cell]
                        for ord in &ORDERS {
                            let cyc = [rep[ord[0]], rep[ord[1]], rep[ord[2]], rep[ord[3]]];
                            let edge = |x: usize, y: usize| self.beat[cyc[x]][cyc[y]] > 0;
                            if !(edge(0, 1) && edge(1, 2) && edge(2, 3) && edge(3, 0)) {
                                continue;
                            }
                            let sp = Self::spread(self.aggregates(&cyc));
                            let mut rv: Vec<Intention> =
                                cyc.iter().map(|&i| self.cands[i].role).collect();
                            rv.sort_unstable();
                            rv.dedup();
                            let roles = rv.len();
                            // Prefer the most balanced stat investment (smallest spread), then more roles.
                            if best.is_none_or(|(_, bsp, br)| {
                                (sp, -(roles as i32)) < (bsp, -(br as i32))
                            }) {
                                best = Some((cyc, sp, roles));
                            }
                        }
                    }
                }
            }
        }
        best.map(|(c, _, _)| c)
    }

    /// The cell (if any) that cannot hold a niche: every one of its candidates either beats all four
    /// cells' fields or loses to them — reported when no 4-cell cycle exists, to name the culprit.
    fn stuck_cell(&self) -> Option<&'static str> {
        CELLS.iter().copied().find(|&cn| {
            let mine: Vec<usize> = (0..self.cands.len())
                .filter(|&i| self.cands[i].cell == cn)
                .collect();
            // "stuck" = no member of this cell both beats and loses to some *other-cell* candidate.
            mine.iter().all(|&i| {
                let beats_other =
                    (0..self.cands.len()).any(|j| self.cands[j].cell != cn && self.beat[i][j] > 0);
                let loses_other =
                    (0..self.cands.len()).any(|j| self.cands[j].cell != cn && self.beat[i][j] < 0);
                !(beats_other && loses_other)
            })
        })
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
    println!(
        "Per-class sum {BUDGET}, stats [{STAT_LO},{STAT_HI}] (no 0/5); AoE sum {BUDGET}−K; roster picked for balanced stat investment (min spread of Σ(stat−1)):"
    );
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

    // 4-cell coexistence: a directed Hamiltonian 4-cycle with one class per cell (all four hold a niche).
    match t.four_cell_cycle() {
        Some(cyc) => {
            println!(
                "\n4-CELL COEXISTENCE — a directed 4-cycle, one class per cell (every capability cell holds a niche):"
            );
            for &i in &cyc {
                let (m, v, tg, ca, f) = t.cands[i].def.stats;
                println!(
                    "  {:<14} M{m}V{v}T{tg}C{ca}F{f} -> {}",
                    t.cands[i].cell,
                    role_tag(t.cands[i].role)
                );
            }
            let names: Vec<&str> = cyc.iter().map(|&i| t.cands[i].cell).collect();
            println!(
                "  cycle: {} ▸ {} ▸ {} ▸ {} ▸ (back to first)",
                names[0], names[1], names[2], names[3]
            );
            let agg = t.aggregates(&cyc);
            println!(
                "  stat investment Σ(stat−1)  M{} V{} T{} C{} F{}   (spread {}, smaller = every stat used evenly)",
                agg[0],
                agg[1],
                agg[2],
                agg[3],
                agg[4],
                Tournament::spread(agg)
            );
            // Head-to-head among the roster (row vs each column, in cycle order) + standing vs the field.
            println!("  head-to-head (row beats col = W; columns in cycle order) + field record:");
            for &i in &cyc {
                let hh: String = cyc
                    .iter()
                    .map(|&j| {
                        if i == j {
                            "  —".to_string()
                        } else {
                            match t.beat[i][j].cmp(&0) {
                                std::cmp::Ordering::Greater => "  W".to_string(),
                                std::cmp::Ordering::Less => "  L".to_string(),
                                std::cmp::Ordering::Equal => "  ·".to_string(),
                            }
                        }
                    })
                    .collect();
                println!(
                    "    {:<14}{hh}   field {}W {}L of {}",
                    t.cands[i].cell,
                    t.wins(i),
                    t.losses(i),
                    n - 1
                );
            }
        }
        None => match t.stuck_cell() {
            Some(cell) => println!(
                "\nNo 4-cell cycle at K={k}: the '{cell}' cell can't hold a non-dominated niche (it only beats, or only loses)."
            ),
            None => println!(
                "\nNo 4-cell cycle at K={k} (no one-per-cell quartet forms a directed 4-cycle), though each cell is individually viable."
            ),
        },
    }

    println!("\nPer-cell balanced exemplar (win-rate nearest 50%):");
    for &cell in &CELLS {
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
