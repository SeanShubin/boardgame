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
//! **Thereafter you declare only an action.** You start each round where you ended. **Position is never declared
//! - it is only ever earned.** That is what makes movement *priced by construction*: `v2_remarshal` proved a
//! costless repositioning offered every round can always be pre-empted by starting in the right place, so it can
//! never be *necessary*. Here you cannot ask to move. You have to win it.
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

use crate::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};

/// A fight not decided in five rounds is a draw, and a draw is not a win (spec 0.4).
pub const MAX_ROUNDS: usize = 5;

/// The three sub-phases, in order. See the module docs for why there are exactly three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPhase {
    Slip,
    Raid,
    Clash,
}

impl SubPhase {
    pub const ALL: [SubPhase; 3] = [SubPhase::Slip, SubPhase::Raid, SubPhase::Clash];

    pub fn label(self) -> &'static str {
        match self {
            SubPhase::Slip => "Slip",
            SubPhase::Raid => "Raid",
            SubPhase::Clash => "Clash",
        }
    }
}

/// Where a body stands within its side's contingent in its region. **Persistent state, not a declaration** - set
/// once at setup, and thereafter changed only by [`Board::promote`] or by slipping into somewhere new.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Post {
    /// **The vanguard.** It can be clashed with and shot at, and it **catches slippers**. Being reachable is the
    /// job.
    Front,
    /// **The rearguard.** Unreachable by clash or by fire while a front stands - a raider must slip in for it. A
    /// *melee* body here is **dead weight**: it cannot attack at all. That is the price of hiding.
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

/// The whole position: the bodies, which region each stands in, and where each is posted.
#[derive(Clone, Debug)]
pub struct Board {
    pub units: Vec<Combatant>,
    /// Which region each body stands in. Ids are arbitrary - only the **partition** is meaningful.
    pub regions: Vec<u8>,
    /// Where each body is posted within its region.
    pub posts: Vec<Post>,
}

impl Board {
    /// Build a position and immediately enforce no-rearguard-without-a-vanguard.
    pub fn new(units: Vec<Combatant>, regions: Vec<u8>, posts: Vec<Post>) -> Board {
        let mut b = Board {
            units,
            regions,
            posts,
        };
        b.sync_ranks();
        b.promote();
        b
    }

    /// **Adapter, not mechanic.** [`Combatant::rank`] is vestigial here - position is a region plus a [`Post`].
    /// But we reuse `combat`'s resolvers unchanged, and [`combat::resolve_engage`] gates every engagement through
    /// `effective_in_rank`: a body whose rank does not match its reach lands **nothing**.
    ///
    /// That gate once silently disabled every ranged body in this model (all units were built at
    /// `Rank::Vanguard`, which it reads as "melee"), so the Marksman never landed a single aimed shot - and every
    /// unit test still passed. Keep rank consistent with reach, or bodies go mute.
    fn sync_ranks(&mut self) {
        for u in &mut self.units {
            u.rank = if u.ranged && !u.melee {
                deckbound_content::rank::Intention::Rearguard
            } else {
                deckbound_content::rank::Intention::Vanguard
            };
        }
    }

    /// **No rearguard without a vanguard.** If a side's front in a region is gone but its back is not, the back is
    /// promoted - on the spot, at the boundary where the front died.
    ///
    /// This is what opens the ground behind a broken line, and it makes *"at the back"* and *"screened"* the same
    /// fact, so there is no separate predicate to get wrong.
    pub fn promote(&mut self) {
        for r in self.occupied() {
            for side in [Side::Party, Side::Foe] {
                let here: Vec<usize> = self
                    .in_region(r)
                    .into_iter()
                    .filter(|&i| self.units[i].side == side)
                    .collect();
                if !here.is_empty() && here.iter().all(|&i| self.posts[i] == Post::Back) {
                    for i in here {
                        self.posts[i] = Post::Front;
                    }
                }
            }
        }
    }

    /// The **vanguard** of `side` in `region` - the living bodies at the front. This is the whole screen: every
    /// one of them can catch a slipper, and it gets shorter each time one of them dies.
    pub fn vanguard(&self, region: u8, side: Side) -> Vec<usize> {
        self.in_region(region)
            .into_iter()
            .filter(|&i| self.units[i].side == side && self.posts[i] == Post::Front)
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
    // A melee body posted behind the line is dead weight: no attack of any kind.
    let hiding = board.posts[i] == Post::Back && !u.ranged;

    if !hiding && (u.melee || u.ranged) {
        for (t, e) in board.units.iter().enumerate() {
            if e.fallen || e.side == u.side {
                continue;
            }
            match board.posts[t] {
                // Their vanguard is reachable by anyone from anywhere: a region is a formation, not a place.
                Post::Front => out.push(Act::Clash(t)),
                // Their rearguard is behind their front. Only a melee body can slip in for it.
                Post::Back if u.melee => {
                    out.extend(ANSWERS.map(|a| Act::Raid(t, a)));
                }
                Post::Back => {}
            }
        }
    }

    // Slip away - retreat from a breached region, or regroup. Anyone may, melee or ranged.
    let mut elsewhere: Vec<u8> = board
        .occupied()
        .into_iter()
        .filter(|&r| r != board.regions[i])
        .collect();
    if board.in_region(board.regions[i]).len() > 1 {
        elsewhere.push(board.open_ground()); // peel off alone
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
    (0..board.units.len())
        .map(|i| {
            if board.units[i].side != Side::Foe || board.units[i].fallen {
                return None;
            }
            let softest = |t: usize| (board.units[t].health, board.units[t].grit);
            let acts = legal_acts(board, i);
            let clash = acts
                .iter()
                .filter_map(|a| match a {
                    Act::Clash(t) => Some((softest(*t), *a)),
                    _ => None,
                })
                .min_by_key(|&(k, _)| k)
                .map(|(_, a)| a);
            let raid = acts
                .iter()
                .filter_map(|a| match a {
                    Act::Raid(t, Answer::Push) => Some((softest(*t), *a)),
                    _ => None,
                })
                .min_by_key(|&(k, _)| k)
                .map(|(_, a)| a);
            clash.or(raid).or(Some(Act::Hold))
        })
        .collect()
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
            let Some(cost) = combat::slip_cost(&board.units, reaching, i) else {
                return Dodge::Stand;
            };
            if board.units[i].fallen || cost > board.units[i].tempo {
                return Dodge::Stand;
            }
            if combat::strike_target(&board.units, reaching, i).is_some() {
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
fn area_strike(board: &mut Board, attacker: usize, region: u8, tier: Post) -> Vec<Contact> {
    if board.units[attacker].fallen || board.units[attacker].tempo == 0 {
        return Vec::new();
    }
    board.units[attacker].tempo -= 1;
    let side = board.units[attacker].side;
    let depth = AREA_REACH.with(|r| *r.borrow());
    board
        .in_region(region)
        .into_iter()
        .filter(|&j| board.units[j].side != side)
        .filter(|&j| depth == AreaReach::WholeRegion || board.posts[j] == tier)
        .map(|j| Contact {
            attacker,
            target: j,
            bid: 0, // no bid: it cannot be evaded, and nobody answers along it
        })
        .collect()
}

/// **How deep an area strike reaches** - the open design question this model turned up, and the reason the raid
/// currently has nothing to be *for*.
///
/// It is a switch rather than a constant because the two answers are a **rule** fork, not a number, and the
/// difference is not one you can tune your way across:
///
/// - [`WholeRegion`](AreaReach::WholeRegion) - a sweep hits **both tiers**. Then an area strike is a *universal
///   solvent for the screen*: it reaches a body behind an intact front, for one card, at **any power level**.
///   Turning its Might down changes how *fast* the back line dies, never whether it is *reachable* - so the
///   front/back structure means nothing to anybody carrying one, and the raid's unique selling point ("the only
///   way past an intact line") is simply false.
/// - [`FrontLine`](AreaReach::FrontLine) - a sweep hits **the tier it was aimed at**. This is the old spec's rule
///   ("an attack may strike a whole *rank* at once"), and the more natural reading of what an area strike even
///   is: it covers a **line**, not a **depth**. The screen keeps meaning, AoE stays the anti-cluster counter
///   against a *wide* front, and the raid stays the only way to a screened body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaReach {
    /// Both tiers. An area strike ignores the screen entirely.
    WholeRegion,
    /// Only the tier it was aimed at. The screen still screens.
    FrontLine,
}

thread_local! {
    static AREA_REACH: std::cell::RefCell<AreaReach> = const { std::cell::RefCell::new(AreaReach::WholeRegion) };
}

/// Set the [`AreaReach`] rule for this thread. A probe knob, not a game setting - it exists so the two answers
/// can be *measured* against each other rather than argued about.
pub fn set_area_reach(reach: AreaReach) {
    AREA_REACH.with(|r| *r.borrow_mut() = reach);
}

/// Land a set of blows: each contact's opening blow, plus - if `pour` - everyone's leftover tempo poured along an
/// edge they can swing on. One order-free, commit-based batch: a blow lands even if its striker died to a
/// simultaneous one.
fn land(board: &mut Board, contacts: &[Contact], sweeps: &[Contact], pour: bool) {
    let blows: Vec<Blows> = (0..board.units.len())
        .filter(|&i| pour && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| {
            combat::strike_target(&board.units, contacts, i).map(|target| Blows {
                unit: i,
                target,
                cards: board.units[i].tempo,
            })
        })
        .collect();
    let all: Vec<Contact> = contacts.iter().chain(sweeps).copied().collect();
    combat::resolve_strike(&mut board.units, &all, &blows);
}

/// Close a sub-phase: finalize deaths, then **promote any back line whose front just collapsed**. That promotion
/// is how the ground behind a broken line opens up *within* a round.
fn close(board: &mut Board, before: &[bool]) -> SubPhaseLog {
    combat::end_sub_phase(&mut board.units);
    let was = board.posts.clone();
    board.promote();
    SubPhaseLog {
        fallen: (0..board.units.len())
            .filter(|&i| before[i] && board.units[i].fallen)
            .collect(),
        promoted: (0..board.units.len())
            .filter(|&i| was[i] == Post::Back && board.posts[i] == Post::Front)
            .collect(),
        health: board.units.iter().map(|u| u.health).collect(),
        posts: board.posts.clone(),
        ..Default::default()
    }
}

fn living(board: &Board) -> Vec<bool> {
    board.units.iter().map(|u| !u.fallen).collect()
}

/// Play **one whole round**: the Reset, then Slip -> Raid -> Clash.
pub fn play_round(board: &mut Board, acts: &[Act]) -> Vec<SubPhaseLog> {
    combat::refresh_round(&mut board.units);
    let mut logs = Vec::new();

    // ---- 1. SLIP -----------------------------------------------------------------------------------------
    //
    // A slip is opposed by **every enemy vanguard in the region you leave and the region you enter** - you are
    // outside your own screen the moment you move, so both ends reach for you. Raid / retreat / regroup are all
    // the same contest; only the destination differs.
    let before = living(board);
    let movers: Vec<(usize, u8, Answer)> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen)
        .filter_map(|i| {
            let d = acts[i].destination(board, i)?;
            Some((i, d, acts[i].answer()?))
        })
        .collect();

    let mut catches: Vec<Engage> = Vec::new();
    for &(i, dest, _) in &movers {
        let foe = if board.units[i].side == Side::Party {
            Side::Foe
        } else {
            Side::Party
        };
        for from in [board.regions[i], dest] {
            for f in board.vanguard(from, foe) {
                if board.units[f].tempo > 0 {
                    catches.push(Engage {
                        attacker: f,
                        target: i,
                        cards: reach_cards(&board.units, f, i),
                    });
                }
            }
        }
    }
    let reaching = engage(board, &catches);

    // Now the slipper answers, **seeing exactly what was committed**. Evade pays in full and breaks every edge;
    // Push and Abort spend nothing and eat the blows. The difference between them is only whether you go.
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| match movers.iter().find(|&&(m, _, _)| m == i) {
            Some(&(_, _, Answer::Evade)) => Dodge::Slip,
            _ => Dodge::Stand,
        })
        .collect();
    let landed = combat::resolve_evade(&mut board.units, &reaching, &dodges);

    // An Abort turns and fights: it spends its tempo swinging back at whoever caught it. Push and Evade keep
    // their pool for the raid.
    let aborting: Vec<usize> = movers
        .iter()
        .filter(|&&(_, _, a)| a == Answer::Abort)
        .map(|&(i, _, _)| i)
        .collect();
    let ripostes: Vec<Blows> = aborting
        .iter()
        .filter(|&&i| !board.units[i].fallen && board.units[i].tempo > 0 && board.units[i].melee)
        .filter_map(|&i| {
            combat::strike_target(&board.units, &landed, i).map(|target| Blows {
                unit: i,
                target,
                cards: board.units[i].tempo,
            })
        })
        .collect();
    combat::resolve_strike(&mut board.units, &landed, &ripostes);

    let mut through = vec![false; board.units.len()];
    let mut log = close(board, &before);
    for &(i, dest, answer) in &movers {
        if board.units[i].fallen {
            continue;
        }
        if answer == Answer::Abort {
            log.aborted.push(i);
            continue; // it chose the fight over the ground
        }
        // Evade paid for it; Push bled for it. Either way it is through.
        board.regions[i] = dest;
        board.posts[i] = Post::Front; // it charged in - it is at the sharp end by definition
        through[i] = true;
        log.through.push(i);
    }
    board.promote(); // leaving a region can strand a back line with no front
    logs.push(log);

    // ---- 2. RAID -----------------------------------------------------------------------------------------
    //
    // The early slot, and the whole of what the slip bought: the raider hits the cannon *before* the cannon
    // fires. Tempo-gated - a raider that evaded with everything arrives with nothing to swing.
    let before = living(board);
    let raids: Vec<(usize, usize)> = (0..board.units.len())
        .filter(|&i| through[i] && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| match acts[i] {
            Act::Raid(t, _) if !board.units[t].fallen => Some((i, t)),
            _ => None,
        })
        .collect();
    exchange(board, &raids, false);
    logs.push(close(board, &before));

    // ---- 3. CLASH ----------------------------------------------------------------------------------------
    let before = living(board);
    let clashes: Vec<(usize, usize)> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| match acts[i] {
            Act::Clash(t) if !board.units[t].fallen => Some((i, t)),
            _ => None,
        })
        .collect();
    exchange(board, &clashes, true);
    logs.push(close(board, &before));

    logs
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged.** Area strikes split off and
/// sweep their target's whole region.
fn exchange(board: &mut Board, attacks: &[(usize, usize)], pour: bool) {
    let mut sweeps: Vec<Contact> = Vec::new();
    let mut aimed: Vec<Engage> = Vec::new();
    for &(a, t) in attacks {
        if board.units[a].aoe {
            // The sweep covers the tier it was aimed at (or both, under `AreaReach::WholeRegion`).
            let (region, tier) = (board.regions[t], board.posts[t]);
            sweeps.extend(area_strike(board, a, region, tier));
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
    let contacts = combat::resolve_evade(&mut board.units, &reaching, &dodges);
    land(board, &contacts, &sweeps, pour);
}

// ---- the doom oracle -----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Winnable,
    /// The search ran out of budget. **Not an answer** - an answer in progress.
    Evaluating,
    Doomed,
}

/// The memo key: per-unit `(health, fallen, post)`, the **canonicalized** partition, and the round.
///
/// Tempo and the damage pile are absent on purpose - both are re-derived by the round Reset, and we only memoize
/// at a **round** boundary. The product's own rule (the round is the one deadline) paying a dividend.
type Key = (Vec<(u32, bool, Post)>, Vec<u8>, usize);

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
                .map(|i| (board.units[i].health, board.units[i].fallen, board.posts[i]))
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
    use deckbound_content::rank::Intention as Rank;

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, Rank::Vanguard, stats, 0, melee, ranged)
    }

    /// A wall in front, a cannon behind it, one enemy - the smallest board that has a formation at all.
    ///
    /// The Ogre is deliberately **rich** (Cadence 5): it can afford to evade the wall *and* still arrive with
    /// enough tempo to price the cannon out of its own evade. A poorer raider cannot, and that is the whole
    /// point of a front line - see [`the_front_drains_a_raider_that_evades_it`].
    ///
    /// The Bastion's Might (3) exceeds the Ogre's Grit (2), so it can actually *bleed* a body that pushes past
    /// it. A wall that cannot hurt what it catches teaches you nothing.
    fn wall_and_cannon() -> Board {
        Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 3, 3, 1, 2], true, false), // 0 - the wall
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true), // 1 - the cannon
                unit("Ogre", Side::Foe, [5, 5, 2, 5, 2], true, false),      // 2 - the raider
            ],
            vec![0, 0, 1],
            vec![Post::Front, Post::Back, Post::Front],
        )
    }

    #[test]
    fn region_labels_are_not_state() {
        assert_eq!(canonical(&[3, 3, 7, 9]), canonical(&[1, 1, 0, 5]));
        assert_ne!(canonical(&[0, 0, 1]), canonical(&[0, 1, 1]));
    }

    /// **NO REARGUARD WITHOUT A VANGUARD.** The front collapses, the back is promoted on the spot - which makes
    /// *"at the back"* and *"screened"* the same fact, and is how the ground behind a broken line opens up.
    #[test]
    fn a_back_line_with_no_front_is_promoted_not_protected() {
        let mut b = wall_and_cannon();
        assert_eq!(b.posts[1], Post::Back, "the cannon starts behind the wall");

        b.units[0].fallen = true;
        b.promote();
        assert_eq!(
            b.posts[1],
            Post::Front,
            "with no wall left, the cannon IS the front - there is no hiding behind nothing"
        );
        assert_eq!(b.vanguard(0, Side::Party), vec![1]);
    }

    /// **A rearguard is reachable only by a raid** - and that is not a special case, it falls out of the menu.
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
            "it may NOT clash the cannon - the wall is in the way"
        );
        assert!(
            acts.iter().any(|a| matches!(a, Act::Raid(1, _))),
            "to reach the cannon it must slip the wall - the Outrider, as a consequence and not a role"
        );
    }

    /// **The slipper always has all three answers.** A vanguard can never simply *stop* you - it can only make
    /// you pay in tempo, in blood, or in the chance you gave up to turn and fight it.
    #[test]
    fn a_slipper_always_has_all_three_answers() {
        let b = wall_and_cannon();
        let acts = legal_acts(&b, 2);
        for answer in ANSWERS {
            assert!(
                acts.contains(&Act::Raid(1, answer)),
                "{answer:?} must always be on the menu"
            );
        }
    }

    /// An **archer cannot raid**: slipping a shield wall to knife the mage is not done with a bow.
    #[test]
    fn an_archer_cannot_raid() {
        let b = Board::new(
            vec![
                unit("Archer", Side::Party, [3, 3, 1, 2, 2], false, true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
            vec![Post::Front, Post::Front, Post::Back],
        );
        let acts = legal_acts(&b, 0);
        assert!(
            acts.contains(&Act::Clash(1)),
            "it shoots their front freely"
        );
        assert!(
            !acts.iter().any(|a| matches!(a, Act::Raid(..))),
            "but it cannot slip a line to reach the mage"
        );
        assert!(
            !acts.contains(&Act::Clash(2)),
            "and it cannot shoot through the wall"
        );
    }

    /// **A melee body at the back is dead weight** - it cannot attack at all. Nothing bans it; posting a Raider
    /// behind the line simply punishes itself. Force, not fiat.
    #[test]
    fn a_melee_body_at_the_back_is_dead_weight() {
        let b = Board::new(
            vec![
                unit("Bastion", Side::Party, [1, 3, 3, 1, 2], true, false),
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false), // hiding, uselessly
                unit("Ogre", Side::Foe, [5, 5, 2, 2, 2], true, false),
            ],
            vec![0, 0, 1],
            vec![Post::Front, Post::Back, Post::Front],
        );
        let acts = legal_acts(&b, 1);
        assert!(
            !acts
                .iter()
                .any(|a| matches!(a, Act::Clash(_) | Act::Raid(..))),
            "it can do nothing but hold or slip out"
        );
        assert!(acts.contains(&Act::Hold));
        assert!(acts.iter().any(|a| matches!(a, Act::Slip(..))));
    }

    /// **THE ONE THAT MATTERS. The screen is a PRICE, not an immunity** - enough Tempo always gets past a front.
    /// A previous cut redirected every blow onto the screen, making a screened body untouchable until its guard
    /// died. That is fiat, and it silently deleted the whole Outrider.
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
            "and it is now standing INSIDE their formation - thems the consequences"
        );
        assert_eq!(b.posts[2], Post::Front, "at the sharp end, by definition");
    }

    /// **PUSH: take the hit and go anyway.** You spend nothing on the line, so you arrive hurt but with your whole
    /// pool in hand. A vanguard cannot **stop** you; it can only bleed you.
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
            "and it paid in blood, not in tempo"
        );
    }

    /// **The front DRAINS a raider that evades it** - and that is the vanguard's whole job.
    ///
    /// A poor raider can still buy its way past the line, but it arrives with nothing left to price the cannon
    /// out of its *own* evade - so the blow whiffs. This is the old Intercept, falling out of the tempo economy
    /// with no interception rule anywhere: *screen and drain, so it reaches the back empty.*
    #[test]
    fn the_front_drains_a_raider_that_evades_it() {
        let mut b = Board::new(
            vec![
                unit("Bastion", Side::Party, [3, 3, 3, 1, 2], true, false),
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
                unit("Runt", Side::Foe, [5, 5, 2, 2, 2], true, false), // Cadence 2: it can pay, but only just
            ],
            vec![0, 0, 1],
            vec![Post::Front, Post::Back, Post::Front],
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
            "but it spent its pool getting there, so the cannon simply evaded the blow"
        );
    }

    /// **ABORT: turn and fight.** You give up the ground and swing at whoever caught you. This is the "repelled"
    /// outcome - but *chosen*, not imposed.
    #[test]
    fn aborting_keeps_you_where_you_are_and_swings_back() {
        let mut b = wall_and_cannon();
        let cannon = b.units[1].health;
        let logs = play_round(
            &mut b,
            &[Act::Clash(2), Act::Clash(2), Act::Raid(1, Answer::Abort)],
        );
        assert!(logs[0].aborted.contains(&2), "it turned back at the line");
        assert_eq!(b.regions[2], 1, "it never left its own ground");
        assert_eq!(b.units[1].health, cannon, "and it never reached the cannon");
    }

    /// **An area strike nukes the whole region, both tiers** - bypassing the screen entirely. The anti-cluster
    /// counter: pile bodies behind a vanguard and you become a target.
    #[test]
    fn an_area_strike_reaches_the_back_line() {
        let mut b = Board::new(
            vec![
                unit("Bombardier", Side::Party, [3, 3, 1, 1, 2], false, true).with_aoe(true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
            vec![Post::Front, Post::Front, Post::Back],
        );
        let mage = b.units[2].health;
        play_round(&mut b, &[Act::Clash(1), Act::Clash(0), Act::Clash(0)]);
        assert!(
            b.units[2].health < mage || b.units[2].fallen,
            "a sweep cannot be screened - a bodyguard soaks an aimed blow but cannot cover an area"
        );
    }

    /// Slipping is also how you **get out**: retreat and regroup are the same mechanic, at the same price.
    #[test]
    fn retreat_is_the_same_mechanic_as_the_raid() {
        let b = wall_and_cannon();
        assert!(
            legal_acts(&b, 1).iter().any(|a| matches!(a, Act::Slip(..))),
            "the cannon can always try to slip away - same contest, different destination"
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
            vec![Post::Front, Post::Front],
        );
        for _ in 0..MAX_ROUNDS {
            if b.outcome().is_some() {
                break;
            }
            play_round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
        }
        assert_eq!(b.outcome(), Some(true), "the Raider wins");
    }

    // ---- the oracle ------------------------------------------------------------------------------------

    fn deep_board() -> Board {
        Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
                unit("The Wall", Side::Foe, [1, 4, 9, 1, 2], true, false),
            ],
            vec![0, 0, 1],
            vec![Post::Front, Post::Back, Post::Front],
        )
    }

    fn hard_board() -> Board {
        Board::new(
            vec![
                unit("Marksman", Side::Party, [1, 1, 1, 1, 1], false, true),
                unit("The Wall", Side::Foe, [9, 9, 9, 3, 3], true, false),
            ],
            vec![0, 1],
            vec![Post::Front, Post::Front],
        )
    }

    /// **SAFETY: a starved oracle is SILENT, never WRONG - at any grant, however cruel.** It may say `Evaluating`
    /// forever; it must never answer `Doomed` or `Winnable` and disagree with the unbounded search.
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

    /// **LIVENESS: an escalating grind converges.** A subtree memoizes only once fully explored, so a grant too
    /// small to settle any new subtree makes no progress however often repeated. Double on every `Evaluating`.
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
