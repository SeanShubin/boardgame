# Combat - who can target whom, and when

> **Auto-generated** from `deckbound_board::targets` (the step machine's reach rules, written out) -
> do not edit by hand; regenerate with `cargo run -p deckbound-board --example targets`. A test fails
> if it drifts. The canonical prose is `docs/games/deckbound/combat-round-sequence.md`.

The round is EIGHT steps, each its own declare/reveal wave, resolved on the spot - so a death at an
early step silences every later one. The two movement steps (2 Withdraw, 4 Crossing) move bodies
instead of striking: an outrider may rejoin its own line, and a vanguard that declared no line strike
may cross, landing as an Outrider.

**Answerable** means the target may strike back along the edge in the same wave: a mutual melee step
trades both ways because both declared; a ranged shot is one-way.

| #   | Step             | Attacker  | Reach  | Target               | Answerable | Condition                                                                             |
| --- | ---------------- | --------- | ------ | -------------------- | ---------- | ------------------------------------------------------------------------------------- |
| 1   | Havoc            | Outrider  | weapon | anyone in its region | yes        | point-blank: both tiers, no screen; its hosts strike back in the same wave            |
| 3   | Skirmish         | Vanguard  | melee  | Vanguard             | yes        | the early trade; a line strike here bars your own crossing this round                 |
| 5   | Defensive Volley | Rearguard | ranged | Outrider             | no         | one-way, the opening blow only                                                        |
| 6   | Raid             | Outrider  | melee  | Rearguard            | no         | this round's arrivals only, in the region they landed in; opening blow only, evadable |
| 7   | Assault          | Vanguard  | melee  | Vanguard             | yes        | the late trade - every vanguard that held back swings here                            |
| 7   | Assault          | Rearguard | ranged | Vanguard             | no         | -                                                                                     |
| 8   | Advance          | Vanguard  | melee  | Rearguard            | yes        | only a rearguard with NO living vanguard at this step (the same-round advance)        |
| 8   | Advance          | Rearguard | ranged | Rearguard            | no         | only a rearguard with NO living vanguard at this step                                 |

## The schedule at a glance

| #   | Step             | Who -> whom                                     |
| --- | ---------------- | ----------------------------------------------- |
| 1   | Havoc            | O->RV, RV->O (in-region, mutual)                |
| 2   | Withdraw         | O may move to its own line                      |
| 3   | Skirmish         | V->V (bars the striker's crossing)              |
| 4   | Crossing         | V may move to their line (if it did not strike) |
| 5   | Defensive Volley | R->O (one-way, opening only)                    |
| 6   | Raid             | O->R (this round's arrivals, opening only)      |
| 7   | Assault          | RV->V (all firepower to bear)                   |
| 8   | Advance          | RV->R (only an unscreened back, AT this step)   |
