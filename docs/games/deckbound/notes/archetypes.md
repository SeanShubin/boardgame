# Deckbound — Character & Monster Archetypes

Context for tuning the systems and for designing the [Mind stance](decision-making.md):
who fights, how they specialize, and why no mechanic is safe to ignore.

## Solo vs cooperative — the core split

- **Solo plays tall.** With no ally to cover a gap, a lone hero must **max every
  axis** — Body, Mind, toughness, Speed — and self-recover. Powerscaling is
  [uncapped](coordination-and-interruption.md#speed-is-the-currency-of-sub-phase), so the
  fantasy is the *Solo-Leveling* one-person army with an answer to everything.
- **Co-op plays wide.** A group wins through **specialization and synergy** — each
  member leans hard into a few axes and relies on the others for the rest. A
  specialist out-performs a generalist *in their lane* but is helpless outside it.

The monster roster is what forces the choice: it is **varied enough that every
mechanic is someone's lifeline**, so a solo hero must be broad and a party must be
diverse.

## Character archetypes

| Archetype              | Leans on                    | Role                                                                                        |
| ---------------------- | --------------------------- | ------------------------------------------------------------------------------------------- |
| **Sovereign** *(solo)* | everything                  | self-sufficient one-army; broad max investment, self-recovers, engages crowds               |
| **Bulwark**            | Body toughness + Speed      | the wall — **Holds**: **absorbs** (toughness) and **drags** Runners (Speed); little offense |
| **Vanguard**           | Power + Speed               | front-line killer; mutual-sub-phase duels and runs                                          |
| **Skirmisher**         | Speed + Precision           | Runner / assassin; slips to the enemy back line, exploits weak spots                        |
| **Channeler**          | elemental / ranged delivery | AoE and status from the back line; glass cannon, needs a wall                               |
| **Tactician**          | Mind                        | the stance, **recovery**, enabling allies' Precision, sealing enemy Minds                   |
| **Spiritualist**       | Spirit                      | the will-breaker — Resolve, Rally, Dread; fear and morale                                   |

**Synergy example:** the Bulwark walls the line so the Channeler nukes
safely; the Tactician recovers the team's options and shocks the enemy caster; the
Skirmisher runs their back line while the Vanguard focus-fires the front. Pull one
specialist and a whole job goes uncovered.

**The wall's coverage is toughness + Speed, not Mind.** A Bulwark's "coverage" is the
two physical lanes: **toughness-absorb** (blows it eats without predicting) and
**Speed-drag** (Runners it catches at the front line). It does **not** cover by
**predicting** — that is **Mind-predict**, the [Tactician's](#character-archetypes)
lane (a focus pool that negates blows by anticipating them). A *complete* wall therefore
wants **all three**: toughness to absorb, Speed to drag, and a Mind (its own or a
Tactician beside it) to predict; a pure-toughness Bulwark still falls to attacks that
bypass body — Fear, Confusion, or a foe it can't drag. Mixing the two coverages up
double-counts a job neither stat does alone.

**God-tier (Sovereign) failure modes.** Because the [budget is
linear](world-and-progression.md#god-vs-party--depth-for-breadth-at-equal-budget), a
solo god is only as broad as it pays to be, and skimping shows two ways:

- **Glass-cannon blur** — all Speed and Power, thin Mind and toughness. It strikes many
  but gets **ganked**: swarm past its one Mind and the overflow free-hits a body that
  can't absorb.
- **Serene genius** — all Mind, thin Speed and body. It predicts everything yet
  **strikes and drags too little to matter**, and a fast attacker drains its focus pool
  faster than it refills.

A **balanced god needs Speed + Mind + Power** (and toughness): Speed to swing and catch,
Mind to predict, Power to cross the thresholds (Juggernaut armor, the drop) that are its
whole reason to concentrate. "Speed swings, Mind reads, toughness endures" applies to one
body as much as to a party.

## Monster archetypes — one per mechanic you can't ignore

Each archetype **punishes neglecting a specific mechanic**, and each is **minded** (it
plays the [stance](decision-making.md)) or **mindless** (a predictable, fixed
[behavior deck](decision-making.md#environment-creatures--hazards-non-player)):

| Monster                                       | Forces you to use…                                                                 | Mind?                         |
| --------------------------------------------- | ---------------------------------------------------------------------------------- | ----------------------------- |
| **Juggernaut** — heavy armor, huge toughness  | the right **damage type** + Power (blunt / pierce, or you tickle it)               | mindless                      |
| **Swarm** — many weak, fast bodies            | **coverage** (bodies) and AoE / Speed, or be overrun and out-run                   | mindless                      |
| **Stalker** — fast, runs for your back line   | **Speed** (interception) or coverage, or your fragile die                          | mindless (priority targeting) |
| **Artillery** — back-line ranged nuker        | **front/back play** — run or interrupt it, or eat ranged death                     | mindless                      |
| **Sentinel** — armored but for one weak point | **Precision** (a Mind) — brute force bounces off                                   | mindless                      |
| **Trickster** — a cunning duelist             | **the RPS stance** — out-predict its weighted tendencies                           | **minded**                    |
| **Fire elemental** — burns                    | **non-metal defense** / heat resist; metal armor is a *liability*                  | mindless                      |
| **Frost elemental** — freezes                 | **tempo** — it Seals your cards and cuts Speed                                     | mindless                      |
| **Storm elemental** — shocks                  | **Mind resilience** — it Seals your tactics, killing stances *and* recovery        | mindless                      |
| **Reaver** — disarms and seals                | **Form depth** — backup capabilities, the re-equip maneuver                        | **minded**                    |
| **Howler** — a corporeal fear-beast           | the **Spirit** aspect — its howl is armor-proof **Fear**; only **Resolve** shields | mindless                      |
| **Attritionist** — drags the fight out        | **recovery** — run your Potential dry and you go predictable and helpless          | mindless                      |

*("Mindless" here means **no [theory of mind](decision-making.md#the-line-theory-of-mind)**
— instinct-driven, not unsophisticated. Such a creature can still reposition, ambush,
and react to the board with rich conditional behavior; it just can't model and counter
*you*.)*

## Why this matters for the Mind RPS

The roster splits by **theory of mind**: most creatures run on **instinct** — rich,
conditional behavior you can study and out-predict, but no model of *you* — while a few
(Tricksters, and other players or stand-ins) are **minds** that predict *back*. The Mind
RPS cycle must satisfy **both**: out-predicting instinct one-way, and dueling a thinking
opponent two-way. The **Sentinel** (Precision) and **Storm** (sealing Minds) also show
why the Mind aspect is load-bearing — whole archetypes exist to punish a character who
skimps on it.

## Open questions

- Exact **stat / deck profiles** for each archetype (numbers arrive with the combat
  pass).
- Which archetypes are **starter** threats vs late, "certain-doom" ones on the
  progression curve.
- The **Spirit** aspect's deeper cards (well underway — see [spirit](spirit.md)).
