# Deckbound — Progression & Geography (design in progress)

> **Status: living, non-authoritative — the "how we get more powerful" half.** This
> captures the first-pass design for the **strategic / run layer**: the world map,
> clearing locations, the currency economy, and buying Upgrades. It is **not** a source of
> truth. When a piece here is settled enough, it graduates onto the Spec **Coverage
> table** via the change discipline in
> [`canon/0-source-of-truth.md`](canon/0-source-of-truth.md) (**spec first**, then
> `booklet.ron` numbers, then code, then tests).
>
> Sibling to [`roadmap.md`](roadmap.md) (this fleshes out its *Geography / Travel / Loot /
> Progression* gaps) and [`scenario-plan.md`](scenario-plan.md). Numbers in here are
> **illustrative only** — real values are AI-seeded, human-tuned, and live in
> `booklet.ron`.
>
> **Deliberately deferred:** *how the game tries to kill you* (run-level defeat) is still
> undefined; this half is about acquiring power, not losing it.

---

## The loop in one breath

Move your **identity card** across a map of **face-down location cards**. Enter a
location → it **flips face-up**, revealing what it offers. Choose to **clear it at a
depth** → win the fight → take its **treasure card** (a typed **currency**) and mark the depth
with a **clear marker**. Currency buys **Upgrades** — cards that make a character permanently
stronger. Specialize one currency type (depth) or spread across many (breadth). The team
shares currency by being **co-located** with the cards.

Every piece below is here because it serves a Charter north star; the tag in each
heading is the one it most rides on.

---

## 1. Geography — the map *(#7 cards-only, #9 metaphor)*

**The world is face-down location cards** in a scenario-authored layout. Because the
cards are rectangles, a scenario can build:

- a **grid** (cards aligned),
- an **offset-hex** field (alternate rows shifted half a card → six neighbours),
- or a **mix** of both in one scenario.

Each character's **identity card** — its `ActorCard` itself, not a separate token — is placed
on a location to show where it is.

**Adjacency & movement.** A character moves **one adjacent space per day** (§6; adjacency is
whatever the layout defines — 4-way on a grid, 6-way on offset-hex). *(No travel cost or risk
beyond this for now — deferred, see open dials.)*

**Fog.** A location stays **face-down until a character enters it**, then flips **face-up**
and stays up — revealing its **name**, and through the name its linked **treasure card** (so
you learn what currency it pays) — which also names the **threat deck** the encounter is
drawn from (§4.1), so you learn the *kind* of fight, not its exact shape. Scouting the map
is itself a push-your-luck act (#2) and
the engine of doom-to-mastery (#5): you find out what a place is by going there.

## 2. Entering & clearing — opt-in depth *(#2 risk/reward, #5 doom-to-mastery)*

Entering a location does **not** start a fight. It only flips the card. Combat is **opt-in
at a chosen depth**:

- A location can be cleared at a **level** (e.g. 1–5). You pick the level you attempt.
- **Clearing level *N* earns levels 1..N** at once.
- A per-location **clear marker** records your **high-water mark** — the deepest level
  cleared there. You may later go **deeper** (clear 5 → adds 4–5) but **never re-clear** at
  or below the mark.
- So a whole location's progress is **one number** (its clear marker). That keeps §2.1's
  discipline — nothing is a maintained meter except Body; everything else is read off the
  table.

**Losing a fight — placeholder stakes, and a lesson.** A failed clear means the character is
**forced to retreat**, having **spent its one encounter for the day** (§6) for nothing, and the
location's **threat persists** — you face the *same*, now-known encounter again. So failure
carries a **two-fold cost** (the day-action is gone; you must still beat that threat to advance)
**and** doubles as the tutorial: you learn the fight, regroup, and return (#1). You raise a
location's mark only by actually **beating its encounter at the depth you want** — there is no
skipping it. *(The cost is intentionally light for now — just lost days against par; once we
design how the game tries to kill you, those days gain real weight.)*

## 3. Currency *(#4 asymmetry, #2 opportunity cost, §2.1 recompute)*

Clearing a location yields its **treasure card**, which carries **currency**. Currency is
**typed** — **one per combat role**: **Iron** (Wall), **Silver** (Infiltrator), **Brass**
(Artillery), **Bone** (Controller), **Salt** (Support) — plus a generic **Gold** for
role-independent **utility**, for **six currencies** in all.

A location mints a **single currency type**, and its **value scales with depth** (its treasure
card pays more the deeper you've cleared — read off the card at the clear marker's level).
Pursuing one currency type **is** a strategy to play that role.

### 3.1 The recompute invariant *(§2.1, #7)*

Currency is never a tracked balance. It is **read off the table**:

```
balance(currency C) = (C from reachable treasure cards, valued at their clear markers)
                − (C printed on the Upgrades you already own)
```

Earned sits on the treasure cards; spent sits on the Upgrades. Nothing is maintained, so the
"only Body is a meter" rule (§2.1) survives the whole economy.

### 3.2 Spending — co-location *(#6 emergence, #7, #9 the stash)*

You may spend a currency only when you can **physically reach** its treasure card:

- a hero can always spend currency on **cards they carry** (they're co-located with their own),
- a treasure card **dropped at a treasury** can be spent by **any teammate who visits** that
  location.

So the team shares power through **logistics, not bookkeeping**: clear it, carry it, or
stash it where allies will pass. **Buying happens on contact** — the moment you have access
to a treasure card you may purchase from the **Upgrade deck**; there is no separate shop or
town system (deliberately, to minimise tracked systems).

### 3.3 Upgrades

An **Upgrade** is **not a new card type** — it is **any catalog character-card** (a power
`Card`, a `TraitCard`, or a weapon) **acquired with currency**; "Upgrade" names
the **role a card plays when bought**. The **Upgrade deck** is just the currency-buyable subset of
the catalog. A bought card joins your deck (a Form attachment or an Action card, §5) and makes the character
**permanently stronger**. For now Upgrades may be bought **freely**; a later mechanic may
**randomise availability**.

> **Resolved — see [`zones-exhaustion-design.md`](zones-exhaustion-design.md).** An Upgrade is
> **either a Form attachment** (permanent — what you *are*; never exhausts) **or an Action card**
> (zone-governed — what you *do*; Spend / Recover). Never an unbounded spammable power — which is
> what makes the acquisition economy balanceable. That doc designs the zone state-machine (Hand /
> Active / Down) that re-pins predictability-as-a-resource (#8) and closes the §5 hole.

## 4. Threat decks — the hidden tutorial *(#1 reward intellect, #6 emergence)*

The specific threat at a location is **randomised by a threat deck**, and there are **as
many threat decks as currency types** (six — the five roles + generic, §7) — one per type. A location of currency type *C* draws its
foes from **threat-deck *C***.

This makes a sequenced row of locations a **diegetic tutorial**: you meet *C*-threats →
earn *C*-currency → buy *C*-Upgrades → and those Upgrades are exactly what answers *C*-threats.
The place teaches the value of its own currency by making you need it.

It also produces a **party-scale depth/breadth tension for free** (#10, motivated): an
individual wants to go **deep** on their role's currency, but the **party** wants **coverage**
against every threat type — so specialise-vs-diversify is pulled in two directions without
any extra rule.

### 4.1 The encounter card *(#7 cards-only, #6 few systems, #2 one dial)*

Flipping a location reveals its **currency type**, which names the **threat deck** the encounter
comes from — so you know the *kind* of fight, not its exact shape. The specific foe
configuration is a **single encounter card**, drawn from that deck **once — the first time
you engage** the location — then **fixed**. That fixed card is the location's **persistent,
learnable threat**: retrying faces the *same* fight, so players learn to solve it (#1).

**Only a fight locks it.** The draw-and-fix happens when you **commit to combat**; merely
flipping the location does not. A **non-core scouting ability** may **preview the pending
encounter without starting a fight** — so a prepared party can read the matchup and angle for
a favourable one. Scouting is a **special ability, not core**; its specifics (including how it
yields a favourable encounter) are deferred.

An encounter card is a **parametric template**. Its composition is fixed, but it is
**evaluated at the level you attempt**, so one card covers every depth instead of one card
per level — going deeper means the *same* foes, escalated:

- **Roster as level-formulas** — e.g. *L1: A + B; L2: add C; L3: add D × (level − 1)*.
- **Stats as level-formulas — thematic** — *any* stat may scale (counts, Body, Speed,
  Mind…), and **which** ones scale expresses the encounter's identity and **signals the
  counter to bring** (#1, #10): a swarm scales counts, a brute scales Body, a blitz scales
  Speed (punishing thin Mind). Stats are set **once at spawn** (e.g. *Body = level × 3*) — and
  since creatures are **deck-built** (§7), each formula *builds that much into the creature's
  deck* (the encounter card is a **deck recipe**), realised as the normal Body pool (§2.1 — not
  a new tracked meter). The only ceiling on this flexibility is the table-arithmetic constraint
  below (#7).
- **A general strategy + its random fill** — the encounter card sets the **overall strategy**
  and **names the decks / compositions** that randomise the specifics: which §7 Creature
  **behavior / instinct deck** drives moves (e.g. *Aggressive Melee* — instinct = decision,
  one-way, reshuffles, never exhausts; reused, not reinvented, #6), and any composition table
  the once-only draw resolves.

**Two layers of randomness, kept apart.**

- **Strategic (resolved once → fixed):** which encounter card, and any composition it rolls.
  Fixed so the location stays a *learnable puzzle* (#1).
- **Tactical (live every fight):** the creatures' §7 instinct decks pick moves each turn — you
  learn the behaviour *distribution*, never a fixed script. Exactly §7's Creature.

The resolved roster then **assembles into the §4 battle** (Vanguard / lanes / Reserve) like
any party; its commitment and Clash choices are driven by the strategy deck.

**One dial, both sides.** The **level** you opt into scales the **reward** (currency value,
§2–§3) *and* the **threat** (this card) **together** — so choosing a depth is a single
honest risk/reward choice (#2), and it's re-derivable (#10): more reward always means a
proportionally bigger fight.

**Table-friendliness is a hard constraint (#7).** The math must be arithmetic a human does
at the table with no tracking — small integers, evaluated once at spawn (*level × 3*,
*× (level − 1)*). A formula that needs a calculator or a running tally has broken the
cards-only pillar; simplify it, or the encounter.

**Classification (house rules).** The *scaling mechanic* (encounters are parametric over
level) is a **rule → Spec-first** when it graduates. The *specific creatures, rosters, and
coefficients* are **data → `booklet.ron`, human-tuned**. Keep them apart.

## 5. Depth vs breadth — the strategic fork *(#2 uncomputable strategy)*

The two axes of growth:

- **Depth** — return to a known location and clear it **deeper** for richer currency and
  stronger Upgrades of one type.
- **Breadth** — explore **new** locations for a **variety** of currency types (and, per §4,
  broader threat coverage).

There is **no clean optimum** — that's the point. Tactics are near-solvable; *this* is the
judgment layer the Charter wants kept uncomputable (#2).

## 6. The clock & the run goal — a balancing harness *(testing instrument)*

> **Provisional & deliberately un-thematic.** This is a **measurement scaffold** for tuning
> scenarios, not the final run-victory design. It stands in for the still-undefined thematic
> run goal (§8 / roadmap) so we can balance *before* we theme.

**The event deck is the clock.** The strategic **event deck** drives world time. For now it
holds **only "1 day passes" cards** — it does nothing but advance the calendar, a placeholder
where real **world events** will later live (roadmap). One draw = **one day**.

**The goal: clear everything, fastest.** A run is complete when **every location is cleared at
its max level**. The **score is the number of days** it took — lower is better, like **golf**:
each scenario has a **par** (the fewest days to do everything possible), and a build/route is
judged against it.

**Why this is the right test instrument.** Minimising days is a single scalar that **stresses
every system at once**: routing (travel), encounter difficulty (a **failed fight wastes a
day** — §2's lost turn is now measured in score), the **currency economy** (do you have the power
to clear max level without grinding?), and the **depth/breadth** ordering. If par is
unreachable, trivially low, or hit by a degenerate line, the scenario is mistuned — exactly
the signal we want (§8's "challenge tuned to party total"). *Par is itself computable by search
over a scenario — a natural future tooling/AI target (roadmap).* The concrete instrument that
realises this goal is the **reference scenario** — a diagnostic A/B/C/final lattice maintained as a
test ([`reference-scenario.md`](reference-scenario.md)).

**The day — the action cycle.** Time advances in **days**. On each day **every character** may,
independently:

- **move** one adjacent space (§1),
- use a **per-day ability** — a class of overworld/strategic abilities (e.g. **scouting**,
  §4.1), distinct from in-encounter tactical play *(which exact abilities is open — see
  dials)*, and
- attempt **one encounter** (§2).

When **all characters have done what they want**, the party **ends the day together** → draw
one **"1 day passes"** event → **day++**. So **the run-clock ticks one day per action-cycle**, and
within a day the party acts **in parallel** — order-independent and co-op, consistent with the
no-initiative principle (§3.1: Speed sizes budgets, never turn order). A **failed encounter spends that
character's one encounter for the day** (it retreats, §2), so "keep trying" plays out **across
days** — exactly the time cost the golf score measures.

**Time vocabulary (largest → smallest):** **Run ⊃ Day ⊃ Encounter ⊃ Round ⊃ Phase**, and within a
Clash, **Beat**. A *Round* is one Deploy→Vanguard→Skirmisher→Reserve pass (§4); a *Day* holds **one
Encounter per character**; **Tempo/Focus refresh each Round**, while **Health and every pool reset
at the Day boundary** (zones-exhaustion §7). *"Turn" is **not** a unit — combat has no turn order
(§3.1).* The complete cycle/phase map is [`game-flow.md`](game-flow.md).

**Full recovery at day's end.** When the day passes, every surviving member's **Body restores
to full** (atop §2.1's restore-on-win), so each day's one encounter is fought **fresh**. There
is **no cross-day attrition** for now — Body is not yet a strategic resource between days.
*(That is a natural lever for later, when we design how the game tries to kill you; until then,
the only thing the run spends is days.)*

**Parallelism is the throughput dial.** Because each character gets a move + an encounter
*per day*, a party clears faster by **spreading out and acting in parallel** — but only if each
member is strong enough to win their encounter alone. So par rewards both **routing** (cover
the map without backtracking) and **even party development** (no one too weak to solo their
share) — exactly what we want to stress-test.

## 7. Characters — clean slate, deck-as-stats, the five roles *(#8 deck-is-character, §6, §2.1)*

**No stats on the identity card.** A character's `ActorCard` is a **bare identity** — a name and
a map token, nothing more. **Every stat and capability lives in the deck** (#8; §6: "a character
is a set of never-shuffled decks"), extending §2.1's "read it off the table" from defense to
*all* stats. Buying a card = raising a stat = gaining an ability — one act, not three.

**Clean slate → a direction.** A character starts with **only generic cards** (a minimal
baseline), and its **first level-1 fight commits a direction** — the role-currency it begins
banking. From there it **specializes** by spending that currency (depth) or **branches** into others
(breadth).

**The five roles = the five non-generic currencies.** Not a chosen list — they fall out of the
§4 triangle and its sub-axes:

| Role            | Falls out of                                           | Currency   | (cast) |
| --------------- | ------------------------------------------------------ | ---------- | ------ |
| **Wall**        | Vanguard that **holds** — Mind→Focus block, Body tank  | **Iron**   | Anvil  |
| **Infiltrator** | Vanguard that **slips** — Speed→Tempo, melee assassin  | **Silver** | Wisp   |
| **Artillery**   | Reserve **ranged damage**                              | **Brass**  | Sear   |
| **Controller**  | Reserve that **strips foes** — Tempo/Focus/Speed, Fear | **Bone**   | Hex    |
| **Support**     | Reserve that **aids allies** — heal/ward/haste/rally   | **Salt**   | Vow    |

Plus the **generic** currency, **Gold** (role-independent utility) → **6 currencies / 6 threat
decks** (§4); the Gold deck is the **clean-slate / early** flavor you fight *before* choosing.

**Depth/breadth is fractal.** Specialize one role (deep) vs cover several (broad) is the *same*
opportunity-cost fork as which locations to clear — now at character-build scale. Party size
sets where you sit on it: many bodies → specialists; few → multi-role; one → a **god** spanning
all five. That spectrum **is** the god ≈ party-total budget (#4/§8).

**Creatures: one engine, different mind.** Creatures are **also deck-built**, so **abilities and
effects resolve through the same engine** as characters — the load-bearing invariant (a card
works identically on either; don't deviate from it). They differ in **exactly one** way: **no
theory of mind** (the Spec §7 Character/Creature line) — a character *reasons and predicts you
back*; a creature runs an **instinct/behavior deck** (its draw is its decision, one-way). A
creature's deck isn't bought card-by-card; it's **assembled from a parametric recipe — the
encounter card (§4.1) is a deck recipe**, its level formulas naming which cards to build and how
they scale. Stats emerge from the deck for creatures too; the recipe just *builds* the deck a
player would otherwise *buy*.

---

## Open dials (carry-forward — not yet decided)

1. **Run-level defeat** — how the game tries to kill you; what ends a run as a loss. Until
   this exists, retreat only costs **lost days vs par** (§2/§6) and there is **no cross-day
   attrition** (full recovery at day's end, §6) — both placeholders. *(Spec §8.)*
2. **Upgrades ↔ exhaustion (§5)** — *designed:* see
   [`zones-exhaustion-design.md`](zones-exhaustion-design.md). An Upgrade is a **Form attachment**
   (permanent) or an **Action card** (zone-governed Spend/Recover). The detailed **Resource layer**
   (health / Tempo / Focus pools) is the remaining open piece there. *(§3.3.)*
3. **Encounter specifics (§4.1)** — *resolved.* Reveal = currency type → threat deck (not the
   card). The encounter is **drawn & fixed on first engagement** — **only a fight locks it**;
   a once-rolled composition stays fixed on retry (the tutorial property). A **non-core
   scouting ability** may preview it without fighting (its own mechanics deferred). Scaling is
   **thematic & flexible** (any stat, signalling its counter, bounded by table arithmetic).
   The encounter card sets the **general strategy** and names the **decks / compositions** for
   randomness — once-fixed *strategic* + live *tactical* (§7) layers.
4. **Travel — deferred (not now).** No travel cost or risk beyond one-space-per-day in the
   current harness; revisit alongside world events. *(Roadmap: Travel.)*
5. **Run victory** — *provisional (§6):* a **balancing harness** — clear **all locations at
   max level**, score = **days elapsed** (golf, vs par). The **thematic** run goal is still
   unnamed. *(Spec §8.)*
6. **Numbers** — currency value per level, depth scaling, Upgrade prices, threat-deck contents.
   AI-seeded, **human-tuned**, in `booklet.ron`.
7. **Naming — resolved.** Companion = **treasure card** (carries typed **currency**); level-tally
   = **clear marker** (renamed off "counter" to dodge the duelist role *Counter* / §3
   *counterattack*); **Upgrade** = the *role* a currency-bought catalog card plays (not a new
   type); **identity card** = the character's `ActorCard`. See *Card catalog & naming audit*.
8. **Per-day abilities — deferred (not now).** No overworld/strategic per-day abilities
   (including **scouting**) in the current harness; §6's "certain type of ability" slot exists
   but is unused for now. Revisit with §5 and the ability taxonomy.

## Card catalog & naming audit

> The whole game spans three card layers — **tactical** (resolving a fight), **character**
> (who a fighter is), and **strategic** (this doc). This audits the **strategic-layer** cards
> we've introduced and checks them against the **existing** print-master vocabulary
> (`booklet.ron`: `ActorCard`, `Card`, `TraitCard`, `ScenarioCard`) and the Spec's physical
> cards.

**Strategic-layer cards (this design).**

| Card               | What it is                                                            | Audit note                                                                                                        |
| ------------------ | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **Location card**  | Face-down map tile; flips face-up on entry (§1).                      | New; no collision.                                                                                                |
| **Identity card**  | A character's position token on the map (§1).                         | **Resolved:** not a new card — the character's `ActorCard` *is* the map token.                                    |
| **Treasure card**  | A location's companion, taken on clearing; carries **currency** (§3). | Renamed from "reward card."                                                                                       |
| **Clear marker**   | Per-location high-water mark = deepest level cleared (§2).            | Renamed off "counter" (collided with duelist role *Counter* + §3 *counterattack*); matches §8's "cleared marker." |
| **Encounter card** | Parametric enemy configuration, scaled by level (§4.1).               | New; resolves to a set of `ActorCard`s.                                                                           |
| **Event card**     | The world clock; for now only *"1 day passes"* (§6).                  | New; no collision.                                                                                                |
| **Upgrade**        | A card bought with currency that strengthens a character (§3.3).      | **Resolved:** not a card type — the **role** a bought catalog card plays (see flag 1).                            |

Decks (not cards): the **threat deck** (one per currency type, of encounter cards), the **event
deck** (the clock), and the **Upgrade deck** (the buyable catalog subset).

**Currency, not a card.** **Currency** (six types — Iron, Silver, Brass, Bone, Salt, and
generic Gold) is a **value printed on treasure cards**, valued at the clear marker — not its
own card type.

**Existing vocabulary (for reference).** `ActorCard` (a fighter: stats, role, driver, weapon,
actions, traits, attack), `Card` (one struct, **three uses** — weapon, power/action card,
passive power), `TraitCard` (armor/ward), `ScenarioCard` (a battle setup). Physical Spec
cards: the **Clash kit** (Strike/Anticipate/Gather/Evade), **number cards 0–9** + **decoy
cards** (§4 assemble/lanes), and **Body Health** cards (§2.1). *(The multi-deck aspect/chord card
kinds — numberless / modifier / passive — are deferred; `future-possibilities.md` entry 4.)*

**Flags the audit raised.**

1. **"Upgrade" is a role, not a card type — *resolved*.** What you buy to get stronger *is* a
   character card (a power `Card`, a `TraitCard`, or a weapon). So **"Upgrade" is the role a card
   plays when purchased**, not a fourth card type, and the **Upgrade deck** = the currency-buyable
   subset of the catalog.
2. **The identity card is the `ActorCard` — *resolved*.** The character already *is* an
   `ActorCard`; that same card is its map token. No separate token card.
3. **Pre-existing overloads (not ours — flagged for hygiene).** **"Ward"** already means three
   things — a power `Card` (grant melee), the **inner-defense cut** stat (§2.2), and the
   **Ward-charm** trait — and **`Card`** is triple-purposed. These predate this doc; a future
   cleanup, out of scope here.

## Maps onto

- **Spec Coverage table** rows this will graduate into: *Strategic layer (world/event
  decks)*, *Geography & travel*, *Loot*, *Progression*, *Run victory/defeat* — all ⬜ stub
  today (`canon/2-spec/README.md`).
- **Stats-as-deck — GRADUATED to Spec §2.3 / §4.3.** Stats now live in the deck (Form); the
  `ActorCard` becomes a bare identity + starting deck. The Spec amendment is done; the `booklet.ron`
  schema + Rust struct + §4 reader migration is the pending **`/spec-sync §2,§4`** code pass.
- **Prior thinking:** `notes/world-and-progression.md`, `notes/cards-and-customization.md`,
  `notes/archetypes.md` (frozen, non-authoritative).
