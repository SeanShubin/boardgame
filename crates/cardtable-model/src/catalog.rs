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
        "Your life. With Toughness it sets your Health — how much you endure before you fall.",
    ),
    (
        "Toughness",
        "The bar your damage-pile must clear: within a sub-phase, damage piles up and each time it reaches your Toughness one Health card flips — leftover is wiped, not carried.",
    ),
    (
        "Cadence",
        "Your pace — how many actions you get each round.",
    ),
    (
        "Finesse",
        "Your skill — the grade of each action; how cheaply you win a contest.",
    ),
];

/// The kits' signature abilities — one per starter (the strike each carries, named for flavor), each
/// `(name, description)`. Two kits strike melee-single (the Executioner's burst vs the Phantom's Tempo)
/// but carry distinct abilities; the description records the underlying reach × area (see
/// `deckbound::duel` / the duel-locks set).
pub const ABILITIES: [(&str, &str); 4] = [
    ("Alpha Strike", "Melee · single target — one big blow."),
    ("Whirlwind", "Melee · area — hits the whole pack."),
    (
        "Stand-Off",
        "Ranged · single target — strikes from the back, no riposte.",
    ),
    ("Slip-and-Cut", "Melee · single target — evades, then cuts."),
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

/// A location **encounter** — the creature stationed at a place on the map and what beats it, as
/// `(location, title, kit, detail)` where `kit` names the [`ROSTER`] entry that answers it. Two tiers:
///
/// - [`SOLO_ENCOUNTERS`] ring the inn (the four map cells orthogonally adjacent to Ashfen Crossing).
///   Each is a single duel-locks creature (`deckbound/data/balance/duel-locks.ron`) soloable by the one
///   kit that answers its lock — Anvil→Executioner, Swarm→Broadsider, Coil→Marksman, Mirage→Phantom.
/// - [`PARTY_ENCOUNTERS`] hold the four corners. Each is a full-party fight that no lone hero survives,
///   but that still leans on one kit's strength to close out.
///
/// Every non-inn location appears in exactly one array, so [`encounter_for`] covers all eight.
pub const SOLO_ENCOUNTERS: [(&str, &str, &str, &str); 4] = [
    (
        "Cinderwatch Keep",
        "The Coiled Sentry",
        "Marksman",
        "A watch-drake that lashes back at any blow it can see. Pick it off from range — the Marksman's Stand-Off draws no riposte. Soloable.",
    ),
    (
        "The Sundered Vault",
        "The Vault Anvil",
        "Executioner",
        "An armored warden that shrugs off small hits. One overwhelming blow cracks it — the Executioner's Alpha Strike. Soloable.",
    ),
    (
        "Thornmarch Gate",
        "The Thorn Swarm",
        "Broadsider",
        "A boiling mass of bramble-imps; single strikes barely thin it. Clear the whole pack at once — the Broadsider's Whirlwind. Soloable.",
    ),
    (
        "The Salt Barrows",
        "The Barrow Mirage",
        "Phantom",
        "A grave-wraith that is never quite where it seems. Slip the feint, then cut — the Phantom's Slip-and-Cut. Soloable.",
    ),
];

/// The four corner encounters — full-party fights that lean on one kit. See [`SOLO_ENCOUNTERS`].
pub const PARTY_ENCOUNTERS: [(&str, &str, &str, &str); 4] = [
    (
        "The Hollow Rampart",
        "Breach of the Rampart",
        "Broadsider",
        "Wave on wave pours through the breach — bring the whole party. The Broadsider's Whirlwind is what holds the line.",
    ),
    (
        "Greywater Ford",
        "Ambush at the Ford",
        "Marksman",
        "Archers wait on the far bank. The full party must cross, but the Marksman's Stand-Off silences the bank first.",
    ),
    (
        "Emberfall Hollow",
        "The Emberfall Beast",
        "Executioner",
        "A great burning brute no lone hero survives. The party endures it while the Executioner lands the killing Alpha Strike.",
    ),
    (
        "Ninefold Deep",
        "Horror of the Ninefold Deep",
        "Phantom",
        "A shifting labyrinth-horror in the deep. The party maps its turns; the Phantom's Slip-and-Cut finds its heart.",
    ),
];

/// The encounter stationed at `location` as `(title, kit, detail)`, or `None` for a location with no
/// encounter (the inn, Ashfen Crossing). Searches both [`SOLO_ENCOUNTERS`] and [`PARTY_ENCOUNTERS`].
pub fn encounter_for(location: &str) -> Option<(&'static str, &'static str, &'static str)> {
    SOLO_ENCOUNTERS
        .iter()
        .chain(PARTY_ENCOUNTERS.iter())
        .find(|(loc, _, _, _)| *loc == location)
        .map(|&(_, title, kit, detail)| (title, kit, detail))
}
