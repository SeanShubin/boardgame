//! **Headless battle transcripts** — a concise, human-and-machine-readable record of a §4 combat,
//! so a problem can be *shown* rather than described.
//!
//! The tabletop UI is the only way to watch a fight interactively, which makes it the only way to
//! discuss one. A transcript fixes that: it runs a named scenario headlessly under the **resolver of
//! record** (the same greedy policy + deterministic creatures as [`crate::solver::auto_resolve`]) and
//! renders every decision **with the arithmetic that drove it** — the gauntlet's advance-vs-catch
//! Drive comparison, each strike's damage past the cut, and the end-of-round Body table. Two readers
//! (and two transcripts, before/after a change) can then point at the exact line where the rules did
//! something unexpected — which is as often a *rules misunderstanding* as a balance bug.
//!
//! A transcript has three sections:
//! 1. **Combat** — the round-by-round play, with arithmetic.
//! 2. **Cards used** — every card that was played, plus the weapons and powers the chargers brought.
//! 3. **Glossary** — what the rulebook says about each keyword on those cards (from the same single
//!    source of truth as the in-app encyclopedia, so it cannot drift).
//!
//! The format is plain text, one event per line with a fixed leading keyword, so it is greppable and
//! diffs cleanly. Output is written to a git-ignored `transcripts/` directory by the `transcript`
//! example (`cargo run -p deckbound --example transcript`).

use std::collections::{BTreeMap, BTreeSet};

use engine::{Game, Outcome, PlayerId};

use crate::actor::Actor;
use crate::cards::{Card, Effect};
use crate::currency::Currency;
use crate::game::{Deckbound, battle_state_with};
use crate::ruleset::Ruleset;
use crate::scenarios::{build_character, effect_rule, rewards_for, zone_behavior_rule};
use crate::solver::greedy;
use crate::state::{Phase, State};
use crate::zones::ZoneBehavior;

/// Hard cap on decision steps (mirrors the solver) so a degenerate scenario terminates.
const MAX_STEPS: usize = 100_000;

/// A named, hand-built battle to transcribe: two rosters and the tuning [`Ruleset`] they fight under.
pub struct TranscriptScenario {
    /// Filename-safe id (becomes `transcripts/<name>.<seed>.txt`).
    pub name: &'static str,
    /// One-line statement of what the scenario is meant to demonstrate.
    pub blurb: &'static str,
    pub heroes: Vec<Actor>,
    pub foes: Vec<Actor>,
    pub ruleset: Ruleset,
}

/// The catalogue of transcribable scenarios. Starts deliberately small: a single **rules tour** that
/// exercises the core machinery (charge split, Muster, the gauntlet's slip / hold / parting-hit, the
/// Skirmish and Reserve strikes, armour and Fear, defeat, refresh, outcome). The per-skill and
/// power-scaling scenarios are later additions.
pub fn transcript_scenarios() -> Vec<TranscriptScenario> {
    vec![rules_tour()]
}

/// One fully-kitted member of each reward suit (the powers that *decide* the §4 phases) against a
/// small mixed creature line — a wall (Brute), a backline-killer (Raider), and a ranged hexer (Seer)
/// — so a single fight shows every core mechanic at least once.
fn rules_tour() -> TranscriptScenario {
    let named = |name: &str, suit: Currency| {
        let mut a = build_character("Novice", &rewards_for(suit));
        a.name = name.to_string();
        a
    };
    let heroes = vec![
        named("Anvil", Currency::Iron), // Wall: Phalanx → holds the line (catch Drive)
        named("Wisp", Currency::Silver), // Infiltrator: high Drive → slips, becomes a Skirmisher
        named("Sear", Currency::Brass), // Artillery: holds back and fires from the Reserve
        named("Hex", Currency::Bone), // Controller: musters a persistent debuff before the gauntlet
    ];
    let foes = vec![
        build_character("Brute", &[]), // a wall to be held against / slipped past
        build_character("Raider", &[]), // a fast charger (the enemy gauntlet)
        build_character("Seer", &[]),  // a ranged Fear caster (the enemy Reserve)
    ];
    TranscriptScenario {
        name: "rules-tour",
        blurb: "every core mechanic once: charge split, Muster, slip/hold/parting-hit, skirmish, reserve fire, armour, Fear, defeat, refresh.",
        heroes,
        foes,
        ruleset: Ruleset::analysis(),
    }
}

/// Run `scn` headlessly under `seed` and render the transcript text (combat + cards + glossary).
pub fn transcribe(scn: &TranscriptScenario, seed: u64) -> String {
    let game = Deckbound;
    let mut state = battle_state_with(
        scn.heroes.clone(),
        scn.foes.clone(),
        false,
        seed,
        scn.ruleset,
    );

    let mut out = String::new();
    header(&mut out, scn, seed, &state);

    let mut printed = state.log.len(); // skip the seeded "-- Round 1 --" line; we render banners
    let mut cur_round = 0u32;
    // Usage accumulated across the fight, to build the card list afterwards.
    let mut played: BTreeMap<String, BTreeSet<String>> = BTreeMap::new(); // card → actors who played it
    let mut chargers: BTreeSet<String> = BTreeSet::new(); // actors who ran the gauntlet (use weapon + powers)

    for _ in 0..MAX_STEPS {
        if game.outcome(&state).is_some() {
            break;
        }
        // New round → close the previous one with its Body table, then open a banner.
        if state.round != cur_round {
            if cur_round != 0 {
                push_line(&mut out, &hp_table(&state, "ENDROUND"));
            }
            cur_round = state.round;
            push_line(&mut out, "");
            push_line(&mut out, &round_banner(cur_round));
        }

        let was_assemble = state.phase == Phase::Assemble;
        let actions = game.legal_actions(&state);
        let action = greedy(&state, &actions);
        if game.apply(&mut state, &action).is_err() {
            push_line(
                &mut out,
                "!! the greedy policy produced an illegal action — stuck.",
            );
            break;
        }

        // The gauntlet has just resolved (we left Assemble): record who charged.
        if was_assemble && state.phase != Phase::Assemble {
            collect_chargers(&mut chargers, &state);
            push_line(&mut out, &charge_summary(&state));
        }
        // Echo the new prose events (the gauntlet crossings, strikes, card plays), indented, and note
        // any card plays for the card list.
        for line in &state.log[printed..] {
            if line.starts_with("--") {
                continue; // our banners supersede the engine's round markers
            }
            if let Some((actor, card)) = parse_play(line) {
                played.entry(card).or_default().insert(actor);
            }
            push_line(&mut out, &format!("  {line}"));
        }
        printed = state.log.len();
    }

    // Final Body table + the verdict.
    push_line(&mut out, &hp_table(&state, "FINAL   "));
    push_line(&mut out, "");
    push_line(&mut out, &outcome_line(&game, &state));

    // The two trailing reference sections.
    let all: Vec<&Actor> = state.heroes.iter().chain(&state.creatures).collect();
    let used = involved_cards(&all, &played, &chargers);
    push_line(&mut out, "");
    card_list(&mut out, &used);
    push_line(&mut out, "");
    glossary(&mut out, &used);
    out
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

fn round_banner(round: u32) -> String {
    format!("── round {round} ──────────────────────────────────────")
}

/// The header: the variables that make the run reproducible, then both rosters with their key stats.
fn header(out: &mut String, scn: &TranscriptScenario, seed: u64, state: &State) {
    push_line(
        out,
        &format!(
            "SCENARIO  {}   seed={seed}   {}",
            scn.name,
            ruleset_label(scn.ruleset)
        ),
    );
    push_line(out, &format!("          {}", scn.blurb));
    push_line(out, "");
    push_line(out, "HEROES");
    for a in &state.heroes {
        push_line(out, &format!("  {}", stat_line(a)));
    }
    push_line(out, "FOES");
    for a in &state.creatures {
        push_line(out, &format!("  {}", stat_line(a)));
    }
}

fn ruleset_label(r: Ruleset) -> String {
    format!(
        "ruleset(max_rounds={}, max_unique={})",
        r.max_rounds, r.max_unique_per_side
    )
}

/// One actor's stat block: the numbers that decide the §4 phases (Drive for the gauntlet, Power for
/// strikes, Speed for the Tempo budget, Body/Resolve/armour for defence).
fn stat_line(a: &Actor) -> String {
    let armor = if a.defense.armor.is_empty() {
        "—".to_string()
    } else {
        a.defense
            .armor
            .iter()
            .map(|(t, v)| format!("{}{v}", t.label().chars().next().unwrap_or('?')))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{:8} {:11} Body {}/{}  res {}  spd {}  drv {}  pow {}  armor {}  [{}]",
        a.name,
        a.role,
        a.defense.body.remaining,
        a.defense.body.max,
        a.defense.resolve,
        a.offense.speed,
        a.offense.drive,
        a.offense.power,
        armor,
        a.attack.label(),
    )
}

/// Who ran the gauntlet vs held back, per side (read after Deploy resolves it).
fn charge_summary(state: &State) -> String {
    let split = |pool: &[Actor], charging: &[bool]| {
        let pick = |want: bool| {
            pool.iter()
                .zip(charging)
                .filter(|(a, c)| **c == want && !a.fallen)
                .map(|(a, _)| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let charged = pick(true);
        let reserve = pick(false);
        format!(
            "charged: {}   reserve: {}",
            if charged.is_empty() { "—" } else { &charged },
            if reserve.is_empty() { "—" } else { &reserve },
        )
    };
    format!(
        "CHARGE   heroes — {}\n         foes   — {}",
        split(&state.heroes, &state.plan.hero_charging),
        split(&state.creatures, &state.plan.foe_charging),
    )
}

/// The race scoreboard: every actor's remaining Body, with a `down` mark for the fallen.
fn hp_table(state: &State, label: &str) -> String {
    let row = |pool: &[Actor]| {
        pool.iter()
            .map(|a| {
                let mark = if a.fallen || a.is_down() { " down" } else { "" };
                format!(
                    "{} {}/{}{mark}",
                    a.name, a.defense.body.remaining, a.defense.body.max
                )
            })
            .collect::<Vec<_>>()
            .join("  ")
    };
    format!(
        "{label} heroes: {}  |  foes: {}",
        row(&state.heroes),
        row(&state.creatures)
    )
}

fn outcome_line(game: &Deckbound, state: &State) -> String {
    let rounds = state.round;
    match game.outcome(state) {
        Some(Outcome::Win(PlayerId(0))) => format!("OUTCOME  WIN (heroes) — round {rounds}"),
        Some(Outcome::Win(_)) => format!("OUTCOME  LOSS (the party fell) — round {rounds}"),
        Some(Outcome::Tie(_)) => format!("OUTCOME  DRAW (round cap reached) — round {rounds}"),
        None => format!("OUTCOME  UNRESOLVED after {rounds} round(s)"),
    }
}

// --- card list + glossary -----------------------------------------------------------------------

/// A card that took part in the fight, with how it got there.
struct CardUse {
    card: Card,
    /// `"Action"` (played), `"Weapon"` (a charger's), or `"Power"` (a charger's passive).
    kind: &'static str,
    /// Who brought it (`"played by Hex"` / `"wielded by Brute"` / `"carried by Anvil"`).
    note: String,
}

/// Parse a `"{actor} plays {card}."` log line (the only un-indented `plays` line). Returns the
/// `(actor, card)` names. Sub-effect lines are indented and so are ignored.
fn parse_play(line: &str) -> Option<(String, String)> {
    if line.starts_with(' ') || !line.ends_with('.') {
        return None;
    }
    let idx = line.find(" plays ")?;
    let actor = line[..idx].to_string();
    let card = line[idx + " plays ".len()..line.len() - 1].to_string();
    Some((actor, card))
}

/// Record the names of every actor that charged this round (the gauntlet consults their weapon and
/// passive powers, so those count as "used").
fn collect_chargers(chargers: &mut BTreeSet<String>, state: &State) {
    for (a, c) in state.heroes.iter().zip(&state.plan.hero_charging) {
        if *c {
            chargers.insert(a.name.clone());
        }
    }
    for (a, c) in state.creatures.iter().zip(&state.plan.foe_charging) {
        if *c {
            chargers.insert(a.name.clone());
        }
    }
}

/// Owners (by name) among `chargers` whose weapon (or passive power) is `card_name`, joined.
fn owners(
    actors: &[&Actor],
    chargers: &BTreeSet<String>,
    card_name: &str,
    is_weapon: bool,
) -> String {
    actors
        .iter()
        .filter(|a| chargers.contains(&a.name))
        .filter(|a| {
            if is_weapon {
                a.weapon.name == card_name
            } else {
                a.actions.iter().any(|c| c.name == card_name && c.passive)
            }
        })
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Assemble the de-duplicated list of cards that took part: every **played** card, plus each
/// charger's **weapon** and **passive powers** (the gauntlet resolves through those). Sorted within
/// each kind by name (the [`BTreeMap`] key).
fn involved_cards(
    actors: &[&Actor],
    played: &BTreeMap<String, BTreeSet<String>>,
    chargers: &BTreeSet<String>,
) -> Vec<CardUse> {
    let weapon_names: BTreeSet<&str> = actors.iter().map(|a| a.weapon.name.as_str()).collect();
    let mut out: BTreeMap<String, CardUse> = BTreeMap::new();

    // Played cards (active), found by name on whoever owns them.
    for (name, who) in played {
        let card = actors
            .iter()
            .flat_map(|a| std::iter::once(&a.weapon).chain(&a.actions))
            .find(|c| &c.name == name);
        if let Some(card) = card {
            let kind = if weapon_names.contains(name.as_str()) {
                "Weapon"
            } else {
                "Action"
            };
            out.entry(name.clone()).or_insert_with(|| CardUse {
                card: card.clone(),
                kind,
                note: format!("played by {}", join(who)),
            });
        }
    }

    // Each charger's weapon and passive powers (the gauntlet resolves through them).
    for a in actors.iter().filter(|a| chargers.contains(&a.name)) {
        let w = &a.weapon;
        out.entry(w.name.clone()).or_insert_with(|| CardUse {
            card: w.clone(),
            kind: "Weapon",
            note: format!("wielded by {}", owners(actors, chargers, &w.name, true)),
        });
        for c in a.actions.iter().filter(|c| c.passive) {
            out.entry(c.name.clone()).or_insert_with(|| CardUse {
                card: c.clone(),
                kind: "Power",
                note: format!("carried by {}", owners(actors, chargers, &c.name, false)),
            });
        }
    }
    out.into_values().collect()
}

fn join(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
}

/// Render the **Cards used** section, grouped Action → Weapon → Power.
fn card_list(out: &mut String, used: &[CardUse]) {
    push_line(
        out,
        "CARDS USED   (every card played, plus the chargers' weapons and powers)",
    );
    for kind in ["Action", "Weapon", "Power"] {
        for u in used.iter().filter(|u| u.kind == kind) {
            let suit = u
                .card
                .role
                .map(|r| format!("[{}]", r.label()))
                .unwrap_or_else(|| "     ".to_string());
            // The card list is a scannable index; the glossary carries the full keyword rules. So show
            // a short tag here: an Action's effect summary, a weapon's damage type, "passive" for a power.
            let summary = match kind {
                "Power" => "passive power".to_string(),
                "Weapon" => match u.card.primary_damage() {
                    Some((_, dt)) => format!("{} weapon", dt.label()),
                    None => "weapon".to_string(),
                },
                _ => u.card.summary(),
            };
            push_line(
                out,
                &format!(
                    "  {:7} {:14} {:8} {:30}  ({})",
                    kind, u.card.name, suit, summary, u.note
                ),
            );
        }
    }
}

/// A short keyword label for one effect (the term defined in the glossary).
fn effect_keyword(e: &Effect) -> String {
    use Effect::*;
    match e {
        Damage { dtype, .. } => format!("Damage ({})", dtype.label()),
        Guard { .. } => "Guard (Brace)".into(),
        Lifeline => "Lifeline (Last Stand)".into(),
        Stagger => "Stagger".into(),
        Sunder { .. } => "Sunder".into(),
        Disarm => "Disarm".into(),
        Shove => "Shove".into(),
        Rally { .. } => "Rally".into(),
        Steel => "Steel".into(),
        Recover => "Recover".into(),
        BankSpeed { .. } => "Bank Speed".into(),
        Mend { .. } => "Mend".into(),
        Ward => "Ward".into(),
        Haste { .. } => "Haste".into(),
        Empower { .. } => "Empower".into(),
        Suppress { .. } => "Suppress".into(),
        Slow { .. } => "Slow".into(),
        Confuse { .. } => "Confuse".into(),
    }
}

fn zone_keyword(z: ZoneBehavior) -> &'static str {
    match z {
        ZoneBehavior::Return => "Zone: Return",
        ZoneBehavior::Spend => "Zone: Spend",
        ZoneBehavior::Lasting => "Zone: Lasting",
    }
}

/// Render the **Glossary** section: every distinct keyword on the used cards, with the rulebook
/// definition (from `effect_rule` / `zone_behavior_rule` / the passive power's text — the same
/// sources the in-app encyclopedia is generated from, so this can't drift).
fn glossary(out: &mut String, used: &[CardUse]) {
    let mut terms: BTreeMap<String, String> = BTreeMap::new();
    for u in used {
        // A passive power: its name is the keyword; its `text` is the rule.
        if u.card.passive && !u.card.text.is_empty() {
            terms
                .entry(u.card.name.clone())
                .or_insert_with(|| u.card.text.clone());
        }
        for e in &u.card.effects {
            terms
                .entry(effect_keyword(e))
                .or_insert_with(|| effect_rule(e));
        }
        // Zone behaviour is a keyword too — define the non-default ones the played cards carry.
        if u.kind != "Power" && u.card.zone != ZoneBehavior::Return {
            let z = u.card.zone;
            terms
                .entry(zone_keyword(z).into())
                .or_insert_with(|| zone_behavior_rule(z).into());
        }
    }
    push_line(
        out,
        "GLOSSARY   (what the rulebook says about each keyword above)",
    );
    for (term, rule) in &terms {
        push_line(out, &format!("  {term:22} {rule}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_tour_transcript_is_deterministic_and_has_all_three_sections() {
        let scn = rules_tour();
        let a = transcribe(&scn, 1);
        let b = transcribe(&scn, 1);
        assert_eq!(a, b, "a transcript must be a pure function of the seed");

        // The rules tour must exercise the machinery it claims to, and carry its reference sections.
        for marker in [
            "SCENARIO",
            "CHARGE",
            "crossing:", // the gauntlet's advance-vs-catch arithmetic
            "ENDROUND",  // at least two rounds (refresh happened)
            "OUTCOME",
            "CARDS USED",
            "GLOSSARY",
        ] {
            assert!(
                a.contains(marker),
                "rules-tour transcript is missing `{marker}`:\n{a}"
            );
        }
    }
}
