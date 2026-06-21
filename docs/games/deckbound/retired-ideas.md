# Deckbound — Retired Ideas (the design graveyard)

> **Status: non-authoritative; a graveyard, not a backlog.** Each entry is an idea that was
> **deliberately decommissioned** — taken *off* the table on purpose (Charter
> [#10](canon/1-charter.md): retire a principle "on purpose rather than letting the design drift").
>
> This is the counterpart to [`future-possibilities.md`](future-possibilities.md): that doc is
> forward-looking ("things we might add after playtest"); this one is backward-looking ("things we
> tried, considered, and *closed*, and why"). An idea lives here so that if it resurfaces, we already
> know **what sank it** and **the bar it must clear to come back** — a revived idea has to answer the
> objections that retired it the first time, not relitigate from zero.
>
> **Reviving an entry** means going through the normal change discipline
> ([`canon/0-source-of-truth.md`](canon/0-source-of-truth.md)): it must clear its stated bar, then go
> spec-first → code → tests. Don't quietly resurrect; clear the bar in writing.

---

## Aspects & the chord — the multi-deck combo system

- **Retired:** 2026-06-21 (moved out of `future-possibilities.md` entry 4, where it had been
  *deferred* since 2026-06-18).
- **Background (frozen):** [`notes/decks-and-aspects.md`](notes/decks-and-aspects.md),
  [`notes/combos.md`](notes/combos.md).

### The idea (what was being designed)

A character is a **set of aspect-decks** — dimensions of action (Body, Mind, Spirit, Magic…). A play
is a **chord**: **one card per aspect-deck, combined commutatively into a single action** (a thrown
strike + a fire manifestation + a prediction of the dodge resolved as *one* act). Card kinds:
**numberless** (quality directly), **modifier** (attaches, order matters), **passive** (standing
trait). Only the tactical aspect (the Clash) is rock-paper-scissors; the rest compose by fixed rules.
Acquiring a capability adds cards, or a whole new aspect-deck.

### Why it was retired

Not because suits *replaced* it — they didn't; the two are orthogonal (suits are stat granularity
and progression identity, the chord is *how cards combine into an action*). It was retired because
**the design moved decisively away from it and nothing now needs it:**

1. **Its intent is already met elsewhere.** "Combine many capabilities in play" is delivered by the
   **single-deck core + the §4.4 per-suit-per-round play cap + the Muster / phase / position split**:
   a party brings many disciplines to bear each round — *sequentially across the round and spread
   across the party*, rather than fused into one simultaneous action. The only thing the chord adds on
   top is **fusion within a single action**, which is depth on top of the core, never load-bearing for
   it.
2. **It fights Charter [#2](canon/1-charter.md) (small, computable tactics).** The chord enriches the
   *single tactical action* combinatorially; #2 says keep the tactical exchange small and well-defined
   so it stays solvable. A multi-deck chord pushes richness into exactly the layer #2 wants lean.
3. **It strains Charter [#11](canon/1-charter.md) (canonical mode stays computable).** Composition
   **multiplies the balance and search surface**; battles must stay near-stateless functions of
   `(build, place)` with modest branching for the par solver to run. The chord works against that.
4. **The simplification trajectory has overtaken it.** Since it was parked, the Mind defense channel
   was merged into Tempo (three channels → two), and the `Aspect` / `keystone` enum was **deleted from
   the code** entirely. The codebase has shed the very vocabulary the chord was built on.
5. **The one organizing idea worth keeping already survived, renamed.** "A character is a set of
   stat-decks" lives on as the Form's **deck × suit** stat grid (Body·Quantity = health count,
   Body·Power = Toughness, …). So retiring the chord loses the *simultaneous-composition mechanic*, not
   the *structuring idea*.

### The bar it must clear to come back

If this resurfaces, it must answer **all** of these before it re-enters the backlog — these are the
considerations that sank it:

1. **Earn its keep over the per-round / per-party spread.** Show a concrete, *felt* expressiveness gap
   that only **fusing layers within one action** can fill — something the §4.4 multi-suit round and the
   position split demonstrably cannot already give. "It's more elegant" is not enough.
2. **Survive #2.** Keep the per-action option space **bounded** and the Clash **solvable** — demonstrate
   the tactical exchange stays small and computable *with* the chord, not just without it.
3. **Survive #11.** Show the canonical mode stays **bounded and par-solvable** — that composition does
   not blow up the search surface (battles still near-stateless functions of `(build, place)`).
4. **Bring playtest evidence first.** Per the backlog discipline, arrive with **clean data from the
   single-deck core** showing the gap — not an armchair case.
5. **Resolve the name collision and the overlap.** "Aspect" was reserved-but-unused and is now gone
   from code; pick a **non-colliding name**, and make the revival the **composition mechanic only** — it
   must not re-introduce the deck × suit stat grid under a new label.
