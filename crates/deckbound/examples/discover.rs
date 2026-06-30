//! **Class-discovery sweep** — generate-and-test over the capability matrix to see whether a *balanced
//! RPS ecology* of classes emerges, instead of hand-authoring them.
//!
//! A "class" is a generic, suit-free body: a **strike card** (one of the melee/ranged × single/aoe matrix,
//! §4.3 capabilities-as-cards) plus an 8-point five-stat allocation. Its **role emerges** from range+stats
//! (it is never a free input). Every candidate is built and fought on the **real engine** (via the
//! `engagement` front-end → `combat::resolve_round`), so these are the shipped game's numbers.
//!
//! It enumerates all candidates, round-robins them 1v1 (both side-assignments, to cancel side bias), and
//! reports the tournament's shape:
//!   - **dominant** classes (beat the entire field) — a balance red flag (a strictly-best class);
//!   - **dead** classes (lose to the entire field);
//!   - a representative **RPS cycle** (A▸B▸C▸A) spanning as many distinct capability cells / roles as
//!     possible — the evidence that no single class is best (a non-transitive, balanced ecology);
//!   - a per-cell **balanced exemplar** (win-rate nearest 50%).
//!
//! Run: `cargo run -p deckbound --example discover`

use deckbound::balance::compositions_k;
use deckbound::engagement::{ClassDef, Intention, Outcome, Stat5, battle, unit_from_class};

/// Total stat points each class is allowed (summed over the five stats), each stat ≥ 1.
const BUDGET: u32 = 8;
/// Per-matchup round cap (matches the RPS-triangle probe).
const ROUNDS: u32 = 8;

struct Candidate {
    def: ClassDef,
    role: Intention,
    cell: &'static str,
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

/// Enumerate every candidate: the 4 capability cells × all 8-point stat allocations (each stat ≥ 1).
fn candidates() -> Vec<Candidate> {
    let cells = [(false, false), (true, false), (false, true), (true, true)];
    let mut out = Vec::new();
    for &(ranged, aoe) in &cells {
        // Each stat ≥ 1: distribute the BUDGET-5 surplus over 5 stats, then add 1 back to each.
        for c in compositions_k(BUDGET - 5, 5) {
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
            });
        }
    }
    out
}

/// Net 1v1 result of `a` vs `b`, side bias cancelled by playing both side-assignments: `+1` a beats b,
/// `-1` b beats a, `0` even (split or mutual draw).
fn duel(a: &ClassDef, b: &ClassDef) -> i8 {
    let side_win = |x: &ClassDef, y: &ClassDef| -> i32 {
        // x on side 0, y on side 1: +1 if x wins, -1 if x loses, 0 draw.
        match battle(
            vec![unit_from_class(x, 0)],
            vec![unit_from_class(y, 1)],
            ROUNDS,
        ) {
            Outcome::Win => 1,
            Outcome::Loss => -1,
            Outcome::Draw => 0,
        }
    };
    (side_win(a, b) - side_win(b, a)).signum() as i8
}

fn main() {
    let cands = candidates();
    let n = cands.len();
    println!(
        "Class-discovery sweep — {n} candidates ({} cells × 8-point allocations, each stat ≥ 1), on the real engine\n",
        4
    );

    // Full pairwise dominance matrix (symmetric): beat[i][j] = +1 if i beats j, -1 if j beats i, 0 even.
    let mut beat = vec![vec![0i8; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let r = duel(&cands[i].def, &cands[j].def);
            beat[i][j] = r;
            beat[j][i] = -r;
        }
    }
    let wins = |i: usize| (0..n).filter(|&j| beat[i][j] > 0).count();
    let losses = |i: usize| (0..n).filter(|&j| beat[i][j] < 0).count();

    // --- Dominators / dead weight (the balance red flags) ---
    let dominators: Vec<usize> = (0..n).filter(|&i| wins(i) == n - 1).collect();
    let dead: Vec<usize> = (0..n).filter(|&i| losses(i) == n - 1).collect();
    println!("DOMINANT (beat the whole field): {}", dominators.len());
    for &i in dominators.iter().take(6) {
        println!("  !! {} [{}]", cands[i].def.name, role_tag(cands[i].role));
    }
    println!("DEAD (lose to the whole field): {}", dead.len());

    // --- Role spread across the viable field ---
    let viable: Vec<usize> = (0..n).filter(|&i| wins(i) > 0 && losses(i) > 0).collect();
    println!("\nViable (win some & lose some): {}/{n}", viable.len());

    // --- A representative RPS cycle: A▸B▸C▸A, maximizing distinct capability cells, then distinct roles. ---
    let mut best: Option<(usize, usize, usize, usize, usize)> = None; // (a,b,c, #cells, #roles)
    'search: for &a in &viable {
        for &b in &viable {
            if beat[a][b] <= 0 {
                continue;
            }
            for &c in &viable {
                if beat[b][c] <= 0 || beat[c][a] <= 0 {
                    continue;
                }
                let cells = {
                    let mut v = vec![cands[a].cell, cands[b].cell, cands[c].cell];
                    v.sort_unstable();
                    v.dedup();
                    v.len()
                };
                let roles = {
                    let mut v = vec![cands[a].role, cands[b].role, cands[c].role];
                    v.sort_unstable();
                    v.dedup();
                    v.len()
                };
                let score = (cells, roles);
                if best.is_none_or(|(_, _, _, bc, br)| score > (bc, br)) {
                    best = Some((a, b, c, cells, roles));
                    if cells == 3 && roles == 3 {
                        break 'search; // can't do better than 3 distinct cells AND 3 roles
                    }
                }
            }
        }
    }
    match best {
        Some((a, b, c, cells, roles)) => {
            println!(
                "\nRPS ECOLOGY found — a non-transitive cycle ({cells} cells, {roles} roles), so no class is strictly best:"
            );
            for &i in &[a, b, c] {
                let (m, v, t, ca, f) = cands[i].def.stats;
                println!(
                    "  {:<14} M{m}V{v}T{t}C{ca}F{f} -> {}",
                    cands[i].cell,
                    role_tag(cands[i].role)
                );
            }
            println!(
                "  cycle: {} ▸ {} ▸ {} ▸ (back to first)",
                cands[a].cell, cands[b].cell, cands[c].cell
            );
        }
        None => println!(
            "\nNO RPS cycle among viable candidates — the field is transitive (a strict pecking order)."
        ),
    }

    // --- Per-cell balanced exemplar: the allocation whose win-rate is nearest 50%. ---
    println!("\nPer-cell balanced exemplar (win-rate nearest 50%):");
    for &cell in &["melee·single", "ranged·single", "melee·aoe", "ranged·aoe"] {
        let pick = (0..n).filter(|&i| cands[i].cell == cell).min_by_key(|&i| {
            let w = wins(i) as i64;
            let l = losses(i) as i64;
            ((w - l).abs(), -(w + l)) // closest to even, prefer more decisive matchups
        });
        if let Some(i) = pick {
            let (m, v, t, ca, f) = cands[i].def.stats;
            println!(
                "  {cell:<14} M{m}V{v}T{t}C{ca}F{f} -> {:<9} ({}W {}L of {})",
                role_tag(cands[i].role),
                wins(i),
                losses(i),
                n - 1
            );
        }
    }
}
