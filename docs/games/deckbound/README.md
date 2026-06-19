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
- **[scenario-plan.md](scenario-plan.md) — combat content blueprint.** The authored target for
  the rebuilt combat: cast, tutorials, and scenarios that realize §1/§3/§4.
- **[game-flow.md](game-flow.md) — the complete game-flow map.** Every cycle and phase, largest to
  smallest (**Run ⊃ Day ⊃ Encounter ⊃ Round ⊃ Phase**, plus the **Clash ⊃ Beat** RPS), each pointing
  to its authoritative source. Start here to see how the whole game is structured.
- **[computability-and-balance.md](computability-and-balance.md) — the determinism discipline.**
  Elaborates **Charter #11**: why the canonical mode (Clash off, decks open) is deterministic and
  single-agent so **par is computable**, the invariants future mechanics must not break (with a
  review checklist), and how that computability is used to balance the game objectively. Read it
  before adding anything that touches randomness, foe behaviour, carried state, build growth, or
  the day clock.
- **Design in progress (non-canonical, on the spec-first path):** the strategic/character layer
  being worked out ahead of graduating onto the Spec —
  [progression-design.md](progression-design.md) (geography, currency, encounters, the day/clock,
  clean-slate deck-as-stats characters), [zones-exhaustion-design.md](zones-exhaustion-design.md)
  (the card zone/exhaustion machinery + resources, i.e. Spec §5), and
  [reference-scenario.md](reference-scenario.md) (a full-game balance harness, maintained as a test).
- **[roadmap.md](roadmap.md) — undefined mechanics & planned features.** A living to-do
  list of what doesn't exist yet (geography, travel, loot, progression, world events,
  run-level victory/defeat; plus features like a human-emulating AI, an in-game
  encyclopedia, card-interaction lists, and print-export).
