//! **Build `rules::combat` combatants from the catalog** - the single place the content layer (kits, creatures,
//! card abilities) is mapped onto the pure rules. Everything that fights over real encounters (the diagonal, the
//! corner tuner, the explorer, the fight UI) builds its units here, so the stat mapping is defined once and
//! cannot drift. (A foe's behaviour is not stored - it emerges from stats via the rules' disruption heuristic.)

use deckbound_content::catalog::{self, Creature, Encounter};
use rules::combat::resolve::{Combatant, Side};

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

/// A creature as a combatant, carrying its reach and area/horde shape. Its behaviour is **not stored** - every
/// foe runs the one disruption heuristic in [`rules::combat`], and a wall holds or a striker raids purely on its
/// stats (Grit vs Might), so there is no per-creature instinct to define or keep from drifting.
pub fn beast(c: &Creature) -> Combatant {
    Combatant::from_stats(c.name, Side::Foe, c.stats, 0, c.melee, c.ranged)
        .with_aoe(c.aoe)
        .as_horde(c.horde)
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
