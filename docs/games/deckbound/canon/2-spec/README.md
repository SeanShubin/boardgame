# Deckbound — Mechanical Spec

**Canonical for mechanics.** This is the precise statement of how Deckbound's
systems work. It is a source of truth (see
[`0-source-of-truth.md`](../0-source-of-truth.md)) — the code conforms to it, not the
other way around. It owns **vocabulary and procedures**, not **numbers** (numbers
live in [`booklet.ron`](../../../../crates/deckbound/data/booklet.ron)).

> **AI assistants:** read [`0-source-of-truth.md`](../0-source-of-truth.md) first. In
> short: edit this Spec to change a *rule*; never to change a *number*. Classify
> every proposal as a mechanics-fix (case 1), an invariant violation (case 2), or
> an intent change (case 3) — using each rule's WHY and GUARANTEES.

---

## How to read a rule

Every rule is a triple. This format is mandatory — it is what makes the
intent-vs-mechanics distinction answerable.

- **RULE** — what the mechanic *is*, stated precisely and operationally. The thing
  the code must implement and the manual must print.
- **WHY** — the intent: the problem it solves and the Charter north star it serves.
  Changing this is changing **design intent** — a human decision.
- **GUARANTEES** — the invariants the rule exists to preserve. A change that keeps
  the RULE's letter but breaks a GUARANTEE is a defect even if it "compiles."

The point of the **WHY** is **motivation**: a rule whose form follows from its intent
is *re-derivable* — a reader who forgets the letter can reconstruct it from the WHY.
That is this Spec's aim, **conceptual integrity**: every rule springs from a few
intents, so you *reconstruct* the mechanics rather than memorize them. So **prefer a
motivated rule — one that carries its own rationale — over a merely short one**, and
treat a rule you cannot motivate as a smell. This is **Charter north star #10
(conceptual integrity)**; theme is one engine of motivation (a rule that falls out of
the fiction is re-derivable from the world), but a rule can equally be motivated by its
consequence. See also [`0-source-of-truth.md`](../0-source-of-truth.md) — "Motivated rules."

Numbers appear only as *(appendix)* illustrations; the real values are in
`booklet.ron` and are human-tuned.

**Keyword rules** additionally carry a **MANUAL** line — the one sentence that
prints in the rulebook / on hover. The engine pairs each keyword's handler with
this line so digital and printed rules can't drift; the Spec is where that line is
authored.

**Cards may supersede the core.** Every rule here is a **default**. A card may
**explicitly override** a specific core rule — and it says so on its face, naming the
rule it bends. This keeps the core small and learnable while letting variety live on the
cards (e.g. the core says only melee Actors skirmish, §4.2; a card can grant a ranged
Skirmisher). A card never *silently* contradicts the core; an unstated conflict is a defect.

---

## Coverage

| System                                            | Spec status | Current design source if not yet specced                                                                                                               |
| ------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **The deterministic core** (separable balance)    | 🟡 seeded    | **§0** — determinism · separable luck layers · objective core balance — `computability-and-balance.md`                                                  |
| **The Clash** (tactical core)                     | ✅ worked    | —                                                                                                                                                      |
| **Defense model** (cut → bar → pool)              | 🟡 seeded    | `notes/stats.md`, `notes/form-and-defeat.md`; **§2.3 stats-as-deck** specced (code/data migration pending `/spec-sync`)                                |
| **Speed/Tempo** (one breadth pool)                | 🟡 seeded    | §3 — Tempo pays offense *and* defense; **Focus/Mind merged out** (2026-06-20); `notes/speed-and-tempo.md`                                               |
| **The battle — the charge & the gauntlet**        | 🟡 seeded    | §4 **respecced** to charge-and-gauntlet (lanes removed, 2026-06-20); code migration pending `/spec-sync §4`. §4.3 actors-are-decks also pending |
| **Zones / exhaustion**                            | 🟡 seeded    | **§5 worked** (zones · Form/Action · verbs · tags); resources 🟡 (stats-as-deck now §2.3/§4.3) — `zones-exhaustion-design.md`                           |
| **Aspects / the chord**                           | ⏸ deferred  | parked → `future-possibilities.md` (entry 4) — single-deck core first                                                                                  |
| **Agents** (Character vs Creature)                | ⬜ stub      | `notes/entities.md`, `notes/decision-making.md`                                                                                                        |
| **Strategic layer** (world/event decks)           | 🟡 seeded    | **§8** (world · clock · role-card rewards · encounters · progression) — `progression-design.md`                                                        |
| **Skirmish victory / defeat**                     | 🟡 seeded    | `notes/form-and-defeat.md` (eliminate the foes / the party falls; in code)                                                                             |
| **Run victory / defeat** (across many skirmishes) | 🟡 seeded    | **§8.2** — victory = clear the objective, scored in Days (golf); **defeat deferred** pending reference-scenario tuning                                 |
| **Geography & travel** (the world map + movement) | 🟡 seeded    | **§8.1** (locations · move 1/Day · fog); travel risk deferred — `progression-design.md`                                                                |
| **Loot / role cards** (clear → reward)            | 🟡 seeded    | **§8.3** — atomic 25-card role-reward pool, scarce, party-assigned permanently; each reward **of a Suit** (Iron · Silver · Brass · Bone · Salt); **no currency** (role-card redesign, *in code 2026-06-19*) — `role-card-redesign.md`         |
| **Progression** (growth between skirmishes)       | 🟡 seeded    | **§8.5** — role = assigned cards · `3+2` tracks, each a **Suit** ↔ **Role** (identity ↔ function) + bundled Stat layer · depth/breadth; play rule §4.4, taxonomy §5.6 (*in code 2026-06-19*) — `role-card-redesign.md`         |

✅ worked = full, the template to follow · 🟡 seeded = a few real rules, not
exhaustive · ⬜ stub = headers + intent only, not yet authoritative · ⏸ deferred = parked to
`future-possibilities.md`.

---

## 0. The deterministic core — separable balance 🟡

Deckbound is built so that **balance is decomposable.** Beneath the played game sits a
**deterministic, perfect-information, single-agent core** — the canonical mode with the Clash
module **off** and creature draw decks and locations **open**. That core is **feasibly
computable**: a scenario's par (fewest Days to clear it) and every combat outcome can be
*computed*, not estimated. **Luck and hidden information are separable layers on top of it.** The
design contract: **solve the core, balance each luck layer in isolation, and the composition is
balanced with high confidence** — without ever solving the full stochastic game. This section is
the binding form of that contract; full rationale, the design-review checklist, and the balancing
method are in [`computability-and-balance.md`](../../computability-and-balance.md) (**Charter
#11**). *(This is the **whole-game** core for tuning; not to be confused with the **Clash**, the
**tactical** core of §1.)*

### 0.1 The core is computable

**RULE.** With the **Clash module off** and creature decks and locations **open**, the game is
**deterministic** (no rule consumes randomness), **perfect-information** (nothing hidden),
**single-agent** in PvE (creatures run a fixed, non-adaptive policy — an environment, not an
opponent that searches back), and **bounded** (a Day cap, finitely many reachable builds,
terminating combat). A run is therefore a finite planning problem with a computable optimum, whose
state is **`(positions, cleared-set, builds, Day)`** — and the **builds** are the campaign's *only*
carried state: combat is stateless, but **progression** is not.

**WHY.** A computable core is the **balance instrument**: it lets us *prove* a scenario is
beatable, *compute* its par, and *check* that no single line dominates (#11). It is how we **keep**
#2's "no solvable collapse" and #4's "balance by scenario" — by *measuring* them instead of hoping.
Lose computability and balance becomes unverifiable.

**GUARANTEES.**
- **Clash off ⇒ a battle's outcome is a pure function** of the two sides' Forms and the encounter —
  no RNG, bit-identical every run.
- Creatures never **adapt to the player's specific plan** (fixed instinct / policy); PvE stays
  single-agent. *(Two human sides is the Versus mode, §3.4 — outside the core.)*
- A battle carries **no _combat_ state between fights** — each is rebuilt from `(build, place)`, so
  no wounds or buffs persist; combat is therefore a **memoizable oracle** over the finite set of
  reachable builds. The campaign's carried state is the **build** (progression, §8.5) plus the
  economy that funds it (§8.3) — *not* combat history.
- **No path-dependent budget.** The build's resource / ownership state must be a function of *what*
  you have, never *how you got it*. Owned assets only **accrue** (monotone), **combine additively and
  order-independently** (§5.2 / §2.3 — commutative Form), and **no operation refunds a spent
  resource** (no sell-back, no consume-then-recover). **This is what keeps progression computable:**
  characters evolve along *many trajectories*, but order-independence collapses them onto a *small set
  of build states*, and monotonicity makes dominance pruning valid (an earlier or superset build
  dominates) — so trajectory-diversity does **not** become state-explosion. **The killer is a
  path-dependent budget, not movement:** freely *rearranging already-owned, monotone assets* (e.g.
  reassigning a card between characters) keeps the build Markovian and is fine; it is **resource
  refund / oscillation, order-dependent stacking, or multiplicative combos** that make the budget
  history-dependent and explode the search. *(This is the precise form of the old "no removal/swap"
  shorthand — sharpened 2026-06-19.)*
- The run is **bounded and terminating** — Days are capped, branching is finite, combat has its
  termination backstop (§1.6).

### 0.2 Luck is a separable layer

**RULE.** Every **randomness or hidden-information** mechanism — the Clash's hidden simultaneous
reveal and randomized creature decks (§1), location fog (§8.1), the event deck (§8.2), threat-deck
draws (§8.4) — is an **optional layer over the core.** Disabling all of them **recovers the
computable core unchanged.** No luck mechanism is load-bearing for core *function*: turning luck off
may make the game easier or more legible, but never breaks it.

**WHY.** Separability is what makes balance decompose (#11): if luck lifts off cleanly, the core can
be solved on its own and each luck layer reasoned about on its own. A luck mechanism welded into
core function would couple the two and destroy the instrument.

**GUARANTEES.**
- There is a switch (conceptual or real) that disables each luck / hidden-info mechanism; with all of
  them off, the game is exactly the §0.1 core.
- No core rule's **correctness** depends on a luck mechanism being present — only its *difficulty* or
  *legibility* may.

### 0.3 Separable balance

**RULE.** Balance is established in two **independent** steps and composed. **(1) The core is
balanced on the solver:** par is computed and the numbers tuned so that *many* **interesting**
strategies tie near par and **no** strategy dominates them — including the **closure check** that no
*unnamed* strategy beats the interesting set. **(2) Each luck layer is balanced in isolation:** shown
neutral / non-dominant on its own terms **before** it is added. A luck layer is not admitted until it
is independently balanced.

**WHY.** If the core is balanced and only **independently-balanced** luck is added, the full
(non-computable) game is balanced with high confidence — without solving the full game (#11). The
player still meets uncomputable strategy (#2) and scenario-borne fairness (#4); the *designer* gets an
objective floor.

**GUARANTEES.**
- Core balance is **objective** — measured against the computed par, not estimated by playtest alone.
  *(Today the harness is the reference scenario's invariant / combat-band checks —
  `reference-scenario.md`; the full par solver is a pending build — see `computability-and-balance.md`
  §4, §8.)*
- No luck mechanism ships **un-balanced on its own** (neutral-in-expectation / no dominant exploit in
  isolation); "balance the whole stochastic game directly" is **never** the method.
- **Par is policy-relative** — always stated relative to a fixed combat resolver; a weak resolver
  biases the result (`computability-and-balance.md` §5).

*(SEEDED — §0 graduates Charter #11 into binding GUARANTEES. §0.1 / §0.2 are structural invariants
the code already upholds (Clash is the sole RNG; battles rebuild from `(build, place)`; Form
combination is commutative, §5.5). §0.3 is the **method**: its instrument — the par solver / balance
harness — is a pending build (a Rust crate or `examples/` program, never an ad-hoc script), so today
core balance leans on the reference-scenario checks. No `TERM` encyclopedia lines: these are
**designer** invariants, not player vocabulary.)*

### 0.4 The analysis envelope — bounding for solvability 🟡

**RULE.** Two of combat's bounds are **pre-game parameters**, not fixed laws — set once before a battle
like the seed and the Clash module, and carried in the **`Ruleset`** (`ruleset.rs`):
- a **round cap** — reaching it ends the fight as a **draw** (in PvE, a draw is no different from a
  loss given current mechanics); and
- a **roster cap** — the max distinct unit *types* per side, where a **swarm counts as one** (identical
  instances are symmetric).

Live play uses `Ruleset::default()` (effectively unbounded — the historical termination backstop, §1.6
/ §4). **Analysis tooling uses `Ruleset::analysis()`** (a short horizon — currently 5 rounds — and a
small roster — 5 types). The bounded envelope is what makes optimal single-combat play **finite and
exactly searchable**: with a hard round horizon whose leaf is *terminal by rule*, there is nothing to
estimate — backward induction is exact, with **no evaluation heuristic** (the usual source of
"strong-but-not-perfect"). The roster cap (with swarm-as-one symmetry) bounds per-round branching.

**WHY.** §0.1 says the core is computable *in principle*; this is the lever that makes it computable *in
practice*. The horizon collapses depth and removes the convergence/backstop reasoning; the draw-on-cap
rule makes the PvE objective a clean boolean ("winnable within the horizon?"), so a perfect player is a
bounded reachability search rather than an open-ended optimizer. In the game-theoretic modes (PvP,
Clash, a simultaneous auction) the same bounds shrink each hidden-simultaneous commit to a *small matrix
game* solvable by LP, so backward induction over the bounded horizon computes the equilibrium.

**GUARANTEES.**
- The round/roster bounds are **parameters**, defaulting to unbounded live play; only the *analysis*
  setup imposes the short envelope, so live balance/behaviour is unchanged by their existence.
- Bounding gives **finiteness / tractability**, which is **orthogonal to rule completeness**: the solver
  still optimizes a *concrete* rule-set, so the §4 open dials (the escalating auction's form, the
  multi-intercept cap, charge-order threading) must be pinned (or the v1 code semantics ratified) before
  "perfect" means *perfect at the designed game*.
- The envelope doubles as a **design assertion**: every intended encounter is winnable within the
  horizon under optimal play; one that is not is **mis-tuned** (too grindy), not merely "hard". A
  not-enforced cap means a cap-draw verdict reads as "violates the round-horizon design target", not
  "the unbounded game cannot win it". *(Empirically the reference campaign resolves within **3** rounds
  under the greedy resolver — comfortably inside the 5-round envelope, so the bound is non-disruptive
  today.)*

*(SEEDED — no `TERM` lines: a **designer/analysis** invariant, not player vocabulary. The `Ruleset`
exists in code; the par-solver that consumes the envelope is the pending build of §0.3 /
`computability-and-balance.md`.)*

---

## 1. The Clash — *the tactical core* ✅

The atom of combat: two Actors **predicting each other** across a hidden, simultaneous
mix-up played with cards. Design background:
[`notes/the-duel.md`](../notes/the-duel.md).

> **History.** This section formerly specced a stance/Edge duel (Marshal · Unleash ·
> Overwhelm · Parry, tracked Edge) and then an interim six-move *charge* duel. Both are
> **superseded by §1.0 (The Clash)** below — the four-card, Force-stealing,
> **ends-on-strike** duel. The old stance/Edge subsections (§1.1–§1.5, §1.8) are kept for
> design history behind banners; their WHY/GUARANTEES carry forward. §1.3 (ends-on-strike)
> is **restored** as current; §1.6 is reworded for it; §3 (Tempo/Focus) is rewritten and
> §3.3 (Exposed) is removed.

> **The Clash is an optional module.** The canonical floor (§4.2) resolves a same-range
> engagement as a **simultaneous trade**; the Clash below *replaces* that trade with a four-card
> mix-up + Force when a scenario enables it. Everything in §3–§4 (roles, phases, the gauntlet,
> Tempo) runs identically either way.
>
> **Reconciliation pending (2026-06-20).** This section still uses the old **Focus / Mind** vocabulary
> (e.g. "reading the foe with Focus unlocks your stance menu"). Those are **merged/removed** — there is
> one **Tempo** pool now (§3.1), and the Clash is **off in the base game** (the campaign uses the §4.2
> trade). A full §1 reconciliation (re-expressing the Clash's read/commit layer in Tempo terms, or
> confirming the Clash keeps its own internal currency) is **deferred** — it is not on the
> base-gauntlet code path. Where §1 conflicts with §2–§4, **§2–§4 win.**

### 1.0 The Clash — four cards, Force, ends-on-strike

**RULE.** A duel is a sequence of **beats**. Each beat both fighters **secretly choose one
card** and reveal **simultaneously** — no one reveals first; any "see their card before you
choose" effect is a special ability, never the core. The duel **ends the instant one or both
are struck**. The kit is four cards, always complete:

- **Strike** — hit *where they are now*. Beats **Gather**; stopped by **Evade**.
- **Anticipate** — hit *where they'll be* (lead the target). Beats **Evade**; stopped by
  **Gather**.
- **Gather** — *hold your ground* (a defense) **and build Force** (+1). Stops **Anticipate**;
  beaten by **Strike**.
- **Evade** — *move*. Stops **Strike**; beaten by **Anticipate**.

**The cycle.** Anticipate ▸ Evade ▸ Strike ▸ Gather ▸ Anticipate (each beats the next), plus
**Strike > Anticipate** when both attack, **Strike vs Strike → trade** (both hit), and
**Anticipate vs Anticipate → whiff**.

**Resolution table** (result shown for the row player):

| you ↓ \ them → | **Gather** | **Evade**         | **Strike**                | **Anticipate** |
| -------------- | ---------- | ----------------- | ------------------------- | -------------- |
| **Strike**     | you hit    | your Force → them | trade (both hit)          | you hit        |
| **Anticipate** | —          | you hit           | you're hit                | —              |
| **Gather**     | +1 Force   | +1 Force          | you're hit                | +1 Force       |
| **Evade**      | —          | —                 | their Force → you (min 1) | you're hit     |

*Enders* (a strike connected → the duel ends): **you hit / you're hit / trade**. Everything
else is the **non-connecting dance** — the duel continues and Force builds.

**Force.** A single count per side (no face-down state). Each Force **doubles** the connecting
hit: damage = `base × 2^Force`, routed through the armor/toughness pipeline (§2). **Gather**
adds +1. The **only** way Force changes hands is **Strike into Evade**: you commit a Strike,
they slip it, and your Force **goes to them** — your own momentum turned against you — and the
evader **always gains at least 1** Force from the slip, even when the Striker had none (a clean
dodge always buys momentum). Force is
**per-duel** (it resets each duel); only **Body** persists between duels. There is **no Force
cap** (unlimited) — building is bounded in practice by ends-on-strike (the duel ends the
instant a blow connects), not by a ceiling. The kit is **infinite-replay**: every card is
always available each beat (no finite hand or discard yet).

**WHY.** The kit is always complete, so a perfect guesser can always answer the card in front
of them — that is what makes the reachability invariants hold for a whole duel.
Ends-on-strike keeps duels short and makes the build-then-land arc tense: you stack Force in
the dance, but the opponent controls whether your loaded blow ever connects. The single steal
vector is re-derivable from one idea — *only an active dodge (Evade) of a committed Strike
reverses; the passive build (Gather) never steals* — and it is the Gandalf-vs-Balrog engine:
a weak fighter can heist a loaded Strike, but reaching for the win is where the trade kills
them (north stars #2 computable, #4 asymmetry, #10 re-derivable).

**GUARANTEES** — under perfect guessing (the analytical lens: *"I happened to guess right"*):
1. **Avoid.** You can pass a duel **un-hit** — every attack has a defense that negates it
   (Strike↦Evade, Anticipate↦Gather).
2. **Land.** You can force a connecting hit — every move has an answering attack
   (Gather↦Strike, Evade↦Anticipate, Strike↦Strike-trade).
3. **Not both, free.** Landing on a committed Striker means **trading** a hit. *Survival is
   free; victory costs exposure.* (Whose hits actually land on whom is set by the breadth
   layer — §3: offense lands, a Focus-defense is reset.)
- **Termination.** Ends-on-strike resolves duels in practice (blind guesses → someone
  eventually misreads → a strike connects); the §1.6 backstop only covers the theoretical
  perfect-mutual-defense edge.

**MANUAL.** *Each beat pick a card: Strike (hit where they are) or Anticipate (where they'll
go) to attack; Gather to hold your ground and build Force; Evade to dodge. A connecting strike
ends the duel; slip a Strike with Evade and you steal their Force (always at least 1).*

**Glossary.** *(Encyclopedia terms — the in-app rules reference is generated from these `TERM`
lines, so it can't drift from this Spec.)*

- **TERM.** `The Clash` (Clash module) — An optional 1v1 mix-up that replaces a same-range trade. Each beat both pick a card and reveal at once: Strike, Anticipate, Gather, Evade.
- **TERM.** `Cards` (Clash module) — Strike beats Gather; Anticipate beats Evade; Gather beats Anticipate; Evade beats Strike. Strike also beats Anticipate; Strike-vs-Strike trades.
- **TERM.** `Force` (Clash module) — Gather builds +1 Force; each Force doubles your connecting hit. Evading a Strike steals the striker's Force (always at least 1).

### 1.1 Edge is per-duel, public, all-or-nothing, linear

> **SUPERSEDED by §1.0 (The Clash).** The tracked Edge meter is replaced by **Charges**
> (durable ×2 cards). The intent below — a *per-duel, public, no-runaway-hoard*
> escalation resource — carries forward: Charges reset each duel, are face-up, and a
> defended Charge flips down rather than compounding.

**RULE.** Every duel starts at **0 Edge** for each side. Edge is built and spent
**inside that duel only** and **does not carry** to any other duel — not even
between two duels involving the same Actor. Both banks are **public**. Spending
Edge spends **all of it** (no partial commit). A spent bank of *n* contributes
exactly *n* (linear).

**WHY.** A per-duel meter is the big simplifier: it removes the cross-round
hoarding, stalling, and runaway-snowball problems a fight-long meter creates, and
keeps the tactical exchange small and computable (Charter §2: *computable
tactics*). Public + all-or-nothing makes it a clean yomi read ("respect the
meter") rather than a hidden-quantity guessing game. Read intent-first, a
side's Edge is *the trouble the other side ran into by overextending into the
clash* — which is why it accrues only inside a mutual engagement (§1.8): with no
one reading you there is no overextension to punish, so no Edge is banked by
either side. Edge is the price of a contested exchange, never a free resource.

**GUARANTEES.**
- No fight-long bank exists; breadth never compounds into one mega-bank (a "god"
  facing many foes is a stack of independent short duels, powerful in each, never
  one accumulating super-bank).
- A bank of *n* can never do more than *n* — no one-shot-from-hoarding.
- Both players can always see the stakes; nothing about Edge is hidden.

### 1.2 The four stances and the triangle

> **SUPERSEDED by §1.0 (The Clash).** The four stances become the **six moves**
> (Strike/Throw/Parry/Evade + Charge/Recover). The intent below — **no dominant
> option**, a throw that beats the block so no stance is safe — carries forward as the
> §1.0 cycle (each attack beats one defense, loses to the other; Throw beats Parry).

**RULE.** Each fighter secretly commits one of **Marshal · Unleash · Overwhelm ·
Parry**; reveal simultaneously.
- **Marshal** *(neutral)* — bank Edge; exposed to Unleash.
- **Unleash** *(strike)* — pour all Edge into a blow; beats Marshal and Overwhelm;
  loses to Parry.
- **Overwhelm** *(throw)* — drive all Edge through a guard; beats Parry; **whiffs**
  (loses its Edge for nothing) against a non-guard (Marshal or Unleash).
- **Parry** *(block)* — beats Unleash; loses to Overwhelm.

The offensive triangle is **Unleash ▸ Overwhelm ▸ Parry ▸ Unleash**; Marshal is
the neutral that feeds it. **Unleash is the only stance that needs no read** — you can
always just swing; **Marshal, Overwhelm, and Parry require reading the foe** (Focus,
§1.8), so an Actor that hasn't read can only Unleash.

**WHY.** Three stances (Marshal/Unleash/Parry) leave a safe square: always-Parry
negates every Unleash with no downside. Overwhelm dissolves it — the throw beats
the block — so no stance is safe (Charter §2/§3: a non-degenerate, learnable
hidden-information game).

**GUARANTEES.**
- There is **no dominant stance**: always-Parry is punished by Overwhelm;
  not-parrying is punished by Unleash.
- Marshal carries no offense — it only banks and exposes — so escalation is always
  a real risk, never free.

**MANUAL.** *Marshal: ready and gather. Unleash: spend everything on a strike.
Overwhelm: punch through a guard. Parry: read the strike, negate it, and steal the
bank.*

### 1.3 Ends-on-strike

> **RESTORED — current in §1.0.** The interim charge duel tried Body-attrition (run until a
> Body hits 0); the four-card Clash **reverts to ends-on-strike**: a duel ends the instant a
> strike connects. Force is built during the non-connecting dance and spent on the one
> connecting blow (`base × 2^Force`); **Body persists across duels**, so a fight to the death
> is several short duels, not one long beat-count. The stance/Edge specifics below are
> superseded, but the *principle* — connection = end — is current.

**RULE.** A 0-Edge Unleash is still a strike. The duel **ends the instant any
Unleash or Overwhelm connects** (mutual included). The only committed attacks that
do **not** end it are a **parried Unleash** (negated and stolen — roles flip) and a
**whiffed Overwhelm** (no guard to break). All non-connecting pairings (both
Marshal, Marshal vs Parry, Marshal vs Overwhelm, Unleash vs Parry, Overwhelm vs
Overwhelm, Parry vs Parry) continue.

**WHY.** Because a base strike already ends it, the mind-game is **opt-in**: if
neither escalates, someone pokes and it's over fast; escalation is push-your-luck.
"Caught while charging" needs no special rule — you just take the hit.

**GUARANTEES.**
- Every duel has a finite, short expected length (a single throw, not a long
  dance).
- No bespoke "interrupt" rule is needed; connection = end.

### 1.4 The Parry steal — the comeback

**RULE.** Parry vs a real Unleash: the Unleash is negated and the Parrier **takes
the Unleasher's entire Edge**. If the Unleash had **0 Edge**, the Parry instead
earns **+1 Edge** (a parry always pays). An **Overwhelm is never stolen**.

**WHY.** The steal is the game's biggest comeback — the lead flips mid-duel — and
"a parry always pays" keeps Parry from ever being a dead move, without making it
safe (Overwhelm still beats it).

**GUARANTEES.**
- A parried Unleash transfers the bank rather than destroying it (the flip).
- Overwhelm's immunity to the steal is what keeps the steal-comeback from making
  Parry oppressive.

*(OPEN — number: does a Parry also deal counter-damage, and how much? Tuning, not
yet decided.)*

### 1.5 Edge scales the card's primary effect

> **SUPERSEDED by §1.0 (The Clash).** In-duel damage now scales by **Charges**
> (`power × 2^charges`, multiplicative) rather than Edge (`+1 per Edge`, linear). The
> separation it protects — the move is the prediction, the charge is the magnitude, the
> card never telegraphs the move — carries forward. (Breadth/Action cards outside a
> Clash are unchanged; §1.7/§3.)

**RULE.** Every card has one **primary effect** (its headline). Spending Edge
scales that effect at a uniform linear rate: **1 Edge = +1 of the primary effect in
its natural unit**, added on top of the card's base magnitude. The default unit is
a strike's **1 Edge = 1 damage**; each non-damage maneuver names its own per-Edge
unit (Sunder = armor pip, Disarm = a card, etc.). No card contains bespoke
Edge-handling logic.

**WHY.** One global rule means cards never "know about" Edge — `Card = what`,
`Stance = the prediction`, `Edge = how much` stay cleanly separated, so a card
never telegraphs the stance.

**GUARANTEES.**
- Adding a card never requires new Edge rules (data-only; no redeploy).
- The Stance (hidden) is decoupled from the card (visible).
- Toughness still gates the result and Power still sets the base Edge adds to (Edge
  is additive, not a bypass).
- A breadth (multi-target) action is unopposed (§1.8), so Edge never applies to it —
  only a *duel's* primary effect scales.

### 1.6 Termination backstop *(engine rule, not public)*

**RULE.** A duel ends the instant a strike connects (§1.3), and under blind, simultaneous
guessing one eventually does (someone misreads). As an **implementation backstop only**: if
**N consecutive beats pass with no strike connecting** *(appendix: e.g. 12)* — the purely
theoretical perfect-mutual-defense case — the duel **breaks off** (both disengage; the foe
still counts as engaged, so it does not also free-hit at round end). A creature whose
instinct drives a winnable duel to the backstop is a bug.

**WHY.** Ends-on-strike (§1.3) resolves real duels via accumulated misreads; the backstop
only guards the corner case where both fighters guess perfectly forever — never a pattern a
real player produces — so it adds no rule anyone meets in play.

**GUARANTEES.**
- The backstop is invisible in normal play and is **not** part of the public rules.
- Every duel terminates: it ends on a connecting strike, or breaks off after N no-connect beats.

### 1.7 Facing a crowd — K duels, two caps

> **SUPERSEDED by §3.** The breadth model is now §3.1/§3.2: **Tempo** = the duels you start
> (results stick), **Focus** = the duels started on you (a Focus-defense is **reset** —
> survival only, no damage to the attacker), a free-hit if uncovered, and a **Tempo
> counterattack** as the only way to damage an aggressor. The linear *god ≈ party* intent
> below carries forward; the Edge/Exposed specifics do not.

**RULE.** Engaging multiple foes is **K simultaneous pairwise duels** (or one
breadth-attack — see Coordination). Two separate per-Actor pools gate K:
**Speed/Tempo** caps how many you can sustain **offensively** (engaging each costs
the target's Speed); **Mind/Focus** caps how many you can **predict** (covering
each costs the attacker's Speed). When Speed affords **K** but Focus covers only
**J < K**, the **K − J** extra duels are **one-way**: you strike, but can't predict,
so those foes **free-hit** you. Going **negative in any one pool** (Tempo or Focus) marks you
**Exposed** table-wide for the round (§3.3) — Speed sets *whether* you can sustain a
duel, never the order duels resolve in.

**WHY.** Routes offense-at-scale through Speed and defense-at-scale through Mind so
neither one stat owns the whole table; makes the gank (overflow free-hits) the
natural counter to a thin Mind (Charter §4: asymmetry by design).

**GUARANTEES.**
- Edge resets per duel, so breadth never compounds (consistent with §1.1).
- "Negate many" stays even in *total* across builds but capped *per body* — the
  linearity invariant the god-vs-party budget depends on.

### 1.8 Duel detection — reading is the contest

> **PARTIALLY SUPERSEDED by §1.0 (The Clash).** The **in-duel read** described below —
> "Focus unlocks your stance menu; without a read you can only Unleash" — is gone: in
> the Clash all six moves are **standing**, so there is no Focus gate *inside* a duel.
> Focus is now purely the **breadth** resource — round-end coverage of foes you did not
> engage (§3.2). What carries forward unchanged: engaging costs **Tempo** (= the foe's
> Speed), an engaged foe does not also free-hit, breadth/self actions are unopposed, and
> a creature does not read you back (its instinct is its move, §7). Read the rest of this
> section for the breadth model; ignore its stance/Edge specifics.

**RULE.** Engaging a foe (Tempo) puts you in a **clash**, resolved by the stance mix-up
(§1.2). **Reading it (Focus) unlocks your stance menu:** with a read you have all four
stances; **without a read your only stance is Unleash** — a blind strike. The read, not
the swing, is what buys the *contest* (Parry, the throw, and Edge); a non-reader can only
swing, and it resolves through **the same duel** as everything else. So two non-readers
both Unleash — the **magnitude trade** (mutual base hits, no Edge) — and one side reading
the other is the **one-way duel** of §1.7: the reader works the full mix-up while the
blind side can only strike, so a blind swing is freely **parried** (§1.4). (Breadth and
self/ally actions read no one and stay unopposed; a foe you never engage that hits you is
a **free-hit**, §3.2.)

**WHY.** The mix-up and Edge only mean anything when you are *reading* — anticipating a
foe so you can Parry, throw, or bank. So a non-reader keeps exactly the one stance that
needs no anticipation (Unleash) and loses the three that do. Making the read the single
switch ties the whole contest to the resource that is *about* prediction, keeps Edge the
price of a contested exchange (§1.1), and **folds the old "trade" into the duel** — a
blind swing is just an Unleash, resolved by the same machinery and freely parried by
anyone reading you — so there is one resolution path, not two.

**GUARANTEES.**
- No Edge accrues without a read — you cannot Marshal without reading, so riskless
  hoarding is structurally impossible (consistent with §1.1).
- Unleash is the only read-free stance; Marshal, Overwhelm, and Parry each require the
  read (§1.2). A blind swing is therefore exploitable — a reader simply Parries it.
- Defense is never free, but its price is **Focus**, not your action: you may act
  (Tempo) *and* read attackers (Focus), yet Focus is capped by Mind, so you cannot read
  everyone — the overflow free-hits you (§3.2). Offense and defense are separate pools
  that meet only at overextension (§3.3).
- A breadth action (one action, many targets) is never a duel — you cannot read a crowd
  — so it is always unopposed (consistent with §1.5).
- A creature need not read you back: its instinct is its stance (§7). The duel is on the
  side that reads; the unread side is §1.7's one-way free-hit.

**MANUAL.** *Engage to clash (Tempo). Read the foe (Focus) to unlock Parry, the throw,
and banking — without a read you can only Unleash. No read, no contest: a blind swing is
freely parried.*

### 1.9 Resolution order — engagement first, attacks before buffs

**RULE.** When several actions resolve in one exchange, they resolve in **descending
engagement** (= descending tempo at stake), in three tiers:
1. **Duels** (reads, §1.8) — RPS, Edge, and their damage settle first.
2. **Uncontested attacks** — incoming strikes no one contested: the undefended blow
   and §1.7's Focus-overflow free-hits.
3. **Self / ally effects** — buffs, heals, and other non-engaging state changes.

Thus **attacks resolve before buffs**: a self-effect cannot negate a blow already
incoming this exchange; it takes hold from the next exchange on. Within a tier order
is immaterial: in the single-deck core all modifiers (attachments) compose **commutatively** (§5),
so nothing is order-dependent. *(The order-dependent **modifier** card-kind is deferred —
`future-possibilities.md`; were it to return, its on-target conflicts would resolve in a **fixed
seat order**, Speed playing no part in timing, §3.1.)* Resolution is fully deterministic.

Within a tier, **resolution order is immaterial by construction**, not by luck. Three
things make it so: each duel's rolls come from a **per-duel keyed RNG** (independent of
when it resolves); damage **accumulates commutatively** (a fixed set of strikes sums to
the same result regardless of the order applied); and **no actor is removed mid-tier** —
a Body reaching 0 is *mortally wounded*, and downs are **finalized only at the tier
boundary** (§1.3: it still lands every blow it committed). Permuting the seat order of a
tier's duels must therefore yield the identical end-state — a built-in property test;
any divergence is an order-dependent mechanic, i.e. a bug. Effects that would depend on
a **sibling duel's outcome** are disallowed within a tier — push them to the next tier
or exchange.

**WHY.** Ordering by engagement settles the contested, tempo-spending core (the
duels) before its consequences, and dissolves the buff-timing paradox with no new
system: you cannot retroactively dodge an in-flight attack by buffing, because the
attack is more engaged and resolves first. Resolving the lone intra-tier collision by
a fixed seat order keeps Speed out of timing entirely (§3.1) and guarantees
determinism without manufacturing a contest the design does not need.

**GUARANTEES.**
- Resolution is total and deterministic given the seed — no real-time, no unresolved
  tie.
- Defense is anticipatory, not reactive: a buff played into an incoming attack does
  not save you from it (human-confirmed intent).
- Speed never affects resolution order: every effect is order-independent (modifiers compose
  commutatively, §5; the deferred order-dependent modifier would use a fixed seat key, not Speed).
- Intra-tier resolution is order-independent by construction (keyed RNG + commutative
  damage + boundary down-checks): an Actor in K duels takes the **sum** of the blows,
  its fall decided by the total at the boundary, and — per §1.3 — it still lands every
  blow it committed. Only the cross-tier order (attacks before buffs) matters.

> **Worked example — the rock and the buff.** A foe throws a rock at me; I spend my
> action buffing myself. The buff does not contest the foe, so this is **not** a duel
> (§1.8): two unopposed actions, no Edge. They resolve by tier (§1.9) — the rock is an
> uncontested attack (tier 2), the buff a self-effect (tier 3) — so **I take the
> rock**, then the buff takes hold; even a buff granting rock-immunity is too late for
> the blow already thrown. To *avoid* the rock I would **read** the thrower — spend
> Focus to Parry it — which costs Focus, not my action, so I could buff **and** read if
> my Focus affords it. Defense is a separate pool, not a forfeited turn.

---

## 2. Defense model — *cut → bar → pool* 🟡

Design source: [`notes/form-and-defeat.md`](../notes/form-and-defeat.md),
[`notes/stats.md`](../notes/stats.md). Seeded below; not yet exhaustive.

> **Naming.** The defensive dimensions — **Body · Spirit** — are the **channels** (outer physical,
> inner fear/will). *(The **Mind / Confusion** channel was **removed** 2026-06-20 with the Tempo/Focus
> merge — see §3.2.)* The word *aspect* is **reserved** for the deferred deck-chord combo layer (§6) and
> is **not** used for these. *(The frozen `notes/` still call the channels "aspects" — read that as
> "channels".)*

### 2.1 One maintained meter

**RULE.** Exactly **one** quantity is a maintained, depleting track: the **Body
Health pool** (face-down cards, per-combat, restored on a win). Every other
defensive quantity — **Armor, Ward, Toughness, Resolve** — is a
**passive stat read off the table**, never spent. **Tempo** is an
ephemeral per-round pool, re-derived each round, not maintained.

**WHY.** "You maintain exactly one meter" is the load-bearing comprehensibility
rule (Charter §7, §9): a human can hold the whole game because only one number is
ever in flux.

**GUARANTEES.**
- Nothing besides Body Health is ever "tracked" between rounds.
- Every other defensive number can be reconstructed from the cards on the table.

### 2.2 Every channel is cut → bar, and only Body has a pool

**RULE.** Each attack is **outer** (physical/elemental → Body) or **inner**
(fear → Spirit). It resolves: **subtract the cut** (Armor for outer, Ward for
inner; per source, typed, never depletes) → **accumulate the remainder into the
round's pile** → **compare the pile to the bar** (Toughness for Body, Resolve for
Spirit). Only the **outer** channel has a **pool** (Health cards) behind the bar;
the **inner** (fear) channel **breaks** when the pile exceeds the bar, with no
pool. Cross-immunity: outer ignores Ward; inner ignores Armor.

**WHY.** A per-source cut answers *many small hits*; a high bar answers *any one
big hit* — non-redundant, so you want both. Typing the cut makes "called shots"
fall out for free (choosing a damage type chooses which channel you attack).

**GUARANTEES.**
- Both channels are structurally parallel (offense · cut · bar · [pool]); only Body has a pool.
- An inner break is a this-round event that clears at round end — **except**
  scared-to-death, the one inner result that bleeds into the Body pool.
- Accumulation is always cards in a zone, never a number in the head.

*(SEEDED — the damage formula, scaling, and the Resolve break threshold are not
yet specced. Numbers live in `booklet.ron`.)*

### 2.3 Stats live in the deck — *stats-as-deck*

**RULE.** A character has **no stats on its identity card** — it is a **bare Actor** (a name and a
map token, §8.1). **Every stat is read off its deck**, from the **Form** zone (§5.2): a
**fundamental card** sets the base, **attachment** cards modify each dimension. So §2.1's "passive
stats read off the table" — **Armor, Ward, Toughness, Resolve**, and likewise **Speed, Drive, Power** — are
**Form-derived**, never authored fields. The **Body Health pool** is a **count × value** Form pool
(Body × Toughness); the **Tempo pool** is **Speed × Drive** — Speed-many cards each worth Drive (§5.5).

**WHY.** "The deck *is* the character" (#8) made literal: it removes a redundant authored stat-block,
so *getting stronger = adding cards* (the Upgrade economy, §8.3), and **§2.1's "read it off the
table" now extends from defense to every stat**. Clean slate = a bare Actor with a minimal Form;
specialization = accreted attachments (§8.5).

**GUARANTEES.**
- No stat exists except as a Form card on the table — nothing is authored on the Actor.
- The §3 / §4 economy is unchanged in behavior; only the **source** of a stat moved (card → deck).
- A bought Upgrade is a Form attachment (or an Action card, §5) — power grows by cards.

---

## 3. Speed · Drive · Tempo — *the breadth budget* 🟡

Design source: [`notes/speed-and-tempo.md`](../notes/speed-and-tempo.md).

> **Locked 2026-06-20.** The breadth economy is the three terms below, ratified together. Earlier forms
> (two pools Tempo/Focus; a per-target-Speed cost; a value-less Tempo) are superseded — see the §3.2–3.4
> history banners. This section is the **single canonical home** for what Speed, Drive, and Tempo are;
> any change that makes one of these three words do another's job has broken the concept (the GUARANTEES
> are the tripwires).

Two permanent **Form** stats size one round-scoped **pool of cards** — the same shape as Health
(Body × Toughness → Health):

- **Speed** — *count*: how many **Tempo** cards you start each combat round with.
- **Drive** — *grade*: the magnitude printed on each of those cards.
- **Tempo** — the *pool*: Speed-many cards, each worth Drive, flipped face-down to spend and rebuilt
  fresh each round. **Spent cards stay spent for the whole round.**

### 3.1 What Tempo and Drive do

**RULE.** **Flipping a Tempo card gates every action** — intercept, engage, strike, strike back. Run
out of face-up Tempo and you can do nothing more this round (so what you pour into the gauntlet is gone
before the exchanges).

**Drive's magnitude does real work in exactly one place — a gauntlet *crossing* (§4).** When a runner
tries to slip past an interceptor, the two hold an **open, escalating Drive auction**: each may keep
flipping Tempo cards, adding that card's Drive to its committed total for *this* crossing; either may
stop; **the higher total wins, ties to the catcher** (caught → stopped; runner strictly higher → slips
by). The committed total **resets at the next opponent**, but the spent cards **do not** return — so
catching or slipping past *more* foes, or *harder* ones, drains more Tempo. **Catching a runner is the
same as engaging it** — the cards spent to catch it pay for the exchange; you never pay twice.

**Everywhere else, Drive's number is irrelevant — only the flip counts.** An **exchange** (a strike) is
**single-card**: flip *one* Tempo card to strike, and the blow is the same whatever the card's Drive
(Drive sizes a crossing, never a blow). An enemy can only attack you by **spending a Tempo card**, and
that attack is Drive-independent. You may **reflexively strike back** at anyone who strikes you in melee
(position is irrelevant — they came to you), but striking back is an action: it costs **one** Tempo
card, and with none to flip you simply **take the hit** (a free hit). *(Ranged fire is one-sided — you
cannot strike back at a Reserve that did not come to you — except in the no-charge open brawl, where two
ranged foes exchange if both spend a card, or where a card grants an exception, §4.2.)*

**WHY.** One pool for act-and-defend makes the cannon/wall axis a live **allocation** (spend it
attacking and you cannot answer a skirmisher) rather than a second stat. Splitting the pool into
**count (Speed)** and **grade (Drive)** gives two clean power dimensions that mean different things:
**Speed = how many crossings/actions you get; Drive = how cheaply you win each crossing.** Confining
Drive to the crossing keeps a strike's force on Power (not on how hard you shoved through the line), and
the per-round depletion is the tension — run the gauntlet hard and you are spent for the exchanges (#2
opportunity cost).

**GUARANTEES.** *(the tripwires — break one and the concept no longer holds)*
- **Speed = count**, **Drive = grade** — both permanent Form stats, never spent; **Tempo = the cards**,
  Speed-many at Drive each, spent within a round and refreshed between rounds.
- **Drive's magnitude affects only a gauntlet crossing** (an escalating auction, ties to the catcher);
  it never scales a strike, an attack, or anything outside a crossing.
- **Every action is one Tempo card** (intercept, engage, strike, strike back); **catching = engaging**
  on the same cards; an exchange is single-card and Drive-blind.
- **Spent Tempo does not return until the round refresh** — gauntlet spending is unavailable for later
  exchanges (the depletion tension); a *crossing's committed total* resets per opponent, the *pool*
  does not.
- **Reflexive strike-back is always available against a melee attacker** for one Tempo card; no card → a
  free hit. Ranged is one-sided save the no-charge open brawl / a card exception.

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Speed` (Resources) — A permanent Form stat: how many **Tempo** cards you start each combat round with (the *count*). It is not a magnitude of movement and never sets turn order.
- **TERM.** `Drive` (Resources) — A permanent Form stat: the magnitude on each **Tempo** card (the *grade*). Its number matters in exactly one place — a gauntlet crossing, where both sides flip Tempo cards and the higher committed Drive wins (ties to the catcher). A strike is the same whatever its Drive.
- **TERM.** `Tempo` (Resources) — The round's pool of action cards: **Speed**-many, each worth **Drive**. Flip one to take any action (intercept, engage, strike, strike back); spent cards stay spent until the round refreshes. Run out and you can't act.

### 3.2 Focus — *merged into Tempo (2026-06-20)*

> **MERGED.** Focus is no longer a separate pool. Defense-in-place — turning an incoming melee blow into
> a **clash** rather than a **free hit** (§4 Skirmish) — is now **paid from Tempo** (§3.1). The **Mind**
> stat and the separate Focus pool are **removed**; the cannon/wall split becomes a Tempo *allocation*
> (spend it all attacking and you cannot answer a skirmisher). The old separate-defense-pool rules
> (defense resets the attacker; per-target Focus cost) retire with it. *(Original text in git history.)*

### 3.3 Overextension — *removed*

> **REMOVED.** The old **Exposed / Focus→0** penalty (overextending Tempo zeroed your Focus)
> is gone. Tempo and Focus are **independent** breadth pools, each hard-capped by its stat,
> and the offense/defense balance now lives entirely in the **Speed-vs-Mind split** — a
> high-Speed/low-Mind fighter natively attacks widely but defends poorly, and the reverse —
> so no coupling penalty is needed. **Pay-after is kept** (§3.1): the action that drives a
> pool negative still happens and is your last, but it carries **no extra penalty**.

### 3.4 The round — orchestration (PvE & PvP)

> **SUPERSEDED by §4 (charge-and-gauntlet).** The round is no longer a player-phase/foe-phase loop over
> formation; it is the **Charge → Muster → Gauntlet → Skirmish → Reserve** model in §4. **Tempo is now the single
> currency** (Focus/Mind merged out, 2026-06-20); order-independence is preserved *per phase*. The
> PvE/PvP text below (and its Focus-defend modes) is kept for design history; where it conflicts with
> §4, §4 wins.

**RULE.** Combat is a sequence of **rounds**. Two orchestrations share the same duel
resolver (§1.0), economy (§3.1–3.2), and formation/reach layer (§4); which runs depends on
whether **both** sides are player-controlled.

**PvE round** — player heroes (multi-action) vs instinct creatures (one-action, §7):
1. **Formation** *(public, §4)* — front/back visible; heroes may shift freely.
2. **Player phase** — each hero spends **Tempo** to **engage** reachable foes (cost = the
   foe's Speed). Each engagement is a **mutual** Clash (results stick: the hero can kill, the
   foe can hit back, the trade is live). An engaged foe **spends its one action defending**,
   so it does **not** also attack this round (engaging neutralizes its attack).
3. **Foe phase** — every **un-engaged** living creature attacks a reachable hero (by its
   target rule, §7). The attacked hero picks a **defense mode**: **Focus-defend** (Focus →
   attacker reset, survive only), **counterattack** (Tempo → mutual, can kill, trade live), or
   **eat the free hit** (base damage, no Force). A foe neither engaged nor covered free-hits.
4. **Refresh** — downs finalize at the boundary (§1.9); survivors reset Tempo/Focus; **Body
   persists**; round++.

**PvP round** — both sides player-controlled (multi-action, so no "engage neutralizes"):
1. **Formation** *(public, §4)* — visible; free shift.
2. **Targeting** — *simultaneous hidden commit → reveal.* Each actor allocates Tempo to
   reach-legal engagements. Reveal the engagement graph; mutual engagements (A→B **and** B→A)
   **merge** into one mutual Clash.
3. **Defense** — *simultaneous hidden commit → reveal.* Each actor under a one-sided attack
   picks its mode (Focus-defend / counterattack / eat) per attacker, from remaining
   Focus/Tempo. **Modes are public on reveal.**
4. **Combat** — all live duels resolve in **lockstep beats** (each beat: every duelist commits
   hidden, all reveal together, all resolve; ended duels drop out), to ends-on-strike.
5. **Refresh** — as PvE.

**WHY.** PvE's asymmetry (multi-action heroes vs one-action creatures) lets the proactive
player phase **use up** a foe's single action by engaging it — a simple, readable proactive→
reactive flow. PvP can't: both sides are multi-action (everyone attacks *and* defends) and
neither may reveal first, so targeting must be **simultaneous**. Splitting **decisions**
(targeting, defense) from **resolution** (combat) is what makes order irrelevant within every
phase.

**GUARANTEES.**
- **Order-independent within each phase:** every targeting/defense decision is committed before
  any duel resolves; duels are independent (no cross-duel effects, §1.9); downs finalize at the
  boundary — so resolving duels in any order yields the identical end-state.
- **No turn order:** one whole side then the other (PvE), or both at once (PvP); Speed sizes
  pools and costs, never initiative (§3.1).
- **One engine:** both orchestrations call the identical Clash and economy; only the round
  skeleton differs, justified by one-action creatures vs multi-action players.

---

## 4. The battle — the charge & the gauntlet 🟡

> **History.** This section has been through several superseded forms: a **front-line / back-line**
> formation, a **speed-pairing** commitment order, then a **lane-based** commitment order (assign
> Vanguard to lanes, stack, hold/slip). All are replaced by the **charge-and-gauntlet** model below.
> The *spine survives* — three roles (Vanguard / Skirmisher / Reserve), hidden commitment, "the front
> protects the back" — but **lanes are gone**: instead of assigning Vanguard to abstract lanes, both
> sides **secretly declare a charge** (who runs in, and in what order), then the two charge-columns
> **thread through each other in a single open gauntlet**. The roles now *emerge* from the gauntlet
> rather than being pre-assigned. The motivation: lanes were the one construct with no physical
> picture, and the "lead holder absorbs all" abstraction produced wasted overkill; the gauntlet makes
> targeting explicit and rides one clean metaphor. **The code now implements this** — the gauntlet
> resolver, the **Muster** window (below), the role passives, and the Controller status effects are
> all live. Old text is in git history (`role-card-redesign` and the combat-redesign commit record the
> rationale).

**The budget (one pool).** **Tempo** is the action economy that gates everything (§3): a `count × value`
pool of **Speed**-many cards, each worth **Drive**, flipped to spend (face-up = available, face-down =
spent, §5). **Flip a card to act** — charge, slip, intercept, strike, fire, *or* defend (turn an
incoming melee blow into a **clash** rather than a **free hit**, §4.2). **Run out and you can't act.**
**Tempo refreshes each round.** **Drive's magnitude matters in exactly one place — a gauntlet crossing**
(an escalating Drive auction, §3 / below); every other action is one Drive-blind card.

Because the **same pool pays for offense *and* defense**, the **cannon/wall axis is an allocation
choice**, not a second stat: pour Tempo into the gauntlet (charging through, or catching runners) and
you have little left to strike or answer a skirmisher; hold it back and you survive but do less. (There
is **no separate Focus/Mind pool** — **merged 2026-06-20**; defending is a Tempo spend.) Both pools share
the **`count × value`** form: **Health = Body × Toughness** (value gates damage, persists), **Tempo =
Speed × Drive** (value bites in a crossing, refreshes).

**RULE.** The three roles **emerge from a charge**:
- **Reserve** — anyone who does **not** charge (holds back; the ranged / support line).
- **Vanguard** — a charger who **stops** at the front (intercepts, or is intercepted) — the melee
  front line.
- **Skirmisher** — a charger who **runs the gauntlet all the way through** to the enemy backfield.

The spine is still the counter-triangle **Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard**: a Vanguard
intercepts chargers in the gauntlet (stops Skirmishers); a Skirmisher who broke through reaches the
otherwise-untouchable Reserve; the Reserve fires on the exposed front from safety.

A round runs **five phases** (the Charge and Muster commits, then three open resolutions):

1. **Charge** *(the one hidden, simultaneous commit).* Each side secretly picks **which Actors charge,
   and in what order** (a face-down ordered column), then **both reveal at once**. Non-chargers are
   **Reserve**. *(Hidden because open ordering would be degenerate — the second mover would just
   reorder to counter. Everything after the reveal is open information.)*
1b. **Muster** *(open, before the gauntlet).* Each side plays its **standing / persistent** cards —
   cards whose effect **lasts the round** rather than resolving on a single target after the gauntlet.
   This is the window for a charging **Wall**'s defenses (Brace, Last Stand, Rally), a **Controller**'s
   debuffs (Slow, Confuse, Dread, and the Stagger / Shove / Disarm riders), and a **Support**'s buffs
   (Mend, Ward, Haste, Steel). Mustered effects are in force **for the gauntlet and the rest of the
   round** — a Slow shrinks a foe's Tempo *before* it crosses; a Stagger costs it its whole turn; a
   Brace banks Tempo for the crossing. The positional **attack** cards (Infiltrator slips, Artillery
   fire) are **not** mustered — they need a post-gauntlet target and wait for their own phase. *(Why
   here: a debuff played in the last phase is wasted — its target has already acted. Muster is the
   point where a persistent effect can actually shape the round.)*
2. **The Gauntlet** *(open).* The two charge-columns **thread through each other**. Resolution is a
   public sequence of **crossings** (see below); chargers who **stop** become **Vanguard**, chargers
   who **break all the way through** become **Skirmishers**. Damage **accumulates** and resolves at the
   phase boundary (§2 card-state: health cards turn face down; *all* face down → defeated).
3. **Skirmish** *(open).* Skirmishers who broke through **and still hold a face-up Tempo card** strike
   the enemy **Reserve** — choosing targets freely (opposing Skirmishers can no longer intercept; their
   chance was the gauntlet). Each strike spends **Tempo**; a Skirmisher with more may keep striking,
   switching targets. The defender answers by **spending Tempo** — flip a Tempo card to turn the blow
   into a **clash** (defend *and* counter); **no Tempo to spare → a free hit**. With the enemy Reserve
   cleared and Tempo left, Skirmishers may turn on the enemy **Vanguard** (who likewise defend by
   spending Tempo). Resolve damage at the phase boundary.
4. **Reserve** *(open, one-sided).* Surviving Reserve spend **Tempo** to fire **ranged** at the enemy
   **Vanguard or Skirmishers** — **never** the enemy Reserve (out of range). Ranged fire is **one-sided**
   (no counter) unless a special ability or **no enemy Vanguard** opens range-on-range, which makes it a
   normal **exchange** (§4.2). A Reserve that **spent Tempo defending** in the Skirmish phase has
   **less left to fire** — one pool pays for both, so survival and firepower compete (the core Reserve
   squeeze). Resolve damage; then **Refresh** (Tempo resets, Body persists, round++).

**A crossing (the gauntlet's atom).** When two opposing chargers meet, they hold an **open, escalating
Drive auction** (§3): each may **keep flipping Tempo cards**, adding each card's **Drive** to its
committed total *for this crossing*; either may stop. **Higher total Drive wins, ties to the catcher.**
- **Caught** (interceptor's committed Drive ≥ the runner's) → the runner **stops** here; the catch
  **engages** them (the cards already spent pay for the exchange — no second cost). Both become Vanguard.
- **Slips** (runner strictly higher) → the runner **advances to the next** enemy in the column.
- **Barge** (spend nothing, take a **free hit**, keep moving) → the hit lands **before** the next
  crossing, then the charger advances.

A unit's Tempo is **one pool** spent across **all** its crossings — there is no fixed "blocker": it may
let one enemy slip by (declining to spend) and still spend remaining Tempo to catch the next, **choosing
whom to prioritize**. The committed total **resets each crossing**, but **spent cards do not return** —
so Tempo poured into the gauntlet is gone before the exchanges (§3 depletion). A unit that **stopped**
stays at its position and **remains an obstacle** for later advancers (Tempo permitting); a unit that
**broke through** the whole opposing column becomes a **Skirmisher**.

**Advancing vs catching (the Wall's hold).** A crossing weighs two distinct uses of Drive: **advance
Drive** (the grade you commit to *slip past*) against the other's **catch Drive** (the grade it commits
to *hold the line*). You slip iff your advance **strictly exceeds** their catch; a tie is held. For a
plain unit the two are equal (its Drive), so the raw "higher slips, tie caught" rule stands. **Wall
powers feed catch only, never advance** — a Wall raises the wall it makes the enemy climb without
itself slipping through on a big number (it is an immovable line, not a runner). This is the seam the
role passives plug into.

**Role powers in the gauntlet (and the round).** The specialist passives are not flavor — each bites a
concrete step:
- **Phalanx** (Wall) — +catch Drive: Walls who stop together intercept as one, holding faster runners.
- **Bodyguard** (Wall) — a stopped Wall steps across to intercept a **surplus** enemy charger that
  would otherwise break through unopposed (guarding the backfield, not just the foe it met).
- **Taunt** (Wall) — draws fire: the Wall is pulled to the **front** of its column so the enemy meets
  it first, sparing the rest of the line.
- **Blitz** (Infiltrator) — the **first slip each round is free** (costs no Tempo).
- **Shadowstep** (Infiltrator) — **win the tie** when slipping (advance ≥ catch, not just >).
- **Backstab / Assassinate** (Infiltrator) — a Skirmisher hits an enemy **Reserve** harder / executes
  it outright (the §10 prize for breaking through).

**Persistent status (Controller debuffs).** A Controller card can hang a **round-scoped status** on a
foe, cleared at Refresh: **Stagger** (loses its action — no strike, card, or strike-back this round),
**Shove** (knocked out of melee — cannot contest a melee blow), **Disarm** (hand fouled — cannot play
its role cards). Played at **Muster**, these degrade the foe's whole round (its gauntlet, its strikes,
its defense); their bite is timing — see the Muster phase.

**Targeting matrix.**

| Chooser        | May target                                                                  |
| -------------- | --------------------------------------------------------------------------- |
| **Vanguard**   | the enemy charger(s) it **meets in the gauntlet**                           |
| **Skirmisher** | the enemy **Reserve** (first), then the enemy **Vanguard** — its prize      |
| **Reserve**    | enemy **Vanguard & Skirmishers**, and **aid own allies** — **never** enemy Reserve |

**No chargers / one side charges.**

- **One side charges, the other holds all back** — the holding side has **no Vanguard**, so its whole
  line is Reserve; the chargers meet **no opposition** in the gauntlet, break through as **Skirmishers**,
  and raid that Reserve. Holding everyone back only **exposes** you (your own Reserve has no front), it
  never reaches the enemy Reserve — there is no charge to break *their* line.
- **Neither side charges** — no front forms anywhere, so the privilege "**Reserve is safe from enemy
  Reserve**" (paid for by fielding a front) **lifts**: it's an **open brawl** — everyone may target
  anyone with whatever range they carry.

**In-round protection is the gauntlet's alone.** Only **intercepting in the gauntlet** can save a
Reserve **this round** — it stops the raider before it becomes a Skirmisher. A later Reserve→Skirmisher
shot is **attrition**: it denies the raider *next* round, it doesn't shield the target *this* one.

**Confluence (order-independence, honestly stated).** The gauntlet is resolved as a sequence, but it is
**confluent**: where two chargers slip past each other, resolving either one's continuation first gives
the **same** result. Order matters **only at a stop** (an interception changes who meets whom next), and
a stop's outcome is **fully determined** by the revealed Tempo. So the phase is "as order-independent as
the physics allows" — deterministic given the charge commitments (preserving §0.1 / #11), just not
freely reorderable across a block. Damage from a barge lands **before** the next crossing (sequential
within one charger's run), but across *independent* runs the order is free.

**What is hidden.** Only the **Charge** (who charges, in what order) is hidden, and only until its
simultaneous reveal. **Everything after is open** — Tempo cards are flipped *face-up* to spend, in view.
The deeper hidden, simultaneous mind-game (a true 1v1 crossing) lives in the optional **Clash module**
(§4.2 / §1.0), layered on top — it is **not** baked into the base gauntlet, which stays
perfect-information and deterministic (#11). Always public: stats (Speed / Body) and the spent/unspent
Tempo pool.

**WHY.** The triangle survives but its mechanics get a **single physical picture**: two lines charging
through each other, choosing at each crossing to stop a runner or push past. Removing lanes removes the
only construct with no metaphor and the "lead holder eats every blow" abstraction (which wasted attacks
as overkill) — targeting is now **explicit** (you choose whom to stop, whom to strike). **Tempo as one
pool** spent on advancing, intercepting, *and defending* makes the core decision crisp — *where do I
spend my initiative?* — and turns the cannon/wall axis into a live allocation (spend it all attacking
and you can't answer a skirmisher; hold it and you survive but do less). **Hiding only the charge** keeps the prediction
game where it must be (you commit blind to their charge) while leaving resolution **open and
computable** — the bluff layer is the Clash's job, not the gauntlet's. "**The front protects the
back**" stays load-bearing: the only route to the enemy Reserve is breaking their charge line, so to
threaten their back you must expose your front.

**GUARANTEES.**
- **The role triangle holds:** Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard; roles **emerge from the
  charge** (Reserve = didn't charge, Vanguard = stopped, Skirmisher = broke through).
- **The Reserve is reachable only by breaking the charge line** — never by enemy Reserve, only by a
  Skirmisher who ran the gauntlet through — *except* the no-charge open brawl.
- **One hidden commit:** only the Charge (units + order) is hidden+simultaneous; **all resolution is
  open**. Base combat is **perfect-information and deterministic** (#11); the hidden mind-game is the
  optional Clash.
- **Confluent resolution:** deterministic given the charge; order-independent except across an
  interception, whose outcome is itself determined.
- **Two pools, both `count × value`:** **Health** (Body × Toughness, persists) and **Tempo** (Speed ×
  Drive, refreshes). **Tempo pays for offense *and* defense** (charge / slip / intercept / skirmish /
  fire, *and* answering a melee blow), so the cannon/wall axis is an allocation choice, not a separate
  stat. **Drive's magnitude bites only in a gauntlet crossing** (§3); a strike is Drive-blind. (No
  Focus/Mind pool — merged.)

**MANUAL.** *Secretly pick who charges and in what order (an ordered face-down column); reveal together.
Non-chargers are your Reserve. **Muster**: before the gauntlet, play your standing cards — a charging
Wall braces or sets a Last Stand, a Controller slows / staggers / disarms the enemy, a Support mends or
hastes the line — these last the whole round and shape what follows. The two charge-columns thread
through each other: at each meeting, both
keep flipping Tempo cards into a **Drive auction** — higher total Drive wins, a tie is caught; or spend
nothing and take a free hit to barge on. Your Tempo is one pool across all your crossings — choose whom
to catch (and Tempo spent here is gone for the exchanges). Those who break all the way through are Skirmishers; those who
stop are your Vanguard. Skirmishers with Tempo left strike the enemy Reserve (it defends by spending
Tempo to make the blow a clash; no Tempo to spare = a free hit), then the enemy Vanguard. Finally your
Reserve fire ranged at the enemy front (never their Reserve) — but Tempo spent defending is Tempo you
can't fire with. No one charges on either side → open brawl.*

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Charge` (Roles) — The hidden, simultaneous declaration of who runs in and in what order. Revealed together; everything after resolves in the open. Non-chargers are the Reserve.
- **TERM.** `Vanguard` (Roles) — A charger who stops at the front — by intercepting, or being intercepted. The melee front line; it strikes first and shields the Reserve.
- **TERM.** `Skirmisher` (Roles) — A charger who runs the gauntlet all the way through to the enemy backfield. The only route to the enemy Reserve; acts after the gauntlet resolves.
- **TERM.** `Reserve` (Roles) — Anyone who did not charge: decisive but fragile (artillery, support). Fires ranged on the enemy front and aids allies, but can never target the enemy Reserve.
- **TERM.** `The triangle` (Roles) — Vanguard beats Skirmisher (intercepts it in the gauntlet); Skirmisher beats Reserve (breaks through to assassinate); Reserve beats Vanguard (fires from safety, untouchable in melee).
- **TERM.** `Muster` (Combat) — The open window after the charge reveal and before the gauntlet, where each side plays its **standing / persistent** cards (Wall defenses, Controller debuffs, Support buffs). Mustered effects last the round and shape the gauntlet; positional attack cards wait for their own phase.
- **TERM.** `The gauntlet` (Combat) — The open phase where the two charge-columns thread through each other; at each crossing a unit spends Tempo to stop the enemy or push past. A unit slips iff its advance Drive exceeds the other's catch Drive (Wall powers raise catch only). Breakthroughs become Skirmishers; those who stop become Vanguard.
- **TERM.** `Slip` (Combat) — At a crossing, push past an enemy: an open Drive auction where both flip Tempo cards and the higher committed Drive wins — you need strictly more than the catcher to slip; a tie is caught. Spend nothing and you barge past taking a free hit.
- **TERM.** `Open brawl` (Combat) — If neither side charges, no front forms and the Reserve's safety lifts: everyone may target anyone with whatever range they carry.
- **TERM.** `Phases` (Round) — Charge (hidden: who runs in, and in what order) → Muster (play standing/persistent cards) → Gauntlet (the columns thread through) → Skirmish (breakthroughs hit the enemy Reserve) → Reserve (ranged fire on the front) → Refresh. Confluent within each phase.

**Still unspecified (open dials — pin before/with implementation).** The structure (charge, gauntlet,
crossings, the three emergent roles, phases, targeting) is settled; these are not:

> **RATIFIED as the resolver-of-record (2026-06-20).** For the balance work (§0.3 — par is
> **policy-relative** to a fixed resolver), the **v1 code semantics are the canonical combat resolver**
> until a measured problem forces a change: the **single-card crossing** (advance Drive vs catch Drive,
> one Tempo flipped per side, Phalanx/Shadowstep/Blitz riders), **index-pairing** of the charge columns
> (after the Taunt sort), and **no multi-intercept** (dials 1–3 below). This keeps base combat
> **deterministic** (the solver stays a maximizer, not an equilibrium-solver) so par is well-defined and
> measurable now. The dials below are **candidate enrichments**, to be pinned *in response to* a balance
> property the resolver-of-record cannot satisfy by tuning — not before. (In particular: do **not** pin
> the auction as hidden-simultaneous unless forced — that is the one change that would make even PvE
> combat a game-theoretic sub-game.)

1. **Crossing numbers / the auction** — the rule is locked (§3: an **escalating Drive auction**, **ties
   to the catcher**, **catching = engaging** on the same cards). **Code implements the v1 single-card
   crossing** (each side flips one Tempo card; advance Drive vs catch Drive, Phalanx/Shadowstep/Blitz
   riders live) — the full **escalating** auction (both sides keep flipping to outbid) is the remaining
   enrichment, along with the **free-hit** magnitude when barging.
2. **Multi-intercept caps** — a stopped unit can intercept later advancers while Tempo lasts; is there a
   cap, or is it purely Tempo-bounded?
3. **Charge order semantics** — confirm the column is strictly front-to-back (an advancer meets the
   enemy column in their charge order) and that a stopped unit holds its column slot.
4. **Skirmish strike cost** — one Tempo per strike (assumed); confirm, and whether switching targets
   costs extra.
5. **Reserve aid kit** — the buffs / heals / debuffs Reserve deliver — Action cards over the §5 zone
   layer (the aspect/combo layer is deferred — `future-possibilities.md`).
6. **Pool model — locked (§3, 2026-06-20).** Two `count × value` pools: **Health = Body × Toughness**
   (value gates damage; persists) and **Tempo = Speed × Drive** (refreshes). **Focus and Mind are
   removed.** Speed = count (how many Tempo cards), **Drive = grade** (per-card magnitude), Tempo = the
   cards. **Drive's magnitude bites only in a gauntlet crossing** (an escalating Drive auction, ties to
   the catcher); every other action is one Drive-blind card. *(The earlier "no Tempo value" call is
   reversed — Drive is the grade, ratified with Speed/Tempo in §3.)*

*(Range/attack dials are resolved by §4.2: "Reserve self-defense" = whether it carries melee; "strike
shape" = a Clash when attacker and target share the range, an auto-hit when they don't.)*

### 4.1 Count-adaptivity — the system degrades to the choices that exist

**RULE.** The commitment layer is **count-adaptive**: any choice with a **single legal option
resolves automatically**, presenting no decision. The charge declaration, the gauntlet crossings,
and Skirmisher/Reserve targeting appear only when party size makes more than one option legal.
Concretely:

- **1 v 1** — each side has one Actor; the only non-degenerate line is to charge it, so the two
  meet in a **single crossing** and fight a **trade** (or a **Clash** with the module on). No charge
  bluff (one unit, one order), no meaningful slip (slipping just delays the same fight), no Reserve,
  no Skirmisher — it is exactly the plain duel (the tutorial case).
- **Small parties (2–3)** — only live choices surface: the **charge declaration** (who runs in, in
  what order) becomes a real choice once you have ≥2 chargers; **stop-or-pass** at a crossing only
  where both are affordable; **Reserve targeting** only when you have a surviving Reserve and a legal
  target.
- **Larger parties** — the full machinery (a bluffed charge column, multiple crossings, breakthroughs
  and interceptions).

**WHY.** Complexity should scale with the number of bodies. The protection layer only *means*
something when you have an ally to protect, so it must be invisible until then — keeping 1 v 1
the clean duel/Clash and ensuring the interface never shows an option that cannot matter at the
current head-count.

**GUARANTEES.**
- 1 v 1 reduces to the §1.0 duel/Clash with **zero** added decisions.
- A choice is presented **iff** it has ≥2 legal options; single-option phases auto-resolve.
- Adding bodies only *adds* choices; it never changes how the smaller case played.

### 4.2 Range & attack type — melee, ranged, both, or neither

**RULE.** Every Actor's offense is **melee**, **ranged**, **both**, or **neither**. Range is
**position-determined**, never chosen: **gauntlet crossings and Skirmisher strikes are melee;
Reserve fire is ranged.** A strike lands at its range; whether the target can **contest** it depends
on owning an attack of that same range:

- **Same range (target can contest)** → in the **deterministic base**, a **simultaneous trade**
  (both deal their base through armor/toughness, §2). When the **optional Clash module** (§1.0)
  is on, the trade is replaced by the four-card Clash + Force.
- **Range mismatch (target cannot contest)** → an **auto-hit**: uncontested, no mix-up, no Force,
  but still through the target's armor/toughness. Armor blunts an auto-hit; **a Tempo defense cannot**
  (spending Tempo turns a *melee* blow into a clash; it does not answer off-range fire).

The **Clash is a module, not the floor** — the game is fully playable with same-range = trade
(see `future-possibilities.md` Entry 3: the strategic layer is rich without RPS).

What follows from it:

- **Skirmishers are melee** (they broke through a melee gauntlet), so the **only core route to an
  enemy Reserve is a melee assassin.** Ranged Actors do **not** skirmish in the core. *(A card may
  explicitly supersede this — e.g. grant a ranged Skirmisher; see "Cards may supersede the
  core.")*
- **Reserve self-defense = whether it carries melee.** A Reserve with a melee attack **trades/Clashes**
  an assassin (fends it off by spending Tempo to clash); a pure caster (no melee) is **auto-hit**
  (assassinated).
- A **melee-less charger is legal but a very bad idea** — it is auto-hit at each crossing and cannot
  answer. (Emergent positioning, not a banned move.)
- **Neither** = pure support (heal / buff / area-control): it makes no attacks and is **always
  auto-hit** when reached — the most decisive-yet-fragile Reserve piece, wholly dependent on the
  wall. Its kit is Action cards over the §5 zone layer.

**WHY.** Range turns the **role triangle** from intent into mechanics: *Skirmisher ▸ Reserve* and
*Reserve ▸ Vanguard* are both **range mismatches** (melee assassin vs no-melee caster; ranged
fire vs no-ranged tank → auto-hits), while same-range meetings are Clashes. It also opens clean
power-design space: keep-at-range tricks, strong-at-ideal/weak-off-range hybrids, and pure-support
"neither" kits — each re-derivable from "do you have the attack for this range?".

**GUARANTEES.**
- A strike is a **Clash** iff attacker and target **share the range**; otherwise an **auto-hit**
  (armor-gated, no Force, no Focus contest).
- Range is **position-determined** (gauntlet / Skirmisher = melee, Reserve = ranged) — never the
  attacker's free pick.
- Core: **only melee Actors skirmish**; a card may explicitly supersede.

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Trade` (Combat) — A same-range engagement: both sides deal their base damage through armor/toughness. In the optional Clash module, the trade is replaced by the four-card mix-up.
- **TERM.** `Auto-hit` (Combat) — A range mismatch: the attacker lands uncontested (the target can't answer at that range). Armor still blunts it; Focus cannot.
- **TERM.** `Attack type` (Combat) — Each Actor is Melee, Ranged, Both, or Neither. Gauntlet crossings & Skirmisher strikes are melee; Reserve fire is ranged. Lacking the matching attack means you're auto-hit.

### 4.3 Actors are decks — *stats-as-deck & the schema*

**RULE.** An **Actor is a deck**, not a stat-block. In `booklet.ron` the actor entry is a **bare
identity** — `name`, `role`, `driver`, and **attack type** (§4.2) — plus a **starting deck**, and
**carries no stat fields**. Its numbers are **read off the deck's Form cards** (§2.3 / §5.2): a
**fundamental card** (base stats, incl. Health = count × value, §5.5) and **attachment** cards. The
§3 / §4 economy reads stats from the deck exactly as before (Speed sizes Tempo, Mind sizes Focus) —
only the *source* moved from the card to the deck.

**Schema migration (the `/spec-sync §2,§4` code pass).**
- `ActorCard`: **drop** `speed / power / precision / body / toughness / resolve / mind / weapon /
  traits`; **keep** `name / role / driver / attack`; **add** a starting `deck`.
- Catalog gains a **fundamental/Form card** (sets base stats + Health count × value) and **attachment**
  cards (each modifies one dimension) — `Card` / `TraitCard`-family entries that carry stat
  contributions.
- The `booklet.ron` data + the Rust `ActorCard` struct + the §4 reader change land **together** in
  the code pass; this Spec amendment is what they conform to. Until then, the running code (stat-bearing
  actors) is a **defect to fix** (code-is-a-defect-report), not the truth.

**WHY.** One representation — the deck — for what a character *is* and *does*; the authored stat-block
was a redundant parallel that drift could split from the cards (§2.1, #10). It also makes the Upgrade
economy (§8) mechanically real: buying a card literally raises a stat.

**GUARANTEES.**
- An Actor's numbers are always recomputable from its deck — no hidden stat-block.
- The §3 / §4 economy is unchanged in *behavior*; only the stat **source** moved (card → deck).
- A card works identically on a player and a creature (§8.4 deck-recipe creatures also build decks).

### 4.4 Role-card play — the ability layer 🟡 *(respecced 2026-06-20; code pending)*

**RULE.** Role cards are an **ability layer** over the physical gauntlet (§4). A character may play
**one role card of each role per round** — several in a round, so long as they are **different roles**.
A god holding all five tracks fires **up to five** effects in a round; a single-role specialist fires
**one** (and chooses which of its cards), then that role is spent until next round. Each card is played
**in its appropriate phase** — the phase that matches its role: a **Wall** card in the **Gauntlet**, an
**Infiltrator** card in **Skirmish**, an **Artillery** card in the **Reserve** phase; **Support /
Controller** cards in the phase their effect fits. Play is **decoupled from the body's gauntlet
position**: a god fires its five effects **across the round's phases**, *not* from five positions at
once — its body still occupies a single §4 position physically. (The Wall/Artillery labels are
**thematic**, not a position gate.)

**WHY.** The per-role cap is the **god-vs-party lever** (#4: god ≈ party). A god holds every track and
fires up to **five** effects in a round — but on **one body**, in one gauntlet position, that the enemy
can **focus-fire**; a five-specialist party fires the same five across **five resilient bodies**. So it
is a **concentration-vs-resilience tradeoff, not dominance** (candidate **BI-3**). Playing each card
**in its appropriate phase** keeps the §4 information gradient intact (a Wall card commits in the
Gauntlet, before the Reserve fires with full info) — a card is **not** held for the most-informed
moment.

**GUARANTEES.**
- One role card of each role, per character, per round; each played in its **appropriate phase** (so the
  §4 gradient holds), **decoupled from the body's physical position**.
- **Order-independent effects (the simultaneity constraint).** A round's role-card effects must
  **combine commutatively**: every effect feeds an **accumulator** (damage piles, heals pile, buffs add
  or set flags) resolved at the phase boundary, and **no played effect multiplies or gates another
  played effect's output.** So a god firing five effects gets the **same result regardless of order**
  (§0.1 / #11). *(Modifiers like Curse stay safe by folding into the build — passive, not a play.)*
- No party size dominates on raw role-card throughput (the #4 budget; candidate **BI-3**, which the
  par solver verifies).

*(The old **positional gate** (a card required its matching position) is **removed** 2026-06-20 — it
capped a god at ~3 and blocked the intended five-effect god; play is now phase-appropriate but
position-decoupled. Code/data + `TERM` lines land with the role-card migration: `role-card-redesign.md`
§8.)*

## 5. Zones / exhaustion — *the card state-machine* 🟡

The post-Clash rewrite of the orphaned exhaustion economy. Full design background:
`zones-exhaustion-design.md`. **Exhaustion replaces cooldowns:** cards-only (#7) forbids a hidden
timer, so using a card **moves it to a visible spent zone** until restored — which is exactly #8's
*"unpredictability erodes as cards exhaust, restored at a tempo cost."* Everything here is
**intra-encounter** (full reset at the Day boundary; strategic layer / `progression-design.md`).

> **Realizes north star #8 via zones.** #8's predictability-as-resource carries over intact (no
> luck; a managed, eroding resource restored at a tempo cost), but its *mechanism* moves from a
> never-shuffled **deck order** to **zone state**. The Charter's #8 still says "decks… order is
> intent"; updating that line is a deliberate Charter act left to the designer — **flagged, not
> done here.**

### 5.1 Three zones — Hand · Active · Down

**RULE.** Every card is in one of three zones, and **facing encodes state, not secrecy**
(face-up = in play / available; face-down = spent / dormant):
- **Hand** — held; cards ready to play.
- **Active** — face-up on the table; everything in effect (Form, Lasting stances, charges).
- **Down** — face-down on the table; spent/dormant cards, recovered to Hand.
Each card declares a **start zone** (most start in Hand; Form and standing stances start Active; a
charge-up / cooldown card can start Down).

**WHY.** Cards-only (#7) forbids hidden timers; zones make each card's status a physical, public
fact. Three is the minimum that distinguishes *held* / *working* / *spent*.

**GUARANTEES.**
- No hidden state — a card's availability is always visible as its zone + facing.
- The core game is **open information**; facing is *state*, never concealment (hidden info is opt-in
  — the Clash card-pick, §1.0, and optional PvP commit-reveal).

### 5.2 Form vs Action — what you are vs what you do

**RULE.** Cards in Active split by behavior:
- **Form** — your fundamental card + attachments (your stats, §5.5). **Permanent: never Spends,
  immune to Disrupt** — it cannot be knocked Down. Stats may be *temporarily reduced* by **Lasting
  debuffs** in Active (Slow, Sunder, Confuse), but the Form card never leaves.
- **Action** — maneuvers, governed by the verbs (§5.3).

**WHY.** *Exhaustion touches what you do, never what you are* — so stats stay stable and
recomputable (§2.1) even as the action economy churns. "Form" is a card **property**, not a fourth
zone (it lives in Active).

**GUARANTEES.**
- A stat never exhausts; only a removable Lasting debuff can modify its value, and removing it
  restores the stat exactly (no maintained meter — §2.1).

### 5.3 The verbs — default-return + Spend · Lasting · Recover · Disrupt

**RULE.** The **default** is: play a card, it **returns to Hand** (reusable next turn). Keywords
modify that:
- **Spend** — play → **Down** (a one-shot until Recovered).
- **Lasting** — play → **Active** (stays working until removed / Disrupted / consumed).
- **Recover** — move a card **Down → Hand** (the restore; costs a beat / Tempo).
- **Disrupt** — an attacker effect: move a target's **Active / Hand → Down** (force-exhaust).
Emergent: **cooldown** = Spend + a gated Recover; **combo** = a card that Recovers a specific card;
**engine** = a Lasting card that Recovers each Round; **disruption** = Disrupt.

**WHY.** A tiny verb set (#6) generates cooldowns / combos / engines with no bespoke per-card logic,
and each card's zone behavior prints as one line (#9/#10). The Clash kit (§1.0) is the simplest
case: four **default-return** cards ("no finite hand yet" = "everything is default-return").

**GUARANTEES.**
- Every card's lifecycle is {default | Spend | Lasting}, optionally acted on by {Recover | Disrupt};
  no other transitions exist.
- Adding cards never adds zone rules — new behavior composes existing verbs + tags (§5.4).

**MANUAL.** *Most cards return to your hand after use. A Spend card goes face-down until you Recover
it (Recover costs a beat). A Lasting card stays in play until removed. Disrupt knocks an enemy card
face-down.*

### 5.4 Tags — bounded cross-card interaction

**RULE.** Cards reference one another **by tag / type, never by name** (the damage types Fire /
Sharp / Blunt are the seed). A card's effect may **consume** tagged cards in Active by moving them
per the verbs. *(Worked example — fire charge-up: two `Charge(fire)` sit Lasting in Active; a Fire
card consumes them — damage ×2×2, Charges → Hand, Fire → Down. All zone-moves; the cost is the
setup Rounds.)*

**WHY.** Tags let cards combo while staying data-only and bounded — a name-reference is brittle and
unbounded; a small tag vocabulary is re-derivable (#6/#10).

**GUARANTEES.**
- Combos are {tag match} × {verb zone-move} — no bespoke combo code.
- Burst is paid for: charges cost the Rounds spent setting them up, not nothing.

### 5.5 Resources — Health · Tempo 🟡

**RULE.** Permanent **Form stats size a fluctuating pool** — you spend the pool, never the stats
(§3.1). There are **two** `count × value` pools in Active: **Health = Body × Toughness** (the value
gates damage) and **Tempo = Speed × Drive** (Speed-many cards, each worth Drive). *(Focus and Mind are
removed — merged 2026-06-20; defense is a Tempo spend.)* Spending moves cards to **Down**; they return
by **Recover** (or the round refresh). A gauntlet-crossing contest compares the **total Drive each side
commits** (§3); any other action just spends one card.
- **Round refresh** *(Tempo)* — at Round end all spent Tempo flips up (re-derived each Round, §2.1) — a
  per-Round budget, not cross-Round attrition.
- **Heal cards** *(Health)* — Recover Health within a fight.
- **Refresh engines** — a Lasting card that Recovers Tempo mid-Round (how a god exceeds base breadth).
**Health is the one pool that persists within a fight** (the maintained meter, §2.1); everything
fully resets at the Day boundary.

**WHY.** One machinery governs actions *and* resources. In co-op PvE (instinct foes don't read you,
§7) the limiter is action-economy / attrition; the predictability-telegraph half of #8 bites in PvP
/ vs Characters. Master tunable: Recover/refresh rate vs Spend rate.

**GUARANTEES.**
- §2.1's "one maintained meter" holds — only Health persists; Tempo/Focus re-derive each Round.
- Pools are recomputable from cards on the table (count × value − spent).

*(SEEDED — **stats-as-deck** is now specced (§2.3 / §4.3). Until the `/spec-sync` code pass migrates
the schema, "Form stat" still resolves via the actor-card stat in the running code. Numbers — pool
sizes, Spend/Recover costs, charge magnitudes — live in `booklet.ron`, human-tuned.)*

**Open dials.** (1) **Attachment composition** — in the single-deck core, attachments **compose
commutatively**; the order-dependent **modifier** variant is part of the deferred aspect/combo layer
(§6 → `future-possibilities.md`). (2) **`TERM` glossary vocabulary + encyclopedia + glossary test** —
land together in the **`/spec-sync §5`** code pass. (3) **Numbers** — `booklet.ron`.

### 5.6 Role-card taxonomy — Base · Modifier · Mode · Stat 🟡 *(in code 2026-06-19)*

**RULE.** A **role card** (§8.3) is exactly one of four kinds:
- **Base** — *played* from Hand; the track's core effect (normal §5.3 zone behaviour).
- **Modifier** — *passive*, lives in **Active** (§5.1); auto-applies to its Base (the scaling card),
  **never separately played** — so a base and its upgrade coexist under the §4.4 per-role cap.
- **Mode** — *played*; an alternative / charged Base (e.g. spend a round for a bigger effect),
  **mutually exclusive with the Base that round**. **[M1, 2026-06-19] Defined but deferred:** the
  first content (`role-card-redesign.md` §10) builds the L5 capstones as **`Spend`-zone Bases**
  instead — the existing §5.3 zone machine already gives the "big, once-per-fight" cooldown a Mode was
  meant to impose, with no new mechanic. The Mode kind stays in the taxonomy for the richer
  "spend-a-round-to-charge" tactical layer when playtest calls for it (→ `future-possibilities.md`).
- **Stat** — a **Form attachment** (§2.3 / §5.2): contributes to the stat block, **not played**.

**WHY.** The split lets richer high-level rewards (#5 power-up, §8.3) coexist with the **one-card-per-
role-per-round** cap (§4.4): Modifiers and Stats ride free; only **Base + Mode** plays count. It reuses
the existing **passive-power vs played-action** distinction (§5.2), so it is not new machinery.

**GUARANTEES.** A reward's cards are **self-contained** — its Modifiers / Stats apply *within* the set;
**no cross-reward multiplicative combo** (§0.1). *(Code/data + `TERM` lines land with the role-card
migration — `role-card-redesign.md` §8, Phase 2.)*

**Confirmed migration mechanics (2026-06-19).** The §10 first-draft content needs six small additions,
resolved at the §4.4/§5.6 spec-sync and pinned here so code follows spec:
- **M1 — Mode → `Spend` Base** (above): capstones are once-per-fight Spend Bases; the Mode kind is deferred.
- **M2 — `Guard`** (Wall L1 *Brace*): a played effect that adds **+Focus to the holder this round** — a
  defensive boost to the wall's block vs slips (§4.2 Focus). Seed +3.
- **M3 — "cannot fall" this round** (Wall L5 *Last Stand*): while active, damage that would down the
  holder leaves it at **1 Body** instead — it cannot be downed for the round.
- **M4 — execute** (Infiltrator L5 *Assassinate*): a Damage card that, on hitting an enemy **Reserve**,
  **downs** that foe regardless of remaining Body.
- **M5 — `Curse` Modifier** (Controller L4): a passive that makes the owner's debuff cards
  (Slow / Confuse / Dread) each hit **+1 additional foe** — the one instance of the Modifier mechanic
  in the draft (lean-new-effect dial, §9.1).
- **M6 — `targets: all`** (Support L5 *Sanctuary*): a buff effect (Mend / Ward / Haste) may target
  **all allies** — a party-wide target mode.

## 6. Aspects / the chord — *deferred*

**Deferred to `future-possibilities.md` (entry 4).** The multi-deck **chord/combo** system (a
character as a set of aspect-decks; a play as one card per aspect, combined) is **parked** until the
**single-deck core** is working fully and tuned against the reference scenario. The core character
model is **one deck** — Form (fundamental + attachments) + Action cards over the §5 zones — not a
chord of aspect-decks.

*(Terminology note: the three **defense channels** Body / Mind / Spirit (§2) are unaffected — they
are damage types / thresholds, not the deferred deck-chord, despite the shared word "aspect.")*

## 7. Agents — Character vs Creature ⬜

*Stub.* The line is **theory of mind**: a Character reasons and predicts you back
(two-way); a Creature draws from a behavior deck (its instinct = its decision,
one-way), reshuffles, never exhausts. Source:
[`notes/entities.md`](../notes/entities.md),
[`notes/decision-making.md`](../notes/decision-making.md).

## 8. Strategic layer — *the run* 🟡

The game outside a single fight: the world map, the clock, **role-card rewards**, encounters, and
progression. Full design background: `progression-design.md` and **`role-card-redesign.md`** (the
reward model now governing §8.3 / §8.5, with §4.4 / §5.6); `reference-scenario.md` is the balance
harness. **Run-level victory is provisional** (a test goal — §8.2); **run-level defeat is deliberately
undefined** — deferred until the mechanics are tested against the reference scenario, so difficulty is
tuned from data, not guessed. Numbers throughout are `booklet.ron`, human-tuned. The **role-card
redesign is now in code** (2026-06-19): no currency / Upgrades — clearing `(track, level)` unlocks an
atomic reward assigned at unlock; combat enforces the §4.4 cap + positional gating; the 25 sets live in
`booklet.ron` (see `role-card-redesign.md` §8–§10). One migration remains pending: **stats-as-deck**
(§2.3 / §4.3 / §5.5).

### 8.1 The world — locations, movement, fog

**RULE.** The world is **face-down location cards** in a scenario-authored layout — a **grid**, an
**offset-hex** field (alternate rows shifted half a card), or a mix. A character's **identity card**
(its Actor) marks where it is. Entering a location **flips it face-up** (revealing its name → its
**Suit** (§8.5) → the **threat deck** it draws from, §8.4) but does **not** start a fight.
Movement is **one adjacent space per Day** (§8.2). *(Travel cost / risk beyond this is deferred.)*

**The grind base — 25 location cards.** The §8.3 reward set is the world's **experience-grind base**:
**one location card per `(Suit, level)`** — five Suits × five levels = **25 cards**, each a single-tier
clear that grants its Suit's rewards `1..=level` (a higher card **subsumes** the lower ones, so they
are skippable — difficulty + travel cost are what discourage leaping ahead). The base set tiles a
**5×5 grid**, placed by a **seed** the world is created with (so a layout is reproducible and a
reference/test scenario is predictable — the seed is a world-creation parameter alongside the combat
seed; the full grid is always connected, so every card is reachable for any seed). A game uses **some
subset of the 25** (usually all) plus **scenario-specific special locations** whose treasures sit
*outside* the 25-card base and change play more dramatically.

**WHY.** Cards-only (#7); a face-down map makes scouting a push-your-luck act (#2) and is the engine
of doom-to-mastery (#5 — you learn a place by going there); a pawn on a map is a clean metaphor (#9).

**GUARANTEES.**
- Entering reveals information only; combat is always opt-in (§8.4).
- Position is a single card on the table — no hidden coordinates.

### 8.2 The clock & the run goal

**RULE.** Time advances in **Days**, driven by the **event deck** (for now only *"1 day passes"*
cards — a placeholder for real world events). Each Day, **every character** may **move one space**,
use a **per-day ability** *(deferred)*, and attempt **one Encounter** (§8.4); all act **in parallel**
(order-independent; no turn order, §1.9 / §3.1). At the **Day boundary** everything **fully resets**
(Health, all pools, Action cards Recover; §5.5). **Run victory (provisional):** clear the scenario's
**objective / final location**; the run is **scored in Days** (golf par — fewer is better). **Run
defeat: undefined** — deferred until tested against the reference scenario.

**WHY.** A single scalar (Days-to-clear) is the balance instrument — it stresses routing, encounter
difficulty, the economy, and the depth/breadth fork at once (#2 strategy; balance-by-scenario #4).
Deferring defeat until we *measure* avoids guessing difficulty before we have data.

**GUARANTEES.**
- The only thing a run spends (for now) is **Days**; nothing yet kills a party (full daily reset →
  no cross-Day attrition).
- No turn order at the strategic layer — characters act in parallel within a Day.

### 8.3 Rewards & role cards 🟡 *(in code 2026-06-19)*

**RULE.** Clearing **level X of a Suit-track Y** unlocks the **reward** for `(Y, X)` — a reward **of
that Suit** (§8.5): a fixed, **atomic set** of cards — role-effect card(s), a bundled generic **Stat**
card, and any passive **Modifier** (§5.6) — **one physical copy each** (scarce). The **party assigns
the whole set, permanently, to one character — at unlock** (the clear that earns it surfaces the
choice; there is no holding pool). Five Suits × five levels = **25 rewards**. **No currency** — clearing
*is* the unlock (clear level N of a Suit ⇒ its levels 1..N). Each card prints its `(suit, level)`
**provenance** (e.g. *Iron · III*), so a set is identifiable by its Suit and stays together.

> **Replaced (2026-06-19) — the currency economy.** §8.3 was *Currency & loot*: clearing earned typed
> **Currency** (Iron/Silver/Brass/Bone/Salt + generic Gold) that bought stat **Upgrades**, balance
> recomputed `earned − spent`. The redesign drops the currency *middleman* — clearing unlocks a
> role-card reward **directly** (the depth/breadth fork lives in routing). The five currencies survive
> only as **track colours/identities**; generic **Gold** becomes the bundled **Stat layer**, not a
> currency. (The *co-location* spend rule was already cut as bookkeeping.) Full design + migration plan:
> [`role-card-redesign.md`](../../role-card-redesign.md).
>
> **Renamed (2026-06-20) — "track colour/identity" → Suit.** Those five surviving identities are now the
> first-class **Suits** (§8.5): **Iron · Silver · Brass · Bone · Salt**, bound 1:1 to the five Roles.
> Treasure is named by its **Suit**, not its Role. **Gold is retired** (no sixth/generic suit — the
> Stat layer is suit-less). Pure vocabulary; no mechanic or number changed.

**WHY.** One-copy scarcity (no stacking) + atomic permanent assignment make *"who carries this"* a
weighty choice (#2 opportunity cost; #4 team balance); the shared pool is a **party-size-independent
power budget** (#4: god ≈ party-total). Direct unlock keeps the build a §0.1 *no-path-dependent-budget*
function of clears + assignment, with the strategic fork in **routing** (§8.1–8.2).

**GUARANTEES.**
- Total reward power = a function of **levels cleared**, shared and distributed — party-size-independent.
- A reward is **atomic** — acquired and assigned as one unit, never sub-drafted or split — so the
  build-space dimension is the **count of rewards, not cards** (§0.1).
- **No path-dependent budget** (§0.1): the build is *which rewards are owned and who holds each*;
  assignment is **permanent** (no sell-back, no oscillation).
- **Power is monotone in level** — within a track a deeper reward is *at least as powerful* as a
  shallower one (the doom-to-mastery curve, #5); complexity is an *optional lever* for that power,
  never the intent.
- One physical copy per reward; each card prints its `(suit, level)` provenance, so scarcity and
  atomic assignment are legible / self-enforcing.

### 8.4 Encounters — the parametric deck-recipe

**RULE.** Combat at a location is **opt-in at a chosen level**. Every location has a **Suit** (§8.5),
its threat's identity. On first engagement a single **encounter card** is drawn from that **Suit's
threat deck** (one deck **per Suit** — five) and then **fixed**: it is the location's **persistent,
learnable threat** (retrying faces the
*same* fight). The encounter card is a **parametric deck-recipe** evaluated at the attempted level —
a roster and **thematic** stat-scaling (which stats scale signals the counter to bring). The **level
is one dial scaling reward and threat together**.

**WHY.** Each threat deck is a **diegetic tutorial** — you meet a **Suit's** threats and unlock that
**Suit's role cards** that answer them (#1 reward intellect; #6 emergence). A fixed, learnable threat
means failure teaches (#1); one dial keeps the risk/reward choice honest and re-derivable (#2 / #10).

**GUARANTEES.**
- Reveal gives the **Suit** (threat deck), never the exact card before you commit a fight.
- A failed clear costs a Day and the threat persists; you advance only by beating it at the depth
  you want.

### 8.5 Progression & roles 🟡 *(in code 2026-06-19)*

**RULE.** A character **is its assigned role cards** — "role" is *emergent*, not a label, and roles
only **accrete** (assignment is permanent, §8.3). There are five **role tracks**, the §4 triangle's
**`3 + 2`**. Each track has two names in **different registers**:
- a **Suit** — its **identity**, a substance: **Iron · Silver · Brass · Bone · Salt**;
- a **Role** — its **function** in combat: **Wall · Infiltrator · Artillery · Controller · Support**.

They are bound **1:1** — **Iron = Wall · Silver = Infiltrator · Brass = Artillery · Bone = Controller ·
Salt = Support**. The **Suit is what a reward / treasure *is*; the Role is what it *does*.** Name a
treasure by its **Suit** — *"an Iron reward,"* never *"a Wall reward"* — so identity never collapses
into function. (Identity and function are deliberately kept in different registers — substance vs.
combat job — so the Suit never merely restates the Role.) A generic **Stat layer** is **bundled into
every reward** and is **suit-less** (the retired generic, **Gold**, is gone — now a stat-card pairing,
not a sixth Suit). A character's **first clear commits a direction**; from there it **specializes**
(depth: pour one track) or **branches** (breadth: cover several). Party size sets the spectrum: many
bodies → specialists (one track each); few → multi-track; one → a **god** spanning all five.

**WHY.** Characters are deliberately unbalanced; coverage and challenge come from the **team and the
scenario** (#4). Depth-vs-breadth is the uncomputable strategic fork (#2), fractally at map and build
scale; the party-size spectrum **is** the god ≈ party-total balance budget (#4). Role-as-assigned-cards
makes "god ≈ party" *concrete* — the **same** shared pool, distributed — and the per-role play cap
(§4.4) is what equalizes their throughput. **A reward needs a noun of its own:** named only by its Role,
*"a Wall treasure"* conflates what it *is* with what it *does* — the **Suit** gives identity its own
register (#10 conceptual integrity — each concept named once, for one job).

**Why exactly five — `3 + 2`.** The role set is the *smallest complete* one on both of combat's axes,
so the count is re-derivable, not arbitrary (#10):
- **Three positional roles = the §4 counter-triangle's vertices:** **Wall = Vanguard** (hold the
  front), **Infiltrator = Skirmisher** (break through the gauntlet), **Artillery = Reserve** (fire from
  safety).
  Three is the *minimal* counter-cycle — RPS needs exactly three.
- **Two effect roles = the complete duality of state-bending:** **Support** *augments* your side (`+`:
  heal / ward / haste), **Controller** *degrades* theirs (`−`: slow / confuse / fear). Two is the
  whole of that duality.

So **5 = a complete engagement cycle (3) + a complete effect pair (2).** **Four** would break one —
drop a position and the triangle is no longer a counter-cycle, or drop an effect and the `+/−` pair is
lopsided. **Six** would need a new orthogonal axis (there isn't an obvious one beyond *where you fight*
and *how you bend state*) or an over-granular *split* of an existing role (refinement, not a new role —
against #6 / the small core).

**GUARANTEES.**
- The five roles are **`3 + 2`**: the §4 triangle's three positions (Vanguard / Skirmisher / Reserve =
  Wall / Infiltrator / Artillery) plus the two effect directions (augment = Support, degrade =
  Controller) — *minimal-complete on both axes*, not an arbitrary list.
- A character's roles = its assigned role-card tracks; they **accrete** (monotone, §0.1).
- **Stats are bundled with role rewards** — the survivability to *use* a role grows *with* the role;
  there is no free-floating generic stat pool (no "stat-mule").
- Five role tracks (the `3 + 2`); the generic is a **Stat layer**, not a sixth track.
- **Each track has exactly one Suit** — a 1:1 Suit↔Role binding. There are **exactly five Suits**
  (Iron · Silver · Brass · Bone · Salt) and **no generic / colourless suit**; the bundled Stat layer is
  **suit-less**.
- A solo god ≈ a full party in total power (the budget difficulty is tuned against).

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Suit` (Roles) — A role track's **identity** (a substance): Iron · Silver · Brass · Bone · Salt, bound 1:1 to a **Role** (Wall · Infiltrator · Artillery · Controller · Support). The Suit is what a reward *is*; the Role is what it *does*. Name treasure by its Suit — "an Iron reward," never "a Wall reward."

*(SEEDED — §8 is the strategic layer's first graduation. The **role-card redesign** (this §8.3 / §8.5
plus §4.4 / §5.6) is now **in code** (2026-06-19): no currency/Upgrades; clearing unlocks an atomic
reward assigned at unlock; combat enforces the §4.4 cap + positional gating; the 25 sets are authored in
`booklet.ron` (Phases 1–4 of [`role-card-redesign.md`](../../role-card-redesign.md) §8). The
**stats-as-deck** power mechanism (§2.3 / §4.3 / §5.5) is still a pending `/spec-sync`. **Travel risk**,
**per-day abilities**, **world events**, and **run-level defeat** are deferred (the last until
reference-scenario testing). Numbers are `booklet.ron`, human-tuned. `TERM` glossary lines + encyclopedia
land with the `/spec-sync §8` code pass.)*
