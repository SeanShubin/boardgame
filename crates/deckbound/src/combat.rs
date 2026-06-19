//! Combat resolution helpers for the §4 lane system: typed strikes, the **range** rule
//! (same-range trade vs auto-hit, §4.2), creature AI for the commitment phases, and card
//! effects. The interactive four-card Clash ([`crate::duel`]) is the optional 1v1 module.

use crate::actor::{Actor, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::stats::{Break, DamageType};

/// The base strike profile: `(raw, damage type, precision)`. Power is the magnitude; the
/// weapon supplies the type; a Damage card adds its own power.
pub fn base_strike(a: &Actor) -> (u32, DamageType, u32) {
    let (card_pow, dtype) = a.weapon.primary_damage().unwrap_or((0, DamageType::Blunt));
    (a.offense.power + card_pow, dtype, a.offense.precision)
}

/// A base [`Strike`] snapshot (for order-independent resolution from phase-start state).
pub fn snapshot(a: &Actor) -> Strike {
    let (raw, dtype, precision) = base_strike(a);
    Strike {
        raw,
        dtype,
        precision,
    }
}

/// Route a strike through the target's defense and narrate it.
pub fn apply_strike(target: &mut Actor, strike: Strike, attacker: &str, log: &mut Vec<String>) {
    let out = target
        .defense
        .take(strike.raw, strike.dtype, strike.precision);
    if out.cards_flipped > 0 {
        log.push(format!(
            "  {attacker} hits {} for {} ({} {}).",
            target.name,
            out.cards_flipped,
            strike.raw,
            strike.dtype.label()
        ));
    }
    if let Some(b) = out.broke {
        log.push(format!("  {} {}!", target.name, break_note(b)));
    }
    if out.down {
        log.push(format!("{} falls!", target.name));
    }
}

fn break_note(b: Break) -> &'static str {
    match b {
        Break::Freeze => "freezes (will broken this round)",
        Break::Flee => "is routed and flees",
        Break::ScaredToDeath => "is scared to death",
        Break::Blind => "is confused — blind this round",
    }
}

/// Pick a target index among `candidates` (indices into `pool`) per a target rule (§4).
pub fn pick_target(pool: &[Actor], candidates: &[usize], rule: TargetRule) -> Option<usize> {
    match rule {
        TargetRule::Front => candidates.first().copied(),
        TargetRule::LowestBody => candidates
            .iter()
            .copied()
            .min_by_key(|&i| pool[i].defense.body.remaining),
        TargetRule::LeastResolute => candidates
            .iter()
            .copied()
            .min_by_key(|&i| pool[i].defense.resolve),
    }
}

/// Living indices of a pool.
pub fn living(pool: &[Actor]) -> Vec<usize> {
    pool.iter()
        .enumerate()
        .filter(|(_, a)| !a.is_down())
        .map(|(i, _)| i)
        .collect()
}

/// Finalize deaths at a phase boundary: a downed Actor becomes `fallen` — unless it has a
/// Lifeline this round (M3 *Last Stand*), which leaves it standing at 1 Body instead.
pub fn tally(pool: &mut [Actor]) {
    for a in pool.iter_mut() {
        if a.is_down() {
            if a.cannot_fall {
                a.defense.body.remaining = a.defense.body.remaining.max(1);
            } else {
                a.fallen = true;
            }
        }
    }
}

/// Apply a hero's action/power card. The deterministic effects (§"cards may supersede the
/// core") are wired here; foes use the same path. `foes`/`allies` are the opposing and
/// friendly pools.
#[allow(clippy::too_many_arguments)]
pub fn play_card(
    card: &crate::cards::Card,
    actor_name: &str,
    actor_power: u32,
    actor_precision: u32,
    foes: &mut [Actor],
    allies: &mut [Actor],
    self_idx: Option<usize>,
    log: &mut Vec<String>,
) {
    log.push(format!("{actor_name} plays {}.", card.name));
    // How many foes / allies an effect touches (§4 AoE); ≥1. A Curse modifier (M5) and Sanctuary
    // (M6) raise this via the card's `targets` at build time.
    let n = (card.targets as usize).max(1);
    for effect in &card.effects {
        match *effect {
            Effect::Damage { power, dtype } => {
                let raw = actor_power + power;
                let alive: Vec<usize> = living(foes);
                for ti in alive.into_iter().take(n) {
                    apply_strike(
                        &mut foes[ti],
                        Strike {
                            raw,
                            dtype,
                            precision: actor_precision,
                        },
                        actor_name,
                        log,
                    );
                }
            }
            Effect::Guard { focus } => {
                // M2 — a defensive boost to this holder's block vs slips this round.
                if let Some(i) = self_idx {
                    allies[i].focus += focus;
                    log.push(format!("  braces (+{focus} Focus)."));
                }
            }
            Effect::Lifeline => {
                // M3 — this round the holder cannot fall (damage leaves it at 1 Body); resolved
                // in `tally` at the phase boundary.
                if let Some(i) = self_idx {
                    allies[i].cannot_fall = true;
                    log.push("  steels for a last stand — it cannot fall this round.".into());
                }
            }
            Effect::Rally { resolve } => {
                for a in allies.iter_mut().filter(|a| !a.is_down()) {
                    a.defense.resolve += resolve;
                }
                log.push(format!("  +{resolve} Resolve to allies."));
            }
            Effect::Mend { body } => {
                // Heal the `n` most-wounded allies (M6 Sanctuary heals all).
                let mut order: Vec<usize> = living(allies);
                order.sort_by_key(|&i| allies[i].defense.body.remaining);
                for ai in order.into_iter().take(n) {
                    let max = allies[ai].defense.body.max;
                    let r = &mut allies[ai].defense.body.remaining;
                    *r = (*r + body).min(max);
                    log.push(format!("  mends {} (+{body} Body).", allies[ai].name));
                }
            }
            Effect::Ward => {
                // Grant a melee guard to `n` melee-less allies so they can self-defend (§4.2).
                use crate::actor::Attack;
                let mut granted = 0;
                for (i, t) in allies.iter_mut().enumerate() {
                    if granted >= n {
                        break;
                    }
                    if t.is_down() || Some(i) == self_idx {
                        continue;
                    }
                    if matches!(t.attack, Attack::Ranged | Attack::Neither) {
                        t.attack = match t.attack {
                            Attack::Ranged => Attack::Both,
                            _ => Attack::Melee,
                        };
                        log.push(format!("  wards {} (gains a melee guard).", t.name));
                        granted += 1;
                    }
                }
            }
            Effect::Haste { tempo } => {
                for t in allies.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo += tempo as i32;
                    log.push(format!("  +{tempo} Tempo to {}.", t.name));
                }
            }
            Effect::Suppress { tempo } => {
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo -= tempo as i32;
                    log.push(format!("  suppresses {} (-{tempo} Tempo).", t.name));
                }
            }
            Effect::Slow { speed } => {
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.offense.speed = t.offense.speed.saturating_sub(speed);
                    log.push(format!("  slows {} (-{speed} Speed).", t.name));
                }
            }
            Effect::Confuse { focus } => {
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.focus = t.focus.saturating_sub(focus);
                    log.push(format!("  confuses {} (-{focus} Focus).", t.name));
                }
            }
            Effect::Sunder { armor } => {
                if let Some(t) = foes.iter_mut().find(|a| !a.is_down()) {
                    for v in t.defense.armor.values_mut() {
                        *v = v.saturating_sub(armor);
                    }
                    log.push(format!("  sunders {}'s armor.", t.name));
                }
            }
            Effect::Steel => {
                if let Some(i) = self_idx {
                    allies[i].defense.fear_pile = 0;
                    allies[i].defense.will_break = None;
                }
                log.push("  nerves settle.".into());
            }
            Effect::BankSpeed { amount } => {
                if let Some(i) = self_idx {
                    allies[i].tempo += amount as i32;
                }
                log.push(format!("  +{amount} Tempo banked."));
            }
            Effect::Stagger | Effect::Disarm | Effect::Shove | Effect::Recover => {
                log.push("  (effect applied)".into());
            }
        }
    }
}

/// Can `defender` contest a strike at `range` (§4.2)? If not, the strike is an auto-hit.
pub fn contests(defender: &Actor, range: Range) -> bool {
    defender.can_contest(range)
}
