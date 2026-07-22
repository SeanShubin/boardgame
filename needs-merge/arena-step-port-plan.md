# Porting the production combat arena to the step machine

Status: staged plan, 2026-07-22. The production card-table app (`crates/boardgame` bin ->
`deckbound_board::CardTableGame`) still fights through the OLD combat stack; the canon round
sequence (`docs/games/deckbound/combat-round-sequence.md`, the eight-step machine in
`crates/rules/src/combat`) runs only in the dev fight simulator and the balance gate. This
plan closes that gap, stage by stage, additive-and-inert until measured, per the house
pattern.

## What stands where today

- **Deployed**: `arena.rs` (~2,300 lines) - a cards-as-truth fight surface over
  `deckbound-board`'s OWN parallel combat stack: `combat.rs` (its own `Combatant` /
  `Engage` / `Contact` economy over `deckbound_content::schedule::SCHEDULE`'s five
  sub-phases), `battle.rs` (its greedy policy), `solver.rs` (its own `Oracle`). A round is
  Marshal (declare ranks into `[Vanguard]`/`[Outrider]`/`[Rearguard]` piles, foes secret in
  `[Muster]`) then five sub-phases of Strike -> React -> Extra mini-steps, with scratch
  `contact` cards.
- **Canon**: `rules::combat` - the eight-step round, per-step declare/reveal waves, live
  tempo, `StepCombat` behind the generic `Game`, the generic `Solver`/`StepScorer`,
  balance-gated by `tests/diagonal.rs`. Driven interactively only by
  `boardgame/examples/fight.rs` (a button UI, not the card table).

The port is therefore not a rules change at all: it is **deleting a duplicate rules stack
and re-expressing the canon machine in card-table vocabulary**. The fight simulator is the
working template for every decision surface (option menus, commit lines, the
round/step/minor-step log, verdict + best-route outlooks, the best marker).

## Design pins (settled before code)

1. **Cards stay the truth; `StepState` is transient engine state.** The persistent board
   (roster, health, tempo as card detail, pile membership) is the source of truth. Opening
   the arena seats a `StepState` FROM the cards (as `read_combatant` does today); each
   Commit advances the engine one wave and writes the step's `StepLog` diff BACK to cards -
   the same diff discipline as `narrate_steps`, so no state change is invisible on the
   table. No scratch contact cards: steps resolve immediately, nothing needs to persist
   between mini-phases.
2. **Rank is pile membership - and now it is EARNED, never declared.** The old arena's
   whole Marshal beat (rank the `[Pool]`, foes hidden in `[Muster]`, reveal on commit)
   disappears: rank is weapon-derived, so a fight opens with both lines already seated in
   their `[Vanguard]`/`[Rearguard]` sub-piles, foes face up. The one moving rank is the
   Outrider, and it moves by RESOLUTION: a crosser's card physically walks into the enemy
   line pile at step 4, and back out at Withdraw. Position earned = the card moved - the
   metaphor the model was built for.
3. **A wave is a staged plan; Commit is the information boundary.** Within a step, each
   eligible party body stages its declaration (tap-to-aim = the generic card
   `association`/Link; pass is the default); the step's Commit card reveals the wave and
   resolves the step on the spot. Foe declarations are scripted (`step_policy`) and
   auto-advance - principle 2 of the canon doc: the rules keep every wave, the UI hides
   declarations that cannot matter. Back rewinds to the previous Commit, never across one.
4. **Steps are the phase deck; minor steps are narration.** The rotating phase deck shows
   the current step (`step 3/8: Skirmish`); waves nobody can act in are auto-advanced and
   logged `- skipped`. Target/Bid/Strike/Resolve are not decision points - they are the log
   card's lines, in the exact `fight-log.txt` format.
5. **One rules stack.** At cutover, `deckbound-board::{combat, battle, solver}` and
   `deckbound_content::schedule` (+ `rank::Intention` if nothing else holds it) are DELETED.
   `units.rs` (kit/beast -> `rules` `Combatant`) is already the bridge the gate uses; the
   arena joins it.

## Execution: ONE SHOT (revised 2026-07-22, user call)

The staged coexistence below was insurance for multi-session work; executed as one
continuous run it is overhead. The revised shape: build the bridge and its no-drift replay
gate first (the net), then rewrite the arena surface directly on it - no switch, no
parallel period - then the deletions, fixture rewrite, regenerated targets, and a full
`verify.sh`. Clean commits at each internally-green boundary, but one sustained pass, old
arena to new, in this order: bridge -> surface -> outlooks -> deletions + fixtures ->
targets + docs -> verify. The original stages remain below as the work breakdown.

## Stages

**A. The engine bridge (additive, inert).** A new module beside the old arena:
seat-from-cards (`Board` piles -> `StepState`), and apply-back (`StepLog` diff -> card
operations: health/tempo detail edits, pile moves for cross/withdraw/dissolve, downed cards
to the fallen zone). Gate: a replay test that drives whole scripted fights through the
bridge and asserts the final card state byte-matches `play_steps` on the same units - the
no-drift guarantee between the two surfaces. Nothing user-visible changes.

**B. The interaction surface (additive, behind a switch).** The wave loop over the bridge:
staged aims via tap/drag association, the Commit card per wave, foe waves auto-advanced,
skipped waves logged, Back-to-previous-commit, the battle log card in the canonical
round/step/minor-step format. Headless model tests play full encounters through
`scene_choices`/`handle_tap`-style entry points, exactly as the old arena's tests do today.
The old arena still ships.

**C. Outlooks (the thinking surface).** Verdict, win/loss line counts, best route, and the
best-route marker per staged choice - the fight simulator's panel, re-expressed as card
outlooks - computed with the generic `Solver`/`StepScorer` under the ui-never-blocks rule:
per-frame bounded grind over a persistent memo (the fight example already demonstrates the
pattern), never a blocking solve on a click.

**D. Cutover and deletion.** `CardTableGame` opens the step arena; the switch and the old
stack go: `arena.rs`'s Marshal/sub-phase machinery, `combat.rs`, `battle.rs`, `solver.rs`,
`deckbound_content::schedule`. The arena fixture tests (the Marshal/formation/sub-phase
suite - including the seven repaired on 2026-07-21) are rewritten against the step arena.
`targets.rs` regenerates the reference table from the step machine's reach rules (the
eight-step who-reaches-whom, replacing the 3x3 sub-phase join); `combat-targets.md`
regenerates. CLAUDE.md's stale "no game wired in yet" line gets corrected in passing.
Gates: full `verify.sh`; the diagonal (untouched - it never depended on the arena); the
stage-A replay test now guarding the production path.

**E. Polish and deploy.** Restart / next-encounter flow, log presentation on the table,
PC-iPad parity audit of every new gesture (tap+drag only, no hover), and a wasm build
check (`trunk`/deploy workflow) since the arena is in the deployed binary.

## Risks and open calls

- **The Muster reveal is lost with Marshal.** The old blind-formation bet was real theater;
  the new model has no formation declaration to hide. If the reveal beat is missed, the
  equivalent tension now lives per-wave (your staged aims vs the unrevealed step) - but
  this is a feel call to confirm at stage B, not a rules question.
- **Wave secrecy in single player.** Against scripted foes every declaration is
  predictable; the UI must stage-then-commit anyway (the commit tenet) without feeling like
  ceremony. Stage B should lean on auto-advancing every forced choice.
- **Tempo as visible card state.** Live mid-round tempo is new to the arena (the old model
  reset per sub-phase). Detail-line rendering may want a dedicated tempo chip row; defer to
  stage E unless it blocks legibility.
- **Undo scope.** Back rewinds within the current wave only (up to the last Commit) - the
  single-player-only rule. Restart covers everything past that.

## What this does NOT touch

The rules layer (`rules::combat`), the canon doc, the balance gate, and the fight
simulator all stand as-is; the deckbound SAMPLE crates (`deckbound`, `tabletop`,
`deckbound-sample`) and the aspirational `canon/2-spec` are out of scope entirely.
