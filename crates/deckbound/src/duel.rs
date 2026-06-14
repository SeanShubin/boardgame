//! The Edge duel — one beat at a time.
//!
//! Two sides each pick a stance (Marshal / Unleash / Overwhelm / Parry); this
//! resolves a single beat: new Edge for each, damage dealt to each, and whether
//! a strike landed (which ends the duel). See
//! `docs/games/deckbound/design/the-duel.md`. Pure and deterministic.

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

    /// All four, in display order.
    pub const ALL: [Stance; 4] =
        [Stance::Marshal, Stance::Unleash, Stance::Overwhelm, Stance::Parry];
}

/// One side entering a beat: its current Edge, its base (0-Edge) strike, and a
/// name for the narration.
#[derive(Clone, Copy, Debug)]
pub struct Side<'a> {
    pub edge: u32,
    pub base: u32,
    pub name: &'a str,
}

/// The result of one beat.
#[derive(Clone, Debug)]
pub struct Beat {
    /// Side A's Edge after the beat (ignored if `ends`).
    pub a_edge: u32,
    /// Side B's Edge after the beat (ignored if `ends`).
    pub b_edge: u32,
    /// Damage dealt **to** A this beat.
    pub a_dmg: u32,
    /// Damage dealt **to** B this beat.
    pub b_dmg: u32,
    /// A strike landed — the duel is over.
    pub ends: bool,
    /// Whether both sides chose Marshal (for the stall backstop).
    pub double_marshal: bool,
    /// Human-readable narration of what happened.
    pub note: String,
}

/// Resolve one beat: side `a` plays `a_stance`, side `b` plays `b_stance`.
pub fn resolve(a: &Side, a_stance: Stance, b: &Side, b_stance: Stance) -> Beat {
    use Stance::*;
    let cont = |a_edge, b_edge, note: String| Beat {
        a_edge,
        b_edge,
        a_dmg: 0,
        b_dmg: 0,
        ends: false,
        double_marshal: a_stance == Marshal && b_stance == Marshal,
        note,
    };
    let end = |a_dmg, b_dmg, note: String| Beat {
        a_edge: 0,
        b_edge: 0,
        a_dmg,
        b_dmg,
        ends: true,
        double_marshal: false,
        note,
    };
    // A's strike hits B for this; B's strike hits A for this.
    let a_hit = a.base + a.edge;
    let b_hit = b.base + b.edge;

    match (a_stance, b_stance) {
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
            b_hit,
            0,
            format!(
                "{} unleashes and catches {} winding up - {b_hit} damage!",
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
            0,
            a_hit,
            format!(
                "{} unleashes and catches {} winding up - {a_hit} damage!",
                a.name, b.name
            ),
        ),
        (Unleash, Unleash) => end(
            b_hit,
            a_hit,
            format!(
                "Both unleash - {} takes {b_hit}, {} takes {a_hit}!",
                a.name, b.name
            ),
        ),
        (Unleash, Overwhelm) => end(
            0,
            a_hit,
            format!(
                "{}'s strike beats {}'s overwhelm - {a_hit} damage!",
                a.name, b.name
            ),
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
            b_hit,
            0,
            format!(
                "{}'s strike beats {}'s overwhelm - {b_hit} damage!",
                b.name, a.name
            ),
        ),
        (Overwhelm, Overwhelm) => cont(
            a.edge,
            b.edge,
            format!("{} and {} clinch - nothing lands.", a.name, b.name),
        ),
        (Overwhelm, Parry) => end(
            0,
            a_hit,
            format!(
                "{}'s overwhelm smashes through {}'s parry - {a_hit} damage!",
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
            b_hit,
            0,
            format!(
                "{}'s overwhelm smashes through {}'s parry - {b_hit} damage!",
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

/// Narration for a successful parry. It takes the attacker's whole bank, or — if
/// there was nothing to take — earns an opening worth one Edge.
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
            base: 1,
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
        // A unleashes (edge 3, base 1 => 4), B marshalling -> B struck for 4.
        let beat = resolve(&side(3), Stance::Unleash, &side(0), Stance::Marshal);
        assert!(beat.ends);
        assert_eq!(beat.b_dmg, 4);
        assert_eq!(beat.a_dmg, 0);
    }

    #[test]
    fn parry_steals_an_unleash_and_continues() {
        // B parries A's unleash (edge 3) -> B steals 3, A -> 0, no strike.
        let beat = resolve(&side(3), Stance::Unleash, &side(1), Stance::Parry);
        assert!(!beat.ends);
        assert_eq!(beat.a_edge, 0);
        assert_eq!(beat.b_edge, 4); // 1 + stolen 3
    }

    #[test]
    fn parrying_an_empty_unleash_earns_one_edge() {
        // B parries A's 0-Edge poke -> nothing to steal, but B gains 1 (an opening).
        let beat = resolve(&side(0), Stance::Unleash, &side(0), Stance::Parry);
        assert!(!beat.ends);
        assert_eq!(beat.b_edge, 1);
        assert_eq!(beat.a_edge, 0);
    }

    #[test]
    fn overwhelm_breaks_a_parry_and_ends() {
        // A overwhelms (edge 2, base 1 => 3) into B's parry -> B struck for 3.
        let beat = resolve(&side(2), Stance::Overwhelm, &side(5), Stance::Parry);
        assert!(beat.ends);
        assert_eq!(beat.b_dmg, 3);
    }

    #[test]
    fn overwhelm_whiffs_against_a_marshaller() {
        let beat = resolve(&side(4), Stance::Overwhelm, &side(0), Stance::Marshal);
        assert!(!beat.ends);
        assert_eq!(beat.a_edge, 0); // lost its Edge
        assert_eq!(beat.b_edge, 1); // marshaller gathered
    }

    #[test]
    fn mutual_unleash_ends_with_both_struck() {
        let beat = resolve(&side(2), Stance::Unleash, &side(1), Stance::Unleash);
        assert!(beat.ends);
        assert_eq!(beat.a_dmg, 2); // base1 + B edge1
        assert_eq!(beat.b_dmg, 3); // base1 + A edge2
    }
}
