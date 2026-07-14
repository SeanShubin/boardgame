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
//! **One declaration per unit, and the move follows from it** ([`Aim`]):
//! - [`Aim::Press`] - my violence is aimed at that enemy. Melee crosses to it; ranged stays and shoots.
//! - [`Aim::Defend`] - my body is between that ally and harm. I move to my ward if it is elsewhere.
//! - [`Aim::Withdraw`] - peel off alone to fresh ground.
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

/// A unit's whole declaration for the round. The **move follows from the aim** - one declaration, not two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Aim {
    /// Aim my violence at this enemy. A melee body crosses to it; a ranged body stays and shoots.
    Press(usize),
    /// Put my body between this ally and harm (a damage redirect). Move to my ward if it is elsewhere.
    Defend(usize),
    /// Peel off alone to fresh ground - the retreat.
    Withdraw,
}

impl Aim {
    pub fn label(self, units: &[Combatant]) -> String {
        match self {
            Aim::Press(t) => format!("Press {}", units[t].name),
            Aim::Defend(w) => format!("Defend {}", units[w].name),
            Aim::Withdraw => "Withdraw".to_string(),
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
        Board { units, regions }
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

/// The region unit `i` ends its move in, given its aim. `None` = it does not move.
pub fn destination(board: &Board, i: usize, aim: Aim) -> Option<u8> {
    let (units, regions) = (&board.units, &board.regions);
    match aim {
        // A ranged body needs no ground - an arrow does not walk. A melee body must close.
        Aim::Press(t) => (!units[i].ranged && regions[t] != regions[i]).then(|| regions[t]),
        Aim::Defend(w) => (regions[w] != regions[i]).then(|| regions[w]),
        Aim::Withdraw => {
            let alone = board.in_region(regions[i]).iter().all(|&j| j == i);
            if alone {
                return None; // you cannot withdraw from solitude
            }
            Some((0u8..).find(|r| !regions.contains(r)).unwrap_or(u8::MAX))
        }
    }
}

/// The legal aims for unit `i` - the count-adaptive candidate list (spec 4.1: a choice is presented only when
/// it has two or more legal options). **This is the branching factor the design lives or dies by**, so it is
/// the one place to look when a cost report comes back bad. Measured at about 7 per unit for a 4v4 - nothing.
///
/// **Press comes first, deliberately.** A reachability search short-circuits on the first winning line, so this
/// order decides which of several winning lines gets *shown*. Enumerating Defend first opened every transcript
/// with a four-way mutual-defend knot - legal, winning, and unreadable. Trying the attack first shows the line
/// a player would recognize. It changes no verdict, only which witness is printed.
pub fn legal_aims(board: &Board, i: usize) -> Vec<Aim> {
    let units = &board.units;
    let mut out = Vec::new();
    for (j, u) in units.iter().enumerate() {
        if !u.fallen && u.side != units[i].side {
            out.push(Aim::Press(j));
        }
    }
    for (j, u) in units.iter().enumerate() {
        if !u.fallen && u.side == units[i].side && j != i {
            out.push(Aim::Defend(j));
        }
    }
    out.push(Aim::Withdraw);
    out
}

/// **The head of the screen chain protecting `w`** - walk up the Defend edges (a living ally, in `w`'s region,
/// declared `Defend(w)`) until nobody is covering. A blow aimed at `w` lands *there* instead.
///
/// This one function is the entire back-access rule. There is no gate and no immunity: to reach the ward you
/// must kill the screen, and the blow that kills the screen is a blow you spent. `seen` breaks cycles - a
/// mutual-defend knot is legal and merely *expensive* (both bodies spend the fight screening each other and
/// nobody attacks), which is a trap the player is allowed to walk into, not a rule.
pub fn screen_head(board: &Board, aims: &[Aim], w: usize) -> usize {
    let (units, regions) = (&board.units, &board.regions);
    let mut at = w;
    let mut seen = vec![false; units.len()];
    loop {
        seen[at] = true;
        let cover = (0..units.len()).find(|&d| {
            !seen[d]
                && !units[d].fallen
                && units[d].side == units[at].side
                && regions[d] == regions[at]
                && aims[d] == Aim::Defend(at)
        });
        match cover {
            Some(d) => at = d,
            None => return at,
        }
    }
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
/// a minimax (spec 0.1 - creatures are an environment, not an opponent that searches back). Every foe presses
/// the living hero it can most cheaply finish.
pub fn foe_aims(board: &Board) -> Vec<Option<Aim>> {
    let units = &board.units;
    let prey = (0..units.len())
        .filter(|&j| units[j].side == Side::Party && !units[j].fallen)
        .min_by_key(|&j| (units[j].health, units[j].grit));
    units
        .iter()
        .map(|u| match (u.side, u.fallen, prey) {
            (Side::Foe, false, Some(p)) => Some(Aim::Press(p)),
            _ => None,
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
/// poured along an edge they can swing on. All aimed damage is redirected through the [`screen_head`]; the
/// `sweeps` are area edges that bypass it. One order-free, commit-based batch (a blow lands even if its striker
/// died in the same sub-phase).
fn strike_along(
    board: &mut Board,
    aims: &[Aim],
    contacts: &[Contact],
    sweeps: &[Contact],
    pour: bool,
) {
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
                target: screen_head(board, aims, t),
                cards: board.units[i].tempo,
            })
        })
        .collect();
    let all: Vec<Contact> = contacts.iter().chain(sweeps).copied().collect();
    combat::resolve_strike(&mut board.units, &all, &blows);
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged**. Only the schedule around
/// them is new. Area strikes split off and sweep their target's whole region.
fn exchange(board: &mut Board, aims: &[Aim], engagements: &[Engage], pour: bool) {
    let (sweeps, aimed): (Vec<Engage>, Vec<Engage>) = engagements
        .iter()
        .partition(|e| board.units[e.attacker].aoe);
    let extra = aoe_sweep(board, &sweeps);
    let reaching = combat::resolve_engage(&mut board.units, &aimed);
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| {
            let Some(cost) = combat::slip_cost(&board.units, &reaching, i) else {
                return Dodge::Stand; // nothing is reaching you
            };
            if board.units[i].fallen || cost > board.units[i].tempo {
                return Dodge::Stand; // you cannot afford it, so it is not on offer
            }
            // An edge you can swing along is worth more than an escape: let them come, and hit back.
            if reaching.iter().any(|c| {
                c.target == i
                    && board.regions[c.attacker] == board.regions[i]
                    && board.units[i].melee
            }) {
                return Dodge::Stand;
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
        .collect();
    let contacts = combat::resolve_evade(&mut board.units, &reaching, &dodges);
    strike_along(board, aims, &contacts, &extra, pour);
    combat::end_sub_phase(&mut board.units);
}

/// Who unit `i` can actually hit right now: its declared quarry if it can reach it (a **ranged** body shoots
/// into any region; a **melee** body reaches only its own), else whatever enemy is standing in front of it -
/// a body in your region did not ask your permission. That fallback is the product's existing mutual-melee
/// rule: *it did not choose the fight.*
fn press_target(board: &Board, aims: &[Aim], i: usize) -> Option<usize> {
    let (units, regions) = (&board.units, &board.regions);
    let can_reach = |t: usize| units[i].ranged || regions[t] == regions[i];
    if let Aim::Press(t) = aims[i]
        && !units[t].fallen
        && can_reach(t)
    {
        return Some(t);
    }
    (0..units.len()).find(|&j| {
        !units[j].fallen
            && units[j].side != units[i].side
            && regions[j] == regions[i]
            && units[i].melee
    })
}

/// Play **one whole round** from the declared aims: the Reset, then Cross -> Arrive -> Contact -> Breach.
/// Returns one [`SubPhaseLog`] per sub-phase, in order.
pub fn play_round(board: &mut Board, aims: &[Aim]) -> Vec<SubPhaseLog> {
    combat::refresh_round(&mut board.units);
    let mut logs = Vec::new();
    let living = |b: &Board| -> Vec<bool> { b.units.iter().map(|u| !u.fallen).collect() };

    // ---- Cross: ground you cross is ground you cross unscreened -----------------------------------------
    // Every living enemy in the region you LEAVE (you turn your back) and the region you ENTER (they watch you
    // come) reaches for you. ONE pile - they silence the same thing (your Arrival), so under the razor they
    // trade rather than sit in ordered boxes. This is Intercept + Volley, merged.
    let dests: Vec<Option<u8>> = (0..board.units.len())
        .map(|i| (!board.units[i].fallen).then(|| destination(board, i, aims[i]))?)
        .collect();

    let mut engagements: Vec<Engage> = Vec::new();
    for (i, dest) in dests.iter().enumerate() {
        let Some(d) = *dest else { continue };
        for e in 0..board.units.len() {
            if board.units[e].fallen
                || board.units[e].side == board.units[i].side
                || board.units[e].tempo == 0
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
    strike_along(board, aims, &contacts, &extra, SubPhase::Cross.pours());
    combat::end_sub_phase(&mut board.units);

    // Arrival is settled here: you got through if nothing reached you, or if you paid to break all of it.
    // Stand at the screen and you are **caught** - you stay where you were, and your aim is spent on the wall.
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

    // ---- Arrive: survivors land and strike. The raid pre-empts the melee. -------------------------------
    let before = living(board);
    let arrivals: Vec<Engage> = (0..board.units.len())
        .filter(|&i| arrived[i] && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| match aims[i] {
            Aim::Press(t) if !board.units[t].fallen && board.regions[t] == board.regions[i] => {
                Some(Engage {
                    attacker: i,
                    target: screen_head(board, aims, t),
                    cards: reach_cards(&board.units, i, t),
                })
            }
            _ => None,
        })
        .collect();
    exchange(board, aims, &arrivals, SubPhase::Arrive.pours());
    logs.push(SubPhaseLog {
        fallen: fell(&before, board),
        ..Default::default()
    });

    // ---- Contact: everyone co-located with an enemy trades ----------------------------------------------
    // ---- Breach: leftover tempo; redirects recomputed, so a dead screen no longer screens ---------------
    for phase in [SubPhase::Contact, SubPhase::Breach] {
        let before = living(board);
        let es: Vec<Engage> = (0..board.units.len())
            .filter(|&i| !board.units[i].fallen && board.units[i].tempo > 0)
            .filter_map(|i| {
                let t = press_target(board, aims, i)?;
                Some(Engage {
                    attacker: i,
                    target: screen_head(board, aims, t),
                    cards: reach_cards(&board.units, i, t),
                })
            })
            .collect();
        exchange(board, aims, &es, phase.pours());
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
    nodes: u64,
    budget: u64,
    aborted: bool,
}

impl Oracle {
    pub fn new(budget: u64) -> Self {
        Oracle {
            memo: HashMap::new(),
            nodes: 0,
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

    /// The verdict for a position at the start of `round`.
    pub fn verdict(&mut self, board: &Board, round: usize) -> Verdict {
        let before = self.aborted;
        let win = self.winnable(board, round, None);
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
        match (win, self.aborted && !before) {
            (true, _) => Verdict::Winnable,
            (false, true) => Verdict::Evaluating,
            (false, false) => Verdict::Doomed,
        }
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
        if self.nodes >= self.budget {
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

        let heroes: Vec<usize> = (0..board.units.len())
            .filter(|&i| board.units[i].side == Side::Party && !board.units[i].fallen)
            .collect();
        let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(board, i)).collect();
        let foes = foe_aims(board);
        let before = self.aborted;

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
        // Only ever cache an HONEST answer. A "no win found" that was really "I gave up" must never be
        // memoized as Doomed: the oracle may be silent, but it may never be wrong.
        let gave_up_here = self.aborted && !before;
        if !gave_up_here {
            self.memo.insert(key, win);
        }
        win
    }
}

fn count(choices: &[Vec<Aim>]) -> usize {
    choices.iter().map(|c| c.len()).product::<usize>().max(1)
}

/// The `pick`-th joint declaration over `who`, mixed-radix, with the foes' scripted aims folded in.
fn assemble(
    board: &Board,
    who: &[usize],
    choices: &[Vec<Aim>],
    pick: usize,
    foes: &[Option<Aim>],
) -> Vec<Aim> {
    let mut aims: Vec<Aim> = vec![Aim::Withdraw; board.units.len()];
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

    /// **Defend is a damage redirect, and it chains.** A blow aimed at the ward lands on the screen instead -
    /// and when the screen dies, the blow lands. Force, not fiat: there is no gate to be immune behind.
    #[test]
    fn the_screen_is_a_redirect_and_dies_like_anything_else() {
        let mut b = Board::opening(vec![
            unit("Marksman", Side::Party, [5, 2, 1, 2, 2], false, true),
            unit("Bastion", Side::Party, [1, 3, 3, 1, 2], true, false),
            unit("Ogre", Side::Foe, [5, 5, 2, 2, 2], true, false),
        ]);
        let aims = [Aim::Withdraw, Aim::Defend(0), Aim::Press(0)];
        assert_eq!(screen_head(&b, &aims, 0), 1, "the Bastion takes the blow");

        b.units[1].fallen = true;
        assert_eq!(
            screen_head(&b, &aims, 0),
            0,
            "the screen fell - now it lands on the ward"
        );
    }

    /// A mutual-defend knot is **legal and merely expensive**, never impassable - the cycle must terminate and
    /// must still leave somebody reachable, or force-not-fiat is broken.
    #[test]
    fn a_defend_cycle_terminates_and_grants_no_immunity() {
        let b = Board::opening(vec![
            unit("A", Side::Party, [3, 2, 1, 2, 2], true, false),
            unit("B", Side::Party, [3, 2, 1, 2, 2], true, false),
            unit("Foe", Side::Foe, [3, 2, 1, 2, 2], true, false),
        ]);
        let aims = [Aim::Defend(1), Aim::Defend(0), Aim::Press(0)];
        let head = screen_head(&b, &aims, 0);
        assert!(head == 0 || head == 1, "the walk terminated on a real body");
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
            let aims = vec![Aim::Press(1), Aim::Press(0)];
            play_round(&mut b, &aims);
        }
        assert_eq!(b.outcome(), Some(true), "the Raider wins");
    }
}
