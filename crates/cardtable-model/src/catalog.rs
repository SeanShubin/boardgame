//! The single source of truth for card **content** — the five stats, the abilities, and the starter
//! roster — as Rust consts (not a data file). Everything that mints a stat card, an ability card, or a
//! starter kit reads its name, description, and values from here, so the concept ("Might" and what it
//! means) is authored once and every deck agrees.
//!
//! Pure and Bevy-free, like the rest of [`crate::model`]. The Stats and Abilities decks in
//! [`crate::fixtures`] project these directly (the concept: name + description), and a recruited
//! character deck instantiates them (the instance: "Might 2" + the same description) via
//! [`crate::model::Recipe`].

/// The five stats, each `(name, description)`. The order is the canonical stat order —
/// `[Might, Vitality, Toughness, Cadence, Finesse]` — shared with [`Recipe::stats`](crate::model::Recipe)
/// so a recipe's values line up positionally with these entries.
pub const STATS: [(&str, &str); 5] = [
    (
        "Might",
        "The force behind a strike; sets how much damage each blow deals.",
    ),
    (
        "Vitality",
        "Your life. With Toughness it sets your Health - how much you endure before you fall.",
    ),
    (
        "Toughness",
        "The bar your damage-pile must clear: within a sub-phase, damage piles up and each time it reaches your Toughness one Health card flips - leftover is wiped, not carried.",
    ),
    (
        "Cadence",
        "Your pace - how many actions you get each round.",
    ),
    (
        "Finesse",
        "Your skill - the grade of each action; how cheaply you win a contest.",
    ),
];

/// The kits' signature abilities — one per starter (the strike each carries, named for flavor), each
/// `(name, description)`. Two kits strike melee-single (the Executioner's burst vs the Phantom's Tempo)
/// but carry distinct abilities; the description records the underlying reach × area (see
/// `deckbound::duel` / the duel-locks set).
pub const ABILITIES: [(&str, &str); 4] = [
    ("Alpha Strike", "Melee | single target - one big blow."),
    ("Whirlwind", "Melee | area - hits the whole pack."),
    (
        "Stand-Off",
        "Ranged | single target - strikes from the back, no riposte.",
    ),
    ("Slip-and-Cut", "Melee | single target - evades, then cuts."),
];

/// The starter roster — the four **duel-locks kits** (`deckbound/data/balance/duel-locks.ron`), each
/// `(name, stats, ability)` where `stats` is `[Might, Vitality, Toughness, Cadence, Finesse]` — the same
/// order as [`STATS`] — and `ability` names an entry in [`ABILITIES`]. Each kit is the sole answer to one
/// creature lock: Executioner→the Anvil, Broadsider→the Swarm, Marksman→the Coil, Phantom→the Mirage.
pub const ROSTER: [(&str, [u8; 5], &str); 4] = [
    ("Executioner", [6, 3, 1, 1, 1], "Alpha Strike"),
    ("Broadsider", [2, 3, 3, 1, 1], "Whirlwind"),
    ("Marksman", [4, 4, 1, 2, 2], "Stand-Off"),
    ("Phantom", [4, 3, 1, 2, 3], "Slip-and-Cut"),
];

/// The description for a stat by name (from [`STATS`]), or `""` if unknown.
pub fn stat_description(name: &str) -> &'static str {
    STATS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, d)| d)
        .unwrap_or_default()
}

/// The description for an ability by name (from [`ABILITIES`]), or `""` if unknown.
pub fn ability_description(name: &str) -> &'static str {
    ABILITIES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, d)| d)
        .unwrap_or_default()
}

/// A kit ability's **reach** as `(melee, ranged)` — which attack types the hero carries. Range is
/// *position-determined* in combat (a Rearguard fires; a Vanguard/Outrider strikes — spec 4.2), but a body
/// must **carry** the matching attack type to be effective in a position: a melee-only body in the
/// Rearguard, or a ranged-only body up front, is legal but lands nothing. The two flags are **independent** —
/// a kit may carry both, or (with no strike card) neither. Today's kits each carry exactly one; unknown
/// abilities default to melee.
pub fn ability_reach(name: &str) -> (bool, bool) {
    match name {
        "Stand-Off" => (false, true), // ranged only
        "Whirlwind" => (true, false), // melee (area)
        "Alpha Strike" => (true, false),
        "Slip-and-Cut" => (true, false),
        _ => (true, false),
    }
}

/// A kit ability's **strike shape** as `(ranged, aoe)` — the reach and area a hero fights with, derived
/// from its signature ability. Kits carry no explicit reach/area flags (unlike a [`Creature`], which
/// stores them), so combat reads the shape here: Stand-Off strikes from range; Whirlwind hits an area;
/// Alpha Strike and Slip-and-Cut are single melee blows. The `ranged` bit is [`ability_reach`]'s ranged
/// flag; unknown abilities default to melee, single.
pub fn ability_shape(name: &str) -> (bool, bool) {
    let (_melee, ranged) = ability_reach(name);
    let aoe = matches!(name, "Whirlwind");
    (ranged, aoe)
}

/// A **creature** — a foe stationed at an encounter, mirrored from the duel-locks balance instrument
/// (`deckbound/data/balance/duel-locks.ron`, the source of truth for the numbers). `cardtable-model` is
/// pure and cannot depend on `deckbound`, so the four locks are re-declared here — kept in step with the
/// RON the same way [`ROSTER`] mirrors that file's kits. `stats` is `[Might, Vitality, Toughness,
/// Cadence, Finesse]` (the [`STATS`] order); `ranged`/`aoe` are the strike shape; `hoard` marks a card
/// that fields Vitality-many one-Health bodies in one pack; `pos` is an authored stance override (the
/// Coil holds the front regardless of its stats). A creature's **intention** and **posture** are not
/// stored — they derive from these fields (see [`creature_intention`] / [`creature_posture`]), so
/// editing a stat re-derives the read-out. `melee`/`ranged` are the creature's **reach** (which attack
/// types it carries — independent, like a kit's [`ability_reach`]); the four locks are all melee.
#[derive(Clone, Copy, Debug)]
pub struct Creature {
    pub name: &'static str,
    pub ability: &'static str,
    pub stats: [u8; 5],
    pub melee: bool,
    pub ranged: bool,
    pub aoe: bool,
    pub hoard: bool,
    pub pos: Option<&'static str>,
}

/// The four duel-locks creatures, mirrored from `duel-locks.ron`. Each is beaten by exactly one kit — a
/// clean diagonal (Anvil→Executioner, Swarm→Broadsider, Coil→Marksman, Mirage→Phantom) — and that answer
/// is a consequence of the numbers, not a keyword (see [`creature_counter`]).
pub const CREATURES: [Creature; 4] = [
    Creature {
        name: "The Anvil",
        ability: "Immovable",
        stats: [1, 2, 5, 1, 1],
        melee: true,
        ranged: false,
        aoe: false,
        hoard: false,
        pos: None,
    },
    Creature {
        name: "The Swarm",
        ability: "Overrun",
        stats: [1, 45, 1, 1, 1],
        melee: true,
        ranged: false,
        aoe: false,
        hoard: true,
        pos: None,
    },
    Creature {
        name: "The Coil",
        ability: "Riposte",
        stats: [6, 4, 2, 2, 1],
        melee: true,
        ranged: false,
        aoe: false,
        hoard: false,
        pos: Some("Vanguard"),
    },
    Creature {
        name: "The Mirage",
        ability: "Feint",
        stats: [6, 5, 2, 1, 2],
        melee: true,
        ranged: false,
        aoe: false,
        hoard: false,
        pos: None,
    },
];

/// The creatures' abilities, each `(name, description)` — the mechanic that makes the creature a lock,
/// in player-facing terms. Parallel to [`ABILITIES`] for kits.
pub const CREATURE_ABILITIES: [(&str, &str); 4] = [
    (
        "Immovable",
        "Toughness above all but the heaviest blow - sub-bar hits are wiped, so only one overwhelming strike cracks it.",
    ),
    (
        "Overrun",
        "A hoard of one-Health bodies. Single strikes kill one at a time and drown; an area attack clears the pack at once.",
    ),
    (
        "Riposte",
        "Strikes back at every melee blow it can reach, twice a round - answer it from range, where it can't reach.",
    ),
    (
        "Feint",
        "Never quite where it seems; low-Finesse blows whiff. Only a faster hand out-paces the feint and lands.",
    ),
];

/// The creature by `name` (from [`CREATURES`]), or `None`.
pub fn creature(name: &str) -> Option<&'static Creature> {
    CREATURES.iter().find(|c| c.name == name)
}

/// A creature's battle **intention** — Vanguard / Outrider / Rearguard — derived exactly as the
/// duel-locks `default_intentions` rule (`duel-locks.ron` §4): an authored [`pos`](Creature::pos) wins;
/// otherwise a ranged creature holds the Rearguard, a creature whose Might meets its Toughness flanks as
/// an Outrider, and the rest brace as a Vanguard. Pure derivation — no stored stance.
pub fn creature_intention(c: &Creature) -> &'static str {
    if let Some(pos) = c.pos {
        pos
    } else if c.ranged {
        "Rearguard"
    } else if c.stats[0] >= c.stats[2] {
        "Outrider"
    } else {
        "Vanguard"
    }
}

/// A creature's **posture** — the one-word tell of *why* it is hard — read off its ability (itself the
/// creature's signature mechanic): `armored`, `hoard`, `ripostes`, or `evasive`.
pub fn creature_posture(c: &Creature) -> &'static str {
    match c.ability {
        "Immovable" => "armored",
        "Overrun" => "hoard",
        "Riposte" => "ripostes",
        "Feint" => "evasive",
        _ => "",
    }
}

/// The [`ROSTER`] kit that cleanly answers a creature's lock — the diagonal, kept for internal labelling
/// and the solver check. Not shown on the card (the player infers the answer from the foe's posture).
pub fn creature_counter(c: &Creature) -> &'static str {
    match c.ability {
        "Immovable" => "Executioner", // armored -> one big blow (Alpha Strike)
        "Overrun" => "Broadsider",    // hoard -> area (Whirlwind)
        "Riposte" => "Marksman",      // ripostes -> ranged, no riposte (Stand-Off)
        "Feint" => "Phantom",         // evasive -> out-tempo (Slip-and-Cut)
        _ => "",
    }
}

/// The description for a creature ability by name (from [`CREATURE_ABILITIES`]), or `""` if unknown.
pub fn creature_ability_description(name: &str) -> &'static str {
    CREATURE_ABILITIES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, d)| d)
        .unwrap_or_default()
}

/// A location **encounter** — the foes stationed at a place on the map. Two tiers, both keyed to the
/// duel-locks creatures ([`CREATURES`]):
///
/// - A **solo** encounter (`party: false`) rings the inn: a single creature — its [`keystone`] — soloable
///   by the one kit that answers its lock. The four adjacent map cells.
/// - A **party** encounter (`party: true`) holds a corner: it fields **all four** creatures, with the
///   [`keystone`] as the bulk (a `×2` stack), so no lone hero clears every lock-type — you need the whole
///   party — yet the fight leans on the kit that answers the keystone.
///
/// No "favours" line is printed: which kit to bring is inferred from the foes' postures on the table.
#[derive(Clone, Copy, Debug)]
pub struct Encounter {
    pub location: &'static str,
    pub title: &'static str,
    pub flavor: &'static str,
    /// The creature name (in [`CREATURES`]) this encounter is built around — the only foe of a solo, the
    /// doubled bulk of a party.
    pub keystone: &'static str,
    pub party: bool,
}

/// Every non-inn location's encounter. Solos ring the inn (adjacent cells); party fights hold the
/// corners. Ashfen Crossing (the inn) has none, so this covers the other eight.
pub const ENCOUNTERS: [Encounter; 8] = [
    // --- solos: one creature, soloable by its answering kit --------------------------------------
    Encounter {
        location: "Cinderwatch Keep",
        title: "The Coiled Sentry",
        flavor: "A watch-drake coiled in the ruined keep, lashing at anything within reach.",
        keystone: "The Coil",
        party: false,
    },
    Encounter {
        location: "The Sundered Vault",
        title: "The Vault Anvil",
        flavor: "An armored warden set to guard the sundered vault, unmoved by lesser blows.",
        keystone: "The Anvil",
        party: false,
    },
    Encounter {
        location: "Thornmarch Gate",
        title: "The Thorn Swarm",
        flavor: "A boiling mass of bramble-imps swarming the gate.",
        keystone: "The Swarm",
        party: false,
    },
    Encounter {
        location: "The Salt Barrows",
        title: "The Barrow Mirage",
        flavor: "A grave-wraith drifting the salt barrows, never quite where it seems.",
        keystone: "The Mirage",
        party: false,
    },
    // --- corners: all four creatures, the keystone doubled ---------------------------------------
    Encounter {
        location: "The Hollow Rampart",
        title: "Breach of the Rampart",
        flavor: "The rampart is breached and everything pours through at once.",
        keystone: "The Swarm",
        party: true,
    },
    Encounter {
        location: "Greywater Ford",
        title: "Ambush at the Ford",
        flavor: "An ambush dug in at the crossing, ready to lash back at anything that wades in.",
        keystone: "The Coil",
        party: true,
    },
    Encounter {
        location: "Emberfall Hollow",
        title: "The Emberfall Beast",
        flavor: "A burning hollow guarded by something that will not fall to a single hand.",
        keystone: "The Anvil",
        party: true,
    },
    Encounter {
        location: "Ninefold Deep",
        title: "Horror of the Ninefold Deep",
        flavor: "The deep shifts and feints; nothing down here holds still.",
        keystone: "The Mirage",
        party: true,
    },
];

/// The encounter stationed at `location`, or `None` for a location with no encounter (the inn, Ashfen
/// Crossing).
pub fn encounter_for(location: &str) -> Option<&'static Encounter> {
    ENCOUNTERS.iter().find(|e| e.location == location)
}

/// The foes an encounter fields, as `(creature, quantity)` — a solo is its single keystone; a party is
/// all four [`CREATURES`] with the keystone doubled. The order is [`CREATURES`] order (keystone first for
/// a solo).
pub fn encounter_foes(e: &Encounter) -> Vec<(&'static Creature, u32)> {
    if e.party {
        CREATURES
            .iter()
            .map(|c| (c, if c.name == e.keystone { 2 } else { 1 }))
            .collect()
    } else {
        creature(e.keystone).map(|c| (c, 1)).into_iter().collect()
    }
}
