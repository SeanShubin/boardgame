# Melee character catalog (v1 fixture)

The roster the combat engine pits against itself. These are **reference-level**
specialists — the baseline shape of each archetype; vertical scaling multiplies a
build's signature stats from here (see `combat-engine-design.md` for the model,
resolution rules, scaling, and the report spec). No budget: builds are **not**
cost-equalized — balance is measured by the engine, not assumed here.

The "beats / loses / weakness" notes are **design intent**, to be confirmed (and
the junk found) by the engine's report, not asserted here.

## Sheet legend

Every line is a `(stat) (value)` pair in the `quantity / magnitude` model:

- `health-quantity` / `health-magnitude` — # body cards / Toughness (damage a card
  accumulates within a round before it flips).
- `armor-pierce` / `armor-slash` / `armor-crush` — flat cut subtracted from each
  incoming strike of that type (the hard wall). A low value = a weak channel.
- `speed-quantity` / `speed-magnitude` — actions (Strikes) per round / initiative.
- `strike-magnitude` / `strike-channel` — power per hit / its damage-type. A second
  weapon adds `strike2-*`.
- `pierce-magnitude` — armor shaved off the target.
- keywords: `persist` (accumulation carries across rounds) · `cleave` (overflow
  cascades to the next card) · `brittle` (armor depletes; needs `armor-quantity`).

---

## Walls — survive by denying channels

```
Bulwark — Plate tank, sword
health-quantity 6
health-magnitude 3
armor-pierce 3
armor-slash 3
armor-crush 1
speed-quantity 2
speed-magnitude 2
strike-magnitude 4
strike-channel slash
```
Armor 3/3/1 · Tough 3 · 6 cards · 2× slash-4 · Beats swarms & slashers (walls
them) · Loses to crush · Weakness: **crush**, low output.

```
Aegis — Plate control, spear (first-strike)
health-quantity 5
health-magnitude 3
armor-pierce 3
armor-slash 3
armor-crush 1
speed-quantity 2
speed-magnitude 5
strike-magnitude 4
strike-channel pierce
```
Armor 3/3/1 · Tough 3 · 5 cards · 2× pierce-4, acts first · Like Bulwark but
seizes initiative and pierces back · Weakness: **crush**.

```
Sentinel — Plate turtle, brittle
health-quantity 6
health-magnitude 4
armor-pierce 3
armor-slash 3
armor-crush 1
armor-quantity 3
speed-quantity 1
speed-magnitude 1
strike-magnitude 3
strike-channel slash
brittle
```
Armor 3/3/1 (×3, decaying) · Tough 4 · 6 cards · 1× slash-3 · Outlasts almost
anything early; many **draws**; folds once armor erodes · Weakness: attrition,
crush, time.

## Bruisers — few huge hits that crack armor

```
Maul — Mail bruiser, warhammer
health-quantity 4
health-magnitude 3
armor-pierce 1
armor-slash 3
armor-crush 3
speed-quantity 1
speed-magnitude 2
strike-magnitude 7
strike-channel crush
```
Armor 1/3/3 · Tough 3 · 4 cards · 1× crush-7 · One blow cracks any armor; **beats
the Plate walls** · Weakness: **pierce**, one action a round.

```
Avalanche — Padded heavy, maul
health-quantity 4
health-magnitude 3
armor-pierce 3
armor-slash 1
armor-crush 3
speed-quantity 1
speed-magnitude 1
strike-magnitude 8
strike-channel crush
```
Armor 3/1/3 · Tough 3 · 4 cards · 1× crush-8 · Demolishes tanks; slowest thing on
the field · Weakness: **slash**, extreme slowness.

```
Ironwood — Padded durable, mace
health-quantity 6
health-magnitude 3
armor-pierce 3
armor-slash 1
armor-crush 3
speed-quantity 2
speed-magnitude 1
strike-magnitude 5
strike-channel crush
```
Armor 3/1/3 · Tough 3 · 6 cards · 2× crush-5 · A bruiser that doesn't die fast;
crush still cracks plate · Weakness: **slash**.

## Swarms — many fast weak hits

```
Hailstorm — Padded swarm, knives
health-quantity 8
health-magnitude 1
armor-pierce 2
armor-slash 0
armor-crush 2
speed-quantity 5
speed-magnitude 3
strike-magnitude 2
strike-channel slash
```
Armor 2/0/2 · Tough 1 · 8 cards · 5× slash-2 · Buries anything with low slash-armor
in flips · Hard-walled by slash-armor ≥ 2 · Weakness: armor it can't bite; dies to
one cleave.

```
Gnat — Mail swarm, awls
health-quantity 7
health-magnitude 1
armor-pierce 0
armor-slash 2
armor-crush 2
speed-quantity 5
speed-magnitude 4
strike-magnitude 2
strike-channel pierce
```
Armor 0/2/2 · Tough 1 · 7 cards · 5× pierce-2 · Fast pierce eats mail and the
crush bruisers · Walled by plate pierce-armor · Weakness: plate, cleave.

## Skirmishers — precise / first-strike

```
Pike — Mail skirmisher, pike (first-strike)
health-quantity 4
health-magnitude 2
armor-pierce 0
armor-slash 2
armor-crush 2
speed-quantity 3
speed-magnitude 5
strike-magnitude 3
strike-channel pierce
```
Armor 0/2/2 · Tough 2 · 4 cards · 3× pierce-3, acts first · Pierce cracks mail and
bruisers from initiative · Weakness: **plate** pierce-armor, crush.

```
Saber — light Plate, fast sword
health-quantity 4
health-magnitude 2
armor-pierce 3
armor-slash 3
armor-crush 1
speed-quantity 4
speed-magnitude 3
strike-magnitude 3
strike-channel slash
```
Armor 3/3/1 · Tough 2 · 4 cards · 4× slash-3 · Fast slash carves padded & swarms
behind a real front · Weakness: **crush**, other plate.

```
Stiletto — glass assassin, precision
health-quantity 3
health-magnitude 1
speed-quantity 4
speed-magnitude 6
strike-magnitude 2
strike-channel pierce
pierce-magnitude 6
```
No armor · Tough 1 · 3 cards · 4× pierce-2, pen-6, acts first · Pierce ignores
nearly all armor and strikes first — can delete a tank before it acts · Any single
connecting hit kills it · Weakness: **fragility**, the initiative race.

## Specialists — keyword-driven

```
Halberd — Mail versatile, two weapons
health-quantity 4
health-magnitude 2
armor-pierce 0
armor-slash 2
armor-crush 2
speed-quantity 2
speed-magnitude 3
strike-magnitude 3
strike-channel slash
strike2-magnitude 4
strike2-channel crush
```
Armor 0/2/2 · Tough 2 · 4 cards · slash-3 / crush-4 (pick per round) · Few hard
walls — slash vs padded, crush vs plate · Weakness: **pierce**, mediocre margins.

```
Reaver — Mail grinder, serrated
health-quantity 4
health-magnitude 2
armor-pierce 0
armor-slash 2
armor-crush 2
speed-quantity 3
speed-magnitude 3
strike-magnitude 2
strike-channel slash
persist
```
Armor 0/2/2 · Tough 2 · 4 cards · 3× slash-2, `persist` · Weak slashes that would
reset instead saw through across rounds — **anti-tank** · Weakness: **pierce**,
anything that kills before the grind pays off.

```
Cleaver — Padded anti-swarm, cleave
health-quantity 5
health-magnitude 2
armor-pierce 2
armor-slash 0
armor-crush 2
speed-quantity 2
speed-magnitude 2
strike-magnitude 4
strike-channel slash
cleave
```
Armor 2/0/2 · Tough 2 · 5 cards · 2× slash-4, `cleave` · One slash overflows across
a low-Toughness swarm's cards — **hard-counters Hailstorm/Gnat** · Weakness:
**slash**-armor, slow vs tanks.

---

## How this roster should behave (to be verified)

- A **type web**: crush > plate > {swarms} > mail/padded > crush.
- Walls beat swarms, lose to bruisers; bruisers beat walls, lose to swarms/
  skirmishers; specialists (cleave/persist/precision) are built as targeted
  counters.
- Every sheet carries a deliberate weakness — "you must have a weakness."

Things to watch once the engine runs:
- **Stiletto** — boss (pen + first strike) or doormat (one-hit death)? A knife-edge.
- **Sentinel** — likely draw-mush (a hard-reset turtle stalls the board).
- **Halberd / Reaver / Pike** — all Mail, similar lines → **clone** risk.

Rules, scaling, detectors, and the report directory: `combat-engine-design.md`.
