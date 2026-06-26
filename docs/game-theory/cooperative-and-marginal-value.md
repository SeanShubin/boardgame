# Cooperative Game Theory — measuring a member's contribution to a team

The counter-system documents and the minimax / Nash tools in `solution-concepts.md` are
**non-cooperative**: agents with opposed interests. A different branch of game theory applies when
agents form a **team** with a shared outcome and you want to know **how much each member is worth.**
This is the right frame for any role / class / unit that is weak alone but decisive in the right company
— supports, enablers, anchors, force-multipliers.

---

## The wrong question and the right one

- **Wrong:** "How strong is this element *in isolation*?" A support / healer / enabler may contribute
  ~nothing solo yet be decisive in a team. Isolated testing measures the wrong quantity.
- **Right:** "How much does the team's outcome change when this member is **present vs absent**, across
  team contexts?" — its **marginal contribution.**

---

## Marginal contribution & the Shapley value

Let `v(coalition)` be the team value function — the outcome a subset of members achieves (use a
**graded** outcome, e.g. how hard a challenge the coalition can clear; a binary win/loss flattens the
signal because *both-win* hides the delta).

- Member `i`'s **marginal contribution** to a coalition `S` is `v(S ∪ {i}) − v(S)`.
- The **Shapley value** `φᵢ` is `i`'s **average marginal contribution over all orderings** (equivalently,
  over all coalitions of the *other* members). It is the unique credit assignment satisfying
  **efficiency** (the parts sum to the whole), **symmetry** (equal contributors get equal credit),
  **null-player** (a member that never helps gets 0), and **additivity**. Crucially it **credits synergy
  automatically**: a super-additive pair (joint value > sum of solo values) has its bonus *split between
  the partners*, so neither freeloads.

**Computable when `v` is computable.** For a small roster the `2^(n−1)` coalitions per member are
enumerable; a *deterministic* `v` makes these **computed, not sampled.**

---

## Report max-marginal *and* average — the specialist test

The single most useful distinction falls out of looking at **both** statistics:

- **Specialist** = **low average, high max** — worthless in most teams, **decisive in its niche.** Fine
  by design.
- **Dead weight** = low average **and** low max — no niche anywhere. Cut it.
- **Generalist / anchor** = high average — lifts most teams.
- **Dominant / over-tuned** = lifts *every* coalition (and may win in a slot it shouldn't own).

So: **judge a member by its peak marginal contribution, never its solo value, and cut only for a low
*max*.** "Only valuable in the right team" is the system working as intended, not a flaw.

---

## Three things that lie to you (more than the math does)

- **Policy-relativity — the big one.** `v(coalition)` depends on **how well the team is played.** A weak
  play policy systematically **under-reads** members whose value needs setup or timing, flipping the
  verdict. Measure with a **strong (near-optimal) policy**, or the numbers are noise. (The general
  statement of this is the *value of unpredictability* / certified-policy discussion in
  `solution-concepts.md` §4–5.)
- **Context coverage.** A member is valuable only in scenarios that *demand* it. A test suite missing a
  member's niche reports a **false negative.** The suite is itself an instrument — and a **coverage
  ledger**: no niche in a *diverse* suite ⇒ genuinely dead weight; no niche only because the suite is
  thin ⇒ a suite bug.
- **Profiles — don't compare value across roles.** Some members are *meant* to solo (an **anchor**);
  others are *meant* to be force-multipliers worth little alone (a **multiplier**). Comparing their raw
  values is a category error. Check each against **its own** intended profile (an anchor must solo its
  scenarios; a multiplier must own a decisive niche), and flag **dominance** = winning in a slot it
  shouldn't own (an anchor clearing a multiplier's niche).

---

## Verdict rule

A member **pulls its weight** ⟺ it has a **decisive max-marginal somewhere in a diverse suite** (a real
niche) **and** is **non-dominant** outside it. There is no single scalar to mis-compare across roles, so
the intended "the anchor solos, the support can't" asymmetry never reads as imbalance.

---

## Tiered build (cheap → principled)

1. **Leave-one-out over a diverse suite** — swap each member for the best alternative; outcome degrades
   ⇒ it contributed. Cheapest; catches obvious dead weight. (Swap, don't delete-to-nothing — the real
   question is *opportunity cost*.)
2. **Pairwise marginals** — surfaces super-additive (synergy) pairs.
3. **Full Shapley + a difficulty-frontier metric** — the principled aggregate, once a strong play
   policy exists.

**See also:** `game-classification.md` (the cooperative cell) · `solution-concepts.md` (the strong
policy this measurement depends on).
