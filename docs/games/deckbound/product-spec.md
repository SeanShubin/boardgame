# Deckbound card-table product — the shipped instance (Spec tier 2)

> **Tier 2: the product's narrower spec.** This documents the concrete **card-table product** — the specific
> instantiation of the mechanics in [`canon/2-spec`](canon/2-spec/README.md). It must stay **compatible** with
> the Spec (tier 1); it only **narrows** it (picks the map, the roster, the encounters, the cell capacities).
> The exact numbers are **not duplicated here** — they live in
> `crates/deckbound-content/src/catalog.rs` (the print master, in code), and this doc points at them so the
> two cannot drift.

## The map — a 3×3 grid

Row-major; the party of four (one per kit) starts at the **centre** (index 4). The four **orthogonal**
neighbours are **solo cells** (one hero each); the four **diagonal corners** are **party fights**; the centre
is the **capstone**. Layout (`fixtures::LOCATIONS`, ordered so Ashfen Crossing falls at centre):

```
The Hollow Rampart   Cinderwatch Keep    Greywater Ford
The Sundered Vault   Ashfen Crossing     Thornmarch Gate
Emberfall Hollow     The Salt Barrows    Ninefold Deep
```

- **Solos** (orthogonal): Cinderwatch Keep, The Sundered Vault, Thornmarch Gate, The Salt Barrows — each
  soloable by exactly one kit (its counter).
- **Party corners** (diagonal): The Hollow Rampart, Greywater Ford, Emberfall Hollow, Ninefold Deep.
- **Capstone** (centre): Ashfen Crossing — everything at once.

## The roster and encounters — read the catalog

The four kits (`catalog::ROSTER`) and the nine encounters with their foe rosters (`catalog::ENCOUNTERS`,
`catalog::encounter_foes`) are the source of truth for names and numbers; this doc does not restate them.
`cargo run -q -p deckbound-board --example regions_diagonal` prints the current balance shape (4 solos + 5
party fights), which the diagonal gate (`cargo test -p deckbound-board --test diagonal`) asserts.

## Cell capacity — one source, not authored twice

Which cells are capacity-1 is **derived**: a cell is a **solo** iff its encounter is a non-party fight
(`Encounter::party == false`), read by `board_game::is_solo_cell`. So the tier-1 mechanic (a cell has a
hero-capacity; overflow swaps) and this tier-2 assignment share the one `party` flag — a cell can never
disagree with the encounter it holds.

## What this product does NOT implement (yet)

The Spec's richer strategic design — the 25-card Suit grid, reward tracks, fog, progression (§8.1–§8.6) — is
**not** in the card-table product. It is future direction ([`future-possibilities.md`](future-possibilities.md)).
The product ships the simple map loop in Spec §8.0.
