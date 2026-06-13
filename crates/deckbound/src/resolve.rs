//! Resolving one round, end to end.
//!
//! Order follows `docs/games/deckbound/design/resolution.md`:
//! **support → morale → charge (gauntlet) → exchange → recover**, with order
//! imposed only where it is inherent. Within the exchange a felled or staggered
//! actor's blow is cancelled; everything else lands. Power is pure magnitude
//! (it cracks armor and drops foes); there is no separate "Power interrupts"
//! rule — only a lethal first-strike, a won read, or a `stagger` card pre-empts.
//!
//! Simplifications in this first playable cut (all faithful in spirit, flagged
//! for the next pass): reach is not enforced (any living foe is targetable in
//! this single-line scenario), tempo is read as Speed (no per-action pool /
//! overextend), momentum banking is not yet modelled, and cards do not exhaust
//! between rounds.

use engine::{Outcome, PlayerId};

use crate::actors::{Behavior, Line, Play};
use crate::read::{Clash, Read, clash};
use crate::state::State;
use crate::stats::DamageType;

/// How much a Rally (or Steel) lifts Resolve — enough that Sefa (base 1) holds
/// against the Howler's Fear 5.
const RALLY_BONUS: i32 = 4;

/// Resolve the whole round from the heroes' recorded declarations.
pub fn resolve_round(state: &mut State) {
    state.log.clear();
    state.log.push(format!("── Round {} ──", state.round));

    reset_transient(state);

    // Snapshot declarations as plain data so we can mutate freely while reading
    // them: (hero index, play, target).
    let decls: Vec<(usize, Play, Option<usize>)> = state
        .declarations
        .iter()
        .enumerate()
        .filter_map(|(i, d)| d.as_ref().map(|d| (i, d.play, d.target)))
        .collect();

    support(state, &decls); // Rally / Steel — before nerve is tested
    morale(state, &decls); // Dread routs, then the Howl
    charge(state); // the gauntlet stops or lets through Runners
    hero_offense(state, &decls); // Strikes, Firestorm, Frostbite
    creature_attacks(state); // the warband answers

    update_outcome(state);
}

fn reset_transient(state: &mut State) {
    for h in &mut state.heroes {
        h.fear_taken = 0;
        h.panicked = false;
    }
    for c in &mut state.creatures {
        c.disabled = false;
        c.reached = None;
    }
}

fn support(state: &mut State, decls: &[(usize, Play, Option<usize>)]) {
    for &(hi, play, _) in decls {
        if state.heroes[hi].is_down() {
            continue;
        }
        match play {
            Play::Rally => {
                let who = state.heroes[hi].name;
                for h in &mut state.heroes {
                    if !h.is_down() {
                        h.rally_bonus = h.rally_bonus.max(RALLY_BONUS);
                    }
                }
                state
                    .log
                    .push(format!("{who} Rallies — the party steels its nerve."));
            }
            Play::Steel => {
                let h = &mut state.heroes[hi];
                let who = h.name;
                h.rally_bonus = h.rally_bonus.max(RALLY_BONUS);
                state.log.push(format!("{who} steels their own nerve."));
            }
            _ => {}
        }
    }
}

fn morale(state: &mut State, decls: &[(usize, Play, Option<usize>)]) {
    // Dread can rout a creature before it acts.
    for &(hi, play, target) in decls {
        if play != Play::Dread || state.heroes[hi].is_down() {
            continue;
        }
        let Some(ci) = target else { continue };
        if !state.creatures[ci].alive() {
            continue;
        }
        let spirit = state.heroes[hi].spirit as i32;
        let who = state.heroes[hi].name;
        let cname = state.creatures[ci].name;
        if spirit > state.creatures[ci].resolve {
            state.creatures[ci].disabled = true;
            state
                .log
                .push(format!("{who}'s Dread routs the {cname} — it cowers."));
        } else {
            state
                .log
                .push(format!("The {cname} shrugs off {who}'s Dread."));
        }
    }

    // The Howl tests the least-resolute hero's nerve.
    for ci in 0..state.creatures.len() {
        let c = &state.creatures[ci];
        if !c.alive() || c.disabled || c.behavior != Behavior::Howl {
            continue;
        }
        let fear = c.fear as i32;
        let cname = c.name;
        let Some(hi) = least_resolute(state) else {
            continue;
        };
        let who = state.heroes[hi].name;
        state
            .log
            .push(format!("The {cname} howls Fear {fear} at {who}."));
        if fear > state.heroes[hi].resolve() {
            let h = &mut state.heroes[hi];
            h.fear_taken += fear;
            h.panicked = true;
            state
                .log
                .push(format!("{who} panics — the action is lost!"));
        } else {
            state.log.push(format!("{who} holds firm."));
        }
    }
}

fn charge(state: &mut State) {
    let drag = state.front_drag();
    for ci in 0..state.creatures.len() {
        let c = &state.creatures[ci];
        if !c.alive() || c.disabled || !c.runner {
            continue;
        }
        let cspeed = c.speed;
        let cname = c.name;
        let Some(target) = lowest_body_any(state) else {
            continue;
        };
        if drag >= cspeed {
            state.log.push(format!(
                "The wall drags the {cname} (drag {drag} ≥ Speed {cspeed}) — stopped cold."
            ));
            if let Some(gi) = strongest_holding_front(state) {
                let (gp, gtype, gname) = {
                    let h = &state.heroes[gi];
                    (h.power, h.strike_type, h.name)
                };
                let flips = state.creatures[ci].take_hit(gp, gtype);
                state
                    .log
                    .push(format!("{gname} strikes the stopped {cname} — {flips} down."));
                if !state.creatures[ci].alive() {
                    state.log.push(format!("The {cname} falls."));
                }
            }
            state.creatures[ci].disabled = true; // staggered: the run is cancelled
        } else {
            let who = state.heroes[target].name;
            state.creatures[ci].reached = Some(target);
            state.log.push(format!(
                "The {cname} (Speed {cspeed}) slips the thin wall (drag {drag}) and bears down on {who}!"
            ));
        }
    }
}

fn hero_offense(state: &mut State, decls: &[(usize, Play, Option<usize>)]) {
    for &(hi, play, target) in decls {
        if state.heroes[hi].is_down() || state.heroes[hi].panicked {
            continue;
        }
        match play {
            Play::Read(Read::Strike) | Play::Bash => {
                if let Some(ci) = target {
                    hero_strike(state, hi, ci);
                }
            }
            Play::Firestorm => firestorm(state, hi),
            Play::Frostbite => {
                if let Some(ci) = target {
                    frostbite(state, hi, ci);
                }
            }
            _ => {}
        }
    }
}

fn creature_attacks(state: &mut State) {
    for ci in 0..state.creatures.len() {
        let c = &state.creatures[ci];
        if !c.alive() || c.disabled {
            continue;
        }
        match c.behavior {
            Behavior::Bluff => ironclad_attack(state, ci),
            Behavior::Swarm => swarm_attack(state, ci),
            Behavior::Runner => {
                if let Some(hi) = state.creatures[ci].reached {
                    let cname = state.creatures[ci].name;
                    let who = state.heroes[hi].name;
                    state
                        .log
                        .push(format!("The {cname} runs {who} down!"));
                    creature_hit_hero(state, ci, hi);
                }
            }
            Behavior::Howl => {} // resolved in morale
        }
    }
}

/// The Ironclad presses the weakest front-liner with a hidden Strike / Feint.
fn ironclad_attack(state: &mut State, ci: usize) {
    let Some(hi) = weakest_front(state).or_else(|| lowest_body_any(state)) else {
        return;
    };
    let cname = state.creatures[ci].name;
    let cpow = state.creatures[ci].power;
    let who = state.heroes[hi].name;

    // The bluff: the deck commits Strike or Feint, hidden until now.
    let strikes = state.rng.below(2) == 0;
    if !strikes {
        state
            .log
            .push(format!("The {cname} feints at {who} — no blow falls."));
        return;
    }

    match engagement_read(state, hi, ci) {
        None => {
            // Not engaging it — a free strike on the exposed hero.
            state
                .log
                .push(format!("The {cname} strikes the exposed {who} (Pow {cpow})."));
            creature_hit_hero(state, ci, hi);
        }
        Some(read) => match clash(Read::Strike, read) {
            Clash::DefenderWins => {
                state
                    .log
                    .push(format!("{who} reads the {cname} and turns the blow aside."));
                // A Riposte counters on a won defense.
                if matches!(
                    state.declarations[hi].as_ref().map(|d| d.play),
                    Some(Play::Riposte)
                ) {
                    hero_strike(state, hi, ci);
                }
            }
            Clash::AttackerWins => {
                state.log.push(format!(
                    "{who}'s Scheme is shattered — the {cname} lands Pow {cpow}!"
                ));
                creature_hit_hero(state, ci, hi);
            }
            Clash::Mirror => {
                state
                    .log
                    .push(format!("{who} and the {cname} trade blows."));
                hero_strike(state, hi, ci);
                creature_hit_hero(state, ci, hi);
            }
        },
    }
}

/// The swarm only reaches heroes who left the wall (are attacking); a held wall
/// keeps them off.
fn swarm_attack(state: &mut State, ci: usize) {
    let count = state.creatures[ci].count;
    let per = state.creatures[ci].power;
    let cname = state.creatures[ci].name;
    if count == 0 {
        return;
    }
    // Exposed = a living front-liner who is attacking this round.
    let exposed: Option<usize> = state
        .heroes
        .iter()
        .enumerate()
        .filter(|(i, h)| {
            !h.is_down()
                && h.line == Line::Front
                && state.declarations[*i]
                    .as_ref()
                    .is_some_and(|d| d.play.is_attacking())
        })
        .min_by_key(|(_, h)| h.body.remaining)
        .map(|(i, _)| i);

    match exposed {
        None => state
            .log
            .push(format!("The wall holds the {cname} at bay.")),
        Some(hi) => {
            let raw = count * per;
            let who = state.heroes[hi].name;
            state
                .log
                .push(format!("{count} {cname} swarm the exposed {who} (Pow {raw})."));
            creature_raw_hit_hero(state, hi, raw, DamageType::Blunt, cname);
        }
    }
}

/// A hero's offensive blow against a creature.
fn hero_strike(state: &mut State, hi: usize, ci: usize) {
    if !state.creatures[ci].alive() {
        return;
    }
    let (power, dtype, who) = {
        let h = &state.heroes[hi];
        (h.power, h.strike_type, h.name)
    };
    let cname = state.creatures[ci].name;
    let flips = state.creatures[ci].take_hit(power, dtype);
    if flips > 0 {
        state.log.push(format!(
            "{who} strikes the {cname} ({}) — {flips} down.",
            dtype.name()
        ));
    } else {
        state
            .log
            .push(format!("{who}'s blow glances off the {cname}."));
    }
    if !state.creatures[ci].alive() {
        state.log.push(format!("The {cname} falls."));
    }
}

/// Sefa's Firestorm: heat to up to five distinct front-line foes (single
/// creatures first, then swarm members; Runners are not in the front).
fn firestorm(state: &mut State, hi: usize) {
    let mag = state.heroes[hi].magic;
    let who = state.heroes[hi].name;

    let mut slots: Vec<usize> = Vec::new();
    for (ci, c) in state.creatures.iter().enumerate() {
        if c.alive() && !c.runner && !c.is_swarm() {
            slots.push(ci);
        }
    }
    for (ci, c) in state.creatures.iter().enumerate() {
        if c.alive() && !c.runner && c.is_swarm() {
            for _ in 0..c.count {
                slots.push(ci);
            }
        }
    }
    slots.truncate(5);
    if slots.is_empty() {
        state
            .log
            .push(format!("{who}'s Firestorm finds nothing in reach."));
        return;
    }

    let before: Vec<(u32, u32, bool)> = state
        .creatures
        .iter()
        .map(|c| (c.body.remaining, c.count, c.alive()))
        .collect();
    for &ci in &slots {
        state.creatures[ci].take_hit(mag, DamageType::Heat);
    }

    let mut targeted: Vec<usize> = slots.clone();
    targeted.sort_unstable();
    targeted.dedup();

    let mut parts = Vec::new();
    for ci in targeted {
        let c = &state.creatures[ci];
        if c.is_swarm() {
            let burned = before[ci].1 - c.count;
            if burned > 0 {
                parts.push(format!("{burned} {} burned ({} left)", c.name, c.count));
            }
        } else if before[ci].2 && !c.alive() {
            parts.push(format!("the {} burns down", c.name));
        } else {
            let lost = before[ci].0 - c.body.remaining;
            if lost > 0 {
                parts.push(format!("{} −{lost}", c.name));
            } else {
                parts.push(format!("{} unscathed", c.name));
            }
        }
    }
    state.log.push(format!(
        "{who}'s Firestorm (heat {mag}) sweeps the front: {}.",
        parts.join(", ")
    ));
}

/// Sefa's Frostbite: cold damage that also slows the target.
fn frostbite(state: &mut State, hi: usize, ci: usize) {
    if !state.creatures[ci].alive() {
        return;
    }
    let (mag, who) = {
        let h = &state.heroes[hi];
        (h.magic, h.name)
    };
    let cname = state.creatures[ci].name;
    let flips = state.creatures[ci].take_hit(mag, DamageType::Cold);
    state.creatures[ci].speed = state.creatures[ci].speed.saturating_sub(2);
    state
        .log
        .push(format!("{who}'s Frostbite chills the {cname} (−{flips}) and slows it."));
    if !state.creatures[ci].alive() {
        state.log.push(format!("The {cname} falls."));
    }
}

/// Apply a creature's own Strike to a hero.
fn creature_hit_hero(state: &mut State, ci: usize, hi: usize) {
    let (raw, ty, cname) = {
        let c = &state.creatures[ci];
        (c.power, c.strike_type, c.name)
    };
    creature_raw_hit_hero(state, hi, raw, ty, cname);
}

/// Apply `raw` typed damage from `source` to a hero, through armor and toughness.
fn creature_raw_hit_hero(
    state: &mut State,
    hi: usize,
    raw: u32,
    ty: DamageType,
    source: &str,
) {
    let who = state.heroes[hi].name;
    let after = raw.saturating_sub(state.heroes[hi].armor.reduce(ty));
    let flips = state.heroes[hi].body.flips_for(after);
    state.heroes[hi].body.remaining -= flips;
    if flips > 0 {
        state
            .log
            .push(format!("The {source} hits {who} for {flips}."));
    } else {
        state
            .log
            .push(format!("The {source}'s blow can't pierce {who}."));
    }
    if state.heroes[hi].is_down() {
        state.log.push(format!("{who} falls!"));
    }
}

/// Whether `hi` is reading creature `ci` this round, and with which read. A
/// holding hero (defensive read) reads all comers; an attacker reads only the
/// foe it committed to.
fn engagement_read(state: &State, hi: usize, ci: usize) -> Option<Read> {
    let d = state.declarations[hi].as_ref()?;
    match d.play {
        Play::Read(Read::Strike) | Play::Bash | Play::Frostbite | Play::Dread => {
            if d.target == Some(ci) {
                Some(Read::Strike)
            } else {
                None
            }
        }
        Play::Read(r) => Some(r), // Block / Evade / Scheme: hold, read all comers
        Play::Riposte => Some(Read::Evade),
        Play::Firestorm | Play::Rally | Play::Steel => None,
    }
}

fn weakest_front(state: &State) -> Option<usize> {
    state
        .heroes
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_down() && h.line == Line::Front)
        .min_by_key(|(_, h)| h.body.remaining)
        .map(|(i, _)| i)
}

fn least_resolute(state: &State) -> Option<usize> {
    state
        .heroes
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_down())
        .min_by_key(|(_, h)| h.resolve())
        .map(|(i, _)| i)
}

fn lowest_body_any(state: &State) -> Option<usize> {
    state
        .heroes
        .iter()
        .enumerate()
        .filter(|(_, h)| !h.is_down())
        .min_by_key(|(_, h)| h.body.remaining)
        .map(|(i, _)| i)
}

fn strongest_holding_front(state: &State) -> Option<usize> {
    state
        .heroes
        .iter()
        .enumerate()
        .filter(|(i, h)| !h.is_down() && h.line == Line::Front && state.is_holding(*i))
        .max_by_key(|(_, h)| h.power)
        .map(|(i, _)| i)
}

/// Decide the fight if it has ended. `Win(0)` is the party; `Win(1)` is the
/// world (the party fell).
pub fn update_outcome(state: &mut State) {
    let foes = state.living_creatures() > 0;
    let heroes = state.living_heroes() > 0;
    if !foes && heroes {
        state.outcome = Some(Outcome::Win(PlayerId(0)));
        state
            .log
            .push("Every foe is down — the party stands victorious!".into());
    } else if !heroes {
        state.outcome = Some(Outcome::Win(PlayerId(1)));
        state
            .log
            .push("The last hero falls — the run ends.".into());
    }
}
