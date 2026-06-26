# Powers redesign — abilities on the Form, utility cards as bookkeeping

> **Handoff doc.** Captured from a combat-design conversation; staged in `needs-merge/` for
> another instance to fold into the canon powers/Form design. It is **input**, not canon.
> Related: Spec §4.6 (the six phases; the **Reckoning** is the last damage phase of a round),
> `log-driven/combat-logs/card-combat-all-mechanics.md` (bare mechanics worked on Form
> abilities), `computability-and-balance.md` §6.1 (necessity test), and the
> emergent-not-fiat removal test.

## The settled architecture

- **All abilities live on the Form** (the character's permanent build) as **passive
  enablers** — having an ability means you *can* use it, repeatably, any time, gated only by
  its **Tempo** cost. Abilities are **open information** and are never drawn, so there is no
  card-draw RNG over your own capabilities (preserves the deterministic, perfect-information
  PvE core).
- **Tempo is the only action currency** (Cadence × Finesse, a fixed per-round pool). The
  **blind bid** remains the sole hidden commit. "Form open, bid hidden."
- **Utility cards are bookkeeping tokens**, *not* a second action resource and *not* a draw.
  They physically track the **persistent state** a complex power imposes (a mark on a target,
  poison stacks, a zone). Their **count is form-derived** (set by the ability's level), so
  they carry no hidden information and no RNG.
- **A "power" and an "attack" are the same object** — a card with
  `(reach, timing, target, cost, effect)`. A power is just an attack with richer trait values
  (area, deferred, a buff or a lingering effect instead of plain Might). This unifies the
  catalog: one schema, attacks and powers a spectrum of trait richness.

### The trait schema

```
reach   : melee (front / charge)  |  ranged (any lane)
timing  : instant (resolves in the phase it's used)  |  deferred (held; resolves at the Reckoning)
target  : single  |  area (every body in the target group / rank)
cost    : N Tempo  |  0 + one-shot (flips face down for the combat)
effect  : Might damage  |  buff (e.g. temporary Tempo)  |  persistent effect (needs a utility token)
```

Instantaneous abilities need **no** utility token (nothing lingers). Persistent abilities
mint a token to track the ongoing state. **Lasting effects are defined by the ability card
itself** — including the effect's **lifetime** (whole-combat, like Health, vs per-round, like
the pile/Tempo).

## Worked example 1 — Hunter's Mark (a persistent debuff)

- On your **Form**: `Hunter's Mark`. In your supply: one or more **mark** tokens (count =
  ability level).
- **Use:** spend Tempo → make a **ranged attack** to *land* the mark on a target.
- **Effect (persistent):** while marked, the target has **−2 Finesse, minimum 1**, until [the
  ability states — e.g. end of combat].
- **Scaling:** higher levels grant **more mark tokens** → more simultaneous marked targets.

Two anti-fiat properties to preserve:

- **Landing the mark is itself an attack** — you must *connect* through the normal contest;
  the target may avoid the marking shot like any ranged attack. The debuff is earned, not
  decreed.
- **`min 1` floor** — the same clamp as the ≤3 resistance cap and "every position dies to
  enough": a debuff can grind a target down but never **zero it out**, so no stack of marks
  produces a hard lock. **Force-not-fiat, enforced by a number.**

## Worked example 2 — Poison (a damage-over-time)

- **Use:** apply via the normal contest, then **place 3 poison markers** on the target
  (utility tokens).
- **Tick:** at **the last damage phase of each round — the Reckoning (§4.6)** — **remove 1
  marker and deal 3 damage** to the target.
- So the tokens are both the **state** (how much poison is left) and the **clock** (it ends
  when the markers run out). The Reckoning is the natural end-of-round tick for any
  lasting/periodic effect.

## Worked example 3 — Rallying Cry (a one-shot pre-combat buff)

- On your **Form**: `Rallying Cry`. **Cost: 0 Tempo, one-shot** — when used it **flips face
  down for the whole combat** and **never resets**.
- **Use:** in the **Standoff** (the Standing window); it **auto-lands** (a buff — no contest).
- **Effect (buff):** each ally gains **+1 temporary Tempo this round**. Temporary Tempo
  **expires at the Lull** (it does not refresh).
- **No utility token** — the effect is instantaneous; the **face-down flip is its own
  bookkeeping** (a one-shot tracker on the ability itself, not a token on the board).

Three patterns worth lifting from this one:

- **One-shot via flip-face-down-for-combat** is a clean, deterministic limiter that
  **generalizes to any once-per-combat ability** — no charges, no timers, just "used → down,
  doesn't reset."
- **Temporary Tempo is the minimal buff** — it grants *more of an existing resource* rather
  than a new stat-modifier, so it adds no new accounting beyond "expires at round end."
- **It is the stand-in for testing the Standoff phase** — a pre-combat effect that pays off in
  a *later* phase, and it can be **load-bearing**: in the worked log a C3 unit uses its
  temporary Tempo as the very action that lands a disrupting Breach blow (without the buff it
  runs dry and the deferred caster survives).

## Open questions for the redesign (flagging, not deciding)

- **Effect lifetime convention** — should persistent effects default to whole-combat or
  per-round, with the card overriding? (Health persists; pile/Tempo reset — pick the default.)
- **Token cleanup** — do markers return to supply when the target dies / combat ends? Does a
  marked target dying free the mark for re-use this combat (interacts with scaling)?
- **Defending the application** — confirm the marking/poisoning attack is dodge/avoidable like
  any attack of its reach, and what a group does (weakest-link vs spill) when an area
  persistent effect lands on it.
- **DoT timing vs disrupt** — poison ticks at the Reckoning; is an *applied* DoT independent
  of its caster (keeps ticking if the caster dies), unlike a *held* deferred attack (dropped
  if the caster dies before release)? Likely yes — once applied, the token is on the target,
  not the caster — but state it.
- **Necessity test** — per §6.1, each new power should come with a scenario it is *required*
  to win; a power with no such scenario is fiat or redundant.
