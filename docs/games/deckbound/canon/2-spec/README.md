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

---

## Coverage

| System                                            | Spec status | Current design source if not yet specced                                   |
| ------------------------------------------------- | ----------- | -------------------------------------------------------------------------- |
| **The Clash** (tactical core)                     | ✅ worked    | —                                                                          |
| **Defense model** (cut → bar → pool)              | 🟡 seeded    | `notes/stats.md`, `notes/form-and-defeat.md`                               |
| **Speed/Tempo + Mind/Focus**                      | 🟡 seeded    | `notes/speed-and-tempo.md`, `notes/mind-and-stances.md`                    |
| **Formation, reach & the gauntlet**               | ✅ worked    | —                                                                          |
| **Zones / exhaustion**                            | ⬜ stub      | `notes/zones.md` *(needs post-Duel rewrite)*                               |
| **Aspects / the chord**                           | ⬜ stub      | `notes/decks-and-aspects.md`                                               |
| **Agents** (Character vs Creature)                | ⬜ stub      | `notes/entities.md`, `notes/decision-making.md`                            |
| **Strategic layer** (world/event decks)           | ⬜ stub      | `notes/world-and-progression.md`                                           |
| **Skirmish victory / defeat**                     | 🟡 seeded    | `notes/form-and-defeat.md` (eliminate the foes / the party falls; in code) |
| **Run victory / defeat** (across many skirmishes) | ⬜ stub      | — *(undefined — a game is many skirmishes; the run-level win/lose is not)* |
| **Geography & travel** (the world map + movement) | ⬜ stub      | — *(not yet explored)*                                                     |
| **Loot** (rewards → new cards/aspects)            | ⬜ stub      | `notes/cards-and-customization.md`                                         |
| **Progression** (growth between skirmishes)       | ⬜ stub      | `notes/world-and-progression.md`, `notes/archetypes.md`                    |

✅ worked = full, the template to follow · 🟡 seeded = a few real rules, not
exhaustive · ⬜ stub = headers + intent only, not yet authoritative.

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
is immaterial **except** for conflicting modifiers on one target (§6); those resolve
in a **fixed seat order** — Speed plays no part in timing (§3.1) — so resolution is
fully deterministic.

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
- Speed never affects resolution order: every effect is order-independent except §6
  modifier-stacking, which uses the fixed seat key.
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

*(SEEDED — exact cover/drain numbers are open; `booklet.ron` / appendix.)*

### 3.3 Overextension — *removed*

> **REMOVED.** The old **Exposed / Focus→0** penalty (overextending Tempo zeroed your Focus)
> is gone. Tempo and Focus are **independent** breadth pools, each hard-capped by its stat,
> and the offense/defense balance now lives entirely in the **Speed-vs-Mind split** — a
> high-Speed/low-Mind fighter natively attacks widely but defends poorly, and the reverse —
> so no coupling penalty is needed. **Pay-after is kept** (§3.1): the action that drives a
> pool negative still happens and is your last, but it carries **no extra penalty**.

### 3.4 The round — orchestration (PvE & PvP)

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

## 4. Formation, reach & the gauntlet ✅

**RULE.** Each side holds a **front line** and a **back line** (unordered sets). Formation is
**public** and **shifts freely between rounds** (front↔back) — a **free action**, so it is
known *before* targeting. **A side must always keep at least one living Actor in its front
line:** a configuration with an empty front line is **illegal**, and the interface never
offers a shift that would produce one (so the last front-liner can never drop to the back).
Reach gates who you can engage:

- **Melee** reaches **front↔front** directly. To reach an enemy **back-liner**, a
  **front-line** attacker must **dive** — back-liners cannot dive.
- **A dive runs the gauntlet of the opposing front line.** Each enemy **guard** (front-liner)
  may spend **Tempo** to **intercept** the diving **runner**. Per interception the runner
  chooses: **push through** — take the guard's **base strike** automatically (a free hit,
  resolved *before* reaching the target) and pay **the guard's Speed** in Tempo to pass; or
  **halt** — abandon the dive and duel that guard instead. If pushing drives the runner's
  **Tempo negative** (it lacks the **combined Speed** of the intercepting guards), it is
  **stopped** — it cannot reach its target this round.
- **Ranged** (an actor/weapon property, available on **either** line) reaches the enemy back
  line **directly, bypassing the gauntlet**.

**WHY.** Formation gives the **wall / runner / artillery** triangle: a front line shields the
squishy back line from **melee** (the gauntlet drags runners) but **never from ranged** — so
you need your own ranged, or to break through, to answer their back line. Public formation
keeps targeting a clean read; routing the dive cost through **combined Speed** makes a thick
wall a real Tempo tax and a fast runner genuinely slippery (Speed-vs-Speed).

**GUARANTEES.**
- The back line is reachable by **melee only through the front line** (dive + gauntlet), by
  **ranged always** (bypass).
- The gauntlet is a **Tempo economy**, not an ordering effect: a runner gets through iff its
  Tempo ≥ the intercepting guards' combined Speed, eating one base hit per guard pushed.
- Formation is public and free to shift — positioning is strategy, never a hidden surprise.
- **The front line is never empty.** A shift may not move a side's last living front-liner to
  the back, so an all-back formation is unreachable and reach (which keys off front-line
  occupancy) stays well-defined. The UI suppresses any reposition that would empty the front.

## 5. Zones / exhaustion ⬜

*Stub — and flagged for rewrite.* Form / Potential / Active; face up/down;
Lasting / Fleeting; **exhaustion = predictability**. Source:
[`notes/zones.md`](../notes/zones.md). **Needs:** the post-Duel rewrite — the old
self-returning stances (Block/Evade/Scheme) no longer exist, so predictability-as-
resource must be re-pinned to the **maneuver/Action cards** you Unleash with. This
is the biggest known mechanical hole (the orphaned exhaustion economy).

## 6. Aspects / the chord ⬜

*Stub.* A character is a set of never-shuffled decks; an action is one card per
aspect, combined commutatively; only Mind (the stance) is rock-paper-scissors.
Card kinds: numberless, modifier (attachment order matters), passive. Source:
[`notes/decks-and-aspects.md`](../notes/decks-and-aspects.md).

## 7. Agents — Character vs Creature ⬜

*Stub.* The line is **theory of mind**: a Character reasons and predicts you back
(two-way); a Creature draws from a behavior deck (its instinct = its decision,
one-way), reshuffles, never exhausts. Source:
[`notes/entities.md`](../notes/entities.md),
[`notes/decision-making.md`](../notes/decision-making.md).

## 8. Strategic layer ⬜

*Stub.* World / scenario / enemy / **event** decks; regions; location level-ladders
with one "cleared" marker; the balance budget (challenge tuned to party *total*);
god-vs-party equivalence; doom-to-mastery. Source:
[`notes/world-and-progression.md`](../notes/world-and-progression.md). **Many
open structural questions** (map representation, event-deck cadence, multi-actor
simultaneous resolution).
