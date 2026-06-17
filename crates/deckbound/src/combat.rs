//! Round helpers: applying typed strikes, the creature phase (focus coverage,
//! free-hits, the gauntlet), and playing action cards (AoE, Rally, …).
//!
//! These are the round-loop mechanics from
//! `docs/games/deckbound/notes/speed-and-tempo.md` and `coordination-and-interruption.md`:
//! **tempo** caps offense, **focus** caps active defense, and what focus can't cover
//! **free-hits**. The interactive duel ([`crate::duel`]) is the engagement atom.

use crate::actor::{Actor, Line, TargetRule};
use crate::cards::Effect;
use crate::duel::Strike;
use crate::state::State;
use crate::stats::{Break, DamageType};

/// The 0-Edge strike profile: `(raw, damage type, precision)`. Power is the
/// magnitude; the weapon supplies the type; a Damage card adds its own power.
pub fn base_strike(a: &Actor) -> (u32, DamageType, u32) {
    let (card_pow, dtype) = a.weapon.primary_damage().unwrap_or((0, DamageType::Blunt));
    (a.offense.power + card_pow, dtype, a.offense.precision)
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

// ---- formation & reach (§4) --------------------------------------------------

/// Living **front-line** indices of a side — the gauntlet's guards.
pub fn front_guards(actors: &[Actor]) -> Vec<usize> {
    actors
        .iter()
        .enumerate()
        .filter(|(_, a)| !a.is_down() && a.line == Line::Front)
        .map(|(i, _)| i)
        .collect()
}

/// Living **back-line** indices of a side.
pub fn back_line(actors: &[Actor]) -> Vec<usize> {
    actors
        .iter()
        .enumerate()
        .filter(|(_, a)| !a.is_down() && a.line == Line::Back)
        .map(|(i, _)| i)
        .collect()
}

/// May hero `h` shift line right now (§4)? Repositioning is free, but a side must always keep
/// **at least one living entity in its front line** — so the last living front-liner may not
/// drop to the back. Moving *to* the front is always legal; moving *to* the back is legal only
/// while another living front-liner remains to hold it.
pub fn can_reposition(actors: &[Actor], h: usize) -> bool {
    let Some(hero) = actors.get(h) else {
        return false;
    };
    if hero.is_down() {
        return false;
    }
    match hero.line {
        Line::Back => true,
        Line::Front => front_guards(actors).iter().any(|&g| g != h),
    }
}

/// Can `attacker` reach `defenders[target]` **without** running the gauntlet (§4)? Ranged
/// bypasses lines entirely; melee reaches the front directly, and the back only once the
/// front line is cleared.
pub fn reaches_directly(attacker: &Actor, defenders: &[Actor], target: usize) -> bool {
    if attacker.ranged {
        return true;
    }
    match defenders[target].line {
        Line::Front => true,
        Line::Back => front_guards(defenders).is_empty(),
    }
}

/// Is this foe's incoming attack a **dive** — a melee runner going for a back-line hero with
/// front-line guards still standing (§4)? Such attacks open the interactive gauntlet.
pub fn is_dive(state: &State, foe: usize, target: usize) -> bool {
    let a = &state.creatures[foe];
    a.runner
        && !a.ranged
        && state.heroes.get(target).map(|h| h.line) == Some(Line::Back)
        && !front_guards(&state.heroes).is_empty()
}

/// Apply a target rule to a candidate set, returning the chosen hero (or `None` if empty).
fn apply_rule(heroes: &[Actor], cand: &[usize], rule: TargetRule) -> Option<usize> {
    match rule {
        TargetRule::Front | TargetRule::Runner => cand.first().copied(),
        TargetRule::LowestBody => cand
            .iter()
            .copied()
            .min_by_key(|&i| heroes[i].defense.body.remaining),
        TargetRule::LeastResolute => cand
            .iter()
            .copied()
            .min_by_key(|&i| heroes[i].defense.resolve),
    }
}

/// Pick a hero index for a creature to attack, per its target rule and **reach** (§4):
/// ranged picks any line, a runner dives for the back line, melee strikes the front.
fn pick_hero_target(state: &State, foe: usize) -> Option<usize> {
    let a = &state.creatures[foe];
    let rule = a
        .behavior()
        .map(|b| b.target_rule)
        .unwrap_or(TargetRule::Front);
    let fronts = front_guards(&state.heroes);
    let backs = back_line(&state.heroes);
    let candidates: Vec<usize> = if a.ranged {
        state
            .heroes
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.is_down())
            .map(|(i, _)| i)
            .collect()
    } else if a.runner {
        if backs.is_empty() { fronts } else { backs }
    } else if fronts.is_empty() {
        backs
    } else {
        fronts
    };
    apply_rule(&state.heroes, &candidates, rule)
}

/// Pick a hero index for a creature to attack, per its target rule, ignoring lines. Returns a
/// living hero, or `None` if the party is down. Used by the legacy [`creature_phase`].
fn pick_target(state: &State, rule: TargetRule) -> Option<usize> {
    let living: Vec<usize> = state
        .heroes
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_down())
        .map(|(i, _)| i)
        .collect();
    apply_rule(&state.heroes, &living, rule)
}

/// The creature phase at round end: every living creature acts. A foe the
/// defending hero can **cover** (focus ≥ the foe's Speed) is negated; otherwise it
/// **free-hits** through the pipeline. A **Runner** is met by the line's combined
/// **drag** (sum of living heroes' Speed): drag ≥ its Speed stops it (and a hero
/// strikes it); else it slips through and free-hits a back-line target.
pub fn creature_phase(state: &mut State) {
    let count = state.creatures.len();
    for ci in 0..count {
        if state.creatures[ci].is_down() || state.engaged[ci] {
            // Foes you dueled or traded this round already exchanged blows; only foes
            // you did not read are uncontested free-hits (§1.8).
            continue;
        }
        let rule = state.creatures[ci]
            .behavior()
            .map(|b| b.target_rule)
            .unwrap_or(TargetRule::Front);
        let runner = state.creatures[ci].runner;
        let foe_speed = state.creatures[ci].offense.speed.max(1);
        let cname = state.creatures[ci].name.clone();
        let (raw, dtype, prec) = base_strike(&state.creatures[ci]);

        if runner {
            // The gauntlet: combined drag vs the runner's Speed.
            let drag: u32 = state
                .heroes
                .iter()
                .filter(|h| !h.is_down())
                .map(|h| h.offense.speed)
                .sum();
            if drag >= foe_speed {
                // Stopped at the wall; the front hero strikes it.
                if let Some(hi) = state.first_living_hero() {
                    let (hraw, hdt, hp) = base_strike(&state.heroes[hi]);
                    let hname = state.heroes[hi].name.clone();
                    state.log.push(format!(
                        "The line drags down the {cname} (drag {drag} ≥ {foe_speed})."
                    ));
                    apply_strike(
                        &mut state.creatures[ci],
                        Strike {
                            raw: hraw,
                            dtype: hdt,
                            precision: hp,
                        },
                        &hname,
                        &mut state.log,
                    );
                }
                continue;
            }
            state.log.push(format!(
                "The {cname} slips the line (drag {drag} < {foe_speed})!"
            ));
        }

        let Some(hi) = pick_target(state, rule) else {
            return;
        };
        // Cover with focus, or eat the free-hit.
        if state.heroes[hi].focus >= foe_speed {
            state.heroes[hi].focus -= foe_speed;
            state.log.push(format!(
                "{} reads the {cname} and negates it ({} focus left).",
                state.heroes[hi].name, state.heroes[hi].focus
            ));
        } else {
            state.log.push(format!(
                "{} can't cover the {cname} — free hit!",
                state.heroes[hi].name
            ));
            apply_strike(
                &mut state.heroes[hi],
                Strike {
                    raw,
                    dtype,
                    precision: prec,
                },
                &cname,
                &mut state.log,
            );
        }
    }
}

/// Play a hero's action card against the warband / party. Returns the tempo cost.
pub fn play_action(state: &mut State, hero: usize, action: usize) -> u32 {
    let card = match state.heroes.get(hero).and_then(|h| h.actions.get(action)) {
        Some(c) => c.clone(),
        None => return 0,
    };
    let hname = state.heroes[hero].name.clone();
    let hero_power = state.heroes[hero].offense.power;
    let precision = state.heroes[hero].offense.precision;
    state.log.push(format!("{hname} plays {}.", card.name));

    for effect in &card.effects {
        match *effect {
            Effect::Damage { power, dtype } => {
                let raw = hero_power + power;
                // Hit up to `targets` living foes (AoE clears a pack).
                let targets: Vec<usize> = state
                    .creatures
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !c.is_down())
                    .map(|(i, _)| i)
                    .take(card.targets as usize)
                    .collect();
                for ti in targets {
                    apply_strike(
                        &mut state.creatures[ti],
                        Strike {
                            raw,
                            dtype,
                            precision,
                        },
                        &hname,
                        &mut state.log,
                    );
                }
            }
            Effect::Rally { resolve } => {
                for h in state.heroes.iter_mut().filter(|h| !h.is_down()) {
                    h.defense.resolve += resolve;
                }
                state
                    .log
                    .push(format!("  the party steels — +{resolve} Resolve to all."));
            }
            Effect::Steel => {
                state.heroes[hero].defense.fear_pile = 0;
                state.heroes[hero].defense.will_break = None;
                state.log.push("  nerves settle.".into());
            }
            Effect::BankSpeed { amount } => {
                state.heroes[hero].tempo += amount as i32;
                state.log.push(format!("  +{amount} tempo banked."));
            }
            Effect::Sunder { armor } => {
                if let Some(ti) = state.first_living_creature() {
                    for v in state.creatures[ti].defense.armor.values_mut() {
                        *v = v.saturating_sub(armor);
                    }
                    state.log.push(format!(
                        "  {}'s armor is sundered.",
                        state.creatures[ti].name
                    ));
                }
            }
            Effect::Disarm | Effect::Shove | Effect::Recover | Effect::Stagger => {
                // First-pass: narrated, mechanically deferred.
                state.log.push("  (effect applied)".into());
            }
        }
    }
    2 // flat tempo cost for an action (tunable)
}

/// Round-end tier 1 — the player's queued **attack** cards (breadth damage), then a
/// decisive-outcome check.
pub fn resolve_attack_cards(state: &mut State) {
    for (hero, idx) in state.queued_cards.clone() {
        if card_is_attack(state, hero, idx) {
            play_action(state, hero, idx);
        }
    }
    crate::game::check_outcome(state);
}

/// Round-end tier 3 — self/ally **buff** cards, after the attacks have landed.
pub fn resolve_buff_cards(state: &mut State) {
    for (hero, idx) in state.queued_cards.clone() {
        if !card_is_attack(state, hero, idx) {
            play_action(state, hero, idx);
        }
    }
}

/// Tier 2's work-list: each **un-engaged** living creature and the hero it attacks (by its
/// target rule). The foe phase resolves these interactively (Defend / Counter / Eat).
pub fn foe_attacks(state: &State) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for ci in 0..state.creatures.len() {
        if state.creatures[ci].is_down() || state.engaged[ci] {
            continue;
        }
        if let Some(hi) = pick_hero_target(state, ci) {
            out.push((ci, hi));
        }
    }
    out
}

/// A foe's uncontested **free hit** (base strike, no Force) on a hero — the "Eat" choice.
pub fn free_hit(state: &mut State, foe: usize, hero: usize) {
    let (raw, dtype, precision) = base_strike(&state.creatures[foe]);
    let cname = state.creatures[foe].name.clone();
    state.log.push(format!(
        "{} takes a free hit from the {cname}.",
        state.heroes[hero].name
    ));
    apply_strike(
        &mut state.heroes[hero],
        Strike {
            raw,
            dtype,
            precision,
        },
        &cname,
        &mut state.log,
    );
}

/// A card whose primary effect is `Damage` is an attack (tier 2); anything else is a
/// support effect (tier 3).
fn card_is_attack(state: &State, hero: usize, idx: usize) -> bool {
    state
        .heroes
        .get(hero)
        .and_then(|h| h.actions.get(idx))
        .map(|c| matches!(c.effects.first(), Some(Effect::Damage { .. })))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenarios;
    use crate::state::{Phase, State};
    use engine::Rng;

    fn state_from(scen: &crate::scenarios::Scenario) -> State {
        let (heroes, creatures) = scen.roster();
        let n_foes = creatures.len();
        State {
            round: 1,
            heroes,
            creatures,
            phase: Phase::Choosing,
            duel: None,
            scenario: Some(scen.clone()),
            exiting: false,
            log: Vec::new(),
            rng: Rng::new(1),
            seed: 1,
            outcome: None,
            engaged: vec![false; n_foes],
            queued_cards: Vec::new(),
            foe_queue: Vec::new(),
            dive: None,
            versus: None,
        }
    }

    #[test]
    fn low_mind_god_is_free_hit_by_the_swarm() {
        // The Blur (Mind 2 → focus 2) cannot cover Speed-3 husks → all free-hit.
        let scen = scenarios::god()
            .into_iter()
            .find(|s| s.name.contains("Blur"))
            .unwrap();
        let mut s = state_from(&scen);
        let before = s.heroes[0].defense.body.remaining;
        creature_phase(&mut s);
        assert!(
            s.heroes[0].defense.body.remaining < before,
            "a thin Mind lets the overflow free-hit"
        );
    }

    #[test]
    fn aoe_clears_a_pack() {
        let scen = scenarios::campaign()
            .into_iter()
            .find(|s| s.name.contains("Warband"))
            .unwrap();
        let mut s = state_from(&scen);
        let hi = s.heroes.iter().position(|h| h.name == "Sefa").unwrap();
        let ai = s.heroes[hi]
            .actions
            .iter()
            .position(|c| c.name == "Firestorm")
            .unwrap();
        let before = s
            .creatures
            .iter()
            .filter(|c| c.name == "Husk" && !c.is_down())
            .count();
        play_action(&mut s, hi, ai);
        let after = s
            .creatures
            .iter()
            .filter(|c| c.name == "Husk" && !c.is_down())
            .count();
        assert!(after < before, "Firestorm should burn multiple husks");
    }

    /// §1.9/§1.3: a hero whose Body hits 0 mid-round is *mortally wounded*, not removed
    /// — it keeps acting (so it lands its committed blows regardless of duel order) and
    /// only falls at the round-end tally.
    #[test]
    fn mortally_wounded_hero_acts_until_the_round_end_tally() {
        let scen = scenarios::god()
            .into_iter()
            .find(|s| s.name.contains("Goliath"))
            .unwrap();
        let mut s = state_from(&scen);
        while !s.heroes[0].is_down() {
            s.heroes[0].defense.take(99, DamageType::Blunt, 0);
        }
        assert!(s.heroes[0].is_down(), "Body is spent");
        assert!(!s.heroes[0].fallen, "but death is not yet finalized");
        assert!(
            s.hero_can_act(0),
            "mortally wounded — still acts this round"
        );
        assert_eq!(s.living_heroes(), 1, "still in the fight until the tally");
        s.heroes[0].fallen = true; // the round-end tally
        assert!(!s.hero_can_act(0), "once fallen, it is out");
        assert_eq!(s.living_heroes(), 0);
    }
}
