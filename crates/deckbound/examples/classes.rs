//! **Roster report** for a generic (suit-free) class set — by default the starter roster in
//! `data/balance/generic-classes.ron`. A "class" is a **strike card** (range × shape, §4.3
//! capabilities-as-cards) plus a five-stat allocation; the **role emerges** from range+stats. Everything
//! runs on the **real engine** (via the `sub-phase` front-end), so the numbers are the shipped game's.
//!
//! It prints: the capability-matrix coverage (which melee/ranged × single/aoe cells are filled), each
//! class's emergent role, and a **head-to-head** — every class fielded as a grouped party of 3 vs every
//! other (both side-assignments, to cancel side bias), with a win/loss grid and each class's record. Use
//! it to eyeball a roster; use `--example discover` to *search* for a balanced one.
//!
//! Usage:
//!   cargo run -p deckbound --example classes                          # the starter roster
//!   cargo run -p deckbound --example classes -- path/to/classes.ron   # any class file

use std::path::PathBuf;

use deckbound::sub_phase::{
    ClassDef, Intention, Outcome, Unit, battle, load_classes, unit_from_class,
};

/// Party size per side — AoE only matters against a group, so class matchups are fought NvN.
const PARTY: usize = 3;
/// Per-matchup round cap.
const ROUNDS: u32 = 8;

fn cell_name(c: &ClassDef) -> &'static str {
    match (c.ranged, c.aoe) {
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

/// A grouped party of `PARTY` clones of a class on `side` (one shared group), the shape a class is tested
/// in: AoE hits the whole party, single fire is walled by the front (§4.5 bodyguard).
fn party(c: &ClassDef, side: u8) -> Vec<Unit> {
    (0..PARTY)
        .map(|_| {
            let mut u = unit_from_class(c, side);
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

/// The capability-matrix coverage — every loaded class binned into {melee,ranged} × {single,aoe}, with its
/// emergent role, so the four-cell coverage (and any empty cell) is visible at a glance.
fn attack_matrix(classes: &[ClassDef]) {
    println!("Capability coverage (melee/ranged × single/aoe)");
    for &ranged in &[false, true] {
        for &aoe in &[false, true] {
            let kind = format!(
                "{}·{}",
                if ranged { "ranged" } else { "melee" },
                if aoe { "aoe" } else { "single" }
            );
            let cell: Vec<String> = classes
                .iter()
                .filter(|c| c.ranged == ranged && c.aoe == aoe)
                .map(|c| format!("{} ({})", c.name, role_tag(unit_from_class(c, 0).intent)))
                .collect();
            let fill = if cell.is_empty() {
                "— (empty)".to_string()
            } else {
                cell.join(", ")
            };
            println!("  {kind:<14} {fill}");
        }
    }
    println!();
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/balance/generic-classes.ron")
        });
    let classes = load_classes(&path);
    let n = classes.len();
    println!("Roster report — {n} classes from {}\n", path.display());

    attack_matrix(&classes);

    println!("Roster (role emerges from range + stats)");
    for c in &classes {
        let (m, v, t, ca, f) = c.stats;
        println!(
            "  {:<11} {:<14} M{m}V{v}T{t}C{ca}F{f} -> {}",
            c.name,
            cell_name(c),
            role_tag(unit_from_class(c, 0).intent)
        );
    }
    println!();

    // Head-to-head: every class (party of 3) vs every other, both side-assignments.
    let beat: Vec<Vec<i8>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    if i == j {
                        0
                    } else {
                        duel(&classes[i], &classes[j])
                    }
                })
                .collect()
        })
        .collect();

    println!("Head-to-head ({PARTY}v{PARTY}, grouped; row beats col = W):");
    // Column header (short names).
    let short: Vec<String> = classes
        .iter()
        .map(|c| c.name.chars().take(4).collect())
        .collect();
    print!("  {:<11}", "");
    for s in &short {
        print!(" {s:>4}");
    }
    println!("   record");
    for i in 0..n {
        print!("  {:<11}", classes[i].name);
        for (j, &b) in beat[i].iter().enumerate() {
            let cell = if i == j {
                "  — ".to_string()
            } else {
                match b.cmp(&0) {
                    std::cmp::Ordering::Greater => "  W ".to_string(),
                    std::cmp::Ordering::Less => "  L ".to_string(),
                    std::cmp::Ordering::Equal => "  · ".to_string(),
                }
            };
            print!("{cell:>5}");
        }
        let w = beat[i].iter().filter(|&&x| x > 0).count();
        let l = beat[i].iter().filter(|&&x| x < 0).count();
        println!("   {w}W {l}L");
    }

    // A one-line verdict: dominant (beats all) or dead (loses to all) classes are balance flags.
    let dominant: Vec<&str> = (0..n)
        .filter(|&i| beat[i].iter().filter(|&&x| x > 0).count() == n - 1)
        .map(|i| classes[i].name.as_str())
        .collect();
    let dead: Vec<&str> = (0..n)
        .filter(|&i| beat[i].iter().filter(|&&x| x < 0).count() == n - 1)
        .map(|i| classes[i].name.as_str())
        .collect();
    println!();
    if dominant.is_empty() && dead.is_empty() {
        println!("No dominant or dead class — no strictly-best or strictly-worst archetype.");
    } else {
        if !dominant.is_empty() {
            println!(
                "!! dominant (beats the whole roster): {}",
                dominant.join(", ")
            );
        }
        if !dead.is_empty() {
            println!("!! dead (loses to the whole roster): {}", dead.join(", "));
        }
    }
}
