//! **Stage 2/3 — board ↔ combat.** Represent a v2 fight on the physical `Tableau` and drive it through the
//! stage-1 mechanics brain ([`crate::combat`]). The *entire* fight state is physical (cards-as-truth):
//!
//! - Each **combatant** is a card in the `[Arena]` zone. Its constant stats are re-derived each resolve from
//!   the source (a hero's `character_recipe`, a foe's `catalog::creature`); its mutable state (rank / HP /
//!   tempo) rides detail lines 0–2, and the player's *staged plan* (active / aim / bid / reaction) rides the
//!   detail lines after that — edited by taps, consumed on commit, cleared at each mini-phase boundary.
//! - The **phase card** carries the walk position `(round, sub-phase, step)` where step is one of
//!   Marshal → Catch → React → Extra. A round is Marshal then five sub-phases, each three one-way steps.
//! - **Contacts** (a landed catch: attacker→target at some bid) are scratch `contact` cards, written at Catch
//!   so React and Extra can read what landed, and torn down at the sub-phase boundary.
//!
//! One [`commit`] advances **one step**: it reads the arena, folds in the party's staged plan (falling back to
//! the greedy AI when nothing is staged) plus the greedy foe plan, runs that step's mechanics order-free,
//! writes HP/tempo/contacts/deaths back, and advances the phase card. [`handle_tap`] edits the staged plan.

use cardtable_model::{CardId, CardKind, PileId, Tableau};
use deckbound::actor::Intention as Rank;
use deckbound::combat::{SCHEDULE, SUB_PHASE_NAMES};

use crate::combat::{self, Catch, Combatant, Contact, ExtraStrike, React, Side};

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

/// The vitality (max HP) of a combatant by name and side — the health bar's full value.
fn max_health(board: &Tableau, name: &str, side: Side) -> u32 {
    match side {
        Side::Party => hero_stats(board, name).map(|(s, _)| s.vitality),
        Side::Foe => foe_stats(name).map(|(s, _)| s.vitality),
    }
    .unwrap_or(0)
}

/// The default opening rank (matching deckbound's stat-derived formation): ranged → Rearguard, else
/// Might ≥ Toughness → Outrider, else Vanguard. The player re-declares (taps to cycle) during Marshal.
fn default_rank(s: &Stats, ranged: bool) -> Rank {
    if ranged {
        Rank::Rearguard
    } else if s.might >= s.toughness {
        Rank::Outrider
    } else {
        Rank::Vanguard
    }
}

// ---- the walk position: (round, sub-phase, step) on the phase card -------------------------------------

/// One of the three one-way mini-phases within a sub-phase, plus the per-round Marshal (rank re-declaration).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Re-declare ranks before the schedule runs (round start).
    Marshal,
    /// Attackers bid tempo to reach targets.
    Catch,
    /// Defenders eat / evade / strike back per incoming contact.
    React,
    /// Units on a surviving contact flip remaining tempo for extra strikes.
    Extra,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Step::Marshal => "Marshal",
            Step::Catch => "Catch",
            Step::React => "React",
            Step::Extra => "Extra",
        }
    }
    fn parse(s: &str) -> Step {
        match s {
            "Catch" => Step::Catch,
            "React" => Step::React,
            "Extra" => Step::Extra,
            _ => Step::Marshal,
        }
    }
}

// ---- combatant card state (rank/HP/tempo on detail 0-2, staged plan on 3+) -----------------------------

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
/// This also **clears the staged plan** (detail is reset to the three base lines).
fn write_combatant(board: &mut Tableau, card: CardId, u: &Combatant, max: u32) {
    let _ = board.set_card_detail(card, detail(u.rank, u.health, max, u.tempo));
}

// ---- the staged plan (detail lines after the base three) ----------------------------------------------

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

/// Rewrite a unit card's detail as the three base lines followed by whatever of the plan is set.
fn write_staged(board: &mut Tableau, card: CardId, s: &Staged) {
    let Some(mut lines) = board
        .card(card)
        .map(|c| c.detail()[..3.min(c.detail().len())].to_vec())
    else {
        return;
    };
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

// ---- contact scratch cards (a landed catch: attacker → target at some bid) -----------------------------

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

/// The combatant cards (in order), their [`Combatant`]s, and the walk position from the phase card.
fn arena_state(board: &Tableau, arena: PileId) -> (Vec<CardId>, Vec<Combatant>, usize, u32, Step) {
    let mut cards = Vec::new();
    let mut units = Vec::new();
    let (mut sub, mut round, mut step) = (0usize, 1u32, Step::Marshal);
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
                sub = (d.get(1).map(|l| num_after(l, "Sub-phase ")).unwrap_or(1) as usize)
                    .saturating_sub(1);
                step = d
                    .get(2)
                    .map(|l| Step::parse(l.strip_prefix("Step: ").unwrap_or("")))
                    .unwrap_or(Step::Marshal);
            }
            _ => {}
        }
    }
    (cards, units, sub, round, step)
}

/// The vitality (max HP) of every combatant, index-aligned with the cards.
fn maxes_of(board: &Tableau, units: &[Combatant]) -> Vec<u32> {
    units
        .iter()
        .map(|u| max_health(board, &u.name, u.side).max(u.health))
        .collect()
}

// ---- opening a fight ----------------------------------------------------------------------------------

/// Open a fight at `place`: build the `[Arena]` zone with a combatant card per stationed hero (moved in) and
/// per instantiated foe (drawn from the Bestiary), a phase card at round 1 · Marshal, and focus it.
pub fn open_fight(board: &mut Tableau, place: PileId) -> Option<PileId> {
    let bestiary = top_deck(board, "Bestiary")?;
    let root = board.root_id();
    let arena = board.add_pile(root, ARENA).ok()?;

    // A hidden meta card remembers the originating place (for teardown) — never rewritten by the phase card.
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

    add_phase_card(board, arena, 1, 0, Step::Marshal);
    let _ = board.focus(arena);
    Some(arena)
}

fn add_phase_card(board: &mut Tableau, arena: PileId, round: u32, sub: usize, step: Step) {
    if let Ok(card) = board.add_card(
        arena,
        cardtable_model::Face::Up {
            title: phase_title(sub, step),
        },
        None,
    ) {
        let _ = board.set_card_kind(card, CardKind::Virtual);
        let _ = board.set_card_type(card, "phase");
        let _ = board.set_card_detail(card, phase_detail(round, sub, step));
    }
}

fn phase_title(sub: usize, step: Step) -> String {
    match step {
        Step::Marshal => "Phase: Marshal".to_string(),
        _ => format!(
            "Phase: {}",
            SUB_PHASE_NAMES.get(sub).copied().unwrap_or("Marshal")
        ),
    }
}

fn phase_detail(round: u32, sub: usize, step: Step) -> Vec<String> {
    vec![
        format!("Round {round}"),
        format!("Sub-phase {}/5", sub + 1),
        format!("Step: {}", step.name()),
    ]
}

fn set_phase(board: &mut Tableau, arena: PileId, round: u32, sub: usize, step: Step) {
    if let Some(card) = board
        .content_cards(arena)
        .into_iter()
        .find(|&c| board.card(c).map(|k| k.card_type()) == Some("phase"))
    {
        let _ = board.set_face(
            card,
            cardtable_model::Face::Up {
                title: phase_title(sub, step),
            },
        );
        let _ = board.set_card_detail(card, phase_detail(round, sub, step));
    }
}

// ---- the greedy AI (foe side always; party side when nothing is staged) --------------------------------

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
        if u.fallen || u.side != Side::Party {
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

/// Whether the fight is over, and who won (`Some(true)` = party). A side loses when all its units are fallen.
pub fn outcome(board: &Tableau, arena: PileId) -> Option<bool> {
    let (_, units, _, _, _) = arena_state(board, arena);
    let party_alive = units.iter().any(|u| u.side == Side::Party && !u.fallen);
    let foes_alive = units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    match (party_alive, foes_alive) {
        (true, true) => None,
        (won, _) => Some(won),
    }
}

/// The label for the Commit control, given the current step (or the fight's decision).
pub fn commit_label(board: &Tableau, arena: PileId) -> &'static str {
    match outcome(board, arena) {
        Some(true) => "Victory — leave",
        Some(false) => "Defeat — leave",
        None => match arena_state(board, arena).4 {
            Step::Marshal => "Ranks set — begin",
            Step::Catch => "Commit catches",
            Step::React => "Commit reactions",
            Step::Extra => "Commit strikes",
        },
    }
}

/// Cards to spend to evade a `bid` at finesse `f_def`: the smallest count whose value strictly exceeds it.
fn evade_cost(bid: u32, f_def: u32, tempo: u32) -> u32 {
    (bid / f_def.max(1) + 1).min(tempo)
}

/// Resolve **one step** — Marshal / Catch / React / Extra — folding the party's staged plan (or the greedy
/// AI when nothing is staged) with the greedy foe plan, then writing the results back and advancing the walk.
/// Returns whether the fight is over.
pub fn commit(board: &mut Tableau, arena: PileId) -> bool {
    let (cards, mut units, sub, round, step) = arena_state(board, arena);
    let maxes = maxes_of(board, &units);
    let writeback = |board: &mut Tableau, units: &[Combatant]| {
        for (i, card) in cards.iter().enumerate() {
            write_combatant(board, *card, &units[i], maxes[i]);
        }
    };

    match step {
        Step::Marshal => {
            set_phase(board, arena, round, 0, Step::Catch);
        }

        Step::Catch => {
            let mut catches = party_catches(board, &cards, &units, sub);
            if catches.is_empty() {
                catches = greedy_catches(&units, Side::Party, sub);
            }
            catches.extend(greedy_catches(&units, Side::Foe, sub));
            let contacts = combat::resolve_catch(&mut units, &catches);
            writeback(board, &units); // clears the staged catch plan
            write_contacts(board, arena, &cards, &contacts);
            set_phase(board, arena, round, sub, Step::React);
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
            set_phase(board, arena, round, sub, Step::Extra);
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

            let next_sub = sub + 1;
            if next_sub >= SCHEDULE.len() {
                combat::refresh_round(&mut units);
                writeback(board, &units);
                set_phase(board, arena, round + 1, 0, Step::Marshal);
            } else {
                writeback(board, &units);
                set_phase(board, arena, round, next_sub, Step::Catch);
            }
        }
    }

    outcome(board, arena).is_some()
}

// ---- editing the staged plan by tap -------------------------------------------------------------------

/// Whether `card` is a live combatant in `arena` (a legal tap target while a fight is up).
pub fn is_combatant(board: &Tableau, arena: PileId, card: CardId) -> bool {
    matches!(
        board.card(card).map(|k| k.card_type()),
        Some("unit") | Some("foe")
    ) && board.pile(arena).is_some_and(|p| p.cards().contains(&card))
}

fn cycle_rank(board: &mut Tableau, card: CardId) {
    let Some(d) = board.card(card).map(|c| c.detail().to_vec()) else {
        return;
    };
    let next = match rank_of(d.first().map(String::as_str).unwrap_or("")) {
        Rank::Vanguard => Rank::Outrider,
        Rank::Outrider => Rank::Rearguard,
        Rank::Rearguard => Rank::Vanguard,
    };
    let mut lines = d;
    if lines.is_empty() {
        lines.push(next.label().into());
    } else {
        lines[0] = next.label().into();
    }
    let _ = board.set_card_detail(card, lines);
}

/// Make `card` the only active party attacker (clears `active` on the rest).
fn select_active(board: &mut Tableau, cards: &[CardId], units: &[Combatant], chosen: usize) {
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
fn aim_active(board: &mut Tableau, cards: &[CardId], units: &[Combatant], sub: usize, foe: usize) {
    let Some(active) = (0..units.len())
        .find(|&i| units[i].side == Side::Party && staged_of(board, cards[i]).active)
    else {
        return;
    };
    if !combat::legal_catch(sub, units[active].rank, units[foe].rank) {
        return;
    }
    let mut s = staged_of(board, cards[active]);
    s.aim = Some(cards[foe]);
    if s.bid == 0 {
        let need = units[foe].finesse.div_ceil(units[active].finesse.max(1));
        s.bid = need.min(units[active].tempo).max(1);
    }
    write_staged(board, cards[active], &s);
}

fn cycle_react(board: &mut Tableau, card: CardId) {
    let mut s = staged_of(board, card);
    s.react = Some(s.react.unwrap_or(ReactKind::Eat).next());
    write_staged(board, card, &s);
}

/// Handle a tap on combatant `card`: edit the staged plan for the current step (a no-op if the tap is
/// meaningless there). Reads the step from the phase card, so it needs no extra state.
pub fn handle_tap(board: &mut Tableau, card: CardId) {
    let Some(arena) = find_arena(board) else {
        return;
    };
    let (cards, units, sub, _round, step) = arena_state(board, arena);
    let Some(i) = cards.iter().position(|&c| c == card) else {
        return;
    };
    let side = units[i].side;
    match step {
        Step::Marshal => {
            if side == Side::Party {
                cycle_rank(board, card);
            }
        }
        Step::Catch => match side {
            Side::Party => {
                if staged_of(board, card).active {
                    cycle_bid(board, card, units[i].tempo);
                } else {
                    select_active(board, &cards, &units, i);
                }
            }
            Side::Foe => aim_active(board, &cards, &units, sub, i),
        },
        Step::React => {
            let incoming = read_contacts(board, arena, &cards)
                .iter()
                .any(|c| c.target == i);
            if side == Side::Party && incoming {
                cycle_react(board, card);
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

/// The active fight's arena zone, if a fight is underway.
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

/// **Fold the fight back** (teardown): return foe cards to the Bestiary; on a **win** clear the encounter;
/// move the party's units back to the place as hero position cards; remove the arena; leave it; a fight
/// spends a day. Conservation-clean (foes round-trip; the arena's scratch is torn down).
pub fn fold_back(board: &mut Tableau, arena: PileId) {
    let won = outcome(board, arena) == Some(true);
    let place = place_of(board, arena);
    let bestiary = top_deck(board, "Bestiary");

    let foes: Vec<CardId> = board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("foe"))
        .collect();
    if let Some(b) = bestiary {
        let _ = board.return_foes_to_bestiary(&foes, b);
    }

    let units: Vec<CardId> = board
        .content_cards(arena)
        .into_iter()
        .filter(|&c| board.card(c).map(|k| k.card_type()) == Some("unit"))
        .collect();
    if let Some(place) = place {
        for u in units {
            let _ = board.set_card_type(u, "hero");
            let _ = board.set_card_detail(u, Vec::new());
            let at = board.pile(place).map_or(0, |p| p.cards().len());
            let _ = board.move_card(u, place, at);
        }
        if won
            && let Some(enc) = board
                .content_cards(place)
                .into_iter()
                .find(|&c| board.card(c).map(|k| k.card_type()) == Some("encounter"))
        {
            let _ = board.remove_card(enc);
        }
    }

    let _ = board.remove_pile(arena);
    board.zoom_out();
    if let (Some(p), Some(e)) = (top_deck(board, "Progress"), top_deck(board, "Events")) {
        let _ = board.advance_day(p, e);
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

        // Recruit Vael with Marksman, march to a place with an encounter.
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
        let (_, units, _, _, step) = arena_state(&board, arena);
        assert_eq!(step, Step::Marshal, "a fresh fight opens at Marshal");
        assert!(
            units.iter().any(|u| u.side == Side::Party),
            "the hero is in the arena"
        );
        assert!(
            units.iter().any(|u| u.side == Side::Foe),
            "foes were instantiated"
        );

        // Drive steps to a decision (greedy party, since nothing is staged).
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

    /// Tapping walks the plan: a party unit cycles rank in Marshal; selects then bids in Catch; a foe tap
    /// aims the active attacker. Committing Catch turns a staged aim+bid into a persisted contact card.
    #[test]
    fn taps_stage_a_plan_and_catch_persists_a_contact() {
        let mut board = sample_table();
        // Build a minimal two-combatant arena by hand: one party unit, one foe, wired into an [Arena].
        // (Reuse the opening-a-fight machinery from the other test's setup would be heavier; here we only
        // need the staging/commit behaviour, so drive the tap/commit surface directly on a real fight.)
        use crate::CardTableGame;
        use crate::Intention;
        use cardtable_model::BoardGame;
        let game = CardTableGame;
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
        let progress = top_deck(&board, "Progress").unwrap();
        let _ = board.move_character(position, place, progress);
        let arena = open_fight(&mut board, place).unwrap();

        // Marshal: tapping the party unit cycles its rank.
        let (cards, units, _, _, _) = arena_state(&board, arena);
        let party = cards[units.iter().position(|u| u.side == Side::Party).unwrap()];
        let before = rank_of(board.card(party).unwrap().detail()[0].as_str());
        handle_tap(&mut board, party);
        let after = rank_of(board.card(party).unwrap().detail()[0].as_str());
        assert_ne!(before, after, "Marshal tap cycled the rank");

        // Commit Marshal → Catch.
        commit(&mut board, arena);
        assert_eq!(arena_state(&board, arena).4, Step::Catch);

        // Catch: put the party unit into a rank that can reach some foe, select it, aim at that foe, bid.
        let (cards, units, sub, _, _) = arena_state(&board, arena);
        let pi = units.iter().position(|u| u.side == Side::Party).unwrap();
        // Force a rank that has a legal target this sub-phase.
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
            assert_eq!(arena_state(&board, arena).4, Step::React);
            // A contact from the party unit should now exist as a scratch card.
            let contacts = read_contacts(&board, arena, &arena_state(&board, arena).0);
            assert!(
                contacts.iter().any(|c| c.attacker == pi),
                "the staged catch landed as a persisted contact"
            );
        }
    }
}
