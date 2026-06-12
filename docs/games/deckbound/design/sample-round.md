# Deckbound — Sample Combat (4 vs 5)

A card-level play-by-play of one round on the **current** design: all four aspects
(Body / Mind / Magic / Spirit), the **gauntlet**, **fear vs resolve**, **reads +
momentum + combos**, **conditional decks**, and the **capability-card + rules-card**
model (plain tokens flip Form↔Dormant; a rules card holds the magnitude; damage
**accumulates within a round** as cards in a zone). Rebalanced toward the
**one-reviver margin** — a real fight that should *cost* a careful party, not a
curbstomp.

Zone shorthand: `Strike: P→A→D` (aggressive → exhausts); `Block: P→A→P` (defensive →
self-returns); `Firestorm: P→A→D` (Fleeting); `+Spd →A` (momentum banked); `Body ×2:
Form→D` (cards flipped down).

## The party — 2 front, 2 back, one lean per aspect

| Hero | Spd · Pow · Pre · (Mag/Spr) · **Resolve** | Body×T | Kit & role |
| --- | --- | --- | --- | --- |
| **Aldric** (Knight) | 3 · 4 · 2 · — · **R4** | 8 × T2 | Plate (−3 phys, **−0 heat**), Shield (Block, **Bash**=blunt). **Holds**; **Resolute** (fearless). *Body wall.* |
| **Vera** (Duelist) | 5 · 3 · 4 · — · R2 | 4 × T1 | Blade (sharp). **Riposte** (combo: Evade → counter → reposition). *Mind reads & momentum.* |
| **Sefa** (Mage) | 2 · 1 · 3 · **Mag 5** · **R1** | 3 × T1 | **Firestorm** (heat AoE), **Frostbite** (cold slow). Glass cannon; **fearful**. *Magic.* |
| **Bram** (Warden) | 3 · 2 · 3 · **Spr 5** · **R4** | 5 × T2 | **Rally**, **Dread**, **Recover**; **passive revive**. *Spirit / will — see [spirit](spirit.md).* |

Resolve is a stat like any other (plain cards + a rules card; fear accumulates within a
round). Everyone also holds the four reads in Potential. **Synergy:** Aldric walls and
ignores fear; Bram steadies and can banish the Wraith *and* revives the fallen; Sefa
nukes once safe; Vera grinds the Ogre.

## The warband — 5 creatures, conditional decks

| Creature | Spd · Pow · (special) | Body×T | Behavior (drawn in **bold**) |
| --- | --- | --- | --- |
| **Ogre** | 2 · 6 · Armor −3 (blunt −1; heat −0) | 8 × T3 | **Press the front** · wounded → **Smash** |
| **Wraith** | 4 · — · **Fear 5**, *incorporeal* (Body & Magic do nothing; only **Spirit/Dread** strips its **Presence 3**) | — | **Haunt the least-resolute** · target fearless → recoil |
| **Stalker** | 6 · 3 · — | **6 × T1** | **Dive the lowest-Body** · alone → flee |
| **Imp ×2** | 4 · 1 · — | 1 × T1 | **Outnumbering → dive** · else press front |

## Form up & declare

- **Players** — Front: Aldric, Vera. Back: Sefa, Bram.
- **Creatures** — Front: Ogre. Diving: Stalker + 2 Imps. The **Wraith**, incorporeal,
  drifts past the wall to **haunt Sefa**.
- A reasonable plan: **Aldric Holds**, **Vera Attacks the Ogre**, **Bram Rallies
  Sefa**, **Sefa casts Firestorm**. (Note: only *one* Holder covers the gauntlet.)

## Resolve in Speed order

**Stalker (Spd 6) — dive → gauntlet.** Runs past Aldric (lone Holder, wall speed = his
Spd 3). 3 < 6, so it **slips the block**; Aldric's **free strike auto-lands** anyway
(`Bash: Pow 4 → Body ×4: Form→D`). But the Stalker is **Body 6** now — it survives at
**2**, **bloodied but through**, and reaches Sefa. *One Holder is a thin gauntlet: fast
**and** tough enough, a diver breaks it.*

**Stalker → Sefa.** Sefa declared **Attack** (casting), not Hold — so she is **not
engaging** the Stalker, and its strike **auto-succeeds**: `Pow 3 → Body ×3: Form→D` →
**Sefa knocked out** before she can cast. Firestorm never goes off. **Bram's passive
revive** catches her — she's back up, but her turn is **spent on the floor**.

**Imps (Spd 4) — gauntlet.** Both run past Aldric; free strikes (Pow 4 ≫ Body 1) →
**both dead**. The wall handles the *weak* divers cleanly.

**Wraith (Spd 4) — haunt Sefa.** Its prey is already down — nothing to frighten — so it
**recoils**, Presence intact. (Bram's Rally, aimed at the Wraith's fear, was the *wrong
threat*: it was a **blade**, not dread, that dropped Sefa.)

**Vera (Spd 5) — read the Ogre.** Ogre telegraphs **Press**; Vera **Ripostes** (`Evade:
P→A→P`, **+Spd →A**) — negates the swing, takes position, but sharp 3 − armor 3 = **0**,
no wound. She's **banking momentum** toward a blow big enough to crack the armor.

**Aldric (Spd 3) — Hold.** Free strikes already landed; he braces, untouched.

**Bram (Spd 3) — Rally** (`P→A→D`) — resolved early, and his **passive revive** already
caught Sefa. A turn spent without stopping the real killer.

**Sefa (Spd 2)** — down; no action.

**Ogre (Spd 2)** — its Press at Vera was **Evaded**; wasted.

## End of round

| | Outcome |
| --- | --- |
| **Heroes** | **Sefa dropped and revived** (no Firestorm; she contributed nothing). Others unhurt; Vera banked **+Spd**. **No permanent loss — the one-reviver margin held.** |
| **Creatures** | **2 of 5 down** (the Imps). **Stalker** survives at Body 2 in the back line (next round, alone → it flees). **Wraith** recoiled, Presence 3. **Ogre** untouched at 8 (no Firestorm landed; Vera's sharp can't bite it). |

This is what **balanced** feels like: a reasonable plan, and the warband still **made
them bleed** — a hero down, the Mage's whole turn lost, two real threats (Ogre, Wraith)
standing. They held only because Aldric walled the imps and **Bram's revive caught
Sefa**. The lesson is legible: against a **tough, fast diver**, one Holder is not
enough — had **Vera also Held**, the two-body gauntlet (wall speed 3 **+1** = 4, and a
second free strike) would have **killed the Stalker on the run** and freed Sefa to
cast. The fix is coverage; the cost of greed is a hero on the floor.

**Careless** (no Holders at all) is a flat wipe: Stalker + Imps gut the back line, the
Wraith scares whoever's left, and nobody is up to revive.

## What it exercises — and what's now settled

In play: all four aspects, the **gauntlet** (coverage as *speed pooled by bodies* — a
lone Holder couldn't keep up), **fear vs resolve** (the Wraith bypassing the wall),
**momentum** vs armor, the **capability/rules-card** model, **stance vs read** (Sefa's
Attack left her undefended), and **revive as the margin**.

- **Resolve** — *settled:* a stat (cards + rules card), **fear accumulates within a
  round, resets between**; overcome it and your own panic harms you. See
  [spirit](spirit.md).
- **Incorporeal** — *settled:* drifts past the gauntlet, stopped only by resolve; only
  **Dread** harms it ([form & defeat](form-and-defeat.md#incorporeal--no-body-no-shelter-for-the-soul)).

Still open (numbers / grammar):

- **Rally / Dread magnitudes** and Rally's **compounding** curve (see [spirit](spirit.md)).
- **Combo grammar** — how a card like *Riposte* bundles Evade + counter + reposition
  (see [combos](combos.md)).
- The exact **scaling** so a balanced fight reliably lands on the one-reviver margin.
