# Combat — the card-table app (App spec)

> **This is the App spec for combat**: how the deployed card-table app *realizes*
> the [Rules spec](../canon/2-spec/combat.md). The Rules spec is the game any
> faithful implementation obeys; this doc is this app's choices on top of it - the
> decision surface it presents, the policies it runs, and the presentation. The
> dividing test: would a faithful tabletop port need it? If yes it is a rule (there);
> if it is this app's call, it is here.
>
> Code: `crates/deckbound-board` (the arena, the decision surface, the policies, the
> oracle) and `crates/cardtable` (the rendering). The pure model is `crates/rules`.

## Vocabulary the player sees

The Rules spec's four minor steps are `Target / Bid / Strike / Resolve`. This app
renames the **Bid** step **Catch** everywhere the player reads (tiles, the log, the
Interaction reference) - "catch" pairs with the defender's "slip" and matches the
codebase's own `foe_catch` / "catcher". Internal identifiers stay `bid` / `reach`.
"Pour" is retired: the extra blows are just **strikes** ("+N strikes"). So the player
sees `Target / Catch / Strike / Resolve`; the log prefixes are `target / catch /
strike / resolve` (plus `move`).

## The decision surface: a strike is up to five beats

The Rules spec says every strike declares a target, a bid, and an extra-strike
count. The app surfaces those as a gesture built from single taps (no chrome, no
numeric widgets - clicks and drags only, PC/iPad parity):

1. **WHO** - tap an asked hero to select it (it carries the ring).
2. **WHAT** - tap `Strike...` to enter targeting, or `Hold` to pass.
3. **WHOM** (Target) - tap a lit enemy in the striker's footprint.
4. **Catch** (the Bid) - tap a `Catch c` tile, `c` in `1..=tempo`: how many tempo
   cards buy the reach past the slip. Higher c is harder to slip but leaves fewer
   strikes.
5. **Strike** - tap a `Strike +p` tile, `p` in `0..=(tempo-catch)`: how many extra
   strikes beyond the free opening blow. `0` = opening blow only; the rest of the
   pool is held back for a dodge or a later step.

Both the catch and the strike count are **required** - an aimed-but-uncommitted
strike stays owed and never commits by itself. **Resolve** is automatic at Commit.

Two shortcuts, from the Rules spec's own shape:
- **Movement steps** (Withdraw / Cross) are a single `Go` / `Stay` choice, no
  catch/strike beats.
- **Area strikes** (a `Sweep`/`Salvo` body) have a fixed one-card reach and no slip
  contest, so there is no catch or strike to pick - a tap on a lit enemy stages the
  whole sweep at once. (Area strikes stay **one sweep**; see Deferred.)

Back out of any beat by re-tapping the commanding body; switching bodies drops a
half-built gesture rather than stranding it. (`arena.rs`: the `aiming` / `bidding` /
`striking` flags and `handle_tap` / `step_choices` / `choose`.)

## The policies this app runs

The Rules spec requires only that each actor's choice be deterministic; it does not
say *how* to choose. This app's choices:

- **The suggested / default bid - `reach_cards`.** For any target the app computes
  the bid that **lands the most damage while predicting the target's sensible
  dodge** (ties toward fewer reach cards, so the rest becomes strikes). This is the
  value the solver and scripted foes attach, the arena's pre-selected optimum, and
  what the oracle scores a player's other picks against. (It is *dodge-aware*, not
  the old "fewest cards the target cannot afford to slip".)
- **The default extra-strike count.** The declaration layer attaches the
  current-behaviour default - **all remaining tempo** at a mutual-melee step, **0**
  at the volley/raid or for an area/horde body (`pour_default`). The player may pick
  any other count on the Strike beat.
- **The automatic dodge - opportunity-cost.** A defender's dodge is auto-resolved,
  not declared. It **slips only when standing would cost more Health/bodies than the
  slip costs tempo** - tempo is one pool for offence and defence, so a body no
  longer disarms itself dodging a blow it could better absorb. (`would_slip` /
  `dodges_against`; slip on `harm > slip_cost`, a tie keeps the tempo.)
- **The scripted foe.** A foe declares by one greedy policy: the max-disruption
  target (`foe_catch`), crossing when the one-ply read says to (`wants_to_cross`),
  the auto-optimal bid, and an all-in strike count. It **passes rather than throw a
  strike that flips nothing** - a body whose Might cannot clear the target's Grit
  keeps its tempo (sub-Grit damage is discarded either way), so a winnable verdict
  never rests on the enemy hitting what it cannot hurt. This is the deterministic
  **subset** the Rules spec allows a creature.

## The doom oracle

Every choice on offer is annotated with where it **leads** - `Winnable` /
`Evaluating` / `Doomed` - computed by the same solver the balance gate asserts, over
the same combat model. It **marks, never bars**: a losing choice stays fully
playable, so a player can make the losing move and find out *why* it loses. Each
Catch tile shows the best outcome over its strike counts; each Strike tile is scored
at its exact `(catch, strikes)`; lit enemy tiles carry their outlook while aiming.
(`choice_outlooks` / `aim_outlook_by_foe` / `score_candidate`, budgeted per frame.)

## Presentation

- **Two progress tracks.** *Step* - the eight phases, current lit, then Reset.
  *Interaction* - the four minor steps `Target / Catch / Strike / Resolve`, the live
  beat lit (Target while aiming, Catch while picking the reach, Strike while picking
  the count; Resolve is automatic). (`scene.rs::build_tracks`.)
- **Tile badges show raw damage, not cards flipped.** A Catch/Strike tile reads
  `N dmg` (or `slipped`), the blows-times-Might this choice banks. Damage - not
  Health-cards-flipped - is the honest per-attacker number, because the Grit pile is
  **shared** across a step: whether a card actually flips also depends on every other
  blow into that target this step, so a single strike's card count would presume no
  other source. The player reads `N dmg` against the target's visible Grit.
  (`strike_report`.)
- **The log** narrates every strike as `target / catch / strike / resolve` with its
  resolution math, so a play session is reconstructable from `combat-log.log`.

## Deferred (app-side)

- **Area-strike dumping.** An area strike stays a single unevadable sweep for one
  tempo; extra tempo is **not** dumped into extra sweeps. Measured: once an area body
  has spare tempo, dumping sweeps trivialises the composition corners - so the rule
  would effectively pin every area kit to Cadence 1. Deferred until a real limiter
  exists, arriving with the armor re-introduction (armor is likewise deferred - a
  per-strike cut before the Grit pile, canon README §2).
- **Foe declared strike-sizing.** A scripted foe spends `{0 or all-in}`; a mid-range
  count is behaviour-card territory.
- **Declared defense.** The dodge is automatic; a player-declared dodge bid is a
  fold-out for playtesting to demand.

## Balance

The diagonal gate (`cargo test -p deckbound-board --test diagonal`) asserts this
app's *default* play - the suggested bid, the all-in strike default, the automatic
dodge - so it is the balance authority for the deployed app. A player's off-default
catch/strike choices are scored by the oracle but do not move the gate; the gate
stays green by construction because the solver plays the defaults.
