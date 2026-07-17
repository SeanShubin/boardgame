//! **The balance + insight gate.** Locks the intended SHAPE of the whole encounter set - which sub-parties win,
//! which lose, and whether a win needs *insight* or just the right *tool* - so a rule or content change that
//! breaks it fails `cargo test`, not just the eye of whoever last ran `regions_diagonal insight`.
//!
//! Insight classes (see [`deckbound_board::verify`]): **T** greedy already wins, **I** only the solver wins (a
//! real read is needed), **X** neither wins. Asserting the class subsumes the balance check too: a WIN sub-party
//! that reads `X` means balance broke; a LOSE control that reads anything but `X` means it became winnable (or
//! cheesable).

use deckbound_board::units::{beast, kit};
use deckbound_board::verify::{insight_class, solver_wins};
use deckbound_content::catalog::{self};
use rules::combat::game::{ClashOnly, Combat};
use rules::combat::resolve::Combatant;

/// The foes of the encounter at `location`, built into combatants.
fn foes(location: &str) -> Vec<Combatant> {
    let e = catalog::ENCOUNTERS
        .iter()
        .find(|e| e.location == location)
        .unwrap_or_else(|| panic!("no encounter at {location}"));
    let mut out = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    out
}

/// The full party and its four strategy sub-parties: `(kits, melee, ranged, single, area)`.
type Parties = (
    Vec<Combatant>,
    Vec<Combatant>,
    Vec<Combatant>,
    Vec<Combatant>,
    Vec<Combatant>,
);
fn parties() -> Parties {
    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    let melee = kits.iter().filter(|k| k.melee).cloned().collect();
    let ranged = kits
        .iter()
        .filter(|k| k.ranged && !k.melee)
        .cloned()
        .collect();
    let single = kits.iter().filter(|k| !k.aoe).cloned().collect();
    let area = kits.iter().filter(|k| k.aoe).cloned().collect();
    (kits, melee, ranged, single, area)
}

/// **Composition corners teach a TOOL.** The right sub-party wins *greedily* (`T`) and so does the full party; the
/// wrong sub-party cannot win at all (`X`). If a win row slips to `X`, balance broke; if a lose row slips off `X`,
/// the wrong tool became viable.
#[test]
fn composition_corners_keep_their_tool_shape() {
    let (kits, melee, ranged, single, area) = parties();

    let em = foes("Emberfall Hollow"); // Concentration
    assert_eq!(insight_class(&kits, &em), 'T', "Concentration: full party");
    assert_eq!(
        insight_class(&single, &em),
        'T',
        "Concentration: single-target wins with the tool"
    );
    assert_eq!(
        insight_class(&area, &em),
        'X',
        "Concentration: area cannot win"
    );

    let gw = foes("Greywater Ford"); // Range
    assert_eq!(insight_class(&kits, &gw), 'T', "Range: full party");
    assert_eq!(
        insight_class(&ranged, &gw),
        'T',
        "Range: ranged wins with the tool"
    );
    assert_eq!(insight_class(&melee, &gw), 'X', "Range: melee cannot win");

    let nd = foes("Ninefold Deep"); // Sweep
    assert_eq!(insight_class(&kits, &nd), 'T', "Sweep: full party");
    assert_eq!(
        insight_class(&area, &nd),
        'T',
        "Sweep: area wins with the tool"
    );
    assert_eq!(
        insight_class(&single, &nd),
        'X',
        "Sweep: single-target cannot win"
    );
}

/// **Insight corners teach PLAY.** The full party wins only with a real read (`I` - greedy loses), every wrong
/// tool is impossible (`X`), and the raid is load-bearing (clash-only loses). If the full-party row slips to `T`,
/// the corner went trivial; if a control slips to `I`, it became cheesable.
#[test]
fn insight_corners_keep_their_play_shape() {
    let (kits, melee, ranged, single, _area) = parties();

    let hr = foes("The Hollow Rampart"); // Raid
    assert_eq!(
        insight_class(&kits, &hr),
        'I',
        "Raid: full party needs a real read"
    );
    assert!(
        !solver_wins::<ClashOnly>(&kits, &hr),
        "Raid: clash-only must lose (the raid carries it)"
    );

    let af = foes("Ashfen Crossing"); // CombinedArms
    assert_eq!(
        insight_class(&kits, &af),
        'I',
        "CombinedArms: full party needs a real read"
    );
    assert_eq!(
        insight_class(&melee, &af),
        'X',
        "CombinedArms: melee-only impossible"
    );
    assert_eq!(
        insight_class(&ranged, &af),
        'X',
        "CombinedArms: ranged-only impossible"
    );
    assert_eq!(
        insight_class(&single, &af),
        'X',
        "CombinedArms: single-only impossible"
    );
    assert!(
        !solver_wins::<ClashOnly>(&kits, &af),
        "CombinedArms: clash-only must lose"
    );
}

/// **Solos stay a clean diagonal** - each is soloable by EXACTLY its keystone's counter kit, no more, no fewer.
#[test]
fn solos_stay_a_clean_diagonal() {
    let kits: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    let names: Vec<&str> = catalog::ROSTER.iter().map(|k| k.0).collect();
    for e in catalog::ENCOUNTERS.iter().filter(|e| !e.party) {
        let f = foes(e.location);
        let want = catalog::creature(e.keystone)
            .map(catalog::creature_counter)
            .unwrap_or("");
        let winners: Vec<&str> = kits
            .iter()
            .zip(&names)
            .filter(|(k, _)| solver_wins::<Combat>(std::slice::from_ref(*k), &f))
            .map(|(_, n)| *n)
            .collect();
        assert_eq!(
            winners,
            vec![want],
            "{}: soloable by exactly [{want}]",
            e.location
        );
    }
}
