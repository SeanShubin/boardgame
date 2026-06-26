//! Combat resolution helpers for the §4.6 **six-phase** model: untyped Might strikes through the
//! per-phase pile (§4.6 / §2.2), the **range** rule (same-range trade, ranged evade-or-auto-hit, §4.2),
//! the **evade contest** ([`try_evade`]), and the phase resolvers — the **Fray** front clash
//! ([`fray_clash`]), the **breach list / per-unit lock** ([`compute_locks`]), the **Volley** charge +
//! **pre-empt** + **flank-intercept** ([`resolve_volley`]), the **Breach** blows ([`resolve_breach`]),
//! and the **Reckoning** ([`resolve_reckoning`]). The interactive four-card Clash ([`crate::duel`]) is
//! the optional 1v1 module.
//!
//! **PRINCIPLE (§4.6).** Within a phase everything is committed up front and resolves
//! order-independently (§1.9), *including the blows of a body that dies in that same phase* (§1.3).
//! The phases impose the only timing: a unit dead at a phase boundary takes no further action, so a
//! death **precludes** a later phase but never reaches back into an earlier one. The pre-empt
//! (Volley before Breach), the disrupt (a breach-kill fizzles a Reckoning spell), and the
//! flank-intercept are all corollaries.

use crate::actor::{Actor, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::state::{Charge, Deferred};
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
/// overkill. Note this does **not** wipe the per-phase pile — that is [`clear_phase_piles`], called
/// once both sides have tallied (the Fray and Volley boundaries, §4.6).
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

/// §4.6 **per-phase pile wipe**: at a phase boundary every target's sub-threshold pile clears, so
/// banked-but-un-flipped damage does **not** carry into the next phase (only Health persists, §2.1).
/// Call this at each phase boundary *after* [`tally`] has finalized that phase's deaths.
pub fn clear_phase_piles(pool: &mut [Actor]) {
    for a in pool.iter_mut() {
        a.defense.clear_pile();
    }
}

/// A unit's per-Tempo-card **Finesse** (floor 1) — the magnitude weighed in the evade contest.
fn advance_finesse(a: &Actor) -> u32 {
    a.offense.finesse.max(1)
}

/// The fewest Tempo cards a defender must commit for `cards × Finesse` to **strictly exceed** a ranged
/// attacker's pressed `volley` — the evade contest (Spec §3.1 / §4.2). A tie lands the strike (the
/// avoider must strictly exceed). Floors at 1.
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

// ===========================================================================================
// §4.6 six-phase resolvers.
//
// All resolvers work from **phase-start snapshots** and apply blows order-independently: a body
// mortally wounded mid-phase still lands the blow it committed (§1.3); deaths are finalized only at
// the phase boundary (`tally`), and the per-phase pile is wiped there too (`clear_phase_piles`).
// ===========================================================================================

/// A single melee engagement (§4.2 same-range trade): `attacker` strikes `target` (paying one Tempo),
/// and `target` **strikes back** if it carries a melee answer, can act, and has Tempo (paying one of
/// its own). Both blows are taken from phase-start snapshots so the trade is symmetric even if one
/// side is mortally wounded (§1.3). Used by the **Fray** front clash and a **flank** (both trades).
fn melee_trade(attacker: &mut Actor, target: &mut Actor, log: &mut Vec<String>) {
    if attacker.tempo <= 0 || !attacker.can_contest_now(Range::Melee) || attacker.is_down() {
        return;
    }
    attacker.tempo -= 1;
    let atk = snapshot(attacker);
    let atk_name = attacker.name.clone();
    // Snapshot the strike-back *before* applying the incoming blow (§1.3: the target lands its
    // committed blow even if this hit downs it).
    let back = (target.can_contest_now(Range::Melee) && !target.is_down() && target.tempo > 0)
        .then(|| {
            target.tempo -= 1;
            (snapshot(target), target.name.clone())
        });
    apply_strike(target, atk, &atk_name, log);
    if let Some((s, n)) = back {
        apply_strike(attacker, s, &n, log);
    }
}

/// A one-directional **ranged shot** (§4.2): `attacker` fires at `target`, paying one Tempo; the
/// target may **evade** by strictly exceeding the volley (cards × Finesse) with its own Tempo (a tie
/// lands). An evaded shot does nothing; otherwise it lands (no melee strike-back — a ranged shot at
/// range draws none here; the rear's *own* answer is sequenced by the caller in the Volley).
fn ranged_shot(attacker: &mut Actor, target: &mut Actor, log: &mut Vec<String>) {
    if attacker.tempo <= 0 || !attacker.can_contest_now(Range::Ranged) || attacker.is_down() {
        return;
    }
    attacker.tempo -= 1;
    let volley = advance_finesse(attacker);
    let atk = snapshot(attacker);
    let atk_name = attacker.name.clone();
    if !target.is_down() && try_evade(target, volley, log) {
        return;
    }
    apply_strike(target, atk, &atk_name, log);
}

/// §4.6 — a single interactive **Fray melee strike**: `attacker` strikes `target`, a trade (the
/// target strikes back if able). The public single-pair entry the interactive game routes through.
pub fn fray_one(attacker: &mut Actor, target: &mut Actor, log: &mut Vec<String>) {
    melee_trade(attacker, target, log);
}

/// §4.6 — a single interactive **instant ranged shot** (`resolve: OnCast`): an evade contest then the
/// shot. The public single-pair entry the interactive game routes through.
pub fn ranged_one(attacker: &mut Actor, target: &mut Actor, log: &mut Vec<String>) {
    ranged_shot(attacker, target, log);
}

/// §4.6 #2 — the **Fray** front clash. Each living Vanguard of `attackers` (in `atk_van`) strikes the
/// first living enemy Vanguard (in `def_van`) it can reach, a melee **trade** (the target strikes
/// back). Returns, per attacker index, the **list of enemy Vanguards it attacked** — the input to the
/// breach list ([`compute_locks`]): *attacking* is what locks (§4.6), not being struck or blocking.
/// Call once per side (heroes-attack-foes, then foes-attack-heroes); deaths finalize at the boundary.
pub fn fray_clash(
    attackers: &mut [Actor],
    atk_van: &[bool],
    defenders: &mut [Actor],
    def_van: &[bool],
    log: &mut Vec<String>,
) -> Vec<Vec<usize>> {
    let mut attacked = vec![Vec::new(); attackers.len()];
    for a in 0..attackers.len() {
        if !atk_van[a] || attackers[a].is_down() || !attackers[a].can_contest_now(Range::Melee) {
            continue;
        }
        // Strike the first living enemy Vanguard standing in the way.
        let Some(t) = (0..defenders.len()).find(|&d| def_van[d] && !defenders[d].is_down()) else {
            continue; // no enemy front to engage — nothing to attack (and nothing to lock onto)
        };
        attacked[a].push(t);
        melee_trade(&mut attackers[a], &mut defenders[t], log);
    }
    attacked
}

/// §4.6 **breach list / per-unit lock.** A Vanguard is **locked** for the round iff *some enemy
/// Vanguard it attacked in the Fray is still alive* — **only attacking locks** (being struck,
/// blocking, or evading never does). `attacked[a]` is the enemy-Vanguard indices `a` struck (from
/// [`fray_clash`]); `enemy` is that enemy pool **after** the Fray's deaths are finalized. Returns the
/// lock flag per attacker: `true` = locked (pinned), `false` = **free** (may charge/flank).
pub fn compute_locks(attacked: &[Vec<usize>], enemy: &[Actor]) -> Vec<bool> {
    attacked
        .iter()
        .map(|targets| {
            targets
                .iter()
                .any(|&t| !enemy[t].fallen && !enemy[t].is_down())
        })
        .collect()
}

/// §4.6 #3 — resolve the **Volley**. `charges` were declared by free Vanguards (`flank = false` → a
/// charge across open ground at the enemy Rearguard, landing in the Breach; `flank = true` → adjacent
/// melee on a surviving enemy Vanguard, a trade **in the Volley**). This function resolves the
/// Volley-phase effects only:
///
/// - **Flanks trade now** (same phase = both land, §4.6) — and a flank-**kill** is recorded so the
///   caller can preclude the dead target's own charge (the **intercept**).
/// - **Charges pre-empt:** for each charge, the rear target **answers first** — a melee target strikes
///   back, a ranged target counter-fires, any target may have spent Tempo dodging — all resolving
///   **before** the Breach. Here we resolve the target's strike-back / counter-fire against the
///   *charger* (the pre-empt blow); the charger's own blow lands later, in [`resolve_breach`].
///
/// `heroes`/`foes` are the two pools; charges carry their own `side`. Deaths finalize at the boundary.
pub fn resolve_volley(
    heroes: &mut [Actor],
    foes: &mut [Actor],
    charges: &[Charge],
    log: &mut Vec<String>,
) {
    // 1. Flanks trade in the Volley (both land). Resolve every flank first; a flank-kill silences the
    //    target's Breach charge because the dead unit is gone by the boundary (the intercept).
    for c in charges.iter().filter(|c| c.flank) {
        let (atk_pool, def_pool): (&mut [Actor], &mut [Actor]) = if c.side == 0 {
            (heroes, foes)
        } else {
            (foes, heroes)
        };
        if c.attacker >= atk_pool.len() || c.target >= def_pool.len() {
            continue;
        }
        log.push(format!(
            "{} flanks {} (a trade, in the Volley).",
            atk_pool[c.attacker].name, def_pool[c.target].name
        ));
        melee_trade(&mut atk_pool[c.attacker], &mut def_pool[c.target], log);
    }

    // 2. Charges: the rear answers FIRST (pre-empt) — its strike-back / counter-fire lands now, before
    //    the charger's Breach blow. A ranged rear counter-fires; a melee rear strikes back; either way
    //    it spends from the shared pool and resolves in the Volley.
    for c in charges.iter().filter(|c| !c.flank) {
        let (atk_pool, def_pool): (&mut [Actor], &mut [Actor]) = if c.side == 0 {
            (heroes, foes)
        } else {
            (foes, heroes)
        };
        if c.attacker >= atk_pool.len() || c.target >= def_pool.len() {
            continue;
        }
        if atk_pool[c.attacker].is_down() || def_pool[c.target].is_down() {
            continue;
        }
        let charger_name = atk_pool[c.attacker].name.clone();
        // The rear's pre-emptive answer at the charger crossing open ground.
        if def_pool[c.target].can_contest_now(Range::Ranged) {
            log.push(format!(
                "{} counter-fires the charging {} (pre-empt).",
                def_pool[c.target].name, charger_name
            ));
            ranged_shot(&mut def_pool[c.target], &mut atk_pool[c.attacker], log);
        } else if def_pool[c.target].can_contest_now(Range::Melee) {
            log.push(format!(
                "{} strikes back at the charging {} (pre-empt).",
                def_pool[c.target].name, charger_name
            ));
            if def_pool[c.target].tempo > 0 {
                def_pool[c.target].tempo -= 1;
                let snap = snapshot(&def_pool[c.target]);
                let n = def_pool[c.target].name.clone();
                apply_strike(&mut atk_pool[c.attacker], snap, &n, log);
            }
        }
    }
}

/// §4.6 #4 — resolve the **Breach**. Each non-flank charge whose **charger survived the Volley** lands
/// its melee blow (`resolve: Breach`) on its Rearguard target. A charger killed in the Volley
/// pre-empt **never lands** (§4.6 pre-empt); a flank already resolved in the Volley. This is where a
/// breacher killing a caster **disrupts** its deferred spell — the kill is finalized here, before the
/// Reckoning (the caller drops the deferred entry for any fallen caster).
pub fn resolve_breach(
    heroes: &mut [Actor],
    foes: &mut [Actor],
    charges: &[Charge],
    log: &mut Vec<String>,
) {
    for c in charges.iter().filter(|c| !c.flank) {
        let (atk_pool, def_pool): (&mut [Actor], &mut [Actor]) = if c.side == 0 {
            (heroes, foes)
        } else {
            (foes, heroes)
        };
        if c.attacker >= atk_pool.len() || c.target >= def_pool.len() {
            continue;
        }
        // Pre-empt: a charger downed in the Volley never lands its Breach blow.
        if atk_pool[c.attacker].is_down() || atk_pool[c.attacker].fallen {
            log.push(format!(
                "{} fell in the Volley — its charge never lands.",
                atk_pool[c.attacker].name
            ));
            continue;
        }
        if def_pool[c.target].is_down() {
            continue;
        }
        if atk_pool[c.attacker].can_contest_now(Range::Melee) {
            log.push(format!(
                "{} breaches and strikes {}.",
                atk_pool[c.attacker].name, def_pool[c.target].name
            ));
            let snap = snapshot(&atk_pool[c.attacker]);
            let n = atk_pool[c.attacker].name.clone();
            apply_strike(&mut def_pool[c.target], snap, &n, log);
        }
    }
}

/// §4.6 #5 — resolve the **Reckoning**: each deferred (`resolve: Reckoning`) spell lands **iff its
/// caster is still alive** (not killed/disrupted in the Breach — disrupt, §4.6). A fizzled spell is
/// logged. The effect is applied through the normal [`play_card`] path (AoE hits every member, §4.5).
pub fn resolve_reckoning(
    heroes: &mut [Actor],
    foes: &mut [Actor],
    deferred: &[Deferred],
    log: &mut Vec<String>,
) {
    for d in deferred {
        let caster_alive = {
            let pool = if d.side == 0 { &*heroes } else { &*foes };
            d.caster < pool.len() && !pool[d.caster].fallen && !pool[d.caster].is_down()
        };
        if !caster_alive {
            log.push(format!(
                "{}'s held {} is dropped — its caster fell before the Reckoning (disrupted).",
                d.name, d.card.name
            ));
            continue;
        }
        log.push(format!("{}'s held {} releases.", d.name, d.card.name));
        if d.side == 0 {
            play_card(
                &d.card,
                &d.name,
                d.offense,
                foes,
                heroes,
                Some(d.caster),
                log,
            );
        } else {
            play_card(
                &d.card,
                &d.name,
                d.offense,
                heroes,
                foes,
                Some(d.caster),
                log,
            );
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
    use crate::scenarios::build_character;
    use crate::zones::ZoneBehavior;

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

    // ---- §4.6 six-phase resolver fixtures ----

    use crate::actor::Attack;

    /// A bare melee combatant with explicit stats and ample Tempo (so a test controls who can act).
    fn fighter(name: &str, might: u32, vit: u32, tough: u32) -> Actor {
        let mut a = build_character("Novice", &[]);
        a.name = name.into();
        a.attack = Attack::Melee;
        a.offense.might = might;
        a.offense.finesse = a.offense.finesse.max(1);
        a.defense.health = crate::stats::Health::new(vit, tough);
        a.weapon = fx(vec![]); // a 0-power weapon so the blow is exactly `might`
        a.tempo = 10;
        a
    }

    /// Run one Fray exchange both ways and recompute the lock list for `heroes`, finalizing deaths and
    /// wiping the per-phase piles at the boundary (§4.6).
    fn fray_round(
        heroes: &mut [Actor],
        h_van: &[bool],
        foes: &mut [Actor],
        f_van: &[bool],
    ) -> Vec<bool> {
        let mut log = Vec::new();
        let attacked = fray_clash(heroes, h_van, foes, f_van, &mut log);
        let _ = fray_clash(foes, f_van, heroes, h_van, &mut log);
        tally(heroes, &mut log);
        tally(foes, &mut log);
        let locks = compute_locks(&attacked, foes);
        clear_phase_piles(heroes);
        clear_phase_piles(foes);
        locks
    }

    /// (a) A Fray **death frees a locker** who then **charges**. Two heroes front two foes; hero 0
    /// kills its foe in the Fray (→ free), hero 1's foe survives (→ locked). The freed hero charges the
    /// enemy Rearguard and lands in the Breach.
    #[test]
    fn fray_death_frees_a_locker_who_then_charges() {
        // Hero 0 (Might 5) one-shots foe 0 (V1/T2 ⇒ 5 ≥ 2 flips its only card). Hero 1 (Might 1) cannot
        // kill foe 1 (V3/T5). Foe Might 0 so the heroes survive the trade-back.
        let mut heroes = vec![fighter("Killer", 5, 5, 5), fighter("Pinned", 1, 5, 5)];
        let mut foes = vec![
            fighter("Doomed", 0, 1, 2),
            fighter("Survivor", 0, 3, 5),
            fighter("Rear", 0, 2, 2), // a Rearguard target
        ];
        let f_van = [true, true, false];
        let locks = fray_round(&mut heroes, &[true, true, false], &mut foes, &f_van);
        assert!(foes[0].fallen, "foe 0 dies in the Fray");
        assert!(!locks[0], "the killer is FREE — its struck foe is dead");
        assert!(locks[1], "hero 1 stays LOCKED — its struck foe lives");

        // The freed hero 0 charges the enemy Rearguard (foe 2); it lands in the Breach.
        let charges = [Charge {
            side: 0,
            attacker: 0,
            target: 2,
            flank: false,
        }];
        let rear0 = foes[2].defense.health.remaining;
        let mut log = Vec::new();
        resolve_volley(&mut heroes, &mut foes, &charges, &mut log); // rear has no answer (Might 0, but it does strike back)
        resolve_breach(&mut heroes, &mut foes, &charges, &mut log);
        assert!(
            foes[2].defense.health.remaining < rear0,
            "the freed charger breaches the Rearguard"
        );
    }

    /// (b) The Volley **pre-empt** kills a charger before its Breach blow. A free hero charges a ranged
    /// rear that counter-fires hard enough to drop the charger in the Volley — so its Breach blow never
    /// lands.
    #[test]
    fn volley_pre_empt_kills_the_charger_before_the_breach() {
        let mut heroes = vec![{
            let mut c = fighter("Charger", 4, 1, 2); // fragile: V1/T2
            c.tempo = 0; // spent everything reaching the rear — no Tempo left to dodge the counter-fire
            c
        }];
        let mut foes = vec![{
            let mut r = fighter("Gunner", 9, 3, 2);
            r.attack = Attack::Ranged; // counter-fires the charge
            r.offense.finesse = 1;
            r
        }];
        let charges = [Charge {
            side: 0,
            attacker: 0,
            target: 0,
            flank: false,
        }];
        let mut log = Vec::new();
        // The charger declares no melee target in the Fray; we drive only the Volley + Breach.
        resolve_volley(&mut heroes, &mut foes, &charges, &mut log);
        tally(&mut heroes, &mut log);
        let rear0 = foes[0].defense.health.remaining;
        resolve_breach(&mut heroes, &mut foes, &charges, &mut log);
        assert!(
            heroes[0].is_down(),
            "the pre-empting counter-fire downs the charger"
        );
        assert_eq!(
            foes[0].defense.health.remaining, rear0,
            "a charger killed in the Volley never lands its Breach blow (pre-empt)"
        );
    }

    /// (c) A **flank-kill intercepts** an enemy charge. Foe is charging the hero rear; a freed hero
    /// flanks and kills it in the Volley, so its Breach charge is precluded.
    #[test]
    fn flank_kill_intercepts_a_charge() {
        let mut heroes = vec![
            fighter("Flanker", 5, 3, 2), // kills the enemy Vanguard it flanks
            fighter("Bryn", 0, 3, 2),    // the rear the foe was charging
        ];
        let mut foes = vec![fighter("Breaker", 5, 1, 2)]; // V1/T2 ⇒ dies to the flank
        let charges = [
            Charge {
                side: 1,
                attacker: 0,
                target: 1,
                flank: false,
            }, // foe charges hero rear (Bryn)
            Charge {
                side: 0,
                attacker: 0,
                target: 0,
                flank: true,
            }, // hero flanks the foe Vanguard
        ];
        let bryn0 = heroes[1].defense.health.remaining;
        let mut log = Vec::new();
        resolve_volley(&mut heroes, &mut foes, &charges, &mut log);
        tally(&mut foes, &mut log);
        resolve_breach(&mut heroes, &mut foes, &charges, &mut log);
        assert!(
            foes[0].is_down(),
            "the flank kills the breaker in the Volley"
        );
        assert_eq!(
            heroes[1].defense.health.remaining, bryn0,
            "the dead breaker's charge is intercepted — Bryn is untouched"
        );
    }

    /// (d) A **Breach-kill disrupts** a deferred (`resolve: Reckoning`) effect. A foe winds up a
    /// Reckoning bomb; a hero breaches and kills the caster in the Breach; at the Reckoning the spell
    /// fizzles (no damage to the heroes).
    #[test]
    fn breach_kill_disrupts_a_deferred_spell() {
        let mut heroes = vec![fighter("Breacher", 9, 3, 2)];
        let mut foes = vec![{
            let mut c = fighter("Caster", 0, 1, 2); // V1/T2 — dies to the breach
            c.attack = Attack::Neither; // no pre-empt answer
            c
        }];
        // The caster's held Reckoning bomb: AoE Might 5 to all heroes.
        let bomb = {
            let mut b = fx(vec![Effect::Damage { power: 5 }]);
            b.targets = 5;
            b.resolve = crate::cards::Resolve::Reckoning;
            b
        };
        let deferred = vec![Deferred {
            side: 1,
            caster: 0,
            card: bomb,
            offense: foes[0].offense,
            name: "Caster".into(),
        }];
        let charges = [Charge {
            side: 0,
            attacker: 0,
            target: 0,
            flank: false,
        }];
        let hero0 = heroes[0].defense.health.remaining;
        let mut log = Vec::new();
        resolve_volley(&mut heroes, &mut foes, &charges, &mut log);
        resolve_breach(&mut heroes, &mut foes, &charges, &mut log);
        tally(&mut foes, &mut log); // finalize the caster's death BEFORE the Reckoning
        assert!(foes[0].fallen, "the breacher kills the caster");
        resolve_reckoning(&mut heroes, &mut foes, &deferred, &mut log);
        assert_eq!(
            heroes[0].defense.health.remaining, hero0,
            "the deferred bomb fizzles — its caster died in the Breach (disrupt)"
        );
    }

    /// (e) The per-phase **pile wipes between phases**: sub-threshold Fray damage does NOT carry into
    /// the Volley. A foe takes a sub-Toughness hit in the Fray; after the boundary wipe its pile is 0,
    /// so an identical Volley-phase hit (still sub-Toughness) does not flip a card.
    #[test]
    fn the_pile_wipes_between_phases() {
        // Foe T5: a Might-3 hit banks 3 (no flip). After the Fray boundary the pile must wipe to 0.
        // (Run a one-directional clash so the foe is struck exactly once — a two-way fray would have
        // the hero strike again as strike-back, which is not what this test isolates.)
        let mut heroes = vec![fighter("Chipper", 3, 5, 5)];
        let mut foes = vec![fighter("Wall", 0, 3, 5)];
        {
            let mut log = Vec::new();
            let _ = fray_clash(&mut heroes, &[true], &mut foes, &[true], &mut log);
            tally(&mut foes, &mut log);
            clear_phase_piles(&mut foes);
        }
        assert_eq!(
            foes[0].defense.health_pile, 0,
            "the Fray boundary wiped the sub-threshold pile"
        );
        assert_eq!(
            foes[0].defense.health.remaining, foes[0].defense.health.max,
            "no Health flipped from a single sub-Toughness hit"
        );
        // A second, identical sub-Toughness hit in the Volley still does not flip (3 < 5) — proof the
        // earlier 3 did not carry (3 + 3 = 6 ≥ 5 would have flipped if it had).
        let mut log = Vec::new();
        heroes[0].tempo = 10;
        let charges = [Charge {
            side: 0,
            attacker: 0,
            target: 0,
            flank: true,
        }]; // a flank = a Volley trade
        // Re-front the foe so the flank is legal melee; foe Might 0 so the trade-back is harmless.
        resolve_volley(&mut heroes, &mut foes, &charges, &mut log);
        assert_eq!(
            foes[0].defense.health.remaining, foes[0].defense.health.max,
            "the carried-over pile was wiped — a second sub-Toughness hit still does not flip"
        );
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
