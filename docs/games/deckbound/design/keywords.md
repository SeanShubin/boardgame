# Deckbound — Keyword Vocabulary (the rulebook glossary)

The other half of the [rulebook spine](engine-architecture.md). Each keyword has an
**engine handler** (what it does) and a **manual line** (what the rulebook prints).
Components are **bags of these**; a new keyword is a rulebook change (one redeploy), a
reused one is free data. This is a first cut sized to the
[sample combat](sample-round.md) — enough to make that fight run.

## Stances (Mind)

| Keyword    | Engine                                                                                               | Manual line                                              |
| ---------- | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| **Strike** | aggressive stance; beats Scheme, loses to Defense; lands damage; exhausts (turned face down)         | *Attack now. Beats a Scheme; a Block or Evade beats it.* |
| **Block**  | defensive stance; beats Strike; on win bank **Power**; returns to hand                               | *Absorb a strike — and bank Power.*                      |
| **Evade**  | defensive stance; beats Strike; on win bank **Speed** + reposition; returns to hand                  | *Slip a strike, reposition — and bank Speed.*            |
| **Scheme** | setup stance; beats Defense, loses to Strike; on win bank **Power+Speed+Precision**; returns to hand | *Set up for a big payoff — but a Strike interrupts it.*  |

## Engagement & positioning

| Keyword               | Engine                                                                                          | Manual line                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Attack**            | commit to a target / run the gauntlet; stance taken only vs a mutual engager; exposed elsewhere | *Commit to a target. You're open to anyone you're not engaging.*     |
| **Hold**              | RPS-respond to all comers; add Speed to the front-line **drag pool**                            | *Guard: defend all comers and help wall Runners.*                    |
| **reach** `[min,max]` | legal target distance in jumps (`f↔f 1, f↔b 2, b↔b 3`)                                          | *How far an attack reaches: melee `[1,1]`, ranged out to `[2,2]`+.*  |
| **run-gauntlet**      | become a Runner; the wall's drag may slow/stop you (Speed − drag)                               | *Run past the front line at a back-line target; the wall drags you.* |

## Offense

| Keyword             | Engine                                                                                                                                                           | Manual line                                             |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **targets** `N`     | hit up to N **distinct** entities within reach (no double-target); each takes magnitude                                                                          | *Strike up to N different foes at once.*                |
| **damage-type** `t` | typed damage; meets armor by type (blunt/sharp/pierce/heat/cold/lightning)                                                                                       | *Its damage type — armor stops some types, not others.* |
| **stagger**         | if this lands first, the target **loses its action** (only on cards that carry it — not a universal rule)                                                        | *Land first with this and the target loses its turn.*   |
| **Pow / Pre** `X`   | magnitude inputs to the damage formula (appendix); **Pow is pure magnitude** — no interrupt job (Mag folds into Pow — conjured/elemental deliveries use Pow too) | *Force / weak-spot.*                                    |

## Lifecycle

| Keyword         | Engine                                                 | Manual line                                          |
| --------------- | ------------------------------------------------------ | ---------------------------------------------------- |
| **Lasting**     | stays in Active (or its zone) and keeps working        | *Its effect persists until removed.*                 |
| **Fleeting**    | resolves once, then turned face down                   | *Happens once, then spent.*                          |
| **self-return** | after resolving, returns to hand (defensive stances)   | *Returns to hand; cautious play never exhausts you.* |
| **exhaust**     | after resolving, turned face down (aggressive plays)   | *Spent after use; recover it to reuse.*              |
| **party-zone**  | lives in the shared party zone; affects the whole side | *A team effect, shared by the party.*                |

## Aspect effects

| Keyword               | Engine                                                                                                                                                                                      | Manual line                                                                                        |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| **armor** `{type:−X}` | reduce incoming damage by type, per source, never cumulative                                                                                                                                | *Cuts damage by type; useless against the type it can't stop.*                                     |
| **Fear** `X`          | inner attack: first **− Ward** (vs-fear), then vs the **Resolve** bar — **Fear must exceed Resolve** → panic; armor does nothing                                                            | *Frightens — armor can't stop it; a vs-fear ward blunts it, then resolve holds.*                   |
| **Ward** `{kind:−X}`  | **passive, typed inner cut** (vs-fear, vs-confusion), per-source, **never depletes**; applied **before** the Resolve / Mind-capacity bar; **not** anti-magic (fireballs meet Armor vs-heat) | *A standing guard against fear or confusion — never the magic itself.*                             |
| **Resolve** `X`       | the will **threshold**: a standing value **Fear must exceed** to break you; **never depletes** (no Health stack)                                                                            | *Your nerve — a bar fear must clear, not a pool that drains.*                                      |
| **Confusion** `X`     | inner attack vs **Mind**: first **− Ward** (vs-confusion), then **shrinks the focus pool** (prediction bandwidth) by the remainder → the attacker's stances **auto-succeed**                | *Muddles the mind — a vs-confusion ward blunts it, then it spoils your predictions, so foes land.* |
| **focus**             | the Mind-sized **defensive pool**: each held point can **negate** one blow by predicting it; **drained by the attacker's Speed** (faster foes cost more), shrunk by Confusion               | *Your read: spend it to negate a blow you predict; fast foes burn it faster.*                      |
| **Rally** `+X`        | party-zone, collective: +Resolve to all allies; Rallies compound                                                                                                                            | *Steel the whole party's nerve; every Rally boosts the rest.*                                      |
| **momentum**          | banked Power/Speed/Precision (Active); a misjudged stance forfeits the bank; multi-win capped                                                                                               | *Winning stances banks advantage to cash later — lose a stance, lose the pile.*                    |

## Creature behavior

| Keyword             | Engine                                                                        | Manual line                                              |
| ------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------- |
| **line** `[conds]`  | deterministic conditional behavior off the visible board (readable)           | *A fixed, readable instinct — no bluff.*                 |
| **deck** `[opts]`   | a shuffled hidden stance among options (the bluff) — the foe you must predict | *A bluffing foe: its choice is hidden, like a player's.* |
| **target-rule** `r` | targeting: *lowest-Body* / *least-resolute* / *front* / *priority*            | *Whom it goes for.*                                      |

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
