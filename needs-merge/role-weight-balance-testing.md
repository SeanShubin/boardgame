# Detecting whether a role "pulls its weight" — a marginal-contribution framework

> **Design note, staged 2026-06-26 (discussion record).** How to *automatically* measure whether a
> role earns its place when its value is **contextual** — useless solo, critical in a team. Not yet
> built; this is the design for the **role-balance layer** of the deferred par-tooling
> (`computability-and-balance.md` §10). Companion to the §6.1 necessity test and the existing
> `balance.rs::check_role_necessity` harness (which this generalizes).
>
> **Promotion target: `docs/game-theory/`** — fold into / sit beside `measurement-mechanics.md`.
> Captured in `needs-merge/` per the parallel-instance convention while a spec-sync is in flight;
> **promote (don't lose)** once that settles. Pairs with `automated-optimal-battle-play.md` (the
> strong policy this measurement depends on).

## The problem
Some roles (Controller, Support, Wall) contribute ~nothing in solo play but are decisive in the right
team. So **isolated testing is the wrong frame** — "how strong is the role alone" measures the wrong
quantity. We need the role's **contribution in context.**

## The reframe — marginal contribution (Shapley), not standalone value
Measure **how much the team's outcome changes when the role is present vs absent**, across team
contexts. This is the **Shapley value** from cooperative game theory: a player's worth = its average
marginal contribution over all coalitions (subsets of the *other* roles). Built for exactly this —
value that only appears in the right company.

**Crisp verdict:** detect role weight by its **peak marginal contribution across team contexts**, not
its average and **never** its solo value. *Pulls weight* ⟺ ∃ a coalition × encounter where adding it is
**decisive**. *Dead weight* ⟺ no such context exists.

## The measurement (exact, because the core is deterministic — §0.1 / §0.4)
`value(coalition, encounter)` is an exact function (deterministic, bounded resolve), so these are
*computed*, not sampled:

1. **Leave-one-out with replacement** *(cheap first pass)* — from a full party / a god, swap each role
   for the **best alternative** (not "remove to nothing"). Outcome degrades ⇒ it contributed. Swap-not-
   delete because a body plays one role/round (§4.4 cap), so the real question is **opportunity cost**.
2. **Coalition sweep → Shapley** *(principled aggregate)* — average marginal contribution over all
   subsets; credits synergy automatically. 5 roles ⇒ 2⁴=16 subsets/role × encounters — feasible.
3. **Report max-marginal *and* average.** This is the key to "useless solo / critical in team":
   - **Specialist** = low average, **high max** (worthless in most teams, decisive in its niche) — *fine*.
   - **Dead weight** = low average **and** low max (no niche) — cut (§6.1).
   So cut a role for a low **max**, never a low average.

## The outcome metric must be graded, not win/loss
Binary win/loss flattens the delta (both-win hides it). Use:
- **rounds-to-clear** (fewer = better — the par metric), or
- **difficulty frontier**: the hardest (scaled) encounter the coalition still beats within the round
  cap. A role's value = how far it **pushes the ceiling**. (Preferred — continuous + computable.)

## Three pitfalls (these lie to you more than the math)
- **Policy-relativity — the big one.** `value(coalition)` depends on how *well* the team is played. We
  *lived* this: the Controller read as dead weight until the greedy was fixed to cast Sunder *before*
  striking — same cards, opposite verdict. A weak policy (greedy) systematically **under-reads** roles
  whose value needs setup/timing. The honest instrument is the **par-solver's optimal play**, not
  greedy (the §5.1 deterministic-proxy-fidelity caveat). "Only valuable under optimal play" ≠ dead — the
  measurer is too weak.
- **Encounter-suite coverage.** A role is valuable only in encounters that *demand* it
  (Controller→high-Toughness walls; Artillery→crowds; Wall→threatened backline; Support→attrition;
  Silence→deferred-casting foes; Pin→breakthroughs). A suite missing a role's niche reports a **false
  negative**. The suite is itself a balance instrument + **coverage ledger**: no niche in a *diverse*
  suite ⇒ genuinely dead weight; no niche only because the suite is thin ⇒ a suite bug.
- **Synergy must be credited, not hidden.** Super-additive pairs (the Controller+Artillery board-wipe:
  Hex drops the wall → AoE shreds the lowered crowd) appear as *joint marginal > sum of individual
  marginals*. Shapley distributes the bonus across both partners (no freeloader). Surface the **synergy
  map** (which pairs are super-additive) — a balance check *and* a design signal.

## Party-size note
Measure the **role's cards' contribution** (whether carried by a solo god or a specialist body), not a
solo single-role body — god ≈ party via conserved Tempo (§4 / party-size conservation). The question is
always "do these cards improve the outcome," party-size-agnostic.

## Verdict rule
- **Pulls weight** ⟺ decisive max-marginal somewhere in a diverse suite (a real niche).
- **Dead weight** ⟺ neither average nor max marginal meaningful (no niche → §6.1 cut).
- **Specialist** ⟺ low average, high max (the team game working as intended).
- **Over-tuned / dominant** ⟂ always-include / lifts *every* coalition's frontier (the closure-check flag).

## Profile-relative measurement — roles have different *structural functions*
Roles aren't fungible: the **Anchor** *holds the front / shields the back* (Wall); the **Striker** *reaches
and kills the target* (Infiltrator/Artillery); the **Multiplier** *amplifies the party, with no kill of its
own* (Support/Controller). A **raw cross-role value scale** — or raw Shapley over *all* coalitions —
**mis-measures this** two ways: (1) it penalizes Multipliers, which contribute ~nothing outside synergy **by
design** (no damage); and (2) it rewards raw "power," including a role's ability to **overpower a weak
encounter alone** — which is **not the metric** (soloing a weak fight is *power-vs-encounter*, a consequence,
**never a goal** — parties always beat solos). Fixes:

- **Declare a structural profile per role** (a *design input*, not derived): **Anchor** (holds/shields —
  Wall), **Striker** (reaches + kills — Infiltrator/Artillery), **Multiplier** (amplifies, no kill —
  Support/Controller), each with an **intended-context domain** (where it's *meant* to be decisive). Measure
  contribution **within that domain** — never average a Multiplier over the coalitions where it's
  intentionally idle.
- **Conformance, not magnitude.** Each role is checked against **its own** profile — an Anchor decisive in a
  **protect** niche, a Striker in a **reach/kill** niche, a Multiplier in an **amplify** niche. **Never
  compare raw value across profiles** (Wall-value vs Support-value is a category error — different structural
  slots; a team fields *both*).
- **Overpowering a weak fight ≠ overvalued.** The real overvaluation signal is **dominance /
  role-substitution**: a role that **clears another role's niche-encounter** (the Wall melting a crowd =
  Artillery's job, or reaching the back = the Striker's). Concrete test: a role is over-tuned iff it clears a
  niche that isn't its own. So the suite's per-role niches double as **cross-role dominance checks** — only
  the keyed role's lever may flip its niche (**niche-exclusivity**).
- **Cop-out guard:** "it's a synergy role" is a valid profile **only** if backed by a decisive niche
  (§6.1); else it's dead weight excused as synergy.

**In the harness (needs rework):** `balance.rs` splits the Wall (`the_wall_is_the_one_role_proven_solo`)
from the others (`each_paired_role_is_necessary_in_its_lock`), and `probe_role_necessity` checks each lock is
**role-exclusive** (the dominance guard). The *split* and the *exclusivity check* are right, but the
**"solo-proven" framing is the conflation to fix** — the Wall's necessity should come from its **protect
niche**, not from soloing a grind (soloing is power-vs-encounter, not the Anchor's defining property).
Re-cast that test around the protect niche. The marginal/Shapley layer must **preserve the profile
segmentation, not flatten it** into one comparable number.

**Net per-role verdict (profile-relative, no scalar to mis-compare):** **decisive within its domain**
(∃ niche) **∧ non-dominant outside it** (∄ niche it clears that isn't its own). The intended differences
between Anchor / Striker / Multiplier then never read as imbalance.

## Relation to what exists
`check_role_necessity` + the hand-crafted `lock_encounter` per role is the **binary, single-context,
∃-a-scenario** version — and somewhat circular (the lock is built to *make* the role necessary). This
framework generalizes it: coalition × encounter sweep, graded metric, swap-comparison, **strong
policy**, Shapley + max-marginal. It is the role-balance pass of the deferred par-tooling
(`computability-and-balance.md` §10) — same solver, one more layer. Build the solver first; this rides on it.

## Tiered build (cheap → principled)
1. **LOO over a diverse suite** (catches obvious dead weight) — cheapest, needs only swap + the graded metric.
2. **Pairwise marginals** (synergy / super-additive pairs) — surfaces combos like Controller+Artillery.
3. **Full Shapley + frontier** (the principled aggregate) — once the par-solver / optimal policy exists.
