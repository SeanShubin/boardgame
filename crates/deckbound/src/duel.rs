//! The Edge duel — one beat at a time.
//!
//! Two sides each pick a [`Stance`] (Marshal / Unleash / Overwhelm / Parry); this
//! resolves a single beat: new Edge for each, any **strike** that landed (typed,
//! so the caller routes it through the [`crate::stats`] pipeline), and whether the
//! duel ends. Pure and deterministic. See `docs/games/deckbound/design/the-duel.md`.

use crate::stats::DamageType;

/// A stance in the duel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    /// Gather — build Edge. Neutral.
    Marshal,
    /// Pour all Edge into a strike. Beats Marshal/Overwhelm; a Parry steals it.
    Unleash,
    /// Drive all Edge through a guard. Beats Parry; whiffs against non-guards.
    Overwhelm,
    /// Read the strike — negate and steal its Edge. Loses to Overwhelm.
    Parry,
}

impl Stance {
    pub fn name(self) -> &'static str {
        match self {
            Stance::Marshal => "Marshal",
            Stance::Unleash => "Unleash",
            Stance::Overwhelm => "Overwhelm",
            Stance::Parry => "Parry",
        }
    }

    pub const ALL: [Stance; 4] = [
        Stance::Marshal,
        Stance::Unleash,
        Stance::Overwhelm,
        Stance::Parry,
    ];
}

/// One side entering a beat: its Edge, its weapon's base power + damage type, its
/// precision, and a name for the narration.
#[derive(Clone, Copy, Debug)]
pub struct Side<'a> {
    pub edge: u32,
    pub power: u32,
    pub dtype: DamageType,
    pub precision: u32,
    pub name: &'a str,
}

/// A strike that landed: the caller routes `raw` of `dtype` (with `precision`)
/// through the target's defense.
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub raw: u32,
    pub dtype: DamageType,
    pub precision: u32,
}

/// The result of one beat.
#[derive(Clone, Debug)]
pub struct Beat {
    pub a_edge: u32,
    pub b_edge: u32,
    /// A strike landing **on A** (from B), if any.
    pub on_a: Option<Strike>,
    /// A strike landing **on B** (from A), if any.
    pub on_b: Option<Strike>,
    /// A strike landed — the duel is over.
    pub ends: bool,
    /// Both chose Marshal (for the stall backstop).
    pub double_marshal: bool,
    pub note: String,
}

fn strike(side: &Side) -> Strike {
    Strike {
        raw: side.power + side.edge,
        dtype: side.dtype,
        precision: side.precision,
    }
}

/// Resolve one beat: side `a` plays `ar`, side `b` plays `br`.
pub fn resolve(a: &Side, ar: Stance, b: &Side, br: Stance) -> Beat {
    use Stance::*;
    let cont = |a_edge, b_edge, note: String| Beat {
        a_edge,
        b_edge,
        on_a: None,
        on_b: None,
        ends: false,
        double_marshal: ar == Marshal && br == Marshal,
        note,
    };
    let end = |on_a, on_b, note: String| Beat {
        a_edge: 0,
        b_edge: 0,
        on_a,
        on_b,
        ends: true,
        double_marshal: false,
        note,
    };
    let a_raw = a.power + a.edge;
    let b_raw = b.power + b.edge;

    match (ar, br) {
        (Marshal, Marshal) => cont(
            a.edge + 1,
            b.edge + 1,
            format!(
                "{} and {} both gather. (Edge {} - {})",
                a.name,
                b.name,
                a.edge + 1,
                b.edge + 1
            ),
        ),
        (Marshal, Unleash) => end(
            Some(strike(b)),
            None,
            format!(
                "{} unleashes and catches {} winding up - {b_raw}!",
                b.name, a.name
            ),
        ),
        (Marshal, Overwhelm) => cont(
            a.edge + 1,
            0,
            format!(
                "{}'s overwhelm finds no guard and whiffs (lost {} Edge); {} gathers.",
                b.name, b.edge, a.name
            ),
        ),
        (Marshal, Parry) => cont(
            a.edge + 1,
            b.edge,
            format!("{} parries at nothing; {} gathers.", b.name, a.name),
        ),
        (Unleash, Marshal) => end(
            None,
            Some(strike(a)),
            format!(
                "{} unleashes and catches {} winding up - {a_raw}!",
                a.name, b.name
            ),
        ),
        (Unleash, Unleash) => end(
            Some(strike(b)),
            Some(strike(a)),
            format!("Both unleash - {} and {} trade blows!", a.name, b.name),
        ),
        (Unleash, Overwhelm) => end(
            None,
            Some(strike(a)),
            format!("{}'s strike beats {}'s overwhelm - {a_raw}!", a.name, b.name),
        ),
        (Unleash, Parry) => cont(
            0,
            b.edge + a.edge.max(1),
            parry_note(b.name, a.name, a.edge),
        ),
        (Overwhelm, Marshal) => cont(
            0,
            b.edge + 1,
            format!(
                "{}'s overwhelm whiffs (lost {} Edge); {} gathers.",
                a.name, a.edge, b.name
            ),
        ),
        (Overwhelm, Unleash) => end(
            Some(strike(b)),
            None,
            format!("{}'s strike beats {}'s overwhelm - {b_raw}!", b.name, a.name),
        ),
        (Overwhelm, Overwhelm) => cont(
            a.edge,
            b.edge,
            format!("{} and {} clinch - nothing lands.", a.name, b.name),
        ),
        (Overwhelm, Parry) => end(
            None,
            Some(strike(a)),
            format!(
                "{}'s overwhelm smashes through {}'s parry - {a_raw}!",
                a.name, b.name
            ),
        ),
        (Parry, Marshal) => cont(
            a.edge,
            b.edge + 1,
            format!("{} parries at nothing; {} gathers.", a.name, b.name),
        ),
        (Parry, Unleash) => cont(
            a.edge + b.edge.max(1),
            0,
            parry_note(a.name, b.name, b.edge),
        ),
        (Parry, Overwhelm) => end(
            Some(strike(b)),
            None,
            format!(
                "{}'s overwhelm smashes through {}'s parry - {b_raw}!",
                b.name, a.name
            ),
        ),
        (Parry, Parry) => cont(
            a.edge,
            b.edge,
            format!("{} and {} both parry - nothing happens.", a.name, b.name),
        ),
    }
}

fn parry_note(parrier: &str, attacker: &str, attacker_edge: u32) -> String {
    if attacker_edge > 0 {
        format!("{parrier} parries {attacker}'s unleash and STEALS {attacker_edge} Edge!")
    } else {
        format!("{parrier} parries {attacker}'s unleash and finds an opening (+1 Edge).")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(edge: u32) -> Side<'static> {
        Side {
            edge,
            power: 1,
            dtype: DamageType::Blunt,
            precision: 0,
            name: "X",
        }
    }

    #[test]
    fn marshal_builds_edge_and_continues() {
        let beat = resolve(&side(2), Stance::Marshal, &side(1), Stance::Marshal);
        assert!(!beat.ends);
        assert_eq!((beat.a_edge, beat.b_edge), (3, 2));
        assert!(beat.double_marshal);
    }

    #[test]
    fn unleash_catches_a_marshaller_and_ends() {
        let beat = resolve(&side(3), Stance::Unleash, &side(0), Stance::Marshal);
        assert!(beat.ends);
        let s = beat.on_b.expect("b struck");
        assert_eq!(s.raw, 4); // power 1 + edge 3
        assert!(beat.on_a.is_none());
    }

    #[test]
    fn parry_steals_an_unleash_and_continues() {
        let beat = resolve(&side(3), Stance::Unleash, &side(1), Stance::Parry);
        assert!(!beat.ends);
        assert_eq!(beat.a_edge, 0);
        assert_eq!(beat.b_edge, 4); // 1 + stolen 3
    }

    #[test]
    fn overwhelm_breaks_a_parry_and_ends() {
        let beat = resolve(&side(2), Stance::Overwhelm, &side(5), Stance::Parry);
        assert!(beat.ends);
        assert_eq!(beat.on_b.unwrap().raw, 3); // power 1 + edge 2
    }

    #[test]
    fn mutual_unleash_strikes_both() {
        let beat = resolve(&side(2), Stance::Unleash, &side(1), Stance::Unleash);
        assert!(beat.ends);
        assert_eq!(beat.on_a.unwrap().raw, 2); // power1 + B edge1
        assert_eq!(beat.on_b.unwrap().raw, 3); // power1 + A edge2
    }
}
