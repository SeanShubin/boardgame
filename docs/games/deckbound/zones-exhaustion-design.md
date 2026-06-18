# Deckbound — Zones & Exhaustion (design in progress)

> **Status: living, non-authoritative.** This is the first-pass design for **Spec §5
> (Zones / exhaustion)** — the system the source-of-truth calls *"the biggest known
> mechanical hole (the orphaned exhaustion economy)."* It is **not** a source of truth; when
> settled it graduates onto the Spec via the change discipline in
> [`canon/0-source-of-truth.md`](canon/0-source-of-truth.md) (**spec first**, then
> `booklet.ron` numbers, then code, then tests).
>
> Sibling to [`progression-design.md`](progression-design.md), which introduced **deck-as-stats**
> and **Upgrades** (its §7) — this doc defines the card machinery those rely on. Numbers here are
> **illustrative**; real values are AI-seeded, human-tuned, in `booklet.ron`.

---

## The core idea — exhaustion, not cooldowns *(#7 cards-only, #8 predictability, #6 few rules)*

A **cooldown is a hidden timer**, which **cards-only (#7) forbids**. The cards-native substitute
is **exhaustion**: using a card **moves it to a visible spent zone**; it's unavailable until
restored. That's a card-state on the table, no hidden counter — and it is exactly what **#8**
asks for: *"unpredictability is a managed resource that erodes as cards exhaust and is restored
only at a tempo cost."*

So the whole system is a **zone state-machine**: every card lives in a zone and carries **its own
rules for entering and leaving zones**, and a card's effect **can move other cards between
zones.** That one idea generates cooldowns, combos, charge-up bursts, engines, and disruption
from a handful of verbs (#6) — and it **re-pins #8** from the old "never-shuffled deck" framing
onto **zone management** (same WHY — no luck, predictability is a visible managed resource; new
mechanism). This is what closes the orphaned-exhaustion hole.

**Scope: intra-encounter only.** With the progression layer's **full recovery at day's end** and
**one encounter per day**, exhaustion shapes the arc of a *single fight* and never carries across
days. That keeps the whole system tractable.

---

## 1. The three zones *(#9 physical metaphor)*

Facing encodes **state, not secrecy**: **face-up = in play / working / available; face-down =
spent / dormant.** (See §4 — the core game is open information.)

| Zone                              | Where / facing       | What's there                                                                                                            |
| --------------------------------- | -------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Hand** — *in your grip*         | held                 | cards ready to play                                                                                                     |
| **Active** — *in play, in effect* | table, **face-up**   | everything working: your **Form** (permanent stat cards), **Lasting** stances/auras, and **charges** waiting to trigger |
| **Down** — *spent / dormant*      | table, **face-down** | used cards and cards on "cooldown," waiting to be **Recovered** to Hand                                                 |

A card declares its **start zone**: most start in **Hand**; Form and standing stances start in
**Active**; a charge-up / cooldown ability can start in **Down**, needing to be readied first.

*(Layout note: physically you'd lay Form cards in their own region — a character mat — for
readability, but that is layout, not a separate rules-zone. Active is one zone.)*

## 2. Card properties within Active — Form vs the rest *(#8, §2.1)*

"Active" is one zone, but cards in it behave differently. The key split is the one the progression
doc's §7 names — **what you *are* vs what you *do*:**

- **Form** — your **fundamental card + attachments** (your stats; see §3). **Permanent: never
  Spends, immune to Disrupt** — it cannot be knocked Down. This is what keeps *"what you are never
  exhausts"* true now that Form is a **property, not a zone wall.**
  - Stats can still be **temporarily reduced** by **Lasting debuffs** sitting in Active (a Slow, a
    Sunder, a Confuse) — remove the debuff and the stat returns. The Form card itself never leaves,
    so stats stay stable and recomputable (§2.1) while debuffs still bite.
- **Lasting** — stances, auras, **charges**: enter Active when played and stay **until removed,
  Disrupted, or consumed**.

So *exhaustion touches what you do, never what you are* — preserved as a card property.

## 3. The verbs — four, plus a default *(#6, #9, #10)*

The **default** *is* the "simple card": play it, it **returns to Hand**, reusable next turn (the
Clash kit is the pure case — see §6). Keywords modify that; each is one printable **MANUAL** line:

| Verb        | Metaphor (its MANUAL line)              | Zone move                                           |
| ----------- | --------------------------------------- | --------------------------------------------------- |
| *(default)* | a jab you keep throwing                 | play → **Hand**                                     |
| **Spend**   | used up, winded                         | play → **Down** (face-down)                         |
| **Lasting** | a held stance / aura                    | play → **Active** (stays until removed)             |
| **Recover** | catch your breath; stand a card back up | **Down → Hand** (the restore; costs a beat / Tempo) |
| **Disrupt** | stagger them — knock it down            | enemy **Active / Hand → Down**                      |

**Emergent from these four (the #6 payoff):**

- **Cooldown** = **Spend** + a gated **Recover** (the gap between is the cooldown).
- **Combo** = a card whose effect **Recovers** (or consumes) a specific tagged card.
- **Engine** = a **Lasting** card that **Recovers** your pile each turn.
- **Disruption** = **Disrupt**.

The single-sentence MANUAL line per verb (and per card) is the **discipline that stops per-card
depth from exploding** past table-memorability (#9) and motivation (#10).

## 4. Tags — bounded cross-card interaction *(#6, #10)*

Cards combo by referencing each other **by type/tag, never by name** — a small, bounded tag
vocabulary (the existing damage types — Fire / Sharp / Blunt — are the seed). This is what lets the
charge example work with no new zone:

> **Worked example — the fire charge-up.**
> 1. **Turn 1–2:** play two **Charge(fire)** cards → they sit in **Active**, Lasting.
> 2. **Turn 3:** play **Fire** from Hand. Its effect reads Active, finds the two **Charge(fire)**,
>    deals damage ×2×2, then **moves** them — Charges → **Hand** (reusable), Fire → **Down**.
>
> All zone-moves, no new zone. The cost is **action-economy** — you spent turns 1–2 charging
> instead of attacking, and Fire is now Down — so **burst is paid for, not free** (magnitude is a
> number, human-tuned). The cycle is **per-card flexible**: charges *Recur* to Hand here
> (recharge at once); a heavier design *Spends* them to Down (a longer cool-down). Same primitives,
> different feel.

## 5. Form = fundamental card + attachments *(progression §7, §2.1, #4)*

"What you are" is built, not authored on the actor card (progression §7 — clean slate, deck-as-stats):

- A **fundamental identity card** *derives* your stats, modified by **attachment** cards along
  each dimension.
- The model generalizes your remembered health design: **Body pool = count × value**, both set by
  the fundamental card and pushed by attachments — two ways to get tougher (more health cards, or
  higher-value ones), itself a depth/breadth fork. This *is* §2.1's "Health pool = face-down
  cards / accumulation is cards in a zone."
- **Upgrades (progression §3.3) = attachments to Form** — buying = attaching a +count / +value /
  +Speed card — always-on, never exhausting, fully recomputable (§2.1). Clean slate = bare
  fundamental card; specialization = accreted attachments.

So an Upgrade is **either** a Form attachment (permanent, what you are) **or** an Action card
(zone-governed, what you do) — never an unbounded spammable power. That is what makes the
acquisition economy balanceable (it was the open dependency in progression §3.3).

## 6. Hidden information — open by default *(#3, §1.9, §4.2)*

The core game is **open information**; **facing = state, not concealment**. Hidden information is
**opt-in** and lives in exactly two places:

- The **Clash** card-pick — the optional module (§4.2) that turns a same-range **mutual trade**
  into *"one side does better/worse."* The base game is fully playable without it.
- An optional **PvP commit-then-reveal** for standoffs — which the **simultaneous-by-phase**
  resolution (§1.9 / §3.4) likely makes unnecessary. In co-op, players simply act in **any order**
  (order-independent); two players who insist on waiting each other out can use the commit-reveal,
  but it is never core.

The **Clash kit is the simplest case of this whole system**: four **default-return** cards, always
in Hand (§1.0's "infinite-replay" = "everything is Recur"); heavier maneuvers add Spend / Recover.

## 7. Resources — Health, Tempo, Focus *(§2.1, §3, #4)*

A permanent **Form stat sizes a fluctuating Resource pool** — you never spend the stat, you spend
the pool it sizes (§3.1, *"Speed sizes Tempo"*):

| Stat (Form, permanent) | sizes → | Resource pool (fluctuates) | spent on                          |
| ---------------------- | ------- | -------------------------- | --------------------------------- |
| **Toughness / Body**   | →       | **Health**                 | taking damage                     |
| **Speed**              | →       | **Tempo**                  | acting (initiate / slip / target) |
| **Mind**               | →       | **Focus**                  | defending (block / survive)       |

Each pool is a **count × value card-pile** in **Active** (face-up = intact / available); spending
it **moves cards to Down** (face-down = lost / spent), and it returns by the same **Recover** verb
(§3). Two growth dials — more cards (granular) or higher value (lumpy) — so Upgrades thicken a pool
along either axis (a fractal of depth/breadth).

**Three Recover sources, by pool:**

- **Round refresh** *(Tempo / Focus only)* — at round start **all spent Tempo/Focus flip back up**.
  They are *re-derived each round* (§2.1), so they are **per-round budgets, not cross-round
  attrition**.
- **Heal cards** *(Health)* — Mend-type effects **Recover** Health *within* a fight.
- **Refresh engines** *(the god)* — a **Lasting** card that **Recovers** Tempo/Focus mid-round, so
  a god **exceeds its base per-round breadth** (acts far more than its Speed alone would allow).

**The §2.1 meter rule is untouched.** **Health is the one pool that persists within a fight** (the
maintained meter); Tempo/Focus auto-refresh each round (not maintained). Everything **fully
Recovers at day's end** (progression §6), and Health restores on a win (§2.1). The card model just
*expresses* §2.1/§3 in the zone machinery — one set of verbs now governs **actions and resources
alike**.

**What exhaustion is *for* — resolved (was point 3).** In **co-op PvE vs instinct creatures** (who
do not read you, §7-Agents), the limiter is **action-economy / attrition**: you cannot spam because
**Spend** sends a card **Down** and **Recover** costs a beat / Tempo, bursts cost setup turns, and
the resource pools cap per-round breadth. The **predictability-telegraph** half of #8 fully bites
only in **PvP / vs Characters**. So a fight is **sequencing a limited budget**, and the master
tunable is **Recover/refresh rate vs Spend rate**.

---

## Open dials (carry-forward — not yet decided)

1. **Attachment composition (§5)** — commutative by default (recompute-clean), with
   order-dependence reserved for explicit "modifier" cards (§6 of the Spec already flags
   "attachment order matters"). Confirm.
2. **Final verb & tag vocabularies** — the four verbs + default and the tag set are seeds; pinning
   the exhaustive list and each MANUAL line *is* the §5 graduation work.
3. **Resources (§7) — designed.** Health / Tempo / Focus = stat-sized **count × value** pools on
   the zone machinery; round-refresh (Tempo/Focus), heal-cards (Health), refresh-engines (god);
   exhaustion-as-attrition is the PvE lean. *Confirmed:* Tempo/Focus fully auto-refresh each
   **Round** (§3). Remaining = numbers (#4).
4. **Numbers** — Spend/Recover costs, charge magnitudes, pool sizes — AI-seeded, human-tuned, in
   `booklet.ron`.

## Maps onto

- **Spec Coverage table:** this is **§5 (Zones / exhaustion)**, now 🟡 graduated
  (`canon/2-spec/README.md`). It also touches **§1.0** (the Clash = the all-default case) and
  **§2.1** (health = face-down cards). *(The multi-deck **aspect/chord combo** layer is deferred —
  `future-possibilities.md` entry 4; the single-deck core uses Form + attachments, composed
  commutatively.)*
- **Companion design:** [`progression-design.md`](progression-design.md) §7 (deck-as-stats,
  clean-slate characters, Upgrades) — together these two docs define the character + economy layer.
- **Re-pins north star #8** (predictability-as-resource) from "never-shuffled deck" onto zone
  management — a deliberate, human-owned refinement of the pillar's *mechanism*, preserving its WHY.
- **Prior thinking:** `notes/zones.md` (frozen, non-authoritative; predates the Clash — needs the
  rewrite this doc is the first pass of).
