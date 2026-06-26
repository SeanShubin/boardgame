//! Combat resolution helpers for the §4 **static-ranks** system: untyped Might strikes, the **range**
//! rule (same-range trade, ranged evade-or-auto-hit, §4.2), **the Line** resolver ([`the_line`] —
//! declared ranks, the card-bound crossing contest), the **evade contest** ([`try_evade`]), creature
//! AI, and card effects. The interactive four-card Clash ([`crate::duel`]) is the optional 1v1 module.

use crate::actor::{Actor, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::stats::Offense;

/// The base strike raw magnitude (untyped Might, §2.2): the actor's **Might**, plus the base
/// attack card's power, plus this round's Empower (`might_bonus`, a Support buff, §4 Salt).
pub fn base_strike(a: &Actor) -> u32 {
    let card_pow = a.weapon.primary_damage().unwrap_or(0);
    a.offense.might + card_pow + a.might_bonus
}

/// A base [`Strike`] snapshot (for order-independent resolution from phase-start state).
pub fn snapshot(a: &Actor) -> Strike {
    Strike {
        raw: base_strike(a),
    }
}

/// Route a strike through the target's defense and **narrate it as card-state transitions** — there is
/// no "life total": damage accumulates, and the only states are *health cards turning face down* and,
/// at the phase boundary, *all of them face down → defeated*. So a strike reads as one of: turned
/// aside (no card moves), damage accumulating (not yet enough to turn a card), or **N health cards
/// turn face down**. Defeat is narrated once, at the boundary (see `tally`), never here.
pub fn apply_strike(target: &mut Actor, strike: Strike, attacker: &str, log: &mut Vec<String>) {
    // Every strike is narrated — the log is how the player verifies the mechanics and learns who
    // acted, so a blow is never silently dropped. Overkill (a simultaneous-phase blow on a target
    // whose health cards are already all face down) is reported as such, not applied again.
    if target.is_down() {
        log.push(format!(
            "  {attacker} hits {}: {} might — its health cards are already all face down.",
            target.name, strike.raw
        ));
        return;
    }
    let out = target.defense.take(strike.raw);
    let name = &target.name;
    // The arithmetic tail, so a transcript reader can verify the result: the accumulated pile and
    // the resulting health meter (no cut today, §2.2).
    let math = format!(
        " [health {}/{}]",
        target.defense.health.remaining, target.defense.health.max
    );
    let what = if out.cards_flipped == 1 {
        " — turns a health card face down.".to_string()
    } else if out.cards_flipped > 1 {
        format!(" — turns {} health cards face down.", out.cards_flipped)
    } else {
        // Accumulating, but not yet a full health card's worth.
        " — damage accumulates.".to_string()
    };
    log.push(format!(
        "  {attacker} hits {name}: {} might{what}{math}",
        strike.raw
    ));
    // Defeat is *not* narrated here: a phase resolves order-independently from snapshots, so several
    // strikes may land on the same target. "All health cards face down → falls" is reported once, when
    // the phase boundary finalizes it (see `tally`) — by then any same-phase healing has netted out.
}

/// Pick a target index among `candidates` (indices into `pool`) per a target rule (§4).
pub fn pick_target(pool: &[Actor], candidates: &[usize], rule: TargetRule) -> Option<usize> {
    match rule {
        TargetRule::Front => candidates.first().copied(),
        TargetRule::LowestBody => candidates
            .iter()
            .copied()
            .min_by_key(|&i| pool[i].defense.health.remaining),
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
                a.defense.health.remaining = a.defense.health.remaining.max(1);
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

/// A unit's **advance** — the per-card **Finesse** it commits to *cross* the line (floor 1). An
/// Outrider's total advance is `cards-bid × advance`.
fn advance_finesse(a: &Actor) -> u32 {
    a.offense.finesse.max(1)
}

/// A Vanguard's **hold** for one catch — its per-card Finesse plus **Phalanx** (+2, catch strength)
/// and **Bulwark** (+2 if any living ally carries it — the line anchors as one). A catch is a single
/// committed card, so this is the grade an Outrider's advance must beat. Phalanx/Bulwark raise the
/// hold only, never advance — a Wall holds; it does not slip through on a high number.
fn hold_finesse(catcher: &Actor, allies: &[Actor]) -> u32 {
    let bulwark = allies.iter().any(|x| !x.is_down() && x.has("Bulwark"));
    advance_finesse(catcher)
        + if catcher.has("Phalanx") { 2 } else { 0 }
        + if bulwark { 2 } else { 0 }
}

/// The fewest cards an Outrider must bid for `cards × advance` to beat `hold` (strictly, or `≥` with
/// **Shadowstep**, which wins a tie). Floors at 1.
fn cards_to_cross(sk: &Actor, hold: u32) -> u32 {
    let adv = advance_finesse(sk);
    let b = if sk.has("Shadowstep") {
        hold.div_ceil(adv) // adv·b ≥ hold
    } else {
        hold / adv + 1 // adv·b > hold
    };
    b.max(1)
}

/// The fewest Tempo cards a defender must commit for `cards × Finesse` to **strictly exceed** a ranged
/// attacker's pressed `volley` — the evade contest (Spec §3.1 / §4.2, the same primitive as a
/// crossing). Mirrors [`cards_to_cross`] without Shadowstep (a tie lands the strike — the avoider must
/// strictly exceed). Floors at 1.
fn cards_to_evade(defender: &Actor, volley: u32) -> u32 {
    let grade = advance_finesse(defender); // per-Tempo-card Finesse (floor 1)
    (volley / grade + 1).max(1) // grade·b > volley
}

/// Resolve a ranged attack against `defender` (Spec §4.2): the defender may **evade** by committing the
/// minimum Tempo to **strictly exceed** the attacker's pressed `volley` (cards × Finesse) — a tie or
/// less and the hit lands. Default policy: the defender evades iff it can afford the minimum cards; spent
/// Tempo stays spent. Returns `true` if the attack was **evaded** (no hit); `false` if it **lands**
/// (the caller then applies the strike).
///
/// `volley` is the attacker's pressed bid (cards × the attacker's Finesse) — by default a single card
/// (the attacker does not pre-press), but a policy/card may press harder.
pub fn try_evade(defender: &mut Actor, volley: u32, log: &mut Vec<String>) -> bool {
    if defender.stunned {
        return false; // no action to spend — takes the free hit
    }
    let need = cards_to_evade(defender, volley) as i32;
    let grade = advance_finesse(defender);
    if need <= defender.tempo {
        defender.tempo -= need;
        log.push(format!(
            "  {} evades (evade {need}×{grade}={} > volley {volley}).",
            defender.name,
            need as u32 * grade,
        ));
        true
    } else {
        log.push(format!(
            "  {} cannot evade the volley ({volley}) — the shot lands.",
            defender.name,
        ));
        false
    }
}

/// Resolve **the Line** (§4 Tier 1, static-ranks). Ranks are *declared*: a charger that **flanks** is an
/// **Outrider** attempting to cross; a charger that does not is a **Vanguard** holding; a non-charger
/// is **Rearguard**. Nobody moves. From the start-of-tier state: **Vanguards strike** the opposing front
/// (one card each), and each **Outrider** runs a **crossing contest** against the enemy Vanguards —
/// **card-bound catch** (a Vanguard catches one Outrider per Tempo card, **Taunt** first; each catch's
/// hold = [`hold_finesse`]). A caught Outrider bids the fewest cards to beat the hold
/// ([`cards_to_cross`]); affordable (or **uncaught**) → it **slips** (the catcher lands a **parting free
/// hit** off the same catch-card), else **held** and it trades. **Blitz** frees an Outrider's first
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

    // 2. Crossing contests — the defender's Vanguards catch the crossing Outriders.
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

/// Resolve the crossing contests for one side's Outriders (`sk` in `skpool`) against the enemy
/// Vanguards (`van` in `vanpool`). Card-bound catch: a Vanguard catches one Outrider per Tempo card,
/// **Taunt** Vanguards first; the highest-Finesse Outriders are caught first; uncaught ones slip free.
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
    runners.sort_by_key(|&s| std::cmp::Reverse(advance_finesse(&skpool[s])));

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
        let hold = hold_finesse(&vanpool[c], vanpool);
        let need = cards_to_cross(&skpool[s], hold);
        let adv = advance_finesse(&skpool[s]);
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
    for effect in &card.effects {
        match *effect {
            Effect::Damage { power } => {
                // Untyped Might (§2.2): the attacker's Might plus the card's own power.
                let raw = attacker.might + power;
                let alive: Vec<usize> = living(foes);
                for ti in alive.into_iter().take(n) {
                    apply_strike(&mut foes[ti], Strike { raw }, actor_name, log);
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
            Effect::Mend { vitality } => {
                // Heal the `n` most-wounded allies (M6 Sanctuary heals all).
                let mut order: Vec<usize> = living(allies);
                order.sort_by_key(|&i| allies[i].defense.health.remaining);
                let amt = vitality;
                for ai in order.into_iter().take(n) {
                    let max = allies[ai].defense.health.max;
                    let r = &mut allies[ai].defense.health.remaining;
                    *r = (*r + amt).min(max);
                    log.push(format!("  mends {} (+{amt} health).", allies[ai].name));
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
                let amt = tempo;
                for t in allies.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo += amt as i32;
                    log.push(format!("  +{amt} Tempo to {}.", t.name));
                }
            }
            Effect::Empower { might } => {
                // Round-scoped +Might to allies (the §4 Salt buff — indirect offense).
                let amt = might;
                for t in allies.iter_mut().filter(|a| !a.is_down()) {
                    t.might_bonus += amt;
                }
                log.push(format!("  empowers the line (+{amt} Might)."));
            }
            Effect::Suppress { tempo } => {
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo -= tempo as i32;
                    log.push(format!("  suppresses {} (-{tempo} Tempo).", t.name));
                }
            }
            Effect::Slow { cadence } => {
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.offense.cadence = t.offense.cadence.saturating_sub(cadence);
                    log.push(format!("  slows {} (-{cadence} Cadence).", t.name));
                }
            }
            Effect::Confuse { tempo } => {
                // Drain a foe's Tempo — less initiative to act *or* defend (merged-pool reframing).
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tempo -= tempo as i32;
                    log.push(format!("  confuses {} (-{tempo} Tempo).", t.name));
                }
            }
            Effect::BankCadence { amount } => {
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
            Effect::Rout => {
                // A Controller status (§4 / Charter #13): drive the foe from the line to the Rearguard
                // this round — it neither holds as a Vanguard nor crosses as an Outrider.
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.routed = true;
                    log.push(format!("  routs {} — driven from the line.", t.name));
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
                order.sort_by_key(|&i| allies[i].defense.health.remaining);
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
            cast: Default::default(),
            resolve: Default::default(),
            one_shot: false,
            role: None,
            kind: RoleKind::Base,
            positional: false,
            modifies: None,
        }
    }

    /// An Outrider (charges + flanks) crossing a Vanguard (charges, holds). Returns `crossed[0]`.
    fn cross(runner: &mut [Actor], wall: &mut [Actor]) -> bool {
        let mut log = Vec::new();
        let (crossed, _) = the_line(runner, &[true], &[true], wall, &[true], &[false], &mut log);
        crossed[0]
    }

    #[test]
    fn phalanx_raises_the_hold_so_a_one_card_runner_is_held() {
        // Runner: Silver L1 → Finesse 2. One card → advance 2.
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
            phalanx[0].has("Phalanx") && hold_finesse(&phalanx[0], &phalanx) == 3,
            "Phalanx raises the hold to 3"
        );
    }

    #[test]
    fn shadowstep_wins_a_tie() {
        // One card each, equal Finesse → advance 2 vs hold 2, a tie. Without Shadowstep, a tie needs a
        // second card (advance 4) to win — with only one card the runner is held.
        let mut a = vec![build_character("Novice", &[])];
        a[0].offense.finesse = 2;
        a[0].tempo = 1;
        let mut b = vec![build_character("Novice", &[])];
        b[0].offense.finesse = 2;
        assert!(!cross(&mut a, &mut b), "an even one-card tie is held");

        // Shadowstep (Silver L3) wins the tie: one card (advance 2 ≥ hold 2) slips.
        let mut a = vec![build_character("Novice", &[rid(Currency::Silver, 3)])];
        a[0].offense.finesse = 2;
        a[0].tempo = 1;
        let mut b = vec![build_character("Novice", &[])];
        b[0].offense.finesse = 2;
        assert!(
            cross(&mut a, &mut b),
            "Shadowstep wins the tie and slips through on one card"
        );
    }

    #[test]
    fn blitz_makes_the_first_slip_cost_no_tempo() {
        let mut a = vec![build_character("Novice", &[rid(Currency::Silver, 2)])]; // owns Blitz
        a[0].offense.finesse = 5; // clearly out-advances the bare wall (hold 1) on one card
        let mut b = vec![build_character("Novice", &[])];
        let tempo_before = a[0].tempo;
        assert!(cross(&mut a, &mut b), "the high-Finesse runner slips");
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

    /// T3 — no-redundant-stat (§8.6): a strike scales off **Might**. A Damage card with higher Might
    /// deals more — the regression guard that the offense stat is consumed.
    #[test]
    fn might_is_consumed_by_strikes() {
        let card = fx(vec![Effect::Damage { power: 0 }]);
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();

        let mut with = vec![build_character("Novice", &[])];
        play_card(
            &card,
            "Striker",
            Offense {
                might: 8,
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
            "Striker",
            Offense::default(),
            &mut without,
            &mut allies,
            None,
            &mut log,
        );
        assert!(
            with[0].defense.health.remaining < without[0].defense.health.remaining,
            "Might must scale a strike (no-redundant-stat, §8.6)"
        );
    }

    /// The **evade contest** (§3.1 / §4.2): the defender commits the minimum Tempo to strictly exceed
    /// the attacker's pressed volley; a tie or less lands the hit.
    #[test]
    fn evade_contest_strictly_exceeds_the_volley() {
        // Volley 1×Finesse=3; defender cadence 2 / finesse 2 commits 2 cards = 4 > 3 → evaded.
        let mut def = build_character("Novice", &[]);
        def.offense.finesse = 2;
        def.tempo = 2;
        let mut log = Vec::new();
        assert!(
            try_evade(&mut def, 3, &mut log),
            "4 > 3 strictly exceeds the volley → evaded"
        );
        assert_eq!(def.tempo, 0, "the two committed cards stay spent");

        // It cannot afford to exceed: finesse 2, only 1 card → 2 ≤ 3, the hit lands.
        let mut def = build_character("Novice", &[]);
        def.offense.finesse = 2;
        def.tempo = 1;
        assert!(
            !try_evade(&mut def, 3, &mut log),
            "1 card (2) cannot exceed volley 3 → the hit lands"
        );
    }

    #[test]
    fn recover_turns_a_health_card_back_up() {
        let mut allies = vec![build_character("Novice", &[])];
        let mut foes = vec![build_character("Novice", &[])];
        // Knock one Health card face down.
        let t = allies[0].defense.health.toughness;
        allies[0].defense.take(t);
        let before = allies[0].defense.health.remaining;
        assert!(before < allies[0].defense.health.max, "a card is face down");
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
            allies[0].defense.health.remaining,
            before + 1,
            "Recover turns one face-down Health card back up"
        );
    }
}
