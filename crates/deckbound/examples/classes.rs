//! Data-driven **generic-class** balance runner for the engagement sim. A "class" here is nothing but an
//! attack (range x shape) plus a five-stat allocation — the role *emerges* from range+stats (see
//! [`deckbound::engagement::intention_for`]). The class set is loaded from a RON file at runtime, so
//! iterating on numbers needs no rebuild: edit the `.ron`, re-run, read the report.
//!
//! Usage:
//!   cargo run -p deckbound --example classes                          # defaults to data/balance/generic-classes.ron
//!   cargo run -p deckbound --example classes -- path/to/classes.ron   # any class file
//!
//! The report exercises the core mechanics:
//!   1. the RPS triangle among the three single-target roles (Hold > Break > Deal > Hold);
//!   2. single vs AoE as a real choice — Marksman vs Artillery against a lone target and a 3-unit group;
//!   3. counters — a balanced party (one of each single-target role) vs extreme mono-formations.

use std::path::PathBuf;

use deckbound::engagement::{
    ClassDef, Intention, Outcome, Unit, battle, default_hits, load_classes, survivors_after,
    unit_from_class,
};

/// Look a class up by name in the loaded set (panics with a clear message if the RON is missing one the
/// report needs — the report is written against the four starter classes).
fn find<'a>(classes: &'a [ClassDef], name: &str) -> &'a ClassDef {
    classes
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("class set is missing required class {name:?}"))
}

/// `n` copies of a class on `side`, all bound to one shared group id (so they reposition collectively and
/// share a bodyguard front). `gid = None` leaves them as lone singleton groups.
fn squad(c: &ClassDef, side: u8, n: usize, gid: Option<u32>) -> Vec<Unit> {
    (0..n)
        .map(|_| {
            let mut u = unit_from_class(c, side);
            u.group = gid;
            u
        })
        .collect()
}

/// The smallest `max_rounds` (1..=cap) at which `side0` *wins* (wipes side1 and survives), or `None` if it
/// never wins within the cap. A compact "rounds to kill" probe built on the public `battle` — re-running a
/// fresh battle per cap, since `battle` consumes its forces.
fn rounds_to_win(
    mk0: impl Fn() -> Vec<Unit>,
    mk1: impl Fn() -> Vec<Unit>,
    cap: u32,
) -> Option<u32> {
    for r in 1..=cap {
        if battle(mk0(), mk1(), r) == Outcome::Win {
            return Some(r);
        }
    }
    None
}

fn rtw_str(r: Option<u32>) -> String {
    match r {
        Some(n) => format!("{n} rounds"),
        None => "never (>cap)".to_string(),
    }
}

fn tag(o: Outcome) -> &'static str {
    match o {
        Outcome::Win => "WIN ",
        Outcome::Loss => "LOSS",
        Outcome::Draw => "draw",
    }
}

/// 1. The RPS triangle among the three single-target roles. Want each *cycle* leg to WIN (Hold>Break,
/// Break>Deal, Deal>Hold) and each *anti* leg to NOT win.
fn rps_triangle(classes: &[ClassDef]) {
    let hold = find(classes, "Wall");
    let brk = find(classes, "Skirmisher");
    let deal = find(classes, "Marksman");
    let roles = [("Hold", hold), ("Break", brk), ("Deal", deal)];

    println!("1. RPS triangle (1v1, single-target roles, default vs default)");
    for (label, c) in &roles {
        let (m, v, t, ca, f) = c.stats;
        println!(
            "   {label:<6} {:<11} M{m} V{v} T{t} C{ca} F{f}  -> {:?}",
            c.name,
            unit_from_class(c, 0).intent
        );
    }
    let cyc = [(0usize, 1usize), (1, 2), (2, 0)];
    print!("   cycle (want WIN): ");
    for &(i, j) in &cyc {
        let o = battle(
            vec![unit_from_class(roles[i].1, 0)],
            vec![unit_from_class(roles[j].1, 1)],
            8,
        );
        print!("{}>{} {}  ", roles[i].0, roles[j].0, tag(o));
    }
    print!("\n   anti   (want !WIN): ");
    for &(i, j) in &cyc {
        let o = battle(
            vec![unit_from_class(roles[j].1, 0)],
            vec![unit_from_class(roles[i].1, 1)],
            8,
        );
        print!("{}>{} {}  ", roles[j].0, roles[i].0, tag(o));
    }
    println!("\n");
}

/// A 3-body defending group: a tough **Wall** front (T3) bodyguarding two soft backs, all held as Vanguards
/// behind the front (so the front soaks aimed fire and the backs stay shielded — the §4.5 bodyguard). The
/// backs are built from `back` but pinned to Vanguard/Endure so they sit in the group rather than running
/// off as their emergent role.
fn shielded_group(front: &ClassDef, back: &ClassDef, side: u8) -> Vec<Unit> {
    let mut g = Vec::new();
    let mut f = unit_from_class(front, side); // Wall -> Vanguard front
    f.group = Some(7);
    g.push(f);
    for _ in 0..2 {
        let mut u = unit_from_class(back, side);
        u.intent = Intention::Vanguard;
        u.hits = default_hits(Intention::Vanguard);
        u.group = Some(7);
        g.push(u);
    }
    g
}

/// 2. Single vs AoE is a real choice. Marksman (M3 single) vs Artillery (M2 area), each against (a) a lone
/// soft target and (b) a shielded group (a tough Wall front bodyguarding two soft backs). Measured by
/// survivors after a fixed number of rounds (a standoff would otherwise collapse to a Draw and hide the
/// difference). Want: the Marksman kills the lone target faster (M3>M2) but is **walled** by the group — its
/// aimed fire only routes to the living front, whose per-phase pile wipes before it accumulates a flip, so
/// the soft backs survive; the Artillery is slower on the lone target but **shreds the whole group**, its
/// area hitting every member at once, unevadable, straight past the bodyguard.
fn single_vs_aoe(classes: &[ClassDef]) {
    let marksman = find(classes, "Marksman");
    let artillery = find(classes, "Artillery");
    let wall = find(classes, "Wall");
    // Lone target: a single Wall held as a Vanguard (V2/T3) — it endures and never reaches the lone enemy
    // Rearguard, so it is a clean punching bag. M3 (Marksman) cracks T3 over two rounds; M2 (Artillery) never
    // crosses T3 alone, exposing the single-target Might gap. Group: a tough Wall front shielding two soft backs.
    let lone_target = || {
        let mut u = unit_from_class(wall, 1);
        u.intent = Intention::Vanguard;
        u.hits = default_hits(Intention::Vanguard);
        vec![u]
    };
    let rounds = 6;

    println!("2. Single vs AoE — Marksman (M3 single) vs Artillery (M2 area)");
    for (label, attacker) in [("Marksman", marksman), ("Artillery", artillery)] {
        let (m, _, _, _, _) = attacker.stats;
        let area = if attacker.aoe { " AoE" } else { "" };
        // (a) lone Wall-Vanguard target: rounds for the attacker to win.
        let lone = rounds_to_win(|| vec![unit_from_class(attacker, 0)], lone_target, rounds);
        // (b) shielded group: a tough Wall front bodyguarding two soft Skirmisher backs; survivors after `rounds`.
        let (_, group_alive) = survivors_after(
            vec![unit_from_class(attacker, 0)],
            shielded_group(wall, find(classes, "Skirmisher"), 1),
            rounds,
        );
        println!(
            "   {label:<9} (M{m}{area}):  vs lone Wall -> {:<13}   vs Wall+2-Skirmisher group -> {}/3 group survive (after {rounds} rounds)",
            rtw_str(lone),
            group_alive,
        );
    }
    println!(
        "   (want: Marksman kills the lone target faster; Artillery shreds more of the group)\n"
    );
}

/// 3. Counters — a balanced party (one Wall, one Skirmisher, one Marksman, played by their emergent roles)
/// vs extreme mono-formations of each role, plus a 3-unit AoE Artillery battery. A row is COUNTERED if the
/// party is not beaten (it wins or holds to a draw); a LOSS is an uncountered extreme.
fn counters(classes: &[ClassDef]) {
    let wall = find(classes, "Wall");
    let skirm = find(classes, "Skirmisher");
    let marks = find(classes, "Marksman");
    let arty = find(classes, "Artillery");

    let party = || {
        vec![
            unit_from_class(wall, 0),
            unit_from_class(skirm, 0),
            unit_from_class(marks, 0),
        ]
    };

    println!("3. Counterability — balanced party (Wall+Skirmisher+Marksman) vs extremes");
    let extremes = [
        ("3x Wall (all-Vanguard wall)", wall),
        ("3x Skirmisher (all-Outrider)", skirm),
        ("3x Marksman (all-Rearguard battery)", marks),
        ("3x Artillery (AoE battery)", arty),
    ];
    for (label, force) in extremes {
        let o = battle(party(), squad(force, 1, 3, None), 8);
        let verdict = match o {
            Outcome::Win => "COUNTERED (party wins)",
            Outcome::Draw => "held (draw)",
            Outcome::Loss => "!! UNCOUNTERED (party loses)",
        };
        println!("   {label:<40} {verdict}");
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

    println!(
        "Generic-class balance report ({} classes from {})\n",
        classes.len(),
        path.display()
    );
    rps_triangle(&classes);
    single_vs_aoe(&classes);
    counters(&classes);
}
