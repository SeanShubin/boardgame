//! Combat resolution helpers for the §4 charge-and-gauntlet system: typed strikes, the **range**
//! rule (same-range trade vs auto-hit, §4.2), the gauntlet resolver, creature AI, and card effects.
//! The interactive four-card Clash ([`crate::duel`]) is the optional 1v1 module.

use crate::actor::{Actor, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::stats::{Break, Channel, DamageType};

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

/// Route a strike through the target's defense and **narrate it as card-state transitions** — there is
/// no "life total": damage accumulates, and the only states are *health cards turning face down* and,
/// at the phase boundary, *all of them face down → defeated*. So a strike reads as one of: turned
/// aside (no card moves), damage accumulating (not yet enough to turn a card), or **N health cards
/// turn face down**. Defeat is narrated once, at the boundary (see `tally`), never here.
pub fn apply_strike(target: &mut Actor, strike: Strike, attacker: &str, log: &mut Vec<String>) {
    let dt = strike.dtype.label();
    let inner = !matches!(strike.dtype.channel(), Channel::Body); // Fear / Confusion break the will/mind
    // Every strike is narrated — the log is how the player verifies the mechanics and learns who
    // acted, so a blow is never silently dropped. Overkill (a simultaneous-phase blow on a target
    // whose health cards are already all face down) is reported as such, not applied again.
    if target.is_down() {
        log.push(format!(
            "  {attacker} hits {}: {} {dt} — its health cards are already all face down.",
            target.name, strike.raw
        ));
        return;
    }
    let out = target
        .defense
        .take(strike.raw, strike.dtype, strike.precision);
    let name = &target.name;
    let what = if out.cards_flipped == 1 {
        " — turns a health card face down.".to_string()
    } else if out.cards_flipped > 1 {
        format!(" — turns {} health cards face down.", out.cards_flipped)
    } else if out.through == 0 {
        // Nothing got through the cut — no card moves.
        if inner {
            " — warded off.".to_string()
        } else {
            " — turned aside by its armor.".to_string()
        }
    } else if inner {
        // Got through to the will/mind pile; the break (if any) is narrated below.
        " — the dread mounts.".to_string()
    } else {
        // Through the armor and accumulating, but not yet a full health card's worth.
        " — damage accumulates.".to_string()
    };
    log.push(format!(
        "  {attacker} hits {name}: {} {dt}{what}",
        strike.raw
    ));
    if let Some(b) = out.broke {
        log.push(format!("  {name} {}!", break_note(b)));
    }
    // Defeat is *not* narrated here: a phase resolves order-independently from snapshots, so several
    // strikes may land on the same target. "All health cards face down → falls" is reported once, when
    // the phase boundary finalizes it (see `tally`) — by then any same-phase healing has netted out.
}

fn break_note(b: Break) -> &'static str {
    match b {
        Break::Freeze => "freezes (will broken this round)",
        Break::Flee => "is routed and flees",
        Break::ScaredToDeath => "is scared to death",
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

/// Finalize deaths at a phase boundary: an Actor whose keystone is gone becomes `fallen` — unless it
/// has a Lifeline this round (M3 *Last Stand*), which leaves it standing at 1 Body instead. This is
/// the **single** place a fall is decided and narrated (once per Actor), after the phase's
/// order-independent damage has fully accumulated — so it reflects the net result, not a mid-stream
/// overkill.
pub fn tally(pool: &mut [Actor], log: &mut Vec<String>) {
    for a in pool.iter_mut() {
        if a.is_down() && !a.fallen {
            if a.cannot_fall {
                a.defense.body.remaining = a.defense.body.remaining.max(1);
            } else {
                a.fallen = true;
                log.push(format!(
                    "{} — all its health cards are face down; defeated.",
                    a.name
                ));
            }
        }
    }
}

/// Resolve the **gauntlet** (§4): the two charge-columns thread through each other. Each side's
/// chargers (in roster order) pair off by order; at each crossing the **faster slips past** (becomes
/// a Skirmisher) while the one it passes lands a **parting free hit**; **equal speed → both stop and
/// trade** (both become Vanguard); **surplus chargers** on the longer column meet no opposition and
/// **break through** cleanly. A charger that stops (or never broke through) is a Vanguard; a
/// non-charger is a Reserve. Returns `(hero_skirmisher, foe_skirmisher)` — who broke through.
///
/// *(v1: the crossing contest uses Speed as the tempo stand-in and auto-resolves; the interactive
/// per-crossing tempo auction is a later enrichment. Damage applies from pre-crossing snapshots.)*
pub fn gauntlet(
    heroes: &mut [Actor],
    hero_charging: &[bool],
    foes: &mut [Actor],
    foe_charging: &[bool],
    log: &mut Vec<String>,
) -> (Vec<bool>, Vec<bool>) {
    let mut hero_skirm = vec![false; heroes.len()];
    let mut foe_skirm = vec![false; foes.len()];
    let h_chargers: Vec<usize> = (0..heroes.len())
        .filter(|&i| hero_charging[i] && !heroes[i].is_down())
        .collect();
    let f_chargers: Vec<usize> = (0..foes.len())
        .filter(|&i| foe_charging[i] && !foes[i].is_down())
        .collect();
    let pairs = h_chargers.len().min(f_chargers.len());

    for k in 0..pairs {
        let h = h_chargers[k];
        let f = f_chargers[k];
        let hs = heroes[h].offense.speed;
        let fs = foes[f].offense.speed;
        if hs > fs {
            // Hero slips past; the foe it passes lands a parting blow.
            let snap = snapshot(&foes[f]);
            let name = foes[f].name.clone();
            apply_strike(&mut heroes[h], snap, &name, log);
            if !heroes[h].is_down() {
                hero_skirm[h] = true;
                log.push(format!("{} breaks through the line!", heroes[h].name));
            }
        } else if fs > hs {
            let snap = snapshot(&heroes[h]);
            let name = heroes[h].name.clone();
            apply_strike(&mut foes[f], snap, &name, log);
            if !foes[f].is_down() {
                foe_skirm[f] = true;
            }
        } else {
            // Equal speed → both stop and trade (both become Vanguard).
            let hsnap = snapshot(&heroes[h]);
            let fsnap = snapshot(&foes[f]);
            let hname = heroes[h].name.clone();
            let fname = foes[f].name.clone();
            apply_strike(&mut heroes[h], fsnap, &fname, log);
            apply_strike(&mut foes[f], hsnap, &hname, log);
        }
    }
    // Surplus chargers on the longer column meet no opposition → break through cleanly.
    for &h in h_chargers.iter().skip(pairs) {
        hero_skirm[h] = true;
        log.push(format!(
            "{} runs an open gauntlet and breaks through!",
            heroes[h].name
        ));
    }
    for &f in f_chargers.iter().skip(pairs) {
        foe_skirm[f] = true;
    }
    (hero_skirm, foe_skirm)
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
            Effect::Guard { tempo } => {
                // M2 (Brace) — a defensive boost: extra Tempo this round to answer blows.
                if let Some(i) = self_idx {
                    allies[i].tempo += tempo as i32;
                    log.push(format!("  braces (+{tempo} Tempo)."));
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
            Effect::Confuse { tempo } => {
                // Drain a foe's Tempo — less initiative to act *or* defend (merged-pool reframing).
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo -= tempo as i32;
                    log.push(format!("  confuses {} (-{tempo} Tempo).", t.name));
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
