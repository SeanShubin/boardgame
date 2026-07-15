//! **v2 combat mechanics — the headless brain** (design brief §"Arena v2", locked 2026-07-09; memory
//! `manual-combat-design`). The plan-then-commit economy, implemented *fresh* per build-decision (B) — the
//! game layer owns combat, this module is pure logic over [`Combatant`]s (no board, no renderer). Stage 2
//! reads the `Board` into these and writes the resolved state back; the unit tests here are the safety net
//! (solver/balance re-validation deferred until the feel is tested).
//!
//! Each combat sub-phase is **three one-way mini-phases** (strict pipeline, no ping-pong):
//! 1. **Engage** — an attacker commits tempo to *reach* a target ([`Engage`]). Which ranks may reach which is
//!    the [`SCHEDULE`] gate. Committing more does not hit harder; it makes you harder to slip. The reach is a
//!    [`Contact`] edge, not yet established.
//! 2. **Evade** — the target now *sees what was committed*, so the price of escaping is exact. It either pays
//!    [`slip_cost`] in full and breaks every engagement reaching it, or it [`Dodge::Stand`]s and spends
//!    nothing. There is no partial slip: underpaying is never a gamble, only a waste, so it is impossible
//!    rather than merely bad.
//! 3. **Strike** — Finesse is done. Each established contact gives its engager **one** opening blow (paid for
//!    by the tempo it already committed, however much that was), and then either end of a **melee** edge may
//!    spend further tempo, one card per strike of Might. A **ranged** edge is one-way. Resolved as one
//!    **order-free, commit-based batch**: a committed blow lands even if its striker dies.
//!
//! The attack decision is therefore a single tension: every card you sink into *reaching* someone is a card
//! you cannot convert into a *blow*. Reach cheaply and they slip you; reach heavily and you arrive with
//! nothing left to swing.
//!
//! And a melee contact is **mutual** — the body you engaged may answer, even in a sub-phase the schedule never
//! paired it against you. It did not choose the fight. You could have let it pass; forcing the issue early is
//! what you pay for.
//!
//! Economy: **Tempo = Cadence** (a per-round pool, refreshed each round); **Finesse** decides *reach and
//! escape* only (`value = cards × finesse`) and never touches damage; **Might = damage**, applied via a
//! grit-accumulate model (a health card flips each time accumulated damage crosses `grit`).

use deckbound_content::rank::Intention as Rank;

/// Which side a combatant fights for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Party,
    Foe,
}

/// A combatant during a fight — the scratch unit v2 resolves over (built from the board in stage 2).
#[derive(Clone, Debug)]
pub struct Combatant {
    pub name: String,
    pub side: Side,
    pub rank: Rank,
    /// Damage each of this unit's strikes deals.
    pub might: u32,
    /// The bid multiplier: one flipped tempo card is worth `finesse` toward a bid.
    pub finesse: u32,
    /// Full tempo pool, refreshed each round.
    pub cadence: u32,
    /// The health bar: accumulated damage flips a health card each `grit` crossed.
    pub grit: u32,
    /// **Armor**: a flat reduction applied to *each individual strike* (`max(0, Might - armor)`). Unlike
    /// Grit (which scales how fast accumulated damage flips a card), armor is a *per-strike floor*: a
    /// strike whose Might does not exceed it deals **nothing**, and no amount of Cadence (more strikes)
    /// changes that. So armor is what makes high per-strike Might *necessary* rather than merely efficient.
    pub armor: u32,
    /// Carries a **melee** blow (effective in the Vanguard / as an Outrider). Independent of `ranged`.
    pub melee: bool,
    /// Carries a **ranged** shot (effective in the Rearguard). Independent of `melee`.
    pub ranged: bool,
    /// Carries an **area** strike (Sweep / Salvo): its strike hits *every* legal enemy in the target rank at
    /// once and is **unevadable** — but it spends only one tempo and cannot concentrate (no extra strikes),
    /// so it trades focus for coverage. The answer to a [`horde`](Self::horde); dead weight against one wall.
    pub aoe: bool,
    /// This combatant is a **horde** (spec 4.6 `Hoard X`): a built-in group of one-Health bodies (its
    /// `health` is the body count). A **single** strike **spills** - the penetrating damage overflows body to
    /// body, felling *Might-many* at once; an **area** strike hits every member at full value, clearing the
    /// whole pack. Its offense scales with the pack - it swarms with one tempo card per living body ("loses an
    /// attack per body killed"). The only group fielded in practice (heroes are ungrouped in the UI).
    pub horde: bool,
    /// Tempo cards left this round.
    pub tempo: u32,
    /// Face-up health cards remaining (Vitality at full); 0 ⇒ fallen at the next boundary.
    pub health: u32,
    /// Damage accumulated this sub-phase; wiped at the sub-phase boundary (sub-threshold damage never carries).
    pub(crate) pending: u32,
    pub fallen: bool,
}

impl Combatant {
    /// Build a fresh combatant at full Health (Vitality) and full Tempo (Cadence), pending 0, not fallen.
    /// `stats` is `[Might, Vitality, Grit, Cadence, Finesse]` (the catalog order); Finesse and Grit
    /// floor at 1 (matching how the arena reads a card). For the headless [`crate::battle`] / [`crate::solver`]
    /// tooling, which builds units from catalog specs rather than from the board.
    pub fn from_stats(
        name: impl Into<String>,
        side: Side,
        rank: Rank,
        stats: [u8; 5],
        armor: u32,
        melee: bool,
        ranged: bool,
    ) -> Self {
        let [might, vitality, grit, cadence, finesse] = stats.map(u32::from);
        Combatant {
            name: name.into(),
            side,
            rank,
            might,
            finesse: finesse.max(1),
            cadence,
            grit: grit.max(1),
            armor,
            melee,
            ranged,
            aoe: false,
            horde: false,
            tempo: cadence,
            health: vitality,
            pending: 0,
            fallen: false,
        }
    }

    /// Builder: mark this body as carrying an **area** strike (Sweep / Salvo). See [`Self::aoe`].
    pub fn with_aoe(mut self, aoe: bool) -> Self {
        self.aoe = aoe;
        self
    }

    /// Builder: make this a **horde** of one-Health bodies (its Vitality is the body count). See
    /// [`Self::horde`].
    pub fn as_horde(mut self, horde: bool) -> Self {
        self.horde = horde;
        self
    }

    /// Accumulate `might` damage, flipping a health card each time the pile crosses `grit`
    /// (deckbound's `take_with_toughness` semantics, inline). Never below zero.
    fn take(&mut self, might: u32) {
        let bar = self.grit.max(1);
        self.pending += might;
        while self.pending >= bar && self.health > 0 {
            self.pending -= bar;
            self.health -= 1;
        }
    }
}

/// An **engagement** declaration (mini-phase 1): `attacker` commits `cards` tempo to reach `target`.
///
/// The tempo is spent whatever happens. Committing more makes the target more expensive to slip - but it buys
/// **no extra damage**: however much you commit, contact yields exactly *one* opening strike. So every card
/// sunk into reaching them is a card you cannot convert into a blow, and that is the whole attack decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Engage {
    pub attacker: usize,
    pub target: usize,
    pub cards: u32,
}

/// A target's answer to everything reaching for it (mini-phase 2).
///
/// **There is no partial slip.** By this point the attacker's commitment is on the table, so the price of
/// slipping is known exactly - which means underpaying is never a gamble, it is knowingly burning tempo. The
/// rational alternative is always to stand still, let them come, and spend that tempo hitting back instead. So
/// a dominated move is not priced, it is made *impossible*: you either pay [`slip_cost`] in full, or you stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dodge {
    /// Spend nothing. They reach you - and on a melee edge you may answer, because *they came to you*.
    Stand,
    /// Pay [`slip_cost`] and break **every** engagement reaching you: one dodge covers your body.
    Slip,
}

/// A strike allocation (mini-phase 3): `unit` spends `cards` tempo for `cards` strikes of Might on `target`,
/// along an established [`Contact`]. Finesse is irrelevant here — contact is already made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blows {
    pub unit: usize,
    pub target: usize,
    pub cards: u32,
}

/// A **contact** edge: `attacker` reached `target`, having committed `bid` value (`cards × F_att`) to do it —
/// the value a slip must strictly exceed.
///
/// A **melee** contact is *mutual*: both ends may spend tempo striking along it. That is how a unit can hurt a
/// rank the schedule has not yet paired it against — it did not choose the fight, the fight came to it, and the
/// engager could have let it pass. A **ranged** contact is one-way: you cannot punch an archer at range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact {
    pub attacker: usize,
    pub target: usize,
    pub bid: u32,
}

// ---- bid math + legality (the atoms) --------------------------------------------------------------

/// Whether `atk` may strike `tgt` in sub-phase `sub` — the rank x phase [`SCHEDULE`] gate.
///
/// There is no re-aiming: the schedule is a complete 3x3 (every role has one slot against each enemy rank),
/// and **an empty target rank simply voids that pairing, for every role**. An Outrider facing no enemy
/// Rearguard loses its Raid exactly as a Vanguard facing no enemy Outriders loses its Intercept — and, like
/// the others, it still has its remaining slots in the Breach. See `schedule::SCHEDULE`.
pub fn legal_strike(sub: usize, atk: Rank, tgt: Rank) -> bool {
    deckbound_content::schedule::SCHEDULE
        .get(sub)
        .is_some_and(|pairs| pairs.contains(&(atk, tgt)))
}

/// Whether a strike thrown *from* `rank` is **ranged** — a Rearguard fires over its own line; a Vanguard or
/// Outrider strikes melee (someone approached). Range is position-determined (spec 4.2).
pub fn rank_is_ranged(rank: Rank) -> bool {
    matches!(rank, Rank::Rearguard)
}

/// Whether a body carrying these reaches is **effective** as an attacker in `rank`: it must carry the attack
/// type the position uses (a Rearguard needs `ranged`; a Vanguard / Outrider needs `melee`). A mismatch is
/// legal to *place* but lands nothing — the spec's "self-sort by attack type" (4.2), enforced as force, not
/// a ban: an ineffective body simply never forms a [`Contact`].
pub fn effective_in_rank(rank: Rank, melee: bool, ranged: bool) -> bool {
    if rank_is_ranged(rank) { ranged } else { melee }
}

/// The **screen** (back-access rule, spec 4.6): a Rearguard fights from behind its own line, so it is only
/// reachable once that line is gone. A Rearguard target may be hit by an **Outrider** (a raid slips past the
/// screen) at any time, but by a **Vanguard or Rearguard** attacker only once the target's side has no living
/// Vanguard — a dropped front opens the back. Non-Rearguard targets are never screened. This is what makes
/// fire from the back safe: while your Vanguard stands, nothing but a raiding Outrider can touch your
/// Rearguard, and that Outrider must first survive the Intercept + Volley gauntlet. Without this gate the
/// SCHEDULE's Breach pairs (V->R, R->R) would let a body reach an enemy Rearguard through a living screen.
pub fn back_access_ok(units: &[Combatant], attacker: Rank, target: usize) -> bool {
    let tgt = &units[target];
    if tgt.rank != Rank::Rearguard {
        return true; // only the Rearguard is screened
    }
    if attacker == Rank::Outrider {
        return true; // the raid bypasses the screen (paid for by the earlier sub-phases)
    }
    // A Vanguard / Rearguard attacker needs the target's front down first.
    !units
        .iter()
        .any(|u| u.side == tgt.side && u.rank == Rank::Vanguard && !u.fallen)
}

/// The **tempo cards** the attacker actually spent to open this contact. `bid` is their *value*
/// (`cards × F_att`), which is what a slip must beat - but the player thinks in cards, and the card count is
/// what they can see being flipped, so the log has to quote it. Exact, since `bid` is built from it.
pub fn reach_cards(units: &[Combatant], c: &Contact) -> u32 {
    c.bid / units[c.attacker].finesse.max(1)
}

/// The tempo `defender` must spend to **slip** — to break every engagement currently reaching it.
///
/// One dodge covers your body, so the price is set by the *largest* commitment against you: enough cards that
/// `cards × F_def` strictly exceeds it. `None` when nothing is reaching you (there is nothing to slip).
///
/// This is why Finesse defends without ever touching damage: a high-Finesse body is **cheap to slip with**,
/// hence expensive to catch, which forces attackers to commit more tempo — and every card they commit is a
/// card they cannot convert into a strike.
pub fn slip_cost(units: &[Combatant], contacts: &[Contact], defender: usize) -> Option<u32> {
    let worst = contacts
        .iter()
        .filter(|c| c.target == defender)
        .map(|c| c.bid)
        .max()?;
    Some(worst / units[defender].finesse.max(1) + 1)
}

/// The one enemy `unit` may pour strikes into this sub-phase, if any — the whole legality of the Strike step.
///
/// It is whoever it is in contact with: the target it engaged, or, on a **melee** edge, the attacker that
/// engaged *it*. The second case is the mutual-melee rule, and it is what lets a body hurt a rank the schedule
/// has not yet paired it against: it did not pick the fight. The engager could have let it pass, and chose not
/// to — that is the price of forcing the issue early.
pub fn strike_target(units: &[Combatant], contacts: &[Contact], unit: usize) -> Option<usize> {
    if let Some(c) = contacts.iter().find(|c| c.attacker == unit) {
        return Some(c.target);
    }
    // Answering along an edge someone else opened: you need a melee blow of your own, and their reach must have
    // been melee too - an archer shooting you from the back line never came within your reach.
    contacts
        .iter()
        .find(|c| c.target == unit && units[unit].melee && !rank_is_ranged(units[c.attacker].rank))
        .map(|c| c.attacker)
}

/// The damage a single strike of `might` deals through `armor`: `max(0, might - armor)`. Per **strike** — so
/// a Might below the armor deals nothing no matter how many strikes land (Cadence cannot penetrate armor).
fn strike(might: u32, armor: u32) -> u32 {
    might.saturating_sub(armor)
}

/// The health a single strike of `might` costs `target`: `strike(might, target.armor)`. Against a **horde**
/// (a group of one-Health bodies) this is **spillover** (spec 4.6): the penetrating damage is applied
/// point-by-point, and since each body is one-Health it overflows to the next on every kill — so a single
/// strike fells *penetrating-Might many* bodies, not one. (The spend-through is handled in [`apply`], which
/// takes the horde's health — its body count — straight down by this amount.)
fn hit(target: &Combatant, might: u32) -> u32 {
    strike(might, target.armor)
}

/// What a blow of `might` would **actually do** to `target` right now: `(health cards flipped, damage left
/// sitting in its pile, the bar it must cross)`.
///
/// Damage is not health. It banks into a per-sub-phase **pile** and only turns a Health card each time that
/// pile crosses the target's Grit; whatever is left is **wiped at the sub-phase boundary**. So a Might
/// under the bar flips nothing on its own — and quoting the raw Might to the player ("deal 7 back") is a
/// false promise. Quote this instead. It is still worth doing under the bar when other blows land on the same
/// target in the same sub-phase: the pile is shared, so damage adds up across attackers.
pub fn pile_effect(target: &Combatant, might: u32) -> (u32, u32, u32) {
    pile_effect_strikes(target, might, 1)
}

/// [`pile_effect`] for `strikes` blows of `might` landing together (Extra strikes: armor bites **per strike**,
/// and all of them bank into the one pile).
pub fn pile_effect_strikes(target: &Combatant, might: u32, strikes: u32) -> (u32, u32, u32) {
    let dmg = hit(target, might) * strikes;
    if target.horde {
        // A horde has no bar to cross: each body is one Health and penetrating damage spills body to body.
        return (dmg.min(target.health), 0, 1);
    }
    let bar = target.grit.max(1);
    let pile = target.pending + dmg;
    let flips = (pile / bar).min(target.health);
    (flips, pile - flips * bar, bar)
}

// ---- the three mini-phases ------------------------------------------------------------------------

/// **Engage.** Spend each attacker's committed tempo and record the reach as a [`Contact`].
///
/// The contact is not yet *established* — the target may pay [`slip_cost`] to break it at the Evade step. The
/// caller (UI) has already gated legality via [`legal_strike`]; the range and screen gates below are the
/// mechanical backstop.
pub fn resolve_engage(units: &mut [Combatant], engagements: &[Engage]) -> Vec<Contact> {
    let mut contacts = Vec::new();
    for c in engagements {
        // Range gate (mechanics backstop, spec 4.2): a body whose reach does not match its position lands
        // nothing here — no contact, no tempo spent. The UI already hides these; this makes it a rule.
        let atk = &units[c.attacker];
        if !effective_in_rank(atk.rank, atk.melee, atk.ranged) {
            continue;
        }
        // Screen backstop (spec 4.6): a strike aimed past a living front at a screened Rearguard lands
        // nothing here — no contact, no tempo spent. The candidate generators already hide these.
        if !back_access_ok(units, atk.rank, c.target) {
            continue;
        }
        // Area strike (Sweep / Salvo): unslippable, hits the targeted **group** for one sweep of Might, for a
        // single tempo card. Its edge is over a horde - one sweep clears the whole pack, where a single strike
        // fells one body. Against a normal body (a group of one) it is just one unslippable hit. It forms no
        // Contact - so it cannot be slipped, and cannot be poured into either: coverage bought at the price of
        // concentration. Damage is applied right here.
        if units[c.attacker].aoe {
            if units[c.attacker].tempo == 0 {
                continue;
            }
            units[c.attacker].tempo -= 1;
            let might = units[c.attacker].might;
            let t = c.target;
            if hit(&units[t], might) > 0 {
                if units[t].horde {
                    units[t].health = 0; // clears the whole targeted group at once
                } else {
                    units[t].take(strike(might, units[t].armor));
                }
            }
            continue;
        }
        let cards = c.cards.min(units[c.attacker].tempo);
        if cards == 0 {
            continue; // you cannot reach for someone without committing to it
        }
        units[c.attacker].tempo -= cards;
        contacts.push(Contact {
            attacker: c.attacker,
            target: c.target,
            bid: cards * units[c.attacker].finesse,
        });
    }
    contacts
}

/// **Evade.** `dodges` is index-aligned with `units`. A [`Dodge::Slip`] pays [`slip_cost`] and breaks **every**
/// engagement reaching that unit; a [`Dodge::Stand`] spends nothing. Returns the *established* contacts.
///
/// A slip the unit cannot afford is not a failed slip - it is not a slip at all. The UI bars it and quotes the
/// price; this is the backstop, and it stands rather than burning tempo for nothing.
pub fn resolve_evade(
    units: &mut [Combatant],
    contacts: &[Contact],
    dodges: &[Dodge],
) -> Vec<Contact> {
    let mut slipped = vec![false; units.len()];
    for (i, dodge) in dodges.iter().enumerate() {
        if *dodge != Dodge::Slip {
            continue;
        }
        let Some(cost) = slip_cost(units, contacts, i) else {
            continue; // nothing reaching you
        };
        if cost > units[i].tempo {
            continue; // cannot afford it: you stand (and keep your tempo for the Strike step)
        }
        units[i].tempo -= cost;
        slipped[i] = true;
    }
    contacts
        .iter()
        .filter(|c| !slipped[c.target])
        .copied()
        .collect()
}

/// **Strike.** Every established contact gives its **engager** one opening blow — paid for by the tempo it
/// already committed, however much that was. Then `blows` spends *further* tempo, one card per strike of Might,
/// from either end of a **melee** edge (a ranged edge is one-way; see [`strike_target`]).
///
/// One order-free, commit-based batch: every blow is collected before any is applied, so a committed strike
/// lands even if its striker dies to a simultaneous one, and mutual deaths resolve cleanly.
pub fn resolve_strike(units: &mut [Combatant], contacts: &[Contact], blows: &[Blows]) {
    let mut damage = vec![0u32; units.len()];
    // The opening blow: one strike, however much tempo bought the reach. This is what makes over-committing
    // cost you - the tempo is gone and it bought exactly one hit.
    for c in contacts {
        damage[c.target] += hit(&units[c.target], units[c.attacker].might);
    }
    for b in blows {
        let cards = b.cards.min(units[b.unit].tempo);
        units[b.unit].tempo -= cards;
        damage[b.target] += hit(&units[b.target], units[b.unit].might) * cards; // per strike
    }
    apply(units, &damage);
}

/// Apply an order-free damage vector to the units. For a normal body `damage` is accumulated Might (fed
/// through the grit pile); for a **horde** it is a **body count** (each penetrating strike already
/// counted one body via [`hit`]), so it comes straight off health — bodies are one-Health, nothing to
/// accumulate.
fn apply(units: &mut [Combatant], damage: &[u32]) {
    for (unit, &dmg) in units.iter_mut().zip(damage) {
        if dmg > 0 {
            if unit.horde {
                unit.health = unit.health.saturating_sub(dmg);
            } else {
                unit.take(dmg);
            }
        }
    }
}

/// End a sub-phase: mark units at zero health fallen. **That is all it does now** — the sub-phase boundary is
/// where the dead stop fighting, not where wounds close.
///
/// The damage pile used to be wiped here, which made Grit a *per-sub-phase concentration gate*: land less
/// than T in one sub-phase and you accomplished literally nothing, and there was no way to see that you had
/// accomplished nothing. Wounds now carry across the sub-phases of a round and close at the [Reset](refresh_round).
///
/// Death is still settled here, and that matters: it keeps the order-free, commit-based batch intact - a
/// committed blow lands even if its striker dies in the same sub-phase, and mutual deaths resolve cleanly.
pub fn end_sub_phase(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        if u.health == 0 {
            u.fallen = true;
        }
    }
}

/// **The Reset** — the round boundary. Tempo stands back up (leftover tempo does not carry across rounds), and
/// the accumulated damage pile **closes**: sub-threshold damage that never turned a Health card is gone.
///
/// A normal body gets its Cadence back; a **horde** gets one card per living body (`health`), so a full pack
/// swarms with many strikes and a thinned one dwindles — which is why chipping it single-file loses and an area
/// clear wins.
///
/// This is the one deadline in a fight: a wound you cannot finish *this round* is a wound you did not inflict.
/// So Grit still demands concentration - but over a whole round's five sub-phases, a grain the player can
/// actually plan at, rather than within a single sub-phase where a blow could vanish unremarked.
pub fn refresh_round(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        u.tempo = if u.horde { u.health.max(1) } else { u.cadence };
        u.pending = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(
        name: &str,
        side: Side,
        rank: Rank,
        might: u32,
        finesse: u32,
        cadence: u32,
        grit: u32,
        health: u32,
    ) -> Combatant {
        Combatant {
            name: name.into(),
            side,
            rank,
            might,
            finesse,
            cadence,
            grit,
            armor: 0,
            melee: true,
            ranged: false,
            aoe: false,
            horde: false,
            tempo: cadence,
            health,
            pending: 0,
            fallen: false,
        }
    }

    /// **The price of escape is exact, and it is set by the largest reach against you.** One dodge covers your
    /// body, so ganging up does not multiply the cost - but the heaviest commitment does raise it.
    #[test]
    fn slipping_is_priced_off_the_heaviest_reach() {
        let u = vec![
            unit("A", Side::Foe, Rank::Vanguard, 2, 2, 4, 1, 5),
            unit("B", Side::Foe, Rank::Vanguard, 2, 1, 4, 1, 5),
            unit("D", Side::Party, Rank::Vanguard, 3, 2, 4, 1, 5), // Finesse 2
        ];
        let reach = |bid| Contact {
            attacker: 0,
            target: 2,
            bid,
        };
        // Value 4 vs Finesse 2: 2 cards is only 4, which does not STRICTLY exceed it. 3 cards does.
        assert_eq!(slip_cost(&u, &[reach(4)], 2), Some(3));
        // A second, lighter reach changes nothing - you dodge once.
        let two = vec![
            reach(4),
            Contact {
                attacker: 1,
                target: 2,
                bid: 1,
            },
        ];
        assert_eq!(slip_cost(&u, &two, 2), Some(3));
        assert_eq!(slip_cost(&u, &[], 2), None, "nothing to slip");
    }

    #[test]
    fn reach_must_match_position() {
        // Position sets the required attack type; the body must carry it.
        assert!(effective_in_rank(Rank::Vanguard, true, false)); // melee up front: yes
        assert!(!effective_in_rank(Rank::Vanguard, false, true)); // ranged up front: dead weight
        assert!(effective_in_rank(Rank::Rearguard, false, true)); // ranged in back: yes
        assert!(!effective_in_rank(Rank::Rearguard, true, false)); // melee in back: dead weight
        assert!(effective_in_rank(Rank::Rearguard, true, true)); // carries both: fine anywhere
    }

    #[test]
    fn catch_gated_by_reach() {
        // A melee-only Rearguard cannot form a contact even where the SCHEDULE permits the rank pair
        // (Breach R->R). No tempo is spent on the fizzled strike.
        let mut mismatch = vec![
            unit("A", Side::Party, Rank::Rearguard, 2, 2, 3, 1, 3), // melee-only (helper default)
            unit("D", Side::Foe, Rank::Rearguard, 1, 1, 2, 1, 3),
        ];
        let strike = Engage {
            attacker: 0,
            target: 1,
            cards: 2,
        };
        assert!(
            resolve_engage(&mut mismatch, &[strike]).is_empty(),
            "melee body fires nothing from the back"
        );
        assert_eq!(
            mismatch[0].tempo, 3,
            "no tempo spent on an ineffective strike"
        );

        // Give it a ranged reach and the same strike lands.
        let mut ranged = mismatch.clone();
        ranged[0].ranged = true;
        assert_eq!(
            resolve_engage(&mut ranged, &[strike]).len(),
            1,
            "a ranged body fires from the back"
        );
    }

    #[test]
    fn screen_gates_the_back_line() {
        // Foe side: a Vanguard screening a Rearguard. Party attackers try to reach the screened Rearguard (2).
        let mut units = vec![
            unit("V", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 3),
            unit("O", Side::Party, Rank::Outrider, 2, 2, 3, 1, 3),
            unit("Screen", Side::Foe, Rank::Vanguard, 2, 2, 3, 1, 3),
            unit("Back", Side::Foe, Rank::Rearguard, 2, 2, 3, 1, 3),
        ];
        // While the enemy front stands, a Vanguard / Rearguard cannot reach the enemy Rearguard...
        assert!(
            !back_access_ok(&units, Rank::Vanguard, 3),
            "screened from the front"
        );
        assert!(
            !back_access_ok(&units, Rank::Rearguard, 3),
            "screened from the back"
        );
        // ...but a raiding Outrider slips past the screen at any time.
        assert!(
            back_access_ok(&units, Rank::Outrider, 3),
            "the raid bypasses the screen"
        );
        // Non-Rearguard targets are never screened.
        assert!(
            back_access_ok(&units, Rank::Vanguard, 2),
            "the front itself is always reachable"
        );

        // Crush the front and the back opens ("bring in the heavy guns"): R->R now lands.
        units[2].fallen = true;
        assert!(
            back_access_ok(&units, Rank::Rearguard, 3),
            "a dropped screen opens the back"
        );

        // Both sides all-Rearguard (no screens anywhere) -> a free-for-all: Rearguards fire on each other.
        let ffa = vec![
            unit("P", Side::Party, Rank::Rearguard, 2, 2, 3, 1, 3),
            unit("F", Side::Foe, Rank::Rearguard, 2, 2, 3, 1, 3),
        ];
        assert!(
            back_access_ok(&ffa, Rank::Rearguard, 1),
            "no screen, no safety"
        );
    }

    /// **A melee contact is mutual; a ranged one is not.** The body you engaged may answer along the edge -
    /// even in a sub-phase the schedule never paired it against you. It did not choose the fight, and you could
    /// have let it pass. But you cannot punch an archer that is shooting you from the back line.
    #[test]
    fn melee_contact_is_mutual_and_ranged_contact_is_one_way() {
        let u = vec![
            unit("Wall", Side::Foe, Rank::Vanguard, 2, 2, 4, 1, 5), // melee reach (a Vanguard)
            unit("Raider", Side::Party, Rank::Outrider, 3, 2, 4, 1, 5),
            unit("Archer", Side::Foe, Rank::Rearguard, 2, 2, 4, 1, 5), // ranged reach
        ];
        let melee = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };
        // The engager may always pour into its own edge...
        assert_eq!(strike_target(&u, &[melee], 0), Some(1));
        // ...and the body it reached may answer, because the reach was melee.
        assert_eq!(strike_target(&u, &[melee], 1), Some(0));

        let shot = Contact {
            attacker: 2,
            target: 1,
            bid: 4,
        };
        assert_eq!(strike_target(&u, &[shot], 2), Some(1), "the archer shoots");
        assert_eq!(
            strike_target(&u, &[shot], 1),
            None,
            "nothing answers a shot from the back line"
        );

        // ...and a body with no melee blow of its own has nothing to answer with either.
        let mut no_melee = u.clone();
        no_melee[1].melee = false;
        no_melee[1].ranged = true;
        assert_eq!(strike_target(&no_melee, &[melee], 1), None);
    }

    /// **KNOWN DEFECT, recorded rather than fixed.** [`strike_target`] answers "who am I in contact with?" by
    /// taking the **first** matching contact. So a body closed on by *two* attackers answers whichever of them
    /// happens to come first in the contact list - which is **seat order**.
    ///
    /// This test asserts the *current* behaviour, so the defect is pinned and cannot be forgotten, and so that
    /// any fix has to come here and delete it deliberately. It is **not** a statement that the behaviour is
    /// right. It is not: Spec 1.9 requires that "permuting the seat order of a tier's duels must yield the
    /// identical end-state - any divergence is an order-dependent mechanic, i.e. a bug."
    ///
    /// It is live. `strike_target` is called from `arena.rs` (the played game), `battle.rs` (the sim) and
    /// `solver.rs` (the doom oracle), so **whenever two bodies clash one, who it hits back is decided by who was
    /// loaded first** - and the oracle inherits the same arbitrariness while claiming certainty.
    ///
    /// The fix is not to sort the contacts. Sorting only writes the arbitrariness down. *Whom you answer* is a
    /// real decision, and it wants to be a **declared** one - which is what the regions model (`rules::combat`) now does ("you fight
    /// who you declared"). Doing the same here is a behaviour change with balance consequences, so it is a
    /// deliberate call, not a drive-by.
    #[test]
    fn strike_target_picks_its_answer_by_seat_order_and_that_is_a_bug() {
        let u = vec![
            unit("Hero", Side::Party, Rank::Vanguard, 2, 2, 4, 1, 5),
            unit("X", Side::Foe, Rank::Vanguard, 2, 2, 4, 1, 5),
            unit("Y", Side::Foe, Rank::Vanguard, 2, 2, 4, 1, 5),
        ];
        let from_x = Contact {
            attacker: 1,
            target: 0,
            bid: 4,
        };
        let from_y = Contact {
            attacker: 2,
            target: 0,
            bid: 4,
        };

        // Same fight. Same bodies. The only difference is which contact was pushed first.
        assert_eq!(
            strike_target(&u, &[from_x, from_y], 0),
            Some(1),
            "it answers X..."
        );
        assert_eq!(
            strike_target(&u, &[from_y, from_x], 0),
            Some(2),
            "...and the identical position answers Y, purely because the list was built the other way round"
        );
    }

    #[test]
    fn schedule_gates_catches() {
        // Intercept (sub-phase 0) is Vanguard -> Outrider only.
        assert!(legal_strike(0, Rank::Vanguard, Rank::Outrider));
        assert!(!legal_strike(0, Rank::Vanguard, Rank::Vanguard));
        assert!(!legal_strike(0, Rank::Rearguard, Rank::Outrider));
        // Clash (sub-phase 3) has (Rearguard,Vanguard) and (Vanguard,Vanguard).
        assert!(legal_strike(3, Rank::Vanguard, Rank::Vanguard));
    }

    /// **The Outrider's slots are the same three everyone gets - only the timing differs.** Its Rearguard slot
    /// comes early (the Raid); the other two land at the Breach, with everything else deep. An earlier version
    /// let the Raid re-aim down a priority list and deleted the Breach pairs, leaving the Outrider with one
    /// slot to every other role's three.
    #[test]
    fn the_outrider_raids_early_and_breaches_late() {
        const RAID: usize = 2;
        const BREACH: usize = 4;
        assert!(legal_strike(RAID, Rank::Outrider, Rank::Rearguard));
        assert!(!legal_strike(RAID, Rank::Outrider, Rank::Vanguard));
        assert!(legal_strike(BREACH, Rank::Outrider, Rank::Vanguard));
        assert!(legal_strike(BREACH, Rank::Outrider, Rank::Outrider));
    }

    #[test]
    fn engaging_spends_tempo_and_records_the_reach() {
        let mut units = vec![
            unit("A", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 3),
            unit("D", Side::Foe, Rank::Outrider, 1, 3, 2, 1, 3),
        ];
        // A commits 2 cards at F2 -> a reach worth 4. It lands unless D pays to escape it.
        let reaching = resolve_engage(
            &mut units,
            &[Engage {
                attacker: 0,
                target: 1,
                cards: 2,
            }],
        );
        assert_eq!(reaching.len(), 1);
        assert_eq!(reaching[0].bid, 4, "value = cards x Finesse");
        assert_eq!(units[0].tempo, 1, "spent 2 of 3 tempo reaching");
    }

    /// **Slip or stand - there is no third answer.** A slip pays the exact price and breaks *everything*
    /// reaching you; standing spends nothing. A slip you cannot afford is not a failed slip, it is not a slip:
    /// you stand, and you keep the tempo.
    #[test]
    fn a_slip_is_paid_in_full_or_not_at_all() {
        let base = || {
            vec![
                unit("A", Side::Party, Rank::Vanguard, 3, 2, 4, 1, 3),
                unit("D", Side::Foe, Rank::Outrider, 2, 3, 4, 1, 3), // Finesse 3, tempo 4
            ]
        };
        let reach = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };

        // Slip: 4 / 3 + 1 = 2 cards. The edge is gone, and no blow can land along it.
        let mut u = base();
        let kept = resolve_evade(&mut u, &[reach], &[Dodge::Stand, Dodge::Slip]);
        assert!(kept.is_empty(), "the slipped edge is gone");
        assert_eq!(u[1].tempo, 2, "paid 2 tempo to escape");

        // Stand: nothing spent, the edge stands.
        let mut u = base();
        let kept = resolve_evade(&mut u, &[reach], &[Dodge::Stand, Dodge::Stand]);
        assert_eq!(kept.len(), 1);
        assert_eq!(u[1].tempo, 4, "standing costs nothing");

        // Cannot afford it: you stand and keep every card. There is no half-slip to burn tempo on.
        let mut u = base();
        u[1].tempo = 1; // needs 2
        let kept = resolve_evade(&mut u, &[reach], &[Dodge::Stand, Dodge::Slip]);
        assert_eq!(kept.len(), 1, "it still reaches you");
        assert_eq!(u[1].tempo, 1, "and you did NOT burn the tempo trying");
    }

    /// **Reaching buys exactly one blow, however much it cost - and then every further card is a blow.** This
    /// is the whole attack tension: tempo sunk into reaching them is tempo you cannot swing with.
    #[test]
    fn contact_gives_one_opening_blow_then_one_blow_per_card() {
        let mut u = vec![
            unit("A", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 5),
            unit("D", Side::Foe, Rank::Outrider, 1, 3, 2, 1, 5),
        ];
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 2,
        };
        // The opening blow alone: Might 2, Grit 1 -> 2 health cards.
        resolve_strike(&mut u, &[contact], &[]);
        assert_eq!(u[1].health, 3, "one opening blow, free");
        assert_eq!(u[0].tempo, 3, "and it cost nothing further");

        // Pour 3 more cards in: 3 blows x Might 2 = 6 more damage.
        resolve_strike(
            &mut u,
            &[],
            &[Blows {
                unit: 0,
                target: 1,
                cards: 3,
            }],
        );
        assert_eq!(u[0].tempo, 0);
        assert_eq!(u[1].health, 0, "6 more damage at grit 1");
    }

    /// The Strike step is one **order-free, commit-based batch**: a blow lands even if its striker dies to a
    /// simultaneous one, so a doomed body still answers.
    #[test]
    fn strikes_are_a_commit_based_batch() {
        let mut u = vec![
            unit("A", Side::Foe, Rank::Vanguard, 3, 2, 4, 1, 3),
            unit("D", Side::Party, Rank::Vanguard, 2, 2, 4, 1, 3),
        ];
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };
        // A's opening blow kills D (3 might, grit 1, 3 health); D answers along the mutual melee edge
        // with its last card, and the answer still lands.
        resolve_strike(
            &mut u,
            &[contact],
            &[Blows {
                unit: 1,
                target: 0,
                cards: 2,
            }],
        );
        assert_eq!(u[1].health, 0, "D is dead");
        assert_eq!(u[0].health, 0, "and it took D with it - 2 blows x Might 2");
    }

    #[test]
    fn round_refresh_restores_tempo_boundary_marks_fallen() {
        let mut u = vec![unit("A", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 1)];
        u[0].tempo = 0;
        refresh_round(&mut u);
        assert_eq!(u[0].tempo, 3, "tempo refreshed to Cadence");
        u[0].health = 0;
        end_sub_phase(&mut u);
        assert!(u[0].fallen);
    }
}
