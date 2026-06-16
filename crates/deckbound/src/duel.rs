//! The Clash — the four-card tactical core.
//!
//! Each beat both fighters pick one [`Move`]; [`resolve`] settles a single beat: any hit
//! that lands (typed, routed through [`crate::stats`]), how each side's **Force** changes
//! (the single per-side escalation count), whether the duel **ends** (ends-on-strike), and
//! a note. Pure and deterministic.
//!
//! Three invariants under perfect guessing (see `docs/games/deckbound/spec/README.md` §1.0):
//! 1. **Avoid** — every attack has a defense that negates it (Strike↦Evade, Anticipate↦Gather).
//! 2. **Land** — every move has an answering attack.
//! 3. **Not both, free** — landing on a committed Strike means trading a hit.

use crate::stats::DamageType;

/// A move in the Clash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// Attack: hit *where they are now*. Beats Gather; stopped by Evade; trades with Strike;
    /// beats Anticipate.
    Strike,
    /// Attack: hit *where they'll be* (lead). Beats Evade; stopped by Gather; loses to Strike.
    Anticipate,
    /// Hold your ground (a defense) and build Force (+1). Stops Anticipate; beaten by Strike.
    Gather,
    /// Move. Stops Strike (and steals the striker's Force); beaten by Anticipate.
    Evade,
}

impl Move {
    pub fn name(self) -> &'static str {
        match self {
            Move::Strike => "Strike",
            Move::Anticipate => "Anticipate",
            Move::Gather => "Gather",
            Move::Evade => "Evade",
        }
    }

    /// The four moves, always available (the kit is complete).
    pub const ALL: [Move; 4] = [Move::Strike, Move::Anticipate, Move::Gather, Move::Evade];

    pub fn is_attack(self) -> bool {
        matches!(self, Move::Strike | Move::Anticipate)
    }
}

/// One side entering a beat: base attack power + type/precision, its Force count, and a name.
#[derive(Clone, Copy, Debug)]
pub struct Side<'a> {
    pub power: u32,
    pub dtype: DamageType,
    pub precision: u32,
    pub force: u32,
    pub name: &'a str,
}

/// A hit that landed: the caller routes `raw` of `dtype` (with `precision`) through defense.
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub raw: u32,
    pub dtype: DamageType,
    pub precision: u32,
}

/// The result of one beat of the Clash.
#[derive(Clone, Debug)]
pub struct Clash {
    /// A hit landing **on A** (from B), if any.
    pub on_a: Option<Strike>,
    /// A hit landing **on B** (from A), if any.
    pub on_b: Option<Strike>,
    /// A's Force after the beat.
    pub a_force: u32,
    /// B's Force after the beat.
    pub b_force: u32,
    /// A strike connected — the duel **ends** (ends-on-strike).
    pub ends: bool,
    pub note: String,
}

/// Damage of an attack: `power × 2^force`, saturating.
fn damage(power: u32, force: u32) -> u32 {
    let mult = if force >= 31 { u32::MAX } else { 1u32 << force };
    power.saturating_mul(mult)
}

/// Does `atk` connect through `def`? Strike connects unless Evaded; Anticipate connects only
/// against an Evade (it leads the move). Non-attacks never connect.
fn connects(atk: Move, def: Move) -> bool {
    match atk {
        Move::Strike => def != Move::Evade,
        Move::Anticipate => def == Move::Evade,
        _ => false,
    }
}

/// Resolve one beat: side `a` plays `am`, side `b` plays `bm`.
pub fn resolve(a: &Side, am: Move, b: &Side, bm: Move) -> Clash {
    // Hits land off the current Force.
    let on_b = connects(am, bm).then(|| Strike {
        raw: damage(a.power, a.force),
        dtype: a.dtype,
        precision: a.precision,
    });
    let on_a = connects(bm, am).then(|| Strike {
        raw: damage(b.power, b.force),
        dtype: b.dtype,
        precision: b.precision,
    });

    let mut a_force = a.force;
    let mut b_force = b.force;

    // The only transfer: a Strike slipped by an Evade hands the striker's Force to the evader.
    if am == Move::Strike && bm == Move::Evade {
        b_force = b_force.saturating_add(a_force);
        a_force = 0;
    }
    if bm == Move::Strike && am == Move::Evade {
        a_force = a_force.saturating_add(b_force);
        b_force = 0;
    }

    // Gather builds, unless its player was hit this beat (interrupted).
    if am == Move::Gather && on_a.is_none() {
        a_force = a_force.saturating_add(1);
    }
    if bm == Move::Gather && on_b.is_none() {
        b_force = b_force.saturating_add(1);
    }

    let ends = on_a.is_some() || on_b.is_some();
    let note = clash_note(a.name, am, b.name, bm, &on_a, &on_b);
    Clash {
        on_a,
        on_b,
        a_force,
        b_force,
        ends,
        note,
    }
}

fn clash_note(
    an: &str,
    am: Move,
    bn: &str,
    bm: Move,
    on_a: &Option<Strike>,
    on_b: &Option<Strike>,
) -> String {
    match (on_a.is_some(), on_b.is_some()) {
        (true, true) => format!("{an} and {bn} trade blows!"),
        (false, true) => format!("{an}'s {} lands on {bn}.", am.name()),
        (true, false) => format!("{bn}'s {} lands on {an}.", bm.name()),
        (false, false) => format!("{an} {} / {bn} {} — nothing lands.", am.name(), bm.name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(power: u32, force: u32) -> Side<'static> {
        Side {
            power,
            dtype: DamageType::Blunt,
            precision: 0,
            force,
            name: "X",
        }
    }

    /// Invariant 1 — **avoid**: every attack has a defense that negates it.
    #[test]
    fn defense_answers_every_attack() {
        let me = side(5, 0);
        let you = side(5, 0);
        // I pick the matching defense → I take nothing (on_a = a hit landing on me).
        assert!(resolve(&me, Move::Evade, &you, Move::Strike).on_a.is_none());
        assert!(
            resolve(&me, Move::Gather, &you, Move::Anticipate)
                .on_a
                .is_none()
        );
    }

    /// Invariant 2 — **land**: for every move the opponent makes, some attack of mine lands.
    #[test]
    fn offense_answers_every_move() {
        let me = side(5, 0);
        let you = side(5, 0);
        let lands = |am: Move, bm: Move| resolve(&me, am, &you, bm).on_b.is_some();
        assert!(lands(Move::Strike, Move::Gather), "Strike hits a holder");
        assert!(
            lands(Move::Anticipate, Move::Evade),
            "Anticipate leads a mover"
        );
        assert!(
            lands(Move::Strike, Move::Strike),
            "Strike trades into Strike"
        );
        assert!(
            lands(Move::Strike, Move::Anticipate),
            "Strike beats Anticipate"
        );
    }

    /// Invariant 3 — **not both, free**: vs a committed Strike, the only landing answer is
    /// Strike, and Strike-vs-Strike trades.
    #[test]
    fn landing_on_a_strike_requires_a_trade() {
        let me = side(5, 0);
        let you = side(5, 0);
        for am in [Move::Anticipate, Move::Gather, Move::Evade] {
            assert!(
                resolve(&me, am, &you, Move::Strike).on_b.is_none(),
                "{am:?} shouldn't land vs Strike"
            );
        }
        let trade = resolve(&me, Move::Strike, &you, Move::Strike);
        assert!(
            trade.on_b.is_some() && trade.on_a.is_some(),
            "Strike lands but you're also hit"
        );
    }

    #[test]
    fn force_doubles_and_builds() {
        let a = side(3, 2); // ×4
        let b = side(3, 0);
        let r = resolve(&a, Move::Strike, &b, Move::Gather); // Strike hits the holder
        assert_eq!(r.on_b.unwrap().raw, 12); // 3 × 2^2
        // Gather builds when uninterrupted.
        let r2 = resolve(&a, Move::Gather, &b, Move::Gather);
        assert_eq!(r2.a_force, 3, "Gather adds one");
        assert_eq!(r2.b_force, 1);
    }

    /// The only Force transfer: a Strike slipped by an Evade is stolen.
    #[test]
    fn evading_a_strike_steals_its_force() {
        let striker = side(3, 3);
        let dodger = side(3, 0);
        let r = resolve(&striker, Move::Strike, &dodger, Move::Evade);
        assert!(
            r.on_a.is_none() && r.on_b.is_none(),
            "the strike is dodged, nothing lands"
        );
        assert_eq!(r.a_force, 0, "the striker loses its Force");
        assert_eq!(r.b_force, 3, "the dodger steals it");
        assert!(!r.ends, "a dodged strike is a dance beat, not an ender");
    }

    /// Ends-on-strike: a connecting move ends the duel; a non-connecting one continues it.
    #[test]
    fn a_connecting_strike_ends_the_duel() {
        let a = side(3, 0);
        let b = side(3, 0);
        assert!(
            resolve(&a, Move::Strike, &b, Move::Gather).ends,
            "a landed strike ends it"
        );
        assert!(
            !resolve(&a, Move::Gather, &b, Move::Gather).ends,
            "two gathers continue"
        );
        assert!(
            !resolve(&a, Move::Anticipate, &b, Move::Gather).ends,
            "a whiffed lead continues"
        );
    }
}
