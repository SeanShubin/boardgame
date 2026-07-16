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
/// `[Might, Vitality, Grit, Cadence, Finesse]` — the order a `Recipe`'s stat values line up with.
pub const STATS: [(&str, &str); 5] = [
    (
        "Might",
        "The force behind a strike; sets how much damage each blow deals.",
    ),
    (
        "Vitality",
        "Your life. With Grit it sets your Health - how much you endure before you fall.",
    ),
    (
        "Grit",
        "The strength of each Health card: damage piles up against you, and every time the pile reaches your Grit one Health card turns. An unfinished pile keeps until the Reset, then closes.",
    ),
    ("Cadence", "How many Tempo cards you get each round."),
    (
        "Finesse",
        "The strength of each Tempo card - what one card is worth toward reaching a foe, or slipping one. It never touches damage.",
    ),
];

/// **Every stat's initial is unique**, across the five stats *and* the two pools they build (Health, Tempo):
/// `M H V G T C F`. So a one-letter abbreviation on a cramped tile is never ambiguous, and the legend never has
/// to disambiguate itself.
///
/// This is why Toughness is called **Grit**: it collided with Tempo. The rename earns its keep twice, because
/// the two pools turn out to be the same shape and the names now say so - *health is Vitality cards, each Grit
/// strong; tempo is Cadence cards, each Finesse strong.*
///
/// Each pool has a **flow** (`strength x count`) that meets it, and the two flows are parallel too:
/// - **damage** = `Might x strikes`, piling against Health - a Health card flips per Grit.
/// - **reach** = `Finesse x tempo cards`, spent in the reach/dodge contest - the higher reach wins.
///
/// So the four-word cross is *Health : Vitality : Grit : damage :: Tempo : Cadence : Finesse : reach*. Every
/// number is a **product**, never a quotient (full parallel in the combat spec's vocabulary section).
pub const POOLS: [&str; 2] = ["Health", "Tempo"];

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
/// Salvo), each `(name, stats, ability)` where `stats` is `[Might, Vitality, Grit, Cadence, Finesse]`
/// — the same order as [`STATS`] — and `ability` names an entry in [`ABILITIES`]. Each kit is the sole solo
/// answer to one creature: Raider (melee single) answers the Wall, Marksman (ranged single) the Duelist,
/// Bombardier (ranged area) the Swarm (a melee front horde, first-struck from range), Bastion (melee area, tanky)
/// the Brood (a ranged back horde, swept from inside) (see [`creature_counter`]).
pub const ROSTER: [(&str, [u8; 5], &str); 4] = [
    ("Raider", [6, 6, 1, 2, 2], "Jab"),       // melee single
    ("Marksman", [5, 2, 1, 2, 2], "Shot"),    // ranged single
    ("Bastion", [1, 3, 3, 1, 2], "Sweep"),    // melee area, tanky (Grit)
    ("Bombardier", [3, 3, 1, 1, 2], "Salvo"), // ranged area, fragile
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

/// A kit's default battle **intention** — Vanguard / Outrider / Rearguard — derived by exactly the rule the
/// engine uses (`default_intentions`) and that [`creature_intention`] applies to foes: a ranged body holds
/// the Rearguard, a body whose Might meets its Grit flanks as an Outrider, and the rest brace as a
/// Vanguard. Pure derivation from the numbers, never stored.
///
/// This is what the build is **for** — the player still *declares* a hero's intention each round in Marshal,
/// so any kit can take any position; this is the one the numbers are shaped for. Putting it on the card is
/// what makes "which position is this hero for?" answerable at a glance.
pub fn kit_intention(stats: [u8; 5], ability: &str) -> &'static str {
    let (_melee, ranged) = ability_reach(ability);
    if ranged {
        "Rearguard"
    } else if stats[0] >= stats[2] {
        // Might >= Grit: it hits harder than it holds, so it crosses to raid rather than brace.
        "Outrider"
    } else {
        "Vanguard"
    }
}

/// A kit's attack **shape** in words — the reach and spread its ability carries. Derived from the ability
/// ([`ability_reach`] / [`ability_shape`]), never stored.
pub fn kit_shape(ability: &str) -> &'static str {
    let (ranged, aoe) = ability_shape(ability);
    match (ranged, aoe) {
        (false, false) => "melee single",
        (false, true) => "melee area",
        (true, false) => "ranged single",
        (true, true) => "ranged area",
    }
}

/// A **creature** — a foe stationed at an encounter. `stats` is `[Might, Vitality, Grit,
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

/// The creatures. Four are a **clean diagonal** - each beaten by exactly one kit (Wall->Raider,
/// Duelist->Marksman, Swarm->Bombardier, Brood->Bastion) as a consequence of the numbers, not a keyword (see
/// [`creature_counter`]). The **Sniper** is the fifth - a **corner-only priority threat** with no solo, answered
/// not by a lone kit but by a melee slip past a cleared screen.
///
/// The two hordes split by position: the **Swarm** is the *melee front* horde (overruns the vanguard - answered
/// from range), the **Brood** the *ranged back* horde (spits from cover - answered by going in and sweeping).
pub const CREATURES: [Creature; 5] = [
    Creature {
        name: "The Wall",
        ability: "Bulwark",
        stats: [1, 4, 6, 1, 2],
        melee: true,
        ranged: false,
        aoe: false,
        horde: false,
        pos: None,
    },
    Creature {
        name: "The Duelist",
        ability: "Riposte",
        // Might 6, not 5: enough to kill a Raider that closes (Vitality 6, Grit 1 - so six lands it exactly).
        // At 5 the Raider survived on 1 and soloed it too, which made the lesson "hit it with anything".
        // "Close in and you trade and DIE - answer it from range" only teaches if the trade actually kills you.
        stats: [6, 5, 1, 2, 2],
        melee: true,
        ranged: false,
        aoe: false,
        horde: false,
        pos: Some("Vanguard"),
    },
    Creature {
        // The **melee front horde**: a swarm that overruns the vanguard with numbers, too fast and deadly to
        // trade with - clear the charge from range with an area strike (answered by the Bombardier).
        name: "The Swarm",
        ability: "Overrun",
        stats: [3, 4, 1, 2, 2],
        melee: true,
        ranged: false,
        aoe: false,
        horde: true,
        pos: Some("Vanguard"),
    },
    Creature {
        // The **ranged back horde**: a brood spitting from cover behind the front. Single strikes drown in the
        // numbers - you must go IN and sweep the nest in melee (answered by the tanky Bastion).
        name: "The Brood",
        ability: "Fusillade",
        stats: [1, 7, 1, 1, 1],
        melee: false,
        ranged: true,
        aoe: false,
        horde: true,
        pos: None,
    },
    Creature {
        // A single deadly marksman in the back line - a **priority threat you must slip a blade in to silence**.
        // Fragile (Vitality 1 - one blow kills it), but its shot picks off a hero, and screened it cannot be
        // answered from range until the screen falls. A corner threat, not a solo.
        name: "The Sniper",
        ability: "Deadeye",
        stats: [5, 1, 1, 2, 3],
        melee: false,
        ranged: true,
        aoe: false,
        horde: false,
        pos: None,
    },
];

/// The creatures' abilities, each `(name, description)` — the mechanic that makes the creature a lock,
/// in player-facing terms. Parallel to [`ABILITIES`] for kits.
pub const CREATURE_ABILITIES: [(&str, &str); 5] = [
    (
        "Bulwark",
        "High Grit; sub-bar blows are wiped, so only one overwhelming strike dents it.",
    ),
    (
        "Riposte",
        "Hits hard and fast in melee; close in and you trade and die - answer it from range.",
    ),
    (
        "Overrun",
        "A melee pack that overruns the vanguard with numbers; too fast and deadly to trade with - clear the charge from range with an area strike.",
    ),
    (
        "Fusillade",
        "A ranged brood spitting from cover behind the front; single strikes drown in the numbers - go in and sweep the nest with a melee area strike.",
    ),
    (
        "Deadeye",
        "A lone marksman in the back line; one deadly shot a round picks off a hero - slip a blade in to silence it before the screen even falls.",
    ),
];

/// The creature by `name` (from [`CREATURES`]), or `None`.
pub fn creature(name: &str) -> Option<&'static Creature> {
    CREATURES.iter().find(|c| c.name == name)
}

/// A creature's battle **intention** — Vanguard / Outrider / Rearguard — derived by the same
/// `default_intentions` rule: an authored [`pos`](Creature::pos) wins; otherwise a ranged creature holds
/// the Rearguard, a creature whose Might meets its Grit flanks as an Outrider, and the rest brace as
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
        "Overrun" => "charging",  // the melee swarm overruns the front
        "Fusillade" => "nesting", // the ranged brood spits from cover
        "Deadeye" => "sniping",
        _ => "",
    }
}

/// The [`ROSTER`] kit that cleanly answers a creature's lock — the diagonal, kept for internal labelling
/// and the solver check. Not shown on the card (the player infers the answer from the foe's posture).
pub fn creature_counter(c: &Creature) -> &'static str {
    match c.ability {
        "Bulwark" => "Raider",     // tough single -> concentrate a big blow (Jab)
        "Riposte" => "Marksman", // hard-hitting single that mauls melee -> answer from range (Shot)
        "Overrun" => "Bombardier", // melee front horde -> ranged area first-strikes the charge (Salvo)
        "Fusillade" => "Bastion", // ranged back horde -> a tough melee area goes in and sweeps it (Sweep)
        "Deadeye" => "", // the Sniper is a corner threat, answered by a melee slip, not a lone kit
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

/// **The lesson a corner is built to teach** — the load-bearing mechanic a party fight leans on, checked by
/// search (see the `regions_diagonal` / `regions_tune_corners` examples). A corner PASSES its behavior iff the
/// full party wins under `Combat` AND the behavior's own necessity test holds:
///
/// The **four corner strategies** are orthogonal - each is a *different* control failing, so none subsumes
/// another - and the **centre** is the capstone that deliberately requires them all:
///
/// - [`Concentration`](Behavior::Concentration): the single-target sub-party wins; the area-only one loses.
/// - [`Range`](Behavior::Range): the ranged-only sub-party wins; the melee-only one loses.
/// - [`Sweep`](Behavior::Sweep): the area sub-party wins; the single-target-only one loses.
/// - [`Raid`](Behavior::Raid): the full party under the clash-only control loses (the raid is load-bearing).
/// - [`CombinedArms`](Behavior::CombinedArms): melee-only, ranged-only, AND single-only all lose, AND clash-only
///   loses - every strategy at once. The centre cell; *meant* to subsume the corners (the graduation exam).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    /// **Concentration** - one overwhelming blow. Single-target wins; area-only loses (small hits wiped by Grit).
    Concentration,
    /// **Range** - answer from range, don't trade. Ranged-only wins; melee-only loses (it closes and dies).
    Range,
    /// **Sweep** - clear a group with an area strike. Area wins; single-target-only loses (it can't drown them).
    Sweep,
    /// **Raid** - infiltrate past a screen. Clash-only loses (the raid is load-bearing).
    Raid,
    /// **CombinedArms** - the capstone (the centre): every strategy at once. Melee-only, ranged-only, and
    /// single-only all lose, AND clash-only loses - so ranged, melee, an area strike, and the raid are ALL
    /// necessary together. It is *meant* to subsume the four corner strategies.
    CombinedArms,
}

/// A location **encounter** — the foes stationed at a place on the map. Three tiers:
///
/// - A **solo** (`party: false`) rings the centre: a single creature — its [`keystone`] — soloable by the one
///   kit that answers its lock. The four adjacent cells.
/// - A **strategy corner** (`party: true`, a non-`CombinedArms` [`behavior`]): a warband that a party can only
///   break with one **strategy** (concentrate / range / sweep / raid) - the four corner cells.
/// - The **capstone** (`party: true`, [`CombinedArms`](Behavior::CombinedArms)): the centre cell, demanding every
///   strategy at once.
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
    /// The **lesson** a corner is built to teach ([`Behavior`]), scored by search. `None` for solos (a solo
    /// teaches its keystone's lock, not a formation behavior).
    pub behavior: Option<Behavior>,
    /// The exact foes fielded, as `(creature name, quantity)` — tuned so a solo is beaten only by its
    /// keystone's kit and a corner falls to the full four-kit party but to no lone kit (see the
    /// `v2_encounters` / `v2_corner_tune` examples).
    pub foes: &'static [(&'static str, u32)],
}

/// The 3x3 map of encounters: **four solos** ring the centre (adjacent cells, one creature each), **four
/// strategy corners** hold the corners, and the **capstone** sits at the centre - Ashfen Crossing, the old inn,
/// now the graduation exam that demands every strategy at once. Nine in all.
pub const ENCOUNTERS: [Encounter; 9] = [
    // --- solos: one creature, soloable only by its answering kit ---------------------------------
    Encounter {
        location: "Cinderwatch Keep",
        title: "The Keep Duelist",
        flavor: "A blade-master holding the ruined keep, striking back at anyone who closes to melee.",
        keystone: "The Duelist",
        party: false,
        behavior: None,
        foes: &[("The Duelist", 1)],
    },
    Encounter {
        location: "The Sundered Vault",
        title: "The Vault Warden",
        flavor: "An armored warden set to guard the sundered vault, unmoved by all but the heaviest blow.",
        keystone: "The Wall",
        party: false,
        behavior: None,
        foes: &[("The Wall", 1)],
    },
    Encounter {
        location: "Thornmarch Gate",
        title: "The Thorn Swarm",
        flavor: "A boiling mass of bramble-imps that overruns the gate in a charging tide of thorns.",
        keystone: "The Swarm",
        party: false,
        behavior: None,
        foes: &[("The Swarm", 1)],
    },
    Encounter {
        location: "The Salt Barrows",
        title: "The Barrow Brood",
        flavor: "A brood nested deep in the salt barrows, spitting bone-shards from the dark behind their kin.",
        keystone: "The Brood",
        party: false,
        behavior: None,
        foes: &[("The Brood", 1)],
    },
    // --- corners: the four orthogonal STRATEGY lessons (each a different control fails) -----------
    Encounter {
        location: "Emberfall Hollow",
        title: "The Emberfall Bulwark",
        flavor: "Twin wardens bar the burning hollow, their guard so heavy that scattered blows are shrugged off - only one concentrated strike dents them.",
        keystone: "The Wall",
        party: true,
        behavior: Some(Behavior::Concentration),
        foes: &[("The Wall", 2)],
    },
    Encounter {
        location: "Greywater Ford",
        title: "Ambush at the Ford",
        flavor: "A blade-master lashes from the reeds of the ford - close in and it cuts you down, so answer it from range.",
        keystone: "The Duelist",
        party: true,
        behavior: Some(Behavior::Range),
        foes: &[("The Duelist", 3)],
    },
    Encounter {
        location: "Ninefold Deep",
        title: "Horror of the Ninefold Deep",
        flavor: "The deep churns: swarm upon swarm boils up out of the dark - no single blade can stem the tide, only an area strike clears it.",
        keystone: "The Swarm",
        party: true,
        behavior: Some(Behavior::Sweep),
        foes: &[("The Swarm", 2)],
    },
    Encounter {
        location: "The Hollow Rampart",
        title: "Breach of the Rampart",
        flavor: "Wardens anchor the breach while a marksman picks the party off from the rubble behind - you must slip a blade past the wall to silence it.",
        keystone: "The Sniper",
        party: true,
        behavior: Some(Behavior::Raid),
        foes: &[("The Wall", 4), ("The Sniper", 1)],
    },
    // --- the centre: the capstone, every strategy at once (the graduation exam) --------------------
    Encounter {
        location: "Ashfen Crossing",
        title: "The Last Stand at Ashfen",
        flavor: "Everything at once at the crossroads: a swarm overruns the front, wardens anchor the line, a sniper marks you from the dark - first-strike the swarm, slip a blade to the sniper, break the wall.",
        keystone: "The Sniper",
        party: true,
        behavior: Some(Behavior::CombinedArms),
        foes: &[("The Swarm", 1), ("The Wall", 2), ("The Sniper", 1)],
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

/// A **Rumor** — the strategy hint shown on an encounter's app-only card (a software `Utility` card, not a
/// physical one): how to defeat this fight, derived from the foes it fields so it stays in step with the
/// tuning. Returns the card's detail lines (ASCII only). A solo names the one kit that breaks it; a corner
/// names the answer to each threat and the kit it leans on.
pub fn encounter_rumor(e: &Encounter) -> Vec<String> {
    let attack = |kit: &str| {
        ROSTER
            .iter()
            .find(|k| k.0 == kit)
            .map(|k| k.2)
            .unwrap_or("")
    };
    if !e.party {
        let Some(c) = creature(e.keystone) else {
            return Vec::new();
        };
        let counter = creature_counter(c);
        vec![
            format!("{}: {}", c.name, creature_ability_description(c.ability)),
            format!(
                "Answer with the {counter} ({}) - the one kit that breaks it alone.",
                attack(counter)
            ),
        ]
    } else {
        let lean = creature(e.keystone)
            .map(creature_counter)
            .unwrap_or("the party");
        let foes: Vec<String> = encounter_foes(e)
            .into_iter()
            .map(|(c, q)| {
                if q > 1 {
                    format!("{} x{q}", c.name)
                } else {
                    c.name.to_string()
                }
            })
            .collect();
        let answers: Vec<String> = encounter_foes(e)
            .into_iter()
            .map(|(c, _)| format!("{} for {}", creature_counter(c), c.name))
            .collect();
        vec![
            format!("A warband: {}.", foes.join(", ")),
            format!(
                "No lone hand breaks it - bring the full party: {}.",
                answers.join(", ")
            ),
            format!("It leans on the {lean} - without it, the line holds."),
        ]
    }
}

/// The five stat **names** in canonical [`STATS`] order — for callers that assemble or parse a character
/// deck (`Board::equip_character` / `character_recipe`), which take the names as data so the model stays
/// game-free.
pub fn stat_names() -> [&'static str; 5] {
    [STATS[0].0, STATS[1].0, STATS[2].0, STATS[3].0, STATS[4].0]
}

/// An encounter's foe **roster** as `(creature name, quantity)` pairs — what `Board::instantiate_from_bank`
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **No two stats may share an initial.** The tiles are 120px wide, so they abbreviate to a single letter;
    /// an abbreviation the player cannot resolve is worse than none, and a legend that has to say "T means
    /// Toughness, not Tempo" has already failed. This is a build-time guarantee, not a convention: mint a stat
    /// that collides and this fails.
    #[test]
    fn every_stat_and_pool_has_a_unique_initial() {
        let words: Vec<&str> = STATS.iter().map(|(name, _)| *name).chain(POOLS).collect();
        let mut seen: Vec<char> = Vec::new();
        for w in &words {
            let c = w.chars().next().expect("a stat has a name");
            assert!(
                !seen.contains(&c),
                "'{c}' is already taken - {words:?} cannot be abbreviated to one letter each"
            );
            seen.push(c);
        }
        assert_eq!(seen.len(), 7, "five stats and the two pools they build");
    }
}
