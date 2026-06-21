# Card Representation — §2.4–§2.7 implementation design

**Status:** design pass for ratification (2026-06-21). Resolves the gaps the `/spec-sync`
completeness gate flagged for Spec **§2.4–§2.7** (Quantity/Power suits · base-2 denominations ·
deck-tree positional notation · reset clocks), so they become *implementable and printable* when the
print/UI/balance layers are scheduled. **No code or `booklet.ron` change is proposed here** — this is
the component-level spec the gate said was missing.

Cross-refs: `canon/2-spec/README.md` §2.1–§2.7; `presentation/card-table-ui.md` (the deck metaphor the
§2.6 tree feeds); the locked decision in §2.3 (a character is a bare identity + Form cards).

---

## 0. The load-bearing decision — *projection, not re-authoring*

§2.4–§2.6 are a **generated view** of the scalar Form, **not** a new data format.

- **Authored truth stays scalar.** `booklet.ron` keeps `(body: 5, toughness: 1, …)` and the engine
  keeps summing a flat `Form` (`form.rs`). This is what makes tuning cheap (edit a number, done) and
  honors "tuning first."
- **The deck-tree of base-2 Quantity/Power leaves is *derived*** from those scalars by a deterministic
  generator (§2 below), exactly as the source-of-truth says human-readable sheets are "generated
  projections of `booklet.ron`, never hand-maintained."
- **Consequence:** no engine refactor, no data migration, no number tuning to do §2.4–§2.6. The only
  new code (when scheduled) is the *generator* + its consumers (a print card-sheet, later the UI tree).

This is the single recommendation that de-risks the whole block: the locked representation is real and
canonical, but it lands as a projection over the unchanged core.

---

## 1. The deck catalogue — every stat as `deck × suit`

Each engine stat maps to exactly one `(deck, suit)` cell. **Pools** carry both suits; **flats** carry
Power only. Suits are global: **Quantity = how many, Power = how hard** (§2.4).

| Deck | Quantity (breadth) | Power (depth) | Channel / note |
|---|---|---|---|
| **Body** | health-card count (`body`) | Toughness (`toughness`) | the maintained pool (§2.1) |
| **Tempo** | Speed (`speed`) | Drive (`drive`) | ephemeral per-round pool (§3) |
| **Strike** | — | strike power (`power`) | the legacy "Power" stat ≡ Strike·Power (§2.4) |
| **Spirit** | — | Spirit (`spirit`) | fear-strike force (inner offense) |
| **Pierce** | — | Precision (`precision`) | armor bypass |
| **Resolve** | — | Resolve (`resolve`) | the inner **bar** — no pool (§2.2), so no Quantity |
| **Guard·⟨type⟩** | — | Armor cut for that type | one deck **per damage type** (see D-armor) |
| **Ward·Fear** | — | Ward cut vs Fear | the inner cut |

Not a suit: **Keystone** (which aspect is lethal, §2 stats) is a categorical **marker card**, not a
magnitude — it names a deck as the kill-condition, it has no Quantity/Power value.

**Quantity appears only on the two genuine pools (Body, Tempo).** Everything else is "a flat magnitude =
Power under its deck," which is why two suit-names cover the whole stat space.

### D-armor — typed cuts become per-type decks

`Armor`/`Ward` are typed in the engine (`armor: BTreeMap<DamageType,u32>`). Recommended: **one
`Guard·⟨type⟩` deck per damage type** (Guard·Blunt, Guard·Sharp, …) and `Ward·Fear`, each a Power-only
deck. The damage **type comes from the deck (position)**, so a leaf still carries only `(suit, value)` —
consistent with §2.6. The alternative (one `Guard` deck with type-tagged leaves) would force a *type*
dimension onto the leaf, breaking the "leaf = suit + value" guarantee. Most actors have only a couple of
non-empty Guard decks, so the tree stays sparse.

> **Out of scope (separate decision):** *collapsing* the eight damage types (Blunt/Sharp/Pierce/Heat/
> Cold/Lightning + Fear/Confusion) to a smaller set would shrink this catalogue, but damage types are
> load-bearing for §2.2's called-shot mechanic — that is a **§2.2 mechanics change (case 3)**, not a
> representation choice, and must be decided on its own. This design takes the types as-given.

---

## 2. The base-2 projection rule

For a scalar value **V** in cell `(deck, suit)`, the leaves are the **binary expansion of V**: one leaf
per set bit, denomination = that bit's place value, **at most one of each** (§2.5). Card-count =
**popcount(V)**. The form is unique (no second way to make V), so the generator is deterministic.

- `body 5` → Body·Quantity `[4] [1]`
- `toughness 6` → Body·Power `[4] [2]`
- `drive 0` → (no leaves)

**Consumable pools (Body·Quantity).** The Health count is the one *maintained* meter (§2.1); as cards
flip face-down the projection re-renders the remaining count's canonical form (16+2 → 16+1 → 8+4+2+1 …).
The engine still decrements the scalar; "make change" is a *view* concern, invisible to the rules.
(Read-once Power stats never change mid-combat, so they never re-render.)

**Why this is safe with the maintained-meter rule:** §2.1 says exactly one quantity is tracked (Body
Health). The projection doesn't add a second tracked number — it *renders* the one that already exists.

---

## 3. §2.7 reset clocks — already conformant

The three clocks §2.7 names are **already implemented**; this is a documentation mapping, **no code**:

| Clock | Engine reality | §2.7 |
|---|---|---|
| **per hit** | `Defense::take` subtracts the Armor cut on every call | Armor |
| **per round** | `Defense::end_round` zeroes `body_pile` (the sub-Toughness remainder) | Toughness |
| **per encounter** | flipped Health cards persist; restored only on a win (§2.1) | Health |

The only *future* §2.7 work is promoting "clock" to a **named, data-driven property** so new clocks
(per-exchange, per-attacker) can be minted — explicitly deferred until a fourth clock is actually wanted.

---

## 4. The "Power" name — coexistence, no rename

The engine's summed-stat field names (`power`, `toughness`, `drive`, `speed`, `body`, `resolve`, …)
**stay as they are**. The Quantity/Power **suits** are the *card/print/UI* vocabulary, living in the
projection layer; the field names are the *summed* vocabulary. They coexist with no churn:
`offense.power` **is** the sum of the Strike·Power leaves. No field rename is needed or proposed.

---

## 5. Worked deck-trees (the rules-tour cast — proof the projection reproduces real builds)

Each is the *generated* tree from the actor's actual summed stats (Novice clean-slate + one full reward
track). Leaves shown as denominations; `pc` = popcount = card-count for that cell.

**Anvil — Iron / Wall** · body 18, tough 6, spd 3, drv 0, pow 1, res 1
```
Body    Quantity [16][2] (pc2)   Power [4][2] (pc2)
Tempo   Quantity [2][1]  (pc2)   Power —      (drv 0)
Strike  Power [1] (pc1)
Resolve Power [1] (pc1)
                                              total 8 leaves
```

**Wisp — Silver / Infiltrator** · body 5, tough 1, spd 9, drv 4, pow 6, res 1
```
Body    Quantity [4][1] (pc2)    Power [1] (pc1)
Tempo   Quantity [8][1] (pc2)    Power [4] (pc1)
Strike  Power [4][2] (pc2)
Resolve Power [1] (pc1)
                                              total 9 leaves
```

**Sear — Brass / Artillery** · body 5, tough 1, spd 3, pow 9, precision 2, res 1
```
Body    Quantity [4][1] (pc2)    Power [1] (pc1)
Tempo   Quantity [2][1] (pc2)
Strike  Power [8][1] (pc2)
Pierce  Power [2] (pc1)
Resolve Power [1] (pc1)
                                              total 9 leaves
```

**Hex — Bone / Controller** · body 5, tough 1, spd 3, pow 1, spirit 8, res 3
```
Body    Quantity [4][1] (pc2)    Power [1] (pc1)
Tempo   Quantity [2][1] (pc2)
Strike  Power [1] (pc1)
Spirit  Power [8] (pc1)
Resolve Power [2][1] (pc2)
                                              total 9 leaves
```

The popcount totals are the per-build "card cost" the §2.5 balance tiebreaker would minimize.

---

## 6. Decisions for ratification

| # | Decision | Recommendation | Type |
|---|---|---|---|
| **R1** | §2.4–§2.6 as a **generated projection** of scalar Form, vs a re-authored `.ron` tree | **Projection** (§0) — tuning-first, no migration, no engine churn | mechanics/encoding |
| **R2** | Typed cuts = **per-type `Guard·⟨type⟩` decks**, vs one type-tagged Guard deck | **Per-type decks** (D-armor) — keeps leaf = `(suit,value)` | encoding |
| **R3** | §2.7 = **already conformant, no code** (clock-as-data deferred), vs implement clock-as-data now | **Conformant, no code** (§3) | mechanics |
| **R4** | "Power" field **rename** vs **coexist** with the suit name | **Coexist** (§4) — zero churn | naming |
| **(note)** | Collapse damage types? | **Out of scope** — separate §2.2 case-3 decision | — |

---

## 7. What graduates where, once ratified

- **Spec:** a short §2.4/§2.6 addendum stating the catalogue + the projection rule (the *concept*; numbers
  stay out of the Spec).
- **`booklet.ron`:** **unchanged** — scalars remain the authored truth.
- **Code (deferred until print/UI/balance scheduled):** a `Form → deck-tree` generator (the projection),
  a popcount metric for the balance harness, and a print card-sheet projection. The UI tree (§2.6) is the
  last consumer, gated behind `card-table-ui.md` and "tuning first."
