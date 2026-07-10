//! **Stage 2/3 — board ↔ combat.** Represent a v2 fight on the physical `Tableau` and drive it through the
//! stage-1 mechanics brain ([`crate::combat`]). The *entire* fight state is physical (cards-as-truth):
//!
//! - **Rank is pile membership.** `[Arena]` holds a sub-pile per rank — `[Vanguard]` / `[Outrider]` /
//!   `[Rearguard]` — plus a `[Pool]` of not-yet-ranked heroes. A combatant *is* in its rank's pile; assigning
//!   a rank is just moving the card there (the generic "move a card into a pile", so drag and tap both work,
//!   no combat-specific input path). A hero's constant stats are re-derived each resolve from the source; its
//!   mutable state (HP / tempo on detail 0–1) and the player's *staged plan* (active / aim / bid / reaction on
//!   detail 2+) ride its card, edited by taps, consumed on commit, cleared at each mini-phase boundary.
//! - The **phase card** (loose in `[Arena]`) carries the walk position `(round, sub-phase, step)` where step
//!   is Marshal → Catch → React → Extra. A round is Marshal then five sub-phases, each three one-way steps.
//! - **Contacts** (a landed catch: attacker→target at a bid) are scratch `contact` cards (loose in `[Arena]`),
//!   written at Catch so React and Extra can read what landed, and torn down at the sub-phase boundary.
//!
//! One [`commit`] advances **one step**, folding the party's staged plan (greedy fallback) with the greedy
//! foe. Marshal gates on an empty `[Pool]` (the renderer disables Start until then); a direct commit fills any
//! stragglers with their default rank. [`handle_tap`] edits the staged plan (and, in Marshal, cycles a hero's
//! rank pile).

use cardtable_model::{CardId, CardKind, PileId, Tableau};
use deckbound::actor::Intention as Rank;
use deckbound::combat::{SCHEDULE, SUB_PHASE_NAMES};

use crate::combat::{self, Catch, Combatant, Contact, ExtraStrike, React, Side};

/// The top-level zone a fight lives in while it runs.
pub const ARENA: &str = "Arena";
/// The holding pile for heroes who have not been assigned a rank yet (Marshal).
pub const POOL: &str = "Pool";

/// The three rank piles, in formation display order (front to back).
const RANK_PILES: [(&str, Rank); 3] = [
    ("Outrider", Rank::Outrider),
    ("Vanguard", Rank::Vanguard),
    ("Rearguard", Rank::Rearguard),
];

// ---- constant stats, derived from the source ([Might, Vitality, Toughness, Cadence, Finesse]) ----------

struct Stats {
    might: u32,
    vitality: u32,
    toughness: u32,
    cadence: u32,
    finesse: u32,
}

fn stats_of(s: [u8; 5], melee: bool, ranged: bool) -> (Stats, bool, bool) {
    (
        Stats {
            might: s[0] as u32,
            vitality: s[1] as u32,
            toughness: s[2] as u32,
            cadence: s[3] as u32,
            finesse: s[4] as u32,
        },
        melee,
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

/// A sub-pile of `arena` by label (a rank pile or the pool).
fn sub_pile(board: &Tableau, arena: PileId, label: &str) -> Option<PileId> {
    board
        .pile(arena)?
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

/// A combatant's stats plus its **reach** `(melee, ranged)` — the attack types it carries (see
/// `catalog::ability_reach`). Returned together so callers can position + gate it in one read.
fn hero_stats(board: &Tableau, name: &str) -> Option<(Stats, bool, bool)> {
    let recipe = board.character_recipe(character_deck(board, name)?)?;
    let (melee, ranged) = cardtable_model::catalog::ability_reach(&recipe.ability);
    Some(stats_of(recipe.stats, melee, ranged))
}

fn foe_stats(name: &str) -> Option<(Stats, bool, bool)> {
    let c = cardtable_model::catalog::creature(name)?;
    Some(stats_of(c.stats, c.melee, c.ranged))
}

/// The vitality (max HP) of a combatant by name and side — the health bar's full value.
fn max_health(board: &Tableau, name: &str, side: Side) -> u32 {
    match side {
        Side::Party => hero_stats(board, name).map(|(s, _, _)| s.vitality),
        Side::Foe => foe_stats(name).map(|(s, _, _)| s.vitality),
    }
    .unwrap_or(0)
}

/// The default opening rank (matching deckbound's stat-derived formation): ranged → Rearguard, else
/// Might ≥ Toughness → Outrider, else Vanguard. Heroes start in the Pool; foes take this automatically.
fn default_rank(s: &Stats, ranged: bool) -> Rank {
    if ranged {
        Rank::Rearguard
    } else if s.might >= s.toughness {
        Rank::Outrider
    } else {
        Rank::Vanguard
    }
}

fn rank_label(rank: Rank) -> &'static str {
    RANK_PILES
        .iter()
        .find(|(_, r)| *r == rank)
        .map(|(l, _)| *l)
        .unwrap_or("Vanguard")
}

// ---- the walk position: (round, sub-phase, step) on the phase card -------------------------------------

/// One of the three one-way mini-phases within a sub-phase, plus the per-round Marshal (rank assignment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Assign / re-assign ranks before the schedule runs (round start).
    Marshal,
    /// Attackers bid tempo to reach targets.
    Catch,
    /// Defenders eat / evade / strike back per incoming contact.
    React,
    /// Units on a surviving contact flip remaining tempo for extra strikes.
    Extra,
}

// ---- combatant card state (HP/tempo on detail 0-1, staged plan on 2+) -----------------------------------

fn detail(hp: u32, max: u32, tempo: u32, finesse: u32, melee: bool, ranged: bool) -> Vec<String> {
    // Finesse rides the card (the game re-derives stats from the source, but the renderer needs it to show
    // affordability) and the reach flags (`Melee` / `Ranged`) ride its line, so the formation can flag which
    // positions a unit is effective in. All three are constant; the staged plan starts after these lines.
    vec![
        format!("HP {hp}/{max}"),
        format!("Tempo {tempo}"),
        format!(
            "Finesse {finesse}{}{}",
            if melee { " Melee" } else { "" },
            if ranged { " Ranged" } else { "" }
        ),
    ]
}

fn num_after(line: &str, prefix: &str) -> u32 {
    line.strip_prefix(prefix)
        .and_then(|s| s.split(['/', ' ']).next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Read one combatant card into a [`Combatant`] — constant stats from the source, mutable state from detail;
/// its `rank` is supplied by the caller (it is the rank pile the card lives in).
fn read_combatant(board: &Tableau, card: CardId, rank: Rank) -> Option<Combatant> {
    let c = board.card(card)?;
    let name = c.front_title().to_string();
    let side = match c.card_type() {
        "unit" => Side::Party,
        "foe" => Side::Foe,
        _ => return None,
    };
    let (stats, melee, ranged) = match side {
        Side::Party => hero_stats(board, &name)?,
        Side::Foe => foe_stats(&name)?,
    };
    let d = c.detail();
    let hp = d
        .first()
        .map(|l| num_after(l, "HP "))
        .unwrap_or(stats.vitality);
    let tempo = d
        .get(1)
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
        melee,
        ranged,
        tempo,
        health: hp,
        pending: 0,
        fallen: hp == 0,
    })
}

/// Write a resolved combatant's mutable state back onto its card (HP / tempo). This also **clears the staged
/// plan** (detail is reset to the two base lines).
fn write_combatant(board: &mut Tableau, card: CardId, u: &Combatant, max: u32) {
    // Reach is a constant of the character, re-derived from the source so it survives the writeback.
    let (melee, ranged) = match u.side {
        Side::Party => hero_stats(board, &u.name).map(|(_, m, r)| (m, r)),
        Side::Foe => foe_stats(&u.name).map(|(_, m, r)| (m, r)),
    }
    .unwrap_or((true, false));
    let _ = board.set_card_detail(
        card,
        detail(u.health, max, u.tempo, u.finesse, melee, ranged),
    );
}

// ---- the staged plan (detail lines after the base two) ------------------------------------------------

/// A defender's staged reaction kind (the cards it spends are derived at commit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReactKind {
    Eat,
    Evade,
    StrikeBack,
}

impl ReactKind {
    fn label(self) -> &'static str {
        match self {
            ReactKind::Eat => "Eat",
            ReactKind::Evade => "Evade",
            ReactKind::StrikeBack => "Strike Back",
        }
    }
    fn next(self) -> ReactKind {
        match self {
            ReactKind::Eat => ReactKind::Evade,
            ReactKind::Evade => ReactKind::StrikeBack,
            ReactKind::StrikeBack => ReactKind::Eat,
        }
    }
}

/// One party unit's staged orders for the current mini-phase (read from / written to its detail).
#[derive(Clone, Copy, Debug, Default)]
struct Staged {
    active: bool,
    aim: Option<CardId>,
    bid: u32,
    react: Option<ReactKind>,
}

fn read_staged(d: &[String]) -> Staged {
    let mut s = Staged::default();
    for line in d.iter().skip(3) {
        if line == "active" {
            s.active = true;
        } else if let Some(id) = line.strip_prefix("aim ") {
            s.aim = id.trim().parse().ok().map(CardId);
        } else if let Some(n) = line.strip_prefix("bid ") {
            s.bid = n.trim().parse().unwrap_or(0);
        } else if let Some(k) = line.strip_prefix("react ") {
            s.react = Some(match k {
                "Evade" => ReactKind::Evade,
                "Strike Back" => ReactKind::StrikeBack,
                _ => ReactKind::Eat,
            });
        }
    }
    s
}

/// Rewrite a unit card's detail as the two base lines followed by whatever of the plan is set.
fn write_staged(board: &mut Tableau, card: CardId, s: &Staged) {
    let Some(d) = board.card(card).map(|c| c.detail().to_vec()) else {
        return;
    };
    let mut lines: Vec<String> = d.into_iter().take(3).collect();
    while lines.len() < 3 {
        lines.push(String::new());
    }
    if s.active {
        lines.push("active".into());
    }
    if let Some(aim) = s.aim {
        lines.push(format!("aim {}", aim.0));
    }
    if s.bid > 0 {
        lines.push(format!("bid {}", s.bid));
    }
    if let Some(r) = s.react {
        lines.push(format!("react {}", r.label()));
    }
    let _ = board.set_card_detail(card, lines);
}

fn staged_of(board: &Tableau, card: CardId) -> Staged {
    board
        .card(card)
        .map(|c| read_staged(c.detail()))
        .unwrap_or_default()
}

// ---- contact scratch cards (loose in [Arena]) ---------------------------------------------------------

fn write_contacts(board: &mut Tableau, arena: PileId, cards: &[CardId], contacts: &[Contact]) {
    for c in contacts {
        if let Ok(card) = board.add_card(
            arena,
            cardtable_model::Face::Down {
                title: "contact".into(),
            },
            None,
        ) {
            let _ = board.set_card_kind(card, CardKind::Virtual);
            let _ = board.set_card_type(card, "contact");
            let _ = board.set_card_detail(
                card,
                vec![
                    format!("from {}", cards[c.attacker].0),
                    format!("to {}", cards[c.target].0),
                    format!("bid {}", c.bid),
                ],
            );
        }
    }
}

/// Read the surviving contact cards back into [`Contact`]s (indices into `cards`).
fn read_contacts(board: &Tableau, arena: PileId, cards: &[CardId]) -> Vec<Contact> {
    let index = |id: u64| cards.iter().position(|c| c.0 == id);
    board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("contact"))
        .filter_map(|c| {
            let d = board.card(c)?.detail().to_vec();
            let atk = index(num_after(d.first()?, "from ") as u64)?;
            let tgt = index(num_after(d.get(1)?, "to ") as u64)?;
            let bid = num_after(d.get(2)?, "bid ");
            Some(Contact {
                attacker: atk,
                target: tgt,
                bid,
            })
        })
        .collect()
}

fn clear_contacts(board: &mut Tableau, arena: PileId) {
    let contacts: Vec<CardId> = board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("contact"))
        .collect();
    for c in contacts {
        let _ = board.remove_card(c);
    }
}

// ---- reading the arena into combatants + the walk position ---------------------------------------------

/// The combatant cards (rank-pile order) and their [`Combatant`]s, plus the walk position from the phase
/// card. Pool (unranked) heroes are not combatants yet — they are excluded until Marshal ranks them.
fn arena_state(board: &Tableau, arena: PileId) -> (Vec<CardId>, Vec<Combatant>, usize, u32, Step) {
    let mut cards = Vec::new();
    let mut units = Vec::new();
    for (label, rank) in RANK_PILES {
        let Some(rp) = sub_pile(board, arena, label) else {
            continue;
        };
        for c in board.content_cards(rp) {
            if let Some(u) = read_combatant(board, c, rank) {
                cards.push(c);
                units.push(u);
            }
        }
    }
    let (sub, round, step) = read_phase(board, arena);
    (cards, units, sub, round, step)
}

fn read_phase(board: &Tableau, arena: PileId) -> (usize, u32, Step) {
    let round = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("round"))
        .and_then(|c| board.card(c))
        .map(|c| num_after(c.front_title(), "Round "))
        .unwrap_or(1);
    // The current major phase is the top of [Phases]; if it is a sub-phase, the mini-phase is the top of
    // [Steps]. Marshal (or a missing deck) means the schedule has not started this round.
    let major = deck_top(board, arena, PHASES);
    match major.as_deref() {
        Some(name) if name != "Marshal" => {
            let sub = SUB_PHASE_NAMES.iter().position(|&n| n == name).unwrap_or(0);
            let step = match deck_top(board, arena, STEPS).as_deref() {
                Some("React") => Step::React,
                Some("Extra") => Step::Extra,
                _ => Step::Catch,
            };
            (sub, round, step)
        }
        _ => (0, round, Step::Marshal),
    }
}

/// The vitality (max HP) of every combatant, index-aligned with the cards.
fn maxes_of(board: &Tableau, units: &[Combatant]) -> Vec<u32> {
    units
        .iter()
        .map(|u| max_health(board, &u.name, u.side).max(u.health))
        .collect()
}

/// Heroes still in the Pool (unranked) — the formation is complete when this is empty.
fn pool_heroes(board: &Tableau, arena: PileId) -> Vec<CardId> {
    sub_pile(board, arena, POOL)
        .map(|p| board.content_cards(p))
        .unwrap_or_default()
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("unit"))
        .collect()
}

/// Whether every hero has been assigned a rank (the Pool is empty) — the Start gate.
pub fn formation_complete(board: &Tableau, arena: PileId) -> bool {
    pool_heroes(board, arena).is_empty()
}

// ---- opening a fight ----------------------------------------------------------------------------------

/// Open a fight at `place`: build the `[Arena]` zone with a `[Pool]` and a pile per rank, a combatant card
/// per stationed hero (moved into the Pool, unranked) and per instantiated foe (drawn from the Bestiary into
/// its default rank pile), a phase card at round 1 · Marshal, and focus it.
pub fn open_fight(board: &mut Tableau, place: PileId) -> Option<PileId> {
    let bestiary = top_deck(board, "Bestiary")?;
    let root = board.root_id();
    let arena = board.add_pile(root, ARENA).ok()?;
    let pool = board.add_pile(arena, POOL).ok()?;
    for (label, _) in RANK_PILES {
        let _ = board.add_pile(arena, label);
    }

    // A hidden meta card remembers the originating place (for teardown) — loose in the arena.
    if let Ok(meta) = board.add_card(
        arena,
        cardtable_model::Face::Down {
            title: format!("place {}", place.0),
        },
        None,
    ) {
        let _ = board.set_card_kind(meta, CardKind::Virtual);
        let _ = board.set_card_type(meta, "arena-meta");
    }

    // Heroes: each stationed hero position card becomes a party combatant, moved into the (unranked) Pool.
    let heroes: Vec<CardId> = board
        .content_cards(place)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
        .collect();
    for card in heroes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, melee, ranged)) = hero_stats(board, &name) {
            let at = board.pile(pool).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, pool, at);
            let _ = board.set_card_type(card, "unit");
            let _ = board.set_card_detail(
                card,
                detail(
                    stats.vitality,
                    stats.vitality,
                    stats.cadence,
                    stats.finesse,
                    melee,
                    ranged,
                ),
            );
        }
    }

    // Foes: instantiate the encounter roster from the Bestiary, then move each into its default rank pile.
    let label = board.pile(place)?.label.clone();
    let foes = board
        .instantiate_encounter_foes(bestiary, arena, &label)
        .ok()?;
    for card in foes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, melee, ranged)) = foe_stats(&name) {
            let _ = board.set_card_type(card, "foe");
            let _ = board.set_card_detail(
                card,
                detail(
                    stats.vitality,
                    stats.vitality,
                    stats.cadence,
                    stats.finesse,
                    melee,
                    ranged,
                ),
            );
            let rank = default_rank(&stats, ranged);
            if let Some(rp) = sub_pile(board, arena, rank_label(rank)) {
                let at = board.pile(rp).map_or(0, |p| p.cards().len());
                let _ = board.move_card(card, rp, at);
            }
        }
    }

    install_phase_decks(board, arena);
    let _ = board.focus(arena);
    Some(arena)
}

/// The two rotating **phase decks**, sub-piles of the arena, so the phase is encoded in the physical card
/// ordering (packing up the deck tells you the phase). `[Phases]` holds the six major phases (Marshal + the
/// five sub-phases); `[Steps]` holds the three mini-phases (Catch/React/Extra). The **top** card of each is
/// the current one; a transition moves the top card to the bottom. A loose `round` card counts the rounds.
const PHASES: &str = "Phases";
const STEPS: &str = "Steps";

/// (Re)install the phase decks + round counter at round 1: Marshal on top of `[Phases]`, Catch on top of
/// `[Steps]`. Each sub-phase card carries its legal rank pairs, so the renderer reads them off the top card.
fn install_phase_decks(board: &mut Tableau, arena: PileId) {
    for label in [PHASES, STEPS] {
        if let Some(p) = sub_pile(board, arena, label) {
            let _ = board.remove_pile(p);
        }
    }
    let stale: Vec<CardId> = board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("round"))
        .collect();
    for c in stale {
        let _ = board.remove_card(c);
    }
    if let Ok(phases) = board.add_pile(arena, PHASES) {
        add_deck_card(board, phases, "Marshal", "phase-major", None);
        for (i, name) in SUB_PHASE_NAMES.iter().enumerate() {
            add_deck_card(board, phases, name, "phase-major", Some(pairs_line(i)));
        }
    }
    if let Ok(steps) = board.add_pile(arena, STEPS) {
        for name in ["Catch", "React", "Extra"] {
            add_deck_card(board, steps, name, "phase-mini", None);
        }
    }
    set_round(board, arena, 1);
}

fn add_deck_card(
    board: &mut Tableau,
    deck: PileId,
    title: &str,
    card_type: &str,
    pairs: Option<String>,
) {
    if let Ok(card) = board.add_card(
        deck,
        cardtable_model::Face::Up {
            title: title.to_string(),
        },
        None,
    ) {
        let _ = board.set_card_kind(card, CardKind::Virtual);
        let _ = board.set_card_type(card, card_type);
        if let Some(p) = pairs {
            let _ = board.set_card_detail(card, vec![format!("Pairs: {p}")]);
        }
    }
}

/// Write the round-counter card (`Round N`, loose in the arena).
fn set_round(board: &mut Tableau, arena: PileId, round: u32) {
    let title = format!("Round {round}");
    if let Some(c) = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("round"))
    {
        let _ = board.set_face(c, cardtable_model::Face::Up { title });
    } else if let Ok(c) = board.add_card(arena, cardtable_model::Face::Up { title }, None) {
        let _ = board.set_card_kind(c, CardKind::Virtual);
        let _ = board.set_card_type(c, "round");
    }
}

/// The title of a phase deck's top (current) card.
fn deck_top(board: &Tableau, arena: PileId, label: &str) -> Option<String> {
    let deck = sub_pile(board, arena, label)?;
    let top = board.pile(deck)?.cards().first().copied()?;
    board.card(top).map(|c| c.front_title().to_string())
}

/// Rotate a phase deck one step: move its top card to the bottom (a phase transition).
fn rotate_deck(board: &mut Tableau, arena: PileId, label: &str) {
    let Some(deck) = sub_pile(board, arena, label) else {
        return;
    };
    let cards = board.pile(deck).map(|p| p.cards()).unwrap_or_default();
    if let Some(&top) = cards.first() {
        let _ = board.move_card(top, deck, cards.len());
    }
}

/// The sub-phase's legal `attacker>target` rank pairs as first-letter codes (e.g. `"V>O,R>V"`).
fn pairs_line(sub: usize) -> String {
    let letter = |r: Rank| r.label().chars().next().unwrap_or('?');
    SCHEDULE
        .get(sub)
        .map(|pairs| {
            pairs
                .iter()
                .map(|(a, t)| format!("{}>{}", letter(*a), letter(*t)))
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

// ---- the greedy AI (foe side always; party side when nothing is staged) --------------------------------

/// Party-agnostic greedy plan for `side`'s catches in sub-phase `sub`: each living unit catches the first
/// living enemy it may legally reach (SCHEDULE), bidding the minimum tempo to land.
fn greedy_catches(units: &[Combatant], side: Side, sub: usize) -> Vec<Catch> {
    let mut catches = Vec::new();
    for (i, u) in units.iter().enumerate() {
        if u.fallen
            || u.side != side
            || u.tempo == 0
            || !combat::effective_in_rank(u.rank, u.melee, u.ranged)
        {
            continue;
        }
        if let Some((t, cards)) = units.iter().enumerate().find_map(|(j, v)| {
            if v.fallen || v.side == side || !combat::legal_catch(sub, u.rank, v.rank) {
                return None;
            }
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
fn greedy_extras(units: &[Combatant], side: Side, surviving: &[Contact]) -> Vec<ExtraStrike> {
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

/// The party's staged catches (aim + bid on party units), keeping only ones the SCHEDULE permits.
fn party_catches(board: &Tableau, cards: &[CardId], units: &[Combatant], sub: usize) -> Vec<Catch> {
    let mut catches = Vec::new();
    for (i, u) in units.iter().enumerate() {
        if u.fallen
            || u.side != Side::Party
            || !combat::effective_in_rank(u.rank, u.melee, u.ranged)
        {
            continue;
        }
        let s = staged_of(board, cards[i]);
        let (Some(aim), true) = (s.aim, s.bid > 0) else {
            continue;
        };
        if let Some(t) = cards.iter().position(|&c| c == aim)
            && combat::legal_catch(sub, u.rank, units[t].rank)
        {
            catches.push(Catch {
                attacker: i,
                target: t,
                cards: s.bid,
            });
        }
    }
    catches
}

/// The party's staged extra strikes (bid on party units still on a surviving contact).
fn party_extras(
    board: &Tableau,
    cards: &[CardId],
    units: &[Combatant],
    surviving: &[Contact],
) -> Vec<ExtraStrike> {
    surviving
        .iter()
        .filter(|c| units[c.attacker].side == Side::Party)
        .filter_map(|c| {
            let bid = staged_of(board, cards[c.attacker]).bid;
            (bid > 0).then_some(ExtraStrike {
                attacker: c.attacker,
                target: c.target,
                cards: bid,
            })
        })
        .collect()
}

// ---- committing one step ------------------------------------------------------------------------------

/// Whether the fight is over, and who won (`Some(true)` = party). A side loses when all its units are fallen;
/// heroes still in the Pool (not yet ranked, during Marshal) count as living party members.
pub fn outcome(board: &Tableau, arena: PileId) -> Option<bool> {
    let (_, units, _, _, _) = arena_state(board, arena);
    let party_alive = units.iter().any(|u| u.side == Side::Party && !u.fallen)
        || !pool_heroes(board, arena).is_empty();
    let foes_alive = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party_alive, foes_alive) {
        (true, true) => None,
        (won, _) => Some(won),
    }
}

/// The label for the Commit control, given the current step (or the fight's decision).
pub fn commit_label(board: &Tableau, arena: PileId) -> &'static str {
    match outcome(board, arena) {
        Some(true) => "Victory - leave",
        Some(false) => "Defeat - leave",
        None => match arena_state(board, arena).4 {
            Step::Marshal => {
                if formation_complete(board, arena) {
                    "Start"
                } else {
                    "Assign every hero a rank"
                }
            }
            // The Step deck already names the mini-phase (Catch/React/Extra), so the button is just "Commit".
            Step::Catch | Step::React | Step::Extra => "Commit",
        },
    }
}

/// Whether the current step offers the **player** any decision. A step with nothing for the party to do —
/// no unit that can legally catch, no incoming contact to react to, no surviving contact to strike along —
/// can be auto-resolved (greedy foe) and skipped. Marshal always needs input (assign ranks / Start).
pub fn step_needs_input(board: &Tableau, arena: PileId) -> bool {
    let (cards, units, sub, _round, step) = arena_state(board, arena);
    let living_party = |i: usize| units[i].side == Side::Party && !units[i].fallen;
    match step {
        Step::Marshal => true,
        Step::Catch => units.iter().enumerate().any(|(i, u)| {
            living_party(i)
                && u.tempo > 0
                && combat::effective_in_rank(u.rank, u.melee, u.ranged)
                && units.iter().any(|v| {
                    v.side == Side::Foe && !v.fallen && combat::legal_catch(sub, u.rank, v.rank)
                })
        }),
        Step::React => read_contacts(board, arena, &cards)
            .iter()
            .any(|c| living_party(c.target)),
        Step::Extra => read_contacts(board, arena, &cards)
            .iter()
            .any(|c| living_party(c.attacker) && units[c.attacker].tempo > 0),
    }
}

/// A compact note for the current step when it is being auto-skipped: `"{Phase}|{Step}|{why}"` (pipe-encoded
/// so the renderer can group consecutive same-phase skips - a whole skipped sub-phase collapses to its name).
/// ASCII - it is stored on a card (so it shows in the physical log too).
pub fn current_skip_line(board: &Tableau, arena: PileId) -> String {
    let (_, _, sub, _, step) = arena_state(board, arena);
    let phase = SUB_PHASE_NAMES.get(sub).copied().unwrap_or("?");
    let (name, reason) = match step {
        Step::Catch => ("Catch", "no legal target"),
        Step::React => ("React", "nothing struck you"),
        Step::Extra => ("Extra", "no surviving contact"),
        Step::Marshal => ("Marshal", ""),
    };
    format!("{phase}|{name}|{reason}")
}

/// Clear the record of auto-skipped steps (a fresh burst starts each time the player commits).
pub fn clear_skips(board: &mut Tableau, arena: PileId) {
    let stale: Vec<CardId> = board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("skiplog"))
        .collect();
    for c in stale {
        let _ = board.remove_card(c);
    }
}

/// Append a line to the skiplog card (a loose card that accumulates the current burst's auto-skips).
pub fn note_skip(board: &mut Tableau, arena: PileId, line: String) {
    if let Some(c) = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("skiplog"))
    {
        let mut d = board
            .card(c)
            .map(|k| k.detail().to_vec())
            .unwrap_or_default();
        d.push(line);
        let _ = board.set_card_detail(c, d);
    } else if let Ok(c) = board.add_card(
        arena,
        cardtable_model::Face::Down {
            title: "skipped".into(),
        },
        None,
    ) {
        let _ = board.set_card_kind(c, CardKind::Virtual);
        let _ = board.set_card_type(c, "skiplog");
        let _ = board.set_card_detail(c, vec![line]);
    }
}

/// Cards to spend to evade a `bid` at finesse `f_def`: the smallest count whose value strictly exceeds it.
fn evade_cost(bid: u32, f_def: u32, tempo: u32) -> u32 {
    (bid / f_def.max(1) + 1).min(tempo)
}

/// Move any Pool stragglers into their default rank pile (a direct commit's safety net; the UI gates Start
/// on an empty Pool, so this only fires for headless / auto play).
fn autofill_pool(board: &mut Tableau, arena: PileId) {
    for card in pool_heroes(board, arena) {
        let Some(name) = board.card(card).map(|c| c.front_title().to_string()) else {
            continue;
        };
        if let Some((stats, _melee, ranged)) = hero_stats(board, &name) {
            let rank = default_rank(&stats, ranged);
            if let Some(rp) = sub_pile(board, arena, rank_label(rank)) {
                let at = board.pile(rp).map_or(0, |p| p.cards().len());
                let _ = board.move_card(card, rp, at);
            }
        }
    }
}

/// Resolve **one step** — Marshal / Catch / React / Extra — folding the party's staged plan (or the greedy
/// AI when nothing is staged) with the greedy foe plan, then writing the results back and advancing the walk.
/// Returns whether the fight is over.
pub fn commit(board: &mut Tableau, arena: PileId) -> bool {
    let (_, _, _, round0, step0) = arena_state(board, arena);
    let _ = round0;
    if step0 == Step::Marshal {
        autofill_pool(board, arena); // safety net; the UI gates Start until the Pool is empty
        rotate_deck(board, arena, PHASES); // Marshal -> the first sub-phase (Intercept); Steps stays at Catch
        return outcome(board, arena).is_some();
    }

    let (cards, mut units, sub, round, step) = arena_state(board, arena);
    let maxes = maxes_of(board, &units);
    let writeback = |board: &mut Tableau, units: &[Combatant]| {
        for (i, card) in cards.iter().enumerate() {
            write_combatant(board, *card, &units[i], maxes[i]);
        }
    };

    match step {
        Step::Marshal => unreachable!("handled above"),

        Step::Catch => {
            let mut catches = party_catches(board, &cards, &units, sub);
            if catches.is_empty() {
                catches = greedy_catches(&units, Side::Party, sub);
            }
            catches.extend(greedy_catches(&units, Side::Foe, sub));
            let contacts = combat::resolve_catch(&mut units, &catches);
            writeback(board, &units); // clears the staged catch plan
            write_contacts(board, arena, &cards, &contacts);
            rotate_deck(board, arena, STEPS); // Catch -> React
        }

        Step::React => {
            let contacts = read_contacts(board, arena, &cards);
            let reactions: Vec<React> = contacts
                .iter()
                .map(|c| {
                    if units[c.target].side != Side::Party {
                        return React::Eat; // greedy foe soaks
                    }
                    match staged_of(board, cards[c.target]).react {
                        Some(ReactKind::Evade) => React::Evade {
                            cards: evade_cost(
                                c.bid,
                                units[c.target].finesse,
                                units[c.target].tempo,
                            ),
                        },
                        Some(ReactKind::StrikeBack) => React::StrikeBack,
                        _ => React::Eat,
                    }
                })
                .collect();
            let surviving = combat::resolve_react(&mut units, &contacts, &reactions);
            writeback(board, &units); // clears the staged reactions
            clear_contacts(board, arena);
            write_contacts(board, arena, &cards, &surviving);
            rotate_deck(board, arena, STEPS); // React -> Extra
        }

        Step::Extra => {
            let surviving = read_contacts(board, arena, &cards);
            let mut extras = party_extras(board, &cards, &units, &surviving);
            if extras.is_empty() {
                extras = greedy_extras(&units, Side::Party, &surviving);
            }
            extras.extend(greedy_extras(&units, Side::Foe, &surviving));
            combat::resolve_extra(&mut units, &extras);
            combat::end_sub_phase(&mut units);
            clear_contacts(board, arena);

            // Advance both decks: Steps back to Catch, Phases on to the next sub-phase. If Phases wraps back
            // to Marshal, a new round has begun - refresh tempo and bump the round counter.
            rotate_deck(board, arena, STEPS);
            rotate_deck(board, arena, PHASES);
            let new_round = deck_top(board, arena, PHASES).as_deref() == Some("Marshal");
            if new_round {
                combat::refresh_round(&mut units);
            }
            writeback(board, &units);
            if new_round {
                set_round(board, arena, round + 1);
            }
        }
    }

    outcome(board, arena).is_some()
}

// ---- editing the plan by tap / drop -------------------------------------------------------------------

/// The pile (Pool or a rank pile) a combatant `card` currently sits in, if it is in the arena.
fn combatant_pile(board: &Tableau, arena: PileId, card: CardId) -> Option<PileId> {
    std::iter::once(POOL)
        .chain(RANK_PILES.iter().map(|(l, _)| *l))
        .filter_map(|label| sub_pile(board, arena, label))
        .find(|&p| board.pile(p).is_some_and(|p| p.cards().contains(&card)))
}

/// Whether `card` is a combatant in `arena` (in the Pool or a rank pile) — a legal tap target.
pub fn is_combatant(board: &Tableau, arena: PileId, card: CardId) -> bool {
    matches!(
        board.card(card).map(|k| k.card_type()),
        Some("unit") | Some("foe")
    ) && combatant_pile(board, arena, card).is_some()
}

/// Whether `pile` is one of the arena's rank piles (a legal formation drop target).
pub fn is_rank_pile(board: &Tableau, arena: PileId, pile: PileId) -> bool {
    RANK_PILES
        .iter()
        .any(|(label, _)| sub_pile(board, arena, label) == Some(pile))
}

/// The current fight step (for the renderer / the drop rule).
pub fn current_step(board: &Tableau, arena: PileId) -> Step {
    read_phase(board, arena).2
}

/// Assign a hero to a rank by moving its card into that rank pile (the drag / drop path). No-op unless it is
/// a party unit and `to` is a rank pile of the arena — rank *is* pile membership.
pub fn assign(board: &mut Tableau, unit: CardId, to: PileId) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    if board.card(unit).map(|k| k.card_type()) == Some("unit") && is_rank_pile(board, arena, to) {
        let at = board.pile(to).map_or(0, |p| p.cards().len());
        let _ = board.move_card(unit, to, at);
    }
}

/// Move a hero to the *next* pile in the Pool → Outrider → Vanguard → Rearguard → Pool cycle (the no-drag
/// rank assignment, and the tap fallback during Marshal).
fn cycle_rank_pile(board: &mut Tableau, arena: PileId, card: CardId) {
    let order: Vec<&str> = std::iter::once(POOL)
        .chain(RANK_PILES.iter().map(|(l, _)| *l))
        .collect();
    let here = combatant_pile(board, arena, card);
    let cur = order
        .iter()
        .position(|&l| sub_pile(board, arena, l) == here)
        .unwrap_or(0);
    let next = order[(cur + 1) % order.len()];
    if let Some(dest) = sub_pile(board, arena, next) {
        let at = board.pile(dest).map_or(0, |p| p.cards().len());
        let _ = board.move_card(card, dest, at);
    }
}

fn select_active(board: &mut Tableau, cards: &[CardId], units: &[Combatant], chosen: usize) {
    // An ineffective body (wrong reach for its position) carries no usable strike this phase, so it never
    // becomes the active attacker - there is nothing for it to aim.
    if !combat::effective_in_rank(
        units[chosen].rank,
        units[chosen].melee,
        units[chosen].ranged,
    ) {
        return;
    }
    for (i, c) in cards.iter().enumerate() {
        if units[i].side != Side::Party {
            continue;
        }
        let mut s = staged_of(board, *c);
        s.active = i == chosen;
        write_staged(board, *c, &s);
    }
}

/// Cycle a unit's staged bid 0 → 1 → … → remaining tempo → 0.
fn cycle_bid(board: &mut Tableau, card: CardId, tempo: u32) {
    let mut s = staged_of(board, card);
    s.bid = if s.bid >= tempo { 0 } else { s.bid + 1 };
    write_staged(board, card, &s);
}

/// Aim the active party attacker at foe index `foe` (if the SCHEDULE permits), seeding a minimum bid.
/// The minimum tempo the attacker must bid to *land* a catch on the target: `ceil(F_target / F_att)`.
fn min_to_land(attacker: &Combatant, target: &Combatant) -> u32 {
    target.finesse.div_ceil(attacker.finesse.max(1)).max(1)
}

fn aim_active(board: &mut Tableau, cards: &[CardId], units: &[Combatant], sub: usize, foe: usize) {
    let Some(active) = (0..units.len())
        .find(|&i| units[i].side == Side::Party && staged_of(board, cards[i]).active)
    else {
        return;
    };
    if !combat::effective_in_rank(
        units[active].rank,
        units[active].melee,
        units[active].ranged,
    ) || !combat::legal_catch(sub, units[active].rank, units[foe].rank)
    {
        return;
    }
    let mut s = staged_of(board, cards[active]);
    // Re-tapping the already-aimed foe cancels the catch (there is no bid-0 "no catch" state to cycle to).
    if s.aim == Some(cards[foe]) {
        s.aim = None;
        s.bid = 0;
        write_staged(board, cards[active], &s);
        return;
    }
    // Only aim if the attacker can actually afford to land it; seed at the minimum landing bid.
    let need = min_to_land(&units[active], &units[foe]);
    if need > units[active].tempo {
        return;
    }
    s.aim = Some(cards[foe]);
    s.bid = need;
    write_staged(board, cards[active], &s);
}

/// Cycle the active attacker's catch bid through the *landing* range `[min_to_land, tempo]`, wrapping back to
/// the minimum - never below it (those bids miss) and never 0 (that is just "no catch"; cancel by re-tapping
/// the foe). A no-op if the unit has no aim.
fn cycle_catch_bid(board: &mut Tableau, cards: &[CardId], units: &[Combatant], active: usize) {
    let s = staged_of(board, cards[active]);
    let Some(aim) = s.aim else {
        return;
    };
    let Some(target) = cards.iter().position(|&c| c == aim) else {
        return;
    };
    let tempo = units[active].tempo;
    let min = min_to_land(&units[active], &units[target]);
    if min > tempo {
        return;
    }
    let next = if s.bid >= tempo {
        min
    } else {
        (s.bid + 1).max(min)
    };
    let mut s = s;
    s.bid = next;
    write_staged(board, cards[active], &s);
}

/// Cycle a defender's staged reaction. `strikeback_ok` gates the Strike Back option: strike-back is
/// melee-vs-melee only (spec 3.1 / the design call), so when the incoming blow is ranged, or the defender
/// carries no melee, the cycle skips straight over Strike Back (Eat -> Evade -> Eat).
fn cycle_react(board: &mut Tableau, card: CardId, strikeback_ok: bool) {
    let mut s = staged_of(board, card);
    let mut next = s.react.unwrap_or(ReactKind::Eat).next();
    if next == ReactKind::StrikeBack && !strikeback_ok {
        next = next.next(); // skip Strike Back -> back to Eat
    }
    s.react = Some(next);
    write_staged(board, card, &s);
}

/// Handle a tap on combatant `card`: in Marshal, cycle its rank pile (the no-drag assignment); in a combat
/// step, edit the staged plan (a no-op if the tap is meaningless there). Reads the step from the phase card.
pub fn handle_tap(board: &mut Tableau, card: CardId) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    let step = current_step(board, arena);
    if step == Step::Marshal {
        if board.card(card).map(|k| k.card_type()) == Some("unit") {
            cycle_rank_pile(board, arena, card);
        }
        return;
    }
    let (cards, units, sub, _round, _step) = arena_state(board, arena);
    let Some(i) = cards.iter().position(|&c| c == card) else {
        return;
    };
    let side = units[i].side;
    match step {
        Step::Marshal => unreachable!("handled above"),
        Step::Catch => match side {
            Side::Party => {
                if staged_of(board, card).active {
                    cycle_catch_bid(board, &cards, &units, i);
                } else {
                    select_active(board, &cards, &units, i);
                }
            }
            Side::Foe => aim_active(board, &cards, &units, sub, i),
        },
        Step::React => {
            let incoming: Vec<Contact> = read_contacts(board, arena, &cards)
                .into_iter()
                .filter(|c| c.target == i)
                .collect();
            if side == Side::Party && !incoming.is_empty() {
                // Strike Back is offered only if this defender carries a melee blow AND at least one incoming
                // strike is melee (its attacker came to you - a Vanguard/Outrider, not a Rearguard's shot).
                let strikeback_ok = units[i].melee
                    && incoming
                        .iter()
                        .any(|c| !combat::rank_is_ranged(units[c.attacker].rank));
                cycle_react(board, card, strikeback_ok);
            }
        }
        Step::Extra => {
            let outgoing = read_contacts(board, arena, &cards)
                .iter()
                .any(|c| c.attacker == i);
            if side == Side::Party && outgoing {
                cycle_bid(board, card, units[i].tempo);
            }
        }
    }
}

// ---- teardown -----------------------------------------------------------------------------------------

/// **The game-side authority for "is a fight modal right now".** The arena is *modal*: it is active whenever
/// it **exists**, regardless of which zone `focus` points at. Every arena decision the game makes — the
/// `Commit` / `Cancel` / `Tap` intentions and the `affordances` it offers — gates on this, **never on the
/// focused pile**, so drilling `focus` into a rank sub-pile can't strip the fight's controls or actions.
/// Mirrors the renderer's `active_arena`; both find the top-level `[Arena]` by label.
pub fn find_arena(board: &Tableau) -> Option<PileId> {
    top_deck(board, ARENA)
}

/// The place a fight was opened from, remembered in the hidden meta card.
fn place_of(board: &Tableau, arena: PileId) -> Option<PileId> {
    board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("arena-meta"))
        .find_map(|c| board.card(c).map(|k| k.front_title().to_string()))
        .map(|s| PileId(num_after(&s, "place ") as u64))
}

/// Every combatant card in the arena (across the Pool and all rank piles), by side-type.
fn all_of_type(board: &Tableau, arena: PileId, card_type: &str) -> Vec<CardId> {
    let mut out = Vec::new();
    for label in std::iter::once(POOL).chain(RANK_PILES.iter().map(|(l, _)| *l)) {
        if let Some(p) = sub_pile(board, arena, label) {
            out.extend(
                board
                    .content_cards(p)
                    .into_iter()
                    .filter(|&c| board.card(c).map(|k| k.card_type()) == Some(card_type)),
            );
        }
    }
    out
}

/// Tear the arena down: return foe cards to the Bestiary, move the party's units back to the place as hero
/// position cards, remove the arena, and leave it. `clear_encounter` removes the beaten encounter (a win);
/// `spend_day` advances the day clock (a resolved fight costs a day, a cancel does not). Conservation-clean.
fn teardown(board: &mut Tableau, arena: PileId, clear_encounter: bool, spend_day: bool) {
    let place = place_of(board, arena);
    let bestiary = top_deck(board, "Bestiary");

    let foes = all_of_type(board, arena, "foe");
    if let Some(b) = bestiary {
        let _ = board.return_foes_to_bestiary(&foes, b);
    }

    let units = all_of_type(board, arena, "unit");
    if let Some(place) = place {
        for u in units {
            let _ = board.set_card_type(u, "hero");
            let _ = board.set_card_detail(u, Vec::new());
            let at = board.pile(place).map_or(0, |p| p.cards().len());
            let _ = board.move_card(u, place, at);
        }
        if clear_encounter
            && let Some(enc) = board
                .content_cards(place)
                .into_iter()
                .find(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
        {
            let _ = board.remove_card(enc);
        }
    }

    // Leave the arena subtree entirely before removing it. Focus may have drilled into a rank sub-pile, so a
    // single `zoom_out` wouldn't fully exit — point focus at the root (the arena is always a root sub-pile)
    // so nothing is left focused on a pile that's about to be removed (which would dangle / panic the draw).
    let root = board.root_id();
    let _ = board.focus(root);
    let _ = board.remove_pile(arena);
    if spend_day
        && let (Some(p), Some(e)) = (top_deck(board, "Progress"), top_deck(board, "Events"))
    {
        let _ = board.advance_day(p, e);
    }
}

/// **Fold the fight back** after a decision: on a **win** the encounter is cleared; the fight spends a day.
pub fn fold_back(board: &mut Tableau, arena: PileId) {
    let won = outcome(board, arena) == Some(true);
    teardown(board, arena, won, true);
}

/// **Cancel the fight** (retreat): tear the arena down with nothing resolved — the encounter is left intact
/// and no day is spent. The clean inverse of [`open_fight`], for backing out of a battle.
pub fn cancel_fight(board: &mut Tableau, arena: PileId) {
    teardown(board, arena, false, false);
}

/// **Restart the fight**: reset every combatant to full HP and fresh tempo, drop the staged plans and any
/// landed contacts, and return to round 1 · Marshal — **keeping the current formation** (rank-pile
/// membership), so you can re-form or just Start again. Foes, place, and encounter are untouched.
pub fn restart_fight(board: &mut Tableau, arena: PileId) {
    clear_contacts(board, arena);
    for label in std::iter::once(POOL).chain(RANK_PILES.iter().map(|(l, _)| *l)) {
        let Some(pile) = sub_pile(board, arena, label) else {
            continue;
        };
        for card in board.content_cards(pile) {
            let Some((name, ctype)) = board
                .card(card)
                .map(|c| (c.front_title().to_string(), c.card_type().to_string()))
            else {
                continue;
            };
            let stats = match ctype.as_str() {
                "unit" => hero_stats(board, &name),
                "foe" => foe_stats(&name),
                _ => None,
            };
            if let Some((s, melee, ranged)) = stats {
                let _ = board.set_card_detail(
                    card,
                    detail(s.vitality, s.vitality, s.cadence, s.finesse, melee, ranged),
                );
            }
        }
    }
    install_phase_decks(board, arena);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardtable_model::sample_table;

    /// A recruited melee kit (Phantom carries Jab) must flag `Melee` (not `no strike`) on its combat card:
    /// `hero_stats` reads the reach off the ability, and `detail` writes the token the renderer parses. Guards
    /// the "Dallen Rook shows no strike" regression - which can only occur if a card carries a stale,
    /// pre-reach detail line (a fight persisted by an older build), never from this live path.
    #[test]
    fn a_melee_kit_flags_melee_on_its_combat_card() {
        use crate::CardTableGame;
        use crate::Intention;
        use cardtable_model::BoardGame;

        let mut board = sample_table();
        let heroes = top_deck(&board, "Heroes").unwrap();
        let kit = top_deck(&board, "Kit").unwrap();
        let hero = board.pile(heroes).unwrap().cards()[0];
        let hero_name = board.card(hero).unwrap().front_title().to_string();
        let phantom = board
            .pile(kit)
            .unwrap()
            .cards()
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.front_title()) == Some("Phantom"))
            .unwrap();
        CardTableGame.apply(
            &mut board,
            &[Intention::Equip {
                identity: hero,
                kit: phantom,
            }],
        );

        let (stats, melee, ranged) = hero_stats(&board, &hero_name).expect("recipe resolves");
        assert!(melee && !ranged, "Jab is melee-only");
        let d = detail(
            stats.vitality,
            stats.vitality,
            stats.cadence,
            stats.finesse,
            melee,
            ranged,
        );
        assert!(
            d[2].contains("Melee"),
            "the combat card carries the Melee token: {:?}",
            d[2]
        );
        assert!(!d[2].contains("Ranged"));
    }

    /// Set up a fight at a place with an encounter, with Vael recruited (Marksman) and marched there.
    fn open_a_fight(board: &mut Tableau) -> PileId {
        use crate::CardTableGame;
        use crate::Intention;
        use cardtable_model::BoardGame;

        let game = CardTableGame;
        let heroes = top_deck(board, "Heroes").unwrap();
        let kit = top_deck(board, "Kit").unwrap();
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
            board,
            &[Intention::Equip {
                identity: vael,
                kit: marksman,
            }],
        );

        let locations = top_deck(board, "Locations").unwrap();
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
        let position = board
            .content_cards(board.pile(locations).unwrap().subpiles()[4])
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.card_type()) == Some("hero"))
            .unwrap();
        let progress = top_deck(board, "Progress").unwrap();
        let _ = board.move_character(position, place, progress);
        open_fight(board, place).expect("a fight opens")
    }

    #[test]
    fn a_fight_opens_with_heroes_in_the_pool_and_foes_ranked() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        assert_eq!(current_step(&board, arena), Step::Marshal);
        assert!(
            !formation_complete(&board, arena),
            "heroes start unranked in the Pool"
        );
        // Foes are already ranked (arena_state only sees ranked combatants).
        let (_, units, _, _, _) = arena_state(&board, arena);
        assert!(units.iter().any(|u| u.side == Side::Foe), "foes are ranked");
        assert!(
            !units.iter().any(|u| u.side == Side::Party),
            "no hero is ranked yet"
        );
    }

    #[test]
    fn assigning_a_hero_to_a_rank_pile_completes_the_formation() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        let hero = pool_heroes(&board, arena)[0];
        let outrider = sub_pile(&board, arena, "Outrider").unwrap();
        assign(&mut board, hero, outrider);
        assert!(
            formation_complete(&board, arena),
            "the only hero is now ranked, so the Pool is empty"
        );
        let (_, units, _, _, _) = arena_state(&board, arena);
        assert!(
            units
                .iter()
                .any(|u| u.side == Side::Party && u.rank == Rank::Outrider),
            "the hero is a combatant in the Outrider rank"
        );
    }

    #[test]
    fn a_fight_resolves_to_a_winner() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        // Commit Marshal (auto-fills the Pool), then drive steps to a decision.
        let mut guard = 0;
        while outcome(&board, arena).is_none() {
            commit(&mut board, arena);
            guard += 1;
            assert!(guard < 2000, "the fight must terminate");
        }
        assert!(
            outcome(&board, arena).is_some(),
            "the fight reached a winner"
        );
    }

    #[test]
    fn cancel_tears_the_arena_down_leaving_the_encounter_intact() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        let place = place_of(&board, arena).expect("the fight remembers its place");
        assert!(
            board
                .content_cards(place)
                .iter()
                .any(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter")),
            "the place has an encounter before the fight"
        );

        // Cancel from the arena (focus == arena, as after open_fight) must not panic and must tear down.
        cancel_fight(&mut board, arena);

        assert!(find_arena(&board).is_none(), "the arena was torn down");
        assert!(
            board
                .content_cards(place)
                .iter()
                .any(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter")),
            "cancel leaves the encounter intact (no win)"
        );
        assert!(
            board
                .content_cards(place)
                .iter()
                .any(|&c| board.card(c).map(|k| k.card_type()) == Some("hero")),
            "the heroes returned to the place"
        );
    }

    #[test]
    fn taps_stage_a_plan_and_catch_persists_a_contact() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        commit(&mut board, arena); // Marshal -> Catch (auto-ranks the hero)
        assert_eq!(current_step(&board, arena), Step::Catch);

        let (cards, units, sub, _, _) = arena_state(&board, arena);
        let pi = units.iter().position(|u| u.side == Side::Party).unwrap();
        if let Some(fi) = (0..units.len()).find(|&j| {
            units[j].side == Side::Foe && combat::legal_catch(sub, units[pi].rank, units[j].rank)
        }) {
            handle_tap(&mut board, cards[pi]); // select active
            assert!(staged_of(&board, cards[pi]).active);
            handle_tap(&mut board, cards[fi]); // aim at the foe (seeds a bid)
            let staged = staged_of(&board, cards[pi]);
            assert_eq!(staged.aim, Some(cards[fi]));
            assert!(staged.bid >= 1, "aiming seeds at least one card of bid");

            commit(&mut board, arena); // resolve Catch
            assert_eq!(current_step(&board, arena), Step::React);
            let contacts = read_contacts(&board, arena, &arena_state(&board, arena).0);
            assert!(
                contacts.iter().any(|c| c.attacker == pi),
                "the staged catch landed as a persisted contact"
            );
        }
    }
}
