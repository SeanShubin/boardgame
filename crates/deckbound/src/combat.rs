//! Combat resolution helpers for the §4 **static-ranks** system: typed strikes, the **range** rule
//! (same-range trade vs auto-hit, §4.2), **the Line** resolver ([`the_line`] — declared ranks, the
//! card-bound crossing contest), creature AI, and card effects. The interactive four-card Clash
//! ([`crate::duel`]) is the optional 1v1 module.

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
    // §2.2 / Charter #13 — fear is control, not damage: the break tier applies a round-scoped status.
    // Freeze Staggers (loses its action); Shaken adds Shove (also cannot defend); Rout adds the
    // Vanguard→Reserve demotion — driven off the line — which the §4 charge step reads from the
    // persisted `will_break` flag at Deploy (see `Deckbound::deploy`). Here we set the in-place flags.
    match out.broke {
        Some(Break::Freeze) => target.stunned = true,
        Some(Break::Shaken | Break::Rout) => {
            target.stunned = true;
            target.shoved = true;
        }
        None => {}
    }
    // Defeat is *not* narrated here: a phase resolves order-independently from snapshots, so several
    // strikes may land on the same target. "All health cards face down → falls" is reported once, when
    // the phase boundary finalizes it (see `tally`) — by then any same-phase healing has netted out.
}

fn break_note(b: Break) -> &'static str {
    match b {
        Break::Freeze => "freezes in fear — loses its action",
        Break::Shaken => "is shaken — cannot act or defend",
        Break::Rout => "is routed — driven from the line",
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

/// A unit's **advance** — the per-card **Daring** it commits to *cross* the line (floor 1). A
/// Skirmisher's total advance is `cards-bid × advance`.
fn advance_daring(a: &Actor) -> u32 {
    a.offense.daring.max(1)
}

/// A Vanguard's **hold** for one catch — its per-card Daring plus **Phalanx** (+2, catch strength)
/// and **Bulwark** (+2 if any living ally carries it — the line anchors as one). A catch is a single
/// committed card, so this is the grade a Skirmisher's advance must beat. Phalanx/Bulwark raise the
/// hold only, never advance — a Wall holds; it does not slip through on a high number.
fn hold_daring(catcher: &Actor, allies: &[Actor]) -> u32 {
    let bulwark = allies.iter().any(|x| !x.is_down() && x.has("Bulwark"));
    advance_daring(catcher)
        + if catcher.has("Phalanx") { 2 } else { 0 }
        + if bulwark { 2 } else { 0 }
}

/// The fewest cards a Skirmisher must bid for `cards × advance` to beat `hold` (strictly, or `≥` with
/// **Shadowstep**, which wins a tie). Floors at 1.
fn cards_to_cross(sk: &Actor, hold: u32) -> u32 {
    let adv = advance_daring(sk);
    let b = if sk.has("Shadowstep") {
        hold.div_ceil(adv) // adv·b ≥ hold
    } else {
        hold / adv + 1 // adv·b > hold
    };
    b.max(1)
}

/// Resolve **the Line** (§4 Tier 1, static-ranks). Ranks are *declared*: a charger that **flanks** is a
/// **Skirmisher** attempting to cross; a charger that does not is a **Vanguard** holding; a non-charger
/// is **Reserve**. Nobody moves. From the start-of-tier state: **Vanguards strike** the opposing front
/// (one card each), and each **Skirmisher** runs a **crossing contest** against the enemy Vanguards —
/// **card-bound catch** (a Vanguard catches one Skirmisher per Tempo card, **Taunt** first; each catch's
/// hold = [`hold_daring`]). A caught Skirmisher bids the fewest cards to beat the hold
/// ([`cards_to_cross`]); affordable (or **uncaught**) → it **slips** (the catcher lands a **parting free
/// hit** off the same catch-card), else **held** and it trades. **Blitz** frees a Skirmisher's first
/// slip. Returns `(hero_crossed, foe_crossed)`. Order-independent (offense is immutable here); deaths
/// finalize at the boundary (`tally`).
pub fn the_line(
    heroes: &mut [Actor],
    h_charging: &[bool],
    h_flank: &[bool],
    foes: &mut [Actor],
    f_charging: &[bool],
    f_flank: &[bool],
    log: &mut Vec<String>,
) -> (Vec<bool>, Vec<bool>) {
    let van = |charging: &[bool], flank: &[bool], pool: &[Actor]| -> Vec<usize> {
        (0..pool.len())
            .filter(|&i| charging[i] && !flank[i] && !pool[i].is_down())
            .collect()
    };
    let skirm = |charging: &[bool], flank: &[bool], pool: &[Actor]| -> Vec<usize> {
        (0..pool.len())
            .filter(|&i| charging[i] && flank[i] && !pool[i].is_down())
            .collect()
    };
    let h_van = van(h_charging, h_flank, heroes);
    let f_van = van(f_charging, f_flank, foes);
    let h_sk = skirm(h_charging, h_flank, heroes);
    let f_sk = skirm(f_charging, f_flank, foes);
    let mut h_crossed = vec![false; heroes.len()];
    let mut f_crossed = vec![false; foes.len()];

    // 1. Vanguard clash — each Vanguard strikes the first opposing Vanguard (one card).
    clash_line(heroes, &h_van, foes, &f_van, log);
    clash_line(foes, &f_van, heroes, &h_van, log);

    // 2. Crossing contests — the defender's Vanguards catch the crossing Skirmishers.
    cross_contest(heroes, &h_sk, foes, &f_van, &mut h_crossed, log);
    cross_contest(foes, &f_sk, heroes, &h_van, &mut f_crossed, log);

    (h_crossed, f_crossed)
}

/// Each Vanguard in `van` strikes the first living opposing Vanguard (one card), if it can still act.
fn clash_line(
    pool: &mut [Actor],
    van: &[usize],
    enemy: &mut [Actor],
    enemy_van: &[usize],
    log: &mut Vec<String>,
) {
    for &v in van {
        let Some(&t) = enemy_van.iter().find(|&&e| !enemy[e].is_down()) else {
            continue; // no opposing front — this Vanguard pours through in the Open
        };
        if pool[v].tempo > 0 && pool[v].can_contest_now(Range::Melee) {
            pool[v].tempo -= 1;
            let snap = snapshot(&pool[v]);
            let name = pool[v].name.clone();
            apply_strike(&mut enemy[t], snap, &name, log);
        }
    }
}

/// Resolve the crossing contests for one side's Skirmishers (`sk` in `skpool`) against the enemy
/// Vanguards (`van` in `vanpool`). Card-bound catch: a Vanguard catches one Skirmisher per Tempo card,
/// **Taunt** Vanguards first; the highest-Daring Skirmishers are caught first; uncaught ones slip free.
fn cross_contest(
    skpool: &mut [Actor],
    sk: &[usize],
    vanpool: &mut [Actor],
    van: &[usize],
    crossed: &mut [bool],
    log: &mut Vec<String>,
) {
    // Catchers = the Vanguards alive at tier start (`van`). A Vanguard mortally wounded by the clash
    // still catches — deaths finalize at the tier boundary (§1.3), so the Line stays order-independent.
    let mut catchers: Vec<usize> = van.to_vec();
    catchers.sort_by_key(|&v| !vanpool[v].has("Taunt")); // Taunt draws the first catch
    let mut runners: Vec<usize> = sk
        .iter()
        .copied()
        .filter(|&s| !skpool[s].is_down())
        .collect();
    runners.sort_by_key(|&s| std::cmp::Reverse(advance_daring(&skpool[s])));

    for s in runners {
        let catcher = catchers
            .iter()
            .copied()
            .find(|&v| vanpool[v].tempo > 0 && vanpool[v].can_contest_now(Range::Melee));
        let Some(c) = catcher else {
            crossed[s] = true; // the wall is out of catch-cards — slips free, no parting hit
            log.push(format!("{} slips an unguarded gap.", skpool[s].name));
            continue;
        };
        vanpool[c].tempo -= 1; // the catch costs the Vanguard a card
        let hold = hold_daring(&vanpool[c], vanpool);
        let need = cards_to_cross(&skpool[s], hold);
        let adv = advance_daring(&skpool[s]);
        let affordable = need as i32 <= skpool[s].tempo;
        log.push(format!(
            "crossing: {}(adv {adv}×{need}={}) vs {}(hold {hold}) → {}",
            skpool[s].name,
            adv * need,
            vanpool[c].name,
            if affordable { "slips" } else { "held" },
        ));
        if affordable {
            if skpool[s].has("Blitz") && !skpool[s].free_slip_used {
                skpool[s].free_slip_used = true;
            } else {
                skpool[s].tempo -= need as i32;
            }
            if vanpool[c].can_contest_now(Range::Melee) {
                let snap = snapshot(&vanpool[c]);
                let name = vanpool[c].name.clone();
                apply_strike(&mut skpool[s], snap, &name, log);
            }
            if !skpool[s].is_down() {
                crossed[s] = true;
            }
        } else {
            skpool[s].tempo -= 1; // tried (one card) and trades with its catcher
            if skpool[s].can_contest_now(Range::Melee) {
                let snap = snapshot(&skpool[s]);
                let name = skpool[s].name.clone();
                apply_strike(&mut vanpool[c], snap, &name, log);
            }
            if vanpool[c].can_contest_now(Range::Melee) {
                let snap = snapshot(&vanpool[c]);
                let name = vanpool[c].name.clone();
                apply_strike(&mut skpool[s], snap, &name, log);
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

    /// A Skirmisher (charges + flanks) crossing a Vanguard (charges, holds). Returns `crossed[0]`.
    fn cross(runner: &mut [Actor], wall: &mut [Actor]) -> bool {
        let mut log = Vec::new();
        let (crossed, _) = the_line(runner, &[true], &[true], wall, &[true], &[false], &mut log);
        crossed[0]
    }

    #[test]
    fn phalanx_raises_the_hold_so_a_one_card_runner_is_held() {
        // Runner: Silver L1 → Daring 2. One card → advance 2.
        // A bare Wall has hold 1 → 2 > 1, the one-card runner slips.
        let mut runner = vec![build_character("Novice", &[rid(Currency::Silver, 1)])];
        runner[0].tempo = 1;
        let mut bare = vec![build_character("Novice", &[])];
        assert!(
            cross(&mut runner, &mut bare),
            "a bare Wall (hold 1) cannot stop advance 2"
        );

        // A Phalanx Wall (Iron L2) raises the hold to 1+2 = 3 — the *one-card* runner (advance 2) is
        // held. (Force, not fiat: a runner with the Tempo to bid 2 cards — advance 4 — still crosses;
        // Phalanx makes it *cost more*, never impossible.)
        let mut runner = vec![build_character("Novice", &[rid(Currency::Silver, 1)])];
        runner[0].tempo = 1;
        let mut phalanx = vec![build_character("Novice", &[rid(Currency::Iron, 2)])];
        assert!(
            !cross(&mut runner, &mut phalanx),
            "Phalanx (hold 3) holds a one-card advance-2 runner"
        );
        assert!(
            phalanx[0].has("Phalanx") && hold_daring(&phalanx[0], &phalanx) == 3,
            "Phalanx raises the hold to 3"
        );
    }

    #[test]
    fn shadowstep_wins_a_tie() {
        // One card each, equal Daring → advance 2 vs hold 2, a tie. Without Shadowstep, a tie needs a
        // second card (advance 4) to win — with only one card the runner is held.
        let mut a = vec![build_character("Novice", &[])];
        a[0].offense.daring = 2;
        a[0].tempo = 1;
        let mut b = vec![build_character("Novice", &[])];
        b[0].offense.daring = 2;
        assert!(!cross(&mut a, &mut b), "an even one-card tie is held");

        // Shadowstep (Silver L3) wins the tie: one card (advance 2 ≥ hold 2) slips.
        let mut a = vec![build_character("Novice", &[rid(Currency::Silver, 3)])];
        a[0].offense.daring = 2;
        a[0].tempo = 1;
        let mut b = vec![build_character("Novice", &[])];
        b[0].offense.daring = 2;
        assert!(
            cross(&mut a, &mut b),
            "Shadowstep wins the tie and slips through on one card"
        );
    }

    #[test]
    fn blitz_makes_the_first_slip_cost_no_tempo() {
        let mut a = vec![build_character("Novice", &[rid(Currency::Silver, 2)])]; // owns Blitz
        a[0].offense.daring = 5; // clearly out-advances the bare wall (hold 1) on one card
        let mut b = vec![build_character("Novice", &[])];
        let tempo_before = a[0].tempo;
        assert!(cross(&mut a, &mut b), "the high-Daring runner slips");
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
