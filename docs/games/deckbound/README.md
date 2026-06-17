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
