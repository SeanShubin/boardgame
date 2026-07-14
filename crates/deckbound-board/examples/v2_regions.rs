//! **Does a PRICED move ever matter? The mirror of `v2_remarshal`.**
//!
//! `v2_remarshal` asked whether a mid-fight re-rank is ever *required* to win, against the honest control
//! (the **best fixed formation**, not a bad one). The answer was no, exhaustively - so re-Marshalling was cut
//! as decoration and the formation was frozen for the whole fight.
//!
//! That result is sound, and it is *why* this probe exists. Read it again:
//!
//!     re-ranking was worthless BECAUSE re-ranking was free.
//!
//! A repositioning that costs nothing and is offered every round can always be pre-empted by simply starting
//! in the right place. It can therefore never be *necessary* - which is exactly what the probe measured. It
//! did not show that position does not matter. It showed that **costless** position does not matter.
//!
//! So: re-ask the identical question, with the identical control, in a model where **moving costs**.
//!
//!     Does there exist a position where NO fixed formation wins, AND moving wins?
//!
//! - **Yes** -> movement is load-bearing. The regions/relations model buys something real, and the UI rewrite
//!   is justified. (needs-merge/regions-and-relations-combat.md)
//! - **No**  -> the model is DEAD, and we learned it for the price of one example program instead of an arena
//!   rewrite.
//!
//! It also reports the **cost** (nodes, memo, wall-clock) against the 24x baseline `v2_remarshal` measured,
//! and prints the **per-move verdict table** - every legal move with its winnable/evaluating/doomed verdict -
//! which is both the doom-oracle data the UI needs and the thing a player would actually read.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_regions`
//!
//! ---
//!
//! # The model under test (regions + relations)
//!
//! **Space is a partition.** Each unit has a region id. Same region = together (in contact, melee reaches).
//! Different regions = apart. There is no map: regions are *derived* from what people declare, and the ids are
//! canonicalized before they enter the memo key (a labelling is not a position).
//!
//! **Each unit declares one AIM, and the move follows from it** - this is the first simplification the probe
//! makes over the design doc, and it is a large branching win:
//!
//! - `Press(enemy)`   - my violence is aimed there. If it is in another region and I am **melee**, I cross to
//!                      it. If I am **ranged**, I stay and shoot (an arrow needs no ground).
//! - `Defend(ally)`   - my body is between you and them. If my ward is elsewhere, I move to it.
//! - `Withdraw`       - peel off alone to fresh ground (the retreat).
//!
//! **Defend is a damage REDIRECT** - the second, bigger simplification, and I think it is an improvement on
//! the doc. A blow aimed at W is redirected to W's living defender in W's region, and that chains. So the
//! "screen" is not a separate contest at all: it is bodyguarding, and it *is* the back-access rule. Kill the
//! Bastion and the blow lands on the Marksman. Force-not-fiat, one rule, no gate.
//!
//! **The one law:** *ground you cross is ground you cross unscreened.* A crosser is engaged by every living
//! enemy in the region it LEAVES (parting blows) and every living enemy in the region it ENTERS (they see it
//! coming). It may pay `slip_cost` to break all of it and arrive - or Stand, in which case it is **caught**:
//! it eats the blows and does **not** arrive. You tried to run past the wall; the wall caught you.
//!
//! **Four sub-phases**, each earning its boundary by exactly one silencing (the razor the product already
//! states at `combat.rs:447` - a sub-phase exists so a death in it can silence something later):
//!
//! | sub-phase   | what happens                                            | a death here silences   |
//! |-------------|---------------------------------------------------------|-------------------------|
//! | **Cross**   | parting blows + fire from the destination, in one pile   | the crosser's Arrival   |
//! | **Arrive**  | survivors land and strike                               | the victim's Contact    |
//! | **Contact** | everyone co-located with an enemy trades                | a screen -> ground opens |
//! | **Breach**  | leftover tempo, redirects recomputed (dead screens gone) | (last)                  |
//!
//! Damage closes at the **Round Reset** only - the product's existing rule, unchanged.
//!
//! # What this probe does NOT search (stated honestly)
//!
//! It searches the **aim layer exhaustively** and holds the **tempo allocation at greedy** for both sides
//! (the same greedy tension `battle.rs` uses: commit the fewest cards they cannot afford to slip; every card
//! saved becomes a blow). That is deliberate - it isolates the *positional* question by comparing like with
//! like, and it is what makes the probe finish. It means a "no" here is evidence, not proof: a fixed formation
//! might still lose under optimal allocation where it wins under greedy. A **"yes" is proof**, though, and
//! "yes" is the answer that costs money.
//!
//! Support is also omitted (no buffs) - it is the aim the solver would most like to drop, and dropping it here
//! keeps the branching honest about the *movement* question.

use std::collections::HashMap;
use std::time::Instant;

use deckbound_board::combat::{self, Blows, Combatant, Contact, Dodge, Engage, Side};
use deckbound_content::catalog::{self, Creature, Encounter};
use deckbound_content::rank::Intention as Rank;

/// The round cap - a fight not decided in five rounds is a draw, and a draw is not a win (spec 0.4).
const MAX_ROUNDS: usize = 5;

// ---- the declaration ------------------------------------------------------------------------------------

/// A unit's whole declaration for the round. The **move follows from the aim** - see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Aim {
    /// Aim my violence at this enemy. Melee crosses to it; ranged stays and shoots.
    Press(usize),
    /// Put my body between this ally and harm (a damage redirect). Move to it if it is elsewhere.
    Defend(usize),
    /// Peel off alone to fresh ground.
    Withdraw,
}

/// The region a unit will be in at the end of its move, given its aim. `None` = it does not move.
fn destination(units: &[Combatant], regions: &[u8], i: usize, aim: Aim) -> Option<u8> {
    match aim {
        Aim::Press(t) => {
            // A ranged body needs no ground: it shoots from where it stands. A melee body must close.
            if units[i].ranged || regions[t] == regions[i] {
                None
            } else {
                Some(regions[t])
            }
        }
        Aim::Defend(w) => (regions[w] != regions[i]).then_some(regions[w]),
        Aim::Withdraw => {
            // Fresh ground: the lowest id nobody is standing on. Only a *move* if we are not already alone.
            let alone = !regions
                .iter()
                .enumerate()
                .any(|(j, r)| j != i && *r == regions[i] && !units[j].fallen);
            if alone {
                return None;
            }
            Some((0u8..).find(|r| !regions.contains(r)).unwrap_or(u8::MAX))
        }
    }
}

/// The legal aims for unit `i` - the **count-adaptive** candidate list (spec 4.1: a choice is presented iff it
/// has >= 2 legal options). This is the branching factor the whole design lives or dies by, so it is the one
/// place to look when the cost report comes back bad.
///
/// **Press comes first**, deliberately. The search short-circuits on the first winning line, so the order here
/// decides which of several winning lines gets *shown*. Enumerating Defend first made every transcript open
/// with a four-way mutual-defend knot - a legal, winning, and utterly unreadable turtle. Trying the attack
/// first shows the line a player would actually recognize. It changes no verdict, only which witness we print.
fn legal_aims(units: &[Combatant], i: usize) -> Vec<Aim> {
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

// ---- the screen: Defend is a damage redirect ------------------------------------------------------------

/// The **head of the screen chain** protecting `w`: walk up the Defend edges (a living ally, in `w`'s region,
/// declared Defend(w)) until nobody is covering. A blow aimed at `w` lands *there* instead.
///
/// This one function is the entire back-access rule. There is no gate and no immunity: to reach the ward you
/// kill the screen, and the blow that kills the screen is the blow you spent. Cycles are broken by `seen` and
/// are merely expensive, never impassable - force-not-fiat holds.
fn screen_head(units: &[Combatant], regions: &[u8], aims: &[Aim], w: usize) -> usize {
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

// ---- greedy tempo allocation (held fixed for both sides, so the comparison is like-for-like) -------------

/// The tempo `defender` would need to slip a reach worth `bid` (the `combat::slip_cost` arithmetic, for a bid
/// not yet committed).
fn slip_price(bid: u32, f_def: u32) -> u32 {
    bid / f_def.max(1) + 1
}

/// The greedy reach: the fewest cards the target cannot afford to slip - so it lands for certain - else one
/// card and take the chance. Every card saved becomes a blow.
fn reach_cards(units: &[Combatant], a: usize, t: usize) -> u32 {
    if units[a].aoe {
        return 1; // an area strike forms no contact and cannot be slipped: one card, no more.
    }
    (1..=units[a].tempo)
        .find(|&c| slip_price(c * units[a].finesse.max(1), units[t].finesse) > units[t].tempo)
        .unwrap_or(1)
}

/// The foe script: a fixed, deterministic policy (so this stays a single-agent reachability search, spec 0.1).
/// Every foe Presses the living hero it can most cheaply finish - lowest health, then lowest grit.
fn foe_aims(units: &[Combatant]) -> Vec<Option<Aim>> {
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

// ---- one round -----------------------------------------------------------------------------------------

/// Play a whole round from the declared aims: refresh, then Cross -> Arrive -> Contact -> Breach. Mutates
/// `units` and `regions` in place. Returns the sub-phase transcript lines when `log` is set.
fn play_round(
    units: &mut [Combatant],
    regions: &mut Vec<u8>,
    aims: &[Aim],
    log: &mut Option<&mut Vec<String>>,
) {
    combat::refresh_round(units);

    // Who is crossing, and to where. A body already caught in a melee still pays to leave it.
    let dests: Vec<Option<u8>> = (0..units.len())
        .map(|i| {
            if units[i].fallen {
                None
            } else {
                destination(units, regions, i, aims[i])
            }
        })
        .collect();

    // ---- 1. CROSS: ground you cross is ground you cross unscreened --------------------------------------
    // Every living enemy in the region you LEAVE (parting blows) and the region you ENTER (they see you
    // coming) reaches for you. One pile - they silence the same thing (your Arrival), so under the razor they
    // trade rather than sit in ordered boxes. This is Intercept + Volley, merged.
    let mut engagements: Vec<Engage> = Vec::new();
    for (i, dest) in dests.iter().enumerate() {
        let Some(d) = *dest else { continue };
        for e in 0..units.len() {
            if units[e].fallen || units[e].side == units[i].side || units[e].tempo == 0 {
                continue;
            }
            if regions[e] == regions[i] || regions[e] == d {
                let cards = reach_cards(units, e, i);
                engagements.push(Engage {
                    attacker: e,
                    target: i,
                    cards,
                });
            }
        }
    }
    let (sweeps, aimed): (Vec<Engage>, Vec<Engage>) =
        engagements.iter().partition(|e| units[e.attacker].aoe);
    let extra = aoe_sweep(units, regions, &sweeps);
    let reaching = combat::resolve_engage(units, &aimed);

    // The crosser answers: pay in full and get through, or Stand - and be CAUGHT. Greedy: slip if it can
    // afford it (getting through is the whole point of moving), else stand and eat it.
    let dodges: Vec<Dodge> = (0..units.len())
        .map(|i| match combat::slip_cost(units, &reaching, i) {
            Some(cost) if cost <= units[i].tempo && dests[i].is_some() => Dodge::Slip,
            _ => Dodge::Stand,
        })
        .collect();
    let slipped: Vec<bool> = (0..units.len())
        .map(|i| dodges[i] == Dodge::Slip && combat::slip_cost(units, &reaching, i).is_some())
        .collect();
    let contacts = combat::resolve_evade(units, &reaching, &dodges);
    strike_along(units, regions, aims, &contacts, &extra, false); // Cross: a snap shot, not your pool
    combat::end_sub_phase(units);

    // Arrival is settled here: you got through if nothing reached you, or if you paid to break all of it.
    // Stand at the screen and you are caught - you stay where you were, and your aim is spent on the wall.
    let mut arrived = vec![false; units.len()];
    for (i, dest) in dests.iter().enumerate() {
        let Some(d) = *dest else { continue };
        if units[i].fallen {
            continue;
        }
        let was_reached = reaching.iter().any(|c| c.target == i);
        if !was_reached || slipped[i] {
            regions[i] = d;
            arrived[i] = true;
        } else if let Some(l) = log.as_deref_mut() {
            l.push(format!(
                "    {} is CAUGHT crossing - it stays put",
                units[i].name
            ));
        }
    }
    if let Some(l) = log.as_deref_mut() {
        l.push(format!("  Cross:   {}", board_line(units, regions, aims)));
    }

    // ---- 2. ARRIVE: survivors land and strike (the raid pre-empts the melee) ---------------------------
    let arrivals: Vec<Engage> = (0..units.len())
        .filter(|&i| arrived[i] && !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| match aims[i] {
            Aim::Press(t) if !units[t].fallen && regions[t] == regions[i] => Some(Engage {
                attacker: i,
                target: screen_head(units, regions, aims, t),
                cards: reach_cards(units, i, t),
            }),
            _ => None,
        })
        .collect();
    resolve_exchange(units, regions, aims, &arrivals, false); // Arrive: the raid is one blow
    if let Some(l) = log.as_deref_mut() {
        l.push(format!("  Arrive:  {}", board_line(units, regions, aims)));
    }

    // ---- 3. CONTACT: everyone co-located with an enemy trades ------------------------------------------
    let melee: Vec<Engage> = (0..units.len())
        .filter(|&i| !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| {
            let t = press_target(units, regions, aims, i, true)?;
            Some(Engage {
                attacker: i,
                target: screen_head(units, regions, aims, t),
                cards: reach_cards(units, i, t),
            })
        })
        .collect();
    resolve_exchange(units, regions, aims, &melee, true); // Contact: now you are in it - pour
    if let Some(l) = log.as_deref_mut() {
        l.push(format!("  Contact: {}", board_line(units, regions, aims)));
    }

    // ---- 4. BREACH: leftover tempo, redirects recomputed - a dead screen no longer screens --------------
    let late: Vec<Engage> = (0..units.len())
        .filter(|&i| !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| {
            let t = press_target(units, regions, aims, i, false)?;
            Some(Engage {
                attacker: i,
                target: screen_head(units, regions, aims, t),
                cards: reach_cards(units, i, t),
            })
        })
        .collect();
    resolve_exchange(units, regions, aims, &late, true); // Breach: spend what is left
    if let Some(l) = log.as_deref_mut() {
        l.push(format!("  Breach:  {}", board_line(units, regions, aims)));
    }
}

/// Who unit `i` can actually hit right now. A **ranged** body shoots into any region. A **melee** body reaches
/// only its own region. `fresh_only` restricts to bodies that have not already had their swing this round
/// (the Contact pass); the Breach pass takes anyone left with tempo.
fn press_target(
    units: &[Combatant],
    regions: &[u8],
    aims: &[Aim],
    i: usize,
    _fresh_only: bool,
) -> Option<usize> {
    // Your declared aim first - that is what you committed to.
    let declared = match aims[i] {
        Aim::Press(t) if !units[t].fallen => Some(t),
        _ => None,
    };
    let can_reach = |t: usize| units[i].ranged || regions[t] == regions[i];
    if let Some(t) = declared
        && can_reach(t)
    {
        return Some(t);
    }
    // Otherwise you fight what is in front of you - a body in your region did not ask your permission. This
    // is the product's existing mutual-melee rule (`combat.rs`: "it did not choose the fight").
    (0..units.len()).find(|&j| {
        !units[j].fallen
            && units[j].side != units[i].side
            && regions[j] == regions[i]
            && units[i].melee
    })
}

/// **The area strike, region-wide.** This is the mechanic the regions model unlocks: once "who is standing
/// together" is a first-class fact on the board, an area strike stops being a single-target special case and
/// becomes what it always should have been - *it hits the whole knot.*
///
/// One tempo card. Every living enemy in the target's region, at **full Might**, **unevadable** (it forms no
/// slippable contact), and it **bypasses the screen** - a bodyguard soaks an aimed blow but cannot cover an
/// area. That last clause is the anti-cluster counter, and it is what prices the whole Defend mechanic: pile
/// bodies behind a screen and you become a *target*. Concentration and coverage now genuinely trade against
/// each other, decided by a positional fact the player controls.
///
/// **Deliberately NOT the product's "one sweep clears the whole pack" rule** (`combat.rs:349`). Once the sweep
/// is region-wide, auto-clearing every pack in the region is absurd: the first version of this probe had the
/// Bombardier's Salvo delete *both* Swarms - sixteen bodies - for one tempo card, and seven of eight encounters
/// collapsed to a round-one wipe. So a horde takes the sweep like anything else: penetrating Might spills body
/// to body (`apply` already does this), felling Might-many. Coverage, not annihilation.
///
/// That over-correction is itself worth recording: **region-AoE is powerful enough that it needs a brake, and
/// the brake belongs in the horde rule, not in the region rule.**
fn aoe_sweep(units: &mut [Combatant], regions: &[u8], sweeps: &[Engage]) -> Vec<Contact> {
    let mut extra = Vec::new();
    for e in sweeps {
        if units[e.attacker].fallen || units[e.attacker].tempo == 0 {
            continue;
        }
        units[e.attacker].tempo -= 1;
        let (side, region) = (units[e.attacker].side, regions[e.target]);
        for j in 0..units.len() {
            if units[j].fallen || units[j].side == side || regions[j] != region {
                continue;
            }
            // A damage-only edge: no bid, so it cannot be slipped, and nobody answers along it.
            extra.push(Contact {
                attacker: e.attacker,
                target: j,
                bid: 0,
            });
        }
    }
    extra
}

/// One Engage -> Evade -> Strike exchange (the inner three, unchanged from the product) over the given
/// engagements, then the sub-phase boundary. Area strikes split off and sweep their target's whole region.
fn resolve_exchange(
    units: &mut [Combatant],
    regions: &[u8],
    aims: &[Aim],
    engagements: &[Engage],
    pour: bool,
) {
    let (sweeps, aimed): (Vec<Engage>, Vec<Engage>) =
        engagements.iter().partition(|e| units[e.attacker].aoe);
    let extra = aoe_sweep(units, regions, &sweeps);
    let reaching = combat::resolve_engage(units, &aimed);
    let dodges: Vec<Dodge> = (0..units.len())
        .map(|i| {
            // Stand if you can answer along the edge (a swing is worth more than an escape); else slip if you
            // can afford it and the blow actually threatens you. The `battle.rs` greedy, verbatim in spirit.
            let Some(cost) = combat::slip_cost(units, &reaching, i) else {
                return Dodge::Stand;
            };
            if units[i].fallen || cost > units[i].tempo {
                return Dodge::Stand;
            }
            if reaching
                .iter()
                .any(|c| c.target == i && regions[c.attacker] == regions[i] && units[i].melee)
            {
                return Dodge::Stand; // an edge you can swing on is worth more than an escape
            }
            let worst = reaching
                .iter()
                .filter(|c| c.target == i)
                .map(|c| units[c.attacker].might)
                .max()
                .unwrap_or(0);
            if worst >= units[i].grit.max(1) {
                Dodge::Slip
            } else {
                Dodge::Stand
            }
        })
        .collect();
    let contacts = combat::resolve_evade(units, &reaching, &dodges);
    strike_along(units, regions, aims, &contacts, &extra, pour);
    combat::end_sub_phase(units);
}

/// The Strike step: each contact's opening blow, plus everyone's leftover tempo poured along an edge they can
/// swing on - all redirected through the screen chain, and applied as one order-free batch.
///
/// `sweeps` are the damage-only area edges: they land their Might but nobody *answers* along them and their
/// striker cannot pour extra tempo into them (coverage bought at the price of concentration).
///
/// `pour` is the sub-phase's answer to *"may I spend my whole pool here?"* and it is what keeps the four
/// sub-phases from collapsing into one. **Cross and Arrive do not pour**: you loose ONE snap shot at a body
/// running past you (the opening blow the reach already paid for) - you cannot stand in the open whaling on
/// someone who is not stopping. **Contact and Breach do pour**: now you are in a melee, and the pool is yours
/// to spend.
///
/// The first version of this probe poured everywhere, and the entire fight resolved in Cross with three dead
/// sub-phases behind it. The pre-empt is a *snap*, not a round.
fn strike_along(
    units: &mut [Combatant],
    regions: &[u8],
    aims: &[Aim],
    contacts: &[Contact],
    sweeps: &[Contact],
    pour: bool,
) {
    let blows: Vec<Blows> = (0..units.len())
        .filter(|&i| pour && !units[i].fallen && units[i].tempo > 0)
        .filter_map(|i| {
            // Whoever you are in contact with: the body you reached, or - on a melee edge - the body that
            // reached you. It came to you; you may answer.
            let t = contacts
                .iter()
                .find(|c| c.attacker == i)
                .map(|c| c.target)
                .or_else(|| {
                    contacts
                        .iter()
                        .find(|c| {
                            c.target == i && units[i].melee && regions[c.attacker] == regions[i]
                        })
                        .map(|c| c.attacker)
                })?;
            Some(Blows {
                unit: i,
                target: screen_head(units, regions, aims, t),
                cards: units[i].tempo,
            })
        })
        .collect();
    // The area edges land alongside the aimed ones, in the same order-free batch.
    let all: Vec<Contact> = contacts.iter().chain(sweeps).copied().collect();
    combat::resolve_strike(units, &all, &blows);
}

// ---- the search ----------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Winnable,
    Doomed,
    Evaluating,
}

/// The memo key. Regions are **canonicalized** (relabelled in first-appearance order) before they go in, so a
/// labelling is not a position - `{A:Bastion, B:Marksman}` and `{B:Bastion, A:Marksman}` are the same state.
/// That canonicalization is what makes the partition a *cheaper* key than a rank assignment
/// (`Bell(n) < 3^n` for every n <= 8), and it is the single most important tractability move in the design.
///
/// `pending` is absent on purpose: it is zeroed by `refresh_round`, and we only ever memoize at a **round**
/// boundary - which is the product's own rule that the round is the one deadline (`combat.rs:464`).
type Key = (Vec<(u32, bool)>, Vec<u8>, usize);

fn canonical(regions: &[u8]) -> Vec<u8> {
    let mut map: HashMap<u8, u8> = HashMap::new();
    regions
        .iter()
        .map(|r| {
            let next = map.len() as u8;
            *map.entry(*r).or_insert(next)
        })
        .collect()
}

fn key_of(units: &[Combatant], regions: &[u8], round: usize) -> Key {
    (
        units.iter().map(|u| (u.health, u.fallen)).collect(),
        canonical(regions),
        round,
    )
}

struct Search {
    memo: HashMap<Key, bool>,
    nodes: u64,
    budget: u64,
    aborted: bool,
}

impl Search {
    fn new(budget: u64) -> Self {
        Search {
            memo: HashMap::new(),
            nodes: 0,
            budget,
            aborted: false,
        }
    }

    /// Can the party force a win from here? `fixed` pins the aims for the whole fight (the CONTROL: declare
    /// once, never move again); `None` lets the party re-declare every round (the TREATMENT).
    ///
    /// The one rule this must never break: an **aborted subtree is not memoized**. A "no win found" that was
    /// really "I gave up" must never be cached as Doomed - the oracle may be silent, but it may never be wrong.
    fn winnable(
        &mut self,
        units: &[Combatant],
        regions: &[u8],
        round: usize,
        fixed: Option<&[Aim]>,
    ) -> bool {
        if !units.iter().any(|u| u.side == Side::Foe && !u.fallen) {
            return true;
        }
        if !units.iter().any(|u| u.side == Side::Party && !u.fallen) {
            return false;
        }
        if round >= MAX_ROUNDS {
            return false; // a draw at the cap is not a win
        }
        if self.nodes >= self.budget {
            self.aborted = true;
            return false;
        }
        let key = key_of(units, regions, round);
        if let Some(&v) = self.memo.get(&key) {
            return v;
        }
        self.nodes += 1;

        let heroes: Vec<usize> = (0..units.len())
            .filter(|&i| units[i].side == Side::Party && !units[i].fallen)
            .collect();
        let foes = foe_aims(units);

        // The party's joint declaration: every combination of legal aims over the living heroes. This is the
        // branching factor the design lives or dies by - it is measured, not assumed.
        let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(units, i)).collect();
        let total: usize = choices.iter().map(|c| c.len()).product::<usize>().max(1);

        let mut win = false;
        let before_abort = self.aborted;
        for pick in 0..total {
            let mut aims: Vec<Aim> = vec![Aim::Withdraw; units.len()];
            for (k, &i) in heroes.iter().enumerate() {
                aims[i] = match fixed {
                    Some(f) => f[i], // the control: the aim you declared at setup, held all fight
                    None => {
                        let n = choices[k].len();
                        let idx = (pick / choices[..k].iter().map(|c| c.len()).product::<usize>())
                            % n.max(1);
                        choices[k][idx]
                    }
                };
            }
            for (i, a) in foes.iter().enumerate() {
                if let Some(a) = a {
                    aims[i] = *a;
                }
            }

            let mut u = units.to_vec();
            let mut r = regions.to_vec();
            play_round(&mut u, &mut r, &aims, &mut None);
            if self.winnable(&u, &r, round + 1, fixed) {
                win = true;
                break;
            }
            if fixed.is_some() {
                break; // the control has exactly one declaration: itself
            }
        }
        // Only cache an honest answer.
        if !(self.aborted && !before_abort) {
            self.memo.insert(key, win);
        }
        win
    }
}

/// The CONTROL: the best **fixed** setup - declare aims once at the Marshal and never move again. This is the
/// honest control `v2_remarshal` insisted on: starting wrong and fixing it does not count, because the party
/// could simply have started right.
fn best_fixed(units: &[Combatant], regions: &[u8], budget: u64) -> (bool, Search) {
    let heroes: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen)
        .collect();
    let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(units, i)).collect();
    let total: usize = choices.iter().map(|c| c.len()).product::<usize>().max(1);
    let mut s = Search::new(budget);
    for pick in 0..total {
        let mut aims: Vec<Aim> = vec![Aim::Withdraw; units.len()];
        for (k, &i) in heroes.iter().enumerate() {
            let n = choices[k].len();
            let idx = (pick / choices[..k].iter().map(|c| c.len()).product::<usize>()) % n.max(1);
            aims[i] = choices[k][idx];
        }
        if s.winnable(units, regions, 0, Some(&aims)) {
            return (true, s);
        }
    }
    (false, s)
}

// ---- reporting -----------------------------------------------------------------------------------------

fn board_line(units: &[Combatant], regions: &[u8], _aims: &[Aim]) -> String {
    let mut by: Vec<(u8, Vec<String>)> = Vec::new();
    for (i, u) in units.iter().enumerate() {
        if u.fallen {
            continue;
        }
        let tag = format!(
            "{}{}({})",
            if u.side == Side::Party { "" } else { "*" },
            u.name,
            u.health
        );
        match by.iter_mut().find(|(r, _)| *r == regions[i]) {
            Some((_, v)) => v.push(tag),
            None => by.push((regions[i], vec![tag])),
        }
    }
    by.sort_by_key(|(r, _)| *r);
    by.iter()
        .map(|(r, v)| format!("[{}: {}]", (b'A' + r) as char, v.join(" ")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn label(units: &[Combatant], a: Aim) -> String {
    match a {
        Aim::Press(t) => format!("Press {}", units[t].name),
        Aim::Defend(w) => format!("Defend {}", units[w].name),
        Aim::Withdraw => "Withdraw".to_string(),
    }
}

fn kit_unit((name, stats, ability): (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}

fn creature_unit(c: &Creature) -> Combatant {
    Combatant::from_stats(
        c.name,
        Side::Foe,
        Rank::Vanguard,
        c.stats,
        0,
        c.melee,
        c.ranged,
    )
    .with_aoe(c.aoe)
    .as_horde(c.horde)
}

fn setup(e: &Encounter) -> (Vec<Combatant>, Vec<u8>) {
    let mut units: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit_unit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            units.push(creature_unit(c));
        }
    }
    // The opening position: the two sides start APART - which is what makes round 1 a mass crossing, and is
    // exactly why the current schedule's fiction works in round 1 and nowhere else.
    let regions = units
        .iter()
        .map(|u| if u.side == Side::Party { 0u8 } else { 1u8 })
        .collect();
    (units, regions)
}

const BUDGET: u64 = 4_000_000;

fn main() {
    println!("v2_regions - does a PRICED move ever turn a loss into a win?");
    println!("the mirror of v2_remarshal, which proved a FREE move never does.\n");

    let mut rescued = Vec::new();
    let mut totals = (0u64, 0usize, 0u128);

    for e in catalog::ENCOUNTERS.iter() {
        let (units, regions) = setup(e);

        let t0 = Instant::now();
        let (fixed_wins, fs) = best_fixed(&units, &regions, BUDGET);
        let fixed_ms = t0.elapsed().as_millis();

        let t1 = Instant::now();
        let mut ms = Search::new(BUDGET);
        let move_wins = ms.winnable(&units, &regions, 0, None);
        let move_ms = t1.elapsed().as_millis();

        totals.0 += ms.nodes;
        totals.1 = totals.1.max(ms.memo.len());
        totals.2 += move_ms;

        let verdict = |w: bool, s: &Search| match (w, s.aborted) {
            (true, _) => "WINNABLE",
            (false, true) => "evaluating (budget)",
            (false, false) => "DOOMED",
        };

        println!("{} - {}", e.location, e.title);
        println!(
            "   fixed setup, never moves : {:<20} ({} nodes, {} memo, {} ms)",
            verdict(fixed_wins, &fs),
            fs.nodes,
            fs.memo.len(),
            fixed_ms
        );
        println!(
            "   re-declare every round   : {:<20} ({} nodes, {} memo, {} ms)",
            verdict(move_wins, &ms),
            ms.nodes,
            ms.memo.len(),
            move_ms
        );

        // The whole question, in one line.
        if move_wins && !fixed_wins && !fs.aborted {
            println!("   >>> MOVEMENT IS LOAD-BEARING HERE: no fixed setup wins, and moving does.");
            rescued.push(e.location);
        }
        println!();
    }

    println!("----------------------------------------------------------------");
    if rescued.is_empty() {
        println!("VERDICT: movement is DECORATION. No encounter is rescued by moving that a");
        println!("         fixed setup could not already win. Same result v2_remarshal got for");
        println!("         free re-ranking - pricing the move did NOT make it matter.");
        println!(
            "         The regions/relations model is not paying for itself. Do not build the UI."
        );
    } else {
        println!(
            "VERDICT: movement is LOAD-BEARING. {} encounter(s) are winnable ONLY by",
            rescued.len()
        );
        println!("         moving, and unwinnable from every fixed setup:");
        for r in &rescued {
            println!("           - {r}");
        }
        println!("         This is what v2_remarshal could not find with a FREE move. Pricing the");
        println!("         move created a decision that did not exist before.");
    }
    println!();
    println!(
        "COST (the 24x baseline from v2_remarshal is what to beat):\n  {} nodes total, {} states in the worst memo, {} ms total",
        totals.0, totals.1, totals.2
    );

    // ---- the transcript: can you READ the fight at round 4? --------------------------------------------
    // This is the judgment the numbers cannot give. Play one party encounter with a plausible line and print
    // the board after every sub-phase. If the fiction is coherent at round 4 - if you can look at the regions
    // and say what is happening and why - the metaphor survived. If it reads as noise, it did not.
    println!("\n----------------------------------------------------------------");
    println!("TRANSCRIPT - the board after every sub-phase. `*` = foe, `(n)` = health.");
    println!("Read it and ask: at round 4, can you still say what is happening and why?\n");

    let e = catalog::ENCOUNTERS
        .iter()
        .find(|e| e.location == "Greywater Ford")
        .expect("Greywater Ford");
    let (mut units, mut regions) = setup(e);
    println!(
        "{} - {}\n  start:   {}\n",
        e.location,
        e.title,
        board_line(&units, &regions, &[])
    );

    for round in 0..MAX_ROUNDS {
        if !units.iter().any(|u| u.side == Side::Foe && !u.fallen)
            || !units.iter().any(|u| u.side == Side::Party && !u.fallen)
        {
            break;
        }
        // The party plays the first line the oracle certifies as still-winnable; if none is, it plays greedy.
        let aims = certified_line(&units, &regions, round);
        println!("Round {}:", round + 1);
        for i in 0..units.len() {
            if units[i].side == Side::Party && !units[i].fallen {
                println!("    {:<12} {}", units[i].name, label(&units, aims[i]));
            }
        }
        let mut lines = Vec::new();
        let mut log = Some(&mut lines);
        play_round(&mut units, &mut regions, &aims, &mut log);
        for l in lines {
            println!("{l}");
        }
        println!();
    }
    let won = !units.iter().any(|u| u.side == Side::Foe && !u.fallen);
    println!(
        "  result: {}",
        if won {
            "party wins"
        } else {
            "party does not win"
        }
    );

    // ---- the per-move verdict table: exactly the doom-oracle data the UI has to surface ----------------
    println!("\n----------------------------------------------------------------");
    println!("PER-MOVE VERDICT TABLE (the doom oracle, as the UI would chart it)");
    println!("The opening position of each party encounter: for each hero, each move it could");
    println!("make, and whether the position is still winnable if it makes it.\n");

    for e in catalog::ENCOUNTERS.iter().filter(|e| e.party) {
        let (units, regions) = setup(e);
        println!("{} - {}", e.location, e.title);
        println!("  board: {}", board_line(&units, &regions, &[]));
        for i in 0..units.len() {
            if units[i].side != Side::Party {
                continue;
            }
            println!("  {}:", units[i].name);
            for a in legal_aims(&units, i) {
                // Pin this one hero's opening aim; let the rest of the party play its best.
                let mut s = Search::new(BUDGET / 8);
                let win = opening_with(&mut s, &units, &regions, i, a);
                let v = match (win, s.aborted) {
                    (true, _) => Verdict::Winnable,
                    (false, true) => Verdict::Evaluating,
                    (false, false) => Verdict::Doomed,
                };
                println!("      {:<22} {:?}", label(&units, a), v);
            }
        }
        println!();
    }
}

/// The line a player following the doom oracle would actually take: the first joint declaration the oracle
/// still certifies as winnable. Falls back to the first legal declaration if the position is already lost -
/// which is itself the honest thing to show (a doomed board still has to be played out).
fn certified_line(units: &[Combatant], regions: &[u8], round: usize) -> Vec<Aim> {
    let heroes: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen)
        .collect();
    let choices: Vec<Vec<Aim>> = heroes.iter().map(|&i| legal_aims(units, i)).collect();
    let total: usize = choices.iter().map(|c| c.len()).product::<usize>().max(1);
    let foes = foe_aims(units);
    let build = |pick: usize| {
        let mut aims: Vec<Aim> = vec![Aim::Withdraw; units.len()];
        for (k, &i) in heroes.iter().enumerate() {
            let n = choices[k].len();
            let idx = (pick / choices[..k].iter().map(|c| c.len()).product::<usize>()) % n.max(1);
            aims[i] = choices[k][idx];
        }
        for (i, a) in foes.iter().enumerate() {
            if let Some(a) = a {
                aims[i] = *a;
            }
        }
        aims
    };
    let mut s = Search::new(BUDGET);
    for pick in 0..total {
        let aims = build(pick);
        let (mut u, mut r) = (units.to_vec(), regions.to_vec());
        play_round(&mut u, &mut r, &aims, &mut None);
        if s.winnable(&u, &r, round + 1, None) {
            return aims;
        }
    }
    build(0)
}

/// "If this hero opens with this aim, is the position still winnable?" - the party's other heroes are free to
/// play their best, and every later round is free. This is the honest per-choice verdict: it asks whether the
/// choice *forecloses* the win, not whether it is optimal in isolation.
fn opening_with(
    s: &mut Search,
    units: &[Combatant],
    regions: &[u8],
    hero: usize,
    aim: Aim,
) -> bool {
    let others: Vec<usize> = (0..units.len())
        .filter(|&i| units[i].side == Side::Party && !units[i].fallen && i != hero)
        .collect();
    let choices: Vec<Vec<Aim>> = others.iter().map(|&i| legal_aims(units, i)).collect();
    let total: usize = choices.iter().map(|c| c.len()).product::<usize>().max(1);
    let foes = foe_aims(units);

    for pick in 0..total {
        let mut aims: Vec<Aim> = vec![Aim::Withdraw; units.len()];
        aims[hero] = aim;
        for (k, &i) in others.iter().enumerate() {
            let n = choices[k].len();
            let idx = (pick / choices[..k].iter().map(|c| c.len()).product::<usize>()) % n.max(1);
            aims[i] = choices[k][idx];
        }
        for (i, a) in foes.iter().enumerate() {
            if let Some(a) = a {
                aims[i] = *a;
            }
        }
        let mut u = units.to_vec();
        let mut r = regions.to_vec();
        play_round(&mut u, &mut r, &aims, &mut None);
        if s.winnable(&u, &r, 1, None) {
            return true;
        }
    }
    false
}
