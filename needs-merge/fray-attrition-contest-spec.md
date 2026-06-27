# §4 Fray — the one Tempo contest (implementation spec)

> **Ratified 2026-06-26.** The implementation-facing model for the §4 attrition Fray, reconciling the
> canon §4 spec with the design decisions taken this session. **Promotion target: `canon/2-spec`
> §4/§4.6** (this *refines* §4: see "Override" below). Built on the exploded transition-list architecture
> (`game.rs` POST_VOLLEY_SCHEDULE). Supersedes the per-action auto-trade Fray.

## The one contest (the core mechanic)
Every engagement is a **single simultaneous Tempo bid** — no iterated raise-war. Each side commits Tempo
cards worth `cards × Finesse`; **the side trying to avoid the outcome must *strictly exceed* the other
(a tie lands the hit / blocks the slip).** Bid Tempo is **spent and gone** (the attrition). One mechanic,
three uses:
- **Block / evade a blow** (defense): the struck unit out-bids the attacker to take no damage. Already
  implemented for **ranged** (`combat::try_evade`); **missing for melee** — this is the core build.
- **Slip the front** (offense/movement): a unit out-bids a contesting Vanguard to **push past to the
  back**, *even while that Vanguard stands*.
- It is **Tempo-negative to be the avoider** (you must spend *more*): a pure defender bleeds dry, runs
  empty, then eats the hit — so blows always connect eventually and Health/Might stay load-bearing.

## Override of canon §4 (slipping past a standing front)
Canon §4 line ~1121 says *"the back opens by attrition, not by slipping… no flanker slips to the back."*
**This is refined, not kept:** a unit with enough Tempo **may slip past a standing Vanguard** to reach the
back. The Vanguard's recourse is the contest — it can **cost the slipper Tempo** and, if the slip is not
out-bid, **land its blow** (and lock the slipper at the front). So slipping a live front is **possible but
risky**; it is a *specialist* play (the Infiltrator), not a free move. The back therefore opens **either**
when the front falls (killed / Routed — §4.6 free-Vanguard breach, *already coded*) **or** when a slipper
out-bids the front. Force-not-fiat throughout: enough Tempo always gets through; nothing is immune.

## Resolution order (settled)
- **Sealed simultaneous single bid** (not sequential, not iterated): both commit, reveal, compare;
  avoider/slipper must strictly exceed. Preserves one-shot brinkmanship; in PvE luck-off the foe's bid is
  fixed instinct so the solver just optimizes one number (no mixing); the hidden-bid mixing exists only in
  PvP — the blind-bid layer the solver already ignores by ratified rule.
- Implemented as its own **step/phase** in the transition list; the two sequential variants (runner-first
  / blocker-first) stay behind a flag so the solver can A/B them for balance before the human rule locks.

## Groups — sum to block, weakest-link to slip (§4.5)
A group **blocking** pools its members' Tempo into one summed bid (a strong wall). A group **slipping or
evading** needs **every member to individually beat the contest** (weakest-link). So a blob is a superb
blocker and a hopeless slipper — **the unit that reaches an exposed back is a lone, high-Tempo body.**

## Failed-slip consequences
A slipper that does **not** strictly exceed the contesting Vanguard: **takes the Vanguard's blow** and is
**locked** at the front (stays a Vanguard, no breach this round). It spent its slip Tempo for nothing —
the deterrent that makes slipping a live front a *specialist* move.

## Infiltrator kit — re-expressed as emergent Tempo-bid modifiers (not retired)
The crossing-era riders re-home onto the contest (same property, now emergent):
- **Shadowstep** → win **ties** in the contest (a tie *slips* instead of *blocks*).
- **Smoke** → your **slip bid is free** this round (commit it without spending the Tempo) — the
  "do-your-job-cheaper" tempo-discount.
- **Slip Strike / Shove** → on a successful slip, the contesting Vanguard takes **no strike-back** /
  is shoved (it paid Tempo to contest and got nothing).
- **Blitz** → the **first slip each round costs no Tempo** (already a role-conditional discount).
- **Assassinate** → unchanged: a strike on an **exposed** Rearguard hits hard enough to empty its pool.

## Already coded vs to build
- **Coded (keep):** positions (Standoff), Phase 1 front clash (Fray) skeleton, Phase 2 **free-Vanguard
  breach** (the Volley charge + per-unit lock, §4.6), ranged evade contest (`try_evade`), the
  POST_VOLLEY_SCHEDULE step list.
- **To build:** the **melee Tempo contest** (block/slip) replacing `melee_trade`'s auto-trade; the
  **defender's/slipper's bid as an explored decision** (commit-then-resolve); **sum-vs-min** for group
  block/slip; the **slip-past-a-standing-front** path (out-bid the Vanguard → breach); re-express the
  Infiltrator cards above.
