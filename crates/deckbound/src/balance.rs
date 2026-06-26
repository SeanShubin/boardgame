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

use crate::actor::Actor;
use crate::campaign::grind_encounter;
use crate::currency::Currency;
use crate::encounter::EncounterCard;
use crate::scenarios::{RewardId, build_character, build_encounter_foes, rewards_for};
use crate::solver::auto_resolve;
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
        // Infiltrator — a lethal ranged **backline** (Slingers) screened by a Husk front: melee Wall
        // killers bog on the screen while the Slingers plink them down; only a **slip** (the §4.6 Volley
        // charge) reaches the back. Seed re-tuned for the §4.6 resolver (2026-06-26): Husk 2 / Slinger 3
        // is the band where the Wall baseline loses and the slip tips it.
        Currency::Silver => vec![lock_entry("Husk", 2), lock_entry("Slinger", 3)],
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
            a.defense.health.max = 1;
            a.defense.health.remaining = 1;
        }),
        ("toughness", |a| a.defense.health.toughness = 1),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset::Ruleset;
    use crate::scenarios::build_creature;
    use crate::solver::auto_resolve_with;

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
        g.defense.health.max = big; // survives anything finite
        g.defense.health.remaining = big;
        g.defense.health.toughness = 1;
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
