//! **Is SLIPPING ever necessary to win?**
//!
//! `v2_remarshal` proved that a **free** mid-fight re-rank never turns a loss into a win: a costless
//! repositioning offered every round can always be pre-empted by simply starting in the right place, so it can
//! never be *necessary*. It did not show that position does not matter - it showed that **costless** position
//! does not matter. So the formation was frozen, and the fiction went with it: the game still narrates a charge
//! every round by bodies that never move.
//!
//! [`deckbound_board::regions`] answers that differently. The formation is declared **once**, in secret, and
//! thereafter **evolves through play**: you never declare a position, you can only *earn* one, by slipping past
//! a line that is trying to catch you. Movement is priced by construction - you cannot ask for it.
//!
//! So the honest question is no longer "fixed formation vs re-declared". It is:
//!
//!     Is there a fight that CANNOT be won by clashing, and CAN be won by slipping?
//!
//! - **Control:** the party may only `Clash` or `Hold`. It never raids, retreats, or regroups.
//! - **Treatment:** the full menu, slipping included.
//!
//! Both get the **best setup**, enumerated exhaustively - the honest control `v2_remarshal` insisted on, since
//! starting wrong and fixing it does not count.
//!
//! A **yes** means slipping is load-bearing in the strongest possible sense: it could not have been pre-empted
//! by starting somewhere else, because the formation was fixed at setup **either way**. A **no** means the
//! mechanic is decoration, and we learned it for the price of one example program.
//!
//! Both sides' tempo allocation is held at **greedy**; only the *declarations* are searched. That is deliberate,
//! so a "no" is evidence rather than proof - but a **"yes" is proof**, and yes is the answer that costs money.
//!
//! Run: `cargo run --release -p deckbound-board --example v2_regions`

use std::time::Instant;

use deckbound_board::combat::{Combatant, Side};
use deckbound_board::regions::{
    self, Act, AreaReach, Board, Oracle, Post, SubPhase, legal_acts, set_area_reach,
};
use deckbound_content::catalog::{self, Creature, Encounter};
use deckbound_content::rank::Intention as Rank;

const BUDGET: u64 = 20_000_000;

fn kit(spec: (&'static str, [u8; 5], &'static str)) -> Combatant {
    let (name, stats, ability) = spec;
    let (melee, ranged) = catalog::ability_reach(ability);
    let (_r, aoe) = catalog::ability_shape(ability);
    Combatant::from_stats(name, Side::Party, Rank::Vanguard, stats, 0, melee, ranged).with_aoe(aoe)
}

fn beast(c: &Creature) -> Combatant {
    Combatant::from_stats(
        c.name,
        Side::Foe,
        Rank::Vanguard,
        c.stats,
        0,
        c.melee,
        c.ranged,
    )
    .with_aoe(c.aoe)
    .as_horde(c.horde)
}

/// The bodies of an encounter: the four kits, then its foes. `aoe_might` scales the Might of every **area**
/// striker - the tuning knob, so we can ask whether AoE is merely *too strong* or structurally *too reaching*.
fn units(e: &Encounter, aoe_might: f32) -> Vec<Combatant> {
    let mut out: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    for u in &mut out {
        if u.aoe {
            u.might = ((u.might as f32) * aoe_might).round().max(1.0) as u32;
        }
    }
    out
}

/// Does the party NEED to slip to win this encounter, under the given rules?
/// Returns `(winnable by clashing only, winnable with the full menu)`.
fn probe(e: &Encounter, aoe_might: f32, reach: AreaReach) -> (bool, bool) {
    set_area_reach(reach);
    probe_units(units(e, aoe_might))
}

/// [`probe`], over an explicit roster.
fn probe_units(us: Vec<Combatant>) -> (bool, bool) {
    let party = catalog::ROSTER.len();
    let (mut clash_only, mut with_slip) = (false, false);
    for (regions, posts) in formations(&us, party) {
        let b = Board::new(us.clone(), regions, posts);
        if !clash_only {
            clash_only = Oracle::new(BUDGET).winnable(&b, 0, true);
        }
        if !with_slip {
            with_slip = Oracle::new(BUDGET).winnable(&b, 0, false);
        }
        if clash_only && with_slip {
            break;
        }
    }
    (clash_only, with_slip)
}

/// **The decisive test: a tough front and a lethal back.**
///
/// The four-arm AoE experiment exonerated area strikes - the raid is never necessary at *any* sweep power or
/// reach. So the cause is elsewhere, and the invariant points straight at it:
///
/// > **No rearguard without a vanguard.** Kill the front and the back is *promoted into reach*. So a screen
/// > never DENIES anything - it only DELAYS. A raid is therefore never a necessity, only a **shortcut**.
///
/// A shortcut is worth buying only when the long way round does not fit in the time you have. So a raid can only
/// be *necessary* against a front that cannot be ground down in time, guarding a back that you cannot afford to
/// leave alive. That is a claim about **encounters**, not about mechanics - and it is testable.
///
/// This is that encounter: a wall with enormous Grit and Health (grinding it takes longer than the round cap
/// allows) sheltering a cannon that kills a hero a round. If the raid does not become necessary *here*, it is
/// not an encounter problem and the mechanic really is broken.
fn tough_front_lethal_back() -> Vec<Combatant> {
    let mut us: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    us.push(
        // The wall: Might 2, Vitality 9, Grit 9, Cadence 3, Finesse 3. Chewing through 9 cards at Grit 9 is far
        // more damage than the party can pile up inside five rounds.
        Combatant::from_stats(
            "The Bulwark",
            Side::Foe,
            Rank::Vanguard,
            [2, 9, 9, 3, 3],
            0,
            true,
            false,
        ),
    );
    us.push(
        // The cannon behind it: Might 6, so it flips a hero's Health every single round it is left alive.
        Combatant::from_stats(
            "The Executioner",
            Side::Foe,
            Rank::Vanguard,
            [6, 3, 2, 2, 2],
            0,
            false,
            true,
        ),
    );
    us
}

/// Every **partition of the party into regions**, as a restricted-growth string (`rgs[k]` is hero `k`'s region,
/// and may exceed the running max by at most one). That enumerates each *partition* exactly once and never a
/// mere relabelling of one - the same idea as `regions::canonical`: what is real is the partition, not the names
/// on it.
fn partitions(n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut rgs = vec![0u8; n];
    let mut going = true;
    while going {
        out.push(rgs.clone());
        going = false;
        for k in (1..n).rev() {
            let ceiling = rgs[..k].iter().copied().max().unwrap_or(0) + 1;
            if rgs[k] < ceiling {
                rgs[k] += 1;
                for x in rgs.iter_mut().skip(k + 1) {
                    *x = 0;
                }
                going = true;
                break;
            }
        }
    }
    out
}

/// **The foes' scripted formation.** They take one region of their own and post themselves the natural way: a
/// body that only shoots holds the **back**, everything that can swing holds the **front**.
///
/// This is not a detail - it is the thing the raid *exists to punish*, and getting it wrong hides the whole
/// question. The first cut posted every foe at the front, so the foes had **no back line at all**, so no raid
/// was ever a legal act, so the probe reported that slipping was decoration. It was measuring a board on which
/// slipping could not be tried.
fn foe_posts(us: &[Combatant]) -> Vec<Post> {
    us.iter()
        .map(|u| {
            if u.ranged && !u.melee {
                Post::Back // the cannon shelters behind the line - which is exactly what a raider comes for
            } else {
                Post::Front
            }
        })
        .collect()
}

/// Every **formation** the party could commit to at the round-1 secret: a partition of the party into regions,
/// plus a front/back post for each hero. The foes take one region of their own, posted by [`foe_posts`].
fn formations(us: &[Combatant], party: usize) -> Vec<(Vec<u8>, Vec<Post>)> {
    let foes = us.len() - party;
    let scripted = foe_posts(us);
    let mut out = Vec::new();
    for p in partitions(party) {
        let foe_region = p.iter().copied().max().unwrap_or(0) + 1;
        for mask in 0..(1u32 << party) {
            let mut posts: Vec<Post> = (0..party)
                .map(|k| {
                    if (mask >> k) & 1 == 1 {
                        Post::Back
                    } else {
                        Post::Front
                    }
                })
                .collect();
            posts.extend(scripted.iter().skip(party).copied());

            let mut regions = p.clone();
            regions.extend(std::iter::repeat_n(foe_region, foes));
            out.push((regions, posts));
        }
    }
    out
}

/// The board as one line: each region, its front line, then its back line after a `|`.
fn board_line(b: &Board, snap: Option<&regions::SubPhaseLog>) -> String {
    b.occupied()
        .iter()
        .map(|&r| {
            let tier = |post: Post| -> Vec<String> {
                // Walk ALL units against the snapshot, not `in_region` - that filters on who is fallen *now*,
                // so a body that was alive at this boundary and died later in the round would vanish from the
                // line that should show it standing. The snapshot is what happened; the board is only where it
                // ended up.
                (0..b.units.len())
                    .filter(|&i| b.regions[i] == r)
                    .filter(|&i| match snap {
                        Some(s) => s.health[i] > 0 && s.posts[i] == post,
                        None => !b.units[i].fallen && b.posts[i] == post,
                    })
                    .map(|i| {
                        let u = &b.units[i];
                        let mark = if u.side == Side::Party { "" } else { "*" };
                        let hp = snap.map_or(u.health, |s| s.health[i]);
                        format!("{}{}({})", mark, u.name, hp)
                    })
                    .collect()
            };
            let (front, back) = (tier(Post::Front), tier(Post::Back));
            let tail = if back.is_empty() {
                String::new()
            } else {
                format!(" | {}", back.join(" "))
            };
            format!("[{}: {}{}]", (b'A' + r) as char, front.join(" "), tail)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The line a player following the oracle would take: the first joint declaration it still certifies as winnable.
/// Falls back to the first legal one when the position is already lost - which is the honest thing to show,
/// because a doomed board still has to be played out.
fn certified_line(b: &Board, round: usize) -> Vec<Act> {
    let heroes: Vec<usize> = (0..b.units.len())
        .filter(|&i| b.units[i].side == Side::Party && !b.units[i].fallen)
        .collect();
    let choices: Vec<Vec<Act>> = heroes.iter().map(|&i| legal_acts(b, i)).collect();
    let total: usize = choices.iter().map(|c| c.len().max(1)).product();
    let foes = regions::foe_acts(b);
    let build = |pick: usize| -> Vec<Act> {
        let mut acts = vec![Act::Hold; b.units.len()];
        for (k, &i) in heroes.iter().enumerate() {
            if choices[k].is_empty() {
                continue;
            }
            let radix: usize = choices[..k].iter().map(|c| c.len().max(1)).product();
            acts[i] = choices[k][(pick / radix) % choices[k].len()];
        }
        for (i, a) in foes.iter().enumerate() {
            if let Some(a) = a {
                acts[i] = *a;
            }
        }
        acts
    };

    let mut o = Oracle::new(BUDGET);
    for pick in 0..total {
        let acts = build(pick);
        let mut probe = b.clone();
        regions::play_round(&mut probe, &acts);
        if o.winnable(&probe, round + 1, false) {
            return acts;
        }
    }
    build(0)
}

fn main() {
    println!(
        "v2_regions - is SLIPPING ever necessary to win?
"
    );
    println!("The formation is declared ONCE and thereafter evolves through play, so the old");
    println!("question (fixed vs re-declared) is moot. Both arms get the BEST setup.");
    println!("  control:   the party may only CLASH or HOLD - it never slips.");
    println!("  treatment: the full menu, slipping included.");
    println!(
        "A raid is NECESSARY exactly when the control loses and the treatment wins.
"
    );

    // Three rule-sets, to settle whether AoE is merely too STRONG or structurally too REACHING.
    //
    // An area strike costs one tempo card - for the Bombardier (Cadence 1) that is its whole round, so it is
    // not free. But it pays the *vanguard* nothing: no slip contest, no catch, no blood, no risk of being
    // repelled. If that is the problem, then TUNING ITS MIGHT CANNOT FIX IT - a weaker sweep kills the back
    // line more slowly, but the screen still costs it nothing. This measures that claim instead of asserting it.
    let arms: [(&str, f32, AreaReach); 4] = [
        (
            "as built:        sweep hits BOTH tiers, full Might",
            1.0,
            AreaReach::WholeRegion,
        ),
        (
            "tuned down:      sweep hits BOTH tiers, HALF Might",
            0.5,
            AreaReach::WholeRegion,
        ),
        (
            "tuned to floor:  sweep hits BOTH tiers, Might 1   ",
            0.0,
            AreaReach::WholeRegion,
        ),
        (
            "RULE CHANGE:     sweep hits ONE tier, full Might  ",
            1.0,
            AreaReach::FrontLine,
        ),
    ];

    let t0 = Instant::now();
    for (label, might, reach) in arms {
        let mut needs_slip = Vec::new();
        for e in catalog::ENCOUNTERS.iter() {
            let (clash_only, with_slip) = probe(e, might, reach);
            if with_slip && !clash_only {
                needs_slip.push(e.location);
            }
        }
        println!(
            "{label}   ->  raid necessary in {}/8 encounters",
            needs_slip.len()
        );
        for r in &needs_slip {
            println!("      - {r}");
        }
    }
    println!(
        "
  ({} ms)
",
        t0.elapsed().as_millis()
    );

    // ---- the decisive test ------------------------------------------------------------------------------
    println!("----------------------------------------------------------------");
    println!("THE DECISIVE TEST: a tough front and a lethal back.");
    println!();
    println!("If AoE is exonerated above, the cause is the invariant: NO REARGUARD WITHOUT A");
    println!("VANGUARD. Kill the front and the back is PROMOTED into reach - so a screen never");
    println!("DENIES anything, it only DELAYS, and a raid is never a necessity, only a shortcut.");
    println!("A shortcut is worth buying only when the long way round does not fit in the time");
    println!("you have. So: a wall too tough to grind inside five rounds, sheltering a cannon");
    println!("that kills a hero every round it lives. If the raid is not necessary HERE, the");
    println!(
        "problem is the mechanic - not the encounters.
"
    );

    set_area_reach(AreaReach::WholeRegion);
    let (clash_only, with_slip) = probe_units(tough_front_lethal_back());
    println!(
        "  clash only, never slip : {}",
        if clash_only {
            "WINNABLE"
        } else {
            "no line wins"
        }
    );
    println!(
        "  the full menu          : {}",
        if with_slip {
            "WINNABLE"
        } else {
            "no line wins"
        }
    );
    if with_slip && !clash_only {
        println!(
            "
  >>> THE RAID IS NECESSARY HERE. The mechanic works; the eight shipped"
        );
        println!("      encounters simply never present a front worth going around.");
        println!("      That is a CONTENT problem, and a content problem is a good problem.");
    } else if !with_slip {
        println!(
            "
  >>> Unwinnable either way - the test is too hard to be informative. Soften it."
        );
    } else {
        println!(
            "
  >>> The raid is STILL not necessary, even against a wall that cannot be ground"
        );
        println!(
            "      down guarding a cannon that cannot be ignored. Then the mechanic is broken:"
        );
        println!("      there is no shape of fight a raid is the answer to.");
    }
    println!(
        "----------------------------------------------------------------
"
    );

    // Put the model back to the shipped rule before the transcript.
    set_area_reach(AreaReach::WholeRegion);

    // ---- the transcript -----------------------------------------------------------------------------------
    println!("\n----------------------------------------------------------------");
    println!("TRANSCRIPT - `*` = foe, `(n)` = health. Front line first; the back line after `|`.");
    println!("The judgment the numbers cannot give: at round 4, can you still say what is");
    println!("happening, and why?\n");

    let e = &catalog::ENCOUNTERS[5]; // Greywater Ford
    let us = units(e, 1.0);
    let party = catalog::ROSTER.len();
    // A plausible opening: the whole party in one region, the melee bodies holding the front, the cannons behind.
    let regions: Vec<u8> = (0..us.len()).map(|i| u8::from(i >= party)).collect();
    // Both sides post themselves the natural way: cannons behind the line, everything that swings in front.
    let posts = foe_posts(&us);

    let mut b = Board::new(us, regions, posts);
    println!(
        "{} - {}\n  setup:  {}\n",
        e.location,
        e.title,
        board_line(&b, None)
    );

    for round in 0..regions::MAX_ROUNDS {
        if b.outcome().is_some() {
            break;
        }
        let acts = certified_line(&b, round);
        println!("Round {}:", round + 1);
        for (i, act) in acts.iter().enumerate() {
            if b.units[i].side == Side::Party && !b.units[i].fallen {
                println!("    {:<12} {}", b.units[i].name, act.label(&b));
            }
        }
        let logs = regions::play_round(&mut b, &acts);
        for (phase, l) in SubPhase::ALL.iter().zip(&logs) {
            let mut notes = Vec::new();
            for &i in &l.through {
                notes.push(format!("{} GETS THROUGH", b.units[i].name));
            }
            for &i in &l.aborted {
                notes.push(format!("{} turns and fights", b.units[i].name));
            }
            for &i in &l.promoted {
                notes.push(format!("{} promoted to the front", b.units[i].name));
            }
            for &i in &l.fallen {
                notes.push(format!("{} FALLS", b.units[i].name));
            }
            let note = if notes.is_empty() {
                String::new()
            } else {
                format!("   ({})", notes.join("; "))
            };
            println!(
                "  {:<8}{}{}",
                format!("{}:", phase.label()),
                board_line(&b, Some(l)),
                note
            );
        }
        println!();
    }
    println!(
        "  result: {}",
        match b.outcome() {
            Some(true) => "the party wins",
            Some(false) => "the party falls",
            None => "draw at the round cap",
        }
    );
}
