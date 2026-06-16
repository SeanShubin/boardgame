# Deckbound — Mechanical Spec

**Canonical for mechanics.** This is the precise statement of how Deckbound's
systems work. It is a source of truth (see
[`SOURCE-OF-TRUTH.md`](../SOURCE-OF-TRUTH.md)) — the code conforms to it, not the
other way around. It owns **vocabulary and procedures**, not **numbers** (numbers
live in [`booklet.ron`](../../../../crates/deckbound/data/booklet.ron)).

> **AI assistants:** read [`SOURCE-OF-TRUTH.md`](../SOURCE-OF-TRUTH.md) first. In
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
consequence. See also [`SOURCE-OF-TRUTH.md`](../SOURCE-OF-TRUTH.md) — "Motivated rules."

Numbers appear only as *(appendix)* illustrations; the real values are in
`booklet.ron` and are human-tuned.

**Keyword rules** additionally carry a **MANUAL** line — the one sentence that
prints in the rulebook / on hover. The engine pairs each keyword's handler with
this line so digital and printed rules can't drift; the Spec is where that line is
authored.

---

## Coverage

| System                                  | Spec status | Current design source if not yet specced                  |
| --------------------------------------- | ----------- | --------------------------------------------------------- |
| **The Clash** (tactical core)           | ✅ worked    | —                                                         |
| **Defense model** (cut → bar → pool)    | 🟡 seeded    | `design/stats.md`, `design/form-and-defeat.md`            |
| **Speed/Tempo + Mind/Focus**            | 🟡 seeded    | `design/speed-and-tempo.md`, `design/mind-and-stances.md` |
| **Coordination / positioning**          | ⬜ stub      | `design/coordination-and-interruption.md`                 |
| **Zones / exhaustion**                  | ⬜ stub      | `design/zones.md` *(needs post-Duel rewrite)*             |
| **Aspects / the chord**                 | ⬜ stub      | `design/decks-and-aspects.md`                             |
| **Agents** (Character vs Creature)      | ⬜ stub      | `design/entities.md`, `design/decision-making.md`         |
| **Strategic layer** (world/event decks) | ⬜ stub      | `design/world-and-progression.md`                         |

✅ worked = full, the template to follow · 🟡 seeded = a few real rules, not
exhaustive · ⬜ stub = headers + intent only, not yet authoritative.

---

## 1. The Clash — *the tactical core* ✅

The atom of combat: two Actors **predicting each other**, resolved beat by beat
inside one round. The fighting-game **strike / throw / block** mix-up, played as
cards. Design background: [`design/the-duel.md`](../design/the-duel.md).

> **History.** This section formerly specced a single-clash **stance/Edge duel**
> (Marshal · Unleash · Overwhelm · Parry, with a tracked Edge meter). That system is
> **superseded by §1.0 (The Clash)** below. The superseded subsections (§1.1, §1.2,
> §1.3, §1.5, and the in-duel read of §1.8) are kept for design history — each carries
> a banner — because their **WHY/GUARANTEES carry forward** into the Clash. The
> breadth and resolution-order rules (§1.6 reworded, §1.7, §1.9) and §3 are unchanged.

### 1.0 The Clash — beats, six moves, charges

**RULE.** A duel is **the Clash**: a sequence of **beats**. Each beat both fighters
**secretly choose one move** and reveal simultaneously; the beat resolves; the Clash
continues until a fighter's **Body reaches 0** (Body-attrition — there is no "ends on
the first strike"). The six moves come in two kinds:

- **Standing** *(always available, never deplete)* — **Strike**, **Throw** (offense);
  **Parry**, **Evade** (defense).
- **Setups** *(the escalation resource — durable face-up cards that replace the Edge
  number)* — **Charge** (place one active **Charge**; each active Charge **doubles**
  your attack damage, ×2 per Charge) and **Recover** (flip your own face-down Charges
  back up). **Charge capacity** is a per-actor stat (`booklet.ron`); Charge is offered
  only below capacity, Recover only with a face-down Charge.

**Counter-rules** (the whole table, re-derivable):
1. **Cycle.** Strike ▸ Evade ▸ Throw ▸ Parry ▸ Strike — each attack beats one defense
   and loses to the other (Strike beats Evade, loses to Parry; Throw beats Parry, loses
   to Evade).
2. **Trade.** Strike vs Strike → **both hit** (the hinge of invariant 3). Strike
   **clips** Throw (Strike > Throw): the striker lands, the thrower does not.
3. **Attacks beat setups.** A connecting Strike/Throw hits a Charging or Recovering foe
   and **interrupts** the setup (it does not resolve that beat).
4. **Setups resolve if unopposed.** Against anything that does not connect (a defense,
   or the other setup), Charge adds a Charge and Recover restores flipped Charges.
5. **A successful defense flips the attacker's active Charges face-down** (disable, not
   destroy — the comeback); Recover restores them.
6. **Damage** = `power × 2^(active Charges)`, routed through the armor/toughness pipeline
   (§2); Body 0 = down.

**WHY.** The standing defenses (Parry/Evade) and standing attacks (Strike/Throw) never
deplete, so a perfect reader can *always* answer the move in front of them — that is
what lets the two reachability invariants hold for the **whole** duel, not just one
beat. Charges-as-cards replace the Edge number with something **visible and durable**,
and the **×2** makes a completed wind-up genuinely lethal: this is the
Gandalf-vs-Balrog engine (north star #4) — a weak fighter can steal a duel with perfect
reads, but the instant a read is wrong the doubled blow lands, and the downside is far
worse for the weaker side, so over many duels the upset is a *bad bet*. Folding the
**trade** cell (Strike vs Strike = both hit) into the cycle is precisely what forbids
landing-for-free against a committed attacker — you cannot buy both invariants at once.
(north stars #2 computable, #4 asymmetry, #10 re-derivable.)

**GUARANTEES** — the three invariants, under **last-word reads** (the opponent commits
face-up, then you choose):
1. **Avoid.** Spending the defensive read, you can pass through the **whole** duel
   **un-hit** if you choose: every attack has a standing defense that negates it
   (Strike↦Parry, Throw↦Evade), and the defenses never deplete.
2. **Land.** Spending offense, you can land a hit by the end if you choose: for every
   move the opponent can make, some standing attack lands (Throw beats Parry; Strike
   beats Evade, trades into Strike, clips Throw; either attack hits a setup).
3. **Not both, free.** You cannot guarantee both at once: against a committed **Strike**
   the *only* landing answer is Strike, and Strike-vs-Strike **trades** — landing on a
   striker means taking a hit.
- **Termination.** Body-attrition ends every duel at Body 0; the engine backstop (§1.6)
  breaks off a Clash that makes no progress, so no duel runs forever.

**MANUAL.** *Each beat, pick a move: Strike or Throw to attack, Parry or Evade to
defend, Charge to wind up a doubling blow, Recover to ready a knocked-down charge.
Strike beats Evade, Throw beats Parry, Strike clips Throw, Strike trades with Strike; a
successful defense knocks the attacker's charges down.*

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

> **SUPERSEDED by §1.0 (The Clash).** Replaced by **Body-attrition**: a Clash runs beat
> by beat until a Body reaches 0, not until the first strike connects. This is the change
> the charge→big-hit arc requires (a single hit no longer ends the duel, so winding up a
> ×2 blow is meaningful). Termination is now guaranteed by Body 0 + the §1.6 backstop.

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

**RULE.** A Clash cannot stall under any reasoning play (some standing attack always
lands — invariant 2, §1.0). As an **implementation backstop only**: after **N
consecutive beats with no Body lost** *(appendix: e.g. 12)* — e.g. two fighters who can
each fully absorb the other, or a defender facing a non-attacker — the duel **breaks
off** (both disengage; the foe still counts as engaged, so it does not also free-hit at
round end). A creature whose instinct drives a winnable Clash to the backstop is a bug.

**WHY.** Body-attrition (§1.3 superseded) means a duel ends on Body 0, not on a single
strike; the backstop guarantees termination in the corner case where neither side can
actually wound the other (armor/toughness fully absorbs every connecting hit), without
adding a rule real players meet.

**GUARANTEES.**
- The backstop is invisible in normal play and is **not** part of the public rules.
- Every Clash terminates: it ends at Body 0, or breaks off after N no-progress beats.

### 1.7 Facing a crowd — K duels, two caps

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

Design source: [`design/form-and-defeat.md`](../design/form-and-defeat.md),
[`design/stats.md`](../design/stats.md). Seeded below; not yet exhaustive.

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

## 3. Speed/Tempo + Mind/Focus — *the two mirrored budgets* 🟡

Design source: [`design/speed-and-tempo.md`](../design/speed-and-tempo.md).

They **mirror in sizing and cost** but are **different in kind**, which is why their
overextension rules (§3.3) are asymmetric *on purpose*. **Speed is mobility** — the
budget for controlling engagement (closing to strike, opening to escape) — and
mobility is **exhaustible**: spend it all and you are *caught*. **Mind is perception**
— the budget for reading the clashes you are in — and perception only **caps out**:
you see what you see, and the rest gets through. Exhausting your legs and the limit of
your eyes are not the same thing, so they do not behave the same.

### 3.1 Speed is the stat; Tempo is the resource

**RULE.** **Speed** is a fixed stat that never depletes; it sets the size of your
**Tempo** pool (= Speed, refreshed each round) and is the **price others pay** to
deal with you on both axes (striking you costs your Speed from their Tempo;
predicting you costs your Speed from their Focus) — a fast foe costs that Speed whether
it makes you *chase* it or merely *match it in the exchange*, so a willing clash is no
cheaper than a chase. You may act while Tempo ≥ 0; pay
**after** each action; the action that takes you **negative** is your last
(overextension), and leaves you **Exposed** table-wide. Speed governs **only** this
economy and the **thresholds** others must clear to deal with you — striking,
predicting, or catching you each costs your Speed (§1.7) — and it **never** sets
resolution order. *Speed decides what can happen at all, not what happens first*: who
wins an exchange is the stance read (§1.2); the order actions resolve in is fixed by
engagement tier and seat (§1.9).

**WHY.** Pay-after-not-before lets a slow fighter always get a base action, makes
the **negative line, not zero, the wall**, and turns "how many can I act on" into
one self-capping pool instead of a separate rule.

**GUARANTEES.**
- Tempo is always re-derivable from Speed minus visible actions (no token needed).
- Overextension is a real option at a real, table-wide price.
- Speed is decoupled from timing: it sizes budgets and sets thresholds, never
  initiative or who-goes-first.

### 3.2 Focus mirrors Tempo

**RULE.** **Focus** is a defensive pool sized to **Mind**, refreshed each round.
Each prediction (covering one foe's duel) costs the **attacker's Speed** out of
Focus. Foes your Focus can't cover **free-hit** you; **Toughness** absorbs what
lands. The unweighted special case — "one slot per attacker, up to Mind" — is every
foe at Speed 1.

**WHY.** Mirroring offense and defense pool-for-pool keeps the god-vs-party budget
linear, and routing attacker-Speed through *Focus* (not Tempo) stops one fast
fighter from owning both offense and defense.

**GUARANTEES.**
- Fast attackers are harder to wall than slow ones (inverse telegraph).
- "Negate many" is even in total across builds, capped per body.

*(SEEDED — exact drain function numbers are open; `booklet.ron` / appendix.)*

### 3.3 Overextension — Exposed, the all-in

**RULE.** Acting past your budget — the action that takes your **Tempo negative** — is
your last this round and marks you **Exposed** — **caught flat-footed**, your mobility
spent: an **all-in** that drops your **Focus to 0** for the round. You can no longer
read anyone, so every foe free-hits you (§3.2)
and any duel you were holding collapses to a magnitude trade. *Over-predicting* (Focus
negative) needs no separate penalty — its overflow already free-hits you (§3.2).
Exposed clears at round refresh.

**WHY.** Speed is **mobility**, and mobility is exhaustible (§3): spend it all chasing
and you cannot flee — you are **caught**, so every foe reaches you with nothing left to
read them by (Focus 0 is the proxy for *caught and can't see straight*). That restores
the coupling a single pool gave for free, and routing the cost through Focus makes it
**self-scaling** — you lose exactly the defense you had, so a thin-Mind brawler loses
little and a Focus-rich duelist loses much — and **situational** — getting caught amid
a crowd drops your guard on the whole table at once. **Mind**, being perception not
mobility, can only **cap out**, never be exhausted into a penalty — which is why
over-predicting needs no rule of its own; the asymmetry is just the two resources being
different in kind. One conditional, no arithmetic, reuses the §3.2 free-hit path — no
new resolution rule.

**GUARANTEES.**
- Overextension is a real, table-wide price (the §3.1 GUARANTEE holds) — never a free
  extra action.
- The cost is Focus (reads), never an ordering effect (consistent with §3.1 / §1.9):
  it bites mixed builds via lost defense and pure-offense builds via the damage they
  then eat — never free for either.
- Going all-in is a **choice**: pay-after (§3.1) still grants the base action, so a
  slow Actor opts into the all-in; it is never forced on a defensive build that
  declines to overreach.

---

## 4. Coordination / positioning ⬜

*Stub.* Front/back lines as unordered sets; reach as jumps
(`f↔f 1, f↔b 2, b↔b 3`); Attack vs Hold; the gauntlet (front line spends combined
Tempo as drag on Runners). Source:
[`design/coordination-and-interruption.md`](../design/coordination-and-interruption.md).
**Needs:** ~~the "duel detection" rule~~ — **specced in §1.8 (reading is the
contest) and §1.9 (resolution order).** Remaining: how positioning/reach feeds
engagement — which foes you *can* reach to read/strike in the first place.

## 5. Zones / exhaustion ⬜

*Stub — and flagged for rewrite.* Form / Potential / Active; face up/down;
Lasting / Fleeting; **exhaustion = predictability**. Source:
[`design/zones.md`](../design/zones.md). **Needs:** the post-Duel rewrite — the old
self-returning stances (Block/Evade/Scheme) no longer exist, so predictability-as-
resource must be re-pinned to the **maneuver/Action cards** you Unleash with. This
is the biggest known mechanical hole (the orphaned exhaustion economy).

## 6. Aspects / the chord ⬜

*Stub.* A character is a set of never-shuffled decks; an action is one card per
aspect, combined commutatively; only Mind (the stance) is rock-paper-scissors.
Card kinds: numberless, modifier (attachment order matters), passive. Source:
[`design/decks-and-aspects.md`](../design/decks-and-aspects.md).

## 7. Agents — Character vs Creature ⬜

*Stub.* The line is **theory of mind**: a Character reasons and predicts you back
(two-way); a Creature draws from a behavior deck (its instinct = its decision,
one-way), reshuffles, never exhausts. Source:
[`design/entities.md`](../design/entities.md),
[`design/decision-making.md`](../design/decision-making.md).

## 8. Strategic layer ⬜

*Stub.* World / scenario / enemy / **event** decks; regions; location level-ladders
with one "cleared" marker; the balance budget (challenge tuned to party *total*);
god-vs-party equivalence; doom-to-mastery. Source:
[`design/world-and-progression.md`](../design/world-and-progression.md). **Many
open structural questions** (map representation, event-deck cadence, multi-actor
simultaneous resolution).
