//! **Build `rules::combat` combatants from the catalog** - the single place the content layer (kits, creatures,
//! card abilities) is mapped onto the pure rules. Everything that fights over real encounters (the diagonal, the
//! corner tuner, the explorer, the fight UI) builds its units here, so the mapping - especially a creature's
//! **instinct** - is defined once and cannot drift.

use deckbound_content::catalog::{self, Creature, Encounter};
use rules::combat::resolve::{Combatant, Instinct, Side};

/// A hero kit as a combatant (heroes have no instinct - a player drives them).
pub fn kit(spec: (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (name, stats, ability) = spec;
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, stats, 0, melee, ranged).with_aoe(aoe)
}

/// The four hero kits, in roster (seat) order.
pub fn party() -> Vec<Combatant> {
    catalog::ROSTER.iter().copied().map(kit).collect()
}

/// **A creature's card instinct** - the deterministic behaviour printed on its card, mapped to the rules enum.
///
/// A body that holds a line keeps the thing behind it screened; a body that hunts leaves its post to chase. The
/// mapping is by the creature's signature ability:
/// - **Bulwark** (the Wall) - *holds the line.* An armoured wall exists to stand and screen; a wall that wanders
///   is not a wall. This is the fix for corners that read "too soft" because the screen was unreliable.
/// - **Riposte** (the Duelist) - *holds the line.* A counter-puncher wants you to come to it ("close in and you
///   trade and die"), so it stands its ground and punishes whoever closes.
/// - **Onslaught** (the Storm) - *hunts.* A charging host's whole identity is to come at you.
/// - **Overrun** (the Swarm) and anything else - *hunts.* A back-line horde is ranged and does not move much
///   regardless; hunting just means it shoots the softest target it can reach.
pub fn instinct_of(ability: &str) -> Instinct {
    match ability {
        "Bulwark" | "Riposte" => Instinct::HoldTheLine,
        _ => Instinct::HuntWeakest,
    }
}

/// A creature as a combatant, carrying its reach, area/horde shape, and its card instinct.
pub fn beast(c: &Creature) -> Combatant {
    Combatant::from_stats(c.name, Side::Foe, c.stats, 0, c.melee, c.ranged)
        .with_aoe(c.aoe)
        .as_horde(c.horde)
        .with_instinct(instinct_of(c.ability))
}

/// Build an encounter's foes with **distinct display names**. When a creature is fielded more than once, its
/// copies are numbered - "The Wall 1", "The Wall 2", ... - so the unit table, the log, and the option list can
/// tell them apart; a creature fielded once keeps its plain name. **Display-only**: the resolver never reads a
/// name (the solver's memo key is stats/rank), so numbering changes nothing about play or balance.
pub fn encounter_beasts(e: &Encounter) -> Vec<Combatant> {
    let mut out = Vec::new();
    for (c, q) in catalog::encounter_foes(e) {
        for i in 0..q {
            let mut b = beast(c);
            if q > 1 {
                b.name = format!("{} {}", c.name, i + 1);
            }
            out.push(b);
        }
    }
    out
}
