# Gear — folding weapons & armor into suits / treasures / stats (identity model)

**Status:** design locked in conversation 2026-06-23; staged for merge. **For the
gear instance** — you own the mechanical definitions (the Plate/Mail/Padded/Cloth
× Pierce/Slash/Crush matchup, currently in the `combat-lab` crate). This doc is
the **integration layer**: how gear attaches to the existing suit / treasure /
stat / role systems without breaking conservation or the Damage Separation Law.

**Decision:** **gear is identity, not an economy.** Gold is retired; there is no
6th location and no new card pool. **Gear *subsumes* the bundled Stat card** in
each of the 25 `(Suit, level)` rewards — the stat card gains a **type tag** and a
**name**, nothing more.

**Merge targets:** a new **§2.8 (Gear)** in the defense/stat block (or an
extension of §2.2), plus an extension of **§8.3 (Rewards)**. Reconcile damage
types with code `stats.rs`.

---

## 1. The core move: gear = a typed Stat card

The five Suits are **substances** (§8.5: "the Suit is what a reward *is*"). So an
"Iron reward, +2 Body +1 Toughness" was *always* a piece of iron — the stats were
written without naming the object. Gear just **names it and tags it**:

- A reward's bundled **Stat card** (§5.6 "Stat" kind, a Form attachment) keeps its
  magnitudes and gains **one** new field: a **type tag**.
  - **Weapon** → a **damage type** (+ Strike/Power magnitude; + a **Fast/Slow**
    window tag if ranged, §4.6).
  - **Armor** → an **armor type** (Plate/Mail/Padded/Cloth).
  - **Implement** (effect roles) → no damage; raises a **Power** stat
    (Dread / Inspiration).
- **No card-count change. Conservation (§4.4) is untouched** — the 25-card pool is
  enriched in place, not grown. The fixed pool still partitions across bodies; god
  ≈ party unchanged.

## 2. Damage types — reconcile to the physical triad + elements + Fear

Code `stats.rs` currently has Blunt / Sharp / Pierce / Heat / Cold / Lightning /
Fear. Align to the gear model:

- **Physical triad: Crush · Slash · Pierce** — rename **Blunt → Crush**, **Sharp →
  Slash**, Pierce unchanged. These are the only types the armor chart reads.
- **Elemental: Heat · Cold · Lightning** — magic-weapon damage. Proposed: they
  **bypass the physical armor chart** (Normal effectiveness, hitting only the flat
  elemental Armor cut). That bypass *is* the magic weapon's appeal — and its own
  counters live in elemental Armor values, not the Plate/Mail/Padded chart.
- **Fear** — unchanged; the Controller's **inner** channel (Ward / Resolve, §2.2),
  never a weapon.

## 3. Armor types as cut-profiles (reuse §2.2, no new multiplier)

§2.2 already models Armor as a **typed per-source cut**. So the armor **type** is
just a **preset profile of per-type cuts** — the combat-lab matchup chart collapses
into the cut numbers §2.2 already subtracts. No ×2/×1/×½ multiplier needed.

| Armor      | Crush cut | Slash cut | Pierce cut | Vulnerable to       |
| ---------- | :-------: | :-------: | :--------: | ------------------- |
| **Plate**  | low       | high      | high       | **Crush**           |
| **Mail**   | high      | high      | low        | **Pierce**          |
| **Padded** | high      | low       | mid        | **Slash**           |
| **Cloth**  | low       | low       | low        | (baseline — see §4) |

Each non-Cloth armor **resists two, is soft to one** of the triad. (Exact numbers
are a booklet dial, like the §4 bid magnitudes.)

## 4. Cloth = the unarmored baseline

Everyone starts at **Cloth** (the Human chassis / clean-slate, §2.3); Plate / Mail
/ Padded are the **upgrades** frontline suits acquire. So a caster who never gets
armor gear simply **stays Cloth** — no special-casing, and only **three** real
armor upgrades exist to author across the pool.

## 5. Per-suit gear (identity-bound)

| Suit / Role              | Weapon / implement    | Deals damage?               | Armor      |
| ------------------------ | --------------------- | --------------------------- | ---------- |
| **Iron / Wall**          | plate + mace / hammer | **Crush**                   | **Plate**  |
| **Silver / Infiltrator** | light blade           | **Slash / Pierce**          | **Padded** |
| **Brass / Artillery**    | bow *or* damage-wand  | **Pierce** / element        | **Mail**   |
| **Bone / Controller**    | staff — **implement** | **no** (raises Dread)       | **Cloth**  |
| **Salt / Support**       | focus — **implement** | **no** (raises Inspiration) | **Cloth**  |

**Damage Separation Law (Charter #13) holds automatically.** Effect-role "gear" is
an *implement* that raises a **Power** stat, never a weapon — a damage-wand is
**Brass**, a control-staff is **Bone**, a blessing-focus is **Salt**. Same
silhouette, role decides function. Only the triangle (Iron / Silver / Brass)
carries damage-weapons.

**Ranged gear → §4.6.** Bows and damage-wands are Artillery ranged attacks and
carry the **Fast / Slow** window tag printed on the gear card.

## 6. What "identity" gives up (and what survives)

Armor type rides on the suit, so the player **cannot** armor up against a specific
threat — the Pierce/Slash/Crush chart is **identity matchup**, not a per-battle
choice. *Chosen deliberately:* a Wall **is** plate.

**Mix-and-match still exists** — but through **cardset assignment**, not a shop.
`build_character` already takes any set of rewards, so assigning a Wall a secondary
**Brass** cardset puts a bow in its hands. The only thing truly gone is choosing
armor *independent of identity*.

## 7. Open dials / migration items (for the gear instance)

1. **Damage-type rename** Blunt→Crush, Sharp→Slash in `stats.rs` and the §2.2
   pipeline (touches the defense reader).
2. **Cut-profile numbers** for Plate/Mail/Padded → booklet, human-tuned.
3. **Elementals vs the chart** — proposed bypass (flat elemental cut only);
   confirm.
4. **Brittle vs permanent armor** — §2.2 says Armor "never depletes"; combat-lab
   has a **brittle** (depletable) keyword. Pick one; identity model defaults to
   §2.2's permanent cut.
5. **Combat-lab keywords** — `pierce` (Half→Normal upgrade), `cleave`, `persist`
   need a canon home or explicit retirement when the matchup graduates out of the
   sandbox crate.
