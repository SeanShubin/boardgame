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

/// The abilities currently in play — the derived strike cards (one per range × area cell; see
/// `deckbound::sub_phase`) the starters carry — each `(name, description)`.
pub const ABILITIES: [(&str, &str); 4] = [
    ("Jab", "Melee · single target"),
    ("Shot", "Ranged · single target"),
    ("Sweep", "Melee · area"),
    ("Salvo", "Ranged · area"),
];

/// The generic starter roster (the suitless classes from `data/balance/generic-classes.ron`), each
/// `(name, stats, ability)` where `stats` is `[Might, Vitality, Toughness, Cadence, Finesse]` — the same
/// order as [`STATS`] — and `ability` names an entry in [`ABILITIES`].
pub const ROSTER: [(&str, [u8; 5], &str); 4] = [
    ("Skirmisher", [2, 2, 1, 2, 1], "Jab"),
    ("Sentinel", [1, 2, 2, 1, 2], "Shot"),
    ("Tempest", [1, 1, 1, 1, 2], "Salvo"),
    ("Cleaver", [1, 1, 2, 1, 1], "Sweep"),
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
