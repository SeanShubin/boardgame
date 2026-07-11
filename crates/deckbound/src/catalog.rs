//! The single source of truth for card **content** — the five stats, the abilities, and the starter
//! roster — as Rust consts (not a data file). Everything that mints a stat card, an ability card, or a
//! starter kit reads its name, description, and values from here, so the concept ("Might" and what it
//! means) is authored once and every deck agrees.
//!
//! Pure and Bevy-free: plain `const` data with no dependency on the board model or renderer. The game's
//! card-table fixtures project the Stats and Abilities decks directly (the concept: name + description), and
//! a recruited character deck instantiates them (the instance: "Might 2" + the same description) via the
//! card-table model's `Recipe`.

/// The five stats, each `(name, description)`. The order is the canonical stat order —
/// `[Might, Vitality, Toughness, Cadence, Finesse]` — the order a `Recipe`'s stat values line up with.
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

/// The four **generic attack** cards — the strike a body carries, one per `(reach x spread)` combination,
/// each `(name, description)`. A kit's identity lives in its *stats*; the attack card only sets **how** it
/// strikes, so two kits with the same attack differ only by their numbers. Reach and area both derive from
/// which of these a body carries (see [`ability_reach`] / [`ability_shape`]). Mirrors the reference sim's
/// strike-card set (`deckbound::sub_phase::strike_card`), so both name the same four attacks.
pub const ABILITIES: [(&str, &str); 4] = [
    ("Jab", "Melee | single target - a close blow to one foe."),
    (
        "Shot",
        "Ranged | single target - a shot at one foe from the back.",
    ),
    (
        "Sweep",
        "Melee | area - a close arc across a whole rank or group.",
    ),
    (
        "Salvo",
        "Ranged | area - fire across a whole rank or group.",
    ),
];

/// The starter roster — the four **reach x spread kits**, one per generic attack (Jab / Shot / Sweep /
/// Salvo), each `(name, stats, ability)` where `stats` is `[Might, Vitality, Toughness, Cadence, Finesse]`
/// — the same order as [`STATS`] — and `ability` names an entry in [`ABILITIES`]. Each kit is the sole solo
/// answer to one creature: Bruiser (melee single) answers the Wall, Marksman (ranged single) the Duelist,
/// Reaver (melee area, tanky) the Swarm, Gunner (ranged area) the Storm (see [`creature_counter`]).
pub const ROSTER: [(&str, [u8; 5], &str); 4] = [
    ("Bruiser", [7, 6, 1, 2, 2], "Jab"),   // melee single
    ("Marksman", [5, 2, 1, 2, 2], "Shot"), // ranged single
    ("Reaver", [1, 3, 3, 1, 2], "Sweep"),  // melee area, tanky (Toughness)
    ("Gunner", [3, 3, 1, 1, 2], "Salvo"),  // ranged area, fragile
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
        "Shot" | "Salvo" => (false, true), // ranged
        "Jab" | "Sweep" => (true, false),  // melee
        _ => (true, false),                // unknown -> melee
    }
}

/// A generic attack's **strike shape** as `(ranged, aoe)` — the reach and area a body fights with, derived
/// from which of the four attacks it carries: `Jab` melee-single, `Shot` ranged-single, `Sweep` melee-area,
/// `Salvo` ranged-area. The `ranged` bit is [`ability_reach`]'s ranged flag; `aoe` is set by the two Sweep /
/// Salvo cards. Unknown names default to melee, single.
pub fn ability_shape(name: &str) -> (bool, bool) {
    let (_melee, ranged) = ability_reach(name);
    let aoe = matches!(name, "Sweep" | "Salvo");
    (ranged, aoe)
}

/// A **creature** — a foe stationed at an encounter. `stats` is `[Might, Vitality, Toughness,
/// Cadence, Finesse]` (the [`STATS`] order); `ranged`/`aoe` are the strike shape; `horde` marks a card
/// that fields Vitality-many one-Health bodies in one pack; `pos` is an authored stance override (the
/// Duelist and the Storm hold the front regardless of their stats). A creature's **intention** and
/// **posture** are not stored — they derive from these fields (see [`creature_intention`] /
/// [`creature_posture`]), so editing a stat re-derives the read-out. `melee`/`ranged` are the creature's
/// **reach** (which attack types it carries — independent, like a kit's [`ability_reach`]).
#[derive(Clone, Copy, Debug)]
pub struct Creature {
    pub name: &'static str,
    pub ability: &'static str,
    pub stats: [u8; 5],
    pub melee: bool,
    pub ranged: bool,
    pub aoe: bool,
    pub horde: bool,
    pub pos: Option<&'static str>,
}

/// The four creatures. Each is beaten by exactly one kit — a clean diagonal (Wall->Bruiser,
/// Duelist->Marksman, Swarm->Reaver, Storm->Gunner) — and that answer is a consequence of the numbers,
/// not a keyword (see [`creature_counter`]).
pub const CREATURES: [Creature; 4] = [
    Creature {
        name: "The Wall",
        ability: "Bulwark",
        stats: [1, 4, 9, 1, 2],
        melee: true,
        ranged: false,
        aoe: false,
        horde: false,
        pos: None,
    },
    Creature {
        name: "The Duelist",
        ability: "Riposte",
        stats: [5, 5, 1, 2, 2],
        melee: true,
        ranged: false,
        aoe: false,
        horde: false,
        pos: Some("Vanguard"),
    },
    Creature {
        name: "The Swarm",
        ability: "Overrun",
        stats: [1, 8, 1, 1, 1],
        melee: false,
        ranged: true,
        aoe: false,
        horde: true,
        pos: None,
    },
    Creature {
        name: "The Storm",
        ability: "Onslaught",
        stats: [2, 12, 1, 2, 1],
        melee: true,
        ranged: false,
        aoe: false,
        horde: true,
        pos: Some("Vanguard"),
    },
];

/// The creatures' abilities, each `(name, description)` — the mechanic that makes the creature a lock,
/// in player-facing terms. Parallel to [`ABILITIES`] for kits.
pub const CREATURE_ABILITIES: [(&str, &str); 4] = [
    (
        "Bulwark",
        "High Toughness; sub-bar blows are wiped, so only one overwhelming strike dents it.",
    ),
    (
        "Riposte",
        "Hits hard and fast in melee; close in and you trade and die - answer it from range.",
    ),
    (
        "Overrun",
        "A pack of one-Health bodies firing from the back line; single strikes drown, an area strike clears the group.",
    ),
    (
        "Onslaught",
        "A pack that charges the front; too fast and deadly to trade with - clear it from range with an area strike.",
    ),
];

/// The creature by `name` (from [`CREATURES`]), or `None`.
pub fn creature(name: &str) -> Option<&'static Creature> {
    CREATURES.iter().find(|c| c.name == name)
}

/// A creature's battle **intention** — Vanguard / Outrider / Rearguard — derived by the same
/// `default_intentions` rule: an authored [`pos`](Creature::pos) wins; otherwise a ranged creature holds
/// the Rearguard, a creature whose Might meets its Toughness flanks as an Outrider, and the rest brace as
/// a Vanguard. Pure derivation — no stored stance.
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
/// creature's signature mechanic): `armored`, `ripostes`, `swarming`, or `charging`.
pub fn creature_posture(c: &Creature) -> &'static str {
    match c.ability {
        "Bulwark" => "armored",
        "Riposte" => "ripostes",
        "Overrun" => "swarming",
        "Onslaught" => "charging",
        _ => "",
    }
}

/// The [`ROSTER`] kit that cleanly answers a creature's lock — the diagonal, kept for internal labelling
/// and the solver check. Not shown on the card (the player infers the answer from the foe's posture).
pub fn creature_counter(c: &Creature) -> &'static str {
    match c.ability {
        "Bulwark" => "Bruiser",  // tough single -> concentrate a big blow (Jab)
        "Riposte" => "Marksman", // hard-hitting single that mauls melee -> answer from range (Shot)
        "Overrun" => "Reaver", // back-line horde -> a tough melee area survives the exchange (Sweep)
        "Onslaught" => "Gunner", // front horde -> ranged area first-strikes it (Salvo)
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
/// four creatures ([`CREATURES`]):
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
    /// The creature name (in [`CREATURES`]) this encounter is built around, and the kit it leans on — the
    /// only foe of a solo, the centrepiece of a party fight (see [`creature_counter`] for the answering kit).
    pub keystone: &'static str,
    pub party: bool,
    /// The exact foes fielded, as `(creature name, quantity)` — tuned so a solo is beaten only by its
    /// keystone's kit and a corner falls to the full four-kit party but to no lone kit (see the
    /// `v2_encounters` / `v2_corner_tune` examples).
    pub foes: &'static [(&'static str, u32)],
}

/// Every non-inn location's encounter. Solos ring the inn (adjacent cells); party fights hold the
/// corners. Ashfen Crossing (the inn) has none, so this covers the other eight.
pub const ENCOUNTERS: [Encounter; 8] = [
    // --- solos: one creature, soloable only by its answering kit ---------------------------------
    Encounter {
        location: "Cinderwatch Keep",
        title: "The Keep Duelist",
        flavor: "A blade-master holding the ruined keep, striking back at anyone who closes to melee.",
        keystone: "The Duelist",
        party: false,
        foes: &[("The Duelist", 1)],
    },
    Encounter {
        location: "The Sundered Vault",
        title: "The Vault Warden",
        flavor: "An armored warden set to guard the sundered vault, unmoved by all but the heaviest blow.",
        keystone: "The Wall",
        party: false,
        foes: &[("The Wall", 1)],
    },
    Encounter {
        location: "Thornmarch Gate",
        title: "The Thorn Swarm",
        flavor: "A boiling mass of bramble-imps loosing thorns from behind the gate.",
        keystone: "The Swarm",
        party: false,
        foes: &[("The Swarm", 1)],
    },
    Encounter {
        location: "The Salt Barrows",
        title: "The Barrow Storm",
        flavor: "A charging host of grave-risen boiling up out of the salt barrows, trampling all before them.",
        keystone: "The Storm",
        party: false,
        foes: &[("The Storm", 1)],
    },
    // --- corners: a warband the full party must break, each leaning on one kit (tuned) -----------
    Encounter {
        location: "The Hollow Rampart",
        title: "Breach of the Rampart",
        flavor: "The rampart is breached: two swarms boil through the gap, a warden anchoring the rubble.",
        keystone: "The Swarm",
        party: true,
        foes: &[("The Wall", 1), ("The Swarm", 2)],
    },
    Encounter {
        location: "Greywater Ford",
        title: "Ambush at the Ford",
        flavor: "An ambush at the crossing: a duelist lashing from the reeds, a warden and a swarm barring the ford.",
        keystone: "The Duelist",
        party: true,
        foes: &[("The Wall", 1), ("The Duelist", 1), ("The Swarm", 1)],
    },
    Encounter {
        location: "Emberfall Hollow",
        title: "The Emberfall Bulwark",
        flavor: "Twin wardens bar the burning hollow; neither will fall to a single hand.",
        keystone: "The Wall",
        party: true,
        foes: &[("The Wall", 2)],
    },
    Encounter {
        location: "Ninefold Deep",
        title: "Horror of the Ninefold Deep",
        flavor: "The deep churns: a charging host boils up out of the dark, screened by a warden and a swarm.",
        keystone: "The Storm",
        party: true,
        foes: &[("The Wall", 1), ("The Swarm", 1), ("The Storm", 2)],
    },
];

/// The encounter stationed at `location`, or `None` for a location with no encounter (the inn, Ashfen
/// Crossing).
pub fn encounter_for(location: &str) -> Option<&'static Encounter> {
    ENCOUNTERS.iter().find(|e| e.location == location)
}

/// The foes an encounter fields, as `(creature, quantity)` — read straight from its authored [`foes`]
/// list (`Encounter::foes`), resolving each name to its [`Creature`]. Unknown names are skipped.
pub fn encounter_foes(e: &Encounter) -> Vec<(&'static Creature, u32)> {
    e.foes
        .iter()
        .filter_map(|&(name, q)| creature(name).map(|c| (c, q)))
        .collect()
}

/// The five stat **names** in canonical [`STATS`] order — for callers that assemble or parse a character
/// deck (`Tableau::equip_character` / `character_recipe`), which take the names as data so the model stays
/// game-free.
pub fn stat_names() -> [&'static str; 5] {
    [STATS[0].0, STATS[1].0, STATS[2].0, STATS[3].0, STATS[4].0]
}

/// An encounter's foe **roster** as `(creature name, quantity)` pairs — what `Tableau::instantiate_from_bank`
/// deals from the Bestiary. Empty for a location with no encounter (the inn).
pub fn encounter_roster(place_label: &str) -> Vec<(&'static str, u32)> {
    encounter_for(place_label)
        .map(|e| {
            encounter_foes(e)
                .into_iter()
                .map(|(c, q)| (c.name, q))
                .collect()
        })
        .unwrap_or_default()
}
