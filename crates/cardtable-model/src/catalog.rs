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
