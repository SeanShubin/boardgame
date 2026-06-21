//! Combatants — **Actors** — their attack profile, and how creatures decide.
//!
//! An Actor is the umbrella (see `docs/games/deckbound/notes/entities.md`): a **Character**
//! is human-driven; a **Creature** follows a scripted `Behavior`. Both carry the full stat
//! block — [`Offense`](crate::stats::Offense) / [`Defense`](crate::stats::Defense) — a weapon,
//! action cards, and the round's **Tempo** pool (= Speed; the single breadth budget since the
//! Focus/Mind merge, §3). Each Actor also has an **attack profile** (§4.2): the range(s) it can
//! strike and contest at, plus round-scoped **status** (Stagger / Shove / Disarm) set by Controller
//! cards and cleared at Refresh.

use engine::Rng;
use serde::Deserialize;

use crate::cards::Card;
use crate::duel::Move;
use crate::stats::{Defense, Offense};

/// The range of an engagement (§4.2). Position-determined: lanes and Skirmisher strikes are
/// melee; Reserve fire is ranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    Melee,
    Ranged,
}

/// What range(s) an Actor can attack and contest at (§4.2). A strike at a range the target
/// cannot answer is an **auto-hit**; a same-range meeting is a trade (or a Clash).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Attack {
    Melee,
    Ranged,
    Both,
    Neither,
}

impl Attack {
    /// Can this profile act / contest at `range`?
    pub fn has(self, range: Range) -> bool {
        matches!(
            (self, range),
            (Attack::Both, _) | (Attack::Melee, Range::Melee) | (Attack::Ranged, Range::Ranged)
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Attack::Melee => "melee",
            Attack::Ranged => "ranged",
            Attack::Both => "melee+ranged",
            Attack::Neither => "support",
        }
    }
}

/// Who drives an Actor's choices.
#[derive(Clone, Debug)]
pub enum Driver {
    /// A Character: the human (or a stand-in) improvises.
    Human,
    /// A Creature: a scripted instinct.
    Creature(Behavior),
}

/// A creature's policy: how eagerly it commits to the Vanguard, whom it targets, and (only
/// when the Clash module is on) how it plays the four-card mix-up.
#[derive(Clone, Debug)]
pub struct Behavior {
    /// 0..=10 — higher commits more Actors to the Vanguard / slips more readily.
    pub aggression: u32,
    pub target_rule: TargetRule,
    /// Clash instinct — used only when the optional Clash module is enabled.
    pub instinct: Instinct,
}

impl Behavior {
    /// This beat's Clash move (Clash module only). `force` is the creature's current Force.
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

/// How a creature chooses each Clash beat: a random **deck** or a deterministic **script**
/// (tutorial dummies). Used only when the Clash module is enabled.
#[derive(Clone, Debug)]
pub enum Instinct {
    Deck(Vec<Move>),
    Script(Script),
}

/// A deterministic Clash algorithm (tutorial dummies).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Always(Move),
    ChargeThenStrike { until: u32 },
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
                    Move::Strike
                } else {
                    Move::Evade
                }
            }
        }
    }
}

/// Whom a creature goes for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum TargetRule {
    /// The first reachable enemy.
    Front,
    /// The most fragile (fewest Body cards).
    LowestBody,
    /// The shakiest nerve (lowest Resolve).
    LeastResolute,
}

/// A combatant.
#[derive(Clone, Debug)]
pub struct Actor {
    pub name: String,
    pub role: String,
    pub offense: Offense,
    pub defense: Defense,
    /// The base strike profile (supplies the damage type).
    pub weapon: Card,
    /// Action/power cards playable in the round (§"cards may supersede the core").
    pub actions: Vec<Card>,
    pub driver: Driver,
    /// Range(s) this Actor can attack and contest at (§4.2).
    pub attack: Attack,

    // round-scoped budgets
    /// The one breadth pool (§3): initiative spent to act *and* to defend. Sized by Speed; refreshes
    /// each round. (Focus/Mind are merged out — defense is a Tempo spend.)
    pub tempo: i32,
    /// Round-scoped: a Lifeline (M3 *Last Stand*) — this round the Actor cannot be downed; damage
    /// that would down it leaves it at 1 Body (resolved in [`crate::combat::tally`]). Reset each round.
    pub cannot_fall: bool,
    /// Round-scoped **Stagger** (a Controller debuff): this round the Actor loses its action — it may
    /// not initiate a strike or play a card, nor strike back. Cleared at Refresh.
    pub stunned: bool,
    /// Round-scoped **Shove** (an Infiltrator/Controller debuff): this round the Actor is knocked out
    /// of melee — it cannot contest a melee blow (no strike-back; takes free hits). Cleared at Refresh.
    pub shoved: bool,
    /// Round-scoped **Disarm** (a Controller debuff): this round the Actor cannot play its role cards
    /// (its hand is fouled). Cleared at Refresh.
    pub disarmed: bool,
    /// Round-scoped **Empower** (a Support buff): bonus Power added to this Actor's strikes this round
    /// (§4 Salt — indirect offense; the force-multiplier amplifies allies' hits). Cleared at Refresh.
    pub power_bonus: u32,
    /// Round-scoped bookkeeping: has this Actor already taken its one free **Blitz** slip this round
    /// (§4 Infiltrator)? Cleared at Refresh.
    pub free_slip_used: bool,
    /// Finalized dead. Body reaching 0 is "mortally wounded" — death is tallied at the phase
    /// boundary, which sets this; once set the Actor is out of the fight.
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

    /// Does this Actor own an attack at `range` (so it can contest there, §4.2)?
    pub fn can_contest(&self, range: Range) -> bool {
        self.attack.has(range)
    }

    /// Can this Actor contest a blow at `range` **right now**, accounting for round-scoped status?
    /// A **Shoved** unit is knocked out of melee (no strike-back at melee); a **Stagger**ed unit
    /// loses its action entirely (no strike-back at any range).
    pub fn can_contest_now(&self, range: Range) -> bool {
        if self.stunned {
            return false;
        }
        if self.shoved && range == Range::Melee {
            return false;
        }
        self.can_contest(range)
    }

    /// Does this Actor carry the named power card (a passive ability, §4 powers)?
    pub fn has(&self, card: &str) -> bool {
        self.actions.iter().any(|c| c.name == card)
    }

    /// Refresh the Tempo pool and clear round-scoped defense + status state.
    pub fn refresh_round(&mut self) {
        self.tempo = self.offense.speed as i32;
        self.cannot_fall = false;
        self.stunned = false;
        self.shoved = false;
        self.disarmed = false;
        self.free_slip_used = false;
        self.power_bonus = 0;
        self.defense.end_round();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_profiles_contest_their_range() {
        assert!(Attack::Melee.has(Range::Melee));
        assert!(!Attack::Melee.has(Range::Ranged));
        assert!(Attack::Ranged.has(Range::Ranged));
        assert!(!Attack::Ranged.has(Range::Melee));
        assert!(Attack::Both.has(Range::Melee) && Attack::Both.has(Range::Ranged));
        assert!(!Attack::Neither.has(Range::Melee) && !Attack::Neither.has(Range::Ranged));
    }

    #[test]
    fn script_charges_then_strikes() {
        assert_eq!(Script::ChargeThenStrike { until: 2 }.pick(0), Move::Gather);
        assert_eq!(Script::ChargeThenStrike { until: 2 }.pick(2), Move::Strike);
    }
}
