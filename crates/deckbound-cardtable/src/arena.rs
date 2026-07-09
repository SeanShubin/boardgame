//! **Stage 2 — board ↔ combat.** Represent a v2 fight on the physical `Tableau` and drive it through the
//! stage-1 mechanics brain ([`crate::combat`]). A combatant is a card in an `[Arena]` zone; its *mutable*
//! state (rank / HP / tempo) rides its detail lines (a first-playable choice — flip-piles are a later purity
//! refinement), while its *constant* stats are re-derived each resolve from the source (a hero's
//! `character_recipe`, a foe's `catalog::creature`). One sub-phase resolves per commit (commit ≡ end-sub-
//! phase): [`resolve_sub_phase`] reads the arena → `Combatant`s → runs Catch/React/Extra order-free → writes
//! HP/tempo/deaths back and advances the phase card.

use cardtable_model::{CardId, CardKind, PileId, Tableau};
use deckbound::actor::Intention as Rank;
use deckbound::combat::{SCHEDULE, SUB_PHASE_NAMES};

use crate::combat::{self, Catch, Combatant, ExtraStrike, React, Side};

/// The top-level zone a fight lives in while it runs.
pub const ARENA: &str = "Arena";

// ---- constant stats, derived from the source ([Might, Vitality, Toughness, Cadence, Finesse]) ----------

struct Stats {
    might: u32,
    vitality: u32,
    toughness: u32,
    cadence: u32,
    finesse: u32,
}

fn stats_of(s: [u8; 5], ranged: bool) -> (Stats, bool) {
    (
        Stats {
            might: s[0] as u32,
            vitality: s[1] as u32,
            toughness: s[2] as u32,
            cadence: s[3] as u32,
            finesse: s[4] as u32,
        },
        ranged,
    )
}

fn top_deck(board: &Tableau, label: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|p| p.label.as_str()) == Some(label))
}

fn character_deck(board: &Tableau, name: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&p| {
            board
                .pile(p)
                .and_then(|q| q.reflects())
                .and_then(|c| board.card(c))
                .map(|c| c.front_title())
                == Some(name)
        })
}

fn hero_stats(board: &Tableau, name: &str) -> Option<(Stats, bool)> {
    let recipe = board.character_recipe(character_deck(board, name)?)?;
    let (ranged, _aoe) = cardtable_model::catalog::ability_shape(&recipe.ability);
    Some(stats_of(recipe.stats, ranged))
}

fn foe_stats(name: &str) -> Option<(Stats, bool)> {
    let c = cardtable_model::catalog::creature(name)?;
    Some(stats_of(c.stats, c.ranged))
}

/// The default opening rank (matching deckbound's stat-derived formation): ranged → Rearguard, else
/// Might ≥ Toughness → Outrider, else Vanguard. The player re-declares before the schedule runs.
fn default_rank(s: &Stats, ranged: bool) -> Rank {
    if ranged {
        Rank::Rearguard
    } else if s.might >= s.toughness {
        Rank::Outrider
    } else {
        Rank::Vanguard
    }
}

// ---- combatant card state (encoded in detail) ----------------------------------------------------------

fn detail(rank: Rank, hp: u32, max: u32, tempo: u32) -> Vec<String> {
    vec![
        rank.label().to_string(),
        format!("HP {hp}/{max}"),
        format!("Tempo {tempo}"),
    ]
}

fn rank_of(label: &str) -> Rank {
    match label {
        "Outrider" => Rank::Outrider,
        "Rearguard" => Rank::Rearguard,
        _ => Rank::Vanguard,
    }
}

fn num_after(line: &str, prefix: &str) -> u32 {
    line.strip_prefix(prefix)
        .and_then(|s| s.split(['/', ' ']).next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Read one combatant card into a [`Combatant`] — constant stats from the source, mutable state from detail.
fn read_combatant(board: &Tableau, card: CardId) -> Option<Combatant> {
    let c = board.card(card)?;
    let name = c.front_title().to_string();
    let side = match c.card_type() {
        "unit" => Side::Party,
        "foe" => Side::Foe,
        _ => return None,
    };
    let (stats, _ranged) = match side {
        Side::Party => hero_stats(board, &name)?,
        Side::Foe => foe_stats(&name)?,
    };
    let d = c.detail();
    let rank = rank_of(d.first().map(String::as_str).unwrap_or(""));
    let hp = d
        .get(1)
        .map(|l| num_after(l, "HP "))
        .unwrap_or(stats.vitality);
    let tempo = d
        .get(2)
        .map(|l| num_after(l, "Tempo "))
        .unwrap_or(stats.cadence);
    Some(Combatant {
        name,
        side,
        rank,
        might: stats.might,
        finesse: stats.finesse.max(1),
        cadence: stats.cadence,
        toughness: stats.toughness.max(1),
        tempo,
        health: hp,
        pending: 0,
        fallen: hp == 0,
    })
}

/// Write a resolved combatant's mutable state back onto its card (HP / tempo; rank unchanged mid-round).
fn write_combatant(board: &mut Tableau, card: CardId, u: &Combatant, max: u32) {
    let _ = board.set_card_detail(card, detail(u.rank, u.health, max, u.tempo));
}

// ---- opening a fight ----------------------------------------------------------------------------------

/// Open a fight at `place`: build the `[Arena]` zone with a combatant card per stationed hero (moved in) and
/// per instantiated foe (drawn from the Bestiary), a phase card, and focus it. Returns the arena pile.
pub fn open_fight(board: &mut Tableau, place: PileId) -> Option<PileId> {
    let bestiary = top_deck(board, "Bestiary")?;
    let root = board.root_id();
    let arena = board.add_pile(root, ARENA).ok()?;

    // Heroes: each stationed hero position card becomes a party combatant (moved into the arena).
    let heroes: Vec<CardId> = board
        .content_cards(place)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
        .collect();
    for card in heroes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, ranged)) = hero_stats(board, &name) {
            let at = board.pile(arena).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, arena, at);
            let _ = board.set_card_type(card, "unit");
            let rank = default_rank(&stats, ranged);
            let _ = board.set_card_detail(
                card,
                detail(rank, stats.vitality, stats.vitality, stats.cadence),
            );
        }
    }

    // Foes: instantiate the encounter roster from the Bestiary into the arena, annotate as combatants.
    let label = board.pile(place)?.label.clone();
    let foes = board
        .instantiate_encounter_foes(bestiary, arena, &label)
        .ok()?;
    for card in foes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, ranged)) = foe_stats(&name) {
            let _ = board.set_card_type(card, "foe");
            let rank = default_rank(&stats, ranged);
            let _ = board.set_card_detail(
                card,
                detail(rank, stats.vitality, stats.vitality, stats.cadence),
            );
        }
    }

    add_phase_card(board, arena, 0, 1);
    let _ = board.focus(arena);
    Some(arena)
}

fn add_phase_card(board: &mut Tableau, arena: PileId, sub: usize, round: u32) {
    let name = SUB_PHASE_NAMES.get(sub).copied().unwrap_or("Marshal");
    if let Ok(card) = board.add_card(
        arena,
        cardtable_model::Face::Up {
            title: format!("Phase: {name}"),
        },
        None,
    ) {
        let _ = board.set_card_kind(card, CardKind::Virtual);
        let _ = board.set_card_type(card, "phase");
        let _ = board.set_card_detail(
            card,
            vec![format!("Round {round}"), format!("Sub-phase {}/5", sub + 1)],
        );
    }
}

// ---- reading the arena into combatants + the card map --------------------------------------------------

/// The combatant cards in the arena (in order) and the current (sub-phase, round) from the phase card.
fn arena_state(board: &Tableau, arena: PileId) -> (Vec<CardId>, Vec<Combatant>, usize, u32) {
    let mut cards = Vec::new();
    let mut units = Vec::new();
    let (mut sub, mut round) = (0usize, 1u32);
    for c in board.content_cards(arena) {
        match board.card(c).map(|k| k.card_type()) {
            Some("unit") | Some("foe") => {
                if let Some(u) = read_combatant(board, c) {
                    cards.push(c);
                    units.push(u);
                }
            }
            Some("phase") => {
                let d = board
                    .card(c)
                    .map(|k| k.detail().to_vec())
                    .unwrap_or_default();
                round = d.first().map(|l| num_after(l, "Round ")).unwrap_or(1);
                sub = d.get(1).map(|l| num_after(l, "Sub-phase ")).unwrap_or(1) as usize;
                sub = sub.saturating_sub(1);
            }
            _ => {}
        }
    }
    (cards, units, sub, round)
}

// ---- the greedy foe AI (a simple, tunable stand-in for the first playable) -----------------------------

/// Party-agnostic greedy plan for `side`'s catches in sub-phase `sub`: each living unit catches the first
/// living enemy it may legally reach (SCHEDULE), bidding the minimum tempo to land.
fn greedy_catches(units: &[Combatant], side: Side, sub: usize) -> Vec<Catch> {
    let mut catches = Vec::new();
    for (i, u) in units.iter().enumerate() {
        if u.fallen || u.side != side || u.tempo == 0 {
            continue;
        }
        if let Some((t, cards)) = units.iter().enumerate().find_map(|(j, v)| {
            if v.fallen || v.side == side || !combat::legal_catch(sub, u.rank, v.rank) {
                return None;
            }
            // minimum cards to land: ceil(F_target / F_att), capped at tempo.
            let need = v.finesse.div_ceil(u.finesse.max(1));
            (need <= u.tempo).then_some((j, need))
        }) {
            catches.push(Catch {
                attacker: i,
                target: t,
                cards,
            });
        }
    }
    catches
}

/// Greedy extra strikes for `side`: each still-contacted unit dumps its remaining tempo on its contact.
fn greedy_extras(
    units: &[Combatant],
    side: Side,
    surviving: &[combat::Contact],
) -> Vec<ExtraStrike> {
    surviving
        .iter()
        .filter(|c| units[c.attacker].side == side && units[c.attacker].tempo > 0)
        .map(|c| ExtraStrike {
            attacker: c.attacker,
            target: c.target,
            cards: units[c.attacker].tempo,
        })
        .collect()
}

// ---- resolving one sub-phase (greedy both sides for now; player plan lands in stage 2b) ---------------

/// Whether the fight is over, and who won (`Some(true)` = party). A side loses when all its units are fallen.
pub fn outcome(board: &Tableau, arena: PileId) -> Option<bool> {
    let (_, units, _, _) = arena_state(board, arena);
    let party_alive = units.iter().any(|u| u.side == Side::Party && !u.fallen);
    let foes_alive = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party_alive, foes_alive) {
        (true, true) => None,
        (won, _) => Some(won),
    }
}

/// Resolve one sub-phase to completion — Catch → React → Extra strikes — greedily for both sides (the player
/// plan overrides the party side in stage 2b), then write the results back and advance the phase card. At the
/// end of the last sub-phase, roll to the next round (refresh tempo). Returns whether the fight is over.
pub fn resolve_sub_phase(board: &mut Tableau, arena: PileId) -> bool {
    let (cards, mut units, sub, round) = arena_state(board, arena);
    let maxes: Vec<u32> = units
        .iter()
        .map(|u| {
            match u.side {
                Side::Party => hero_stats(board, &u.name).map(|(s, _)| s.vitality),
                Side::Foe => foe_stats(&u.name).map(|(s, _)| s.vitality),
            }
            .unwrap_or(u.health)
        })
        .collect();

    // Catch (both sides), then React, then Extra strikes — one order-free pass.
    let mut catches = greedy_catches(&units, Side::Party, sub);
    catches.extend(greedy_catches(&units, Side::Foe, sub));
    let contacts = combat::resolve_catch(&mut units, &catches);

    // Reactions (defender's choice) — greedy stand-in eats everything for now; the player plan overrides the
    // party side in stage 2b, and a smarter foe AI lands later.
    let reactions = vec![React::Eat; contacts.len()];
    let surviving = combat::resolve_react(&mut units, &contacts, &reactions);

    let mut extras = greedy_extras(&units, Side::Party, &surviving);
    extras.extend(greedy_extras(&units, Side::Foe, &surviving));
    combat::resolve_extra(&mut units, &extras);

    combat::end_sub_phase(&mut units);

    // Write mutable state back.
    for (card, u, &max) in itertools_zip3(&cards, &units, &maxes) {
        write_combatant(board, *card, u, max);
    }

    // Advance the phase; roll the round over after the last sub-phase (refresh tempo).
    let next_sub = sub + 1;
    if next_sub >= SCHEDULE.len() {
        combat::refresh_round(&mut units);
        for (card, u, &max) in itertools_zip3(&cards, &units, &maxes) {
            write_combatant(board, *card, u, max);
        }
        set_phase(board, arena, 0, round + 1);
    } else {
        set_phase(board, arena, next_sub, round);
    }

    outcome(board, arena).is_some()
}

/// Zip three slices (no itertools dep).
fn itertools_zip3<'a>(
    a: &'a [CardId],
    b: &'a [Combatant],
    c: &'a [u32],
) -> impl Iterator<Item = (&'a CardId, &'a Combatant, &'a u32)> {
    a.iter().zip(b).zip(c).map(|((x, y), z)| (x, y, z))
}

fn set_phase(board: &mut Tableau, arena: PileId, sub: usize, round: u32) {
    let name = SUB_PHASE_NAMES.get(sub).copied().unwrap_or("Marshal");
    if let Some(card) = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("phase"))
    {
        let _ = board.set_face(
            card,
            cardtable_model::Face::Up {
                title: format!("Phase: {name}"),
            },
        );
        let _ = board.set_card_detail(
            card,
            vec![format!("Round {round}"), format!("Sub-phase {}/5", sub + 1)],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardtable_model::sample_table;

    /// A hero can be recruited, marched to an encounter, and a fight opened + auto-resolved to a decision.
    #[test]
    fn a_fight_opens_and_resolves_to_a_winner() {
        use crate::CardTableGame;
        use crate::Intention;
        use cardtable_model::BoardGame;

        let game = CardTableGame;
        let mut board = sample_table();

        // Recruit Vael with Marksman, march to Cinderwatch Keep (a place with an encounter).
        let heroes = top_deck(&board, "Heroes").unwrap();
        let kit = top_deck(&board, "Kit").unwrap();
        let vael = board
            .pile(heroes)
            .unwrap()
            .cards()
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.front_title()) == Some("Vael Thornbrand"))
            .unwrap();
        let marksman = board
            .pile(kit)
            .unwrap()
            .cards()
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.front_title()) == Some("Marksman"))
            .unwrap();
        game.apply(
            &mut board,
            &[Intention::Equip {
                identity: vael,
                kit: marksman,
            }],
        );

        let locations = top_deck(&board, "Locations").unwrap();
        // Cinderwatch Keep is a place holding an encounter; find one adjacent-marchable place with an encounter.
        let place = board
            .pile(locations)
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| {
                board
                    .content_cards(p)
                    .iter()
                    .any(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
            })
            .unwrap();
        // Move the character's map position onto that place directly (test setup).
        let position = board
            .content_cards(
                top_deck(&board, "Locations")
                    .map(|loc| board.pile(loc).unwrap().subpiles()[4])
                    .unwrap(),
            )
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
            .unwrap();
        let progress = top_deck(&board, "Progress").unwrap();
        let _ = board.move_character(position, place, progress);

        let arena = open_fight(&mut board, place).expect("a fight opens");
        let (_, units, _, _) = arena_state(&board, arena);
        assert!(
            units.iter().any(|u| u.side == Side::Party),
            "the hero is in the arena"
        );
        assert!(
            units.iter().any(|u| u.side == Side::Foe),
            "foes were instantiated"
        );

        // Drive sub-phases to a decision (bounded).
        let mut guard = 0;
        while outcome(&board, arena).is_none() {
            resolve_sub_phase(&mut board, arena);
            guard += 1;
            assert!(guard < 500, "the fight must terminate");
        }
        assert!(
            outcome(&board, arena).is_some(),
            "the fight reached a winner"
        );
    }
}
