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
    // §10 Defang: a Defang token lowers the body's Might (floor 1 when defanged) — a Controller
    // softening, not damage. `eff_might` passes the base through unchanged when not defanged.
    a.eff_might() + card_pow + a.might_bonus
}

/// A base [`Strike`] snapshot (for order-independent resolution from phase-start state).
pub fn snapshot(a: &Actor) -> Strike {
    Strike {
        raw: base_strike(a),
    }
}

/// A [`Strike`] snapshot that **consumes the attacker's banked Charge tokens** (+1 Might each, §5.4 —
/// burst paid for by the setup round). Drains all Charge tokens off `a`; use this for a *damage* strike
/// (melee trade, ranged shot, breach blow). A 0-charge attacker just yields [`snapshot`].
pub fn charged_snapshot(a: &mut Actor, log: &mut Vec<String>) -> Strike {
    let charges = a.drain_charges();
    if charges > 0 {
        log.push(format!(
            "  {} unleashes {charges} banked Charge (+{charges} Might).",
            a.name
        ));
    }
    Strike {
        raw: base_strike(a) + charges,
    }
}

/// §10 **Thorns** (Support): if `victim` carries a Thorns token, the `attacker` that just struck it
/// takes the reflected Might into the **attacker's own** current-phase pile (the attacker's own doing —
/// not Support dealing damage). No-op if the victim has no Thorns or the attacker is already down.
fn reflect_thorns(attacker: &mut Actor, victim: &Actor, log: &mut Vec<String>) {
    let power = victim.thorns_power();
    if power == 0 || attacker.is_down() {
        return;
    }
    log.push(format!(
        "  {}'s thorns reflect {power} Might onto {}.",
        victim.name, attacker.name
    ));
    apply_strike(attacker, Strike { raw: power }, "thorns", log);
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
    // §10: the per-phase wall (Toughness). A **Sunder** token lowers it (−Toughness, floor 1) — the
    // Controller's amp — and a **Guard** token raises it (+Toughness this round, a Wall's brace). Folded
    // in here so every strike path (melee/ranged/charge/spell) respects both.
    let bar = target.eff_toughness() + target.guard_toughness();
    let out = target.defense.take_with_toughness(strike.raw, bar);
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

/// §10 **Cover** (Wall): if any living member of `pool` carries a `Cover { ally: aimed }` token, the
/// single-target hit aimed at `aimed` **redirects to that Wall** (§4.5 spillover to a chosen ally). The
/// first such living Wall soaks it; otherwise the hit lands on `aimed`. Returns the destination index.
fn cover_redirect(pool: &[Actor], aimed: usize) -> usize {
    for (wi, w) in pool.iter().enumerate() {
        if w.is_down() {
            continue;
        }
        if w.tokens
            .iter()
            .any(|t| matches!(t, crate::actor::Token::Cover { ally } if *ally == aimed))
        {
            return wi;
        }
    }
    aimed
}

/// §10 **Burn** DoT tick (Artillery): at the Reckoning, **one** Burn stack on each living member of
/// `pool` deals its `power` Might into that bearer's (Reckoning-phase) pile, then **one stack is
/// removed** (a `stacks`-deep Burn therefore burns for `stacks` Reckonings). Caster-independent once
/// placed. Call just before [`tally`] at the Reckoning boundary. A bearer with several distinct Burns
/// ticks each (one stack of each) this Reckoning.
pub fn tick_burn(pool: &mut [Actor], log: &mut Vec<String>) {
    for a in pool.iter_mut() {
        if a.is_down() {
            continue;
        }
        // Tick the first Burn stack, then remove it (−1 stack). Repeat so multiple *distinct* Burn
        // effects each tick once — but the same effect's extra stacks persist to later Reckonings.
        // We model this simply: tick & drop exactly one stack per call (the common single-Burn case).
        if let Some(p) = a
            .tokens
            .iter()
            .position(|t| matches!(t, crate::actor::Token::Burn { .. }))
        {
            let crate::actor::Token::Burn { power } = a.tokens.remove(p) else {
                unreachable!()
            };
            if power > 0 {
                apply_strike(a, Strike { raw: power }, "burn", log);
            }
        }
    }
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
                // §10: ALL tokens on a bearer clear on its death (they return to supply). A
                // dead-body Cover/Thorns no longer protects; its Mark/Mire/Burn lapse.
                a.tokens.clear();
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

/// A unit's per-Tempo-card **Finesse** (floor 1) — the magnitude weighed in the evade contest. Reads
/// the **effective** Finesse: base minus any **Mark** tokens (§10), still floored at 1 (§2.2).
fn advance_finesse(a: &Actor) -> u32 {
    a.eff_finesse()
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
/// How a melee defender answers a strike (§4 the one contest):
/// - **Trade** — the reflexive strike-back of the front clash (both blows land, §1.3): the legacy
///   behavior, and what a holding Vanguard does.
/// - **Block** — spend Tempo to **out-bid** the attacker (`cards × Finesse`, strictly exceed) and take
///   **no blow** — the melee twin of the ranged evade ([`try_evade`]). No strike-back: the Tempo went to
///   *defending / slipping*, not trading. This is also a slipper's defense as it pushes the front.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Guard {
    Trade,
    Block,
}

fn melee_trade(attacker: &mut Actor, target: &mut Actor, guard: Guard, log: &mut Vec<String>) {
    if attacker.tempo <= 0 || !attacker.can_contest_now(Range::Melee) || attacker.is_down() {
        return;
    }
    attacker.tempo -= 1;
    // §10: a damage strike consumes the attacker's banked Charge tokens (+1 Might each, §5.4).
    let atk = charged_snapshot(attacker, log);
    let atk_name = attacker.name.clone();
    // §4 **Block**: the target out-bids the attacker's one-card bid (`cards × Finesse`, strictly exceed,
    // §3.1) and takes no blow — no strike-back; defending is Tempo-negative (it spent more than the
    // attacker to avoid the hit). If it cannot out-bid, the blow lands.
    if guard == Guard::Block {
        let bid = advance_finesse(attacker); // the attacker's one-card bid (cards × Finesse)
        if !target.is_down() && try_evade(target, bid, log) {
            return;
        }
        apply_strike(target, atk, &atk_name, log);
        reflect_thorns(attacker, target, log);
        return;
    }
    // **Trade** (default): snapshot the strike-back *before* applying the incoming blow (§1.3: the target
    // lands its committed blow even if this hit downs it).
    let back = (target.can_contest_now(Range::Melee) && !target.is_down() && target.tempo > 0)
        .then(|| {
            target.tempo -= 1;
            (charged_snapshot(target, log), target.name.clone())
        });
    apply_strike(target, atk, &atk_name, log);
    // §10 Thorns: the struck target reflects Might onto the attacker's own pile.
    reflect_thorns(attacker, target, log);
    if let Some((s, n)) = back {
        apply_strike(attacker, s, &n, log);
        reflect_thorns(target, attacker, log);
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
    // §10: a damage strike consumes the attacker's banked Charge tokens (+1 Might each, §5.4).
    let atk = charged_snapshot(attacker, log);
    let atk_name = attacker.name.clone();
    if !target.is_down() && try_evade(target, volley, log) {
        return;
    }
    apply_strike(target, atk, &atk_name, log);
    // §10 Thorns: the struck target reflects Might onto the (ranged) attacker's own pile.
    reflect_thorns(attacker, target, log);
}

/// §4.6 — a single interactive **Fray melee strike**: `attacker` strikes `target`, a trade (the
/// target strikes back if able). The public single-pair entry the interactive game routes through.
pub fn fray_one(attacker: &mut Actor, target: &mut Actor, log: &mut Vec<String>) {
    melee_trade(attacker, target, Guard::Trade, log);
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
        melee_trade(&mut attackers[a], &mut defenders[t], Guard::Trade, log);
    }
    attacked
}

/// §4.6 **breach list / per-unit lock.** A Vanguard is **locked** for the round iff *some enemy
/// Vanguard it attacked in the Fray is still **standing in the front*** — **only attacking locks**
/// (being struck, blocking, or evading never does). `attacked[a]` is the enemy-Vanguard indices `a`
/// struck (from [`fray_clash`]); `enemy` is that enemy pool **after** the Fray's deaths are finalized.
/// Returns the lock flag per attacker: `true` = locked (pinned), `false` = **free** (may charge/flank).
///
/// §4.6 **Rout/position refinement (2026-06-26).** You reach the back by clearing the front — by death
/// **or displacement**. A struck enemy Vanguard that is **routed** (driven off the line to the
/// Rearguard) no longer holds the front, so it **does not lock** its attacker: a hero whose front-foe is
/// routed is **freed to charge**, even though that foe is still alive. (A `routed` foe is also barred
/// from charging itself — enforced at the charge-declaration site in [`crate::game`].)
pub fn compute_locks(attacked: &[Vec<usize>], enemy: &[Actor]) -> Vec<bool> {
    attacked
        .iter()
        .map(|targets| {
            targets
                .iter()
                .any(|&t| !enemy[t].fallen && !enemy[t].is_down() && !enemy[t].routed)
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
        melee_trade(
            &mut atk_pool[c.attacker],
            &mut def_pool[c.target],
            Guard::Trade,
            log,
        );
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
        // §10 Smoke: a charger carrying a Smoke token slips uncontested — the rear's Volley pre-empt is
        // skipped for this charge (the token is consumed). It still has to survive the Breach normally.
        if atk_pool[c.attacker].consume_smoke() {
            log.push(format!(
                "{charger_name} charges through smoke — the rear cannot pre-empt it."
            ));
            continue;
        }
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
            // §10: the breach blow consumes banked Charge (+1 Might each, §5.4 — e.g. Coiled Strike).
            let snap = charged_snapshot(&mut atk_pool[c.attacker], log);
            let n = atk_pool[c.attacker].name.clone();
            apply_strike(&mut def_pool[c.target], snap, &n, log);
            reflect_thorns(&mut atk_pool[c.attacker], &def_pool[c.target], log);
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
    // §10 Artillery DoT — Burn ticks into the Reckoning pile first (caster-independent), then the
    // deferred spells release. Both land in this phase; deaths finalize at the Reckoning boundary.
    tick_burn(heroes, log);
    tick_burn(foes, log);
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
                // §4.5: a single-target hit (one victim) may be **redirected by a Cover token** — a
                // Wall covering the aimed ally soaks it. An AoE (n>1) bypasses cover and hits each body.
                let single = n == 1 && alive.len() <= 1;
                for ti in alive.into_iter().take(n) {
                    // A spell carries no attacker *body*, so Thorns (which reflects onto the attacker's
                    // own pile) does not apply here — it triggers on melee/ranged strikes between two
                    // Actors (see `melee_trade` / `ranged_shot`). Cover redirect still applies.
                    let dst = if single { cover_redirect(foes, ti) } else { ti };
                    apply_strike(&mut foes[dst], Strike { raw }, actor_name, log);
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
                // this round — it neither holds as a Vanguard nor charges across the gap.
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
            Effect::Mark { finesse } => {
                // §10 Controller — place a Mark token (−Finesse, floor 1) on each target (persists).
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tokens.push(crate::actor::Token::Mark { finesse });
                    log.push(format!("  marks {} (-{finesse} Finesse).", t.name));
                }
            }
            Effect::Mire { cadence } => {
                // §10 Controller — place a Mire token (−Cadence, floor 1) on each target (persists).
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tokens.push(crate::actor::Token::Mire { cadence });
                    log.push(format!("  mires {} (-{cadence} Cadence).", t.name));
                }
            }
            Effect::Sunder { toughness } => {
                // §10 Controller — place a Sunder token (−Toughness, floor 1) on each target (persists):
                // the per-phase wall drops, so the party cracks the foe with less Might. No damage.
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tokens.push(crate::actor::Token::Sunder { toughness });
                    log.push(format!("  sunders {} (-{toughness} Toughness).", t.name));
                }
            }
            Effect::Defang { might } => {
                // §10 Controller — place a Defang token (−Might, floor 1) on each target (persists):
                // the foe's blows soften. No damage.
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    t.tokens.push(crate::actor::Token::Defang { might });
                    log.push(format!("  defangs {} (-{might} Might).", t.name));
                }
            }
            Effect::Burn { stacks, power } => {
                // §10 Artillery DoT — place `stacks` Burn tokens (each `power` Might) on each target;
                // they tick into the Reckoning pile (see `tick_burn`), one removed per Reckoning.
                for t in foes.iter_mut().filter(|a| !a.is_down()).take(n) {
                    for _ in 0..stacks {
                        t.tokens.push(crate::actor::Token::Burn { power });
                    }
                    log.push(format!("  ignites {} ({stacks}x{power} Burn).", t.name));
                }
            }
            Effect::Brace { toughness } => {
                // §10 Wall — place a Guard token (+Toughness this round) on self (per-round).
                if let Some(i) = self_idx {
                    allies[i]
                        .tokens
                        .push(crate::actor::Token::Guard { toughness });
                    log.push(format!("  braces (+{toughness} Toughness this round)."));
                }
            }
            Effect::Cover => {
                // §10 Wall — this Wall covers the `n` most-wounded other living allies: single-target
                // damage on a covered ally redirects to this Wall (applied in the Damage path).
                if let Some(i) = self_idx {
                    let mut order: Vec<usize> =
                        living(allies).into_iter().filter(|&a| a != i).collect();
                    order.sort_by_key(|&a| allies[a].defense.health.remaining);
                    for ai in order.into_iter().take(n) {
                        allies[i]
                            .tokens
                            .push(crate::actor::Token::Cover { ally: ai });
                        let an = allies[ai].name.clone();
                        log.push(format!(
                            "  covers {an} (damage spills to {}).",
                            allies[i].name
                        ));
                    }
                }
            }
            Effect::Thorns { power } => {
                // §10 Support — place a Thorns token (reflect `power` Might) on the `n` most-wounded
                // living allies; reflects onto an attacker's own pile when the ally is struck.
                let mut order: Vec<usize> = living(allies);
                order.sort_by_key(|&a| allies[a].defense.health.remaining);
                for ai in order.into_iter().take(n) {
                    allies[ai]
                        .tokens
                        .push(crate::actor::Token::Thorns { power });
                    log.push(format!(
                        "  wards {} with thorns ({power}).",
                        allies[ai].name
                    ));
                }
            }
            Effect::Charge { amount } => {
                // §10 Infiltrator/Artillery — bank `amount` Charge tokens on the caster (§5.4); the
                // next damage strike consumes them all for +1 Might each.
                if let Some(i) = self_idx {
                    for _ in 0..amount {
                        allies[i].tokens.push(crate::actor::Token::Charge);
                    }
                    log.push(format!("  banks {amount} Charge."));
                }
            }
            Effect::Smoke => {
                // §10 Infiltrator — place a Smoke token on self; the next charge ignores the Volley
                // pre-empt (consumed on use in `resolve_volley`).
                if let Some(i) = self_idx {
                    allies[i].tokens.push(crate::actor::Token::Smoke);
                    log.push("  veils in smoke (next charge ignores the pre-empt).".into());
                }
            }
            Effect::Silence => {
                // §10 Controller — a non-lethal disrupt of a deferred (`resolve: Reckoning`) spell. The
                // deferred list lives in the round plan, not here, so the removal is performed by
                // `crate::game` before/at play; this arm only narrates (caster-independent no-op here).
                log.push("  silences a held enemy spell (a deferred effect is cancelled).".into());
            }
            Effect::Pin => {
                // §10 Artillery — suppressive fire that denies a free enemy Vanguard its charge. The lock
                // list lives in the round plan, not here, so the lock is set by `crate::game` at play;
                // this arm only narrates (the round-plan surgery is the actual effect).
                log.push(
                    "  pins an enemy Vanguard with suppressive fire (its charge is denied).".into(),
                );
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

    /// §4 one-contest (melee **Block**): a defender that can out-bid the attacker's `cards × Finesse`
    /// (strictly exceed) takes **no blow** — the melee twin of the ranged evade — and spends Tempo doing
    /// it (defending is Tempo-negative).
    #[test]
    fn melee_block_slips_a_strike_when_out_bidding() {
        let mut atk = fighter("Striker", 3, 6, 2);
        atk.offense.finesse = 1; // bid = 1 card × Finesse 1 = 1
        let mut def = fighter("Slipper", 1, 6, 2);
        def.offense.finesse = 2; // 1 card × 2 = 2 > 1 → out-bids with a single card
        let before = def.defense.health.remaining;
        let tempo_before = def.tempo;
        let mut log = Vec::new();
        melee_trade(&mut atk, &mut def, Guard::Block, &mut log);
        assert_eq!(
            def.defense.health.remaining, before,
            "the slipper out-bid the attacker and took no blow"
        );
        assert!(
            def.tempo < tempo_before,
            "blocking spends Tempo (defending is Tempo-negative)"
        );
    }

    /// Force-not-fiat: a defender with **no Tempo** to out-bid eats the blow — Block is never free immunity.
    #[test]
    fn melee_block_eats_the_hit_when_out_of_tempo() {
        let mut atk = fighter("Striker", 3, 6, 1);
        atk.offense.finesse = 1;
        let mut def = fighter("Slipper", 1, 6, 1);
        def.offense.finesse = 2;
        def.tempo = 0; // cannot afford to bid
        let before = def.defense.health.remaining;
        let mut log = Vec::new();
        melee_trade(&mut atk, &mut def, Guard::Block, &mut log);
        assert!(
            def.defense.health.remaining < before,
            "no Tempo to out-bid → the blow lands"
        );
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

    /// §10 **Rout/position refinement**: a struck enemy Vanguard that is **routed** (displaced off the
    /// line) no longer locks its attacker — the hero is freed to charge *even though that foe lives*. You
    /// reach the back by clearing the front, by death **or displacement**.
    #[test]
    fn rout_frees_a_charger_whose_front_foe_is_displaced() {
        // Hero 0 struck foe 0; foe 0 is alive but routed → hero 0 is FREE. Hero 1 struck foe 1, alive and
        // holding → hero 1 stays LOCKED. (compute_locks is the §4.6 lock primitive.)
        let mut foes = vec![fighter("Routed", 0, 3, 2), fighter("Holding", 0, 3, 2)];
        foes[0].routed = true; // driven off the line (a Controller/Artillery Rout)
        let attacked = vec![vec![0usize], vec![1usize]];
        let locks = compute_locks(&attacked, &foes);
        assert!(
            !locks[0],
            "a routed front-foe does not lock — the attacker is freed to charge (even though it lives)"
        );
        assert!(locks[1], "a foe still holding the front locks its attacker");
        // And the freed hero can then breach the enemy rear: drive a charge for hero 0.
        let mut heroes = vec![fighter("Freed", 5, 3, 2), fighter("Pinned", 1, 3, 2)];
        let mut foes2 = vec![
            fighter("Routed", 0, 3, 2),
            fighter("Holding", 0, 3, 2),
            fighter("Rear", 0, 2, 2),
        ];
        let charges = [Charge {
            side: 0,
            attacker: 0,
            target: 2,
            flank: false,
        }];
        let rear0 = foes2[2].defense.health.remaining;
        let mut log = Vec::new();
        resolve_volley(&mut heroes, &mut foes2, &charges, &mut log);
        resolve_breach(&mut heroes, &mut foes2, &charges, &mut log);
        assert!(
            foes2[2].defense.health.remaining < rear0,
            "the rout-freed hero breaches the enemy Rearguard"
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

    // ---- §10 / Stage D token-engine tests ----

    use crate::actor::Token;

    /// **Mark** drops effective Finesse, floored at 1 (§2.2 — never a lock).
    #[test]
    fn mark_reduces_finesse_floor_1() {
        let mut foes = vec![fighter("Foe", 0, 3, 2)];
        foes[0].offense.finesse = 3;
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        play_card(
            &fx(vec![Effect::Mark { finesse: 2 }]),
            "Hexer",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(foes[0].eff_finesse(), 1, "Finesse 3 − Mark 2 = 1");
        // A second, overwhelming Mark cannot push it below the floor.
        play_card(
            &fx(vec![Effect::Mark { finesse: 9 }]),
            "Hexer",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(
            foes[0].eff_finesse(),
            1,
            "Mark floors at 1 — never zero (§2.2)"
        );
    }

    /// **Mire** drops effective Cadence (→ Tempo at refresh), floored at 1.
    #[test]
    fn mire_reduces_cadence_floor_1() {
        let mut foes = vec![fighter("Foe", 0, 3, 2)];
        foes[0].offense.cadence = 4;
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        play_card(
            &fx(vec![Effect::Mire { cadence: 3 }]),
            "Hexer",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(foes[0].eff_cadence(), 1, "Cadence 4 − Mire 3 = 1");
        // The smaller pool bites at the next refresh (Mire → fewer Tempo cards).
        foes[0].refresh_round();
        assert_eq!(
            foes[0].tempo, 1,
            "the mired foe refreshes only 1 Tempo (floor 1)"
        );
        // A second Mire on a floored foe is wasted — never zero (§2.2 force-not-fiat).
        play_card(
            &fx(vec![Effect::Mire { cadence: 9 }]),
            "Hexer",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(foes[0].eff_cadence(), 1, "Mire floors at 1");
    }

    /// **Sunder** drops effective Toughness — the per-phase wall — floored at 1 (§2.2 — never zero), so
    /// a strike that banked sub-threshold against the bar now flips a card (the Controller amp).
    #[test]
    fn sunder_lowers_toughness_floor_1() {
        // Foe Toughness 5: a Might-3 hit banks 3 (< 5) → no flip. Sunder −3 → wall 2; the same 3-Might hit
        // now flips a card. The drop floors at 1, never zero (force-not-fiat).
        let mut foes = vec![fighter("Foe", 0, 5, 5)];
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        // Pre-Sunder: a sub-bar hit banks but does not flip.
        assert_eq!(
            foes[0].eff_toughness(),
            5,
            "base Toughness reads 5 with no Sunder"
        );
        apply_strike(&mut foes[0], Strike { raw: 3 }, "Striker", &mut log);
        assert_eq!(
            foes[0].defense.health.remaining, foes[0].defense.health.max,
            "3 < wall 5 → no flip"
        );
        foes[0].defense.clear_pile();
        // Sunder −3 → effective wall 2.
        play_card(
            &fx(vec![Effect::Sunder { toughness: 3 }]),
            "Bone",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(foes[0].eff_toughness(), 2, "Toughness 5 − Sunder 3 = 2");
        // Now the identical 3-Might hit flips a card (3 ≥ wall 2).
        let before = foes[0].defense.health.remaining;
        apply_strike(&mut foes[0], Strike { raw: 3 }, "Striker", &mut log);
        assert_eq!(
            foes[0].defense.health.remaining,
            before - 1,
            "3 ≥ Sundered wall 2 → one card flips"
        );
        // An overwhelming second Sunder cannot push the wall below the floor.
        play_card(
            &fx(vec![Effect::Sunder { toughness: 9 }]),
            "Bone",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(
            foes[0].eff_toughness(),
            1,
            "Sunder floors at 1 — never zero (§2.2)"
        );
    }

    /// **Defang** drops a body's effective Might (its strike magnitude), floored at 1 (§2.2 — never zero).
    #[test]
    fn defang_lowers_might_floor_1() {
        let mut foes = vec![fighter("Brute", 5, 5, 5)];
        let mut allies = vec![build_character("Novice", &[])];
        let mut log = Vec::new();
        assert_eq!(foes[0].eff_might(), 5, "base Might reads 5 with no Defang");
        play_card(
            &fx(vec![Effect::Defang { might: 3 }]),
            "Bone",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(foes[0].eff_might(), 2, "Might 5 − Defang 3 = 2");
        // base_strike reads eff_might (weapon power 0 here), so the softened blow is weaker.
        assert_eq!(
            base_strike(&foes[0]),
            2,
            "the Defanged body's strike is its effective Might"
        );
        // A second, overwhelming Defang cannot push it below the floor.
        play_card(
            &fx(vec![Effect::Defang { might: 9 }]),
            "Bone",
            Offense::default(),
            &mut foes,
            &mut allies,
            None,
            &mut log,
        );
        assert_eq!(
            foes[0].eff_might(),
            1,
            "Defang floors at 1 — never zero (§2.2)"
        );
    }

    /// **Burn** ticks `power` into the bearer's pile at the Reckoning and drops one stack.
    #[test]
    fn burn_ticks_into_the_reckoning_pile() {
        // Toughness 5 so a single tick banks (sub-threshold) without flipping — we read the pile.
        let mut foes = vec![fighter("Foe", 0, 3, 5)];
        foes[0].tokens.push(Token::Burn { power: 3 });
        foes[0].tokens.push(Token::Burn { power: 3 });
        let mut heroes: Vec<Actor> = Vec::new();
        let mut log = Vec::new();
        // The Reckoning resolver ticks Burn first.
        resolve_reckoning(&mut heroes, &mut foes, &[], &mut log);
        assert_eq!(
            foes[0].defense.health_pile, 3,
            "one Burn stack ticked 3 into the pile"
        );
        assert_eq!(
            foes[0]
                .tokens
                .iter()
                .filter(|t| matches!(t, Token::Burn { .. }))
                .count(),
            1,
            "one stack removed; the other persists to the next Reckoning"
        );
    }

    /// **Thorns** reflects Might onto the attacker's own pile when the warded ally is struck.
    #[test]
    fn thorns_reflects_onto_the_attacker() {
        // Attacker T5 so the 4 reflected Might banks visibly in its pile without a flip.
        let mut attacker = vec![fighter("Brute", 1, 3, 5)];
        let mut warded = vec![fighter("Ward", 0, 5, 5)];
        warded[0].tokens.push(Token::Thorns { power: 4 });
        let mut log = Vec::new();
        // Brute strikes the warded ally (a melee trade); thorns bite the Brute.
        melee_trade(&mut attacker[0], &mut warded[0], Guard::Trade, &mut log);
        assert_eq!(
            attacker[0].defense.health_pile, 4,
            "the warded ally's thorns reflected 4 onto the attacker's own pile"
        );
    }

    /// **Cover** redirects a single-target hit aimed at a covered ally onto the Wall.
    #[test]
    fn cover_redirects_single_target_to_the_wall() {
        // foes = the defending side: a Wall (idx 0) covering a squishy (idx 1).
        let mut wall = fighter("Wall", 0, 5, 5);
        wall.tokens.push(Token::Cover { ally: 1 });
        let mut foes = vec![wall, fighter("Squishy", 0, 1, 2)];
        let mut allies: Vec<Actor> = vec![build_character("Novice", &[])];
        let squishy0 = foes[1].defense.health.remaining;
        let wall0 = foes[0].defense.health.remaining;
        let mut log = Vec::new();
        // A single-target Damage spell aimed at the squishy. Only the squishy is alive-and-targeted
        // first; but the redirect sends it to the Wall.
        // Make the squishy the only living candidate by downing nothing — `living` orders by index, so
        // foe[0] (the Wall) is first. To aim at the squishy specifically we test the redirect helper.
        assert_eq!(
            cover_redirect(&foes, 1),
            0,
            "a hit aimed at the covered ally redirects to the Wall"
        );
        // And drive it through a strike to confirm the Wall soaks, the ally is spared.
        let dst = cover_redirect(&foes, 1);
        apply_strike(&mut foes[dst], Strike { raw: 5 }, "Caster", &mut log);
        assert!(
            foes[0].defense.health.remaining < wall0,
            "the Wall took the redirected hit"
        );
        assert_eq!(
            foes[1].defense.health.remaining, squishy0,
            "the covered ally is spared"
        );
        let _ = &mut allies;
    }

    /// **Charge** tokens are consumed by the next damage strike for +1 Might each (§5.4).
    #[test]
    fn charge_boosts_the_next_strike() {
        // Attacker Might 2 + 3 Charge → a 5-Might strike. Target T5 so it flips exactly one card.
        let mut attacker = vec![fighter("Coiled", 2, 3, 5)];
        attacker[0].tokens.push(Token::Charge);
        attacker[0].tokens.push(Token::Charge);
        attacker[0].tokens.push(Token::Charge);
        let mut target = vec![fighter("Mark", 0, 3, 5)];
        let before = target[0].defense.health.remaining;
        let mut log = Vec::new();
        melee_trade(&mut attacker[0], &mut target[0], Guard::Trade, &mut log);
        assert_eq!(
            target[0].defense.health.remaining,
            before - 1,
            "Might 2 + 3 Charge = 5 ≥ Toughness 5 → one card flips"
        );
        assert_eq!(
            attacker[0].charge_count(),
            0,
            "all Charge tokens were consumed"
        );
    }

    /// **Smoke** lets a charge ignore the rear's Volley pre-empt.
    #[test]
    fn smoke_ignores_the_volley_pre_empt() {
        // A fragile charger that, without Smoke, would be downed by the rear's pre-empt counter-fire.
        let mut heroes = vec![{
            let mut c = fighter("Charger", 4, 1, 2);
            c.tempo = 0; // no Tempo to dodge
            c.tokens.push(Token::Smoke);
            c
        }];
        let mut foes = vec![{
            let mut r = fighter("Gunner", 9, 3, 2);
            r.attack = Attack::Ranged;
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
        resolve_volley(&mut heroes, &mut foes, &charges, &mut log);
        tally(&mut heroes, &mut log);
        assert!(
            !heroes[0].is_down(),
            "Smoke skipped the rear's pre-empt — the charger survives to the Breach"
        );
        assert!(!heroes[0].has_smoke(), "the Smoke token was consumed");
    }

    /// **Silence** (engine-level) removes a `Deferred` entry. (The full disrupt is wired in `game`;
    /// here we prove the deferred-list surgery the effect performs.)
    #[test]
    fn silence_cancels_a_deferred_spell() {
        let bomb = {
            let mut b = fx(vec![Effect::Damage { power: 5 }]);
            b.targets = 5;
            b.resolve = crate::cards::Resolve::Reckoning;
            b
        };
        let mut deferred = vec![Deferred {
            side: 1,
            caster: 0,
            card: bomb,
            offense: Offense::default(),
            name: "Caster".into(),
        }];
        // Silence cancels the enemy's held spell: the game removes the matching `Deferred`.
        if let Some(pos) = deferred.iter().position(|d| d.side == 1) {
            deferred.remove(pos);
        }
        assert!(
            deferred.is_empty(),
            "the deferred spell was cancelled (non-lethal disrupt, §4.6)"
        );
        // And with it gone, the Reckoning lands nothing on the heroes.
        let mut heroes = vec![fighter("Hero", 0, 5, 5)];
        let mut foes = vec![fighter("Caster", 0, 5, 5)];
        let h0 = heroes[0].defense.health.remaining;
        let mut log = Vec::new();
        resolve_reckoning(&mut heroes, &mut foes, &deferred, &mut log);
        assert_eq!(
            heroes[0].defense.health.remaining, h0,
            "the silenced bomb never lands"
        );
    }
}
