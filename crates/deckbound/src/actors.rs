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
    pub name: &'static str,
    pub role: &'static str,
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
    pub name: &'static str,
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

/// Build the sample-combat party and warband.
pub fn roster() -> (Vec<Hero>, Vec<Creature>) {
    use DamageType::*;
    use Play::{Bash, Dread, Firestorm, Frostbite, Rally, Riposte, Steel};
    use Read::{Block, Evade, Scheme, Strike};

    // The three holding reads every hero shares.
    let reads = || vec![Play::Read(Block), Play::Read(Evade), Play::Read(Scheme)];

    let heroes = vec![
        Hero {
            name: "Aldric",
            role: "Knight",
            speed: 4,
            power: 5,
            magic: 0,
            spirit: 0,
            resolve_base: 4,
            rally_bonus: 0,
            fear_taken: 0,
            panicked: false,
            body: Body::new(8, 2),
            armor: Armor::new(vec![(Sharp, 3), (Blunt, 1)]), // heat passes whole
            line: Line::Front,
            strike_type: Blunt,
            plays: {
                let mut p = reads();
                p.push(Play::Read(Strike));
                p.push(Bash);
                p
            },
        },
        Hero {
            name: "Vera",
            role: "Duelist",
            speed: 5,
            power: 3,
            magic: 0,
            spirit: 0,
            resolve_base: 2,
            rally_bonus: 0,
            fear_taken: 0,
            panicked: false,
            body: Body::new(4, 1),
            armor: Armor::none(),
            line: Line::Front,
            strike_type: Sharp,
            plays: {
                let mut p = reads();
                p.push(Play::Read(Strike));
                p.push(Riposte);
                p
            },
        },
        Hero {
            name: "Sefa",
            role: "Mage",
            speed: 2,
            power: 1,
            magic: 5,
            spirit: 0,
            resolve_base: 1,
            rally_bonus: 0,
            fear_taken: 0,
            panicked: false,
            body: Body::new(3, 1),
            armor: Armor::none(),
            line: Line::Back,
            strike_type: Blunt,
            plays: {
                let mut p = vec![Firestorm, Frostbite];
                p.extend(reads());
                p
            },
        },
        Hero {
            name: "Bram",
            role: "Warden",
            speed: 3,
            power: 2,
            magic: 0,
            spirit: 5,
            resolve_base: 4,
            rally_bonus: 0,
            fear_taken: 0,
            panicked: false,
            body: Body::new(5, 2),
            armor: Armor::none(),
            line: Line::Back,
            strike_type: Blunt,
            plays: {
                let mut p = vec![Rally, Dread, Steel];
                p.extend(reads());
                p
            },
        },
    ];

    let creatures = vec![
        Creature {
            name: "Ironclad",
            speed: 2,
            power: 6,
            fear: 0,
            resolve: 5,
            body: Body::new(8, 3),
            armor: Armor::new(vec![(Sharp, 4), (Blunt, 3)]), // only heat cracks it
            line: Line::Front,
            strike_type: Sharp,
            behavior: Behavior::Bluff,
            count: 1,
            runner: false,
            disabled: false,
            reached: None,
        },
        Creature {
            name: "Stalker",
            speed: 6,
            power: 3,
            fear: 0,
            resolve: 3,
            body: Body::new(6, 1),
            armor: Armor::none(),
            line: Line::Front,
            strike_type: Sharp,
            behavior: Behavior::Runner,
            count: 1,
            runner: true,
            disabled: false,
            reached: None,
        },
        Creature {
            name: "Howler",
            speed: 4,
            power: 0,
            fear: 5,
            resolve: 2,
            body: Body::new(4, 1),
            armor: Armor::none(),
            line: Line::Front,
            strike_type: Heat, // unused; it attacks nerve, not body
            behavior: Behavior::Howl,
            count: 1,
            runner: false,
            disabled: false,
            reached: None,
        },
        Creature {
            name: "Husks",
            speed: 3,
            power: 1,
            fear: 0,
            resolve: 0,
            body: Body::new(1, 1),
            armor: Armor::none(),
            line: Line::Front,
            strike_type: Blunt,
            behavior: Behavior::Swarm,
            count: 6,
            runner: false,
            disabled: false,
            reached: None,
        },
    ];

    (heroes, creatures)
}
