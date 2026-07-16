//! **The regions model** - the candidate successor to the rank + 5-sub-phase schedule.
//! Design: `needs-merge/regions-and-relations-combat.md`.
//!
//! Additive and **inert**: nothing shipped calls it. It reuses [`crate::combat`]'s resolvers (Engage / Evade /
//! Strike, the grit pile, the round Reset) unchanged - only the *geometry* and the *schedule* are new, so the
//! two models can sit side by side without either disturbing the other.
//!
//! # The problem it exists to solve
//!
//! The five sub-phases (Intercept / Volley / Raid / Clash / Breach) describe exactly one situation: bodies
//! moving through open ground. That is perfect in round 1 - the lines are closing - and nonsense by round 3,
//! when nobody is in transit and the game runs the crossing gauntlet anyway. The spec **re-rolls** the formation
//! every round (no continuity); the product **freezes** it forever (no evolution). Neither has any transit after
//! round 1, and the schedule is made of nothing but transit.
//!
//! The fix is neither. **Declare the formation once, then let it evolve through play.**
//!
//! # The model
//!
//! **One region per side.** The party stands on its ground, the foes on theirs - two formations facing each
//! other, with no ground between. Each body is posted at the **front** or the **back** of its formation,
//! weapon-derived (a ranged-only body avoids melee at the back, everything else stands front). There is no
//! partition to choose and no setup phase: the fight opens on round 1's first declaration.
//!
//! **You declare only an action.** You start each round where you ended. **Position is never
//! declared; it is only ever earned.** That is what makes movement *priced by construction*: `v2_remarshal`
//! proved a costless repositioning offered every round can always be pre-empted by starting in the right place,
//! so it can never be *necessary*. Here you cannot ask to move; you have to win it.
//!
//! **A region is a formation, not a place.** It is not somewhere you travel to. It says only two things: who
//! screens you (your own front line), and who gets caught in one area strike (your group). Melee reaches any
//! enemy vanguard in any region - there is no ground between you.
//!
//! ## Three invariants that do most of the work
//!
//! - **A collapsed vanguard does not promote its rearguard.** When a front line dies its back **stays a back**
//!   (`Rearguard`): it keeps its phase slot and still fires first, it merely loses its screen and becomes
//!   clashable. That is what keeps range meaning something. (The one promotion that *does* happen -
//!   [`Board::promote`] - is a different thing: an outrider becoming the formation on ground it has cleared.)
//! - **A melee body at the back is dead weight.** It cannot attack; that is the price of hiding behind the
//!   vanguard. Nothing bans it - posting a Raider at the back simply punishes itself. Force, not fiat.
//! - **Slipping is the only movement, and it is one-way.** You cross into the enemy's ground and promote to an
//!   `Outrider`; there is no retreat back out. Reaching a screened body and going loose in the enemy ranks are
//!   the same mechanic.
//!
//! ## What a body can do ([`Act`])
//!
//! | | |
//! |---|---|
//! | [`Act::Clash`] | strike an enemy **vanguard**. Free - melee or ranged, any region. |
//! | [`Act::Raid`] | **slip** their front to strike an enemy **rearguard**, and end up standing in their region. Melee only. |
//! | [`Act::Slip`] | **slip** across into the enemy's ground, becoming an outrider. No retreat once inside. |
//! | [`Act::Hold`] | nothing. |
//!
//! A **rearguard cannot reach an enemy rearguard** - not until that side's vanguard collapses and its back is
//! promoted into view.
//!
//! ## The slip - the one contest, and the heart of the model
//!
//! A slip is opposed by **every enemy vanguard in the region you leave and the region you enter**: you are
//! outside your own screen the moment you move, so both ends reach for you. Then - *seeing exactly what was
//! committed against you* - you pick one of three ([`Answer`]):
//!
//! | | |
//! |---|---|
//! | **Evade** | pay [`combat::slip_cost`] in full. Through, untouched. Expensive. |
//! | **Push** | pay nothing. Through **anyway** - and eat every blow they land. |
//! | **Abort** | turn and fight. You stay where you were, you take the hits, and you swing back at whoever caught you. |
//!
//! **A vanguard can never actually stop you** - it can only make you bleed for it, or make turning back look
//! attractive. That is `force, not fiat` in its strongest form: the screen is a *price*, and you always get to
//! decide whether to pay it in tempo, in blood, or not at all.
//!
//! The old triangle falls out with no ranks anywhere:
//!
//! > `Raider > Cannon > Vanguard > Raider` - the front bleeds the raider, the raider reaches the cannon early,
//! > and only the cannon's Might cracks the front's Grit.
//!
//! ## The schedule - three sub-phases
//!
//! The razor (which `combat.rs` already states): **a sub-phase exists for exactly one reason - so a death in it
//! can silence something later.** Nothing else earns a boundary.
//!
//! | | | a death here silences |
//! |---|---|---|
//! | **Slip** | every slip contest, at once | the raider's strike |
//! | **Raid** | those who got through strike the backs they came for | that victim's own clash |
//! | **Clash** | everyone else trades | (last) |
//!
//! Damage closes at the **Round Reset** only - `combat.rs`'s rule, unchanged. Deaths finalize at each sub-phase
//! boundary, and a front that dies there **promotes its back on the spot** - which is how the ground behind a
//! broken line opens up *within* a round.

use std::collections::HashMap;

use super::resolve::{
    Combatant, Contact, Dodge, Engage, Instinct, Side, can_answer, end_sub_phase, refresh_round,
    resolve_evade, slip_cost,
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

/// **How a slipper answers the bodies reaching for it** - and it is a real decision, taken *after* seeing exactly
/// what was committed against it.
///
/// A vanguard can never simply **stop** you. It can only make you pay - in tempo, in blood, or in the chance you
/// gave up to turn and fight it instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Answer {
    /// **Pay [`combat::slip_cost`] in full.** Through, untouched - and poorer. There is no partial evade: by now
    /// the price is known exactly, so underpaying is never a gamble, only a waste.
    Evade,
    /// **Take the hit and go anyway.** Spend nothing on the crossing, eat every blow they land, and arrive - hurt,
    /// but with your whole pool still in hand to swing with.
    Push,
    /// **Turn and fight.** Give up the ground, take the hits, and spend your tempo swinging back at whoever caught
    /// you. This is the "repelled" outcome - but *chosen*, not imposed.
    Abort,
}

/// What a body does with its round. **The only thing it declares.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    /// Strike an enemy **vanguard**. Free - nothing is in the way. Melee or ranged, any region, because a region
    /// is a formation and not a place.
    Clash(usize),
    /// **Slip their front to strike an enemy rearguard**, and end the round standing inside their formation - in
    /// melee range of the vanguard you just went around. Thems the consequences.
    ///
    /// Melee only. You land as an [`Outrider`](Rank::Outrider): the rank is not a role you pick but *what
    /// reaching a screened body costs*.
    Raid(usize, Answer),
    /// **Strike a body in your OWN region** - an enemy outrider loose in your ranks, or (if you are the outrider)
    /// any host body. No screen applies in-region: the ranks stopped protecting anyone the moment a body got
    /// inside them. Not a crossing, so it carries no evade-answer.
    Melee(usize),
    /// **Slip into the enemy's ground** - cross the gap and stand in their formation, promoting to an
    /// [`Outrider`](Rank::Outrider). The same contest as a raid, only without a declared target. **Committed: an
    /// outrider cannot slip back out.**
    Slip(u8, Answer),
    /// Nothing.
    Hold,
}

impl Act {
    /// The region this act moves you to, if it moves you at all.
    fn destination(self, board: &Board, i: usize) -> Option<u8> {
        let here = board.regions[i];
        match self {
            Act::Raid(t, _) => (board.regions[t] != here).then(|| board.regions[t]),
            Act::Slip(r, _) => (r != here).then_some(r),
            _ => None,
        }
    }

    fn answer(self) -> Option<Answer> {
        match self {
            Act::Raid(_, a) | Act::Slip(_, a) => Some(a),
            _ => None,
        }
    }

    pub fn label(self, board: &Board) -> String {
        let how = |a: Answer| match a {
            Answer::Evade => "evade the line",
            Answer::Push => "push through, take the hits",
            Answer::Abort => "turn and fight if caught",
        };
        match self {
            Act::Clash(t) => format!("Clash {}", board.units[t].name),
            Act::Raid(t, a) => format!("Raid {} ({})", board.units[t].name, how(a)),
            Act::Melee(t) => format!("Melee {}", board.units[t].name),
            Act::Slip(r, a) => {
                let where_to = match board.in_region(r).first() {
                    Some(&i) => format!("to {}", board.units[i].name),
                    None => format!("to open ground {}", (b'A' + r) as char),
                };
                format!("Slip away {where_to} ({})", how(a))
            }
            Act::Hold => "Hold".to_string(),
        }
    }
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

// ---- what a body may declare ---------------------------------------------------------------------------

/// The three ways to answer a line reaching for you. Every slip offers all three - that is the point.
const ANSWERS: [Answer; 3] = [Answer::Evade, Answer::Push, Answer::Abort];

/// **Are `a` and `b` interchangeable targets?** - the same position AND the same body, so a strike aimed at one
/// yields an isomorphic successor to the same strike aimed at the other. Two such foes are the same target, and
/// offering both is wasted branching.
///
/// "Same position" is region + rank (what decides *how* it is reached - Melee / Raid / Clash);
/// "same body" is every stat that shapes the exchange (health, might, grit, cadence, finesse, armor), the reach
/// and shape flags (melee, ranged, aoe, horde), the side, AND the **instinct** (two same-stat foes that will
/// script differently are genuinely different bodies). `tempo`/`pending` are absent on purpose: both re-derive at
/// the round Reset, so at any state a search visits they equal cadence / zero.
fn interchangeable(board: &Board, a: usize, b: usize) -> bool {
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
        && x.instinct == y.instinct
}

/// **Every legal action for unit `i`** - the count-adaptive candidate list (spec 4.1: a choice is offered only
/// where it would do something). This is the branching factor, so it is the first place to look when a cost
/// report comes back bad.
///
/// The rules that keep it small are the design's, not optimizations:
///
/// - **A melee body at the back can only Hold or Slip out.** It is hiding behind the vanguard - that is the price
///   of the post, and it is why posting a Raider at the back punishes itself.
/// - **A rearguard cannot reach an enemy rearguard.** Not until that side's front collapses, at which point its
///   back is promoted and simply *becomes* a vanguard - a legal `Clash` target like any other.
/// - **Only a melee body raids.** Slipping a shield wall to knife the mage behind it is not done with a bow.
///
/// **Clash first, deliberately.** A reachability search short-circuits on the first winning line, so this order
/// decides which of several winning lines gets *shown* - and a straight attack is the line a player recognizes.
pub fn legal_acts(board: &Board, i: usize) -> Vec<Act> {
    let u = &board.units[i];
    let mut out = Vec::new();
    if u.fallen {
        return out;
    }
    let here = board.regions[i];

    if u.melee || u.ranged {
        for (t, e) in board.units.iter().enumerate() {
            if e.fallen || e.side == u.side {
                continue;
            }
            // **Symmetric-target dedup (sound).** If a lower-index enemy is interchangeable with this one, it
            // already stands in for it - the two strikes yield isomorphic successors, so keeping only the
            // lowest-index representative collapses the horde branching with no loss. The moment one is wounded it
            // diverges (different health) and becomes targetable in its own right again.
            if (0..t).any(|t2| interchangeable(board, t2, t)) {
                continue;
            }
            if board.regions[t] == here {
                // **In your own region.** The two are intermingled (one of you is an outrider), so there is no
                // screen between you: any weapon reaches any enemy body here.
                out.push(Act::Melee(t));
            } else if board.ranks[t] == Rank::Outrider {
                // A loose enemy body in another region is dealt with in-region by the formation that hosts it,
                // never reached across the gap. Not a target from here.
            } else if board.is_screened(t) {
                // A **screened** rearguard: reach it only by RAIDING across, past the front that guards it (melee
                // only). The front intercepts the raid in the Crossing Ring - that is the whole worth of the screen.
                if u.melee {
                    out.extend(ANSWERS.map(|a| Act::Raid(t, a)));
                }
            } else {
                // A vanguard, OR an **exposed** rearguard whose front has fallen. Either is clashable across the
                // gap by any weapon - it is *always targetable*, so standing unscreened is never shelter. But an
                // exposed BACK can ALSO be raided (melee): a raider reaches it in the Crossing Ring, *before* it
                // would fire in the Outer Ring, so being unscreened is never an advantage either - a screen is
                // what buys a back its first shot. (Raid pushed first so a scripted raider prefers the earlier,
                // silencing reach.)
                if board.ranks[t] == Rank::Rearguard && u.melee {
                    out.extend(ANSWERS.map(|a| Act::Raid(t, a)));
                }
                out.push(Act::Clash(t));
            }
        }
    }

    // Slip - the one movement, and **only the Vanguard crosses**. The front line is who charges into the enemy's
    // ground (promoting to outrider); a Rearguard stays back and fires (it reaches an enemy back by outliving the
    // enemy front, not by slipping), and an outrider is committed - there is no retreat.
    if board.ranks[i] == Rank::Vanguard {
        for r in board.occupied().into_iter().filter(|&r| r != here) {
            out.extend(ANSWERS.map(|a| Act::Slip(r, a)));
        }
    }

    out.push(Act::Hold);
    out
}

/// The foe script: a fixed, deterministic policy, so this stays a **single-agent reachability search** and not a
/// minimax (spec 0.1 - creatures are an environment, not an opponent that searches back).
///
/// Each foe goes for the hero it can most cheaply finish, preferring a **clash** it can simply take over a
/// **raid** it has to pay for; and when it must raid, it pushes through rather than turning back.
pub fn foe_acts(board: &Board) -> Vec<Option<Act>> {
    (0..board.units.len()).map(|i| foe_act(board, i)).collect()
}

/// **The one act a single scripted foe takes** - its instinct applied to the board. `None` if `i` is not a living
/// foe (a hero chooses; a corpse does nothing). This is the whole of a creature's decision, so it is the single
/// option [`super::game`] offers when a foe reaches the declaration cursor - a creature "declares" like a hero,
/// its turn just has exactly one legal move.
pub fn foe_act(board: &Board, i: usize) -> Option<Act> {
    if board.units[i].side != Side::Foe || board.units[i].fallen {
        return None;
    }
    let acts = legal_acts(board, i);
    let softest = |t: usize| (board.units[t].health, board.units[t].grit);
    // The one behavioural switch, dispatched on the creature's card instinct.
    let allowed = |a: &&Act| match board.units[i].instinct {
        // Hunt anything: a clash, an in-region melee (dig out an outrider, or - if it is the outrider - strike a
        // host), or a raid PUSHED through a line. The aggressive default - it will leave its own post to do it.
        Instinct::HuntWeakest => {
            matches!(
                a,
                Act::Clash(_) | Act::Melee(_) | Act::Raid(_, Answer::Push)
            )
        }
        // Hold the line: a clash, or an in-region melee - both fight in place. NEVER a raid or a slip, so the
        // body behind this one stays screened. What makes a wall a wall.
        Instinct::HoldTheLine => matches!(a, Act::Clash(_) | Act::Melee(_)),
    };
    acts.iter()
        .filter(allowed)
        .filter_map(|a| match a {
            Act::Clash(t) | Act::Raid(t, _) | Act::Melee(t) => Some((softest(*t), *a)),
            _ => None,
        })
        .min_by_key(|&(k, _)| k)
        .map(|(_, a)| a)
        .or(Some(Act::Hold))
}

/// **Who would intercept a crossing** - the enemy vanguard(s) that reach for `mover` if it crosses to region
/// `dest`. This is the resolver's own interception rule (the front line at each enemy-owned zone the crossing
/// touches), surfaced so a narrative UI can name *who* catches you *before* you commit an [`Answer`] - matching
/// the order the fiction hands you the decision (declare the crossing, see who caught you, then answer).
///
/// Deterministic, so it can be shown at declaration time: a foe already committed to its own crossing (per
/// [`foe_act`]) is in transit and cannot hold the line, exactly as the resolver's [`play_round`] would find. Meant
/// for a hero's crossing (the catchers are foes); an empty result means the crossing is unopposed.
pub fn catchers(board: &Board, mover: usize, dest: u8) -> Vec<usize> {
    let enemy = other_side(board.units[mover].side);
    let mut out = Vec::new();
    for zone in [board.regions[mover], dest] {
        if board.owner(zone) != Some(enemy) {
            continue; // a friendly zone lets its own pass; only an enemy formation reaches for a crosser
        }
        for f in board.vanguard(zone, enemy) {
            let in_transit = foe_act(board, f)
                .and_then(|a| a.destination(board, f))
                .is_some();
            if !in_transit && !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
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

/// What happened in one sub-phase - enough for a transcript or a renderer to say *why* the board changed.
#[derive(Clone, Debug, Default)]
pub struct SubPhaseLog {
    /// **Which phase of the round this is** - the ring and step, e.g. `"Inner Ring: Outriders"`,
    /// `"Crossing Ring: Intercept"`, `"Outer Ring: Clash"`. Set by [`play_round`] at each step so a transcript can
    /// say *where* in the round every strike and card-flip happened, not just *that* it happened. Empty on a log
    /// built outside `play_round`.
    pub phase: &'static str,
    /// Got through - standing somewhere new now.
    pub through: Vec<usize>,
    /// Turned and fought instead: it stayed where it was.
    pub aborted: Vec<usize>,
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
    /// Every body's **rank** at this boundary. Same reason: without it a promotion that happens in the last
    /// sub-phase appears to have been true all round.
    pub ranks: Vec<Rank>,
    /// **Every strike that landed in this sub-phase**, source-attributed: who hit whom, and how many blows. A
    /// renderer reads Might per blow from the attacker to say *where* each body's damage came from (a sweep
    /// records one `Hit` per swept contact, so an area strike is attributed to its sweeper too).
    pub hits: Vec<Hit>,
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
            // **A pack is many bodies, and an area strike catches every one of them.**
            //
            // This is the second axis, and it is independent of reach: *which* bodies in the region a sweep
            // touches is settled by the tier; *how much of a body* it touches is settled here. A horde is one
            // `Combatant` whose Health is its body count, so an aimed blow felling `Might`-many bodies looks
            // like damage - but a sweep landing on the same one body deals `Might` **once**, which made a sweep
            // strictly WORSE against a pack than an aimed blow. Exactly backwards.
            //
            // A sweep clears it. Armour aside, there is nowhere in a pack to not be.
            if might > board.units[j].armor {
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
/// # Why this does not call `combat::resolve_strike`
///
/// **An aimed blow fells ONE body of a pack.** `combat`'s model spills - a horde is one [`Combatant`] whose
/// Health is its body count, so `Might` damage takes `Might` bodies off it in a single strike. That reads as
/// damage, but it makes a **Might-7 aimed blow a better horde-killer than a sweep**, which inverts the two
/// horde locks and no stat tuning can flip them back.
///
/// One blow kills one of them. That is also the more natural fiction: you stab one, you do not scythe seven.
/// **Width against a pack is what a sweep is FOR** - it catches every body at once, whatever its Might - and if
/// an aimed blow can do the same thing by being big, the sweep has no job.
///
/// So the damage model here is:
/// - **Normal body:** each strike banks `max(0, Might - armor)` into the per-round pile; a Health card flips
///   every time the pile clears Grit. The pile closes at the Reset (`combat::refresh_round`), unchanged.
/// - **Horde:** each strike fells exactly **one** body. A sweep clears the pack outright ([`area_strike`]).
fn land(board: &mut Board, contacts: &[Contact], sweeps: &[Contact], extra: &[Blows]) -> Vec<Hit> {
    // Collect every blow first, apply nothing yet: an order-free, commit-based batch, so a blow lands even if
    // its striker dies to a simultaneous one.
    let n = board.units.len();
    let mut damage = vec![0u32; n]; // banked Might, for a normal body
    let mut felled = vec![0u32; n]; // bodies struck off, for a horde

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
        if board.units[t].horde {
            felled[t] += hits; // one blow, one body - Might buys nothing against a pack
        } else {
            // A horde attacker swings as ONE volley: every living body lands together, so its blow is the whole
            // body count times Might. Armour stops each little hit, so it is subtracted **per body** - a swarm of
            // Might-1 bodies does nothing to an armoured target, however many of them there are.
            let per_body = board.units[a].might.saturating_sub(board.units[t].armor);
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
            // Reset and only there, so a wound you cannot finish this round is a wound you did not inflict.
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
type Attack = (usize, usize);

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
fn close(board: &mut Board, before: &[bool]) -> SubPhaseLog {
    end_sub_phase(&mut board.units);
    SubPhaseLog {
        fallen: (0..board.units.len())
            .filter(|&i| before[i] && board.units[i].fallen)
            .collect(),
        health: board.units.iter().map(|u| u.health).collect(),
        ranks: board.ranks.clone(),
        ..Default::default()
    }
}

fn living(board: &Board) -> Vec<bool> {
    board.units.iter().map(|u| !u.fallen).collect()
}

/// **A body that is slipping is in NEITHER line.** It has left the front and not yet arrived anywhere - it is a
/// third position, in transit, and while it is there it can neither screen nor be screened.
///
/// So it **cannot catch another slipper**: you are outside the line the moment you leave it. Without this, a
/// body could be running across open ground and simultaneously holding the wall it just abandoned.
fn in_transit(board: &Board, acts: &[Act], i: usize) -> bool {
    !board.units[i].fallen && acts[i].destination(board, i).is_some()
}

/// The bodies of `side` **actually holding the line** in `region` right now: posted at the front, and not off
/// slipping somewhere.
fn holding_line(board: &Board, acts: &[Act], region: u8, side: Side) -> Vec<usize> {
    board
        .vanguard(region, side)
        .into_iter()
        .filter(|&f| !in_transit(board, acts, f))
        .collect()
}

/// The bodies of `side` **at the back** of `region`, not off slipping - the cannons that get to volley an
/// incoming raider before it lands.
fn back_line(board: &Board, acts: &[Act], region: u8, side: Side) -> Vec<usize> {
    board
        .in_region(region)
        .into_iter()
        .filter(|&i| {
            board.units[i].side == side
                && board.ranks[i] == Rank::Rearguard
                && !in_transit(board, acts, i)
        })
        .collect()
}

/// One pass of bodies reaching for the slippers: they commit, the slipper answers as it declared, the blows
/// land. Returns the contacts that stuck (a slipper that Evaded broke them all).
///
/// This is used **twice** - once for the front line (Intercept) and once for the back line (Volley) - and the two
/// are separate sub-phases for a reason that is pure tempo economy: **if the front already killed the slipper, the
/// back does not waste its shot on a corpse.** That saved card is the whole reason the boundary earns its place
/// under the razor.
///
/// A catcher reaches with its **own weapon shape**. An *aimed* catcher engages one slipper, which may Evade to
/// break the edge. An **area** catcher **volleys the whole crossing** - the Crossing-ring sweep footprint: one
/// card blankets every crosser it is catching, **unevadably** (a salvo across the charge - a horde crosser is
/// cleared outright, anyone else takes an unanswerable contact). So an area weapon defends a charge exactly as it
/// attacks one, and single-ranged-vs-single-melee resolves like a bypassed vanguard: the back volleys the crosser,
/// in its true shape, before the crosser can land.
fn reach_for_slippers(
    board: &mut Board,
    catchers: &[(usize, usize)], // (catcher, slipper)
    movers: &[(usize, u8, Answer)],
) -> Vec<Hit> {
    // The live (catcher, slipper) pairs this pass - a fallen catcher or slipper, or a spent catcher, reaches for
    // nothing.
    let live: Vec<(usize, usize)> = catchers
        .iter()
        .copied()
        .filter(|&(f, s)| {
            !board.units[f].fallen && !board.units[s].fallen && board.units[f].tempo > 0
        })
        .collect();

    // AIMED catchers engage one slipper each; that slipper may Evade to break the edge.
    let engagements: Vec<Engage> = live
        .iter()
        .filter(|&&(f, _)| !board.units[f].aoe)
        .map(|&(f, s)| Engage {
            attacker: f,
            target: s,
            cards: reach_cards(&board.units, f, s),
        })
        .collect();
    let reaching = engage(board, &engagements);

    // The slipper answers, seeing exactly what was committed. Evade pays in full and breaks every AIMED edge;
    // Push and Abort spend nothing and eat the blows. (An area volley below is not an edge you can slip.)
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| match movers.iter().find(|&&(m, _, _)| m == i) {
            Some(&(_, _, Answer::Evade)) => Dodge::Slip,
            _ => Dodge::Stand,
        })
        .collect();
    let mut landed = resolve_evade(&mut board.units, &reaching, &dodges);

    // AREA catchers volley the whole crossing (the Crossing-ring sweep footprint): one card, every crosser they
    // catch, unevadably. A horde crosser is cleared outright (recorded, applied after `land` so it counts at full
    // size in this batch); anyone else takes an unanswerable contact. This is the same "an area strike hits a whole
    // band" rule as Clash/Raid/Melee, applied to the band of bodies mid-crossing.
    let mut sweepers: Vec<usize> = live
        .iter()
        .filter(|&&(f, _)| board.units[f].aoe)
        .map(|&(f, _)| f)
        .collect();
    sweepers.sort_unstable();
    sweepers.dedup();
    let mut felled: Vec<Hit> = Vec::new();
    for f in sweepers {
        if board.units[f].tempo == 0 {
            continue;
        }
        board.units[f].tempo -= 1; // one card blankets the whole crossing
        let might = board.units[f].might;
        let caught: Vec<usize> = live
            .iter()
            .filter(|&&(g, _)| g == f)
            .map(|&(_, s)| s)
            .collect();
        for s in caught {
            if board.units[s].fallen {
                continue;
            }
            if board.units[s].horde {
                if might > board.units[s].armor {
                    let bodies = board.units[s].health;
                    if bodies > 0 {
                        felled.push(Hit {
                            attacker: f,
                            target: s,
                            hits: bodies,
                        });
                    }
                }
            } else {
                landed.push(Contact {
                    attacker: f,
                    target: s,
                    bid: 0, // an area volley cannot be evaded and nobody answers along it
                });
            }
        }
    }

    // An Abort turns and lays about it - at EVERY body that caught it, evenly. Not "the first one in the list":
    // picking one by iteration order would make who dies depend on who sits at index 0 (Spec 1.9). Splitting the
    // pool is symmetric in the catchers, so it needs no tie-break and cannot be gamed by re-seating.
    let mut ripostes: Vec<Blows> = Vec::new();
    for &(i, _, answer) in movers {
        if answer != Answer::Abort
            || board.units[i].fallen
            || board.units[i].tempo == 0
            || !board.units[i].melee
        {
            continue;
        }
        let caught_by: Vec<usize> = landed
            .iter()
            .filter(|c| c.target == i)
            .map(|c| c.attacker)
            .collect();
        let each = board.units[i].tempo / caught_by.len().max(1) as u32;
        if each == 0 {
            continue;
        }
        for c in caught_by {
            ripostes.push(Blows {
                unit: i,
                target: c,
                cards: each,
            });
        }
    }

    let mut hits = land(board, &landed, &[], &ripostes);
    // Apply the volley's horde clears AFTER `land` read commit-time bodies, so an aborter's riposte still lands
    // and a swept horde still counted at full size in this batch (commit-batch simultaneity, Spec 1.9).
    for h in &felled {
        board.units[h.target].health = board.units[h.target].health.saturating_sub(h.hits);
    }
    hits.extend(felled);
    hits
}

/// The enemy of a side.
fn other_side(s: Side) -> Side {
    match s {
        Side::Party => Side::Foe,
        Side::Foe => Side::Party,
    }
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
fn dissolve(board: &mut Board) {
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
fn home_of(board: &Board, side: Side, avoid: u8) -> Option<u8> {
    (0..board.units.len())
        .find(|&i| {
            !board.units[i].fallen
                && board.units[i].side == side
                && board.ranks[i] != Rank::Outrider
                && board.regions[i] != avoid
        })
        .map(|i| board.regions[i])
}

/// Play **one whole round**: the Reset, then the three distance **rings** nearest-first - the **Inner Ring**
/// (Intruders, distance zero, a single simultaneous strike), the **Crossing Ring** (closing into a formation),
/// and the **Outer Ring** (Across the gap - Fire then Clash). Each ring resolves in **strikes**, and deaths
/// finalize at each strike boundary, so a body killed early is silenced in every later strike (the razor).
pub fn play_round(board: &mut Board, acts: &[Act]) -> Vec<SubPhaseLog> {
    refresh_round(&mut board.units);
    let mut logs = Vec::new();

    // ---- INNER RING: INTRUDERS (distance zero) ---------------------------------------------------------
    //
    // Every region holding both a formation and enemy outriders resolves its in-place fight with NO screen: the
    // outrider strikes any host it declared, the host strikes any outrider it declared. It is ONE simultaneous
    // strike - melee and ranged together, no "ranged first". The ranged-first rule of the outer rings only exists
    // to model *closing the distance* (an arrow lands before a swordsman crosses the gap); but here nobody is
    // closing - the crossing happened on an earlier round, so everyone is already point-blank and intermingled.
    // No distance, so no order. An outrider is past the screen, so a melee sweep here catches EVERY enemy in the
    // region, both tiers.
    let melees: Vec<Attack> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| match acts[i] {
            Act::Melee(t)
                if !board.units[t].fallen
                    && board.regions[t] == board.regions[i]
                    && board.units[t].side != board.units[i].side =>
            {
                Some((i, t))
            }
            _ => None,
        })
        .collect();
    if !melees.is_empty() {
        let before = living(board);
        let hits = exchange(board, &melees, true, true);
        let mut lg = close(board, &before);
        lg.phase = "Inner Ring: Outriders";
        lg.hits = hits;
        logs.push(lg);
    }
    // Outriders whose host formation is now gone are outriders of nothing - the state dissolves and they rejoin
    // their own line (see `dissolve`). Resolved once here, at the Inner Ring boundary where the havoc lands.
    dissolve(board);

    // ---- CROSSING RING: CROSSINGS (closing into a formation) -------------------------------------------
    //
    // Every declared Raid/Slip sends its body across as a transient. An enemy formation reaches for a crosser at
    // BOTH ends it touches - the zone ENTERED and the zone LEFT - because you are outside your own screen the
    // moment you move. At each such end the FRONT intercepts (spears) THEN the BACK volleys the survivors (bows),
    // so a front-killed crosser is not volleyed. A friendly zone never reaches for its own, so a rally between
    // friendly zones is free; but an outrider pulling OUT of enemy ranks is opposed by the ranks it leaves (the
    // crossing in reverse).
    let movers: Vec<(usize, u8, Answer)> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen)
        .filter_map(|i| Some((i, acts[i].destination(board, i)?, acts[i].answer()?)))
        .collect();

    let mut front_catchers: Vec<(usize, usize)> = Vec::new();
    let mut back_catchers: Vec<(usize, usize)> = Vec::new();
    for &(i, dest, _) in &movers {
        let enemy = other_side(board.units[i].side);
        for zone in [board.regions[i], dest] {
            if board.owner(zone) != Some(enemy) {
                continue; // only an enemy-owned zone reaches for a crosser; a friendly zone lets its own pass
            }
            for f in holding_line(board, acts, zone, enemy) {
                front_catchers.push((f, i));
            }
            for c in back_line(board, acts, zone, enemy) {
                back_catchers.push((c, i));
            }
        }
    }

    let before = living(board);
    let hits = reach_for_slippers(board, &front_catchers, &movers);
    let mut lg = close(board, &before);
    lg.phase = "Crossing Ring: Intercept";
    lg.hits = hits;
    logs.push(lg);
    let before = living(board);
    let hits = reach_for_slippers(board, &back_catchers, &movers);
    let mut lg = close(board, &before);
    lg.phase = "Crossing Ring: Volley";
    lg.hits = hits;
    logs.push(lg);

    // LAND: survivors that got through leave the line and arrive. Into an enemy zone they promote to outrider
    // (loose inside the ranks); a rally into a friendly zone rejoins that formation at its weapon rank.
    let mut through = vec![false; board.units.len()];
    let mut landing = SubPhaseLog {
        health: board.units.iter().map(|u| u.health).collect(),
        ranks: board.ranks.clone(),
        ..Default::default()
    };
    for &(i, dest, answer) in &movers {
        if board.units[i].fallen {
            continue;
        }
        if answer == Answer::Abort {
            landing.aborted.push(i);
            continue; // it turned and fought; it never left
        }
        let into_enemy = board.owner(dest) == Some(other_side(board.units[i].side));
        board.regions[i] = dest;
        board.ranks[i] = if into_enemy {
            Rank::Outrider
        } else {
            Board::weapon_rank(&board.units[i])
        };
        through[i] = true;
        landing.through.push(i);
    }
    if let Some(last) = logs.last_mut() {
        last.through = landing.through.clone();
        last.aborted = landing.aborted.clone();
    }

    // The raiders that got through strike the rearguard they came for - before it can fire in the Outer Ring. Tempo-gated:
    // a raider that evaded with everything arrives with nothing to swing. A Raid sweep covers the back line it is
    // now standing among (its tier).
    let before = living(board);
    let raids: Vec<Attack> = movers
        .iter()
        .filter(|&&(i, _, _)| through[i] && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|&(i, _, _)| match acts[i] {
            Act::Raid(t, _) if !board.units[t].fallen && board.regions[t] == board.regions[i] => {
                Some((i, t))
            }
            _ => None,
        })
        .collect();
    let hits = exchange(board, &raids, false, false);
    let mut lg = close(board, &before);
    lg.phase = "Crossing Ring: Raid";
    lg.hits = hits;
    logs.push(lg);

    // ---- OUTER RING: ACROSS THE GAP (Fire then Clash) -------------------------------------------------
    //
    // The standing formations trade at each other's vanguards in other regions. Every back line fires first
    // (holding off IS being quicker), then every front line closes and trades. Ranged before melee.
    for tier in [Rank::Rearguard, Rank::Vanguard] {
        let before = living(board);
        let attacks: Vec<Attack> = (0..board.units.len())
            .filter(|&i| {
                !board.units[i].fallen && board.units[i].tempo > 0 && board.ranks[i] == tier
            })
            .filter_map(|i| match acts[i] {
                Act::Clash(t) if !board.units[t].fallen => Some((i, t)),
                _ => None,
            })
            .collect();
        let hits = exchange(board, &attacks, true, false);
        let mut lg = close(board, &before);
        lg.phase = if tier == Rank::Rearguard {
            "Outer Ring: Fire"
        } else {
            "Outer Ring: Clash"
        };
        lg.hits = hits;
        logs.push(lg);
    }

    logs
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged.** Area strikes split off and
/// sweep their target's region: the tier aimed at, or - when `sweep_whole` - both tiers (an in-region melee, past
/// the screen).
fn exchange(board: &mut Board, attacks: &[Attack], pour: bool, sweep_whole: bool) -> Vec<Hit> {
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
    hits
}

// ---- the doom oracle -----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Winnable,
    /// The search ran out of budget. **Not an answer** - an answer in progress.
    Evaluating,
    Doomed,
}

/// The memo key: per-unit `(health, fallen, rank)`, the **canonicalized** partition, and the round. The rank
/// carries it all - two positions that differ only by who is loose inside the enemy ranks (an outrider) are
/// genuinely different positions.
///
/// Tempo and the damage pile are absent on purpose - both are re-derived by the round Reset, and we only memoize
/// at a **round** boundary. The product's own rule (the round is the one deadline) paying a dividend.
type Key = (Vec<(u32, bool, Rank)>, Vec<u8>, usize);

/// **The doom oracle.** Holds the memo, so the first evaluation walks the tree and every later one is a lookup.
///
/// Budgeted and restartable: give it a node budget and it answers [`Verdict::Evaluating`] rather than lying.
/// **The one rule it may never break:** an incomplete subtree is never memoized. A "no win found" that was really
/// "I gave up" must never be cached as `Doomed`. **The oracle may be silent; it may never be wrong.**
pub struct Oracle {
    memo: HashMap<Key, bool>,
    nodes: u64,
    /// Positions evaluated in **this** walk. The budget bounds *this*, not the lifetime total - otherwise a
    /// resumed walk re-spends its allowance on nodes it is merely re-treading and never gets deeper.
    walk: u64,
    budget: u64,
    aborted: bool,
}

impl Oracle {
    pub fn new(budget: u64) -> Self {
        Oracle {
            memo: HashMap::new(),
            nodes: 0,
            walk: 0,
            budget,
            aborted: false,
        }
    }

    pub fn nodes(&self) -> u64 {
        self.nodes
    }
    pub fn states(&self) -> usize {
        self.memo.len()
    }
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    /// **Allow the next walk `nodes` positions, and clear the abort flag** - the frame tick of a resumable oracle.
    /// The memo survives, so each retry re-treads its settled positions for free and pushes the frontier deeper.
    ///
    /// **Liveness is the caller's job**: a subtree memoizes only once fully explored, so a grant too small to
    /// settle *any* new subtree makes no progress however often repeated. Escalate - double on every
    /// `Evaluating`. Safety is *not* the caller's job and never depends on the grant.
    pub fn grant(&mut self, nodes: u64) {
        self.walk = 0;
        self.budget = nodes;
        self.aborted = false;
    }

    pub fn verdict(&mut self, board: &Board, round: usize) -> Verdict {
        let before = self.aborted;
        let win = self.winnable(board, round, false);
        self.judge(win, before)
    }

    /// A **win is a proof** (we hold a witness line), so it stands even when other branches were abandoned. A
    /// **loss is a proof only if the tree was exhausted** - otherwise all we learned is that we have not found a
    /// win *yet*, and the honest word for that is `Evaluating`.
    fn judge(&self, win: bool, before: bool) -> Verdict {
        match (win, self.aborted && !before) {
            (true, _) => Verdict::Winnable,
            (false, true) => Verdict::Evaluating,
            (false, false) => Verdict::Doomed,
        }
    }

    /// **"If this hero does this, is the position still winnable?"** - the per-move verdict the UI charts. The
    /// rest of the party plays its best and every later round is free, so this asks whether the choice
    /// *forecloses* the win - not whether it is optimal. That is what a player actually wants to know.
    pub fn verdict_for(&mut self, board: &Board, round: usize, hero: usize, act: Act) -> Verdict {
        let before = self.aborted;
        let others: Vec<usize> = (0..board.units.len())
            .filter(|&i| board.units[i].side == Side::Party && !board.units[i].fallen && i != hero)
            .collect();
        let choices: Vec<Vec<Act>> = others.iter().map(|&i| legal_acts(board, i)).collect();
        let foes = foe_acts(board);
        let mut win = false;
        for pick in 0..count(&choices) {
            let mut acts = assemble(board, &others, &choices, pick, &foes);
            acts[hero] = act;
            let mut b = board.clone();
            play_round(&mut b, &acts);
            if self.winnable(&b, round + 1, false) {
                win = true;
                break;
            }
        }
        self.judge(win, before)
    }

    /// Can the party force a win from here?
    ///
    /// `no_slip` is the **control**, and it is the experiment. The formation is now declared once *by design*, so
    /// the old question ("fixed setup vs re-declared every round") is moot. The honest question is now:
    ///
    /// > **Is slipping ever necessary?**
    ///
    /// Under `no_slip` the party may only clash or hold - it may never raid, retreat, or regroup. If a fight is
    /// winnable without ever slipping, the mechanic bought nothing *there*. If some fight is winnable **only** by
    /// slipping, movement is load-bearing - and it is load-bearing in the strongest possible sense, because it
    /// could not have been pre-empted by starting somewhere else: the formation was fixed at setup either way.
    pub fn winnable(&mut self, board: &Board, round: usize, no_slip: bool) -> bool {
        if let Some(done) = board.outcome() {
            return done;
        }
        if round >= MAX_ROUNDS {
            return false; // a draw at the cap is not a win
        }
        if self.walk >= self.budget {
            self.aborted = true;
            return false;
        }
        let key = (
            (0..board.units.len())
                .map(|i| (board.units[i].health, board.units[i].fallen, board.ranks[i]))
                .collect(),
            canonical(&board.regions),
            round,
        );
        if let Some(&v) = self.memo.get(&key) {
            return v;
        }
        self.nodes += 1;
        self.walk += 1;

        let heroes: Vec<usize> = (0..board.units.len())
            .filter(|&i| board.units[i].side == Side::Party && !board.units[i].fallen)
            .collect();
        let choices: Vec<Vec<Act>> = heroes
            .iter()
            .map(|&i| {
                let mut acts = legal_acts(board, i);
                if no_slip {
                    acts.retain(|a| matches!(a, Act::Clash(_) | Act::Hold));
                }
                acts
            })
            .collect();
        let foes = foe_acts(board);

        // **Each node must judge its OWN subtree.** Stash the caller's abort flag and start clean, or this node
        // inherits a *sibling's* give-up and mistakes it for its own completeness - which would cache an
        // incomplete "no win found" as a **proven Doomed**, the one thing the oracle may never do.
        let outer = self.aborted;
        self.aborted = false;

        let mut win = false;
        for pick in 0..count(&choices) {
            let acts = assemble(board, &heroes, &choices, pick, &foes);
            let mut b = board.clone();
            play_round(&mut b, &acts);
            if self.winnable(&b, round + 1, no_slip) {
                win = true;
                break;
            }
        }

        let incomplete = self.aborted;
        self.aborted = outer || incomplete;
        if win || !incomplete {
            self.memo.insert(key, win); // cache only what we can actually prove
        }
        win
    }
}

fn count(choices: &[Vec<Act>]) -> usize {
    choices.iter().map(|c| c.len().max(1)).product::<usize>()
}

/// The `pick`-th joint declaration over `who`, mixed-radix, with the foes' scripted acts folded in.
fn assemble(
    board: &Board,
    who: &[usize],
    choices: &[Vec<Act>],
    pick: usize,
    foes: &[Option<Act>],
) -> Vec<Act> {
    let mut acts: Vec<Act> = vec![Act::Hold; board.units.len()];
    for (k, &i) in who.iter().enumerate() {
        if choices[k].is_empty() {
            continue;
        }
        let radix: usize = choices[..k].iter().map(|c| c.len().max(1)).product();
        acts[i] = choices[k][(pick / radix) % choices[k].len()];
    }
    for (i, a) in foes.iter().enumerate() {
        if let Some(a) = a {
            acts[i] = *a;
        }
    }
    acts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, stats, 0, melee, ranged)
    }

    /// A wall in front, a cannon behind it, one enemy - the smallest board that has a formation at all. Posts are
    /// derived from the weapon: the Bastion (melee) is front, the Marksman (ranged) is back, the Ogre (melee) is
    /// front of its own region.
    ///
    /// The Ogre is deliberately **rich** (Cadence 5): it can afford to evade the wall *and* still arrive with
    /// enough tempo to price the cannon out of its own evade. The Bastion's Might (3) exceeds the Ogre's Grit (2),
    /// so it can actually *bleed* a body that pushes past it.
    fn wall_and_cannon() -> Board {
        Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 3, 3, 1, 2], true, false), // 0 - the wall (front)
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true), // 1 - the cannon (back)
                unit("Ogre", Side::Foe, [5, 5, 2, 5, 2], true, false), // 2 - the raider (front)
            ],
            vec![0, 0, 1],
        )
    }

    #[test]
    fn region_labels_are_not_state() {
        assert_eq!(canonical(&[3, 3, 7, 9]), canonical(&[1, 1, 0, 5]));
        assert_ne!(canonical(&[0, 0, 1]), canonical(&[0, 1, 1]));
    }

    // ---- RANK IS DERIVED --------------------------------------------------------------------------------

    /// **Rank is derived from the weapon, not chosen.** A ranged-only body is a `Rearguard`; anything that can
    /// strike in melee is a `Vanguard`. Fixed at construction, for heroes and foes alike; nobody starts an
    /// `Outrider` (that rank is only ever earned).
    #[test]
    fn rank_is_derived_from_the_weapon() {
        let b = Board::new(
            vec![
                unit("Sword", Side::Party, [3, 3, 1, 2, 2], true, false),
                unit("Bow", Side::Party, [3, 3, 1, 2, 2], false, true),
                unit("Skirmisher", Side::Foe, [3, 3, 1, 2, 2], true, true), // dual - deferred, treated as vanguard
            ],
            vec![0, 0, 1],
        );
        assert_eq!(b.ranks[0], Rank::Vanguard, "a melee body stands front");
        assert_eq!(
            b.ranks[1],
            Rank::Rearguard,
            "a ranged-only body stands back"
        );
        assert_eq!(
            b.ranks[2],
            Rank::Vanguard,
            "a dual body is a vanguard (the deferred case)"
        );
        assert!(
            b.ranks.iter().all(|&r| r != Rank::Outrider),
            "nobody starts an outrider"
        );
    }

    // ---- REACH: SCREEN, RAID, CLASH ---------------------------------------------------------------------

    /// **A back whose front has fallen is targetable but still a back.** It is not promoted into the line - it
    /// keeps its post (so it still fires first) but loses its screen (so anyone may clash it now). Force, not fiat.
    #[test]
    fn a_back_whose_front_has_fallen_is_targetable_but_still_a_back() {
        let mut b = wall_and_cannon();
        assert_eq!(
            b.ranks[1],
            Rank::Rearguard,
            "the cannon starts behind the wall"
        );
        assert!(b.is_screened(1), "and is screened by it");
        assert!(
            !legal_acts(&b, 2).contains(&Act::Clash(1)),
            "so the Ogre cannot simply clash it - it must raid"
        );

        b.units[0].fallen = true; // the wall dies
        assert_eq!(
            b.ranks[1],
            Rank::Rearguard,
            "the cannon is STILL a cannon - not promoted"
        );
        assert!(!b.is_screened(1), "but nothing is screening it any more");
        assert!(
            legal_acts(&b, 2).contains(&Act::Clash(1)),
            "so it is targetable now - force, not fiat"
        );
    }

    /// **A lone archer at the back shoots before a swordsman can swing.** It is fully targetable, and wins anyway
    /// because it fires in the Outer Ring.s Fire while the swordsman closes in the later Clash. Posted at the front, the
    /// same body closes and dies - but the post is the weapon, so a ranged body is *always* back.
    #[test]
    fn a_lone_archer_shoots_before_a_swordsman_can_swing() {
        let mut b = Board::new(
            vec![
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
                unit("Duelist", Side::Foe, [5, 5, 1, 2, 2], true, false),
            ],
            vec![0, 1],
        );
        assert_eq!(
            b.ranks[0],
            Rank::Rearguard,
            "a ranged body is derived to the back"
        );
        let mut rounds = 0;
        while b.outcome().is_none() && rounds < MAX_ROUNDS {
            play_round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
            rounds += 1;
        }
        assert_eq!(
            b.outcome(),
            Some(true),
            "the archer kills it before it can swing"
        );
    }

    /// **A rearguard is reachable only by a raid** - it falls out of the menu.
    #[test]
    fn a_rearguard_is_reachable_only_by_a_raid() {
        let b = wall_and_cannon();
        let acts = legal_acts(&b, 2); // the Ogre, melee
        assert!(
            acts.contains(&Act::Clash(0)),
            "it may clash the wall freely"
        );
        assert!(
            !acts.contains(&Act::Clash(1)),
            "but not clash the screened cannon"
        );
        assert!(
            acts.iter().any(|a| matches!(a, Act::Raid(1, _))),
            "to reach the cannon it must raid the wall"
        );
    }

    /// **The slipper always has all three answers.**
    #[test]
    fn a_slipper_always_has_all_three_answers() {
        let b = wall_and_cannon();
        let acts = legal_acts(&b, 2);
        for answer in ANSWERS {
            assert!(
                acts.contains(&Act::Raid(1, answer)),
                "{answer:?} must be on the menu"
            );
        }
    }

    /// **An archer cannot raid**: slipping a shield wall to knife the mage is not done with a bow.
    #[test]
    fn an_archer_cannot_raid() {
        let b = Board::new(
            vec![
                unit("Archer", Side::Party, [3, 3, 1, 2, 2], false, true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        let acts = legal_acts(&b, 0);
        assert!(
            acts.contains(&Act::Clash(1)),
            "it shoots their front freely"
        );
        assert!(
            !acts.iter().any(|a| matches!(a, Act::Raid(..))),
            "but cannot raid the mage"
        );
        assert!(
            !acts.contains(&Act::Clash(2)),
            "and cannot shoot through the wall"
        );
    }

    /// **Interchangeable foes collapse to one target (sound dedup).** Three identical foes in one region offer
    /// exactly one Clash target between them; the moment one is wounded it diverges and becomes targetable again,
    /// so the menu grows back to two (the wounded body + a representative of the still-identical pair).
    #[test]
    fn interchangeable_foes_collapse_to_a_single_target() {
        let mut b = Board::new(
            vec![
                unit("Hero", Side::Party, [5, 4, 1, 2, 2], true, false), // 0
                unit("A", Side::Foe, [3, 4, 1, 2, 2], true, false),      // 1
                unit("B", Side::Foe, [3, 4, 1, 2, 2], true, false),      // 2
                unit("C", Side::Foe, [3, 4, 1, 2, 2], true, false),      // 3
            ],
            vec![0, 1, 1, 1],
        );
        let clash_targets = |b: &Board| -> Vec<usize> {
            legal_acts(b, 0)
                .into_iter()
                .filter_map(|a| match a {
                    Act::Clash(t) | Act::Melee(t) => Some(t),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            clash_targets(&b),
            vec![1],
            "three identical foes offer exactly one target - the lowest-index representative"
        );

        b.units[2].health -= 1; // wound B: it is no longer interchangeable with A/C
        assert_eq!(
            clash_targets(&b),
            vec![1, 2],
            "the wounded body diverges and is targetable again, alongside a representative of the pair"
        );
    }

    // ---- THE CROSSING (Crossing Ring) --------------------------------------------------------------------------

    /// **THE ONE THAT MATTERS. The screen is a PRICE, not an immunity** - enough Tempo gets past a front and
    /// reaches the body behind it. And a landed raider ends up standing INSIDE the enemy formation, as an
    /// outrider.
    #[test]
    fn evading_the_line_reaches_the_body_behind_it() {
        let mut b = wall_and_cannon();
        let cannon = b.units[1].health;
        play_round(
            &mut b,
            &[Act::Clash(2), Act::Clash(2), Act::Raid(1, Answer::Evade)],
        );
        assert!(
            b.units[1].health < cannon || b.units[1].fallen,
            "a high-tempo body must be able to buy its way past an intact front"
        );
        assert_eq!(
            b.regions[2], 0,
            "and it is now standing inside their formation"
        );
        assert_eq!(
            b.ranks[2],
            Rank::Outrider,
            "as an outrider - loose in the enemy ranks"
        );
    }

    /// **PUSH: take the hit and go anyway.** A vanguard cannot stop you; it can only bleed you.
    #[test]
    fn pushing_through_costs_blood_instead_of_tempo() {
        let mut b = wall_and_cannon();
        let ogre = b.units[2].health;
        play_round(
            &mut b,
            &[Act::Clash(2), Act::Clash(2), Act::Raid(1, Answer::Push)],
        );
        assert_eq!(b.regions[2], 0, "it went through regardless");
        assert!(
            b.units[2].health < ogre || b.units[2].fallen,
            "and paid in blood, not tempo"
        );
    }

    /// **The front DRAINS a raider that evades it.** A poor raider buys its way past the line but arrives with
    /// nothing left to price the cannon out of its own evade - so the blow whiffs.
    #[test]
    fn the_front_drains_a_raider_that_evades_it() {
        let mut b = Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 3, 3, 1, 2], true, false),
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
                unit("Runt", Side::Foe, [5, 5, 2, 2, 2], true, false), // Cadence 2: it can pay, but only just
            ],
            vec![0, 0, 1],
        );
        let cannon = b.units[1].health;
        play_round(
            &mut b,
            &[Act::Clash(2), Act::Clash(2), Act::Raid(1, Answer::Evade)],
        );
        assert_eq!(
            b.regions[2], 0,
            "it got through - the screen is a price, not a wall"
        );
        assert_eq!(
            b.units[1].health, cannon,
            "but spent its pool getting there, so the cannon evaded"
        );
    }

    /// **ABORT: turn and fight.** You give up the ground and swing at whoever caught you.
    #[test]
    fn aborting_keeps_you_where_you_are_and_swings_back() {
        let mut b = wall_and_cannon();
        let cannon = b.units[1].health;
        let logs = play_round(
            &mut b,
            &[Act::Clash(2), Act::Clash(2), Act::Raid(1, Answer::Abort)],
        );
        assert!(
            logs.iter().any(|l| l.aborted.contains(&2)),
            "it turned back at the line"
        );
        assert_eq!(b.regions[2], 1, "it never left its own ground");
        assert_ne!(b.ranks[2], Rank::Outrider, "and it is no outrider");
        assert_eq!(b.units[1].health, cannon, "and it never reached the cannon");
    }

    /// **The back line volleys an incoming raider.** The cannons defend themselves - front (Intercept) then back
    /// (Volley), so the raider is shot by the wall AND by the cannon it was coming for.
    #[test]
    fn the_back_line_volleys_an_incoming_raider() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 3, 2], true, false),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        let raider = b.units[0].health;
        play_round(
            &mut b,
            &[Act::Raid(2, Answer::Push), Act::Clash(0), Act::Clash(0)],
        );
        assert_eq!(b.regions[0], 1, "it got through");
        assert!(
            b.units[0].health < raider,
            "and was shot on the way in - by the wall and the cannon"
        );
    }

    /// **A body in transit cannot hold the line it just left.** While it is crossing it can neither screen nor be
    /// screened - so it cannot catch a body coming the other way into the ground it abandoned.
    #[test]
    fn a_body_in_transit_cannot_hold_the_line_it_just_left() {
        let b = Board::new(
            vec![
                unit("Guard", Side::Party, [3, 4, 1, 2, 2], true, false),
                unit("Cannon", Side::Party, [4, 2, 1, 2, 2], false, true),
                unit("Ogre", Side::Foe, [5, 5, 2, 3, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 0, 1, 1],
        );
        let acts = [
            Act::Raid(3, Answer::Evade),
            Act::Clash(2),
            Act::Raid(1, Answer::Evade),
            Act::Clash(0),
        ];
        assert!(
            in_transit(&b, &acts, 0),
            "the Guard is in transit, not on the line"
        );
        assert!(
            holding_line(&b, &acts, 0, Side::Party).is_empty(),
            "so it is NOT holding the line it just left"
        );
    }

    // ---- PERSISTENT OUTRIDERS + DISSOLUTION (the new idea) ---------------------------------------------

    /// **A landed raider is a PERSISTENT outrider.** It is still standing in the enemy zone the next round, loose
    /// in their ranks, and it fights with Melee (in-region, no screen), not a fresh raid.
    #[test]
    fn a_landed_raider_persists_as_an_outrider_and_melees() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 9, 1, 4, 2], true, false), // 0 - rich enough to cross
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),     // 1 - their front
                unit("Mage", Side::Foe, [3, 6, 9, 2, 2], false, true), // 2 - their back (Grit 9: survives)
            ],
            vec![0, 1, 1],
        );
        // Round 1: the Raider crosses in for the Mage and lands as an outrider in region 1. The hosts hold, so
        // the tough-Grit Mage survives the raid strike and is still there to be dug out next round.
        play_round(&mut b, &[Act::Raid(2, Answer::Push), Act::Hold, Act::Hold]);
        assert_eq!(b.regions[0], 1, "it is in the enemy zone");
        assert_eq!(b.ranks[0], Rank::Outrider, "and is an outrider there");

        // Next round it is offered Melee (in-region, no screen) at the host bodies - not a raid.
        let acts = legal_acts(&b, 0);
        assert!(
            acts.iter().any(|a| matches!(a, Act::Melee(_))),
            "an outrider melees in-region: {acts:?}"
        );
        assert!(
            !acts.iter().any(|a| matches!(a, Act::Raid(..))),
            "it does not raid - it is already inside"
        );
        // It can reach the Mage directly now, past the (still-standing) Wall's screen.
        assert!(
            acts.contains(&Act::Melee(2)),
            "it melees the cannon it came for"
        );
    }

    /// **An outrider whose host is wiped dissolves - as the last of its side, it stands where it is.** When
    /// outriders kill every body of a zone's formation they are outriders of nothing, so the state ends. Here the
    /// Raider is the *only* body of its side, so it simply becomes the formation where it stands (no line
    /// elsewhere to rejoin).
    #[test]
    fn an_outrider_of_a_wiped_host_dissolves_in_place_as_the_last_of_its_side() {
        // A lone party Raider already loose inside a foe zone; region 1 holds a single soft foe.
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [9, 9, 1, 4, 2], true, false), // 0 - the outrider
                unit("Scout", Side::Foe, [1, 2, 1, 1, 1], true, false), // 1 - lone host in region 1
                unit("Ogre", Side::Foe, [4, 6, 2, 2, 2], true, false),  // 2 - a second zone
            ],
            vec![1, 1, 2],
        );
        b.ranks[0] = Rank::Outrider; // the Raider raided region 1 last round
        assert_eq!(
            b.owner(1),
            Some(Side::Foe),
            "region 1 is the foe's while its Scout lives"
        );
        play_round(&mut b, &[Act::Melee(1), Act::Hold, Act::Hold]);
        assert!(b.units[1].fallen, "the Scout is dead");
        assert_ne!(
            b.ranks[0],
            Rank::Outrider,
            "so the Raider is no longer an outrider - the state dissolved"
        );
        assert_eq!(
            b.regions[0], 1,
            "the last of its side, it stays where it stands"
        );
        assert_eq!(b.owner(1), Some(Side::Party), "and region 1 is the party's");
    }

    /// **An outrider with a line to rejoin comes home.** When its host is wiped but its side still holds a
    /// formation elsewhere, the dissolved outrider reverts and **rejoins that line** rather than garrisoning
    /// empty ground.
    #[test]
    fn an_outrider_of_a_wiped_host_rejoins_its_own_line() {
        // A Guard holds the party line in region 0; a Raider is loose in the foe zone (region 1) about to kill
        // its lone host.
        let mut b = Board::new(
            vec![
                unit("Guard", Side::Party, [3, 5, 1, 2, 2], true, false), // 0 - the party line, region 0
                unit("Raider", Side::Party, [9, 9, 1, 4, 2], true, false), // 1 - the outrider, region 1
                unit("Scout", Side::Foe, [1, 2, 1, 1, 1], true, false), // 2 - the lone host, region 1
            ],
            vec![0, 1, 1],
        );
        b.ranks[1] = Rank::Outrider; // the Raider raided region 1 last round
        play_round(&mut b, &[Act::Hold, Act::Melee(2), Act::Hold]);
        assert!(b.units[2].fallen, "the Scout is dead");
        assert_ne!(
            b.ranks[1],
            Rank::Outrider,
            "the Raider's outrider state dissolved"
        );
        assert_eq!(
            b.regions[1], 0,
            "and it rejoined the party's line in region 0, not held empty ground"
        );
    }

    /// **Every sub-phase log is stamped with its phase, and a card-flip shows in the phase that caused it.** A
    /// plain clash resolves in the Outer Ring: the log names it, carries the strike, and its health snapshot shows
    /// the foe's flip *there* - while the Fire snapshot just before it still shows full health. This is what lets a
    /// transcript say *when* in the round a card turned, not merely *that* it did.
    #[test]
    fn each_phase_is_labeled_and_localizes_its_card_flips() {
        let mut b = Board::new(
            vec![
                unit("Hero", Side::Party, [3, 4, 1, 2, 2], true, false),
                unit("Foe", Side::Foe, [3, 4, 1, 2, 2], true, false),
            ],
            vec![0, 1],
        );
        let start_foe = b.units[1].health;
        let logs = play_round(&mut b, &[Act::Clash(1), Act::Hold]);
        assert!(
            logs.iter().all(|l| !l.phase.is_empty()),
            "every emitted phase is stamped with its name"
        );
        let clash = logs
            .iter()
            .find(|l| l.phase == "Outer Ring: Clash")
            .expect("a Clash phase");
        assert!(!clash.hits.is_empty(), "the clash carries the strike");
        assert!(
            clash.health[1] < start_foe,
            "the foe's card-flip shows in the Clash phase"
        );
        let fire = logs
            .iter()
            .find(|l| l.phase == "Outer Ring: Fire")
            .expect("a Fire phase");
        assert_eq!(
            fire.health[1], start_foe,
            "and not before it - the Fire step drew no blood"
        );
    }

    // ---- AoE WIDTH --------------------------------------------------------------------------------------

    /// **A sweep cannot reach a body a single strike could not.** A screened back line is safe from a sweep lobbed
    /// at their front: width, never reach.
    #[test]
    fn a_sweep_cannot_reach_a_body_a_single_strike_could_not() {
        let mut b = Board::new(
            vec![
                unit("Bombardier", Side::Party, [3, 3, 1, 1, 2], false, true).with_aoe(true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        let (wall, mage) = (b.units[1].health, b.units[2].health);
        play_round(&mut b, &[Act::Clash(1), Act::Clash(0), Act::Hold]);
        assert!(b.units[1].health < wall, "it sweeps their front line");
        assert_eq!(
            b.units[2].health, mage,
            "but must not touch the body behind that line"
        );
    }

    /// **Width comes free once the reach is paid for.** A melee area striker that crosses in sweeps the whole
    /// region it lands in - both tiers, because an outrider is past the screen.
    #[test]
    fn a_raider_with_an_area_strike_sweeps_the_region_it_lands_in() {
        let mut b = Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 4, 3, 5, 2], true, false).with_aoe(true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [4, 2, 1, 2, 2], false, true),
                unit("Seer", Side::Foe, [4, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1, 1],
        );
        let (mage, seer) = (b.units[2].health, b.units[3].health);
        play_round(
            &mut b,
            &[
                Act::Raid(2, Answer::Evade),
                Act::Clash(0),
                Act::Clash(0),
                Act::Clash(0),
            ],
        );
        assert_eq!(b.regions[0], 1, "it got in");
        assert!(
            b.units[2].health < mage && b.units[3].health < seer,
            "and swept the whole back line it landed among"
        );
    }

    /// **An outrider's melee sweep catches EVERY enemy in the region, both tiers.** Standing inside the ranks, its
    /// area strike no longer respects the screen.
    #[test]
    fn an_outrider_melee_sweep_catches_both_tiers() {
        let mut b = Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 9, 3, 5, 2], true, false).with_aoe(true), // 0 - the outrider
                unit("Wall", Side::Foe, [1, 4, 1, 1, 2], true, false), // 1 - front
                unit("Mage", Side::Foe, [1, 4, 1, 1, 2], false, true), // 2 - back
            ],
            vec![1, 1, 1],
        );
        b.ranks[0] = Rank::Outrider; // loose inside the foe zone
        let (wall, mage) = (b.units[1].health, b.units[2].health);
        play_round(&mut b, &[Act::Melee(1), Act::Hold, Act::Hold]);
        assert!(
            b.units[1].health < wall,
            "it sweeps the front it is standing among"
        );
        assert!(
            b.units[2].health < mage,
            "AND the back - an outrider is past the screen"
        );
    }

    /// **A sweep clears a pack; an aimed blow does not.** The horde axis, independent of reach.
    #[test]
    fn a_sweep_clears_a_pack_and_an_aimed_blow_does_not() {
        let felled = |stats: [u8; 5], aoe: bool, pack: u32| -> u32 {
            let mut b = Board::new(
                vec![
                    unit("Hero", Side::Party, stats, true, true).with_aoe(aoe),
                    unit("Horde", Side::Foe, [1, pack as u8, 1, 1, 1], true, false).as_horde(true),
                ],
                vec![0, 1],
            );
            play_round(&mut b, &[Act::Clash(1), Act::Hold]);
            pack - b.units[1].health
        };
        for pack in [8, 20, 40] {
            assert_eq!(
                felled([1, 3, 3, 1, 2], true, pack),
                pack,
                "Sweep clears a pack of {pack}"
            );
            assert_eq!(
                felled([3, 3, 1, 1, 2], true, pack),
                pack,
                "Salvo clears a pack of {pack}"
            );
        }
        let raider = felled([7, 6, 1, 2, 2], false, 8);
        assert!(
            raider <= 2,
            "an aimed blow fells one body per strike, not Might-many: {raider}"
        );
    }

    // ---- FOE SCRIPT -------------------------------------------------------------------------------------

    /// **The scripted foe actually uses the raid** when the softest thing on the board is a screened body.
    #[test]
    fn the_foes_raid_a_screened_body_when_it_is_the_softest_thing_on_the_board() {
        let b = Board::new(
            vec![
                unit("Bastion", Side::Party, [1, 6, 4, 1, 2], true, false),
                unit("Marksman", Side::Party, [5, 1, 1, 2, 2], false, true),
                unit("Ogre", Side::Foe, [5, 5, 2, 3, 2], true, false),
            ],
            vec![0, 0, 1],
        );
        let acts = foe_acts(&b);
        assert_eq!(
            acts[2],
            Some(Act::Raid(1, Answer::Push)),
            "it goes through the line for the soft body behind it"
        );
    }

    /// **HoldTheLine: a wall never abandons its post** - it clashes or melees in place, never raids or slips.
    #[test]
    fn a_hold_the_line_creature_never_leaves_its_region() {
        let mut b = Board::new(
            vec![
                unit("Softie", Side::Party, [1, 1, 1, 1, 1], false, true),
                unit("Wall", Side::Foe, [3, 6, 3, 2, 2], true, false),
                unit("Cannon", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        b.units[1].instinct = Instinct::HoldTheLine;
        let hold = foe_acts(&b);
        assert!(
            !matches!(hold[1], Some(Act::Raid(..)) | Some(Act::Slip(..))),
            "a HoldTheLine wall must never raid or slip: {:?}",
            hold[1]
        );
        assert!(
            matches!(
                hold[1],
                Some(Act::Clash(_)) | Some(Act::Melee(_)) | Some(Act::Hold)
            ),
            "it holds its post and clashes/melees what it can, or waits: {:?}",
            hold[1]
        );
    }

    // ---- RESOLVER INVARIANTS ----------------------------------------------------------------------------

    /// A messy board with every mechanic on it at once. If an invariant breaks anywhere, it breaks here.
    fn messy() -> (Board, Vec<Act>) {
        let b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false), // 0 front
                unit("Bastion", Side::Party, [1, 3, 3, 1, 2], true, false).with_aoe(true), // 1 front
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),               // 2 back
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false), // 3 front
                unit("Duelist", Side::Foe, [5, 5, 1, 2, 2], true, false), // 4 front
                unit("Swarm", Side::Foe, [2, 6, 1, 2, 2], false, true).as_horde(true), // 5 back
            ],
            vec![0, 0, 0, 1, 1, 1],
        );
        let acts = vec![
            Act::Raid(5, Answer::Evade), // the Raider crosses in for their back line
            Act::Clash(3),
            Act::Clash(4),
            Act::Clash(0),
            Act::Raid(2, Answer::Push), // the Duelist pushes through for our cannon
            Act::Clash(1),
        ];
        (b, acts)
    }

    /// **ORDER-INDEPENDENCE (Spec 1.9).** Permuting the seat order must yield the identical end-state.
    #[test]
    fn resolution_does_not_depend_on_seat_order() {
        let (b, acts) = messy();
        let n = b.units.len();

        let mut base = b.clone();
        play_round(&mut base, &acts);

        for shift in 1..n {
            let perm: Vec<usize> = (0..n).map(|i| (i + shift) % n).collect();
            let inv = {
                let mut v = vec![0usize; n];
                for (new, &old) in perm.iter().enumerate() {
                    v[old] = new;
                }
                v
            };
            let remap = |a: Act| match a {
                Act::Clash(t) => Act::Clash(inv[t]),
                Act::Raid(t, x) => Act::Raid(inv[t], x),
                Act::Melee(t) => Act::Melee(inv[t]),
                other => other,
            };

            let mut shuffled = Board::new(
                perm.iter().map(|&o| b.units[o].clone()).collect(),
                perm.iter().map(|&o| b.regions[o]).collect(),
            );
            let shuffled_acts: Vec<Act> = perm.iter().map(|&o| remap(acts[o])).collect();
            play_round(&mut shuffled, &shuffled_acts);

            for old in 0..n {
                let new = inv[old];
                assert_eq!(
                    (
                        base.units[old].health,
                        base.units[old].fallen,
                        base.regions[old],
                        base.ranks[old],
                    ),
                    (
                        shuffled.units[new].health,
                        shuffled.units[new].fallen,
                        shuffled.regions[new],
                        shuffled.ranks[new],
                    ),
                    "{} came out differently when re-seated (shift {shift})",
                    base.units[old].name
                );
            }
        }
    }

    /// **One body, two attackers: the answer must not depend on seating.**
    #[test]
    fn a_body_closed_on_from_two_sides_answers_the_same_way_whatever_the_seating() {
        let build = |names: [(&str, Side); 3]| {
            Board::new(
                vec![
                    unit(names[0].0, names[0].1, [4, 9, 1, 3, 1], true, false),
                    unit(names[1].0, names[1].1, [4, 9, 1, 3, 1], true, false),
                    unit(names[2].0, names[2].1, [4, 9, 1, 3, 1], true, false),
                ],
                vec![0, 1, 1],
            )
        };
        let mut a = build([("Hero", Side::Party), ("X", Side::Foe), ("Y", Side::Foe)]);
        play_round(&mut a, &[Act::Hold, Act::Clash(0), Act::Clash(0)]);
        let mut b = build([("Hero", Side::Party), ("Y", Side::Foe), ("X", Side::Foe)]);
        play_round(&mut b, &[Act::Hold, Act::Clash(0), Act::Clash(0)]);
        assert_eq!(
            (a.units[1].health, a.units[2].health),
            (b.units[2].health, b.units[1].health),
            "the hero answered a different attacker purely because the foes swapped seats"
        );
    }

    /// **Determinism.** Same position, same declarations, same board every time.
    #[test]
    fn resolution_is_deterministic() {
        let (b, acts) = messy();
        let run = || {
            let mut x = b.clone();
            play_round(&mut x, &acts);
            (
                x.units
                    .iter()
                    .map(|u| (u.health, u.fallen))
                    .collect::<Vec<_>>(),
                x.regions.clone(),
                x.ranks.clone(),
            )
        };
        assert_eq!(run(), run());
    }

    /// **Nothing goes out of bounds** across a whole fight.
    #[test]
    fn no_body_ends_a_round_in_an_impossible_state() {
        let (mut b, acts) = messy();
        let start: Vec<u32> = b.units.iter().map(|u| u.health).collect();
        for _ in 0..MAX_ROUNDS {
            if b.outcome().is_some() {
                break;
            }
            play_round(&mut b, &acts);
            for (i, u) in b.units.iter().enumerate() {
                assert!(u.health <= start[i], "{} healed itself", u.name);
                assert_eq!(
                    u.fallen,
                    u.health == 0,
                    "{} is fallen iff out of Health",
                    u.name
                );
            }
        }
    }

    /// **A committed blow lands even if its striker dies.**
    #[test]
    fn a_committed_blow_lands_even_if_its_striker_dies() {
        let mut b = Board::new(
            vec![
                unit("A", Side::Party, [9, 1, 1, 1, 1], true, false),
                unit("B", Side::Foe, [9, 1, 1, 1, 1], true, false),
            ],
            vec![0, 1],
        );
        play_round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
        assert!(b.units[0].fallen && b.units[1].fallen, "both blows landed");
        assert_eq!(b.outcome(), Some(false), "a mutual wipe is not a party win");
    }

    /// **A fallen body does nothing.**
    #[test]
    fn a_fallen_body_takes_no_part() {
        let mut b = messy().0;
        b.units[3].fallen = true;
        b.units[3].health = 0;
        assert!(legal_acts(&b, 3).is_empty(), "a corpse declares nothing");
        assert!(
            !b.vanguard(1, Side::Foe).contains(&3),
            "and it screens nothing"
        );
        assert!(
            foe_acts(&b)[3].is_none(),
            "and the script does not order it about"
        );
    }

    /// **The damage pile closes at the Reset, and only there.**
    #[test]
    fn sub_threshold_damage_does_not_carry_between_rounds() {
        let mut b = Board::new(
            vec![
                unit("Gnat", Side::Party, [1, 9, 1, 1, 9], true, false),
                unit("Boulder", Side::Foe, [0, 3, 5, 1, 1], true, false),
            ],
            vec![0, 1],
        );
        let boulder = b.units[1].health;
        for _ in 0..MAX_ROUNDS {
            play_round(&mut b, &[Act::Clash(1), Act::Hold]);
        }
        assert_eq!(
            b.units[1].health, boulder,
            "five rounds of sub-bar chip add up to nothing"
        );
    }

    /// **Every fight terminates.**
    #[test]
    fn every_fight_terminates() {
        let (mut b, acts) = messy();
        for _ in 0..MAX_ROUNDS {
            if b.outcome().is_some() {
                return;
            }
            play_round(&mut b, &acts);
        }
        assert!(b.outcome().is_some() || b.alive(Side::Party) && b.alive(Side::Foe));
    }

    /// **You fight who you declared.** A body that declared Hold does not swing back.
    #[test]
    fn a_body_fights_only_what_it_declared() {
        let mut b = Board::new(
            vec![
                unit("Attacker", Side::Party, [3, 5, 1, 1, 9], true, false),
                unit("Defender", Side::Foe, [4, 5, 1, 3, 1], true, false),
            ],
            vec![0, 1],
        );
        let (attacker, defender) = (b.units[0].health, b.units[1].health);
        play_round(&mut b, &[Act::Clash(1), Act::Hold]);
        assert!(b.units[1].health < defender, "the blow lands");
        assert_eq!(
            b.units[0].health, attacker,
            "Hold means hold - it takes the hit for free"
        );

        let mut c = Board::new(
            vec![
                unit("Attacker", Side::Party, [3, 5, 1, 1, 9], true, false),
                unit("Defender", Side::Foe, [4, 5, 1, 3, 1], true, false),
            ],
            vec![0, 1],
        );
        play_round(&mut c, &[Act::Clash(1), Act::Clash(0)]);
        assert!(
            c.units[0].health < attacker && c.units[1].health < defender,
            "both declared the fight, so both pay for it"
        );
    }

    /// The fight terminates and someone wins - the model does not stall.
    #[test]
    fn a_fight_resolves() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
                unit("Foe", Side::Foe, [1, 2, 1, 1, 1], true, false),
            ],
            vec![0, 1],
        );
        for _ in 0..MAX_ROUNDS {
            if b.outcome().is_some() {
                break;
            }
            play_round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
        }
        assert_eq!(b.outcome(), Some(true), "the Raider wins");
    }

    // ---- THE ORACLE -------------------------------------------------------------------------------------

    fn deep_board() -> Board {
        Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
                unit("The Wall", Side::Foe, [1, 4, 9, 1, 2], true, false),
            ],
            vec![0, 0, 1],
        )
    }

    fn hard_board() -> Board {
        Board::new(
            vec![
                unit("Marksman", Side::Party, [1, 1, 1, 1, 1], false, true),
                unit("The Wall", Side::Foe, [9, 9, 9, 3, 3], true, false),
            ],
            vec![0, 1],
        )
    }

    /// **SAFETY: a starved oracle is SILENT, never WRONG.**
    #[test]
    fn a_starved_oracle_is_silent_never_wrong() {
        for board in [deep_board(), hard_board()] {
            let truth = Oracle::new(u64::MAX).verdict(&board, 0);
            assert_ne!(truth, Verdict::Evaluating, "the control must settle");
            for grant in [1u64, 3, 8, 21] {
                let mut o = Oracle::new(0);
                for _ in 0..120 {
                    o.grant(grant);
                    let got = o.verdict(&board, 0);
                    assert!(
                        got == Verdict::Evaluating || got == truth,
                        "grant {grant} answered {got:?}, but the truth is {truth:?}"
                    );
                }
            }
        }
    }

    /// **LIVENESS: an escalating grind converges.**
    #[test]
    fn it_converges_when_the_grant_escalates() {
        for board in [deep_board(), hard_board()] {
            let truth = Oracle::new(u64::MAX).verdict(&board, 0);
            let mut o = Oracle::new(0);
            let mut grant = 1u64;
            let mut got = Verdict::Evaluating;
            for _ in 0..64 {
                o.grant(grant);
                got = o.verdict(&board, 0);
                if got != Verdict::Evaluating {
                    break;
                }
                grant *= 2;
            }
            assert_eq!(got, truth, "an escalating grind must reach the truth");
        }
    }
}
