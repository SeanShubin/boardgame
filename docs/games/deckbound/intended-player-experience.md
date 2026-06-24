# Deckbound — Intended Player Experience

> **Non-authoritative synthesis.** This reads the *felt experience* off the binding sources — the
> [Charter](canon/1-charter.md) north stars, the [Spec](canon/2-spec/README.md), and the balance
> instruments ([balance-invariants.md](balance-invariants.md),
> [reference-scenario.md](reference-scenario.md)). It is a lens, not a law: where it disagrees with
> canon, **canon wins** ([0-source-of-truth](canon/0-source-of-truth.md)). Read each promise alongside
> the invariant or principle that protects it — the invariant is *why* the feeling is guaranteed.

> **One sentence.** The game should feel like a fair, legible climb where you repeatedly run into a
> wall, recognise exactly which power you lack, go earn it, and feel it visibly turn the next fight —
> until the summit asks you to prove you've become complete.

The enforcing checks live in
[`reference.rs::check_invariants`](../../../crates/deckbound/src/reference.rs) (structure + gating),
[`reference.rs::check_combat_bands`](../../../crates/deckbound/src/reference.rs) (the gates under real
auto-resolved combat), and the grind-ladder properties in
[`balance.rs`](../../../crates/deckbound/src/balance.rs).

## The shape of the journey

**You always start with a way forward, and you can see the summit.** Everything in the world is
reachable from where you begin; each discipline has one clear place to *learn* it and one clear place
where it is *tested*; and there is a single climactic destination everyone is climbing toward.

- *Invariant:* reachability from A; one builder (B) + one gate (C) per path; the objective is the
  final location (`check_invariants`).
- *Feeling:* the world never reads as a maze or a dead end — there is always a legible next step and a
  known peak.

**The first fight welcomes you in.** The opening encounter is winnable with nothing equipped.

- *Invariant:* A is clearable clean-slate (`check_invariants` / `check_combat_bands`).
- *Feeling:* you learn the system by *playing* it, not by being punished for showing up unprepared.
  This is the gentle front edge of Charter **#5 (from doom to mastery)**.

## The core loop: feel the lack, then feel the fix

This is the heart of what the invariants protect. Each discipline's **gate cannot be passed until you
have actually earned and equipped that discipline's power** — but it opens cleanly once you have.

- *Invariant:* C[p] is **not** clearable without covering track *p* (the "no coverage leak" rule), but
  **is** clearable after building B[p]; and under real combat a bare party *loses* the gate while an
  equipped party *wins* it (`check_invariants` + `check_combat_bands`).
- *Feeling:* you hit a wall you genuinely cannot pass with your current toolkit → you go acquire the
  matching power → you come back and watch *that specific power* decide the fight. The game
  deliberately makes you **suffer an absence before granting the presence**, so the lesson lands.
  Because a bare party loses and an equipped party wins the *same* fight, outcomes are honest and
  diagnostic: a loss tells you exactly what you are missing, and a win is clearly attributable to the
  thing you brought — never to invisible stat creep.
- *Charter:* the operational form of **#12 (roles are load-bearing; necessity is *earned*, not
  granted)** — the gate outpaces the other disciplines, it does not forbid them.

## Depth is layered, and nothing is skippable

**Going deeper into a discipline is meaningfully harder without its foundations.** You cannot leapfrog
to the top tier of a suit; the lower rewards are load-bearing for the higher gates.

- *Invariant (grind ladder):* an L4-equipped party clears L5; an L1-only party does not (`balance.rs`).
- *Feeling:* mastery is earned in layers, not bought in one jump.

**Every discipline matters — the finale demands the whole toolkit.** The climax is beatable with full
coverage and *unbeatable with any single discipline missing.*

- *Invariant:* Final is clearable with full coverage and **not** clearable with a track absent; under
  real combat the final needs the full roster (`check_invariants` + `check_combat_bands`).
- *Feeling:* no path is decorative; each one has a moment where it is *the* answer. You cannot tunnel a
  single strategy to victory — the summit is a test of completeness, and it makes every player's
  specialty necessary. This is Charter **#4 (asymmetry by design, balance by scenario)** made felt.

## Many ways to build, with a gentle nudge toward sharing the load

The design wants **several viable team shapes**, not one correct answer. A "god" build — one
super-character carrying many disciplines with the rest as fodder — should be *viable but not optimal*,
while a balanced, evenly-advanced party is at least as strong.

- *Invariant (grind ladder):* an L5 falls to five characters carrying that suit's L4, **or** to a
  single god (L4 across suits) plus fodder; even advancement is at least as good as god + minions
  (`balance.rs`). The sharper, run-level form is
  [**BI-1** (role diversity dominates monotony)](balance-invariants.md#bi-1--role-diversity-dominates-monotony-).
- *Feeling:* players get real expressive freedom in how they compose, but the game quietly rewards
  spreading capability across the party — which, mechanically, is the **positional coverage** of the §4
  charge-and-gauntlet system: the non-positional suits (Controller / Support) are always available,
  while the positional suits (Wall hold / Infiltrator slip / Artillery fire) are **rivalrous on
  stance**, so a wide, evenly-built party covers more of the board each round than one concentrated
  hero can.

## What carries the weight

Underneath it all is a deliberate split: **stats keep you alive; powers decide the fight.** Health and
armour are your buffer against losing, but the active skills and unique passives are what actually win
or lose a combat.

- *Charter:* **#12** subordinates stats to roles ("stats are instruments, not first-class"); the
  interesting *decisions* carry the outcome, not the number-stacking.
- *Feeling:* the fun lives in bringing the right discipline's answer to bear at the right moment — and,
  per the co-op design intent, **each player owns that decision independently**, with their own
  per-character power budget (the §4.4 per-role play cap is *per actor*) rather than competing with
  teammates for a shared one. Tactics are a sharp, near-solvable skill; *which* power to chase and
  *when* to spend it is judgment (Charter **#2**).

## How the promises map to the checks

| Promise to the player                              | Enforcing invariant / property                                   |
| -------------------------------------------------- | ---------------------------------------------------------------- |
| The world is navigable and has a clear peak        | reachability; one B/C per path; objective = final                |
| A gentle on-ramp                                   | A clearable clean-slate                                          |
| Feel the lack, then the fix                        | C[p] gated on covering track *p*; bare loses / equipped wins     |
| Depth is layered                                   | L4 clears L5; L1-only does not                                   |
| No dead disciplines; the finale tests completeness | Final needs full coverage; not clearable with a track missing    |
| Multiple viable builds; even ≥ god                 | L5 falls to 5×L4 **or** god + fodder; BI-1                       |
| Powers decide; stats survive                       | Charter #12 (active powers / unique passives over generic stats) |

**See also:** [Charter](canon/1-charter.md) (#2, #4, #5, #12) ·
[balance-invariants.md](balance-invariants.md) (the checkable registry these feelings rest on) ·
[reference-scenario.md](reference-scenario.md) (the harness) ·
[Spec §8](canon/2-spec/README.md) (roles, rewards, the grind ladder).
