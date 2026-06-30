//! Combatants — **Actors** — their attack profile, and how creatures decide.
//!
//! An Actor is the umbrella (see `docs/games/deckbound/notes/entities.md`): a **Character**
//! is human-driven; a **Creature** follows a scripted `Behavior`. Both carry the full stat
//! block — [`Offense`](crate::stats::Offense) / [`Defense`](crate::stats::Defense) — a weapon,
//! action cards, and the round's **Tempo** pool (= Cadence; the single breadth budget since the
//! Focus/Mind merge, §3). Each Actor also has an **attack profile** (§4.2): the range(s) it can
//! strike and contest at, plus round-scoped **status** (Stagger / Shove / Disarm) set by Controller
//! cards and cleared at Refresh.

use engine::Rng;
use serde::{Deserialize, Serialize};

use crate::cards::Card;
use crate::duel::Move;
use crate::form::Form;
use crate::stats::{Defense, Offense};

/// The range of an engagement (§4.2). Position-determined: Vanguard and Outrider strikes are
/// melee; Rearguard fire is ranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Range {
    Melee,
    Ranged,
}

/// A unit's declared **intention** for the round (§4) — the position it takes, and the role it plays in
/// the engagement schedule (§4.6). Re-declared each round; declaring is free and may *fail* (force-not-fiat).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Intention {
    /// Hold the line (front): the shield; screens enemy Outriders, fights the front, cleans up last.
    Vanguard,
    /// Break the line (flank): raids the enemy Rearguard directly, exposed to the enemy front and back first.
    Outrider,
    /// Deal from the back: fires/buffs/degrades from safety; the only answer to a Vanguard's Toughness.
    Rearguard,
}

impl Intention {
    /// The role this intention is **designed to beat** (its cycle prey, Hold▸Break▸Deal▸Hold) — the
    /// efficient default spends scarce Tempo on its prey first, falling back only when none is crackable.
    pub fn prey(self) -> Intention {
        match self {
            Intention::Vanguard => Intention::Outrider,
            Intention::Outrider => Intention::Rearguard,
            Intention::Rearguard => Intention::Vanguard,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Intention::Vanguard => "Vanguard",
            Intention::Outrider => "Outrider",
            Intention::Rearguard => "Rearguard",
        }
    }
}

/// A **utility token** (§10 / `power-catalog-rewrite.md` §1): card-tracked state placed on an Actor.
/// Tokens make persistent / charging / accumulating state **physical** (§5.1 cards-only — never human
/// memory). Each token sits on its bearer; ALL tokens clear on the bearer's death and at combat end,
/// and the per-round **Guard** token is additionally cleared at the Lull (`refresh_round`).
///
/// Floors (§2.2 force-not-fiat): Mark/Mire each clamp their stat at **min 1** independently, so a
/// maximally Marked+Mired foe still has Finesse 1 **and** Cadence 1 → Tempo ≥ 1 — a debuff stack can
/// never lock a foe (no second kill-condition).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Token {
    /// **Guard** (Wall): +Toughness while present (raises the per-phase wall). Per-round — cleared at
    /// the Lull (`refresh_round`).
    Guard { toughness: u32 },
    /// **Cover** (Wall): the bearer is a Wall covering ally index `ally` — single-target damage aimed
    /// at that ally **redirects to this Wall** (§4.5 spillover extended to a chosen ally). AoE still
    /// hits the ally directly.
    Cover { ally: usize },
    /// **Mark** (Controller): −`finesse` Finesse (floor 1) while present.
    Mark { finesse: u32 },
    /// **Mire** (Controller): −`cadence` Cadence (floor 1) while present (fewer Tempo cards).
    Mire { cadence: u32 },
    /// **Sunder** (Controller): −`toughness` Toughness (floor 1) while present — lowers the per-phase
    /// wall this body presents, so the party cracks it with less Might.
    Sunder { toughness: u32 },
    /// **Defang** (Controller): −`might` Might (floor 1) to this body's strike magnitude while present.
    Defang { might: u32 },
    /// **Burn** (Artillery DoT): each Reckoning, deal `power` Might into the bearer's per-engagement
    /// pile (it ticks in the last engagement, the Breach) and remove one stack. Caster-independent once placed.
    Burn { power: u32 },
    /// **Thorns** (Support): when this ally is struck, the attacker takes `power` Might into the
    /// attacker's own current-phase pile (Support's reflected "offense").
    Thorns { power: u32 },
    /// **Charge** (Infiltrator/Artillery): one banked step of magnitude; the unit's next damage strike
    /// **consumes all Charge tokens for +1 Might each** (§5.4 — burst paid for by the setup round).
    Charge,
    /// **Smoke** (Infiltrator): the unit's next charge **ignores the rear's Volley pre-empt** (a
    /// guaranteed breach); consumed on use.
    Smoke,
}

/// What range(s) an Actor can attack and contest at (§4.2). A strike at a range the target
/// cannot answer is an **auto-hit**; a same-range meeting is a trade (or a Clash).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Driver {
    /// A Character: the human (or a stand-in) improvises.
    Human,
    /// A Creature: a scripted instinct.
    Creature(Behavior),
}

/// A creature's policy: how eagerly it commits to the Vanguard, whom it targets, and (only
/// when the Clash module is on) how it plays the four-card mix-up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Behavior {
    /// 0..=10 — higher commits more Actors to the Vanguard / slips more readily.
    pub aggression: u32,
    pub target_rule: TargetRule,
    /// Clash instinct — used only when the optional Clash module is enabled.
    pub instinct: Instinct,
    /// §13 enemy roles — Health cards this creature **mends** on its most-wounded ally each Fray instead
    /// of attacking (`0` = not a healer).
    pub heal: u32,
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Instinct {
    Deck(Vec<Move>),
    Script(Script),
}

/// A deterministic Clash algorithm (tutorial dummies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetRule {
    /// The first reachable enemy.
    Front,
    /// The most fragile (fewest health cards).
    LowestBody,
}

/// A combatant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    pub role: String,
    pub offense: Offense,
    pub defense: Defense,
    /// The **Form** (§2.3 stats-as-deck): the stat cards this Actor's `offense`/`defense` are summed
    /// from — the `Human` baseline plus each treasure's Stat card (and any trait / scaling). Retained
    /// so a build is auditable: the totals are *derivable* from these cards. Not used in resolution.
    pub form: Form,
    /// The base strike profile (the actor's base attack card).
    pub weapon: Card,
    /// Action/power cards playable in the round (§"cards may supersede the core").
    pub actions: Vec<Card>,
    pub driver: Driver,
    /// Range(s) this Actor can attack and contest at (§4.2).
    pub attack: Attack,

    /// §4.5 **area** strike: when `true`, this Actor's strike hits **every** member of the target's
    /// group at full Might, is **unevadable**, and bypasses the spillover bodyguard (it banks into the
    /// [`crate::stats::PendingDamage::aoe`] pool). `false` = a single aimed blow. No live creature sets
    /// this yet, so the AoE pool stays 0 in existing scenarios. Mirrors the sim's `Unit.aoe`.
    #[serde(default)]
    pub aoe: bool,

    /// §10 **utility tokens** placed on this Actor — card-tracked persistent state (§5.1). Mark / Mire
    /// / Burn / Thorns / Cover persist for the combat; **Guard** is per-round (cleared at the Lull).
    /// ALL tokens clear on this Actor's death and at combat end. See [`Token`].
    pub tokens: Vec<Token>,

    // round-scoped budgets
    /// The one breadth pool (§3): initiative spent to act *and* to defend. Sized by Cadence; refreshes
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
    /// Round-scoped **Rout** (a Controller debuff, §4 / Charter #13): this round the Actor is driven
    /// from the line to the Rearguard — it neither holds as a Vanguard nor charges across the gap.
    /// Cleared at Refresh.
    pub routed: bool,
    /// Round-scoped **Empower** (a Support buff): bonus Might added to this Actor's strikes this round
    /// (§4 Salt — indirect offense; amplifies allies' hits). Cleared at Refresh.
    pub might_bonus: u32,
    /// Round-scoped bookkeeping: has this Actor already taken its one free **Blitz** slip this round
    /// (§4 Infiltrator)? Cleared at Refresh.
    pub free_slip_used: bool,
    /// Finalized dead. Body reaching 0 is "mortally wounded" — death is tallied at the phase
    /// boundary, which sets this; once set the Actor is out of the fight.
    pub fallen: bool,
    /// §4.6 **one-shot** bookkeeping: the names of `one_shot: true` cards this Actor has already used
    /// this combat — flipped face-down for the rest of the fight (never reset by `refresh_round`; the
    /// tempo-gated replacement for `zone: Spend`). A one-shot whose name is listed is no longer playable.
    #[serde(default)]
    pub spent_one_shots: Vec<String>,
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

    // ---- §10 utility tokens (card-tracked state) ----

    /// The **effective Finesse** read in bids/contests: base Finesse minus all **Mark** tokens, floored
    /// at 1 (§2.2 — Marks can never lock; the stat saturates at the floor). Offense reads consult this.
    pub fn eff_finesse(&self) -> u32 {
        let drop: u32 = self
            .tokens
            .iter()
            .map(|t| match t {
                Token::Mark { finesse } => *finesse,
                _ => 0,
            })
            .sum();
        self.offense.finesse.saturating_sub(drop).max(1)
    }

    /// The **effective Cadence** (the Tempo-pool size at refresh): base Cadence minus all **Mire**
    /// tokens. A Mire never floors **below 1** (§2.2 — Mire can never lock; the stat saturates at the
    /// floor). With no Mire present the base value passes through unchanged (a genuine 0-Cadence body is
    /// not floored up — only the *debuff* is clamped).
    pub fn eff_cadence(&self) -> u32 {
        let drop: u32 = self
            .tokens
            .iter()
            .map(|t| match t {
                Token::Mire { cadence } => *cadence,
                _ => 0,
            })
            .sum();
        if drop == 0 {
            self.offense.cadence
        } else {
            self.offense.cadence.saturating_sub(drop).max(1)
        }
    }

    /// The **effective Toughness** read as the per-phase wall: base Toughness minus all **Sunder**
    /// tokens, floored at 1 when sundered (§2.2 — Sunder can never drop the wall to 0, which would flip
    /// every card at once). With no Sunder present the base value passes through unchanged (a genuine
    /// Toughness-0 body is not floored up — only the *debuff* is clamped, like [`eff_cadence`]). The
    /// per-round **Guard** token bonus is folded in by the strike path (`apply_strike`), not here.
    pub fn eff_toughness(&self) -> u32 {
        let drop: u32 = self
            .tokens
            .iter()
            .map(|t| match t {
                Token::Sunder { toughness } => *toughness,
                _ => 0,
            })
            .sum();
        if drop == 0 {
            self.defense.health.toughness()
        } else {
            self.defense.health.toughness().saturating_sub(drop).max(1)
        }
    }

    /// The **effective Might** of this Actor's strikes: base Might minus all **Defang** tokens, floored
    /// at 1 when defanged (§2.2 — Defang can never silence a foe to a 0 blow). With no Defang present the
    /// base value passes through unchanged (a genuine Might-0 body is not floored up — only the *debuff*
    /// is clamped, like [`eff_cadence`]). The strike path (`base_strike`) reads this.
    pub fn eff_might(&self) -> u32 {
        let drop: u32 = self
            .tokens
            .iter()
            .map(|t| match t {
                Token::Defang { might } => *might,
                _ => 0,
            })
            .sum();
        if drop == 0 {
            self.offense.might
        } else {
            self.offense.might.saturating_sub(drop).max(1)
        }
    }

    /// The total **Guard** token Toughness on this Actor (added to the per-phase wall in the pile
    /// pipeline; per-round, cleared at the Lull).
    pub fn guard_toughness(&self) -> u32 {
        self.tokens
            .iter()
            .map(|t| match t {
                Token::Guard { toughness } => *toughness,
                _ => 0,
            })
            .sum()
    }

    /// The total banked **Charge** count on this Actor (consumed for +1 Might each by the next strike).
    pub fn charge_count(&self) -> u32 {
        self.tokens
            .iter()
            .filter(|t| matches!(t, Token::Charge))
            .count() as u32
    }

    /// Consume **all** Charge tokens, returning the count (the next strike's +Might, §5.4).
    pub fn drain_charges(&mut self) -> u32 {
        let n = self.charge_count();
        self.tokens.retain(|t| !matches!(t, Token::Charge));
        n
    }

    /// Is a **Smoke** token present (the next charge ignores the Volley pre-empt)?
    pub fn has_smoke(&self) -> bool {
        self.tokens.iter().any(|t| matches!(t, Token::Smoke))
    }

    /// Consume one **Smoke** token (on a charge); returns `true` if one was present.
    pub fn consume_smoke(&mut self) -> bool {
        if let Some(p) = self.tokens.iter().position(|t| matches!(t, Token::Smoke)) {
            self.tokens.remove(p);
            true
        } else {
            false
        }
    }

    /// The total **Thorns** reflect power on this Actor (Might bounced onto an attacker's own pile).
    pub fn thorns_power(&self) -> u32 {
        self.tokens
            .iter()
            .map(|t| match t {
                Token::Thorns { power } => *power,
                _ => 0,
            })
            .sum()
    }

    /// Refresh the Tempo pool and clear round-scoped defense + status state. The pool is sized by
    /// **effective Cadence** (base − Mire tokens, floor 1, §2.2), so a mired foe refreshes fewer Tempo
    /// cards. The per-round **Guard** token is cleared here (Mark/Mire/Burn/Thorns/Cover persist for the
    /// combat — they clear only on death / combat end).
    pub fn refresh_round(&mut self) {
        self.tokens.retain(|t| !matches!(t, Token::Guard { .. }));
        self.tempo = self.eff_cadence() as i32;
        self.cannot_fall = false;
        self.stunned = false;
        self.shoved = false;
        self.disarmed = false;
        self.routed = false;
        self.free_slip_used = false;
        self.might_bonus = 0;
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
