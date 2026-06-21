//! §8.1 / §8.2 — the **world map** and the **day clock**.
//!
//! The world is **face-down location cards** in a layout (grid or offset-hex); a character's
//! position is its identity card on a location, and entering a location flips it **face-up** (fog,
//! §8.1). Time advances in **Days**; each character may **move one adjacent space** and attempt one
//! encounter per Day, and the **Day boundary fully resets** combat state (§8.2, handled by the
//! encounter). **Run victory** = clear the **objective** location at its max level; the run is
//! scored in **Days** (§8.2 golf — run defeat is deferred).

use engine::Rng;
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

    /// The role-track rewards a party has unlocked so far (§8.3): a location of suit `Y` cleared to
    /// level `N` unlocks `(Y, 1..=N)`. Generic (Gold) locations mint no reward suit. **De-duplicated**:
    /// with a per-level ladder (several cards of the same suit), clearing more than one tier — or a
    /// high tier that subsumes the lower ones — would otherwise emit the same `(suit, level)` twice;
    /// each unlock is reported once. The build is a pure function of the clear markers (§0.1).
    pub fn unlocked(&self) -> Vec<(Currency, u32)> {
        let mut out = Vec::new();
        for (loc, &lvl) in self.locations.iter().zip(&self.cleared) {
            if loc.currency == Currency::Gold {
                continue; // generic locations are not a reward suit (§8.5)
            }
            for level in 1..=lvl {
                let reward = (loc.currency, level);
                if !out.contains(&reward) {
                    out.push(reward);
                }
            }
        }
        out
    }
}

/// The five reward **suits**, in canonical order — the grind tracks (Gold is the generic, non-reward
/// suit, §8.5, excluded). Each appears as five level cards in [`base_locations`].
pub const REWARD_SUITS: [Currency; 5] = [
    Currency::Iron,
    Currency::Silver,
    Currency::Brass,
    Currency::Bone,
    Currency::Salt,
];

/// The **25 base grind locations** (§8.3): one card per `(suit, level)`, five reward suits × levels
/// `1..=5` — the experience-grind base. Each card is a single-tier clear whose `max_level` *is* its
/// level, so clearing it grants that suit's rewards `1..=level` cumulatively: a higher card **subsumes
/// the lower ones** (they become skippable, though difficulty + travel cost discourage leaping ahead).
/// Order is suit-major (Iron L1..L5, Silver L1..L5, …); coordinates are placeholders until placed by
/// [`place_on_grid`].
pub fn base_locations() -> Vec<Location> {
    let mut out = Vec::with_capacity(25);
    for suit in REWARD_SUITS {
        for level in 1..=5u32 {
            out.push(Location {
                name: format!("{} L{level}", suit.label()),
                coord: Coord::new(0, 0),
                currency: suit,
                max_level: level,
            });
        }
    }
    out
}

/// Place up to 25 `locations` onto a shuffled **5×5 grid**, deterministically by `seed` — so a world
/// is **reproducible** (a reference/test scenario passes a fixed seed for a predictable layout, like
/// the combat seed). Only coordinates are assigned; card *order* is preserved. A seeded Fisher–Yates
/// shuffle of the 25 cells picks each card's cell. Panics if `locations.len() > 25`.
pub fn place_on_grid(mut locations: Vec<Location>, seed: u64) -> Vec<Location> {
    assert!(
        locations.len() <= 25,
        "a 5x5 grid holds at most 25 locations"
    );
    let mut cells: Vec<Coord> = (0..5)
        .flat_map(|row| (0..5).map(move |col| Coord::new(col, row)))
        .collect();
    let mut rng = Rng::new(seed);
    for i in (1..cells.len()).rev() {
        cells.swap(i, rng.below(i + 1));
    }
    for (loc, cell) in locations.iter_mut().zip(cells) {
        loc.coord = cell;
    }
    locations
}

/// Build a **base grind run**: the 25 `(suit, level)` cards on a seeded random 5×5 grid. `objective`
/// and `start` index [`base_locations`] order (suit-major). The full grid is 4-connected, so every
/// card is reachable for **any** seed; the seed only permutes *where* each card sits (and thus travel
/// distances / par). Scenario-specific special locations are layered on top of this base elsewhere.
pub fn base_grind_run(seed: u64, objective: usize, start: usize, party: usize) -> Run {
    let locations = place_on_grid(base_locations(), seed);
    Run::new(Layout::Grid, locations, objective, start, party)
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

    #[test]
    fn base_locations_are_25_one_card_per_suit_per_level() {
        let locs = base_locations();
        assert_eq!(locs.len(), 25);
        for suit in REWARD_SUITS {
            assert_ne!(suit, Currency::Gold, "the grind suits exclude Gold");
            for level in 1..=5 {
                assert!(
                    locs.iter()
                        .any(|l| l.currency == suit && l.max_level == level),
                    "missing {} L{level}",
                    suit.label()
                );
            }
        }
    }

    #[test]
    fn clearing_a_higher_card_subsumes_the_lower_ones() {
        // Iron L4 cleared alone ⇒ Iron 1..=4 unlocked; the lower Iron cards are never needed, and L5
        // is not granted. (Suit-major order: Iron is index 0..5, so Iron L4 is index 3.)
        let mut run = base_grind_run(1, 24, 0, 1);
        let iron_l4 = 3;
        assert_eq!(run.locations[iron_l4].currency, Currency::Iron);
        assert_eq!(run.locations[iron_l4].max_level, 4);
        run.record_clear(iron_l4, run.locations[iron_l4].max_level);
        let unlocked = run.unlocked();
        for level in 1..=4 {
            assert!(unlocked.contains(&(Currency::Iron, level)));
        }
        assert!(!unlocked.contains(&(Currency::Iron, 5)));
    }

    #[test]
    fn unlocked_dedups_across_a_suits_ladder() {
        // Clearing both Iron L2 (index 1) and Iron L4 (index 3) yields the cumulative union once —
        // no duplicate (Iron, 1) / (Iron, 2).
        let mut run = base_grind_run(1, 24, 0, 1);
        run.record_clear(1, run.locations[1].max_level);
        run.record_clear(3, run.locations[3].max_level);
        let unlocked = run.unlocked();
        assert_eq!(
            unlocked
                .iter()
                .filter(|&&x| x == (Currency::Iron, 1))
                .count(),
            1,
            "each unlock appears once"
        );
        for level in 1..=4 {
            assert!(unlocked.contains(&(Currency::Iron, level)));
        }
    }

    #[test]
    fn grid_placement_is_seeded_reproducible_and_full() {
        let coords = |r: &Run| r.locations.iter().map(|l| l.coord).collect::<Vec<_>>();
        let a = base_grind_run(7, 24, 0, 1);
        let b = base_grind_run(7, 24, 0, 1);
        let c = base_grind_run(8, 24, 0, 1);
        assert_eq!(coords(&a), coords(&b), "same seed ⇒ identical layout");
        assert_ne!(coords(&a), coords(&c), "different seed ⇒ different layout");
        // Every card sits on its own cell, all within the 5×5 grid (25 distinct cells).
        let mut cells = coords(&a);
        cells.sort_by_key(|c| (c.row, c.col));
        cells.dedup();
        assert_eq!(cells.len(), 25, "every card on a distinct cell");
        assert!(
            a.locations
                .iter()
                .all(|l| (0..5).contains(&l.coord.col) && (0..5).contains(&l.coord.row))
        );
    }
}
