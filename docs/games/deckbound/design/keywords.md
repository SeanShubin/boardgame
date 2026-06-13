# Deckbound — Keyword Vocabulary (the rulebook glossary)

The other half of the [rulebook spine](engine-architecture.md). Each keyword has an
**engine handler** (what it does) and a **manual line** (what the rulebook prints).
Components are **bags of these**; a new keyword is a rulebook change (one redeploy), a
reused one is free data. This is a first cut sized to the
[sample combat](sample-round.md) — enough to make that fight run.

## Reads (Mind)

| Keyword    | Engine                                                                                          | Manual line                                              |
| ---------- | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| **Strike** | aggressive read; beats Scheme, loses to Defense; lands damage; exhausts (Active→Dormant)        | *Attack now. Beats a Scheme; a Block or Evade beats it.* |
| **Block**  | defensive read; beats Strike; on win bank **Power**; self-returns                               | *Absorb a strike — and bank Power.*                      |
| **Evade**  | defensive read; beats Strike; on win bank **Speed** + reposition; self-returns                  | *Slip a strike, reposition — and bank Speed.*            |
| **Scheme** | setup read; beats Defense, loses to Strike; on win bank **Power+Speed+Precision**; self-returns | *Set up for a big payoff — but a Strike interrupts it.*  |

## Stance & positioning

| Keyword               | Engine                                                                                  | Manual line                                                          |
| --------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Attack**            | commit to a target / run the gauntlet; read only vs a mutual engager; exposed elsewhere | *Commit to a target. You're open to anyone you're not engaging.*     |
| **Hold**              | RPS-respond to all comers; add Speed to the front-line **drag pool**                    | *Guard: defend all comers and help wall Runners.*                    |
| **reach** `[min,max]` | legal target distance in jumps (`f↔f 1, f↔b 2, b↔b 3`)                                  | *How far an attack reaches: melee `[1,1]`, ranged out to `[2,2]`+.*  |
| **run-gauntlet**      | become a Runner; the wall's drag may slow/stop you (Speed − drag)                       | *Run past the front line at a back-line target; the wall drags you.* |

## Offense

| Keyword                 | Engine                                                                                                    | Manual line                                             |
| ----------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **targets** `N`         | hit up to N **distinct** entities within reach (no double-target); each takes magnitude                   | *Strike up to N different foes at once.*                |
| **damage-type** `t`     | typed damage; meets armor by type (blunt/sharp/pierce/heat/cold/lightning)                                | *Its damage type — armor stops some types, not others.* |
| **stagger**             | if this lands first, the target **loses its action** (only on cards that carry it — not a universal rule) | *Land first with this and the target loses its turn.*   |
| **Pow / Pre / Mag** `X` | magnitude inputs to the damage formula (appendix); **Pow is pure magnitude** — no interrupt job           | *Force / weak-spot / spell power.*                      |

## Lifecycle

| Keyword         | Engine                                                 | Manual line                                          |
| --------------- | ------------------------------------------------------ | ---------------------------------------------------- |
| **Lasting**     | stays in Active (or its zone) and keeps working        | *Its effect persists until removed.*                 |
| **Fleeting**    | resolves once, then → Dormant                          | *Happens once, then spent.*                          |
| **self-return** | after resolving, → Potential (defensive reads)         | *Returns to hand; cautious play never exhausts you.* |
| **exhaust**     | after resolving, → Dormant (aggressive plays)          | *Spent after use; recover it to reuse.*              |
| **party-zone**  | lives in the shared party zone; affects the whole side | *A team effect, shared by the party.*                |

## Aspect effects

| Keyword               | Engine                                                                             | Manual line                                                                 |
| --------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **armor** `{type:−X}` | reduce incoming damage by type, per source, never cumulative                       | *Cuts damage by type; useless against the type it can't stop.*              |
| **Fear** `X`          | inner attack vs **Resolve**; armor does nothing; overcome → panic                  | *Frightens — armor can't stop it; only resolve does.*                       |
| **Resolve** `X`       | the will track; fear erodes it; accumulates within a round                         | *Your nerve; fear chips it, it steadies between rounds.*                    |
| **Rally** `+X`        | party-zone, collective: +Resolve to all allies; Rallies compound                   | *Steel the whole party's nerve; every Rally boosts the rest.*               |
| **momentum**          | banked Power/Speed/Precision (Active); misread forfeits the bank; multi-win capped | *Winning reads banks advantage to cash later — lose a read, lose the pile.* |

## Creature behavior

| Keyword             | Engine                                                                   | Manual line                                              |
| ------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------- |
| **line** `[conds]`  | deterministic conditional behavior off the visible board (readable)      | *A fixed, readable instinct — no bluff.*                 |
| **deck** `[opts]`   | a shuffled hidden read among options (the bluff) — the foe you must read | *A bluffing foe: its choice is hidden, like a player's.* |
| **target-rule** `r` | targeting: *lowest-Body* / *least-resolute* / *front* / *priority*       | *Whom it goes for.*                                      |

## Traits

| Keyword         | Engine                                                                           | Manual line                           |
| --------------- | -------------------------------------------------------------------------------- | ------------------------------------- |
| **Resolute**    | immune to fear (no Resolve check breaks it)                                      | *Fearless — fear can't shake you.*    |
| **Fanatic**     | fear-immune, but cannot retreat / must press                                     | *Fearless, but can't back down.*      |
| **Coward**      | low Resolve; flees when pressed                                                  | *Easily frightened; breaks and runs.* |
| **Bloodlust**   | must Attack; cannot Hold/defend                                                  | *Must attack — no guarding.*          |
| **Incorporeal** | *(parked)* no Body; physical can't touch; keys on Presence; only Spirit harms it | *(deferred special card.)*            |

## Open

- The handlers above are **intent**, not yet implemented; building them is the rulebook
  code.
- A few are placeholders awaiting [appendix](engine-architecture.md) numbers (the damage
  formula, momentum conversions, the multi-win cap, the drag aggregate).
- New scenarios will surface new keywords — each a one-time rulebook addition.
