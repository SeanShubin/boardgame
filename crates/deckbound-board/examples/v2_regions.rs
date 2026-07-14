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
use deckbound_board::regions::{self, Act, Board, Oracle, Post, SubPhase, legal_acts};
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

/// The bodies of an encounter: the four kits, then its foes.
fn units(e: &Encounter) -> Vec<Combatant> {
    let mut out: Vec<Combatant> = catalog::ROSTER.iter().copied().map(kit).collect();
    for (c, q) in catalog::encounter_foes(e) {
        for _ in 0..q {
            out.push(beast(c));
        }
    }
    out
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
    println!("v2_regions - is SLIPPING ever necessary to win?\n");
    println!("The formation is declared ONCE and thereafter evolves through play, so the old");
    println!("question (fixed vs re-declared) is moot. Both arms get the BEST setup.");
    println!("  control:   the party may only CLASH or HOLD - it never slips.");
    println!("  treatment: the full menu, slipping included.\n");

    let mut needs_slip = Vec::new();
    let (mut nodes, mut worst, mut ms) = (0u64, 0usize, 0u128);

    for e in catalog::ENCOUNTERS.iter() {
        let us = units(e);
        let party = catalog::ROSTER.len();

        let t0 = Instant::now();
        let (mut clash_only, mut with_slip) = (false, false);
        let (mut n, mut w) = (0u64, 0usize);
        for (regions, posts) in formations(&us, party) {
            let b = Board::new(us.clone(), regions, posts);
            for (arm, no_slip) in [(&mut clash_only, true), (&mut with_slip, false)] {
                if **&arm {
                    continue; // already answered by an earlier formation
                }
                let mut o = Oracle::new(BUDGET);
                *arm = o.winnable(&b, 0, no_slip);
                n += o.nodes();
                w = w.max(o.states());
            }
            if clash_only && with_slip {
                break;
            }
        }
        let dt = t0.elapsed().as_millis();
        nodes += n;
        worst = worst.max(w);
        ms += dt;

        let say = |b: bool| if b { "WINNABLE" } else { "no line wins" };
        println!("{} - {}", e.location, e.title);
        println!("   clash only, never slip : {}", say(clash_only));
        println!("   the full menu          : {}", say(with_slip));
        println!("   ({n} nodes, {w} memo, {dt} ms)");
        if with_slip && !clash_only {
            println!("   >>> SLIPPING IS LOAD-BEARING: no clash-only line wins, and a slip does.");
            needs_slip.push(e.location);
        }
        println!();
    }

    println!("----------------------------------------------------------------");
    if needs_slip.is_empty() {
        println!("VERDICT: slipping is DECORATION. Every fight winnable at all is winnable by");
        println!("         clashing. The mechanic is not paying for itself - the same answer");
        println!("         v2_remarshal got, and it deserves to be taken just as seriously.");
    } else {
        println!(
            "VERDICT: slipping is LOAD-BEARING. {} encounter(s) cannot be won by clashing,",
            needs_slip.len()
        );
        println!("         and can be won by slipping:");
        for r in &needs_slip {
            println!("           - {r}");
        }
        println!("         And no better SETUP could have pre-empted it: the formation was fixed");
        println!("         at round 1 in both arms. That is what v2_remarshal could not find.");
    }
    println!("\nCOST: {nodes} nodes, {worst} states in the worst memo, {ms} ms total");

    // ---- the transcript -----------------------------------------------------------------------------------
    println!("\n----------------------------------------------------------------");
    println!("TRANSCRIPT - `*` = foe, `(n)` = health. Front line first; the back line after `|`.");
    println!("The judgment the numbers cannot give: at round 4, can you still say what is");
    println!("happening, and why?\n");

    let e = &catalog::ENCOUNTERS[5]; // Greywater Ford
    let us = units(e);
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
        for i in 0..b.units.len() {
            if b.units[i].side == Side::Party && !b.units[i].fallen {
                println!("    {:<12} {}", b.units[i].name, acts[i].label(&b));
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
