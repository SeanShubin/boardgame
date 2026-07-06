//! A **derived 2D combat layout** (§4 sub-phase-schedule model): a pure, read-only view of the
//! board as *side × rank × slot*, computed from existing [`State`] — never authoritative state that
//! could desync.
//!
//! The combat board is stored as two 1D pools ([`State::heroes`] / [`State::creatures`]) plus a
//! per-unit [`Intention`] tag and a per-unit group id (`State::plan.*_intent` / `*_group`). There is no
//! explicit 2D geometry in `State`. [`CombatLayout`] reconstructs one on demand:
//!
//! - Two **sides**: 0 = heroes, 1 = creatures.
//! - Each side has three **ranks**, one per [`Intention`]: Vanguard (front / hold), Outrider (flank /
//!   break), Rearguard (back / deal). A rank's units are exactly the side's units whose declared
//!   intention is that rank.
//! - Each rank is an ordered list of **slots**, in the unit's pool order. That order *is* the
//!   front-to-back / spillover order (§4.5): the first slot of a rank is the front of that rank.
//! - A [`Slot`] records the unit's pool index, its group id, and a `down` flag.
//! - **Group adjacency:** two slots in the same rank are adjacent group members iff they carry the
//!   same group id ([`Rank::group_runs`] returns the contiguous runs of equal group id).
//!
//! **Downed actors are included** (each [`Slot`] carries `down: bool`) rather than skipped, so the
//! layout describes the full declared formation; callers that only want the live board filter on
//! `!down`. This keeps the view a faithful function of `State` (skipping would lose pool indices).
//!
//! Named `CombatLayout` (not `Layout`) to avoid colliding with [`crate::world::Layout`] (the world-map
//! grid/hex layout), which is already re-exported at the crate root.

use serde::{Deserialize, Serialize};

use crate::actor::Intention;
use crate::state::State;

/// One unit's place in a rank: which unit (its index in its side's pool), its group id, and whether it
/// is currently downed. Slots within a [`Rank`] are kept in pool order (the spillover order, §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// The unit's index in its side's pool (`State::heroes` / `State::creatures`).
    pub unit: usize,
    /// The unit's group id this round (units sharing an id are one group, §4.5).
    pub group: usize,
    /// Whether the unit is currently downed (all health cards face down). Included, not skipped.
    pub down: bool,
}

/// One rank of one side: the ordered slots of all units that declared a given [`Intention`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rank {
    /// Which intention this rank holds (Vanguard / Outrider / Rearguard).
    pub intention: Intention,
    /// The rank's units, in pool order (front to back / spillover order, §4.5).
    pub slots: Vec<Slot>,
}

impl Rank {
    /// The contiguous runs of equal group id within this rank, as ranges of indices into [`Rank::slots`].
    /// Each run is a maximal block of adjacent group members; ungrouped (distinct-id) units form
    /// singleton runs. Order follows `slots`.
    pub fn group_runs(&self) -> Vec<std::ops::Range<usize>> {
        let mut runs = Vec::new();
        let mut i = 0;
        while i < self.slots.len() {
            let g = self.slots[i].group;
            let start = i;
            while i < self.slots.len() && self.slots[i].group == g {
                i += 1;
            }
            runs.push(start..i);
        }
        runs
    }
}

/// One side of the board: its three ranks in fixed Vanguard / Outrider / Rearguard order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideLayout {
    /// 0 = heroes, 1 = creatures.
    pub side: u8,
    pub vanguard: Rank,
    pub outrider: Rank,
    pub rearguard: Rank,
}

impl SideLayout {
    /// The rank holding a given intention.
    pub fn rank(&self, intention: Intention) -> &Rank {
        match intention {
            Intention::Vanguard => &self.vanguard,
            Intention::Outrider => &self.outrider,
            Intention::Rearguard => &self.rearguard,
        }
    }
}

/// The full derived 2D combat layout: both sides, each as three ranks of ordered slots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatLayout {
    /// Index 0 = heroes, index 1 = creatures.
    pub sides: [SideLayout; 2],
}

impl CombatLayout {
    /// Derive the layout from a [`State`] — a pure function of `s_pool` + `s_intent` + `s_group`.
    pub fn of(state: &State) -> Self {
        let sides = [build_side(state, 0), build_side(state, 1)];
        CombatLayout { sides }
    }

    /// The layout for one side.
    pub fn side(&self, side: u8) -> &SideLayout {
        &self.sides[side as usize]
    }
}

/// Build one side's three ranks from its pool, intentions, and group ids, preserving pool order.
fn build_side(state: &State, side: u8) -> SideLayout {
    let pool = state.s_pool(side);
    let intent = state.s_intent(side);
    let group = state.s_group(side);
    let mut sl = SideLayout {
        side,
        vanguard: Rank {
            intention: Intention::Vanguard,
            slots: Vec::new(),
        },
        outrider: Rank {
            intention: Intention::Outrider,
            slots: Vec::new(),
        },
        rearguard: Rank {
            intention: Intention::Rearguard,
            slots: Vec::new(),
        },
    };
    for (i, actor) in pool.iter().enumerate() {
        // Default to Vanguard / own-index if the plan vectors are short (defensive; they are sized to
        // the pool by `Round::sized`).
        let intention = intent.get(i).copied().unwrap_or(Intention::Vanguard);
        let g = group.get(i).copied().unwrap_or(i);
        let slot = Slot {
            unit: i,
            group: g,
            down: actor.is_down(),
        };
        match intention {
            Intention::Vanguard => sl.vanguard.slots.push(slot),
            Intention::Outrider => sl.outrider.slots.push(slot),
            Intention::Rearguard => sl.rearguard.slots.push(slot),
        }
    }
    sl
}

impl State {
    /// The derived 2D combat layout (§4): *side × rank × slot* with group adjacency. A pure view of the
    /// current declared formation — see [`CombatLayout`].
    pub fn layout(&self) -> CombatLayout {
        CombatLayout::of(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{Attack, Intention};
    use crate::scenarios::build_character;

    /// A bare melee combatant (mirrors `combat`'s test helper) — enough to populate a pool.
    fn fighter(name: &str, might: u32, vit: u32, tough: u32) -> crate::actor::Actor {
        let mut a = build_character("Novice", &[]);
        a.name = name.into();
        a.attack = Attack::Melee;
        a.offense.might = might;
        a.offense.finesse = a.offense.finesse.max(1);
        a.defense.health = crate::stats::Health::new(vit, tough);
        a.tempo = 10;
        a
    }

    /// Build a small battle, declare mixed intentions + a shared group, and assert each unit lands in
    /// the right side / rank / slot with correct group adjacency.
    #[test]
    fn layout_places_units_by_side_rank_and_group() {
        // Heroes: H0 Vanguard (group 7), H1 Vanguard (group 7), H2 Rearguard (group 2).
        // The two grouped Vanguards must be adjacent group members; the Rearguard sits alone.
        let heroes = vec![
            fighter("H0", 2, 3, 1),
            fighter("H1", 2, 3, 1),
            fighter("H2", 1, 2, 1),
        ];
        // Creatures: C0 Outrider, C1 Vanguard.
        let foes = vec![fighter("C0", 2, 3, 1), fighter("C1", 2, 3, 1)];
        let mut state = crate::game::battle_state(heroes, foes, false, 3);

        state.plan.hero_intent = vec![
            Intention::Vanguard,
            Intention::Vanguard,
            Intention::Rearguard,
        ];
        state.plan.hero_group = vec![7, 7, 2];
        state.plan.foe_intent = vec![Intention::Outrider, Intention::Vanguard];
        state.plan.foe_group = vec![0, 1];

        let layout = state.layout();

        // --- Heroes (side 0). ---
        let h = layout.side(0);
        assert_eq!(h.side, 0);
        // Vanguard rank: H0 then H1, in pool order, both group 7.
        assert_eq!(h.vanguard.slots.len(), 2);
        assert_eq!(h.vanguard.slots[0].unit, 0);
        assert_eq!(h.vanguard.slots[1].unit, 1);
        assert_eq!(h.vanguard.slots[0].group, 7);
        assert_eq!(h.vanguard.slots[1].group, 7);
        // Adjacency: the two grouped Vanguards form a single contiguous run.
        assert_eq!(h.vanguard.group_runs(), vec![0..2]);
        // Outrider rank empty; Rearguard rank holds H2 alone (its own run).
        assert!(h.outrider.slots.is_empty());
        assert_eq!(h.rearguard.slots.len(), 1);
        assert_eq!(h.rearguard.slots[0].unit, 2);
        assert_eq!(h.rearguard.slots[0].group, 2);
        assert_eq!(h.rearguard.group_runs(), vec![0..1]);

        // --- Creatures (side 1). ---
        let c = layout.side(1);
        assert_eq!(c.side, 1);
        assert_eq!(c.vanguard.slots.len(), 1);
        assert_eq!(c.vanguard.slots[0].unit, 1);
        assert_eq!(c.outrider.slots.len(), 1);
        assert_eq!(c.outrider.slots[0].unit, 0);
        assert!(c.rearguard.slots.is_empty());

        // The down flag tracks is_down (all start up).
        assert!(!h.vanguard.slots[0].down);

        // Round-trips through RON like the rest of State's parts.
        let text = ron::ser::to_string(&layout).expect("layout serializes");
        let back: CombatLayout = ron::from_str(&text).expect("and deserializes");
        assert_eq!(back, layout);
    }

    /// Two adjacent groups in the same rank produce two separate runs (adjacency respects group id).
    #[test]
    fn group_runs_split_on_group_change() {
        let rank = Rank {
            intention: Intention::Vanguard,
            slots: vec![
                Slot {
                    unit: 0,
                    group: 1,
                    down: false,
                },
                Slot {
                    unit: 1,
                    group: 1,
                    down: false,
                },
                Slot {
                    unit: 2,
                    group: 4,
                    down: false,
                },
                Slot {
                    unit: 3,
                    group: 1,
                    down: false,
                },
            ],
        };
        // The trailing group-1 slot is NOT adjacent to the leading group-1 run (a group-4 slot splits
        // them): three runs, not two.
        assert_eq!(rank.group_runs(), vec![0..2, 2..3, 3..4]);
    }
}
