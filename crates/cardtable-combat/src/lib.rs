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

use cardtable_model::{CardId, PileId, Tableau, catalog};
use deckbound::Actor;
use deckbound::balance::{DuelUnit, Stat5, build_duel_unit};

/// The result of a resolved encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatOutcome {
    Win,
    Loss,
}

/// Resolve the encounter at `place`: gather every hero stationed there and the encounter's foes, decide the
/// fight with `deckbound`'s seeded, deterministic [`winnable`](deckbound::winnable) resolver, and apply the
/// consequences to `table`. **Win** clears the foes (and the encounter header) back to the Bestiary;
/// **loss** retreats the heroes to the inn (Ashfen Crossing); either way a day passes. A place with no
/// heroes or no foes is a no-op that reports [`CombatOutcome::Loss`].
pub fn resolve_encounter(table: &mut Tableau, place: PileId, seed: u64) -> CombatOutcome {
    let heroes = hero_units(table, place);
    let foes = foe_units(table, place);
    if heroes.is_empty() || foes.is_empty() {
        return CombatOutcome::Loss;
    }
    let hero_actors: Vec<Actor> = heroes.iter().map(build_duel_unit).collect();
    let foe_actors: Vec<Actor> = foes.iter().map(build_duel_unit).collect();
    let outcome = if deckbound::winnable(hero_actors, foe_actors, seed) {
        CombatOutcome::Win
    } else {
        CombatOutcome::Loss
    };
    apply_consequences(table, place, outcome);
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

/// The heroes stationed at `place` as combat units — each `location-token` mapped to its character deck
/// (by hero name), whose [`Recipe`](cardtable_model::Recipe) gives the stats + ability. The ability sets
/// the strike shape via [`catalog::ability_shape`]; a kit leaves its stance stat-derived (`pos: None`).
fn hero_units(table: &Tableau, place: PileId) -> Vec<DuelUnit> {
    table
        .content_cards(place)
        .into_iter()
        .filter_map(|c| {
            let card = table.card(c)?;
            if card.card_type() != "location-token" {
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

/// The encounter's foes at `place` as combat units — each `foe` card resolved to its [`catalog::Creature`]
/// (stats + reach / area / hoard / stance), expanded by the card's quantity so a `×2` keystone fields two.
fn foe_units(table: &Tableau, place: PileId) -> Vec<DuelUnit> {
    let mut units = Vec::new();
    for c in table.content_cards(place) {
        let Some(card) = table.card(c) else { continue };
        if card.card_type() != "foe" {
            continue;
        }
        let Some(creature) = catalog::creature(card.name()) else {
            continue;
        };
        for _ in 0..card.quantity().max(1) {
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

/// Fold the result onto the table: **win** sends every `foe` and the `encounter` header back to the
/// Bestiary (conservation-clean; the encounter is cleared); **loss** retreats every hero token to the inn.
/// Either way [`advance_day`](Tableau::advance_day) once — the fight costs a day.
fn apply_consequences(table: &mut Tableau, place: PileId, outcome: CombatOutcome) {
    match outcome {
        CombatOutcome::Win => {
            if let Some(bestiary) = find_pile(table, "Bestiary") {
                for c in cards_of_types(table, place, &["foe", "encounter"]) {
                    let _ = table.move_card(c, bestiary, 0);
                }
            }
        }
        CombatOutcome::Loss => {
            if let Some(inn) = ashfen(table) {
                for c in cards_of_types(table, place, &["location-token"]) {
                    let _ = table.move_card(c, inn, 0);
                }
            }
        }
    }
    if let (Some(day), Some(progress), Some(events)) = (
        find_pile(table, "Day"),
        find_pile(table, "Progress"),
        find_pile(table, "Events"),
    ) {
        let _ = table.advance_day(day, progress, events);
    }
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

/// The inn — Ashfen Crossing, the centre place inside the Locations grid.
fn ashfen(table: &Tableau) -> Option<PileId> {
    let locations = find_pile(table, "Locations")?;
    subpiles(table, locations).into_iter().find(|&p| {
        table
            .pile(p)
            .is_some_and(|pile| pile.label == "Ashfen Crossing")
    })
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

    /// Recruit Identity hero #0 with `recipe`, station the party, then move that hero's location-token to
    /// the place labelled `dest`. Returns the destination place id.
    fn station_at(t: &mut Tableau, recipe: Recipe, dest: &str) -> PileId {
        let (identity, stats, numbers, abilities) = (
            deck(t, "Identity"),
            deck(t, "Stats"),
            deck(t, "Numbers"),
            deck(t, "Abilities"),
        );
        let hero = t.content_cards(identity)[0];
        let name = t.card(hero).unwrap().name().to_string();
        t.equip_character(hero, &recipe, stats, numbers, abilities)
            .unwrap();
        let ashfen = ashfen(t).unwrap();
        t.sync_party(deck(t, "Day"), ashfen, deck(t, "Roster"))
            .unwrap();
        // The hero's location-token now rests at the inn; march it to `dest`.
        let token = t
            .content_cards(ashfen)
            .into_iter()
            .find(|&c| {
                let k = t.card(c).unwrap();
                k.card_type() == "location-token" && k.front_title() == name
            })
            .expect("the stationed hero's token");
        let place = t
            .pile(deck(t, "Locations"))
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| t.pile(p).unwrap().label == dest)
            .unwrap();
        t.move_character(token, place, deck(t, "Day")).unwrap();
        place
    }

    fn foe_count(t: &Tableau, place: PileId) -> usize {
        cards_of_types(t, place, &["foe"]).len()
    }

    /// The right kit wins: a Marksman clears Cinderwatch Keep's Coil — the foes leave the place and a day
    /// passes.
    #[test]
    fn answering_kit_wins_and_clears_the_foes() {
        let mut t = sample_table();
        let day_before = t.current_day(deck(&t, "Progress"));
        let place = station_at(&mut t, marksman(), "Cinderwatch Keep");
        assert!(foe_count(&t, place) > 0, "the Coil is stationed here");

        let outcome = resolve_encounter(&mut t, place, 1);
        assert_eq!(outcome, CombatOutcome::Win);
        assert_eq!(
            foe_count(&t, place),
            0,
            "defeated foes cleared from the place"
        );
        assert_eq!(
            t.current_day(deck(&t, "Progress")),
            day_before + 1,
            "the fight cost a day"
        );
    }

    /// The wrong kit loses: an Executioner cannot crack Cinderwatch Keep's Coil — it retreats to the inn,
    /// the foes remain, and a day passes.
    #[test]
    fn wrong_kit_loses_and_retreats_to_the_inn() {
        let mut t = sample_table();
        let place = station_at(&mut t, executioner(), "Cinderwatch Keep");
        let inn = ashfen(&t).unwrap();

        let outcome = resolve_encounter(&mut t, place, 1);
        assert_eq!(outcome, CombatOutcome::Loss);
        assert!(foe_count(&t, place) > 0, "the foes still hold the place");
        let retreated = cards_of_types(&t, inn, &["location-token"])
            .iter()
            .any(|&c| t.card(c).unwrap().front_title() == "Vael Thornbrand");
        assert!(retreated, "the beaten hero is back at the inn");
    }
}
