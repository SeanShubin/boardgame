//! **Headless battle transcripts** — a concise, human-and-machine-readable record of a §4 combat,
//! so a problem can be *shown* rather than described.
//!
//! The tabletop UI is the only way to watch a fight interactively, which makes it the only way to
//! discuss one. A transcript fixes that: it runs a named scenario headlessly under the **resolver of
//! record** (the same greedy policy + deterministic creatures as [`crate::solver::auto_resolve`]) and
//! renders every decision **with the arithmetic that drove it** — the gauntlet's advance-vs-catch
//! Daring comparison, each strike's damage past the cut, and the end-of-round Body table. Two readers
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
use crate::scenarios::{
    build_character, build_creature, card_level, effect_rule, rewards_for, zone_behavior_rule,
};
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
/// exercises the core machinery (rank allocation, Muster, the gauntlet's slip / hold / parting-hit, the
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
        named("Anvil", Currency::Iron), // Wall: Phalanx → holds the line (the hold)
        named("Wisp", Currency::Silver), // Infiltrator: high Daring → slips, becomes a Skirmisher
        named("Sear", Currency::Brass), // Artillery: holds back and fires from the Reserve
        named("Hex", Currency::Bone), // Controller: musters a persistent debuff before the gauntlet
    ];
    // The tour fields four MAX-LEVEL specialists, so the foe line must be a real threat to run past
    // round 1 (so refresh and the later-round mechanics show). Each creature is reinforced in Body /
    // Toughness so it survives a round of the Infiltrator's multi-strike. Their **Resolve is varied on
    // purpose** so Hex's one Unmake (fear ≈ 12) shows the whole control ladder at once — the higher the
    // Resolve, the milder the break: Freeze (lose the action) → Shaken (also can't defend) → Rout
    // (driven off the line to the Reserve). Numbers are seeded for a multi-round fight; tune to taste.
    let threat = |name: &str, resolve: u32| {
        let mut a = build_creature(name);
        a.defense.resolve = resolve;
        a.defense.body.max += 8;
        a.defense.body.remaining += 8;
        a.defense.body.toughness += 2;
        a
    };
    // Four foes span the whole Resolve dial, so one Unmake shows fear's counter-dial end to end: the
    // bravest **shrugs it off** (and stays to fight, keeping the crossing contest), and each lower
    // Resolve breaks one tier harder.
    let foes = vec![
        threat("Brute", 8),  // R 8: fear **Freezes** the armored wall (loses its action)
        threat("Husk", 14), // R 14: fear **resisted** — it fights on and catches the slip (the crossing)
        threat("Seer", 5),  // R 5: fear **Shakes** the ranged caster (also cannot defend)
        threat("Raider", 3), // R 3: fear **Routs** the fast charger — driven off the line (b2 demotion)
    ];
    TranscriptScenario {
        name: "rules-tour",
        blurb: "every core mechanic once: rank allocation (Vanguard/Skirmisher/Reserve), Muster, slip/hold/parting-hit, skirmish, reserve fire, armour, fear's control ladder (resist / Freeze / Shaken / Rout + the Rout demotion), defeat, refresh.",
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
            push_line(&mut out, &ranks_summary(&state));
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
    push_line(
        out,
        "FORM  each stat = base-2 quantifier cards summing to its value · Q Quantity (how many) · P Power (each) · §2.4–§2.6",
    );
    push_line(out, "");
    push_line(out, "HEROES");
    for a in &state.heroes {
        out.push_str(&form_block(a));
    }
    push_line(out, "FOES");
    for a in &state.creatures {
        out.push_str(&form_block(a));
    }
}

fn ruleset_label(r: Ruleset) -> String {
    format!(
        "ruleset(max_rounds={}, max_unique={})",
        r.max_rounds, r.max_unique_per_side
    )
}

/// Decompose `v` into its base-2 **quantifier cards** (the set bits), high to low: `18 → [16, 2]`.
fn base2_cards(v: u32) -> Vec<u32> {
    (0..32)
        .rev()
        .map(|i| 1u32 << i)
        .filter(|bit| v & bit != 0)
        .collect()
}

/// One suit cell: the quantifier cards summing to `v`, e.g. `16+2 = 18`; `—` when zero.
fn suit_cell(v: u32) -> String {
    let cards = base2_cards(v);
    if cards.is_empty() {
        "—".to_string()
    } else {
        let cards = cards
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("+");
        format!("{cards} = {v}")
    }
}

/// §2.4–§2.6 — an actor's build as the **cards on the table**: every stat is a deck of base-2
/// quantifier cards, **Quantity** (how many) × **Power** (each), summing to its value. A **pool**
/// (Body, Tempo) carries both suits; a flat stat carries Power only. This renders what the §2.3 build
/// card instantiates, so the tabletop representation can be audited directly. (Body shows the full
/// pool; depletion during play is the maintained meter, §2.1.)
fn form_block(a: &Actor) -> String {
    // §2.5 — the build's physical card-cost: how many base-2 quantifier cards it takes (the popcount
    // of every stat, summed). Each set bit is one card on the table.
    let pc = |v: u32| v.count_ones();
    let cards: u32 = pc(a.defense.body.max)
        + pc(a.defense.body.toughness)
        + pc(a.offense.speed)
        + pc(a.offense.daring)
        + pc(a.offense.power)
        + pc(a.offense.dread)
        + pc(a.offense.inspiration)
        + pc(a.offense.precision)
        + pc(a.defense.resolve)
        + a.defense.armor.values().map(|v| pc(*v)).sum::<u32>()
        + a.defense.ward.values().map(|v| pc(*v)).sum::<u32>();
    let mut out = format!(
        "  {} — {} · {} [{}]  ({cards} cards)\n",
        a.name,
        a.role,
        a.weapon.name,
        a.attack.label()
    );
    // The actor's **deck** — the cards it can bring to a round, so "X plays Y" is verifiable against
    // the roster (the `cards` count above is the *Form* / stat cards; these are the ones that *do*
    // something). Split into played **Actions** and always-on **Powers** (passives); the weapon is in
    // the header. Foes list theirs too, so the enemy kit is legible.
    let deck = |passive: bool| -> Vec<&str> {
        a.actions
            .iter()
            .filter(|c| c.passive == passive)
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
    };
    let actions = deck(false);
    if !actions.is_empty() {
        out.push_str(&format!(
            "      {:<11} {}\n",
            "Actions",
            actions.join(" · ")
        ));
    }
    let powers = deck(true);
    if !powers.is_empty() {
        out.push_str(&format!("      {:<11} {}\n", "Powers", powers.join(" · ")));
    }
    let pool = |label: &str, q: u32, p: u32| {
        format!(
            "      {label:<11} Q {:<13} P {}\n",
            suit_cell(q),
            suit_cell(p)
        )
    };
    let flat = |label: &str, p: u32| format!("      {label:<11} P {}\n", suit_cell(p));

    out.push_str(&pool("Body", a.defense.body.max, a.defense.body.toughness));
    out.push_str(&pool("Tempo", a.offense.speed, a.offense.daring));
    if a.offense.power > 0 {
        out.push_str(&flat("Strike", a.offense.power));
    }
    if a.offense.dread > 0 {
        out.push_str(&flat("Dread", a.offense.dread));
    }
    if a.offense.inspiration > 0 {
        out.push_str(&flat("Inspiration", a.offense.inspiration));
    }
    if a.offense.precision > 0 {
        out.push_str(&flat("Pierce", a.offense.precision));
    }
    if a.defense.resolve > 0 {
        out.push_str(&flat("Resolve", a.defense.resolve));
    }
    for (t, v) in &a.defense.armor {
        if *v > 0 {
            out.push_str(&flat(&format!("Guard·{}", t.label()), *v));
        }
    }
    for (t, v) in &a.defense.ward {
        if *v > 0 {
            out.push_str(&flat(&format!("Ward·{}", t.label()), *v));
        }
    }
    out
}

/// Who ran the gauntlet vs held back, per side (read after Deploy resolves it).
fn ranks_summary(state: &State) -> String {
    // The §4 Assemble declares **three** ranks (Spec §4): a charger that holds is a **Vanguard**, a
    // charger that flanks is a **Skirmisher**, a non-charger is a **Reserve**. (A unit Routed at Muster
    // is driven to the Reserve — its charge flag is cleared, b2.) Show all three, not a charged/held
    // binary, so the line matches the rank the rules assign.
    let split = |pool: &[Actor], charging: &[bool], flank: &[bool]| {
        let pick = |f: &dyn Fn(usize) -> bool| {
            (0..pool.len())
                .filter(|&i| f(i))
                .map(|i| pool[i].name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        // A charger was *declared* this round (the plan resets each round and only living units are
        // assigned), so show it in its rank even if it then fell in the gauntlet — that keeps the line
        // consistent with the crossing/clash log below it. Only the Reserve filters the dead, to drop
        // prior-round casualties (which carry the reset all-false flags).
        let vanguard = pick(&|i| charging[i] && !flank[i]);
        let skirmisher = pick(&|i| charging[i] && flank[i]);
        let reserve = pick(&|i| !charging[i] && !pool[i].fallen);
        let cell = |s: String| if s.is_empty() { "—".to_string() } else { s };
        format!(
            "vanguard: {}   skirmisher: {}   reserve: {}",
            cell(vanguard),
            cell(skirmisher),
            cell(reserve),
        )
    };
    format!(
        "RANKS    heroes — {}\n         foes   — {}",
        split(
            &state.heroes,
            &state.plan.hero_charging,
            &state.plan.hero_flank
        ),
        split(
            &state.creatures,
            &state.plan.foe_charging,
            &state.plan.foe_flank
        ),
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
            // Suit + reward level — the card's `(track, level)` coordinate. Weapons and pool cards
            // carry no role/level, so the tag is blank for them.
            let suit = match (u.card.role, card_level(&u.card.name)) {
                (Some(r), Some(l)) => format!("[{} L{l}]", r.label()),
                (Some(r), None) => format!("[{}]", r.label()),
                _ => String::new(),
            };
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
                    "  {:7} {:14} {:11} {:30}  ({})",
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
        Fortify { .. } => "Fortify (Shield Wall)".into(),
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
            "RANKS", // the §4 Assemble rank allocation (Vanguard / Skirmisher / Reserve)
            "skirmisher:", // the rank line names all three ranks, not a charged/held binary
            "crossing:", // a Skirmisher's card-bound crossing contest (§4 the Line)
            "ENDROUND", // at least two rounds — refresh happened
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

    /// Golden snapshot (regression guard): the rules-tour transcript is a stable, committed artifact.
    /// Any change to mechanics, rendering, or the cast surfaces here as a diff to **review and ratify**
    /// — e.g. this catches a foe's Body silently shifting. To update after an *intended* change:
    /// regenerate (`cargo run -p deckbound --example transcript`) and copy `transcripts/rules-tour.1.txt`
    /// over `crates/deckbound/src/snapshots/rules-tour.1.txt`.
    #[test]
    fn rules_tour_transcript_matches_golden() {
        let got = transcribe(&rules_tour(), 1);
        let want = include_str!("snapshots/rules-tour.1.txt");
        // Normalise line endings so a CRLF checkout (Windows) doesn't spuriously fail.
        let norm = |s: &str| s.replace("\r\n", "\n");
        assert_eq!(
            norm(&got),
            norm(want),
            "rules-tour transcript drifted from its golden snapshot. If intended, regenerate and update \
             crates/deckbound/src/snapshots/rules-tour.1.txt (see this test's doc comment)."
        );
    }
}
