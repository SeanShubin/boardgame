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
    Combatant, Contact, Dodge, Engage, Side, can_answer, end_sub_phase, refresh_round,
    resolve_evade, resolve_evade_pooled, slip_cost,
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
/// **How a crosser answers the VANGUARD** (the interception - the first of the two independent crossings, and the
/// only one that decides through-vs-stay). Read off *where you put your tempo* around the catchers' strikes:
/// before them (Evade = slip the line), never (Push), or after them (Abort). Abort carries the after-spend
/// explicitly: the strike-back allocation, a list of `(catcher, strikes)`, one tempo per strike, across the melee
/// bodies that caught you. Spending anything makes it an Abort (you stopped to trade, so you are repelled);
/// spending nothing is a Push. The REARGUARD is answered separately - see [`Volley`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Answer {
    /// **Slip the line:** out-bid the whole vanguard catch pool ([`combat::slip_cost_pooled`]) - `tempo x Finesse`
    /// strictly beating the summed bids. Through the front untouched, and poorer. All-or-nothing.
    Evade,
    /// **Take the catch and go anyway.** Spend nothing striking back, eat the vanguard's blows, and arrive.
    Push,
    /// **Turn and fight.** Give up the ground and spend tempo swinging back at the melee bodies that caught you:
    /// `(catcher, strikes)` pairs, one tempo per strike. The "repelled" outcome - chosen, not imposed. The resolver
    /// applies each pair only to a catcher that actually caught you in melee and is still standing.
    Abort(Vec<(usize, u32)>),
}

/// **How a crosser answers the REARGUARD** (the volley - the second, independent crossing). A volley only ever
/// *damages*; it never halts. Chosen independently of the [`Answer`] to the vanguard - that is the evade-priority
/// split: slip the line but eat the arrows, or take the catch but dodge the arrows, whichever the tempo and the
/// threats favour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Volley {
    /// **Dodge the arrows:** out-bid the whole rearguard shot pool ([`combat::slip_cost_pooled`]). No volley
    /// damage - but the dodge tempo is spent.
    Dodge,
    /// **Eat the arrows:** spend nothing on the volley, take the shots, keep the tempo.
    Eat,
}

/// What a body does with its round. **The only thing it declares.**
///
/// No longer `Copy`: a [`Cross`](Act::Cross)'s [`Answer`] may carry a strike-back allocation (a `Vec`), so an act
/// is cloned, not blitted. The cost is nil - acts are declared once per body per round.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    /// Strike an enemy **vanguard**. Free - nothing is in the way. Melee or ranged, any region, because a region
    /// is a formation and not a place.
    Clash(usize),
    /// **Cross into the enemy's line** - slip past their front and stand inside their formation as an
    /// [`Outrider`](Rank::Outrider). It is one crossing with an optional on-arrival strike:
    /// - `Cross(Some(t), _)` also strikes **rearguard** `t` as it lands (a **raid**; melee only; the strike lands
    ///   in the Crossing Ring, *before* `t` fires in the Outer Ring - the whole worth of reaching the back line).
    /// - `Cross(None, _)` just crosses and stands (a repositioning **slip**), striking nobody this round.
    ///
    /// A raid reaches the BACK line only (a front body you [`Clash`](Act::Clash)). The two crossings are answered
    /// independently: [`Answer`] takes the vanguard interception (slip / push / halt), [`Volley`] takes the
    /// rearguard volley (dodge / eat). The way back out is [`Retreat`](Act::Retreat) - withdrawal is priced by the
    /// Inner Ring, not banned.
    Cross(Option<usize>, Answer, Volley),
    /// **Strike a body in your OWN region** - an enemy outrider loose in your ranks, or (if you are the outrider)
    /// any host body. No screen applies in-region: the ranks stopped protecting anyone the moment a body got
    /// inside them. Not a crossing, so it carries no evade-answer.
    Melee(usize),
    /// **Withdraw from the enemy ranks** (outrider only): optionally strike `t` in the Inner Ring like a
    /// [`Melee`](Act::Melee), then rejoin your own line at the Inner Ring boundary, at weapon rank. The change
    /// itself is FREE - the price is standing the Inner Ring among the hosts, where every body around you had its
    /// declared chance to strike. (This demotes the old "a crossing is committed, no retreat" rule: commitment was
    /// a means to simplicity, not a goal - the schedule prices the exit instead of banning it.)
    Retreat(Option<usize>),
    /// Nothing.
    Hold,
}

impl Act {
    /// The region this act moves you to, if it moves you at all. A [`Cross`](Act::Cross) always heads for the one
    /// enemy region (the single occupied region that is not your own); everything else stays put.
    fn destination(&self, board: &Board, i: usize) -> Option<u8> {
        let here = board.regions[i];
        match self {
            Act::Cross(..) => board.occupied().into_iter().find(|&r| r != here),
            _ => None,
        }
    }

    /// The vanguard answer (the interception), if this act is a crossing.
    fn front(&self) -> Option<&Answer> {
        match self {
            Act::Cross(_, a, _) => Some(a),
            _ => None,
        }
    }

    /// The rearguard answer (the volley), if this act is a crossing.
    fn volley(&self) -> Option<Volley> {
        match self {
            Act::Cross(_, _, v) => Some(*v),
            _ => None,
        }
    }

    pub fn label(&self, board: &Board) -> String {
        let how = |a: &Answer, v: Volley| {
            let front = match a {
                Answer::Evade => "slip the line".to_string(),
                Answer::Push => "push through".to_string(),
                Answer::Abort(alloc) => strike_back_label(board, alloc),
            };
            let back = match v {
                Volley::Dodge => "dodge the arrows",
                Volley::Eat => "eat the arrows",
            };
            format!("{front}, {back}")
        };
        match self {
            Act::Clash(t) => format!("Clash {}", board.units[*t].name),
            Act::Cross(Some(t), a, v) => format!("Raid {} ({})", board.units[*t].name, how(a, *v)),
            Act::Cross(None, a, v) => format!("Cross into their line ({})", how(a, *v)),
            Act::Melee(t) => format!("Melee {}", board.units[*t].name),
            Act::Retreat(Some(t)) => {
                format!("Strike {} and withdraw", board.units[*t].name)
            }
            Act::Retreat(None) => "Withdraw to your own line".to_string(),
            Act::Hold => "Hold".to_string(),
        }
    }
}

/// A legible phrase for a strike-back allocation: who it hits and how hard. Empty (nobody struck) reads as a bare
/// stand, which the legal-act enumeration never emits (an empty Abort is a Push) but a hand-built act might.
fn strike_back_label(board: &Board, alloc: &[(usize, u32)]) -> String {
    let live: Vec<String> = alloc
        .iter()
        .filter(|&&(_, cards)| cards > 0)
        .map(|&(c, cards)| format!("{} x{}", board.units[c].name, cards))
        .collect();
    if live.is_empty() {
        "turn and fight".to_string()
    } else {
        format!("turn and fight: {}", live.join(", "))
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

/// **Every crossing this body may declare to `target`** - the product of the two independent crossings, the
/// vanguard [`Answer`] and the rearguard [`Volley`]. This is where the crossing's branching factor is set, so both
/// axes are deliberately pruned:
///
/// - **Front (vanguard).** Uncontested (nothing predicted to catch you) -> one option, a plain
///   [`Push`](Answer::Push, Volley::Eat). Contested -> [`Evade`](Answer::Evade, Volley::Dodge) (slip), [`Push`](Answer::Push, Volley::Eat) (take it, cross),
///   and one [`Abort`](Answer::Abort) per candidate allocation from [`strikeback_candidates`].
/// - **Back (rearguard).** Only a real choice when a rearguard is predicted to volley - then [`Dodge`](Volley::Dodge)
///   and [`Eat`](Volley::Eat); otherwise only [`Eat`](Volley::Eat) (dodging nothing is dominated).
///
/// The two are enumerated as a **product** - the evade-priority split means every front-answer pairs with every
/// back-answer. Catchers/volleyers are **predicted geometrically** (no `foe_act`), because this runs *inside*
/// [`legal_acts`], which the scripted foes call through [`greedy_act`] - a `foe_act`-based prediction would recurse.
/// Over-prediction only mints an option the resolver later no-ops; it is never unsound.
fn crossing_acts(board: &Board, i: usize, target: Option<usize>) -> Vec<Act> {
    let here = board.regions[i];
    let Some(dest) = board.occupied().into_iter().find(|&r| r != here) else {
        return vec![Act::Cross(target, Answer::Push, Volley::Eat)]; // no enemy ground: degenerate
    };
    let cs = predicted_catchers(board, i, dest);
    let fronts: Vec<Answer> = if cs.is_empty() {
        vec![Answer::Push] // uncontested front: every answer is the same crossing
    } else {
        let mut f = vec![Answer::Evade, Answer::Push];
        for alloc in strikeback_candidates(board, i, &cs) {
            f.push(Answer::Abort(alloc));
        }
        f
    };
    let volleys: &[Volley] = if predicted_volley(board, i, dest) {
        &[Volley::Dodge, Volley::Eat]
    } else {
        &[Volley::Eat] // no volleyer: dodging is dominated
    };
    let mut out = Vec::new();
    for front in &fronts {
        for &v in volleys {
            out.push(Act::Cross(target, front.clone(), v));
        }
    }
    out
}

/// **Who would catch a crossing to `dest`, by geometry alone** - the enemy vanguard holding either zone the
/// crossing touches (the one left and the one entered). Unlike [`catchers`] it does NOT consult `foe_act` (so it
/// cannot recurse when called from within [`legal_acts`]); it therefore over-includes a foe that will actually be
/// in transit. That is safe for minting strike-back candidates: a pair aimed at a non-catcher is dropped in
/// [`reach_for_slippers`].
fn predicted_catchers(board: &Board, mover: usize, dest: u8) -> Vec<usize> {
    let enemy = other_side(board.units[mover].side);
    let mut out = Vec::new();
    for zone in [board.regions[mover], dest] {
        if board.owner(zone) != Some(enemy) {
            continue;
        }
        for f in board.vanguard(zone, enemy) {
            if !out.contains(&f) {
                out.push(f);
            }
        }
    }
    out
}

/// **Would a rearguard volley a crossing to `dest`?** - geometry only, like [`predicted_catchers`]. When none
/// would, dodging avoids nothing, so [`crossing_acts`] offers only [`Volley::Eat`].
fn predicted_volley(board: &Board, mover: usize, dest: u8) -> bool {
    let enemy = other_side(board.units[mover].side);
    [board.regions[mover], dest].iter().any(|&zone| {
        board.owner(zone) == Some(enemy)
            && board
                .in_region(zone)
                .into_iter()
                .any(|j| board.units[j].side == enemy && board.ranks[j] == Rank::Rearguard)
    })
}

/// **Strikes to fell catcher `c` under `mover`'s Might** - the min tempo a focus needs to silence it, so a
/// strike-back allocation never over-invests in one body when the tempo could threaten another. `u32::MAX` when
/// `mover` cannot penetrate `c`'s Grit at all (no number of strikes helps - do not focus there).
fn down_cost(board: &Board, mover: usize, c: usize) -> u32 {
    let per = board.units[mover]
        .might
        .saturating_sub(board.units[c].armor);
    if per == 0 {
        return u32::MAX;
    }
    if board.units[c].horde {
        // one PENETRATING blow fells one body; a sub-Grit blow fells none however many you throw.
        if per >= board.units[c].grit.max(1) {
            board.units[c].health.max(1)
        } else {
            u32::MAX
        }
    } else {
        // the pile banks `per` a strike and flips a card each Grit crossed; felling clears health x Grit in total.
        (board.units[c].health.max(1) * board.units[c].grit.max(1)).div_ceil(per)
    }
}

/// **The strike-back allocations worth searching** for a mover caught by `catchers` - the pruned free-allocation
/// space (spec: ignore expenditures with no observable effect). Two shapes, each a concrete `(catcher, strikes)`
/// list summing to at most the mover's tempo:
///
/// - **Spread**: one tempo per catcher, round-robin - threaten every catcher at once.
/// - **Focus-each**: for each catcher, spend the min-to-down it, then spill the remainder to the others in turn -
///   concentrate to actually silence one (or two) rather than dent all.
///
/// A catcher the mover cannot hurt (`down_cost == MAX`) is never focused and takes no spill. Duplicates collapse,
/// so a single-catcher crossing yields exactly one allocation. The human UI may offer any allocation on top of
/// these; this bounded set is only what the solver enumerates.
fn strikeback_candidates(
    board: &Board,
    mover: usize,
    catchers: &[usize],
) -> Vec<Vec<(usize, u32)>> {
    let t = board.units[mover].tempo;
    // No strike-back for a non-melee body (nothing to swing) or an AREA body: an aoe strike is always the
    // untargeted sweep and is **never used in retaliation** (the aoe invariant). A caught area crosser pushes.
    if t == 0 || catchers.is_empty() || !board.units[mover].melee || board.units[mover].aoe {
        return Vec::new();
    }
    let pairs = |alloc: &[u32]| -> Vec<(usize, u32)> {
        catchers
            .iter()
            .zip(alloc)
            .filter(|&(_, &n)| n > 0)
            .map(|(&c, &n)| (c, n))
            .collect()
    };
    let mut out: Vec<Vec<(usize, u32)>> = Vec::new();

    // Spread: round-robin one tempo at a time.
    let mut spread = vec![0u32; catchers.len()];
    for s in 0..t {
        spread[(s as usize) % catchers.len()] += 1;
    }
    out.push(pairs(&spread));

    // Focus-each: prioritise one catcher (min-to-down), then spill by index to the rest.
    for fi in 0..catchers.len() {
        let mut alloc = vec![0u32; catchers.len()];
        let mut left = t;
        let order = std::iter::once(fi).chain((0..catchers.len()).filter(move |&j| j != fi));
        for j in order {
            if left == 0 {
                break;
            }
            let cost = down_cost(board, mover, catchers[j]);
            if cost == u32::MAX {
                continue; // cannot hurt this one - do not sink tempo into it
            }
            let spend = cost.min(left);
            alloc[j] += spend;
            left -= spend;
        }
        // Tempo to spare (everyone downable and then some): pile the rest on the first catcher we can hurt.
        if left > 0
            && let Some(j) =
                (0..catchers.len()).find(|&j| down_cost(board, mover, catchers[j]) != u32::MAX)
        {
            alloc[j] += left;
        }
        let p = pairs(&alloc);
        if !p.is_empty() {
            out.push(p);
        }
    }

    out.sort();
    out.dedup();
    out
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
                // An OUTRIDER may also strike this body and then WITHDRAW - the fighting exit. Same inner-ring
                // strike as the Melee; the rejoin happens free at the boundary (the ring was the price).
                if board.ranks[i] == Rank::Outrider {
                    out.push(Act::Retreat(Some(t)));
                }
            } else if board.ranks[t] == Rank::Outrider {
                // A loose enemy body in another region is dealt with in-region by the formation that hosts it,
                // never reached across the gap. Not a target from here.
            } else if board.is_screened(t) {
                // A **screened** rearguard: reach it only by RAIDING across, past the front that guards it (melee
                // only). The front intercepts the raid in the Crossing Ring - that is the whole worth of the screen.
                if u.melee {
                    out.extend(crossing_acts(board, i, Some(t)));
                }
            } else {
                // A vanguard, OR an **exposed** rearguard whose front has fallen. Either is clashable across the
                // gap by any weapon - it is *always targetable*, so standing unscreened is never shelter. But an
                // exposed BACK can ALSO be raided (melee): a raider reaches it in the Crossing Ring, *before* it
                // would fire in the Outer Ring, so being unscreened is never an advantage either - a screen is
                // what buys a back its first shot. (Raid pushed first so a scripted raider prefers the earlier,
                // silencing reach.)
                //
                // A raid reaches the BACK line only (a front body you Clash directly). Letting a raid strike a
                // front body on arrival hands melee a Crossing-ring pre-emption of the Outer Clash - a ranged-style
                // "fire first" that broke the Swarm's answer-from-range solo (measured: it went 2-kit soft).
                if board.ranks[t] == Rank::Rearguard && u.melee {
                    out.extend(crossing_acts(board, i, Some(t)));
                }
                out.push(Act::Clash(t));
            }
        }
    }

    // An outrider may WITHDRAW without striking: rejoin its own line at the Inner Ring boundary, free - the ring
    // it stands in is the price of leaving.
    if board.ranks[i] == Rank::Outrider {
        out.push(Act::Retreat(None));
    }

    // Cross with NO target - the plain slip, the one movement, and **only the Vanguard crosses**. The front line
    // is who charges into the enemy's ground (promoting to outrider); a Rearguard stays back and fires, and an
    // outrider that wants back out declares a Retreat. The destination is the one enemy region, so it needs no
    // target.
    //
    // Offered ONLY when the enemy has a SCREENED back to reach. Going outrider is worth a crossing only if there is
    // a body behind their front you cannot already touch: a screened rearguard. A backless enemy (no rearguard) or
    // an EXPOSED one (a rearguard whose vanguard has fallen) is already clashable directly, so a slip toward it
    // reaches nothing a `Clash` does not - a legal but always-dominated line, kept off the menu. This prunes the
    // option for its two menu readers (the solver's search and the UI), NOT the resolver: `play_round` still
    // resolves a hand-built slip, because it never consults `legal_acts`. (Raids are already gated on a reachable
    // back, so only this unconditional slip needs the guard.)
    if board.ranks[i] == Rank::Vanguard
        && let Some(dest) = board.occupied().into_iter().find(|&r| r != here)
        && board
            .in_region(dest)
            .into_iter()
            .any(|j| board.units[j].side != u.side && board.is_screened(j))
    {
        out.extend(crossing_acts(board, i, None));
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

/// **How much visible disruption an act does to the enemy, this turn** - the whole of a creature's instinct,
/// ranked lexicographically: enemies **downed**, then their health cards **flipped**, then **positional**
/// advantage. Higher is better on every field. There is no per-creature switch: the *same* greedy read runs for
/// every creature, so behaviour EMERGES from stats - a low-Might wall flips little whatever it does and holds to
/// keep its own back screened; a hard striker flips most by reaching a soft back, so it raids. One ply, no solver.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Disruption {
    downs: u32,
    flips: u32,
    positional: i32,
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

/// The positional advantage of `act` for foe `i` - the term that keeps a creature in the rank its stats are built
/// for, which a one-ply flip count cannot see. Reaching an enemy back is already paid for by the flips it banks,
/// so this only prices what leaving costs. Two costs, both on a **crossing** (the one act that changes rank):
///
/// - **Screening** - abandoning a post that still screens a friendly rearguard exposes it to the enemy.
/// - **Role-fit** - a crossing trades a vanguard for an outrider. That rank pays off for a **striker** (offence
///   over defence, Might above Grit), loose in the enemy back; a **tank** (Grit >= Might) is worth far more
///   holding the front, so leaving its best rank is a cost.
///
/// Together they are the emergent form of "a wall never abandons its post": no creature is *told* to hold - a
/// Might-1 body simply scores holding above a raid worth a single flip, because the raid throws away a rank its
/// stats were made for.
fn positional(board: &Board, i: usize, act: &Act) -> i32 {
    if !matches!(act, Act::Cross(..)) {
        return 0; // only a crossing leaves the post / changes rank
    }
    let u = &board.units[i];
    let (region, side) = (board.regions[i], u.side);
    let mut cost = 0;
    let screens_a_back = board
        .in_region(region)
        .into_iter()
        .any(|j| board.units[j].side == side && board.ranks[j] == Rank::Rearguard);
    if screens_a_back && board.vanguard(region, side) == [i] {
        cost -= 1; // last of the screen: leaving unguards the back behind it
    }
    if u.might <= u.grit {
        cost -= 1; // a tank out of the vanguard is a tank wasted
    }
    cost
}

fn disruption(board: &Board, i: usize, act: &Act) -> Disruption {
    let (downs, flips) = match act {
        Act::Clash(t) | Act::Melee(t) | Act::Cross(Some(t), _, _) | Act::Retreat(Some(t)) => {
            strike_yield(board, i, *t)
        }
        Act::Cross(None, _, _) | Act::Retreat(None) | Act::Hold => (0, 0),
    };
    Disruption {
        downs,
        flips,
        positional: positional(board, i, act),
    }
}

/// **The greedy disruption act for ANY living body** - the same one-ply heuristic the scripted foes run, but
/// side-agnostic, so it also drives a *naive* hero (a party that plays without insight). `None` only if `i` is
/// fallen. Picks the act of greatest [`Disruption`], with a deterministic tiebreak.
pub fn greedy_act(board: &Board, i: usize) -> Option<Act> {
    if board.units[i].fallen {
        return None;
    }
    // Deterministic tiebreak among equal-disruption acts: strike the softest enemy (lowest hp), then the leftmost
    // (lowest index); a targetless act sorts last so a real strike always wins the tie. A final act-kind order
    // (clash over a pushed raid over an evaded one over a slip over hold) settles two acts on the same target.
    let target_hp = |act: &Act| match act {
        Act::Clash(t) | Act::Melee(t) | Act::Cross(Some(t), _, _) | Act::Retreat(Some(t)) => {
            (board.units[*t].health, *t)
        }
        _ => (u32::MAX, usize::MAX),
    };
    let act_pref = |act: &Act| match act {
        Act::Clash(_) | Act::Melee(_) => 0u8,
        Act::Cross(_, Answer::Push, _) => 1,
        Act::Cross(_, Answer::Evade, _) => 2,
        Act::Cross(_, Answer::Abort(_), _) => 3,
        // A scripted foe prefers staying in (Melee, pref 0) over the fighting exit on an equal-disruption tie: an
        // outrider's instinct is havoc, so it withdraws only when withdrawal out-DISRUPTS staying (it never does
        // under the current metric). Emergent, not fiat - the option is on its menu like anyone's.
        Act::Retreat(Some(_)) => 4,
        Act::Retreat(None) => 5,
        Act::Hold => 6,
    };
    legal_acts(board, i)
        .into_iter()
        .max_by(|a, b| {
            disruption(board, i, a)
                .cmp(&disruption(board, i, b))
                // lower hp / index / act-pref preferred, so reverse them into the max
                .then_with(|| target_hp(b).cmp(&target_hp(a)))
                .then_with(|| act_pref(b).cmp(&act_pref(a)))
        })
        .or(Some(Act::Hold))
}

/// **The one act a single scripted foe takes** - [`greedy_act`] restricted to foes (a hero chooses; a corpse does
/// nothing). This is the single option [`super::game`] offers when a foe reaches the declaration cursor - a
/// creature "declares" like a hero, its turn just has exactly one legal move, the one its instinct dictates.
pub fn foe_act(board: &Board, i: usize) -> Option<Act> {
    if board.units[i].side != Side::Foe {
        return None;
    }
    greedy_act(board, i)
}

/// **The catch instinct - which enemy crosser this body intercepts,** the deterministic policy a scripted foe
/// declares in the CATCH WAVE (and the default a greedy party plays). One catch per catcher: it picks the crosser
/// it would most disrupt (downs, then flips - the same [`Disruption`] read as everything else), lowest index
/// breaking ties, and it always catches when an enemy crosses - a formation does not watch a runner go by.
/// `None` when no enemy is crossing (or this body cannot strike at all).
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

/// **Who would intercept a crossing** - the enemy vanguard(s) that would *declare a catch* on `mover` if it
/// crosses. Catching is a DECLARED choice (the catch wave), so this predicts the scripted foes' declarations:
/// every enemy vanguard not itself crossing (per [`foe_act`]) follows [`foe_catch`], which always catches.
/// Surfaced so a narrative UI can name *who* catches you *before* you commit an [`Answer`] - matching the order
/// the fiction hands you the decision (declare the crossing, see who caught you, then answer). Meant for a hero's
/// crossing (the catchers are foes); an empty result means the front does not contest it.
pub fn catchers(board: &Board, mover: usize, _dest: u8) -> Vec<usize> {
    (0..board.units.len())
        .filter(|&f| {
            !board.units[f].fallen
                && board.units[f].side != board.units[mover].side
                && board.ranks[f] == Rank::Vanguard
                && foe_act(board, f)
                    .map(|a| a.destination(board, f).is_none())
                    .unwrap_or(false)
                && foe_catch(board, f, &[mover]) == Some(mover)
        })
        .collect()
}

/// **The default catch declarations for a whole round** - every eligible body (living, Vanguard/Rearguard, not
/// itself crossing) declaring by [`foe_catch`]. This is what the scripted foes play, what a greedy (no-search)
/// party plays, and the convenient porting shim for a driver that has no catch wave of its own.
pub fn default_catches(board: &Board, acts: &[Act]) -> Vec<Option<usize>> {
    let crossers: Vec<usize> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen && acts[i].destination(board, i).is_some())
        .collect();
    (0..board.units.len())
        .map(|e| {
            if board.units[e].fallen
                || acts[e].destination(board, e).is_some()
                || board.ranks[e] == Rank::Outrider
            {
                return None;
            }
            foe_catch(board, e, &crossers)
        })
        .collect()
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
    /// **Which phase of the round this is** - the ring and step, e.g. `"Inner Ring: Outriders"`,
    /// `"Crossing Ring: Intercept"`, `"Outer Ring: Clash"`. Set by [`play_round`] at each step so a transcript can
    /// say *where* in the round every strike and card-flip happened, not just *that* it happened. Empty on a log
    /// built outside `play_round`.
    pub phase: &'static str,
    /// Got through - standing somewhere new now.
    pub through: Vec<usize>,
    /// Turned and fought instead: it stayed where it was.
    pub aborted: Vec<usize>,
    /// **Withdrew from the enemy ranks** at the Inner Ring boundary - an outrider that declared a
    /// [`Retreat`](Act::Retreat) and lived to make it, rejoining its own line at weapon rank.
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
        tempo: board.units.iter().map(|u| u.tempo).collect(),
        ranks: board.ranks.clone(),
        regions: board.regions.clone(),
        ..Default::default()
    }
}

fn living(board: &Board) -> Vec<bool> {
    board.units.iter().map(|u| !u.fallen).collect()
}

// (The automatic screen sweep - `in_transit` / `holding_line` / `back_line` - is gone: catching is a DECLARED
// choice now (the catch wave), so the catcher sets are built from `catches` in `play_round`. The old "a body in
// transit cannot hold the line" rule is free: a crossing body is not eligible to declare a catch at all.)

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
    movers: &[(usize, u8, &Answer, Volley)],
    is_front: bool, // the Intercept pass (vanguard) vs the Volley pass (rearguard)
) -> (Vec<Hit>, Vec<Reach>) {
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

    // The slipper answers, seeing exactly what was committed - and it answers the TWO crossings independently. In
    // the Intercept pass a mover slips iff its vanguard [`Answer`] is `Evade`; in the Volley pass iff its rearguard
    // [`Volley`] is `Dodge`. That is the evade-priority split: front and back are separate spends. (An area volley
    // below is not an edge you can slip.)
    let dodges: Vec<Dodge> = (0..board.units.len())
        .map(|i| {
            let evading = movers.iter().any(|&(m, _, front, volley)| {
                m == i
                    && if is_front {
                        matches!(front, Answer::Evade)
                    } else {
                        volley == Volley::Dodge
                    }
            });
            if evading { Dodge::Slip } else { Dodge::Stand }
        })
        .collect();
    // The crossing pools: a slipper must out-bid the WHOLE line reaching it, not merely the worst single catcher.
    let mut landed = resolve_evade_pooled(&mut board.units, &reaching, &dodges);
    // Record the aimed contest NOW, before the unevadable area contacts join `landed` - so `evaded` reflects the
    // genuine slip contest only.
    let reaches = reaches_of(&reaching, &landed);

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
                // A sweep clears the whole crossing pack iff it penetrates Grit (see `area_strike`).
                if might.saturating_sub(board.units[s].armor) >= board.units[s].grit.max(1) {
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

    // An Abort turns and fights: the crosser HALTS, and by the engagement rule stopping to fight earns it one FREE
    // blow (the mutual clash) plus whatever paid strikes its declared allocation buys. Both reach only catchers
    // that struck it in MELEE (the one-way rule, same as `can_answer`): you cannot swing back at a rearguard that
    // volleyed you from across the gap. So this pass answers the front line (vanguard); the back-line volley pass,
    // whose catchers are ranged, answers nobody. Push (`Answer::Push`) earns no blow - it fled, it did not engage.
    // (Naturally front-only: in the Volley pass every catcher is a ranged rearguard, so `melee_caught` below is
    // empty and no strike-back or free blow is built. The `is_front` flag is not needed to gate it.)
    let mut ripostes: Vec<Blows> = Vec::new();
    let mut free_blows: Vec<Contact> = Vec::new();
    for &(i, _, a, _) in movers {
        let Answer::Abort(alloc) = a else { continue };
        // An AREA body never strikes back (it only ever sweeps); a corpse and a non-melee body cannot. Note the
        // absence of a `tempo == 0` guard: the free blow costs no tempo, so even a crosser that spent its whole
        // pool failing to evade still gets its one reflexive clash when it halts.
        if board.units[i].fallen || !board.units[i].melee || board.units[i].aoe {
            continue;
        }
        let melee_caught: Vec<usize> = landed
            .iter()
            .filter(|c| c.target == i)
            .map(|c| c.attacker)
            .filter(|&c| !board.units[c].fallen && board.units[c].melee)
            .collect();
        if melee_caught.is_empty() {
            continue; // caught only from range: nothing to turn and fight
        }
        // The FREE blow: one no-tempo strike (the clash) at the melee catcher it committed most to (its focus),
        // lowest index breaking ties.
        let focus = melee_caught
            .iter()
            .copied()
            .max_by_key(|&c| {
                let paid = alloc
                    .iter()
                    .find(|&&(cc, _)| cc == c)
                    .map_or(0, |&(_, n)| n);
                (paid, std::cmp::Reverse(c))
            })
            .unwrap();
        free_blows.push(Contact {
            attacker: i,
            target: focus,
            bid: 0,
        });
        // Then the PAID strike-back: one tempo per strike, only at the melee catchers it named (`land` caps the
        // total at the tempo actually left; a stale or mis-aimed pair is simply dropped).
        for &(c, cards) in alloc {
            if cards == 0 || !melee_caught.contains(&c) {
                continue;
            }
            ripostes.push(Blows {
                unit: i,
                target: c,
                cards,
            });
        }
    }
    // The crosser's free blows join the opening-strike batch - one blow each, no tempo, alongside the catchers'.
    landed.extend(free_blows);

    let mut hits = land(board, &landed, &[], &ripostes);
    // Apply the volley's horde clears AFTER `land` read commit-time bodies, so an aborter's riposte still lands
    // and a swept horde still counted at full size in this batch (commit-batch simultaneity, Spec 1.9).
    for h in &felled {
        board.units[h.target].health = board.units[h.target].health.saturating_sub(h.hits);
    }
    hits.extend(felled);
    (hits, reaches)
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
pub fn play_round(board: &mut Board, acts: &[Act], catches: &[Option<usize>]) -> Vec<SubPhaseLog> {
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
    // A Retreat's optional strike is the SAME inner-ring strike as a Melee - the fighting exit swings first, and
    // leaves at the boundary below.
    let melees: Vec<Attack> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|i| match acts[i] {
            Act::Melee(t) | Act::Retreat(Some(t))
                if !board.units[t].fallen
                    && board.regions[t] == board.regions[i]
                    && board.units[t].side != board.units[i].side =>
            {
                Some((i, t))
            }
            _ => None,
        })
        .collect();
    // Every declared withdrawal this round - resolved at the boundary, whether or not any strike ran.
    let retreating: Vec<usize> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen && matches!(acts[i], Act::Retreat(_)))
        .collect();
    if !melees.is_empty() || !retreating.is_empty() {
        let before = living(board);
        let (hits, reaches) = exchange(board, &melees, true, true);
        let mut lg = close(board, &before);
        lg.phase = "Inner Ring: Outriders";
        lg.hits = hits;
        lg.reaches = reaches;
        logs.push(lg);
    }
    // Outriders whose host formation is now gone are outriders of nothing - the state dissolves and they rejoin
    // their own line (see `dissolve`). Resolved once here, at the Inner Ring boundary where the havoc lands.
    dissolve(board);
    // WITHDRAWAL: a declared Retreat that survived the ring rejoins its own line now, at weapon rank - free,
    // because the ring it just stood was the price (every host had its declared chance at it). A body dissolution
    // already moved is home already; one felled in the ring never leaves.
    let mut withdrew: Vec<usize> = Vec::new();
    for &i in &retreating {
        if board.units[i].fallen || board.ranks[i] != Rank::Outrider {
            continue;
        }
        board.regions[i] =
            home_of(board, board.units[i].side, board.regions[i]).unwrap_or(board.regions[i]); // last of its side: the line re-forms where it stands
        board.ranks[i] = Board::weapon_rank(&board.units[i]);
        withdrew.push(i);
    }
    // Dissolution and withdrawal move bodies (rank + region), so fold the result back into the Inner Ring's
    // snapshot - they are Inner-Ring-boundary events, and a transcript must not read them as having happened a
    // phase later. (When no Inner strike or withdrawal ran, both are no-ops and there is nothing to fold.)
    if let Some(inner) = logs.last_mut() {
        inner.ranks = board.ranks.clone();
        inner.regions = board.regions.clone();
        inner.withdrew = withdrew;
    }

    // ---- CROSSING RING: CROSSINGS (closing into a formation) -------------------------------------------
    //
    // Every declared Raid/Slip sends its body across as a transient. An enemy formation reaches for a crosser at
    // BOTH ends it touches - the zone ENTERED and the zone LEFT - because you are outside your own screen the
    // moment you move. At each such end the FRONT intercepts (spears) THEN the BACK volleys the survivors (bows),
    // so a front-killed crosser is not volleyed. A friendly zone never reaches for its own, so a rally between
    // friendly zones is free; but an outrider pulling OUT of enemy ranks is opposed by the ranks it leaves (the
    // crossing in reverse).
    // A mover carries both crossing answers: the vanguard `Answer` (front) and the rearguard `Volley` (back),
    // keyed to their respective passes below.
    let movers: Vec<(usize, u8, &Answer, Volley)> = (0..board.units.len())
        .filter(|&i| !board.units[i].fallen)
        .filter_map(|i| {
            Some((
                i,
                acts[i].destination(board, i)?,
                acts[i].front()?,
                acts[i].volley()?,
            ))
        })
        .collect();

    // **CATCHING IS DECLARED, AND ADDITIVE.** The catcher sets come from the round's CATCH WAVE (`catches`), not
    // from geometry: a body intercepts (vanguard) or volleys (rearguard) the ONE enemy crosser its catch names -
    // or nobody, if it declined. The catch is an engagement in ADDITION to the body's own act, priced in tempo
    // (the measured delta-2 finding: making a catch consume the act collapsed the corners). Resolver-enforced
    // validity, menu-independent: a fallen/crossing catcher, an outrider, or a catch naming a non-mover simply
    // drops. An AREA catcher's catch is a sweep across the whole enemy crossing band - area is width, so naming
    // any crosser catches them all.
    let is_mover: Vec<bool> = {
        let mut v = vec![false; board.units.len()];
        for &(i, _, _, _) in &movers {
            v[i] = true;
        }
        v
    };
    let mut front_catchers: Vec<(usize, usize)> = Vec::new();
    let mut back_catchers: Vec<(usize, usize)> = Vec::new();
    for f in 0..board.units.len() {
        let Some(named) = catches[f] else { continue };
        if board.units[f].fallen || is_mover[f] {
            continue;
        }
        let valid = |t: usize| {
            is_mover[t] && !board.units[t].fallen && board.units[t].side != board.units[f].side
        };
        let targets: Vec<usize> = if board.units[f].aoe {
            (0..board.units.len()).filter(|&t| valid(t)).collect()
        } else if valid(named) {
            vec![named]
        } else {
            Vec::new()
        };
        for t in targets {
            match board.ranks[f] {
                Rank::Vanguard => front_catchers.push((f, t)),
                Rank::Rearguard => back_catchers.push((f, t)),
                Rank::Outrider => {} // an outrider holds no line to catch from - it fights the Inner Ring
            }
        }
    }

    let before = living(board);
    let (hits, reaches) = reach_for_slippers(board, &front_catchers, &movers, true);
    let mut lg = close(board, &before);
    lg.phase = "Crossing Ring: Intercept";
    lg.hits = hits;
    lg.reaches = reaches;
    logs.push(lg);
    let before = living(board);
    let (hits, reaches) = reach_for_slippers(board, &back_catchers, &movers, false);
    let mut lg = close(board, &before);
    lg.phase = "Crossing Ring: Volley";
    lg.hits = hits;
    lg.reaches = reaches;
    logs.push(lg);

    // LAND: survivors that got through leave the line and arrive. Into an enemy zone they promote to outrider
    // (loose inside the ranks); a rally into a friendly zone rejoins that formation at its weapon rank.
    let mut through = vec![false; board.units.len()];
    let mut landing = SubPhaseLog {
        health: board.units.iter().map(|u| u.health).collect(),
        ranks: board.ranks.clone(),
        ..Default::default()
    };
    for &(i, dest, front, _volley) in &movers {
        if board.units[i].fallen {
            continue;
        }
        // Through-vs-stay is decided by the VANGUARD answer only; the volley never halts.
        if matches!(front, Answer::Abort(_)) {
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
        // The land happens AFTER this phase's snapshot, so fold its rank/region moves back into it - otherwise a
        // crossing reads its PRE-land rank here (a raider still a Vanguard) and the real change to Outrider surfaces
        // a phase late as a spurious second move. One crossing, one phase. (Same fix as dissolution.)
        last.ranks = board.ranks.clone();
        last.regions = board.regions.clone();
    }

    // The raiders that got through strike the rearguard they came for - before it can fire in the Outer Ring. Tempo-gated:
    // a raider that evaded with everything arrives with nothing to swing. A Raid sweep covers the back line it is
    // now standing among (its tier).
    let before = living(board);
    let raids: Vec<Attack> = movers
        .iter()
        .filter(|&&(i, _, _, _)| through[i] && !board.units[i].fallen && board.units[i].tempo > 0)
        .filter_map(|&(i, _, _, _)| match acts[i] {
            Act::Cross(Some(t), _, _)
                if !board.units[t].fallen && board.regions[t] == board.regions[i] =>
            {
                Some((i, t))
            }
            _ => None,
        })
        .collect();
    let (hits, reaches) = exchange(board, &raids, false, false);
    let mut lg = close(board, &before);
    lg.phase = "Crossing Ring: Raid";
    lg.hits = hits;
    lg.reaches = reaches;
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
        let (hits, reaches) = exchange(board, &attacks, true, false);
        let mut lg = close(board, &before);
        lg.phase = if tier == Rank::Rearguard {
            "Outer Ring: Fire"
        } else {
            "Outer Ring: Clash"
        };
        lg.hits = hits;
        lg.reaches = reaches;
        logs.push(lg);
    }

    logs
}

/// One Engage -> Evade -> Strike exchange - **the product's inner three, unchanged.** Area strikes split off and
/// sweep their target's region: the tier aimed at, or - when `sweep_whole` - both tiers (an in-region melee, past
/// the screen).
fn exchange(
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
            acts[hero] = act.clone();
            let mut b = board.clone();
            let catches = default_catches(&b, &acts);
            play_round(&mut b, &acts, &catches);
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
            let catches = default_catches(&b, &acts);
            play_round(&mut b, &acts, &catches);
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
        acts[i] = choices[k][(pick / radix) % choices[k].len()].clone();
    }
    for (i, a) in foes.iter().enumerate() {
        if let Some(a) = a {
            acts[i] = a.clone();
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

    /// Play a round under the DEFAULT catch declarations ([`default_catches`]: every eligible body catches per
    /// the instinct) - the auto-catch behavior most of these tests were written against, now stated explicitly.
    /// A test about the catch wave itself calls [`play_round`] with its own `catches`.
    fn round(b: &mut Board, acts: &[Act]) -> Vec<SubPhaseLog> {
        let catches = default_catches(b, acts);
        play_round(b, acts, &catches)
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
            round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
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
            acts.iter().any(|a| matches!(a, Act::Cross(Some(1), _, _))),
            "to reach the cannon it must raid the wall"
        );
    }

    /// **The slipper always has all three answers.**
    #[test]
    fn a_slipper_always_has_all_three_answers() {
        let b = wall_and_cannon();
        let acts = legal_acts(&b, 2);
        assert!(
            acts.contains(&Act::Cross(Some(1), Answer::Evade, Volley::Dodge)),
            "Evade must be on the menu"
        );
        assert!(
            acts.contains(&Act::Cross(Some(1), Answer::Push, Volley::Eat)),
            "Push must be on the menu"
        );
        assert!(
            acts.iter().any(
                |a| matches!(a, Act::Cross(Some(1), Answer::Abort(alloc), _) if !alloc.is_empty())
            ),
            "at least one Abort (strike-back) must be on the menu"
        );
    }

    /// **The strike-back is a real allocation, not a fixed spread.** Caught by two walls, a raider is offered both
    /// a FOCUS (pour its whole tempo onto one, to silence it) and a SPREAD (a blow at each) - the pruned free
    /// space the solver searches. Walls are Grit 3, so downing one costs the raider's whole 3 tempo: focus and
    /// spread genuinely diverge.
    #[test]
    fn strike_back_offers_focus_and_spread() {
        let b = Board::new(
            vec![
                unit("Raider", Side::Party, [4, 4, 1, 3, 2], true, false),
                unit("WallA", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("WallB", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [3, 3, 1, 1, 2], false, true),
            ],
            vec![0, 1, 1, 1],
        );
        // The raid on the screened Mage (unit 3), caught by both walls (units 1 and 2).
        let allocs: Vec<Vec<(usize, u32)>> = legal_acts(&b, 0)
            .into_iter()
            .filter_map(|a| match a {
                Act::Cross(Some(3), Answer::Abort(alloc), _) => Some(alloc),
                _ => None,
            })
            .collect();
        // A FOCUS: the whole 3 tempo onto a single wall.
        assert!(
            allocs.contains(&vec![(1, 3)]) && allocs.contains(&vec![(2, 3)]),
            "a focus onto each wall must be offered, got {allocs:?}"
        );
        // A SPREAD: a strike at each wall in the same allocation.
        assert!(
            allocs.iter().any(|al| al.len() == 2),
            "a spread across both walls must be offered, got {allocs:?}"
        );
        // Every allocation spends at most the raider's tempo (3), and only on the catchers.
        for al in &allocs {
            let total: u32 = al.iter().map(|&(_, n)| n).sum();
            assert!(total <= 3, "an allocation cannot overspend tempo: {al:?}");
            assert!(
                al.iter().all(|&(c, _)| c == 1 || c == 2),
                "strike-back only targets the catchers: {al:?}"
            );
        }
    }

    /// **A backless enemy offers no plain slip - but the resolver still resolves one.** The menu (what the solver
    /// and UI read from `legal_acts`) drops `Cross(None)` when there is no enemy rearguard to reach; the engine
    /// (`play_round`), which never consults the menu, resolves a hand-built backless slip exactly as before.
    #[test]
    fn a_backless_enemy_hides_the_slip_but_the_engine_still_resolves_it() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [4, 4, 1, 3, 2], true, false),
                unit("Brute", Side::Foe, [3, 4, 1, 2, 2], true, false), // lone melee front: no rearguard
            ],
            vec![0, 1],
        );
        // Off the MENU: no plain slip is offered when nothing sits behind their front.
        assert!(
            !legal_acts(&b, 0)
                .iter()
                .any(|a| matches!(a, Act::Cross(None, _, _))),
            "a backless enemy must not offer a plain slip"
        );
        // Still in the ENGINE: hand it a backless slip and it crosses just the same.
        round(
            &mut b,
            &[Act::Cross(None, Answer::Push, Volley::Eat), Act::Hold],
        );
        assert_eq!(
            b.regions[0], 1,
            "the resolver still crosses a hand-built slip"
        );
        assert_eq!(
            b.ranks[0],
            Rank::Outrider,
            "and promotes it to outrider, menu or no menu"
        );
    }

    /// **Only a SCREENED back earns a slip.** A screened rearguard (a live vanguard in front of it) is worth
    /// crossing for - you cannot Clash it. An EXPOSED rearguard (its vanguard fallen, here simply absent) is
    /// clashable directly, so the slip toward it is dominated and stays off the menu, exactly like a backless enemy.
    #[test]
    fn only_a_screened_back_offers_the_slip() {
        // Screened: the Wall fronts the Mage, so the Mage cannot be Clashed - a slip is worth offering.
        let screened = Board::new(
            vec![
                unit("Raider", Side::Party, [4, 4, 1, 3, 2], true, false),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        assert!(
            legal_acts(&screened, 0)
                .iter()
                .any(|a| matches!(a, Act::Cross(None, _, _))),
            "a screened back must offer the slip"
        );
        // Exposed: the Mage stands alone (no vanguard), so it is directly clashable - no slip.
        let exposed = Board::new(
            vec![
                unit("Raider", Side::Party, [4, 4, 1, 3, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 1],
        );
        assert!(
            !legal_acts(&exposed, 0)
                .iter()
                .any(|a| matches!(a, Act::Cross(None, _, _))),
            "an exposed back must not offer the slip"
        );
        assert!(
            legal_acts(&exposed, 0).contains(&Act::Clash(1)),
            "but the exposed back is clashable directly"
        );
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
            !acts.iter().any(|a| matches!(a, Act::Cross(Some(_), _, _))),
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
        round(
            &mut b,
            &[
                Act::Clash(2),
                Act::Clash(2),
                Act::Cross(Some(1), Answer::Evade, Volley::Dodge),
            ],
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
        round(
            &mut b,
            &[
                Act::Clash(2),
                Act::Clash(2),
                Act::Cross(Some(1), Answer::Push, Volley::Eat),
            ],
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
        round(
            &mut b,
            &[
                Act::Clash(2),
                Act::Clash(2),
                Act::Cross(Some(1), Answer::Evade, Volley::Dodge),
            ],
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
        let logs = round(
            &mut b,
            &[
                Act::Clash(2),
                Act::Clash(2),
                Act::Cross(Some(1), Answer::Abort(vec![(0, 1)]), Volley::Eat),
            ],
        );
        assert!(
            logs.iter().any(|l| l.aborted.contains(&2)),
            "it turned back at the line"
        );
        assert_eq!(b.regions[2], 1, "it never left its own ground");
        assert_ne!(b.ranks[2], Rank::Outrider, "and it is no outrider");
        assert_eq!(b.units[1].health, cannon, "and it never reached the cannon");
    }

    /// **A volleying rearguard is never answered** (the one-way rule). Even a hand-built Abort that names the
    /// ranged Archer as a strike-back target lands nothing on it - you cannot swing back at a body that shot you
    /// from range. The resolver enforces this whatever the allocation says.
    #[test]
    fn strike_back_never_answers_a_ranged_catcher() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [5, 5, 1, 4, 2], true, false),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Archer", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        let archer = b.units[2].health;
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Abort(vec![(2, 4)]), Volley::Eat), // names the ranged Archer
                Act::Hold,
                Act::Hold,
            ],
        );
        assert_eq!(
            b.units[2].health, archer,
            "the strike-back cannot reach the rearguard that volleyed from range"
        );
    }

    /// **Backs the worked example in round-sequence.md.** Round 1: the Raider raids the Sniper - pushes the Wall,
    /// dodges the volley - reaches the back as an outrider and kills it. Round 2: the Inner Ring, outrider vs Wall.
    /// If these outcomes change, the doc's sample log is stale.
    #[test]
    fn worked_round_example() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [3, 5, 1, 4, 1], true, false),
                unit("Wall", Side::Foe, [1, 3, 1, 2, 1], true, false),
                unit("Sniper", Side::Foe, [3, 2, 1, 2, 1], false, true),
            ],
            vec![0, 1, 1],
        );
        // Round 1: raid the Sniper - PUSH the Wall, DODGE the volley.
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Dodge),
                Act::Hold,
                Act::Hold,
            ],
        );
        assert_eq!(
            b.units[0].health, 4,
            "Raider took the Wall's 1, dodged the Sniper"
        );
        assert!(b.units[2].fallen, "the raid killed the Sniper");
        assert_eq!(
            b.ranks[0],
            Rank::Outrider,
            "Raider is now an outrider inside"
        );
        // Round 2: the Inner Ring - Raider (outrider) vs the Wall, point-blank.
        round(&mut b, &[Act::Melee(1), Act::Melee(0), Act::Hold]);
        assert!(
            b.units[1].fallen,
            "the Raider felled the Wall in the inner ring"
        );
        // The Wall spends its whole pool point-blank (opening + one poured strike) = 2 before it dies.
        assert_eq!(
            b.units[0].health, 2,
            "the Wall's parting blows cost the Raider 2"
        );
    }

    /// **Withdrawal: the way back out is priced, not banned.** An outrider may strike in the Inner Ring and then
    /// rejoin its own line at the boundary - free, because the ring was the price (the hosts had their declared
    /// chance). This is the demotion of "a crossing is committed": raids become round-trips.
    #[test]
    fn an_outrider_may_withdraw_after_the_inner_ring() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [3, 5, 1, 4, 1], true, false),
                unit("Archer", Side::Party, [2, 3, 1, 1, 1], false, true), // holds the home line
                unit("Wall", Side::Foe, [1, 6, 6, 2, 1], true, false), // Grit 6: survives the exit strike
                unit("Sniper", Side::Foe, [3, 2, 1, 2, 1], false, true),
            ],
            vec![0, 0, 1, 1],
        );
        // Round 1: cross in (the worked example's raid - push the Wall, dodge the volley, kill the Sniper).
        round(
            &mut b,
            &[
                Act::Cross(Some(3), Answer::Push, Volley::Dodge),
                Act::Hold,
                Act::Hold,
                Act::Hold,
            ],
        );
        assert_eq!(b.ranks[0], Rank::Outrider, "in, loose in their ranks");
        // The withdrawal is on the menu, in both forms.
        let acts = legal_acts(&b, 0);
        assert!(
            acts.contains(&Act::Retreat(None)),
            "bare withdrawal offered"
        );
        assert!(
            acts.contains(&Act::Retreat(Some(2))),
            "the fighting exit offered"
        );
        // Round 2: strike the Wall AND withdraw. The strike is a full inner-ring melee; the Wall's own declared
        // strike still lands (the price of the ring); then the Raider walks out while the Wall still stands.
        let logs = round(
            &mut b,
            &[Act::Retreat(Some(2)), Act::Hold, Act::Melee(0), Act::Hold],
        );
        assert!(
            b.units[2].health < 6 && !b.units[2].fallen,
            "the fighting exit swung, and the Wall still stands (no dissolution)"
        );
        assert_eq!(
            b.units[0].health,
            4 - 2,
            "and still paid the ring: the Wall's declared blows landed on the way out"
        );
        assert_eq!(b.regions[0], 0, "home again");
        assert_eq!(
            b.ranks[0],
            Rank::Vanguard,
            "back in its own line, at weapon rank"
        );
        assert!(
            logs.iter().any(|l| l.withdrew.contains(&0)),
            "the transcript records the withdrawal at the Inner Ring boundary"
        );
    }

    /// **A body felled in the Inner Ring never leaves.** Withdrawal resolves at the boundary, AFTER the ring's
    /// strikes - a corpse declared a Retreat, but corpses rejoin nothing.
    #[test]
    fn a_felled_withdrawer_does_not_go_home() {
        let mut b = Board::new(
            vec![
                unit("Scout", Side::Party, [1, 1, 1, 1, 1], true, false),
                unit("Brute", Side::Foe, [5, 4, 1, 3, 1], true, false),
                unit("Mage", Side::Foe, [3, 2, 1, 2, 1], false, true),
            ],
            vec![0, 1, 1],
        );
        // Put the fragile Scout inside as an outrider by hand (tests share module access).
        b.regions[0] = 1;
        b.ranks[0] = Rank::Outrider;
        round(&mut b, &[Act::Retreat(None), Act::Melee(0), Act::Hold]);
        assert!(b.units[0].fallen, "the Brute fells it in the ring");
        assert_eq!(b.regions[0], 1, "and it never made it home");
    }
    /// back, the crosser can PUSH the line (eat the trivial catch) yet DODGE the arrows - a combination the old
    /// single `Answer` could not express. Dodging the volley, decided independently of the front, is the difference
    /// between living and dying.
    #[test]
    fn the_two_crossings_are_chosen_independently() {
        let make = || {
            Board::new(
                vec![
                    unit("Raider", Side::Party, [3, 6, 1, 4, 2], true, false),
                    unit("Wall", Side::Foe, [1, 4, 6, 1, 2], true, false), // weak Might-1 front
                    unit("Archer", Side::Foe, [5, 3, 1, 2, 2], false, true), // lethal back
                ],
                vec![0, 1, 1],
            )
        };
        // Both volley answers are on the menu when a rearguard volleys.
        let acts = legal_acts(&make(), 0);
        assert!(
            acts.iter()
                .any(|a| matches!(a, Act::Cross(Some(2), _, Volley::Dodge))),
            "dodge must be offered"
        );
        assert!(
            acts.iter()
                .any(|a| matches!(a, Act::Cross(Some(2), _, Volley::Eat))),
            "eat must be offered"
        );
        // Push the (weak) line but DODGE the (lethal) arrows.
        let mut b = make();
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Dodge),
                Act::Hold,
                Act::Hold,
            ],
        );
        let dodged = b.units[0].health;
        // Same front answer, but EAT the arrows: the volley now lands.
        let mut b2 = make();
        round(
            &mut b2,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Eat),
                Act::Hold,
                Act::Hold,
            ],
        );
        let ate = b2.units[0].health;
        assert!(
            dodged > ate,
            "dodging the volley (independently of the front) saves health: dodged={dodged} ate={ate}"
        );
    }

    /// **Halting earns a free blow.** A crosser that halts is engaging, so it lands one free strike (no tempo) at
    /// a melee catcher even with an EMPTY paid allocation - the mutual clash. The Wall takes damage it would not
    /// have taken from a mere stand.
    #[test]
    fn halting_lands_one_free_blow() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [5, 5, 1, 3, 2], true, false),
                unit("Wall", Side::Foe, [1, 3, 2, 1, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        let wall = b.units[1].health;
        // Halt with NO paid strike-back: only the free clash should land.
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Abort(vec![]), Volley::Eat),
                Act::Hold,
                Act::Hold,
            ],
        );
        assert!(
            b.units[1].health < wall,
            "halting lands its one free blow on the melee catcher, empty allocation or not"
        );
    }

    /// **An area body never strikes back** (the aoe invariant: aoe is always the untargeted sweep, never a
    /// retaliation). It is offered no strike-back on the menu, and the resolver drops a hand-built one.
    #[test]
    fn an_area_body_never_strikes_back() {
        let b = Board::new(
            vec![
                unit("Sweeper", Side::Party, [4, 4, 1, 3, 2], true, false).with_aoe(true),
                unit("Wall", Side::Foe, [1, 4, 3, 1, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        assert!(
            !legal_acts(&b, 0)
                .iter()
                .any(|a| matches!(a, Act::Cross(_, Answer::Abort(alloc), _) if !alloc.is_empty())),
            "an area body must not be offered a strike-back"
        );
        let mut b2 = b.clone();
        let wall = b2.units[1].health;
        round(
            &mut b2,
            &[
                Act::Cross(Some(2), Answer::Abort(vec![(1, 3)]), Volley::Eat),
                Act::Hold,
                Act::Hold,
            ],
        );
        assert_eq!(
            b2.units[1].health, wall,
            "an area body's Abort lands no strike-back"
        );
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
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Eat),
                Act::Clash(0),
                Act::Clash(0),
            ],
        );
        assert_eq!(b.regions[0], 1, "it got through");
        assert!(
            b.units[0].health < raider,
            "and was shot on the way in - by the wall and the cannon"
        );
    }

    /// **A body in transit cannot catch a body coming the other way.** Under declared catching this is free: a
    /// crossing body is not eligible for the catch wave at all, so even a catch NAMING the other crosser (hand-
    /// built here) drops at the resolver - it holds no line while it runs. The transcript proves it: no
    /// engagement (or blow) runs from the crossing Guard to the crossing Ogre.
    #[test]
    fn a_body_in_transit_cannot_hold_the_line_it_just_left() {
        let mut b = Board::new(
            vec![
                unit("Guard", Side::Party, [3, 4, 1, 2, 2], true, false),
                unit("Cannon", Side::Party, [4, 2, 1, 2, 2], false, true),
                unit("Ogre", Side::Foe, [5, 5, 2, 3, 2], true, false),
                unit("Mage", Side::Foe, [4, 3, 1, 2, 2], false, true),
            ],
            vec![0, 0, 1, 1],
        );
        let logs = play_round(
            &mut b,
            &[
                Act::Cross(Some(3), Answer::Evade, Volley::Dodge),
                Act::Clash(2),
                Act::Cross(Some(1), Answer::Evade, Volley::Dodge),
                Act::Clash(0),
            ],
            // The Guard (itself crossing) hand-declares a catch on the Ogre: the resolver must drop it.
            &[Some(2), None, None, None],
        );
        for lg in &logs {
            assert!(
                !lg.reaches.iter().any(|r| r.attacker == 0 && r.target == 2)
                    && !lg.hits.iter().any(|h| h.attacker == 0 && h.target == 2),
                "the crossing Guard reaches for nobody - it holds no line while it runs ({})",
                lg.phase
            );
        }
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
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Eat),
                Act::Hold,
                Act::Hold,
            ],
        );
        assert_eq!(b.regions[0], 1, "it is in the enemy zone");
        assert_eq!(b.ranks[0], Rank::Outrider, "and is an outrider there");

        // Next round it is offered Melee (in-region, no screen) at the host bodies - not a raid.
        let acts = legal_acts(&b, 0);
        assert!(
            acts.iter().any(|a| matches!(a, Act::Melee(_))),
            "an outrider melees in-region: {acts:?}"
        );
        assert!(
            !acts.iter().any(|a| matches!(a, Act::Cross(Some(_), _, _))),
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
        round(&mut b, &[Act::Melee(1), Act::Hold, Act::Hold]);
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
        round(&mut b, &[Act::Hold, Act::Melee(2), Act::Hold]);
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
        let logs = round(&mut b, &[Act::Clash(1), Act::Hold]);
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
        // Tempo is snapshotted the same way: the Hero (a Vanguard) spends its reach in the Clash, nothing in the
        // Fire before it - so a transcript can charge the spend to the exact ring.
        assert_eq!(
            fire.tempo[0], b.units[0].cadence,
            "the Hero spent no tempo in the Fire step"
        );
        assert!(
            clash.tempo[0] < b.units[0].cadence,
            "and spent its reach in the Clash"
        );
    }

    /// **A tempo spend with no blow behind it is still recorded.** When a raider evades the line, the interceptor's
    /// reach lands nothing and the raider pays a slip cost - two tempo expenditures that produce *no* `Hit`. The
    /// per-phase tempo snapshot must still show them both, or the transcript would report a silent state change (a
    /// pool drained with nothing in the log to explain it). This is the data the fight log reads to make the evade
    /// cost visible.
    #[test]
    fn an_evaded_crossing_still_records_both_tempo_spends() {
        // A Raider in region 0 raids the foe cannon in region 1, evading the Wall that screens it.
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [6, 6, 1, 2, 2], true, false), // 0 - the crosser
                unit("Wall", Side::Foe, [1, 4, 6, 1, 2], true, false), // 1 - the screen (region 1 front)
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true), // 2 - the cannon it came for
            ],
            vec![0, 1, 1],
        );
        let (raider_tp, wall_tp) = (b.units[0].cadence, b.units[1].cadence);
        let logs = round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Evade, Volley::Dodge),
                Act::Hold,
                Act::Hold,
            ],
        );
        let intercept = logs
            .iter()
            .find(|l| l.phase == "Crossing Ring: Intercept")
            .expect("an Intercept phase");
        // The Wall reached and the Raider slipped it, so NOTHING landed - yet both paid.
        assert!(
            !intercept
                .hits
                .iter()
                .any(|h| h.attacker == 1 && h.target == 0),
            "the Wall's reach lands no blow - the Raider evaded it"
        );
        assert!(
            intercept.tempo[0] < raider_tp,
            "but the Raider still paid a slip cost (recorded, not invisible)"
        );
        assert!(
            intercept.tempo[1] < wall_tp,
            "and the Wall still paid to reach (recorded, not invisible)"
        );
        // The contest itself is recorded: the Wall's reach on the Raider, its bid, and that it was slipped - the
        // two numbers (bid vs. the Finesse-weighted slip) the log needs to explain why the escape worked.
        let reach = intercept
            .reaches
            .iter()
            .find(|r| r.attacker == 1 && r.target == 0)
            .expect("the Wall's reach on the Raider is recorded");
        assert!(reach.evaded, "and it is marked slipped");
        let wall_cards = wall_tp - intercept.tempo[1]; // the tempo the Wall committed to the reach
        assert_eq!(
            reach.bid,
            wall_cards * b.units[1].finesse,
            "with its bid = cards x Finesse (1 tempo x Finesse 2 = 2)"
        );
    }

    /// **A crossing is one event, in one phase.** The land moves a raider (rank -> Outrider, into the enemy region)
    /// AFTER the Crossing-ring snapshot is taken, so that move must be folded back into it. Otherwise the crossing
    /// reads its PRE-land rank (a raider still a Vanguard) and the real change resurfaces a phase later as a
    /// spurious second move. Assert the Volley snapshot already shows the landed Outrider in the enemy region.
    #[test]
    fn a_crossing_lands_in_its_own_phase() {
        let mut b = Board::new(
            vec![
                unit("Raider", Side::Party, [6, 8, 3, 3, 2], true, false), // 0 - crosses (Push), tanky enough to land
                unit("Wall", Side::Foe, [1, 4, 6, 1, 2], true, false), // 1 - the screen, region 1
                unit("Mage", Side::Foe, [5, 2, 1, 2, 2], false, true), // 2 - the cannon it came for
            ],
            vec![0, 1, 1],
        );
        let logs = round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Eat),
                Act::Hold,
                Act::Hold,
            ],
        );
        let volley = logs
            .iter()
            .find(|l| l.phase == "Crossing Ring: Volley")
            .expect("a Volley phase");
        assert!(
            volley.through.contains(&0),
            "the Raider crossed in the Volley phase"
        );
        assert_eq!(
            volley.ranks[0],
            Rank::Outrider,
            "and its snapshot ALREADY shows it landed as an Outrider"
        );
        assert_eq!(
            volley.regions[0], 1,
            "in the enemy region, this phase - not a phase later"
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
        round(&mut b, &[Act::Clash(1), Act::Clash(0), Act::Hold]);
        assert!(b.units[1].health < wall, "it sweeps their front line");
        assert_eq!(
            b.units[2].health, mage,
            "but must not touch the body behind that line"
        );
    }

    /// **Width comes free once the reach is paid for.** A melee area striker that crosses in sweeps the whole
    /// region it lands in - both tiers, because an outrider is past the screen. It **pushes** in: paying the reach
    /// in blood keeps its whole pool in hand to sweep with (an Evade against the pooled back-line volley would drain
    /// the very tempo the sweep needs).
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
        round(
            &mut b,
            &[
                Act::Cross(Some(2), Answer::Push, Volley::Eat),
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
        round(&mut b, &[Act::Melee(1), Act::Hold, Act::Hold]);
        assert!(
            b.units[1].health < wall,
            "it sweeps the front it is standing among"
        );
        assert!(
            b.units[2].health < mage,
            "AND the back - an outrider is past the screen"
        );
    }

    /// **A horde's bodies are separate Grit-strong pools - no spill.** An aimed blow fells at most ONE body, and
    /// only if it penetrates Grit; overkill and sub-Grit both waste, so another body needs another blow. A sweep
    /// still clears the WHOLE pack at once, if it penetrates. Width, not power, is what clears a pack cheaply.
    #[test]
    fn a_horde_defends_by_grit_and_a_sweep_must_penetrate_it() {
        // (attacker stats, aoe, horde Grit, pack) -> bodies felled in one round of Clash.
        let felled = |stats: [u8; 5], aoe: bool, grit: u8, pack: u32| -> u32 {
            let mut b = Board::new(
                vec![
                    unit("Hero", Side::Party, stats, true, true).with_aoe(aoe),
                    unit("Horde", Side::Foe, [1, pack as u8, grit, 1, 1], true, false)
                        .as_horde(true),
                ],
                vec![0, 1],
            );
            round(&mut b, &[Act::Clash(1), Act::Hold]);
            pack - b.units[1].health
        };
        // A sweep that PENETRATES Grit clears the whole pack, however big.
        for pack in [8, 20] {
            assert_eq!(
                felled([5, 3, 1, 1, 2], true, 4, pack),
                pack,
                "a Might-5 sweep clears a Grit-4 pack of {pack}"
            );
        }
        // A sweep that does NOT reach Grit clears nothing - Grit now bites.
        assert_eq!(
            felled([3, 3, 1, 1, 2], true, 4, 8),
            0,
            "a Might-3 sweep cannot dent a Grit-4 pack"
        );
        // Aimed fire fells ONE body per penetrating blow - no spill, so a big Might does not out-kill tempo. A
        // handful of blows -> a handful of bodies, nowhere near a spilling `floor(Might*blows / Grit)`.
        let aimed = felled([6, 6, 1, 2, 2], false, 4, 20);
        assert!(
            (1..=3).contains(&aimed),
            "aimed fells about one body per blow, not the pack: {aimed}"
        );
        // A sub-Grit aimed blow dents NOTHING - the pools do not accumulate across bodies.
        assert_eq!(
            felled([3, 6, 1, 2, 2], false, 4, 20),
            0,
            "a Might-3 aimed blow cannot penetrate a Grit-4 body"
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
            Some(Act::Cross(Some(1), Answer::Push, Volley::Eat)),
            "it goes through the line for the soft body behind it"
        );
    }

    /// **A tank holds its post - emergently, with no instinct tag.** A wall (Grit >= Might) is worth far more
    /// screening the front than loose in the enemy back, so the disruption heuristic's role-fit term scores holding
    /// above any raid on the creature's stats alone. What makes a wall a wall is now its numbers, not a flag.
    #[test]
    fn a_tank_holds_the_line_without_being_told() {
        let b = Board::new(
            vec![
                unit("Softie", Side::Party, [1, 1, 1, 1, 1], false, true),
                unit("Wall", Side::Foe, [3, 6, 3, 2, 2], true, false), // Might 3 <= Grit 3: a tank
                unit("Cannon", Side::Foe, [5, 2, 1, 2, 2], false, true),
            ],
            vec![0, 1, 1],
        );
        let hold = foe_acts(&b);
        assert!(
            !matches!(hold[1], Some(Act::Cross(..))),
            "a tank must never raid or slip out of the vanguard: {:?}",
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
            Act::Cross(Some(5), Answer::Evade, Volley::Dodge), // the Raider crosses in for their back line
            Act::Clash(3),
            Act::Clash(4),
            Act::Clash(0),
            Act::Cross(Some(2), Answer::Push, Volley::Eat), // the Duelist pushes through for our cannon
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
        round(&mut base, &acts);

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
                Act::Cross(Some(t), x, v) => Act::Cross(Some(inv[t]), x, v),
                Act::Melee(t) => Act::Melee(inv[t]),
                Act::Retreat(Some(t)) => Act::Retreat(Some(inv[t])),
                other => other,
            };

            let mut shuffled = Board::new(
                perm.iter().map(|&o| b.units[o].clone()).collect(),
                perm.iter().map(|&o| b.regions[o]).collect(),
            );
            let shuffled_acts: Vec<Act> = perm.iter().map(|&o| remap(acts[o].clone())).collect();
            round(&mut shuffled, &shuffled_acts);

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
        round(&mut a, &[Act::Hold, Act::Clash(0), Act::Clash(0)]);
        let mut b = build([("Hero", Side::Party), ("Y", Side::Foe), ("X", Side::Foe)]);
        round(&mut b, &[Act::Hold, Act::Clash(0), Act::Clash(0)]);
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
            round(&mut x, &acts);
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
            round(&mut b, &acts);
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
        round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
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
            round(&mut b, &[Act::Clash(1), Act::Hold]);
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
            round(&mut b, &acts);
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
        round(&mut b, &[Act::Clash(1), Act::Hold]);
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
        round(&mut c, &[Act::Clash(1), Act::Clash(0)]);
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
            round(&mut b, &[Act::Clash(1), Act::Clash(0)]);
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
