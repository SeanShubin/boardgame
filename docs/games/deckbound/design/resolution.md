# Deckbound — Resolution Procedure (the round)

> **SUPERSEDED — Clash mechanics.** The tactical core is now the **Clash**
> ([`spec/README.md` §1.0](../spec/README.md#10-the-clash--beats-six-moves-charges),
> rationale in [the-duel.md](the-duel.md)): a beat-by-beat duel of six moves
> (**Strike · Throw · Parry · Evade · Charge · Recover**), durable ×2 **Charge**
> cards instead of momentum banks, run to **Body 0**. The Strike/Scheme/Defense
> cycle, the momentum bank/forfeit, and "a duel ends on a single strike" below are
> stale. The **round skeleton** here — form up → declare → reveal simultaneously →
> Clash → recover, with the gauntlet/Charge phase and minimal inherent ordering —
> still holds in spirit; resolution order is canonically specced in §1.9. Read for
> the procedure's shape; trust the spec for what a clash resolves to.

The deterministic algorithm the engine runs each round — the backbone of the
[rulebook](engine-architecture.md). Numbers (drag aggregate, damage formula, momentum
conversions, thresholds) live in the **appendix**; this is the *procedure*.

## The round, end to end

1. **Form up.** Each side arranges **front / back lines**, revealed and fixed for the
   round. (In a fixed scenario this is authored.)
2. **Declare** — every actor, with formations open:
   - a **stance** (Attack / Hold),
   - **target(s)** legal under [reach + targeting](coordination-and-interruption.md),
   - one **stance** (Strike / Block / Evade / Scheme), committed **face-down**,
   - any **plays** (action cards), committed face-down.
   Creatures declare from their behavior: a **line** resolves deterministically off the
   visible board; a **deck** supplies a **shuffled, hidden** stance (the bluff).
3. **Reveal.** All stances and plays flip **simultaneously**.
4. **Clash** — the meat: **Charge** (the gauntlet) then **Exchange** (the blows), below.
5. **Recover.** Clear partial (sub-threshold) damage; **refill tempo** to Speed;
   self-returning stances (Block/Evade/Scheme) return to hand; aggressive/Fleeting turned face down;
   drop **Exposed** markers; **check win/loss**.

## Clash (step 4) — Charge, then Exchange

Each actor's **tempo** = its Speed for the round. The Clash has **two sub-steps**, and
their order is the *one* sequence that is inherent: a Runner must face the wall before it
can reach the back line.

### a. Charge — the gauntlet

For each lane being run, the front line's **drag pool** = the appendix's aggregate of its
Guards' Speeds (first-pass: **sum**). Defenders **allocate** drag across the Runners:

- **drag ≥ a Runner's Speed → stopped** — halted at the wall; an engaging Guard then
  **strikes** it (its **Power** is the damage; a **[stagger](keywords.md)** card also
  cancels its action),
- **drag < its Speed → slowed** — it passes with **leftover tempo = Speed − drag**,
- **no drag → through at full Speed**.

This fixes **who reaches the back line** before the Exchange.

### b. Exchange — the blows, settled at once

Everything else resolves **simultaneously**, governed by a **single priority rule** — the
only ordering inside the Exchange:

> **A pending blow is cancelled if its actor is *dropped or staggered* by a faster blow
> (more leftover tempo) before it would land.** Equal tempo → **both land** (trades and
> mutual kills are real). Nothing else sequences.

So pre-emption needs **no Power threshold**: **Power is pure magnitude**, and *dropping*
the target (a lethal first-strike) is what stops their blow — as is a **stagger** keyword.
Within that rule:

- **Pairwise clashes.** Each engagement resolves the stance cycle
  (`Strike → Scheme → Defense → Strike`) **per pair**. A multi-target attack commits **one**
  stance against each engaged target pairwise; a target **not predicting the attacker** (it can't
  afford the stance) **auto-takes** it. Stances are **bandwidth-limited by
  [Mind](mind-and-stances.md)** — you defend only as many attackers as your Mind affords.
- **Damage.** A landed attack's **magnitude** (Power / Precision / type vs armor +
  toughness — appendix formula) turns capability cards **face down in Form**. **Fear** erodes
  **Resolve** the same way.
- **Momentum.** A won stance banks Power / Speed / Precision into **Active**; a misjudged stance
  **forfeits the bank**. A multi-target win banks at the appendix **cap** (so breadth
  doesn't out-snowball depth).
- **Knockout.** Any actor whose **keystone** (usually Body) loses its last card is
  **knocked out** — out for the rest of the combat.

## Win / loss

- **Every enemy down with ≥ 1 hero standing → win** → the party
  [resets to full](form-and-defeat.md#knockout-recovery-and-the-wipe).
- **The last standing hero falls → loss** → the run ends.

## Why this minimal ordering

Order is imposed only where it is **inherent**; everywhere else the Exchange is
simultaneous, so the engine can settle pairs independently:

- **Charge before Exchange** — a Runner faces the wall before it can strike past it.
- **Pre-emption within the Exchange** — a felled (or staggered) fighter can't swing. The
  priority rule captures this *without* a global turn order: rank pending blows by leftover
  tempo only to decide **who fells whom first**, then resolve everything at once.

What the build still has to fix is **numbers and one interaction**, not the shape:

- **Drag allocation** — how the defenders' choice of which Runner to drag is offered and
  recorded (an interaction; deterministic once chosen).
- **Tie-breaks** past "equal tempo → both land" — e.g. mutual lethal first-strikes (a
  mutual kill by default).
- **Casting / Scheme tempo cost (RESOLVED).** A cast is a **physical action**, so it
  **draws tempo** like any action; the only open part is the Scheme's cost (the
  [speed-and-tempo](speed-and-tempo.md) question).
