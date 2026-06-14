//! The sample-combat roster — the "components" of the fight, with first-pass
//! appendix numbers. This is authored data; a real build would load it from a
//! component file. See `docs/games/deckbound/design/sample-round.md`.

use crate::read::Read;
use crate::stats::{Armor, Body, DamageType};

/// Which rank an actor stands in. The front line gates bodies; the back line
/// shelters behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Front,
    Back,
}

/// One play a hero can declare in a round. A play is either one of the four
/// basic reads or a signature card.
///
/// Plays split into **attacking** (commit to a target, leaving you exposed
/// elsewhere) and **holding** (a defensive read or support — you defend all
/// comers and, on the front line, feed the gauntlet's drag).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// One of the four basic reads. `Strike` is attacking; the rest hold.
    Read(Read),
    /// Aldric — a fast blunt Strike off the shield; carries stagger.
    Bash,
    /// Vera — an Evade that counters: defend, then a free riposte.
    Riposte,
    /// Sefa — heat, up to five distinct front-line targets at once.
    Firestorm,
    /// Sefa — cold; wounds and slows (cheaper for the wall to stop next round).
    Frostbite,
    /// Bram — raise the whole party's Resolve; lasts (a party-zone effect).
    Rally,
    /// Bram — a Fear attack on a foe's nerve; bypasses armor entirely.
    Dread,
    /// Bram — recover your own nerve, clearing accumulated fear.
    Steel,
}

impl Play {
    /// Attacking plays commit to a target and expose you elsewhere.
    pub fn is_attacking(self) -> bool {
        matches!(
            self,
            Play::Read(Read::Strike)
                | Play::Bash
                | Play::Firestorm
                | Play::Frostbite
                | Play::Dread
        )
    }

    /// Whether this play needs a single chosen target creature. (`Firestorm`
    /// picks its own targets; the holding/support plays need none.)
    pub fn needs_target(self) -> bool {
        matches!(
            self,
            Play::Read(Read::Strike) | Play::Bash | Play::Frostbite | Play::Dread
        )
    }

    /// A short label for menus and the log.
    pub fn name(self) -> &'static str {
        match self {
            Play::Read(r) => r.name(),
            Play::Bash => "Bash",
            Play::Riposte => "Riposte",
            Play::Firestorm => "Firestorm",
            Play::Frostbite => "Frostbite",
            Play::Rally => "Rally",
            Play::Dread => "Dread",
            Play::Steel => "Steel",
        }
    }
}

/// A player-controlled hero.
#[derive(Clone, Debug)]
pub struct Hero {
    pub name: String,
    pub role: String,
    pub speed: u32,
    pub power: u32,
    pub magic: u32,
    pub spirit: u32,
    pub resolve_base: i32,
    /// A persistent Rally / Steel buff (the party-zone, Lasting effect).
    pub rally_bonus: i32,
    /// Fear suffered this round; cleared at round start.
    pub fear_taken: i32,
    /// Set when fear overcomes Resolve this round — the hero's action is lost.
    pub panicked: bool,
    pub body: Body,
    pub armor: Armor,
    pub line: Line,
    /// The damage type of this hero's basic Strike.
    pub strike_type: DamageType,
    pub plays: Vec<Play>,
}

impl Hero {
    /// Current nerve: base, plus any lasting Rally, minus this round's fear.
    pub fn resolve(&self) -> i32 {
        self.resolve_base + self.rally_bonus - self.fear_taken
    }

    pub fn is_down(&self) -> bool {
        self.body.is_down()
    }
}

/// How a creature decides what to do — its "lock" in the scenario.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    /// A hidden Strike / Feint deck — the bluffer you must read.
    Bluff,
    /// Run the gauntlet for the lowest-Body hero.
    Runner,
    /// Howl Fear at the least-resolute hero, over the wall.
    Howl,
    /// A swarm that shambles at the front line.
    Swarm,
}

/// A non-player creature.
#[derive(Clone, Debug)]
pub struct Creature {
    pub name: String,
    pub speed: u32,
    pub power: u32,
    /// Fear rating (for `Howl`); 0 for non-fearful foes.
    pub fear: u32,
    /// Nerve, for resisting a Dread.
    pub resolve: i32,
    pub body: Body,
    pub armor: Armor,
    pub line: Line,
    pub strike_type: DamageType,
    pub behavior: Behavior,
    /// Swarm size; 1 for a single creature.
    pub count: u32,
    pub runner: bool,
    // --- transient, reset each round ---
    /// Its action was cancelled this round (stopped at the wall, or routed).
    pub disabled: bool,
    /// A Runner that slipped the wall reached this hero index.
    pub reached: Option<usize>,
}

impl Creature {
    pub fn is_swarm(&self) -> bool {
        matches!(self.behavior, Behavior::Swarm)
    }

    /// Whether any of this creature still stands (a swarm by its count).
    pub fn alive(&self) -> bool {
        if self.is_swarm() {
            self.count > 0
        } else {
            !self.body.is_down()
        }
    }

    /// Apply one hit of `raw` damage of `ty`. Returns cards flipped (a single
    /// creature) or members killed (a swarm — at most one per hit).
    pub fn take_hit(&mut self, raw: u32, ty: DamageType) -> u32 {
        let after = raw.saturating_sub(self.armor.reduce(ty));
        if self.is_swarm() {
            let killed = if after >= self.body.toughness { 1 } else { 0 };
            self.count = self.count.saturating_sub(killed);
            killed
        } else {
            let flips = self.body.flips_for(after);
            self.body.remaining -= flips;
            flips
        }
    }
}

/// Construct a hero with the per-round transient fields cleared. The booklet
/// loader supplies a card's data — no hero is defined in code.
#[allow(clippy::too_many_arguments)]
pub fn new_hero(
    name: String,
    role: String,
    speed: u32,
    power: u32,
    magic: u32,
    spirit: u32,
    resolve_base: i32,
    body: Body,
    armor: Armor,
    line: Line,
    strike_type: DamageType,
    plays: Vec<Play>,
) -> Hero {
    Hero {
        name,
        role,
        speed,
        power,
        magic,
        spirit,
        resolve_base,
        rally_bonus: 0,
        fear_taken: 0,
        panicked: false,
        body,
        armor,
        line,
        strike_type,
        plays,
    }
}

/// Construct a creature with the per-round transient fields cleared.
#[allow(clippy::too_many_arguments)]
pub fn new_creature(
    name: String,
    speed: u32,
    power: u32,
    fear: u32,
    resolve: i32,
    body: Body,
    armor: Armor,
    line: Line,
    strike_type: DamageType,
    behavior: Behavior,
    count: u32,
    runner: bool,
) -> Creature {
    Creature {
        name,
        speed,
        power,
        fear,
        resolve,
        body,
        armor,
        line,
        strike_type,
        behavior,
        count,
        runner,
        disabled: false,
        reached: None,
    }
}
