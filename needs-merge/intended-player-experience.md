# Intended player experience — the north star behind the invariants

What the game *ought to feel like*, derived from the diagnostic invariants and balance
properties (`crates/deckbound/src/reference.rs::check_invariants` /
`check_combat_bands`, and the grind-ladder properties in
`crates/deckbound/src/balance.rs`). This is the experiential intent those tests
enforce — read the invariant, then read the feeling it is protecting.

> **One sentence:** the game should feel like a fair, legible climb where you
> repeatedly run into a wall, recognize exactly which power you lack, go earn it,
> and feel it visibly turn the next fight — until the summit asks you to prove
> you've become complete.

## The shape of the journey

**You always start with a way forward, and you can see the summit.** Everything
in the world is reachable from where you begin; each discipline has one clear
place to *learn* it and one clear place where it is *tested*; and there is a
single climactic destination everyone is climbing toward.

- *Invariant:* reachability from A; one builder (B) + one gate (C) per path;
  the objective is the final location.
- *Feeling:* the world never reads as a maze or a dead end — there is always a
  legible next step and a known peak.

**The first fight welcomes you in.** The opening encounter is winnable with
nothing equipped.

- *Invariant:* A is clearable clean-slate.
- *Feeling:* you learn the system by *playing* it, not by being punished for
  showing up unprepared.

## The core loop: feel the lack, then feel the fix

This is the heart of what the invariants protect. Each discipline's **gate cannot
be passed until you have actually earned and equipped that discipline's power** —
but it opens cleanly once you have.

- *Invariant:* C[p] is **not** clearable without covering track *p* (the "no
  coverage leak" rule), but **is** clearable after building B[p]; and under real
  combat a bare party *loses* the gate while an equipped party *wins* it.
- *Feeling:* you hit a wall you genuinely cannot pass with your current toolkit →
  you go acquire the matching power → you come back and watch *that specific
  power* decide the fight. The game deliberately makes you **suffer an absence
  before granting the presence**, so the lesson lands. Because a bare party loses
  and an equipped party wins the *same* fight, outcomes are honest and
  diagnostic: a loss tells you exactly what you are missing, and a win is clearly
  attributable to the thing you brought — never to invisible stat creep.

## Depth is layered, and nothing is skippable

**Going deeper into a discipline is meaningfully harder without its foundations.**
You cannot leapfrog to the top tier of a suit; the lower rewards are load-bearing
for the higher gates.

- *Invariant (grind ladder):* an L4-equipped party clears L5; an L1-only party
  does not.
- *Feeling:* mastery is earned in layers, not bought in one jump.

**Every discipline matters — the finale demands the whole toolkit.** The climax is
beatable with full coverage and *unbeatable with any single discipline missing.*

- *Invariant:* Final is clearable with full coverage and **not** clearable with a
  track absent; under real combat the final needs the full roster.
- *Feeling:* no path is decorative; each one has a moment where it is *the*
  answer. You cannot tunnel a single strategy to victory — the summit is a test of
  completeness, and it makes every player's specialty necessary.

## Many ways to build, with a gentle nudge toward sharing the load

The design wants **several viable team shapes**, not one correct answer. A "god"
build — one super-character carrying many disciplines with the rest as fodder —
should be *viable but not optimal*, while a balanced, evenly-advanced party is at
least as strong.

- *Invariant (grind ladder):* an L5 falls to five characters carrying that suit's
  L4, **or** to a single god (L4 across suits) plus fodder; even advancement is at
  least as good as god + minions.
- *Feeling:* players get real expressive freedom in how they compose, but the game
  quietly rewards spreading capability across the party — which, mechanically, is
  the positional coverage of the charge-and-gauntlet system (non-positional suits
  are always available; the positional suits are rivalrous on stance, so a wide,
  evenly-built party covers more of the board each round than one concentrated
  hero).

## What carries the weight

Underneath it all is a deliberate split: **stats keep you alive; powers decide the
fight.** Health and gear are your buffer against losing, but the active skills and
unique passives are what actually win or lose a combat.

- *Intent:* active powers and unique passives outweigh generic stat padding; the
  interesting *decisions* carry the outcome, not the number-stacking.
- *Feeling:* the fun lives in bringing the right discipline's answer to bear at the
  right moment — and, per the co-op design intent, each player owns that decision
  independently, with their own per-character power budget rather than competing
  with teammates for a shared one.

## How the invariants map to the promises

| Promise to the player | Enforcing invariant / property |
| --- | --- |
| The world is navigable and has a clear peak | reachability; one B/C per path; objective = final |
| A gentle on-ramp | A clearable clean-slate |
| Feel the lack, then the fix | C[p] gated on covering track *p*; bare loses / equipped wins |
| Depth is layered | L4 clears L5; L1-only does not |
| No dead disciplines; the finale tests completeness | Final needs full coverage; not clearable with a track missing |
| Multiple viable builds; even ≥ god | L5 falls to 5×L4 **or** god + fodder |
| Powers decide; stats survive | active powers / unique passives > generic stats (design intent) |
