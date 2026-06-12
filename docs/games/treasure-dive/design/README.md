# Treasure Dive — Design Notes

## Purpose

Treasure Dive exists to exercise the framework end to end with the smallest game
that is still a real game: a shuffled deck, turns, per-player score, a clear
win condition, and exactly the kind of one-or-two-button decision the `tabletop`
renderer is built to present. It is the reference other games are modelled on.

## Target experience

A quick, tense push-your-luck filler — decisions every few seconds, a game in a
couple of minutes. The fun is the rising dread of one more dive.

## The central tension

Every successful dive both **helps and hurts**: it adds value to bank, but it
also adds a suit, which raises the chance the next flip busts. That single
coupling is the whole game — the player is always weighing a known gain against
a growing, quantifiable risk.

## The numbers

- **Six suits × values 1–6 = 36 cards.** Six suits make early dives fairly safe
  and late dives clearly dangerous; the risk curve is steep enough to feel but
  not punishing from the first flip.
- **Values 1–6** keep banked scores in a readable range and make the
  bank-now-or-push decision a real arithmetic trade-off rather than a coin flip.
- **Bust loses the whole pile.** An all-or-nothing penalty is what gives
  surfacing its weight; a softer penalty would flatten the decision.

## Open questions / variants

- A small bonus for collecting many distinct suits in one dive, to reward brave
  dives without removing the bust risk.
- Per-suit special abilities (à la Dead Man's Draw) — a natural next step once
  the framework supports richer card effects.
- Tuning suit count and value range for different player counts.
