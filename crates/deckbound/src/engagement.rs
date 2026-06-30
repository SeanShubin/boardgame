//! A **balance front-end** over the canon engagement-schedule combat model (Spec §4 / §4.5 / §4.6). It is
//! a thin authoring + reporting layer: a "class" is an attack (range × shape) plus a five-stat allocation,
//! the role *emerges* from range+stats ([`intention_for`]), and the hold/break/deal triangle, the
//! single-vs-AoE trade, and the counterability of extreme formations are probed without the full game loop.
//!
//! **It runs on the REAL engine.** Earlier this module re-implemented the whole resolver (its own schedule,
//! cycling, groups, policy) as a parallel copy that had to be hand-synced — and drifted (the
//! default-intention rule fell behind canon). That copy is **gone**: [`battle`] / [`survivors_after`] /
//! [`battle_traced`] now build real [`Actor`]s from the [`Unit`] specs and drive the live
//! [`combat::resolve_round`], so a balance number here is exactly what the shipped game produces. The only
//! thing this layer still owns is the **default policy** stand-in (which intention a body declares, via
//! [`intention_for`]) — the load-bearing assumption for approximating uncomputable hidden-simultaneous PvP.
//!
//! The damage core (Might into a per-engagement pile, Toughness gates the flip) and the evade/endure rule
//! (a Vanguard endures, others evade — [`crate::policy::role_evades`]) live in the engine. The per-unit
//! [`HitMode`] / `hits` field on [`Unit`] is therefore **informational** (the role default); the resolver
//! does not read it.

pub use crate::actor::Intention;
use crate::actor::{Actor, Attack};
use crate::cards::{Card, Effect};
use crate::combat;
use crate::game::battle_state_with;
use crate::ruleset::Ruleset;
use crate::state::State;
use crate::stats::{Defense, Offense};
use serde::Deserialize;

/// How a unit answers incoming blows (the **hit policy**, part of a [`Strategy`]) — the one card-writable
/// reaction knob. `Evade`: spend Tempo to dodge a blow that would flip a card, when affordable (and so
/// arrive at later engagements poorer). `Endure`: never spend Tempo on defense — eat every blow and keep
/// the whole pool for offense (the reckless "ignore hits to reach the back" line).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HitMode {
    Evade,
    Endure,
}

/// A lean combatant for the sim: the stat block (reusing the real [`Offense`]/[`Defense`] so the damage
/// math is identical to the live game), an attack range, a side, an intention, the hit policy, and the
/// round Tempo pool.
#[derive(Clone, Debug)]
pub struct Unit {
    pub name: String,
    pub side: u8,
    pub intent: Intention,
    pub offense: Offense,
    pub defense: Defense,
    pub attack: Attack,
    /// How this unit reacts to incoming blows (Evade vs Endure) — set by its [`Strategy`].
    pub hits: HitMode,
    /// Whether this unit's strike is **area** (§4.5): it hits **every** member of the target's group at
    /// full Might, is **unevadable**, and bypasses the spillover bodyguard. `false` = a single aimed blow.
    pub aoe: bool,
    /// Group binding (§4.5). `None` = a lone unit (its own singleton group). `Some(g)` binds it to the
    /// same-side units sharing `g`: they **reposition collectively** (block / spill / a melee crossing pays
    /// every member) but **target individually**. Vector order within a group is front-to-back.
    pub group: Option<u32>,
    /// Round-scoped Tempo pool (= Cadence), shared across the whole schedule.
    pub tempo: i32,
}

/// `(Might, Vitality, Toughness, Cadence, Finesse)` — same tuning tuple as the level harness.
pub type Stat5 = (u32, u32, u32, u32, u32);

/// The default **intention assignment** (part of the policy): ranged bodies deal from the Rearguard;
/// **aggressive** melee (Might ≥ Toughness — a glassy striker) breaks the line as an Outrider; **durable**
/// melee (Toughness > Might — a wall) holds as a Vanguard. Predictable from the stat line, so a player
/// knows what their stand-in will pick. (Might-vs-Toughness, not Finesse — the re-tuned tank carries F2 to
/// run down the Outrider, so Finesse no longer separates the wall from the skirmisher.)
pub fn default_intention(class: &str, s: Stat5) -> Intention {
    intention_for(class == "Mage", s)
}

/// The intention a unit emerges into, from its **explicit range** and stat line (the data-driven core of a
/// [`ClassDef`] — the role is not a name, it falls out of range+stats): a **ranged** body deals from the
/// Rearguard; an **aggressive** melee (Might ≥ Toughness, a glassy striker) breaks the line as an Outrider;
/// a **durable** melee (Toughness > Might, a wall) holds as a Vanguard. [`default_intention`] delegates here
/// (treating `class == "Mage"` as the only ranged class) so the legacy name-based call keeps its behavior.
pub fn intention_for(ranged: bool, s: Stat5) -> Intention {
    let (might, _, toughness, _, _) = s;
    if ranged {
        Intention::Rearguard
    } else if might >= toughness {
        Intention::Outrider
    } else {
        Intention::Vanguard
    }
}

/// A **generic class**: a combatant defined purely by an attack (range × shape) plus a 5-stat allocation —
/// no Suit / Power identity. The role *emerges* from `ranged` + `stats` via [`intention_for`]; `aoe` flips
/// the strike from a single aimed blow to an area strike (§4.5). Deserialized from a RON class set so the
/// roster is editable at runtime (edit the `.ron`, re-run the `classes` example, no rebuild).
#[derive(Clone, Debug, Deserialize)]
pub struct ClassDef {
    pub name: String,
    pub ranged: bool,
    pub aoe: bool,
    pub stats: Stat5,
}

/// Build a sim [`Unit`] from a [`ClassDef`]: the attack is `Ranged` iff `c.ranged` (else `Melee`); `aoe`
/// and the stats copy across; the intention emerges from range+stats ([`intention_for`]) and the hit policy
/// follows that role ([`default_hits`]). The weapon is implicit power-0, so a strike's raw force is Might.
pub fn unit_from_class(c: &ClassDef, side: u8) -> Unit {
    let (m, v, t, ca, f) = c.stats;
    let intent = intention_for(c.ranged, c.stats);
    Unit {
        name: c.name.clone(),
        side,
        intent,
        offense: Offense {
            might: m,
            cadence: ca,
            finesse: f,
        },
        defense: Defense::new(v, t),
        attack: if c.ranged {
            Attack::Ranged
        } else {
            Attack::Melee
        },
        hits: default_hits(intent),
        aoe: c.aoe,
        group: None,
        tempo: ca as i32,
    }
}

/// Load a RON class set (a `[ClassDef]` list) from `path`. Panics with a readable message on a read/parse
/// failure — this is a runtime balance tool, not production code. Exposed for the `classes` example runner.
pub fn load_classes(path: &std::path::Path) -> Vec<ClassDef> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read class file {}: {e}", path.display()));
    ron::from_str(&text)
        .unwrap_or_else(|e| panic!("cannot parse class file {}: {e}", path.display()))
}

/// The thematic **default hit policy** by role: a **Vanguard endures** — it holds the line and *takes*
/// the blows (and so keeps all its Tempo for offense); the fragile **Outrider** and **Rearguard evade**.
/// This is load-bearing under cycling: an *evading* tank would dodge the Rearguard's fire and break
/// Deal>Hold, so "the wall stands and takes it" is what keeps the glass cannon's counter to the turtle.
pub fn default_hits(intent: Intention) -> HitMode {
    match intent {
        Intention::Vanguard => HitMode::Endure,
        Intention::Outrider | Intention::Rearguard => HitMode::Evade,
    }
}

/// Build a sim unit from a class name + stat tuple, assigning its default intention and hit policy. `Mage`
/// is ranged; the others melee. The weapon is implicit power-0, so a strike's raw force is Might.
pub fn make_unit(class: &str, s: Stat5, side: u8) -> Unit {
    let (m, v, t, c, f) = s;
    let attack = if class == "Mage" {
        Attack::Ranged
    } else {
        Attack::Melee
    };
    let intent = default_intention(class, s);
    Unit {
        name: class.to_string(),
        side,
        intent,
        offense: Offense {
            might: m,
            cadence: c,
            finesse: f,
        },
        defense: Defense::new(v, t),
        attack,
        hits: default_hits(intent),
        aoe: false,
        group: None,
        tempo: c as i32,
    }
}

// ====================================================================================================
// Delegation to the REAL engine. The sim no longer re-implements the resolver/policy/schedule — it
// builds real [`Actor`]s from the [`Unit`] specs, drops them into a combat [`State`], and runs the live
// [`combat::resolve_round`] (the same code the game ships). So the balance numbers come from the engine
// itself, not a hand-synced copy that can drift (it once did — the default-intention rule). The per-unit
// `hits` field is informational here: the real resolver derives Endure/Evade from role
// ([`crate::policy::role_evades`] — a Vanguard endures, others evade), not from a per-Actor knob.
// ====================================================================================================

/// A fixed seed for the headless battles. Combat with the Clash module off is deterministic (no rng in
/// [`combat::resolve_round`]), so the exact value does not matter — only that it is stable.
const SIM_SEED: u64 = 1;

/// A generic, suit-free **strike card** for the balance harness — the capability that grants the base
/// strike (§4.3). Its `reach` sets the range (melee `[1,1]` / ranged `[2,2]`) and `targets` the shape
/// (single `1` / area `2`), so the four cards cover the whole **melee/ranged × single/aoe** matrix. The
/// weapon is power 0, so a strike's raw force is exactly Might (the sim's pricing). An Actor's
/// `attack`/`aoe` derive from this card (§4.3), the same as in the live build path.
fn strike_card(ranged: bool, aoe: bool) -> Card {
    let name = match (ranged, aoe) {
        (false, false) => "Jab",  // melee · single
        (true, false) => "Shot",  // ranged · single
        (false, true) => "Sweep", // melee · area
        (true, true) => "Salvo",  // ranged · area
    };
    Card {
        name: name.to_string(),
        reach: if ranged { [2, 2] } else { [1, 1] },
        targets: if aoe { 2 } else { 1 },
        effects: vec![Effect::Damage { power: 0 }],
        ..Card::default()
    }
}

/// Build a real [`Actor`] from a sim [`Unit`] spec: a bare clean-slate body whose five stats and
/// **strike card** come from the spec. The generic strike card ([`strike_card`]) is the **source** of the
/// attack profile — `attack`/`aoe` derive from its `reach`/`targets` (§4.3), exactly as the live build
/// path does — so the harness exercises capabilities-as-cards end to end. Tempo is set from Cadence by
/// `refresh_round`.
fn unit_to_actor(u: &Unit) -> Actor {
    let mut a = crate::scenarios::build_character("Novice", &[]);
    a.name = u.name.clone();
    a.offense = u.offense;
    a.defense = u.defense.clone();
    let weapon = strike_card(matches!(u.attack, Attack::Ranged), u.aoe);
    a.attack = Attack::from_reach(weapon.reach);
    a.aoe = weapon.targets > 1;
    a.weapon = weapon;
    a.refresh_round();
    a
}

/// Map the sim's `Option<u32>` group tags to the engine's per-unit group-id vector (units sharing a tag
/// share a group; an untagged unit is its own singleton at its index — the engine's `group_of` keys off
/// equal ids). Same-tag units collapse to the first such index.
fn group_ids(units: &[Unit]) -> Vec<usize> {
    units
        .iter()
        .enumerate()
        .map(|(i, u)| match u.group {
            None => i,
            Some(g) => units.iter().position(|v| v.group == Some(g)).unwrap_or(i),
        })
        .collect()
}

/// Assemble a combat [`State`] from the two sides' [`Unit`] specs: real Actors, the declared intentions,
/// and the group layout, under the analysis [`Ruleset`]. The round loop drives [`combat::resolve_round`]
/// over it directly (no Marshal/Deploy action layer — these are stat-only bodies with no cards to play).
fn battle_state_from(side0: &[Unit], side1: &[Unit]) -> State {
    let heroes: Vec<Actor> = side0.iter().map(unit_to_actor).collect();
    let foes: Vec<Actor> = side1.iter().map(unit_to_actor).collect();
    let mut state = battle_state_with(heroes, foes, false, SIM_SEED, Ruleset::analysis());
    state.plan.hero_intent = side0.iter().map(|u| u.intent).collect();
    state.plan.foe_intent = side1.iter().map(|u| u.intent).collect();
    state.plan.hero_group = group_ids(side0);
    state.plan.foe_group = group_ids(side1);
    state
}

/// Read the side-0 outcome from a resolved board, or `None` if both sides still stand (fight continues).
fn decide(state: &State) -> Option<Outcome> {
    let a = state.heroes.iter().any(|u| !u.is_down());
    let b = state.creatures.iter().any(|u| !u.is_down());
    match (a, b) {
        (true, false) => Some(Outcome::Win),
        (false, true) => Some(Outcome::Loss),
        (false, false) => Some(Outcome::Draw),
        (true, true) => None,
    }
}

/// Refresh every living body's per-round state (Tempo back to Cadence) between rounds — the engine's
/// resolver assumes Tempo is refreshed at the round start.
fn refresh(state: &mut State) {
    for a in state.heroes.iter_mut().chain(state.creatures.iter_mut()) {
        if !a.is_down() {
            a.refresh_round();
        }
    }
}

/// Run a battle while collecting a step-by-step trace (for tuning by inspection). Returns the outcome
/// and the engine's per-round combat log.
pub fn battle_traced(
    side0: Vec<Unit>,
    side1: Vec<Unit>,
    max_rounds: u32,
) -> (Outcome, Vec<String>) {
    let mut state = battle_state_from(&side0, &side1);
    state.log.clear();
    let mut log = Vec::new();
    for r in 0..max_rounds {
        log.push(format!("== round {} ==", r + 1));
        combat::resolve_round(&mut state);
        log.append(&mut state.log);
        if let Some(o) = decide(&state) {
            return (o, log);
        }
        refresh(&mut state);
    }
    (Outcome::Draw, log)
}

/// Run exactly `rounds` rounds (no early stop) and return `(side0_alive, side1_alive)` — a survivor count
/// for measuring partial progress (e.g. how many of a shielded group an attacker has cracked), where a
/// terminal [`battle`] outcome would collapse a standoff to a Draw and hide the difference.
pub fn survivors_after(side0: Vec<Unit>, side1: Vec<Unit>, rounds: u32) -> (usize, usize) {
    let mut state = battle_state_from(&side0, &side1);
    for _ in 0..rounds {
        combat::resolve_round(&mut state);
        refresh(&mut state);
    }
    let a = state.heroes.iter().filter(|u| !u.is_down()).count();
    let b = state.creatures.iter().filter(|u| !u.is_down()).count();
    (a, b)
}

/// Outcome of a battle for side 0's perspective.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Win,
    Loss,
    Draw,
}

/// Run a battle to a decision or the round cap. Win = the enemy is wiped and you have survivors; a mutual
/// wipe or a timeout is a Draw (PvE convention: a Draw is not a Win).
pub fn battle(side0: Vec<Unit>, side1: Vec<Unit>, max_rounds: u32) -> Outcome {
    let mut state = battle_state_from(&side0, &side1);
    for _ in 0..max_rounds {
        combat::resolve_round(&mut state);
        if let Some(o) = decide(&state) {
            return o;
        }
        refresh(&mut state);
    }
    Outcome::Draw
}

// ---- experiment runners: the RPS triangle + the equal-size composition matrix ----

/// A tuning triad, in F/A/M order: `[(class, stats); 3]` = hold / break / deal.
pub type Triad = [(&'static str, Stat5); 3];

fn party(counts: &[u32], triad: &Triad, side: u8) -> Vec<Unit> {
    let mut p = Vec::new();
    for (i, &(class, s)) in triad.iter().enumerate() {
        for _ in 0..counts[i] {
            p.push(make_unit(class, s, side));
        }
    }
    p
}

fn tag(o: Outcome) -> &'static str {
    match o {
        Outcome::Win => "WIN ",
        Outcome::Loss => "LOSS",
        Outcome::Draw => "draw",
    }
}

/// **The RPS triangle at 1v1.** Prints each role vs each other role. The triangle holds iff Hold beats
/// Break, Break beats Deal, and Deal beats Hold (the off-cycle direction should *not* also win).
pub fn rps_report(triad: &Triad) -> String {
    let names = ["Hold", "Break", "Deal"];
    let mut out = String::from("RPS triangle (1v1, default vs default):\n");
    for r in triad {
        let (c, (m, v, t, ca, f)) = (r.0, r.1);
        out.push_str(&format!("  {c:<9} M{m} V{v} T{t} C{ca} F{f}\n"));
    }
    let cyc = [(0usize, 1usize), (1, 2), (2, 0)];
    out.push_str("  cycle (want WIN): ");
    for &(i, j) in &cyc {
        let o = battle(
            vec![make_unit(triad[i].0, triad[i].1, 0)],
            vec![make_unit(triad[j].0, triad[j].1, 1)],
            8,
        );
        out.push_str(&format!("{}>{} {}  ", names[i], names[j], tag(o)));
    }
    out.push_str("\n  anti   (want !WIN): ");
    for &(i, j) in &cyc {
        let o = battle(
            vec![make_unit(triad[j].0, triad[j].1, 0)],
            vec![make_unit(triad[i].0, triad[i].1, 1)],
            8,
        );
        out.push_str(&format!("{}>{} {}  ", names[j], names[i], tag(o)));
    }
    out.push('\n');
    out
}

/// **Equal-size composition matrix** under mutual default play. For each size and each player
/// composition, its W/L/D record across all enemy compositions of that size. Balanced = no composition
/// dominates and none is dead.
pub fn matrix_report(triad: &Triad, max_n: u32) -> String {
    let tags = ["F", "A", "M"];
    let mut out = String::from("Engagement-model matrix (equal size, default vs default):\n\n");
    for n in 1..=max_n {
        let comps = crate::balance::compositions_k(n, 3);
        out.push_str(&format!("== size {n} ==\n"));
        for pc in &comps {
            let (mut w, mut l, mut d) = (0, 0, 0);
            for ec in &comps {
                match battle(party(pc, triad, 0), party(ec, triad, 1), 8) {
                    Outcome::Win => w += 1,
                    Outcome::Loss => l += 1,
                    Outcome::Draw => d += 1,
                }
            }
            let label: String = (0..3)
                .filter(|&i| pc[i] > 0)
                .map(|i| format!("{}{}", pc[i], tags[i]))
                .collect();
            out.push_str(&format!("  {label:<8} {w:>2}W {l:>2}L {d:>2}D\n"));
        }
        out.push('\n');
    }
    out
}

// ====================================================================================================
// Strategies & counterability — extreme, card-writable scripts and the check that a balanced party
// counters each. A Strategy is a deterministic, public-information decision tree simple enough to print
// on a card: where each unit stands (the intention plan) + how it answers a blow (the hit policy).
// Targeting is not a free choice — it follows the schedule + the **positive-effect rule** (take the
// highest-priority action that actually flips a card / lands; never a futile or self-destructive one),
// which is the engine invariant baked into `choose_target` / `should_avoid` / the strike-back gate.
// ====================================================================================================

/// How a strategy assigns each unit's **intention** (the position half of the script).
#[derive(Clone, Copy, Debug)]
pub enum IntentPlan {
    AllVanguard,
    AllOutrider,
    AllRearguard,
    /// The balanced default: by stats — ranged → Rearguard, high-Finesse melee → Outrider, else Vanguard.
    ByStats,
}

/// A **strategy**: a deterministic, public-information script — where every unit stands (`plan`). The hit
/// policy follows each unit's role default ([`default_hits`]); the targeting/positive-effect rule is the
/// engine invariant. Card-writable.
#[derive(Clone, Copy, Debug)]
pub struct Strategy {
    pub name: &'static str,
    pub plan: IntentPlan,
}

impl Strategy {
    fn intent_for(&self, class: &str, s: Stat5) -> Intention {
        match self.plan {
            IntentPlan::AllVanguard => Intention::Vanguard,
            IntentPlan::AllOutrider => Intention::Outrider,
            IntentPlan::AllRearguard => Intention::Rearguard,
            IntentPlan::ByStats => default_intention(class, s),
        }
    }
}

/// A roster entry: a class name + its stat tuple `(M, V, T, C, F)`.
pub type Recruit = (&'static str, Stat5);

/// Build one side from a roster under a strategy (assign each unit its intention; the hit policy follows
/// the role default, [`default_hits`]).
pub fn build_side(roster: &[Recruit], strat: &Strategy, side: u8) -> Vec<Unit> {
    roster
        .iter()
        .map(|&(class, s)| {
            let mut u = make_unit(class, s, side);
            u.intent = strat.intent_for(class, s);
            u.hits = default_hits(u.intent);
            u
        })
        .collect()
}

/// `n` copies of one recruit (an extreme mono-roster).
pub fn n_of(class: &'static str, s: Stat5, n: usize) -> Vec<Recruit> {
    vec![(class, s); n]
}

// The triad stats, re-tuned for the cycling model (probe_retune): hold / break / deal. The Fighter grew
// C1F1→C2F2 (and *endures* by role default) so the wall can run down the slippery Outrider under cycling
// — evasion is stronger now that Tempo refreshes each round.
const HOLD: Stat5 = (1, 2, 3, 2, 2); // Fighter — Vanguard (endures)
const BREAK: Stat5 = (2, 1, 1, 2, 2); // Assassin — Outrider
const DEAL: Stat5 = (3, 1, 2, 1, 1); // Mage — Rearguard

/// The **balanced party** (one of each role, played by stats with the Evade hit policy) — the reference a
/// counter must hold against.
pub fn balanced_party() -> (Vec<Recruit>, Strategy) {
    let roster = vec![("Fighter", HOLD), ("Assassin", BREAK), ("Mage", DEAL)];
    let strat = Strategy {
        name: "balanced",
        plan: IntentPlan::ByStats,
    };
    (roster, strat)
}

// ---- builders for the extreme scenarios (all stat lines are the triad's 8-point budget, so an extreme
// wins by *formation*, never by being bigger) ----

/// One unit with an explicit intention (and the hit policy that role defaults to).
fn lone(class: &str, s: Stat5, intent: Intention, side: u8) -> Unit {
    let mut u = make_unit(class, s, side);
    u.intent = intent;
    u.hits = default_hits(intent);
    u
}

/// `n` copies of one class at one intention (a mono-formation).
fn mono(class: &'static str, s: Stat5, intent: Intention, n: usize, side: u8) -> Vec<Unit> {
    (0..n).map(|_| lone(class, s, intent, side)).collect()
}

/// A single **group** (one shared `gid`) from `(class, stats, intention)` members in front-to-back order.
fn grouped(members: &[(&'static str, Stat5, Intention)], gid: u32, side: u8) -> Vec<Unit> {
    members
        .iter()
        .map(|&(c, s, i)| {
            let mut u = lone(c, s, i, side);
            u.group = Some(gid);
            u
        })
        .collect()
}

/// The **extreme** enemy forces (side 1), each pushing one mechanic to a limit a balanced party should
/// counter. All stat lines are 8-point (triad-budget), so the test is of formation, not raw numbers.
pub fn extreme_sides() -> Vec<(&'static str, Vec<Unit>)> {
    use Intention::{Outrider, Rearguard, Vanguard};
    let reckless = {
        let mut v = mono("Assassin", BREAK, Outrider, 3, 1);
        for u in &mut v {
            u.hits = HitMode::Endure; // ignore hits to reach the back
        }
        v
    };
    let aoe_battery = {
        let mut v = mono("Mage", DEAL, Rearguard, 3, 1);
        for u in &mut v {
            u.aoe = true; // area fire — hits a whole rank/group, unevadable, bypasses bodyguards
        }
        v
    };
    vec![
        // A wall of *generic* budget-8 tanks (not the party's beefier tuned HOLD Fighter) — the extreme
        // tests formation, so it must not out-budget the party.
        (
            "all-Vanguard wall",
            mono("Fighter", (1, 2, 3, 1, 1), Vanguard, 3, 1),
        ),
        (
            "all-Outrider (evasive)",
            mono("Assassin", BREAK, Outrider, 3, 1),
        ),
        ("all-Outrider (reckless/Endure)", reckless),
        ("all-Rearguard battery", mono("Mage", DEAL, Rearguard, 3, 1)),
        (
            "glass cannons (max Might 4, 8pt)",
            mono("Mage", (4, 1, 1, 1, 1), Rearguard, 3, 1),
        ),
        (
            "fortress (max Toughness 4, 8pt)",
            mono("Fighter", (1, 1, 4, 1, 1), Vanguard, 3, 1),
        ),
        (
            "bodyguard group (Fighter front shields 2 Mages)",
            grouped(
                &[
                    ("Fighter", HOLD, Vanguard),
                    ("Mage", DEAL, Vanguard),
                    ("Mage", DEAL, Vanguard),
                ],
                0,
                1,
            ),
        ),
        ("AoE battery (3 area Mages)", aoe_battery),
    ]
}

/// **Counterability report.** Run the balanced party against each extreme and report the result. A
/// balanced party *counters* an extreme when the extreme does **not** beat it (the balanced party WINs or
/// the engagement is a draw). A LOSS row is an uncountered extreme — a balance gap to fix.
pub fn counter_report() -> String {
    let (party, pstrat) = balanced_party();
    let mut out = String::from("Counterability — balanced party vs extreme scenarios:\n");
    for (label, force) in extreme_sides() {
        let o = battle(build_side(&party, &pstrat, 0), force, 8);
        let verdict = match o {
            Outcome::Win => "COUNTERED (party wins)",
            Outcome::Draw => "held (draw)",
            Outcome::Loss => "!! UNCOUNTERED (party loses)",
        };
        out.push_str(&format!("  {label:<48} {verdict}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_intentions_match_the_triad() {
        assert_eq!(
            default_intention("Fighter", (1, 2, 3, 1, 1)),
            Intention::Vanguard
        );
        assert_eq!(
            default_intention("Assassin", (1, 1, 1, 2, 2)),
            Intention::Outrider
        );
        assert_eq!(
            default_intention("Mage", (3, 1, 1, 1, 1)),
            Intention::Rearguard
        );
    }

    /// Counterability: the balanced party vs each extreme scenario.
    /// `cargo test -p deckbound probe_counters -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_counters() {
        println!("{}", counter_report());
    }

    /// The guarded property: the balanced party is **not beaten** by any extreme scenario — each is
    /// countered (a win) or held (a draw). A Loss row is an uncountered extreme (a balance gap to fix).
    #[test]
    fn balanced_party_counters_every_extreme() {
        let (party, pstrat) = balanced_party();
        for (label, force) in extreme_sides() {
            let o = battle(build_side(&party, &pstrat, 0), force, 8);
            assert_ne!(o, Outcome::Loss, "uncountered extreme: {label}");
        }
    }

    /// AoE bypasses the bodyguard. A **tough front** (T4) shields two squishy Mages; a lone attacker (one
    /// Tempo/round, M3) cannot crack the front with **aimed** fire (per-engagement pile wipes before it
    /// accumulates a flip), so the back Mages live — but **AoE** lands on every member at once and kills
    /// them through the shield. `cargo test -p deckbound probe_aoe_vs_group -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_aoe_vs_group() {
        use Intention::{Rearguard, Vanguard};
        let group = || {
            grouped(
                &[
                    ("Fighter", (1, 2, 4, 1, 1), Vanguard), // tough front bodyguard (T4)
                    ("Mage", (3, 1, 2, 1, 1), Vanguard),    // shielded back
                    ("Mage", (3, 1, 2, 1, 1), Vanguard),    // shielded back
                ],
                0,
                1,
            )
        };
        for aoe in [false, true] {
            let mut atk = vec![lone("Mage", (3, 1, 2, 1, 1), Rearguard, 0)];
            atk[0].aoe = aoe;
            // The T4 front survives both (M3 < T4); the two soft backs live under aimed fire (walled by
            // the bodyguard) but die under AoE — so the 3-body group leaves 3 survivors aimed, 1 under AoE.
            let (_, group_alive) = survivors_after(atk, group(), 4);
            println!(
                "{} attacker → group survivors after 4 rounds: {}/3 (front + back Mages)",
                if aoe { "AoE  " } else { "aimed" },
                group_alive,
            );
        }
    }

    /// Trace specific matchups to calibrate understanding of the resolver.
    /// `cargo test -p deckbound probe_trace_engagement -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_trace_engagement() {
        const TRIAD: Triad = [
            ("Fighter", (1, 2, 3, 2, 2)), // hold: T3 bounces the breaker; C2F2 runs it down; endures the dealer
            ("Assassin", (2, 1, 1, 2, 2)), // break: M2 cracks the dealer's T1; C2 to dodge + raid
            ("Mage", (3, 1, 2, 1, 1)), // deal: M3 cracks the tank; T2 bounces the tank's fallback M1
        ];
        for &(i, j, label) in &[(0usize, 1usize, "Hold vs Break"), (2, 0, "Deal vs Hold")] {
            let (o, log) = battle_traced(
                vec![make_unit(TRIAD[i].0, TRIAD[i].1, 0)],
                vec![make_unit(TRIAD[j].0, TRIAD[j].1, 1)],
                8,
            );
            println!("### {label}: {o:?}");
            for line in log {
                println!("{line}");
            }
            println!();
        }
    }

    /// The level-1 stat triad, ported as-is to the new model — expected to *not* hold the triangle yet
    /// (the numbers were tuned for the old phase order). `cargo test -p deckbound probe_engagement -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_engagement() {
        const TRIAD: Triad = [
            ("Fighter", (1, 2, 3, 2, 2)), // hold: T3 bounces the breaker; C2F2 runs it down; endures the dealer
            ("Assassin", (2, 1, 1, 2, 2)), // break: M2 cracks the dealer's T1; C2 to dodge + raid
            ("Mage", (3, 1, 2, 1, 1)), // deal: M3 cracks the tank; T2 bounces the tank's fallback M1
        ];
        println!("{}", rps_report(&TRIAD));
        println!("{}", matrix_report(&TRIAD, 3));
    }

    #[test]
    fn mirror_is_a_draw() {
        // Symmetric default play on identical rosters resolves to a draw (no side-0 bias).
        let s0 = vec![make_unit("Fighter", (1, 2, 3, 1, 1), 0)];
        let s1 = vec![make_unit("Fighter", (1, 2, 3, 1, 1), 1)];
        assert_eq!(battle(s0, s1, 8), Outcome::Draw);
    }
}
