# Deckbound vs Grand Archive vs Magic — timing, variety, and what to learn

> **Status: non-authoritative design study.** Neither `canon/` (it doesn't govern) nor
> `notes/` (frozen history). A comparative synthesis, in the spirit of
> [`intended-player-experience.md`](intended-player-experience.md): it reads a design lens off
> three games and proposes candidate north stars. **Acting on the two candidate additions
> (§4 N2, N5) means the normal change discipline** in
> [`canon/0-source-of-truth.md`](canon/0-source-of-truth.md): spec first, human decides. Until
> then they are proposals, not canon.

*Origin: the user is enamored with Grand Archive (GA), sees it sharing Deckbound's goal —
"the variety of Magic without the card-timing insanity" — and asked what the comparison
teaches, whether it implies north stars, and how the **difference between GA and MTG** should
influence Deckbound. Folded in from `needs-merge/` on 2026-07-06.*

> **Two honesty caveats.**
> 1. **"the variety of MTG without the card timing insanity" is the user's phrasing, not
>    canon.** It appears nowhere else in `docs/`. The closest canonical support is Charter #2
>    ("Computable tactics, uncomputable strategy," [`canon/1-charter.md`](canon/1-charter.md))
>    and constraint C4 ([`notes/constraints.md`](notes/constraints.md)). Treat the phrase as
>    the *question*, not a spec claim.
> 2. **Grand Archive is otherwise absent from the repo.** The GA comparison is built from the
>    live rules at `rules.gatcg.com` (fetched 2026-07-06), not from any Deckbound source. GA
>    facts below are cited to the rules site; where I *infer* rather than quote, I say so.

---

## 0. The one-sentence thesis

All three games want build variety. They differ almost entirely on **one axis: when, and by
what mechanism, one player's decision is allowed to touch another's.** MTG permits reaction
*constantly and by default*; GA keeps the same machine but *throttles* it; Deckbound
*abolishes reaction* and replaces it with *anticipation*. The "timing insanity" the user
dislikes is not a property of variety — it is a property of **where interaction sits relative
to information reveal.** That is the whole finding; everything below is its consequences.

*(Refined after designer review — see §7. The deeper driver is a single **complexity /
computability budget**: "anticipation vs reaction" is a downstream consequence, and Deckbound's
no-stack stance is a **provisional** budget decision motivated by Charter #2/#11, not a
permanent identity commitment. Read §7 before treating §0/§4 as settled.)*

---

## 1. The three interaction models, precisely

### Magic: The Gathering — the open stack
- **Mechanism:** a LIFO **stack** with **priority** passed after *every* action; any player
  may add to the stack whenever they hold priority.
- **Reactivity:** universal and default. "Instant speed" is a broad card class; a large
  fraction of the pool can be held and played *in response to what you just saw*.
- **Resources:** lands, drawn from the same library as spells → the **mana screw / flood**
  variance tax. Variety is entangled with this luck.
- **Combos:** unbounded and **multiplicative** — the engine of both MTG's brilliance and its
  non-computability (and much of the "insanity").
- **Variety source:** *emergent* from a vast card pool + open interaction. The **Aggro /
  Control / Combo** archetype triangle is *not designed in* — it precipitates out of thousands
  of card interactions (our own game-theory library holds this up as a correct nested-counter
  system: [`../../game-theory/examples-done-right.md`](../../game-theory/examples-done-right.md)).
- **Cost:** rules complexity that needs a judge corps; long reactive turns; the Urza's-era
  balance blowouts ([`../../game-theory/examples-done-wrong.md`](../../game-theory/examples-done-wrong.md)).

### Grand Archive — the *throttled* stack (the middle path)
GA is the interesting case: it **keeps MTG's engine and adds governors.**
- **Still a LIFO Effect Stack** with timestamps, and priority — renamed **"Opportunity"** and
  defined as "the chance to act *in response to* player actions … an effect or player action
  pending resolution, or … a phase change."
- **Governor 1 — reactivity is opt-in per card, not a universal speed class.** A card is
  reactive only if it carries the **Fast** modifier ("may be activated while that player has
  Opportunity"). **Allies (creatures) and Attacks are innately *Slow*** — "only … activated by
  the turn player during their main phase when no effects are on the Effects Stack." That is
  *sorcery speed by default;* reaction is a designed, marked privilege.
- **Governor 2 — Opportunity is front-loaded, not continuous.** The turn player gets it only
  at the *start* of Recollection, Main, and End; a phase ends only once "the Effects Stack is
  empty [and] all players have passed Opportunity in succession." No priority ping-pong after
  every action.
- **Governor 3 — resources are decoupled from the main deck.** Two decks: a **Main Deck** and
  a separate singleton **Material Deck** (must hold ≥1 Level-0 champion). A single **champion**
  levels up across the match (**Lineage**), objects are **rested** to pay costs (**Reservable**),
  and cost scales to champion level (**Efficiency**: "costs LV less"). *Inference, flagged:*
  with no lands in the main deck, GA sharply reduces MTG's mana screw/flood — resource
  development is not hostage to draws.
- **Turn:** Wake Up → Materialize → Recollection → Draw → Main (Combat inside) → End.
- **Net:** GA proves you can keep a real stack, real instant-speed *texture*, and real
  interactive depth **while taming the insanity** — by making reaction rare, marked, costed,
  and windowed, and by removing resource variance.

### Deckbound — no stack at all; anticipation replaces reaction
- **Mechanism:** *hidden simultaneous commit → simultaneous reveal → deterministic
  resolution.* **No stack, no priority, no instant speed** — the central deliberate departure
  (constraint **C4**: "each side commits a face-down card, then both reveal at once. No agent
  may react to another's choice after seeing it within the same exchange").
- **What replaces the stack:** the **sub-phase schedule** — a *fixed* sequence
  (Intercept → Volley → Raid → Clash → Breach), declared to be *the entire* timing system
  (Spec §4.6: "there are no other timing rules"). Within one sub-phase everything committed
  lands order-independently; ordering exists only so an earlier death can *preclude* a later
  action.
- **Reactivity → anticipation:** "**Defense is anticipatory, not reactive: a buff played into
  an incoming attack does not save you from it**" (Spec §1.9); attacks resolve before buffs.
  You cannot **hold** a card for a better-informed moment — "the blind-bid commit is
  simultaneous … never *held*."
- **The contest that replaces counter-wars:** one **single simultaneous Tempo bid** per
  attack; defender must strictly beat it; "**There is no iterated raise-war**" (Spec §4.2).
- **Resources:** the most radical anti-MTG stance of the three — **players draw nothing at
  random.** "A player's own cards carry no luck — no shuffle, no random draw" (Charter #8);
  "the deck *is* the character" (Spec §2.3). There is no resource axis to be screwed on.
- **Combos:** strictly **additive / commutative / order-independent — never multiplicative or
  gating** (Spec §5.6: "no played effect multiplies or gates another's output").
- **Variety source:** *designed in*, at the build layer — five Roles bound 1:1 to five Suits
  (Iron/Wall, Silver/Infiltrator, Brass/Artillery, Bone/Controller, Salt/Support), a playstyle
  RPS triangle (Aggressor ▸ Glass-Cannon ▸ Turtle) built **into the combat rules**, plus the
  depth-vs-breadth "specialist vs god" fork.
- **Hard structural root:** Charter #7, "**playable by hand, cards only** — no computer
  required." A stack with timestamps, negation bookkeeping, and continuous priority is
  arguably *unplayable by hand* — so the anti-timing stance is not taste, it is *forced* by the
  medium.

---

## 2. The spectrum, at a glance

Every axis tells the same story: **MTG open → GA throttled → Deckbound bounded-by-construction.**

| Axis                              | Magic                                 | Grand Archive                                     | Deckbound                               |
| --------------------------------- | ------------------------------------- | ------------------------------------------------- | --------------------------------------- |
| Interaction mechanism             | Open LIFO stack + continuous priority | LIFO stack, priority **windowed** at phase starts | **No stack**; fixed sub-phase schedule  |
| When you act vs. info reveal      | **After** reveal (reactive), always   | **After** reveal, but only in Fast windows        | **Before** reveal (**anticipatory**)    |
| Reactivity is…                    | Universal / default                   | **Opt-in per card** (Fast); creatures Slow        | **Forbidden** by C4                     |
| Escalation                        | Unbounded counter-wars                | Bounded (stack empties, pass in succession)       | **One bid, higher wins** — no raise-war |
| Resource variance                 | Mana screw / flood                    | Decoupled via Material Deck (inferred: low)       | **Zero** — no random draw for players   |
| Combos                            | Multiplicative, unbounded             | Chained but throttled (Slow default)              | **Additive/commutative**, bounded       |
| Where variety lives               | Emergent from huge card pool          | Champion + elements/classes + card types          | **Designed** Roles/Suits + built-in RPS |
| Hand-playable without a computer? | No                                    | Barely (timestamps, negation)                     | **Yes, by mandate** (#7)                |

**GA and Deckbound are directional allies against MTG.** They disagree only on *degree*. GA
throttles the reactive engine; Deckbound removes it. Both *agree* that variety must not be
paid for with resource luck, unbounded combos, or continuous reactive bookkeeping.

---

## 3. What we can learn

1. **Variety is orthogonal to reactivity.** The single most important lesson. MTG makes
   players feel its variety *requires* the stack; GA disproves this (Slow-by-default, still
   enormous build variety); Deckbound disproves it harder (no stack, variety fully intact via
   designed Roles). You can pursue maximal build variety and minimal timing complexity at
   once — they are not the same dial. This validates Charter #2.

2. **The real axis is *anticipation vs reaction*, not *simple vs complex*.** Deckbound isn't
   "MTG with fewer options" — it is MTG's decision moved to *before* the reveal, a genuinely
   different game of skill (reading/prediction, not response). GA sits on the reaction side
   (rationed); Deckbound is the only one on the anticipation side. **This is Deckbound's
   clearest *differentiator* from both** — though (per §7) it is a *consequence* of a
   provisional complexity/computability budget decision, not a permanent identity commitment.

3. **Every commercially-proven "fix" for MTG's variance, Deckbound already exceeds.** GA's
   Material Deck removes mana screw; Deckbound removes *all* player draw luck. GA's Slow
   default tames reaction; Deckbound abolishes it. Deckbound is consistently the *limit case*
   of the direction GA is walking — the market has validated the direction; Deckbound is
   further down the road.

4. **GA is the proof that the middle exists.** There is a successful, published design point
   between "open stack" and "no stack." If pure anticipation ever feels flat, Deckbound need
   not jump all the way back to MTG — GA marks a reachable, throttled compromise.

5. **Reaction, where it survives, should be a marked/costed *card property*, never a *speed
   class*.** GA's **Fast** keyword is exactly this. The spec already agrees in principle — any
   "see their card before you choose" effect is "a special ability, never the core" (Spec
   §1.0). GA is independent confirmation the marked-exception pattern is sound and shippable.

6. **Combos are where "insanity" is really born.** MTG's *timing* complexity and its *combo*
   complexity are the same phenomenon: the stack is what lets combos be multiplicative and
   unbounded. Deckbound's additive-only rule and GA's Slow-default are two brakes on the *same*
   runaway. Keep the brake.

---

## 4. Candidate north stars this implies

Framed as **candidates / confirmations.** N1, N3, N4 restate existing Charter stars in this
comparison's language. N2 and N5 were *proposed* as genuinely new additions but were
**revised/withdrawn on designer review** — see §7; the corrected characterizations are below,
kept to record the reasoning.

- **N1 — Variety on the card axis, austerity on the timing axis.** (Confirms Charter #2.)
  Spend the complexity budget on *what builds can be*, never on *when things can happen*. A new
  timing rule is a smell; new build/Role/Suit expression is the goal.

- **N2 — *Not* a north star; a provisional complexity/computability constraint.** (Revised —
  §7.) The earlier draft proposed "read, don't respond" as an aspirational identity. The
  designer's correction: there is **no objection to the stack in principle**, and the no-stack
  choice is **not permanent**. The stack is excluded because it *inflicts complexity and a loss
  of computability the design is not ready to take on right now* — i.e. it is a present-tense
  application of **Charter #2 and #11**, not a preference for anticipation over reaction.
  Anticipation is therefore a *side effect* of the budget decision, not the goal, and a stack
  (or a slice of one) could return if its computability cost were solved.

- **N3 — No variance tax on variety.** (Confirms Charter #8.) Both GA and Deckbound reject
  MTG's mana model; Deckbound rejects all player draw luck. Never reintroduce resource/draw
  randomness as the price of a bigger card pool — now with two independent industry witnesses.

- **N4 — Reactivity, if ever admitted, is a rare marked card, never a mode.** (Confirms Spec
  §1.0; GA's **Fast** is the template.) If a future card peeks or responds, it is a costed,
  explicitly-keyworded exception the core rules never assume exists.

- **N5 — Dissolved on review; not a real gap.** (§7.) The apparent difference is *structural,
  not conceptual*. Deckbound already has progression — through **card sets between battles.**
  GA's champion leveling only looks like a distinct feature if GA is read as *one long battle*;
  read as *many small battles*, its progression happens *between* them, exactly as Deckbound's
  does. There is no missing "anchor" to add — only a choice of progression *granularity*
  (in-battle vs between-battle), which is itself a complexity-allocation decision (§7).

---

## 5. How the GA↔MTG *difference specifically* should influence Deckbound

The GA↔MTG delta is: **GA bolted governors onto MTG's engine without removing the stack.**
Deckbound removed the engine entirely. So the delta hands Deckbound a menu of governors it
could adopt **without ever adding a stack** — plus one warning.

**Adopt in spirit (low risk, high fit):**
- **(a) Treat the sub-phase schedule as Deckbound's "Slow default," and be proud of it.** GA's
  "Allies/Attacks are Slow" and Deckbound's "attacks resolve before buffs / anticipatory
  defense" are the *same instinct*: commit-and-resolve is the default; reacting is the
  exception. The one-line bridge for TCG players: *"everything in Deckbound is Slow; nothing is
  Fast."*
- **(b) Keep resource-austerity as a permanent moat.** GA needed a whole second deck to solve
  what Deckbound solved by fiat. Never "add resources for variety" — Deckbound is already
  ahead here; protect it.

**Consider as future pressure-relief valves (medium risk — only if playtest reveals a gap;
these are [`future-possibilities.md`](future-possibilities.md) material):**
- **(c) A tiny, GA-Fast-style "reveal-response" card class.** *Only* if pure anticipation ever
  tests as too flat/solved-feeling. The spec already sanctions "see their card" effects as rare
  special abilities. Copy GA's *rationing* (marked, costed, windowed), not MTG's *default*.
  **Guardrail:** never a mode the core assumes (N4), and it stresses Charter #7 — GA's
  timestamps/negation are exactly the bookkeeping #7 forbids, so any Deckbound reaction must
  resolve *without* a stack to track.
- **(d) ~~A GA-champion-style progression anchor~~ — withdrawn (§7).** Not an import Deckbound
  needs: it already progresses between battles via card sets, and GA's champion arc is the same
  progression at a different *granularity*. The only live question is where to spend
  progression complexity (in-battle vs between-battle) — a budget choice, not a missing feature.

**The warning — do NOT drift toward GA's actual mechanism:**
GA is seductive precisely because it's a polished middle. But every governor it adds
(timestamps, negation targeting activation-instances, continuous-effect dependencies) is
bookkeeping a human cannot reliably run by hand — exactly what Charter #7 forbids and #11
(computable-by-construction) prevents. **GA's stack is learnable; Deckbound's absence-of-stack
is *hand-playable*.** That is a category advantage, not a smaller GA. The future move is to make
anticipation *deeper and richer* (more to read, more bluff texture in the blind bid), **not** to
re-grow a stack until Deckbound becomes a worse GA. Chase interactive tension through
**prediction depth**, not **reaction windows.**

---

## 6. Next steps

**No canon changes proposed.** After review (§7) both candidate additions collapse: **N2** is a
*provisional constraint* already implied by Charter #2/#11 (no new north star), and **N5** is not
a real gap (progression already exists between battles). One optional housekeeping move: note in
[`computability-and-balance.md`](computability-and-balance.md) that the no-stack choice is a
**computability-motivated, revisitable** exclusion rather than a fixed identity — so a future
designer doesn't mistake a deferral for a principle. Otherwise, nothing to promote.

---

## 7. Refinements from designer review (2026-07-06)

Two corrections that reframe §4's proposals and the doc's thesis:

**On N2 — "no stack" is a provisional constraint, not a north star.** There is no objection to
the stack in principle. The stack is excluded because it *inflicts complexity and a loss of
computability the design is not ready to take on right now* — a present-tense application of
Charter #2 (computable tactics) and #11 (computable by construction), not an aesthetic
preference for anticipation over reaction. It is therefore **revisitable**: if the computability
cost were solved, a stack (or a slice of one) is not forbidden by identity. The "read, don't
respond" framing overstated a pragmatic deferral as a permanent value.

**On N5 — the progression difference is structural, not conceptual.** Deckbound already has
progression: card sets *between* battles. GA's champion leveling only looks like a distinct
feature if GA is read as *one long battle*; read as *many small battles*, its progression
happens between them, exactly as Deckbound's does. There is no missing anchor to add — only a
choice of progression *granularity*.

**The unifying lens both corrections point to: one complexity / computability budget, allocated
differently.** The three games are better understood not by *where interaction sits relative to
reveal* (§0's original axis) but by *how each spends a finite complexity budget*:
- **MTG** spends lavishly on the interaction engine (the stack) — and pays in
  non-computability.
- **Deckbound** deliberately spends *elsewhere* — richer battle structure (5 phases, 3
  intentions) and a strategic layer (locations, movement) — and *saves* on the interaction
  engine (no stack) to stay computable. Complexity is **reallocated, not merely reduced.**
- **GA** sits between: a throttled stack bought at a moderate computability cost.

Read this way, "variety without timing insanity" is really *"choose where to spend complexity,
and refuse the expenditures whose cost — here, computability — you can't currently afford."*
That is the durable takeaway, and it is **already canon as Charter #2/#11** — so it needs no new
north star. §0's "interaction relative to reveal" axis remains a useful *descriptive* cut across
the three games, but the *prescriptive* driver for Deckbound is the budget.
