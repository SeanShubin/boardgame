# The round sequence

Status: **the shipped model**, 2026-07-21. The canonical step-by-step procedure
for one combat round, written so a human can run it at a table - and it is what
`rules::combat` actually runs: the round is **eight steps**, each its own
declare/reveal wave (`step_game`), each resolved on the spot (`steps.rs`), and
the combat log (`scripts/combat.sh` / `scripts\combat.ps1`, mirrored to
`fight-log.txt`) prints these exact coordinates - `[round N - step K/8: Name]`.
The crossing's bid math lives in [crossing-bid-tree.md](crossing-bid-tree.md);
this document is the *frame* - the order of play, and who may do what when.

## Two principles, stated once

**1. Every step is a simultaneous declare/reveal.** At each step BOTH sides
secretly declare their part, then reveal together, then the step resolves
deterministically from what was revealed. The rulebook is the same for both
sides at every step - there is no step one side has and the other lacks.

**2. Perfect information collapses choices; it does not remove steps.** Hidden
information is the only thing that makes a declaration a gamble. Against a fully
predictable (scripted) enemy there is no hidden information, so most declarations
have exactly one sensible value - the rest are dominated. The **rules keep every
step for both sides**; the **user interface** is what hides a declaration that
cannot change the outcome and auto-advances a step where you have no real choice.
So: nothing in this sequence is ever skipped - only choices that cannot matter
are hidden from the player.

Read the sequence below as the full rules. What a solver or UI *shows* is this
same sequence with the dominated choices pruned.

## Design intent: the two shapes

The sequence exists to produce two recognizable strategic shapes. Every rule is
judged by whether it serves them with a **thematically coherent cost** - not by
being mechanically clever. The mechanics stay flexible; what must hold is that the
costs read true.

**The opening shape (round one).** The vanguard's crossers strike ahead to
**disrupt** the enemy's soft-but-dangerous rearguard - assassinate it if they can,
but at minimum tie it up - buying the main army safety from that fire. The price is
real: a crosser becomes an outrider, exposed inside the enemy ranks and liable to
be wiped out earlier than it would have been in formation. Behind that screen the
vanguards (and whatever tempo the rearguards keep) exchange their damage; and once
an enemy vanguard collapses, its rearguard is exposed and gets cleaned up.

**The ongoing shape (later rounds).** Everyone pays the consequences of where they
stand. Outriders are the sharpest expression of it: loose inside the enemy, they
wreak havoc but are vulnerable to everyone at once.

**Disruption, not a guaranteed kill.** Reaching a rearguard does NOT guarantee a
hit - it may still dodge (the raid strike is evadable, by design). The outrider's
job is to disrupt the rearguard's damage: by killing it, *or* by keeping it
occupied - burning its tempo, threatening it - until the enemy vanguard falls and
the exposed rearguard is reached and finished. The kill is only one form of the
disruption, which is why the raid is not guaranteed.

## Terminology

- **Vanguard** - front rank, strikes in **melee**.
- **Rearguard** - back rank, strikes at **range**; screened while its own
  vanguard stands.
- **Outrider** - a body loose *inside* the enemy formation, having crossed in.
- **Tempo** - each body's pool of action cards, refreshed to Cadence at round
  end. Every bid and every strike spends tempo.
- A **bid** is weighted by **Finesse** (and multiplied by the attacker's **body
  count** - a horde reaches with many hands). A **strike** does **Might**
  damage. **Engaging earns one free opening strike** - the clash, or the shot -
  and every extra strike costs one more tempo. A **melee** engagement is two-way
  (it can be answered); a **ranged** shot is one-way (it cannot). See the
  engagement rule under Global rules.

## The round: eight steps, each declared, revealed, and RESOLVED in turn

A round is eight steps. At each step, every **eligible** body (hero and foe,
one loop) secretly declares its part - a strike target, a pass, or a move -
then the step reveals and **resolves immediately**: damage lands, deaths close,
positions change. The next step's declarations are made against the board *as
it now stands*, which is the whole point of the order - a death at an early
step silences a later one, and a front that collapses mid-round is advanced
upon in that same round.

Eligibility is the branching rule: a body with no rank, target, or tempo for a
step simply has no declaration there. The primitive under every strike is the
same **Interaction**: name targets, bid contact, strike (free opening blow plus
paid extras), down what falls.

| # | Step | Who -> whom | What happens |
|---|---|---|---|
| 1 | **Inner** | O->RV, RV->O | Point-blank: every prior-round outrider and its hosts trade declared strikes - both tiers, no screen, mutual, aoe sweeps the region. A kill here opens a hole before anything else runs. Afterwards an outrider whose host formation is wiped **dissolves** back to its own line. |
| 2 | **Withdraw** | O may move to V | Every surviving outrider may leave, rejoining its own line at weapon rank - free; standing step 1 was the price. A body felled at step 1 never leaves. |
| 3 | **Early Trade** | V->V | The early front trade - and the **interception window**: strike the body you predict will run. Blind: crossings are declared at step 4, after this resolves - a real feint layer between humans, dominated-choice-pruned against scripted foes. |
| 4 | **Cross** | V may move to O | Only a vanguard that declared **no line strike** this round may cross; it walks uncontested and lands as an **Outrider**. The step-3 window behind it and the step-5 volley ahead of it are the price - the screen is a price, not a wall. |
| 5 | **Volley** | R->O | The rearguards' one-way shots at outriders, fresh or old - the opening blow only. Holding off IS being quicker. |
| 6 | **Raid** | O->R | THIS round's arrivals strike a back-line target - the opening blow only, evadable (a reached rearguard may dodge, spending tempo it then cannot fire). Prior-round outriders acted at step 1. |
| 7 | **Late Trade** | RV->V | The late front trade: rearguard fire plus every vanguard that held back. A would-be crosser that thought better of it swings HERE - "halt" is emergent, not a rule. |
| 8 | **Advance** | RV->R | Only against a rearguard with **no living vanguard at this step** - the same-round advance on a collapsed front. An exposed back is reachable the round its screen dies, not the round after. |

### Round end

Tempo refreshes to Cadence (leftover does not carry). Damage piles close - an
unfinished wound is gone. Deaths finalize; a rearguard whose vanguard fell
keeps its **rank** and its early-fire slots (steps 5 and 7) - exposure makes it
reachable, never a different thing.

## Global rules that cut across the steps

- **Engaging melee earns one free blow; ranged is one-way.** Whoever reaches a
  target in melee lands one free opening strike (the clash itself); a mutual
  melee step trades both ways because both declared. A body firing from
  **range** lands its shot but is **never answered**: you cannot strike back at
  something you never reached.
- **Tempo is live across the whole round.** The mutual melee steps (1, 3, 7, 8)
  pour the striker's whole remaining pool into its declared target; the volley
  (5) and the raid (6) land the opening blow only. A body may act at every step
  it can fund - and an all-in pour at an early step is exactly why it has
  nothing left at a late one. One-strike-per-round is emergent for low-Cadence
  bodies, never a rule.
- **Contact bids auto-size; defense is the automatic greedy dodge.** The *whom*
  is declared; the contest's *how hard* is computed (`reach_cards` - the fewest
  cards the target cannot afford to slip, else the minimum). Reaching is never
  a guaranteed hit.
- **Area strikes never target and never retaliate.** An area (aoe) body's
  strike is *always* the untargeted regional sweep - every enemy in the tier it
  is aimed at, unevadable, one tempo. Width, never depth: a body you could not
  single-target, you cannot sweep.
- **Position is never declared; it is only ever earned.** The two movement
  steps (2 and 4) are the only movement in the game, and both are priced by the
  steps around them, not by a toll of their own.
- **The screen is a price, not a wall.** Nothing stops a crossing: the line
  strikes its prediction at step 3, the back volleys the arrival at step 5.
  What the crosser pays is standing in those windows - in blood, or in the
  tempo it burns dodging.

## Implementation status (2026-07-21)

**The sequence above IS the shipped model.** `combat/steps.rs` resolves each
step from a `StepScript` of declarations; `combat/step_game.rs` is the round as
a `Game` - eight declaration waves, each wave resolved when it completes, live
tempo in the solver key; `regions.rs` keeps only the physics (the board, the
exchange, the grit pile, area strikes) and the two instinct reads
(`foe_catch`, `wants_to_cross`). The old two-wave model (`Act`/`Answer`, the
catch wave, the pooled crossing contest, `play_round`) is **deleted** - each of
its choices re-emerged as an ordinary step: evade = the contact dodge against
step-3 strikes; push = crossing at step 4 after taking them; halt = staying and
swinging at step 7.

**Measured (2026-07-21): the step machine reproduces the wave model's balance
EXACTLY** - 4/4 solos (each foe soloable by exactly its counter kit), 14/14
insight cells, both clash-only controls still lose - with zero content
re-tuning. The named risks (raids cheapening, screen deterrence, solver
tractability) did not materialize; the diagonal gate
(`cargo test -p deckbound-board --test diagonal`) asserts the step machine.

What remains, deliberately deferred:

- **Declared pour sizes** - strike extras beyond {0, pool} are behavior-card
  territory (foes) and a decision-richness add (party).
- **Declared defense** - the dodge is the automatic greedy; making it a
  declared bid is a fold-out for playtesting to demand.

## History (how this model settled, 2026-07-20/21)

The sequence grew from a brainstorm that factored combat into one **Interaction
primitive** (target -> contact bid -> strike with free opening + paid extras ->
resolve) applied over a **rank-pair schedule** - position IS rank, given one
region per side. Its deltas were implemented and measured one at a time against
the diagonal gate, first inside the two-wave frame (acts + catches, resolved
through three rings), then by dissolving that frame into the steps:

1. **Withdrawal (O->V)** - landed clean; the "a crossing is committed, no
   retreat" tenet was demoted (a means to simplicity, not a goal). The Inner
   Ring alone proved a sufficient price.
2. **Catch = the clash you declared (one act, one strike)** - REVERTED on
   measurement: the Sweep and Raid corners collapsed to unwinnable.
   **CAVEAT (added 2026-07-20): that experiment was CONFOUNDED.** It bundled
   three changes - (a) the catch consumes the catcher's act, (b) no strike-phase
   extras for catchers (the pile-on was unimplemented, so a cheap catch left the
   rest of the pool STRANDED - an implementation artifact, not a property of the
   act-consuming design), and (c) catch targeting moved from everyone-catches to
   only-your-declared-target. The collapse condemns that bundle; the clean
   comparison (act-consuming WITH the strike-phase pour) was never run. With the
   pour in place, the two models differ only in **split-freedom**: whether one
   body may divide its pool across two targets in the same round.
3. **The catch wave** - a genuine second declaration per body, additive and
   tempo-priced. Landed clean; the solver searched it, foes played the catch
   instinct.
4. **The pile-on** (the strike-phase extras) - first implemented as an always-
   finish resolver instinct: MEASURED and rejected (Raid corner X; the counter-
   knob broke Ashfen's clash-only guard - the Sniper cannot be both lethal
   enough to demand silencing and an executioner of the spent runner who comes
   to silence it). Resolved by moving the pour to the DECLARATION (pour 0 or
   the finishing pour): physics untouched, allocation is policy. Foes default
   to mission-focus; an executioner is a future behavior-card trait. Diagonal
   held 4/4 + 5/5 with the party's pours fully searchable.
5. **The step machine (2026-07-21)** - the eight-step schedule made literal:
   per-step declare/reveal with immediate resolution, so targeting reacts to
   same-round deaths (the collapsed front is advanced upon at step 8 of the
   round it fell). The crossing's bespoke contest machinery (`Answer`,
   `Volley`, the pooled bid, the free blow on halt, the strike-back
   allocation, the catch wave) dissolved into ordinary steps. Measured EXACT
   balance agreement with the wave model, then the wave model was deleted.

**Canon ruling (2026-07-20): the additive model, with split-freedom.** A body may
be in as many engagements as its tempo funds - across the steps it can afford -
and may split its pool across targets if it can afford to (at high Finesse that
may even be the strong play). The only requirement on creatures is DETERMINISM:
a creature may carry a rule for how it splits, but need not have one.
