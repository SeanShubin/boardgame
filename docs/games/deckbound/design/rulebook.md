# Deckbound — Tabletop Rulebook *(sample combat · first pass)*

A **player-facing** manual for the [sample combat](sample-round.md) — written to be read
and played by a human at a table. Eventually this is *generated* from the engine +
[keyword glossary](keywords.md); for now it's hand-written, as **pressure on
comprehensibility**. Numbers tagged *(appendix)* are first-pass and tunable. Guiding
hand: **simpler is better — but a solid metaphor lets a player carry more.**

---

## 1. The goal

You are a small party fighting a warband, **entirely with cards**. Win a fight by
**defeating every enemy while at least one of you is still standing** — then your whole
party is **restored to full** for the next fight. If your **last** standing hero falls,
the run is over.

## 2. The table

Lay cards out in nested areas — *where a card sits says what it affects*:

- Each **character** has four zones: **Form** (who you are: health + gear + stats),
  **Potential** (what you can still play), **Active** (in play now), **Dormant** (spent).
- Your side shares a **Party zone** (team effects live here).
- The two sides face off in a **front line** and a **back line** — your front shields
  your back.

**Health is cards.** Your Body is a little stack of plain cards; a hit **flips** them
face-down (to Dormant). Lose your last Body card and you're **down** for the fight.

## 3. A round

Everyone acts at once, in four beats:

1. **Form up** — set who's in your front and back line (this fight, it's fixed).
2. **Declare** (face-down, so it's hidden): your **stance** (Attack or Hold), your
   **target**, and your **read** (see §4). You see everyone's *positions* — but not their
   reads — before choosing.
3. **Reveal** — flip everything at once.
4. **Resolve** — §5, then tidy up: clear scratches, refresh, return your guards' reads.

## 4. The read — rock-paper-scissors of intent

Every clash, each side secretly picks one **read**:

> **Strike** beats **Scheme** · **Block/Evade** (defense) beats **Strike** · **Scheme**
> beats **defense**.

- **Strike** — attack now.
- **Block** — soak a strike (and **bank Power**).
- **Evade** — dodge and reposition (and **bank Speed**).
- **Scheme** — set up; if it lands, **bank a lot** — but a Strike spoils it.

Win a read and you **bank momentum** (Power/Speed/Precision cards) to spend later; **lose**
a read (your intent gets countered) and you **forfeit your whole bank**. Build with
defense and Scheme, cash out with a Strike — gambling the pile each time.

**You only read a foe you're engaging** (Holding, or trading attacks with them). Anyone
attacking you that you *can't* watch hits **free** — and you can only watch as many at
once as your **Mind** allows.

## 5. Resolving a clash — Speed times, Power weighs

Two stats, two jobs, no overlap:

- **Speed = who lands first.** Each round your Speed is a pool of **tempo** you spend
  acting; whoever has more tempo *left* in a clash **strikes first**. Equal tempo → both
  land (trades, even mutual kills, are real).
- **Power = how hard it lands** — magnitude only. It cracks armor and toughness, and a
  big enough blow **drops** the target.

You only **stop** a foe's blow by **dropping them first** (no swinging once felled) or
**out-reading them** (a defense beats their Strike, a Strike spoils their Scheme). Some
cards carry **stagger** — "land first and they lose their turn" — but that's a *card's*
trick, not something every blow does.

Then **damage**: a landed hit's strength meets the target's **armor** (by type) and
**toughness**, flipping Body cards down *(appendix: the formula)*.

## 6. The wall, reach, and running it

Your front line is a **shield wall**. Bodies must get *through* it; shots and spells fly
*over* it.

- **Melee** reaches only the **enemy in front of you** (the front lines clash).
- **Reach** is distance in "jumps": `front↔front = 1, front↔back = 2, back↔back = 3`.
  A **ranged** weapon or spell reaches one rank deeper — so a front-line archer hits the
  enemy **back line**, a back-line caster hits the enemy **front**. *Reach shoots over the
  wall.*
- A **Runner** sprints past the front line at your back line. The guards **don't chase —
  they drag.** Their **combined Speed** is subtracted from the Runner: cover its Speed and
  it's **stopped** (and struck); fall short and it gets through, **slowed** by what you
  spent; a *much* faster Runner twists past, barely trimmed. *(appendix: drag = the sum of
  the guards' Speeds.)*

So a back-line mage is safe from **swords** behind the wall — but not from enemy **arrows
or fear**; for those, kill or interrupt the shooter.

## 7. One attack, several foes

An attack lists how many **targets** it hits (distinct foes — no double-hitting); its
strength lands on each. A wide swing or a Firestorm hits several; against a swarm it
clears as many as it has **targets**, and the rest spill past. But **width forgoes
finesse** — one broad blow can't out-guess each foe; everyone reading you answers it
their own way. To out-think a *specific* foe, fight them one-on-one.

## 8. Fear, resolve, and morale

**Fear** is an attack on your **Resolve**, not your body — armor does nothing; only nerve
holds. If fear **beats** your Resolve you panic *(appendix: fear must exceed resolve)*. A
**Rally** raises the **whole party's** Resolve, and every Rally stacks with the others —
morale is built together.

## 9. Down, recovery, and the end

A downed hero is **out for the fight** — no mid-fight revival. Win with anyone standing
and **everyone comes back to full**; lose the last one and the run ends. So a fight is
all-or-nothing at the edges: survive with one hero up and you recover everything.

---

## Appendix *(first pass — all tunable)*

| Dial                 | Value (first pass)                                                                  |
| -------------------- | ----------------------------------------------------------------------------------- |
| **Toughness**        | cards flipped = ⌊ damage ÷ quantity ⌋; quantity is the card's "T"                   |
| **Damage**           | (Power *or* Mag, + banked Power) − armor[type], then ÷ toughness                    |
| **Read cycle**       | Strike > Scheme > Defense > Strike; mirrors settled by tempo then Power             |
| **Bank**             | Block → +1 Power · Evade → +1 Speed · Scheme → +1 Power +1 Speed +1 Precision       |
| **Cash**             | +1 Power onto a Strike · +1 Speed = an extra action · +1 Precision = ignore 1 armor |
| **Drag**             | front line's drag = **sum** of its guards' Speeds                                   |
| **Reach**            | melee `[1,1]`, ranged `[2,2]`; jumps f↔f 1, f↔b 2, b↔b 3                            |
| **Fear**             | panics if **fear > Resolve** (equal holds)                                          |
| **Multi-target win** | banks momentum **once**, not per target                                             |

## Card listing *(components)*

**Your party**

| Hero                | Stats                                             | Cards (plain language)                                                                                                                  |
| ------------------- | ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Aldric** · Knight | Spd 4 · Pow 5 · Body 8 (T2) · Resolve 4           | Plate (armor: heat gets through), Shield (Block + Bash), **Bash** (fast blunt strike, staggers), the four reads. *The wall — melee.*    |
| **Vera** · Duelist  | Spd 5 · Pow 3 · Body 4 (T1) · Resolve 2           | Blade (sharp), **Riposte** (Evade → counter → reposition, banks Speed), the four reads. *The reader — melee.*                           |
| **Sefa** · Mage     | Spd 2 · Mag 5 · Body 3 (T1) · Resolve 1 (fearful) | **Firestorm** (heat, hits 5 targets, once), **Frostbite** (cold, slows), the four reads. *Glass cannon — ranged [2,2].*                 |
| **Bram** · Warden   | Spd 3 · Spr 5 · Body 5 (T2) · Resolve 4           | **Rally** (party-wide Resolve), **Dread** (fear attack), **Steel** (recover your nerve), the four reads. *Spirit — ranged/inner [2,2].* |

**The warband**

| Foe          | Stats                                                           | Behavior                                                                           |
| ------------ | --------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| **Ironclad** | Spd 2 · Pow 6 · Body 8 (T3) · plate (heat gets through) · melee | *bluffs* (a hidden read deck); only heat cracks it, its blow is lethal unless read |
| **Stalker**  | Spd 6 · Pow 3 · Body 6 (T1) · Runner                            | runs the gauntlet for Sefa                                                         |
| **Howler**   | Spd 4 · Fear 5 · Body 4 (T1) · front-line caster, reach [2,2]   | howls at the least-resolute (Sefa) over the wall                                   |
| **Husk ×6**  | Spd 3 · Pow 1 · Body 1 · melee · swarm                          | shamble at the front                                                               |

---

## Design-pressure notes *(for us, not the player)*

Writing this surfaced where comprehensibility is tight:

- **The two-stat clash reads cleanly** because each has one job ("Speed = who's first,
  Power = how hard it hits"). Keep that phrasing — it's the load. Pre-emption is just
  "drop them first or out-read them," not a separate Power rule.
- **Drag** needs the *subtraction* metaphor ("the wall drags a Runner") to be graspable;
  without it, "combined Speed vs the Runner's Speed" is abstract. The metaphor carries it.
- **Reads + bandwidth + free hits** is the densest spot. It only stays manageable via the
  one-line rule "you read only whom you can watch; the rest hit free." If a reader trips
  here, this is the rule to simplify or illustrate.
- **Momentum bank/cash** is a second economy on top of the clash — the most "extra" thing
  to hold in your head. Watch whether it earns its complexity in play; it may want the
  most simplification.
- Everything else rides comfortably on a metaphor (wall, gauntlet, reach-as-jumps, fear
  vs nerve, health-as-cards).
