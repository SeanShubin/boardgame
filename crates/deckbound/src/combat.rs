//! Combat resolution for the §4.6 **engagement-schedule** model. Damage is untyped Might into the
//! per-engagement pile (pile→bar→pool, §2.2), gated by the target's effective Toughness ([`apply_strike`]),
//! with the one **Tempo contest** ([`try_evade`] / [`avoid_cost`]): a defender strictly out-bids the
//! attacker (`cards × Finesse`) to avoid a blow that would flip a card.
//!
//! The round resolves the fixed [`SCHEDULE`] of engagements — Intercept (`V→O`), Volley (`R→O`),
//! Raid (`O→R`), Clash (`R→V`, `V→V`), Breach (`V→R`, `O→V`, `O→O`) — each a list of
//! `(attacker-role, target-role)` pairs. Each engagement **cycles to exhaustion**:
//! [`resolve_engagement_cycle`] declares every eligible attacker on **both sides** against the same
//! pre-apply board (targets chosen by [`crate::policy`] — role priorities, the back-access / shield gate,
//! focus-fire), spends Tempo, then applies the two pools together (AoE to all members, aimed spillover
//! front-to-back) plus Thorns and melee strike-backs. [`step`] performs one
//! atomic transition of this walk (one engagement-cycle [`Stage::Cycle`] — all pairs both sides resolved
//! together, §1.9 — or an engagement [`Stage::Boundary`] that
//! finalizes deaths via [`tally`] and wipes the per-engagement pile via [`clear_phase_piles`]), holding
//! its cursor in [`State::resolution`] so the resolution serializes through RON and can be observed one
//! step at a time. [`resolve_round`] just drives `step` to completion.
//!
//! [`resolve_reckoning`], [`tick_burn`], and the token applications in [`play_card`] (Burn / Charge /
//! Guard / Cover / Thorns / the Controller debuffs, §10) are wired here for the cast/resolve and
//! status layers. The interactive four-card Clash ([`crate::duel`]) is the optional 1v1 module.
//!
//! **PRINCIPLE (§1.9 / §1.3).** Within one engagement everything resolves order-independently,
//! *including the blow of a body that dies in that same engagement*. The schedule order is the only
//! timing: a unit dead at an engagement boundary takes no further action, so a death **precludes** a
//! later engagement but never reaches back into an earlier one (the disrupt — a kill before the last
//! engagement fizzles a deferred Reckoning spell — is a corollary).

use crate::actor::{Actor, Intention, Range, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::policy;
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
        target.defense.health.remaining(),
        target.defense.health.max()
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
            .min_by_key(|&i| pool[i].defense.health.remaining()),
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
                // Keep at least one card face-up (turn one up if the pool just emptied).
                if a.defense.health.is_empty() {
                    a.defense.health.turn_up();
                }
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
// The §4.6 engagement-schedule resolver lives below (`resolve_pair` / `step` / `resolve_round`),
// alongside the shared strike helpers above (`base_strike`, `snapshot`, `charged_snapshot`,
// `apply_strike`, `reflect_thorns`, `cover_redirect`, `try_evade`, `tick_burn`, `play_card`,
// `resolve_reckoning`). The superseded six-phase resolvers (the Fray clash, the breach-list lock,
// the Volley/Breach blows) have been removed; their behavior is subsumed by `resolve_pair`.
// ===========================================================================================

// ====================================================================================================
// §4.6 The engagement-schedule resolver — ports the validated `engagement.rs` algorithm onto `Actor`s.
// Mechanics: schedule order, cycling-to-exhaustion, the two-pool AoE/spillover accumulator, group
// spillover / melee-crossing payment / weakest-link slip, conditional R→R, back-access gate, and
// melee-reflexive strike-back. The *decisions* (target priority, focus-fire, evade, when to stop) come
// from `crate::policy`; this resolver only applies the rules they imply.
// ====================================================================================================

/// The members of unit `i`'s group within `pool`, in pool (front-to-back) order. `group[j]` is `j`'s
/// group id; a unit shares a group only with same-side units carrying the same id. Mirrors the sim's
/// `group_of`. (Today live groups default to each unit's own index, so every unit is its own singleton
/// — group mechanics are behavior-neutral until grouping is declared.)
fn group_of(group: &[usize], i: usize) -> Vec<usize> {
    let gid = group.get(i).copied().unwrap_or(i);
    (0..group.len())
        .filter(|&j| group.get(j).copied().unwrap_or(j) == gid)
        .collect()
}

/// A stable per-group key (its lowest member index) — used to pay the **collective melee crossing**
/// once per cycle rather than once per striking member. Mirrors the sim's `group_rep`.
fn group_rep(group: &[usize], i: usize) -> usize {
    group_of(group, i).into_iter().min().unwrap_or(i)
}

/// The front-most **living** member of a group — the bodyguard that soaks aimed (single-target) blows.
/// Mirrors the sim's `front_living`.
fn front_living(pool: &[Actor], members: &[usize]) -> Option<usize> {
    members.iter().copied().find(|&j| !pool[j].is_down())
}

/// §4.6 spillover cascade: bank `amount` of aimed Might into a group's living front, overflowing only
/// on a **death** (the unflipped remainder spills to the next living member). A surviving front soaks
/// the rest (the bodyguard). `members` is the cascade head (the front living soaker first), front to
/// back. Each absorbing blow is narrated via [`apply_strike`] so the transcript verifies it. Mirrors
/// the sim's `cascade`.
fn cascade(
    pool: &mut [Actor],
    members: &[usize],
    mut amount: u32,
    attacker: &str,
    log: &mut Vec<String>,
) {
    for &m in members {
        if amount == 0 {
            break;
        }
        if pool[m].is_down() {
            continue;
        }
        apply_strike(&mut pool[m], Strike { raw: amount }, attacker, log);
        if pool[m].is_down() {
            amount = pool[m].defense.health_pile(); // unflipped remainder overflows
            pool[m].defense.clear_pile();
        } else {
            amount = 0; // fully absorbed by the surviving front
        }
    }
}

/// One declared, committed strike awaiting apply (the read of the attacker captured at declare).
struct Decl {
    ai: usize,
    ti: usize,
    might: u32,
    fa: u32,
    melee: bool,
    aoe: bool,
}

/// **Declare** one cycle's strikes for `atk_side` at schedule engagement `step_idx` — a single pass over
/// every living attacker of `atk_side`, collecting each unit's governing strike for **this engagement
/// across all its pairs** (the priority list, timed against the engagement index by
/// [`policy::governing_target`], picks the target — so a unit that should hold for a later engagement, or
/// strike a different pair of *this* engagement, is captured correctly). A melee group pays the collective
/// crossing (every living member −1 Tempo) once per cycle; a working Tempo view prevents a unit/group
/// over-committing within the pass. Spends the Tempo (the actual crossing/strike payment) on `atk_side`
/// and returns the committed [`Decl`]s. Read-only on the defender side.
fn declare_side(state: &mut State, atk_side: u8, step_idx: usize) -> Vec<Decl> {
    let def_side = 1 - atk_side;
    // Which attacker roles even act this engagement (so we skip a unit whose role has no pair here).
    let atk_roles: Vec<Intention> = SCHEDULE[step_idx].iter().map(|&(a, _)| a).collect();
    // --- Read-only declare: collect decls + the Tempo each crossing/strike will spend. ---
    let decls: Vec<Decl> = {
        let atk = state.s_pool(atk_side);
        let atk_int = state.s_intent(atk_side);
        let atk_grp = state.s_group(atk_side);
        let def = state.s_pool(def_side);
        let def_int = state.s_intent(def_side);
        let n = atk.len();
        let mut crossed = vec![false; n];
        let mut tempo: Vec<i32> = atk.iter().map(|a| a.tempo).collect();
        let mut out: Vec<Decl> = Vec::new();
        for ai in 0..n {
            let Some(&atk_role) = atk_int.get(ai) else {
                continue;
            };
            if atk[ai].is_down() || !atk_roles.contains(&atk_role) {
                continue;
            }
            let Some((_role, ti)) =
                policy::governing_target(step_idx, &atk[ai], atk_role, atk, atk_int, def, def_int)
            else {
                continue; // holds for a later engagement, or no crackable target this engagement
            };
            let melee = atk[ai].attack.has(Range::Melee);
            let rep = group_rep(atk_grp, ai);
            let affordable = if melee {
                crossed[rep] || tempo[ai] >= 1
            } else {
                tempo[ai] >= 1
            };
            if !affordable {
                continue;
            }
            if melee {
                if !crossed[rep] {
                    for m in group_of(atk_grp, ai) {
                        if !atk[m].is_down() {
                            tempo[m] = (tempo[m] - 1).max(0);
                        }
                    }
                    crossed[rep] = true;
                }
            } else {
                tempo[ai] -= 1;
            }
            out.push(Decl {
                ai,
                ti,
                might: base_strike(&atk[ai]),
                fa: atk[ai].eff_finesse(),
                melee,
                aoe: atk[ai].aoe,
            });
        }
        out
    };
    // --- Commit the Tempo spend (mirrors the declare working view). ---
    {
        let (atk_grp, atk_pool): (Vec<usize>, &mut [Actor]) = if atk_side == 0 {
            (state.plan.hero_group.clone(), &mut state.heroes)
        } else {
            (state.plan.foe_group.clone(), &mut state.creatures)
        };
        let mut crossed = vec![false; atk_pool.len()];
        for d in &decls {
            if d.melee {
                let rep = group_rep(&atk_grp, d.ai);
                if !crossed[rep] {
                    for m in group_of(&atk_grp, d.ai) {
                        if !atk_pool[m].is_down() {
                            atk_pool[m].tempo = (atk_pool[m].tempo - 1).max(0);
                        }
                    }
                    crossed[rep] = true;
                }
            } else {
                atk_pool[d.ai].tempo -= 1;
            }
        }
    }
    decls
}

/// **Apply** `atk_side`'s committed `decls` onto the defender side (the two-pool accumulator, §4.6):
/// evades pay their bid; AoE banks full Might into **every** target-group member FIRST (counted in-pile,
/// unevadable, no spillover); then aimed spillover cascades front-to-back per group (a lone Evade soaker
/// may dodge; a group walls). Thorns reflect onto each attacker that landed an aimed blow; melee soakers
/// strike back. Mirrors `engagement.rs`'s per-cycle apply. The `might` in each [`Decl`] was captured at
/// declare, so a unit killed earlier this cycle still lands its committed blow (§1.3).
fn apply_side(state: &mut State, atk_side: u8, decls: &[Decl], log: &mut Vec<String>) {
    let def_side = 1 - atk_side;
    let def_grp: Vec<usize> = state.s_group(def_side).to_vec();
    let def_int: Vec<Intention> = state.s_intent(def_side).to_vec();
    let dn = state.s_len(def_side);
    let mut aoe_add: Vec<u32> = vec![0; dn];
    let mut spill_add: Vec<u32> = vec![0; dn];
    let mut sbacks: Vec<(usize, usize)> = Vec::new(); // (soaker, attacker idx)
    let mut evades: Vec<(usize, usize, i32)> = Vec::new(); // (soaker, attacker idx, cost)
    let mut hits: Vec<(usize, usize)> = Vec::new(); // (soaker, attacker idx) — landed aimed blows
    {
        let def = state.s_pool(def_side);
        for d in decls {
            let members = group_of(&def_grp, d.ti);
            if d.aoe {
                for m in members {
                    aoe_add[m] += d.might; // unevadable, full value to each (§4.5)
                }
                continue;
            }
            let soaker = front_living(def, &members).unwrap_or(d.ti);
            // §4.6 Endure-vs-Evade: a Vanguard ENDURES — it holds the line and takes the blow (keeping
            // its Tempo for offense); only an Outrider / Rearguard evades (the sim's `default_hits`). A
            // Vanguard that dodged would break Deal▸Hold (no blow could ever crack the front).
            let soaker_evades = def_int.get(soaker).copied().is_none_or(policy::role_evades);
            if members.len() == 1
                && soaker_evades
                && policy::should_avoid(&def[soaker], d.might, d.fa)
            {
                let cost = policy::avoid_cost(d.fa, def[soaker].eff_finesse());
                evades.push((soaker, d.ai, cost));
                continue;
            }
            spill_add[soaker] += d.might;
            hits.push((soaker, d.ai));
            if d.melee {
                sbacks.push((soaker, d.ai));
            }
        }
    }

    {
        let atk_names: Vec<String> = state
            .s_pool(atk_side)
            .iter()
            .map(|a| a.name.clone())
            .collect();
        let def_pool: &mut [Actor] = if def_side == 0 {
            &mut state.heroes
        } else {
            &mut state.creatures
        };
        for (soaker, ai, cost) in &evades {
            def_pool[*soaker].tempo -= *cost;
            log.push(format!(
                "{} avoids {}'s strike (-{cost}t).",
                def_pool[*soaker].name, atk_names[*ai]
            ));
        }
        for m in 0..def_pool.len() {
            if aoe_add[m] > 0 && !def_pool[m].is_down() {
                def_pool[m].defense.pending.aoe += aoe_add[m];
                apply_strike(
                    &mut def_pool[m],
                    Strike { raw: aoe_add[m] },
                    "area fire",
                    log,
                );
            }
        }
        for s in 0..def_pool.len() {
            if spill_add[s] > 0 {
                // Cascade head: the soaker first, then the rest of its group behind it.
                let head: Vec<usize> = group_of(&def_grp, s)
                    .into_iter()
                    .skip_while(|&m| m != s)
                    .collect();
                let head = if head.is_empty() { vec![s] } else { head };
                cascade(def_pool, &head, spill_add[s], "strike", log);
            }
        }
    }

    // §10 Thorns: a soaker reflects its power onto each attacker that landed an aimed blow this cycle
    // (onto the attacker's own pile). Unevadable AoE draws no Thorns (no aimed soaker).
    for (soaker, atk_i) in &hits {
        let (atk_pool, def_pool): (&mut [Actor], &mut [Actor]) = if atk_side == 0 {
            (&mut state.heroes, &mut state.creatures)
        } else {
            (&mut state.creatures, &mut state.heroes)
        };
        reflect_thorns(&mut atk_pool[*atk_i], &def_pool[*soaker], log);
    }

    // Reflexive strike-backs: only a melee blow draws one, only from a melee-capable soaker, for one
    // Tempo, when it can crack the attacker (focus-fire on the attacker, §4.6). A soaker that committed
    // before dying still answers (§1.3) — gated on Tempo, not on surviving.
    for (soaker, atk_i) in sbacks {
        let (atk_pool, def_pool): (&mut [Actor], &mut [Actor]) = if atk_side == 0 {
            (&mut state.heroes, &mut state.creatures)
        } else {
            (&mut state.creatures, &mut state.heroes)
        };
        if policy::should_strike_back(&def_pool[soaker], &atk_pool[atk_i]) {
            def_pool[soaker].tempo -= 1;
            let snap = snapshot(&def_pool[soaker]);
            let sn = def_pool[soaker].name.clone();
            apply_strike(&mut atk_pool[atk_i], snap, &sn, log);
        }
    }
}

/// Resolve **one engagement-cycle** at schedule engagement `step_idx` (§4.6 / §1.9): a single declare
/// pass collects **every** unit's strike for this engagement across **all its pairs and both sides**
/// (order-independent — both sides declare against the same pre-apply board), then both sides' strikes
/// apply together (AoE-first → spillover cascade → Thorns → strike-backs). Returns `true` if any side
/// committed a strike (the engagement should cycle again — the per-engagement pile persists), `false`
/// when the engagement is exhausted (the caller then crosses the boundary). **Decision-agnostic**: all
/// target / focus-fire / evade / strike-back choices come from [`crate::policy`].
fn resolve_engagement_cycle(state: &mut State, step_idx: usize) -> bool {
    // Both sides declare against the **same** pre-apply state (declaring spends only the declaring
    // side's own Tempo and reads board health, which no declare mutates — so the order of the two
    // declares does not matter, §1.9).
    let decls_0 = declare_side(state, 0, step_idx);
    let decls_1 = declare_side(state, 1, step_idx);
    if decls_0.is_empty() && decls_1.is_empty() {
        return false; // exhausted — no positive-effect strike left this engagement
    }
    let mut log = std::mem::take(&mut state.log);
    // Apply both sides. Each side's strikes mutate only the *other* pool's Health (strike-backs reach
    // back across, but `apply_strike` is `is_down`-safe and `might` was captured at declare), so the
    // two applies are independent and order-independent.
    apply_side(state, 0, &decls_0, &mut log);
    apply_side(state, 1, &decls_1, &mut log);
    state.log = log;
    true
}

/// §4.6 — the fixed **engagement schedule**: five engagements, each a list of `(attacker, target)` role
/// pairs resolved in order. This is the single source of truth shared by [`resolve_round`] and the
/// steppable [`step`] machine — they must walk it identically.
pub const SCHEDULE: &[&[(Intention, Intention)]] = {
    use Intention::{Outrider, Rearguard, Vanguard};
    &[
        &[(Vanguard, Outrider)],                        // Intercept
        &[(Rearguard, Outrider)],                       // Volley
        &[(Outrider, Rearguard)],                       // Raid
        &[(Rearguard, Vanguard), (Vanguard, Vanguard)], // Clash
        &[
            (Vanguard, Rearguard),
            (Outrider, Vanguard),
            (Outrider, Outrider),
            // §4.6 conditional pair: a Rearguard fires on the enemy back-line, but **only once the
            // enemy Vanguard has fallen** (the dropped screen opens the back). Gated by the back-access
            // rule in `policy::can_reach`, so it is a no-op while the enemy front lives.
            (Rearguard, Rearguard),
        ], // Breach
    ]
};

/// Where the steppable resolver's cursor sits within the current engagement. One [`step`] performs
/// exactly one of these transitions, leaving `State` in a serializable resting micro-state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    /// Run **one engagement-cycle** of the current engagement (§4.6): a single declare pass across all
    /// its pairs and both sides, applied together (see [`resolve_engagement_cycle`]). If anything
    /// committed, the engagement cycles again (stays in `Cycle`); otherwise it advances to [`Boundary`].
    Cycle,
    /// The engagement is exhausted — finalize deaths ([`tally`]) and wipe the per-engagement pile
    /// ([`clear_phase_piles`]) on both pools (the §4.6 boundary), then advance to the next engagement.
    Boundary,
}

/// The in-flight resolution cursor for the §4.6 engagement schedule, held in [`State::resolution`] while
/// a round resolves. It indexes into [`SCHEDULE`] (`step` = engagement) and tracks the [`Stage`]
/// (cycling vs the boundary). Each [`step`] advances it one atomic transition; when it runs off the end
/// of the schedule the resolution is complete and [`step`] returns `false`.
///
/// (Today every micro-step still happens synchronously inside `apply(Deploy)` — see [`resolve_round`] —
/// so the live engine never *rests* mid-resolution; the cursor exists so a caller *can* observe the
/// in-between states, and so the whole resolution serializes through RON.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Resolution {
    /// Index into [`SCHEDULE`] — which engagement (Intercept … Breach) is current.
    pub step: usize,
    /// The cursor within the engagement: cycling, or its boundary.
    pub stage: Stage,
}

impl Resolution {
    /// A fresh cursor at the very start of the schedule (Intercept, first cycle).
    pub fn start() -> Self {
        Resolution {
            step: 0,
            stage: Stage::Cycle,
        }
    }
}

/// Perform **one atomic transition** of the §4.6 engagement-schedule resolution on `state`, advancing
/// (and, when needed, initializing) [`State::resolution`]. Returns `true` if more steps remain and
/// `false` when the resolution is complete (the cursor is then cleared and the round should advance).
///
/// One step does exactly one of:
/// - run **one engagement-cycle** ([`resolve_engagement_cycle`]) — a declare-all-pairs-both-sides pass
///   plus its joint apply; if it committed, the engagement cycles again, else it advances to the
///   boundary, or
/// - cross the engagement **boundary**: finalize deaths ([`tally`]) on both pools and wipe the
///   per-engagement pile ([`clear_phase_piles`]) on both, then move to the next engagement.
///
/// The sequence of steps reproduces [`resolve_round`]'s exact end state — `resolve_round` is just
/// `while step(state) {}`.
pub fn step(state: &mut State) -> bool {
    let mut cur = state.resolution.unwrap_or_else(Resolution::start);
    if cur.step >= SCHEDULE.len() {
        // Already complete (defensive): clear and report done.
        state.resolution = None;
        return false;
    }
    match cur.stage {
        Stage::Cycle => {
            // One engagement-cycle. While it makes progress, stay in `Cycle` (the pile persists across
            // cycles); when a cycle commits nothing, the engagement is exhausted → its boundary.
            if !resolve_engagement_cycle(state, cur.step) {
                cur.stage = Stage::Boundary;
            }
        }
        Stage::Boundary => {
            // Engagement boundary: finalize deaths, then wipe the per-engagement pile (§4.6).
            let mut log = std::mem::take(&mut state.log);
            tally(&mut state.heroes, &mut log);
            tally(&mut state.creatures, &mut log);
            state.log = log;
            clear_phase_piles(&mut state.heroes);
            clear_phase_piles(&mut state.creatures);
            cur.step += 1;
            cur.stage = Stage::Cycle;
        }
    }
    if cur.step >= SCHEDULE.len() {
        // Walked off the end — resolution complete.
        state.resolution = None;
        false
    } else {
        state.resolution = Some(cur);
        true
    }
}

/// §4.6 — resolve one round over the **engagement schedule**, in place on `state`. Tempo is assumed
/// refreshed for the round. Each unit acts by its declared intention (`state.s_intent`); the resolution
/// policy (prey-with-fallback, every-Tempo-spend-must-matter) is ported from `engagement.rs`. Each
/// engagement is a §1.9 boundary: after it, deaths finalize and the per-engagement pile wipes.
///
/// Drives the steppable [`step`] machine to completion — the phase-boundary end state is identical to
/// resolving the schedule in one synchronous pass.
pub fn resolve_round(state: &mut State) {
    state.resolution = Some(Resolution::start());
    while step(state) {}
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
                order.sort_by_key(|&i| allies[i].defense.health.remaining());
                let amt = vitality;
                for ai in order.into_iter().take(n) {
                    allies[ai].defense.health.heal(amt);
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
                    order.sort_by_key(|&a| allies[a].defense.health.remaining());
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
                order.sort_by_key(|&a| allies[a].defense.health.remaining());
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
                // pre-empt (consumed on use when the charge is resolved).
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
                order.sort_by_key(|&i| allies[i].defense.health.remaining());
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

    /// The §4.6 evade/contest cost: cards × Fd must strictly exceed Fa → floor(Fa/Fd)+1. (Now lives in
    /// the policy module — the contest *cost* is a decision input, not a mechanic.)
    #[test]
    fn avoid_cost_is_a_threshold() {
        use crate::policy::avoid_cost;
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
        assert_eq!(def.defense.health.remaining(), 2);
        def.defense.clear_pile();
        apply_strike(&mut def, Strike { raw: 3 }, "A", &mut log); // 3 ≥ 3 → one card down
        assert_eq!(def.defense.health.remaining(), 1);
    }

    /// Recover turns one face-down Health card back up (§5 card-state).
    #[test]
    fn recover_turns_a_health_card_back_up() {
        let mut a = fighter("R", 1, 3, 1);
        a.defense.take(1); // one card flips (Toughness 1)
        assert_eq!(a.defense.health.remaining(), 2);
        a.defense.recover_card();
        assert_eq!(a.defense.health.remaining(), 3);
    }

    /// Behavior preservation: driving the round one [`step`] at a time reproduces the **exact** end
    /// state of the batched [`resolve_round`] — same Health, same Tempo, same fallen flags, same log.
    #[test]
    fn stepping_reproduces_resolve_round_exactly() {
        let heroes = vec![fighter("Hero", 3, 4, 1), fighter("Squire", 2, 3, 1)];
        let foes = vec![fighter("Brute", 3, 4, 1), fighter("Imp", 2, 3, 1)];
        let mut batched = crate::game::battle_state(heroes.clone(), foes.clone(), false, 11);
        let mut stepped = crate::game::battle_state(heroes, foes, false, 11);

        // Batched: one synchronous pass.
        resolve_round(&mut batched);
        assert!(
            batched.resolution.is_none(),
            "resolve_round clears the cursor"
        );

        // Stepped: explicit micro-steps, each leaving a serializable resting micro-state.
        stepped.resolution = Some(Resolution::start());
        let mut guard = 0;
        while step(&mut stepped) {
            // Every resting micro-state must round-trip through RON.
            let text = ron::ser::to_string(&stepped).expect("mid-resolution state serializes");
            let _back: State = ron::from_str(&text).expect("and deserializes");
            guard += 1;
            assert!(guard < 1000, "step machine failed to terminate");
        }
        assert!(
            stepped.resolution.is_none(),
            "step clears the cursor when done"
        );

        // The two end states match field-for-field where it matters.
        assert_eq!(stepped.log, batched.log, "logs identical");
        for (a, b) in stepped.heroes.iter().zip(batched.heroes.iter()) {
            assert_eq!(a.defense.health.remaining(), b.defense.health.remaining());
            assert_eq!(a.defense.health_pile(), b.defense.health_pile());
            assert_eq!(a.tempo, b.tempo);
            assert_eq!(a.fallen, b.fallen);
        }
        for (a, b) in stepped.creatures.iter().zip(batched.creatures.iter()) {
            assert_eq!(a.defense.health.remaining(), b.defense.health.remaining());
            assert_eq!(a.defense.health_pile(), b.defense.health_pile());
            assert_eq!(a.tempo, b.tempo);
            assert_eq!(a.fallen, b.fallen);
        }
    }

    /// With **no AoE source** in a scenario (no Actor sets `aoe`), the AoE pool stays 0 across a whole
    /// round — every existing scenario is in this regime, so the two-pool model is observable structure
    /// that does not change their resolution. (P6a makes a populated AoE pool *possible*; see
    /// `aoe_attacker_shreds_a_group_through_the_bodyguard` for the populated path.)
    #[test]
    fn aoe_pending_pool_stays_zero() {
        let heroes = vec![fighter("Hero", 3, 4, 1)];
        let foes = vec![fighter("Brute", 2, 3, 1)];
        let mut state = crate::game::battle_state(heroes, foes, false, 5);
        resolve_round(&mut state);
        for a in state.heroes.iter().chain(state.creatures.iter()) {
            assert_eq!(a.defense.pending.aoe, 0, "no AoE source this phase");
        }
    }

    /// A bare **ranged** combatant (so it deals from the Rearguard) with a 0-power weapon.
    fn shooter(name: &str, might: u32, vit: u32, tough: u32) -> Actor {
        let mut a = fighter(name, might, vit, tough);
        a.attack = Attack::Ranged;
        a
    }

    /// §4.5/§4.6 — **AoE bypasses the bodyguard.** A tough front (T4 Vanguard) groups with and shields
    /// two squishy Mages behind it (same group id). A lone aimed attacker (M3, one strike per cycle)
    /// cannot crack the front, so aimed fire never reaches the back — the Mages live. An **AoE** attacker
    /// of the same Might lands on *every* group member at once (unevadable, no spillover), killing the
    /// shielded back through the shield. Mirrors the sim's `probe_aoe_vs_group`.
    #[test]
    fn aoe_attacker_shreds_a_group_through_the_bodyguard() {
        use crate::actor::Intention;
        let make_group = || {
            vec![
                fighter("Front", 1, 2, 4), // tough front bodyguard (T4)
                shooter("BackA", 3, 1, 2), // shielded back (squishy)
                shooter("BackB", 3, 1, 2),
            ]
        };
        let backs_alive = |aoe: bool| -> usize {
            let attacker = {
                let mut a = shooter("Sniper", 3, 4, 1);
                a.aoe = aoe;
                // One Tempo per round (like the sim's M3/C1 Mage): a single aimed strike cannot
                // accumulate a flip on the T4 front before the engagement boundary wipes the pile.
                a.offense.cadence = 1;
                a.tempo = 1;
                a
            };
            let mut state = crate::game::battle_state(vec![attacker], make_group(), false, 7);
            // Force the foe formation into one group (front-to-back) all declared Vanguard so the front
            // is the cascade soaker; the hero is a lone Rearguard shooter.
            state.plan.hero_intent = vec![Intention::Rearguard];
            state.plan.foe_intent = vec![Intention::Vanguard; 3];
            state.plan.foe_group = vec![0, 0, 0]; // one shared group
            for _ in 0..4 {
                resolve_round(&mut state);
                state.round += 1;
                for a in state.heroes.iter_mut().chain(state.creatures.iter_mut()) {
                    if !a.is_down() {
                        a.refresh_round();
                    }
                }
            }
            crate::combat::tally(&mut state.creatures, &mut state.log);
            state
                .creatures
                .iter()
                .skip(1)
                .filter(|a| !a.is_down())
                .count()
        };
        assert_eq!(
            backs_alive(false),
            2,
            "aimed fire cannot reach the shielded back"
        );
        assert_eq!(
            backs_alive(true),
            0,
            "AoE lands on every member — the back dies through the shield"
        );
    }
}
