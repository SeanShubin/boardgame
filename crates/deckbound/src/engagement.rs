//! The **engagement-schedule** combat model (the brainstormed redesign in
//! `log-driven/brainstorming/phases.md`), built **isolated** from the live 12-phase engine so we can
//! test whether the hold/break/deal triangle survives it before adopting it as the official spec.
//!
//! A round: declare intentions (V/O/R) → resolve a fixed engagement **schedule** (Intercept → Volley →
//! Raid → Clash → Breach) over one shared
//! per-round **Tempo** pool → reset. The damage core is unchanged (Might into a per-phase pile, Toughness
//! gates the flip — reused from [`crate::stats`]). The schedule is **permissive on purpose** (every
//! attacker→target role-pair except Rearguard→Rearguard is legal): balance must emerge from the
//! **efficiency gradient** in the *order* (an Outrider raids the back in `Raid` before the back fires in
//! `Clash`; a Vanguard only reaches it in `Breach`, too late), never from banning a pair — **force, not
//! fiat**.
//!
//! Because true PvP is simultaneous + hidden (not computable), balance is approximated by a **default
//! policy** (a reasonable, predictable human stand-in) playing both sides; we then check the roster is
//! balanced *under that policy*. The policy is the load-bearing assumption — kept simple and documented.

use crate::actor::{Attack, Range};
use crate::stats::{Defense, Offense};

/// A unit's declared role/position for the round. The intention *is* the role (hold/break/deal).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intention {
    /// Hold the line (front).
    Vanguard,
    /// Break the line (flank/raid).
    Outrider,
    /// Deal from behind (back).
    Rearguard,
}

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

impl Unit {
    fn alive(&self) -> bool {
        !self.defense.is_down()
    }
    fn is_melee(&self) -> bool {
        self.attack.has(Range::Melee)
    }
}

/// `(Might, Vitality, Toughness, Cadence, Finesse)` — same tuning tuple as the level harness.
pub type Stat5 = (u32, u32, u32, u32, u32);

/// The default **intention assignment** (part of the policy): ranged bodies deal from the Rearguard;
/// high-Finesse melee bodies break the line as Outriders; everyone else holds as a Vanguard. Predictable
/// from the stat line, so a player knows what their stand-in will pick.
pub fn default_intention(class: &str, s: Stat5) -> Intention {
    let ranged = class == "Mage";
    let (_, _, _, _, finesse) = s;
    if ranged {
        Intention::Rearguard
    } else if finesse >= 2 {
        Intention::Outrider
    } else {
        Intention::Vanguard
    }
}

/// Build a sim unit from a class name + stat tuple, assigning its default intention. `Mage` is ranged;
/// the others melee. The weapon is implicit power-0, so a strike's raw force is Might.
pub fn make_unit(class: &str, s: Stat5, side: u8) -> Unit {
    let (m, v, t, c, f) = s;
    let attack = if class == "Mage" {
        Attack::Ranged
    } else {
        Attack::Melee
    };
    Unit {
        name: class.to_string(),
        side,
        intent: default_intention(class, s),
        offense: Offense {
            might: m,
            cadence: c,
            finesse: f,
        },
        defense: Defense::new(v, t),
        attack,
        hits: HitMode::Evade,
        aoe: false,
        group: None,
        tempo: c as i32,
    }
}

/// One engagement pair: an attacker intention strikes a target intention.
type Pair = (Intention, Intention);

/// The fixed schedule, in resolution order — the *ordering* is the whole interception/Reckoning system.
/// Each step may hold several role-pairs; each step **cycles to exhaustion** (units keep committing
/// positive-effect strikes until no one will spend Tempo), and the per-phase pile wipes at the step
/// boundary. `Rearguard → Rearguard` is legal **only in the Breach, and only once the enemy Vanguard has
/// fallen** (the dropped screen opens the back-line to fire — gated in [`choose_target`]).
const SCHEDULE: &[(&str, &[Pair])] = {
    use Intention::{Outrider as O, Rearguard as R, Vanguard as V};
    &[
        ("Intercept", &[(V, O)]),
        ("Volley", &[(R, O)]),
        ("Raid", &[(O, R)]),
        ("Clash", &[(R, V), (V, V)]),
        ("Breach", &[(V, R), (O, V), (O, O), (R, R)]),
    ]
};

/// Tempo a defender must spend to **avoid** one attack: it out-bids by `cards × Finesse`, strictly
/// exceeding the attack's Finesse (one card per declared attack). So `cards × Fd > Fa` → the minimum
/// `cards` is `floor(Fa / Fd) + 1`. High defender Finesse makes dodging cheap; a high-Finesse attack
/// makes itself expensive to dodge.
fn avoid_cost(attacker_finesse: u32, defender_finesse: u32) -> i32 {
    let fd = defender_finesse.max(1);
    (attacker_finesse / fd) as i32 + 1
}

/// A declared attack, pending resolution.
struct Decl {
    atk: usize,
    def: usize,
    might: u32,
    finesse: u32,
    melee: bool,
    aoe: bool,
}

// ---- groups (§4.5): bind for defense + position, target individually ----

/// The members of `i`'s group, in declared (vector / front-to-back) order. A lone unit (`group: None`)
/// is its own singleton.
fn group_of(units: &[Unit], i: usize) -> Vec<usize> {
    match units[i].group {
        None => vec![i],
        Some(g) => (0..units.len())
            .filter(|&j| units[j].side == units[i].side && units[j].group == Some(g))
            .collect(),
    }
}

/// A stable per-group key (its lowest member index) — used to pay the **collective melee crossing** once
/// per cycle rather than once per striking member.
fn group_rep(units: &[Unit], i: usize) -> usize {
    group_of(units, i).into_iter().min().unwrap_or(i)
}

/// The front-most **living** member of a group — the bodyguard that soaks aimed (single-target) blows.
fn front_living(units: &[Unit], members: &[usize]) -> Option<usize> {
    members.iter().copied().find(|&j| units[j].alive())
}

/// Does `side`'s **enemy** still field a living Vanguard? The back-access gate: a Rearguard is shielded
/// while its own side's Vanguard lives (§4.6) — checked against the *target's* side.
fn vanguard_alive(units: &[Unit], side: u8) -> bool {
    units
        .iter()
        .any(|u| u.alive() && u.side == side && u.intent == Intention::Vanguard)
}

/// Is `(atk, tgt)` a legal pair anywhere in the schedule? (Reach upper-bound for the focus-fire estimate;
/// the actual strike is still gated by [`choose_target`]'s back-access and the schedule order.)
fn role_can_attack(atk: Intention, tgt: Intention) -> bool {
    SCHEDULE
        .iter()
        .any(|(_, pairs)| pairs.iter().any(|&(a, t)| a == atk && t == tgt))
}

/// **Focus-fire test (the positive-effect rule judged at the target, §4.6).** A weak strike that cannot
/// flip a card *alone* is still worth committing if the **combined** Might the team can pile onto this
/// target — plus what is already banked in its pile — crosses its Toughness. Sums the Might of living
/// allies that can reach this target's role; the per-engagement pile persists across cycles, so chip
/// ganging up on a wall converges.
fn team_can_crack(units: &[Unit], attacker: usize, target: usize) -> bool {
    let side = units[attacker].side;
    let role = units[target].intent;
    let tough = units[target].defense.health.toughness;
    let pile = units[target].defense.health_pile;
    let sum: u32 = units
        .iter()
        .filter(|u| u.alive() && u.side == side && role_can_attack(u.intent, role))
        .map(|u| u.offense.might)
        .sum();
    sum + pile >= tough
}

// ---- the default policy (the reasonable, predictable human stand-in) ----

/// Pick the best target for `attacker` among living enemies of `tgt_role`. Three gates: **back-access**
/// (a Rearguard is reachable only by an Outrider's raid, or once its side's Vanguard has fallen, §4.6);
/// **crackability** judged at the target (it flips to this strike given its banked pile, *or* the team's
/// combined Might can crack it — focus-fire); then pick the **lowest remaining health** to finish kills.
/// Returns the chosen enemy (single-target damage routes to that enemy's group **front** at apply).
fn choose_target(units: &[Unit], attacker: usize, tgt_role: Intention) -> Option<usize> {
    let side = units[attacker].side;
    let might = units[attacker].offense.might;
    let atk_intent = units[attacker].intent;
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.alive() && u.side != side && u.intent == tgt_role)
        // back-access: the back is shielded while its own Vanguard lives, unless this is an Outrider's raid.
        .filter(|(_, u)| {
            tgt_role != Intention::Rearguard
                || atk_intent == Intention::Outrider
                || !vanguard_alive(units, u.side)
        })
        // crackable now (pile + this strike), or by the team's combined Might (focus-fire).
        .filter(|(i, u)| {
            might + u.defense.health_pile >= u.defense.health.toughness
                || team_can_crack(units, attacker, *i)
        })
        .min_by_key(|(i, u)| (u.defense.health.remaining, *i))
        .map(|(i, _)| i)
}

/// The role each intention is **designed to beat** — its cycle prey (Hold→Break→Deal→Hold). The efficient
/// default spends a unit's scarce Tempo only on its prey (where its attack has unique value: only the
/// Rearguard can kill a Vanguard, etc.). The other schedule pairs stay legal — they are the "go around
/// the design" deviations, available but costed — but the default does not take them.
fn prey(i: Intention) -> Intention {
    match i {
        Intention::Vanguard => Intention::Outrider,
        Intention::Outrider => Intention::Rearguard,
        Intention::Rearguard => Intention::Vanguard,
    }
}

/// Is there a living enemy of `i`'s **prey** role that `i` can actually crack? If so, the efficient
/// default holds its scarce Tempo for that prey (where its strike has unique value — only the Rearguard
/// can kill a Vanguard, etc.) rather than spending it on a lower-value fallback rank now. Only when no
/// crackable prey remains does a unit fall back to whatever rank the schedule lets it reach — so it is
/// never useless, but "going around" is always second choice.
fn flippable_prey_alive(units: &[Unit], i: usize) -> bool {
    let side = units[i].side;
    let might = units[i].offense.might;
    let pr = prey(units[i].intent);
    units.iter().any(|u| {
        u.alive() && u.side != side && u.intent == pr && might >= u.defense.health.toughness
    })
}

/// Should `defender` spend Tempo to avoid this incoming attack? Avoid only blows that would actually flip
/// a Health card (Might ≥ effective Toughness — sub-threshold hits wipe harmlessly at the step boundary),
/// and only if it can afford the bid. Keep the cheapest, most-dangerous dodges.
fn should_avoid(d: &Unit, might: u32, finesse: u32) -> bool {
    let cost = avoid_cost(finesse, d.offense.finesse);
    might >= d.defense.health.toughness && d.tempo >= cost
}

/// Resolve one round in place: refresh Tempo, walk the schedule, declare→resolve→apply each step.
fn run_round(units: &mut [Unit]) {
    run_round_logged(units, &mut None);
}

/// As [`run_round`], but if `log` is `Some`, append a human-readable trace of each step's declarations
/// and outcomes — for tuning by inspection.
fn run_round_logged(units: &mut [Unit], log: &mut Option<Vec<String>>) {
    macro_rules! note {
        ($($a:tt)*) => { if let Some(l) = log.as_mut() { l.push(format!($($a)*)); } };
    }
    let uid = |units: &[Unit], i: usize| format!("s{}:{}", units[i].side, units[i].name);

    for u in units.iter_mut() {
        u.tempo = u.offense.cadence as i32;
    }

    for (sname, pairs) in SCHEDULE.iter() {
        note!("-- {sname} --");
        // Each step CYCLES to exhaustion (§4.6): keep committing positive-effect strikes until no one
        // will spend Tempo. The per-phase pile persists across cycles (chip combines) and wipes only at
        // the step boundary. Deaths finalize at each cycle's apply, so a unit's committed blows land but
        // a corpse cannot open a new cycle.
        loop {
            let n = units.len();
            // --- DECLARE one pass: units target individually; a melee group pays the crossing (every
            // member −1 Tempo) once per cycle, then each member with a target strikes. ---
            let mut decls: Vec<Decl> = Vec::new();
            let mut crossed = vec![false; n]; // by group_rep — the collective melee move, paid once
            let mut struck = vec![false; n]; // each unit strikes at most once per cycle
            let mut committed = false;
            for &(atk_role, tgt_role) in *pairs {
                for i in 0..n {
                    if !units[i].alive() || units[i].intent != atk_role || struck[i] {
                        continue;
                    }
                    // Efficient default: hold for your prey when one is crackable; else fall back.
                    if tgt_role != prey(units[i].intent) && flippable_prey_alive(units, i) {
                        continue;
                    }
                    let melee = units[i].is_melee();
                    let rep = group_rep(units, i);
                    // Tempo eligibility: ranged pays its own card; a melee unit pays the crossing unless
                    // its group already crossed this cycle (then its strike rides the same move, free).
                    let affordable = if melee {
                        crossed[rep] || units[i].tempo >= 1
                    } else {
                        units[i].tempo >= 1
                    };
                    if !affordable {
                        continue;
                    }
                    let Some(t) = choose_target(units, i, tgt_role) else {
                        continue;
                    };
                    if melee {
                        if !crossed[rep] {
                            // The whole group crosses as one — every living member spends one Tempo, even
                            // those with no attack (§4.5 collective reposition). Tempo-hungry by design.
                            for m in group_of(units, i) {
                                if units[m].alive() {
                                    units[m].tempo = (units[m].tempo - 1).max(0);
                                }
                            }
                            crossed[rep] = true;
                        }
                    } else {
                        units[i].tempo -= 1;
                    }
                    struck[i] = true;
                    committed = true;
                    note!(
                        "  {} -> {} (M{}{})",
                        uid(units, i),
                        uid(units, t),
                        units[i].offense.might,
                        if units[i].aoe { " AoE" } else { "" }
                    );
                    decls.push(Decl {
                        atk: i,
                        def: t,
                        might: units[i].offense.might,
                        finesse: units[i].offense.finesse,
                        melee,
                        aoe: units[i].aoe,
                    });
                }
            }
            if !committed {
                break;
            }

            // --- APPLY this cycle (the two-pool accumulator, §4.6). AoE banks full Might into every
            // target-group member's pile; aimed fire banks into the group's spillover, routed to the
            // living front. A lone Evade unit may dodge an aimed blow; a group walls (weakest-link slip,
            // so it eats) and AoE is unevadable. ---
            let mut aoe_add = vec![0u32; n]; // per-member area damage
            let mut spill_add = vec![0u32; n]; // aimed damage, keyed by the front soaker it routes to
            let mut sbacks: Vec<(usize, usize)> = Vec::new(); // (soaker, attacker) reflexive melee answers
            for d in &decls {
                if d.aoe {
                    for m in group_of(units, d.def) {
                        aoe_add[m] += d.might; // unevadable, full value to each (§4.5)
                    }
                    continue;
                }
                let members = group_of(units, d.def);
                let soaker = front_living(units, &members).unwrap_or(d.def);
                if members.len() == 1
                    && units[soaker].hits == HitMode::Evade
                    && should_avoid(&units[soaker], d.might, d.finesse)
                {
                    let cost = avoid_cost(d.finesse, units[soaker].offense.finesse);
                    units[soaker].tempo -= cost;
                    note!("  {} avoids (-{}t)", uid(units, soaker), cost);
                    continue;
                }
                spill_add[soaker] += d.might;
                if d.melee {
                    sbacks.push((soaker, d.atk));
                }
            }
            // AoE first — counted in each member's pile before spillover cascades (§4.6 worked example).
            for m in 0..n {
                if aoe_add[m] > 0 && units[m].alive() {
                    let bar = units[m].defense.health.toughness;
                    units[m].defense.take_with_toughness(aoe_add[m], bar);
                }
            }
            // Spillover cascades front-to-back through each targeted group, overflowing only on a death.
            for s in 0..n {
                if spill_add[s] > 0 {
                    let members = group_of(units, s);
                    cascade(units, &members, spill_add[s]);
                }
            }
            // Reflexive strike-backs: only a melee blow draws one, only from a melee-capable soaker, for
            // one Tempo, and only when it can crack the attacker (positive-effect). A soaker that committed
            // before dying still answers (§1.3), so this is not gated on it surviving the blow.
            for (soaker, atk) in sbacks {
                if units[soaker].is_melee()
                    && units[soaker].tempo >= 1
                    && units[atk].alive()
                    && units[soaker].offense.might + units[atk].defense.health_pile
                        >= units[atk].defense.health.toughness
                {
                    units[soaker].tempo -= 1;
                    let bar = units[atk].defense.health.toughness;
                    let mb = units[soaker].offense.might;
                    units[atk].defense.take_with_toughness(mb, bar);
                    note!(
                        "  {} strikes back M{} on {}",
                        uid(units, soaker),
                        mb,
                        uid(units, atk)
                    );
                }
            }
        }

        // --- Step boundary: wipe every per-phase pile (sub-threshold damage does not carry). ---
        for u in units.iter_mut() {
            u.defense.clear_pile();
        }
    }
}

/// Spillover cascade (§4.6): apply `amount` of aimed Might to a group front-to-back. The front living
/// member absorbs into its pile (which already holds any AoE) until it **dies**; only then does the
/// unflipped leftover overflow to the next. A surviving front soaks the rest (the bodyguard).
fn cascade(units: &mut [Unit], members: &[usize], mut amount: u32) {
    for &m in members {
        if amount == 0 {
            break;
        }
        if !units[m].alive() {
            continue;
        }
        let bar = units[m].defense.health.toughness;
        let out = units[m].defense.take_with_toughness(amount, bar);
        if out.down {
            amount = units[m].defense.health_pile; // unflipped remainder overflows
            units[m].defense.clear_pile();
        } else {
            amount = 0; // fully absorbed by the surviving front
        }
    }
}

/// Run a battle while collecting a step-by-step trace (for tuning by inspection). Returns the outcome
/// and the log.
pub fn battle_traced(
    mut side0: Vec<Unit>,
    mut side1: Vec<Unit>,
    max_rounds: u32,
) -> (Outcome, Vec<String>) {
    let mut units: Vec<Unit> = Vec::new();
    units.append(&mut side0);
    units.append(&mut side1);
    let mut log = Some(Vec::new());
    let mut outcome = Outcome::Draw;
    for r in 0..max_rounds {
        if let Some(l) = log.as_mut() {
            l.push(format!("== round {} ==", r + 1));
        }
        run_round_logged(&mut units, &mut log);
        let a = units.iter().any(|u| u.side == 0 && u.alive());
        let b = units.iter().any(|u| u.side == 1 && u.alive());
        match (a, b) {
            (true, false) => {
                outcome = Outcome::Win;
                break;
            }
            (false, true) => {
                outcome = Outcome::Loss;
                break;
            }
            (false, false) => {
                outcome = Outcome::Draw;
                break;
            }
            (true, true) => {}
        }
    }
    (outcome, log.unwrap_or_default())
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
pub fn battle(mut side0: Vec<Unit>, mut side1: Vec<Unit>, max_rounds: u32) -> Outcome {
    let mut units: Vec<Unit> = Vec::new();
    units.append(&mut side0);
    units.append(&mut side1);
    for _ in 0..max_rounds {
        run_round(&mut units);
        let a = units.iter().any(|u| u.side == 0 && u.alive());
        let b = units.iter().any(|u| u.side == 1 && u.alive());
        match (a, b) {
            (true, false) => return Outcome::Win,
            (false, true) => return Outcome::Loss,
            (false, false) => return Outcome::Draw,
            (true, true) => {}
        }
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

/// A **strategy**: a deterministic, public-information script — where every unit stands (`plan`) and how
/// it answers a blow (`hits`). Card-writable; the targeting/positive-effect rule is the engine invariant.
#[derive(Clone, Copy, Debug)]
pub struct Strategy {
    pub name: &'static str,
    pub plan: IntentPlan,
    pub hits: HitMode,
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

/// Build one side from a roster under a strategy (assign each unit its intention + hit policy).
pub fn build_side(roster: &[Recruit], strat: &Strategy, side: u8) -> Vec<Unit> {
    roster
        .iter()
        .map(|&(class, s)| {
            let mut u = make_unit(class, s, side);
            u.intent = strat.intent_for(class, s);
            u.hits = strat.hits;
            u
        })
        .collect()
}

/// `n` copies of one recruit (an extreme mono-roster).
pub fn n_of(class: &'static str, s: Stat5, n: usize) -> Vec<Recruit> {
    vec![(class, s); n]
}

// The validated triad stats (see `deckbound-level-1-balance` / Spec §4): hold / break / deal.
const HOLD: Stat5 = (1, 2, 3, 1, 1); // Fighter — Vanguard
const BREAK: Stat5 = (2, 1, 1, 2, 2); // Assassin — Outrider
const DEAL: Stat5 = (3, 1, 2, 1, 1); // Mage — Rearguard

/// The **balanced party** (one of each role, played by stats with the Evade hit policy) — the reference a
/// counter must hold against.
pub fn balanced_party() -> (Vec<Recruit>, Strategy) {
    let roster = vec![("Fighter", HOLD), ("Assassin", BREAK), ("Mage", DEAL)];
    let strat = Strategy {
        name: "balanced",
        plan: IntentPlan::ByStats,
        hits: HitMode::Evade,
    };
    (roster, strat)
}

// ---- builders for the extreme scenarios (all stat lines are the triad's 8-point budget, so an extreme
// wins by *formation*, never by being bigger) ----

/// One unit with an explicit intention.
fn lone(class: &str, s: Stat5, intent: Intention, side: u8) -> Unit {
    let mut u = make_unit(class, s, side);
    u.intent = intent;
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
        ("all-Vanguard wall", mono("Fighter", HOLD, Vanguard, 3, 1)),
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
    fn avoid_cost_is_a_threshold() {
        // cards × Fd > Fa.  Fa2/Fd2 → 2; Fa2/Fd1 → 3; Fa1/Fd2 → 1; Fa1/Fd1 → 2.
        assert_eq!(avoid_cost(2, 2), 2);
        assert_eq!(avoid_cost(2, 1), 3);
        assert_eq!(avoid_cost(1, 2), 1);
        assert_eq!(avoid_cost(1, 1), 2);
    }

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
            let mut all: Vec<Unit> = Vec::new();
            all.append(&mut atk);
            let mut g = group();
            all.append(&mut g);
            for _ in 0..4 {
                run_round(&mut all);
            }
            let backs = all
                .iter()
                .filter(|u| u.side == 1 && u.name == "Mage" && u.alive())
                .count();
            println!(
                "{} attacker → shielded back Mages alive after 4 rounds: {}/2",
                if aoe { "AoE  " } else { "aimed" },
                backs
            );
        }
    }

    /// Trace specific matchups to calibrate understanding of the resolver.
    /// `cargo test -p deckbound probe_trace_engagement -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_trace_engagement() {
        const TRIAD: Triad = [
            ("Fighter", (1, 2, 3, 1, 1)), // hold: only M3 cracks T3; its M1 can't crack the dealer
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
            ("Fighter", (1, 2, 3, 1, 1)), // hold: only M3 cracks T3; its M1 can't crack the dealer
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
