//! §8.1 / §8.2 — the **world map** and the **day clock**.
//!
//! The world is **face-down location cards** in a layout (grid or offset-hex); a character's
//! position is its identity card on a location, and entering a location flips it **face-up** (fog,
//! §8.1). Time advances in **Days**; each character may **move one adjacent space** and attempt one
//! encounter per Day, and the **Day boundary fully resets** combat state (§8.2, handled by the
//! encounter). **Run victory** = clear the **objective** location at its max level; the run is
//! scored in **Days** (§8.2 golf — run defeat is deferred).

use serde::Deserialize;

use crate::currency::Currency;

/// A map coordinate. Rectangles tile as a grid or an offset-hex field (§8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Coord {
    pub col: i32,
    pub row: i32,
}

impl Coord {
    pub fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }
}

/// How the location cards tile (§8.1): a **4-neighbour grid** or a **6-neighbour offset-hex**
/// (odd-r — odd rows shifted right by half a card).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Deserialize)]
pub enum Layout {
    #[default]
    Grid,
    OffsetHex,
}

impl Layout {
    /// The neighbours of `c` (4 on a grid, 6 on offset-hex).
    pub fn neighbours(self, c: Coord) -> Vec<Coord> {
        let (col, row) = (c.col, c.row);
        match self {
            Layout::Grid => vec![
                Coord::new(col - 1, row),
                Coord::new(col + 1, row),
                Coord::new(col, row - 1),
                Coord::new(col, row + 1),
            ],
            Layout::OffsetHex => {
                // odd-r: the up/down diagonals shift by row parity.
                let (a, b) = if row.rem_euclid(2) == 1 {
                    (0, 1)
                } else {
                    (-1, 0)
                };
                vec![
                    Coord::new(col - 1, row),
                    Coord::new(col + 1, row),
                    Coord::new(col + a, row - 1),
                    Coord::new(col + b, row - 1),
                    Coord::new(col + a, row + 1),
                    Coord::new(col + b, row + 1),
                ]
            }
        }
    }

    pub fn adjacent(self, from: Coord, to: Coord) -> bool {
        self.neighbours(from).contains(&to)
    }
}

/// A location (§8.1): face-down until entered. It mints one **currency** type (§8.3 → its threat
/// deck, §8.4) and is clearable up to `max_level`.
#[derive(Clone, Debug, Deserialize)]
pub struct Location {
    pub name: String,
    pub coord: Coord,
    pub currency: Currency,
    pub max_level: u32,
}

/// The run state (§8.2): the world clock, character positions, fog, and per-location clear markers.
#[derive(Clone, Debug)]
pub struct Run {
    pub day: u32,
    pub layout: Layout,
    pub locations: Vec<Location>,
    /// Index of the objective — clearing it at max level wins the run (§8.2 provisional).
    pub objective: usize,
    /// Each character's location index (its identity card's position).
    pub positions: Vec<usize>,
    /// Per-location: flipped face-up yet?
    pub revealed: Vec<bool>,
    /// Per-location **clear marker** — the high-water mark (0 = uncleared, §8.2/§8.3).
    pub cleared: Vec<u32>,
}

impl Run {
    /// Start a run: the party begins co-located at `start` (which is revealed), the rest face-down.
    pub fn new(
        layout: Layout,
        locations: Vec<Location>,
        objective: usize,
        start: usize,
        party: usize,
    ) -> Self {
        let n = locations.len();
        let mut revealed = vec![false; n];
        revealed[start] = true;
        Run {
            day: 0,
            layout,
            locations,
            objective,
            positions: vec![start; party],
            revealed,
            cleared: vec![0; n],
        }
    }

    /// Can `character` move to location `to` this Day? (one adjacent space, §8.1).
    pub fn can_move(&self, character: usize, to: usize) -> bool {
        let from = self.positions[character];
        from != to
            && self
                .layout
                .adjacent(self.locations[from].coord, self.locations[to].coord)
    }

    /// Move a character one adjacent space and flip the destination face-up (§8.1). Returns false
    /// if the move is illegal (not adjacent).
    pub fn move_to(&mut self, character: usize, to: usize) -> bool {
        if !self.can_move(character, to) {
            return false;
        }
        self.positions[character] = to;
        self.revealed[to] = true;
        true
    }

    /// Record a clear at `level` (clamped to the location's max); the marker only advances (§8.2).
    pub fn record_clear(&mut self, loc: usize, level: u32) {
        let cap = self.locations[loc].max_level;
        self.cleared[loc] = self.cleared[loc].max(level.min(cap));
    }

    /// Won when the objective is cleared at its max level (§8.2 provisional run victory).
    pub fn is_won(&self) -> bool {
        self.cleared[self.objective] >= self.locations[self.objective].max_level
    }

    /// End-of-day: advance the calendar. (Full combat reset is handled by the encounter, §8.2.)
    pub fn end_day(&mut self) {
        self.day += 1;
    }

    /// The role-track rewards a party has unlocked so far (§8.3): a location of track `Y` cleared to
    /// level `N` unlocks `(Y, 1..=N)`. Generic (Gold) locations mint no reward track. The build is a
    /// pure function of the clear markers — no currency, no path-dependent budget (§0.1).
    pub fn unlocked(&self) -> Vec<(Currency, u32)> {
        let mut out = Vec::new();
        for (loc, &lvl) in self.locations.iter().zip(&self.cleared) {
            if loc.currency == Currency::Gold {
                continue; // generic locations are not a reward track (§8.5)
            }
            for level in 1..=lvl {
                out.push((loc.currency, level));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(name: &str, col: i32, row: i32, currency: Currency, max_level: u32) -> Location {
        Location {
            name: name.into(),
            coord: Coord::new(col, row),
            currency,
            max_level,
        }
    }

    #[test]
    fn grid_has_four_neighbours_hex_has_six() {
        let c = Coord::new(2, 2);
        assert_eq!(Layout::Grid.neighbours(c).len(), 4);
        assert_eq!(Layout::OffsetHex.neighbours(c).len(), 6);
        assert!(Layout::Grid.adjacent(c, Coord::new(2, 3)));
        assert!(!Layout::Grid.adjacent(c, Coord::new(3, 3))); // diagonal not adjacent on a grid
        assert!(Layout::OffsetHex.adjacent(c, Coord::new(2, 3))); // even-row hex down-left/right
    }

    #[test]
    fn movement_respects_adjacency_and_reveals() {
        // A: (0,0) start; B: (1,0) adjacent; C: (3,0) far.
        let locs = vec![
            loc("A", 0, 0, Currency::Gold, 1),
            loc("B", 1, 0, Currency::Iron, 5),
            loc("C", 3, 0, Currency::Salt, 5),
        ];
        let mut run = Run::new(Layout::Grid, locs, 2, 0, 1);
        assert!(run.revealed[0] && !run.revealed[1]); // fog: only the start is up
        assert!(!run.move_to(0, 2)); // C is not adjacent
        assert!(run.move_to(0, 1)); // B is adjacent
        assert!(run.revealed[1]); // entering flips it face-up
        assert_eq!(run.positions[0], 1);
    }

    #[test]
    fn clear_marker_advances_and_caps_and_wins() {
        let locs = vec![
            loc("A", 0, 0, Currency::Gold, 1),
            loc("Final", 1, 0, Currency::Iron, 5),
        ];
        let mut run = Run::new(Layout::Grid, locs, 1, 0, 1);
        run.record_clear(1, 3);
        assert_eq!(run.cleared[1], 3);
        run.record_clear(1, 2); // never regresses
        assert_eq!(run.cleared[1], 3);
        assert!(!run.is_won());
        run.record_clear(1, 9); // clamps to max_level 5
        assert_eq!(run.cleared[1], 5);
        assert!(run.is_won());
        // Unlocked rewards: the Iron Final cleared to 5 ⇒ (Iron, 1..=5); the generic start mints none.
        assert_eq!(
            run.unlocked(),
            vec![
                (Currency::Iron, 1),
                (Currency::Iron, 2),
                (Currency::Iron, 3),
                (Currency::Iron, 4),
                (Currency::Iron, 5),
            ]
        );
    }
}
