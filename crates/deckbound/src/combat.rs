//! Round helpers: applying typed strikes, the creature phase (focus coverage,
//! free-hits, the gauntlet), and playing action cards (AoE, Rally, …).
//!
//! These are the round-loop mechanics from
//! `docs/games/deckbound/design/speed-and-tempo.md` and `coordination-and-interruption.md`:
//! **tempo** caps offense, **focus** caps active defense, and what focus can't cover
//! **free-hits**. The interactive duel ([`crate::duel`]) is the engagement atom.

use crate::actor::{Actor, TargetRule};
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

/// Pick a hero index for a creature to attack, per its target rule. Returns a
/// living hero, or `None` if the party is down.
fn pick_target(state: &State, rule: TargetRule) -> Option<usize> {
    let living: Vec<usize> = state
        .heroes
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_down())
        .map(|(i, _)| i)
        .collect();
    match rule {
        TargetRule::Front | TargetRule::Runner => living.first().copied(),
        TargetRule::LowestBody => living
            .into_iter()
            .min_by_key(|&i| state.heroes[i].defense.body.remaining),
        TargetRule::LeastResolute => living
            .into_iter()
            .min_by_key(|&i| state.heroes[i].defense.resolve),
    }
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

/// Resolve the round end in tiers (§1.9). The player's duels and trades already landed
/// live this round; now apply queued breadth-attack cards, then the uncontested
/// free-hits from foes nobody read, then self/ally cards (buffs) — attacks before buffs.
pub fn resolve_round(state: &mut State) {
    // Tier: the player's remaining attacks (breadth-damage cards).
    for (hero, idx) in state.queued_cards.clone() {
        if card_is_attack(state, hero, idx) {
            play_action(state, hero, idx);
        }
    }
    crate::game::check_outcome(state);
    if state.outcome.is_some() {
        return;
    }
    // Tier: uncontested attacks — every foe the party did not read free-hits. (Defeat
    // is not finalized here; heroes mortally wounded this tier fall at the round-end
    // tally — §1.9 down-checks at the boundary.)
    state.log.push("-- unread foes strike --".into());
    creature_phase(state);
    // Tier: self/ally effects (buffs), after the attacks.
    for (hero, idx) in state.queued_cards.clone() {
        if !card_is_attack(state, hero, idx) {
            play_action(state, hero, idx);
        }
    }
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

    /// §3.3: an Exposed hero's Focus is 0, so it covers no one and the whole swarm
    /// free-hits — even if it had Focus to spare before overextending. (The Blur has no
    /// armor, unlike Kael, so the husks' chip actually lands.)
    #[test]
    fn exposed_hero_is_free_hit_by_the_whole_swarm() {
        let scen = scenarios::god()
            .into_iter()
            .find(|s| s.name.contains("Blur"))
            .unwrap();
        let mut s = state_from(&scen);
        s.heroes[0].focus = 99;
        s.heroes[0].expose();
        assert_eq!(s.heroes[0].focus, 0, "expose() zeroes Focus");
        let before = s.heroes[0].defense.body.remaining;
        creature_phase(&mut s);
        assert!(
            s.heroes[0].defense.body.remaining < before,
            "an exposed hero covers no one — the whole swarm free-hits"
        );
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
