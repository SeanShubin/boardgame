//! **v2 combat mechanics — the headless brain** (design brief §"Arena v2", locked 2026-07-09; memory
//! `manual-combat-design`). The plan-then-commit economy, implemented *fresh* per build-decision (B) — the
//! game layer owns combat, this module is pure logic over [`Combatant`]s (no board, no renderer). Stage 2
//! reads the `Tableau` into these and writes the resolved state back; the unit tests here are the safety net
//! (solver/balance re-validation deferred until the feel is tested).
//!
//! Each combat sub-phase is **three one-way mini-phases** (strict pipeline, no ping-pong):
//! 1. **Catch** — an attacker bids tempo to reach a target; the catch lands when `cards × F_att ≥ F_target`
//!    (may over-flip to raise the bar an evade must clear). Which ranks may catch which is the [`SCHEDULE`]
//!    gate. A landed catch is a [`Contact`] edge.
//! 2. **React** — per incoming catch the defender **eats** (free, default), **evades** (`cards × F_def >` the
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

use deckbound::actor::Intention as Rank;
use deckbound::combat::SCHEDULE;

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
    /// Tempo cards left this round.
    pub tempo: u32,
    /// Face-up health cards remaining (Vitality at full); 0 ⇒ fallen at the next boundary.
    pub health: u32,
    /// Damage accumulated this sub-phase; wiped at the sub-phase boundary (sub-threshold damage never carries).
    pending: u32,
    pub fallen: bool,
}

impl Combatant {
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

/// A catch declaration: `attacker` bids `cards` tempo to reach `target` (indices into the combatant slice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Catch {
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

/// Whether `atk` may catch `tgt` in sub-phase `sub` — the rank×phase [`SCHEDULE`] gate.
pub fn legal_catch(sub: usize, atk: Rank, tgt: Rank) -> bool {
    SCHEDULE
        .get(sub)
        .is_some_and(|pairs| pairs.contains(&(atk, tgt)))
}

/// A catch lands when the attacker's bid value reaches the target's finesse: `cards × F_att ≥ F_target`.
pub fn catch_lands(cards: u32, f_att: u32, f_target: u32) -> bool {
    cards * f_att >= f_target
}

/// An evade succeeds when the defender's bid **strictly exceeds** the attacker's spent value.
pub fn evade_succeeds(cards: u32, f_def: u32, atk_spent: u32) -> bool {
    cards * f_def > atk_spent
}

// ---- the three mini-phases ------------------------------------------------------------------------

/// **Catch.** Spend each attacker's bid (capped at its remaining tempo) and keep the catches that land as
/// [`Contact`] edges (`bid = cards × F_att`, the value an evade must beat). The caller (UI) has already
/// gated legality via [`legal_catch`].
pub fn resolve_catch(units: &mut [Combatant], catches: &[Catch]) -> Vec<Contact> {
    let mut contacts = Vec::new();
    for c in catches {
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
                damage[tgt] += units[atk].might;
                surviving.push(*contact);
            }
            React::Evade { cards } => {
                let cards = cards.min(units[tgt].tempo);
                units[tgt].tempo -= cards;
                if evade_succeeds(cards, units[tgt].finesse, contact.bid) {
                    // miss + break contact: no damage, dropped from the surviving edges.
                } else {
                    damage[tgt] += units[atk].might; // the evade failed — the hit still lands
                    surviving.push(*contact);
                }
            }
            React::StrikeBack => {
                let spent = 1.min(units[tgt].tempo);
                units[tgt].tempo -= spent;
                damage[tgt] += units[atk].might; // take the hit...
                damage[atk] += units[tgt].might; // ...and counter (commit-based, even if the soaker dies)
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
        damage[e.target] += units[e.attacker].might * cards; // 1 card = 1 strike of Might
    }
    apply(units, &damage);
}

/// Apply an order-free damage vector (accumulated Might per unit).
fn apply(units: &mut [Combatant], damage: &[u32]) {
    for (unit, &dmg) in units.iter_mut().zip(damage) {
        if dmg > 0 {
            unit.take(dmg);
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

/// Start a round: refresh every unit's tempo to its Cadence (leftover tempo does not carry across rounds).
pub fn refresh_round(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        u.tempo = u.cadence;
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
            tempo: cadence,
            health,
            pending: 0,
            fallen: false,
        }
    }

    #[test]
    fn bid_math() {
        // Catch: F2 catching F3 needs 2 cards (2×2=4 ≥ 3); 1 card (1×2=2) falls short.
        assert!(!catch_lands(1, 2, 3));
        assert!(catch_lands(2, 2, 3));
        // Evade must STRICTLY exceed the attacker's spent value.
        assert!(!evade_succeeds(2, 2, 4)); // 4 is not > 4
        assert!(evade_succeeds(3, 2, 4)); // 6 > 4
    }

    #[test]
    fn schedule_gates_catches() {
        // Intercept (sub-phase 0) is Vanguard -> Outrider only.
        assert!(legal_catch(0, Rank::Vanguard, Rank::Outrider));
        assert!(!legal_catch(0, Rank::Vanguard, Rank::Vanguard));
        assert!(!legal_catch(0, Rank::Rearguard, Rank::Outrider));
        // Clash (sub-phase 3) has (Rearguard,Vanguard) and (Vanguard,Vanguard).
        assert!(legal_catch(3, Rank::Vanguard, Rank::Vanguard));
    }

    #[test]
    fn catch_spends_tempo_and_records_contact() {
        let mut units = vec![
            unit("A", Side::Party, Rank::Vanguard, 2, 2, 3, 1, 3),
            unit("D", Side::Foe, Rank::Outrider, 1, 3, 2, 1, 3),
        ];
        // A bids 2 cards at F2 vs D's F3 -> lands (4 ≥ 3), spent value 4.
        let contacts = resolve_catch(
            &mut units,
            &[Catch {
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
        // A (Might 3) catches D on one edge; D reacts three ways across three runs.
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
        assert_eq!(u[1].health, 3, "evaded — no damage");
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
