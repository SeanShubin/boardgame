# Combat - who can target whom, and when

> **Auto-generated** from `deckbound_content::schedule::SCHEDULE` joined with the range rule
> (`combat::rank_is_ranged`) and the screen rule (`combat::back_access_ok`) - do not edit by hand;
> regenerate with `cargo run -p deckbound-board --example targets`. A test fails if it drifts.

The schedule is a **complete 3x3**: every rank gets exactly one slot against each enemy rank, so it
does not decide *who* may hit *whom* - everyone eventually reaches everyone. It decides **when**. An
empty target rank simply voids that pairing, for every rank, with no exception.

**Answerable** means the target may spend its own tempo striking back along the edge. A melee contact
is mutual - the body you engaged did not choose the fight, and you could have let it pass. A ranged
contact is one-way: you cannot punch an archer at range.

| Sub-phase | Attacker | Reach | Target | Answerable | Condition |
|---|---|---|---|---|---|
| Intercept | Vanguard | melee | Outrider | yes | - |
| Volley | Rearguard | ranged | Outrider | no | - |
| Raid | Outrider | melee | Rearguard | yes | - |
| Clash | Rearguard | ranged | Vanguard | no | - |
| Clash | Vanguard | melee | Vanguard | yes | - |
| Breach | Vanguard | melee | Rearguard | yes | only once the target's own Vanguard has fallen (the screen) |
| Breach | Outrider | melee | Vanguard | yes | - |
| Breach | Outrider | melee | Outrider | yes | - |
| Breach | Rearguard | ranged | Rearguard | no | only once the target's own Vanguard has fallen (the screen) |

## The 3x3, by *when*

| Attacker \ Target | Vanguard | Outrider | Rearguard |
|---|---|---|---|
| **Vanguard** | Clash | Intercept | Breach |
| **Outrider** | Breach | Breach | Raid |
| **Rearguard** | Clash | Volley | Breach |
