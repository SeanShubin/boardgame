//! **The regions / relations combat model** — the candidate successor to the rank + 5-sub-phase schedule.
//! Design: `needs-merge/regions-and-relations-combat.md`. Proved load-bearing by `examples/v2_regions`.
//!
//! This module is **additive and inert**: nothing in the shipped game calls it. It reuses [`crate::combat`]'s
//! resolvers (Engage / Evade / Strike, the grit pile, the round Reset) unchanged - only the *schedule* and the
//! *geometry* are new. So the two models can be compared side by side without either disturbing the other.
//!
//! # Why the rank model breaks, and what this fixes
//!
//! The five sub-phases (Intercept / Volley / Raid / Clash / Breach) describe exactly one situation: bodies
//! moving through open ground. That picture is perfect in round 1 - the lines are closing, the gap is real -
//! and it is *nonsense* by round 3, when nobody is in transit and the game runs the crossing gauntlet anyway.
//! The spec re-rolls ranks every round without anyone moving; the product froze them forever and still narrates
//! a charge each round. **Neither model has any transit after round 1, and the schedule is made of nothing but
//! transit.**
//!
//! So: fire the gauntlet on **movement** instead of on the round, and charge for movement.
//!
//! # The model
//!
//! **Space is a partition.** Each unit has a region id. Same region = *together* (in contact; melee reaches;
//! an area strike catches you both). Different regions = *apart*. There is **no map** - regions are derived
//! from what people declare, and the ids are [`canonical`]ized before they enter a memo key, because a
//! *labelling* is not a *position*. "The back" is not a rank: it is **any region with no enemy in it**.
//!
//! **Each body declares one [`Aim`], and its move follows from it.** Five variants, covering all four things the
//! design asked for - who is together or apart, who they defend, who they assassinate (support is unbuilt):
//!
//! - [`Aim::Press`] - violence at that enemy. Melee crosses to it; an arrow does not walk.
//! - [`Aim::Defend`] - put my body between that ally and harm.
//! - [`Aim::Regroup`] - get in *behind* an ally (so *they* can guard *me* - the opposite direction to Defend).
//! - [`Aim::Hold`] - stay put, do nothing.
//! - [`Aim::Peel`] - retreat alone onto empty ground.
//!
//! A separate `Move` axis crossed with an `Aim` axis was **built and measured, and it cost 200x for no
//! expressiveness this enum lacks** - see [`Aim`]. The tree is not deeper; the cost is per-node enumeration.
//!
//! **[`Defend`](Aim::Defend) is a damage REDIRECT** ([`screen_head`]), not a separate contest. A blow aimed at
//! W lands on W's living defender instead, and that chains. So the screen, the bodyguard and the back-access
//! rule are **one mechanic** - with no gate and no immunity. Kill the screen and the blow lands. Force, not
//! fiat. Cycles are broken by the walk and are merely *expensive*, never impassable.
//!
//! **The one law:** *ground you cross is ground you cross unscreened - including your own screen.* A crosser is
//! reached by every living enemy in the region it **leaves** (you turn your back) and every living enemy in the
//! region it **enters** (they watch you come). It pays [`combat::slip_cost`] to break all of it and arrive, or
//! it Stands - and is **caught**: it eats the blows and does *not* arrive. The Outrider's "exposed both ways"
//! stops being a property of a role and becomes what happens to *anyone* who moves.
//!
//! # Four sub-phases, not five
//!
//! The razor (which `combat.rs` already states: the sub-phase boundary "is where the dead stop fighting, not
//! where wounds close"): **a sub-phase exists for exactly one reason - so that a death in it can silence
//! something later.** Two effects that should *trade* go in the same sub-phase. Nothing else earns a boundary.
//!
//! | [`SubPhase`] | what happens                                              | a death here silences  |
//! |--------------|-----------------------------------------------------------|------------------------|
//! | `Cross`      | parting blows + the destination's fire, in **one** pile    | the crosser's Arrival  |
//! | `Arrive`     | survivors land and strike                                 | the victim's Contact   |
//! | `Contact`    | everyone co-located with an enemy trades                   | a screen -> ground opens |
//! | `Breach`     | leftover tempo; redirects recomputed (dead screens gone)   | (last)                 |
//!
//! `Cross` is **Intercept + Volley merged**: they silence the same thing (the Raid) but not each other, so
//! under the razor they should *trade in one pile* rather than sit in ordered boxes - and merging them lets the
//! screen's damage and the volley's damage **combine** against the runner, which is what the fiction wanted.
//!
//! It reproduces the old schedule *exactly* in round 1 with the old formation shape. It is a strict
//! generalization - and unlike the old one, it still means something in round 4.
//!
//! **Pouring.** `Cross` and `Arrive` give **one blow** (the opening blow the reach already paid for); `Contact`
//! and `Breach` **pour** the pool. You cannot stand in the open whaling on a body that is running *past* you -
//! a crossing draws a snap shot, not your whole round. (Without this the entire fight resolves in `Cross` and
//! the other three sub-phases are dead.)
//!
//! Damage closes at the **round Reset** only - `combat.rs`'s existing rule, unchanged.

use std::collections::HashMap;

use crate::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};

/// A fight not decided in five rounds is a draw, and a draw is not a win (spec 0.4).
pub const MAX_ROUNDS: usize = 5;

/// The four sub-phases, in schedule order. See the module docs for why there are exactly four.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubPhase {
    /// Parting blows and the destination's fire - everything that punishes movement, in one pile.
    Cross,
    /// Survivors of the crossing land and strike. The raid pre-empts the melee.
    Arrive,
    /// Everyone co-located with an enemy trades.
    Contact,
    /// The blows waiting on ground that just opened - a dead screen no longer screens.
    Breach,
}

impl SubPhase {
    pub const ALL: [SubPhase; 4] = [
        SubPhase::Cross,
        SubPhase::Arrive,
        SubPhase::Contact,
        SubPhase::Breach,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SubPhase::Cross => "Cross",
            SubPhase::Arrive => "Arrive",
            SubPhase::Contact => "Contact",
            SubPhase::Breach => "Breach",
        }
    }

    /// Whether a body may spend its **whole pool** here, or only land the opening blow its reach already paid
    /// for. See the module docs: the pre-empt is a snap shot, not a round.
    pub fn pours(self) -> bool {
        matches!(self, SubPhase::Contact | SubPhase::Breach)
    }
}

/// **Where a body stands within its own side's contingent in its region.**
///
/// This is the vanguard/rearguard of the old rank model, *localized to a region* - and it is the thing that
/// makes a region a **formation** rather than a bag of bodies. There is no "who guards whom": the front
/// collectively screens the back, and **any** front can catch a slipper going for **any** back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Post {
    /// **The front.** You can be shot, you can be clashed with, and **you can catch a slipper** going for anyone
    /// behind you. Being reachable *is* the job.
    Front,
    /// **The back.** Ranged fire cannot pick you out while a front stands; melee must **slip past** the front to
    /// reach you. Not safety by decree - safety somebody else is paying for.
    Back,
}

/// **What a body does with its round.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    /// **Strike that enemy.** A melee body crosses to its region (paying the gauntlet) and then either *clashes*
    /// with it (if it is a front) or *slips past* its front to reach it (if it is a back). A ranged body shoots
    /// from where it stands - an arrow does not walk - and may only pick out a **front**.
    Strike(usize),
    /// Stay where you are, and do not attack. Take your post and wait.
    Wait,
    /// Cross to that region without attacking - to regroup with allies, or to reinforce a line.
    Join(u8),
    /// Retreat alone onto fresh, empty ground. Not free: you are crossing, so everything hostile in the region
    /// you leave gets a parting blow at you.
    Peel,
}

/// **A body's whole declaration: a post, and an act.**
///
/// This restores the Vanguard / Outrider / Rearguard triangle *without ranks* - and it restores it as **force,
/// not fiat**, which the previous cut had quietly broken:
///
/// - **Vanguard** = [`Post::Front`]. Hold the line, and catch what tries to get past you.
/// - **Rearguard** = [`Post::Back`] with a ranged strike. Deal from safety somebody else is paying for.
/// - **Outrider** = a **melee `Strike` at an enemy in the `Back`**. It is not a role and not even a separate
///   declaration - it is simply *what pressing a screened target means*. You bid Tempo to slip the front; get
///   through and you are on the cannon, get caught and your tempo went to the body that caught you.
///
/// The previous version claimed this same mapping and did not deliver it: every aimed blow was silently
/// **redirected** onto the screen, so a screened body was *immune* until its guard died. That is fiat. The
/// screen is now a **price** - and enough Tempo always pays it.
///
/// `Slip > Cannon > Front > Slip`: the front catches the slipper (the old Intercept), the slipper reaches the
/// cannon early (the old Raid), and only the cannon's Might cracks the front's Grit (the old Clash). The
/// playstyle triangle, intact, with no ranks anywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Aim {
    pub post: Post,
    pub act: Act,
}

impl Aim {
    pub fn new(post: Post, act: Act) -> Self {
        Aim { post, act }
    }

    /// The do-nothing declaration every body always has.
    pub const WAIT: Aim = Aim {
        post: Post::Front,
        act: Act::Wait,
    };

    /// A player-facing sentence.
    pub fn label(self, board: &Board) -> String {
        let units = &board.units;
        let post = match self.post {
            Post::Front => "front",
            Post::Back => "back",
        };
        match self.act {
            Act::Strike(t) => {
                // Name the *kind* of attack, because that is the decision - clash or slip.
                format!("Strike {} (from the {post})", units[t].name)
            }
            Act::Wait => format!("Hold the {post}"),
            Act::Join(r) => match board.in_region(r).first() {
                Some(&i) => format!("Fall in with {}", units[i].name),
                None => format!("Move to region {}", (b'A' + r) as char),
            },
            Act::Peel => "Peel away alone".to_string(),
        }
    }
}

/// The whole mutable position: the bodies, and which region each stands in.
#[derive(Clone, Debug)]
pub struct Board {
    pub units: Vec<Combatant>,
    /// `regions[i]` is the region unit `i` stands in. Ids are arbitrary; only the *partition* is meaningful.
    pub regions: Vec<u8>,
}

impl Board {
    /// The opening position: the two sides **apart**. That is what makes round 1 a mass crossing - and is
    /// exactly why the old schedule's fiction works in round 1 and nowhere else.
    pub fn opening(units: Vec<Combatant>) -> Board {
        let regions = units
            .iter()
            .map(|u| if u.side == Side::Party { 0u8 } else { 1u8 })
            .collect();
        let mut b = Board { units, regions };
        b.sync_ranks();
        b
    }

    /// **Adapter, not mechanic.** [`Combatant::rank`] is vestigial in this model - position is a *region* plus a
    /// [`Post`], not a rank. But we reuse `combat`'s resolvers unchanged, and [`combat::resolve_engage`] gates
    /// every engagement through `effective_in_rank`: a body whose rank does not match its reach lands nothing.
    /// So the rank field has to be kept consistent with the reach, or it silently *disables* bodies.
    ///
    /// It did exactly that. Every unit was built at `Rank::Vanguard`, which `effective_in_rank` reads as
    /// "melee" - so **every ranged body in this model was mute**, unable to form a contact at all. The
    /// Bombardier only appeared to work because an area strike goes through [`aoe_sweep`], which bypasses that
    /// gate. The Marksman had never landed a single aimed shot.
    ///
    /// Caught by `enough_tempo_slips_the_screen_and_reaches_the_body_behind_it`, which is the sort of thing
    /// end-to-end tests are for: every unit test passed while half the roster was silently switched off.
    fn sync_ranks(&mut self) {
        for u in &mut self.units {
            u.rank = if u.ranged && !u.melee {
                deckbound_content::rank::Intention::Rearguard
            } else {
                deckbound_content::rank::Intention::Vanguard
            };
        }
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

    /// The living bodies standing in `region`.
    pub fn in_region(&self, region: u8) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&i| !self.units[i].fallen && self.regions[i] == region)
            .collect()
    }

    /// The regions that currently hold a living body, in ascending id order - the piles a renderer would draw.
    pub fn occupied(&self) -> Vec<u8> {
        let mut rs: Vec<u8> = (0..self.units.len())
            .filter(|&i| !self.units[i].fallen)
            .map(|i| self.regions[i])
            .collect();
        rs.sort_unstable();
        rs.dedup();
        rs
    }

    /// Is this region **safe ground** - no enemy of `side` standing in it? This is what replaces "the back":
    /// it is not a rank, it is a fact about the board, and it stops being true the moment somebody walks in.
    pub fn is_safe(&self, region: u8, side: Side) -> bool {
        !self
            .in_region(region)
            .iter()
            .any(|&i| self.units[i].side != side)
    }
}

/// Canonicalize a region assignment: relabel in first-appearance order, so a *labelling* is not a *position*.
///
/// `{A: Bastion, B: Marksman}` and `{B: Bastion, A: Marksman}` are the same board, and a memo that treats them
/// as different is paying for nothing. This is the single most important tractability move in the design: it
/// makes the partition a **cheaper** state than a rank assignment (`Bell(n) < 3^n` for every `n <= 8`, which
/// covers every real encounter - 4140 vs 6561 at eight bodies).
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

/// The **first empty region** - the ground an [`Act::Peel`] runs to. Counts only the living, so a corpse's old
/// region is reusable and the ids do not creep upward all fight.
fn empty_ground(board: &Board) -> u8 {
    let taken = board.occupied();
    (0u8..).find(|r| !taken.contains(r)).unwrap_or(u8::MAX)
}

/// The region unit `i` ends its move in. `None` = it does not move (and so pays no crossing).
///
/// **The move follows from the act.** A melee body that strikes must close; an arrow does not walk.
pub fn destination(board: &Board, i: usize, aim: Aim) -> Option<u8> {
    let (units, regions) = (&board.units, &board.regions);
    let here = regions[i];
    match aim.act {
        Act::Strike(t) => (!units[i].ranged && regions[t] != here).then(|| regions[t]),
        Act::Wait => None,
        Act::Join(r) => (r != here).then_some(r),
        Act::Peel => (board.in_region(here).len() > 1).then(|| empty_ground(board)),
    }
}

/// **The front line of `side` in `region`** - the living bodies posted there at [`Post::Front`].
///
/// This is the whole screen. Every one of them can catch a slipper going for *anyone* behind them, and while
/// even one stands, ranged fire cannot pick out the bodies at their back. It is not a decree: it is a list of
/// bodies, and it gets shorter every time one of them dies.
pub fn front_line(board: &Board, aims: &[Aim], region: u8, side: Side) -> Vec<usize> {
    board
        .in_region(region)
        .into_iter()
        .filter(|&i| board.units[i].side == side && aims[i].post == Post::Front)
        .collect()
}

/// **Is `target` screened?** - it is posted at the back, *and* its side still has a front standing in its
/// region to do the screening. A back line with no front left is simply exposed.
pub fn is_screened(board: &Board, aims: &[Aim], target: usize) -> bool {
    aims[target].post == Post::Back
        && !front_line(board, aims, board.regions[target], board.units[target].side).is_empty()
}

/// **Declaring is free, and it may fail.** Legality here deliberately does *not* ask where the enemy is posted,
/// because posts are declared **simultaneously** - you do not know their formation when you commit to yours.
///
/// So a ranged body may name any enemy, and if that enemy turns out to be screened, the shot simply **hits the
/// wall instead** ([`attack_of`] re-checks at resolution and falls back to what it can actually pick out). You
/// aimed at the mage; there was a shield in the way; you shot the shield. That is the spec's "a misplaced
/// intention simply fails" - punished by outcome, never barred by rule.
/// **Every legal declaration for unit `i`** - the count-adaptive candidate list (spec 4.1: a choice is offered
/// only where it would actually *do* something). **This is the branching factor the design lives or dies by.**
///
/// Two prunings keep it small, and both are rules rather than optimizations:
///
/// 1. **A body that crosses arrives at the front.** You charged in; you are at the sharp end by definition. So a
///    crossing act carries no post choice - which is exactly right, and it halves the branch for every act that
///    moves.
/// 2. **A post choice only exists where there is a line to be part of.** A lone body in a region is exposed
///    whatever it calls itself, so it is offered no post (spec 4.1: a choice with one legal option is not a
///    choice).
///
/// The ranged-targeting rule is *aim-dependent* (it asks where the enemy is posted), so this takes the current
/// `aims` - which for the search is the enemy's declaration for this round.
///
/// **Strike comes first, deliberately.** A reachability search short-circuits on the first winning line, so this
/// order decides which of several winning lines gets *shown*, and an attack is the line a player recognizes.
pub fn legal_aims(board: &Board, i: usize) -> Vec<Aim> {
    let units = &board.units;
    let here = board.regions[i];
    let alone = board.in_region(here).len() <= 1;

    // A post is a real choice only when you have company to form a line with.
    let posts: &[Post] = if alone {
        &[Post::Front]
    } else {
        &[Post::Front, Post::Back]
    };

    let mut out = Vec::new();

    // Strikes. A crossing strike arrives at the front, so it carries no post choice.
    for (t, u) in units.iter().enumerate() {
        if u.fallen || u.side == units[i].side || !(units[i].melee || units[i].ranged) {
            continue;
        }
        let crosses = !units[i].ranged && board.regions[t] != here;
        if crosses {
            out.push(Aim::new(Post::Front, Act::Strike(t)));
        } else {
            for &p in posts {
                out.push(Aim::new(p, Act::Strike(t)));
            }
        }
    }

    // Hold your post and wait.
    for &p in posts {
        out.push(Aim::new(p, Act::Wait));
    }

    // Move without attacking - arriving at the front, as any crosser does.
    for r in board.occupied() {
        if r != here {
            out.push(Aim::new(Post::Front, Act::Join(r)));
        }
    }
    if !alone {
        out.push(Aim::new(Post::Front, Act::Peel));
    }
    out
}

// ---- greedy tempo allocation ---------------------------------------------------------------------------

/// The tempo `f_def` would need to slip a reach worth `bid` - `combat::slip_cost`'s arithmetic, for a bid that
/// has not been committed yet.
fn slip_price(bid: u32, f_def: u32) -> u32 {
    bid / f_def.max(1) + 1
}

/// The greedy reach: the fewest cards the target **cannot afford to slip** - so it lands for certain - else one
/// card and take the chance. Every card saved becomes a blow. (An area strike forms no contact and cannot be
/// slipped, so it costs exactly one and nothing is gained by committing more.)
pub fn reach_cards(units: &[Combatant], a: usize, t: usize) -> u32 {
    if units[a].aoe {
        return 1;
    }
    (1..=units[a].tempo)
        .find(|&c| slip_price(c * units[a].finesse.max(1), units[t].finesse) > units[t].tempo)
        .unwrap_or(1)
}

/// The foe script: a fixed, deterministic policy, so this stays a **single-agent reachability search** and not
/// a minimax (spec 0.1 - creatures are an environment, not an opponent that searches back).
///
/// Every foe hunts the living hero it can most cheaply finish: a **melee** body closes on it (and pays the
/// crossing like anyone else); a **ranged** body holds its ground and shoots. Simple, and it makes the foes
/// exert exactly the pressure the geometry is supposed to answer.
pub fn foe_aims(board: &Board) -> Vec<Option<Aim>> {
    let units = &board.units;
    (0..units.len())
        .map(|i| {
            if units[i].side != Side::Foe || units[i].fallen {
                return None;
            }
            // Hunt the living hero it can most cheaply finish. If that body turns out to be screened, the
            // blow lands on the screen instead - the foes discover the party's formation the same way the
            // party discovers theirs: by hitting it.
            let prey = (0..units.len())
                .filter(|&j| units[j].side == Side::Party && !units[j].fallen)
                .min_by_key(|&j| (units[j].health, units[j].grit));
            // The foes hold the front: they are the wall the party has to get through, and a scripted policy
            // that hid in its own back line would be an environment that refuses to fight.
            prey.map(|p| Aim::new(Post::Front, Act::Strike(p)))
        })
        .collect()
}

// ---- resolution ----------------------------------------------------------------------------------------

/// What happened in one sub-phase - enough for a renderer or a transcript to say *why* the board changed.
#[derive(Clone, Debug, Default)]
pub struct SubPhaseLog {
    /// Bodies that tried to cross and were **caught** - they stay put and their aim is spent on the wall.
    pub caught: Vec<usize>,
    /// Bodies that got through and are now standing somewhere new.
    pub arrived: Vec<usize>,
    /// Bodies that fell at this boundary.
    pub fallen: Vec<usize>,
}

/// **The area strike, region-wide.** The mechanic the regions model unlocks: once *who is standing together* is
/// a fact on the board, an area strike stops being a single-target special case and becomes what it always
/// should have been - **it hits the whole knot.**
///
/// One tempo card. Every living enemy in the target's region, at full Might, **unevadable** (it forms no
/// slippable contact), and it **bypasses the screen** - a bodyguard soaks an aimed blow but cannot cover an
/// area. That last clause is the anti-cluster counter, and it is what *prices* the whole Defend mechanic: pile
/// bodies behind a screen and you become a **target**. Concentration and coverage finally trade against each
/// other, decided by a positional fact the player controls.
///
/// **Deliberately NOT the product's "one sweep clears the whole pack" horde rule** (`combat.rs`). Region-wide,
/// that is absurd: it let one Salvo delete *both* Swarms - sixteen bodies - for a single card, and seven of
/// eight encounters collapsed to a round-one wipe. A horde takes a sweep like anything else (penetrating Might
/// spills body to body). **The brake belongs in the horde rule, not in the region rule.**
fn aoe_sweep(board: &mut Board, sweeps: &[Engage]) -> Vec<Contact> {
    let mut extra = Vec::new();
    for e in sweeps {
        let a = e.attacker;
        if board.units[a].fallen || board.units[a].tempo == 0 {
            continue;
        }
        board.units[a].tempo -= 1;
        let (side, region) = (board.units[a].side, board.regions[e.target]);
        for j in 0..board.units.len() {
            if board.units[j].fallen || board.units[j].side == side || board.regions[j] != region {
                continue;
            }
            // A damage-only edge: no bid, so it cannot be slipped and nobody answers along it.
            extra.push(Contact {
                attacker: a,
                target: j,
                bid: 0,
            });
        }
    }
    extra
}

/// The Strike step: each contact's opening blow, plus - if the sub-phase `pours` - everyone's leftover tempo
/// poured along an edge they can swing on. `sweeps` are the area edges (unevadable, unanswerable). One
/// order-free, commit-based batch: a blow lands even if its striker died in the same sub-phase.
///
/// **No redirect.** Damage goes where it was aimed, because *getting* it aimed there was already paid for at
/// the catch (see [`resolve_strikes`]). The previous version silently redirected every aimed blow onto the
/// screen, which made a screened body immune until its guard died - fiat, not force.
fn strike_along(board: &mut Board, contacts: &[Contact], sweeps: &[Contact], pour: bool) {
    let blows: Vec<Blows> = (0..board.units.len())
        .filter(|&i| pour && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| {
            // Whoever you are in contact with: the body you reached, or - on a melee edge - the body that
            // reached *you*. It came to you; you may answer.
            let t = contacts
                .iter()
                .find(|c| c.attacker == i)
                .map(|c| c.target)
                .or_else(|| {
                    contacts
                        .iter()
                        .find(|c| {
                            c.target == i
                                && board.units[i].melee
                                && board.regions[c.attacker] == board.regions[i]
                        })
                        .map(|c| c.attacker)
                })?;
            Some(Blows {
                unit: i,
                target: t,
                cards: board.units[i].tempo,
            })
        })
        .collect();
    let all: Vec<Contact> = contacts.iter().chain(sweeps).copied().collect();
    combat::resolve_strike(&mut board.units, &all, &blows);
}

/// The greedy dodge: stand if you can answer along the edge, else slip if you can afford it and the blow
/// actually threatens you. (The `battle.rs` greedy, in spirit.)
fn dodges_against(board: &Board, reaching: &[Contact]) -> Vec<Dodge> {
    (0..board.units.len())
        .map(|i| {
            let Some(cost) = combat::slip_cost(&board.units, reaching, i) else {
                return Dodge::Stand; // nothing is reaching you
            };
            if board.units[i].fallen || cost > board.units[i].tempo {
                return Dodge::Stand; // you cannot afford it, so it is not on offer
            }
            if reaching.iter().any(|c| {
                c.target == i
                    && board.regions[c.attacker] == board.regions[i]
                    && board.units[i].melee
            }) {
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

/// One body attacking another: `(attacker, target)`.
type Attack = (usize, usize);

/// **The catch, and then the blow** - one sub-phase of strikes, and the heart of the model.
///
/// `attacks` is `(attacker, target)`: who is trying to hit whom.
///
/// 1. **The catch.** Every attacker going for a **screened** body (a [`Post::Back`] with a living front) is
///    engaged by *that whole front line* - **any** front can catch a slipper going for **any** back. This is the
///    old Intercept, and it needs no new mechanic: it is [`combat::resolve_engage`].
/// 2. **The slip.** The would-be slipper pays [`combat::slip_cost`] to break every catch on it, or it stands.
///    Paying is *getting through*. Standing is *being caught*.
/// 3. **The blow.** A slipper who got through strikes the body it crossed for. A slipper who was **caught**
///    strikes the front that caught it instead - its tempo went there, exactly as it did in the old Intercept.
///    Everyone else simply clashes with what they aimed at.
///
/// So the screen is a **price**, never an immunity. Out-bid the front and you are on the cannon; fail, and you
/// are in a fight with the wall, poorer for having tried. **Force, not fiat.**
///
/// An **area** strike skips all of it: it nukes the whole region, both tiers, unevadable. That is the
/// anti-cluster counter, and it is what stops a deep formation from being free - a bodyguard soaks an aimed
/// blow but cannot cover an area.
fn resolve_strikes(board: &mut Board, aims: &[Aim], attacks: &[Attack], pour: bool) {
    let (sweeps, aimed): (Vec<Attack>, Vec<Attack>) =
        attacks.iter().partition(|&&(a, _)| board.units[a].aoe);

    let sweep_engagements: Vec<Engage> = sweeps
        .iter()
        .map(|&(a, t)| Engage {
            attacker: a,
            target: t,
            cards: 1,
        })
        .collect();
    let extra = aoe_sweep(board, &sweep_engagements);

    // ---- 1. the catch: the front line reaches for every slipper ------------------------------------------
    let slippers: Vec<Attack> = aimed
        .iter()
        .copied()
        .filter(|&(a, t)| board.units[a].melee && is_screened(board, aims, t))
        .collect();

    let mut catches: Vec<Engage> = Vec::new();
    for &(a, t) in &slippers {
        let side = board.units[t].side;
        for f in front_line(board, aims, board.regions[t], side) {
            if board.units[f].tempo > 0 {
                catches.push(Engage {
                    attacker: f,
                    target: a,
                    cards: reach_cards(&board.units, f, a),
                });
            }
        }
    }
    let reaching = combat::resolve_engage(&mut board.units, &catches);

    // ---- 2. the slip: pay in full and get through, or stand and be caught --------------------------------
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| {
            let is_slipper = slippers.iter().any(|&(a, _)| a == i);
            match combat::slip_cost(&board.units, &reaching, i) {
                // It crossed for the body behind the line; getting through is the whole point, so it pays if it
                // can afford to.
                Some(cost) if is_slipper && cost <= board.units[i].tempo => Dodge::Slip,
                _ => Dodge::Stand,
            }
        })
        .collect();
    let caught = combat::resolve_evade(&mut board.units, &reaching, &dodges);

    // ---- 3. the blow ------------------------------------------------------------------------------------
    let engagements: Vec<Engage> = aimed
        .iter()
        .filter(|&&(a, _)| !board.units[a].fallen && board.units[a].tempo > 0)
        .map(|&(a, t)| {
            // Caught: your tempo goes to the body that caught you. Through: you are on what you crossed for.
            let tgt = caught
                .iter()
                .find(|c| c.target == a)
                .map(|c| c.attacker)
                .unwrap_or(t);
            Engage {
                attacker: a,
                target: tgt,
                cards: reach_cards(&board.units, a, tgt),
            }
        })
        .collect();

    let striking = combat::resolve_engage(&mut board.units, &engagements);
    let dodges = dodges_against(board, &striking);
    let contacts = combat::resolve_evade(&mut board.units, &striking, &dodges);

    // A front that caught a slipper is now in a fight with it - that edge lands its blows too.
    let all: Vec<Contact> = contacts.iter().chain(caught.iter()).copied().collect();
    strike_along(board, &all, &extra, pour);
    combat::end_sub_phase(&mut board.units);
}

/// Who unit `i` is actually attacking this sub-phase: its declared quarry if it can still reach it, else
/// whatever enemy is standing in its region - *a body in your region did not ask your permission*. That
/// fallback is the product's existing mutual-melee rule: it did not choose the fight.
///
/// A **ranged** body can only pick out what it could pick out at declaration time: a front, or anyone once that
/// front is gone. A **melee** body reaches anyone in its own region - a screened target is **priced**, not
/// barred.
fn attack_of(board: &Board, aims: &[Aim], i: usize) -> Option<Attack> {
    let (units, regions) = (&board.units, &board.regions);
    if units[i].fallen || units[i].tempo == 0 {
        return None;
    }
    let reaches = |t: usize| {
        !units[t].fallen
            && units[t].side != units[i].side
            && if units[i].ranged {
                !is_screened(board, aims, t)
            } else {
                units[i].melee && regions[t] == regions[i]
            }
    };
    if let Act::Strike(t) = aims[i].act
        && reaches(t)
    {
        return Some((i, t));
    }
    (0..units.len()).find(|&j| reaches(j)).map(|j| (i, j))
}

/// Play **one whole round** from the declared aims: the Reset, then Cross -> Arrive -> Contact -> Breach.
/// Returns one [`SubPhaseLog`] per sub-phase, in order.
pub fn play_round(board: &mut Board, aims: &[Aim]) -> Vec<SubPhaseLog> {
    combat::refresh_round(&mut board.units);
    let mut logs = Vec::new();
    let living = |b: &Board| -> Vec<bool> { b.units.iter().map(|u| !u.fallen).collect() };

    // ---- Cross: ground you cross is ground you cross unscreened ------------------------------------------
    // Every living enemy **who is standing still** in the region you LEAVE (you turn your back) and the region
    // you ENTER (they watch you come) reaches for you. ONE pile - they silence the same thing (your Arrival),
    // so under the razor they trade rather than sit in ordered boxes. This is Intercept + Volley, merged.
    //
    // **A body that is itself crossing cannot catch anyone.** That is the one law applied to *both* ends - you
    // are outside your own screen the moment you move, so you are in no position to be anybody else's. Without
    // it, two sides charging each other each run the full gauntlet on the other, everyone spends their whole
    // pool in the Cross, and Arrive / Contact / Breach are dead sub-phases with nothing left to spend. (That is
    // exactly what the first cut did: the fight resolved entirely in the crossing, every round.)
    let mut dests: Vec<Option<u8>> = (0..board.units.len())
        .map(|i| (!board.units[i].fallen).then(|| destination(board, i, aims[i]))?)
        .collect();

    // **Two bodies charging each other MEET.** Without this they swap regions and pass like ships - the Raider
    // arrives where the Ogre was, the Ogre arrives where the Raider was, neither ever lands a blow, and the
    // fight loops until the round cap. (It really did: `a_fight_resolves` caught it.)
    //
    // Simultaneous mutual charge has no non-arbitrary resolution - somebody has to give ground - so we pick a
    // rule and state it: **they meet on the lower-numbered ground.** Which ground is a tie-break with no
    // meaning; *that they meet* is the part that matters. Flagged as an arbitrary dial.
    let charging_at = |i: usize| match aims[i].act {
        Act::Strike(t) if board.units[i].melee && !board.units[i].ranged => Some(t),
        _ => None,
    };
    for i in 0..board.units.len() {
        let Some(t) = charging_at(i) else { continue };
        if charging_at(t) == Some(i) {
            let ground = board.regions[i].min(board.regions[t]);
            dests[i] = (board.regions[i] != ground).then_some(ground);
            dests[t] = (board.regions[t] != ground).then_some(ground);
        }
    }

    let mut engagements: Vec<Engage> = Vec::new();
    for (i, dest) in dests.iter().enumerate() {
        let Some(d) = *dest else { continue };
        for (e, e_dest) in dests.iter().enumerate() {
            if board.units[e].fallen
                || board.units[e].side == board.units[i].side
                || board.units[e].tempo == 0
                || e_dest.is_some()
            // it is crossing too - it cannot hold ground it is not standing on
            {
                continue;
            }
            if board.regions[e] == board.regions[i] || board.regions[e] == d {
                engagements.push(Engage {
                    attacker: e,
                    target: i,
                    cards: reach_cards(&board.units, e, i),
                });
            }
        }
    }
    let before = living(board);
    let (sweeps, aimed): (Vec<Engage>, Vec<Engage>) = engagements
        .iter()
        .partition(|e| board.units[e.attacker].aoe);
    let extra = aoe_sweep(board, &sweeps);
    let reaching = combat::resolve_engage(&mut board.units, &aimed);

    // The crosser answers: pay in full and get through, or Stand - and be CAUGHT.
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| match combat::slip_cost(&board.units, &reaching, i) {
            Some(cost) if cost <= board.units[i].tempo && dests[i].is_some() => Dodge::Slip,
            _ => Dodge::Stand,
        })
        .collect();
    let slipped: Vec<bool> = (0..board.units.len())
        .map(|i| {
            dodges[i] == Dodge::Slip && combat::slip_cost(&board.units, &reaching, i).is_some()
        })
        .collect();
    let contacts = combat::resolve_evade(&mut board.units, &reaching, &dodges);
    strike_along(board, &contacts, &extra, SubPhase::Cross.pours());
    combat::end_sub_phase(&mut board.units);

    // Arrival is settled here: you got through if nothing reached you, or if you paid to break all of it.
    // Stand in the open and you are **caught** - you stay where you were, and the crossing was spent for
    // nothing.
    let mut log = SubPhaseLog {
        fallen: fell(&before, board),
        ..Default::default()
    };
    let mut arrived = vec![false; board.units.len()];
    for (i, dest) in dests.iter().enumerate() {
        let Some(d) = *dest else { continue };
        if board.units[i].fallen {
            continue;
        }
        if !reaching.iter().any(|c| c.target == i) || slipped[i] {
            board.regions[i] = d;
            arrived[i] = true;
            log.arrived.push(i);
        } else {
            log.caught.push(i);
        }
    }
    logs.push(log);

    // ---- Arrive: the survivors of the crossing land and strike. The raid pre-empts the melee. -------------
    // Only the bodies that just crossed act here - that early slot is the whole of what the crossing bought.
    let before = living(board);
    let arrivals: Vec<Attack> = (0..board.units.len())
        .filter(|&i| arrived[i])
        .filter_map(|i| attack_of(board, aims, i))
        .collect();
    resolve_strikes(board, aims, &arrivals, SubPhase::Arrive.pours());
    logs.push(SubPhaseLog {
        fallen: fell(&before, board),
        ..Default::default()
    });

    // ---- Contact: everyone who can reach an enemy trades. ------------------------------------------------
    // ---- Breach: whatever tempo is left, against a line that may have just broken. ------------------------
    // Both walk the same code, and the difference between them is entirely a matter of *when*: a front that
    // dies in Contact is not there to catch a slipper in the Breach, so the ground behind it has opened.
    for phase in [SubPhase::Contact, SubPhase::Breach] {
        let before = living(board);
        let attacks: Vec<Attack> = (0..board.units.len())
            .filter_map(|i| attack_of(board, aims, i))
            .collect();
        resolve_strikes(board, aims, &attacks, phase.pours());
        logs.push(SubPhaseLog {
            fallen: fell(&before, board),
            ..Default::default()
        });
    }
    logs
}

fn fell(before: &[bool], board: &Board) -> Vec<usize> {
    (0..board.units.len())
        .filter(|&i| before[i] && board.units[i].fallen)
        .collect()
}

// ---- the doom oracle -----------------------------------------------------------------------------------

/// What the oracle says about a position - the three states, exactly as the shipped one names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Some line from here wins.
    Winnable,
    /// The search ran out of budget. **Not an answer** - an answer in progress.
    Evaluating,
    /// The tree is exhausted and no line wins.
    Doomed,
}

/// The memo key: per-unit `(health, fallen)`, the **canonicalized** partition, and the round.
///
/// `tempo` and `pending` are absent on purpose - both are re-derived by the round Reset, and we only ever
/// memoize at a **round** boundary. That is the product's own rule (the round is the one deadline) paying a
/// dividend: the key is small because the model has exactly one place where state closes.
type Key = (Vec<(u32, bool)>, Vec<u8>, usize);

/// **The doom oracle.** Holds the memo, so the first evaluation walks the tree and every later one is a lookup.
///
/// Budgeted and restartable: give it a node budget and it answers [`Verdict::Evaluating`] rather than lying.
/// **The one rule it must never break:** an aborted subtree is *not* memoized. A "no win found" that was really
/// "I gave up" must never be cached as `Doomed`. The oracle may be silent; it may never be wrong.
pub struct Oracle {
    memo: HashMap<Key, bool>,
    /// Cumulative positions evaluated across every walk - the cost report.
    nodes: u64,
    /// Positions evaluated in **this** walk. The budget bounds *this*, not the lifetime total - otherwise a
    /// resumed walk would re-spend its allowance on the nodes it is merely re-treading, and never get deeper.
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

    /// Positions actually evaluated (a memo hit costs nothing) - the cost report.
    pub fn nodes(&self) -> u64 {
        self.nodes
    }
    /// Distinct positions the memo holds - the memory an in-app oracle would carry.
    pub fn states(&self) -> usize {
        self.memo.len()
    }
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    /// **Allow the next walk `nodes` positions, and clear the abort flag** - the frame tick of a resumable
    /// oracle.
    ///
    /// This is what makes the oracle affordable *inside a frame*: ask a question with a small grant, and if it
    /// comes back [`Verdict::Evaluating`], grant more next frame and ask again. The **memo survives**, so each
    /// retry re-treads its settled positions for free and pushes the frontier deeper. It is a *restart*, not a
    /// resume - the walk begins again at the root - but every position already settled is a lookup, so the
    /// answer converges in a handful of frames, with no threads and no half-built state to get wrong.
    ///
    /// The budget bounds **this walk**, not the lifetime total. (It used to bound the total, which was a
    /// liveness bug: a resumed walk re-spent its whole allowance on the nodes it was merely re-treading, so
    /// with a small grant it could never settle a single subtree and never converged at all. It stayed
    /// *silent* rather than wrong - the safety invariant held - but it also never answered.)
    ///
    /// **Liveness is the caller's job.** A subtree only memoizes once it is fully explored, so a grant smaller
    /// than the work needed to settle *any* new subtree makes no progress at all, however many times it is
    /// repeated. A caller that grinds must therefore **escalate** its grant when a walk settles nothing - see
    /// `examples/region_board`'s `grind`.
    ///
    /// Safety is *not* the caller's job, and never depends on the grant: an aborted subtree is never memoized,
    /// so a starved oracle can only ever be **silent**, never wrong.
    pub fn grant(&mut self, nodes: u64) {
        self.walk = 0;
        self.budget = nodes;
        self.aborted = false;
    }

    /// The verdict for a position at the start of `round`.
    pub fn verdict(&mut self, board: &Board, round: usize) -> Verdict {
        let before = self.aborted;
        let win = self.winnable(board, round, None);
        self.judge(win, before)
    }

    /// Turn a search result into a verdict. A **win is a proof** (we hold a witness line), so it stands even if
    /// other branches were abandoned. A **loss is only a proof if the tree was exhausted** - otherwise all we
    /// learned is that we did not find a win *yet*, and the honest word for that is `Evaluating`.
    fn judge(&self, win: bool, before: bool) -> Verdict {
        match (win, self.aborted && !before) {
            (true, _) => Verdict::Winnable,
            (false, true) => Verdict::Evaluating,
            (false, false) => Verdict::Doomed,
        }
    }

    /// **"If this hero declares this, is the position still winnable?"** - the per-move verdict the UI charts.
    /// The rest of the party is free to play its best, and every later round is free, so this asks whether the
    /// choice *forecloses* the win - not whether it is optimal in isolation. That is the honest question: a
    /// player wants to know what kills them, not what is merely suboptimal.
    pub fn verdict_for(&mut self, board: &Board, round: usize, hero: usize, aim: Aim) -> Verdict {
        let before = self.aborted;
        let others: Vec<usize> = (0..board.units.len())
            .filter(|&i| board.units[i].side == Side::Party && !board.units[i].fallen && i != hero)
            .collect();
        let choices: Vec<Vec<Aim>> = others.iter().map(|&i| legal_aims(board, i)).collect();
        let foes = foe_aims(board);
        let mut win = false;
        for pick in 0..count(&choices) {
            let mut aims = assemble(board, &others, &choices, pick, &foes);
            aims[hero] = aim;
            let mut b = board.clone();
            play_round(&mut b, &aims);
            if self.winnable(&b, round + 1, None) {
                win = true;
                break;
            }
        }
        self.judge(win, before)
    }

    /// Can the party force a win from here? `fixed` pins the aims for the whole fight - the **control** the
    /// design is measured against (declare once at setup, never move again). `None` lets it re-declare each
    /// round.
    pub fn winnable(&mut self, board: &Board, round: usize, fixed: Option<&[Aim]>) -> bool {
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
            board.units.iter().map(|u| (u.health, u.fallen)).collect(),
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
        let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(board, i)).collect();
        let foes = foe_aims(board);

        // **Each node must judge its OWN subtree.** Stash the caller's abort flag and start clean, or this node
        // would inherit a *sibling's* give-up and mistake it for its own completeness.
        //
        // This is the bug that made a bounded oracle unsound. The old guard was `aborted && !before`, with
        // `before` read from the shared flag - so only the FIRST node to run out of budget in a walk was
        // protected. Every node visited after it saw `before == true`, computed `gave_up_here == false`, and
        // cached its incomplete "no win found" as a **proven Doomed**. A starved oracle would then hand back
        // confidently wrong answers - the one thing it may never do.
        let outer = self.aborted;
        self.aborted = false;

        let mut win = false;
        for pick in 0..count(&choices) {
            let aims = match fixed {
                Some(f) => {
                    let mut a = f.to_vec();
                    for (i, fa) in foes.iter().enumerate() {
                        if let Some(fa) = fa {
                            a[i] = *fa;
                        }
                    }
                    a
                }
                None => assemble(board, &heroes, &choices, pick, &foes),
            };
            let mut b = board.clone();
            play_round(&mut b, &aims);
            if self.winnable(&b, round + 1, fixed) {
                win = true;
                break;
            }
            if fixed.is_some() {
                break; // the control has exactly one declaration: itself
            }
        }

        let incomplete = self.aborted; // something under THIS node ran out of budget
        self.aborted = outer || incomplete; // restore the caller's flag, and propagate ours

        // Cache only what we can actually prove.
        //
        // A **win is a proof**: we hold a witness line, and abandoning the branches we never needed does not
        // weaken it. So a win is memoized even from an incomplete walk - which is also what lets the search
        // short-circuit without throwing away what it found.
        //
        // A **loss is a proof only if the tree was exhausted**. Otherwise all we learned is that we have not
        // found a win *yet*, and caching that as Doomed is precisely the lie this oracle must never tell.
        if win || !incomplete {
            self.memo.insert(key, win);
        }
        win
    }
}

fn count(choices: &[Vec<Aim>]) -> usize {
    choices.iter().map(|c| c.len()).product::<usize>().max(1)
}

/// The `pick`-th joint declaration over `who`, mixed-radix, with the foes' scripted orders folded in.
fn assemble(
    board: &Board,
    who: &[usize],
    choices: &[Vec<Aim>],
    pick: usize,
    foes: &[Option<Aim>],
) -> Vec<Aim> {
    let mut aims: Vec<Aim> = vec![Aim::WAIT; board.units.len()];
    for (k, &i) in who.iter().enumerate() {
        let radix: usize = choices[..k].iter().map(|c| c.len()).product();
        aims[i] = choices[k][(pick / radix.max(1)) % choices[k].len().max(1)];
    }
    for (i, a) in foes.iter().enumerate() {
        if let Some(a) = a {
            aims[i] = *a;
        }
    }
    aims
}

/// **The control:** the best *fixed* setup - declare aims once at the Marshal and never move again. This is the
/// honest control `v2_remarshal` insisted on: starting wrong and fixing it does not count, because the party
/// could simply have started right. The whole design is measured against this.
pub fn best_fixed(board: &Board, budget: u64) -> (bool, Oracle) {
    let heroes: Vec<usize> = (0..board.units.len())
        .filter(|&i| board.units[i].side == Side::Party && !board.units[i].fallen)
        .collect();
    let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(board, i)).collect();
    let foes = foe_aims(board);
    let mut o = Oracle::new(budget);
    for pick in 0..count(&choices) {
        let aims = assemble(board, &heroes, &choices, pick, &foes);
        if o.winnable(board, 0, Some(&aims)) {
            return (true, o);
        }
    }
    (false, o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use deckbound_content::rank::Intention as Rank;

    fn unit(name: &str, side: Side, stats: [u8; 5], melee: bool, ranged: bool) -> Combatant {
        Combatant::from_stats(name, side, Rank::Vanguard, stats, 0, melee, ranged)
    }

    /// A labelling is not a position: permuting the region ids must not change the state.
    #[test]
    fn region_labels_are_not_state() {
        assert_eq!(canonical(&[3, 3, 7, 9]), canonical(&[1, 1, 0, 5]));
        assert_eq!(canonical(&[0, 1, 0]), vec![0, 1, 0]);
        assert_ne!(canonical(&[0, 0, 1]), canonical(&[0, 1, 1]));
    }

    /// **The back is not a rank - it is a region with no enemy in it**, and it stops being the back the moment
    /// somebody walks in. This is the whole shielding model, and it is a fact about the board, not a decree.
    #[test]
    fn the_back_is_wherever_no_enemy_stands() {
        let mut b = Board::opening(vec![
            unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
            unit("Ogre", Side::Foe, [5, 5, 2, 2, 2], true, false),
        ]);
        assert!(b.is_safe(0, Side::Party), "no foe stands with the Marksman");
        b.regions[1] = 0; // the Ogre walks in
        assert!(
            !b.is_safe(0, Side::Party),
            "the back stopped being the back"
        );
    }

    // ---- the front line: the vanguard/rearguard, localized to a region ----------------------------------

    /// A three-body party: a wall at the front, a cannon at the back, and a foe.
    fn wall_and_cannon() -> (Board, Vec<Aim>) {
        let b = Board::opening(vec![
            unit("Bastion", Side::Party, [1, 3, 3, 1, 2], true, false), // 0 - the wall
            unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true), // 1 - the cannon
            unit("Ogre", Side::Foe, [5, 5, 2, 3, 2], true, false),      // 2 - the slipper
        ]);
        let aims = vec![
            Aim::new(Post::Front, Act::Wait),      // Bastion holds the line
            Aim::new(Post::Back, Act::Strike(2)),  // Marksman deals from behind it
            Aim::new(Post::Front, Act::Strike(1)), // the Ogre goes for the cannon
        ];
        (b, aims)
    }

    /// **The front screens the back - and the screen is a list of bodies, not a decree.**
    #[test]
    fn the_front_screens_the_back_until_it_does_not() {
        let (mut b, aims) = wall_and_cannon();
        assert_eq!(front_line(&b, &aims, 0, Side::Party), vec![0]);
        assert!(
            is_screened(&b, &aims, 1),
            "the cannon is behind a living wall"
        );

        b.units[0].fallen = true;
        assert!(
            front_line(&b, &aims, 0, Side::Party).is_empty(),
            "the wall fell"
        );
        assert!(
            !is_screened(&b, &aims, 1),
            "a back line with no front is simply exposed - no immunity survives the body that paid for it"
        );
    }

    /// **THE ONE THAT WAS BROKEN. The screen is a PRICE, not an immunity.**
    ///
    /// The previous cut redirected every aimed blow onto the screen, so a screened body could not be touched
    /// until its guard died. That is fiat, and it silently deleted the Outrider - the whole role whose purpose
    /// is to go *around* an intact line. Enough Tempo must always get through.
    ///
    /// Here the Ogre (Cadence 3) can afford to slip the Bastion (Cadence 1) and reach the Marksman behind it.
    #[test]
    fn enough_tempo_slips_the_screen_and_reaches_the_body_behind_it() {
        let (mut b, aims) = wall_and_cannon();
        let cannon_health = b.units[1].health;
        play_round(&mut b, &aims);
        assert!(
            b.units[1].health < cannon_health || b.units[1].fallen,
            "a high-tempo body must be able to buy its way past an intact front - force, not fiat"
        );
    }

    /// ...and the price is real: a slipper that **cannot** afford the front is **caught**, and its tempo goes to
    /// the body that caught it. The screen works; it is just not free.
    #[test]
    fn a_slipper_that_cannot_pay_is_caught_by_the_front() {
        let mut b = Board::opening(vec![
            unit("Bastion", Side::Party, [1, 4, 3, 3, 3], true, false), // a thick, high-tempo wall
            unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
            unit("Runt", Side::Foe, [3, 3, 1, 1, 1], true, false), // one card, one finesse: it cannot pay
        ]);
        let aims = vec![
            Aim::new(Post::Front, Act::Wait),
            Aim::new(Post::Back, Act::Strike(2)),
            Aim::new(Post::Front, Act::Strike(1)), // it goes for the cannon anyway
        ];
        let cannon_health = b.units[1].health;
        play_round(&mut b, &aims);
        assert_eq!(
            b.units[1].health, cannon_health,
            "it could not out-bid the wall, so it never reached the cannon"
        );
    }

    /// **An arrow cannot be aimed through a shield wall.** A ranged body that names a screened target does not
    /// get barred - it simply hits the front instead. Declaring is free, and it may fail.
    #[test]
    fn a_shot_at_a_screened_body_hits_the_screen_instead() {
        let b = Board::opening(vec![
            unit("Archer", Side::Party, [3, 3, 1, 2, 2], false, true), // 0
            unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),     // 1 - their front
            unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),     // 2 - their back
        ]);
        let aims = vec![
            Aim::new(Post::Front, Act::Strike(2)), // the Archer names the Mage...
            Aim::new(Post::Front, Act::Wait),
            Aim::new(Post::Back, Act::Wait),
        ];
        assert!(is_screened(&b, &aims, 2), "the Mage is behind the Wall");
        assert_eq!(
            attack_of(&b, &aims, 0),
            Some((0, 1)),
            "...and the shot lands on the Wall - not barred, just wasted on the body in the way"
        );

        // The declaration was legal all along: you do not know their formation when you commit to yours.
        assert!(legal_aims(&b, 0).contains(&Aim::new(Post::Front, Act::Strike(2))));
    }

    /// **A body that crosses arrives at the front.** You charged in; you are at the sharp end by definition -
    /// so a crossing act carries no post choice, which is also what halves the branching factor.
    #[test]
    fn a_crosser_arrives_at_the_front() {
        let b = Board::opening(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            unit("Ally", Side::Party, [1, 3, 3, 1, 2], true, false),
            unit("Ogre", Side::Foe, [5, 5, 2, 2, 2], true, false),
        ]);
        let charging: Vec<Aim> = legal_aims(&b, 0)
            .into_iter()
            .filter(|a| matches!(a.act, Act::Strike(2)))
            .collect();
        assert_eq!(
            charging,
            vec![Aim::new(Post::Front, Act::Strike(2))],
            "charging into their region offers exactly one post: the front"
        );

        // Staying put, on the other hand, is a real choice of post - there is a line to be part of.
        let waits: Vec<Aim> = legal_aims(&b, 0)
            .into_iter()
            .filter(|a| a.act == Act::Wait)
            .collect();
        assert_eq!(waits.len(), 2, "front or back, when you have company");
    }

    /// A **lone** body is exposed whatever it calls itself, so it is offered no post choice (spec 4.1: a choice
    /// with one legal option is not a choice).
    #[test]
    fn a_lone_body_has_no_line_to_hide_in() {
        let mut b = Board::opening(vec![
            unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
            unit("Ogre", Side::Foe, [5, 5, 2, 2, 2], true, false),
        ]);
        b.regions = vec![0, 1];
        let posts: Vec<Post> = legal_aims(&b, 0).into_iter().map(|a| a.post).collect();
        assert!(
            posts.iter().all(|&p| p == Post::Front),
            "alone is alone - there is no back to stand in"
        );
    }

    /// **An area strike nukes the whole region, both tiers** - it bypasses the screen entirely. That is the
    /// anti-cluster counter, and it is what stops a deep formation from being free.
    #[test]
    fn an_area_strike_reaches_the_back_line() {
        let mut b = Board::opening(vec![
            unit("Bombardier", Side::Party, [3, 3, 1, 1, 2], false, true).with_aoe(true),
            unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
            unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true),
        ]);
        b.regions = vec![0, 1, 1];
        let aims = vec![
            Aim::new(Post::Front, Act::Strike(1)),
            Aim::new(Post::Front, Act::Wait),
            Aim::new(Post::Back, Act::Wait),
        ];
        assert!(is_screened(&b, &aims, 2), "the Mage is behind the Wall");
        let mage = b.units[2].health;
        play_round(&mut b, &aims);
        assert!(
            b.units[2].health < mage || b.units[2].fallen,
            "a sweep cannot be screened - a bodyguard soaks an aimed blow but cannot cover an area"
        );
    }

    /// The fight terminates and someone wins - the model does not stall.
    #[test]
    fn a_fight_resolves() {
        let mut b = Board::opening(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [1, 2, 1, 1, 1], true, false),
        ]);
        for _ in 0..MAX_ROUNDS {
            if b.outcome().is_some() {
                break;
            }
            let aims = vec![
                Aim::new(Post::Front, Act::Strike(1)),
                Aim::new(Post::Front, Act::Strike(0)),
            ];
            play_round(&mut b, &aims);
        }
        assert_eq!(b.outcome(), Some(true), "the Raider wins");
    }

    // ---- the oracle ------------------------------------------------------------------------------------

    /// A **winnable but deep** board. It matters that this is `Winnable` and not shallow - a board that is
    /// Doomed at depth 1 settles for free and could never catch an oracle that gives up early and calls it
    /// Doomed.
    fn deep_board() -> Board {
        Board::opening(vec![
            unit("Raider", Side::Party, [7, 6, 1, 2, 2], true, false),
            unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
            unit("The Wall", Side::Foe, [1, 4, 9, 1, 2], true, false),
        ])
    }

    /// A board that really is lost - so "never wrong" is tested in *both* directions, and a permanently
    /// optimistic oracle could not pass by accident.
    fn hard_board() -> Board {
        Board::opening(vec![
            unit("Marksman", Side::Party, [1, 1, 1, 1, 1], false, true),
            unit("The Wall", Side::Foe, [9, 9, 9, 3, 3], true, false),
        ])
    }

    /// **SAFETY: a starved oracle is SILENT, never WRONG - at any grant, however cruel.**
    ///
    /// The invariant the whole in-app indicator rests on, and it must not depend on the *size* of the grant.
    /// Starve it one node at a time and it may say `Evaluating` forever - allowed, and honest - but it must
    /// never once answer `Doomed` or `Winnable` and disagree with the unbounded search. **A certainty indicator
    /// may be silent; it may never be wrong.**
    #[test]
    fn a_starved_oracle_is_silent_never_wrong() {
        for board in [deep_board(), hard_board()] {
            let truth = Oracle::new(u64::MAX).verdict(&board, 0);
            assert_ne!(
                truth,
                Verdict::Evaluating,
                "the control must actually settle"
            );

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

    /// **LIVENESS: an escalating grind converges on the unbounded verdict.**
    ///
    /// A subtree memoizes only once it is fully explored, so a grant too small to settle *any* new subtree can
    /// make no progress however often it is repeated. Escalation is the caller's job, and the policy that works
    /// is the blunt one: **double the grant on every `Evaluating`.**
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
            assert_eq!(
                got, truth,
                "an escalating grind must reach the unbounded verdict"
            );
        }
    }

    /// The same contract for the per-move chart: every row converges, and never lies on the way there.
    #[test]
    fn the_chart_converges_to_the_truth() {
        let board = deep_board();
        for aim in legal_aims(&board, 0) {
            let truth = Oracle::new(u64::MAX).verdict_for(&board, 0, 0, aim);
            let mut o = Oracle::new(0);
            let mut grant = 1u64;
            let mut got = Verdict::Evaluating;
            for _ in 0..64 {
                o.grant(grant);
                got = o.verdict_for(&board, 0, 0, aim);
                assert!(
                    got == Verdict::Evaluating || got == truth,
                    "{aim:?} answered {got:?} on the way to {truth:?}"
                );
                if got != Verdict::Evaluating {
                    break;
                }
                grant *= 2;
            }
            assert_eq!(got, truth, "{aim:?} never settled");
        }
    }
}
