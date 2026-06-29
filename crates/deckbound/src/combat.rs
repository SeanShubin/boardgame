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

use crate::actor::{Actor, Intention, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::state::{Deferred, State};
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
fn cards_to_evade(defender: &Actor, volley: u32, wins_ties: bool) -> u32 {
    let grade = advance_finesse(defender); // per-Tempo-card Finesse (floor 1)
    if wins_ties {
        // §4 **Shadowstep** (Infiltrator): a tie *slips* — the avoider needs only `cards × grade ≥
        // volley` (ceil division), one card cheaper on an exact tie.
        volley.div_ceil(grade).max(1)
    } else {
        (volley / grade + 1).max(1) // grade·b > volley (strict; a tie lands the hit)
    }
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
    // §4 Shadowstep (Infiltrator): win ties in the contest — a tie slips instead of landing.
    let wins_ties = defender.has("Shadowstep");
    let need = cards_to_evade(defender, volley, wins_ties) as i32;
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Guard {
    /// Reflexive strike-back (the holding Vanguard's front clash) — the default.
    #[default]
    Trade,
    /// Spend Tempo to out-bid and avoid the blow (the slipper's defense).
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
        // §4 **Blitz** (Infiltrator): the first slip each round is free — an uncontested escape that costs
        // no Tempo (the role-conditional tempo-discount). Cleared at Refresh.
        if !target.is_down() && target.has("Blitz") && !target.free_slip_used {
            target.free_slip_used = true;
            log.push(format!(
                "  {} slips free (Blitz — first slip is free).",
                target.name
            ));
            return;
        }
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

/// §4.6 — a single interactive **Fray melee strike**: `attacker` strikes `target`, who answers per its
/// `guard` (§4 one-contest): **Trade** strikes back (the front clash) / **Block** out-bids to slip the
/// blow. The public single-pair entry the interactive game routes through.
pub fn fray_one(attacker: &mut Actor, target: &mut Actor, guard: Guard, log: &mut Vec<String>) {
    melee_trade(attacker, target, guard, log);
}

/// §4 **interception** (the front strikes the runner): a `charger` crossing toward the enemy Rearguard
/// is struck by each living enemy front Vanguard (`front`, `van` marks who is a Vanguard). The charger
/// **slips** each via the Finesse contest ([`try_evade`]) — spending its own Tempo — or takes the blow.
/// A *wide* front drains a crosser slip-by-slip (the weakest-link race), so only a lone **high-Finesse,
/// high-Tempo** body crosses a guarded line; the rest are cut down at it. No new mechanic — the existing
/// melee strike + Tempo contest, applied to the cross. Each interceptor spends one Tempo.
pub fn intercept(charger: &mut Actor, front: &mut [Actor], van: &[bool], log: &mut Vec<String>) {
    for (v, interceptor) in front.iter_mut().enumerate() {
        if charger.is_down() {
            return; // cut down crossing — it never reaches the back
        }
        if !van.get(v).copied().unwrap_or(false)
            || interceptor.is_down()
            || interceptor.stunned
            || interceptor.tempo <= 0
            || !interceptor.can_contest_now(Range::Melee)
        {
            continue;
        }
        interceptor.tempo -= 1;
        let bid = advance_finesse(interceptor); // the interceptor's one-card bid (cards × Finesse)
        if try_evade(charger, bid, log) {
            continue; // slipped this interceptor (Tempo spent on both sides — the attrition)
        }
        let snap = snapshot(interceptor);
        let n = interceptor.name.clone();
        apply_strike(charger, snap, &n, log);
    }
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
// ====================================================================================================
// §4.6 The engagement-schedule resolver — ports the validated `engagement.rs` algorithm onto `Actor`s.
// (Single-target core: schedule order, the one Tempo contest, prey-with-fallback targeting, strike-back.
// Group spillover / AoE integration is a follow-on — see needs-merge/engine-migration-to-engagement-model.md.)
// ====================================================================================================

/// Tempo a defender spends to avoid one attack (§4.6 contest): cards × Fd must strictly exceed Fa, so the
/// minimum cards is `floor(Fa / Fd) + 1`.
fn avoid_cost(attacker_finesse: u32, defender_finesse: u32) -> i32 {
    (attacker_finesse / defender_finesse.max(1)) as i32 + 1
}

/// Is a Rearguard target reachable by this attacker? The shield holds: a Rearguard may be struck only by
/// an **Outrider** (the raid bypasses the shield) or once the defending side has **no living Vanguard**
/// (the front fell — the Breach's `V→R`).
fn rearguard_reachable(attacker_is_outrider: bool, def: &[Actor], def_int: &[Intention]) -> bool {
    if attacker_is_outrider {
        return true;
    }
    !def.iter()
        .enumerate()
        .any(|(j, u)| !u.is_down() && def_int[j] == Intention::Vanguard)
}

/// Does `me` have a **crackable prey** alive on the enemy side? (Prey = `me`'s cycle target.) If so the
/// efficient default holds Tempo for it rather than spending on a fallback rank now.
fn prey_alive(me: &Actor, prey: Intention, def: &[Actor], def_int: &[Intention]) -> bool {
    def.iter()
        .enumerate()
        .any(|(j, u)| !u.is_down() && def_int[j] == prey && me.eff_might() >= u.eff_toughness())
}

/// Pick the best target for `me` among enemies of `tgt_role` it can **crack** (Might ≥ effective
/// Toughness — never swing at what you can't crack), reachable (shield rule for a Rearguard), lowest
/// remaining Health first (finish kills).
fn choose(
    me: &Actor,
    me_is_outrider: bool,
    tgt_role: Intention,
    def: &[Actor],
    def_int: &[Intention],
) -> Option<usize> {
    let reach_back =
        tgt_role != Intention::Rearguard || rearguard_reachable(me_is_outrider, def, def_int);
    if !reach_back {
        return None;
    }
    def.iter()
        .enumerate()
        .filter(|(j, u)| {
            !u.is_down() && def_int[*j] == tgt_role && me.eff_might() >= u.eff_toughness()
        })
        .min_by_key(|(j, u)| (u.defense.health.remaining, *j))
        .map(|(j, _)| j)
}

/// Resolve one engagement pair (attacker role → target role) for `atk_side`, in place on `state`.
fn resolve_pair(state: &mut State, atk_side: u8, atk_role: Intention, tgt_role: Intention) {
    let def_side = 1 - atk_side;
    // --- Declare (read-only): each eligible attacker picks its target, focusing prey first. ---
    let decls: Vec<(usize, usize)> = {
        let atk = state.s_pool(atk_side);
        let atk_int = state.s_intent(atk_side);
        let def = state.s_pool(def_side);
        let def_int = state.s_intent(def_side);
        let mut out = Vec::new();
        for ai in 0..atk.len() {
            if atk[ai].is_down() || atk_int[ai] != atk_role || atk[ai].tempo < 1 {
                continue;
            }
            // Prey-with-fallback: on a non-prey pair, hold Tempo if a crackable prey is still alive.
            if tgt_role != atk_role.prey() && prey_alive(&atk[ai], atk_role.prey(), def, def_int) {
                continue;
            }
            let outrider = atk_role == Intention::Outrider;
            if let Some(ti) = choose(&atk[ai], outrider, tgt_role, def, def_int) {
                out.push((ai, ti));
            }
        }
        out
    };
    // --- Resolve (mutable): spend Tempo, contest, eat + strike-back. ---
    let mut log = std::mem::take(&mut state.log);
    for (ai, ti) in decls {
        let (atk_pool, def_pool): (&mut [Actor], &mut [Actor]) = if atk_side == 0 {
            (&mut state.heroes, &mut state.creatures)
        } else {
            (&mut state.creatures, &mut state.heroes)
        };
        if atk_pool[ai].is_down() || def_pool[ti].is_down() || atk_pool[ai].tempo < 1 {
            continue;
        }
        atk_pool[ai].tempo -= 1; // spend to attack (drained whether or not it is dodged)
        let melee = atk_pool[ai].attack.has(Range::Melee);
        let fa = atk_pool[ai].eff_finesse();
        let might = base_strike(&atk_pool[ai]);
        let bar = def_pool[ti].eff_toughness();
        let cost = avoid_cost(fa, def_pool[ti].eff_finesse());
        // The defender avoids only a blow that would flip a card, and only if it can afford the bid.
        if might >= bar && def_pool[ti].tempo >= cost {
            def_pool[ti].tempo -= cost;
            log.push(format!(
                "{} avoids {}'s strike (-{cost}t).",
                def_pool[ti].name, atk_pool[ai].name
            ));
            continue;
        }
        // Eat it.
        let snap = snapshot(&atk_pool[ai]);
        let an = atk_pool[ai].name.clone();
        apply_strike(&mut def_pool[ti], snap, &an, &mut log);
        reflect_thorns(&mut atk_pool[ai], &def_pool[ti], &mut log);
        // Strike back at a melee attacker if a card is free and it can crack the attacker (a corpse
        // cannot react; never burn Tempo on a strike that bounces off Toughness).
        if melee
            && !def_pool[ti].is_down()
            && !atk_pool[ai].is_down()
            && def_pool[ti].tempo >= 1
            && base_strike(&def_pool[ti]) >= atk_pool[ai].eff_toughness()
        {
            def_pool[ti].tempo -= 1;
            let snap2 = snapshot(&def_pool[ti]);
            let dn = def_pool[ti].name.clone();
            apply_strike(&mut atk_pool[ai], snap2, &dn, &mut log);
        }
    }
    state.log = log;
}

/// §4.6 — resolve one round over the **engagement schedule**, in place on `state`. Tempo is assumed
/// refreshed for the round. Each unit acts by its declared intention (`state.s_intent`); the resolution
/// policy (prey-with-fallback, every-Tempo-spend-must-matter) is ported from `engagement.rs`. Each
/// engagement is a §1.9 boundary: after it, deaths finalize and the per-engagement pile wipes.
pub fn resolve_round(state: &mut State) {
    use Intention::{Outrider, Rearguard, Vanguard};
    const SCHEDULE: &[&[(Intention, Intention)]] = &[
        &[(Vanguard, Outrider)],                        // Intercept
        &[(Rearguard, Outrider)],                       // Volley
        &[(Outrider, Rearguard)],                       // Raid
        &[(Rearguard, Vanguard), (Vanguard, Vanguard)], // Clash
        &[
            (Vanguard, Rearguard),
            (Outrider, Vanguard),
            (Outrider, Outrider),
        ], // Breach
    ];
    for step in SCHEDULE {
        for &(atk_role, tgt_role) in *step {
            resolve_pair(state, 0, atk_role, tgt_role);
            resolve_pair(state, 1, atk_role, tgt_role);
        }
        // Engagement boundary: finalize deaths, then wipe the per-engagement pile (§4.6).
        let mut log = std::mem::take(&mut state.log);
        tally(&mut state.heroes, &mut log);
        tally(&mut state.creatures, &mut log);
        state.log = log;
        clear_phase_piles(&mut state.heroes);
        clear_phase_piles(&mut state.creatures);
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
    use crate::actor::Attack;
    use crate::duel::Strike;
    use crate::scenarios::build_character;

    /// A bare melee combatant with explicit stats and a 0-power weapon (so the blow is exactly `might`).
    fn fighter(name: &str, might: u32, vit: u32, tough: u32) -> Actor {
        let mut a = build_character("Novice", &[]);
        a.name = name.into();
        a.attack = Attack::Melee;
        a.offense.might = might;
        a.offense.finesse = a.offense.finesse.max(1);
        a.defense.health = crate::stats::Health::new(vit, tough);
        a.tempo = 10;
        a
    }

    /// The §4.6 evade/contest cost: cards × Fd must strictly exceed Fa → floor(Fa/Fd)+1.
    #[test]
    fn avoid_cost_is_a_threshold() {
        assert_eq!(avoid_cost(2, 2), 2);
        assert_eq!(avoid_cost(2, 1), 3);
        assert_eq!(avoid_cost(1, 2), 1);
        assert_eq!(avoid_cost(1, 1), 2);
    }

    /// §2.2 pile→bar→pool: a sub-Toughness blow banks but flips nothing; a blow ≥ Toughness flips a card.
    #[test]
    fn a_strike_banks_might_and_toughness_gates_the_flip() {
        let mut def = fighter("D", 1, 2, 3); // Vitality 2, Toughness 3
        let mut log = Vec::new();
        apply_strike(&mut def, Strike { raw: 1 }, "A", &mut log); // 1 < 3 → no flip
        assert_eq!(def.defense.health.remaining, 2);
        def.defense.clear_pile();
        apply_strike(&mut def, Strike { raw: 3 }, "A", &mut log); // 3 ≥ 3 → one card down
        assert_eq!(def.defense.health.remaining, 1);
    }

    /// Recover turns one face-down Health card back up (§5 card-state).
    #[test]
    fn recover_turns_a_health_card_back_up() {
        let mut a = fighter("R", 1, 3, 1);
        a.defense.take(1); // one card flips (Toughness 1)
        assert_eq!(a.defense.health.remaining, 2);
        a.defense.recover_card();
        assert_eq!(a.defense.health.remaining, 3);
    }
}
