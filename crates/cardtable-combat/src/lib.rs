//! **cardtable-combat** — the bridge that resolves an encounter on the card table using `deckbound`'s
//! battle-tested combat, then folds the result back onto the [`Tableau`].
//!
//! This is the one place the product reaches into the reference game: the pure [`cardtable_model`] never
//! learns about `deckbound`, and the Bevy renderer only records *that* a fight was asked for. Here we read
//! the location's heroes and foes off their cards, translate each into a combat unit (a hero's stats come
//! from its deck via [`Tableau::character_recipe`]; a foe's from [`catalog::creature`]), decide the fight
//! with the headless resolver, and apply the consequences. No Bevy — a plain function over the model, so it
//! unit-tests in isolation.
//!
//! The combatants carry only a 5-stat line and a strike shape (reach / area / hoard / stance); the ability
//! *name* is flavour, ignored by the resolver — mechanics are all numbers, matching the duel-locks proof.

use cardtable_model::{CardId, CardKind, Face, PileId, Tableau, catalog};
use deckbound::Actor;
use deckbound::balance::{DuelUnit, Stat5, build_duel_unit};

/// The result of a resolved encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatOutcome {
    Win,
    Loss,
}

/// Resolve the encounter at `place`: gather every hero stationed there and the encounter's foes, **play out**
/// the fight with `deckbound`'s seeded, deterministic resolver ([`resolve_logged`](deckbound::resolve_logged),
/// which also returns the turn-by-turn log), and apply the consequences to `table`.
///
/// Combat is a resolved event that leaves a single **virtual combat-log card** at the location (replacing any
/// prior one — one log per location). Foes are **virtual**, read from the catalog rather than dealt as cards:
/// on a **win** the encounter header is cleared (nothing returns to the Bestiary); on a **loss** it stays, so
/// the fight can be retried. Either way the heroes remain where they are and a day passes. A place with no
/// heroes or no encounter is a no-op reporting [`CombatOutcome::Loss`].
pub fn resolve_encounter(table: &mut Tableau, place: PileId, seed: u64) -> CombatOutcome {
    let heroes = hero_units(table, place);
    let foes = foe_units(table, place);
    if heroes.is_empty() || foes.is_empty() {
        return CombatOutcome::Loss;
    }
    let hero_actors: Vec<Actor> = heroes.iter().map(build_duel_unit).collect();
    let foe_actors: Vec<Actor> = foes.iter().map(build_duel_unit).collect();
    let (won, log) = deckbound::resolve_logged(hero_actors, foe_actors, seed);
    let outcome = if won == Some(true) {
        CombatOutcome::Win
    } else {
        CombatOutcome::Loss
    };
    apply_consequences(table, place, outcome, log);
    outcome
}

/// `[u8; 5]` → the resolver's `(Might, Vitality, Toughness, Cadence, Finesse)` tuple.
fn stat5(s: [u8; 5]) -> Stat5 {
    (
        s[0] as u32,
        s[1] as u32,
        s[2] as u32,
        s[3] as u32,
        s[4] as u32,
    )
}

/// The heroes stationed at `place` as combat units — each `hero` position copy mapped to its character
/// deck (by hero name), whose [`Recipe`](cardtable_model::Recipe) gives the stats + ability. The ability
/// sets the strike shape via [`catalog::ability_shape`]; a kit leaves its stance stat-derived (`pos: None`).
fn hero_units(table: &Tableau, place: PileId) -> Vec<DuelUnit> {
    table
        .content_cards(place)
        .into_iter()
        .filter_map(|c| {
            let card = table.card(c)?;
            if card.card_type() != "hero" {
                return None;
            }
            let name = card.front_title().to_string();
            let recipe = table.character_recipe(character_deck(table, &name)?)?;
            let (ranged, aoe) = catalog::ability_shape(&recipe.ability);
            Some(DuelUnit {
                name,
                ability: recipe.ability,
                stats: stat5(recipe.stats),
                ranged,
                aoe,
                count: 1,
                hoard: false,
                pos: None,
            })
        })
        .collect()
}

/// The encounter's foes at `place` as combat units. Foes are **virtual** — the location holds only an
/// encounter card, so the roster comes from the catalog: the place's label names the encounter, and
/// [`catalog::encounter_foes`] gives its `(creature, quantity)` list, expanded so a `×2` keystone fields two.
fn foe_units(table: &Tableau, place: PileId) -> Vec<DuelUnit> {
    let Some(name) = table.pile(place).map(|p| p.label.clone()) else {
        return Vec::new();
    };
    let Some(encounter) = catalog::encounter_for(&name) else {
        return Vec::new();
    };
    let mut units = Vec::new();
    for (creature, qty) in catalog::encounter_foes(encounter) {
        for _ in 0..qty {
            units.push(DuelUnit {
                name: creature.name.to_string(),
                ability: creature.ability.to_string(),
                stats: stat5(creature.stats),
                ranged: creature.ranged,
                aoe: creature.aoe,
                count: 1,
                hoard: creature.hoard,
                pos: creature.pos.map(str::to_string),
            });
        }
    }
    units
}

/// Fold the result onto the table. A location holds at most **one** combat-log card, so any prior one is
/// removed first. Foes are **virtual** (the Bestiary supply never left home), so a **win** just clears the
/// `encounter` header — nothing to send back; on a **loss** it stays, so the fight can be retried. Then the
/// new virtual log card is left at the place and [`advance_day`](Tableau::advance_day) runs once — the fight
/// costs a day. The heroes are not moved either way.
fn apply_consequences(
    table: &mut Tableau,
    place: PileId,
    outcome: CombatOutcome,
    log: Vec<String>,
) {
    // One log per location: clear the previous combat's card before recording this one.
    for c in cards_of_types(table, place, &["log"]) {
        let _ = table.remove_card(c);
    }
    if outcome == CombatOutcome::Win {
        // The encounter is defeated: remove its header. The foes were virtual, so there is nothing to
        // return to the Bestiary — its stacked supply was only ever *read*, never moved.
        for c in cards_of_types(table, place, &["encounter"]) {
            let _ = table.remove_card(c);
        }
    }
    add_log_card(table, place, outcome, log);
    if let (Some(progress), Some(events)) =
        (find_pile(table, "Progress"), find_pile(table, "Events"))
    {
        let _ = table.advance_day(progress, events);
    }
}

/// Leave a **virtual** combat-log card at `place`: a non-physical [`CardKind::Virtual`] card typed `log`,
/// titled by the outcome, with a one-line summary (Medium) over the full turn-by-turn narration (its Large
/// panel). It renders and expands like any card but is excluded from the physical tally.
fn add_log_card(table: &mut Tableau, place: PileId, outcome: CombatOutcome, log: Vec<String>) {
    let (title, summary) = match outcome {
        CombatOutcome::Win => ("Victory", "The party cleared the encounter."),
        CombatOutcome::Loss => ("Defeat", "The party was beaten back."),
    };
    let Ok(id) = table.add_card(
        place,
        Face::Up {
            title: title.into(),
        },
        None,
    ) else {
        return;
    };
    let _ = table.set_card_type(id, "log");
    let _ = table.set_card_kind(id, CardKind::Virtual);
    let _ = table.set_card_detail(id, vec![summary.to_string()]);
    let _ = table.set_card_panel(id, log);
}

/// The content cards of `place` whose `card_type` is one of `types`.
fn cards_of_types(table: &Tableau, place: PileId, types: &[&str]) -> Vec<CardId> {
    table
        .content_cards(place)
        .into_iter()
        .filter(|&c| {
            table
                .card(c)
                .is_some_and(|k| types.contains(&k.card_type()))
        })
        .collect()
}

/// The character deck for hero `name` — the root sub-pile that [`reflects`](cardtable_model::Pile::reflects)
/// an identity card with that title.
fn character_deck(table: &Tableau, name: &str) -> Option<PileId> {
    subpiles(table, table.root_id()).into_iter().find(|&p| {
        table
            .pile(p)
            .and_then(|pile| pile.reflects())
            .and_then(|id| table.card(id))
            .is_some_and(|c| c.front_title() == name)
    })
}

/// A top-level deck by label.
fn find_pile(table: &Tableau, label: &str) -> Option<PileId> {
    subpiles(table, table.root_id())
        .into_iter()
        .find(|&p| table.pile(p).is_some_and(|pile| pile.label == label))
}

/// The sub-pile ids of `pile` (empty if unknown).
fn subpiles(table: &Tableau, pile: PileId) -> Vec<PileId> {
    table.pile(pile).map(|p| p.subpiles()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardtable_model::{Recipe, sample_table};

    /// The four inn-adjacent solo encounters and the kit that answers each (the duel-locks diagonal).
    fn marksman() -> Recipe {
        Recipe {
            stats: [4, 4, 1, 2, 2],
            ability: "Stand-Off".into(),
        }
    }
    fn executioner() -> Recipe {
        Recipe {
            stats: [6, 3, 1, 1, 1],
            ability: "Alpha Strike".into(),
        }
    }

    fn deck(t: &Tableau, label: &str) -> PileId {
        find_pile(t, label).expect("a top-level deck")
    }

    /// The inn — Ashfen Crossing, the centre place inside the Locations grid.
    fn ashfen(t: &Tableau) -> PileId {
        subpiles(t, deck(t, "Locations"))
            .into_iter()
            .find(|&p| t.pile(p).unwrap().label == "Ashfen Crossing")
            .expect("the inn")
    }

    /// Recruit Heroes deck hero #0 with `recipe` (dealing its four copies, including a position copy at
    /// the inn and a Progress move marker), then march that position copy to the place `dest`. Returns the
    /// destination place id.
    fn station_at(t: &mut Tableau, recipe: Recipe, dest: &str) -> PileId {
        let (heroes, stats, numbers, abilities, progress) = (
            deck(t, "Heroes"),
            deck(t, "Stats"),
            deck(t, "Numbers"),
            deck(t, "Abilities"),
            deck(t, "Progress"),
        );
        let ashfen = ashfen(t);
        let name = t
            .card(t.content_cards(heroes)[0])
            .unwrap()
            .name()
            .to_string();
        t.equip_character(
            &name, &recipe, heroes, stats, numbers, abilities, ashfen, progress,
        )
        .unwrap();
        // The hero's position copy now rests at the inn; march it to `dest`.
        let position = t
            .content_cards(ashfen)
            .into_iter()
            .find(|&c| {
                let k = t.card(c).unwrap();
                k.card_type() == "hero" && k.front_title() == name
            })
            .expect("the stationed hero's position copy");
        let place = t
            .pile(deck(t, "Locations"))
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| t.pile(p).unwrap().label == dest)
            .unwrap();
        t.move_character(position, place, progress).unwrap();
        place
    }

    /// Whether an unresolved encounter still sits at `place`. Foes are virtual (no physical foe cards), so
    /// the encounter header is the marker of a pending fight — present at setup, removed on a win.
    fn has_encounter(t: &Tableau, place: PileId) -> bool {
        !cards_of_types(t, place, &["encounter"]).is_empty()
    }

    /// The one combat-log card at `place` (its title), if any.
    fn log_title(t: &Tableau, place: PileId) -> Option<String> {
        cards_of_types(t, place, &["log"])
            .first()
            .map(|&c| t.card(c).unwrap().name().to_string())
    }

    /// The right kit wins: a Marksman clears Cinderwatch Keep's Coil — the encounter is cleared (foes are
    /// virtual, so nothing returns to the Bestiary), a non-physical "Victory" log card is left behind (with
    /// a real narration panel), and a day passes.
    #[test]
    fn answering_kit_wins_clears_foes_and_leaves_a_log() {
        let mut t = sample_table();
        let day_before = t.current_day(deck(&t, "Progress"));
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        assert!(has_encounter(&t, place), "the Coil's encounter waits here");

        let outcome = resolve_encounter(&mut t, place, 1);
        assert_eq!(outcome, CombatOutcome::Win);
        assert!(!has_encounter(&t, place), "the cleared encounter is gone");
        assert_eq!(log_title(&t, place).as_deref(), Some("Victory"));
        // The place now holds only its "Location" label + the hero token; the log card is virtual, so it
        // does not raise the physical tally (the encounter header is gone; foes were never physical).
        assert_eq!(t.physical_card_count(place), 2);
        assert_eq!(
            t.current_day(deck(&t, "Progress")),
            day_before + 1,
            "the fight cost a day"
        );
    }

    /// The wrong kit loses: an Executioner cannot crack the Coil. The foes REMAIN (retriable), the hero
    /// stays at the location (no retreat), a "Defeat" log card is left, and a day passes.
    #[test]
    fn wrong_kit_loses_foes_remain_hero_stays() {
        let mut t = sample_table();
        let place = station_at(&mut t, executioner(), "Cinderwatch Keep");

        let outcome = resolve_encounter(&mut t, place, 1);
        assert_eq!(outcome, CombatOutcome::Loss);
        assert!(has_encounter(&t, place), "the foes still hold the place");
        assert_eq!(log_title(&t, place).as_deref(), Some("Defeat"));
        let stayed = cards_of_types(&t, place, &["hero"])
            .iter()
            .any(|&c| t.card(c).unwrap().front_title() == "Vael Thornbrand");
        assert!(stayed, "the hero stays put on a loss");
        let at_inn = cards_of_types(&t, ashfen(&t), &["hero"])
            .iter()
            .any(|&c| t.card(c).unwrap().front_title() == "Vael Thornbrand");
        assert!(!at_inn, "the hero did not retreat to the inn");
    }

    /// One combat-log card per location: a second fight replaces the first's log rather than stacking.
    #[test]
    fn a_new_fight_replaces_the_previous_log() {
        let mut t = sample_table();
        let place = station_at(&mut t, executioner(), "Cinderwatch Keep");
        resolve_encounter(&mut t, place, 1); // loss — foes remain, "Defeat" logged
        resolve_encounter(&mut t, place, 1); // fight again
        assert_eq!(
            cards_of_types(&t, place, &["log"]).len(),
            1,
            "only the latest combat's log remains"
        );
    }
}
