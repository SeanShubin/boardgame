//! The **engagement-schedule** combat model (the brainstormed redesign in
//! `log-driven/brainstorming/phases.md`), built **isolated** from the live 12-phase engine so we can
//! test whether the hold/break/deal triangle survives it before adopting it as the official spec.
//!
//! A round: declare intentions (V/O/R) → resolve a fixed engagement **schedule** a–e over one shared
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

/// A lean combatant for the sim: the stat block (reusing the real [`Offense`]/[`Defense`] so the damage
/// math is identical to the live game), an attack range, a side, an intention, and the round Tempo pool.
#[derive(Clone, Debug)]
pub struct Unit {
    pub name: String,
    pub side: u8,
    pub intent: Intention,
    pub offense: Offense,
    pub defense: Defense,
    pub attack: Attack,
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
        tempo: c as i32,
    }
}

/// One engagement pair: an attacker intention strikes a target intention.
type Pair = (Intention, Intention);

/// The fixed schedule, in resolution order — the *ordering* is the whole interception/Reckoning system.
/// Each step may hold several role-pairs; all of a step's attacks declare before any resolve, and the
/// per-phase pile wipes at the step boundary. (`Rearguard → Rearguard` is the one illegal pair.)
const SCHEDULE: &[(&str, &[Pair])] = {
    use Intention::{Outrider as O, Rearguard as R, Vanguard as V};
    &[
        ("Intercept", &[(V, O)]),
        ("Volley", &[(R, O)]),
        ("Raid", &[(O, R)]),
        ("Clash", &[(R, V), (V, V)]),
        ("Breach", &[(V, R), (O, V), (O, O)]),
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
}

// ---- the default policy (the reasonable, predictable human stand-in) ----

/// Pick the best target for `attacker` among living enemies of `tgt_role`: only ones it can actually
/// **flip a card on** (Might ≥ their effective Toughness — never swing at what you can't crack and eat a
/// pointless strike-back), focusing the highest-threat role (dealer first), and within a role the
/// **lowest remaining health** (finish kills). Returns an index, or `None` if nothing is crackable.
fn choose_target(units: &[Unit], attacker: usize, tgt_role: Intention) -> Option<usize> {
    let side = units[attacker].side;
    let might = units[attacker].offense.might;
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            u.alive()
                && u.side != side
                && u.intent == tgt_role
                && might >= u.defense.health.toughness
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
        // --- Declare: each unit (both sides) commits one attack on its **prey** (the efficient default:
        // it spends its scarce Tempo only where its strike has unique value, not on go-around pairs). ---
        let mut decls: Vec<Decl> = Vec::new();
        let order: Vec<usize> = (0..units.len()).collect();
        for &(atk_role, tgt_role) in *pairs {
            for &i in &order {
                if !units[i].alive() || units[i].intent != atk_role {
                    continue;
                }
                if units[i].tempo < 1 {
                    continue;
                }
                // Screener → primary force → cleanup: focus your prey when one is crackable (its strike
                // has unique value there), and only fall back to another rank the schedule reaches when no
                // crackable prey remains. So a Vanguard screens Outriders, then fights Vanguards, then
                // cleans up Rearguards — never useless, but "going around" is always the second choice.
                if tgt_role != prey(units[i].intent) && flippable_prey_alive(units, i) {
                    continue;
                }
                if let Some(t) = choose_target(units, i, tgt_role) {
                    units[i].tempo -= 1;
                    note!(
                        "  {} -> {} (M{})",
                        uid(units, i),
                        uid(units, t),
                        units[i].offense.might
                    );
                    decls.push(Decl {
                        atk: i,
                        def: t,
                        might: units[i].offense.might,
                        finesse: units[i].offense.finesse,
                        melee: units[i].is_melee(),
                    });
                }
            }
        }

        // --- Resolve + Apply: each defender handles its incoming, most-dangerous first; avoid if worth
        // it and affordable, else eat (Might into the pile) and strike back if a melee attacker. ---
        let mut by_def: Vec<Vec<usize>> = vec![Vec::new(); units.len()];
        for (di, d) in decls.iter().enumerate() {
            by_def[d.def].push(di);
        }
        for def in 0..units.len() {
            if by_def[def].is_empty() {
                continue;
            }
            let mut incoming = by_def[def].clone();
            incoming.sort_by_key(|&di| std::cmp::Reverse(decls[di].might));
            for di in incoming {
                let (might, finesse, melee, atk) = (
                    decls[di].might,
                    decls[di].finesse,
                    decls[di].melee,
                    decls[di].atk,
                );
                if !units[def].alive() {
                    // already down this step — committed blows still landed via the pile; skip extras
                }
                if should_avoid(&units[def], might, finesse) {
                    let cost = avoid_cost(finesse, units[def].offense.finesse);
                    units[def].tempo -= cost;
                    note!("  {} avoids (-{}t)", uid(units, def), cost);
                    continue;
                }
                // Eat it.
                let bar = units[def].defense.health.toughness;
                let out = units[def].defense.take_with_toughness(might, bar);
                if out.cards_flipped > 0 {
                    note!(
                        "  {} eats M{} -> -{} card{} ({} left){}",
                        uid(units, def),
                        might,
                        out.cards_flipped,
                        if out.cards_flipped == 1 { "" } else { "s" },
                        units[def].defense.health.remaining,
                        if out.down { " DOWN" } else { "" }
                    );
                }
                // Strike back at a melee attacker if a card is free — but only when it can actually flip
                // a card (never burn Tempo on a strike that bounces off the attacker's Toughness), and a
                // corpse can't react (only pre-declared blows land when mortally wounded).
                if melee
                    && units[def].alive()
                    && units[def].tempo >= 1
                    && units[atk].alive()
                    && units[def].offense.might >= units[atk].defense.health.toughness
                {
                    units[def].tempo -= 1;
                    let bar = units[atk].defense.health.toughness;
                    let mb = units[def].offense.might;
                    let so = units[atk].defense.take_with_toughness(mb, bar);
                    if so.cards_flipped > 0 {
                        note!(
                            "  {} strikes back M{} -> -{} ({} left){}",
                            uid(units, def),
                            mb,
                            so.cards_flipped,
                            units[atk].defense.health.remaining,
                            if so.down { " DOWN" } else { "" }
                        );
                    }
                }
            }
        }

        // --- Step boundary: wipe every per-phase pile (sub-threshold damage does not carry). ---
        for u in units.iter_mut() {
            u.defense.clear_pile();
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
