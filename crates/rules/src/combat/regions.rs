//! **The combat model's PHYSICS and geometry** - the board (formations, regions, ranks), the shared exchange
//! machinery every step resolves through (engage -> evade -> land, the grit pile, area strikes), the transcript
//! types ([`SubPhaseLog`], [`Hit`], [`Reach`]), and the one-ply instinct reads ([`foe_catch`],
//! [`wants_to_cross`]) the scripted policies are built from.
//!
//! The DECISION layer lives in [`super::step_game`] (the eight-step round as a `Game`) and the per-step
//! RESOLVERS in [`super::steps`]; this module is what they both stand on. The rules it keeps are the settled
//! canon (see `needs-merge/round-sequence.md`):
//!
//! - **One region per side; a region is a formation, not a place.** It says who screens you and who an area
//!   strike catches - nothing else. Position is never declared, only earned.
//! - **Rank is weapon-derived** (melee -> [`Rank::Vanguard`], ranged-only -> [`Rank::Rearguard`]);
//!   [`Rank::Outrider`] is the one rank earned by playing (crossing in), shed by withdrawing or when the ground
//!   is taken.
//! - **A collapsed vanguard does not promote its rearguard.** The back stays a back: it keeps its early fire and
//!   merely loses its screen, becoming reachable - which is what keeps range meaning something.
//! - **The screen is a price, not a wall**; every contest is Finesse-weighted tempo against tempo, every strike
//!   Might against the Grit pile, and a horde is many one-Health, Grit-strong bodies with NO spill between them.
use std::collections::HashMap;

use super::resolve::{
    Combatant, Contact, Dodge, Engage, Side, can_answer, end_sub_phase, resolve_evade, slip_cost,
};

/// A pour of extra tempo along a contact: `unit` spends `cards` more strikes of Might on `target`, beyond the
/// opening blow its reach already bought. The regions damage step ([`land`]) owns applying these.
#[derive(Clone, Copy, Debug)]
struct Blows {
    unit: usize,
    target: usize,
    cards: u32,
}

/// A fight not decided in five rounds is a draw, and a draw is not a win (spec 0.4).
pub const MAX_ROUNDS: usize = 5;

/// **A body's rank - where it stands, and the one thing that can be earned mid-fight.**
///
/// Two of the three are fixed by the weapon and never chosen: a body that can strike in melee is a
/// [`Vanguard`](Rank::Vanguard) at the front, a ranged-only body a [`Rearguard`](Rank::Rearguard) at the back.
/// The third is a **promotion** - cross into the enemy's ground and survive, and you become an
/// [`Outrider`](Rank::Outrider), loose inside their ranks. It is the only rank you reach by *playing*, and you
/// revert to your weapon rank the moment your side takes the ground you are standing on.
///
/// - A **vanguard** is in the fight. It can be clashed, it **catches slippers**, and it swings **last** (the
///   Clash) - closing to melee is the slowest thing you can do.
/// - A **rearguard** is holding off. It **fires first** (the Volley), it cannot be clashed while its own vanguard
///   stands (a raider has to come *in* for it), and a **melee** body ranked here is **dead weight** - a sword
///   cannot reach from the back.
/// - An **outrider** is past every screen, in enemy territory, part of no formation: it strikes any enemy in its
///   region and is struck by any of them, until it dies or its side takes the ground and it rejoins the line.
///
/// **A rearguard whose vanguard has collapsed is not promoted to vanguard.** This is the collapsed-vanguard rule,
/// and it is the hinge the whole design turns on: the back **stays** a back. It becomes *targetable* - anyone may
/// now clash it directly, no raid required (force, not fiat) - but it **keeps its phase slot**, so it still
/// shoots *before* the front swings. That is what makes range mean something: a lone archer is perfectly
/// targetable and gets **the first hit** anyway. (Promotion-*to-vanguard* destroyed exactly this - it turned
/// every unscreened cannon into just another front-line body - so only the *outrider* promotion survives.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rank {
    /// **Front, seeks melee** (weapon-fixed). Clashable, catches slippers, swings last.
    Vanguard,
    /// **Back, avoids melee** (weapon-fixed). Fires first; reachable only by a raid while a vanguard still stands,
    /// and by anyone once it does not. A melee body ranked here does nothing at all.
    Rearguard,
    /// **Loose in the enemy's ranks** (earned by crossing in). Past every screen: it strikes any enemy in its
    /// region and is struck by any of them. Reverts to its weapon rank when its side takes the ground.
    Outrider,
}
/// The whole position: the bodies, which region each stands in, and each body's rank.
#[derive(Clone, Debug)]
pub struct Board {
    pub units: Vec<Combatant>,
    /// Which region each body stands in. Ids are arbitrary - only the **partition** is meaningful.
    pub regions: Vec<u8>,
    /// Each body's [`Rank`]. **Vanguard/Rearguard are derived from the weapon and fixed for the whole fight**
    /// (ranged-only -> `Rearguard`, else `Vanguard`); **Outrider is earned** by crossing into the enemy's ground,
    /// and reverts to the weapon rank the moment the body's side takes that ground. Index-aligned with `units`.
    pub ranks: Vec<Rank>,
}

impl Board {
    /// Build a position. **Rank is derived from the weapon** (ranged-only -> `Rearguard`, else `Vanguard`) and
    /// nobody starts an outrider.
    pub fn new(units: Vec<Combatant>, regions: Vec<u8>) -> Board {
        let ranks = units.iter().map(Board::weapon_rank).collect();
        Board {
            units,
            regions,
            ranks,
        }
    }

    /// The rank a body takes from its weapon: a ranged-only body avoids melee (`Rearguard`); anything that can
    /// strike in melee stands at the front (`Vanguard`). A dual melee+ranged body is `Vanguard` (the deferred
    /// case - there are none). Never returns `Outrider`: that rank is only ever reached by promotion.
    pub fn weapon_rank(u: &Combatant) -> Rank {
        if u.ranged && !u.melee {
            Rank::Rearguard
        } else {
            Rank::Vanguard
        }
    }

    /// **Who owns `region`** - the side of its living **formation** bodies (vanguards and rearguards, never
    /// outriders). There is at most one such side. `None` when the region holds only outriders or is empty.
    pub fn owner(&self, region: u8) -> Option<Side> {
        self.in_region(region)
            .into_iter()
            .find(|&i| self.ranks[i] != Rank::Outrider)
            .map(|i| self.units[i].side)
    }

    /// **Is `target` screened?** - it is a **rearguard** whose side still has a living vanguard in its region to
    /// do the screening.
    ///
    /// An **outrider** is never screened (it is not a rearguard - it is loose inside the enemy ranks, adjacent to
    /// everyone), and a rearguard with no vanguard left is exposed: still a rearguard (it keeps its phase slot),
    /// simply now reachable.
    pub fn is_screened(&self, target: usize) -> bool {
        self.ranks[target] == Rank::Rearguard
            && !self
                .vanguard(self.regions[target], self.units[target].side)
                .is_empty()
    }

    /// The **vanguard** of `side` in `region` - the living bodies ranked [`Vanguard`](Rank::Vanguard). This is the
    /// whole screen: every one can catch a crosser, and it gets shorter each time one dies. An outrider is never
    /// part of a vanguard.
    pub fn vanguard(&self, region: u8, side: Side) -> Vec<usize> {
        self.in_region(region)
            .into_iter()
            .filter(|&i| self.ranks[i] == Rank::Vanguard && self.units[i].side == side)
            .collect()
    }

    pub fn in_region(&self, region: u8) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&i| !self.units[i].fallen && self.regions[i] == region)
            .collect()
    }

    /// The regions holding a living body, ascending - the piles a renderer would draw.
    pub fn occupied(&self) -> Vec<u8> {
        let mut rs: Vec<u8> = (0..self.units.len())
            .filter(|&i| !self.units[i].fallen)
            .map(|i| self.regions[i])
            .collect();
        rs.sort_unstable();
        rs.dedup();
        rs
    }

    pub fn alive(&self, side: Side) -> bool {
        self.units.iter().any(|u| u.side == side && !u.fallen)
    }

    /// `Some(true)` = the party won, `Some(false)` = the party is dead, `None` = still going.
    pub fn outcome(&self) -> Option<bool> {
        match (self.alive(Side::Party), self.alive(Side::Foe)) {
            (true, true) => None,
            (party, _) => Some(party),
        }
    }
}

/// Canonicalize a region assignment: relabel in first-appearance order, so a *labelling* is not a *position*.
/// `{A: Bastion, B: Marksman}` and `{B: Bastion, A: Marksman}` are the same board; a memo that tells them apart
/// is paying for nothing.
pub fn canonical(regions: &[u8]) -> Vec<u8> {
    let mut map: HashMap<u8, u8> = HashMap::new();
    regions
        .iter()
        .map(|r| {
            let next = map.len() as u8;
            *map.entry(*r).or_insert(next)
        })
        .collect()
}

/// **Are `a` and `b` interchangeable targets?** - the same position AND the same body, so a strike aimed at one
/// yields an isomorphic successor to the same strike aimed at the other. Two such foes are the same target, and
/// offering both is wasted branching.
///
/// "Same position" is region + rank (what decides *how* it is reached - Melee / Raid / Clash);
/// "same body" is every stat that shapes the exchange (health, might, grit, cadence, finesse, armor), the reach
/// and shape flags (melee, ranged, aoe, horde), and the side. (There is no instinct to compare any more: every foe
/// runs the one disruption heuristic, so two same-stat foes script identically.) `tempo`/`pending` are absent on
/// purpose: both re-derive at the round Reset, so at any state a search visits they equal cadence / zero.
pub(super) fn interchangeable(board: &Board, a: usize, b: usize) -> bool {
    let (x, y) = (&board.units[a], &board.units[b]);
    !x.fallen
        && !y.fallen
        && board.regions[a] == board.regions[b]
        && board.ranks[a] == board.ranks[b]
        && x.side == y.side
        && x.health == y.health
        && x.might == y.might
        && x.grit == y.grit
        && x.cadence == y.cadence
        && x.finesse == y.finesse
        && x.armor == y.armor
        && x.melee == y.melee
        && x.ranged == y.ranged
        && x.aoe == y.aoe
        && x.horde == y.horde
}

/// A cheap analytic read of the `(downs, flips)` an aimed strike from `a` onto `t` banks this round - Might per
/// blow over the target's Grit, across the strikes the attacker's tempo buys (the opening blow the reach pays for,
/// plus one per poured card). No resolution: a heuristic estimate, not the solver.
fn strike_yield(board: &Board, a: usize, t: usize) -> (u32, u32) {
    let (att, tar) = (&board.units[a], &board.units[t]);
    let per_blow = att.might.saturating_sub(tar.armor);
    if per_blow == 0 {
        return (0, 0);
    }
    let strikes = 1 + att.tempo.saturating_sub(reach_cards(&board.units, a, t));
    if tar.horde {
        // No spill: an aimed blow fells one body per penetrating strike, a sweep clears the whole pack.
        if per_blow < tar.grit.max(1) {
            return (0, 0);
        }
        let felled = if att.aoe {
            tar.health
        } else {
            strikes.min(tar.health)
        };
        (u32::from(felled >= tar.health), felled)
    } else {
        let flips = (per_blow * strikes / tar.grit.max(1)).min(tar.health);
        (u32::from(flips >= tar.health), flips)
    }
}

/// **Would the one-ply greedy CROSS?** - the step policy's intent read, the surviving core of the old greedy
/// act. A vanguard crosses iff the best raid it could reach (the top `(downs, flips)` over enemy rearguards)
/// **strictly** beats the best strike available from home; **ties keep the body home** (clash-first, the wave
/// greedy's own preference, and where the old model's positional tiebreak - "the last screen stays, a tank
/// holds the front" - collapsed to: a penalty below a tie can never win a strict comparison).
///
/// No creature is *told* to hold - a Might-1 body simply scores holding above a raid worth nothing, and an
/// exposed back scores the same from either side of the line, so nobody crosses for what the line already
/// reaches.
pub fn wants_to_cross(board: &Board, i: usize) -> bool {
    let u = &board.units[i];
    if u.fallen || board.ranks[i] != Rank::Vanguard || !u.melee {
        return false;
    }
    let enemy = |t: usize| !board.units[t].fallen && board.units[t].side != u.side;
    // The raid read: the best yield over enemy rearguards across the gap.
    let raid = (0..board.units.len())
        .filter(|&t| {
            enemy(t) && board.ranks[t] == Rank::Rearguard && board.regions[t] != board.regions[i]
        })
        .map(|t| strike_yield(board, i, t))
        .max();
    let Some(raid) = raid else {
        return false;
    };
    // The stay-home read: the best strike available without leaving - an enemy in this region, a clashable
    // vanguard, or an exposed back.
    let stay = (0..board.units.len())
        .filter(|&t| {
            enemy(t)
                && (board.regions[t] == board.regions[i]
                    || board.ranks[t] == Rank::Vanguard
                    || (board.ranks[t] == Rank::Rearguard && !board.is_screened(t)))
        })
        .map(|t| strike_yield(board, i, t))
        .max()
        .unwrap_or((0, 0));
    raid > stay
}

/// **The catch instinct** - which enemy crosser (or, generally, which candidate) this body strikes: the pick
/// with the top `(downs, flips)` yield, lowest index breaking ties. `None` when the list is empty (or this body
/// cannot strike at all).
pub fn foe_catch(board: &Board, catcher: usize, crossers: &[usize]) -> Option<usize> {
    let u = &board.units[catcher];
    if u.fallen || (!u.melee && !u.ranged) {
        return None;
    }
    crossers
        .iter()
        .copied()
        .filter(|&m| !board.units[m].fallen && board.units[m].side != u.side)
        .max_by_key(|&m| {
            let (downs, flips) = strike_yield(board, catcher, m);
            (downs, flips, std::cmp::Reverse(m))
        })
}

// ---- resolution ----------------------------------------------------------------------------------------

/// One strike, recorded for the combat log: `attacker` landed `hits` blows on `target`. A renderer attributes
/// the round's damage from these (Might per blow is read from the attacker).
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub attacker: usize,
    pub target: usize,
    pub hits: u32,
}

/// **A contested reach, recorded for the log.** `attacker` committed `bid` (`cards x Finesse x body-count`) to
/// reach `target`; `evaded` says whether the target paid a slip to break it. This is the ONLY record of the slip
/// contest a renderer gets: an evaded reach lands no [`Hit`], so without the bid the log could show a Tempo pool
/// drained on both sides but never the two numbers that decided it (the bid, and the Finesse the slip weighed
/// against it). Both operands of the comparison, made visible.
#[derive(Clone, Copy, Debug)]
pub struct Reach {
    pub attacker: usize,
    pub target: usize,
    pub bid: u32,
    pub evaded: bool,
}

/// Build the [`Reach`] records for one contest: every engagement that was formed, tagged with whether the target
/// slipped it (present in `reaching` but not in the post-evade `landed` set).
fn reaches_of(reaching: &[Contact], landed: &[Contact]) -> Vec<Reach> {
    reaching
        .iter()
        .map(|c| Reach {
            attacker: c.attacker,
            target: c.target,
            bid: c.bid,
            evaded: !landed
                .iter()
                .any(|l| l.attacker == c.attacker && l.target == c.target),
        })
        .collect()
}

/// What happened in one sub-phase - enough for a transcript or a renderer to say *why* the board changed.
#[derive(Clone, Debug, Default)]
pub struct SubPhaseLog {
    /// **Which step of the round this is** - e.g. `"Inner"`, `"Cross"`, `"Late Trade"`. Set by the step
    /// resolvers ([`super::steps`]) so a transcript can say *where* in the round every strike and card-flip
    /// happened, not just *that* it happened. Empty on a log built outside the step schedule.
    pub phase: &'static str,
    /// Got through - standing somewhere new now.
    pub through: Vec<usize>,
    /// Turned and fought instead: it stayed where it was.
    pub aborted: Vec<usize>,
    /// **Withdrew from the enemy ranks** at the Withdraw step - an outrider that declared the move and lived
    /// to make it, rejoining its own line at weapon rank.
    pub withdrew: Vec<usize>,
    /// Promoted from back to front, because the front ahead of it collapsed.
    pub promoted: Vec<usize>,
    pub fallen: Vec<usize>,
    /// **Every body's health at this boundary** - a snapshot, so a transcript can show what the board looked
    /// like *here* rather than at the end of the round.
    ///
    /// Without it a caller can only read the *final* board, and every sub-phase line prints identically - which
    /// hid the fact that all the damage was landing in one place. A log you cannot trust to say *when* is worse
    /// than no log.
    pub health: Vec<u32>,
    /// **Every body's Tempo at this boundary** - a snapshot, same shape and motive as `health`. The diff against
    /// the phase before it is what a body *spent* this phase (a reach, a pour, a slip); without it a transcript
    /// can say a blow landed but never what it cost to land.
    pub tempo: Vec<u32>,
    /// Every body's **rank** at this boundary. Same reason: without it a promotion that happens in the last
    /// sub-phase appears to have been true all round.
    pub ranks: Vec<Rank>,
    /// Every body's **region** at this boundary. Same snapshot discipline: a crossing or a dissolution moves a
    /// body, and a transcript that only reads the final board cannot say *which phase* it moved in.
    pub regions: Vec<u8>,
    /// **Every strike that landed in this sub-phase**, source-attributed: who hit whom, and how many blows. A
    /// renderer reads Might per blow from the attacker to say *where* each body's damage came from (a sweep
    /// records one `Hit` per swept contact, so an area strike is attributed to its sweeper too).
    pub hits: Vec<Hit>,
    /// **Every contested reach this sub-phase**, with its bid and whether it was slipped ([`Reach`]). Carries the
    /// slip-contest numbers a `Hit` cannot: an evaded reach lands nothing, so this is where the bid it committed -
    /// and, read against the target's Finesse, the slip that beat it - become legible.
    pub reaches: Vec<Reach>,
}

fn slip_price(bid: u32, f_def: u32) -> u32 {
    bid / f_def.max(1) + 1
}

/// **Commit tempo to reach a target** - [`combat::resolve_engage`] with the *rank model's screen taken out*.
///
/// We cannot call `combat::resolve_engage` here, and the reason is worth stating: it runs `back_access_ok`,
/// which is the **old rank model's** back-access rule - it silently discards any engagement aimed at a
/// `Rank::Rearguard` while that side still has a living `Rank::Vanguard`. That is a *screen*, and this model
/// already has one: [`Rank`] plus the slip contest. Inheriting the old one on top of it made a screened body
/// **unreachable by any raid at all** - the exact fiat this whole redesign exists to remove, smuggled back in
/// through a helper. (It silently deleted the Outrider a second time. Caught by
/// `evading_the_line_reaches_the_body_behind_it`.)
///
/// Everything else is identical: the tempo is spent whatever happens, and committing more buys **no extra
/// damage** - only a bid the defender must strictly beat. Every card sunk into *reaching* is a card you cannot
/// convert into a *blow*, and that is the whole attack decision.
fn engage(board: &mut Board, engagements: &[Engage]) -> Vec<Contact> {
    let mut contacts = Vec::new();
    for e in engagements {
        let u = &board.units[e.attacker];
        if u.fallen || !(u.melee || u.ranged) {
            continue;
        }
        let cards = e.cards.min(u.tempo);
        if cards == 0 {
            continue; // you cannot reach for someone without committing to it
        }
        let finesse = u.finesse.max(1);
        // A horde reaches as one: every living body grabs together, so a single tempo card's bid is multiplied by
        // the whole body count. That is what makes a swarm **hard to slip** - both when it catches a crosser and
        // when it pins its own target - even though its Cadence tempo is small. (Its damage is likewise the whole
        // swarm at once; see `land`.) The defence side gets no such multiplier (see `slip_cost`): a swarm is a
        // fearsome catcher and a poor evader.
        let mult = if u.horde { u.health.max(1) } else { 1 };
        board.units[e.attacker].tempo -= cards;
        contacts.push(Contact {
            attacker: e.attacker,
            target: e.target,
            bid: cards * finesse * mult,
        });
    }
    contacts
}

/// The greedy reach: the fewest cards the target cannot afford to evade - so it lands for certain - else one card
/// and take the chance. Every card saved becomes a blow.
pub fn reach_cards(units: &[Combatant], a: usize, t: usize) -> u32 {
    if units[a].aoe {
        return 1; // an area strike forms no contact and cannot be evaded
    }
    // A horde's bid is amplified by its body count (see `engage`), so it pins a target with far fewer cards.
    let bodies = if units[a].horde {
        units[a].health.max(1)
    } else {
        1
    };
    let per_card = units[a].finesse.max(1) * bodies;
    (1..=units[a].tempo)
        .find(|&c| slip_price(c * per_card, units[t].finesse) > units[t].tempo)
        .unwrap_or(1)
}

/// The greedy dodge for an ordinary exchange (not a slip): stand if you can answer along the edge, else evade if
/// you can afford it and the blow really threatens you.
fn dodges_against(board: &Board, reaching: &[Contact]) -> Vec<Dodge> {
    (0..board.units.len())
        .map(|i| {
            let Some(cost) = slip_cost(&board.units, reaching, i) else {
                return Dodge::Stand;
            };
            if board.units[i].fallen || cost > board.units[i].tempo {
                return Dodge::Stand;
            }
            if can_answer(&board.units, reaching, i).is_some() {
                return Dodge::Stand; // an edge you can swing along beats an escape
            }
            let worst = reaching
                .iter()
                .filter(|c| c.target == i)
                .map(|c| board.units[c.attacker].might)
                .max()
                .unwrap_or(0);
            if worst >= board.units[i].grit.max(1) {
                Dodge::Slip
            } else {
                Dodge::Stand
            }
        })
        .collect()
}

/// **An area strike nukes a whole region** - every enemy in it, *both tiers*, at full Might, unevadable, for one
/// tempo card. It **bypasses the screen entirely**.
///
/// That is the anti-cluster counter, and it is what prices the whole formation: pile bodies behind a vanguard and
/// you become a **target**. A bodyguard soaks an aimed blow but cannot cover an area, so concentration and
/// coverage genuinely trade - decided by a positional fact the player controls.
///
/// It deliberately does **not** carry the product's "one sweep clears the whole pack" horde rule: region-wide,
/// that let one Salvo delete sixteen bodies for a single card and collapsed seven of eight encounters to a
/// round-one wipe. A horde takes a sweep like anything else, spilling body to body.
///
/// Returns the aimed sweep contacts (normal bodies, resolved in [`land`]) and, separately, the horde kills as
/// [`Hit`]s: a horde is cleared here directly rather than through `land`, so its attribution is recorded here or
/// nowhere. The kills are recording-only - the health was already zeroed - so a renderer can name the sweeper.
fn area_strike(
    board: &mut Board,
    attacker: usize,
    region: u8,
    tier: Rank,
    both_tiers: bool,
) -> (Vec<Contact>, Vec<Hit>) {
    if board.units[attacker].fallen || board.units[attacker].tempo == 0 {
        return (Vec::new(), Vec::new());
    }
    board.units[attacker].tempo -= 1;
    let side = board.units[attacker].side;
    let depth = AREA_REACH.with(|r| *r.borrow());
    // A melee sweep catches both tiers (the striker is past the screen); a Clash/Raid sweep catches only the
    // tier it was aimed at, unless the WholeRegion probe knob overrides it.
    let whole = both_tiers || depth == AreaReach::WholeRegion;
    let might = board.units[attacker].might;

    let caught: Vec<usize> = board
        .in_region(region)
        .into_iter()
        .filter(|&j| board.units[j].side != side)
        .filter(|&j| whole || board.ranks[j] == tier)
        .collect();

    let mut contacts = Vec::new();
    let mut felled = Vec::new();
    for j in caught {
        if board.units[j].horde {
            // **A pack is many bodies, and an area strike catches every one of them at once.**
            //
            // Width is the sweep's axis: it hits every body in the tier simultaneously. Each body is a one-Health,
            // Grit-strong unit, so the sweep clears the WHOLE pack iff a single blow **penetrates** that Grit
            // (`Might - armor >= Grit`). A tough enough horde shrugs a weak sweep off entirely - and is ground down
            // instead by aimed fire, which banks into its pile a Grit at a time ([`land`]).
            if might.saturating_sub(board.units[j].armor) >= board.units[j].grit.max(1) {
                let bodies = board.units[j].health;
                if bodies > 0 {
                    // Record the clear but DO NOT apply it yet. The whole exchange is one commit-batch, so a sweep
                    // must not shrink a swept horde's OWN volley - its body count is read at commit ([`land`], and
                    // the multiplier in [`engage`]). `exchange` applies these clears after `land`, alongside every
                    // other blow in the batch, so simultaneity holds (Spec 1.9).
                    felled.push(Hit {
                        attacker,
                        target: j,
                        hits: bodies,
                    });
                }
            }
        } else {
            contacts.push(Contact {
                attacker,
                target: j,
                bid: 0, // no bid: it cannot be evaded, and nobody answers along it
            });
        }
    }
    (contacts, felled)
}

/// **How far an area strike reaches.** The rule is [`FrontLine`](AreaReach::FrontLine), and it follows from one
/// sentence:
///
/// > **An area strike multiplies your TARGETS. It does not extend your REACH.**
/// > A body you could not single-target, you cannot sweep.
///
/// Reach is what the screen governs; width is what an area strike governs. They are **different axes**, and a
/// sweep that reached through a line was quietly buying both. That is not a balance number - it is a category
/// error, and no amount of tuning fixes a category error. (We tried: the sweep was measured at full Might, half
/// Might and Might 1, and *none* of it changed whether a raid was ever necessary. Of course it did not. Turning
/// a sweep's damage down changes how fast the back line dies, never whether it is **reachable**.)
///
/// So a sweep hits every enemy **in the tier it was aimed at**, in that region:
///
/// - **Clash** their front, and you sweep their whole front line. Wide, and free of the screen - because their
///   front was never behind anything.
/// - **Raid** their back, and you sweep their whole back line - because you *are standing in it now*. The reach
///   was paid for at the line, exactly like any other blow. Width came free; depth did not.
///
/// It leaves an area strike as the honest anti-cluster counter (bunch up and one card catches all of you) while
/// leaving the **raid as the only door to a screened body** - which is what a screen is *for*. It also makes the
/// Bastion's Sweep have to **go in after** the back-line Swarm it is built to answer, rather than lobbing a
/// sweep through the wall from outside.
///
/// [`WholeRegion`] is retained only as a probe knob, to keep the comparison re-runnable rather than
/// re-litigable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaReach {
    /// **The rule.** A sweep hits every enemy in the tier it was aimed at. Width, never depth.
    FrontLine,
    /// Both tiers - an area strike ignores the screen entirely. **Not the rule**; kept so the measurement that
    /// rejected it can be re-run.
    WholeRegion,
}

thread_local! {
    static AREA_REACH: std::cell::RefCell<AreaReach> = const { std::cell::RefCell::new(AreaReach::FrontLine) };
}

/// Override the [`AreaReach`] rule for this thread. A **probe knob, not a game setting** - it exists so the two
/// answers stay measurable against each other rather than arguable.
pub fn set_area_reach(reach: AreaReach) {
    AREA_REACH.with(|r| *r.borrow_mut() = reach);
}

/// Land a set of blows: each contact's opening blow, plus whatever `extra` tempo is poured after it. One
/// order-free, commit-based batch - a blow lands even if its striker died to a simultaneous one.
///
/// **`extra` is passed in, never inferred.** It used to be derived with `combat::strike_target`, which answers
/// "who am I in contact with?" by taking the **first** matching contact - iteration order. A body closed on from
/// two sides therefore poured its entire pool into whichever attacker happened to be seated first, so **who died
/// depended on who was at index 0**. Spec 1.9 forbids exactly that ("permuting the seat order must yield the
/// identical end-state; any divergence is an order-dependent mechanic, i.e. a bug"), and it is the kind of bug
/// that hides forever because every rule test still passes.
///
/// The fix is not to sort the contacts. It is that *whom you swing at* is a **real decision**, and this model
/// already has a place to make it: your declaration. So you pour into **what you declared**, and nothing else
/// decides it for you.
///
/// (`combat::strike_target` still has this flaw, and the shipped rank model still calls it - so the live game
/// has the same order-dependence whenever two bodies clash one. Out of scope here; flagged, not touched.)
///
/// # The damage model: a horde's bodies are SEPARATE Grit-strong pools (no spill)
///
/// **Each of a horde's Health cards is a body, of the same Grit, with its OWN pile.** A `[6 Vitality, 4 Grit,
/// horde]` foe is six one-Health bodies each `Grit 4` strong, acting in unison. Damage does **not spill** between
/// them: an aimed blow fells at most **one** body, and only if it *penetrates* that body's Grit
/// (`Might - armor >= Grit`). Overkill is wasted, so a big Might does not out-kill tempo - **to fell another body
/// you spend another blow** (another tempo). A sub-Grit blow dents nothing.
///
/// So the two counters split cleanly by *width*, not raw power:
/// - **Aimed fire** grinds a pack **one body per blow** - `K` bodies cost ~`K` tempo. Expensive against a swarm.
/// - **A sweep** ([`area_strike`]) hits every body at once, clearing the WHOLE pack for one card - but only if it
///   penetrates. That breadth is the sweep's whole job against a horde.
///
/// (A **normal** body is the familiar pile: bank `max(0, Might - armor)` per blow, flip a Health card each time
/// the pile clears Grit. The pile closes each sub-phase, so an unfinished wound is not inflicted.)
fn land(board: &mut Board, contacts: &[Contact], sweeps: &[Contact], extra: &[Blows]) -> Vec<Hit> {
    // Collect every blow first, apply nothing yet: an order-free, commit-based batch, so a blow lands even if
    // its striker dies to a simultaneous one.
    let n = board.units.len();
    let mut damage = vec![0u32; n]; // banked Might, against a normal body's Grit pile
    let mut felled = vec![0u32; n]; // bodies struck off a horde - one per PENETRATING blow, no spill between them

    // Gather every strike as `(attacker, target, hits)` first, so nothing is applied while we are still reading.
    let mut strikes: Vec<(usize, usize, u32)> = Vec::new();

    // Each established contact gives its engager one opening blow - already paid for by the tempo it committed.
    for c in contacts.iter().chain(sweeps) {
        strikes.push((c.attacker, c.target, 1));
    }
    // ...and then whatever tempo was poured after it, one strike per card.
    for b in extra {
        let cards = b.cards.min(board.units[b.unit].tempo);
        board.units[b.unit].tempo -= cards;
        strikes.push((b.unit, b.target, cards));
    }

    // The source-attributed transcript: one entry per landed strike (`hits > 0`), so a renderer can say WHERE a
    // body's damage came from. Recorded whether or not the blow flips a card - banked sub-Grit damage still came
    // from that attacker.
    let mut log: Vec<Hit> = Vec::new();
    for (a, t, hits) in strikes {
        if hits == 0 {
            continue;
        }
        log.push(Hit {
            attacker: a,
            target: t,
            hits,
        });
        let per_body = board.units[a].might.saturating_sub(board.units[t].armor);
        if board.units[t].horde {
            // Separate per-body pools, no spill: each blow fells ONE body iff it penetrates that body's Grit. A
            // sub-Grit blow does nothing (overkill and under-kill are both wasted); another body needs another blow.
            if per_body >= board.units[t].grit.max(1) {
                felled[t] += hits;
            }
        } else {
            // A horde ATTACKER swings as ONE volley: every living body lands together, so its blow is the whole
            // body count times Might. Armour stops each little hit, so it is subtracted **per body**.
            let per = if board.units[a].horde {
                per_body * board.units[a].health.max(1)
            } else {
                per_body
            };
            damage[t] += per * hits;
        }
    }

    for i in 0..n {
        if board.units[i].horde {
            board.units[i].health = board.units[i].health.saturating_sub(felled[i]);
        } else if damage[i] > 0 {
            // The grit pile: bank the Might, flip a Health card each time it clears the bar. It closes at the
            // sub-phase boundary, so a wound you cannot finish is a wound you did not inflict.
            let bar = board.units[i].grit.max(1);
            board.units[i].pending += damage[i];
            while board.units[i].pending >= bar && board.units[i].health > 0 {
                board.units[i].pending -= bar;
                board.units[i].health -= 1;
            }
        }
    }
    log
}

/// One body attacking another: `(attacker, target)`.
pub(super) type Attack = (usize, usize);

/// The blows a body pours **into the target it declared**, once its reach is paid for. No pour without a
/// declaration: a body that declared `Hold` holds.
fn poured(board: &Board, attacks: &[Attack], contacts: &[Contact]) -> Vec<Blows> {
    attacks
        .iter()
        .filter(|&&(a, t)| {
            !board.units[a].fallen
                && !board.units[a].horde // a horde swings ONE volley (body-count x Might); it does not pour extra
                && board.units[a].tempo > 0
                && contacts.iter().any(|c| c.attacker == a && c.target == t)
        })
        .map(|&(a, t)| Blows {
            unit: a,
            target: t,
            cards: board.units[a].tempo,
        })
        .collect()
}

/// Close a sub-phase: **finalize deaths** and snapshot the boundary. Finalizing here is how the ground behind a
/// broken line opens up *within* a round - a rearguard whose vanguard just fell becomes exposed (clashable) the
/// moment that death is settled, no promotion needed.
pub(super) fn close(board: &mut Board, before: &[bool]) -> SubPhaseLog {
    end_sub_phase(&mut board.units);
    SubPhaseLog {
        fallen: (0..board.units.len())
            .filter(|&i| before[i] && board.units[i].fallen)
            .collect(),
        health: board.units.iter().map(|u| u.health).collect(),
        tempo: board.units.iter().map(|u| u.tempo).collect(),
        ranks: board.ranks.clone(),
        regions: board.regions.clone(),
        ..Default::default()
    }
}

pub(super) fn living(board: &Board) -> Vec<bool> {
    board.units.iter().map(|u| !u.fallen).collect()
}

/// **The outrider state dissolves when its host is gone.** "Outrider" means *intermingled with an enemy
/// formation, past their screen* - so a body loose in a zone with **no enemy formation left** (its havoc wiped
/// the last of it) is an outrider of nothing. The state ends: the body reverts to its weapon rank and, if its
/// side still holds a **line** elsewhere, **rejoins it** - there is no ground to garrison in a two-formation
/// fight, only a line to hold. A body that is the *last* of its side simply becomes the formation where it
/// stands. Called at the Inner Ring boundary, where an outrider's havoc is what wipes a formation.
///
/// (This replaced the old *promotion* - "clearing a zone takes the ground" - a multi-region leftover that, with
/// one region per side, mostly just coincided with the win and mishandled a mutual raid.)
pub(super) fn dissolve(board: &mut Board) {
    for r in board.occupied() {
        let bodies = board.in_region(r);
        if bodies.iter().any(|&i| board.ranks[i] != Rank::Outrider) {
            continue; // a formation still holds this zone - outriders here stay loose among it
        }
        // No formation here: every body is a loose outrider with no host, so each dissolves.
        for i in bodies {
            let home = home_of(board, board.units[i].side, r);
            board.ranks[i] = Board::weapon_rank(&board.units[i]);
            if let Some(home) = home {
                board.regions[i] = home; // rejoin your own line
            }
            // else: you are the last of your side - become the formation where you stand.
        }
    }
}

/// A region OTHER than `avoid` where `side` still stands as a **formation** (a living non-outrider body), or
/// `None` if `side` holds no line to rejoin.
pub(super) fn home_of(board: &Board, side: Side, avoid: u8) -> Option<u8> {
    (0..board.units.len())
        .find(|&i| {
            !board.units[i].fallen
                && board.units[i].side == side
                && board.ranks[i] != Rank::Outrider
                && board.regions[i] != avoid
        })
        .map(|i| board.regions[i])
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged.** Area strikes split off and
/// sweep their target's region: the tier aimed at, or - when `sweep_whole` - both tiers (an in-region melee, past
/// the screen).
pub(super) fn exchange(
    board: &mut Board,
    attacks: &[Attack],
    pour: bool,
    sweep_whole: bool,
) -> (Vec<Hit>, Vec<Reach>) {
    let mut sweeps: Vec<Contact> = Vec::new();
    let mut sweep_hits: Vec<Hit> = Vec::new();
    let mut aimed: Vec<Engage> = Vec::new();
    for &(a, t) in attacks {
        if board.units[a].aoe {
            let (region, tier) = (board.regions[t], board.ranks[t]);
            let (contacts, felled) = area_strike(board, a, region, tier, sweep_whole);
            sweeps.extend(contacts);
            sweep_hits.extend(felled);
        } else {
            aimed.push(Engage {
                attacker: a,
                target: t,
                cards: reach_cards(&board.units, a, t),
            });
        }
    }
    let reaching = engage(board, &aimed);
    let dodges = dodges_against(board, &reaching);
    let contacts = resolve_evade(&mut board.units, &reaching, &dodges);
    let reaches = reaches_of(&reaching, &contacts); // the aimed slip contest, bids and who slipped

    // Pour into what you DECLARED - never into whoever the contact list happened to list first.
    let extra: Vec<Blows> = if pour {
        poured(board, attacks, &contacts)
    } else {
        Vec::new()
    };
    let mut hits = land(board, &contacts, &sweeps, &extra);
    // Apply the sweep's horde clears NOW - after `land` has read every attacker's body count at commit, so a horde
    // swept in this same batch still delivered its full volley (commit-batch simultaneity, Spec 1.9).
    for h in &sweep_hits {
        board.units[h.target].health = board.units[h.target].health.saturating_sub(h.hits);
    }
    hits.extend(sweep_hits); // horde kills are recorded in area_strike for the transcript
    (hits, reaches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::steps::{StepScript, play_steps};

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    // ---- geometry -------------------------------------------------------------------------------------

    /// Rank is derived from the weapon, never chosen: ranged-only stands at the back, anything that can strike
    /// in melee (including a dual) stands at the front, and nobody starts an outrider.
    #[test]
    fn rank_is_derived_from_the_weapon() {
        let b = Board::new(
            vec![
                unit("Sword", Side::Party, [5, 5, 1, 2, 2], true, false),
                unit("Bow", Side::Party, [5, 5, 1, 2, 2], false, true),
                unit("Both", Side::Party, [5, 5, 1, 2, 2], true, true),
            ],
            vec![0, 0, 0],
        );
        assert_eq!(
            b.ranks,
            vec![Rank::Vanguard, Rank::Rearguard, Rank::Vanguard]
        );
    }

    /// A rearguard is screened exactly while its side still has a living vanguard in its region: the screen is
    /// bodies, not a rule, so it is gone the moment the last vanguard falls - no promotion, just exposure.
    #[test]
    fn the_screen_is_bodies_not_a_rule() {
        let mut b = Board::new(
            vec![
                unit("Wall", Side::Foe, [1, 3, 4, 1, 2], true, false),
                unit("Sniper", Side::Foe, [5, 1, 1, 2, 3], false, true),
            ],
            vec![0, 0],
        );
        assert!(b.is_screened(1), "the wall screens its sniper");
        b.units[0].fallen = true;
        assert!(
            !b.is_screened(1),
            "the screen died with the body doing the screening"
        );
        assert_eq!(
            b.ranks[1],
            Rank::Rearguard,
            "the exposed back is NOT promoted - it keeps its rank and its early fire"
        );
    }

    /// `canonical` relabels regions in first-appearance order, so two labellings of the same partition memoize
    /// as the same position.
    #[test]
    fn canonical_is_the_partition_not_the_labels() {
        assert_eq!(canonical(&[7, 7, 2, 2]), vec![0, 0, 1, 1]);
        assert_eq!(canonical(&[2, 2, 7, 7]), vec![0, 0, 1, 1]);
        assert_eq!(canonical(&[1, 0, 1]), vec![0, 1, 0]);
    }

    /// Two same-stat, same-position enemies are one target; any stat, flag, or position difference splits them.
    #[test]
    fn interchangeable_collapses_true_twins_only() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [6, 6, 1, 2, 2], true, false),
                unit("Rat", Side::Foe, [2, 3, 1, 1, 1], true, false),
                unit("Rat", Side::Foe, [2, 3, 1, 1, 1], true, false),
            ],
            vec![0, 1, 1],
        );
        assert!(interchangeable(&b, 1, 2), "true twins are one target");
        b.units[2].health = 2;
        assert!(
            !interchangeable(&b, 1, 2),
            "a wounded twin is its own target"
        );
    }

    // ---- the damage model -----------------------------------------------------------------------------

    /// The step schedule is order-free: permuting the seat order of the units yields the identical end-state
    /// (Spec 1.9). Two attackers close on one wall from both seat orders.
    #[test]
    fn the_exchange_is_order_free() {
        let a = unit("RaiderA", Side::Party, [4, 6, 1, 2, 2], true, false);
        let b = unit("RaiderB", Side::Party, [3, 6, 1, 2, 2], true, false);
        let w = unit("Wall", Side::Foe, [2, 5, 5, 2, 2], true, false);

        let mut fwd = Board::new(vec![a.clone(), b.clone(), w.clone()], vec![0, 0, 1]);
        let mut rev = Board::new(vec![b, a, w], vec![0, 0, 1]);
        let script = StepScript::default();
        play_steps(&mut fwd, &script);
        play_steps(&mut rev, &script);

        // Same partition, mirrored seats: A/B swap indexes, the wall keeps its own.
        assert_eq!(fwd.units[2].health, rev.units[2].health, "the wall's fate");
        assert_eq!(fwd.units[0].health, rev.units[1].health, "RaiderA's fate");
        assert_eq!(fwd.units[1].health, rev.units[0].health, "RaiderB's fate");
    }

    /// **A horde's bodies are separate Grit-strong pools, no spill**: an aimed penetrating blow fells exactly one
    /// body per strike (overkill wasted), and a sub-Grit blow fells nothing at all no matter how many land.
    #[test]
    fn horde_bodies_do_not_spill() {
        let mut pen = unit("Giant", Side::Party, [9, 9, 3, 2, 1], true, false);
        pen.tempo = 2; // reach (1 card) + pour 1 = 2 strikes this batch
        let mut pack = unit("Pack", Side::Foe, [1, 6, 3, 1, 1], true, false);
        pack.horde = true;
        let mut b = Board::new(vec![pen, pack.clone()], vec![0, 1]);
        exchange(&mut b, &[(0, 1)], true, false);
        assert_eq!(
            b.units[1].health, 4,
            "two penetrating strikes fell exactly two bodies - Might 9 overkill is wasted"
        );

        let mut weak = unit("Jab", Side::Party, [2, 9, 3, 4, 1], true, false);
        weak.tempo = 4;
        let mut b = Board::new(vec![weak, pack], vec![0, 1]);
        exchange(&mut b, &[(0, 1)], true, false);
        assert_eq!(
            b.units[1].health, 6,
            "Might 2 never penetrates Grit 3: four strikes fell nothing"
        );
    }

    /// **A horde attacker swings one volley**: body-count x Might in a single blow, so a big pack cracks what no
    /// single member could dent.
    #[test]
    fn a_horde_strikes_as_one_volley() {
        let mut pack = unit("Pack", Side::Foe, [1, 6, 3, 1, 1], true, false);
        pack.horde = true;
        let hero = unit("Hero", Side::Party, [1, 5, 5, 1, 2], true, false);
        let mut b = Board::new(vec![hero, pack], vec![0, 1]);
        exchange(&mut b, &[(1, 0)], true, false);
        // 6 bodies x Might 1 = 6 banked against Grit 5: one card flips, 1 remains in the pile.
        assert_eq!(b.units[0].health, 4, "the volley is the whole pack at once");
    }

    /// An area strike hits the tier it was aimed at - width, never depth ([`AreaReach::FrontLine`]): a sweep
    /// aimed at the front does not reach the screened back behind it.
    #[test]
    fn a_sweep_multiplies_targets_not_reach() {
        let mut bomber = unit("Bomber", Side::Party, [6, 3, 1, 2, 2], false, true);
        bomber.aoe = true;
        let mut b = Board::new(
            vec![
                bomber,
                unit("WallA", Side::Foe, [1, 3, 2, 1, 2], true, false),
                unit("WallB", Side::Foe, [1, 3, 2, 1, 2], true, false),
                unit("Sniper", Side::Foe, [5, 3, 1, 2, 3], false, true),
            ],
            vec![0, 1, 1, 1],
        );
        exchange(&mut b, &[(0, 1)], true, false); // aimed at the front tier
        assert!(
            b.units[1].health < 3 && b.units[2].health < 3,
            "the sweep catches the WHOLE front line"
        );
        assert_eq!(
            b.units[3].health, 3,
            "the screened back is beyond the sweep's reach"
        );
    }

    /// A sweep clears a whole pack iff a single blow penetrates the per-body Grit; a tough pack shrugs it off.
    #[test]
    fn a_sweep_clears_a_pack_only_by_penetrating() {
        let mut bomber = unit("Bomber", Side::Party, [4, 3, 1, 2, 2], false, true);
        bomber.aoe = true;
        let mut pack = unit("Pack", Side::Foe, [1, 8, 3, 1, 1], true, false);
        pack.horde = true;
        let mut tough = unit("Shells", Side::Foe, [1, 8, 5, 1, 1], true, false);
        tough.horde = true;

        let mut b = Board::new(vec![bomber.clone(), pack], vec![0, 1]);
        exchange(&mut b, &[(0, 1)], true, false);
        assert_eq!(
            b.units[1].health, 0,
            "Might 4 >= Grit 3: the whole pack falls at once"
        );

        let mut b = Board::new(vec![bomber, tough], vec![0, 1]);
        exchange(&mut b, &[(0, 1)], true, false);
        assert_eq!(
            b.units[1].health, 8,
            "Might 4 < Grit 5: the sweep dents nothing"
        );
    }

    /// The grit pile closes at the sub-phase boundary: banked sub-Grit damage is discarded, so a wound you
    /// cannot finish within the batch is a wound you did not inflict.
    #[test]
    fn the_grit_pile_closes_at_the_boundary() {
        let jab = unit("Jab", Side::Party, [2, 5, 1, 1, 2], true, false);
        let wall = unit("Wall", Side::Foe, [1, 3, 5, 1, 2], true, false);
        let mut b = Board::new(vec![jab, wall], vec![0, 1]);
        let before = living(&b);
        exchange(&mut b, &[(0, 1)], true, false);
        assert_eq!(b.units[1].pending, 2, "2 banked against the Grit-5 bar");
        close(&mut b, &before);
        assert_eq!(
            b.units[1].pending, 0,
            "the boundary sweeps the unfinished pile"
        );
        assert_eq!(b.units[1].health, 3, "no card flipped");
    }

    /// `reach_cards` bids the fewest cards the target cannot afford to slip; a horde's body-count multiplier
    /// makes its single card a pinning bid.
    #[test]
    fn reach_bids_just_enough_to_pin() {
        let a = unit("Raider", Side::Party, [6, 6, 1, 4, 2], true, false);
        let slippery = unit("Dancer", Side::Foe, [2, 3, 1, 3, 4], true, false);
        let units = vec![a, slippery];
        // Every affordable bid leaves the Dancer a slip it can pay - so bid the minimum and take the chance.
        assert_eq!(
            reach_cards(&units, 0, 1),
            1,
            "cannot price the dodge out - bid the minimum"
        );

        let mut pack = unit("Pack", Side::Party, [1, 6, 3, 1, 1], true, false);
        pack.horde = true;
        let units = vec![pack, unit("Hero", Side::Foe, [5, 5, 1, 2, 2], true, false)];
        // 1 card x Finesse 1 x 6 bodies = bid 6 -> slip price 6/2+1 = 4 > tempo 2: pinned by one card.
        assert_eq!(
            reach_cards(&units, 0, 1),
            1,
            "the pack pins with a single card"
        );
    }

    // ---- the instinct reads ---------------------------------------------------------------------------

    /// `foe_catch` picks the candidate with the top (downs, flips) yield; a down beats any dent.
    #[test]
    fn foe_catch_takes_the_best_yield() {
        let b = Board::new(
            vec![
                unit("Brute", Side::Foe, [5, 5, 1, 2, 2], true, false),
                unit("Tank", Side::Party, [3, 9, 5, 2, 2], true, false),
                unit("Wisp", Side::Party, [5, 1, 1, 2, 2], true, false),
            ],
            vec![0, 1, 1],
        );
        assert_eq!(
            foe_catch(&b, 0, &[1, 2]),
            Some(2),
            "the one it can DOWN beats the one it can only dent"
        );
    }

    /// `wants_to_cross`: a striker crosses for a back it can down when home yields nothing; ties stay home, so
    /// nobody crosses for an exposed back the line already reaches; and a body that cannot dent the back at all
    /// never crosses.
    #[test]
    fn crossing_intent_is_a_strict_yield_comparison() {
        // A lone striker with a juicy enemy back and a wall it cannot dent: cross.
        let b = Board::new(
            vec![
                unit("Raider", Side::Party, [6, 6, 1, 3, 2], true, false),
                unit("Wall", Side::Foe, [1, 4, 7, 1, 2], true, false),
                unit("Sniper", Side::Foe, [5, 1, 1, 2, 3], false, true),
            ],
            vec![0, 1, 1],
        );
        assert!(
            wants_to_cross(&b, 0),
            "the raid downs the sniper; home yields nothing"
        );

        // The same back, EXPOSED (its wall dead): reachable from the line, so the yields tie - stay home.
        let mut b = b.clone();
        b.units[1].fallen = true;
        assert!(
            !wants_to_cross(&b, 0),
            "nobody crosses for what the line already reaches - ties stay home"
        );

        // A Might-1 body downs nothing anywhere: it never crosses.
        let b = Board::new(
            vec![
                unit("Pillar", Side::Party, [1, 6, 5, 2, 2], true, false),
                unit("Wall", Side::Foe, [1, 4, 7, 1, 2], true, false),
                unit("Sniper", Side::Foe, [5, 1, 5, 2, 3], false, true),
            ],
            vec![0, 1, 1],
        );
        assert!(
            !wants_to_cross(&b, 0),
            "a raid worth nothing never beats holding the line"
        );
    }
}
