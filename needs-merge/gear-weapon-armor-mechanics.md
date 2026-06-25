# Gear — weapon/armor combat mechanics (the mechanical definitions)

**Status:** validated in the `combat-lab` crate over conversation 2026-06-23; staged
for merge. This is the **companion** to [`gear-identity-fold-in.md`](gear-identity-fold-in.md)
(the integration layer — how gear attaches to suits/rewards/stats). That doc assigns
*this* instance ownership of the **mechanical definitions**: the
Plate/Mail/Padded/Cloth × Pierce/Slash/Crush matchup and how a hit resolves. This
is that spec.

**Executable source of truth:** `crates/combat-lab/` (Bevy-free, deterministic). The
type chart is `effectiveness()` in `lib.rs`; resolution is `resolver::grind`; the
balance detectors (`detect.rs`) and the report directory (`combat-reports/`) are the
validation harness. The canon mechanical rule should match this reference
implementation, not the other way around.

---

> **⚠ SUPERSEDED in part (2026-06-24, deckbound commit `0d88a52`).** The stat redesign
> landed: stats are now the five **might / vitality / toughness / speed / daring**, the
> Fear channel is gone, and **armor + damage-types are deferred to gear**, with **§2.2
> now explicitly anticipating their return as a "pre-pile *subtract*" (a cut).** That
> **resolves §0 below in favour of subtractive** and **retires the multiplicative
> Latin-square chart (§1)**: the live gear model is **subtractive per-type resistance
> (capped ≤3 so it tilts, never walls) + multi-type "coverage" weapons** on the
> five-stat chassis, implemented and validated in `combat-lab`. RPS is now **emergent**
> — each specialist resists its prey's damage type; no counter table is written — and
> verified by unit test. Read §1–§4 as the design history that *informed* the
> resistance model. The current canonical mechanics are the resistance model; this
> doc's body needs a rewrite to match (next pass).

---

## 0. The decision the merge must make first — *multiplier vs subtractive cut*

**This is a genuine conflict with seeded canon §2.2 and must be resolved by a human
before either gear doc lands.** `gear-identity-fold-in.md` §3 assumes the chart
collapses into §2.2's subtractive per-type Armor cut ("no ×2/×1/×½ multiplier
needed"). The mechanical work in this session deliberately went the *other* way, and
the data says that matters:

|                      | **§2.2 subtractive cut** (canon today)                                            | **Multiplicative type chart** (this session)                       |
| -------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Rule                 | `bite = max(0, Strike − Armor[type])`                                             | `bite = Strike × {2, 1, ½}` (½ floored, **min 1**)                 |
| Small hits           | **fully blocked** if `Strike ≤ Armor`                                             | reduced, never nullified                                           |
| Immunities (∞ edges) | **44** of 91 in the 14-build fixture                                              | **4**                                                              |
| Counter structure    | irregular; armor-stacking near-bosses                                             | a **regular Latin square** (Copeland-variance-0 *by construction*) |
| Game-theory verdict  | immunities = "the worst counter" (`docs/game-theory/measurement-mechanics.md` §7) | soft counters = the Fire Emblem "done right" case                  |

**Classification (per `0-source-of-truth.md`):** swapping §2.2's *outer* cut from
subtractive to multiplicative is a **case-3 intent change** to one §2.2 sub-intent —
"a per-source cut answers *many small hits*." It does **not** break §2.2's structural
GUARANTEES (parallel channels, only Body has a pool, accumulation-as-cards, the
Toughness bar). It relocates the "many small hits" answer:

> In the multiplicative model the "many small hits" answer is no longer the cut — it
> is the **Toughness bar + the per-round reset**: a hit stream whose *per-round*
> accumulation never reaches Toughness is wiped each round and so is walled (the
> handful of remaining ∞ edges are exactly these, and all are vertical-scaling
> breachable). So §2.2's *two* non-redundant mitigations (cut for small, bar for big)
> **fold into one** (the bar), with the type chart as an orthogonal *soft* multiplier.
> Simpler, and immunity-light.

**Recommendation:** adopt the multiplicative chart; rewrite §2.2's outer cut as a
multiplier and let Toughness carry the small-hit answer. It is the validated model and
the one that makes the counter system regular. **But this is a human intent call** —
if canon prefers the literal "armor hard-stops chip damage" flavor, keep §2.2
subtractive and accept the immunity cost. Everything below assumes the recommendation.

---

## 1. The type chart — a regular Latin square

Three weapon channels × three armor types (Cloth = the off-wheel neutral). Each
channel **beats one armor (×2)**, is **resisted by one (×½)**, and is **neutral to one
(×1)** — every row and column holds exactly one of each:

| Strike ↓ \ Armor → | Plate | Mail | Padded | Cloth |
| ------------------ | :---: | :--: | :----: | :---: |
| **Pierce**         | ×½    | ×2   | ×1     | ×1    |
| **Slash**          | ×1    | ×½   | ×2     | ×1    |
| **Crush**          | ×2    | ×1   | ×½     | ×1    |

Mnemonic (re-derivable per `0-source-of-truth.md`): **pierce beats mail, slash beats
padding, crush beats plate**, each resisted by the next armor around — *mail catches
the cut, parts to the point; padding eats the blow, opens to the blade; plate turns
the point, rings to the hammer.* This is the regular 3-element counter cycle the
game-theory docs call "done right" at the **unit/tactical** level (a damage-type RPS is
correct here — it is *not* a faction-level cycle, see `nested-counter-systems.md`).

**Damage-type names:** Pierce / Slash / Crush. Aligns with the integration doc's
proposed `stats.rs` rename (**Blunt → Crush, Sharp → Slash**). Elemental damage
(Heat/Cold/Lightning) and Fear are **out of this chart** — elementals hit only their
own flat cut (integration doc §2); Fear is the inner channel (§2.2), never a weapon.

**Cloth = unarmored baseline:** ×1 to everything, off the wheel. Everyone starts Cloth;
Plate/Mail/Padded are the three real upgrades (integration doc §4).

---

## 2. How a hit resolves (per attacker action)

Reuses canon vocabulary (Strike = the outer attack stat, Toughness = Body·Power bar,
Health = Body·Quantity pool):

1. **Type chart (soft):** `bite = Strike × effectiveness(channel, armor)`. Integer
   rule: `×2 → 2·Strike`, `×1 → Strike`, `×½ → max(1, Strike/2)` (floored). Min-1 is
   what guarantees the chart adds **no immunity**.
2. **Toughness bar (accumulate within the round):** add `bite` to the active Health
   card's round pile; when the pile reaches **Toughness**, **flip one card**; overflow
   is discarded.
3. **End of round:** un-flipped accumulation is **wiped** (the hard reset). Damage that
   did not flip a card that round is wasted — this is the only wall source.
4. **Death:** all Health cards flipped.

Two kinds of waste give the depth: **overflow** (a hit past Toughness) and **sub-floor**
(a round's accumulation that never reaches Toughness). This matches §2.2's
"accumulate into the pile, compare to the bar," with the type chart inserted at step 1
in place of the subtractive cut.

---

## 3. Keywords — definitions and canon home

Four optional modifiers (combat-lab `Keywords`). Per the Spec's "cards may supersede
the core" rule, these belong as **keyword cards** (with a MANUAL line each), not core —
except where noted. Each needs a canon keyword entry or explicit retirement (integration
doc item 5).

- **`pierce`** (armor-piercing, attacker) — upgrades a resisted **×½ → ×1**. This is the
  redefinition of the old subtractive Precision/Pierce stat for the multiplicative model
  (it shaved cut; now it negates resistance). *Recommend: keep — it is the natural
  armor-pen verb.*
- **`cleave`** (attacker) — on a flip, overflow cascades into the next card instead of
  being discarded; one hit can flip `floor(bite/Toughness)` cards. *Anti-swarm.*
- **`persist`** (attacker) — round-end accumulation is **carried**, not wiped. Lets a
  low-bite attacker saw through high Toughness over several rounds. *Anti-tank.* (Does
  nothing against a ×½ that already can't reach the bar in the steady state — it only
  defeats the per-round reset.)
- **`brittle`** (defender) — **CONFLICTS with §2.2's "Armor never depletes" GUARANTEE.**
  In combat-lab, armor withstands `armor_quantity` strikes then **shatters to neutral
  (Cloth)**. *Recommend: not core — either retire, or make it an explicit card override
  that names the §2.2 rule it bends (integration doc item 4). The identity model defaults
  to permanent armor.*

---

## 4. Validation — why these numbers are trustworthy

The mechanical model is exercised by the `combat-lab` harness on a 15-archetype fixture
(3 tanks / 3 bruisers / 3 swarms / 3 skirmishers / 3 keyword specialists, all stats
capped at 9). Current scorecard:

- **15 / 15 viable** (non-dominated), **0 bosses / doormats / clones**, **one
  roundtable** (single SCC).
- **Copeland variance 3.60** (down from 53.86 under subtractive armor; 0 = perfectly
  regular), **Nash distance 0.274** from uniform.
- **Texture:** mostly soft edges, **4 walls**, all Toughness-floor (scaling-breachable),
  0 stalemates.

The detectors (Copeland / out-degree variance, immunity count, Nash distance, SCC,
dominance, clone, texture) are the named measures from `docs/game-theory/`. **Re-run
the harness after any merge that touches the chart or resolution** — it is the
regression instrument for this system and will be re-run again as gear folds into the
suit/reward layer.

---

## 5. Open items / hand-off

For the **merge** (human-gated):
1. **Resolve §0** — multiplier vs subtractive cut. Everything else assumes multiplier.
2. **`brittle`** vs §2.2 "never depletes" — retire or card-override.
3. **Keyword homes** — `pierce`/`cleave`/`persist` as keyword cards with MANUAL lines,
   or retire.

For the **gear-identity instance** (already flagged in its doc): cut-profile numbers
become **multiplier values** if §0 lands on multiplier (the three are fixed at ×2/×1/×½,
so there is *nothing per-armor to tune* — a feature: the chart is balanced by
construction, only Strike/Toughness/Speed magnitudes are dials). Elemental bypass and
the Damage Separation Law are unchanged by this doc.

Numbers (Strike/Toughness/Speed/Health magnitudes) are **booklet dials**, ≤ 9 in the
pre-god regime; the chart multipliers are **structural** and fixed.
