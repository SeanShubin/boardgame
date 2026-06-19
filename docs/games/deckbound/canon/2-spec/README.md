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
| **Speed/Tempo + Mind/Focus**                      | 🟡 seeded    | `notes/speed-and-tempo.md`, `notes/mind-and-stances.md`                                                                                                |
| **The battle — roles & commitment order**         | 🟡 seeded    | §4 specced **and implemented** (lanes, powers, Clash, hotseat PvP); **§4.3 actors-are-decks** specced (actor-stat→deck migration pending `/spec-sync`) |
| **Zones / exhaustion**                            | 🟡 seeded    | **§5 worked** (zones · Form/Action · verbs · tags); resources 🟡 (stats-as-deck now §2.3/§4.3) — `zones-exhaustion-design.md`                           |
| **Aspects / the chord**                           | ⏸ deferred  | parked → `future-possibilities.md` (entry 4) — single-deck core first                                                                                  |
| **Agents** (Character vs Creature)                | ⬜ stub      | `notes/entities.md`, `notes/decision-making.md`                                                                                                        |
| **Strategic layer** (world/event decks)           | 🟡 seeded    | **§8** (world · clock · role-card rewards · encounters · progression) — `progression-design.md`                                                        |
| **Skirmish victory / defeat**                     | 🟡 seeded    | `notes/form-and-defeat.md` (eliminate the foes / the party falls; in code)                                                                             |
| **Run victory / defeat** (across many skirmishes) | 🟡 seeded    | **§8.2** — victory = clear the objective, scored in Days (golf); **defeat deferred** pending reference-scenario tuning                                 |
| **Geography & travel** (the world map + movement) | 🟡 seeded    | **§8.1** (locations · move 1/Day · fog); travel risk deferred — `progression-design.md`                                                                |
| **Loot / role cards** (clear → reward)            | 🟡 seeded    | **§8.3** — atomic 25-card role-reward pool, scarce, party-assigned permanently; **no currency** (role-card redesign, *migration pending*) — `role-card-redesign.md`         |
| **Progression** (growth between skirmishes)       | 🟡 seeded    | **§8.5** — role = assigned cards · `3+2` tracks + bundled Stat layer · depth/breadth; play rule §4.4, taxonomy §5.6 (*migration pending*) — `role-card-redesign.md`         |

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
> mix-up + Force when a scenario enables it. Everything in §3–§4 (lanes, roles, phases,
> Tempo/Focus) runs identically either way.

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

> **Naming.** The three defensive dimensions — **Body · Mind · Spirit** — are the **channels**.
> The word *aspect* is **reserved** for the deferred deck-chord combo layer (§6) and is **not** used
> for these. *(The frozen `notes/` still call the channels "aspects" — read that as "channels".)*

### 2.1 One maintained meter

**RULE.** Exactly **one** quantity is a maintained, depleting track: the **Body
Health pool** (face-down cards, per-combat, restored on a win). Every other
defensive quantity — **Armor, Ward, Toughness, Resolve, Mind-capacity** — is a
**passive stat read off the table**, never spent. **Tempo** and **Focus** are
ephemeral per-round counts, re-derived each round, not maintained.

**WHY.** "You maintain exactly one meter" is the load-bearing comprehensibility
rule (Charter §7, §9): a human can hold the whole game because only one number is
ever in flux.

**GUARANTEES.**
- Nothing besides Body Health is ever "tracked" between rounds.
- Every other defensive number can be reconstructed from the cards on the table.

### 2.2 Every channel is cut → bar, and only Body has a pool

**RULE.** Each attack is **outer** (physical/elemental → Body) or **inner**
(fear → Spirit, confusion → Mind). It resolves: **subtract the cut** (Armor for
outer, Ward for inner; per source, typed, never depletes) → **accumulate the
remainder into the round's pile** → **compare the pile to the bar** (Toughness for
Body, Resolve for Spirit, Mind-capacity for Mind). Only the **outer** channel has a
**pool** (Health cards) behind the bar; inner channels **break** when the pile
exceeds the bar, with no pool. Cross-immunity: outer ignores Ward; inner ignores
Armor.

**WHY.** A per-source cut answers *many small hits*; a high bar answers *any one
big hit* — non-redundant, so you want both. Typing the cut makes "called shots"
fall out for free (choosing a damage type chooses which channel you attack).

**GUARANTEES.**
- The three channels are structurally parallel (offense · cut · bar · [pool]).
- An inner break is a this-round event that clears at round end — **except**
  scared-to-death, the one inner result that bleeds into the Body pool.
- Accumulation is always cards in a zone, never a number in the head.

*(SEEDED — the damage formula, scaling, and Resolve/Mind break thresholds are not
yet specced. Numbers live in `booklet.ron`.)*

### 2.3 Stats live in the deck — *stats-as-deck*

**RULE.** A character has **no stats on its identity card** — it is a **bare Actor** (a name and a
map token, §8.1). **Every stat is read off its deck**, from the **Form** zone (§5.2): a
**fundamental card** sets the base, **attachment** cards modify each dimension. So §2.1's "passive
stats read off the table" — **Armor, Ward, Toughness, Resolve, Mind-capacity**, and likewise
**Speed, Power, Mind** — are **Form-derived**, never authored fields. The **Body Health pool** is the
**count × value** Form pool, and the **Tempo / Focus** pools are sized by the Form's Speed / Mind
(§5.5).

**WHY.** "The deck *is* the character" (#8) made literal: it removes a redundant authored stat-block,
so *getting stronger = adding cards* (the Upgrade economy, §8.3), and **§2.1's "read it off the
table" now extends from defense to every stat**. Clean slate = a bare Actor with a minimal Form;
specialization = accreted attachments (§8.5).

**GUARANTEES.**
- No stat exists except as a Form card on the table — nothing is authored on the Actor.
- The §3 / §4 economy is unchanged in behavior; only the **source** of a stat moved (card → deck).
- A bought Upgrade is a Form attachment (or an Action card, §5) — power grows by cards.

---

## 3. Speed/Tempo + Mind/Focus — *the two breadth budgets* 🟡

Design source: [`notes/speed-and-tempo.md`](../notes/speed-and-tempo.md).

Tempo and Focus are **pure breadth** — they decide *which duels you are a full participant
in*, never *which cards you hold* (the kit is always complete, §1.0). They **mirror in sizing
and cost** but differ in **role**: **Tempo** is **initiative** (the duels you *start*),
**Focus** is **reaction** (the duels *started on you*). Speed sizes Tempo, Mind sizes Focus,
and dealing with any foe costs **that foe's Speed** on whichever axis. Each is independently
hard-capped by its stat; there is no coupling between them (§3.3 removed).

### 3.1 Tempo — admission to the duels you start

**RULE.** **Speed** is a fixed stat; it sizes your **Tempo** pool (= Speed, refreshed each
round) and is the **price others pay** to engage you. Spend Tempo to **initiate a duel** (cost
= the foe's Speed): inside you have your full kit and **results stick** — you can damage or
kill. **Counterattacking** a duel started on you also costs Tempo (§3.2), so **every kill —
initiated or countered — draws from this one pool**. Pay-**after**: you may take the action
that drives Tempo **negative** (so even a fighter too slow to afford a foe still gets one
action); that action is your **last** for the round. Speed sets budgets and thresholds only —
it **never** sets resolution order (§1.9).

**WHY.** A single capped offense pool keeps kill output on one tunable dial — the linchpin of
the god ≈ party-total budget (a god clears at most Tempo-many foes, never farming extra kills
off defense). Pay-after guarantees the slow fighter an action and makes the **negative line,
not zero, the wall**.

**GUARANTEES.**
- Kill output is capped at one pool (Tempo); offense and defense are separate dials.
- Tempo is re-derivable from Speed minus visible actions (no token needed).
- Speed sizes budgets / sets thresholds, never initiative or who-goes-first.

- **TERM.** `Tempo` (Resources) — Offense budget (= Speed). Spent to slip a lane and to pick Skirmisher/Reserve targets; the cost is the opponent's Speed.

### 3.2 Focus — admission to the duels started on you

**RULE.** **Focus** is sized to **Mind** (refreshed each round). Spend Focus to **defend** a
duel started on you (cost = the attacker's Speed): you play the full duel (§1.0), **but the
attacker is reset afterward** — you can avoid, survive, and disengage, but you **cannot damage
the attacker**. Defense is **survival, never victory**. A foe your Focus cannot cover
**free-hits** you (you eat the blow, no duel; **Toughness** absorbs what lands). When attacked
you may instead spend **Tempo to counterattack** (§3.1) → a full **mutual** clash where
results stick both ways and the trade is live.

**WHY.** Routing defense through its own pool keeps the god ≈ party budget linear; making a
defense **reset the attacker** means defending never deals damage, so being swarmed cannot
*feed* you (no free counter-kills) — numbers stay a real threat and a god plays as a
**pressured duelist**, not a counterattack reaper. Counterattacking costs Tempo, so the
single-kill-vector property holds.

**GUARANTEES.**
- A Focus-defense deals **no** damage to the attacker (survival only); the only way to win is
  Tempo (initiate or counterattack).
- Fast attackers cost more Focus to cover (inverse telegraph); overflow free-hits.
- "Negate many" is even in total across builds, capped per body.

Under the §4 battle system, Focus also pays for **blocking** — a Vanguard holding its lane spends
Focus = the slipper's Speed to stop a Vanguard trying to slip past (a funded block wins). This is
how the wall protects the Reserve: deny the slip, and no Skirmisher is created to reach the back.
A Vanguard that **tries to slip but cannot afford it** eats a **free hit** (the holder strikes
it) — an attack-of-opportunity, not a contradiction of "a self-defense deals no damage."

*(SEEDED — exact cover/drain numbers are open; `booklet.ron` / appendix.)*

- **TERM.** `Focus` (Resources) — Defense budget (= Mind). Spent to block slips and to survive incoming hits; the cost is the attacker's Speed. Fast-but-thin slips well; high-Mind holds the wall.

### 3.3 Overextension — *removed*

> **REMOVED.** The old **Exposed / Focus→0** penalty (overextending Tempo zeroed your Focus)
> is gone. Tempo and Focus are **independent** breadth pools, each hard-capped by its stat,
> and the offense/defense balance now lives entirely in the **Speed-vs-Mind split** — a
> high-Speed/low-Mind fighter natively attacks widely but defends poorly, and the reverse —
> so no coupling penalty is needed. **Pay-after is kept** (§3.1): the action that drives a
> pool negative still happens and is your last, but it carries **no extra penalty**.

### 3.4 The round — orchestration (PvE & PvP)

> **SUPERSEDED by §4 (commitment-order battle system).** The round is no longer a
> player-phase/foe-phase loop over front/back formation; it is the **Vanguard → Skirmisher →
> Reserve** declaration + three-phase resolution in §4. **Tempo (§3.1) and Focus (§3.2)
> remain the currencies** (costs = the opponent's Speed, pay-after), and order-independence is
> preserved *per phase*. The PvE/PvP text below is kept for design history; where it conflicts
> with §4, §4 wins.

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

## 4. The battle — roles, lanes & commitment order 🟡

> **History.** This section has been through two superseded forms: a **front-line / back-line
> + gauntlet** formation, then a **speed-pairing** commitment order (Vanguard matched by
> Speed, interposition to protect Reserve). Both are replaced by the **lane-based commitment
> order** below — same *concept* (roles by when you commit; Vanguard protects Reserve), new
> *mechanics* (you assign Vanguard to **lanes** and may **stack** them; protection is holding
> a lane, not a later interpose). Carry-over note: there is **no "front line never empty"
> rule** — fielding no Vanguard is legal (and, vs a fielded foe, self-defeating). The **code
> now implements this section** (the lane round in base mode, the optional 1v1 Clash module,
> the seven powers as passives, and hotseat PvP); manual lane assignment in PvP and a couple
> of cost dials remain first-pass. Old text is in git history.

**RULE.** Roles are **Vanguard** (committed to the front) and **Reserve** (everyone else);
**Skirmishers** are made mid-round — a Vanguard that **slips** its lane. The spine is a
counter-triangle: **Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard** — the Vanguard holds the line
and strikes first (stopping Skirmishers); a Skirmisher slips the wall to reach the otherwise
untouchable Reserve; the Reserve fires on the exposed front from safety.

A round **interleaves declaration and resolution in information order.** Every declaration is
**cross-side simultaneous** (both commit hidden, reveal together — never reveal-first); each
later choice acts on the *already-resolved, public* board, which is what makes the gradient:

1. **Vanguard count.** Both sides secretly pick how many Actors to commit to the Vanguard
   (`0`..party) → reveal (physically: face-down **number cards 0–9**). **Lanes = the smaller
   of the two counts.** Everyone not committed is **Reserve**.
2. **Lane assignment.** Both sides secretly assign their Vanguard across the lanes — **≥1 per
   lane**; the side that committed more **stacks** its surplus into chosen lanes (local
   superiority). **Decoy cards hide which** lanes are overstacked → reveal.
3. **Hold or slip.** In each lane every Vanguard secretly chooses **hold** (stand, fight the
   lane, and **block** slippers) or **slip** (try to leave → become a Skirmisher) → reveal.
   Slipping costs **Tempo = the lane's combined enemy Speed**; a holder blocks with **Focus =
   the slipper's Speed**, and a **funded block wins** (defense beats a slip). **Stack** your
   slippers to exhaust the holders' Focus so the overflow gets through; attempt a slip you
   can't afford and you take a **free hit** and stay.
4. **Resolve the Vanguard phase** — lane clashes and free hits (downs at the phase tally).
   Slippers who **survive become Skirmishers**.
5. **Skirmisher targets.** Surviving Skirmishers secretly target **anyone** → reveal. **Resolve
   the Skirmisher phase.**
6. **Reserve targets.** Surviving Reserve secretly target **anyone except enemy Reserve** →
   reveal. **Resolve the Reserve phase.**
7. **Refresh** — Tempo/Focus reset; Body persists; round++.

Each phase is **order-independent** (act from phase-start state, downs finalized at a phase-end
tally — the §1.9 tally, scoped to the phase). The gradient is automatic: Skirmishers choose
after the Vanguard phase has resolved, Reserve after the Skirmisher phase — so a Reserve slain
in the Skirmisher phase is simply gone before it can choose (assassination **interrupts** for
free).

**Targeting matrix.**

| Chooser        | May target                                                                         |
| -------------- | ---------------------------------------------------------------------------------- |
| **Vanguard**   | the enemy Vanguard **sharing its lane**                                            |
| **Skirmisher** | **anyone** (it slipped the wall)                                                   |
| **Reserve**    | enemy **Vanguard & Skirmishers**, and **aid own allies** — **never** enemy Reserve |

**Zero lanes — a side fields no Vanguard.** Lanes = the smaller count, so committing 0 Vanguard
makes 0 lanes. Two cases, kept distinct so a wall can never be *bypassed*:

- **One side at 0, the other fielded a front** — the 0-side presented no wall: it has no
  Vanguard, hence **no Skirmishers** (it cannot reach the enemy Reserve), while the enemy's
  Vanguard, with no lane to hold, **advance as Skirmishers** and raid the 0-side's Reserve
  freely. Declaring 0 only **exposes** you — it never unlocks the enemy Reserve. No exploit.
- **Both at 0** (e.g. two militia mobs — "no one wants to get close") — no front exists
  anywhere, so the privilege "**Reserve is safe from enemy Reserve**", which is *paid for by
  fielding a front*, **lifts**: it is an **open brawl** — no melee lanes, and **everyone may
  target anyone**.

**In-round protection is the wall's alone.** Because phases resolve in order, only **holding a
lane** (blocking a slip, in the Vanguard phase) can save a Reserve **this round** — it stops the
raider before it becomes a Skirmisher. A **Skirmisher → Skirmisher** trade (same phase, both
land) and a **Reserve → Skirmisher** shot (later phase) are **attrition**: they deny the raider
*next* round, they do not shield the target *this* one.

**Reveal timeline — what is hidden until when.**

| Hidden information                                              | Revealed at       |
| --------------------------------------------------------------- | ----------------- |
| Vanguard **count** (the bluff)                                  | step 1            |
| **Lane assignment** (which lanes are overstacked)               | step 2            |
| **Hold/slip** choices (and blocks)                              | step 3            |
| **Skirmisher targets**                                          | step 5            |
| **Reserve targets**                                             | step 6            |
| Each fighter's **Clash card** that beat                         | per beat, in-duel |
| *Always public:* stats (Speed/Mind/Body) and the **Focus pool** | from the start    |

**Costs.** **Tempo = offense** — slipping a lane, or a Skirmisher/Reserve target-pick — cost =
the opponent's Speed. **Focus = defense** — blocking a slip, or surviving an incoming hit —
cost = the attacker's Speed. Pay-after applies. **Tempo is hidden** (counts and assignments are
bluffed); **Focus is public** (the later, informed choices depend on defensive state being
known). This is the cannon/wall axis: fast-but-thin-Mind slips well and holds poorly;
high-Mind-but-slow holds the lane but cannot slip.

**WHY.** The **role triangle** gives every role a distinct value *and* a predator: Vanguard
holds-and-strikes-first (beats Skirmishers), Skirmishers slip the wall (the only path to the
enemy Reserve), Reserve fires from safety and is untouchable by Vanguard. **Lanes** add what
speed-pairing lacked — **chosen matchups** and **force concentration** (stack a lane to push
Skirmishers through). Protection is **one upstream layer** — *hold the lane* — not a second
interpose step: cleaner, but it means losing a lane *is* the assassination. **"Vanguard protects
Reserve"** stays load-bearing (the only route to the enemy Reserve is a Skirmisher who slipped a
lane), so **to threaten their back you must expose your front** and the all-Reserve hoard is
self-defeating — no "must field a Vanguard" rule needed. **0 lanes = mutual refusal of melee →
open brawl**, which is the only time Reserve loses its safety. The **hidden count + decoy lane
assignment** make wall depth and concentration a bluff (matching-pennies). The info gradient is
just "**act after the prior phase resolves**", which also hands you the "kill the caster before
it fires" interrupt for free. Speed reads as **slipperiness** — the tax to slip you or stop you.

**GUARANTEES.**
- **The role triangle holds:** Vanguard ▸ Skirmisher ▸ Reserve ▸ Vanguard.
- **Reserve is reachable only through the wall** — never by enemy Reserve, never by a lane-bound
  Vanguard; only by a Skirmisher (a Vanguard that slipped a lane) — *except* the 0-lane open
  brawl.
- **No wall-bypass:** declaring 0 Vanguard never reaches the enemy Reserve (it only exposes
  you); open brawl requires **mutual** 0.
- **Order-independent within each phase** (phase-start state, phase-end tally); phases run
  Vanguard → Skirmisher → Reserve.
- **No reveal-first:** every declaration is **cross-side simultaneous**; hidden info becomes
  public only at its step's reveal (timeline above); the gradient is round-scale, never
  beat-scale.
- **Cannon/wall axis preserved:** Tempo (hidden, offense — slip/target) and Focus (public,
  defense — block/survive) stay split; both costs scale with the **opponent's Speed**.

**MANUAL.** *Secretly pick how many go to the Vanguard (number cards); the smaller count sets
the lanes, everyone else is Reserve. Assign your Vanguard to the lanes — stack to overwhelm,
bluff which. Each Vanguard holds (fight + block) or slips past to become a Skirmisher: slipping
costs Tempo, blocking costs Focus, a funded block wins. Resolve the front; survivors who slipped
are Skirmishers and may hit anyone; then Reserve fire on the enemy front (never enemy Reserve)
and aid allies. No Vanguard on either side → open brawl, hit anyone.*

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Vanguard` (Roles) — Your committed front line. Vanguard hold lanes (and block slips) or slip past to become Skirmishers. They strike first and shield the Reserve.
- **TERM.** `Skirmisher` (Roles) — A Vanguard that slipped its lane. Skirmishers may target anyone — the only path to the enemy Reserve. They act after the front resolves.
- **TERM.** `Reserve` (Roles) — Everyone not in the Vanguard: decisive but fragile (artillery, support). Acts last with the most info; fires on the enemy front and aids allies, but can never target the enemy Reserve.
- **TERM.** `The triangle` (Roles) — Vanguard beats Skirmisher (holds the wall, strikes first); Skirmisher beats Reserve (slips in to assassinate); Reserve beats Vanguard (fires from safety, untouchable in melee).
- **TERM.** `Lanes` (Lanes) — The number of lanes is the smaller side's Vanguard count. Each lane is where opposing Vanguard meet; the larger side stacks its surplus.
- **TERM.** `Stacking` (Lanes) — Putting more than one Vanguard in a lane to overwhelm its wall — local superiority. The count, and where you stack, is the strategy.
- **TERM.** `Hold vs Slip` (Lanes) — Each Vanguard holds (fight the lane and block slips) or slips (try to leave and become a Skirmisher). Slipping costs Tempo = the enemy lane's combined Speed.
- **TERM.** `Block` (Lanes) — A holding Vanguard spends Focus = the slipper's Speed to stop a slip; a funded block wins. Overwhelm the wall's Focus (stack slippers) to push through.
- **TERM.** `Zero lanes` (Lanes) — If a side fields no Vanguard there is no front: it's exposed (no exploit — you still can't reach their Reserve). If neither fields a front, it's an open brawl — anyone may hit anyone.
- **TERM.** `Phases` (Round) — Muster (set Vanguard/Reserve) → Assign (place lanes) → Slip (hold/slip) → Vanguard resolves → Skirmishers strike → Reserve acts → refresh. Order-independent within each phase.

**Still unspecified (open dials — pin before/with implementation).** The structure (lanes,
phases, targeting, reveal order, triangle) is settled; these are not:

1. **Slip/block resolution numbers** — the tie rule is set (**a funded block wins**), but the
   exact cost coefficients and how a *stacked* lane's combined Speed prices a slip need pinning.
2. **Stacking caps** — is lane-stacking unbounded? Is there a cap on slippers per lane, or on how
   many slips one holder's Focus can block?
3. **Smaller side's assignment** — it is forced to 1-per-lane but still chooses *which* fighter
   faces *which* lane (matchup choice) — confirm and state.
4. **Vanguard's Tempo cost** — does committing/holding a Vanguard cost Tempo, or only slips and
   Skirmisher/Reserve target-picks?
5. **Failed-slip free hit** — exact magnitude of the hit eaten when a slip is unaffordable.
6. **Zero-lane asymmetric details** — the unopposed Vanguard "advance as Skirmishers": do they
   pay Tempo, and in which phase do they strike?
7. **Reserve "aid allies" kit** — the buffs/heals/debuffs Reserve deliver — Action cards over the
   §5 zone layer (the aspect/combo layer is deferred — `future-possibilities.md`).
8. **Acting across phases** — caps on one Actor doing several things (a holder blocking several
   slippers; a multi-action god across phases) — governed by Tempo/Focus, exact limits a dial.

*(Two former dials are now resolved by §4.2 Range & attack type: "Reserve self-defense" =
whether the Reserve carries a melee attack; "strike shape" = a Clash when attacker and target
share the range, an auto-hit when they don't.)*

### 4.1 Count-adaptivity — the system degrades to the choices that exist

**RULE.** The commitment layer is **count-adaptive**: any choice with a **single legal option
resolves automatically**, presenting no decision. The count bluff, lane assignment, hold/slip,
and Skirmisher/Reserve targeting appear only when party size makes more than one option legal.
Concretely:

- **1 v 1** — each side has one Actor; the only non-degenerate line is to commit it as Vanguard,
  so the two share the one lane and fight a **single Clash**. No count bluff, no lane-assignment
  choice, no meaningful slip (slipping just delays the same fight), no Reserve, no Skirmisher —
  it is exactly the plain duel (the tutorial case).
- **Small parties (2–3)** — only live choices surface: lane assignment becomes a real choice once
  you have surplus Vanguard to stack; **hold/slip** only where both options are affordable;
  **Reserve targeting** only when you have a surviving Reserve and a legal target.
- **Larger parties** — the full machinery (bluffed counts, decoy lane assignments, multiple
  slippers, stacked lanes).

**WHY.** Complexity should scale with the number of bodies. The protection layer only *means*
something when you have an ally to protect, so it must be invisible until then — keeping 1 v 1
the clean Clash and ensuring the interface never shows an option that cannot matter at the
current head-count.

**GUARANTEES.**
- 1 v 1 reduces to the §1.0 Clash with **zero** added decisions.
- A choice is presented **iff** it has ≥2 legal options; single-option phases auto-resolve.
- Adding bodies only *adds* choices; it never changes how the smaller case played.

### 4.2 Range & attack type — melee, ranged, both, or neither

**RULE.** Every Actor's offense is **melee**, **ranged**, **both**, or **neither**. Range is
**position-determined**, never chosen: **lane combat and Skirmisher strikes are melee; Reserve
fire is ranged.** A strike lands at its range; whether the target can **contest** it depends on
owning an attack of that same range:

- **Same range (target can contest)** → in the **deterministic base**, a **simultaneous trade**
  (both deal their base through armor/toughness, §2). When the **optional Clash module** (§1.0)
  is on, the trade is replaced by the four-card Clash + Force.
- **Range mismatch (target cannot contest)** → an **auto-hit**: uncontested, no mix-up, no Force,
  but still through the target's armor/toughness. Armor blunts an auto-hit; **Focus cannot**
  (Focus contests trades/Clashes and blocks slips, not off-range fire).

The **Clash is a module, not the floor** — the game is fully playable with same-range = trade
(see `future-possibilities.md` Entry 3: the strategic layer is rich without RPS).

What follows from it:

- **Skirmishers are melee** (they come from melee lanes), so the **only core route to an enemy
  Reserve is a melee assassin.** Ranged Actors do **not** skirmish in the core. *(A card may
  explicitly supersede this — e.g. grant a ranged Skirmisher; see "Cards may supersede the
  core.")*
- **Reserve self-defense = whether it carries melee.** A Reserve with a melee attack **Clashes**
  an assassin (fends it off); a pure caster (no melee) is **auto-hit** (assassinated).
- A **melee-less Vanguard in a lane is legal but a very bad idea** — it is auto-hit by its lane
  opponent and cannot answer. (Emergent positioning, not a banned move.)
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
- Range is **position-determined** (lane / Skirmisher = melee, Reserve = ranged) — never the
  attacker's free pick.
- Core: **only melee Actors skirmish**; a card may explicitly supersede.

**Glossary.** *(Encyclopedia terms — generated from these `TERM` lines into the in-app reference.)*

- **TERM.** `Trade` (Combat) — A same-range engagement: both sides deal their base damage through armor/toughness. In the optional Clash module, the trade is replaced by the four-card mix-up.
- **TERM.** `Auto-hit` (Combat) — A range mismatch: the attacker lands uncontested (the target can't answer at that range). Armor still blunts it; Focus cannot.
- **TERM.** `Attack type` (Combat) — Each Actor is Melee, Ranged, Both, or Neither. Lanes & Skirmisher strikes are melee; Reserve fire is ranged. Lacking the matching attack means you're auto-hit.

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

### 4.4 Role-card play — one per role per round 🟡 *(migration pending)*

**RULE.** A character may play **at most one role card of each role per round** — so it may play
several role cards in a round, as long as they are **different roles**. A **positional** role card
(Wall / Infiltrator / Artillery) is playable only from the matching §4 position (Vanguard / Skirmisher /
Reserve); an **effect** role card (Support / Controller) is **position-agnostic**.

**WHY.** The per-role cap is the **god-vs-party lever** (#4: god ≈ party). A god holds every track, but
— one body in one position — it plays roughly *one positional + the two effect* cards per round, while a
five-specialist party plays *~five* across five bodies: a **throughput tradeoff, not dominance** (the
god trades throughput + resilience for best-of-pool flexibility). Positional coherence reins the god in
**emergently** — one body cannot occupy three positions (#9, a rule that falls out of the fiction).

**GUARANTEES.**
- One role card of each role, per character, per round; a positional card requires its position.
- No party size dominates on raw role-card throughput (the #4 budget; candidate **BI-3**, which the
  par solver verifies).

*(Positional coherence is the **current** rule — the designer may revisit it later — `future-possibilities.md`.
Code/data + `TERM` lines land with the role-card migration: `role-card-redesign.md` §8, Phase 2.)*

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

### 5.5 Resources — Health · Tempo · Focus 🟡

**RULE.** A permanent **Form stat sizes a fluctuating pool** — you spend the pool, never the stat
(§3.1): **Toughness/Body → Health**, **Speed → Tempo**, **Mind → Focus**. Each pool is a
**count × value** card-pile in Active; spending moves cards to **Down**; they return by **Recover**.
- **Round refresh** *(Tempo / Focus)* — at Round end all spent Tempo/Focus flip up (re-derived each
  Round, §2.1) — per-Round budgets, not cross-Round attrition.
- **Heal cards** *(Health)* — Recover Health within a fight.
- **Refresh engines** — a Lasting card that Recovers Tempo/Focus mid-Round (how a god exceeds base
  breadth).
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

### 5.6 Role-card taxonomy — Base · Modifier · Mode · Stat 🟡 *(migration pending)*

**RULE.** A **role card** (§8.3) is exactly one of four kinds:
- **Base** — *played* from Hand; the track's core effect (normal §5.3 zone behaviour).
- **Modifier** — *passive*, lives in **Active** (§5.1); auto-applies to its Base (the scaling card),
  **never separately played** — so a base and its upgrade coexist under the §4.4 per-role cap.
- **Mode** — *played*; an alternative / charged Base (e.g. spend a round for a bigger effect),
  **mutually exclusive with the Base that round**.
- **Stat** — a **Form attachment** (§2.3 / §5.2): contributes to the stat block, **not played**.

**WHY.** The split lets richer high-level rewards (#5 power-up, §8.3) coexist with the **one-card-per-
role-per-round** cap (§4.4): Modifiers and Stats ride free; only **Base + Mode** plays count. It reuses
the existing **passive-power vs played-action** distinction (§5.2), so it is not new machinery.

**GUARANTEES.** A reward's cards are **self-contained** — its Modifiers / Stats apply *within* the set;
**no cross-reward multiplicative combo** (§0.1). *(Code/data + `TERM` lines land with the role-card
migration — `role-card-redesign.md` §8, Phase 2.)*

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
tuned from data, not guessed. Numbers throughout are `booklet.ron`, human-tuned. **Two migrations are
pending in code:** the **role-card redesign** (the currency/Upgrade economy still runs — see
`role-card-redesign.md` §8) and **stats-as-deck** (§2.3 / §4.3 / §5.5).

### 8.1 The world — locations, movement, fog

**RULE.** The world is **face-down location cards** in a scenario-authored layout — a **grid**, an
**offset-hex** field (alternate rows shifted half a card), or a mix. A character's **identity card**
(its Actor) marks where it is. Entering a location **flips it face-up** (revealing its name → its
**Currency type** → the **threat deck** it draws from, §8.4) but does **not** start a fight.
Movement is **one adjacent space per Day** (§8.2). *(Travel cost / risk beyond this is deferred.)*

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

### 8.3 Rewards & role cards 🟡 *(migration pending)*

**RULE.** Clearing **level X of role-track Y** unlocks the **reward** for `(Y, X)`: a fixed, **atomic
set** of cards — role-effect card(s), a bundled generic **Stat** card, and any passive **Modifier**
(§5.6) — **one physical copy each** (scarce). The **party assigns the whole set, permanently, to one
character.** Five tracks × five levels = **25 rewards**. **No currency** — clearing *is* the unlock
(clear level N of a track ⇒ its levels 1..N). Each card prints its `(role, level)` **provenance**, so a
set is identifiable and stays together.

> **Replaced (2026-06-19) — the currency economy.** §8.3 was *Currency & loot*: clearing earned typed
> **Currency** (Iron/Silver/Brass/Bone/Salt + generic Gold) that bought stat **Upgrades**, balance
> recomputed `earned − spent`. The redesign drops the currency *middleman* — clearing unlocks a
> role-card reward **directly** (the depth/breadth fork lives in routing). The five currencies survive
> only as **track colours/identities**; generic **Gold** becomes the bundled **Stat layer**, not a
> currency. (The *co-location* spend rule was already cut as bookkeeping.) Full design + migration plan:
> [`role-card-redesign.md`](../../role-card-redesign.md).

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
- One physical copy per reward; each card prints its `(role, level)` provenance, so scarcity and
  atomic assignment are legible / self-enforcing.

### 8.4 Encounters — the parametric deck-recipe

**RULE.** Combat at a location is **opt-in at a chosen level**. On first engagement a single
**encounter card** is drawn from the location's **threat deck** (one deck **per role track** — five)
and then **fixed**: it is the location's **persistent, learnable threat** (retrying faces the
*same* fight). The encounter card is a **parametric deck-recipe** evaluated at the attempted level —
a roster and **thematic** stat-scaling (which stats scale signals the counter to bring). The **level
is one dial scaling reward and threat together**.

**WHY.** Each threat deck is a **diegetic tutorial** — you meet track-C threats and unlock the **track-C
role cards** that answer them (#1 reward intellect; #6 emergence). A fixed, learnable threat means
failure teaches (#1); one dial keeps the risk/reward choice honest and re-derivable (#2 / #10).

**GUARANTEES.**
- Reveal gives the **type** (threat deck), never the exact card before you commit a fight.
- A failed clear costs a Day and the threat persists; you advance only by beating it at the depth
  you want.

### 8.5 Progression & roles 🟡 *(migration pending)*

**RULE.** A character **is its assigned role cards** — "role" is *emergent*, not a label, and roles
only **accrete** (assignment is permanent, §8.3). The five **role tracks** are the §4 triangle's
**`3 + 2`** — **Wall · Infiltrator · Artillery · Controller · Support** — each with the track
colour/identity it banks (the former currency colours **Iron · Silver · Brass · Bone · Salt**). A
generic **Stat layer** is **bundled into every reward** (the old generic currency, **Gold**, is gone —
now a stat-card pairing, not a currency or a sixth role). A character's **first clear commits a
direction**; from there it **specializes** (depth: pour one track) or **branches** (breadth: cover
several). Party size sets the spectrum: many bodies → specialists (one track each); few → multi-track;
one → a **god** spanning all five.

**WHY.** Characters are deliberately unbalanced; coverage and challenge come from the **team and the
scenario** (#4). Depth-vs-breadth is the uncomputable strategic fork (#2), fractally at map and build
scale; the party-size spectrum **is** the god ≈ party-total balance budget (#4). Role-as-assigned-cards
makes "god ≈ party" *concrete* — the **same** shared pool, distributed — and the per-role play cap
(§4.4) is what equalizes their throughput.

**Why exactly five — `3 + 2`.** The role set is the *smallest complete* one on both of combat's axes,
so the count is re-derivable, not arbitrary (#10):
- **Three positional roles = the §4 counter-triangle's vertices:** **Wall = Vanguard** (hold),
  **Infiltrator = Skirmisher** (slip past the wall), **Artillery = Reserve** (fire from safety).
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
- Five role tracks (the `3 + 2`); the generic is a **Stat layer**, not a currency.
- A solo god ≈ a full party in total power (the budget difficulty is tuned against).

*(SEEDED — §8 is the strategic layer's first graduation. The **role-card redesign** (this §8.3 / §8.5
plus §4.4 / §5.6) is graduated as *intent* but **not yet in code** — the currency/Upgrade economy still
runs; the migration is Phases 1–4 in [`role-card-redesign.md`](../../role-card-redesign.md) §8. The
**stats-as-deck** power mechanism (§2.3 / §4.3 / §5.5) is also a pending `/spec-sync`. **Travel risk**,
**per-day abilities**, **world events**, and **run-level defeat** are deferred (the last until
reference-scenario testing). Numbers are `booklet.ron`, human-tuned. `TERM` glossary lines + encyclopedia
land with the `/spec-sync §8` code pass.)*
