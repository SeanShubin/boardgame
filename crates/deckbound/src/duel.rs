//! The Clash — the card-based tactical core (supersedes the stance/Edge duel).
//!
//! Each beat both fighters pick one [`Move`]; [`resolve`] settles a single beat: any hit
//! that lands (typed, routed through [`crate::stats`]), how each side's **Charges** change
//! (the durable ×2 escalation that replaced Edge), and a note. Pure and deterministic.
//!
//! It is built to guarantee three invariants under last-word reads (see
//! `docs/games/deckbound/spec/README.md`, §1 The Clash):
//! 1. **Avoid** — a complete, standing defense (Parry answers Strike, Evade answers Throw).
//! 2. **Land** — a complete offense (an attack lands on every move).
//! 3. **Not both, free** — landing on a committed Strike means trading a hit.

use crate::stats::DamageType;

/// A move in the Clash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// Offense. Beaten by Parry; clips Throw; trades with Strike.
    Strike,
    /// Offense. Beaten by Evade and by Strike; beats Parry.
    Throw,
    /// Defense. Negates a Strike (and flips that attacker's Charges down).
    Parry,
    /// Defense. Negates a Throw (and flips that attacker's Charges down).
    Evade,
    /// Setup. Place one active Charge (durable ×2 to your attacks). Exposed.
    Charge,
    /// Setup. Flip your face-down Charges back up. Exposed.
    Recover,
}

impl Move {
    pub fn name(self) -> &'static str {
        match self {
            Move::Strike => "Strike",
            Move::Throw => "Throw",
            Move::Parry => "Parry",
            Move::Evade => "Evade",
            Move::Charge => "Charge",
            Move::Recover => "Recover",
        }
    }

    /// The two standing defenses — always available (this is what makes "avoid" hold for
    /// the whole duel; defense never depletes).
    pub const DEFENSES: [Move; 2] = [Move::Parry, Move::Evade];
    /// The two standing attacks — always available ("land" on demand).
    pub const ATTACKS: [Move; 2] = [Move::Strike, Move::Throw];

    pub fn is_attack(self) -> bool {
        matches!(self, Move::Strike | Move::Throw)
    }
}

/// One side entering a beat: its base attack power + type/precision, its Charge state
/// (`up` active, `down` flipped, `max` capacity), and a name for narration.
#[derive(Clone, Copy, Debug)]
pub struct Side<'a> {
    pub power: u32,
    pub dtype: DamageType,
    pub precision: u32,
    pub up: u32,
    pub down: u32,
    pub max: u32,
    pub name: &'a str,
}

/// A hit that landed: the caller routes `raw` of `dtype` (with `precision`) through the
/// target's defense.
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub raw: u32,
    pub dtype: DamageType,
    pub precision: u32,
}

/// A side's Charge state after a beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Charges {
    pub up: u32,
    pub down: u32,
}

/// The result of one beat of the Clash.
#[derive(Clone, Debug)]
pub struct Clash {
    /// A hit landing **on A** (from B), if any.
    pub on_a: Option<Strike>,
    /// A hit landing **on B** (from A), if any.
    pub on_b: Option<Strike>,
    pub a: Charges,
    pub b: Charges,
    /// At least one hit landed this beat (for the stall backstop).
    pub connected: bool,
    pub note: String,
}

/// Damage of an attack: `power × 2^up` (each active Charge doubles), saturating.
fn damage(power: u32, up: u32) -> u32 {
    let mult = 1u32.checked_shl(up.min(16)).unwrap_or(u32::MAX);
    power.saturating_mul(mult)
}

/// Does `atk` (an attack) connect through `def`? Strike connects unless Parried; Throw
/// connects unless Evaded or out-struck (Strike clips Throw).
fn connects(atk: Move, def: Move) -> bool {
    match atk {
        Move::Strike => def != Move::Parry,
        Move::Throw => def != Move::Evade && def != Move::Strike,
        _ => false,
    }
}

/// Did `their` move successfully *defend* my attack (which flips my Charges face-down)?
/// Only a matching defense counts — being out-struck (Strike vs Throw) does not.
fn defended(my_atk: Move, their: Move) -> bool {
    (my_atk == Move::Strike && their == Move::Parry)
        || (my_atk == Move::Throw && their == Move::Evade)
}

/// Resolve one beat: side `a` plays `am`, side `b` plays `bm`.
pub fn resolve(a: &Side, am: Move, b: &Side, bm: Move) -> Clash {
    let (mut a_up, mut a_down) = (a.up, a.down);
    let (mut b_up, mut b_down) = (b.up, b.down);

    // Hits land off the *current* (pre-flip) charges.
    let on_b = connects(am, bm).then(|| Strike {
        raw: damage(a.power, a_up),
        dtype: a.dtype,
        precision: a.precision,
    });
    let on_a = connects(bm, am).then(|| Strike {
        raw: damage(b.power, b_up),
        dtype: b.dtype,
        precision: b.precision,
    });

    // A defended attack loses its active Charges to face-down (the comeback).
    if defended(am, bm) {
        a_down += a_up;
        a_up = 0;
    }
    if defended(bm, am) {
        b_down += b_up;
        b_up = 0;
    }

    // Setups resolve only if not interrupted by a connecting attack this beat.
    if on_a.is_none() {
        match am {
            Move::Charge if a_up + a_down < a.max => a_up += 1,
            Move::Recover => {
                a_up += a_down;
                a_down = 0;
            }
            _ => {}
        }
    }
    if on_b.is_none() {
        match bm {
            Move::Charge if b_up + b_down < b.max => b_up += 1,
            Move::Recover => {
                b_up += b_down;
                b_down = 0;
            }
            _ => {}
        }
    }

    let note = clash_note(a.name, am, b.name, bm, &on_a, &on_b);
    Clash {
        on_a,
        on_b,
        a: Charges {
            up: a_up,
            down: a_down,
        },
        b: Charges {
            up: b_up,
            down: b_down,
        },
        connected: on_a.is_some() || on_b.is_some(),
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

    fn side(power: u32, up: u32, down: u32, max: u32) -> Side<'static> {
        Side {
            power,
            dtype: DamageType::Blunt,
            precision: 0,
            up,
            down,
            max,
            name: "X",
        }
    }

    /// Invariant 1 — **avoid**: every attack has a defense that negates it (complete
    /// defense). With Parry/Evade always available (standing, enforced by the engine),
    /// this gives "never hit, whole duel" under last-word reads.
    #[test]
    fn defense_answers_every_attack() {
        let s = side(5, 0, 0, 3);
        // I see your attack and pick the matching defense → I take nothing.
        assert!(resolve(&s, Move::Strike, &s, Move::Parry).on_a.is_none());
        assert!(resolve(&s, Move::Throw, &s, Move::Evade).on_a.is_none());
    }

    /// Invariant 2 — **land**: for every move the opponent can make, some attack lands.
    #[test]
    fn offense_answers_every_move() {
        let me = side(5, 0, 0, 3);
        let you = side(5, 0, 0, 3);
        let lands = |am: Move, bm: Move| resolve(&me, am, &you, bm).on_b.is_some();
        assert!(lands(Move::Throw, Move::Parry), "Throw beats Parry");
        assert!(lands(Move::Strike, Move::Evade), "Strike beats Evade");
        assert!(
            lands(Move::Strike, Move::Strike),
            "Strike trades into Strike"
        );
        assert!(lands(Move::Strike, Move::Throw), "Strike clips Throw");
        assert!(lands(Move::Strike, Move::Charge), "Strike hits a charger");
        assert!(
            lands(Move::Strike, Move::Recover),
            "Strike hits a recoverer"
        );
    }

    /// Invariant 3 — **not both, free**: against a committed Strike, the *only* landing
    /// answer is Strike, and Strike-vs-Strike trades — so you cannot land without being hit.
    #[test]
    fn landing_on_a_strike_requires_a_trade() {
        let me = side(5, 0, 0, 3);
        let you = side(5, 0, 0, 3);
        // The only A move that lands on a B Strike:
        for am in [
            Move::Throw,
            Move::Parry,
            Move::Evade,
            Move::Charge,
            Move::Recover,
        ] {
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
    fn charges_double_damage_and_persist() {
        let a = side(3, 2, 0, 3); // 2 active charges → ×4
        let b = side(3, 0, 0, 3);
        let r = resolve(&a, Move::Strike, &b, Move::Evade); // Strike beats Evade
        assert_eq!(r.on_b.unwrap().raw, 12); // 3 × 2^2
        assert_eq!(r.a, Charges { up: 2, down: 0 }, "charges persist (durable)");
    }

    #[test]
    fn a_defended_strike_flips_the_charges_down() {
        let a = side(3, 2, 0, 3);
        let b = side(3, 0, 0, 3);
        let r = resolve(&a, Move::Strike, &b, Move::Parry);
        assert!(r.on_b.is_none(), "parried — no damage");
        assert_eq!(
            r.a,
            Charges { up: 0, down: 2 },
            "the parry flipped both charges down"
        );
    }

    #[test]
    fn charge_builds_and_recover_restores() {
        let a = side(3, 0, 0, 2);
        let b = side(3, 0, 0, 2);
        // Charge unopposed → +1 active.
        let r = resolve(&a, Move::Charge, &b, Move::Parry);
        assert_eq!(r.a, Charges { up: 1, down: 0 });
        // Recover flips down → up.
        let a2 = side(3, 0, 2, 2);
        let r2 = resolve(&a2, Move::Recover, &b, Move::Parry);
        assert_eq!(r2.a, Charges { up: 2, down: 0 });
    }

    #[test]
    fn an_attack_interrupts_a_charge() {
        let a = side(3, 0, 0, 3); // charging
        let b = side(3, 0, 0, 3);
        let r = resolve(&a, Move::Charge, &b, Move::Strike); // B strikes the charger
        assert!(r.on_a.is_some(), "the charger is hit");
        assert_eq!(
            r.a,
            Charges { up: 0, down: 0 },
            "the charge was interrupted"
        );
    }
}
