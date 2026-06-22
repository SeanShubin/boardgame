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
| **Card representation** (suits · base-2 · tree · clocks) | ✅ locked | **§2.4–§2.7** locked 2026-06-21 (Quantity/Power · base-2 denominations · deck-tree positional notation · reset clocks); code/data migration pending `/spec-sync` |
| **Speed/Tempo** (one breadth pool)                | 🟡 seeded    | §3 — Tempo pays offense *and* defense; **Focus/Mind merged out** (2026-06-20); `notes/speed-and-tempo.md`                                               |
| **The battle — declare ranks, hold the line**     | 🟡 seeded    | §4 **respecced** to the static-ranks model (the threading gauntlet removed) and **code synced** (`the_line`, 2026-06-21). §4.3 actors-are-decks still pending |
| **Zones / exhaustion**                            | 🟡 seeded    | **§5 worked** (zones · Form/Action · verbs · tags); resources 🟡 (stats-as-deck now §2.3/§4.3) — `zones-exhaustion-design.md`                           |
| **Aspects / the chord**                           | ✖ retired   | decommissioned → `retired-ideas.md` (the bar to revive is recorded there)                                                                              |
| **Agents** (Character vs Creature)                | ⬜ stub      | `notes/entities.md`, `notes/decision-making.md`                                                                                                        |
| **Strategic layer** (world/event decks)           | 🟡 seeded    | **§8** (world · clock · role-card rewards · encounters · progression) — `progression-design.md`                                                        |
| **Skirmish victory / defeat**                     | 🟡 seeded    | `notes/form-and-defeat.md` (eliminate the foes / the party falls; in code)                                                                             |
| **Run victory / defeat** (across many skirmishes) | 🟡 seeded    | **§8.2** — victory = clear the objective, scored in Days (golf); **defeat deferred** pending reference-scenario tuning                                 |
| **Geography & travel** (the world map + movement) | 🟡 seeded    | **§8.1** (locations · move 1/Day · fog); travel risk deferred — `progression-design.md`                                                                |
| **Loot / role cards** (clear → reward)            | 🟡 seeded    | **§8.3** — atomic 25-card role-reward pool, scarce, party-assigned permanently; each reward **of a Suit** (Iron · Silver · Brass · Bone · Salt); **no currency** (role-card redesign, *in code 2026-06-19*) — `role-card-redesign.md`         |
| **Progression** (growth between skirmishes)       | 🟡 seeded    | **§8.5** — role = assigned cards · `3+2` tracks, each a **Suit** ↔ **Role** (identity ↔ function) + bundled Stat layer · depth/breadth; play rule §4.4, taxonomy §5.6 (*in code 2026-06-19*) — `role-card-redesign.md`         |

✅ worked = full, the template to follow · 🟡 seeded = a few real rules, not
exhaustive · ⬜ stub = headers + intent only, not yet authoritative · ⏸ deferred = parked to
`future-possibilities.md` · ✖ retired = decommissioned, parked to `retired-ideas.md`.

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
  still optimizes a *concrete* rule-set, so the §4 open dials (the bid / free-hit magnitudes) must be
  pinned (or the static-ranks semantics ratified) before "perfect" means
  *perfect at the designed game*.
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
so nothing is order-dependent. *(The order-dependent **modifier** card-kind is retired with the chord
layer — `retired-ideas.md`; were it to return, its on-target conflicts would resolve in a **fixed
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
  commutatively, §5; the retired order-dependent modifier would use a fixed seat key, not Speed).
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
> merge — see §3.2.)* The word *aspect* is **reserved** for the retired deck-chord combo layer (§6) and
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
(fear → Spirit); its **base magnitude is the channel's attack stat — Strike for outer, Dread for
inner** (the Controller's fear-projection), plus any card power. It resolves: **subtract the cut**
(Armor for outer, Ward for
inner; per source, typed, never depletes) → **accumulate the remainder into the
round's pile** → **compare the pile to the bar** (Toughness for Body, Resolve for
Spirit). Only the **outer** channel has a **pool** (Health cards) behind the bar — so it is the **only
channel that deals damage**. The **inner** (fear) channel has **no pool and deals no damage**: when the
pile exceeds the bar it **breaks the will into a round-scoped control status**, tiered by how far past
Resolve it clears (Freeze / Shaken / Rout — §4 Controller status). **Fear never touches Body.**
Cross-immunity: outer ignores Ward; inner ignores Armor.

**WHY.** A per-source cut answers *many small hits*; a high bar answers *any one
big hit* — non-redundant, so you want both. Typing the cut makes "called shots"
fall out for free (choosing a damage type chooses which channel you attack).

**GUARANTEES.**
- Both channels are structurally parallel (offense · cut · bar), but only the **outer** has a **pool** —
  and that pool is what makes it the **only damaging channel**.
- An inner break is a this-round **control status** that clears at Refresh; it **never reduces Body**.
  The inner channel deals **no direct damage** — death is the outer channel's alone. *(Proposed
  2026-06-21; supersedes the former scared-to-death Body-bleed.)*
- **Fear is the Controller's control, not its damage** — consistent with §8.6's damage-separation law.
- Accumulation is always cards in a zone, never a number in the head.

*(SEEDED — the damage formula, scaling, and the Resolve break threshold are not
yet specced. Numbers live in `booklet.ron`.)*

### 2.3 Stats live in the deck — *stats-as-deck*

> **Locked 2026-06-21.** *No actor's identity card carries stats* — hero or creature. Stats always live
> on separate **build cards**. The hero/creature difference is the build card's **lifecycle**, set by
> **progression** — not whether stats are "printed." (Refined 2026-06-21 from an earlier character-bare /
> creature-printed split into this single rule — see WHY.)

**RULE.** An actor's **identity card** is **bare** — a name, a role, a map token (§8.1), nothing more —
for **every** actor, hero or creature. All of its stats live on separate **build cards**, read as the
**Form** (§5.2 / §2.4–§2.6): so §2.1's "passive stats read off the table" — **Armor, Ward, Toughness,
Resolve**, and likewise **Speed, Daring, Power** — are **build-card-derived**, never authored on the
identity. A build card encodes one build's stats; two kinds behave identically but differ in
**lifecycle**:

- **Hero build card** — encodes a **starting build** (a clean slate, or a pre-set scenario kit). In the
  printed game it is a **setup artifact**: at setup you instantiate the hero's Form from it, then **set
  it aside**. The live Form is thereafter the hero's stats, and it **grows** as Upgrades are added
  (§8.3/§8.5) — so the starting-build card is a template, never live state.
- **Monster build card** — encodes a creature's **fixed core stats**. It **persists** in play as the
  creature's stat representation, because a creature never progresses (the build *is* the creature).

Changeable, maintained state — the **Body Health pool**, **Tempo** — is tracked **as normal** (§2.1),
separate from the build cards. *(Numbers live in `booklet.ron`.)*

**WHY.** "The deck *is* the character" (#8), generalised: an identity is *who*, a build is *what*, and
keeping *what* on cards makes **every point of strength a card you can point to**. The single splitter is
**progression**: a hero's Form **diverges** from any starting build (it gains cards), so that build card
can only be a setup template — keeping live stats on the assembled Form is what makes "stronger = more
cards" true. A creature **never** diverges, so its build card can *be* its standing representation. This
**refines** the earlier "character bare / creature printed" wording: putting creature stats "on the
identity card" wrongly implied two rules. There is **one** — *identities are bare; stats are build
cards* — and the hero/creature difference falls out of **lifecycle**, not of where stats are printed.

**GUARANTEES.**
- **No identity card carries stats** — hero or creature alike. Stats are always build cards / Form cards.
- **Hero build cards are setup-only:** they instantiate a starting Form and are then set aside; the live
  (and possibly grown) Form is the hero's stats. Heroes with the *same* assembled Form play identically,
  however the build was specified.
- **Monster build cards persist** as a creature's fixed stats; a creature neither gains nor sheds build
  cards in play.
- The splitter is **progression, not type**: a build that can grow keeps its card a setup template; a
  build fixed for life may persist as the representation.
- **Data note:** in `booklet.ron`, `ActorCard.base` is an **inline build card** — empty for a bare
  campaign hero (the Novice; its build is the separate clean-slate + reward cards), populated for a
  creature or a fixed scenario-hero kit (its build card, stored inline). Combat resolves every actor
  through the one Form path.

### 2.4 The two suits — *Quantity & Power*

> **Locked 2026-06-21.** Every Form stat is one of two named suits. The names are the whole stat
> vocabulary; learn them once, read them everywhere.

**RULE.** A Form card carries a **suit** and a value, and is one of exactly **two** suits:
- **Quantity** — *breadth*: how many cards (a count). Only **pooled** stats have a Quantity — **Body**
  (Health cards) and **Speed** (Tempo cards).
- **Power** — *depth*: how much each card is worth (a per-card magnitude). **Every** stat has a Power:
  **Toughness** (per Health card), **Daring** (per Tempo card), and the flat magnitudes — the strike
  stat, **Dread**, **Inspiration**, **Precision**, **Resolve**, **Armor**, **Ward**.

The suit classifies; the **deck** (§2.6) names the stat. So each existing stat is a **(deck × suit)**
cell: Body·Quantity = health count, Body·Power = Toughness, Tempo·Quantity = Speed, Tempo·Power = Daring,
Strike·Power = the strike stat. A pooled stat has **both** suits; a flat stat has **Power only**. A leaf
card itself prints only *(suit, value)* — which stat it feeds is fixed by the deck it sits in.

**Inspiration scales augments.** The Salt force-multiplier: a Support buff's magnitude gains the
augmenter's **Inspiration** — Mend +Inspiration Body, Rally +Inspiration Resolve, Empower +Inspiration
Power, Haste +Inspiration Tempo — exactly as an attack's magnitude gains its channel's attack stat
(Strike / Dread, §2.2). It is the augment axis's magnitude dial and the **Support** role's signature
(#12); flat buffs with no magnitude (Steel, Ward) do not scale.

**WHY.** Two suits are the entire stat vocabulary, so a player learns "**Quantity = how many, Power = how
hard**" once and reads it on every stat — §2.1's count×value shape generalised from defense to all of
them, and §2.3's "the deck is the character" (#8) made addressable. **Power is the quantum of meaning**:
it sets the smallest difference the game will represent (a Body·Power-4 Health card flips only after 4
damage banks), so the power-fantasy scaling pours into **Power** — huge effect, card count flat — while
**Quantity** stays small and every card on the table stays a meaningful state. The collision with the
legacy strike stat *"Power"* is intentional, not a clash: that stat simply **is** Strike·Power, the
canonical flat instance of the suit — Power is the magnitude atom.

**GUARANTEES.**
- No stat exists outside the two suits **{Quantity, Power}**.
- **Quantity** appears only on pooled stats (Body, Speed); **every** stat has a Power.
- Suit meaning is **global** — Quantity is always a count, Power always a per-card magnitude; a suit is
  never rebound to a different role under a different deck.
- The legacy **"Power" stat = Strike·Power**; the shared name is the same concept, not two.

### 2.5 Base-2 denominations

> **Locked 2026-06-21.** Suit cards come in powers of two, one of each — the uniquely-canonical,
> fewest-cards encoding.

**RULE.** Suit cards come in **base-2 denominations** — 1, 2, 4, 8, 16, … — with **at most one of each
denomination per suit per deck**. A stat's value is the **sum** of its denomination cards. Because no
denomination repeats, **every value has exactly one representation** (its binary expansion), and a value
*V* costs **popcount(*V*)** cards.

**WHY.** One-of-each base-2 is the **unique, minimal** encoding: there is never a second way to show 18
(= 16 + 2), so a value reads and renders unambiguously, and card-count = set-bits = **O(log V)** — a stat
can scale into the power-fantasy range while the table stays sparse (§2.4: scale via Power, keep Quantity
small). The base also fixes the game's **natural numbers**: balance values gravitate to powers of two and
their sparse sums — an intentional binary aesthetic, and a ready cost metric (popcount).

**GUARANTEES.**
- One copy max of each denomination, per suit per deck → the canonical (binary) form is unique.
- A stat's card-cost is **popcount(value)**; doubling a stat is **+1 card** (one new denomination).
- **Consumable interaction:** decrementing a consumed Quantity pool (Body Health) past a high denomination
  "makes change" — the digital UI re-renders the canonical form; a printed edition may instead hold a
  *consumed* pool at unit denomination. Read-once Power stats are always free to denominate.
- The popcount cost is a **tiebreaker only** — it never overrides a balance target, and it never collapses
  stats that differ by reset clock (§2.7).

### 2.6 The deck hierarchy — *positional notation*

> **Locked 2026-06-21.** A character's Form is a tree of decks; a card's meaning is its path.

**RULE.** A character's Form is a **tree of decks**, and a leaf card's **meaning is its path**. The root
is the bare identity card (§2.3); its children are the **stat decks** (Body, Tempo, Strike, …); a stat
deck's children are its **suit decks** (Quantity, Power); the **leaves** are the base-2 denomination
cards. A deck's **face shows its rolled-up total**; opening it reveals the addends that sum to it. **Only
leaves carry values** — an intermediate deck is pure position, never a number.

**WHY.** Positional notation is what lets the **generic** denomination cards (§2.5) be reused across every
stat: a "Power 4" leaf means Toughness under Body and Daring under Tempo — meaning comes from the **path**,
not the card, so the print vocabulary collapses to *{denomination × suit}*. The tree also **enforces
position for free** (a card can't be orphaned — it lives inside its deck) and **maps to physical
containment** (nested banded bundles). Deck-face = sum is §2.1's "read it off the table" made navigable:
the total you act on, the addends you audit.

**GUARANTEES.**
- A leaf's meaning = *(its path) × (its denomination)*; the same leaf under two decks is two different stats.
- Only leaves hold values; a number on an intermediate deck is a defect — meaning lives at exactly one level.
- A deck's face equals the **sum** of its contents (Form is commutative, §5.2 — order within a deck is irrelevant).
- Positional encoding governs the **static Form tableau only** — never a shuffled or drawn pile. Action
  cards (§5) stay intrinsically meaningful; you may not positionally encode a deck you draw from.

### 2.7 Reset clocks — *when mitigation discards*

> **Locked 2026-06-21.** A mitigation layer is defined by *when* it discards, not only how much.

**RULE.** A defensive layer carries a **reset clock** — when the damage it absorbs is discarded — and the
clock is part of the stat. The Body channel stacks three:
- **Armor** — **per hit**: the cut applies to each blow independently; sub-cut damage is discarded at once.
- **Toughness** — **per round**: damage banks into the round's pile and flips a Health card each Toughness;
  the sub-Toughness remainder clears at round end (§2.2).
- **Health (Quantity)** — **per encounter**: a flipped Health card stays flipped until combat ends
  (restored on a win, §2.1).

The clock is **orthogonal to magnitude** — the same Power can sit on any clock — and choosing the clock is
a design dimension in its own right.

**WHY.** The clocks are **non-redundant because they counter different damage *shapes***: per-hit Armor
erases **many small** hits (each shaved in full); high per-round Toughness lumps **few big** hits into
rare, meaningful flips; per-encounter Health is raw, shape-agnostic capacity. Keeping all three is
**several strategies toward one end (survival)** — armor and a tough hide are *different on purpose*, and
"it matters *when* they discard" is precisely why they do not collapse into one stat (the §2.2 WHY's
"many small vs any one big," generalised to a timing axis). The clock is also where new mitigation
flavours are **minted without new complexity** — a per-exchange or per-attacker cut is a fresh strategy at
the same card cost.

**GUARANTEES.**
- Every mitigation layer names a reset clock; two layers of equal magnitude on different clocks are
  **distinct** stats, not duplicates.
- The popcount tiebreaker (§2.5) breaks ties **within** a clock, **never across** — it must never collapse
  Armor into Toughness.
- The clocks form a **closed, named set** per channel; adding a clock is a Spec change (a new mitigation
  kind), not free data.

---

## 3. Speed · Daring · Tempo — *the breadth budget* 🟡

Design source: [`notes/speed-and-tempo.md`](../notes/speed-and-tempo.md).

> **Locked 2026-06-20.** The breadth economy is the three terms below, ratified together. Earlier forms
> (two pools Tempo/Focus; a per-target-Speed cost; a value-less Tempo) are superseded — see the §3.2–3.4
> history banners. This section is the **single canonical home** for what Speed, Daring, and Tempo are;
> any change that makes one of these three words do another's job has broken the concept (the GUARANTEES
> are the tripwires).

Two permanent **Form** stats size one round-scoped **pool of cards** — the same shape as Health
(Body × Toughness → Health):

- **Speed** — *count*: how many **Tempo** cards you start each combat round with.
- **Daring** — *grade*: the magnitude printed on each of those cards.
- **Tempo** — the *pool*: Speed-many cards, each worth Daring, flipped face-down to spend and rebuilt
  fresh each round. **Spent cards stay spent for the whole round.**

### 3.1 What Tempo and Daring do

**RULE.** **Flipping a Tempo card gates every *action*** — a strike, a crossing contest (slip or catch),
a strike back. **Standing in a rank, letting a foe slip by, and *absorbing* a blow are free** — Tempo is
the currency of acting on the enemy, not of mere presence. Run out of face-up Tempo and you can take no
more actions this round (you still hold your rank and soak with Body).

**Daring's magnitude does real work in exactly one place — a *crossing contest* (§4).** When a Skirmisher
tries to cross a Vanguard, the two hold a **single simultaneous Daring bid**: each commits some Tempo
cards, worth (cards × Daring); **the higher total wins, ties to the catcher** (Shadowstep overrides) —
strictly higher → slips by; equal-or-less → held. The bid cards are spent (they do **not** return until
refresh), so crossing *more* walls, or *harder* ones, drains more Tempo. A **held** Skirmisher and its
catcher **trade** (each deals Power) on the cards they committed — you never pay twice.

**Everywhere else, Daring's number is irrelevant — only the flip counts.** An **exchange** (a strike) is
**single-card**: flip *one* Tempo card to strike, and the blow is the same whatever the card's Daring
(Daring sizes a crossing, never a blow). An enemy can only attack you by **spending a Tempo card**, and
that attack is Daring-independent. You may **reflexively strike back** at anyone who strikes you in melee
(position is irrelevant — they came to you), but striking back is an action: it costs **one** Tempo
card, and with none to flip you simply **take the hit** (a free hit). *(Ranged fire is one-sided — you
cannot strike back at a Reserve that did not come to you — except in the no-front open brawl, where two
ranged foes exchange if both spend a card, or where a card grants an exception, §4.2.)*

**WHY.** One pool for act-and-defend makes the cannon/wall axis a live **allocation** (spend it
attacking and you cannot answer a skirmisher) rather than a second stat. Splitting the pool into
**count (Speed)** and **grade (Daring)** gives two clean power dimensions that mean different things:
**Speed = how many crossings/actions you get; Daring = how cheaply you win each crossing.** Confining
Daring to the crossing keeps a strike's force on Power (not on how hard you shoved through the line), and
the per-round depletion is the tension — press the crossings hard and you are spent for the strikes (#2
opportunity cost).

**GUARANTEES.** *(the tripwires — break one and the concept no longer holds)*
- **Speed = count**, **Daring = grade** — both permanent Form stats, never spent; **Tempo = the cards**,
  Speed-many at Daring each, spent within a round and refreshed between rounds.
- **Daring's magnitude affects only a crossing contest** (a single simultaneous bid, ties to the
  catcher); it never scales a strike, an attack, or anything outside a crossing.
- **Every action is one Tempo card** (strike, contest, strike back); **standing and soaking are free**;
  a strike is single-card and Daring-blind.
- **Spent Tempo does not return until the round refresh** — cards bid on a crossing are unavailable for
  later strikes (the depletion tension); the bid is per-contest, the pool does not refill mid-round.
- **Reflexive strike-back is always available against a melee attacker** for one Tempo card; no card → a
  free hit. Ranged is one-sided save the no-front open brawl / a card exception.

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Speed` (Resources) — A permanent Form stat: how many **Tempo** cards you start each combat round with (the *count*). It is not a magnitude of movement and never sets turn order.
- **TERM.** `Daring` (Resources) — A permanent Form stat: the magnitude on each **Tempo** card (the *grade*). Its number matters in exactly one place — a crossing contest, where both sides commit Tempo cards (cards × Daring) and the higher total wins (ties to the catcher). A strike is the same whatever its Daring.
- **TERM.** `Tempo` (Resources) — The round's pool of action cards: **Speed**-many, each worth **Daring**. Flip one to take any action (strike, contest a crossing, strike back) — standing and soaking are free; spent cards stay spent until the round refreshes.

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

> **SUPERSEDED by §4 (static ranks).** The round is no longer a player-phase/foe-phase loop over
> formation; it is the **Assemble → the Line → the Open → Refresh** model in §4. **Tempo is now the single
> currency** (Focus/Mind merged out, 2026-06-20); order-independence is preserved *per tier*. The
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

## 4. The battle — declare ranks, hold the line 🟡 *(static-ranks model; **code synced 2026-06-21** — `combat.rs::the_line`. Remaining enrichments: the per-contest bid-magnitude escalation (dial 1) and the BI-3 force-not-fiat test.)*

> **History.** Superseded forms: front/back formation → speed-pairing → lane assignment → the
> **charge-and-gauntlet** (secret charge + threading columns, roles *emergent*). All replaced by the
> **static-ranks** model below. The *spine survives* — three roles (Vanguard / Skirmisher / Reserve),
> hidden commitment, "the front protects the back" — but the **threading gauntlet is gone**: roles are
> now **declared, not emergent**, nobody moves, and each Skirmisher's crossing is a single **Daring
> contest** against the wall, resolved in two ordered tiers. Motivation: the gauntlet's threading
> (column pairing, surplus loops, interception-across) was the one genuinely complex construct and
> bought nothing the static model can't express more cleanly. Rationale + decision trail:
> `future-possibilities.md` entry 8. *(Earlier code implemented the gauntlet; the static-ranks code
> lands in the §4 spec-sync.)*

**The budget (one pool).** **Tempo** is the action economy (§3): a `count × value` pool of
**Speed**-many cards, each worth **Daring**, flipped to spend (§5). **A flipped card buys exactly one
action — of any kind:** strike, contest a crossing (slip or catch), or strike back. **Standing in a
rank, letting a foe slip by, and *absorbing* a blow are all free** — Tempo is the currency of *acting
on the enemy*, not of mere presence. **Run out and you can no longer act** (you still hold your rank and
soak hits with Body). **Tempo refreshes each round.** **Daring's magnitude is read in exactly one
place — a crossing contest** (below); every other action is one Daring-blind card whose damage is set by
**Power**.

Because the **same pool pays for offense and defense**, the cannon/wall axis is an **allocation
choice**, not a second stat: spend Tempo catching runners and striking and you have none left to strike
back; hold it and you survive but do less. Both pools share the `count × value` form: **Health = Body ×
Toughness** (persists), **Tempo = Speed × Daring** (refreshes). *(No Focus/Mind pool — merged 2026-06-20.)*

**RULE — three declared ranks.** Each side secretly assigns every Actor a rank, then both reveal:

- **Vanguard** — the melee front line; holds, and may spend Tempo to **catch** crossing Skirmishers —
  **as many as it can pay catch-bids for** (Speed = catch breadth, Daring = catch strength).
- **Skirmisher** — the flankers; each attempts to **cross** the enemy line to reach the backfield.
- **Reserve** — the ranged / support line behind the front; fires over it, and can be reached only by a
  Skirmisher who crossed (or once the front is broken).

The counter-triangle holds — **Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard**: a Vanguard catches
Skirmishers; a Skirmisher who crosses reaches the otherwise-untouchable Reserve; the Reserve fires on
the exposed front.

**The round.**

1. **Assemble** *(the one hidden, simultaneous commit).* Each side secretly assigns ranks **and** commits
   its **bids** — how many Tempo cards each Skirmisher throws at its crossing, how many each Vanguard
   throws at a catch, and (defender's choice) **which Vanguard catches which Skirmisher** — and plays its
   **standing cards** (a Wall's Brace / Last Stand, a Controller's Slow / Stagger / Disarm, a Support's
   Mend / Haste), whose effect **lasts the round** and shapes what follows. Reveal together; nobody moves
   after.
2. **Tier 1 — the Line.** From the start-of-round snapshot, resolved together (deaths tally at the tier
   boundary — a unit mortally wounded here still lands every blow it committed, §1.3):
   - **Vanguards strike** the opposing Vanguard (a card per blow).
   - **Crossing contests** *(the only place Daring is read)*: each Skirmisher's committed **advance**
     (cards × Daring) vs its assigned Vanguard's committed **hold** (cards × Daring + Phalanx).
     Strictly greater → **slips** (the bypassed Vanguard lands a **parting free hit**, Power); ≤ →
     **held** (Skirmisher and catcher **trade**, both Power; the Skirmisher does not cross). **Ties to
     the catcher** unless the Skirmisher has **Shadowstep**. Each catch is a **separate bid** a Vanguard
     pays from its own Tempo, so a wall holds only as many Skirmishers as it can afford to contest; a
     Skirmisher **no Vanguard pays to catch slips free** — no contest, no parting hit.
3. **Tier 2 — the Open.** From the post-Tier-1 snapshot, resolved together (tally at the boundary):
   - **Crossed Skirmishers strike** — behind the line, they may hit **any** enemy rank (Reserve first,
     the prize; the Vanguard from behind; or an enemy Skirmisher). One card per strike, **act while
     cards remain**; a melee blow on a foe that can't answer melee (a Reserve) is **free**.
   - **A Vanguard whose opposite is dead pours through** and strikes the now-undefended enemy Reserve /
     Skirmisher (free).
   - **Reserves fire** ranged at the enemy Vanguard + Skirmisher (never the enemy Reserve) — picking off
     exposed Skirmishers; auto-hits any target with no ranged answer.
   - **Strike-backs:** a struck unit that can answer the range and has a card left strikes back (Power);
     otherwise it eats a free hit.
4. **Refresh.** Clear round-scoped status; refill Tempo to Speed; round++. (A battle not closed by the
   ruleset round cap is a draw — §0.4.)

**The crossing contest (the only Daring read).** A single **simultaneous Daring bid**, *not* an iterated
auction: each side has committed *k* cards at Assemble, worth *k ×* Daring; the higher total crosses /
holds, tie to the catcher (Shadowstep overrides); bid cards are spent. A Skirmisher's **advance** is
weighed against the Vanguard's **hold**: Wall powers feed **hold only**, never advance — a Wall raises
the number you must clear without itself slipping through (an immovable line, not a runner).

**Demise — protection comes from the line.** Each rank's vulnerability is *how much line stands between
it and the enemy*:

| Rank | Dies to | Safe from |
| --- | --- | --- |
| **Vanguard** (is the line) | direct engagement (enemy Vanguard); a crossed enemy Skirmisher from behind | being flanked while it holds — it faces forward |
| **Skirmisher** (left the line) | the wall's **catch / parting hit**; the enemy **Reserve's** fire; an enemy **Skirmisher** in the open | the committed enemy **Vanguard** — a holding line cannot wheel and chase |
| **Reserve** (behind the line) | a Skirmisher who **crossed**; its own **Vanguard wiped**, then enemy Vanguards pour through | everything, *while its line holds* |

So the **Skirmisher is the exposed rank** — it bought reach by giving up cover, and dies in the open. It
is thus both **spear and screen**: field it to assassinate the enemy Reserve *and* to kill the enemy's
Skirmishers before they reach yours.

**Role powers (re-homed).** Each bites a concrete step: **Phalanx** (+hold — more catch strength),
**Bulwark** (+hold to *every* allied Vanguard — the line anchors as one), **Taunt** (must be assigned
the first catch),
**Blitz** (the first slip each round is a free card), **Shadowstep** (win the tied contest), **Backstab
/ Assassinate** (a crossed Skirmisher hits an enemy Reserve harder / executes it — the §10 prize).

**Persistent status (Controller debuffs).** A Controller card hangs a round-scoped status, cleared at
Refresh: **Stagger** (no action — no strike, contest, or strike-back), **Shove** (out of melee — cannot
contest a melee blow), **Disarm** (cannot play role cards), **Rout** (driven from the Vanguard to the
Reserve — removed from the front *before* the charge resolves, so it cannot hold the line or trade this
round). Played at **Assemble**, they degrade the foe's whole round.

**Fear applies these by tier (§2.2), not damage.** Fear cleared past **Resolve / 2·Resolve / 3·Resolve**
escalates: **Freeze** = Stagger (loses its action) → **Shaken** = Stagger + Shove (also cannot defend) →
**Rout** = Stagger + Shove + driven to the Reserve (helpless and off the line). This is the Controller's
*control* in place of direct damage (§8.6). *(Tier→effect mapping and the Assemble-time Rout are seeded
2026-06-21 — tune to taste.)*

**Bypass = open season.** Once a unit is *behind the enemy line* — a Skirmisher that **slipped** past a
Vanguard, or a Vanguard that **killed** the one it faced — **every enemy rank is fair game to it**. The
line only protects what is still in front of it; bypass it (by slipping or by killing it) and the
backfield is open.

**Targeting matrix.**

| Chooser | May target |
| --- | --- |
| **Vanguard** | the enemy Vanguard it faces; once that is dead, it **pours through** to **any** rank behind it (Reserve or Skirmisher) |
| **Skirmisher** | **held** → only its catcher; **crossed** → **any** enemy rank (Reserve first, then the Vanguard from behind, or an enemy Skirmisher) |
| **Reserve** | enemy **Vanguard & Skirmishers**, and **aid own allies** — **never** the enemy Reserve |

**Edge cases.** *All-Reserve (no front):* nothing can catch, so every enemy Skirmisher slips free and
raids the Reserve — holding everyone back only exposes you. *No Skirmishers:* the lines trade and the
Reserves fire — attrition the heavier line wins. *Neither side fields a front:* the "Reserve is safe"
privilege (paid for by fielding a front) lifts — open brawl, anyone may target anyone in range.

**Protection is the wall's, this round.** Only catching a Skirmisher at the line — or screening it with
your own Skirmisher, or shooting it with the Reserve — saves the backfield this round; a wiped Vanguard
stops protecting its Reserve immediately. **No rank is ever permanently safe** — every unit is reachable
by enough force (the **force-not-fiat** invariant, §0.3 / balance-invariants BI-3).

**Determinism (two ordered tiers).** Each tier resolves from a snapshot, **order-independent within**
(permuting the units yields the identical end-state — the §1.9 property), **strictly sequenced between**
(the Line, then the Open); deaths finalize at each tier boundary, so a mortally-wounded unit still lands
every blow it committed (§1.3). The base battle is **perfect-information and deterministic given the
Assemble commit** (#11); the crossing's **bid** is a single simultaneous commitment (not an iterated
raise-war), so it stays computable. The hidden, simultaneous mind-game is the optional **Clash** (§1.0).

**What is hidden.** Only the **Assemble commit** — ranks + bids + catch-assignments — and only until the
simultaneous reveal. Everything after is open; Tempo is flipped face-up to spend. Always public: stats
(Speed / Body) and the spent/unspent pool.

**WHY.** The triangle survives on a single physical picture — a battle line with a front, flankers, and
a rear — but the **threading gauntlet is gone**: roles are declared, nobody moves, and a Skirmisher's
fate is one **Daring contest** against the wall, not a walk through a column. **Tempo as one pool spent
only on *acting*** (striking, contesting, answering) makes the core decision crisp — *where do I spend
my initiative?* — while standing and soaking stay free, so a spent wall is still a wall. **Daring read
in one place** keeps the three stats orthogonal: **Speed** = how many actions, **Daring** = whether you
cross, **Power** = how hard you hit. **Force, not fiat:** nothing is forbidden by rule — a low-Daring
Skirmisher can still cross by overspending, an out-numbered wall is overwhelmed, an over-powered lone
hero wipes a weaker party — opposition is always *cost*, never *impossibility*.

**GUARANTEES.**

- **The role triangle holds:** Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard; roles are **declared**.
- **The Reserve is reachable only by crossing the line** — a Skirmisher who beats the wall, or a
  Vanguard pouring through a broken front — never by enemy Reserve (except the no-front open brawl).
- **One hidden commit:** only the Assemble commit (ranks + bids + assignment) is hidden + simultaneous;
  all resolution is open. Base combat is perfect-information and deterministic (#11); the hidden
  mind-game is the optional Clash.
- **Two ordered tiers, snapshot-resolved:** order-independent within a tier, strictly sequenced between;
  a mortally-wounded unit still lands its committed blows (§1.3).
- **Two pools, both `count × value`:** Health (persists) and Tempo (refreshes). **Tempo buys actions
  only** (strike / contest / strike-back); standing and soaking are free. **Daring bites only in a
  crossing**; a strike is Daring-blind (Power sets damage).
- **Force, not fiat:** every rank is killable by enough force — no immunity, no hard cap, no skill-gate.
  A no-skills, infinite-stat character wipes any finite party in one round (BI-3).

**MANUAL.** *Secretly assign each Actor a rank — Vanguard (front), Skirmisher (flanker), Reserve
(rear) — and commit your bids: how many Tempo cards each Skirmisher throws at its crossing, each
Vanguard at a catch, and which Vanguard catches which Skirmisher; play your standing cards (braces,
debuffs, buffs — they last the round) in the same commit. Reveal together; no one moves. **The Line:**
Vanguards strike across (a card a blow); each Skirmisher's advance (cards × Daring) meets its catcher's
hold — strictly more slips (and takes the wall's parting hit), equal-or-less is held and trades; an
uncaught Skirmisher slips free. **The Open:** Skirmishers who crossed strike anything behind the line
(the Reserve is the prize); a Vanguard whose foe is dead pours through; Reserves fire at the enemy front
and pick off Skirmishers in the open; anyone struck at a range it can answer strikes back (a card), else
takes a free hit. Standing and soaking cost nothing — only acting spends Tempo. Refresh and go again.*

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Assemble` (Roles) — The one hidden, simultaneous commit: each side assigns every Actor a rank (Vanguard / Skirmisher / Reserve), commits its crossing / catch bids and which Vanguard catches which Skirmisher, and plays its standing cards. Revealed together; everything after resolves in the open, and nobody moves.
- **TERM.** `Vanguard` (Roles) — The declared melee front line. Holds, and may spend Tempo to catch crossing Skirmishers — as many as it can pay catch-bids for (Speed = breadth, Daring = strength); once the enemy Vanguard it faces is dead it pours through. Shields the Reserve.
- **TERM.** `Skirmisher` (Roles) — A declared flanker that attempts to cross the enemy line. Held → it trades with its catcher; crossed → it reaches the backfield, where any enemy rank is fair game. The route (besides a broken front) to the enemy Reserve.
- **TERM.** `Reserve` (Roles) — The declared ranged / support line behind the front. Fires over it and aids allies, can never target the enemy Reserve, and is reached only by a Skirmisher who crossed or a Vanguard pouring through a broken front.
- **TERM.** `The triangle` (Roles) — Vanguard beats Skirmisher (catches it at the line); Skirmisher beats Reserve (crosses to assassinate); Reserve beats Vanguard (fires from safety, untouchable in melee).
- **TERM.** `The Line` (Combat) — Tier 1: Vanguards strike across, and each crossing Skirmisher's advance Daring is weighed against its catcher's hold. Resolved from a start-of-round snapshot; deaths tally at the boundary.
- **TERM.** `Crossing` (Combat) — A Skirmisher's attempt to pass the wall: a single simultaneous Daring bid (committed cards × Daring). Strictly more than the catcher's hold slips (taking a parting free hit); equal-or-less is held and trades; an uncaught Skirmisher slips free. Wall powers raise the hold only.
- **TERM.** `The Open` (Combat) — Tier 2: crossed Skirmishers strike anything behind the line (the Reserve is the prize), a Vanguard whose foe is dead pours through, Reserves fire on the front and pick off exposed Skirmishers, and the struck strike back if they can answer the range.
- **TERM.** `Open brawl` (Combat) — If neither side fields a front, no line forms and the Reserve's safety lifts: everyone may target anyone with whatever range they carry.
- **TERM.** `Phases` (Round) — Assemble (hidden: ranks + bids + standing cards) → the Line (Vanguards trade, Skirmishers contest the crossing) → the Open (breakthroughs strike, Reserves fire) → Refresh. Order-independent within each tier, strictly sequenced between.

**Open dials (pin with implementation).** The structure (Assemble, the two tiers, the crossing contest,
the three declared roles, targeting) is settled; these are not:

> **RATIFICATION (2026-06-21).** §4 is the **static-ranks** model, and the **code is synced** —
> `combat.rs::the_line` is the **resolver-of-record** the balance work (§0.3) measures against:
> card-bound catch (one card per catch, defender-assigned, Taunt first), the Skirmisher bids the fewest
> cards to beat the hold, two snapshot tiers, ties to the catcher, Shadowstep / Phalanx / Bulwark / Blitz
> riders. The **bid is a single simultaneous commitment** (not an iterated auction), so base PvE combat
> stays a maximizer, not an equilibrium-solver — par stays well-defined. *(v1 prices a catch at one card;
> the per-contest **bid-magnitude escalation** — a Vanguard committing several cards to raise one hold —
> is the remaining enrichment, dial 1.)*

1. **Bid & free-hit magnitudes** — the contest *rule* is locked (single simultaneous bid, committed
   cards × Daring, ties to the catcher, Shadowstep overrides); the **numbers** (the parting-hit weight,
   strike Power) live in `booklet.ron`, human-tuned.
2. **Catch capacity — locked (card-bound).** A Vanguard catches as many Skirmishers as it pays
   catch-bids for (Speed = breadth, Daring = strength); **Bodyguard is retired** (its old +1 niche is
   now just the card budget).
3. **Skirmish strike cost** — one Tempo per strike (a unit acts while it holds cards); confirm whether
   switching targets costs extra.
4. **Reserve aid kit** — the buffs / heals / debuffs a Reserve delivers — Action cards over the §5 zone
   layer (the aspect/combo layer is retired — `retired-ideas.md`).
5. **Pool model — locked (§3, 2026-06-20).** Two `count × value` pools: **Health = Body × Toughness**
   (persists) and **Tempo = Speed × Daring** (refreshes). **Focus and Mind are removed.** Speed = count
   (how many Tempo cards), **Daring = grade** (per-card magnitude). **Daring is read only in a crossing
   contest**; every other action is one Daring-blind card (Power sets damage), and **standing / soaking
   cost no card at all**.

*(Range/attack dials are resolved by §4.2: "Reserve self-defense" = whether it carries melee; "strike
shape" = a Clash when attacker and target share the range, an auto-hit when they don't.)*

### 4.1 Count-adaptivity — the system degrades to the choices that exist

**RULE.** The commitment layer is **count-adaptive**: any choice with a **single legal option
resolves automatically**, presenting no decision. Rank assignment, the crossing bids, catch
assignment, and Skirmisher / Reserve targeting appear only when party size makes more than one option
legal. Concretely:

- **1 v 1** — each side has one Actor; ranks are moot (front meets front), so the two simply **trade**
  (or fight a **Clash** with the module on). No rank bluff, no crossing (one unit can't both hold and
  flank), no Reserve, no Skirmisher — it is exactly the plain duel (the tutorial case).
- **Small parties (2–3)** — only live choices surface: **rank assignment** becomes real once a second
  body makes a front-vs-flank split meaningful; a **crossing bid** only where a Skirmisher faces a
  catcher both can afford; **targeting** only with a surviving target and a legal line to it.
- **Larger parties** — the full machinery (a bluffed formation, several crossings, breakthroughs,
  pour-throughs, and screening).

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
**position-determined**, never chosen: **crossing contests and Skirmisher strikes are melee;
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

- **Skirmishers are melee** (they cross a melee line), so the **only core route to an
  enemy Reserve is a melee assassin.** Ranged Actors do **not** skirmish in the core. *(A card may
  explicitly supersede this — e.g. grant a ranged Skirmisher; see "Cards may supersede the
  core.")*
- **Reserve self-defense = whether it carries melee.** A Reserve with a melee attack **trades/Clashes**
  an assassin (fends it off by spending Tempo to clash); a pure caster (no melee) is **auto-hit**
  (assassinated).
- A **melee-less Vanguard or Skirmisher is legal but a very bad idea** — it is auto-hit when it meets
  melee and cannot answer. (Emergent positioning, not a banned move.)
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
- Range is **position-determined** (the Line / Skirmisher = melee, Reserve = ranged) — never the
  attacker's free pick.
- Core: **only melee Actors skirmish**; a card may explicitly supersede.

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Trade` (Combat) — A same-range engagement: both sides deal their base damage through armor/toughness. In the optional Clash module, the trade is replaced by the four-card mix-up.
- **TERM.** `Auto-hit` (Combat) — A range mismatch: the attacker lands uncontested (the target can't answer at that range). Armor still blunts it; Focus cannot.
- **TERM.** `Attack type` (Combat) — Each Actor is Melee, Ranged, Both, or Neither. Crossing contests & Skirmisher strikes are melee; Reserve fire is ranged. Lacking the matching attack means you're auto-hit.

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

**RULE.** Role cards are an **ability layer** over the physical battle (§4). A character may play
**one role card of each role per round** — several in a round, so long as they are **different roles**.
A god holding all five tracks fires **up to five** effects in a round; a single-role specialist fires
**one** (and chooses which of its cards), then that role is spent until next round. Each card is played
**when its effect fits**: a **Wall**'s standing defenses and the **Controller** / **Support** persistent
debuffs and buffs at **Assemble** (they last the round); an **Infiltrator**'s slip and an **Artillery**'s
fire resolve in their tier (the Line / the Open). Play is **decoupled from the body's rank**: a god fires
its effects **across the round**, *not* from five ranks at once — its body still occupies a single rank.
(The role labels are **thematic**, not a rank gate.)

**WHY.** The per-role cap is the **god-vs-party lever** (#4: god ≈ party). A god holds every track and
fires up to **five** effects in a round — but on **one body**, in one rank, that the enemy can
**focus-fire**; a five-specialist party fires the same five across **five resilient bodies**. So it is a
**concentration-vs-resilience tradeoff, not dominance** (candidate **BI-4**). The cap is **per-role, not
per-round, on purpose**: it is what lets a god **combo across Roles** — combine *different* Roles in one
round (Controller-degrade, then Artillery-fire, then Support-buff). Such combos are the god's distinctive
reward for breadth, and they are valued **precisely because the Roles differ in kind** (Charter #12) —
comboing two stat-reskins would be pointless doubling. They stay **additive / commutative**: each effect
feeds an accumulator and **no played effect multiplies or gates another's output** (the GUARANTEE below,
§0.1 / #11), so "combo" means *diverse effects concentrated in one round*, **never** a multiplying chain.
Because the **Assemble commit is simultaneous**, a card is **committed up front or resolves in its
tier** — never *held* for a more-informed moment (there is no information gradient to exploit).

**GUARANTEES.**
- One role card of each role, per character, per round; each played **when its effect fits** (Assemble
  for persistent effects; the relevant tier for slips / fire), **decoupled from the body's physical
  rank**.
- **Order-independent effects (the simultaneity constraint).** A round's role-card effects must
  **combine commutatively**: every effect feeds an **accumulator** (damage piles, heals pile, buffs add
  or set flags) resolved at the phase boundary, and **no played effect multiplies or gates another
  played effect's output.** So a god firing five effects gets the **same result regardless of order**
  (§0.1 / #11). *(Modifiers like Curse stay safe by folding into the build — passive, not a play.)*
- No party size dominates on raw role-card throughput (the #4 budget; candidate **BI-4**, which the
  par solver verifies).

*(The old **positional gate** (a card required its matching position) is **removed** 2026-06-20 — it
capped a god at ~3 and blocked the intended five-effect god; play is now moment-appropriate (Assemble or
its tier) but rank-decoupled. Code/data + `TERM` lines land with the role-card migration:
`role-card-redesign.md` §8.)*

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
gates damage) and **Tempo = Speed × Daring** (Speed-many cards, each worth Daring). *(Focus and Mind are
removed — merged 2026-06-20; defense is a Tempo spend.)* Spending moves cards to **Down**; they return
by **Recover** (or the round refresh). A gauntlet-crossing contest compares the **total Daring each side
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
commutatively**; the order-dependent **modifier** variant is part of the retired aspect/combo layer
(§6 → `retired-ideas.md`). (2) **`TERM` glossary vocabulary + encyclopedia + glossary test** —
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

## 6. Aspects / the chord — *retired*

**Retired to `retired-ideas.md` (decommissioned 2026-06-21).** The multi-deck **chord/combo** system
(a character as a set of aspect-decks; a play as one card per aspect, combined) was **dropped**: the
single-deck core — Form (fundamental + attachments) + Action cards over the §5 zones — plus the §4.4
per-suit-per-round play deliver its intent, and a fused-action chord works against Charter #2 (small,
computable tactics). `retired-ideas.md` records the full rationale and **the bar it must clear to
return**. *(Section kept as a stable §6 anchor; the heading is referenced elsewhere.)*

*(Terminology note: the two **defense channels** Body / Spirit (§2) are unaffected — they are damage
types / thresholds, not the retired deck-chord, despite the shared word "aspect.")*

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

### 8.6 The role set is necessary-and-sufficient 🟡

**RULE.** The five Roles (`3 + 2`, §8.5) are **necessary and sufficient** for the campaign, measured on
the reference scenarios under the analysis envelope (§0.4):
- **Sufficient.** A party whose **collective coverage includes all five Roles** can clear the reference
  campaign under optimal play.
- **Necessary (each Role load-bearing).** For **each** Role R, a party identical except that **R's
  coverage is removed** **fails at least one** reference scenario — the scenario that is R's *lock*.
- **Distinct.** Each Role has a **signature mechanic** (Wall: the hold / Phalanx; Infiltrator: slip /
  Blitz; Artillery: ranged fire; Controller: round-scoped status; Support: buff / heal) that is
  **invoked and outcome-changing** in at least one reference scenario; no two Roles clear their lock by
  the same mechanic.

The invariant is **campaign-scope**: an **individual** conflict may be winnable by one Role alone, or
unwinnable for the Role it is built to humble — a single-Role spotlight is a **tutorial** (§8.4) in that
Role's powers and limits, **not** a violation.

**WHY.** §8.5 establishes the role set is minimal-complete *by counting* (a triangle + an effect pair).
§8.6 makes that completeness a **measured property of play** (Charter #11, #12): "uniquely valuable"
becomes *demonstrably the only key to some lock*, "behave differently" becomes *demonstrably a different
mechanic*, and the stat layer gets its acceptance test — a stat earns its slot **iff** it lies on some
Role's load-bearing path (Charter #12: *stats serve the Roles*). Without a measure, role-necessity is a
slogan; the leave-one-out check turns it into a regression test.

**GUARANTEES.**
- The reference campaign has, for each Role, a **designated lock scenario** unwinnable without that Role,
  wired as a **regression test** (#11: the par solver is a regression test). Losing necessity for any
  Role **fails the build**.
- **Damage belongs to the triangle.** Only the three §4-triangle Roles — **Wall, Infiltrator,
  Artillery** — deal **direct damage**. The two effect Roles never do: **Controller** *degrades*
  (round-scoped status, no damage — §2.2 fear is pure control) and **Support** *augments* (buff / heal,
  no damage). A Controller or Support card that dealt direct damage would collapse the 3+2 distinction
  (**Charter #13**). *(Locked 2026-06-21.)*
- **Necessity is emergent, not by fiat.** A lock scenario makes its Role necessary through the foe's
  **stats and behaviour** (an offense you must *disable*, an armor you must *pierce*, a backfield you
  must *reach*) — **never** through an **immunity** or keyword that *bans* the other Roles. Emergence
  test: with R removed, the others are **outpaced within the analysis envelope** (§0.4), not
  **forbidden** by a rule. An immunity gate that manufactures necessity is a **defect** — it satisfies
  the necessity check while violating Charter #12 / #6. *(Systemic channel cross-immunity, §2.2, is not a
  fiat gate — it is a symmetric system, not a per-foe script.)*
- **No redundant stat:** every stat the engine carries is **read** on some Role's resolution path; a stat
  the engine never consumes is a **failing** state, not a latent one.
- The invariant is **campaign-scope** (some scenario per Role), never per-encounter; single-Role
  tutorials are intended, not breaches.
- Measured on the **core** (§0.1) under the **analysis envelope** (§0.4); like all balance claims it is
  **policy-relative** to the resolver-of-record (§0.3) and **blind to fun / feel** (the human ratifies
  those).

*(SEEDED — a designer/solver invariant graduating Charter #12; **no `TERM` line** (not player
vocabulary). The enforcing tests — sufficiency, leave-one-out necessity, no-redundant-stat, distinctness
— ride the par-solver / balance harness (§0.3, `computability-and-balance.md`); the cheap "is every stat
consumed?" check can land **ahead of** the solver. The five **designated lock scenarios** — one per Role,
doubling as the role tutorials — are an authoring task on the reference set (§8.4).)*
