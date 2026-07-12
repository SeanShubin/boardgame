//! **v2 combat mechanics — the headless brain** (design brief §"Arena v2", locked 2026-07-09; memory
//! `manual-combat-design`). The plan-then-commit economy, implemented *fresh* per build-decision (B) — the
//! game layer owns combat, this module is pure logic over [`Combatant`]s (no board, no renderer). Stage 2
//! reads the `Board` into these and writes the resolved state back; the unit tests here are the safety net
//! (solver/balance re-validation deferred until the feel is tested).
//!
//! Each combat sub-phase is **three one-way mini-phases** (strict pipeline, no ping-pong):
//! 1. **Strike** — an attacker bids tempo to reach a target; the strike lands when `cards × F_att ≥ F_target`
//!    (may over-flip to raise the bar an evade must clear). Which ranks may strike which is the [`SCHEDULE`]
//!    gate. A landed strike is a [`Contact`] edge.
//! 2. **React** — per incoming strike the defender **eats** (free, default), **evades** (`cards × F_def >` the
//!    attacker's *spent* bid → the hit misses and the edge breaks), or **strikes back** (1 card, unevadable —
//!    take the hit *and* counter). Resolved as one **order-free, commit-based batch**: a committed strike
//!    lands even if its unit dies, and a doomed soaker still ripostes.
//! 3. **Extra strikes** — finesse-free: units still on an un-evaded [`Contact`] flip **remaining** tempo for
//!    extra hits (1 card = 1 strike of Might, unevadable).
//!
//! Economy: **Tempo = Cadence** (a per-round pool, refreshed each round); **Finesse** is the bid multiplier
//! (bid value = `cards × finesse`); **Might = damage**, applied via a toughness-accumulate model (a health
//! card flips each time accumulated damage crosses `toughness`) — tempo *never* changes how hard a hit is,
//! only whether it lands and how many land.

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
    /// The health bar: accumulated damage flips a health card each `toughness` crossed.
    pub toughness: u32,
    /// **Armor**: a flat reduction applied to *each individual strike* (`max(0, Might - armor)`). Unlike
    /// Toughness (which scales how fast accumulated damage flips a card), armor is a *per-strike floor*: a
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
    /// `stats` is `[Might, Vitality, Toughness, Cadence, Finesse]` (the catalog order); Finesse and Toughness
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
        let [might, vitality, toughness, cadence, finesse] = stats.map(u32::from);
        Combatant {
            name: name.into(),
            side,
            rank,
            might,
            finesse: finesse.max(1),
            cadence,
            toughness: toughness.max(1),
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

    /// Accumulate `might` damage, flipping a health card each time the pile crosses `toughness`
    /// (deckbound's `take_with_toughness` semantics, inline). Never below zero.
    fn take(&mut self, might: u32) {
        let bar = self.toughness.max(1);
        self.pending += might;
        while self.pending >= bar && self.health > 0 {
            self.pending -= bar;
            self.health -= 1;
        }
    }
}

/// A strike declaration: `attacker` bids `cards` tempo to reach `target` (indices into the combatant slice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strike {
    pub attacker: usize,
    pub target: usize,
    pub cards: u32,
}

/// A defender's reaction to one incoming [`Contact`] (the React mini-phase).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum React {
    /// Take the hit, free. The default.
    Eat,
    /// Flip `cards` tempo; if `cards × F_def >` the attacker's spent bid the hit misses and the edge breaks.
    Evade { cards: u32 },
    /// 1 card, unevadable: take the hit *and* land a counter.
    StrikeBack,
}

/// An extra-strike allocation (mini-phase 3): `attacker` flips `cards` tempo, each a Might strike on `target`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtraStrike {
    pub attacker: usize,
    pub target: usize,
    pub cards: u32,
}

/// A landed contact edge: `attacker` connected on `target`, having spent `bid` value (`cards × F_att`) —
/// the value an evade must strictly exceed.
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

/// A strike lands when the attacker's bid value reaches the target's finesse: `cards × F_att ≥ F_target`.
pub fn catch_lands(cards: u32, f_att: u32, f_target: u32) -> bool {
    cards * f_att >= f_target
}

/// An evade succeeds when the defender's bid **strictly exceeds** the attacker's spent value.
pub fn evade_succeeds(cards: u32, f_def: u32, atk_spent: u32) -> bool {
    cards * f_def > atk_spent
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
/// pile crosses the target's Toughness; whatever is left is **wiped at the sub-phase boundary**. So a Might
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
    let bar = target.toughness.max(1);
    let pile = target.pending + dmg;
    let flips = (pile / bar).min(target.health);
    (flips, pile - flips * bar, bar)
}

// ---- the three mini-phases ------------------------------------------------------------------------

/// **Strike.** Spend each attacker's bid (capped at its remaining tempo) and keep the strikes that land as
/// [`Contact`] edges (`bid = cards × F_att`, the value an evade must beat). The caller (UI) has already
/// gated legality via [`legal_strike`].
pub fn resolve_strike(units: &mut [Combatant], strikes: &[Strike]) -> Vec<Contact> {
    let mut contacts = Vec::new();
    for c in strikes {
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
        // Area strike (Sweep / Salvo): unevadable, hits the targeted **group** for one sweep of Might, for a
        // single tempo card. Its edge is over a horde - one sweep clears the whole pack, where a single strike
        // fells one body. Against a normal body (a group of one) it is just one unevadable hit. It forms no
        // Contact - no React to an area, and so no extra-strikes phase: coverage bought at the price of
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
        units[c.attacker].tempo -= cards;
        let f_att = units[c.attacker].finesse;
        let f_tgt = units[c.target].finesse;
        if cards > 0 && catch_lands(cards, f_att, f_tgt) {
            contacts.push(Contact {
                attacker: c.attacker,
                target: c.target,
                bid: cards * f_att,
            });
        }
    }
    contacts
}

/// **React.** `reactions` is index-aligned with `contacts`. Resolve as one order-free, commit-based batch:
/// collect every strike + counter first (a committed strike lands even if its unit dies), then apply. Returns
/// the surviving (un-evaded) contact edges — the ones the Extra-strikes phase runs along.
pub fn resolve_react(
    units: &mut [Combatant],
    contacts: &[Contact],
    reactions: &[React],
) -> Vec<Contact> {
    let mut damage = vec![0u32; units.len()];
    let mut surviving = Vec::new();
    for (contact, react) in contacts.iter().zip(reactions) {
        let (atk, tgt) = (contact.attacker, contact.target);
        match *react {
            React::Eat => {
                damage[tgt] += hit(&units[tgt], units[atk].might);
                surviving.push(*contact);
            }
            React::Evade { cards } => {
                let cards = cards.min(units[tgt].tempo);
                units[tgt].tempo -= cards;
                if evade_succeeds(cards, units[tgt].finesse, contact.bid) {
                    // miss + break contact: no damage, dropped from the surviving edges.
                } else {
                    damage[tgt] += hit(&units[tgt], units[atk].might); // evade failed - hit lands
                    surviving.push(*contact);
                }
            }
            React::StrikeBack => {
                damage[tgt] += hit(&units[tgt], units[atk].might); // take the hit...
                // ...and counter, but only **melee-vs-melee**: you strike back at a foe that *approached*
                // you (a melee strike), and only if you carry a melee blow. Against a ranged shot, or with
                // no melee of your own, there is nothing to answer with - you simply eat it. One Tempo card,
                // Finesse-irrelevant; commit-based (the counter lands even if the soaker dies).
                let incoming_melee = !rank_is_ranged(units[atk].rank);
                if incoming_melee && units[tgt].melee && units[tgt].tempo > 0 {
                    units[tgt].tempo -= 1;
                    damage[atk] += hit(&units[atk], units[tgt].might);
                }
                surviving.push(*contact);
            }
        }
    }
    apply(units, &damage);
    surviving
}

/// **Extra strikes.** Each allocation flips `cards` tempo for `cards` Might strikes on `target`, unevadable.
/// One order-free batch. The caller (UI) constrains allocations to units still on a surviving [`Contact`].
pub fn resolve_extra(units: &mut [Combatant], extras: &[ExtraStrike]) {
    let mut damage = vec![0u32; units.len()];
    for e in extras {
        let cards = e.cards.min(units[e.attacker].tempo);
        units[e.attacker].tempo -= cards;
        damage[e.target] += hit(&units[e.target], units[e.attacker].might) * cards; // per strike
    }
    apply(units, &damage);
}

/// Apply an order-free damage vector to the units. For a normal body `damage` is accumulated Might (fed
/// through the toughness pile); for a **horde** it is a **body count** (each penetrating strike already
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

/// End a sub-phase: wipe the accumulated (sub-threshold) damage pile and mark units at zero health fallen.
pub fn end_sub_phase(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        u.pending = 0;
        if u.health == 0 {
            u.fallen = true;
        }
    }
}

/// Start a round: refresh every unit's tempo (leftover tempo does not carry across rounds). A normal body
/// gets its Cadence; a **horde** gets one card per living body (`health`), so a full pack swarms with many
/// strikes and a thinned one dwindles — which is why chipping it single-file loses and an area clear wins.
pub fn refresh_round(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        u.tempo = if u.horde { u.health.max(1) } else { u.cadence };
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
        toughness: u32,
        health: u32,
    ) -> Combatant {
        Combatant {
            name: name.into(),
            side,
            rank,
            might,
            finesse,
            cadence,
            toughness,
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

    #[test]
    fn bid_math() {
        // Strike: F2 striking F3 needs 2 cards (2×2=4 ≥ 3); 1 card (1×2=2) falls short.
        assert!(!catch_lands(1, 2, 3));
        assert!(catch_lands(2, 2, 3));
        // Evade must STRICTLY exceed the attacker's spent value.
        assert!(!evade_succeeds(2, 2, 4)); // 4 is not > 4
        assert!(evade_succeeds(3, 2, 4)); // 6 > 4
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
        let strike = Strike {
            attacker: 0,
            target: 1,
            cards: 2,
        };
        assert!(
            resolve_strike(&mut mismatch, &[strike]).is_empty(),
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
            resolve_strike(&mut ranged, &[strike]).len(),
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

    #[test]
    fn strikeback_is_melee_versus_melee_only() {
        // Incoming melee (attacker is a Vanguard), defender carries melee -> counters.
        let mut u = vec![
            unit("A", Side::Foe, Rank::Vanguard, 2, 2, 4, 1, 5),
            unit("D", Side::Party, Rank::Vanguard, 3, 2, 4, 1, 5),
        ];
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };
        resolve_react(&mut u, &[contact], &[React::StrikeBack]);
        assert_eq!(
            u[0].health, 2,
            "melee counter landed (5 -> 2 at might 3, toughness 1)"
        );
        assert_eq!(u[1].tempo, 3, "spent 1 Tempo on the strike-back");

        // Incoming ranged (attacker is a Rearguard): no strike-back, just take the hit.
        let mut u = vec![
            unit("A", Side::Foe, Rank::Rearguard, 2, 2, 4, 1, 5),
            unit("D", Side::Party, Rank::Vanguard, 3, 2, 4, 1, 5),
        ];
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };
        resolve_react(&mut u, &[contact], &[React::StrikeBack]);
        assert_eq!(u[0].health, 5, "no counter against a ranged shot");
        assert_eq!(
            u[1].tempo, 4,
            "no Tempo spent - nothing to strike back with"
        );

        // Incoming melee but the defender is ranged-only: nothing to answer with.
        let mut u = vec![
            unit("A", Side::Foe, Rank::Vanguard, 2, 2, 4, 1, 5),
            unit("D", Side::Party, Rank::Rearguard, 3, 2, 4, 1, 5),
        ];
        u[1].melee = false;
        u[1].ranged = true;
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };
        resolve_react(&mut u, &[contact], &[React::StrikeBack]);
        assert_eq!(u[0].health, 5, "a ranged-only body has no melee counter");
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
    fn catch_spends_tempo_and_records_contact() {
        let mut units = vec![
            unit("A", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 3),
            unit("D", Side::Foe, Rank::Outrider, 1, 3, 2, 1, 3),
        ];
        // A bids 2 cards at F2 vs D's F3 -> lands (4 ≥ 3), spent value 4.
        let contacts = resolve_strike(
            &mut units,
            &[Strike {
                attacker: 0,
                target: 1,
                cards: 2,
            }],
        );
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].bid, 4);
        assert_eq!(units[0].tempo, 1, "spent 2 of 3 tempo");
    }

    #[test]
    fn react_eat_evade_strikeback_are_commit_based() {
        // A (Might 3) strikes D on one edge; D reacts three ways across three runs.
        let base = || {
            vec![
                unit("A", Side::Party, Rank::Vanguard, 3, 2, 4, 1, 3),
                unit("D", Side::Foe, Rank::Outrider, 2, 3, 4, 1, 3),
            ]
        };
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };

        // Eat: D takes 3 (toughness 1 -> 3 cards flip); A untouched.
        let mut u = base();
        resolve_react(&mut u, &[contact], &[React::Eat]);
        assert_eq!(u[1].health, 0);
        assert_eq!(u[0].health, 3);

        // Evade: D flips 2 (2×3=6 > 4) -> misses, no damage, tempo spent.
        let mut u = base();
        resolve_react(&mut u, &[contact], &[React::Evade { cards: 2 }]);
        assert_eq!(u[1].health, 3, "evaded - no damage");
        assert_eq!(u[1].tempo, 2, "spent 2 tempo evading");

        // Strike-back: D takes 3 AND counters for 2; both land even though D is doomed (commit-based).
        let mut u = base();
        let surviving = resolve_react(&mut u, &[contact], &[React::StrikeBack]);
        assert_eq!(u[1].health, 0, "D took the hit");
        assert_eq!(
            u[0].health, 1,
            "A took the 2-Might counter (3 -> 1 card flips: 2/1)"
        );
        assert_eq!(surviving.len(), 1, "still in contact for extra strikes");
    }

    #[test]
    fn evade_breaks_contact_so_no_extra_strikes() {
        let mut u = vec![
            unit("A", Side::Party, Rank::Vanguard, 3, 2, 4, 1, 3),
            unit("D", Side::Foe, Rank::Outrider, 2, 3, 4, 1, 3),
        ];
        let contact = Contact {
            attacker: 0,
            target: 1,
            bid: 4,
        };
        let surviving = resolve_react(&mut u, &[contact], &[React::Evade { cards: 2 }]);
        assert!(surviving.is_empty(), "the evaded edge is gone from phase 3");
    }

    #[test]
    fn extra_strikes_are_one_might_per_card() {
        let mut u = vec![
            unit("A", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 5),
            unit("D", Side::Foe, Rank::Outrider, 1, 3, 2, 1, 5),
        ];
        // A has 3 tempo; flip all 3 -> 3 strikes × Might 2 = 6 damage; toughness 1 -> 5 cards flip (capped).
        resolve_extra(
            &mut u,
            &[ExtraStrike {
                attacker: 0,
                target: 1,
                cards: 3,
            }],
        );
        assert_eq!(u[0].tempo, 0);
        assert_eq!(u[1].health, 0, "6 damage at toughness 1 flips all 5");
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
