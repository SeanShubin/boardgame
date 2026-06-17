//! Combatants — **Actors** — and how non-player ones decide.
//!
//! An Actor is the umbrella (see `docs/games/deckbound/notes/entities.md`): a
//! **Character** is human-driven (improvises); a **Creature** follows a scripted
//! `Behavior` (a **decision deck** of moves + a target rule). Both carry the full stat
//! block — [`Offense`](crate::stats::Offense) and [`Defense`](crate::stats::Defense) — plus
//! a weapon and action cards, and round budgets (**tempo** = Speed, **focus** = Mind).

use engine::Rng;
use serde::Deserialize;

use crate::cards::Card;
use crate::duel::Move;
use crate::stats::{Defense, Offense};

/// Who drives an Actor's choices.
#[derive(Clone, Debug)]
pub enum Driver {
    /// A Character: the human (or a stand-in) improvises.
    Human,
    /// A Creature: a scripted instinct (a decision deck).
    Creature(Behavior),
}

/// How a creature chooses each beat: a random **deck** (real foes — the deck's composition
/// *is* its mixed strategy) or a deterministic **script** (tutorial dummies — algorithmic, so
/// a lesson plays out the same way every time, §7 `decision-making.md`).
#[derive(Clone, Debug)]
pub enum Instinct {
    Deck(Vec<Move>),
    Script(Script),
}

/// A deterministic creature algorithm (for tutorials) — built to punish a player who hasn't
/// learned the lesson and fold to the one who has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    /// Always the same move — a pure one-lesson dummy.
    Always(Move),
    /// Gather (doubling Force) until Force reaches `until`, then unload a Strike (the
    /// wind-up killshot you learn to interrupt).
    ChargeThenStrike { until: u32 },
    /// Evade until a player's whiffed Strike hands over Force, then Strike it back with their
    /// own momentum — so Striking a dodger is punished, while Anticipate beats it cleanly.
    Counter,
}

impl Script {
    fn pick(self, force: u32) -> Move {
        match self {
            Script::Always(m) => m,
            Script::ChargeThenStrike { until } => {
                if force >= until {
                    Move::Strike
                } else {
                    Move::Gather
                }
            }
            Script::Counter => {
                if force > 0 {
                    Move::Strike // punish: hit you with the Force you just lost
                } else {
                    Move::Evade // bait / dodge; Anticipate beats this
                }
            }
        }
    }
}

/// A creature's scripted instinct and whom it targets.
#[derive(Clone, Debug)]
pub struct Behavior {
    pub instinct: Instinct,
    pub target_rule: TargetRule,
}

impl Behavior {
    /// This beat's move. `force` is the creature's current Force (used by scripts); `rng` is
    /// the per-beat keyed RNG (used by decks), so draws stay order-independent (§1.9).
    pub fn pick(&self, force: u32, rng: &mut Rng) -> Move {
        match &self.instinct {
            Instinct::Deck(d) => {
                if d.is_empty() {
                    Move::Strike
                } else {
                    d[rng.below(d.len())]
                }
            }
            Instinct::Script(s) => s.pick(force),
        }
    }
}

/// Whom a creature goes for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum TargetRule {
    /// The front of the line (default).
    Front,
    /// The most fragile (fewest Body cards).
    LowestBody,
    /// The shakiest nerve (lowest Resolve).
    LeastResolute,
    /// Dives for the back line through the gauntlet (§4).
    Runner,
}

/// Which line an Actor stands in (§4). Public, free to shift between rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Line {
    Front,
    Back,
}

/// A combatant.
#[derive(Clone, Debug)]
pub struct Actor {
    pub name: String,
    pub role: String,
    pub offense: Offense,
    pub defense: Defense,
    /// The base strike profile.
    pub weapon: Card,
    /// Extra action cards (AoE, Rally, Dread, …) playable in the round.
    pub actions: Vec<Card>,
    pub driver: Driver,
    /// This actor crosses the gauntlet rather than holding a line (§4).
    pub runner: bool,
    /// Front or back line (§4 formation).
    pub line: Line,
    /// Reaches the enemy back line directly, bypassing the gauntlet (§4).
    pub ranged: bool,

    // round-scoped budgets
    pub tempo: i32,
    pub focus: u32,
    /// Finalized dead. Body reaching 0 is "mortally wounded" — the actor fights on and
    /// lands its committed blows; death is tallied at the round boundary (§1.9), which
    /// sets this. Once set it persists (the actor is out of the fight).
    pub fallen: bool,
}

impl Actor {
    pub fn is_down(&self) -> bool {
        self.defense.is_down()
    }

    pub fn is_human(&self) -> bool {
        matches!(self.driver, Driver::Human)
    }

    pub fn behavior(&self) -> Option<&Behavior> {
        match &self.driver {
            Driver::Creature(b) => Some(b),
            Driver::Human => None,
        }
    }

    /// Refresh tempo & focus and clear round-scoped state.
    pub fn refresh_round(&mut self) {
        self.tempo = self.offense.speed as i32;
        self.focus = self.defense.mind;
        self.defense.end_round();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Card, Effect, Lifecycle};
    use crate::stats::DamageType;

    fn weapon() -> Card {
        Card {
            name: "Fist".into(),
            targets: 1,
            reach: [1, 1],
            lifecycle: Lifecycle::Fleeting,
            effects: vec![Effect::Damage {
                power: 1,
                dtype: DamageType::Blunt,
            }],
        }
    }

    #[test]
    fn refresh_resets_budgets_to_speed_and_mind() {
        let mut a = Actor {
            name: "X".into(),
            role: "Y".into(),
            offense: Offense {
                speed: 5,
                ..Default::default()
            },
            defense: Defense::new(8, 1, 4, 3),
            weapon: weapon(),
            actions: vec![],
            driver: Driver::Human,
            runner: false,
            line: Line::Front,
            ranged: false,
            tempo: 0,
            focus: 0,
            fallen: false,
        };
        a.refresh_round();
        assert_eq!(a.tempo, 5);
        assert_eq!(a.focus, 3);
    }

    #[test]
    fn a_deck_draws_and_a_script_winds_up() {
        let mut rng = Rng::new(1);
        let deck = Behavior {
            instinct: Instinct::Deck(vec![Move::Strike]),
            target_rule: TargetRule::Front,
        };
        assert_eq!(deck.pick(0, &mut rng), Move::Strike);
        // The charger gathers until loaded, then strikes.
        let charger = Behavior {
            instinct: Instinct::Script(Script::ChargeThenStrike { until: 2 }),
            target_rule: TargetRule::Front,
        };
        assert_eq!(charger.pick(0, &mut rng), Move::Gather);
        assert_eq!(charger.pick(1, &mut rng), Move::Gather);
        assert_eq!(charger.pick(2, &mut rng), Move::Strike);
    }
}
