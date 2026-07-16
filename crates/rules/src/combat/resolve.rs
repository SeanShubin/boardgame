//! **The combat primitives** - the small, order-free machine every strike resolves through, and the unit it
//! resolves over. No rules of *formation* live here (those are [`super::regions`]); this is only the physics of
//! a single contested exchange: a bid, a slip, a batch of blows.
//!
//! It is deliberately tiny and rank-free. The regions model does have a [`Rank`](super::regions::Rank)
//! (Vanguard / Rearguard / Outrider), but rank governs *geometry* up in [`super::regions`] and is kept out of the
//! physics entirely: a body's reach down here is just its `melee`/`ranged` flags, and nothing in this file
//! mentions a rank at all.

/// Which side a combatant fights for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Party,
    Foe,
}

/// **A creature's instinct** - its deterministic, card-expressible behaviour when the game (not a player) drives
/// it. One line on a card, one branch in [`super::regions::foe_acts`]. Heroes never have an instinct (a player
/// chooses); it only steers scripted foes.
///
/// It exists because the scripted default was actively *wrong* for some creatures: a body that hunts the
/// weakest hero will **leave its own screening post** to do it, exposing the cannon it was meant to shield. A
/// per-creature instinct lets a wall be a wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instinct {
    /// **Hunt.** Go for the weakest body on the board, wherever it stands - clash it, or raid past a line for
    /// it. The aggressive default.
    HuntWeakest,
    /// **Hold the line.** Never leave this region (never raid, never slip): stand your post and strike whatever
    /// enemy front you can reach, so the body behind you stays screened. What makes a wall a wall.
    HoldTheLine,
}

/// A combatant during a fight - the scratch unit the resolver works over.
///
/// The five stats are the shared chassis: **Might** (damage per strike), **Vitality** (Health cards),
/// **Grit** (the bar a damage pile must clear to flip a card), **Cadence** (the Tempo pool), **Finesse** (the
/// bid multiplier - what one Tempo card is worth in a reach/slip contest).
#[derive(Clone, Debug)]
pub struct Combatant {
    pub name: String,
    pub side: Side,
    /// Damage each of this unit's strikes deals.
    pub might: u32,
    /// The bid multiplier: one flipped tempo card is worth `finesse` toward a bid.
    pub finesse: u32,
    /// Full tempo pool, refreshed each round.
    pub cadence: u32,
    /// The health bar: accumulated damage flips a health card each `grit` crossed.
    pub grit: u32,
    /// A flat per-strike floor: a strike whose Might does not exceed it deals nothing, and more Cadence (more
    /// strikes) cannot change that. Zero for the current roster - kept because the resolver already honours it.
    pub armor: u32,
    /// Carries a **melee** blow. Independent of `ranged`.
    pub melee: bool,
    /// Carries a **ranged** shot. Independent of `melee`.
    pub ranged: bool,
    /// Carries an **area** strike (a Sweep / Salvo). See [`super::regions`] for what it reaches.
    pub aoe: bool,
    /// A **horde**: one body whose `health` is a *body count* of one-Health members (a swarm).
    pub horde: bool,
    /// Tempo cards left this round.
    pub tempo: u32,
    /// Face-up health cards remaining; 0 ⇒ fallen at the next boundary.
    pub health: u32,
    /// Damage accumulated this round; closes at the Reset ([`refresh_round`]).
    pub(crate) pending: u32,
    pub fallen: bool,
    /// How the game drives this body when it is a scripted foe. Ignored for a hero (a player chooses).
    pub instinct: Instinct,
}

impl Combatant {
    /// Build a fresh combatant at full Health and full Tempo. `stats` is `[Might, Vitality, Grit, Cadence,
    /// Finesse]`; Finesse and Grit floor at 1.
    pub fn from_stats(
        name: impl Into<String>,
        side: Side,
        stats: [u8; 5],
        armor: u32,
        melee: bool,
        ranged: bool,
    ) -> Self {
        let [might, vitality, grit, cadence, finesse] = stats.map(u32::from);
        Combatant {
            name: name.into(),
            side,
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
            instinct: Instinct::HuntWeakest,
        }
    }

    /// Builder: set this body's [`Instinct`] (its scripted-foe behaviour).
    pub fn with_instinct(mut self, instinct: Instinct) -> Self {
        self.instinct = instinct;
        self
    }

    /// Builder: mark this body as carrying an **area** strike.
    pub fn with_aoe(mut self, aoe: bool) -> Self {
        self.aoe = aoe;
        self
    }

    /// Builder: make this a **horde** of one-Health bodies (its Vitality is the body count).
    pub fn as_horde(mut self, horde: bool) -> Self {
        self.horde = horde;
        self
    }
}

/// An **engagement**: `attacker` commits `cards` tempo to reach `target`. The tempo is spent whatever happens;
/// committing more only makes the target more expensive to slip - it buys no extra damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Engage {
    pub attacker: usize,
    pub target: usize,
    pub cards: u32,
}

/// A target's answer to everything reaching for it. There is no partial slip: by now the price is known
/// exactly, so you either pay [`slip_cost`] in full or you stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dodge {
    /// Spend nothing; they reach you.
    Stand,
    /// Pay [`slip_cost`] and break **every** engagement reaching you.
    Slip,
}

/// An established **contact**: `attacker` reached `target`, having committed `bid` value (`cards x finesse`) -
/// the value a slip must strictly exceed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact {
    pub attacker: usize,
    pub target: usize,
    pub bid: u32,
}

/// The tempo `defender` must spend to **slip** - to break every engagement currently reaching it. One dodge
/// covers your body, so the price is set by the *largest* commitment against you: enough cards that
/// `cards x finesse` strictly exceeds it. `None` when nothing is reaching you.
///
/// This is why Finesse defends without touching damage: a high-Finesse body is cheap to slip with, hence
/// expensive to catch, so an attacker must commit more tempo - and every card committed is a card it cannot
/// turn into a blow.
pub fn slip_cost(units: &[Combatant], contacts: &[Contact], defender: usize) -> Option<u32> {
    let worst = contacts
        .iter()
        .filter(|c| c.target == defender)
        .map(|c| c.bid)
        .max()?;
    Some(worst / units[defender].finesse.max(1) + 1)
}

/// Whether `unit` has an enemy in a **melee edge** it can answer along: the body it engaged, or - the mutual
/// rule - a body that engaged *it* with a melee reach (a shot from range is never answered).
///
/// This is the rank-free form of the old `strike_target`: reach is read from the `melee`/`ranged` flags, never
/// a rank. A ranged attacker is one whose reach was ranged and *not* melee - it fired from afar and never came
/// within answering distance.
pub fn can_answer(units: &[Combatant], contacts: &[Contact], unit: usize) -> Option<usize> {
    if let Some(c) = contacts.iter().find(|c| c.attacker == unit) {
        return Some(c.target);
    }
    contacts
        .iter()
        .find(|c| {
            let a = &units[c.attacker];
            c.target == unit && units[unit].melee && (!a.ranged || a.melee)
        })
        .map(|c| c.attacker)
}

/// **Evade.** `dodges` is index-aligned with `units`. A [`Dodge::Slip`] pays [`slip_cost`] and breaks every
/// engagement reaching that unit; a [`Dodge::Stand`] spends nothing. Returns the *established* contacts. A slip
/// the unit cannot afford is not a failed slip - it is not a slip at all, and the unit stands.
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
            continue;
        };
        if cost > units[i].tempo {
            continue;
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

/// End a sub-phase: mark units at zero health **fallen**, and **close the damage pile**. A committed blow still
/// landed even if its striker died here. (EXPLORATION: the pile now closes at every strike boundary, not only at
/// the Reset - so sub-threshold damage never carries between strikes, only blows within one strike combine.)
pub fn end_sub_phase(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        if u.health == 0 {
            u.fallen = true;
        }
        u.pending = 0;
    }
}

/// **The Reset** - the round boundary, and the one deadline in a fight. Tempo stands back up to Cadence (leftover
/// does not carry), and the accumulated damage pile **closes**: sub-threshold damage that never turned a Health
/// card is gone. A horde resets like anyone else - its size is spent as a **body-count volley** (see [`super`]'s
/// `land`) and a **body-count reach** (see `engage`), not as extra tempo.
pub fn refresh_round(units: &mut [Combatant]) {
    for u in units.iter_mut() {
        u.tempo = u.cadence;
        u.pending = 0;
    }
}
