# Deckbound — Reference Scenario (the balance harness)

> **Status: living, non-authoritative — a test fixture.** This is a **generic full-game
> scenario** whose purpose is to **detect imbalance anywhere in the game**, and which is
> **maintained as a test** so it keeps catching regressions as the Spec evolves. It is
> deliberately described **structurally** (parameterised by *progression path*), not as
> specific content, so it survives spec changes and rescales if the role/currency count changes.
>
> It exercises **all** the design docs at once: combat (`canon/2-spec` §1–§4), the **zone /
> resource** machinery ([`zones-exhaustion-design.md`](zones-exhaustion-design.md)), and the
> **geography / currency / progression** loop ([`progression-design.md`](progression-design.md),
> esp. its golf goal §6). Numbers are AI-seeded, human-tuned in `booklet.ron`.

---

## The shape — a diagnostic lattice

Let **P = the progression paths** (currently the **five roles**: Iron / Silver / Brass / Bone /
Salt — `progression §7`). The scenario is four location *sets*:

| Set           | Count            | What it is                                                                                                                                                                            |
| ------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A — Start** | 1                | The clean-slate entry. Clearable with **generic (Gold)** capability only — no path chosen yet.                                                                                        |
| **B — Build** | **one per path** | A power-building location for path *p*: clearing it **unlocks track-*p*'s role-card rewards** (§8.3) and teaches why *p* matters (the diegetic tutorial, `progression §4`).                      |
| **C — Gate**  | **one per path** | A location **designed to be impossible to clear unless path *p* has been built on its B-location.** C[*p*] is the *proof* that path *p* delivers.                                     |
| **D — Final** | 1                | One location tuned to challenge a party that has cleared **A + all B + all C**. **Near-impossible** unless the party has **covered every path thoroughly *and* plays strategically**. |

**Goal:** clear **D** in the **minimum number of days** (the golf par, `progression §6`).
**Meta-goal:** **surface any unbalanced aspect of the game** — because each location is a *probe*
for a specific balance property, a failure pinpoints *which* assumption broke.

## The invariants it checks *(this is the test)*

Each location asserts something; its failure detects a specific imbalance.

1. **A is clearable from a clean slate** (generic only).
   *Fails if:* onboarding is too hard, or the generic baseline is mis-sized.
2. **B[*p*] is clearable by investing path *p*** (and unlocks track-*p* rewards).
   *Fails if:* a path can't actually build power, or builds trivially.
3. **C[*p*] is clearable *iff* B[*p*] was cleared** — a **two-sided** invariant:
   - **clearable *with* path *p*** → path *p* delivers enough power.
   - **NOT clearable *without* path *p*** → path *p* is **necessary** — nothing else substitutes.
   *Fails if:* (a) C[*p*] falls **without** path *p* → that path is redundant, or another path
   leaks coverage into its gate, or the threat is too weak; (b) C[*p*] is unbeatable **with**
   path *p* → that path under-delivers, or the threat is overtuned.
4. **D is clearable *iff* all paths are built *and* play is strategic.**
   *Fails if:* D falls with a **path missing** (that path is redundant); D is **impossible even
   fully built** (overtuned); or D falls to **brute stats with no strategy** (a north-star **#2**
   violation — strategy must matter).
5. **Par (min days to clear D) sits in a sane band.**
   *Fails if:* par is trivially low or unreachable, or a **degenerate fast line** exists — the
   economy/pacing (`progression §6`) is mistuned.

Invariants **1–3 and 5 are crisply checkable**; **4's "strategy must matter"** needs a solver or
judgment (see below) — it's the one assertion that isn't a pure pass/fail on clear-ability.

## Why this catches *everything*

The lattice is built so the systems can only all pass together if they're mutually balanced:

- **B vs C** isolates **each path in turn** — power-building *and* necessity — so an over- or
  under-powered role can't hide.
- **D** forces **coverage × strategy** — it's the integration test that no single dominant path or
  brute-stat line can clear.
- **Par** turns the whole run into **one scalar** (days), so economy, routing, encounter
  difficulty, resource pacing, and the depth/breadth fork all register on a single dial.

## Maintained as a test — implemented

The lattice and its invariants are now a **live regression test** ([`reference.rs`](../../../crates/deckbound/src/reference.rs)):

- **The run-scenario fixture** is built in code (`reference_scenario`): the A/B/C/Final map over the
  world grid, per-location encounters, and the demand lattice. (It is built directly, not yet from a
  `booklet.ron` run-scenario schema — that authored object is still a future convenience.)
- **The evaluator** runs two ways: an **analytical** gating check (`check_invariants`) over
  **role-track reward coverage** — A/B free, C[*p*] clearable **iff** track *p* is covered, Final
  **iff** all tracks covered — and a **combat-real** band check (`check_combat_bands`) that
  auto-resolves a bare party (loses each C), a track-*p* specialist built from `rewards_for(p)` (wins
  C[*p*]), and the full party vs a short one at the Final. Both are asserted in the test suite.

*Re-expressed for the role-card model (2026-06-19):* the old currency-balance demands
(`Counter`/`AllPaths`) became **coverage** demands (`NeedTrack`/`AllTracks`) — a party covers track *p*
by holding any of its rewards. A full combat-resolving / par-search evaluator (the human-emulating AI
roadmap item) can still replace the band check later without changing the lattice.

## Open — making necessity *emergent*, not headcount (§8.6)

The necessity invariants (3, 4) are satisfied **structurally** today: `check_invariants` uses
**coverage** demands (`NeedTrack`/`AllTracks`), and `check_combat_bands` makes the Final a **Husk
swarm**, so a path-missing party loses on **raw headcount**, not because the absent role's *mechanic*
is the unique key. Charter **#12** / Spec **§8.6** require necessity to be **emergent, not by fiat**:
each role must be load-bearing because its *mechanic* is the efficient answer — *outpaced, not
forbidden* (no immunity gates). The open work:

- **Author one mechanic-lock per role** so each C[*p*] (and the Final's demand) is unwinnable within
  the analysis envelope without *p*'s signature mechanic — e.g. **Brass/Artillery** → a heavily-armored
  foe only **Pierce** cracks; **Iron/Wall** → a frontal grind only a **hold** survives;
  **Silver/Infiltrator** → a backfield only a **slip** reaches; **Bone/Controller** → an offense you
  must **disable** (Stagger/Slow), not out-trade; **Salt/Support** → an attrition fight only **buffs**
  outlast. Each must be *emergent* (the other roles still act, they just lose the race), never an
  immunity that bans them.
- **This closes T2 and T3 together.** The same authored locks make the signature *stats* **decisive** —
  `balance.rs::stat_necessity_report` (the T3 probe) currently reports **Pierce / Daring / Dread /
  Inspiration** as *not decisive in the grind ladder* precisely because the Husk content never forces
  their use. (The precise *consumption* guards already exist: `combat.rs`'s
  `dread_is_consumed_by_fear_attacks` / `inspiration_is_consumed_by_augments`.)

`reference.rs` carries an inline NOTE at the Final band-check marking this headcount-vs-emergent gap.

## Maps onto

- **Tests, in one fixture:** `canon/2-spec` §1–§4 (combat), `zones-exhaustion-design.md`
  (resources/exhaustion), `progression-design.md` (geography, currency, encounters, the golf goal).
- **Spec Coverage table:** depends on the still-⬜ **Strategic layer**, **Geography & travel**,
  **Run victory/defeat**, and **Progression** rows graduating (`canon/2-spec/README.md`).
- **Roadmap:** realises `roadmap.md`'s *Victory/defeat* (the run goal) and the **par-solver /
  human-emulating AI** tooling.
- **Rescales automatically:** described over **P paths**, so changing the role/currency count (e.g.
  the 4/5/6 decision in `progression §7`) just changes how many B and C locations exist.
