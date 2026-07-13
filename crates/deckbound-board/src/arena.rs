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
use crate::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};

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
    /// Attackers commit tempo to **reach** a target. More tempo makes them harder to slip; it buys no damage.
    Engage,
    /// Each target - seeing exactly what was committed - pays the exact price to slip, or stands.
    Evade,
    /// Each engager's one free opening blow, then either end of a melee contact pours in tempo, 1 card = 1 hit.
    Strike,
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

/// One party unit's staged orders for the current mini-phase (read from / written to its detail).
///
/// `hold` is the **explicit** "this hero does nothing here" - distinct from having decided nothing yet. Both
/// produce no strike, but only one of them means the player has answered, and Commit gates on the difference.
///
/// `aim`/`bid` are the Engage commitment; `dodge` is the Evade answer; at Strike, `bid` is the number of blows.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Staged {
    pub(crate) active: bool,
    pub(crate) aim: Option<CardId>,
    pub(crate) bid: u32,
    pub(crate) hold: bool,
    pub(crate) dodge: Option<Dodge>,
}

fn dodge_label(d: Dodge) -> &'static str {
    match d {
        Dodge::Slip => "Slip",
        Dodge::Stand => "Stand",
    }
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
        } else if let Some(k) = line.strip_prefix("dodge ") {
            s.dodge = Some(if k == "Slip" {
                Dodge::Slip
            } else {
                Dodge::Stand
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
    if let Some(d) = s.dodge {
        lines.push(format!("dodge {}", dodge_label(d)));
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
                Some("Evade") => Step::Evade,
                Some("Strike") => Step::Strike,
                _ => Step::Engage,
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
        for name in ["Engage", "Evade", "Strike"] {
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

/// The party's staged engagements (aim + committed tempo), keeping only ones the SCHEDULE permits.
fn party_engagements(
    board: &Board,
    cards: &[CardId],
    units: &[Combatant],
    sub: usize,
) -> Vec<Engage> {
    let mut out = Vec::new();
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
            out.push(Engage {
                attacker: i,
                target: t,
                cards: s.bid,
            });
        }
    }
    out
}

/// The party's staged blows: tempo poured into an established contact, by either end of a melee edge.
fn party_blows(
    board: &Board,
    cards: &[CardId],
    units: &[Combatant],
    contacts: &[Contact],
) -> Vec<Blows> {
    (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen)
        .filter_map(|i| {
            let bid = staged_of(board, cards[i]).bid;
            let target = combat::strike_target(units, contacts, i)?;
            (bid > 0).then_some(Blows {
                unit: i,
                target,
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
            Step::Evade => format!("{} has not answered", u.name),
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
        // Reach alone is not enough: the schedule must actually pair this rank against a living, reachable
        // enemy rank *this sub-phase*. Omitting that was the bug - an Outrider at the Clash looked ready.
        Step::Engage => {
            combat::effective_in_rank(u.rank, u.melee, u.ranged)
                && units.iter().enumerate().any(|(j, v)| {
                    v.side == Side::Foe
                        && !v.fallen
                        && combat::legal_strike(sub, u.rank, v.rank)
                        && combat::back_access_ok(units, u.rank, j)
                })
        }
        // You are only asked to answer if something is reaching you AND you could actually afford to escape it.
        // A slip you cannot pay for is not a choice you are being offered.
        Step::Evade => combat::slip_cost(units, contacts, i).is_some_and(|cost| cost <= u.tempo),
        // You may pour tempo into any edge you are on - the one you opened, or a melee one opened on you.
        Step::Strike => combat::strike_target(units, contacts, i).is_some(),
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
        Step::Engage => !s.hold && s.aim.is_none(),
        Step::Evade => s.dodge.is_none(),
        Step::Strike => !s.hold && s.bid == 0,
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
    if step == Step::Marshal {
        return true; // assign ranks / Start
    }
    // The same authority the board and the Commit gate read: if nothing is being asked of any hero, there is
    // nothing here for the player to do, and the step can be resolved greedily and skipped.
    let contacts = read_contacts(board, arena, &cards);
    (0..units.len()).any(|i| can_act(&units, &contacts, sub, step, i))
}

/// A compact note for the current step when it is being auto-skipped: `"{Phase}|{Step}|{why}|{pairs}"`
/// (pipe-encoded so the renderer can group by sub-phase). `pairs` is the sub-phase's `attacker>target` rank
/// codes (e.g. `"V>O"`), so the renderer can show *which targeting* was passed over without knowing the
/// SCHEDULE itself. ASCII - it is stored on a card (so it shows in the physical log too).
pub fn current_skip_line(board: &Board, arena: PileId) -> String {
    let (_, _, sub, _, step) = arena_state(board, arena);
    let phase = SUB_PHASE_NAMES.get(sub).copied().unwrap_or("?");
    let (name, reason) = match step {
        Step::Engage => ("Engage", "no legal target"),
        Step::Evade => ("Evade", "nothing reaching you"),
        Step::Strike => ("Strike", "no contact to strike along"),
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

        Step::Engage => {
            let mut engagements = party_engagements(board, &cards, &units, sub);
            // The greedy fallback is for headless play (tests, the solver), where nobody gave orders. It must
            // NOT fire when the player deliberately Held: an empty list is then their decision, not an absent
            // one.
            if engagements.is_empty() && !party_has_orders(board, &cards, &units) {
                engagements = Greedy.engagements(&units, Side::Party, sub);
            }
            engagements.extend(Greedy.engagements(&units, Side::Foe, sub));
            let reaching = combat::resolve_engage(&mut units, &engagements);
            writeback(board, &units); // clears the staged engagement plan
            write_contacts(board, arena, &cards, &reaching);
            rotate_deck(board, arena, STEPS); // Engage -> Evade
        }

        Step::Evade => {
            let reaching = read_contacts(board, arena, &cards);
            let dodges: Vec<Dodge> = (0..units.len())
                .map(|i| {
                    if units[i].side != Side::Party {
                        return Greedy.dodge(&units, &reaching, i);
                    }
                    match staged_of(board, cards[i]).dodge {
                        Some(d) => d,
                        None => Dodge::Stand, // nothing was asked, or nothing was answered: you stand
                    }
                })
                .collect();
            let contacts = combat::resolve_evade(&mut units, &reaching, &dodges);
            writeback(board, &units); // clears the staged dodges
            clear_contacts(board, arena);
            write_contacts(board, arena, &cards, &contacts);
            rotate_deck(board, arena, STEPS); // Evade -> Strike
        }

        Step::Strike => {
            let contacts = read_contacts(board, arena, &cards);
            let mut blows = party_blows(board, &cards, &units, &contacts);
            if blows.is_empty() && !party_has_orders(board, &cards, &units) {
                blows = Greedy.blows(&units, Side::Party, &contacts);
            }
            blows.extend(Greedy.blows(&units, Side::Foe, &contacts));
            combat::resolve_strike(&mut units, &contacts, &blows);
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

/// The party unit a step's choice cards are *about*: the one the player selected, or - when only one hero is
/// being asked anything - that one, so a lone decision needs no selecting first.
pub(crate) fn focused_party(board: &Board, arena: PileId) -> Option<(CardId, usize)> {
    let (cards, units, sub, _round, step) = arena_state(board, arena);
    if step == Step::Marshal {
        return None;
    }
    let contacts = read_contacts(board, arena, &cards);
    let asked: Vec<usize> = (0..units.len())
        .filter(|&i| can_act(&units, &contacts, sub, step, i))
        .collect();
    let i = asked
        .iter()
        .copied()
        .find(|&i| staged_of(board, cards[i]).active)
        .or_else(|| (asked.len() == 1).then(|| asked[0]))?;
    Some((cards[i], i))
}

/// What `n` blows of `might` really do to `target`, in words - for a choice card's consequence line.
///
/// **A Might is not a health count.** It banks into the target's damage pile and only turns a Health card each
/// time that pile crosses Toughness; whatever is left **closes at the Reset**, the round boundary. So "deal 7
/// back" against a Toughness 9 Wall promises damage it cannot deliver. Say what the pile does with it instead -
/// including that the wound *keeps* for the rest of the round, which is what makes a blow under the bar worth
/// striking at all.
fn blows_phrase(target: &combat::Combatant, might: u32, n: u32) -> String {
    let (flips, pile, bar) = combat::pile_effect_strikes(target, might, n);
    let name = &target.name;
    let total = might * n;
    if flips > 0 {
        format!("{total} damage: {name} loses {flips} health")
    } else if pile == 0 {
        format!("{total} damage: {name}'s armor stops it")
    } else {
        format!("{total} into {name}'s pile: {pile}/{bar} - it keeps until the Reset")
    }
}

/// What taking a choice card does to the staged plan. The card says it in words; this is the same thing in
/// data. Every decision in a fight is one of these - **a tap on the table never decides anything**, it only
/// says *which* hero or foe we are talking about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChoiceAction {
    /// Engage: reach for this foe's card (committing one tempo to start).
    Aim(CardId),
    /// Engage: drop the reach and pick a target again.
    Unaim,
    /// Engage: commit this many tempo to the reach. Strike: pour this many blows in.
    Bid(u32),
    /// Evade: pay the exact price and break everything reaching you, or stand and keep the tempo to hit back.
    Dodge(Dodge),
    /// Engage / Strike: this hero deliberately does nothing here. **Not** the same as having decided nothing.
    Hold,
}

/// **Every decision on offer right now, as cards** - each carrying what it costs and does, and, when it cannot
/// be taken, why not. This is the whole decision surface of a fight: Engage's targets and commitments, Evade's
/// slip-or-stand, Strike's blows.
///
/// The rule it encodes: *a tap on a card says **which**; a choice card says **what***.
pub(crate) fn step_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (_, _, _, _, step) = arena_state(board, arena);
    match step {
        Step::Marshal => Vec::new(),
        Step::Engage => engage_choices(board, arena),
        Step::Evade => evade_choices(board, arena),
        Step::Strike => strike_choices(board, arena),
    }
}

/// The choice cards for the current step, for the renderer.
pub fn scene_choices(board: &Board, arena: PileId) -> Vec<Choice> {
    step_choices(board, arena)
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

/// **Engage.** With no target yet, the cards are the targets. Once reaching, they are the *commitment* - and
/// each says the thing that makes this a decision: what it costs the target to slip you, and how many blows
/// you will have left if they cannot. Every card you sink into reaching them is a card you cannot swing with.
fn engage_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (cards, units, sub, _round, _step) = arena_state(board, arena);
    let Some((card, i)) = focused_party(board, arena) else {
        return Vec::new();
    };
    let u = &units[i];
    let s = staged_of(board, card);
    let mut out = Vec::new();

    match s.aim.and_then(|a| cards.iter().position(|&c| c == a)) {
        Some(t) => {
            let foe = &units[t];
            for n in 1..=u.tempo {
                let value = n * u.finesse.max(1);
                let price = value / foe.finesse.max(1) + 1; // what it costs them to slip this
                let blows = 1 + (u.tempo - n); // the opening blow is paid for, however much you committed
                let text = if price > foe.tempo {
                    format!(
                        "value {value} - {} cannot escape ({price} tempo, has {}); then {blows} blows",
                        foe.name, foe.tempo
                    )
                } else {
                    format!(
                        "value {value} - {} escapes for {price} tempo; else {blows} blows",
                        foe.name
                    )
                };
                out.push((
                    Choice::new(format!("Commit {n} tempo"), text).chosen(s.bid == n),
                    ChoiceAction::Bid(n),
                ));
            }
            out.push((
                Choice::new("Reach elsewhere", "pick another target"),
                ChoiceAction::Unaim,
            ));
        }
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
                    Choice::new(
                        format!("Reach for {}", foe.name),
                        format!("Might {} a blow, once you have them", u.might),
                    ),
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

/// **Evade.** You can see exactly what they committed, so the price of escaping is exact - which is why there
/// is no partial slip to offer: underpaying would never be a gamble, only a waste. Two cards: pay it in full,
/// or stand, keep the tempo, and (on a melee edge) hit back with it.
fn evade_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (cards, units, _sub, _round, _step) = arena_state(board, arena);
    let Some((card, i)) = focused_party(board, arena) else {
        return Vec::new();
    };
    let reaching = read_contacts(board, arena, &cards);
    let u = &units[i];
    let s = staged_of(board, card).dodge;
    let Some(cost) = combat::slip_cost(&units, &reaching, i) else {
        return Vec::new();
    };

    let incoming: Vec<&Contact> = reaching.iter().filter(|c| c.target == i).collect();
    let taken: u32 = incoming.iter().map(|c| units[c.attacker].might).sum();
    let answer = incoming
        .iter()
        .any(|c| u.melee && !combat::rank_is_ranged(units[c.attacker].rank));

    // **Standing does not spend anything, and it does not decide anything either.** How many blows you answer
    // with is the *Strike* step's decision (0, 1, ... up to your tempo) - so this card must say what standing
    // leaves you able to do, not what it will do. Quoting "then 2 blows back" here promised an allocation the
    // player had not made and could not change, and hid the 0-and-1 options entirely.
    let stand_text = if answer {
        format!(
            "take {taken} damage, spend nothing - keeps {} tempo to answer with at Strike",
            u.tempo
        )
    } else {
        format!("take {taken} damage, spend nothing - they are at range, nothing to answer with")
    };
    let stand = Choice::new("Stand", stand_text).chosen(s == Some(Dodge::Stand));

    let slip_text = if cost <= u.tempo {
        format!(
            "spend {cost} tempo - nothing reaches you this phase; {} tempo left",
            u.tempo - cost
        )
    } else {
        format!("spend {cost} tempo - nothing reaches you this phase")
    };
    let slip = Choice::new("Slip", slip_text).chosen(s == Some(Dodge::Slip));
    let slip = if cost <= u.tempo {
        slip
    } else {
        slip.barred(format!("needs {cost} tempo, you have {}", u.tempo))
    };

    vec![
        (slip, ChoiceAction::Dodge(Dodge::Slip)),
        (stand, ChoiceAction::Dodge(Dodge::Stand)),
    ]
}

/// **Strike.** Finesse is done; contact is made. One card per blow, each blow a Might - so this is where the
/// tempo you did *not* sink into reaching them turns into damage.
fn strike_choices(board: &Board, arena: PileId) -> Vec<(Choice, ChoiceAction)> {
    let (cards, units, _sub, _round, _step) = arena_state(board, arena);
    let Some((card, i)) = focused_party(board, arena) else {
        return Vec::new();
    };
    let contacts = read_contacts(board, arena, &cards);
    let Some(t) = combat::strike_target(&units, &contacts, i) else {
        return Vec::new();
    };
    let u = &units[i];
    let s = staged_of(board, card);

    // **The opening blow is already bought.** If this unit opened the contact, the tempo it committed at Engage
    // has already paid for one blow, which lands whatever it does now. So a card here spends one *more* card
    // for one *more* blow, and every quoted total has to include the free one - otherwise the numbers on the
    // cards are simply wrong. A unit merely *answering* along a melee edge someone else opened has no opening
    // blow: it never committed anything.
    let opening = u32::from(contacts.iter().any(|c| c.attacker == i));

    let mut out = Vec::new();
    for n in 1..=u.tempo {
        let label = if opening > 0 {
            format!("Strike {n} more")
        } else {
            format!("Strike back {n}x")
        };
        out.push((
            Choice::new(label, blows_phrase(&units[t], u.might, opening + n)).chosen(s.bid == n),
            ChoiceAction::Bid(n),
        ));
    }
    let hold_text = if opening > 0 {
        format!(
            "no more blows - the opening blow still lands ({}); keeps {} tempo",
            blows_phrase(&units[t], u.might, 1),
            u.tempo
        )
    } else {
        format!("do not answer - keeps {} tempo for a later phase", u.tempo)
    };
    out.push((
        Choice::new("Hold", hold_text).chosen(s.hold),
        ChoiceAction::Hold,
    ));
    out
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
    let Some((card, _)) = focused_party(board, arena) else {
        return;
    };
    let mut s = staged_of(board, card);
    match action {
        // Reaching starts at one card - the cheapest reach, and the most tempo kept back for blows. The player
        // raises it from the commitment cards, each of which says what the extra card actually buys.
        ChoiceAction::Aim(foe) => {
            s.aim = Some(foe);
            s.bid = 1;
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
        ChoiceAction::Dodge(d) => s.dodge = Some(d),
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
    let contacts = read_contacts(board, arena, &cards);
    match (step, side) {
        (Step::Marshal, _) => unreachable!("handled above"),
        // A foe tap only means anything while reaching: it is the shortcut for picking a target.
        (Step::Engage, Side::Foe) => aim_active(board, &cards, &units, sub, i),
        // Any hero this step is asking something of may be selected - that is the whole rule, and it is the
        // same `can_act` the choice cards and the Commit gate read.
        (_, Side::Party) if can_act(&units, &contacts, sub, step, i) => {
            select_active(board, &cards, &units, i)
        }
        _ => {}
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

    /// **Nothing is decided by default, and Commit says what is missing.** The Raider is reached at Intercept
    /// and can Slip or Stand - so it is being *asked*. Committing before it answers would silently enter Stand
    /// as if the player had chosen it.
    #[test]
    fn commit_is_barred_until_a_reached_hero_answers_and_names_who() {
        let mut board = sample_table();
        // The Raider marches alone: it ranks Outrider, so The Wall (an enemy Vanguard) screens it at Intercept.
        let arena = open_a_fight_at(&mut board, "Raider", Some("The Sundered Vault"));
        commit(&mut board, arena); // Marshal -> Intercept / Engage
        commit(&mut board, arena); // Engage -> Evade: The Wall is reaching for the Raider

        let (_, units, _, _, step) = arena_state(&board, arena);
        assert_eq!(step, Step::Evade);
        let (card, i) = focused_party(&board, arena).expect("a hero is being reached for");
        assert!(
            staged_of(&board, card).dodge.is_none(),
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
        let stand = scene_choices(&board, arena)
            .iter()
            .position(|c| c.label == "Stand")
            .expect("Stand is always offered");
        choose(&mut board, stand);
        assert_eq!(pending_decision(&board, arena), None);
        assert_eq!(commit_label(&board, arena), "Commit");
    }

    /// **Reaching buys ONE blow; the tempo you keep back buys the rest - and the pile spans the round.** The
    /// Raider (Might 7) reaches The Wall (Toughness 9) with one card, stands its ground, then pours its
    /// remaining card in: 7 + 7 = 14 into one pile, which crosses 9 and turns a Health card. That is the only
    /// way anything cracks a Wall.
    #[test]
    fn one_card_of_reach_plus_one_blow_cracks_the_wall() {
        let mut board = sample_table();
        // The lone Wall - the fight the Raider is built to solo.
        let arena = open_a_fight_at(&mut board, "Raider", Some("The Sundered Vault"));
        // Rank the Raider as a Vanguard, so it meets The Wall in the Clash (Vanguard -> Vanguard).
        let raider = pool_heroes(&board, arena)[0];
        let van = sub_pile(&board, arena, "Vanguard").unwrap();
        let _ = board.move_card(raider, van, 0);
        commit(&mut board, arena); // Start / Reveal

        let wall_card = {
            let (cards, units, _, _, _) = arena_state(&board, arena);
            cards[units.iter().position(|u| u.side == Side::Foe).unwrap()]
        };
        let wall = |b: &Board| read_combatant(b, wall_card, Rank::Vanguard).unwrap();
        let me = |b: &Board| {
            let (cards, units, _, _, _) = arena_state(b, arena);
            cards[(0..units.len())
                .find(|&i| units[i].side == Side::Party)
                .unwrap()]
        };
        let hp0 = wall(&board).health;

        // Walk to the Clash, answering every step on the way.
        let clash = 3;
        let mut guard = 0;
        while arena_state(&board, arena).2 != clash || arena_state(&board, arena).4 != Step::Engage
        {
            answer_greedily(&mut board, arena);
            commit(&mut board, arena);
            guard += 1;
            assert!(guard < 40, "the Clash must arrive");
        }

        // Engage: reach for The Wall with the seeded single card - the cheapest reach, keeping tempo for blows.
        let c = me(&board);
        handle_tap(&mut board, c);
        let reach = scene_choices(&board, arena)
            .iter()
            .position(|c| c.label.starts_with("Reach for The Wall"))
            .expect("a Vanguard may reach the enemy Vanguard at the Clash");
        choose(&mut board, reach);
        assert_eq!(staged_of(&board, c).bid, 1, "reaching starts at one card");
        commit(&mut board, arena);

        // Evade: The Wall (greedy) stands - it is in melee and would rather answer. We are reached in turn.
        answer_greedily(&mut board, arena);
        commit(&mut board, arena);

        // Strike: the opening blow is already paid for. Pour the last card in.
        assert_eq!(arena_state(&board, arena).4, Step::Strike);
        let c = me(&board);
        handle_tap(&mut board, c);
        let choices = scene_choices(&board, arena);
        let one_more = choices
            .iter()
            .position(|c| c.label == "Strike 1 more")
            .expect("one card left to swing with");
        // The card must count the free opening blow: one MORE card is 2 blows, 14 damage, which cracks 9.
        assert_eq!(
            choices[one_more].consequence,
            "14 damage: The Wall loses 1 health"
        );
        choose(&mut board, one_more);
        commit(&mut board, arena);

        assert_eq!(
            wall(&board).health,
            hp0 - 1,
            "the opening blow (7) plus one more (7) = 14, which crosses Toughness 9"
        );
        assert_eq!(
            wall(&board).pending,
            5,
            "the remainder is an open wound - it keeps until the Reset"
        );
    }

    /// Answer whatever the current step is asking, so a test can walk a fight without hand-playing it.
    fn answer_greedily(board: &mut Board, arena: PileId) {
        let mut guard = 0;
        while pending_decision(board, arena).is_some() {
            let choices = scene_choices(board, arena);
            let Some(i) = choices.iter().position(|c| {
                c.enabled() && (c.label == "Hold" || c.label == "Stand" || c.label == "Slip")
            }) else {
                break;
            };
            choose(board, i);
            guard += 1;
            assert!(guard < 20, "every asked hero must be answerable");
        }
    }

    /// **At Engage the choice row IS the target list, then the commitment.** Tapping only says *which* hero;
    /// the order is always a card that states what it does. And Hold is a real order - a hero that could reach
    /// must say it is not reaching, so "no target" never has to mean two different things.
    #[test]
    fn engage_offers_targets_then_commitments_and_hold_is_a_real_order() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, "Bastion", Some("The Sundered Vault"));
        commit(&mut board, arena); // Marshal -> Intercept / Engage

        let (cards, units, _, _, _) = arena_state(&board, arena);
        let hero = (0..units.len())
            .find(|&i| units[i].side == Side::Party)
            .expect("the Bastion marched");

        // Walk to the Clash, where a Vanguard may reach the enemy Vanguard.
        let mut guard = 0;
        while arena_state(&board, arena).2 != 3 || arena_state(&board, arena).4 != Step::Engage {
            answer_greedily(&mut board, arena);
            commit(&mut board, arena);
            guard += 1;
            assert!(guard < 40, "the Clash must arrive");
        }
        handle_tap(&mut board, cards[hero]);
        let labels = |b: &Board| -> Vec<String> {
            scene_choices(b, arena)
                .iter()
                .map(|c| c.label.clone())
                .collect()
        };
        assert_eq!(labels(&board), vec!["Reach for The Wall", "Hold"]);

        // Reach: the row becomes the commitment, and the hero no longer owes an order.
        assert!(
            pending_decision(&board, arena).is_some(),
            "a hero with no target owes an order"
        );
        choose(&mut board, 0);
        assert!(labels(&board).iter().any(|l| l.starts_with("Commit ")));
        assert!(labels(&board).contains(&"Reach elsewhere".to_string()));
        assert_eq!(pending_decision(&board, arena), None);

        // Hold is equally an answer: it clears the reach, and Commit still comes live.
        let back = labels(&board)
            .iter()
            .position(|l| l == "Reach elsewhere")
            .unwrap();
        choose(&mut board, back);
        let hold = labels(&board).iter().position(|l| l == "Hold").unwrap();
        choose(&mut board, hold);
        assert!(staged_of(&board, cards[hero]).hold);
        assert_eq!(
            pending_decision(&board, arena),
            None,
            "holding IS deciding - it must not read as undecided"
        );
    }

    /// **Evade decides slip-or-stand, and NOTHING else.** How many blows you answer with is the Strike step's
    /// decision - 0, 1, ... up to your tempo. The Stand card used to read "then 2 blows back: 14 damage", which
    /// promised an allocation the player had never made, could not change, and whose 0-and-1 alternatives it
    /// hid entirely. Standing spends nothing; it *keeps* the tempo.
    #[test]
    fn standing_spends_nothing_and_the_blows_are_chosen_at_strike() {
        let mut board = sample_table();
        let arena = open_a_fight_at(&mut board, "Raider", Some("The Sundered Vault"));
        commit(&mut board, arena); // Marshal -> Intercept / Engage
        commit(&mut board, arena); // Engage -> Evade: The Wall reaches for the Raider

        assert_eq!(arena_state(&board, arena).4, Step::Evade);
        let (card, i) = focused_party(&board, arena).expect("a hero is being reached for");
        let tempo = arena_state(&board, arena).1[i].tempo;

        let stand = scene_choices(&board, arena);
        let stand = stand.iter().find(|c| c.label == "Stand").unwrap();
        assert_eq!(
            stand.consequence,
            format!("take 1 damage, spend nothing - keeps {tempo} tempo to answer with at Strike"),
            "Stand must not pre-commit the blows"
        );

        let idx = scene_choices(&board, arena)
            .iter()
            .position(|c| c.label == "Stand")
            .unwrap();
        choose(&mut board, idx);
        commit(&mut board, arena);

        // ...and NOW the allocation is asked for, with every option from 0 (Hold) upward.
        assert_eq!(arena_state(&board, arena).4, Step::Strike);
        assert_eq!(
            arena_state(&board, arena).1[i].tempo,
            tempo,
            "standing spent nothing"
        );
        handle_tap(&mut board, card);
        let labels: Vec<String> = scene_choices(&board, arena)
            .iter()
            .map(|c| c.label.clone())
            .collect();
        for n in 1..=tempo {
            assert!(
                labels.contains(&format!("Strike back {n}x")),
                "answering with {n} blows must be on offer: {labels:?}"
            );
        }
        assert!(labels.contains(&"Hold".to_string()), "so must 0 blows");
    }

    /// **A choice must not promise damage it cannot deliver.** The Raider's blow (Might 7) on The Wall
    /// (Toughness 9) flips nothing on its own: it banks into the pile and keeps until the Reset. The card has to
    /// say where the blow goes and how long it lasts, because that is the whole reason to strike under the bar.
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
            blows_phrase(&wall, 7, 1),
            "7 into The Wall's pile: 7/9 - it keeps until the Reset"
        );
        // Two blows in one go DO cross it - which is exactly what makes keeping tempo back worth it.
        assert_eq!(
            blows_phrase(&wall, 7, 2),
            "14 damage: The Wall loses 1 health"
        );
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
    fn taps_stage_a_plan_and_engaging_persists_a_reach() {
        let mut board = sample_table();
        let arena = open_a_fight(&mut board);
        commit(&mut board, arena); // Marshal -> Engage (auto-ranks the hero)
        assert_eq!(current_step(&board, arena), Step::Engage);

        let (cards, units, sub, _, _) = arena_state(&board, arena);
        let pi = units.iter().position(|u| u.side == Side::Party).unwrap();
        if let Some(fi) = (0..units.len()).find(|&j| {
            units[j].side == Side::Foe && combat::legal_strike(sub, units[pi].rank, units[j].rank)
        }) {
            handle_tap(&mut board, cards[pi]); // select
            assert!(staged_of(&board, cards[pi]).active);
            handle_tap(&mut board, cards[fi]); // reach for the foe (seeds one card)
            let staged = staged_of(&board, cards[pi]);
            assert_eq!(staged.aim, Some(cards[fi]));
            assert_eq!(staged.bid, 1, "reaching starts at the cheapest commitment");

            commit(&mut board, arena); // resolve Engage
            assert_eq!(current_step(&board, arena), Step::Evade);
            let contacts = read_contacts(&board, arena, &arena_state(&board, arena).0);
            assert!(
                contacts.iter().any(|c| c.attacker == pi),
                "the staged reach persisted as a contact for the Evade step"
            );
        }
    }
}

#[cfg(test)]
mod choice_tests {
    use super::*;

    fn combatant(name: &str, side: Side, rank: Rank, melee: bool, ranged: bool) -> Combatant {
        let mut c = Combatant::from_stats(name, side, rank, [3, 5, 1, 2, 2], 0, melee, ranged);
        c.tempo = 2;
        c
    }

    /// **The question that started all this, answered by the model instead of a keyword.** You cannot strike
    /// back at a shot from the back line - not because a rule forbids it, but because a ranged contact is
    /// one-way: nothing came within your reach. Standing against an archer therefore buys you *nothing*, and
    /// the Stand card has to say so, or the player cannot tell a rule from a bug.
    #[test]
    fn standing_against_a_shot_buys_nothing_and_the_card_says_so() {
        let hero = combatant("hero", Side::Party, Rank::Outrider, true, false); // carries melee
        let shooter = combatant("shooter", Side::Foe, Rank::Rearguard, false, true); // fires from the back
        let units = vec![hero, shooter];
        let reaching = vec![Contact {
            attacker: 1,
            target: 0,
            bid: 2,
        }];
        assert_eq!(
            combat::strike_target(&units, &reaching, 0),
            None,
            "a ranged edge is one-way - there is nothing to answer along it"
        );
    }

    /// The Wall's case: a melee Vanguard reaching the crossing Outrider. The edge is **mutual**, so standing is
    /// a real posture - you take the blow and answer it with everything you kept back. That is emergent from the
    /// edge, not a Strike Back keyword.
    #[test]
    fn standing_against_a_melee_reach_lets_you_answer_with_everything_you_kept() {
        let hero = combatant("Raider", Side::Party, Rank::Outrider, true, false);
        let wall = combatant("The Wall", Side::Foe, Rank::Vanguard, true, false);
        let units = vec![hero, wall];
        let reaching = vec![Contact {
            attacker: 1,
            target: 0,
            bid: 2,
        }];
        assert_eq!(
            combat::strike_target(&units, &reaching, 0),
            Some(1),
            "they came to you - you may answer, in a phase the schedule never gave you"
        );
    }

    /// **A slip you cannot pay for is not on offer**, and the card says the price rather than vanishing. With
    /// Finesse 2 against a reach worth 6, escaping costs 4 tempo - and a body holding 2 simply cannot.
    #[test]
    fn an_unaffordable_slip_is_barred_with_its_price() {
        let hero = combatant("hero", Side::Party, Rank::Vanguard, true, false); // Finesse 2, tempo 2
        let wall = combatant("The Wall", Side::Foe, Rank::Vanguard, true, false);
        let units = vec![hero, wall];
        let reaching = vec![Contact {
            attacker: 1,
            target: 0,
            bid: 6,
        }];
        // 6 / 2 + 1 = 4 cards to strictly exceed it.
        assert_eq!(combat::slip_cost(&units, &reaching, 0), Some(4));
        assert!(
            !can_act(&units, &reaching, 0, Step::Evade, 0),
            "with 2 tempo it cannot escape, so it is not being asked anything - it stands"
        );
    }
}
