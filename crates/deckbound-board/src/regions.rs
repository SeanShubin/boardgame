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

    /// **Is `target` screened?** - it is posted at the back, *and* its side still has a living front in its
    /// region to do the screening.
    ///
    /// A back with **no** front left is **not promoted** - it stays a back, keeps its phase slot, and simply
    /// becomes reachable. That is the collapsed-vanguard rule: a screen is a body, and when the body is gone the
    /// screening stops, but the *intent* does not change. The cannon is still a cannon; it is just out in the
    /// open now.
    pub fn is_screened(&self, target: usize) -> bool {
        self.posts[target] == Post::Back
            && !self
                .vanguard(self.regions[target], self.units[target].side)
                .is_empty()
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
            if board.is_screened(t) {
                // Behind a living front. Only a melee body can slip in for it.
                if u.melee {
                    out.extend(ANSWERS.map(|a| Act::Raid(t, a)));
                }
            } else {
                // At the front, or a back whose front has collapsed - either way, reachable by anyone from
                // anywhere. A region is a formation, not a place.
                out.push(Act::Clash(t));
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
            let acts = legal_acts(board, i);
            // **Hunt the weakest body on the board, wherever it is standing.** If it is behind their line, go
            // through the line for it - that is what the raid is *for*.
            //
            // The previous script was `clash.or(raid)`: it preferred a clash and only raided when no clash
            // existed at all. Since a side always has a front (no rearguard without a vanguard), a clash always
            // existed - so **the foes never once raided**. The party's cannons were untouchable forever, not by
            // design but by omission, and the party won every attrition race by default. That single `or` was
            // holding the whole balance question shut.
            let softest = |t: usize| (board.units[t].health, board.units[t].grit);
            acts.iter()
                .filter_map(|a| match a {
                    // Push, not Evade: a scripted body spends its pool on the kill, not on the crossing.
                    Act::Clash(t) | Act::Raid(t, Answer::Push) => Some((softest(*t), *a)),
                    _ => None,
                })
                .min_by_key(|&(k, _)| k)
                .map(|(_, a)| a)
                .or(Some(Act::Hold))
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
    let might = board.units[attacker].might;

    let caught: Vec<usize> = board
        .in_region(region)
        .into_iter()
        .filter(|&j| board.units[j].side != side)
        .filter(|&j| depth == AreaReach::WholeRegion || board.posts[j] == tier)
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
    combat::end_sub_phase(&mut board.units);
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
    let landed = combat::resolve_evade(&mut board.units, &reaching, &dodges);

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

/// Play **one whole round**: the Reset, then Intercept -> Volley -> Raid -> Clash.
pub fn play_round(board: &mut Board, acts: &[Act]) -> Vec<SubPhaseLog> {
    combat::refresh_round(&mut board.units);
    let mut logs = Vec::new();

    let movers: Vec<(usize, u8, Answer)> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen)
        .filter_map(|i| {
            let d = acts[i].destination(board, i)?;
            Some((i, d, acts[i].answer()?))
        })
        .collect();

    // Every enemy body that gets a reach at a slipper, in the region it LEAVES and the region it ENTERS - you
    // are outside your own screen the moment you move, so both ends reach for you.
    let mut front_catchers: Vec<(usize, usize)> = Vec::new();
    let mut back_catchers: Vec<(usize, usize)> = Vec::new();
    for &(i, dest, _) in &movers {
        let foe = if board.units[i].side == Side::Party {
            Side::Foe
        } else {
            Side::Party
        };
        for from in [board.regions[i], dest] {
            for f in holding_line(board, acts, from, foe) {
                front_catchers.push((f, i));
            }
            for c in back_line(board, acts, from, foe) {
                back_catchers.push((c, i));
            }
        }
    }

    // ---- 1. INTERCEPT: the front line reaches for the slippers ------------------------------------------
    let before = living(board);
    let caught_by_front = reach_for_slippers(board, &front_catchers, &movers);
    logs.push(close(board, &before));

    // ---- 2. VOLLEY: the back line fires on whoever is still coming --------------------------------------
    //
    // The cannons defend themselves. A slipper the front already killed is not shot at twice - which is exactly
    // why this is its own sub-phase rather than one pile with the Intercept: the saved card is real.
    let before = living(board);
    let caught_by_back = reach_for_slippers(board, &back_catchers, &movers);
    logs.push(close(board, &before));

    // Arrival is settled now: a slipper that Evaded broke every edge and is through; one that Pushed bled for it
    // and is through anyway; one that Aborted turned and fought, and never left.
    let caught: Vec<&Contact> = caught_by_front.iter().chain(&caught_by_back).collect();
    let mut through = vec![false; board.units.len()];
    let mut log = SubPhaseLog {
        health: board.units.iter().map(|u| u.health).collect(),
        posts: board.posts.clone(),
        ..Default::default()
    };
    for &(i, dest, answer) in &movers {
        if board.units[i].fallen {
            continue;
        }
        if answer == Answer::Abort {
            log.aborted.push(i);
            continue; // it chose the fight over the ground
        }
        let _ = &caught; // (a Push takes the blows and goes anyway; an Evade broke them)
        board.regions[i] = dest;
        board.posts[i] = Post::Front; // it charged in - it is at the sharp end by definition
        through[i] = true;
        log.through.push(i);
    }
    if let Some(last) = logs.last_mut() {
        last.through = log.through.clone();
        last.aborted = log.aborted.clone();
    }

    // ---- 3. RAID: those who got through strike the backs they came for ----------------------------------
    //
    // The early slot, and the whole of what the slip bought: the raider hits the cannon *before* the cannon
    // fires in the Clash. Tempo-gated - a raider that evaded with everything arrives with nothing to swing.
    let before = living(board);
    let raids: Vec<Attack> = (0..board.units.len())
        .filter(|&i| through[i] && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| match acts[i] {
            Act::Raid(t, _) if !board.units[t].fallen => Some((i, t)),
            _ => None,
        })
        .collect();
    exchange(board, &raids, false);
    logs.push(close(board, &before));

    // ---- 4. FIRE, then 5. CLASH -------------------------------------------------------------------------
    //
    // **The back lines fire first, and the front lines close last.** That is not a schedule quirk - it is what
    // the two posts MEAN. `Front` is "seek melee", and closing is the slowest thing you can do; `Back` is "avoid
    // melee", and a body that never closed gets its shot away first.
    //
    // A death in Fire silences that body's Clash, which is the whole value of a cannon - and the whole reason a
    // cannon is worth screening. It is also what lets a lone archer beat a swordsman it could never out-trade:
    // it is targetable, it just gets to shoot first.
    for (phase, tier) in [(SubPhase::Fire, Post::Back), (SubPhase::Clash, Post::Front)] {
        let _ = phase;
        let before = living(board);
        let attacks: Vec<Attack> = (0..board.units.len())
            .filter(|&i| {
                !board.units[i].fallen && board.units[i].tempo > 0 && board.posts[i] == tier
            })
            .filter_map(|i| match acts[i] {
                Act::Clash(t) if !board.units[t].fallen => Some((i, t)),
                _ => None,
            })
            .collect();
        exchange(board, &attacks, true);
        logs.push(close(board, &before));
    }

    logs
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged.** Area strikes split off and
/// sweep their target's whole region.
fn exchange(board: &mut Board, attacks: &[Attack], pour: bool) {
    let mut sweeps: Vec<Contact> = Vec::new();
    let mut aimed: Vec<Engage> = Vec::new();
    for &(a, t) in attacks {
        if board.units[a].aoe {
            // The sweep covers the tier it was aimed at: width, never depth.
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

    /// **THE COLLAPSED VANGUARD.** A back whose front has fallen is **not promoted**. It stays a back: it becomes
    /// *targetable* - anyone may clash it now, no raid required (force, not fiat) - but it **keeps its phase
    /// slot**, so it still fires *before* the front line swings.
    ///
    /// This is the hinge the whole design turns on. Promotion destroyed it: it turned every unscreened cannon
    /// into just another front-line body, which left `ranged` gating **nothing a solo fight could see** - and so
    /// a lone archer was simply a worse swordsman, and no stat tuning could ever have fixed that.
    ///
    /// Front and back are a statement of **intent** - *seek melee* or *avoid it* - and losing your screen does
    /// not change your intent. The cannon is still a cannon. It is just out in the open now.
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
            "the cannon is STILL a cannon - it is not promoted into the line"
        );
        assert!(!b.is_screened(1), "but nothing is screening it any more");
        assert!(
            legal_acts(&b, 2).contains(&Act::Clash(1)),
            "so it is targetable now - force, not fiat"
        );
    }

    /// **AND THAT IS WHAT MAKES RANGE MEAN SOMETHING.** A lone archer is perfectly targetable - and **still gets
    /// the first hit**, because holding off *is* being quicker.
    ///
    /// The Raider out-trades the Duelist and survives on 1. The Marksman could never out-trade it - Might 5 into
    /// Might 5, Vitality 2 into Vitality 5 - and wins anyway, because it shoots in the **Fire** sub-phase and the
    /// Duelist swings in the **Clash**. *Close in and you trade and die; answer it from range.*
    #[test]
    fn a_lone_archer_at_the_back_shoots_before_a_swordsman_can_swing() {
        let duel = |hero: [u8; 5], melee: bool, ranged: bool, post: Post| -> Option<bool> {
            let mut b = Board::new(
                vec![
                    unit("Hero", Side::Party, hero, melee, ranged),
                    unit("Duelist", Side::Foe, [5, 5, 1, 2, 2], true, false),
                ],
                vec![0, 1],
                vec![post, Post::Front],
            );
            let mut rounds = 0;
            while b.outcome().is_none() && rounds < MAX_ROUNDS {
                play_round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
                rounds += 1;
            }
            b.outcome()
        };

        assert_eq!(
            duel([5, 2, 1, 2, 2], false, true, Post::Back),
            Some(true),
            "the Marksman posts to the back, is fully targetable, and kills it before it can swing"
        );
        assert_eq!(
            duel([5, 2, 1, 2, 2], false, true, Post::Front),
            Some(false),
            "...and the SAME body, posted to the front, closes and dies. The post is the whole difference."
        );
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
        assert!(
            logs.iter().any(|l| l.aborted.contains(&2)),
            "it turned back at the line"
        );
        assert_eq!(b.regions[2], 1, "it never left its own ground");
        assert_eq!(b.units[1].health, cannon, "and it never reached the cannon");
    }

    /// **AN AREA STRIKE MULTIPLIES TARGETS, IT DOES NOT EXTEND REACH.** A body you could not single-target,
    /// you cannot sweep - so a screened back line is safe from a sweep lobbed at their front.
    ///
    /// Reach is what the screen governs; width is what an area strike governs. A sweep that reached through the
    /// line was buying both, and that is a category error rather than a balance number - which is why tuning it
    /// (full Might, half, floor) never once changed whether a raid was necessary.
    #[test]
    fn a_sweep_cannot_reach_a_body_a_single_strike_could_not() {
        let mut b = Board::new(
            vec![
                unit("Bombardier", Side::Party, [3, 3, 1, 1, 2], false, true).with_aoe(true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
            vec![Post::Front, Post::Front, Post::Back],
        );
        let (wall, mage) = (b.units[1].health, b.units[2].health);
        // The Mage HOLDS: this test is about where a sweep reaches, not about who shoots first. (Leave it
        // firing and it kills the Bombardier in the Fire sub-phase before the sweep ever lands - which is the
        // collapsed-vanguard rule working, but it is not what we are measuring here.)
        play_round(&mut b, &[Act::Clash(1), Act::Clash(0), Act::Hold]);
        assert!(b.units[1].health < wall, "it sweeps their front line");
        assert_eq!(
            b.units[2].health, mage,
            "and it must NOT touch the body behind that line - the sweep has no reach it did not pay for"
        );
    }

    /// ...but width still comes free once the reach IS paid for. A **raider** carrying an area strike sweeps the
    /// whole back line, because it is standing in it. The Bastion has to go *in* after the Swarm it answers.
    #[test]
    fn a_raider_with_an_area_strike_sweeps_the_back_line_it_slipped_into() {
        let mut b = Board::new(
            vec![
                // A melee area striker with the tempo to buy its way past the wall.
                unit("Bastion", Side::Party, [3, 4, 3, 5, 2], true, false).with_aoe(true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [4, 2, 1, 2, 2], false, true),
                unit("Seer", Side::Foe, [4, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1, 1],
            vec![Post::Front, Post::Front, Post::Back, Post::Back],
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
            "and swept the WHOLE back line, not just the body it came for - width is free once you have paid              for the reach"
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

    /// **The scripted foe must actually USE the raid.** This is the bug that hid the whole balance question for
    /// three commits, so it gets a test rather than a comment.
    ///
    /// The old script was `clash.or(raid)` - prefer a clash, raid only if no clash exists. But a side *always*
    /// has a front (no rearguard without a vanguard), so a clash always existed, so **the foes never once
    /// raided**. The party's cannons were untouchable forever - not by design, but by omission - and the party
    /// won every attrition race by default. The probe then reported, quite correctly, that raiding was never
    /// *necessary*: it was measuring a world in which the opponent had unilaterally disarmed.
    ///
    /// An environment that refuses to use a mechanic cannot tell you whether that mechanic matters.
    #[test]
    fn the_foes_raid_a_screened_body_when_it_is_the_softest_thing_on_the_board() {
        let b = Board::new(
            vec![
                // A tough front the foe could clash, screening a cannon it would much rather kill.
                unit("Bastion", Side::Party, [1, 6, 4, 1, 2], true, false),
                unit("Marksman", Side::Party, [5, 1, 1, 2, 2], false, true),
                unit("Ogre", Side::Foe, [5, 5, 2, 3, 2], true, false),
            ],
            vec![0, 0, 1],
            vec![Post::Front, Post::Back, Post::Front],
        );
        let acts = foe_acts(&b);
        assert_eq!(
            acts[2],
            Some(Act::Raid(1, Answer::Push)),
            "it must go through the line for the soft body behind it, not settle for the wall in front"
        );
    }

    // ---- resolver invariants ---------------------------------------------------------------------------
    //
    // Everything above tests a RULE. These test the RESOLVER: the properties that must hold whatever the rules
    // say, and that a bug would quietly break while every rule test still passed. Tuning on top of a broken
    // resolver is worse than not tuning, so these come first.

    /// A messy board with every mechanic on it at once - two regions, both tiers, melee and ranged, an area
    /// striker, a horde, and a raid in flight. If an invariant breaks anywhere, it breaks here.
    fn messy() -> (Board, Vec<Act>) {
        let b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false), // 0
                unit("Bastion", Side::Party, [1, 3, 3, 1, 2], true, false).with_aoe(true), // 1
                unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true), // 2
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),     // 3
                unit("Duelist", Side::Foe, [5, 5, 1, 2, 2], true, false),  // 4
                unit("Swarm", Side::Foe, [2, 6, 1, 2, 2], false, true).as_horde(true), // 5
            ],
            vec![0, 0, 0, 1, 1, 1],
            vec![
                Post::Front,
                Post::Front,
                Post::Back,
                Post::Front,
                Post::Front,
                Post::Back,
            ],
        );
        let acts = vec![
            Act::Raid(5, Answer::Evade), // the Raider slips in for their back line
            Act::Clash(3),
            Act::Clash(4),
            Act::Clash(0),
            Act::Raid(2, Answer::Push), // the Duelist pushes through for our cannon
            Act::Clash(1),
        ];
        (b, acts)
    }

    /// **ORDER-INDEPENDENCE - the property test the whole design leans on** (Spec 1.9).
    ///
    /// Permuting the seat order of the units must yield the **identical** end-state. Nothing may depend on who
    /// happens to be at index 0: not who lands a blow first, not who catches a slipper, not who dies.
    ///
    /// This is the invariant that makes a "commit-based, order-free batch" mean anything, and it is the one a
    /// resolver bug is most likely to break *quietly* - every rule test still passes while the outcome silently
    /// depends on iteration order. If it ever fails, some effect is reading state another effect already wrote,
    /// and the fix is to move it to its own sub-phase (or its own pile), never to re-sort the units.
    #[test]
    fn resolution_does_not_depend_on_seat_order() {
        let (b, acts) = messy();
        let n = b.units.len();

        // The reference run, in the natural order.
        let mut base = b.clone();
        play_round(&mut base, &acts);

        // Every rotation is a permutation; that is enough to catch an order dependence without a shuffler.
        for shift in 1..n {
            let perm: Vec<usize> = (0..n).map(|i| (i + shift) % n).collect(); // new seat -> old index
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
                other => other,
            };

            let mut shuffled = Board::new(
                perm.iter().map(|&o| b.units[o].clone()).collect(),
                perm.iter().map(|&o| b.regions[o]).collect(),
                perm.iter().map(|&o| b.posts[o]).collect(),
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
                    ),
                    (
                        shuffled.units[new].health,
                        shuffled.units[new].fallen,
                        shuffled.regions[new],
                        shuffled.posts[new],
                    ),
                    "{} came out differently when the units were re-seated (shift {shift}) - something in \
                     the resolver depends on iteration order",
                    base.units[old].name
                );
            }
        }
    }

    /// **The nastiest order-dependence case: one body, two attackers.**
    ///
    /// A body closed on from two sides has to pick which one to swing back at, and `land` picks it with
    /// `contacts.iter().find(..)` - **the first contact in the list**, which is iteration order. That is exactly
    /// the shape of bug the seat-order property exists to catch, so it gets its own test with the case forced,
    /// rather than relying on `messy()` happening to contain it.
    ///
    /// If this fails, the fix is *not* to sort the contacts - it is that the choice of whom to answer is a real
    /// decision the model is currently making silently, and it should be a declared one.
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
                vec![Post::Front, Post::Front, Post::Front],
            )
        };
        // The lone hero (seat 0) is clashed by BOTH foes and has tempo left to answer with.
        let mut a = build([("Hero", Side::Party), ("X", Side::Foe), ("Y", Side::Foe)]);
        play_round(&mut a, &[Act::Hold, Act::Clash(0), Act::Clash(0)]);

        // Same fight, the two foes seated the other way round.
        let mut b = build([("Hero", Side::Party), ("Y", Side::Foe), ("X", Side::Foe)]);
        play_round(&mut b, &[Act::Hold, Act::Clash(0), Act::Clash(0)]);

        // X is seat 1 in `a` and seat 2 in `b`; Y the reverse. Total damage dealt to the foes must match, and so
        // must its distribution - the hero cannot hit a *different body* just because the seats moved.
        assert_eq!(
            (a.units[1].health, a.units[2].health),
            (b.units[2].health, b.units[1].health),
            "the hero answered a different attacker purely because the foes swapped seats"
        );
    }

    /// **Determinism.** The same position and the same declarations must produce the same board, every time.
    /// There is no RNG in this model, and if this ever fails there is a hidden one.
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
                x.posts.clone(),
            )
        };
        assert_eq!(run(), run());
    }

    /// **Nothing goes out of bounds.** Tempo never underflows, health never exceeds what a body started with,
    /// and a fallen body is at zero. A resolver bug shows up here first.
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
                    "{} is fallen iff it is out of Health",
                    u.name
                );
            }
        }
    }

    /// **A committed blow lands even if its striker dies.** The order-free, commit-based batch: you swing, and
    /// the swing counts, even when the answering blow kills you in the same sub-phase. Mutual death resolves
    /// cleanly rather than depending on who was evaluated first.
    #[test]
    fn a_committed_blow_lands_even_if_its_striker_dies() {
        let mut b = Board::new(
            vec![
                // Two glass cannons that each kill the other outright.
                unit("A", Side::Party, [9, 1, 1, 1, 1], true, false),
                unit("B", Side::Foe, [9, 1, 1, 1, 1], true, false),
            ],
            vec![0, 1],
            vec![Post::Front, Post::Front],
        );
        play_round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
        assert!(b.units[0].fallen && b.units[1].fallen, "both blows landed");
        assert_eq!(
            b.outcome(),
            Some(false),
            "a mutual wipe is not a party win - and it is not order-dependent either"
        );
    }

    /// **A fallen body does nothing.** It cannot clash, raid, slip, catch a slipper, or be offered any act at
    /// all. Death silences, and it silences completely.
    #[test]
    fn a_fallen_body_takes_no_part() {
        let mut b = messy().0;
        b.units[3].fallen = true; // the Wall dies
        b.units[3].health = 0;
        assert!(legal_acts(&b, 3).is_empty(), "a corpse declares nothing");
        assert!(
            !b.vanguard(1, Side::Foe).contains(&3),
            "and it screens nothing - a dead line catches no slipper"
        );
        assert!(
            foe_acts(&b)[3].is_none(),
            "and the script does not order it about"
        );
    }

    /// **The damage pile closes at the Round Reset, and only there.** Sub-threshold damage - a hit that never
    /// turned a Health card - must not carry into the next round. Otherwise chip damage accumulates forever and
    /// Grit stops being a wall at all.
    #[test]
    fn sub_threshold_damage_does_not_carry_between_rounds() {
        // Might 1 into Grit 5: it can never flip a card, however many rounds it is given.
        let mut b = Board::new(
            vec![
                unit("Gnat", Side::Party, [1, 9, 1, 1, 9], true, false),
                unit("Boulder", Side::Foe, [0, 3, 5, 1, 1], true, false),
            ],
            vec![0, 1],
            vec![Post::Front, Post::Front],
        );
        let boulder = b.units[1].health;
        for _ in 0..MAX_ROUNDS {
            play_round(&mut b, &[Act::Clash(1), Act::Hold]);
        }
        assert_eq!(
            b.units[1].health, boulder,
            "five rounds of chip under the bar must add up to exactly nothing - the pile closes each Reset"
        );
    }

    /// **Every fight terminates.** Whatever the declarations, the round cap ends it - there is no line that
    /// stalls the resolver.
    #[test]
    fn every_fight_terminates() {
        let (mut b, acts) = messy();
        for _ in 0..MAX_ROUNDS {
            if b.outcome().is_some() {
                return;
            }
            play_round(&mut b, &acts);
        }
        // A draw at the cap is a legitimate end - what matters is that we got here at all.
        assert!(b.outcome().is_some() || b.alive(Side::Party) && b.alive(Side::Foe));
    }

    /// **YOU FIGHT WHO YOU DECLARED.** A body that declared `Hold` does not swing back, however hard it is hit.
    ///
    /// This is a deliberate departure from the shipped rank model, where a melee contact is *mutual* and a body
    /// answers even when the schedule never paired it against you ("it did not choose the fight"). That rule
    /// makes sense there, because a unit's target is chosen for it by the schedule - it never *had* a say.
    ///
    /// Here it had one. Declarations are simultaneous, so a body that wanted to trade could have declared
    /// `Clash` back and traded. `Hold` is a real choice, and taking a free hit is what it costs.
    ///
    /// And it is what buys **order-independence**: the moment a body answers something it did not name, the
    /// resolver has to pick *which* attacker - and it was picking the first one in the contact list, so who died
    /// depended on who sat at index 0. See
    /// [`a_body_closed_on_from_two_sides_answers_the_same_way_whatever_the_seating`]. There is no way to have
    /// both an undeclared riposte and a seat-independent resolver without inventing a tie-break, and a tie-break
    /// is just an order dependence you have written down.
    #[test]
    fn a_body_fights_only_what_it_declared() {
        let mut b = Board::new(
            vec![
                unit("Attacker", Side::Party, [3, 5, 1, 1, 9], true, false),
                unit("Defender", Side::Foe, [4, 5, 1, 3, 1], true, false),
            ],
            vec![0, 1],
            vec![Post::Front, Post::Front],
        );
        let (attacker, defender) = (b.units[0].health, b.units[1].health);

        play_round(&mut b, &[Act::Clash(1), Act::Hold]); // the defender declares NOTHING
        assert!(b.units[1].health < defender, "the blow lands");
        assert_eq!(
            b.units[0].health, attacker,
            "and Hold means hold - it takes the hit for free, because it never declared a fight"
        );

        // Declare the trade, and the trade happens.
        let mut c = Board::new(
            vec![
                unit("Attacker", Side::Party, [3, 5, 1, 1, 9], true, false),
                unit("Defender", Side::Foe, [4, 5, 1, 3, 1], true, false),
            ],
            vec![0, 1],
            vec![Post::Front, Post::Front],
        );
        play_round(&mut c, &[Act::Clash(1), Act::Clash(0)]);
        assert!(
            c.units[0].health < attacker && c.units[1].health < defender,
            "both declared the fight, so both pay for it"
        );
    }

    /// **A SWEEP CLEARS A PACK.** The second axis of an area strike, and it is independent of the first.
    ///
    /// - *Which bodies in the region does it touch?* -> the ones you could have single-targeted: the tier you can
    ///   reach ([`AreaReach`]). Width, never depth.
    /// - *How much of a body does it touch?* -> **all of it.** A horde is many bodies, and there is nowhere in a
    ///   pack to not be.
    ///
    /// It was inverted. A horde is one [`Combatant`] whose Health is its body count, so an aimed blow deals
    /// `Might` straight off that count - felling **Might-many bodies** - and gets to *pour*, landing several. A
    /// sweep landed on the same one body for `Might` **once**, and could not pour. So a sweep was **strictly
    /// worse** against a pack than an aimed blow, which made the two anti-horde kits the worst horde-killers in
    /// the game:
    ///
    /// ```text
    /// BEFORE, on a 12-body pack, in one round:
    ///   Raider     (Might 7, single) -> 12 of 12     <- wiped it outright
    ///   Bastion    (Might 1, AREA)   ->  1 of 12     <- the Swarm's designated counter
    ///   Bombardier (Might 3, AREA)   ->  3 of 12     <- the Storm's designated counter
    /// ```
    ///
    /// Two of the four balance locks are horde locks, so two of the four were inverted - and no stat tuning
    /// could have flipped them back.
    #[test]
    fn a_sweep_clears_a_pack_and_an_aimed_blow_does_not() {
        let felled = |stats: [u8; 5], aoe: bool, pack: u32| -> u32 {
            let mut b = Board::new(
                vec![
                    unit("Hero", Side::Party, stats, true, true).with_aoe(aoe),
                    unit("Horde", Side::Foe, [1, pack as u8, 1, 1, 1], true, false).as_horde(true),
                ],
                vec![0, 1],
                vec![Post::Front, Post::Front],
            );
            play_round(&mut b, &[Act::Clash(1), Act::Hold]);
            pack - b.units[1].health
        };

        // **A sweep clears a pack of ANY size** - one card, whatever its Might. A pack has nowhere to hide.
        for pack in [8, 20, 40] {
            assert_eq!(
                felled([1, 3, 3, 1, 2], true, pack),
                pack,
                "the Bastion's Sweep clears a pack of {pack} - Might 1, and it does not matter"
            );
            assert_eq!(
                felled([3, 3, 1, 1, 2], true, pack),
                pack,
                "the Bombardier's Salvo clears a pack of {pack}"
            );
        }

        // **An aimed blow fells ONE body.** Not `Might`-many: one blow, one of them. Spec 4.6 spills instead,
        // and `combat::resolve_strike` implements the spill - which made a Might-7 aimed blow a BETTER
        // horde-killer than a sweep, inverting both horde locks. Width against a pack is what a sweep is *for*;
        // if a big enough single blow can do the same job, the sweep has no job.
        //
        // So the Raider chews two bodies a round (its opening blow, plus one poured card) whatever its Might -
        // and Might buys it nothing here at all. That is the lock.
        let raider = felled([7, 6, 1, 2, 2], false, 8);
        assert!(
            raider <= 2,
            "an aimed blow fells one body per strike - Might 7 bought it nothing: it felled {raider}"
        );
    }

    /// **THE VOLLEY: the back line fires on an incoming raider.** The cannons defend themselves - which is what
    /// makes a screened rearguard powerful rather than merely hidden.
    ///
    /// And it is its own sub-phase, not one pile with the Intercept, for a reason that is pure tempo economy:
    /// **a slipper the front already killed is not shot at twice.** The saved card is real, and that is what
    /// earns the boundary under the razor.
    #[test]
    fn the_back_line_volleys_an_incoming_raider() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [7, 6, 1, 3, 2], true, false), // 0 - coming for the cannon
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),     // 1 - their front
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),     // 2 - their back
            ],
            vec![0, 1, 1],
            vec![Post::Front, Post::Front, Post::Back],
        );
        let raider = b.units[0].health;
        // It pushes through: it spends nothing on the line, so every blow the line lands, lands.
        play_round(
            &mut b,
            &[Act::Raid(2, Answer::Push), Act::Clash(0), Act::Clash(0)],
        );
        assert_eq!(b.regions[0], 1, "it got through");
        assert!(
            b.units[0].health < raider,
            "and it was shot on the way in - by the wall AND by the cannon it was coming for"
        );
    }

    /// **SLIPPING IS A THIRD POSITION.** A body in transit has left the front and not arrived anywhere: while it
    /// is there it can neither screen nor be screened, and in particular **it cannot catch another slipper**.
    ///
    /// Without this, a body could be running across open ground and simultaneously holding the wall it just
    /// abandoned.
    #[test]
    fn a_body_in_transit_cannot_hold_the_line_it_just_left() {
        let b = Board::new(
            vec![
                unit("Guard", Side::Party, [3, 4, 1, 2, 2], true, false), // 0 - our front...
                unit("Cannon", Side::Party, [4, 2, 1, 2, 2], false, true), // 1 - ...screening this
                unit("Ogre", Side::Foe, [5, 5, 2, 3, 2], true, false),    // 2 - raiding our cannon
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),    // 3 - their back
            ],
            vec![0, 0, 1, 1],
            vec![Post::Front, Post::Back, Post::Front, Post::Back],
        );
        // Our Guard slips away to raid THEIR back at the same moment their Ogre raids ours.
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
            "so it is NOT holding the line it just left - it cannot catch the Ogre coming the other way"
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
