# Variety as a balance objective — reward combination, punish repetition

> **Ratified balancer preference, staged 2026-06-26.** A balanced game should make **epic wins
> possible by combining roles + mechanics**, and **punish repetitive (single-mechanic-spam) play**.
> **Promotion target: `computability-and-balance.md`** (the §6 balance method / closure-check family).
> Pairs with `role-weight-balance-testing.md` (synergy detection) and the closure check in
> `automated-balance-testing-roadmap.md`. Rides the delivered solver (`deckbound::{solve, winnable}`).

## It's a BALANCER objective, not a PLAYER objective (the load-bearing distinction)
The **player** always plays to win — the solver's lexicographic objective (win → fewer rounds → fewest
downed → most Health) stays **exactly as is**. You never make the *player* play sub-optimally to "be
varied." Variety is a property the **balancer** checks about the **game's tuning**, using solver outputs.
*(One harmless exception: a variety tiebreak among **equal-value** optimal lines, so the witness/tutorial
trace reads richer — cosmetic, never changes a verdict.)*

## Two measurable criteria (via the solver; no solver change)
Both ride **party-restriction** (a single-mechanic/role kit = monotone; the full kit = combined) +
**optimal-line analysis** (`solve().line`).

1. **Variety rewarded — "epic wins from combining."** Compare `solve(full kit)` vs the best
   `solve(single-mechanic kit)`. **Combo premium = value_full − value_mono.** Positive ⇒ combining
   strictly beats spamming (variety rewarded). Strongest form — **epic**: ∃ encounters where the
   **full party wins but every monotone restriction loses** (combining is *required*). A balanced game
   has many such combo-unlocked peaks.
2. **Repetition punished — monotone-spam is sub-optimal / non-dominant.** The optimal `line` should be
   **diverse**. Flags: (a) the optimal line is monotone-spam (low mechanic/role diversity) across the
   suite; (b) a single repeated mechanic **wins everything** (a dominant repetitive mechanic = a
   repetition-flavored **closure** violation).

## The tension with closure, resolved
Variety (reward strong combos) and closure (no line dominates) pull opposite ways — an epic combo that's
**too** strong becomes a dominant strategy. Resolution: **combos must be *situationally* epic, not
*universally* dominant** — powerful in their niche, with real setup cost / counterplay / encounter-
dependence (the **Controller + Artillery board-wipe** is the model: devastating, but a glass-cannon
concentration play that's evadable + disruptable). Joint target: **a diverse near-par set with situational
combo peaks; no universal dominance — monotone OR combo.**

## Where it fits
Sharpens §0.3's "many **interesting** strategies tie near par":
- the near-par set must be **diverse** (varied mechanics),
- **monotone-spam must be sub-par**,
- and the **best outcomes (epic wins) must require combining**.
It is the variety-oriented half of the closure-check family, and it promotes the role-weight **synergy
detection** (super-additive pairs) from a tolerated phenomenon to a **goal** — a balanced game *wants*
super-additive combos, bounded by non-dominance.

## Verdict criteria (add to the balancer's gates)
- **Combo premium > 0** — combining beats spamming (ideally with combo-*unlocked* wins = epic).
- **No monotone-spam dominance** — no single mechanic wins everything; the optimal line is diverse.
- **No combo dominance** — epic combos stay situational; closure still holds.
