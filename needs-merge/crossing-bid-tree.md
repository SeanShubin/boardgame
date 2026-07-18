# The crossing as a bid tree

Status: design spec, agreed in conversation 2026-07-17. Not yet implemented.
Supersedes the flat `Answer` trichotomy (Evade / Push / Abort) in
`crates/rules/src/combat/regions.rs`.

## Why

The starting goal was narrow: make the abort riposte a real choice (today it is
the round-robin auto-default from commit 535a38a). Pulling on that thread turned
out to re-open the whole crossing contest. The three answers a slipper can give
today are a flat, declared-up-front enum. They should instead **fall out as the
leaves of a two-spend bid tree**, so the riposte allocation is not a bolt-on but
one of the bids.

## The internal logic (one axis)

The three classic answers are just *when you spend tempo relative to the enemy's
strikes*:

- **before** them, to prevent  -> Evade
- **after** them, to retaliate  -> Abort (and you are repelled: you stopped to fight)
- **never**                     -> Push (eat it, cross anyway if you live)

Same per-body tempo pool, three timings. You never *declare* an answer. You
declare a raid, then spend (or decline to spend) tempo at two points; the label
is derived from where the tempo went.

## The tree

```
Raid declared                                        [beat 1, raider]
  |
  catchers chosen + assigned to raiders              [beat 2, foe: ELECTIVE]
  |
  evade bid   vs   catch pool                         [beat 3, simultaneous, hidden]
  |
  +-- evade strictly beats the pool ................. EVADE   (through, untouched)
  |
  +-- caught -> each catcher lands its 1 free strike
         |
         extra-strike allocation                      [beat 4, simultaneous, hidden]
         |
         +-- raider spends 0 .......................... PUSH  (cross to outrider if alive)
         |
         +-- raider spends >0 (across the catchers) ... ABORT (repelled; stay home)
```

Both simultaneous beats are genuine hidden bids (commit, then reveal), so the
crossing carries a real mind-game. This deliberately re-opens what the current
fixed `slip_cost` closed off ("underpaying is never a gamble, only a waste"): a
hidden pool means you can now overpay (wasteful) or underpay (caught AND
depleted).

## The rules, precisely

### Beat 1 -- raid (raider)
Unchanged from today: the raider declares which of its vanguard cross.

### Beat 2 -- catch (foe, elective)
NEW: catching is a foe *choice*, not automatic geometry. The enemy picks which
of its vanguard catch, and which raider each targets. Multiple catchers may gang
one raider; another raider may go uncaught. The choice is driven by the
creature's **behavior card** (see "Foe behavior" below) -- the one deterministic,
legible policy that governs every foe beat.

### Beat 3 -- evade vs catch (simultaneous, hidden)
- Both sides bid tempo, weighted by **Finesse**, but the two sides weight
  differently by body-count:
  - **Catch strength = `tempo cards x Finesse x bodies`.** A mob is many hands
    pinning you.
  - **Evade strength = `tempo cards x Finesse`** -- no body-count. A horde
    evades no better for being many, because **only one of the horde has to be
    caught** for the whole horde to be caught.
- The upshot: **a horde is a great catcher and a lousy evader** -- the asymmetry
  is entirely emergent from where the body-count multiplier appears, not a
  per-creature switch.
- The catchers on one raider are **cumulative**: the raider faces the *pool*
  (sum of each catcher's catch strength).
- The raider evades cleanly iff its evade strength **strictly beats** the pool.
  All-or-nothing: beat the pool and you slip every catcher; fall short and every
  catcher caught you.
- Spent is spent, win or lose. A failed evade is doubly punishing: the tempo is
  gone AND you are caught.

### The opening strike -- double duty (defender's edge)
If the raider is caught, **each catcher lands exactly one free strike**, however
many cards it bid to catch. The catch bid pulls double duty: it is both the
contest bid and the opening blow. The raider gets **no** free strike -- it spent
its tempo trying to evade, the catchers spent theirs trying to strike, so the
asymmetry is earned, not fiat. Opening strikes do **Might** damage (Grit/armor
apply, can flip a health card, like any strike).

Worked example (a catcher): 3 cards to catch + 3 extra cards
= 1 opening strike (from the catch) + 3 extra strikes = **4 strikes**, from 6 cards.

### Beat 4 -- extra strikes (simultaneous, hidden)
Only reached if the raider was caught.
- Each extra tempo card = one strike, weighted by **Might** (`tempo cards x Might`
  per the normal strike/damage step).
- **Catchers** may pile on: extra cards -> more strikes on the raider.
- **Raider** retaliates: allocate K cards as strikes across the catchers that
  caught it ("they came to me"). This is the original riposte-allocation ask.
  - `K > 0`  -> the raider stopped to trade -> **repelled** (Abort): it stays home.
  - `K == 0` -> **Push**: it eats the hits and crosses to outrider if it lives.
- Simultaneity: when the raider picks its beat-4 spend it knows the **opening**
  damage it took, but NOT how hard the catchers are about to press (their extra
  bids reveal at the same time).
- Draws from the same per-body pool as the evade bid, so contesting evade hard
  leaves less to strike back with.

### Derived labels
`Answer` as a stored primitive goes away. The model stores an **evade bid** and a
**strike-back allocation**; the log derives the label:

| beat-3 result | strike-back | label |
|---|---|---|
| beat the pool | (n/a)       | Evade |
| caught        | 0           | Push  |
| caught        | > 0         | Abort |

## Foe behavior: the behavior card

Every foe beat -- catch selection and assignment (beat 2), catch-bid sizing
(beat 3), extra-strike sizing and targeting (beat 4) -- is driven by the
creature's **behavior card**: a single, legible, deterministic policy that is the
**one source of truth** for how the creature plays.

- **One artifact, two jobs.** The card is legible enough for a human to run at the
  table, and because it is a deterministic function of the board it is *also* the
  exact policy the solver plugs in. There is no separate "solver foe" and "tabletop
  foe" to keep in sync: the game you balance and the game you play are the same
  object. No fidelity gap by construction.
- **The heuristic and solver are TOOLING, not the shipped behavior.** `greedy_act`
  (the multi-part downs/flips/position heuristic), the solver, and the two-results
  insight grid (c32a6c2 / 9f6693f) are discovery instruments. During tuning you run
  them to find what is "most effective most of the time" for a creature, then write
  that onto its card. The insight grid (card policy vs solver optimal) shows, per
  situation, how much the card gives up against perfect play -- deliberate, legible
  slack a creature is allowed, and exactly the slack the solver then measures.
- **A policy, not a mechanic (why this is not fiat).** 959eeb0 deleted a per-
  creature *mechanic* (an Instinct enum the resolver branched on to change how a
  fight resolves). A behavior card is a per-creature *policy*: it only chooses which
  legal act a creature takes; it changes nothing about resolution. Per-creature
  policies are deterministic and legitimate (a knight moving unlike a bishop is not
  fiat). The invariant is simply **deterministic**, which a written card satisfies.
- **Consequence for dedup.** Behavior now rides on the creature, not purely on its
  stats, so "two stat-identical foes script identically" no longer strictly holds.
  Minor bookkeeping for `interchangeable` (the symmetric-target dedup in
  `legal_acts`): fold card identity into the equality check, or ensure distinct
  behaviors never share a stat line.
- **The offense-vs-denial question dissolves.** It was never a global design choice:
  the card simply *states* what this creature does, discovered per creature in
  tuning. "The role the stats imply" is whatever the card ends up saying.

## The behavior-card format

### What a card must decide (completeness)
Every foe decision in a round, each a pure function of the board:

1. Its own act + target -- clash / raid / slip / melee / hold.
2. Catch or not, and *which* crosser (beat 2).
3. Catch-bid size (beat 3).
4. If it raids: evade-bid size (beat 3).
5. If caught raiding: push vs strike back, and the strike-back allocation (beat 4).
6. As a catcher: extra strikes on the raider it caught (beat 4).

A format that cannot say all six is incomplete.

### The format: an ordered decision list
A card is a short, ordered list of clauses. **First match fires**; the list is
**exhaustive** (a final `ELSE`); every target pick ends in one **global tie-break**
(lowest body index). That is exactly what makes it both human-runnable ("go down
the list, do the first that applies") and a total deterministic function.

```
WHEN <condition> : <verb> <selector> [<tempo>]
```

Closed vocabulary (small on purpose; grows only by the removal test):

- **Conditions** -- `raider crosses me`, `I am caught`, `a back is exposed`,
  `an intruder is in my region`, `else`.
- **Verbs** -- `clash`, `raid`, `slip`, `melee`, `catch`, `strike-back`, `push`,
  `hold`.
- **Selectors** (each a total order over candidates, and each the *concrete face*
  of a `greedy_act` disruption term -- this is what keeps the card tied to the
  tuning tooling):
  - `weakest` -- fewest health cards left -> the **down** term
  - `softest` -- lowest Grit after armor -> the **flip** term (most cards/strike)
  - `deepest` -- crosser aimed furthest into your back / highest Might -> **deny**
  - `richest-back` -- softest exposed rearguard -> **position**
  - `nearest` -- same region first
- **Tempo** (each resolves to an integer from **visible** state -- never the
  opponent's hidden bid):
  - `commit` -- bid to beat the target's *ceiling* (`tempo x Finesse [x bodies]`);
    if the arithmetic cannot win, spend **zero** (never throw tempo at a lost
    contest)
  - `probe N` -- a fixed floor of N cards
  - `to-down` -- the minimum strikes to bring the target to 0, else nothing
  - `all` -- full remaining tempo as strikes
  - `keep` -- stop, husband the rest

`commit`/`probe` are contest postures usable by *either* side of a hidden bid:
`catch ... commit` sizes a catch pool, `raid ... commit` sizes an evade. That is
the one place a fixed assumption about the opponent lives (bid against their
ceiling); everywhere else the selectors and the tie-break decide.

### Two example cards
```
REAVER  (melee bruiser -- a hitter)
  WHEN I am caught       : strike-back weakest to-down, else push
  WHEN raider crosses me : catch deepest commit
  WHEN a back is exposed : raid richest-back commit
  ELSE                   : clash weakest to-down, keep
```
```
WARDEN  (tanky, high Toughness/Finesse -- a denier)
  WHEN raider crosses me : catch deepest commit
  WHEN I am caught       : push          (never trades; its tempo is for catching)
  ELSE                   : clash softest, keep
```
Same language, two legible roles: the difference is which clauses come first and
which selectors they name. The offense/denial split is just `deepest` high in the
list (Warden) vs `weakest`/`to-down` first (Reaver).

### Soundness
- **Total + exhaustive** -> a pure function of the board (tie-break + final `ELSE`).
- **Tempo reads only visible state** -> the hidden contest stays deterministic; a
  perfect player (the solver) computes the foe's bid exactly and never gambles.
  The "simultaneous bid" is therefore a human-bounded-rationality layer, not true
  hidden info -- the very layer the solver is blind to.
- **Greedy, no lookahead** past this round's resolution -> single-agent search
  preserved.
- **`commit` that cannot win spends 0** -> no wasted tempo, and the horde
  asymmetry falls out with no special case: a horde's `x bodies` makes a catch
  ceiling easy to beat (auto-catches cheaply), while an evader rarely out-bids a
  horde pool.

### Public vs secret
- **Public is the tuning ground, and a sound bound.** The solver sees every card
  in full; that is what makes it a deterministic single-agent environment. Because
  hidden info never *helps* a player, balance measured on fully-public cards is a
  cheap **upper bound** on the secret-info game -- so we tune public and get the
  secret version's balance approximately for free.
- **Secret is a tabletop-only overlay.** Hiding a card at the physical table adds
  bluff; it can only make the human's job harder, never easier. It changes nothing
  about the format or the tuning.

### Where it lives
A typed decision list in the catalog (code) -- `Vec<Rule>` over enum `Condition` /
`Verb` / `Selector` / `Tempo` -- **not** RON yet. The vocabulary is still moving;
types catch a malformed card at compile time, matching the existing "catalog is
code, not a data file yet" stance. Migrate to RON (like `data/balance/*.ron`) once
the vocabulary settles. Add a selector or condition **only** when a real creature's
effective behavior cannot be written without it (removal test).

## Solver

Foes are deterministic, so the solver plugs in the foe policy (the behavior card
above) and **exploits** it: it plays the info-complete version of every hidden bid (it knows the pool, so
it bids exactly enough or zero, and never gambles). Thus:
- The solver measures **raw balance** -- an upper bound on winnability.
- The hidden-bid mind-game is a **human-only** layer on top, which can only make
  a fight harder for a human than for the solver. This matches the existing
  deterministic-proxy fidelity stance (solver blind to the mind-game layer).

Search scope: the solver searches the raider's **evade-bid size** and its
**strike-back allocation**. Because unspent tempo is lost at the round reset,
"spend up to your tempo" and "spend your whole tempo" produce the same outcomes,
so the search space is bounded by the pool anyway.

The **no-observable-effect prune is a wanted optimization, not a requirement,
and model integrity comes first.** IF it can be done provably-outcome-preserving
-- skip a strike that cannot contribute to a health-card flip (cannot crack
Grit/armor, or is overkill past a flip) -- take it, because it collapses a lot of
dead branches. If it cannot be made clean, the solver simply enumerates the full
bounded space and nothing is lost but speed. Never trade a correct outcome for a
smaller tree.

## Model shape (representation)

- Drop `enum Answer { Evade, Push, Abort }` as a declared field of `Act::Cross`.
- A crossing carries a raider **evade bid** and a **strike-back allocation**
  (per-catcher strikes, variable length over the caught-by set).
- Because the catcher set is only final once all acts commit, the strike-back
  allocation is a **second decision node** (a reactive beat), not part of the
  up-front `Act`. The foe's catch assignment and both bid beats are likewise
  decision nodes driven by the (deterministic) foe policy.

## Touch-points in current code

- `regions.rs` `enum Answer`, `const ANSWERS`, and every `ANSWERS.map(...)` in
  `legal_acts` -- the flat trichotomy to dismantle.
- `regions.rs` `reach_for_slippers` (the round-robin riposte block, ~1179-1216)
  -- where the fixed allocation lives today; becomes the beat-4 decision.
- `regions.rs` `play_round` `movers` construction and the Crossing-ring phases
  -- where beats 3/4 interleave with resolution.
- `regions.rs` `catchers()` -- already surfaces the predicted catcher set; the
  foe policy for beat 2 builds on it.
- `resolve.rs` `resolve_evade`, `reach_cards`, `slip_cost`, and `land` -- the
  Finesse-weighted contest and the Might-weighted strike step to reuse.
- `game.rs` `Decider` / `Instinct` / `foe_act` -- the foe policy that must grow
  from one act into a multi-beat policy; the solver's `best`/`winnable` search
  that must gain the evade-bid and strike-back decision nodes.

## Open / deferred

- **The starting selector/condition set is deliberately minimal** and grows only
  by the removal test as real creatures are authored -- so the first cards may want
  a term the list above lacks; add it then, not speculatively.
- Whether the no-observable-effect prune (see Solver) is cleanly achievable
  against `land`'s damage model, or is dropped in favour of full enumeration.
