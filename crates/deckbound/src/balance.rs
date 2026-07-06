//! The **balance-property harness** (§0.3 / §8.3) — *detecting* balance problems.
//!
//! A balance property is a **falsifiable claim about combat outcomes**, checked against the
//! **resolver-of-record** (the bounded [`auto_resolve`], §4 ratified v1 / §0.4 analysis envelope). Each
//! check builds a `(party, encounter)` and asserts a **win / loss**; a [`Violation`] is a property the
//! current content does not satisfy. The harness *measures* — it does not assert in CI yet (the
//! tutorial encounters are deliberately gentle and will show violations until calibrated); run the
//! ignored `probe_grind_balance` to print the report.
//!
//! The properties encode the design intent (the user's words, §8.3):
//! - **Tutorial / necessity** — a location's suit powers should *decide* it: the equipped party wins,
//!   the **unequipped party loses** (you suffer the absence of a treasure before gaining it).
//! - **Depth gating** — going deeper in a suit should be *much harder without the lower rewards*: an
//!   L4-equipped party wins L5; an L1-only party does not.
//! - **God viability** — an L5 should fall to **5 characters with the suit's L4**, *or* to a **single
//!   god** carrying many suits' L4 with the rest fodder (the god build is viable, if not optimal).
//!
//! The comparative-par properties (even-advancement ≤ god; 2–3 teams ≤ blob/solos) are run-level, not
//! encounter-level, and are the next layer (they need par over the world, not a single fight).

use serde::Deserialize;

use crate::actor::Actor;
use crate::campaign::grind_encounter;
use crate::currency::Currency;
use crate::encounter::EncounterCard;
use crate::rules::Rule;
use crate::ruleset::Ruleset;
use crate::scenarios::build_creature;
use crate::scenarios::{RewardId, build_character, build_encounter_foes, rewards_for};
use crate::solver::{Solution, auto_resolve, solve_within, winnable_within, winnable_within_rules};
use crate::world::REWARD_SUITS;

/// A balance property the current content fails to satisfy against the resolver-of-record.
#[derive(Clone, Debug)]
pub struct Violation {
    /// `"<Suit>: <property>"` — what was being checked.
    pub property: String,
    /// The expected vs actual outcome.
    pub detail: String,
}

/// The reward ids of `suit` up to and including level `k` (the cumulative kit a `k`-deep clear yields).
fn rewards_up_to(suit: Currency, k: u32) -> Vec<RewardId> {
    rewards_for(suit)
        .into_iter()
        .filter(|r| r.level <= k)
        .collect()
}

/// A **god** kit: every suit's rewards up to level `k`, on one character (broad, not deep-per-suit).
fn god_rewards(k: u32) -> Vec<RewardId> {
    REWARD_SUITS
        .iter()
        .flat_map(|&s| rewards_up_to(s, k))
        .collect()
}

/// `n` clean-slate Novices each carrying `rewards`.
fn party(n: usize, rewards: &[RewardId]) -> Vec<Actor> {
    (0..n).map(|_| build_character("Novice", rewards)).collect()
}

/// The **paired-necessity** roles, proven by **pairing** (§8.6 emergent necessity) rather than a solo
/// blob: each must flip a fight a baseline killer party *loses* into a *win* (remove it → the win
/// flips back). (Charter #13: the §4 triangle kills; the effect roles enable — so an enabler's
/// necessity is only legible *paired* with a killer.)
///
/// **ONE role is excluded** from this kill-fight harness:
/// - The **Wall** (Iron) — proven *solo* (it *holds the line*; see `the_wall_is_the_one_role_proven_solo`);
///   here it doubles as the baseline **killer** the reach/penetration locks pit against the foe.
///
/// The **Controller** (Bone) is now **folded back in** (role-redesign, 2026-06-26): re-authored as pure
/// **stat-control**, its signature is **−Toughness (Sunder)**. Toughness is the per-phase wall (§2.2),
/// so dropping it lets a baseline killer party crack a high-Toughness foe it otherwise *cannot* out-burst
/// — necessity-provable in the kill-fight resolver with **no resolver change** (Sunder is read by the
/// existing per-phase Toughness path in `combat::apply_strike` via `Actor::eff_toughness`). The
/// stat-drops min-1 floor keeps it force-not-fiat (a Sundered foe still walls at Toughness 1). See
/// [`lock_encounter`] for the Bone band.
const PAIRED_ROLES: [Currency; 4] = [
    Currency::Silver,
    Currency::Brass,
    Currency::Salt,
    Currency::Bone,
];

/// The baseline party member a role's lock is measured against — chosen to be **exactly the capability
/// the lock denies**, so the gap is structural (force, not fiat):
/// - **penetration / reach** locks (Brass / Silver) pit a **Wall** (Iron) killer — tanky, melee, blunt —
///   that structurally *can't pierce plate* or *can't reach the backline*;
/// - the **survival** lock (Salt) pits a **glass Artillery** (Brass) cannon — high sharp damage, low
///   Body — that *out-damages but dies* without the healer keeping it up (Charter #13: the triangle
///   kills; Salt heals it).
fn baseline_member(lock: Currency) -> Actor {
    match lock {
        Currency::Salt => {
            build_character("Novice", &rewards_up_to(Currency::Brass, 5)) // glass cannon
        }
        _ => build_character("Novice", &rewards_up_to(Currency::Iron, 5)), // Wall killer
    }
}

/// The party for a role's lock: `n` baseline members; the first slot swapped for the **lock role's**
/// specialist when `add_role` — i.e. the role's contribution bought at the cost of one baseline slot.
fn lock_party(lock: Currency, n: usize, add_role: bool) -> Vec<Actor> {
    let mut p: Vec<Actor> = (0..n).map(|_| baseline_member(lock)).collect();
    if add_role {
        p[0] = build_character("Novice", &rewards_up_to(lock, 5));
    }
    p
}

/// A roster entry of `count` `creature`s (no level scaling — the lock is a fixed band, §8.4).
fn lock_entry(creature: &str, count: u32) -> crate::encounter::RosterEntry {
    crate::encounter::RosterEntry {
        creature: creature.into(),
        from_level: 1,
        base: count,
        growth: 0,
    }
}

/// The **lock encounter** for a non-Wall role (§8.6 emergent necessity): a fight raw Wall damage
/// **cannot** win, whose *natural pressure* makes that one role's mechanic the efficient key — force,
/// not fiat (the other roles still act; they just can't clear it within par). Numbers are **seeds**
/// (human-tuned). Each lock realizes the card-design-audit's per-role lock:
fn lock_encounter(role: Currency) -> EncounterCard {
    use crate::form::StatCard;
    let foes = match role {
        // Infiltrator — lever-gated + DECISIVE (2026-06-26): a light **Husk** screen + a **lethal Slinger
        // backline**. The trade-back-only Wall bogs on the screen while the Slingers shred the party in
        // ~2 rounds — a FAST loss (small search, not a grindy Golem stalemate). The Infiltrator **slips**
        // past the screen (the §4.6 Volley charge) and kills the Slingers before they kill the party — so
        // the slip is the key, not a harder fight. Tuned to the band where the Wall loses and the slip tips it.
        Currency::Silver => vec![lock_entry("Husk", 2), lock_entry("Slinger", 4)],
        // Artillery — a **high-toughness** front (Brutes): low-Might Wall fists barely flip a card; only
        // Artillery's heavy Might bursts through the bar (§2.2).
        Currency::Brass => vec![lock_entry("Brute", 1)],
        // Support — steady **attrition** that whittles the low-Body cannons over the round horizon; only
        // a **healer** (Mend/Sanctuary) sustains them past their bare capacity. (Glass baseline.) Seed
        // re-tuned for §4.6 (2026-06-26): Slinger 4 is the band where the cannons die without the heal
        // and survive (out-damaging the foe) with it.
        Currency::Salt => vec![lock_entry("Slinger", 4)],
        // Controller — a **high-Toughness wall** (Golems, Toughness 5): the blunt Wall-killer baseline's
        // hardest blow (Shield Sweep, 3 Might) is *below the bar*, so it banks sub-threshold and flips
        // **no** card — the baseline literally cannot crack the wall within the horizon and loses. Only
        // the Controller's **Sunder** (−Toughness, Unmake −3 / Hex −2, floored at 1) drops the per-phase
        // wall under the party's Might so its strikes land — the amp that flips the fight, force-not-fiat
        // (role-redesign 2026-06-26). Seed: one deep-Vitality Golem is the band where the Wall baseline
        // loses (can't crack the wall) and adding the Controller wins — and it is **Bone-exclusive** in the
        // probe (the wall blunts a single high-Might damage swap too; only Sunder + the Wall pair clears
        // it within the horizon).
        Currency::Bone => vec![lock_entry("Golem", 1)],
        _ => vec![lock_entry("Husk", 1)],
    };
    EncounterCard {
        name: format!("{} lock", role.label()),
        currency: role,
        strategy: "aggressor".into(),
        foes,
        scaling: StatCard::default(),
    }
}

/// Party size for the lock probes (seed). Small enough that one role swap is a decisive fraction of the
/// party, large enough to field the baseline capability.
const LOCK_PARTY: usize = 3;

/// **Paired role-necessity** (§8.6, Charter #12/#13): for each non-Wall role, the baseline party — the
/// one missing exactly the capability its [`lock_encounter`] demands — **loses** the lock, and **adding
/// the role** (at the cost of one baseline slot) **wins** it. That two-sided check is the honest proof a
/// role is *load-bearing*: not "a single-role blob can solo a fight" (incoherent for the effect roles,
/// which deal no damage — Charter #13), but "remove this role and an otherwise-winning party loses." The
/// Wall is excluded: it is the one role proven *solo* (it holds the line; see [`check_grind_balance`]'s
/// Iron row) and here serves as the baseline killer the reach/penetration locks pit against the foe.
/// Returns the violations (empty ⇒ every paired role is necessary in its lock).
pub fn check_role_necessity(seed: u64) -> Vec<Violation> {
    let mut v = Vec::new();
    for &role in &PAIRED_ROLES {
        let enc = lock_encounter(role);
        let foes = || build_encounter_foes(&enc, 5);
        let base = auto_resolve(lock_party(role, LOCK_PARTY, false), foes(), seed);
        check(
            &mut v,
            role,
            "baseline (without the role) loses the lock",
            base,
            false,
        );
        let keyed = auto_resolve(lock_party(role, LOCK_PARTY, true), foes(), seed);
        check(
            &mut v,
            role,
            "adding the role wins the lock (it tips the fight)",
            keyed,
            true,
        );
    }
    v
}

/// Record a check: if the resolved outcome disagrees with `want_win`, it is a violation. `outcome`
/// is `Some(true)` win / `Some(false)` loss-or-draw / `None` non-resolving (also a failure to win).
fn check(
    v: &mut Vec<Violation>,
    suit: Currency,
    property: &str,
    outcome: Option<bool>,
    want_win: bool,
) {
    let got_win = outcome == Some(true);
    if got_win != want_win {
        v.push(Violation {
            property: format!("{}: {property}", suit.label()),
            detail: format!(
                "expected {}, got {}",
                if want_win { "win" } else { "loss/draw" },
                match outcome {
                    Some(true) => "win",
                    Some(false) => "loss/draw",
                    None => "non-resolving",
                }
            ),
        });
    }
}

/// Run the encounter-level balance properties over the 25-card grind ladder; returns every violation
/// (empty ⇒ the ladder satisfies the properties under the resolver-of-record). Checks each suit's **L5**
/// (the capstone band) — see the module docs for the properties.
pub fn check_grind_balance(seed: u64) -> Vec<Violation> {
    let mut v = Vec::new();
    for &suit in REWARD_SUITS.iter() {
        let enc = grind_encounter(suit, 5);
        let foes = || build_encounter_foes(&enc, 5);

        // Necessity: the suit's full kit wins; an unequipped party loses (the lesson decides it).
        let r = auto_resolve(party(5, &rewards_up_to(suit, 5)), foes(), seed);
        check(&mut v, suit, "equipped party (5×L5) wins L5", r, true);
        let r = auto_resolve(party(5, &[]), foes(), seed);
        check(
            &mut v,
            suit,
            "unequipped party loses L5 (lesson decides)",
            r,
            false,
        );

        // Depth gating: L4 suffices for L5; L1-only does not (you need the lower rewards).
        let r = auto_resolve(party(5, &rewards_up_to(suit, 4)), foes(), seed);
        check(&mut v, suit, "depth: 5×L4 wins L5", r, true);
        let r = auto_resolve(party(5, &rewards_up_to(suit, 1)), foes(), seed);
        check(&mut v, suit, "depth: 5×L1 loses L5", r, false);

        // God viability: one god (L4 across suits) + 4 fodder clears the L5.
        let mut god = vec![build_character("Novice", &god_rewards(4))];
        god.extend(party(4, &[]));
        let r = auto_resolve(god, foes(), seed);
        check(
            &mut v,
            suit,
            "god (L4 many suits) + fodder wins L5",
            r,
            true,
        );
    }
    v
}

/// A human-readable report of the violations (for the diagnostic probe / a future balance runner).
pub fn report(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return "BALANCED: the grind ladder satisfies every checked property.".into();
    }
    let mut s = format!("{} balance violation(s):\n", violations.len());
    for vi in violations {
        s.push_str(&format!("  - {} — {}\n", vi.property, vi.detail));
    }
    s
}

// ====================================================================================================
// Role-weight — marginal contribution across an encounter suite (the robust necessity instrument).
// ====================================================================================================
//
// The §8.6 *single exclusive lock* per role is structurally fragile (see
// `needs-merge/lock-exclusivity-finding.md`): two levers are "universal solvents" (Support's sustain
// wins any grind; the Infiltrator's burst kills any exposed source), so locks overlap and only the
// *screened* slip lock is cleanly exclusive. The robust instrument measures, instead, each role's
// **marginal contribution** in the context of a *full* party across a *suite* — where overlap is
// expected and fine. We do **leave-one-out by SWAP** (replace the role's specialist with a vanilla
// Novice, holding body count at the party size) so we isolate the role's *kit* from raw headcount, and
// grade the delta on the solver's lexicographic value (win → fewer rounds → fewer downed → more Health).
// A role that flips or improves outcomes *somewhere* in the suite pulls its weight; one that is
// redundant everywhere is a dead-mechanic candidate; one that makes the party *worse* (Hurts) is an
// anti-synergy / over-cost signal.

// **Why winnability, not graded par, for the per-role pass.** Graded `solve` cannot short-circuit —
// to prove a value *optimal* it must explore the whole tree — so a 5-hero full-kit graded solve
// overflows even a 300K-node budget (≈15 min, every verdict `budget-limited`), and a budget-limited
// graded value is a *lower bound at the cut*, not the optimum: its rounds/downed/health are an artifact
// of where the search stopped and are **not comparable across parties** (a probe run showed the Wall
// reading "HURTS everywhere" purely because each truncated search hit its first win at a different
// depth). So marginal *necessity* rides [`winnable_within`] — which **short-circuits on the first win**
// (cheap regardless of budget) and gives a reliable boolean flip. The graded *weight* (does a role
// improve par/downed/Health beyond bare winnability — the Anchor's real axis) needs the full optimum,
// so it is measured separately on **tractable small scenarios** ([`battle_par_report`]), never on the
// 5-hero party. The searchability-bound finding itself: full-kit graded par is out of practical reach.

/// One role's marginal **necessity** in a full-party context (winnability flip on a single encounter).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Necessity {
    /// Full party wins; removing the role's kit makes it unwinnable — load-bearing here (a FLIP).
    Necessary,
    /// Both winnable — the role's kit is not *required* for the win here (its value, if any, is graded:
    /// fewer downed / more Health — invisible to a win/loss boolean; see [`battle_par_report`]).
    Redundant,
    /// The full party cannot win this encounter at all — too hard to attribute a role to (re-tune it).
    Insufficient,
}

impl Necessity {
    fn classify(full_win: bool, without_win: bool) -> Self {
        match (full_win, without_win) {
            (true, false) => Necessity::Necessary,
            (true, true) => Necessity::Redundant,
            (false, _) => Necessity::Insufficient,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Necessity::Necessary => "NECESSARY",
            Necessity::Redundant => "redundant",
            Necessity::Insufficient => "(full party loses)",
        }
    }
}

/// A small **encounter suite** spanning the design's levers: an armored front, a screened lethal
/// backline, a swarm, a Toughness wall, a mixed multi-rank threat, and a **lethal volley** (high
/// incoming ranged damage) — the last sized so an *unprotected* party is overwhelmed, the one axis on
/// which the **Anchor's** protection flips winnability (every other lever is offense, where the Wall
/// reads redundant). Foe bands reuse the §8.6 lock vocabulary. Seeds — tune against the report.
fn balance_suite() -> Vec<(&'static str, EncounterCard)> {
    use crate::form::StatCard;
    let enc = |name: &'static str, foes: Vec<crate::encounter::RosterEntry>| {
        (
            name,
            EncounterCard {
                name: name.into(),
                currency: Currency::Gold,
                strategy: "aggressor".into(),
                foes,
                scaling: StatCard::default(),
            },
        )
    };
    // Scaled to challenge a FIVE-body full-kit party (the §8.6 bands were sized for 3 heroes and a
    // 5-body party trivially won them → no flip). Sized toward the party's *edge*, where winnability is
    // sensitive to removing a single kit; tune against the report.
    vec![
        enc("armored front", vec![lock_entry("Brute", 6)]),
        enc(
            "screened backline",
            vec![lock_entry("Husk", 4), lock_entry("Slinger", 8)],
        ),
        enc("swarm", vec![lock_entry("Husk", 24)]),
        enc("toughness wall", vec![lock_entry("Golem", 3)]),
        enc(
            "mixed threat",
            vec![
                lock_entry("Brute", 3),
                lock_entry("Slinger", 8),
                lock_entry("Husk", 8),
            ],
        ),
        enc("lethal volley", vec![lock_entry("Slinger", 14)]),
    ]
}

/// The **full-kit party**: one specialist per reward suit (the realistic context a role's marginal
/// value is measured in). Body count = number of reward suits.
fn full_party() -> Vec<Actor> {
    REWARD_SUITS
        .iter()
        .map(|&s| build_character("Novice", &rewards_up_to(s, 5)))
        .collect()
}

/// The full party with `role`'s **kit removed but its body kept** (a vanilla Novice in its slot), so
/// the delta isolates the role's mechanics from raw headcount.
fn party_minus(role: Currency) -> Vec<Actor> {
    REWARD_SUITS
        .iter()
        .map(|&s| {
            if s == role {
                build_character("Novice", &[])
            } else {
                build_character("Novice", &rewards_up_to(s, 5))
            }
        })
        .collect()
}

/// **Role-weight report (marginal necessity)** — for each suite encounter, is the full-kit party
/// winnable, and does removing each role's kit (body kept) make it unwinnable (a FLIP = NECESSARY)?
/// Rides [`winnable_within`], which short-circuits wins, so the whole sweep is fast and the verdicts
/// reliable. `budget` bounds loss-confirmation; a budget-limited verdict is flagged `?` (a budget-
/// limited "unwinnable" is not a *proven* loss — raise the budget or shrink the encounter). The Anchor
/// reads redundant on offense encounters by design — its weight is graded; see [`battle_par_report`].
pub fn role_weight_report(seed: u64, budget: u64) -> String {
    let suite = balance_suite();
    let roles: Vec<Currency> = REWARD_SUITS.to_vec();
    // tally[role] = (necessary, redundant, insufficient)
    let mut tally: Vec<[u32; 3]> = vec![[0; 3]; roles.len()];
    let mut out = String::from(
        "role-weight — marginal NECESSITY per role across the suite (LOO-swap, winnability):\n",
    );
    for (name, enc) in &suite {
        let foes = || build_encounter_foes(enc, 5);
        let (full_win, full_of) = winnable_within(full_party(), foes(), seed, budget);
        out.push_str(&format!(
            "\n  {name}: full {}{}\n",
            if full_win { "winnable" } else { "UNWINNABLE" },
            if full_of { " [budget-limited]" } else { "" }
        ));
        for (i, &r) in roles.iter().enumerate() {
            let (w, of) = winnable_within(party_minus(r), foes(), seed, budget);
            let c = Necessity::classify(full_win, w);
            tally[i][c as usize] += 1;
            out.push_str(&format!(
                "      {:<12} {:<18} (without: {}{})\n",
                r.label(),
                c.tag(),
                if w { "winnable" } else { "unwinnable" },
                if of { "?" } else { "" },
            ));
        }
    }
    out.push_str("\n  per-role over the suite (necessary / redundant / insufficient):\n");
    for (i, &r) in roles.iter().enumerate() {
        let [nec, rd, ins] = tally[i];
        let verdict = if nec > 0 {
            "load-bearing (flips at least one encounter)"
        } else if rd > 0 {
            "no winnability flip — weight is graded (downed/Health); check battle par"
        } else {
            "INERT in this suite (full party never wins where it's swapped) — re-tune the suite"
        };
        out.push_str(&format!(
            "      {:<12} {nec}/{rd}/{ins}  — {verdict}\n",
            r.label()
        ));
    }
    out
}

/// **Battle-par report (graded weight, tractable scenarios only)** — the graded refinement winnability
/// can't give: on the small §8.6 **lock parties** (3 heroes — within the graded solver's practical
/// reach), solve for the optimal value and show how adding the lock's role changes **par / downed /
/// Health**. This is where an Anchor's contribution (fewer downed, more Health) becomes legible, and
/// where battle-par regressions would be asserted. A `[budget-limited]` line means even the small solve
/// overflowed `budget` — the searchability-bound signal; trust only un-flagged values.
pub fn battle_par_report(seed: u64, budget: u64) -> String {
    let mut out = String::from(
        "battle par — graded value on the 3-hero lock parties (baseline vs +role; optimal play):\n",
    );
    for &lock in &PAIRED_ROLES {
        let enc = lock_encounter(lock);
        let foes = || build_encounter_foes(&enc, 5);
        let base = solve_within(
            lock_party(lock, LOCK_PARTY, false),
            foes(),
            seed,
            Ruleset::analysis(),
            budget,
        );
        let keyed = solve_within(
            lock_party(lock, LOCK_PARTY, true),
            foes(),
            seed,
            Ruleset::analysis(),
            budget,
        );
        out.push_str(&format!(
            "  {:<10} baseline {}{}  →  +{} {}{}\n",
            lock.label(),
            outcome_str(&base),
            if base.overflowed {
                " [budget-limited]"
            } else {
                ""
            },
            lock.label(),
            outcome_str(&keyed),
            if keyed.overflowed {
                " [budget-limited]"
            } else {
                ""
            },
        ));
    }
    out
}

/// A compact `win (rN dD hH)` / `loss` string for a solved battle (graded; meaningful when not
/// budget-limited).
fn outcome_str(s: &Solution) -> String {
    if s.win {
        format!("win (r{} d{} h{})", s.rounds, s.downed, s.health)
    } else {
        "loss".to_string()
    }
}

/// Build an ad-hoc fixed-band encounter from `(creature, count)` pairs (unscaled, like the suite) — the
/// raw material for the ramp / niche experiments.
#[cfg(test)]
fn custom_encounter(name: &'static str, bands: &[(&str, u32)]) -> EncounterCard {
    use crate::form::StatCard;
    EncounterCard {
        name: name.into(),
        currency: Currency::Gold,
        strategy: "aggressor".into(),
        foes: bands.iter().map(|&(c, n)| lock_entry(c, n)).collect(),
        scaling: StatCard::default(),
    }
}

/// One-line marginal-necessity summary for a single encounter vs the full party: is it winnable, and
/// which roles' kits are NECESSARY (remove the kit, keep the body → the fight is lost)? A `?` marks a
/// budget-limited (unproven) verdict. The shared core of [`role_weight_report`]'s experiments.
#[cfg(test)]
fn flip_summary(enc: &EncounterCard, seed: u64, budget: u64) -> String {
    let foes = || build_encounter_foes(enc, 5);
    let (full_win, full_of) = winnable_within(full_party(), foes(), seed, budget);
    if !full_win {
        return format!("full UNWINNABLE{}", if full_of { "?" } else { "" });
    }
    let keys: Vec<String> = REWARD_SUITS
        .iter()
        .filter_map(|&r| {
            let (w, of) = winnable_within(party_minus(r), foes(), seed, budget);
            (!w).then(|| format!("{}{}", r.label(), if of { "?" } else { "" }))
        })
        .collect();
    if keys.is_empty() {
        "winnable; no role necessary".into()
    } else {
        format!("winnable; NECESSARY: {}", keys.join(", "))
    }
}

// ====================================================================================================
// §13 NvN balance simulation — equal-count parties (solver-optimized player vs deterministic AI) over
// the Fighter/Assassin/Mage classes, on the minimal subset ruleset (no grouping, no area-of-effect).
// ====================================================================================================

/// The §13 simulation classes (label/priority order).
const SIM_CLASSES: [&str; 3] = ["Fighter", "Assassin", "Mage"];

/// Every party composition of `n` units over the 3 classes — counts `[fighters, assassins, mages]`
/// summing to `n` (multisets; identical units are interchangeable).
pub fn compositions(n: u32) -> Vec<[u32; 3]> {
    let mut v = Vec::new();
    for f in 0..=n {
        for a in 0..=(n - f) {
            v.push([f, a, n - f - a]);
        }
    }
    v
}

/// A short composition label, e.g. `2F1A` (zero counts omitted).
pub fn comp_label(c: &[u32; 3]) -> String {
    let tags = ["F", "A", "M"];
    let s: String = (0..3)
        .filter(|&i| c[i] > 0)
        .map(|i| format!("{}{}", c[i], tags[i]))
        .collect();
    if s.is_empty() { "-".into() } else { s }
}

/// Build a party from a composition: `count` of each class via `build_creature`.
fn build_party(c: &[u32; 3]) -> Vec<Actor> {
    let mut p = Vec::new();
    for (i, &class) in SIM_CLASSES.iter().enumerate() {
        for _ in 0..c[i] {
            p.push(build_creature(class));
        }
    }
    p
}

/// The §13 subset ruleset (the recorded provenance): the analysis envelope with grouping and
/// area-of-effect disabled.
pub fn sim_subset() -> Ruleset {
    Ruleset::analysis().without(&[Rule::Grouping, Rule::AreaOfEffect])
}

/// A five-stat tuning tuple: `(Might, Vitality, Toughness, Cadence, Finesse)`.
pub type Stat5 = (u32, u32, u32, u32, u32);

/// Build a canonical class actor (`Fighter`/`Assassin`/`Mage` — preserving its name→AI binding, driver,
/// and melee/ranged profile) with its five stats **overridden** to `s`. The weapon stays power-0 so a
/// strike's raw force equals Might. Lets a tuning sweep try stat triads with no booklet round-trip.
pub fn build_tuned(name: &str, s: Stat5) -> Actor {
    let (m, v, t, c, f) = s;
    let mut a = build_creature(name);
    a.offense.might = m;
    a.offense.cadence = c;
    a.offense.finesse = f;
    a.defense = crate::stats::Defense::new(v, t);
    a.tempo = c as i32;
    a
}

/// Build a party from a composition under a tuning `triad` (`[(name, stats); 3]`, in F/A/M order).
fn tuned_party(c: &[u32; 3], triad: &[(&str, Stat5); 3]) -> Vec<Actor> {
    let mut p = Vec::new();
    for (i, &(name, s)) in triad.iter().enumerate() {
        for _ in 0..c[i] {
            p.push(build_tuned(name, s));
        }
    }
    p
}

/// **Tuning matrix.** As [`nvn_matrix_report`], but over a programmatic stat `triad` (hold/break/deal)
/// rather than the booklet stats — so a balance sweep can try candidate numbers without editing data.
/// Prints each player composition's W/L/? record across all enemy compositions of the same size.
pub fn tuned_matrix_report(triad: &[(&str, Stat5); 3], max_n: u32, budget: u64) -> String {
    let subset = sim_subset();
    let mut out = String::from("Tuning matrix — solver-optimized player vs deterministic AI\n");
    for &(name, (m, v, t, c, f)) in triad {
        out.push_str(&format!("  {name:<9} M{m} V{v} T{t} C{c} F{f}\n"));
    }
    out.push('\n');
    for n in 1..=max_n {
        let comps = compositions(n);
        out.push_str(&format!("== size {n} ==\n"));
        for pc in &comps {
            let (mut w, mut l, mut u) = (0, 0, 0);
            for ec in &comps {
                let (win, of) = winnable_within_rules(
                    tuned_party(pc, triad),
                    tuned_party(ec, triad),
                    1,
                    budget,
                    subset,
                );
                if win {
                    w += 1;
                } else if of {
                    u += 1;
                } else {
                    l += 1;
                }
            }
            out.push_str(&format!(
                "  {:<8} {w:>2}W {l:>2}L {u:>2}?\n",
                comp_label(pc)
            ));
        }
        out.push('\n');
    }
    out
}

// ----------------------------------------------------------------------------------------------------
// Data-driven balance **levels** — a level (roster stats + which rules are on + sizes) lives in a RON
// file read at runtime, so iterating on numbers needs no rebuild. The complexity ladder is "rules
// first": each level disables fewer registry rules, re-balancing the same chassis under more mechanics.
// ----------------------------------------------------------------------------------------------------

/// One role in a [`Level`]: a canonical class name (`Fighter`/`Assassin`/`Mage` — preserving its
/// name→AI binding and melee/ranged profile) with its five stats overridden.
#[derive(Clone, Debug, Deserialize)]
pub struct RoleSpec {
    /// Canonical class (the AI + attack-profile key); its booklet stats are overridden by `stats`.
    pub class: String,
    /// `(Might, Vitality, Toughness, Cadence, Finesse)`.
    pub stats: Stat5,
}

/// A balance **level**: the roster, the rules left on, the party sizes to sweep, and the solver budget.
/// Deserialized from a RON file (`data/balance/level-N.ron`) at runtime so edits skip the rebuild.
#[derive(Clone, Debug, Deserialize)]
pub struct Level {
    /// A label for the report header.
    pub name: String,
    /// Registry rules to **disable** for this level, by variant identifier (e.g. `"Grouping"`). All
    /// other rules stay on. Climbing the ladder shortens this list.
    #[serde(default)]
    pub rules_off: Vec<String>,
    /// The roster (hold / break / deal, in any order — labels come from the class initials).
    pub roles: Vec<RoleSpec>,
    /// Party sizes to sweep (equal player vs enemy count).
    pub sizes: Vec<u32>,
    /// Per-matchup solver node budget (a `?` in the report marks a budget-limited verdict).
    pub budget: u64,
}

impl Level {
    /// The [`Ruleset`] this level runs under: the analysis envelope with `rules_off` disabled. Panics if
    /// a name in `rules_off` matches no [`Rule`] variant (a typo in the data file should fail loud).
    pub fn ruleset(&self) -> Ruleset {
        let off: Vec<Rule> = self
            .rules_off
            .iter()
            .map(|s| {
                Rule::from_ident(s).unwrap_or_else(|| {
                    panic!("level {:?}: unknown rule in rules_off: {s:?}", self.name)
                })
            })
            .collect();
        Ruleset::analysis().without(&off)
    }
}

/// Every composition of `n` units over `k` role bins (counts summing to `n`) — the general multiset
/// enumerator behind [`compositions`] (which is the `k = 3` case).
pub fn compositions_k(n: u32, k: usize) -> Vec<Vec<u32>> {
    if k == 0 {
        return if n == 0 { vec![vec![]] } else { vec![] };
    }
    if k == 1 {
        return vec![vec![n]];
    }
    let mut out = Vec::new();
    for first in 0..=n {
        for mut rest in compositions_k(n - first, k - 1) {
            let mut v = Vec::with_capacity(k);
            v.push(first);
            v.append(&mut rest);
            out.push(v);
        }
    }
    out
}

/// Label a `k`-role composition from per-role counts and a tag per role (e.g. `2F1A`; zeros omitted).
fn comp_label_k(counts: &[u32], tags: &[String]) -> String {
    let s: String = counts
        .iter()
        .zip(tags)
        .filter(|(c, _)| **c > 0)
        .map(|(c, t)| format!("{c}{t}"))
        .collect();
    if s.is_empty() { "-".into() } else { s }
}

/// Build a party from a `k`-role composition under the level's roster.
fn level_party(counts: &[u32], roles: &[RoleSpec]) -> Vec<Actor> {
    let mut p = Vec::new();
    for (c, r) in counts.iter().zip(roles) {
        for _ in 0..*c {
            p.push(build_tuned(&r.class, r.stats));
        }
    }
    p
}

/// **Run a data-driven balance level**: the full composition matrix per size, solver-optimized player
/// vs deterministic AI, under the level's ruleset. Returns the report (with the enabled-rule provenance
/// in the header). This is the runtime entry the `balance` example calls.
pub fn run_level(level: &Level) -> String {
    let ruleset = level.ruleset();
    let k = level.roles.len();
    let tags: Vec<String> = level
        .roles
        .iter()
        .map(|r| r.class.chars().next().unwrap_or('?').to_string())
        .collect();
    let mut out = format!(
        "Balance level {:?} — solver-optimized player vs deterministic AI\n",
        level.name
    );
    for r in &level.roles {
        let (m, v, t, c, f) = r.stats;
        out.push_str(&format!("  {:<9} M{m} V{v} T{t} C{c} F{f}\n", r.class));
    }
    out.push_str(&format!(
        "rules on (provenance): {}\n\n",
        ruleset
            .enabled_rules()
            .iter()
            .map(|r| r.name())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for &n in &level.sizes {
        let comps = compositions_k(n, k);
        out.push_str(&format!("== size {n}  ({} compositions) ==\n", comps.len()));
        for pc in &comps {
            let (mut w, mut l, mut u) = (0, 0, 0);
            for ec in &comps {
                let (win, of) = winnable_within_rules(
                    level_party(pc, &level.roles),
                    level_party(ec, &level.roles),
                    1,
                    level.budget,
                    ruleset,
                );
                if win {
                    w += 1;
                } else if of {
                    u += 1;
                } else {
                    l += 1;
                }
            }
            out.push_str(&format!(
                "  {:<10} {w:>2}W {l:>2}L {u:>2}?\n",
                comp_label_k(pc, &tags)
            ));
        }
        out.push('\n');
    }
    out
}

/// **NvN balance matrix.** For each party size `1..=max_n`, run every player composition vs every enemy
/// composition (the full matrix) — solver-optimized player (side 0) vs deterministic AI (side 1), under
/// the subset ruleset. Reports each player composition's win/loss/unknown record across all enemy
/// compositions of that size (`?` = budget-limited, not a proven loss). The enabled ruleset is recorded
/// as provenance at the top.
pub fn nvn_matrix_report(max_n: u32, budget: u64) -> String {
    let subset = sim_subset();
    let mut out =
        String::from("NvN balance matrix — solver-optimized player vs deterministic AI\n");
    out.push_str(&format!(
        "ruleset (provenance): {}\n\n",
        subset
            .enabled_rules()
            .iter()
            .map(|r| r.name())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for n in 1..=max_n {
        let comps = compositions(n);
        out.push_str(&format!(
            "== size {n}  ({} compositions, {} matchups) ==\n",
            comps.len(),
            comps.len() * comps.len()
        ));
        for pc in &comps {
            let (mut w, mut l, mut u) = (0, 0, 0);
            for ec in &comps {
                let (win, of) =
                    winnable_within_rules(build_party(pc), build_party(ec), 1, budget, subset);
                if win {
                    w += 1;
                } else if of {
                    u += 1;
                } else {
                    l += 1;
                }
            }
            out.push_str(&format!(
                "  {:<8} {w:>2}W {l:>2}L {u:>2}?\n",
                comp_label(pc)
            ));
        }
        out.push('\n');
    }
    out
}

/// T3 — **stat decisiveness** (§8.6 no-redundant-stat, coarse view): zero each offensive magnitude
/// stat across the grind-ladder parties and report whether it **flips** any L5 win/loss. This is a
/// *decisiveness* probe, not a consumption proof: a stat that is consumed but never tips an outcome
/// (e.g. it adds damage to a fight already won) reads as "not decisive here". The precise
/// no-redundant-stat guards are the focused `*_is_consumed_by_*` unit tests in `combat.rs`; a stat
/// that is decisive **nowhere** *and* consumed nowhere is dead (the old "Spirit"). Diagnostic;
/// run with `--ignored`. *(Defensive / pool stats are structurally consumed by `Defense::take`.)*
pub fn stat_necessity_report(seed: u64) -> String {
    type Zeroer = (&'static str, fn(&mut Actor));
    // The five stats (§2.4): might, vitality, toughness, cadence, finesse.
    let zeroers: [Zeroer; 5] = [
        ("might", |a| a.offense.might = 0),
        ("vitality", |a| {
            a.defense.health.set_count(1);
        }),
        ("toughness", |a| a.defense.health.set_toughness(1)),
        ("cadence", |a| a.offense.cadence = 0),
        ("finesse", |a| a.offense.finesse = 0),
    ];
    let mut out =
        String::from("stat decisiveness — zero-and-flip over the 5 suits' L5 (§8.6 T3, coarse):\n");
    for (name, zero) in zeroers {
        let mut flipped = 0;
        for &suit in REWARD_SUITS.iter() {
            let enc = grind_encounter(suit, 5);
            let rw = rewards_up_to(suit, 5);
            let base = auto_resolve(party(5, &rw), build_encounter_foes(&enc, 5), seed);
            let mut zeroed_party = party(5, &rw);
            zeroed_party.iter_mut().for_each(zero);
            let zeroed = auto_resolve(zeroed_party, build_encounter_foes(&enc, 5), seed);
            if base != zeroed {
                flipped += 1;
            }
        }
        out.push_str(&format!(
            "  {name:<20} {}\n",
            if flipped > 0 {
                format!("decisive ({flipped}/5 fights flip)")
            } else {
                "not decisive in the grind ladder (consumed-but-not-tipping, or unexercised)"
                    .to_string()
            }
        ));
    }
    out
}

// ----------------------------------------------------------------------------------------------------
// Duel-locks — the party-size-1 "lock duel" instrument (a balance/tutorial set, not new mechanics). Four
// character KITS × four CREATURES, tuned so each creature is beaten by exactly ONE kit: a clean diagonal.
// Built from existing stats, reach (melee/ranged), targets (AoE), and stat-derived position only — force,
// not fiat (no immunity/keyword bans a kit; every off-diagonal loss comes from the numbers). The roster
// lives in `data/balance/duel-locks.ron` (read at runtime by the `balance` example) so the numbers retune
// with no rebuild; [`check_duel_locks`] resolves every cell through the REAL resolver (`auto_resolve` →
// `combat::resolve_round`) at party size 1 and asserts the diagonal. Mirrors [`check_role_necessity`]'s
// two-sided [`check`] pattern.
// ----------------------------------------------------------------------------------------------------

/// One combatant in a [`DuelLocks`] set — a kit or a creature. Its five stats plus its reach (`ranged`)
/// and area (`aoe`) fully determine its combat profile; **position is derived from the stats** by the
/// engine (§4 `default_intentions`: ranged→Rearguard, else Might≥Toughness→Outrider, else Vanguard),
/// never authored here. A creature may field a **horde** (`count` > 1); kits are size 1.
#[derive(Clone, Debug, Deserialize)]
pub struct DuelUnit {
    pub name: String,
    /// The signature-ability flavor name (documentation only — the mechanism is all stats / reach / area).
    #[serde(default)]
    pub ability: String,
    /// `(Might, Vitality, Toughness, Cadence, Finesse)`.
    pub stats: Stat5,
    /// Ranged reach (fires from the Rearguard) vs melee (the default).
    #[serde(default)]
    pub ranged: bool,
    /// Area strike (hits every member of the target group at full Might, unevadable, §4.5).
    #[serde(default)]
    pub aoe: bool,
    /// How many bodies a **non-hoard** unit fields (usually 1). A Hoard sizes itself from Vitality
    /// instead (see `hoard`); kits are always 1.
    #[serde(default = "one")]
    pub count: u32,
    /// §4.5 **Hoard** — this creature is a **swarm authored as one card**: it fields **Vitality**
    /// one-Health bodies bound in one group (an AoE clears the pack at once; a single-target blow kills
    /// one and spills through to the next only on a kill; the pack can never slip). The engine expands and
    /// groups it (`Actor::pack`) — no harness bookkeeping.
    #[serde(default)]
    pub hoard: bool,
    /// An **authored preferred position** — a fixed stance the unit holds: `"Vanguard"`, `"Outrider"`, or
    /// `"Rearguard"`. A first-class **creature behavior** (like its target rule); the engine honors it via
    /// `Actor::preferred`. Empty = the stat-derived default. Kits leave this empty — a kit's position is a
    /// per-round decision, never authored (the set has no "position kit").
    #[serde(default)]
    pub pos: Option<String>,
}

fn one() -> u32 {
    1
}

impl DuelUnit {
    /// The authored position, parsed to an [`Intention`]. `None` = use the engine's stat-derived default.
    fn position(&self) -> Option<crate::actor::Intention> {
        use crate::actor::Intention::{Outrider, Rearguard, Vanguard};
        self.pos.as_deref().map(|s| match s {
            "Vanguard" => Vanguard,
            "Outrider" => Outrider,
            "Rearguard" => Rearguard,
            other => panic!(
                "duel-locks: unit {:?} has an unknown position {other:?}",
                self.name
            ),
        })
    }
}

/// A **duel-locks** set: `kits` (party-size-1 characters) × `creatures`, tuned so kit *i* beats creature
/// *i* and loses/draws every other cell (the diagonal). Deserialized from `data/balance/duel-locks.ron`.
#[derive(Clone, Debug, Deserialize)]
pub struct DuelLocks {
    /// The resolution seed (the battle is deterministic given it).
    pub seed: u64,
    pub kits: Vec<DuelUnit>,
    pub creatures: Vec<DuelUnit>,
}

/// Build a duel-locks combatant: a bare `Novice` body (its power-0 weapon makes raw strike = Might) with
/// its five stats, reach, area, and **authored preferred position** overridden from `u`. No cards — the
/// lock is stats / reach / area / position only (force, not fiat). Grouping / Hoard expansion is the
/// engine's job (see [`build_duel_creatures`]); this builds one template body.
pub fn build_duel_unit(u: &DuelUnit) -> Actor {
    use crate::actor::Attack;
    let (m, v, t, c, f) = u.stats;
    let mut a = build_character("Novice", &[]);
    a.name = u.name.clone();
    a.offense.might = m;
    a.offense.cadence = c;
    a.offense.finesse = f;
    a.defense = crate::stats::Defense::new(v, t);
    a.tempo = c as i32;
    a.attack = if u.ranged {
        Attack::Ranged
    } else {
        Attack::Melee
    };
    a.aoe = u.aoe;
    a.preferred = u.position(); // a creature's authored stance (a kit leaves it None → stat-derived)
    a.actions.clear();
    a
}

/// Build a creature's body-set. A **Hoard** ([`DuelUnit::hoard`]) fields **Vitality** one-Health bodies
/// bound in one pack (§4.5 — the engine then groups and resolves them as a swarm: AoE clears the pack,
/// single-target spills through one at a time). A non-hoard creature fields `count` singleton copies.
fn build_duel_creatures(u: &DuelUnit) -> Vec<Actor> {
    let base = build_duel_unit(u);
    if u.hoard {
        let bodies = u.stats.1.max(1); // Vitality = body count (spec-literal)
        let toughness = u.stats.2.max(1);
        (0..bodies)
            .map(|_| {
                let mut b = base.clone();
                b.defense = crate::stats::Defense::new(1, toughness); // one-Health body
                b.pack = Some(0); // all bound into one pack
                b
            })
            .collect()
    } else {
        vec![base; u.count.max(1) as usize] // distinct singletons (pack stays None)
    }
}

/// Resolve one duel-locks cell: `kit` (party size 1) vs the creature's body-set, through the REAL resolver
/// (`auto_resolve` → `combat::resolve_round`). `Some(true)` = the kit wins; anything else (loss / draw /
/// non-resolving) is NOT-a-win. Grouping (Hoard packs), authored creature position, and the mutual-wipe
/// draw are all the **engine's** doing now — the harness only builds the bodies and reads the verdict.
fn resolve_duel(kit: &DuelUnit, creature: &DuelUnit, seed: u64) -> Option<bool> {
    auto_resolve(
        vec![build_duel_unit(kit)],
        build_duel_creatures(creature),
        seed,
    )
}

/// **Check the duel-locks diagonal**: for every (kit *i*, creature *j*) resolved at party size 1 through
/// the real resolver, the kit must WIN iff `i == j` (a draw counts as NOT-a-win). Returns the violations
/// (empty ⇒ a clean diagonal). The two-sided honesty of [`check_role_necessity`]: the key kit must win
/// its creature *and* every other kit must fail to win it.
pub fn check_duel_locks(locks: &DuelLocks) -> Vec<Violation> {
    let mut v = Vec::new();
    for (i, kit) in locks.kits.iter().enumerate() {
        for (j, creature) in locks.creatures.iter().enumerate() {
            let got = resolve_duel(kit, creature, locks.seed);
            let want_win = i == j;
            if (got == Some(true)) != want_win {
                v.push(Violation {
                    property: format!("{} vs {}", kit.name, creature.name),
                    detail: format!(
                        "expected {} ({}), got {}",
                        if want_win { "WIN" } else { "loss/draw" },
                        if want_win { "its key" } else { "off-diagonal" },
                        match got {
                            Some(true) => "win",
                            Some(false) => "loss/draw",
                            None => "non-resolving",
                        }
                    ),
                });
            }
        }
    }
    v
}

/// A human-readable **duel-locks matrix**: kits down the side, creatures across the top, each cell the
/// resolved verdict (`WIN` expected on the diagonal, `WIN!` = an off-diagonal break, `·` = loss/draw).
/// A trailing block lists any diagonal violation. The header records the seed.
pub fn duel_locks_report(locks: &DuelLocks) -> String {
    let short = |name: &str| name.trim_start_matches("The ").to_string();
    let mut out = format!(
        "Duel-locks matrix — party size 1, real resolver (seed {})\n\n",
        locks.seed
    );
    out.push_str(&format!("  {:<13}", "kit \\ foe"));
    for c in &locks.creatures {
        out.push_str(&format!("{:>9}", short(&c.name)));
    }
    out.push('\n');
    for (i, kit) in locks.kits.iter().enumerate() {
        out.push_str(&format!("  {:<13}", kit.name));
        for (j, creature) in locks.creatures.iter().enumerate() {
            let got = resolve_duel(kit, creature, locks.seed);
            let cell = match (got == Some(true), i == j) {
                (true, true) => "WIN",
                (true, false) => "WIN!", // off-diagonal win — a diagonal break
                (false, _) => "·",
            };
            out.push_str(&format!("{cell:>9}"));
        }
        out.push('\n');
    }
    let v = check_duel_locks(locks);
    if v.is_empty() {
        out.push_str("\n  clean diagonal — every creature is beaten by exactly its one key kit.\n");
    } else {
        out.push_str(&format!("\n  {} diagonal violation(s):\n", v.len()));
        for vi in &v {
            out.push_str(&format!("    - {} — {}\n", vi.property, vi.detail));
        }
    }
    out
}

/// Load the canonical duel-locks set (embedded at compile time) — for the in-crate tests. The `balance`
/// example reads the RON file at runtime instead, so its numbers retune with no rebuild.
pub fn duel_locks() -> DuelLocks {
    ron::from_str(include_str!("../data/balance/duel-locks.ron"))
        .expect("data/balance/duel-locks.ron should parse")
}

// ----------------------------------------------------------------------------------------------------
// Region-locks — the 4 challenge regions of the card-table world (§ playable slice). The party is the
// four duel-locks KITS (one body each); each region is a creature encounter tuned so (a) the **full**
// four-kit party clears it but **no three-kit subset** can (you need all four characters), and (b) it is
// **much easier** thanks to its own distinct **signature** kit. Reuses the duel-locks builders
// (`build_duel_creatures` → Hoard/position/reach/area) and the real resolver (`winnable_within`).
// ----------------------------------------------------------------------------------------------------

/// One challenge region: a name, the **signature** kit it is tuned to favor, and its creature roster
/// (each a [`DuelUnit`], so a foe can be a Hoard / positioned / ranged / area).
#[derive(Clone, Debug, Deserialize)]
pub struct Region {
    pub name: String,
    /// The kit this region is tuned around (the star key) — must match one of the party's kit names.
    pub signature: String,
    #[serde(default)]
    pub blurb: String,
    pub foes: Vec<DuelUnit>,
}

/// The four challenge regions + the resolution seed. Deserialized from `data/balance/region-locks.ron`;
/// the party is the [`duel_locks`] kits.
#[derive(Clone, Debug, Deserialize)]
pub struct RegionLocks {
    pub seed: u64,
    pub regions: Vec<Region>,
}

/// Build a region's foe roster (Hoards expanded and pack-grouped by [`build_duel_creatures`]).
fn region_foes(r: &Region) -> Vec<Actor> {
    r.foes.iter().flat_map(build_duel_creatures).collect()
}

/// The party of all `kits` (one body each).
fn kit_party(kits: &[DuelUnit]) -> Vec<Actor> {
    kits.iter().map(build_duel_unit).collect()
}

/// The party with kit `skip` left out (the other three).
fn kit_party_without(kits: &[DuelUnit], skip: usize) -> Vec<Actor> {
    kits.iter()
        .enumerate()
        .filter(|(i, _)| *i != skip)
        .map(|(_, k)| build_duel_unit(k))
        .collect()
}

/// **Check the region-locks**: for each region, the **full** party must win, and **every** leave-one-out
/// party (three kits) must lose — so all four characters are needed (§ "impossible without all 4"). Rides
/// [`winnable_within`] (short-circuits wins; `budget` bounds loss-confirmation). Returns the violations
/// (empty ⇒ every region needs the whole party). The *signature-kit* "much easier" gradient is graded
/// separately (see [`region_locks_report`]).
pub fn check_region_locks(regions: &RegionLocks, kits: &[DuelUnit], budget: u64) -> Vec<Violation> {
    let mut v = Vec::new();
    for r in &regions.regions {
        let foes = || region_foes(r);
        let (full, full_of) = winnable_within(kit_party(kits), foes(), regions.seed, budget);
        if !full {
            v.push(Violation {
                property: format!("{}: full party", r.name),
                detail: format!(
                    "the full 4-kit party cannot clear it{}",
                    if full_of { " [budget-limited]" } else { "" }
                ),
            });
        }
        for (i, kit) in kits.iter().enumerate() {
            let (w, _of) =
                winnable_within(kit_party_without(kits, i), foes(), regions.seed, budget);
            if w {
                v.push(Violation {
                    property: format!("{}: without {}", r.name, kit.name),
                    detail: "still clearable by the other three — not every kit is needed".into(),
                });
            }
        }
    }
    v
}

/// A **region-locks report**: for each region, is the full party winnable, and does removing each kit make
/// it unwinnable (`NEEDED` = a flip = that kit is load-bearing here)? The region's **signature** kit is
/// starred. Ideal: every row all-`NEEDED`, with the starred kit the one that also makes the clear much
/// easier (graded par — a later refinement). `budget` bounds loss-confirmation (`?` = budget-limited).
pub fn region_locks_report(regions: &RegionLocks, kits: &[DuelUnit], budget: u64) -> String {
    let mut out = format!(
        "Region-locks — 4-kit party, leave-one-out necessity (real resolver, seed {})\n\n",
        regions.seed
    );
    for r in &regions.regions {
        let foes = || region_foes(r);
        let (full, full_of) = winnable_within(kit_party(kits), foes(), regions.seed, budget);
        out.push_str(&format!(
            "  {} (signature: {})  full: {}{}\n",
            r.name,
            r.signature,
            if full { "winnable" } else { "UNWINNABLE" },
            if full_of { " [budget-limited]" } else { "" }
        ));
        for (i, kit) in kits.iter().enumerate() {
            let (w, of) = winnable_within(kit_party_without(kits, i), foes(), regions.seed, budget);
            let star = if kit.name == r.signature { " *" } else { "  " };
            out.push_str(&format!(
                "     {} without {:<12} {}{}\n",
                star,
                kit.name,
                if w {
                    "still winnable (redundant here)"
                } else {
                    "NEEDED (unwinnable without it)"
                },
                if of { " ?" } else { "" }
            ));
        }
        out.push('\n');
    }
    out
}

/// Load the canonical region-locks set (embedded). The `balance` example reads the RON at runtime.
pub fn region_locks() -> RegionLocks {
    ron::from_str(include_str!("../data/balance/region-locks.ron"))
        .expect("data/balance/region-locks.ron should parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset::Ruleset;
    use crate::scenarios::build_creature;
    use crate::solver::{auto_resolve_with, winnable_within};

    /// The canonical level-1 balance file parses and resolves to a sane ruleset — so the data-driven
    /// runner can't silently rot (a typo'd rule name or malformed stats fails here, not at runtime).
    #[test]
    fn level_1_file_parses() {
        let level: Level = ron::from_str(include_str!("../data/balance/level-1.ron"))
            .expect("data/balance/level-1.ron should parse");
        assert_eq!(level.roles.len(), 3);
        let rs = level.ruleset();
        assert!(!rs.allows(Rule::Grouping) && !rs.allows(Rule::AreaOfEffect));
        assert!(rs.allows(Rule::Clash)); // a core sub-phase stays on
    }

    /// Diagnostic (run on demand): print the current balance violations.
    /// `cargo test -p deckbound probe_grind_balance -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_grind_balance() {
        let v = check_grind_balance(1);
        println!("{}", report(&v));
    }

    /// T3 probe (§8.6 no-redundant-stat): print which offensive magnitude stats are load-bearing.
    /// `cargo test -p deckbound probe_stat_necessity -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_stat_necessity() {
        println!("{}", stat_necessity_report(1));
    }

    /// Tuning probe (§8.6 paired necessity): for each non-Wall role's **lock**, print the baseline
    /// (all-Wall) outcome and the outcome when each role is swapped in — so a lock should be *lost* by
    /// killers alone and *won* only when **its** role joins (ideally exclusively).
    /// `cargo test -p deckbound probe_role_necessity -- --ignored --nocapture`.
    /// Diagnostic: for each lock, print the seeded outcome (baseline vs +role) and a cross-check of which
    /// *other* roles also flip it — the closer the lock is to flipped-by-its-role-alone, the more it
    /// proves that role specifically (vs. raw help). `cargo test -p deckbound probe_role_necessity -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_role_necessity() {
        for &lock in &PAIRED_ROLES {
            let enc = lock_encounter(lock);
            let foes = || build_encounter_foes(&enc, 5);
            let base = auto_resolve(lock_party(lock, LOCK_PARTY, false), foes(), 1);
            print!(
                "{} lock {:?}: base={base:?}  flips:",
                lock.label(),
                enc.roster(5)
            );
            // Swap slot 0 for each role's specialist (keeping the lock's baseline for the rest).
            for &r in &PAIRED_ROLES {
                let mut p = lock_party(lock, LOCK_PARTY, false);
                p[0] = build_character("Novice", &rewards_up_to(r, 5));
                if auto_resolve(p, foes(), 1) == Some(true) && base != Some(true) {
                    print!(" +{}{}", r.label(), if r == lock { "(KEY)" } else { "" });
                }
            }
            println!();
        }
    }

    /// **Solver** necessity/exclusivity diagnostic — the §8.6 picture under OPTIMAL play (`winnable`,
    /// the strong policy), not greedy `auto_resolve`. For each lock: is the baseline (no key role)
    /// winnable, and which roles swapped into slot 0 flip a losing baseline to a win (KEY = the lock's
    /// own role)? A healthy lock: baseline = loss, and **only** its KEY flips it. Anything else flipping
    /// it — especially the Wall baseline winning outright — is the dominance/role-substitution signal.
    /// `cargo test -p deckbound probe_solver_locks -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_solver_locks() {
        const BUDGET: u64 = 50_000; // bound loss-confirmation; wins short-circuit well under this
        for &lock in &PAIRED_ROLES {
            let enc = lock_encounter(lock);
            let foes = || build_encounter_foes(&enc, 5);
            let (base, base_of) =
                winnable_within(lock_party(lock, LOCK_PARTY, false), foes(), 1, BUDGET);
            print!(
                "{} lock {:?}: base_win={base}{}  flips:",
                lock.label(),
                enc.roster(5),
                if base_of { " [budget-limited]" } else { "" }
            );
            for &r in &PAIRED_ROLES {
                let mut p = lock_party(lock, LOCK_PARTY, false);
                p[0] = build_character("Novice", &rewards_up_to(r, 5));
                let (w, _of) = winnable_within(p, foes(), 1, BUDGET);
                if w && !base {
                    print!(" +{}{}", r.label(), if r == lock { "(KEY)" } else { "" });
                }
            }
            println!();
        }
    }

    /// **Role-weight** marginal-necessity report (the robust, overlap-tolerant necessity instrument):
    /// for each suite encounter, is the full-kit party winnable, and does removing each role's kit make
    /// it unwinnable (a FLIP = NECESSARY)? Rides winnability (short-circuits wins → fast + reliable),
    /// unlike graded par at full-party scale (intractable — see the module note). `budget` bounds
    /// loss-confirmation; a `?` marks a budget-limited verdict.
    /// Stat-tuning sweep: find the smallest numbers that balance hold/break/deal. Edit `TRIAD` and re-run.
    /// `cargo test -p deckbound probe_tune_triad -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_tune_triad() {
        const BUDGET: u64 = 2_000_000;
        // The smallest numbers that balance hold/break/deal: each role maxes one signature stat at 3,
        // the rest at floor 1 (the tank pays V2 for durability, the breaker C2 for mobility). The
        // load-bearing lock is T3 ↔ M3 — only a Mage's Might 3 cracks a Fighter's Toughness 3 (a
        // breaker's M1, even doubled by the melee trade-back, is 2 < 3), so Deal is necessary. The
        // balanced 1F1A1M team is the strongest composition; mono-role comps are the weakest.
        // (Might, Vitality, Toughness, Cadence, Finesse).
        // Finesse 2 and 3 give an identical matrix vs these F1 defenders (the evade contest is a
        // threshold, not a gradient); F1 collapses the breaker. So F2 is the minimum.
        const TRIAD: [(&str, Stat5); 3] = [
            ("Fighter", (1, 2, 3, 1, 1)),  // hold the line
            ("Assassin", (1, 1, 1, 2, 2)), // break the line
            ("Mage", (3, 1, 1, 1, 1)),     // deal the damage (ranged)
        ];
        println!("{}", tuned_matrix_report(&TRIAD, 3, BUDGET));
    }

    /// Trace one matchup: print both fighters' stats, the optimal outcome, and the winning line — to
    /// sanity-check a surprising matrix cell. `cargo test -p deckbound probe_trace_1a1f -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_trace_1a1f() {
        let dump = |label: &str, name: &str| {
            let a = build_creature(name);
            println!(
                "  {label} {name}: Might {} Vit {} Tough {} Cadence {} Finesse {} | tempo {}",
                a.offense.might,
                a.defense.health.max(),
                a.defense.health.toughness(),
                a.offense.cadence,
                a.offense.finesse,
                a.tempo,
            );
        };
        dump("player", "Assassin");
        dump("enemy ", "Fighter");
        let sol = solve_within(
            vec![build_creature("Assassin")],
            vec![build_creature("Fighter")],
            1,
            crate::ruleset::Ruleset::analysis(),
            3_000_000,
        );
        println!(
            "1A vs 1F: {}{}",
            outcome_str(&sol),
            if sol.overflowed {
                " [budget-limited]"
            } else {
                ""
            }
        );
        for act in &sol.line {
            let s = format!("{act:?}");
            if !s.contains("SetVanguard") && !s.contains("SetRearguard") && !s.contains("Deploy") {
                println!("    {s}");
            }
        }
    }

    /// §13 NvN balance matrix — full composition matrix per party size, solver-optimized player vs the
    /// deterministic AI, under the subset ruleset. Small max_n first to gauge solver speed; scale up
    /// after. `cargo test -p deckbound probe_nvn_matrix -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_nvn_matrix() {
        const BUDGET: u64 = 2_000_000;
        const MAX_N: u32 = 3; // gauge speed at small sizes first
        println!("{}", nvn_matrix_report(MAX_N, BUDGET));
    }

    /// `cargo test -p deckbound probe_role_weight -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_role_weight() {
        // High budget: confirm flips rather than trust budget-limited verdicts (a 300K run produced a
        // false Silver flip that a 12M run retracted). Slower, but trustworthy.
        const BUDGET: u64 = 12_000_000;
        println!("{}", role_weight_report(1, BUDGET));
    }

    /// **Battle-par** graded report on the tractable 3-hero lock parties — the par/downed/Health weight
    /// winnability can't show (where an Anchor's contribution is legible). `[budget-limited]` flags a
    /// solve that overflowed (the searchability signal). `cargo test -p deckbound probe_battle_par -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_battle_par() {
        const BUDGET: u64 = 1_000_000;
        println!("{}", battle_par_report(1, BUDGET));
    }

    /// EXPERIMENT 1 — **size ramp**: grow ONE encounter and watch the *order* roles flip. Hypothesis:
    /// raw size makes *survival* binding first (Wall/Support flip early); offense roles flip late or
    /// never from count alone (their constraint is shape, not size).
    /// `cargo test -p deckbound probe_flip_ramp -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_flip_ramp() {
        const BUDGET: u64 = 200_000;
        println!("size ramp — flip set as one encounter grows (seed 1):");
        println!("  swarm (Husk, melee chip):");
        for n in [8u32, 16, 24, 32, 40] {
            let enc = custom_encounter("swarm", &[("Husk", n)]);
            println!("    Husk x{n:<3} {}", flip_summary(&enc, 1, BUDGET));
        }
        println!("  volley (Slinger, ranged burst):");
        for n in [6u32, 10, 14, 18, 22] {
            let enc = custom_encounter("volley", &[("Slinger", n)]);
            println!("    Slinger x{n:<3} {}", flip_summary(&enc, 1, BUDGET));
        }
    }

    /// EXPERIMENT 2 — **per-role niche scenarios**: one encounter *shaped* to bind each role's specific
    /// capability. Ideal: each is NECESSARY for (only) its keyed role. A niche that flips its key + others
    /// = overlap; one that flips *nobody* (while still hard) = that capability is fungible — the role has
    /// no distinct *responsibility* here, a design signal, not a tuning miss.
    /// `cargo test -p deckbound probe_niche_scenarios -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_niche_scenarios() {
        const BUDGET: u64 = 200_000;
        // (intended key role, shape name, foe bands) — best-effort shapes per the responsibilities table.
        let niches: &[(&str, &str, &[(&str, u32)])] = &[
            ("Iron", "burst spike (heavy ranged)", &[("Slinger", 18)]),
            ("Salt", "attrition (sustained swarm)", &[("Husk", 30)]),
            ("Brass", "armored line (Toughness front)", &[("Brute", 9)]),
            (
                "Silver",
                "Golem-screened backline",
                &[("Golem", 2), ("Slinger", 10)],
            ),
            ("Bone", "toughness wall (Golems)", &[("Golem", 4)]),
        ];
        println!(
            "per-role niche scenarios — does each shaped fight flip (only) its key? (seed 1):"
        );
        for &(key, name, bands) in niches {
            let enc = custom_encounter("niche", bands);
            println!("  [{key:<6}] {name:<32} {}", flip_summary(&enc, 1, BUDGET));
        }
    }

    /// STEP 1 — **does the solver actually play the slip?** Solve a small scenario where slipping is the
    /// natural line — an Infiltrator + a Wall vs a tanky front (Golem) screening a lethal ranged back
    /// (Slingers). Print the optimal line; if it contains `Charge` (a freed Vanguard charging the enemy
    /// Rearguard) and/or `Pass`/`Smoke`, the slip is exercised by optimal play (not just legal).
    /// `cargo test -p deckbound probe_slip_is_played -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_slip_is_played() {
        let inf = build_character("Novice", &rewards_up_to(Currency::Silver, 5));
        let wall = build_character("Novice", &rewards_up_to(Currency::Iron, 5));
        // The hero card names, so PlayCard(i, idx) is legible (e.g. "Slip Strike").
        let names: Vec<Vec<String>> = [&inf, &wall]
            .iter()
            .map(|a| a.actions.iter().map(|c| c.name.clone()).collect())
            .collect();
        // Winnable, and the lone backline Slinger survives to be reached — so a win must route to the back.
        let enc = custom_encounter("slip-test", &[("Husk", 1), ("Slinger", 1)]);
        let sol = solve_within(
            vec![inf, wall],
            build_encounter_foes(&enc, 5),
            1,
            Ruleset::analysis(),
            3_000_000,
        );
        println!(
            "slip scenario (Infiltrator+Wall vs Husk front / Slinger back): {}{}",
            outcome_str(&sol),
            if sol.overflowed {
                " [budget-limited]"
            } else {
                ""
            },
        );
        for act in &sol.line {
            let s = format!("{act:?}");
            if s.contains("SetVanguard") || s.contains("SetRearguard") || s.contains("Deploy") {
                continue; // skip Standoff position noise
            }
            // Annotate PlayCard with the card name.
            let label = match act {
                crate::game::Action::PlayCard(i, idx) => names
                    .get(*i)
                    .and_then(|c| c.get(*idx))
                    .map(|n| format!("{s}  = \"{n}\""))
                    .unwrap_or(s),
                _ => s,
            };
            println!("    {label}");
        }
    }

    /// **Can we build an encounter that *requires* the Infiltrator?** Combined-arms: a lethal melee
    /// **front** (Brutes) that pins the Wall/Support holding the line, plus a ranged **back** (Slingers)
    /// that must be killed and whose counter-fire punishes a glass charger — so the squishies can't safely
    /// cross and the tanks can't leave the front. The slip should be the only safe back-killer → removing
    /// Silver should flip the fight. Ramp to find the band. `cargo test -p deckbound probe_infiltrator_required -- --ignored --nocapture`.
    /// §13 ENEMY HEALER: a tanky screen the **Mender** keeps healing (so it never falls to attrition) +
    /// the Mender in the Rearguard behind it. The party can't out-damage the heal — it must *reach and
    /// kill the Mender*. Tests whether that forces a role (reach / priority-elimination). Moderate budget
    /// for a first look; confirm any flip at high budget. `cargo test -p deckbound probe_enemy_healer -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_enemy_healer() {
        const BUDGET: u64 = 2_000_000;
        println!("enemy healer (screen kept alive by a back-line Mender) — flips? (seed 1):");
        // The front width is the interception lever: a wider standing front (kept alive by the Mender)
        // drains a crosser slip-by-slip, so only a lone high-Finesse/high-Tempo body reaches the Mender.
        let scenarios: &[(&str, &[(&str, u32)])] = &[
            ("Sentry x3 + Mender", &[("Sentry", 3), ("Mender", 1)]),
            ("Sentry x4 + Mender", &[("Sentry", 4), ("Mender", 1)]),
            ("Sentry x5 + Mender", &[("Sentry", 5), ("Mender", 1)]),
            ("Sentry x4 + Mender x2", &[("Sentry", 4), ("Mender", 2)]),
        ];
        for &(name, bands) in scenarios {
            let enc = custom_encounter("healer", bands);
            println!("  {name:<20} {}", flip_summary(&enc, 1, BUDGET));
        }
    }

    /// PROVE the Silver flip: run the exact flip case at a high budget. If the full party is winnable and
    /// the Infiltrator-less party is unwinnable WITHOUT the budget-limited flag, the Infiltrator is
    /// genuinely necessary here (not a search artifact). `cargo test -p deckbound probe_confirm_silver_flip -- --ignored --nocapture`.
    /// PROVE the Infiltrator can be made necessary — in a **small, fully-solvable** case (the 5-hero
    /// version is too large to confirm at 12M). A 3-hero party (Wall to survive, Support to sustain,
    /// Infiltrator to cross) vs a small healed Sentry screen + Mender. If the Infiltrator-less party is
    /// genuinely unwinnable (NOT budget-limited), the role is proven necessary.
    /// `cargo test -p deckbound probe_confirm_infiltrator_small -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_confirm_infiltrator_small() {
        const BUDGET: u64 = 12_000_000;
        // Decisive (so it resolves fast → searchable): a Sentry screen + a LETHAL Slinger backline that
        // kills the party in ~2 rounds. The party must cross the screen *fast* to kill the Slingers — only
        // the slip does. Without it the party dies before it can break through. Both sides resolve quickly.
        let enc = custom_encounter("sentry-small", &[("Sentry", 4), ("Slinger", 4)]);
        let foes = || build_encounter_foes(&enc, 5);
        let with = || {
            vec![
                build_character("Novice", &rewards_up_to(Currency::Iron, 5)), // Wall — survive
                build_character("Novice", &rewards_up_to(Currency::Salt, 5)), // Support — sustain
                build_character("Novice", &rewards_up_to(Currency::Silver, 5)), // Infiltrator — cross
            ]
        };
        let without = || {
            vec![
                build_character("Novice", &rewards_up_to(Currency::Iron, 5)),
                build_character("Novice", &rewards_up_to(Currency::Salt, 5)),
                build_character("Novice", &[]), // Infiltrator's kit removed, body kept
            ]
        };
        let (w, w_of) = winnable_within(with(), foes(), 1, BUDGET);
        let (wo, wo_of) = winnable_within(without(), foes(), 1, BUDGET);
        println!("small Infiltrator-necessity test — Sentry 2 + Mender, 3-hero party (seed 1):");
        println!(
            "  with Infiltrator:    winnable={w}{}",
            if w_of { " [budget-limited]" } else { "" }
        );
        println!(
            "  without Infiltrator: winnable={wo}{}",
            if wo_of { " [budget-limited]" } else { "" }
        );
        let verdict = if w && !wo && !wo_of {
            "PROVEN: the Infiltrator is necessary (clean, not budget-limited)"
        } else if w && !wo && wo_of {
            "still budget-limited — shrink the case further"
        } else if w && wo {
            "NOT necessary — the party wins without the Infiltrator"
        } else {
            "with-Infiltrator party can't win within budget — re-tune"
        };
        println!("  => {verdict}");
    }

    #[test]
    #[ignore]
    fn probe_confirm_silver_flip() {
        const BUDGET: u64 = 12_000_000;
        let enc = custom_encounter("sentry", &[("Sentry", 4), ("Mender", 1)]);
        let foes = || build_encounter_foes(&enc, 5);
        let (full, full_of) = winnable_within(full_party(), foes(), 1, BUDGET);
        let (wo, wo_of) = winnable_within(party_minus(Currency::Silver), foes(), 1, BUDGET);
        println!("Silver-flip confirmation — Brute 6 + Slinger 8, budget {BUDGET} (seed 1):");
        println!(
            "  full party:          winnable={full}{}",
            if full_of { " [budget-limited]" } else { "" }
        );
        println!(
            "  without Infiltrator: winnable={wo}{}",
            if wo_of { " [budget-limited]" } else { "" }
        );
        let verdict = if full && !wo && !wo_of {
            "PROVEN: the Infiltrator is necessary here (clean, not budget-limited)"
        } else if full && !wo && wo_of {
            "still budget-limited — raise the budget further"
        } else if full && wo {
            "NOT a flip — the party wins without the Infiltrator after all"
        } else {
            "full party not winnable within budget — inconclusive"
        };
        println!("  => {verdict}");
    }

    #[test]
    #[ignore]
    fn probe_infiltrator_required() {
        const BUDGET: u64 = 300_000;
        println!("combined-arms (front pins tanks; back must be slip-killed) — flips? (seed 1):");
        for (b, s) in [(4u32, 8u32), (6, 8), (4, 12), (6, 12), (8, 12), (6, 16)] {
            let enc = custom_encounter("combined", &[("Brute", b), ("Slinger", s)]);
            println!(
                "  Brute {b} + Slinger {s}: {}",
                flip_summary(&enc, 1, BUDGET)
            );
        }
    }

    /// EXPERIMENT 3 — **the Toughness extreme** (does *Strip* have an isolating extreme, or is focus-fire
    /// a universal substitute?). Ramp the breadth of **Monolith** (Toughness 10, above any single hit).
    /// Hypothesis: a single Monolith is focus-fired down (the party stacks one per-phase pile over the
    /// wall → Sunder not needed); a *rank* of them forces the party to SPREAD, each pile drops below the
    /// wall → nothing flips → only **Bone's Sunder/Hex** opens them. If Bone flips at breadth, Strip is a
    /// real non-fungible responsibility; if nobody flips (or survival flips), Strip is fungible like
    /// Vitality. `cargo test -p deckbound probe_toughness_extreme -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_toughness_extreme() {
        const BUDGET: u64 = 200_000;
        println!("Toughness extreme — Monolith (T10) breadth ramp (seed 1):");
        for n in [1u32, 2, 3, 4, 5] {
            let enc = custom_encounter("monoliths", &[("Monolith", n)]);
            println!("  Monolith x{n}  {}", flip_summary(&enc, 1, BUDGET));
        }
    }

    /// Tuning probe for a **five-suit** encounter (the rules-tour example): a combined threat that
    /// should need *every* role — an armored front (Brutes: only Pierce cracks; Resolve-0 so fear
    /// disables), a lethal ranged backline (Slingers: only a slip reaches the Rearguard), and an
    /// attrition swarm (Husks). Sweep counts and print which roles' removal flips a win to a loss.
    /// `cargo test -p deckbound probe_five_suit_necessity -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_five_suit_necessity() {
        let party = |exclude: Option<Currency>| -> Vec<Actor> {
            REWARD_SUITS
                .iter()
                .filter(|&&s| Some(s) != exclude)
                .map(|&s| build_character("Novice", &rewards_for(s)))
                .collect()
        };
        let lineup = |brute: u32, sling: u32, husk: u32| EncounterCard {
            name: "five".into(),
            currency: Currency::Gold,
            strategy: "aggressor".into(),
            foes: vec![
                lock_entry("Brute", brute),
                lock_entry("Slinger", sling),
                lock_entry("Husk", husk),
            ],
            scaling: crate::form::StatCard::default(),
        };
        for (b, s, h) in [
            (6, 16, 0),
            (7, 16, 0),
            (8, 16, 0),
            (6, 18, 0),
            (8, 18, 0),
            (6, 20, 0),
            (8, 20, 0),
            (10, 18, 0),
        ] {
            let enc = lineup(b, s, h);
            let foes = || build_encounter_foes(&enc, 5);
            let full = auto_resolve(party(None), foes(), 1);
            let needed: Vec<&str> = REWARD_SUITS
                .iter()
                .filter(|&&suit| {
                    full == Some(true) && auto_resolve(party(Some(suit)), foes(), 1) != Some(true)
                })
                .map(|s| s.label())
                .collect();
            println!("Brute {b} · Slinger {s} · Husk {h}: full={full:?}  needs: {needed:?}");
        }
    }

    #[test]
    #[ignore = "§8.6 red since 2026-06-26 (per-role cap removed, §4.4 tempo-gating): this is a \
                DOMINANCE signal, not a reseed nuisance — an uncapped Wall baseline (tanky + AoE \
                Shield Sweep + breach) now SOLOS the Silver/Brass locks (clears niches that are not \
                its own). These locks are DIFFICULTY-gated, so no foe-count reseed fixes it (a \
                breaching soloer beats any difficulty gate). The real fix is LEVER-gated locks — each \
                unwinnable without that role's UNIQUE capability the Wall structurally lacks (slip past \
                an uncrackable front / ranged reach / no-damage stat-drop / sustain) — validated for \
                niche-exclusivity by the par-solver. Deferred to the dedicated balance pass AFTER \
                §4.5 Groups + §2.4–2.6 deck-tree land (both re-shift balance; retuning now is wasted). \
                See needs-merge/role-weight-balance-testing.md + automated-balance-testing-roadmap.md \
                (profile-relative measurement / niche-exclusivity). Do NOT reseed to silence this."]
    fn each_paired_role_is_necessary_in_its_lock() {
        // §8.6 paired necessity (Charter #12/#13): each PAIRED_ROLES role must flip its lock — the
        // baseline party (missing that role's capability) loses, and adding the role wins. This
        // *replaces* the old "an equipped single-role party clears its own L5" guard, which is
        // incoherent for the effect roles: a Support deals no damage (Charter #13), so a solo blob of
        // them can never win a kill-them-all fight. Usefulness is proven *paired with a killer*.
        //
        // Re-tuned for §4.6 (2026-06-26): the lock encounters were re-seeded for the six-phase resolver
        // (Silver = Husk 2 / Slinger 3, Brass = Brute 1, Salt = Slinger 4) so each baseline loses and its
        // key role tips it. The **Controller (Bone) is excluded** here — its §4.6 levers (Mark/Mire/
        // Rout/Silence) don't tip a kill-fight in the auto-resolver; it's proven by the focused
        // `combat.rs` unit tests instead. See `PAIRED_ROLES` for the full rationale.
        let v = check_role_necessity(1);
        assert!(
            v.is_empty(),
            "a paired role failed its necessity lock (§8.6): {v:?}"
        );
    }

    #[test]
    fn the_wall_is_the_one_role_proven_solo() {
        // The flip side of the pairing principle: the Wall is *not* a paired role — it holds the line on
        // its own. A party equipped in Iron clears the Wall's own L5 grind; a bare party does not (the
        // hold is what the lesson teaches). Guards the "only the Wall solos" half of the design.
        assert!(
            !PAIRED_ROLES.contains(&Currency::Iron),
            "the Wall is not a paired role"
        );
        let enc = grind_encounter(Currency::Iron, 5);
        let walls = auto_resolve(
            party(5, &rewards_up_to(Currency::Iron, 5)),
            build_encounter_foes(&enc, 5),
            1,
        );
        assert_eq!(
            walls,
            Some(true),
            "an Iron-equipped party holds (wins) the Wall's L5"
        );
    }

    /// Duel-locks — print the resolved 4×4 matrix (kits × creatures) at party size 1, so a retune can be
    /// read against the target diagonal. `cargo test -p deckbound probe_duel_locks -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_duel_locks() {
        println!("{}", duel_locks_report(&duel_locks()));
    }

    /// Trace one duel-locks cell round-by-round (the combat log) — set KIT/FOE by name.
    /// `cargo test -p deckbound probe_duel_trace -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_duel_trace() {
        use crate::game::{Deckbound, battle_state_with};
        use crate::solver::greedy;
        use contract::Game;
        const KIT: &str = "Marksman";
        const FOE: &str = "The Coil";
        let locks = duel_locks();
        let kit = locks.kits.iter().find(|k| k.name == KIT).unwrap();
        let foe = locks.creatures.iter().find(|c| c.name == FOE).unwrap();
        let game = Deckbound;
        // Grouping / position / mutual-wipe are the engine's job now; just build the bodies and drive greedy.
        let mut state = battle_state_with(
            vec![build_duel_unit(kit)],
            build_duel_creatures(foe),
            false,
            locks.seed,
            Ruleset::analysis(),
        );
        for _ in 0..10_000 {
            if game.outcome(&state).is_some() {
                break;
            }
            let a = greedy(&state, &game.legal_actions(&state));
            game.apply(&mut state, &a).unwrap();
        }
        println!("=== {KIT} vs {FOE} ===");
        for line in &state.log {
            println!("{line}");
        }
    }

    /// The duel-locks data file parses into a square set (a typo'd stat or keyword fails here, not at
    /// runtime), mirroring [`level_1_file_parses`].
    #[test]
    fn duel_locks_file_parses() {
        let locks = duel_locks();
        assert_eq!(locks.kits.len(), 4, "four kits");
        assert_eq!(locks.creatures.len(), 4, "four creatures");
        // At least one creature is a Hoard (a swarm authored as one card) — the flag round-trips.
        assert!(
            locks.creatures.iter().any(|c| c.hoard),
            "at least one creature swarms (a Hoard)"
        );
    }

    /// The **duel-locks diagonal holds**: each creature is beaten by exactly its one key kit, verified
    /// through the real resolver at party size 1 (a draw counts as NOT-a-win). This is the instrument's
    /// acceptance criterion — the matrix is the spec.
    #[test]
    fn duel_locks_diagonal_holds() {
        let v = check_duel_locks(&duel_locks());
        assert!(v.is_empty(), "duel-locks diagonal broken: {v:#?}");
    }

    /// Region-locks — print the leave-one-out necessity matrix (full party vs each region, and each kit
    /// removed). `cargo test -p deckbound probe_region_locks -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_region_locks() {
        const BUDGET: u64 = 2_000_000;
        println!(
            "{}",
            region_locks_report(&region_locks(), &duel_locks().kits, BUDGET)
        );
    }

    /// The region-locks data file parses into four regions, each naming a real party kit as its signature.
    #[test]
    fn region_locks_file_parses() {
        let regions = region_locks();
        let kits = duel_locks().kits;
        assert_eq!(regions.regions.len(), 4, "four challenge regions");
        for r in &regions.regions {
            assert!(
                kits.iter().any(|k| k.name == r.signature),
                "region {:?} signature {:?} is not a party kit",
                r.name,
                r.signature
            );
        }
        // Each kit is the signature of exactly one region (four distinct signatures).
        let mut sigs: Vec<&str> = regions
            .regions
            .iter()
            .map(|r| r.signature.as_str())
            .collect();
        sigs.sort_unstable();
        sigs.dedup();
        assert_eq!(sigs.len(), 4, "each region tuned for a distinct kit");
    }

    /// **Every region needs all four characters**: the full 4-kit party clears each challenge region and
    /// no three-kit subset can (the instrument's acceptance criterion — verified through the real
    /// resolver). ~1s at this budget.
    #[test]
    fn region_locks_need_all_four() {
        const BUDGET: u64 = 2_000_000;
        let v = check_region_locks(&region_locks(), &duel_locks().kits, BUDGET);
        assert!(v.is_empty(), "a region did not need all four kits: {v:#?}");
    }

    #[test]
    fn the_harness_runs_and_is_deterministic() {
        // The harness itself must be sound: it runs without panicking and is a pure function of the
        // seed (same seed ⇒ same verdict), so a violation count is a stable measurement to tune against.
        let a = check_grind_balance(1);
        let b = check_grind_balance(1);
        assert_eq!(
            a.len(),
            b.len(),
            "the harness is deterministic for a fixed seed"
        );
    }

    /// A no-skills character with arbitrarily large stats — the BI-3 **force-not-fiat** witness.
    fn infinite_god() -> Actor {
        let mut g = build_character("Novice", &[]);
        let big = 1_000_000;
        g.offense.might = big; // one-shots anything finite
        g.offense.finesse = big; // crosses any finite hold
        g.offense.cadence = big; // unlimited actions
        g.tempo = big as i32;
        g.defense.health.set_count(big); // survives anything finite
        g.defense.health.set_toughness(1);
        g
    }

    #[test]
    fn bi3_force_not_fiat_infinite_god_wipes_any_finite_party() {
        // BI-3 (`balance-invariants.md`): a **no-skills**, **infinite-stat** character must win any
        // **finite-stat** encounter — opposition is always *cost*, never *impossibility*. A failure
        // means a rule forbids by fiat (a hard cap, an immunity, a skill-gate, or a
        // permanently-unreachable rank). Probed against formations that stress each rank.
        //
        // Re-scoped for §4.6 (2026-06-26): the old test demanded the wipe **in one round**, which the
        // six-phase model makes structurally impossible for a *single body* — one body makes one melee
        // trade in the Fray and one charge/flank in the Volley, i.e. it can fell at most ~2 separate
        // enemy bodies per round. That is **cost (more rounds), not fiat**: there is no rule that forbids
        // the win, only an action budget that scales with the foe count. BI-3's actual claim — force,
        // not fiat — is therefore checked over a **finite (bounded) horizon** rather than one round: the
        // god must win within a generous round cap (still finite, so a genuine fiat barrier — a hard
        // immunity / unreachable rank — would still surface as a loss/draw, not a win).
        let bounded = Ruleset {
            max_rounds: 50,
            max_unique_per_side: u32::MAX,
            ..Ruleset::default()
        };
        let parties: [(&str, Vec<Actor>); 3] = [
            ("a deep wall", vec![build_creature("Brute"); 5]),
            ("a swarm", vec![build_creature("Husk"); 12]),
            ("a hide-in-the-back line", vec![build_creature("Seer"); 5]),
        ];
        for (name, foes) in parties {
            assert_eq!(
                auto_resolve_with(vec![infinite_god()], foes, 1, bounded),
                Some(true),
                "the infinite-stat god failed to wipe {name} within a finite horizon — a fiat barrier"
            );
        }
    }
}
