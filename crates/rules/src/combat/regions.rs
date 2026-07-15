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
//! **Setup - round 1 only, secret and simultaneous.** Partition the party into **regions**, and post each body at
//! the **front** or the **back** of its region. Foes do the same by script, so their arrangement is effectively
//! public.
//!
//! **Thereafter you declare only an action.** You start each round where you ended. **Position is never
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
//! - **No rearguard without a vanguard** ([`Board::promote`]). A front collapses, its back is promoted to front.
//!   So *"at the back"* and *"screened"* are the **same fact** - a whole predicate disappears.
//! - **A melee body at the back is dead weight.** It cannot attack; that is the price of hiding behind the
//!   vanguard. Nothing bans it - posting a Raider at the back simply punishes itself. Force, not fiat.
//! - **Slipping is the only movement in the game.** Retreat, regroup and raiding are *one mechanic* with
//!   different destinations.
//!
//! ## What a body can do ([`Act`])
//!
//! | | |
//! |---|---|
//! | [`Act::Clash`] | strike an enemy **vanguard**. Free - melee or ranged, any region. |
//! | [`Act::Raid`] | **slip** their front to strike an enemy **rearguard**, and end up standing in their region. Melee only. |
//! | [`Act::Slip`] | **slip** to another region. Retreat, or regroup. |
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

/// The four sub-phases, in order. Each earns its boundary by exactly one silencing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPhase {
    /// The **front line** reaches for the slippers. A death here silences the Volley *and* the Raid.
    Intercept,
    /// The **back line** fires on whoever is still coming. The cannons defend themselves - and a slipper the
    /// front already killed is not shot at twice, which is exactly why this is its own sub-phase: **the saved
    /// card is real.** A death here silences the Raid.
    Volley,
    /// Those who got through strike the backs they came for - *before* those backs get to fire. A death here
    /// silences that victim's own shot, and that is the whole of what the raid buys.
    Raid,
    /// **The back lines fire.** Holding off *is* being quicker: a body that never closed shoots before the ones
    /// that did can swing. A death here silences that victim's Clash - which is exactly why a screened cannon is
    /// worth screening, and why a lone archer can beat a swordsman it could never out-trade.
    Fire,
    /// **The front lines close, and trade.** Seeking melee is the slowest thing you can do, so it lands last.
    Clash,
}

impl SubPhase {
    pub const ALL: [SubPhase; 5] = [
        SubPhase::Intercept,
        SubPhase::Volley,
        SubPhase::Raid,
        SubPhase::Fire,
        SubPhase::Clash,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SubPhase::Intercept => "Intercept",
            SubPhase::Volley => "Volley",
            SubPhase::Raid => "Raid",
            SubPhase::Fire => "Fire",
            SubPhase::Clash => "Clash",
        }
    }
}

/// **Where a body stands in its line - and it is a statement of INTENT, not a rank.**
///
/// *Seek melee*, or *avoid it*. That is the whole of it, and everything else follows:
///
/// - A body at the **front** is in the fight. It can be clashed, it **catches slippers**, and it swings **last**
///   (the Clash) - because closing to melee is the slowest thing you can do.
/// - A body at the **back** is holding off. It **fires first** (the Volley), it cannot be clashed while its own
///   front stands (a raider has to come *in* for it), and a **melee** body posted here is **dead weight** - it
///   declared that it is avoiding melee, and a sword cannot do anything else.
///
/// **A back whose front has collapsed is not promoted.** This is the collapsed-vanguard rule, and it is the
/// hinge the whole design turns on: the back **stays** a back. It becomes *targetable* - anyone may now clash it
/// directly, no raid required (force, not fiat) - but it **keeps its phase slot**, and so it still shoots
/// *before* the front swings.
///
/// That is what makes range mean something. A lone archer posts to the back, is perfectly targetable, and gets
/// **the first hit** anyway. *Close in and you trade and die; answer it from range.* Promotion destroyed exactly
/// this: it turned every unscreened cannon into just another front-line body, and left "ranged" gating nothing a
/// solo fight could see.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Post {
    /// **Seek melee.** In the fight: clashable, catches slippers, swings last.
    Front,
    /// **Avoid melee.** Fires first; reachable only by a raid while a front still stands, and reachable by
    /// anyone once it does not. A melee body here does nothing at all.
    Back,
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
    /// Melee only. This is the old Outrider entire: not a role, but *what reaching a screened body costs*.
    Raid(usize, Answer),
    /// **Strike a body in your OWN region** - an intruder loose in your ranks, or (if you are the intruder) any
    /// host body. No screen applies in-region: the tiers stopped protecting anyone the moment a body got inside
    /// them. Not a crossing, so it carries no evade-answer.
    Melee(usize),
    /// **Slip to another region** - retreat out of a region that has been breached, or regroup with allies. The
    /// same contest as a raid; only the destination differs.
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

/// The whole position: the bodies, which region each stands in, where each is posted, and which are intruders.
#[derive(Clone, Debug)]
pub struct Board {
    pub units: Vec<Combatant>,
    /// Which region each body stands in. Ids are arbitrary - only the **partition** is meaningful.
    pub regions: Vec<u8>,
    /// Where each body is posted within its region. **Derived from the weapon and fixed for the whole fight**:
    /// a ranged-only body is `Back`, everything else `Front`. It is never chosen and never mutated - post
    /// describes intent (seek melee / avoid it), and a weapon does not change mid-fight. An intruder's post is
    /// meaningless while it is loose (it is not part of any formation), and it resumes exactly this value the
    /// moment it promotes.
    pub posts: Vec<Post>,
    /// **Is body `i` an intruder** - loose in a zone owned by the enemy, having raided/slipped in and not yet
    /// promoted or retreated. Index-aligned with `units`. An intruder is never screened and is never part of a
    /// vanguard or a formation: it is past the screen, in enemy territory, unscreened.
    pub intruders: Vec<bool>,
}

impl Board {
    /// Build a position. **Post is derived from the weapon** (ranged-only -> `Back`, else `Front`) and nobody
    /// starts an intruder.
    pub fn new(units: Vec<Combatant>, regions: Vec<u8>) -> Board {
        let posts = units.iter().map(Board::weapon_post).collect();
        let n = units.len();
        Board {
            units,
            regions,
            posts,
            intruders: vec![false; n],
        }
    }

    /// The post a body takes from its weapon: a ranged-only body avoids melee (`Back`); anything that can strike
    /// in melee stands at the `Front`. A dual melee+ranged body is `Front` (the deferred case - there are none).
    pub fn weapon_post(u: &Combatant) -> Post {
        if u.ranged && !u.melee {
            Post::Back
        } else {
            Post::Front
        }
    }

    /// **Who owns `region`** - the side of its living, non-intruder bodies (its *formation*). There is at most
    /// one such side. `None` when the region holds only intruders or is empty.
    pub fn owner(&self, region: u8) -> Option<Side> {
        self.in_region(region)
            .into_iter()
            .find(|&i| !self.intruders[i])
            .map(|i| self.units[i].side)
    }

    /// **Is `target` screened?** - it is a **non-intruder** posted at the back, *and* its side still has a living
    /// front (its formation's vanguard) in its region to do the screening.
    ///
    /// An intruder is **never** screened - it is loose inside the enemy ranks, adjacent to everyone. A back with
    /// no front left is **not promoted**: it stays a back, keeps its phase slot, and simply becomes reachable.
    pub fn is_screened(&self, target: usize) -> bool {
        !self.intruders[target]
            && self.posts[target] == Post::Back
            && !self
                .vanguard(self.regions[target], self.units[target].side)
                .is_empty()
    }

    /// The **vanguard** of `side` in `region` - the living, **non-intruder** bodies at the front. This is the
    /// whole screen: every one can catch a crosser, and it gets shorter each time one dies. Intruders are never
    /// part of a vanguard.
    pub fn vanguard(&self, region: u8, side: Side) -> Vec<usize> {
        self.in_region(region)
            .into_iter()
            .filter(|&i| {
                !self.intruders[i] && self.units[i].side == side && self.posts[i] == Post::Front
            })
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

    /// The first region nobody stands in - where a body slips to when it just wants *out*.
    pub fn open_ground(&self) -> u8 {
        let taken = self.occupied();
        (0u8..).find(|r| !taken.contains(r)).unwrap_or(u8::MAX)
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
            if board.regions[t] == here {
                // **In your own region.** The two are intermingled (one of you is an intruder), so there is no
                // screen between you: any weapon reaches any enemy body here.
                out.push(Act::Melee(t));
            } else if board.intruders[t] {
                // A loose enemy body in another region is dealt with in-region by the formation that hosts it,
                // never reached across the gap. Not a target from here.
            } else if board.is_screened(t) {
                // A screened rearguard in another region. Only a melee body can cross in for it.
                if u.melee {
                    out.extend(ANSWERS.map(|a| Act::Raid(t, a)));
                }
            } else {
                // An enemy vanguard (front, or a collapsed back) in another region: reachable by any weapon.
                out.push(Act::Clash(t));
            }
        }
    }

    // Slip - the one movement. It goes ONLY to a region that already holds bodies: an enemy zone (raid-style) or
    // a friendly zone (rally). No slipping onto empty ground - EXCEPT an intruder may always retreat to a fresh
    // home region, so a stranded raider is never trapped.
    let mut elsewhere: Vec<u8> = board
        .occupied()
        .into_iter()
        .filter(|&r| r != here)
        .collect();
    if board.intruders[i] {
        elsewhere.push(board.open_ground()); // the intruder's sanctioned retreat onto open ground
    }
    for r in elsewhere {
        out.extend(ANSWERS.map(|a| Act::Slip(r, a)));
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
        // Hunt anything: a clash, an in-region melee (dig out an intruder, or - if it is the intruder - strike a
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

// ---- resolution ----------------------------------------------------------------------------------------

/// What happened in one sub-phase - enough for a transcript or a renderer to say *why* the board changed.
#[derive(Clone, Debug, Default)]
pub struct SubPhaseLog {
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
    /// Every body's **post** at this boundary. Same reason: without it a promotion that happens in the last
    /// sub-phase appears to have been true all round.
    pub posts: Vec<Post>,
}

fn slip_price(bid: u32, f_def: u32) -> u32 {
    bid / f_def.max(1) + 1
}

/// **Commit tempo to reach a target** - [`combat::resolve_engage`] with the *rank model's screen taken out*.
///
/// We cannot call `combat::resolve_engage` here, and the reason is worth stating: it runs `back_access_ok`,
/// which is the **old rank model's** back-access rule - it silently discards any engagement aimed at a
/// `Rank::Rearguard` while that side still has a living `Rank::Vanguard`. That is a *screen*, and this model
/// already has one: [`Post`] plus the slip contest. Inheriting the old one on top of it made a screened body
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
        board.units[e.attacker].tempo -= cards;
        contacts.push(Contact {
            attacker: e.attacker,
            target: e.target,
            bid: cards * finesse,
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
    (1..=units[a].tempo)
        .find(|&c| slip_price(c * units[a].finesse.max(1), units[t].finesse) > units[t].tempo)
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
fn area_strike(
    board: &mut Board,
    attacker: usize,
    region: u8,
    tier: Post,
    both_tiers: bool,
) -> Vec<Contact> {
    if board.units[attacker].fallen || board.units[attacker].tempo == 0 {
        return Vec::new();
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
        .filter(|&j| whole || board.posts[j] == tier)
        .collect();

    let mut contacts = Vec::new();
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
                board.units[j].health = 0;
            }
        } else {
            contacts.push(Contact {
                attacker,
                target: j,
                bid: 0, // no bid: it cannot be evaded, and nobody answers along it
            });
        }
    }
    contacts
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
fn land(board: &mut Board, contacts: &[Contact], sweeps: &[Contact], extra: &[Blows]) {
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

    for (a, t, hits) in strikes {
        if hits == 0 {
            continue;
        }
        if board.units[t].horde {
            felled[t] += hits; // one blow, one body - Might buys nothing against a pack
        } else {
            let per = board.units[a].might.saturating_sub(board.units[t].armor);
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

/// Close a sub-phase: finalize deaths, then **promote any back line whose front just collapsed**. That promotion
/// is how the ground behind a broken line opens up *within* a round.
fn close(board: &mut Board, before: &[bool]) -> SubPhaseLog {
    end_sub_phase(&mut board.units);
    SubPhaseLog {
        fallen: (0..board.units.len())
            .filter(|&i| before[i] && board.units[i].fallen)
            .collect(),
        health: board.units.iter().map(|u| u.health).collect(),
        posts: board.posts.clone(),
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
                && board.posts[i] == Post::Back
                && !in_transit(board, acts, i)
        })
        .collect()
}

/// One pass of bodies reaching for the slippers: they commit, the slipper answers as it declared, the blows
/// land. Returns the contacts that stuck (a slipper that Evaded broke them all).
///
/// This is used **twice** - once for the front line ([`SubPhase::Intercept`]) and once for the back line
/// ([`SubPhase::Volley`]) - and the two are separate sub-phases for a reason that is pure tempo economy: **if
/// the front already killed the slipper, the back does not waste its shot on a corpse.** That saved card is the
/// whole reason the boundary earns its place under the razor.
fn reach_for_slippers(
    board: &mut Board,
    catchers: &[(usize, usize)], // (catcher, slipper)
    movers: &[(usize, u8, Answer)],
) -> Vec<Contact> {
    let engagements: Vec<Engage> = catchers
        .iter()
        .filter(|&&(f, s)| {
            !board.units[f].fallen && !board.units[s].fallen && board.units[f].tempo > 0
        })
        .map(|&(f, s)| Engage {
            attacker: f,
            target: s,
            cards: reach_cards(&board.units, f, s),
        })
        .collect();
    let reaching = engage(board, &engagements);

    // The slipper answers, seeing exactly what was committed. Evade pays in full and breaks every edge; Push and
    // Abort spend nothing and eat the blows.
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| match movers.iter().find(|&&(m, _, _)| m == i) {
            Some(&(_, _, Answer::Evade)) => Dodge::Slip,
            _ => Dodge::Stand,
        })
        .collect();
    let landed = resolve_evade(&mut board.units, &reaching, &dodges);

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
    land(board, &landed, &[], &ripostes);
    landed
}

/// The enemy of a side.
fn other_side(s: Side) -> Side {
    match s {
        Side::Party => Side::Foe,
        Side::Foe => Side::Party,
    }
}

/// **Zone promotion - clearing ground takes it.** Any region whose owning formation has been wiped, leaving only
/// living intruders, flips to those intruders' side *on the spot*: they stop being intruders and become the
/// region's formation, resuming their weapon-fixed posts (which never changed). Called at the Band-1 boundary,
/// where an intruder's havoc is what wipes a formation.
fn promote(board: &mut Board) {
    for r in board.occupied() {
        let bodies = board.in_region(r);
        let has_formation = bodies.iter().any(|&i| !board.intruders[i]);
        if !has_formation {
            for i in bodies {
                board.intruders[i] = false; // they own the ground now
            }
        }
    }
}

/// Play **one whole round**: the Reset, then the three distance bands nearest-first - **Intruders** (distance
/// zero), **Crossings** (closing into a formation), and **Across the gap** (Fire then Clash). Deaths finalize at
/// each step boundary, so a body killed early is silenced in every later step (the razor).
pub fn play_round(board: &mut Board, acts: &[Act]) -> Vec<SubPhaseLog> {
    refresh_round(&mut board.units);
    let mut logs = Vec::new();

    // ---- BAND 1: INTRUDERS (distance zero) --------------------------------------------------------------
    //
    // Every region holding both a formation and enemy intruders resolves its in-place fight with NO screen: the
    // intruder strikes any host it declared, the host strikes any intruder it declared. Ranged before melee (the
    // shot lands before the swing). An intruder is past the screen, so a melee sweep here catches EVERY enemy in
    // the region, both tiers.
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
    for want_ranged in [true, false] {
        let group: Vec<Attack> = melees
            .iter()
            .filter(|&&(a, _)| (board.units[a].ranged && !board.units[a].melee) == want_ranged)
            .copied()
            .collect();
        if group.is_empty() {
            continue;
        }
        let before = living(board);
        exchange(board, &group, true, true);
        logs.push(close(board, &before));
    }
    // Clearing a zone's formation with intruders takes it - flip those intruders to the owning side, on the spot.
    promote(board);

    // ---- BAND 2: CROSSINGS (closing into a formation) ---------------------------------------------------
    //
    // Every declared Raid/Slip sends its body across as a transient. An enemy formation reaches for a crosser at
    // BOTH ends it touches - the zone ENTERED and the zone LEFT - because you are outside your own screen the
    // moment you move. At each such end the FRONT intercepts (spears) THEN the BACK volleys the survivors (bows),
    // so a front-killed crosser is not volleyed. A friendly zone never reaches for its own, so a rally between
    // friendly zones is free; but an intruder pulling OUT of enemy ranks is opposed by the ranks it leaves (the
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
    reach_for_slippers(board, &front_catchers, &movers);
    logs.push(close(board, &before));
    let before = living(board);
    reach_for_slippers(board, &back_catchers, &movers);
    logs.push(close(board, &before));

    // LAND: survivors that got through leave their post and arrive. Into an enemy zone they become intruders
    // (loose inside the ranks, no post); a rally into a friendly zone joins that formation, non-intruder.
    let mut through = vec![false; board.units.len()];
    let mut landing = SubPhaseLog {
        health: board.units.iter().map(|u| u.health).collect(),
        posts: board.posts.clone(),
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
        board.intruders[i] = into_enemy;
        through[i] = true;
        landing.through.push(i);
    }
    if let Some(last) = logs.last_mut() {
        last.through = landing.through.clone();
        last.aborted = landing.aborted.clone();
    }

    // The raiders that got through strike the rearguard they came for - before it can fire in Band 3. Tempo-gated:
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
    exchange(board, &raids, false, false);
    logs.push(close(board, &before));

    // ---- BAND 3: ACROSS THE GAP (Fire then Clash) -------------------------------------------------------
    //
    // The standing formations trade at each other's vanguards in other regions. Every back line fires first
    // (holding off IS being quicker), then every front line closes and trades. Ranged before melee.
    for tier in [Post::Back, Post::Front] {
        let before = living(board);
        let attacks: Vec<Attack> = (0..board.units.len())
            .filter(|&i| {
                !board.units[i].fallen
                    && !board.intruders[i]
                    && board.units[i].tempo > 0
                    && board.posts[i] == tier
            })
            .filter_map(|i| match acts[i] {
                Act::Clash(t) if !board.units[t].fallen => Some((i, t)),
                _ => None,
            })
            .collect();
        exchange(board, &attacks, true, false);
        logs.push(close(board, &before));
    }

    logs
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged.** Area strikes split off and
/// sweep their target's region: the tier aimed at, or - when `sweep_whole` - both tiers (an in-region melee, past
/// the screen).
fn exchange(board: &mut Board, attacks: &[Attack], pour: bool, sweep_whole: bool) {
    let mut sweeps: Vec<Contact> = Vec::new();
    let mut aimed: Vec<Engage> = Vec::new();
    for &(a, t) in attacks {
        if board.units[a].aoe {
            let (region, tier) = (board.regions[t], board.posts[t]);
            sweeps.extend(area_strike(board, a, region, tier, sweep_whole));
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
    land(board, &contacts, &sweeps, &extra);
}

// ---- the doom oracle -----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Winnable,
    /// The search ran out of budget. **Not an answer** - an answer in progress.
    Evaluating,
    Doomed,
}

/// The memo key: per-unit `(health, fallen, post, intruder)`, the **canonicalized** partition, and the round.
/// Two positions that differ only by who is loose inside the enemy ranks are genuinely different positions.
///
/// Tempo and the damage pile are absent on purpose - both are re-derived by the round Reset, and we only memoize
/// at a **round** boundary. The product's own rule (the round is the one deadline) paying a dividend.
type Key = (Vec<(u32, bool, Post, bool)>, Vec<u8>, usize);

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
                .map(|i| {
                    (
                        board.units[i].health,
                        board.units[i].fallen,
                        board.posts[i],
                        board.intruders[i],
                    )
                })
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

    // ---- POST IS DERIVED --------------------------------------------------------------------------------

    /// **Post is derived from the weapon, not chosen.** A ranged-only body is `Back`; anything that can strike in
    /// melee is `Front`. Fixed at construction, for heroes and foes alike.
    #[test]
    fn post_is_derived_from_the_weapon() {
        let b = Board::new(
            vec![
                unit("Sword", Side::Party, [3, 3, 1, 2, 2], true, false),
                unit("Bow", Side::Party, [3, 3, 1, 2, 2], false, true),
                unit("Skirmisher", Side::Foe, [3, 3, 1, 2, 2], true, true), // dual - deferred, treated as front
            ],
            vec![0, 0, 1],
        );
        assert_eq!(b.posts[0], Post::Front, "a melee body stands front");
        assert_eq!(b.posts[1], Post::Back, "a ranged-only body stands back");
        assert_eq!(
            b.posts[2],
            Post::Front,
            "a dual body is front (the deferred case)"
        );
        assert!(b.intruders.iter().all(|&x| !x), "nobody starts an intruder");
    }

    // ---- REACH: SCREEN, RAID, CLASH ---------------------------------------------------------------------

    /// **A back whose front has fallen is targetable but still a back.** It is not promoted into the line - it
    /// keeps its post (so it still fires first) but loses its screen (so anyone may clash it now). Force, not fiat.
    #[test]
    fn a_back_whose_front_has_fallen_is_targetable_but_still_a_back() {
        let mut b = wall_and_cannon();
        assert_eq!(b.posts[1], Post::Back, "the cannon starts behind the wall");
        assert!(b.is_screened(1), "and is screened by it");
        assert!(
            !legal_acts(&b, 2).contains(&Act::Clash(1)),
            "so the Ogre cannot simply clash it - it must raid"
        );

        b.units[0].fallen = true; // the wall dies
        assert_eq!(
            b.posts[1],
            Post::Back,
            "the cannon is STILL a cannon - not promoted"
        );
        assert!(!b.is_screened(1), "but nothing is screening it any more");
        assert!(
            legal_acts(&b, 2).contains(&Act::Clash(1)),
            "so it is targetable now - force, not fiat"
        );
    }

    /// **A lone archer at the back shoots before a swordsman can swing.** It is fully targetable, and wins anyway
    /// because it fires in Band 3's Fire while the swordsman closes in the later Clash. Posted at the front, the
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
            b.posts[0],
            Post::Back,
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

    // ---- THE CROSSING (Band 2) --------------------------------------------------------------------------

    /// **THE ONE THAT MATTERS. The screen is a PRICE, not an immunity** - enough Tempo gets past a front and
    /// reaches the body behind it. And a landed raider ends up standing INSIDE the enemy formation, as an
    /// intruder.
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
        assert!(b.intruders[2], "as an intruder - loose in the enemy ranks");
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
        assert!(!b.intruders[2], "and it is no intruder");
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

    // ---- PERSISTENT INTRUDERS + PROMOTION (the new idea) ------------------------------------------------

    /// **A landed raider is a PERSISTENT intruder.** It is still standing in the enemy zone the next round, loose
    /// in their ranks, and it fights with Melee (in-region, no screen), not a fresh raid.
    #[test]
    fn a_landed_raider_persists_as_an_intruder_and_melees() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 9, 1, 4, 2], true, false), // 0 - rich enough to cross
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),     // 1 - their front
                unit("Mage", Side::Foe, [3, 6, 9, 2, 2], false, true), // 2 - their back (Grit 9: survives)
            ],
            vec![0, 1, 1],
        );
        // Round 1: the Raider crosses in for the Mage and lands as an intruder in region 1. The hosts hold, so
        // the tough-Grit Mage survives the raid strike and is still there to be dug out next round.
        play_round(&mut b, &[Act::Raid(2, Answer::Push), Act::Hold, Act::Hold]);
        assert_eq!(b.regions[0], 1, "it is in the enemy zone");
        assert!(b.intruders[0], "and is an intruder there");

        // Next round it is offered Melee (in-region, no screen) at the host bodies - not a raid.
        let acts = legal_acts(&b, 0);
        assert!(
            acts.iter().any(|a| matches!(a, Act::Melee(_))),
            "an intruder melees in-region: {acts:?}"
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

    /// **Zone promotion - clearing a formation takes the ground.** When intruders kill every body of a zone's
    /// formation, that zone flips to their side on the spot: the intruders become the formation.
    #[test]
    fn clearing_a_zones_formation_promotes_the_intruders() {
        // A party Raider already loose inside a two-zone foe formation; region 1 holds a single soft foe.
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [9, 9, 1, 4, 2], true, false), // 0 - the intruder
                unit("Scout", Side::Foe, [1, 2, 1, 1, 1], true, false), // 1 - lone body in region 1
                unit("Ogre", Side::Foe, [4, 6, 2, 2, 2], true, false), // 2 - region 2, a second zone
            ],
            vec![1, 1, 2],
        );
        b.intruders[0] = true; // the Raider raided region 1 last round
        assert_eq!(
            b.owner(1),
            Some(Side::Foe),
            "region 1 is the foe's while its Scout lives"
        );
        // The intruder melees the lone host to death; the Ogre in region 2 holds.
        play_round(&mut b, &[Act::Melee(1), Act::Hold, Act::Hold]);
        assert!(b.units[1].fallen, "the Scout is dead");
        assert!(
            !b.intruders[0],
            "so the Raider is promoted - no longer an intruder"
        );
        assert_eq!(
            b.owner(1),
            Some(Side::Party),
            "and region 1 has flipped to the party"
        );
    }

    /// **An intruder pulling out is opposed by the ranks it leaves.** Retreat is a crossing in reverse: you are
    /// outside your screen the moment you move, so the enemy formation you abandon reaches for you. A pushed
    /// retreat arrives - but bloodied. (Before the both-ends fix, only the destination opposed a crosser, so a
    /// retreat into a friendly zone was free.)
    #[test]
    fn an_intruder_retreat_is_opposed_by_the_zone_it_leaves() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 3, 1, 4, 2], true, false), // 0 - the intruder (Grit 1)
                unit("Wall", Side::Foe, [2, 6, 3, 2, 2], true, false), // 1 - the ranks it leaves (Might 2 > Grit 1)
                unit("Ally", Side::Party, [3, 4, 2, 2, 2], true, false), // 2 - holds the friendly home zone
            ],
            vec![1, 1, 0],
        );
        b.intruders[0] = true; // the Raider is loose inside the foe's region 1

        let before = b.units[0].health;
        // It pushes out to the friendly region 0. It arrives - but the Wall it leaves swings at it on the way.
        play_round(&mut b, &[Act::Slip(0, Answer::Push), Act::Hold, Act::Hold]);

        assert_eq!(b.regions[0], 0, "a pushed retreat still arrives");
        assert!(
            !b.intruders[0],
            "and it has rejoined a friendly zone, no longer an intruder"
        );
        assert!(
            b.units[0].health < before,
            "but the ranks it left drew blood on the way out ({before} -> {})",
            b.units[0].health
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
    /// region it lands in - both tiers, because an intruder is past the screen.
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

    /// **An intruder's melee sweep catches EVERY enemy in the region, both tiers.** Standing inside the ranks, its
    /// area strike no longer respects the screen.
    #[test]
    fn an_intruder_melee_sweep_catches_both_tiers() {
        let mut b = Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 9, 3, 5, 2], true, false).with_aoe(true), // 0 - the intruder
                unit("Wall", Side::Foe, [1, 4, 1, 1, 2], true, false), // 1 - front
                unit("Mage", Side::Foe, [1, 4, 1, 1, 2], false, true), // 2 - back
            ],
            vec![1, 1, 1],
        );
        b.intruders[0] = true; // loose inside the foe zone
        let (wall, mage) = (b.units[1].health, b.units[2].health);
        play_round(&mut b, &[Act::Melee(1), Act::Hold, Act::Hold]);
        assert!(
            b.units[1].health < wall,
            "it sweeps the front it is standing among"
        );
        assert!(
            b.units[2].health < mage,
            "AND the back - an intruder is past the screen"
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
                        base.posts[old],
                        base.intruders[old],
                    ),
                    (
                        shuffled.units[new].health,
                        shuffled.units[new].fallen,
                        shuffled.regions[new],
                        shuffled.posts[new],
                        shuffled.intruders[new],
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
                x.intruders.clone(),
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
