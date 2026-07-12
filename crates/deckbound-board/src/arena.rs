//! **Stage 2/3 — board ↔ combat.** Represent a v2 fight on the physical `Board` and drive it through the
//! stage-1 mechanics brain ([`crate::combat`]). The *entire* fight state is physical (cards-as-truth):
//!
//! - **Rank is pile membership.** `[Arena]` holds a sub-pile per rank — `[Vanguard]` / `[Outrider]` /
//!   `[Rearguard]` — plus a `[Pool]` of not-yet-ranked heroes. A combatant *is* in its rank's pile; assigning
//!   a rank is just moving the card there (the generic "move a card into a pile", so drag and tap both work,
//!   no combat-specific input path). A hero's constant stats are re-derived each resolve from the source; its
//!   mutable state (HP / tempo on detail 0–1) and the player's *staged plan* (active / aim / bid / reaction on
//!   detail 2+) ride its card, edited by taps, consumed on commit, cleared at each mini-phase boundary.
//! - The **phase card** (loose in `[Arena]`) carries the walk position `(round, sub-phase, step)` where step
//!   is Marshal → Strike → React → Extra. A round is Marshal then five sub-phases, each three one-way steps.
//! - **Contacts** (a landed strike: attacker→target at a bid) are scratch `contact` cards (loose in `[Arena]`),
//!   written at Strike so React and Extra can read what landed, and torn down at the sub-phase boundary.
//!
//! One [`commit`] advances **one step**, folding the party's staged plan (greedy fallback) with the greedy
//! foe. Marshal gates on an empty `[Pool]` (the renderer disables Start until then); a direct commit fills any
//! stragglers with their default rank. [`handle_tap`] edits the staged plan (and, in Marshal, cycles a hero's
//! rank pile).

use cardtable_model::{Board, CardId, CardKind, Choice, PileId};
use deckbound_content::rank::Intention as Rank;
use deckbound_content::schedule::{SCHEDULE, SUB_PHASE_NAMES};

use crate::battle::{Greedy, Policy};
use crate::combat::{self, Combatant, Contact, ExtraStrike, React, Side, Strike};

/// The top-level zone a fight lives in while it runs.
pub const ARENA: &str = "Arena";
/// The holding pile for heroes who have not been assigned a rank yet (Marshal).
pub const POOL: &str = "Pool";
/// The foes' holding pile: they stand here, face up and fully readable, but **out of formation**, for as long
/// as the player is declaring theirs.
///
/// Combat is a bet made blind - Marshal declares a formation without seeing the enemy's, and that secrecy is
/// what simulates simultaneity. But you are entitled to know *who* you are fighting; only *where they stand*
/// is withheld. So the foes muster here, and [`reveal`] moves them into their rank piles when you commit.
/// Their formation cannot leak during your declaration because it is not on the table yet - which is a
/// stronger guarantee than a renderer that merely declines to draw it.
pub const MUSTER: &str = "Foes";

/// The three rank piles, in formation display order (front to back).
pub(crate) const RANK_PILES: [(&str, Rank); 3] = [
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

fn stats_of(s: [u8; 5], melee: bool, ranged: bool, aoe: bool) -> (Stats, bool, bool, bool) {
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
        aoe,
    )
}

fn top_deck(board: &Board, label: &str) -> Option<PileId> {
    board
        .pile(board.root_id())?
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|p| p.label.as_str()) == Some(label))
}

/// A sub-pile of `arena` by label (a rank pile or the pool).
pub(crate) fn sub_pile(board: &Board, arena: PileId, label: &str) -> Option<PileId> {
    board
        .pile(arena)?
        .subpiles()
        .into_iter()
        .find(|&p| board.pile(p).map(|p| p.label.as_str()) == Some(label))
}

fn character_deck(board: &Board, name: &str) -> Option<PileId> {
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

/// A combatant's stats plus its **reach** `(melee, ranged)` and **area** flag — the attack shape it carries
/// (see `catalog::ability_reach` / `ability_shape`). Returned together so callers position + gate + display
/// it in one read.
fn hero_stats(board: &Board, name: &str) -> Option<(Stats, bool, bool, bool)> {
    let recipe = board.character_recipe(
        character_deck(board, name)?,
        &deckbound_content::catalog::stat_names(),
    )?;
    let (melee, ranged) = deckbound_content::catalog::ability_reach(&recipe.ability);
    let (_ranged, aoe) = deckbound_content::catalog::ability_shape(&recipe.ability);
    Some(stats_of(recipe.stats, melee, ranged, aoe))
}

fn foe_stats(name: &str) -> Option<(Stats, bool, bool, bool)> {
    let c = deckbound_content::catalog::creature(name)?;
    Some(stats_of(c.stats, c.melee, c.ranged, c.aoe))
}

/// The vitality (max HP) of a combatant by name and side — the health bar's full value.
pub(crate) fn max_health(board: &Board, name: &str, side: Side) -> u32 {
    match side {
        Side::Party => hero_stats(board, name).map(|(s, _, _, _)| s.vitality),
        Side::Foe => foe_stats(name).map(|(s, _, _, _)| s.vitality),
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
    Strike,
    /// Defenders eat / evade / strike back per incoming contact.
    React,
    /// Units on a surviving contact flip remaining tempo for extra strikes.
    Extra,
}

// ---- combatant card state (HP/tempo on detail 0-1, staged plan on 2+) -----------------------------------

#[allow(clippy::too_many_arguments)]
fn detail(
    hp: u32,
    max_hp: u32,
    tempo: u32,
    max_tempo: u32,
    finesse: u32,
    melee: bool,
    ranged: bool,
    area: bool,
    pile: u32,
) -> Vec<String> {
    // Health and Tempo are both **stacks of cards** you flip, so both read the same way: `up / total`. Health
    // is Vitality-many cards (damage flips them down); Tempo is Cadence-many (bidding and striking flip them
    // down, and they all stand back up each round). Showing only the remainder - as Tempo used to - hides how
    // much of the pool is already spent, which is the whole decision in a bid.
    //
    // Finesse rides the card (the game re-derives stats from the source, but the renderer needs it to show
    // affordability); the reach flags (`Melee` / `Ranged`) and the shape flag (`Area`) ride its line so the
    // formation can flag effective positions and the renderer can style an area strike's targeting cue. All
    // are constant; the staged plan starts after these lines.
    //
    // **The damage pile rides the card too**, and it must: it is the one piece of mutable combat state that
    // spans the three mini-phases of a sub-phase (a Strike's blow and an Extra's blow bank into the same pile
    // and only flip a Health card together). It used to live nowhere - rebuilt as 0 on every read - so it was
    // silently wiped at every *step* boundary instead of the sub-phase boundary, and two 7s against a
    // Toughness 9 never added up. The cards are the state; anything that survives a step has to be on one.
    vec![
        format!("Health {hp}/{max_hp}"),
        format!("Tempo {tempo}/{max_tempo}"),
        format!(
            "Finesse {finesse}{}{}{}",
            if melee { " Melee" } else { "" },
            if ranged { " Ranged" } else { "" },
            if area { " Area" } else { "" }
        ),
        format!("Pile {pile}"),
    ]
}

/// The number of leading detail lines that are the unit's *state*. The staged plan starts after them.
const BASE_LINES: usize = 4;

fn num_after(line: &str, prefix: &str) -> u32 {
    line.strip_prefix(prefix)
        .and_then(|s| s.split(['/', ' ']).next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

/// Read one combatant card into a [`Combatant`] — constant stats from the source, mutable state from detail;
/// its `rank` is supplied by the caller (it is the rank pile the card lives in).
pub(crate) fn read_combatant(board: &Board, card: CardId, rank: Rank) -> Option<Combatant> {
    let c = board.card(card)?;
    let name = c.front_title().to_string();
    let side = match c.card_type() {
        "unit" => Side::Party,
        "foe" => Side::Foe,
        _ => return None,
    };
    let (stats, melee, ranged, aoe) = match side {
        Side::Party => hero_stats(board, &name)?,
        Side::Foe => foe_stats(&name)?,
    };
    let d = c.detail();
    // Both lines read `up / total`; `num_after` stops at the `/`, so it reads the *up* count - what's left.
    let hp = d
        .first()
        .map(|l| num_after(l, "Health "))
        .unwrap_or(stats.vitality);
    let tempo = d
        .get(1)
        .map(|l| num_after(l, "Tempo "))
        .unwrap_or(stats.cadence);
    // A horde is a foe-only property (heroes are never grouped in the UI); area came from the read above.
    let horde =
        side == Side::Foe && deckbound_content::catalog::creature(&name).is_some_and(|c| c.horde);
    Some(Combatant {
        name,
        side,
        rank,
        might: stats.might,
        finesse: stats.finesse.max(1),
        cadence: stats.cadence,
        toughness: stats.toughness.max(1),
        armor: 0,
        melee,
        ranged,
        aoe,
        horde,
        tempo,
        health: hp,
        // The sub-phase damage pile, carried on the card so it survives from Strike through React to Extra.
        pending: d.get(3).map(|l| num_after(l, "Pile ")).unwrap_or(0),
        fallen: hp == 0,
    })
}

/// Write a resolved combatant's mutable state back onto its card (HP / tempo). This also **clears the staged
/// plan** (detail is reset to the two base lines).
fn write_combatant(board: &mut Board, card: CardId, u: &Combatant, max: u32) {
    // Reach is a constant of the character, re-derived from the source so it survives the writeback.
    let (melee, ranged) = match u.side {
        Side::Party => hero_stats(board, &u.name).map(|(_, m, r, _)| (m, r)),
        Side::Foe => foe_stats(&u.name).map(|(_, m, r, _)| (m, r)),
    }
    .unwrap_or((true, false));
    let _ = board.set_card_detail(
        card,
        detail(
            u.health, max, u.tempo, u.cadence, u.finesse, melee, ranged, u.aoe, u.pending,
        ),
    );
}

// ---- the staged plan (detail lines after the base two) ------------------------------------------------

/// A defender's staged reaction kind (the cards it spends are derived at commit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReactKind {
    Eat,
    Evade,
    StrikeBack,
}

impl ReactKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            ReactKind::Eat => "Eat",
            ReactKind::Evade => "Evade",
            ReactKind::StrikeBack => "Strike Back",
        }
    }
}

/// One party unit's staged orders for the current mini-phase (read from / written to its detail).
///
/// `hold` is the **explicit** "this hero does nothing here" - distinct from having decided nothing yet. Both
/// produce no strike, but only one of them means the player has answered, and Commit gates on the difference.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Staged {
    pub(crate) active: bool,
    pub(crate) aim: Option<CardId>,
    pub(crate) bid: u32,
    pub(crate) hold: bool,
    pub(crate) react: Option<ReactKind>,
}

fn read_staged(d: &[String]) -> Staged {
    let mut s = Staged::default();
    for line in d.iter().skip(BASE_LINES) {
        if line == "active" {
            s.active = true;
        } else if line == "hold" {
            s.hold = true;
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
fn write_staged(board: &mut Board, card: CardId, s: &Staged) {
    let Some(d) = board.card(card).map(|c| c.detail().to_vec()) else {
        return;
    };
    let mut lines: Vec<String> = d.into_iter().take(BASE_LINES).collect();
    while lines.len() < BASE_LINES {
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
    if s.hold {
        lines.push("hold".into());
    }
    if let Some(r) = s.react {
        lines.push(format!("react {}", r.label()));
    }
    let _ = board.set_card_detail(card, lines);
}

pub(crate) fn staged_of(board: &Board, card: CardId) -> Staged {
    board
        .card(card)
        .map(|c| read_staged(c.detail()))
        .unwrap_or_default()
}

// ---- contact scratch cards (loose in [Arena]) ---------------------------------------------------------

fn write_contacts(board: &mut Board, arena: PileId, cards: &[CardId], contacts: &[Contact]) {
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
pub(crate) fn read_contacts(board: &Board, arena: PileId, cards: &[CardId]) -> Vec<Contact> {
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

fn clear_contacts(board: &mut Board, arena: PileId) {
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
pub(crate) fn arena_state(
    board: &Board,
    arena: PileId,
) -> (Vec<CardId>, Vec<Combatant>, usize, u32, Step) {
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

fn read_phase(board: &Board, arena: PileId) -> (usize, u32, Step) {
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
                _ => Step::Strike,
            };
            (sub, round, step)
        }
        _ => (0, round, Step::Marshal),
    }
}

/// The vitality (max HP) of every combatant, index-aligned with the cards.
pub(crate) fn maxes_of(board: &Board, units: &[Combatant]) -> Vec<u32> {
    units
        .iter()
        .map(|u| max_health(board, &u.name, u.side).max(u.health))
        .collect()
}

/// Heroes still in the Pool (unranked) — the formation is complete when this is empty.
/// The foes still standing in the muster - on the table but not yet in formation (see [`MUSTER`]).
fn muster_foes(board: &Board, arena: PileId) -> Vec<CardId> {
    sub_pile(board, arena, MUSTER)
        .map(|p| board.content_cards(p))
        .unwrap_or_default()
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("foe"))
        .collect()
}

/// The inverse of [`reveal`]: the **living** foes step back out of formation into the muster, at the top of
/// each new round. Intentions are re-declared every round, so each Marshal is a fresh blind bet - and the
/// player must be able to see who is still standing while making it.
///
/// The fallen stay where they fell (a corpse is not "remaining"), which also keeps [`outcome`]'s reading of
/// the muster honest: anything in it is alive.
fn unrank_foes(board: &mut Board, arena: PileId) {
    let Some(muster) = sub_pile(board, arena, MUSTER) else {
        return;
    };
    let (cards, units, _, _, _) = arena_state(board, arena);
    for (i, u) in units.iter().enumerate() {
        if u.side == Side::Foe && !u.fallen {
            let at = board.pile(muster).map_or(0, |p| p.cards().len());
            let _ = board.move_card(cards[i], muster, at);
        }
    }
}

/// **Reveal**: both formations go down at once. The player's is already placed; the foes now step out of the
/// muster into their rank piles. Called on the Marshal commit, which is the moment the blind bet is settled -
/// after this, everything resolves in the open.
fn reveal(board: &mut Board, arena: PileId) {
    for card in muster_foes(board, arena) {
        let Some(name) = board.card(card).map(|c| c.front_title().to_string()) else {
            continue;
        };
        let Some((stats, _melee, ranged, _aoe)) = foe_stats(&name) else {
            continue;
        };
        if let Some(rp) = sub_pile(board, arena, rank_label(default_rank(&stats, ranged))) {
            let at = board.pile(rp).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, rp, at);
        }
    }
}

fn pool_heroes(board: &Board, arena: PileId) -> Vec<CardId> {
    sub_pile(board, arena, POOL)
        .map(|p| board.content_cards(p))
        .unwrap_or_default()
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("unit"))
        .collect()
}

/// Whether every hero has been assigned a rank (the Pool is empty) — the Start gate.
pub fn formation_complete(board: &Board, arena: PileId) -> bool {
    pool_heroes(board, arena).is_empty()
}

// ---- opening a fight ----------------------------------------------------------------------------------

/// Open a fight at `place`: build the `[Arena]` zone with a `[Pool]` and a pile per rank, a combatant card
/// per stationed hero (moved into the Pool, unranked) and per instantiated foe (drawn from the Bestiary into
/// its default rank pile), a phase card at round 1 · Marshal, and focus it.
pub fn open_fight(board: &mut Board, place: PileId) -> Option<PileId> {
    let bestiary = top_deck(board, "Bestiary")?;
    let root = board.root_id();
    let arena = board.add_pile(root, ARENA).ok()?;
    let pool = board.add_pile(arena, POOL).ok()?;
    let muster = board.add_pile(arena, MUSTER).ok()?;
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
        if let Some((stats, melee, ranged, aoe)) = hero_stats(board, &name) {
            let at = board.pile(pool).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, pool, at);
            let _ = board.set_card_type(card, "unit");
            let _ = board.set_card_detail(
                card,
                detail(
                    stats.vitality,
                    stats.vitality,
                    stats.cadence,
                    stats.cadence,
                    stats.finesse,
                    melee,
                    ranged,
                    aoe,
                    0,
                ),
            );
        }
    }

    // Foes: instantiate the encounter roster from the Bestiary into the **muster** - on the table, face up,
    // fully readable, but not yet in formation. `reveal` ranks them when the player commits their own.
    let label = board.pile(place)?.label.clone();
    let foes = board
        .instantiate_from_bank(
            bestiary,
            arena,
            &deckbound_content::catalog::encounter_roster(&label),
        )
        .ok()?;
    for card in foes {
        let name = board.card(card).map(|c| c.front_title().to_string())?;
        if let Some((stats, melee, ranged, aoe)) = foe_stats(&name) {
            let _ = board.set_card_type(card, "foe");
            let _ = board.set_card_detail(
                card,
                detail(
                    stats.vitality,
                    stats.vitality,
                    stats.cadence,
                    stats.cadence,
                    stats.finesse,
                    melee,
                    ranged,
                    aoe,
                    0,
                ),
            );
            let at = board.pile(muster).map_or(0, |p| p.cards().len());
            let _ = board.move_card(card, muster, at);
        }
    }

    install_phase_decks(board, arena);
    let _ = board.focus(arena);
    Some(arena)
}

/// The two rotating **phase decks**, sub-piles of the arena, so the phase is encoded in the physical card
/// ordering (packing up the deck tells you the phase). `[Phases]` holds the six major phases (Marshal + the
/// five sub-phases); `[Steps]` holds the three mini-phases (Strike/React/Extra). The **top** card of each is
/// the current one; a transition moves the top card to the bottom. A loose `round` card counts the rounds.
const PHASES: &str = "Phases";
const STEPS: &str = "Steps";

/// (Re)install the phase decks + round counter at round 1: Marshal on top of `[Phases]`, Strike on top of
/// `[Steps]`. Each sub-phase card carries its legal rank pairs, so the renderer reads them off the top card.
fn install_phase_decks(board: &mut Board, arena: PileId) {
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
        for name in ["Strike", "React", "Extra"] {
            add_deck_card(board, steps, name, "phase-mini", None);
        }
    }
    set_round(board, arena, 1);
}

fn add_deck_card(
    board: &mut Board,
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
fn set_round(board: &mut Board, arena: PileId, round: u32) {
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
fn deck_top(board: &Board, arena: PileId, label: &str) -> Option<String> {
    let deck = sub_pile(board, arena, label)?;
    let top = board.pile(deck)?.cards().first().copied()?;
    board.card(top).map(|c| c.front_title().to_string())
}

/// Rotate a phase deck one step: move its top card to the bottom (a phase transition).
fn rotate_deck(board: &mut Board, arena: PileId, label: &str) {
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
//
// The party-agnostic greedy strike / extra logic is [`battle::Greedy`] (the tooling policy); the arena reuses
// it here for the foe (always) and the party fallback, so there is a single implementation of the greedy plan.
// (The arena's *react* is simpler than the policy's - the greedy foe just eats; see the React step.)

/// The party's staged strikes (aim + bid on party units), keeping only ones the SCHEDULE permits.
fn party_strikes(board: &Board, cards: &[CardId], units: &[Combatant], sub: usize) -> Vec<Strike> {
    let mut strikes = Vec::new();
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
            && combat::legal_strike(sub, u.rank, units[t].rank)
            && combat::back_access_ok(units, u.rank, t)
        {
            strikes.push(Strike {
                attacker: i,
                target: t,
                cards: s.bid,
            });
        }
    }
    strikes
}

/// The party's staged extra strikes (bid on party units still on a surviving contact).
fn party_extras(
    board: &Board,
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

/// Whether the player has given **any** order this step - an aim, a bid, or an explicit Hold. Distinguishes
/// "the party does nothing because that is what I decided" from "nobody is driving", which is what the greedy
/// fallback is for.
fn party_has_orders(board: &Board, cards: &[CardId], units: &[Combatant]) -> bool {
    units.iter().enumerate().any(|(i, u)| {
        if u.side != Side::Party {
            return false;
        }
        let s = staged_of(board, cards[i]);
        s.hold || s.aim.is_some() || s.bid > 0
    })
}

// ---- committing one step ------------------------------------------------------------------------------

/// How a fight ended. A battle is decided by breaking a line - or, failing that, by the clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every foe is fallen.
    Victory,
    /// Every hero is fallen.
    Defeat,
    /// Neither line broke within the round cap: the two sides disengage.
    Draw,
}

/// Whether the fight is over, and how. A side loses when all its units are fallen; heroes still in the Pool
/// (not yet ranked, during Marshal) count as living party members. If both lines still stand once the
/// **round cap** is spent, the fight is a [`Outcome::Draw`] - the same cap the batch resolver honours
/// ([`crate::battle::MAX_ROUNDS`]), so the two engines end a stalemate at the same moment.
pub fn outcome(board: &Board, arena: PileId) -> Option<Outcome> {
    let (_, units, _, round, _) = arena_state(board, arena);
    // Heroes still in the Pool and foes still in the muster are on the table but not yet ranked (Marshal), so
    // they are not combatants - they are very much alive, though, and a fight must not read as won because
    // the enemy has not stepped forward yet.
    let party_alive = units.iter().any(|u| u.side == Side::Party && !u.fallen)
        || !pool_heroes(board, arena).is_empty();
    let foes_alive = units.iter().any(|u| u.side == Side::Foe && !u.fallen)
        || !muster_foes(board, arena).is_empty();
    match (party_alive, foes_alive) {
        (false, _) => Some(Outcome::Defeat),
        (true, false) => Some(Outcome::Victory),
        // The round counter is bumped as each new round opens, so it reads `MAX_ROUNDS + 1` exactly when the
        // last permitted round has finished with both lines intact.
        (true, true) if round as usize > crate::battle::MAX_ROUNDS => Some(Outcome::Draw),
        (true, true) => None,
    }
}

/// A decision the player owes before this step can be committed, named as the Commit control's label. `None`
/// means Commit is live.
///
/// **Nothing is decided by default.** A hero under fire that has not answered has *not* chosen to Eat - it has
/// chosen nothing, and committing would silently make the choice for it. So Commit is barred and *says whose
/// answer is missing*: a disabled control that will not say why is indistinguishable from a bug, the same rule
/// that makes a barred choice carry its reason.
pub fn pending_decision(board: &Board, arena: PileId) -> Option<String> {
    let (cards, units, sub, _round, step) = arena_state(board, arena);
    if step == Step::Marshal {
        return (!formation_complete(board, arena)).then(|| "Assign every hero a rank".to_string());
    }
    let contacts = read_contacts(board, arena, &cards);
    let staged: Vec<Staged> = cards.iter().map(|&c| staged_of(board, c)).collect();
    units.iter().enumerate().find_map(|(i, u)| {
        owes_order(&units, &contacts, &staged, sub, step, i).then(|| match step {
            Step::React => format!("{} has not answered", u.name),
            _ => format!("{} has no orders", u.name),
        })
    })
}

/// Whether **anything is being asked of** party unit `i` this step - it has some legal move here, whether or
/// not it has chosen one yet. A hero with no tempo, no legal target this sub-phase, or nothing to answer with
/// is simply not in this step's conversation.
///
/// This is the single authority for "can this hero act right now", and the board must read the *same* thing
/// the rules do. A Raider standing in the Outrider rank during the Clash has no schedule slot at all - and it
/// has to look as unusable as a foe you cannot select, not merely a touch dimmer than one you can.
pub(crate) fn can_act(
    units: &[Combatant],
    contacts: &[Contact],
    sub: usize,
    step: Step,
    i: usize,
) -> bool {
    let u = &units[i];
    if u.side != Side::Party || u.fallen || u.tempo == 0 {
        return false;
    }
    match step {
        Step::Marshal => false,
        Step::React => {
            let (evade_ok, strikeback_ok) = react_options(units, contacts, i);
            evade_ok || strikeback_ok
        }
        // Reach alone is not enough: the schedule must actually pair this rank against a living, reachable
        // enemy rank *this sub-phase*. Omitting that was the bug - an Outrider at the Clash looked ready.
        Step::Strike => {
            combat::effective_in_rank(u.rank, u.melee, u.ranged)
                && units.iter().enumerate().any(|(j, v)| {
                    v.side == Side::Foe
                        && !v.fallen
                        && combat::legal_strike(sub, u.rank, v.rank)
                        && combat::back_access_ok(units, u.rank, j)
                })
        }
        Step::Extra => contacts.iter().any(|c| c.attacker == i),
    }
}

/// Whether party unit `i` still **owes the player's order** this step: something is being asked of it
/// ([`can_act`]) and it has not said what. The Commit gate bars itself on this, and the board lights the hero
/// on it, so the two can never disagree about who is holding things up.
///
/// A hero that *could* act must say so, **including deliberately doing nothing** - "no aim" cannot mean both
/// "I chose to keep my tempo" and "I have not looked yet".
pub(crate) fn owes_order(
    units: &[Combatant],
    contacts: &[Contact],
    staged: &[Staged],
    sub: usize,
    step: Step,
    i: usize,
) -> bool {
    if !can_act(units, contacts, sub, step, i) {
        return false;
    }
    let s = staged[i];
    match step {
        Step::Marshal => false,
        Step::React => s.react.is_none(),
        Step::Strike => !s.hold && s.aim.is_none(),
        Step::Extra => !s.hold && s.bid == 0,
    }
}

/// The label for the Commit control, given the current step (or the fight's decision). When a decision is
/// owed, the label names it and the control is barred (see [`pending_decision`]).
pub fn commit_label(board: &Board, arena: PileId) -> String {
    match outcome(board, arena) {
        Some(Outcome::Victory) => "Victory - leave".to_string(),
        Some(Outcome::Defeat) => "Defeat - leave".to_string(),
        Some(Outcome::Draw) => "Draw - leave".to_string(),
        None => match pending_decision(board, arena) {
            Some(owed) => owed,
            // The Step deck already names the mini-phase (Strike/React/Extra), so the button is just "Commit".
            None if arena_state(board, arena).4 == Step::Marshal => "Start".to_string(),
            None => "Commit".to_string(),
        },
    }
}

/// Whether the current step offers the **player** any decision. A step with nothing for the party to do —
/// no unit that can legally strike, no incoming contact to react to, no surviving contact to strike along —
/// can be auto-resolved (greedy foe) and skipped. Marshal always needs input (assign ranks / Start).
pub fn step_needs_input(board: &Board, arena: PileId) -> bool {
    let (cards, units, sub, _round, step) = arena_state(board, arena);
    let living_party = |i: usize| units[i].side == Side::Party && !units[i].fallen;
    match step {
        Step::Marshal => true,
        Step::Strike => units.iter().enumerate().any(|(i, u)| {
            living_party(i)
                && u.tempo > 0
                && combat::effective_in_rank(u.rank, u.melee, u.ranged)
                && units.iter().enumerate().any(|(j, v)| {
                    v.side == Side::Foe
                        && !v.fallen
                        && combat::legal_strike(sub, u.rank, v.rank)
                        && combat::back_access_ok(&units, u.rank, j)
                })
        }),
        Step::React => {
            let contacts = read_contacts(board, arena, &cards);
            // Only a real choice needs input: a struck unit that can afford to evade or strike back. One that
            // can only Eat (e.g. no Tempo) auto-resolves.
            units.iter().enumerate().any(|(i, _)| {
                living_party(i) && {
                    let (evade_ok, strikeback_ok) = react_options(&units, &contacts, i);
                    evade_ok || strikeback_ok
                }
            })
        }
        Step::Extra => read_contacts(board, arena, &cards)
            .iter()
            .any(|c| living_party(c.attacker) && units[c.attacker].tempo > 0),
    }
}

/// A compact note for the current step when it is being auto-skipped: `"{Phase}|{Step}|{why}|{pairs}"`
/// (pipe-encoded so the renderer can group by sub-phase). `pairs` is the sub-phase's `attacker>target` rank
/// codes (e.g. `"V>O"`), so the renderer can show *which targeting* was passed over without knowing the
/// SCHEDULE itself. ASCII - it is stored on a card (so it shows in the physical log too).
pub fn current_skip_line(board: &Board, arena: PileId) -> String {
    let (_, _, sub, _, step) = arena_state(board, arena);
    let phase = SUB_PHASE_NAMES.get(sub).copied().unwrap_or("?");
    let (name, reason) = match step {
        Step::Strike => ("Strike", "no legal target"),
        Step::React => ("React", "no reaction available"),
        Step::Extra => ("Extra", "no surviving contact"),
        Step::Marshal => ("Marshal", ""),
    };
    format!("{phase}|{name}|{reason}|{}", pairs_line(sub))
}

/// Clear the record of auto-skipped steps (a fresh burst starts each time the player commits).
pub fn clear_skips(board: &mut Board, arena: PileId) {
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
pub fn note_skip(board: &mut Board, arena: PileId, line: String) {
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

/// The reactions defender `i` can actually **afford** against its incoming contacts (Eat is always free, so it
/// is not returned): `evade_ok` needs enough Tempo to strictly beat some incoming bid; `strikeback_ok` needs a
/// Tempo card *and* a melee answer to a melee blow (spec: they came to you). A defender with neither is forced
/// to Eat, so the UI must offer no choice and the step auto-resolves - a unit with no Tempo cannot evade or
/// strike back.
pub(crate) fn react_options(units: &[Combatant], contacts: &[Contact], i: usize) -> (bool, bool) {
    let u = &units[i];
    if u.fallen || u.tempo == 0 {
        return (false, false);
    }
    let incoming = || contacts.iter().filter(|c| c.target == i);
    let evade_ok = incoming().any(|c| c.bid / u.finesse.max(1) < u.tempo);
    let strikeback_ok =
        u.melee && incoming().any(|c| !combat::rank_is_ranged(units[c.attacker].rank));
    (evade_ok, strikeback_ok)
}

/// Move any Pool stragglers into their default rank pile (a direct commit's safety net; the UI gates Start
/// on an empty Pool, so this only fires for headless / auto play).
fn autofill_pool(board: &mut Board, arena: PileId) {
    for card in pool_heroes(board, arena) {
        let Some(name) = board.card(card).map(|c| c.front_title().to_string()) else {
            continue;
        };
        if let Some((stats, _melee, ranged, _aoe)) = hero_stats(board, &name) {
            let rank = default_rank(&stats, ranged);
            if let Some(rp) = sub_pile(board, arena, rank_label(rank)) {
                let at = board.pile(rp).map_or(0, |p| p.cards().len());
                let _ = board.move_card(card, rp, at);
            }
        }
    }
}

/// Resolve **one step** — Marshal / Strike / React / Extra — folding the party's staged plan (or the greedy
/// AI when nothing is staged) with the greedy foe plan, then writing the results back and advancing the walk.
/// Returns whether the fight is over.
pub fn commit(board: &mut Board, arena: PileId) -> bool {
    let (_, _, _, round0, step0) = arena_state(board, arena);
    let _ = round0;
    if step0 == Step::Marshal {
        autofill_pool(board, arena); // safety net; the UI gates Start until the Pool is empty
        reveal(board, arena); // both formations go down at once: the foes leave the muster and take their ranks
        // Round start: refresh every unit's tempo to its pool - Cadence, or body count for a horde (which
        // "swarms with one card per living body"). The end-of-round refresh only covers later rounds, so the
        // opening round is set here; idempotent for the rounds that were already refreshed.
        let (cards, mut units, _, _, _) = arena_state(board, arena);
        combat::refresh_round(&mut units);
        let maxes = maxes_of(board, &units);
        for (i, card) in cards.iter().enumerate() {
            write_combatant(board, *card, &units[i], maxes[i]);
        }
        rotate_deck(board, arena, PHASES); // Marshal -> the first sub-phase (Intercept); Steps stays at Strike
        return outcome(board, arena).is_some();
    }

    let (cards, mut units, sub, round, step) = arena_state(board, arena);
    let maxes = maxes_of(board, &units);
    let writeback = |board: &mut Board, units: &[Combatant]| {
        for (i, card) in cards.iter().enumerate() {
            write_combatant(board, *card, &units[i], maxes[i]);
        }
    };

    match step {
        Step::Marshal => unreachable!("handled above"),

        Step::Strike => {
            let mut strikes = party_strikes(board, &cards, &units, sub);
            // The greedy fallback is for headless play (tests, the solver), where nobody gave orders. It must
            // NOT fire when the player deliberately Held: an empty strike list is then their decision, not an
            // absent one.
            if strikes.is_empty() && !party_has_orders(board, &cards, &units) {
                strikes = Greedy.strikes(&units, Side::Party, sub);
            }
            strikes.extend(Greedy.strikes(&units, Side::Foe, sub));
            let contacts = combat::resolve_strike(&mut units, &strikes);
            writeback(board, &units); // clears the staged strike plan
            write_contacts(board, arena, &cards, &contacts);
            rotate_deck(board, arena, STEPS); // Strike -> React
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
            if extras.is_empty() && !party_has_orders(board, &cards, &units) {
                extras = Greedy.extras(&units, Side::Party, &surviving);
            }
            extras.extend(Greedy.extras(&units, Side::Foe, &surviving));
            combat::resolve_extra(&mut units, &extras);
            combat::end_sub_phase(&mut units);
            clear_contacts(board, arena);

            // Advance both decks: Steps back to Strike, Phases on to the next sub-phase. If Phases wraps back
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
                // The lines break and re-form: intentions are declared afresh every round, so the surviving
                // foes step back out of formation into the muster. You go into each Marshal seeing exactly who
                // is left and what they carry - and, as on the first round, not where they will stand.
                unrank_foes(board, arena);
            }
        }
    }

    outcome(board, arena).is_some()
}

// ---- editing the plan by tap / drop -------------------------------------------------------------------

/// The pile (Pool or a rank pile) a combatant `card` currently sits in, if it is in the arena.
fn combatant_pile(board: &Board, arena: PileId, card: CardId) -> Option<PileId> {
    std::iter::once(POOL)
        .chain(RANK_PILES.iter().map(|(l, _)| *l))
        .filter_map(|label| sub_pile(board, arena, label))
        .find(|&p| board.pile(p).is_some_and(|p| p.cards().contains(&card)))
}

/// Whether `card` is a combatant in `arena` (in the Pool or a rank pile) — a legal tap target.
pub fn is_combatant(board: &Board, arena: PileId, card: CardId) -> bool {
    matches!(
        board.card(card).map(|k| k.card_type()),
        Some("unit") | Some("foe")
    ) && combatant_pile(board, arena, card).is_some()
}

/// Whether `pile` is one of the arena's rank piles (a legal formation drop target).
pub fn is_rank_pile(board: &Board, arena: PileId, pile: PileId) -> bool {
    RANK_PILES
        .iter()
        .any(|(label, _)| sub_pile(board, arena, label) == Some(pile))
}

/// The current fight step (for the renderer / the drop rule).
pub fn current_step(board: &Board, arena: PileId) -> Step {
    read_phase(board, arena).2
}

/// Assign a hero to a rank by moving its card into that rank pile (the drag / drop path). No-op unless it is
/// a party unit and `to` is a rank pile of the arena — rank *is* pile membership.
pub fn assign(board: &mut Board, unit: CardId, to: PileId) {
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
fn cycle_rank_pile(board: &mut Board, arena: PileId, card: CardId) {
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

fn select_active(board: &mut Board, cards: &[CardId], units: &[Combatant], chosen: usize) {
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

/// Aim the active party attacker at foe index `foe` (if the SCHEDULE permits), seeding a minimum bid.
/// The minimum tempo the attacker must bid to *land* a strike on the target: `ceil(F_target / F_att)`.
fn min_to_land(attacker: &Combatant, target: &Combatant) -> u32 {
    if attacker.aoe {
        return 1; // an area strike is unevadable - one card, no bid to raise
    }
    target.finesse.div_ceil(attacker.finesse.max(1)).max(1)
}

fn aim_active(board: &mut Board, cards: &[CardId], units: &[Combatant], sub: usize, foe: usize) {
    let Some(active) = (0..units.len())
        .find(|&i| units[i].side == Side::Party && staged_of(board, cards[i]).active)
    else {
        return;
    };
    if !combat::effective_in_rank(
        units[active].rank,
        units[active].melee,
        units[active].ranged,
    ) || !combat::legal_strike(sub, units[active].rank, units[foe].rank)
        || !combat::back_access_ok(units, units[active].rank, foe)
    {
        return;
    }
    let mut s = staged_of(board, cards[active]);
    // Re-tapping the already-aimed foe cancels the strike (there is no bid-0 "no strike" state to cycle to).
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

/// Select the defender the React choices apply to (clearing any other selection). Unlike the *attacker*
/// selection at Strike, there is no effectiveness gate: you are being hit whether or not you can hit back.
fn select_defender(board: &mut Board, cards: &[CardId], chosen: usize) {
    for (i, &c) in cards.iter().enumerate() {
        let mut s = staged_of(board, c);
        let want = i == chosen;
        if s.active != want {
            s.active = want;
            write_staged(board, c, &s);
        }
    }
}

/// The struck party defender the React decision is *about*: the one the player selected, or — when only one
/// hero is under fire — that one, so a lone defender needs no selecting first. `None` outside React, or when
/// nobody the player controls was hit.
pub(crate) fn struck_defender(board: &Board, arena: PileId) -> Option<(CardId, usize)> {
    let (cards, units, _sub, _round, step) = arena_state(board, arena);
    if step != Step::React {
        return None;
    }
    let contacts = read_contacts(board, arena, &cards);
    let struck: Vec<usize> = (0..units.len())
        .filter(|&i| {
            units[i].side == Side::Party
                && !units[i].fallen
                && contacts.iter().any(|c| c.target == i)
        })
        .collect();
    let i = struck
        .iter()
        .copied()
        .find(|&i| staged_of(board, cards[i]).active)
        .or_else(|| (struck.len() == 1).then(|| struck[0]))?;
    Some((cards[i], i))
}

/// The reactions on offer to the struck defender — **each carrying what it will cost and do**, and, when it
/// cannot be taken, *why not*.
///
/// This is the answer to the question that started all this: looking at the screen, you could not tell whether
/// Strike Back was legitimately on offer, because nothing said what had hit you. A barred option is therefore
/// still listed, with its reason ("the blow was ranged - nothing to answer"), rather than silently vanishing:
/// an absent option teaches nothing and reads as a bug.
/// What a blow of `might` really does to `target`, in words - for a choice card's consequence line.
///
/// **A Might is not a health count.** It banks into the target's damage pile and only turns a Health card each
/// time that pile crosses Toughness; whatever is left **closes at the Lull**, the round boundary. So "deal 7
/// back" against a Toughness 9 Wall promises damage it cannot deliver. Say what the pile does with it instead -
/// including that the wound *keeps* for the rest of the round, which is what makes a blow under the bar worth
/// striking at all.
fn damage_phrase(target: &combat::Combatant, might: u32) -> String {
    let (flips, pile, bar) = combat::pile_effect(target, might);
    let name = &target.name;
    if flips > 0 {
        format!("{might} damage: {name} loses {flips} health")
    } else if pile == 0 {
        format!("{might} damage: {name}'s armor stops it")
    } else {
        format!("{might} into {name}'s pile: {pile}/{bar} - it keeps until the Lull")
    }
}

/// What taking a choice card does to the staged plan. The card says it in words; this is the same thing in
/// data. Every decision in a fight is one of these - **a tap on the table never decides anything**, it only
/// says *which* hero or foe we are talking about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChoiceAction {
    /// React: how the struck defender answers.
    React(ReactKind),
    /// Strike: aim the selected hero at this foe's card (seeding the minimum bid that can land).
    Aim(CardId),
    /// Strike: drop the aim and pick a target again.
    Unaim,
    /// Strike / Extra: spend this many tempo cards.
    Bid(u32),
    /// Strike / Extra: this hero deliberately does nothing here. **Not** the same as having decided nothing.
    Hold,
}

/// **Every decision on offer right now, as cards** - each carrying what it costs and does, and, when it cannot
/// be taken, why not. This is the whole decision surface of a fight: React's answers, Strike's targets and
/// bids, Extra's follow-up strikes.
///
/// The rule it encodes: *a tap on a card says **which**; a choice card says **what***. Tapping used to decide
/// things by cycling in place (rank, bid, reaction), which could name an option but never its consequence -
/// and silently skipped the ones you could not afford, so a missing option was indistinguishable from a bug.
pub(crate) fn step_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (_, _, _, _, step) = arena_state(board, arena);
    match step {
        Step::Marshal => Vec::new(),
        Step::React => react_choices_inner(board, arena),
        Step::Strike => strike_choices(board, arena),
        Step::Extra => extra_choices(board, arena),
    }
}

/// The choice cards for the current step, for the renderer.
pub fn scene_choices(board: &Board, arena: PileId) -> Vec<Choice> {
    step_choices(board, arena)
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

/// Strike: the selected hero's orders. With no aim yet these are the **targets** (one card each, quoting what
/// the blow would do to that foe's pile); once aimed they become the **bid** (how much tempo to commit, and
/// what the target can still slip).
fn strike_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (cards, units, sub, _round, _step) = arena_state(board, arena);
    let Some(i) = active_party(board, &cards, &units) else {
        return Vec::new();
    };
    let u = &units[i];
    let s = staged_of(board, cards[i]);
    let mut out = Vec::new();

    match s.aim.and_then(|a| cards.iter().position(|&c| c == a)) {
        // Aimed: choose the bid. `bid = cards x Finesse`, and the target slips it if that value fails to beat
        // its own Finesse - so quote the value, not just the count.
        Some(t) => {
            let floor = min_to_land(u, &units[t]);
            for n in 1..=u.tempo {
                let value = n * u.finesse.max(1);
                let text = if n < floor {
                    format!(
                        "value {value} - {} slips it on Finesse {}",
                        units[t].name, units[t].finesse
                    )
                } else {
                    format!(
                        "value {value} - lands; {}",
                        damage_phrase(&units[t], u.might)
                    )
                };
                let c = Choice::new(format!("Bid {n} tempo"), text).chosen(s.bid == n);
                out.push((
                    if n < floor {
                        c.barred("too light to land")
                    } else {
                        c
                    },
                    ChoiceAction::Bid(n),
                ));
            }
            out.push((
                Choice::new("Aim elsewhere", "pick another target"),
                ChoiceAction::Unaim,
            ));
        }
        // Not aimed: choose the target.
        None => {
            for (j, foe) in units.iter().enumerate() {
                if foe.side != Side::Foe
                    || foe.fallen
                    || !combat::legal_strike(sub, u.rank, foe.rank)
                    || !combat::back_access_ok(&units, u.rank, j)
                {
                    continue;
                }
                out.push((
                    Choice::new(format!("Strike {}", foe.name), damage_phrase(foe, u.might)),
                    ChoiceAction::Aim(cards[j]),
                ));
            }
            let hold = Choice::new("Hold", format!("keep {} tempo for a later phase", u.tempo))
                .chosen(s.hold);
            out.push((hold, ChoiceAction::Hold));
        }
    }
    out
}

/// Extra strikes: the selected hero presses a blow it already landed. Each card is Might again, into the same
/// pile - which is exactly when striking under the bar pays off.
fn extra_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (cards, units, _sub, _round, _step) = arena_state(board, arena);
    let Some(i) = active_party(board, &cards, &units) else {
        return Vec::new();
    };
    let contacts = read_contacts(board, arena, &cards);
    let Some(t) = contacts.iter().find(|c| c.attacker == i).map(|c| c.target) else {
        return Vec::new();
    };
    let u = &units[i];
    let s = staged_of(board, cards[i]);
    let mut out = Vec::new();
    for n in 1..=u.tempo {
        let (flips, pile, bar) = combat::pile_effect_strikes(&units[t], u.might, n);
        let text = if flips > 0 {
            format!("{} loses {flips} health", units[t].name)
        } else {
            format!(
                "{pile}/{bar} in {}'s pile - not enough to crack",
                units[t].name
            )
        };
        out.push((
            Choice::new(format!("Strike {n} more"), text).chosen(s.bid == n),
            ChoiceAction::Bid(n),
        ));
    }
    out.push((
        Choice::new("Hold", format!("keep {} tempo for a later phase", u.tempo)).chosen(s.hold),
        ChoiceAction::Hold,
    ));
    out
}

/// The party unit the player has selected (by tapping its card), if any.
fn active_party(board: &Board, cards: &[CardId], units: &[Combatant]) -> Option<usize> {
    (0..units.len()).find(|&i| {
        units[i].side == Side::Party && !units[i].fallen && staged_of(board, cards[i]).active
    })
}

fn react_choices_inner(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let Some((card, i)) = struck_defender(board, arena) else {
        return Vec::new();
    };
    let (cards, units, _sub, _round, _step) = arena_state(board, arena);
    let contacts = read_contacts(board, arena, &cards);
    let u = &units[i];
    let incoming: Vec<&Contact> = contacts.iter().filter(|c| c.target == i).collect();
    // Nothing is pre-chosen: an unanswered blow shows every card unlit, and Commit stays barred until the
    // player says which way this hero answers. A default here would decide for them and read as their choice.
    let staged = staged_of(board, card).react;
    let (evade_ok, strikeback_ok) = react_options(&units, &contacts, i);

    // Eat: every incoming blow lands, at its attacker's Might.
    let might: u32 = incoming.iter().map(|c| units[c.attacker].might).sum();
    let eat = Choice::new("Eat", format!("take {}", damage_phrase(u, might)))
        .chosen(staged == Some(ReactKind::Eat));

    // Evade: beat the bid. The cheapest incoming blow is the one worth quoting.
    let need = incoming
        .iter()
        .map(|c| c.bid / u.finesse.max(1) + 1)
        .min()
        .unwrap_or(0);
    let evade = Choice::new("Evade", format!("spend {need} tempo to slip it"))
        .chosen(staged == Some(ReactKind::Evade));
    let evade = if evade_ok {
        evade
    } else if u.tempo == 0 {
        evade.barred("no tempo left")
    } else {
        evade.barred(format!("needs {need} tempo, you have {}", u.tempo))
    };

    // Strike Back: melee-vs-melee only - you answer a foe that *approached* you.
    let counter = incoming
        .iter()
        .find(|c| !combat::rank_is_ranged(units[c.attacker].rank))
        .map(|c| &units[c.attacker]);
    let back_text = match counter {
        Some(foe) => format!("spend 1 tempo, deal {}", damage_phrase(foe, u.might)),
        None => format!("spend 1 tempo, deal {} back", u.might),
    };
    let back = Choice::new("Strike Back", back_text).chosen(staged == Some(ReactKind::StrikeBack));
    let back = if strikeback_ok {
        back
    } else if u.tempo == 0 {
        back.barred("no tempo left")
    } else if !u.melee {
        back.barred("you carry no melee blow")
    } else {
        back.barred("the blow was ranged - nothing to answer")
    };

    vec![
        (eat, ChoiceAction::React(ReactKind::Eat)),
        (evade, ChoiceAction::React(ReactKind::Evade)),
        (back, ChoiceAction::React(ReactKind::StrikeBack)),
    ]
}

/// Take the choice card at `index` (into [`step_choices`]). A barred option does nothing.
pub fn choose(board: &mut Board, index: usize) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    let Some((choice, action)) = step_choices(board, arena).into_iter().nth(index) else {
        return;
    };
    if !choice.enabled() {
        return;
    }
    let (cards, units, _sub, _round, step) = arena_state(board, arena);
    let card = match step {
        Step::React => struck_defender(board, arena).map(|(c, _)| c),
        _ => active_party(board, &cards, &units).map(|i| cards[i]),
    };
    let Some(card) = card else {
        return;
    };
    let mut s = staged_of(board, card);
    match action {
        ChoiceAction::React(kind) => s.react = Some(kind),
        ChoiceAction::Aim(foe) => {
            // Seed the lightest bid that can actually land, so the aim is never a no-op the player has to
            // discover; they raise it from the bid cards.
            let (Some(a), Some(t)) = (
                cards.iter().position(|&c| c == card),
                cards.iter().position(|&c| c == foe),
            ) else {
                return;
            };
            s.aim = Some(foe);
            s.bid = min_to_land(&units[a], &units[t]).min(units[a].tempo).max(1);
            s.hold = false;
        }
        ChoiceAction::Unaim => {
            s.aim = None;
            s.bid = 0;
        }
        ChoiceAction::Bid(n) => {
            s.bid = n;
            s.hold = false;
        }
        ChoiceAction::Hold => {
            s.hold = true;
            s.aim = None;
            s.bid = 0;
        }
    }
    write_staged(board, card, &s);
}

/// Handle a tap on combatant `card`: in Marshal, cycle its rank pile (the no-drag assignment); in a combat
/// step, edit the staged plan (a no-op if the tap is meaningless there). Reads the step from the phase card.
pub fn handle_tap(board: &mut Board, card: CardId) {
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
    // **A tap says which, never what.** It selects the hero we are giving orders to, or (as a shortcut) the
    // foe we mean; the order itself is always taken from a choice card, which says what it costs and does.
    // Tapping used to cycle a bid, a rank or a reaction in place - naming an option but never its consequence.
    match step {
        Step::Marshal => unreachable!("handled above"),
        Step::Strike => match side {
            Side::Party => select_active(board, &cards, &units, i),
            Side::Foe => aim_active(board, &cards, &units, sub, i),
        },
        Step::React => {
            let contacts = read_contacts(board, arena, &cards);
            if side == Side::Party && contacts.iter().any(|c| c.target == i) {
                select_defender(board, &cards, i);
            }
        }
        Step::Extra => {
            let outgoing = read_contacts(board, arena, &cards)
                .iter()
                .any(|c| c.attacker == i);
            if side == Side::Party && outgoing {
                select_active(board, &cards, &units, i);
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
pub fn find_arena(board: &Board) -> Option<PileId> {
    top_deck(board, ARENA)
}

/// The place a fight was opened from, remembered in the hidden meta card.
fn place_of(board: &Board, arena: PileId) -> Option<PileId> {
    board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("arena-meta"))
        .find_map(|c| board.card(c).map(|k| k.front_title().to_string()))
        .map(|s| PileId(num_after(&s, "place ") as u64))
}

/// Every combatant card in the arena (across the Pool and all rank piles), by side-type.
fn all_of_type(board: &Board, arena: PileId, card_type: &str) -> Vec<CardId> {
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
fn teardown(board: &mut Board, arena: PileId, clear_encounter: bool, spend_day: bool) {
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
pub fn fold_back(board: &mut Board, arena: PileId) {
    let won = outcome(board, arena) == Some(Outcome::Victory);
    teardown(board, arena, won, true);
}

/// **Cancel the fight** (retreat): tear the arena down with nothing resolved — the encounter is left intact
/// and no day is spent. The clean inverse of [`open_fight`], for backing out of a battle.
pub fn cancel_fight(board: &mut Board, arena: PileId) {
    teardown(board, arena, false, false);
}

/// **Restart the fight**: reset every combatant to full HP and fresh tempo, drop the staged plans and any
/// landed contacts, and return to round 1 · Marshal — **keeping the current formation** (rank-pile
/// membership), so you can re-form or just Start again. Foes, place, and encounter are untouched.
pub fn restart_fight(board: &mut Board, arena: PileId) {
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
            if let Some((s, melee, ranged, aoe)) = stats {
                let _ = board.set_card_detail(
                    card,
                    detail(
                        s.vitality, s.vitality, s.cadence, s.cadence, s.finesse, melee, ranged,
                        aoe, 0,
                    ),
                );
            }
            // The foes go back into the muster: a restart returns to the moment *before* the Reveal, so their
            // formation must be off the table again while the player re-declares theirs.
            if ctype == "foe"
                && let Some(m) = sub_pile(board, arena, MUSTER)
            {
                let at = board.pile(m).map_or(0, |p| p.cards().len());
                let _ = board.move_card(card, m, at);
            }
        }
    }
    install_phase_decks(board, arena);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_table;

    /// **Nothing is decided by default, and Commit says what is missing.** The Raider is struck at Intercept
    /// and can Eat, Evade or Strike Back - so it is being *asked*. Committing before it answers would silently
    /// enter Eat as if the player had chosen it.
    #[test]
    fn commit_is_barred_until_a_struck_hero_answers_and_names_who() {
        let mut board = sample_table();
        // The Raider marches alone: it ranks Outrider, so The Wall (an enemy Vanguard) screens it at Intercept.
        let arena = open_a_fight_with(&mut board, "Raider");
        commit(&mut board, arena); // Marshal -> Intercept / Strike
        commit(&mut board, arena); // Strike -> React: The Wall's blow has landed on the Raider

        let (cards, units, _, _, step) = arena_state(&board, arena);
        assert_eq!(step, Step::React);
        let (card, i) = struck_defender(&board, arena).expect("a hero is under fire");
        assert!(
            staged_of(&board, card).react.is_none(),
            "nothing pre-chosen"
        );
        assert!(
            scene_choices(&board, arena).iter().all(|c| !c.chosen),
            "no option shows as taken before the player takes one"
        );

        let who = units[i].name.clone();
        assert_eq!(
            pending_decision(&board, arena),
            Some(format!("{who} has not answered"))
        );
        assert_eq!(
            commit_label(&board, arena),
            format!("{who} has not answered")
        );

        // Answer, and Commit comes live.
        let eat = scene_choices(&board, arena)
            .iter()
            .position(|c| c.label == "Eat")
            .expect("Eat is always offered");
        choose(&mut board, eat);
        assert_eq!(pending_decision(&board, arena), None);
        assert_eq!(commit_label(&board, arena), "Commit");
        let _ = cards;
    }

    /// **The damage pile spans the whole sub-phase.** A Raider (Might 7) that strikes The Wall (Toughness 9)
    /// and then presses with an Extra strike banks 7 + 7 = 14 into one pile, which crosses 9 and flips a Health
    /// card. That is the only way anything cracks a Wall, and it is the whole reason to strike under the bar.
    ///
    /// It did not work: `pending` was rebuilt as 0 on every read of a card, so the pile was wiped at each
    /// *step* boundary rather than the sub-phase boundary, and the two 7s never met. The cards are the state -
    /// anything that has to survive a step has to be written on one.
    #[test]
    fn a_catch_and_an_extra_bank_into_the_same_pile_and_crack_the_wall() {
        let mut board = sample_table();
        // The lone Wall - the fight the Raider is built to solo. (Against the corner it dies inside a round,
        // which would end the fight before the Lull we are here to test.)
        let arena = open_a_fight_at(&mut board, "Raider", Some("The Sundered Vault"));
        // Rank the Raider as a Vanguard, so it meets The Wall in the Clash (Vanguard -> Vanguard).
        let raider = pool_heroes(&board, arena)[0];
        let van = sub_pile(&board, arena, "Vanguard").unwrap();
        let _ = board.move_card(raider, van, 0);
        commit(&mut board, arena); // Start / Reveal

        let clash = 3;
        while arena_state(&board, arena).2 != clash {
            commit(&mut board, arena);
        }
        // Read The Wall off its card, not out of `arena_state`: at the Lull it has stepped back into the
        // muster, so it is no longer a ranked combatant - but it is the same card, wounds and all.
        let wall_card = {
            let (cards, units, _, _, _) = arena_state(&board, arena);
            cards[units.iter().position(|u| u.side == Side::Foe).unwrap()]
        };
        let wall = |b: &Board| read_combatant(b, wall_card, Rank::Vanguard).unwrap();
        let hp0 = wall(&board).health;

        // Strike: aim at The Wall and bid 1 tempo (the blow's damage is Might, never the bid).
        let me = |b: &Board| {
            let (cards, units, _, _, _) = arena_state(b, arena);
            (0..units.len())
                .find(|&i| units[i].side == Side::Party)
                .map(|i| cards[i])
                .unwrap()
        };
        let c = me(&board);
        handle_tap(&mut board, c);
        let strike = scene_choices(&board, arena)
            .iter()
            .position(|c| c.label.starts_with("Strike The Wall"))
            .expect("a Vanguard may strike the enemy Vanguard at Clash");
        choose(&mut board, strike);
        commit(&mut board, arena); // Strike -> React: contact is made (damage lands when the defender answers)

        // React: eat the counter-blow, keeping tempo for the Extra. The Wall soaks ours: 7 into its pile.
        if pending_decision(&board, arena).is_some() {
            let eat = scene_choices(&board, arena)
                .iter()
                .position(|c| c.label == "Eat")
                .unwrap();
            choose(&mut board, eat);
        }
        commit(&mut board, arena); // React -> Extra
        assert_eq!(
            wall(&board).pending,
            7,
            "the blow banked 7, and the pile survives the step boundary"
        );
        assert_eq!(wall(&board).health, hp0, "7 alone cannot crack Toughness 9");

        // Extra: press once more. 7 + 7 = 14 >= 9, and a Health card turns.
        let c = me(&board);
        handle_tap(&mut board, c);
        let more = scene_choices(&board, arena)
            .iter()
            .position(|c| c.label == "Strike 1 more")
            .expect("a surviving contact may be pressed");
        choose(&mut board, more);
        commit(&mut board, arena); // Extra -> the next sub-phase

        assert_eq!(
            wall(&board).health,
            hp0 - 1,
            "14 in one pile crosses Toughness 9 - The Wall loses a Health card"
        );
        // The 5 left over is a real, open wound: it carries into the next sub-phase, and only closes at the
        // Lull. (It used to be wiped here, which is what made a blow under the bar a blow into the void.)
        assert_eq!(
            wall(&board).pending,
            5,
            "the remainder carries across the sub-phase boundary"
        );

        // ...and the Lull closes it. Walk to the top of the next round.
        let mut guard = 0;
        while arena_state(&board, arena).4 != Step::Marshal && outcome(&board, arena).is_none() {
            commit(&mut board, arena);
            guard += 1;
            assert!(guard < 100, "the round must come back round to Marshal");
        }
        assert_eq!(
            wall(&board).pending,
            0,
            "the Lull closes every unfinished wound - the one deadline in a fight"
        );
    }

    /// **At Strike the choice row IS the target list, then the bid.** Tapping only says *which* hero; the order
    /// is always a card that states what it does. And Hold is a real order - a hero that could strike must say
    /// it is not striking, so "no aim" never has to mean two different things.
    #[test]
    fn catch_offers_targets_then_bids_and_hold_is_a_real_order() {
        let mut board = sample_table();
        let arena = open_a_fight_with(&mut board, "Bastion"); // a Vanguard, so it meets The Wall at Clash
        commit(&mut board, arena); // Marshal -> Intercept / Strike

        // Nothing selected: nothing to decide yet, and the tap is what selects.
        assert!(scene_choices(&board, arena).is_empty());
        let (cards, units, _, _, _) = arena_state(&board, arena);
        let hero = (0..units.len())
            .find(|&i| units[i].side == Side::Party)
            .expect("the Bastion marched");
        handle_tap(&mut board, cards[hero]);

        // Walk to Clash, where a Vanguard may strike the enemy Vanguard.
        while arena_state(&board, arena).2 != 3 {
            commit(&mut board, arena);
        }
        handle_tap(&mut board, cards[hero]);
        let labels = |b: &Board| -> Vec<String> {
            scene_choices(b, arena)
                .iter()
                .map(|c| c.label.clone())
                .collect()
        };
        assert_eq!(labels(&board), vec!["Strike The Wall", "Hold"]);

        // Aim: the row becomes the bid, and the hero no longer owes an order.
        assert!(
            pending_decision(&board, arena).is_some(),
            "unaimed hero owes an order"
        );
        choose(&mut board, 0);
        assert!(labels(&board).iter().any(|l| l.starts_with("Bid ")));
        assert!(labels(&board).contains(&"Aim elsewhere".to_string()));
        assert_eq!(pending_decision(&board, arena), None);

        // Hold is equally an answer: it clears the aim, and Commit still comes live.
        let hold = labels(&board)
            .iter()
            .position(|l| l == "Aim elsewhere")
            .unwrap();
        choose(&mut board, hold); // back to the target list
        let hold = labels(&board).iter().position(|l| l == "Hold").unwrap();
        choose(&mut board, hold);
        assert!(staged_of(&board, cards[hero]).hold);
        assert_eq!(
            pending_decision(&board, arena),
            None,
            "holding IS deciding - it must not read as undecided"
        );
    }

    /// **A choice must not promise damage it cannot deliver.** The Raider (Might 7) striking back at The Wall
    /// (Toughness 9) used to read "deal 7 back" - and it flipped nothing: 7 banks into the Wall's pile and
    /// never crosses 9 on its own. The card has to say where the blow actually goes, and how long it keeps
    /// (until the Lull), because that is the whole reason to strike under the bar.
    #[test]
    fn a_blow_under_the_bar_is_quoted_against_the_pile_not_as_health() {
        let wall = combat::Combatant::from_stats(
            "The Wall",
            Side::Foe,
            Rank::Vanguard,
            [1, 4, 9, 1, 2],
            0,
            true,
            false,
        );
        assert_eq!(
            damage_phrase(&wall, 7),
            "7 into The Wall's pile: 7/9 - it keeps until the Lull"
        );

        // Land another 2 in the same sub-phase and the pile crosses: now the promise is real.
        let mut wall = wall;
        wall.pending = 7;
        assert_eq!(damage_phrase(&wall, 2), "2 damage: The Wall loses 1 health");
    }

    /// A melee kit (the Raider carries Jab) must flag `Melee` (not `no strike`) on its combat card:
    /// `hero_stats` reads the reach off the ability, and `detail` writes the token the renderer parses. Guards
    /// the "hero shows no strike" regression - which can only occur if a card carries a stale, pre-reach
    /// detail line (a fight persisted by an older build), never from this live path.
    #[test]
    fn a_melee_kit_flags_melee_on_its_combat_card() {
        // The Raider starts in the party (a hero is its kit), so its build is already assembled.
        let board = sample_table();
        let (stats, melee, ranged, aoe) = hero_stats(&board, "Raider").expect("recipe resolves");
        assert!(melee && !ranged, "Jab is melee-only");
        assert!(!aoe, "Jab is single-target");
        let d = detail(
            stats.vitality,
            stats.vitality,
            stats.cadence,
            stats.cadence,
            stats.finesse,
            melee,
            ranged,
            aoe,
            0,
        );
        assert!(
            d[2].contains("Melee"),
            "the combat card carries the Melee token: {:?}",
            d[2]
        );
        assert!(!d[2].contains("Ranged") && !d[2].contains("Area"));
    }

    /// Set up a fight at a place with an encounter, with the Marksman marched there.
    fn open_a_fight(board: &mut Board) -> PileId {
        open_a_fight_with(board, "Marksman")
    }

    /// The same fight setup, reachable from the sibling `choice_tests` module.
    pub(super) fn open_a_fight_for_tests(board: &mut Board) -> PileId {
        open_a_fight(board)
    }

    /// As [`open_a_fight`], but march the hero of the named kit (so tests can pick the hero's attack type).
    /// The party starts assembled and stationed at Ashfen — a hero *is* its kit — so there is nothing to
    /// recruit: just walk the one we want out to the encounter, leaving the rest at home.
    fn open_a_fight_with(board: &mut Board, kit_name: &str) -> PileId {
        open_a_fight_at(board, kit_name, None)
    }

    /// March one kit's hero to `place_name` (or the first place with an encounter) and open the fight. Naming
    /// the place matters: the first encounter cell is a **corner**, which fields four creatures and kills any
    /// lone hero inside a round - useless for testing anything that has to survive to round 2.
    fn open_a_fight_at(board: &mut Board, kit_name: &str, place_name: Option<&str>) -> PileId {
        let locations = top_deck(board, "Locations").unwrap();
        let ashfen = board.pile(locations).unwrap().subpiles()[4];
        let place = board
            .pile(locations)
            .unwrap()
            .subpiles()
            .into_iter()
            .find(|&p| {
                let named = place_name
                    .is_none_or(|want| board.pile(p).map(|k| k.label.as_str()) == Some(want));
                named
                    && board
                        .content_cards(p)
                        .iter()
                        .any(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
            })
            .unwrap();
        // This kit's hero map-position card, standing at the home cell.
        let position = board
            .content_cards(ashfen)
            .into_iter()
            .find(|&c| {
                board.card(c).map(|k| (k.card_type(), k.front_title())) == Some(("hero", kit_name))
            })
            .unwrap_or_else(|| panic!("{kit_name} is stationed at Ashfen"));
        let progress = top_deck(board, "Progress").unwrap();
        let _ = board.move_character(position, place, progress);
        open_fight(board, place).expect("a fight opens")
    }

    /// **You may read the foes; you may not read their formation.** Marshal is a blind bet - the secrecy of the
    /// two declarations is what simulates simultaneity - so the enemy stands in the muster, fully legible, and
    /// only takes its ranks at the Reveal, when the player commits. The formation cannot leak because it is not
    /// on the table yet: a stronger guarantee than a renderer that merely declines to draw it.
    #[test]
    fn foes_muster_unranked_at_marshal_and_take_the_field_at_reveal() {
        let mut board = sample_table();
        // The Raider against the lone Wall: the fight it is designed to solo, and it takes several rounds.
        let arena = open_a_fight_at(&mut board, "Raider", Some("The Sundered Vault"));
        assert_eq!(current_step(&board, arena), Step::Marshal);
        assert!(
            !formation_complete(&board, arena),
            "heroes start unranked in the Pool"
        );

        // On the table and readable, but in no rank - so `arena_state` (which reads the ranks) sees no foe.
        assert!(
            !muster_foes(&board, arena).is_empty(),
            "the foes are present"
        );
        let (_, units, _, _, _) = arena_state(&board, arena);
        assert!(
            units.is_empty(),
            "nobody is in formation yet - neither side"
        );
        // ...and the fight must not read as already won just because the enemy has not stepped forward.
        assert_eq!(outcome(&board, arena), None);

        commit(&mut board, arena); // Start: the Reveal - both formations go down at once
        assert!(muster_foes(&board, arena).is_empty(), "the muster empties");
        let (_, units, _, _, _) = arena_state(&board, arena);
        assert!(units.iter().any(|u| u.side == Side::Foe), "foes are ranked");
        assert!(units.iter().any(|u| u.side == Side::Party), "so are we");

        // **Every round is a fresh blind bet.** Intentions are re-declared each round, so at the next Marshal
        // the surviving foes have stepped back out of formation - you can see who is left, but not where they
        // will stand. Without this the enemy simply vanished from every Marshal after the first.
        let mut guard = 0;
        while arena_state(&board, arena).4 != Step::Marshal && outcome(&board, arena).is_none() {
            commit(&mut board, arena);
            guard += 1;
            assert!(guard < 100, "the round must come back round to Marshal");
        }
        assert_eq!(arena_state(&board, arena).3, 2, "round 2");
        assert!(
            !muster_foes(&board, arena).is_empty(),
            "the survivors are back in the muster, readable and unranked"
        );
    }

    /// The `read_combatant` plumbing carries area / horde from the source: a Bastion (Sweep) hero flags `aoe`,
    /// and every foe faithfully mirrors its catalog `aoe`/`horde` (guards against a hardcoded-false regress).
    #[test]
    fn combat_reads_carry_aoe_for_a_sweep_hero_and_horde_for_foes() {
        let mut board = sample_table();
        let arena = open_a_fight_with(&mut board, "Bastion"); // the Bastion carries Sweep, an area attack
        let bastion = pool_heroes(&board, arena)
            .into_iter()
            .find(|&c| board.card(c).map(|k| k.front_title()) == Some("Bastion"))
            .expect("the Bastion is in the pool");
        let van = sub_pile(&board, arena, "Vanguard").unwrap();
        assign(&mut board, bastion, van);

        let (_, units, _, _, _) = arena_state(&board, arena);
        let sweep = units
            .iter()
            .find(|u| u.name == "Bastion")
            .expect("the Sweep hero is ranked");
        assert!(
            sweep.aoe,
            "a Bastion (Sweep) is an area attacker on its combat read"
        );
        assert!(!sweep.horde, "a hero is never a horde");
        for u in units.iter().filter(|u| u.side == Side::Foe) {
            let c =
                deckbound_content::catalog::creature(&u.name).expect("foe is a catalog creature");
            assert_eq!(u.horde, c.horde, "{} horde flag", u.name);
            assert_eq!(u.aoe, c.aoe, "{} aoe flag", u.name);
        }
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

    /// The five-round cap is the fight's clock: two lines that cannot break each other part as a draw. Without
    /// it a stalemate walks forever, and the batch resolver (which does cap) would disagree with the arena.
    #[test]
    fn the_round_cap_ends_an_unbroken_fight_in_a_draw() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        commit(&mut board, arena); // leave Marshal; both lines are intact and standing
        assert_eq!(outcome(&board, arena), None, "the fight is still live");

        set_round(&mut board, arena, crate::battle::MAX_ROUNDS as u32 + 1);
        assert_eq!(
            outcome(&board, arena),
            Some(Outcome::Draw),
            "past the cap with both lines standing"
        );
        assert_eq!(commit_label(&board, arena), "Draw - leave");
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
        commit(&mut board, arena); // Marshal -> Strike (auto-ranks the hero)
        assert_eq!(current_step(&board, arena), Step::Strike);

        let (cards, units, sub, _, _) = arena_state(&board, arena);
        let pi = units.iter().position(|u| u.side == Side::Party).unwrap();
        if let Some(fi) = (0..units.len()).find(|&j| {
            units[j].side == Side::Foe && combat::legal_strike(sub, units[pi].rank, units[j].rank)
        }) {
            handle_tap(&mut board, cards[pi]); // select active
            assert!(staged_of(&board, cards[pi]).active);
            handle_tap(&mut board, cards[fi]); // aim at the foe (seeds a bid)
            let staged = staged_of(&board, cards[pi]);
            assert_eq!(staged.aim, Some(cards[fi]));
            assert!(staged.bid >= 1, "aiming seeds at least one card of bid");

            commit(&mut board, arena); // resolve Strike
            assert_eq!(current_step(&board, arena), Step::React);
            let contacts = read_contacts(&board, arena, &arena_state(&board, arena).0);
            assert!(
                contacts.iter().any(|c| c.attacker == pi),
                "the staged strike landed as a persisted contact"
            );
        }
    }
}

#[cfg(test)]
mod choice_tests {
    use super::*;
    use crate::sample_table;

    /// Drive a fight to React with the Marksman (ranged) struck by... nothing yet - we build the state we need
    /// directly, because what is under test is what the *choices say*, not how the fight got here.
    fn defender_choices(defender: Combatant, attacker: Combatant, bid: u32) -> Vec<Choice> {
        // A tiny stand-in for the arena read: react_choices' logic is react_options + the consequence text, so
        // exercise them together through the same predicate the UI uses.
        let units = vec![defender, attacker];
        let contacts = vec![Contact {
            attacker: 1,
            target: 0,
            bid,
        }];
        let (evade_ok, strikeback_ok) = react_options(&units, &contacts, 0);
        let u = &units[0];
        let might: u32 = units[1].might;
        let need = bid / u.finesse.max(1) + 1;
        let eat = Choice::new("Eat", format!("take the hit ({might} might)"));
        let evade = Choice::new("Evade", format!("spend {need} tempo to slip it"));
        let evade = if evade_ok {
            evade
        } else if u.tempo == 0 {
            evade.barred("no tempo left")
        } else {
            evade.barred(format!("needs {need} tempo, you have {}", u.tempo))
        };
        let back = Choice::new(
            "Strike Back",
            format!("spend 1 tempo, deal {} back", u.might),
        );
        let back = if strikeback_ok {
            back
        } else if u.tempo == 0 {
            back.barred("no tempo left")
        } else if !u.melee {
            back.barred("you carry no melee blow")
        } else {
            back.barred("the blow was ranged - nothing to answer")
        };
        vec![eat, evade, back]
    }

    fn combatant(name: &str, side: Side, rank: Rank, melee: bool, ranged: bool) -> Combatant {
        let mut c = Combatant::from_stats(name, side, rank, [3, 5, 1, 2, 2], 0, melee, ranged);
        c.tempo = 2;
        c
    }

    /// **The question that started this.** A ranged blow cannot be answered — you strike back at a foe that
    /// *approached* you. The option must still be shown, saying so, rather than silently vanishing: an absent
    /// option teaches nothing and is indistinguishable from a bug.
    #[test]
    fn strike_back_is_barred_against_a_ranged_blow_and_says_why() {
        let hero = combatant("hero", Side::Party, Rank::Outrider, true, false); // carries melee
        let shooter = combatant("shooter", Side::Foe, Rank::Rearguard, false, true); // fires from the back
        let choices = defender_choices(hero, shooter, 2);

        let back = &choices[2];
        assert_eq!(back.label, "Strike Back");
        assert!(!back.enabled(), "a ranged shot cannot be struck back at");
        assert_eq!(back.why_not, "the blow was ranged - nothing to answer");
    }

    /// The Wall's case: a melee Vanguard intercepting the crossing Outrider. Strike Back **is** legitimate,
    /// and the card says what it will do.
    #[test]
    fn strike_back_is_offered_against_a_melee_blow_with_its_consequence() {
        let hero = combatant("Raider", Side::Party, Rank::Outrider, true, false);
        let wall = combatant("The Wall", Side::Foe, Rank::Vanguard, true, false);
        let choices = defender_choices(hero, wall, 2);

        let back = &choices[2];
        assert!(back.enabled(), "a melee blow can be answered");
        assert_eq!(back.consequence, "spend 1 tempo, deal 3 back");

        // ...and eating it says what it costs, rather than just naming the action.
        assert_eq!(choices[0].consequence, "take the hit (3 might)");
    }

    /// A spent unit cannot buy its way out: both paid options are barred, and both say so.
    #[test]
    fn a_unit_with_no_tempo_is_told_why_it_can_only_eat() {
        let mut hero = combatant("hero", Side::Party, Rank::Vanguard, true, false);
        hero.tempo = 0;
        let wall = combatant("The Wall", Side::Foe, Rank::Vanguard, true, false);
        let choices = defender_choices(hero, wall, 2);

        assert!(choices[0].enabled(), "Eat is always free");
        assert!(!choices[1].enabled());
        assert_eq!(choices[1].why_not, "no tempo left");
        assert!(!choices[2].enabled());
        assert_eq!(choices[2].why_not, "no tempo left");
    }

    /// The React choices exist only at React, and only for a hero who was actually hit. A fresh fight opens at
    /// Marshal, where nothing has struck anyone, so the game asks for no decision — the choice row is empty
    /// rather than showing three inert cards.
    #[test]
    fn no_decision_is_asked_for_when_nobody_is_under_fire() {
        let mut board = sample_table();
        let arena = super::tests::open_a_fight_for_tests(&mut board);
        assert_eq!(current_step(&board, arena), Step::Marshal);
        assert!(
            scene_choices(&board, arena).is_empty(),
            "at Marshal nothing has struck anyone - there is no reaction to choose"
        );
    }
}
