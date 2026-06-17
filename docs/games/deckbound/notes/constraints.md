# Deckbound — Constraints

Hard constraints specific to **Deckbound**. These are deliberate limits that
shape every other design decision; treat them as inviolable unless explicitly
revisited. (Other games in this framework may choose different constraints —
these are Deckbound's.)

## C1 — Fully playable without a computer

The game must be completely playable by hand, e.g. if the cards were physically
printed. A computer may *assist* (a digital implementation is the goal), but it
must never be *required*. Consequences:

- **All randomness is physical** — it comes from shuffling, never from a hidden
  computation.
- **The game *system* needs no computer.** Environment creatures, hazards,
  scenarios, and rules adjudication all run by hand: reshuffle a deck, draw, and
  apply a card. No per-turn math a human couldn't do quickly.
- **Human roles are played by humans** — who may reason and apply game theory in
  their heads. A computer is an *optional* stand-in for a human *player*, never
  for the game system; when present it may compute game theory directly, which is
  legitimate because it only does what a human in that seat could do
  ([C3](#c3--every-agent-is-bound-by-the-same-rules)). Without a computer, human
  roles are filled by humans, or are simply absent — the default co-op scenario
  pits players against environment decks. See
  [decision-making](decision-making.md).

## C2 — Cards only

The game is made of **cards and nothing else** — no board, no dice, no separate
tokens. This still leaves real flexibility, because **cards can represent
resources**:

- Tracks (health, power, tempo) are rows or stacks of cards.
- Counters and markers are cards.
- Randomness is **shuffling**, not dice.

If something needs to be represented, it is represented as cards.

## C3 — Every agent is bound by the same rules

The computer stand-in represents a human and must be **bound by the same rules
as a human**. It has the same legal moves and the same resource limits; it
cannot see hidden information a human could not, and it cannot play a card a
human in its position could not. The deck mechanism enforces this naturally: an
opponent deck contains only legal plays, so it can never cheat. The environment
is likewise constrained to what the fiction allows.

## C4 — Hidden, simultaneous choice must be physical

The core mechanic is hidden information resolved by simultaneous reveal. This has
to be achievable with cards alone: each side **commits a face-down card**, then
both **reveal at once**. No agent may react to another's choice after seeing it
within the same exchange.

## Implications

- Anything that would need a computer to adjudicate must be redesigned into a
  lookup, a deck, or a card.
- **Environment-creature** behavior and difficulty are expressed through **deck
  composition**, not algorithms — a harder creature is a differently-built deck.
  Human-level opponents, by contrast, reason directly (a computer or a person).
- Solo and co-op play are first-class: players can take on the world using only
  shuffled environment decks — no computer and no human opponent required.
