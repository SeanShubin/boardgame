# Deckbound

A cooperative, simulation-style, hidden-information fantasy card game made
entirely of cards. **Early design — not yet playable.**

The documentation is split by **authority**:

- **`canon/` — the binding sources of truth.** If it's in here, it governs.
  - **[0-source-of-truth.md](canon/0-source-of-truth.md) — START HERE.** Where truth
    lives, who wins on a contradiction, and **how AI assistants are expected to behave**.
    Read it before changing anything.
  - [1-charter.md](canon/1-charter.md) — the intent (the ten north stars).
  - [2-spec/](canon/2-spec/README.md) — the mechanics (RULE / WHY / GUARANTEES).
  - [3-booklet.md](canon/3-booklet.md) — pointer to the print master (`booklet.ron`),
    which lives in the crate because the game loads it at build time.
- **`notes/` — frozen design exploration.** Non-authoritative, often stale; kept as
  history. Start with the [index](notes/README.md). Superseded by `canon/` where they
  disagree.
- **`presentation/` — player-facing material.** The [rules placeholder](presentation/rules/README.md)
  and an [interactive tutorial](presentation/tutorial.html).
- **[future-possibilities.md](future-possibilities.md) — design backlog.** A living,
  non-authoritative parking lot for big speculative changes awaiting playtest before any
  decision. Neither canon nor frozen notes.
- **[retired-ideas.md](retired-ideas.md) — the design graveyard.** The counterpart to the backlog:
  ideas that were **deliberately decommissioned**, each with *why* it was retired and **the bar it must
  clear to come back** — so a revived idea has to answer the objections that sank it.
- **[scenario-plan.md](scenario-plan.md) — combat content blueprint.** The authored target for
  the rebuilt combat: cast, tutorials, and scenarios that realize §1/§3/§4.
- **[game-flow.md](game-flow.md) — the complete game-flow map.** Every cycle and phase, largest to
  smallest (**Run ⊃ Day ⊃ Encounter ⊃ Round ⊃ Phase**, plus the **Clash ⊃ Beat** RPS), each pointing
  to its authoritative source. Start here to see how the whole game is structured.
- **[combat-round-sequence.md](combat-round-sequence.md) — the SHIPPED combat model's canon.**
  The eight-step round (Havoc / Withdraw / Skirmish / Cross / Defensive Volley / Raid / Assault /
  Advance) and the four-minor-step Interaction primitive (Target → Bid → Strike → Resolve), as
  implemented in `crates/rules/src/combat` and asserted by the diagonal balance gate. Documents
  the playable reference combat; it does not amend the aspirational `canon/2-spec`.
- **[computability-and-balance.md](computability-and-balance.md) — the determinism discipline.**
  Elaborates **Charter #11**: why the canonical mode (Clash off, decks open) is deterministic and
  single-agent so **par is computable**, the invariants future mechanics must not break (with a
  review checklist), and how that computability is used to balance the game objectively. Read it
  before adding anything that touches randomness, foe behaviour, carried state, build growth, or
  the day clock.
- **[balance-invariants.md](balance-invariants.md) — the balance-target registry.** A living list of
  the checkable balance properties the tuned numbers must satisfy (e.g. *a one-of-each-role party
  beats every same-size single-role party*), each an instance of "interesting beats boring." Verified
  by the harness / par solver; a satisfied invariant is a regression guard.
- **[intended-player-experience.md](intended-player-experience.md) — the experiential north star.** A
  non-authoritative synthesis that reads the *felt experience* off the Charter, the Spec, and the
  balance instruments: the gentle on-ramp, the feel-the-lack-then-the-fix loop, layered depth, the
  no-dead-disciplines finale, and the stats-survive / powers-decide split — each promise mapped to the
  invariant that enforces it.
- **[tcg-comparison-timing-and-variety.md](tcg-comparison-timing-and-variety.md) — the cross-game design
  study.** A non-authoritative comparison of Deckbound against Grand Archive and Magic: The Gathering.
  The organizing lens (its §7): the three games spend one finite **complexity / computability budget**
  differently — MTG buys an open stack and pays in non-computability; Deckbound *reallocates* that budget
  to battle structure and geography while *saving* on the interaction engine (no stack) to stay computable;
  Grand Archive sits between. Two proposed north stars (N2, N5) were reviewed and **folded back into
  existing Charter #2/#11** rather than added — "no stack" is a provisional, computability-motivated
  constraint, not a permanent identity.
- **Design in progress (non-canonical, on the spec-first path):** the strategic/character layer
  being worked out ahead of graduating onto the Spec —
  [progression-design.md](progression-design.md) (geography, currency, encounters, the day/clock,
  clean-slate deck-as-stats characters), [zones-exhaustion-design.md](zones-exhaustion-design.md)
  (the card zone/exhaustion machinery + resources, i.e. Spec §5), and
  [reference-scenario.md](reference-scenario.md) (a full-game balance harness, maintained as a test).
- **[name-bank.md](name-bank.md) — authored flavor names.** A curated, living pool of
  setting names (locations, adventurers, …) to draw on when authoring decks, characters,
  and scenarios. Non-canonical until a card actually uses a name.
- **[roadmap.md](roadmap.md) — undefined mechanics & planned features.** A living to-do
  list of what doesn't exist yet (geography, travel, loot, progression, world events,
  run-level victory/defeat; plus features like a human-emulating AI, an in-game
  encyclopedia, card-interaction lists, and print-export).
