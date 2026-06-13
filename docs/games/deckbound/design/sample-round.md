# Deckbound — Sample Combat (four aspects, one fight)

A scenario built to **exercise every core mechanic** and to make the four heroes
**need each other.** Each enemy is a *lock* only one aspect's *key* opens, and they
arrive **together** — so the party must split correctly or fall. No incorporeal, no edge
cases: just Body, Mind, Magic, Spirit, the [tempo](speed-and-tempo.md) loop, and
[coordination](coordination-and-interruption.md).

Zone shorthand: `Strike: P→A→D` (aggressive → exhausts); `Evade: P→A→P` (defensive →
self-returns); `Firestorm: P→A→D` (Fleeting); `+Spd →A` (momentum banked).

## The party — one lean per aspect

| Hero                | Spd · Pow · (Mag/Spr) · **Resolve** | Body×T | Aspect & job                                                                                                                                         |
| ------------------- | ----------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Aldric** (Knight) | 4 · **5** · — · R4                  | 8×T2   | **Body** — Plate (sharp −3, blunt −1, **heat −0**), Shield/**Bash**. The wall; **catches** the Runner (drag) and **strikes** it — Bash **staggers**. |
| **Vera** (Duelist)  | **5** · 3 · — · R2                  | 4×T1   | **Mind** — Blade, **Riposte**. High reads: **out-duels** the Ironclad and **bleeds** the Runner's tempo.                                             |
| **Sefa** (Mage)     | 2 · 1 · **Mag 5** · R1              | 3×T1   | **Magic** — **Firestorm** (heat, 5 targets), Frostbite (cold). The *only* one who cracks the Ironclad; **fearful**.                                  |
| **Bram** (Warden)   | 3 · 2 · **Spr 5** · R4              | 5×T2   | **Spirit** — **Rally**, **Dread**, Steel. **Shields Sefa's nerve** and breaks enemy morale.                                                          |

**Reach:** Aldric & Vera fight **melee `[1,1]`**; Sefa & Bram are **ranged / inner
`[2,2]`**, reaching the enemy front from the back.

## The warband — four locks, four keys

| Creature     | Spd · Pow · (special)                               | Body×T | Decides via                                            | The lock it is                                                                                                            |
| ------------ | --------------------------------------------------- | ------ | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| **Ironclad** | 2 · **6** · plate (sharp −4, blunt −3, **heat −0**) | 8×T3   | **behavior deck** (Strike / Feint — it *bluffs*)       | **Magic + Mind.** Blades & blunt bounce — **only heat** cracks it; and its Pow 6 is lethal unless its attack is **read**. |
| **Stalker**  | **6** · 3                                           | 6×T1   | **line:** run the lowest-Body (Sefa)                   | **Body + tempo.** Too fast for one guard — needs the **gauntlet** (bleed + catch).                                        |
| **Howler**   | 4 · **Fear 5**                                      | 4×T1   | **line:** howl at the least-resolute; fearless → cower | **Spirit.** Its howl is **armor-proof Fear** — the wall can't help; only **resolve** does.                                |
| **Husk ×6**  | 3 · 1                                               | 1×T1   | **swarm line:** shamble at the front                   | **AoE.** A single blade kills one of six; **fire clears a pack.**                                                         |

**Reach:** the Ironclad & Husks **melee `[1,1]`** your front; the **Howler** is a
**front-line fear-caster at `[2,2]`** (reaching Sefa in your back); the **Stalker** is a
**Runner** — it crosses the gauntlet, not reach.

Note the **three representation kinds** in one warband: a **deck** foe (the bluffing
Ironclad — it [earns a deck](physical-representation.md#packing-a-creature-onto-one-card)),
two **line** foes (Stalker, Howler), and a **swarm** (Husks = one card + a count).

## Form up & declare

- **Players** — Front: Aldric, Vera. Back: Sefa, Bram.
- **Creatures** — Front (pressing): **Ironclad + Husk ×6**, plus the **Howler** — a
  front-line **fear-caster** at reach `[2,2]`, so its howl reaches Sefa in your back.
  **Running the gauntlet:** the Stalker, for Sefa.
- **The coordinated plan:** **Aldric & Vera Hold** the front; **Sefa casts Firestorm**;
  **Bram Rallies the party** (crucially Sefa).

## How it resolves — reveal simultaneous, clashes by tempo

**Body + tempo — the gauntlet.** The heroes' front line pools its tempo as **drag:
Aldric 4 + Vera 5 = 9.** The **Stalker** (Speed 6) runs — **9 ≥ 6**, so the line
**stops it.** The defenders put **Aldric** on it; his **Bash** (`P→A`)
**lands** — `Bash → Body ×5: Form→D` leaves the Stalker at **Body 1**, halted before
Sefa (the drag already stopped the run; Bash also carries **stagger**). *No sequence — the line's combined tempo simply out-drags a Speed-6 runner, and the
party spends its tanky, high-Power body to make the stop.*

**Mind — the read.** The **Ironclad** presses, its committed choice **hidden in its
deck** (it Strikes). **Vera reads it** and plays **Riposte** (`Evade: P→A→P`): Evade
beats Strike → its **Pow 6 is negated**, she takes position and **banks +Speed** (`+Spd
→A`). *Only Mind reliably beats it — a wrong read eats six Power and the front folds.*

**Magic — armor means nothing to heat.** **Sefa's Firestorm** (`P→A→D`, heat) strikes
**5 targets** across the front. The Ironclad's plate is **heat −0**, so it finally takes
a wound — `Mag 5 ÷ T3 = 1` → **8 → 7** (blade and mace bounced; only heat gets through).
The other four targets are **Husks** (Body 1) → **burned**; the **two Husks outside the
five overflow** (count **6 → 2**). *Heat cracks the lock; breadth guts the pack — but
only as many as it has targets.*

**Spirit — resolve over fear.** From the enemy front line the **Howler** howls **Fear 5**
at Sefa (reach `[2,2]`, over the wall — armor-proof). Her own Resolve is 1 — but **Bram's
Rally** (a **party-wide** lift) has raised hers to **5**: Fear 5 vs Resolve 5 → it
**washes over her**, and she casts unshaken. *Armor never entered into it; only resolve
held.*

**Round settles.** Bash and Firestorm exhaust to **Dormant**; Vera's **Riposte
self-returns** to Potential; her **+Spd** stays Active; **Rally** stays in the party
zone.

## End of round

|               | Outcome                                                                                                                                                                                           |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Heroes**    | **None down.** Vera banked +Spd; Aldric stopped the run (exposed, unhurt); Sefa cast safely.                                                                                                      |
| **Creatures** | **4 Husks burned** (2 left). **Stalker** stopped at Body 1. **Ironclad** 8 → 7 — *cracked, but standing*: it's a multi-round kill only Sefa can finish. **Howler** still up, held off by resolve. |

A clean, **coordinated** round. The Ironclad is the **win condition** — Sefa burns it
down over several rounds while the front holds the line and Bram keeps her nerve.

## Why all four are necessary — the cascade

Pull any one hero and the lynchpin (**Sefa**, the only Ironclad-killer) dies or fails,
and the Ironclad's Pow 6 grinds the party to a **wipe**:

- **No Aldric** → the front-line drag drops below the Stalker's 6 → it gets through and **kills Sefa.**
- **No Vera** → Aldric's drag alone (4) is **< the Stalker's 6** → it gets through; *and*
  the Ironclad's Strike goes unread → **six Power lands** on the front.
- **No Sefa** → nothing cracks the Ironclad's plate → it **never dies.**
- **No Bram** → the Howler breaks Sefa's nerve → **no Firestorm** → the Ironclad lives.

Four simultaneous threats, four keys, four heroes. **You cannot solve it in sequence —
only together.**

## Core mechanics this exercises

- **Tempo gauntlet** (combined drag stops the Runner): Stalker vs Vera + Aldric, pooled `9 ≥ 6`.
- **Charge + stagger**: combined drag stops the Stalker (9 ≥ 6); Aldric's Bash bloodies and **staggers** it.
- **Reads / RPS + momentum**: Vera Evades the Ironclad's hidden Strike, banks +Speed.
- **Typed damage vs armor**: only heat passes the Ironclad's plate; blade/blunt bounce.
- **Targets / breadth**: Firestorm hits **5 targets** — four Husks burn, two overflow (swarm = one card + a count).
- **Fear vs resolve**: the Howler's armor-proof Fear, answered by Rally.
- **Engagement bandwidth**: Vera holds two jobs at once (bleed the Runner, read the boss).
- **Coordination**: the cascade above — the whole point.

## Representable as cards in zones

Heroes are **play-mats** (Form / Potential / Active / Dormant) with a tempo track;
creatures are **one stat-block card each** — the Ironclad carrying a small **behavior
deck**, the Stalker and Howler a printed **line**, the Husks **one card + a count**;
Rally lives in the shared **party zone**; damage and tempo are **derivable numbers**.
Nothing here needs more than the [representation toolkit](physical-representation.md)
already established. (Worked card-by-card play exists for the prior draft; this scenario
maps the same way.)

## Open questions

- **Numbers** — the read-cycle outcomes, the exact heat/armor and Fear/Resolve values,
  and how many rounds the Ironclad should take to fall.
- **The Ironclad's deck** — how many options it bluffs among, and how Vera's Mind tilts
  the read (see [speed & tempo](speed-and-tempo.md) and [the Mind](mind-and-reads.md)).
- Whether **Bram** should also **Dread** the Howler down over rounds, or leave it as a
  standing pressure that forces a sustained Rally.
