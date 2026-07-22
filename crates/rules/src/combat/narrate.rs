//! **The shared narration formatter** - the one storyteller every combat driver reads from, so the fight
//! simulator's log, the card-table arena's journal, and any future surface can never tell the story of a
//! resolution differently. Input: the board as it stood BEFORE the steps resolved, and the [`StepLog`]
//! transcript those steps produced (recorded live via `StepState::set_record`, or re-simulated via
//! [`super::steps::play_steps`] - identical by determinism). Output: the canonical log lines, one step header
//! (`  step K/8: Name`) per step that did anything, each event line carrying its minor step in the prefix
//! column (`target` / `bid` / `strike` / `resolve`, plus `move`).

use super::regions::{Board, Rank, StepLog};
use super::resolve::Combatant;

/// The verb for a body's attack, by **reach x shape** - so the log line carries melee/ranged and single/area in
/// the verb itself, no tag needed. A horde keeps its shape verb; the `x N bodies` in the damage clause marks the
/// volley.
pub(crate) fn strike_verb(u: &Combatant) -> &'static str {
    match (u.ranged && !u.melee, u.aoe) {
        (true, true) => "salvos on",
        (true, false) => "fires on",
        (false, true) => "sweeps",
        (false, false) => "strikes",
    }
}

/// A `StepLog` step string (the step resolvers' labels) to the same step coordinate - so the narration and
/// the wave headers speak one language.
fn label_coord(step: &'static str) -> (u8, &'static str) {
    match step {
        "Step 1: Havoc" => (1, "Havoc"),
        "Step 2: Withdraw" => (2, "Withdraw"),
        "Step 3: Skirmish" => (3, "Skirmish"),
        "Step 4: Crossing" => (4, "Crossing"),
        "Step 5: Defensive Volley" => (5, "Defensive Volley"),
        "Step 6: Raid" => (6, "Raid"),
        "Step 7: Assault" => (7, "Assault"),
        "Step 8: Advance" => (8, "Advance"),
        other => (0, other),
    }
}

/// **The round, step by step - every state change spelled out, none left invisible.** Re-runs the
/// deterministic resolution on a throwaway clone of the pre-round board (identical to what `self.state` just
/// resolved) and walks the [`StepLog`] transcript `play_round` returns.
///
/// Output is a **coordinate language**: a `[ring N] NAME` header when a ring opens, a `ring.subphase Subphase`
/// header per active sub-step, and every event line prefixed with its **exchange step** (`reach` / `dodge` /
/// `strike` / `absorb` / `move` / `downed`) - so any line locates itself as `round . step . event-kind`.
///
/// **The completeness rule: a body's every mutable field is snapshotted each step, and a change to any of them
/// prints a line.** Tempo, Health, rank and region are all diffed against the step before, so no spend, flip,
/// crossing or dissolution can happen silently. A tempo spend with no blow behind it (a slipper paying to evade,
/// a catcher whose target slipped away) was exactly the kind of change that used to hide; now it does not.
///
/// Within a step, the four minor steps of the Interaction primitive, in order - each event line carries its
/// minor step in the prefix column:
/// - **target** - every aimed pair that resolved (landed or dodged), one line each.
/// - **bid** - the contact contest: tempo flipped x Finesse (x bodies) = the reach generated, against the dodge
///   floor it had to clear - or the dodge that answered it, or a reach that connected with nothing.
/// - **strike** - the blows: the free opening blow plus the pour, at Might per blow (a horde swings its whole
///   body count at once; against a horde it *fells bodies* instead of banking damage).
/// - **resolve** - the damage actually applies: Health cards flipped at the Grit bar, remainders discarded when
///   the pile closes, and any body that emptied is DOWNED here (being downed is one thing resolve can do).
/// - (`move` closes the step: crossings, withdrawals, and dissolved outriders rejoining their line.)
///
/// Snapshots enter the first step at full Health and full Tempo (Cadence, stood back up by the Reset); indices are
/// stable across the clone, so names / stats are read from `before`. A step that did nothing prints nothing.
pub fn narrate(before: &Board, transcript: &[StepLog]) -> Vec<String> {
    let rank_word = |r: Rank| match r {
        Rank::Vanguard => "a Vanguard",
        Rank::Rearguard => "a Rearguard",
        Rank::Outrider => "an Outrider",
    };

    let mut out = Vec::new();
    let mut prev_hp: Vec<u32> = before.units.iter().map(|u| u.health).collect();
    let mut prev_tp: Vec<u32> = before.units.iter().map(|u| u.cadence).collect(); // Reset stands tempo up to Cadence
    let mut prev_rk: Vec<Rank> = before.ranks.clone();
    let mut prev_rg: Vec<u8> = before.regions.clone();
    for log in transcript {
        // Each event tagged with its exchange step; rendered under the step's header with a step column.
        let mut lines: Vec<(&'static str, String)> = Vec::new();

        // --- Strikes: sum blows per (attacker -> target) in strike order, then one line each. ---
        let mut order: Vec<(usize, usize)> = Vec::new();
        let mut blows: Vec<u32> = Vec::new();
        for hit in &log.hits {
            let key = (hit.attacker, hit.target);
            match order.iter().position(|k| *k == key) {
                Some(p) => blows[p] += hit.hits,
                None => {
                    order.push(key);
                    blows.push(hit.hits);
                }
            }
        }
        // --- Targets: every aimed pair that resolved this step, landed or dodged - the strike declarations as
        // resolution actually read them (a stale declaration has already dropped). ---
        for &(a, t) in &order {
            lines.push((
                "target",
                format!("{} -> {}", before.units[a].name, before.units[t].name),
            ));
        }
        for r in log.reaches.iter().filter(|r| r.evaded) {
            lines.push((
                "target",
                format!(
                    "{} -> {}",
                    before.units[r.attacker].name, before.units[r.target].name
                ),
            ));
        }

        // Each strike, in the pool -> flow vocabulary: the attacker FLIPS tempo (at its Finesse) to GENERATE the
        // reach that lands the contact - the BID line, both compared numbers on the page - then STRIKES for
        // damage (Might per blow), pouring any tempo it held back. The bid/tempo is a per-attacker fact - a
        // sweep hits many for one flip - so it is stated once, on the attacker's first strike this step.
        let mut tempo_said: Vec<usize> = Vec::new();
        for (&(a, t), &n) in order.iter().zip(&blows) {
            let (an, tn) = (&before.units[a].name, &before.units[t].name);
            // Reach x shape rides the VERB (melee/ranged, single/area). Rank is NOT tagged here - it is listed in
            // the opening roster and narrated when it changes.
            let verb = strike_verb(&before.units[a]);
            let mult = if before.units[a].horde {
                prev_hp[a].max(1) // body count ENTERING this step - what `land` and the bid both read
            } else {
                1
            };
            // The bid (once per attacker): tempo flipped x Finesse x bodies = the reach it generated, against the
            // dodge floor it had to clear. Recovered from the recorded bid, so it always matches `land`. What it
            // did NOT flip for the bid is the pour - stated on the strike line, where it becomes blows.
            let mut strike_prefix = String::new();
            if !tempo_said.contains(&a) {
                tempo_said.push(a);
                let f = before.units[a].finesse.max(1);
                let total = prev_tp[a].saturating_sub(log.tempo[a]);
                match log
                    .reaches
                    .iter()
                    .find(|r| r.attacker == a && r.target == t && !r.evaded)
                {
                    Some(r) => {
                        let rt = r.bid / (f * mult).max(1); // tempo cards that made the bid (bid = rt x f x bodies)
                        let fclause = if before.units[a].horde {
                            format!("Finesse {f} x {mult} bodies")
                        } else {
                            format!("Finesse {f}")
                        };
                        // The dodge FLOOR the reach had to clear: the TARGET's utmost dodge - its whole tempo x its
                        // Finesse (no body multiplier, even for a horde). `reach_cards` sizes the bid to meet this,
                        // and the reacher wins ties - so a connecting bid shows BOTH compared numbers (the reach,
                        // and the dodge it beat), not just its own reach. This is why the tempo cannot be smaller:
                        // below the floor the target simply slips the blow, so only tempo ABOVE it can pour.
                        let tf = before.units[t].finesse.max(1);
                        let tt = prev_tp[t];
                        let against = if tt == 0 {
                            format!(" ({tn} has no tempo to dodge)")
                        } else {
                            format!(
                                " - clears {tn}'s top dodge {} ({tt} tempo x F{tf}), reacher wins ties",
                                tt * tf
                            )
                        };
                        lines.push((
                            "bid",
                            format!(
                                "{an}: flips {rt} tempo at {fclause} = {} reach{against}",
                                r.bid
                            ),
                        ));
                        let pour = total.saturating_sub(rt);
                        if pour > 0 {
                            strike_prefix = format!("pours {pour} more tempo, ");
                        }
                    }
                    // A sweep forms no reach contest (unevadable), so it has no bid line; its one-card cost
                    // rides the strike line, and the verb already says it swept.
                    None => strike_prefix = format!("flips {total} tempo, "),
                }
            }
            let reach = strike_prefix;
            let body = if before.units[t].horde {
                // A horde's bodies are separate Grit-strong pools, no spill. A blow fells a body iff it penetrates
                // (Might - armor >= Grit); a sweep clears the WHOLE pack at once, an aimed blow one body per blow.
                // Both operands of the gate on the page.
                let g = before.units[t].grit.max(1);
                let m = before.units[a].might;
                if m.saturating_sub(before.units[t].armor) < g {
                    format!("cannot dent the pack (Might {m} < Grit {g})")
                } else if before.units[a].aoe {
                    format!("fells the whole pack, {n} bodies (Might {m} >= Grit {g})")
                } else {
                    format!("fells {n} bodies (Might {m} >= Grit {g}, one per blow)")
                }
            } else {
                // Aimed fire on a horde banks into its Grit pile like any body (a horde is defence-normal now).
                // Banked damage = (Might - armor) per blow; a horde attacker swings its whole body count at once.
                let per_blow = before.units[a].might.saturating_sub(before.units[t].armor);
                let dmg = per_blow * mult * n;
                let armor = before.units[t].armor;
                // Show armor only when it bites, so `Might - armor = per-blow` is legible (0 for the roster today).
                let base = if armor > 0 {
                    format!(
                        "Might {} - armor {armor} = {per_blow}",
                        before.units[a].might
                    )
                } else {
                    format!("Might {}", before.units[a].might)
                };
                let how = if before.units[a].horde {
                    format!("{base} x {mult} bodies")
                } else if n > 1 {
                    format!("{base}, {n} strikes")
                } else {
                    base
                };
                format!("{dmg} damage ({how})")
            };
            lines.push(("strike", format!("{an} {verb} {tn}: {reach}{body}")));
        }

        // --- Tempo spent with NO blow behind it - the slip contest, ordered CAUSE BEFORE EFFECT. Resolution is
        // sequential: the reaching side commits first (`engage`), then the defender responds having seen the exact
        // bid (`resolve_evade`). So the reaches are logged first, then the dodges that answer them. Both are
        // products (flip tempo x Finesse = reach); the higher reach wins. A landed strike was stated above. ---
        //
        // Pass 1 - the reaches that were dodged (a landed reach became a Hit, so a body still here reached for a
        // target that out-reached it).
        for i in 0..log.tempo.len() {
            let spent = prev_tp[i].saturating_sub(log.tempo[i]);
            if spent == 0 || log.hits.iter().any(|h| h.attacker == i) {
                continue;
            }
            let f = before.units[i].finesse.max(1);
            let mult = if before.units[i].horde {
                prev_hp[i].max(1)
            } else {
                1
            };
            for r in log.reaches.iter().filter(|r| r.attacker == i) {
                let cards = r.bid / (f * mult).max(1);
                let fclause = if before.units[i].horde {
                    format!("Finesse {f} x {mult} bodies")
                } else {
                    format!("Finesse {f}")
                };
                lines.push((
                    "bid",
                    format!(
                        "{} reaches for {}: flips {cards} tempo at {fclause} to generate {} reach, dodged",
                        before.units[i].name,
                        before.units[r.target].name,
                        r.bid
                    ),
                ));
            }
        }
        // Pass 2 - the dodge that ANSWERED each reach (it saw the bid, then out-reached it), plus any other tempo
        // that bought no reach.
        for i in 0..log.tempo.len() {
            let spent = prev_tp[i].saturating_sub(log.tempo[i]);
            if spent == 0
                || log.hits.iter().any(|h| h.attacker == i)
                || log.reaches.iter().any(|r| r.attacker == i)
            {
                continue; // no spend, already struck, or already shown as a reacher in pass 1
            }
            let name = &before.units[i].name;
            if let Some(worst) = log
                .reaches
                .iter()
                .filter(|r| r.target == i && r.evaded)
                .map(|r| r.bid)
                .max()
            {
                // The same flow, responding: flip tempo to generate reach that OUTWEIGHS the incoming bid. Both
                // values on the page (multiply, never divide): "4 reach clears the 2 reaching it".
                let f = before.units[i].finesse.max(1);
                let dodge = spent * f;
                lines.push((
                    "bid",
                    format!(
                        "{name}: flips {spent} tempo at Finesse {f} to generate {dodge} reach, dodging the {worst} reaching it"
                    ),
                ));
            } else {
                lines.push((
                    "bid",
                    format!("{name}: flips {spent} tempo, no reach connects"),
                ));
            }
        }

        // --- Movements this step owns: the crossings (the step-4 log) and the withdrawals (the step-2 log).
        // Each step is its own transcript entry now, so its moves print in its OWN section as ordinary `move`
        // lines - no borrowed sub-step headers.
        for &i in &log.through {
            lines.push((
                "move",
                format!(
                    "{}: walks into their line, now {}",
                    before.units[i].name,
                    rank_word(log.ranks[i])
                ),
            ));
        }
        for &i in &log.withdrew {
            lines.push((
                "move",
                format!(
                    "{}: withdraws from the enemy ranks, rejoining its line as {}",
                    before.units[i].name,
                    rank_word(log.ranks[i])
                ),
            ));
        }

        // --- Absorb / flips (normal bodies): the pile closes each sub-step, so pair damage with the cards it
        // flipped. A HORDE is not here - its bodies are felled per penetrating blow (the strike line), not piled -
        // so it is skipped. First total the banked damage per target (armor per blow, a horde attacker's whole body
        // count at once) - same formula as the strike lines - so the two always agree. ---
        let mut dmg_to = vec![0u32; log.health.len()];
        for h in &log.hits {
            if before.units[h.target].horde {
                continue; // a horde takes body-fells (per blow), not pile damage
            }
            let per_blow = before.units[h.attacker]
                .might
                .saturating_sub(before.units[h.target].armor);
            let bodies = if before.units[h.attacker].horde {
                prev_hp[h.attacker].max(1)
            } else {
                1
            };
            dmg_to[h.target] += per_blow * bodies * h.hits;
        }
        for i in 0..log.health.len() {
            if before.units[i].horde {
                continue; // felled per blow on the strike line, not a Grit pile
            }
            let (h0, h1) = (prev_hp[i], log.health[i]);
            let name = &before.units[i].name;
            let grit = before.units[i].grit.max(1);
            if h1 < h0 {
                // Flip a Health card at Grit each to ABSORB damage. Flipped x Grit is what the cards soaked; the
                // pile closes each sub-step, so any damage past that is discarded.
                let flipped = h0 - h1;
                let absorbed = flipped * grit;
                let overflow = dmg_to[i].saturating_sub(absorbed);
                let over = if overflow > 0 {
                    format!(" ({overflow} overflow, discarded)")
                } else {
                    String::new()
                };
                let remain = if h1 > 0 {
                    format!(", {h1} health left")
                } else {
                    String::new()
                };
                lines.push((
                    "resolve",
                    format!(
                        "{name}: flips {flipped} health at Grit {grit} to absorb {absorbed} damage{over}{remain}"
                    ),
                ));
            } else if dmg_to[i] > 0 {
                // Banked damage that flipped no card: short of Grit, and the pile clears when this sub-step closes.
                lines.push((
                    "resolve",
                    format!(
                        "{name}: takes {} damage - under Grit {grit}, no health flips (discarded)",
                        dmg_to[i]
                    ),
                ));
            }
        }

        // --- Rank / region changes not already narrated by a crossing (a dissolved outrider rejoining its line). A
        // crosser's new rank was shown on its crossing line; anything else that moved or changed rank is caught
        // here, so no repositioning is silent. ---
        for i in 0..log.ranks.len() {
            if log.through.contains(&i) || log.withdrew.contains(&i) {
                continue; // its move was narrated as a crossing / a withdrawal
            }
            let rank_changed = prev_rk[i] != log.ranks[i];
            let region_changed = prev_rg[i] != log.regions[i];
            if !rank_changed && !region_changed {
                continue;
            }
            let name = &before.units[i].name;
            if prev_rk[i] == Rank::Outrider && log.ranks[i] != Rank::Outrider {
                // Its host formation was wiped, so the outrider state dissolved.
                if region_changed {
                    lines.push((
                        "move",
                        format!(
                            "{name}: outrider dissolves - rejoins its own line as {}",
                            rank_word(log.ranks[i])
                        ),
                    ));
                } else {
                    lines.push((
                        "move",
                        format!(
                            "{name}: outrider dissolves - reforms as {} where it stands",
                            rank_word(log.ranks[i])
                        ),
                    ));
                }
            } else if region_changed {
                lines.push((
                    "move",
                    format!(
                        "{name}: moves across the line (now {})",
                        rank_word(log.ranks[i])
                    ),
                ));
            } else {
                lines.push((
                    "move",
                    format!("{name}: becomes {}", rank_word(log.ranks[i])),
                ));
            }
        }

        // --- Deaths this step. ---
        for &i in &log.fallen {
            let name = &before.units[i].name;
            if before.units[i].horde {
                lines.push((
                    "resolve",
                    format!("{name}: wiped out - no bodies remaining"),
                ));
            } else {
                lines.push(("resolve", format!("{name}: downed - no health remaining")));
            }
        }

        prev_hp = log.health.clone();
        prev_tp = log.tempo.clone();
        prev_rk = log.ranks.clone();
        prev_rg = log.regions.clone();
        if !lines.is_empty() {
            // The coordinate: this step's own header, then the events in resolution order, each in its step
            // column - so any line reads as round . step K/8 . event-kind.
            let (k, name) = label_coord(log.step);
            out.push(format!("  step {k}/8: {name}"));
            let rank = |s: &str| match s {
                "target" => 0,
                "bid" => 1,
                "strike" => 2,
                "resolve" => 3,
                _ => 4, // move - dissolution and repositioning close the step
            };
            let mut evs = lines;
            evs.sort_by_key(|(s, _)| rank(s));
            for (step, text) in evs {
                out.push(format!("      {step:<7} {text}"));
            }
        }
    }
    out
}
