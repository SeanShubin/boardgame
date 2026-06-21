//! Combat resolution helpers for the §4 charge-and-gauntlet system: typed strikes, the **range**
//! rule (same-range trade vs auto-hit, §4.2), the gauntlet resolver, creature AI, and card effects.
//! The interactive four-card Clash ([`crate::duel`]) is the optional 1v1 module.

use crate::actor::{Actor, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::stats::{Break, Channel, DamageType, Offense};

/// The attack magnitude an actor brings to a damage type's **channel**: an outer (Body) strike scales
/// off **Strike** (`power`), an inner (Fear) strike off **Dread** — the two channels' parallel attack
/// stats (Spec §2.2 / §2.4). Card power and round buffs add on top.
fn channel_attack(off: Offense, dtype: DamageType) -> u32 {
    match dtype.channel() {
        Channel::Body => off.power,
        Channel::Fear => off.dread,
    }
}

/// The base strike profile: `(raw, damage type, precision)`. The channel's attack stat is the
/// magnitude; the weapon supplies the type; a Damage card adds its own power.
pub fn base_strike(a: &Actor) -> (u32, DamageType, u32) {
    let (card_pow, dtype) = a.weapon.primary_damage().unwrap_or((0, DamageType::Blunt));
    // `power_bonus` is this round's Empower (a Support buff, §4 Salt) — indirect offense.
    (
        channel_attack(a.offense, dtype) + card_pow + a.power_bonus,
        dtype,
        a.offense.precision,
    )
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
    let inner = !matches!(strike.dtype.channel(), Channel::Body); // Fear breaks the will
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
    // The arithmetic tail, so a transcript reader can verify the cut and the result: how much got
    // **through** the cut (Armor outer / Ward inner) and the channel's resulting meter.
    let math = if inner {
        format!(
            " [{} past ward; fear {}/{}]",
            out.through, target.defense.fear_pile, target.defense.resolve
        )
    } else {
        format!(
            " [{} past armor; Body {}/{}]",
            out.through, target.defense.body.remaining, target.defense.body.max
        )
    };
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
        "  {attacker} hits {name}: {} {dt}{what}{math}",
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

/// Finalize deaths at a phase boundary: an Actor whose Body is gone becomes `fallen` — unless it
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

/// The §4 outcome of a single gauntlet crossing.
enum Cross {
    HeroSlips,
    FoeSlips,
    BothStop,
}

/// A unit's **advance** — the **Daring** it commits to *slip past* an opponent (its Form Daring,
/// floor 1). Slipping needs advance Daring strictly greater than the other's **hold**.
fn advance_grade(a: &Actor) -> u32 {
    a.offense.daring.max(1)
}

/// A unit's **hold** — the grade it commits to *hold the line* (its advance Daring plus the
/// **Phalanx** bonus). A Wall that owns *Phalanx* combines its effort with the line, +2 to the hold
/// (a v1 flat stand-in for "Walls who stop together intercept as one"). Phalanx raises only the
/// hold, never advance — a Wall holds; it does not slip through on a high number.
fn hold_grade(a: &Actor) -> u32 {
    advance_grade(a) + if a.has("Phalanx") { 2 } else { 0 }
}

/// **Taunt** draws fire: a Wall that owns *Taunt* is pulled to the front of its charge column, so the
/// enemy meets it first (sparing the rest of the line). Stable-sort keeps roster order otherwise.
fn taunt_order(chargers: &mut [usize], pool: &[Actor]) {
    chargers.sort_by_key(|&i| !pool[i].has("Taunt"));
}

/// Resolve the **gauntlet** (§4): the two charge-columns thread through each other. **Taunt** Walls
/// are pulled to the front; chargers then pair off by column order. Each crossing is decided by
/// **Daring** (§3): the **higher** committed advance **slips past** (becomes a Skirmisher) while the one
/// it passes lands a **parting free hit**; a **tie stops both** (both become Vanguard) — unless a
/// slipper owns **Shadowstep** (it wins the tie). Both spend a Tempo card to contest, except an
/// Infiltrator's first **Blitz** slip (free). **Surplus** chargers on the longer column break through
/// cleanly — unless a stopped enemy Wall with **Bodyguard** intercepts one. A charger that stops (or
/// never broke through) is a Vanguard; a non-charger is a Reserve. Returns `(hero_skirmisher,
/// foe_skirmisher)` — who broke through.
///
/// *(v1: each crossing flips one Tempo card per side — the full escalating Daring auction is a later
/// enrichment. Damage applies from pre-crossing snapshots, so the phase stays order-independent.)*
pub fn gauntlet(
    heroes: &mut [Actor],
    hero_charging: &[bool],
    foes: &mut [Actor],
    foe_charging: &[bool],
    log: &mut Vec<String>,
) -> (Vec<bool>, Vec<bool>) {
    let mut hero_skirm = vec![false; heroes.len()];
    let mut foe_skirm = vec![false; foes.len()];
    let mut h_chargers: Vec<usize> = (0..heroes.len())
        .filter(|&i| hero_charging[i] && !heroes[i].is_down())
        .collect();
    let mut f_chargers: Vec<usize> = (0..foes.len())
        .filter(|&i| foe_charging[i] && !foes[i].is_down())
        .collect();
    taunt_order(&mut h_chargers, heroes);
    taunt_order(&mut f_chargers, foes);
    let pairs = h_chargers.len().min(f_chargers.len());

    for k in 0..pairs {
        let h = h_chargers[k];
        let f = f_chargers[k];
        // A unit slips past iff its **advance** Daring beats the other's **hold**; a tie is
        // caught (held), unless the slipper owns **Shadowstep** (it wins the tie). Phalanx feeds catch
        // only, so a Wall holds the line rather than slipping through.
        let h_slips = advance_grade(&heroes[h]) > hold_grade(&foes[f])
            || (advance_grade(&heroes[h]) == hold_grade(&foes[f]) && heroes[h].has("Shadowstep"));
        let f_slips = advance_grade(&foes[f]) > hold_grade(&heroes[h])
            || (advance_grade(&foes[f]) == hold_grade(&heroes[h]) && foes[f].has("Shadowstep"));
        // Decide the crossing first, then charge Tempo — so a Blitz free slip can skip its own cost.
        // Only one side can out-advance the other; a mutual Shadowstep tie holds (both stop).
        let outcome = match (h_slips, f_slips) {
            (true, false) => Cross::HeroSlips,
            (false, true) => Cross::FoeSlips,
            _ => Cross::BothStop,
        };
        // Narrate the crossing **with the arithmetic that decided it** (a unit slips iff its advance
        // advance strictly exceeds the other's hold): the four committed grades and the verdict,
        // so the rules are auditable from the transcript.
        let (h_adv, h_cat) = (advance_grade(&heroes[h]), hold_grade(&heroes[h]));
        let (f_adv, f_cat) = (advance_grade(&foes[f]), hold_grade(&foes[f]));
        let verdict = match outcome {
            Cross::HeroSlips => format!("{} slips ({h_adv}>{f_cat})", heroes[h].name),
            Cross::FoeSlips => format!("{} slips ({f_adv}>{h_cat})", foes[f].name),
            Cross::BothStop => format!("both hold ({h_adv}≤{f_cat}, {f_adv}≤{h_cat})"),
        };
        log.push(format!(
            "crossing: {}(adv {h_adv}/catch {h_cat}) × {}(adv {f_adv}/catch {f_cat}) → {verdict}",
            heroes[h].name, foes[f].name
        ));
        // Tempo: each contests for one card, but an Infiltrator's first Blitz slip is free.
        let blitz_free = |a: &mut Actor, slipped: bool| {
            if slipped && a.has("Blitz") && !a.free_slip_used {
                a.free_slip_used = true;
            } else {
                a.tempo -= 1;
            }
        };
        blitz_free(&mut heroes[h], matches!(outcome, Cross::HeroSlips));
        blitz_free(&mut foes[f], matches!(outcome, Cross::FoeSlips));
        // A gauntlet blow is a melee strike, so a unit that has **lost its action** (Stagger) or been
        // **knocked out of melee** (Shove) lands none — it still holds the line / takes the parting
        // hit, but cannot strike back (the same status `pending_targets` honours in Skirmish/Reserve).
        let h_can = heroes[h].can_contest_now(Range::Melee);
        let f_can = foes[f].can_contest_now(Range::Melee);
        match outcome {
            Cross::HeroSlips => {
                // Hero out-dares → slips past; the foe it passes lands a parting blow (if able).
                if f_can {
                    let snap = snapshot(&foes[f]);
                    let name = foes[f].name.clone();
                    apply_strike(&mut heroes[h], snap, &name, log);
                }
                if !heroes[h].is_down() {
                    hero_skirm[h] = true;
                    log.push(format!("{} breaks through the line!", heroes[h].name));
                }
            }
            Cross::FoeSlips => {
                if h_can {
                    let snap = snapshot(&heroes[h]);
                    let name = heroes[h].name.clone();
                    apply_strike(&mut foes[f], snap, &name, log);
                }
                if !foes[f].is_down() {
                    foe_skirm[f] = true;
                }
            }
            Cross::BothStop => {
                // Caught: both stop and trade (both become Vanguard) — each blow only if its dealer
                // can still act.
                let hsnap = snapshot(&heroes[h]);
                let fsnap = snapshot(&foes[f]);
                let hname = heroes[h].name.clone();
                let fname = foes[f].name.clone();
                if f_can {
                    apply_strike(&mut heroes[h], fsnap, &fname, log);
                }
                if h_can {
                    apply_strike(&mut foes[f], hsnap, &hname, log);
                }
            }
        }
    }
    // Surplus chargers on the longer column meet no opposition → break through cleanly, unless a
    // stopped enemy Wall with **Bodyguard** steps across to intercept one (guarding the backfield).
    let mut foe_guards = bodyguards(&f_chargers, &foe_skirm, foes, pairs);
    for &h in h_chargers.iter().skip(pairs) {
        if heroes[h].is_down() {
            continue;
        }
        if let Some(g) = foe_guards.pop() {
            intercept(heroes, h, foes, g, log);
        } else {
            hero_skirm[h] = true;
            log.push(format!(
                "{} runs an open gauntlet and breaks through!",
                heroes[h].name
            ));
        }
    }
    let mut hero_guards = bodyguards(&h_chargers, &hero_skirm, heroes, pairs);
    for &f in f_chargers.iter().skip(pairs) {
        if foes[f].is_down() {
            continue;
        }
        if let Some(g) = hero_guards.pop() {
            intercept(foes, f, heroes, g, log);
        } else {
            foe_skirm[f] = true;
        }
    }
    (hero_skirm, foe_skirm)
}

/// The stopped Vanguard interceptors (charged, didn't break through, still up, Tempo to spend) on a
/// side that own **Bodyguard** — each can step across to catch one surplus enemy charger.
fn bodyguards(chargers: &[usize], skirm: &[bool], pool: &[Actor], pairs: usize) -> Vec<usize> {
    chargers
        .iter()
        .take(pairs)
        .copied()
        .filter(|&i| {
            !skirm[i]
                && !pool[i].is_down()
                && pool[i].has("Bodyguard")
                && pool[i].tempo > 0
                // A Staggered / Shoved Wall has lost its action — it cannot step across to intercept.
                && pool[i].can_contest_now(Range::Melee)
        })
        .collect()
}

/// A `guard` (in `gpool`) intercepts a surplus `runner` (in `rpool`): both spend a Tempo card and
/// trade from pre-crossing snapshots; the runner is stopped (no breakthrough).
fn intercept(
    rpool: &mut [Actor],
    runner: usize,
    gpool: &mut [Actor],
    guard: usize,
    log: &mut Vec<String>,
) {
    rpool[runner].tempo -= 1;
    gpool[guard].tempo -= 1;
    let rsnap = snapshot(&rpool[runner]);
    let gsnap = snapshot(&gpool[guard]);
    let rname = rpool[runner].name.clone();
    let gname = gpool[guard].name.clone();
    log.push(format!("{gname} guards the line — intercepts {rname}!"));
    apply_strike(&mut rpool[runner], gsnap, &gname, log);
    // The runner only strikes back if it can still act (Stagger / Shove suppress the trade).
    if rpool[runner].can_contest_now(Range::Melee) {
        apply_strike(&mut gpool[guard], rsnap, &rname, log);
    }
}

/// Apply a hero's action/power card. The deterministic effects (§"cards may supersede the
/// core") are wired here; foes use the same path. `foes`/`allies` are the opposing and
/// friendly pools.
#[allow(clippy::too_many_arguments)]
pub fn play_card(
    card: &crate::cards::Card,
    actor_name: &str,
    attacker: Offense,
    foes: &mut [Actor],
    allies: &mut [Actor],
    self_idx: Option<usize>,
    log: &mut Vec<String>,
) {
    log.push(format!("{actor_name} plays {}.", card.name));
    // How many foes / allies an effect touches (§4 AoE); ≥1. A Curse modifier (M5) and Sanctuary
    // (M6) raise this via the card's `targets` at build time.
    let n = (card.targets as usize).max(1);
    // The Support's force-multiplier (§2.4): each augment gains +Inspiration on its magnitude.
    let insp = attacker.inspiration;
    for effect in &card.effects {
        match *effect {
            Effect::Damage { power, dtype } => {
                // The damage type's channel selects the attack stat: outer strikes scale off Strike
                // (`power`), inner (Fear) strikes off **Dread** (§2.2 parallel channels). Card power adds.
                let raw = channel_attack(attacker, dtype) + power;
                let alive: Vec<usize> = living(foes);
                for ti in alive.into_iter().take(n) {
                    apply_strike(
                        &mut foes[ti],
                        Strike {
                            raw,
                            dtype,
                            precision: attacker.precision,
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
            Effect::Fortify { armor } => {
                // Shield Wall: the Wall raises temporary Armor over the whole line this round.
                for t in allies.iter_mut().filter(|a| !a.is_down()) {
                    t.defense.armor_bonus += armor;
                }
                log.push(format!(
                    "  raises a shield wall (+{armor} Armor to the line)."
                ));
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
                let amt = resolve + insp;
                for a in allies.iter_mut().filter(|a| !a.is_down()) {
                    a.defense.resolve += amt;
                }
                log.push(format!("  +{amt} Resolve to allies."));
            }
            Effect::Mend { body } => {
                // Heal the `n` most-wounded allies (M6 Sanctuary heals all).
                let mut order: Vec<usize> = living(allies);
                order.sort_by_key(|&i| allies[i].defense.body.remaining);
                let amt = body + insp;
                for ai in order.into_iter().take(n) {
                    let max = allies[ai].defense.body.max;
                    let r = &mut allies[ai].defense.body.remaining;
                    *r = (*r + amt).min(max);
                    log.push(format!("  mends {} (+{amt} Body).", allies[ai].name));
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
                let amt = tempo + insp;
                for t in allies.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo += amt as i32;
                    log.push(format!("  +{amt} Tempo to {}.", t.name));
                }
            }
            Effect::Empower { power } => {
                // Round-scoped +Power to allies (the §4 Salt force-multiplier — indirect offense).
                let amt = power + insp;
                for t in allies.iter_mut().filter(|a| !a.is_down()) {
                    t.power_bonus += amt;
                }
                log.push(format!("  empowers the line (+{amt} Power)."));
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
            Effect::Stagger => {
                // A Controller debuff: the foe loses its action this round (no strike, no card, no
                // strike-back). Played at Muster, it bites the whole round (§4).
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.stunned = true;
                    log.push(format!("  staggers {} (loses its action).", t.name));
                }
            }
            Effect::Shove => {
                // Knock the foe out of melee: this round it cannot contest a melee blow (no
                // strike-back; takes free hits).
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.shoved = true;
                    log.push(format!("  shoves {} out of the line.", t.name));
                }
            }
            Effect::Disarm => {
                // Foul the foe's hand: this round it cannot play its role cards.
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.disarmed = true;
                    log.push(format!("  disarms {} (cannot play cards).", t.name));
                }
            }
            Effect::Recover => {
                // Turn a face-down Health card back up on the most-wounded ally/allies (§5).
                let mut order: Vec<usize> = living(allies);
                order.sort_by_key(|&i| allies[i].defense.body.remaining);
                for ai in order.into_iter().take(n) {
                    if allies[ai].defense.recover_card() > 0 {
                        log.push(format!(
                            "  {} turns a health card back up.",
                            allies[ai].name
                        ));
                    }
                }
            }
        }
    }
}

/// Can `defender` contest a strike at `range` (§4.2)? If not, the strike is an auto-hit.
pub fn contests(defender: &Actor, range: Range) -> bool {
    defender.can_contest(range)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Card, RoleKind};
    use crate::currency::Currency;
    use crate::scenarios::{RewardId, build_character};
    use crate::zones::ZoneBehavior;

    fn rid(track: Currency, level: u32) -> RewardId {
        RewardId { track, level }
    }

    /// A throwaway played card carrying `effects` (for testing the effect wiring directly).
    fn fx(effects: Vec<Effect>) -> Card {
        Card {
            name: "Test".into(),
            text: String::new(),
            flavor: String::new(),
            targets: 1,
            reach: [1, 1],
            zone: ZoneBehavior::Return,
            tags: vec![],
            passive: false,
            effects,
            role: None,
            kind: RoleKind::Base,
            positional: false,
            modifies: None,
        }
    }

    #[test]
    fn phalanx_lets_a_wall_hold_a_runner_a_bare_wall_cannot() {
        // Runner: Silver L1 grants Daring 2 (advance 2).
        // A bare Wall has hold 1 → the runner slips past.
        let mut runners = vec![build_character("Novice", &[rid(Currency::Silver, 1)])];
        let mut bare_wall = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        let (skirm, _) = gauntlet(&mut runners, &[true], &mut bare_wall, &[true], &mut log);
        assert!(
            skirm[0],
            "a bare Wall (hold 1) cannot stop a Daring-2 runner"
        );

        // A Phalanx Wall (Iron L2) has hold 1+2 = 3 ≥ the runner's advance 2 → caught.
        let mut runners = vec![build_character("Novice", &[rid(Currency::Silver, 1)])];
        let mut phalanx_wall = vec![build_character("Novice", &[rid(Currency::Iron, 2)])];
        let mut log = Vec::new();
        let (skirm, _) = gauntlet(&mut runners, &[true], &mut phalanx_wall, &[true], &mut log);
        assert!(!skirm[0], "a Phalanx Wall (hold 3) holds a Daring-2 runner");
        assert!(
            phalanx_wall[0].has("Phalanx") && advance_grade(&phalanx_wall[0]) == 1,
            "Phalanx feeds catch only — the Wall never advances on it"
        );
    }

    #[test]
    fn shadowstep_wins_a_tie() {
        // Equal advance/hold and no Shadowstep → the tie is caught (both stop).
        let mut a = vec![build_character("Novice", &[])];
        let mut b = vec![build_character("Novice", &[])];
        a[0].offense.daring = 2;
        b[0].offense.daring = 2;
        let mut log = Vec::new();
        let (ask, bsk) = gauntlet(&mut a, &[true], &mut b, &[true], &mut log);
        assert!(!ask[0] && !bsk[0], "an even tie stops both");

        // Shadowstep (Silver L3) on A, forced to the same Daring → A wins the tie and slips.
        let mut a = vec![build_character("Novice", &[rid(Currency::Silver, 3)])];
        let mut b = vec![build_character("Novice", &[])];
        a[0].offense.daring = 2;
        b[0].offense.daring = 2;
        let mut log = Vec::new();
        let (ask, _) = gauntlet(&mut a, &[true], &mut b, &[true], &mut log);
        assert!(ask[0], "Shadowstep wins the tie and slips through");
    }

    #[test]
    fn blitz_makes_the_first_slip_cost_no_tempo() {
        let mut a = vec![build_character("Novice", &[rid(Currency::Silver, 2)])]; // owns Blitz
        a[0].offense.daring = 5; // clearly out-advances the bare blocker
        let mut b = vec![build_character("Novice", &[])];
        let tempo_before = a[0].tempo;
        let mut log = Vec::new();
        let (ask, _) = gauntlet(&mut a, &[true], &mut b, &[true], &mut log);
        assert!(ask[0], "the high-Daring runner slips");
        assert_eq!(
            a[0].tempo, tempo_before,
            "Blitz: the first slip each round costs no Tempo"
        );
        assert!(a[0].free_slip_used, "the free slip is now spent");
    }

    #[test]
    fn stagger_shove_and_disarm_set_round_status() {
        let mut foes = vec![build_character("Novice", &[])];
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        play_card(
            &fx(vec![Effect::Stagger, Effect::Shove, Effect::Disarm]),
            "Hexer",
            Offense::default(),
            &mut foes,
            &mut allies,
            Some(0),
            &mut log,
        );
        assert!(foes[0].stunned, "Stagger sets the lose-its-action flag");
        assert!(foes[0].shoved, "Shove sets the no-melee-contest flag");
        assert!(foes[0].disarmed, "Disarm sets the no-cards flag");
        assert!(
            !foes[0].can_contest_now(Range::Melee),
            "a staggered/shoved foe cannot contest melee"
        );
    }

    /// T3 — no-redundant-stat (§8.6): an inner (Fear) attack scales off **Dread**. Zeroing Dread must
    /// reduce the fear dealt — the regression guard that would have caught the old dead "Spirit".
    #[test]
    fn dread_is_consumed_by_fear_attacks() {
        let card = fx(vec![Effect::Damage {
            power: 0,
            dtype: DamageType::Fear,
        }]);
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();

        let mut with = vec![build_character("Novice", &[])];
        play_card(
            &card,
            "Hexer",
            Offense {
                dread: 8,
                ..Default::default()
            },
            &mut with,
            &mut allies,
            None,
            &mut log,
        );
        let mut without = vec![build_character("Novice", &[])];
        play_card(
            &card,
            "Hexer",
            Offense::default(),
            &mut without,
            &mut allies,
            None,
            &mut log,
        );
        assert!(
            with[0].defense.fear_pile > without[0].defense.fear_pile,
            "Dread must scale a fear attack (no-redundant-stat, §8.6)"
        );
    }

    /// T3 — no-redundant-stat (§8.6): a Support **augment** scales off **Inspiration**. A Mend with
    /// Inspiration restores more Body than one without — proof the Salt signature is consumed.
    #[test]
    fn inspiration_is_consumed_by_augments() {
        let card = fx(vec![Effect::Mend { body: 1 }]);
        let mut foes = vec![build_character("Novice", &[])];
        let mut log = Vec::new();

        let wounded = || {
            let mut a = build_character("Novice", &[]);
            a.defense.body.remaining = 1; // wounded but alive (a down ally isn't a Mend target)
            a
        };

        let mut hi = vec![wounded()];
        play_card(
            &card,
            "Vow",
            Offense {
                inspiration: 5,
                ..Default::default()
            },
            &mut foes,
            &mut hi,
            Some(0),
            &mut log,
        );
        let mut lo = vec![wounded()];
        play_card(
            &card,
            "Vow",
            Offense::default(),
            &mut foes,
            &mut lo,
            Some(0),
            &mut log,
        );
        assert!(
            hi[0].defense.body.remaining > lo[0].defense.body.remaining,
            "Inspiration must scale an augment (no-redundant-stat, §8.6)"
        );
    }

    #[test]
    fn recover_turns_a_health_card_back_up() {
        let mut allies = vec![build_character("Novice", &[])];
        let mut foes = vec![build_character("Novice", &[])];
        // Knock one Health card face down.
        let t = allies[0].defense.body.toughness;
        allies[0].defense.take(t, DamageType::Blunt, 0);
        let before = allies[0].defense.body.remaining;
        assert!(before < allies[0].defense.body.max, "a card is face down");
        let mut log = Vec::new();
        play_card(
            &fx(vec![Effect::Recover]),
            "Medic",
            Offense::default(),
            &mut foes,
            &mut allies,
            Some(0),
            &mut log,
        );
        assert_eq!(
            allies[0].defense.body.remaining,
            before + 1,
            "Recover turns one face-down Health card back up"
        );
    }
}
