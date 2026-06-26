//! Data-driven cards & effects.
//!
//! A Clash **Move** lives in [`crate::duel`]; an **Action card** carries effect(s) played in the
//! round. Each card declares a **zone behavior** ([`ZoneBehavior`], §5.3: Return / Spend / Lasting)
//! and optional **tags** (charge/combo interaction, §5.4). Cards are loaded from
//! `data/booklet.ron`, so numbers retune without recompiling; a card's magnitude flows through the
//! [`crate::stats`] pile→bar→pool pipeline (untyped Might, §2.2).

use serde::Deserialize;

use crate::currency::Currency;
use crate::zones::ZoneBehavior;

/// The §5.6 role-card taxonomy kind. `Stat` cards are [`crate::form::StatCard`]s (Form
/// attachments), not `Card`s, so they are not in this enum. `Mode` is defined-but-deferred
/// (M1, 2026-06-19): the first content builds capstones as `Spend`-zone `Base` cards instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum RoleKind {
    /// Played from Hand — the track's core effect.
    #[default]
    Base,
    /// Passive — auto-applies to its Base; never separately played (the scaling card).
    Modifier,
    /// Played — an alternative / charged Base (deferred; unused in the first content).
    Mode,
}

/// A single effect a card can carry. Magnitudes are the card's own printed values (§2.4) — there is
/// no signature force-multiplier stat.
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Effect {
    /// Deal `power` untyped **Might** damage (§2.2 — no type, no cut), per target.
    Damage { power: u32 },
    /// Add `tempo` to the holder this round — a defensive Guard boost (more initiative to answer
    /// blows) (M2, Wall L1 *Brace*).
    Guard { tempo: u32 },
    /// This round the holder **cannot fall**: damage that would down it leaves it at 1 health
    /// (M3, Wall L5 *Last Stand*).
    Lifeline,
    /// On a landed hit, the target loses its action this round.
    Stagger,
    /// Rip a card from the target's hand.
    Disarm,
    /// Break the target out of the line (a Shove).
    Shove,
    /// Drive the target from the line to the Rearguard this round (a Rout — a Controller status, §4).
    Rout,
    /// Turn a face-down card back up (a Recover).
    Recover,
    /// Bank +`amount` Cadence (extra tempo this round).
    BankCadence { amount: u32 },
    /// Restore `vitality` Health cards to the most-wounded ally (a Mend).
    Mend { vitality: u32 },
    /// Grant a melee attack to a defenseless ally for the round (a Ward, §4.2).
    Ward,
    /// Grant +`tempo` Tempo to an ally (a Haste).
    Haste { tempo: u32 },
    /// Raise allies' **Might** by `might` this round (an Empower — the Support buff's indirect
    /// offense; round-scoped, §4 Salt).
    Empower { might: u32 },
    /// Strip `tempo` Tempo from a foe (a Suppress).
    Suppress { tempo: u32 },
    /// Cut `cadence` Cadence from a foe (a Slow — cheaper to block/engage).
    Slow { cadence: u32 },
    /// Drain `tempo` from a foe (a Confuse) — scramble it so it has less initiative to act *or*
    /// defend (the merged-pool reframing of the old "can't block").
    Confuse { tempo: u32 },

    // ---- §10 / `power-catalog-rewrite.md` §1 utility-token effects (Stage D) ----
    /// **Mark** (Controller): place a Mark token on each target — **−`finesse` Finesse (floor 1)**
    /// while present (persistent for the combat).
    Mark { finesse: u32 },
    /// **Mire** (Controller): place a Mire token on each target — **−`cadence` Cadence (floor 1)**,
    /// shrinking the foe's Tempo pool (persistent for the combat).
    Mire { cadence: u32 },
    /// **Sunder** (Controller): place a Sunder token on each target — **−`toughness` Toughness
    /// (floor 1)** while present (persistent for the combat). Lowers the per-phase wall so the party's
    /// strikes crack a foe they otherwise could not out-burst — the amp / necessity-maker (§10).
    Sunder { toughness: u32 },
    /// **Defang** (Controller): place a Defang token on each target — **−`might` Might (floor 1 when
    /// defanged)** to its strike magnitude while present (persistent for the combat). Softens the foe's
    /// blows without dealing damage (§10).
    Defang { might: u32 },
    /// **Burn** (Artillery DoT): place `stacks` Burn tokens (each carrying `power` Might) on each
    /// target — at every Reckoning a token deals `power` into the bearer's Reckoning pile and is
    /// removed (caster-independent once placed).
    Burn { stacks: u32, power: u32 },
    /// **Brace** (Wall): place a Guard token on self — **+`toughness` Toughness** this round (per-round;
    /// cleared at the Lull). Distinct from the older [`Guard`](Effect::Guard) (Tempo) effect.
    Brace { toughness: u32 },
    /// **Cover** (Wall): self (a Wall) assigns a Cover token to one ally — **single-target** damage
    /// aimed at that ally **redirects to the Wall** (§4.5 spillover extended to a chosen ally); AoE
    /// still hits the ally.
    Cover,
    /// **Thorns** (Support): place a Thorns token (carrying `power` Might) on an ally — when that ally
    /// is **struck**, the attacker takes `power` into the **attacker's own** pile.
    Thorns { power: u32 },
    /// **Charge** (Infiltrator/Artillery): bank `amount` Charge tokens on the caster — the unit's next
    /// damage strike **consumes all Charge tokens for +1 Might each** (§5.4).
    Charge { amount: u32 },
    /// **Smoke** (Infiltrator): place a Smoke token on self — the unit's next charge **ignores the
    /// rear's Volley pre-empt** (a guaranteed breach); consumed on use.
    Smoke,
    /// **Silence** (Controller): cancel one enemy **deferred** (`resolve: Reckoning`) spell — a
    /// non-lethal disrupt (§4.6). Handled at [`crate::game`] (removes a `Deferred` entry).
    Silence,
    /// **Pin** (Artillery): suppressive fire that **denies a free enemy Vanguard its charge** this round
    /// — sets the target's lock so [`crate::combat::resolve_volley`] / charge declaration skips it (the
    /// space-control rider on the area cards, §10). Handled at [`crate::game`] (touches the round plan's
    /// lock list, not [`play_card`]) — like [`Silence`](Effect::Silence).
    Pin,
}

/// §4.6 — the **cast window**: where an ability may be used (Tempo paid & committed). Abilities are
/// open, repeatable, tempo-gated ("Form open, bid hidden").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Cast {
    /// The Standoff — own-side buffs / braces, auto-land before the Fray.
    Standing,
    /// The **Strike window** = the Fray *and* the Volley; a card usable in one is usable in both.
    #[default]
    Strike,
}

/// §4.6 — the **resolution gate**: which phase's pile an ability's effect lands in. The disruption
/// window is `resolve − cast` measured in gates (`OnCast` ⇒ zero ⇒ undisruptable, §1.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Resolve {
    /// Resolves in the phase it was used (the old *instant*).
    #[default]
    OnCast,
    /// Lands in the Breach (a charge).
    Breach,
    /// Lands in the Reckoning (a deferred spell / a DoT tick).
    Reckoning,
}

/// An Action card: its effect(s), how many foes it hits, its §5 zone behavior, and tags.
#[derive(Clone, Debug, Deserialize)]
pub struct Card {
    pub name: String,
    /// A human rules description, used to generate the encyclopedia's **Powers** entries from
    /// card data (the card is the source of truth for what it does). Empty for plain weapons.
    #[serde(default)]
    pub text: String,
    /// Evocative in-world flavor — prose, not rules (the `text` field carries the mechanics).
    /// Lives entirely in `data/booklet.ron`; never authored in code.
    #[serde(default)]
    pub flavor: String,
    /// Distinct foes hit (AoE). 1 = single target.
    #[serde(default = "one")]
    pub targets: u32,
    /// Reach in jumps `[min, max]` (melee `[1,1]`, ranged `[2,2]`). Informational
    /// for now; positioning is approximated.
    #[serde(default = "melee")]
    pub reach: [u32; 2],
    /// §5.3 — what the card does to itself after it is played (default **Return** to Hand).
    #[serde(default)]
    pub zone: ZoneBehavior,
    /// §5.4 — type tags for charge/combo interaction (e.g. `["Charge(fire)"]`). Empty by default.
    #[serde(default)]
    pub tags: Vec<String>,
    /// A passive ability (a §4 power detected by name) rather than a played effect card.
    #[serde(default)]
    pub passive: bool,
    #[serde(default)]
    pub effects: Vec<Effect>,
    // ---- §4.6 timing (cast / resolve); defaults = the old "instant", usable in the Strike window ----
    /// §4.6 — the window this ability may be used in (default `Strike` = the Fray *and* Volley).
    #[serde(default)]
    pub cast: Cast,
    /// §4.6 — which phase's pile the effect resolves into (default `OnCast`).
    #[serde(default)]
    pub resolve: Resolve,
    /// §4.6 — a one-shot: flips face-down for the whole combat when used (never resets). The
    /// tempo-gated replacement for `zone: Spend` on abilities.
    #[serde(default)]
    pub one_shot: bool,
    // ---- role-card metadata (§5.6 / §4.4); defaults keep plain cards role-free ----
    /// The role track this card belongs to (§8.3) — `None` for non-role cards (weapons, the
    /// pre-built scenario kits).
    #[serde(default)]
    pub role: Option<Currency>,
    /// Which taxonomy kind (§5.6).
    #[serde(default)]
    pub kind: RoleKind,
    /// A **positional** role card (Wall / Infiltrator / Artillery) — playable only from the
    /// matching §4 position (§4.4 D2). Effect cards (Support / Controller) are position-agnostic.
    #[serde(default)]
    pub positional: bool,
    /// A `Modifier` names the Base it auto-applies to when both are owned (§5.6); folded at
    /// build time (e.g. Curse → +1 debuff target).
    #[serde(default)]
    pub modifies: Option<String>,
}

fn one() -> u32 {
    1
}
fn melee() -> [u32; 2] {
    [1, 1]
}

impl Card {
    /// The card's primary damage (untyped Might power), if it deals damage. This is what
    /// Force scales when the card is Unleashed/Overwhelmed.
    pub fn primary_damage(&self) -> Option<u32> {
        self.effects.iter().find_map(|e| match e {
            Effect::Damage { power } => Some(*power),
            _ => None,
        })
    }

    pub fn has_stagger(&self) -> bool {
        self.effects.iter().any(|e| matches!(e, Effect::Stagger))
    }

    /// A short one-line summary for the card UI.
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for e in &self.effects {
            parts.push(match e {
                Effect::Damage { power } => format!("might {power}"),
                Effect::Guard { tempo } => format!("brace +{tempo} tempo"),
                Effect::Lifeline => "cannot fall".into(),
                Effect::Stagger => "stagger".into(),
                Effect::Disarm => "disarm".into(),
                Effect::Shove => "shove".into(),
                Effect::Rout => "rout".into(),
                Effect::Recover => "recover".into(),
                Effect::BankCadence { amount } => format!("+{amount} cadence"),
                Effect::Mend { vitality } => format!("mend +{vitality}"),
                Effect::Ward => "ward (grant melee)".into(),
                Effect::Haste { tempo } => format!("haste +{tempo}"),
                Effect::Empower { might } => format!("empower +{might} might"),
                Effect::Suppress { tempo } => format!("suppress -{tempo} tempo"),
                Effect::Slow { cadence } => format!("slow -{cadence} cadence"),
                Effect::Confuse { tempo } => format!("confuse -{tempo} tempo"),
                Effect::Mark { finesse } => format!("mark -{finesse} finesse"),
                Effect::Mire { cadence } => format!("mire -{cadence} cadence"),
                Effect::Sunder { toughness } => format!("sunder -{toughness} tough"),
                Effect::Defang { might } => format!("defang -{might} might"),
                Effect::Burn { stacks, power } => format!("burn {stacks}x{power}"),
                Effect::Brace { toughness } => format!("brace +{toughness} tough"),
                Effect::Cover => "cover an ally".into(),
                Effect::Thorns { power } => format!("thorns {power}"),
                Effect::Charge { amount } => format!("charge +{amount}"),
                Effect::Smoke => "smoke".into(),
                Effect::Silence => "silence".into(),
                Effect::Pin => "pin (deny a charge)".into(),
            });
        }
        if self.targets > 1 {
            parts.push(format!("x{} targets", self.targets));
        }
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn firestorm() -> Card {
        Card {
            name: "Firestorm".into(),
            text: String::new(),
            flavor: String::new(),
            targets: 5,
            reach: [2, 2],
            zone: ZoneBehavior::Spend,
            tags: vec![],
            passive: false,
            effects: vec![Effect::Damage { power: 5 }],
            cast: Cast::Strike,
            resolve: Resolve::OnCast,
            one_shot: false,
            role: None,
            kind: RoleKind::Base,
            positional: false,
            modifies: None,
        }
    }

    #[test]
    fn primary_damage_reads_the_first_damage_effect() {
        let p = firestorm().primary_damage().unwrap();
        assert_eq!(p, 5);
    }

    #[test]
    fn summary_mentions_aoe() {
        assert!(firestorm().summary().contains("targets"));
    }
}
