# Regions and relations: a formation that survives round one

**Status:** exploration, staged for merge. Author: one instance, 2026-07-13.
**Question:** the three intentions are right, but the rank metaphor dies after the opening
clash. Can we keep the intentions, keep the phase structure, keep the solver, and get a
formation that means something in round 4?

**Answer this document argues:** yes, and the move is to stop treating *rank* as a slot you
re-audition for and start treating it as a *relation* you hold. The five sub-phases are not a
round clock -- they are a **crossing gauntlet**. Fire them on movement instead of on the round,
charge for movement, and the metaphor holds at round 4 the same way it holds at round 1. The
schedule gets **shorter** (four sub-phases, not five), not longer.

---

## 0. MEASURED (2026-07-13) -- `examples/v2_regions.rs`

The design has been **built headlessly and probed**. Run it:
`cargo run --release -p deckbound-board --example v2_regions`

### The verdict: movement is LOAD-BEARING

`v2_remarshal` asked whether a mid-fight re-rank is ever *required* to win, against the honest
control (the **best fixed formation**). The answer was no, exhaustively -- so re-Marshalling was
cut as decoration. `v2_regions` re-asks the identical question, with the identical control, with
a **priced** move:

|                        | winnable from the best fixed setup? | winnable only by moving?                                 |
| ---------------------- | ----------------------------------- | -------------------------------------------------------- |
| 4 **solo** encounters  | all 4                               | **0**                                                    |
| 4 **party** encounters | 1                                   | **3** -- Greywater Ford, Emberfall Hollow, Ninefold Deep |

**Three of the four party encounters are unwinnable from every fixed setup, and winnable by
moving.** That is the result `v2_remarshal` could not find with a *free* move. Pricing the move
created a decision that did not previously exist.

And the pattern is exactly right: **movement is worthless in a solo and decisive in a party
fight.** Complexity scales with the number of bodies -- which is §4.1 count-adaptivity, arriving
on its own rather than being stipulated.

### The tractability fear (§9) was unfounded

|                         | measured       |
| ----------------------- | -------------- |
| nodes, all 8 encounters | **836**        |
| worst single memo       | **577 states** |
| wall clock, all 8       | **~110 ms**    |

Against the **24x** baseline `v2_remarshal` measured for a per-round `3^heroes` re-declaration,
this is nothing. Three things did it, and all three were predicted in §9:
1. **Canonicalized partitions** (region *labels* never enter the key).
2. **Count-adaptive aims** (§4.1, already canon).
3. **Short-circuit on the first win** -- it is a reachability search, not a minimax.

The in-app doom oracle can afford this comfortably. **§9.4's "single thing to prototype first"
is answered: it passes.**

### The per-move verdict table works, and it discriminates

The probe emits exactly the chart the UI needs. From Greywater Ford's opening board:

```
Raider:
    Press The Wall         Doomed
    Press The Duelist      Doomed
    Press The Swarm        Doomed
    Defend Marksman        Winnable
    Defend Bastion         Winnable
    Defend Bombardier      Winnable
    Withdraw               Winnable
```

**The Raider must not charge in round one.** Every attack it can make loses the fight; every
defensive posture keeps it alive. The party's only real damage (Might 7) dies in the crossing
gauntlet and is never there for the fight it was needed for. That is a legible, teachable,
*positional* decision -- and the current model **cannot express it at all.**

### Three things the probe changed about the design

1. **The move follows from the aim.** A unit declares **one** thing, not two: `Press(enemy)` /
   `Defend(ally)` / `Withdraw`, and where it ends up falls out (melee crosses to its quarry;
   ranged stays and shoots; Defend moves to its ward; Withdraw peels off alone). This collapses
   §5.2's move+aim into a single declaration and is a large branching win.
2. **Defend is a damage REDIRECT, not a separate contest.** A blow aimed at W lands on W's living
   defender instead, and that chains. So the screen *is* the bodyguard rule, and it *is* the
   back-access rule -- one mechanic, no gate, no immunity. Kill the screen and the blow lands.
   This is simpler than §5.3/§5.4 and strictly better. Cycles are handled and merely expensive.
3. **The pre-empt is a snap shot, not a round.** The first build let everyone pour their whole
   pool into `Cross`, and the fight resolved there with three dead sub-phases behind it. You
   cannot stand in the open whaling on a body that is *running past you*. **Cross and Arrive give
   one blow** (the one the reach already paid for); **Contact and Breach pour.** That is what
   keeps the four sub-phases from collapsing into one.

### Region-AoE (your ask) -- it works, and it needs a brake

Region-based AoE is a real upgrade and it falls out for free: a sweep hits **every enemy in the
target's region**, is unevadable, and **bypasses the screen** (a bodyguard soaks an aimed blow but
cannot cover an area). That last clause is the anti-cluster counter, and it is what *prices* the
whole Defend mechanic -- pile bodies behind a screen and you become a **target**. Concentration and
coverage now genuinely trade off, decided by a positional fact the player controls.

**But it needs a brake.** The first build carried the product's existing "one sweep clears the
whole pack" horde rule into the region. Region-wide, that is absurd: the Bombardier's Salvo deleted
*both* Swarms -- sixteen bodies -- for one tempo card, and **seven of eight encounters collapsed to
a round-one wipe.** The fix: a horde takes a sweep like anything else (penetrating Might spills body
to body). **The brake belongs in the horde rule, not in the region rule.**

### What the probe did not test (honest)

- **Tempo allocation is held at greedy** for both sides; only the *aim* layer is searched
  exhaustively. This is deliberate (it compares like with like, and it is what makes the probe
  finish), but it means a **"no" would have been evidence, not proof**. A **"yes" is proof**, and
  yes is what came back.
- **Support is omitted** (no buffs), so the branching numbers above are for a 3-aim model.
- The **all-Defend mutual turtle** is a legal winning line in some encounters. It should be checked
  that it is not *dominant* -- it is currently only hidden from the transcript by enumeration order.

---

## 1. What is actually true today (the spec and the product disagree)

Before redesigning, note that the two sources of truth have drifted apart on precisely the
three axes this redesign touches. This is worth fixing regardless of what we decide.

|                                       | Spec (`canon/2-spec` §4, §4.6)                                                           | Product (`deckbound-board`)                                                                                                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **When is the formation declared?**   | Re-declared **every round** (§4 step 1, "Marshal (hidden, simultaneous -- every round)") | **Once.** `arena.rs:1145`: *"Marshal happens ONCE. Its card is discarded, not rotated"* -- the round wraps Breach -> Intercept with no formation step                       |
| **The attack contest**                | A **single simultaneous blind bid**; defender must strictly beat it                      | **Three one-way mini-phases**: Engage (attacker commits) -> Evade (target *sees the commitment* and pays the exact `slip_cost`, or stands) -> Strike. No blind bid anywhere |
| **When does unapplied damage clear?** | At **every sub-phase** boundary (§4.6 "every pile wipes at its own sub-phase boundary")  | At the **Round Reset**. `combat.rs:447`: the sub-phase boundary *"is where the dead stop fighting, not where wounds close"*                                                 |
| **Groups**                            | Full system (§4.5): sum-to-block, min-to-slip, spillover, bodyguarding                   | Not built. Only `horde` exists; *"heroes are ungrouped in the UI"*                                                                                                          |
| **Armor**                             | Deferred to gear (§2.2: "there is no cut today")                                         | **Built** -- a per-strike floor                                                                                                                                             |

Two of those product decisions are *better than the spec* and this design keeps them. One of
them is the whole problem.

### 1.1 The load-bearing finding: re-Marshalling was deleted because it was worthless

`examples/v2_remarshal.rs` asked whether a mid-fight re-rank is ever *required* to win, against
the honest control (the best fixed formation, not a bad one). The answer was no -- across every
kit and the full party against all eight encounters, exhaustively. It cost 24x to model and
bought nothing, so it was cut, and the fixed-formation solver became correct by construction.

**That result is sound, and it is the strongest argument for this redesign.** Read it carefully:

> re-ranking is worthless **because re-ranking is free**.

A repositioning that costs nothing and is available every round can *always* be pre-empted by
simply starting in the right place. It is therefore never *necessary* -- which is exactly what
the probe measured. The probe did not discover that position doesn't matter. It discovered that
**costless** position doesn't matter. Any free teleport, offered every round, is decoration.

So the product froze the formation. And that is where the fiction died.

---

## 2. The diagnosis, precisely

**The five sub-phases are a transit gauntlet being run as a round clock.**

Intercept / Volley / Raid / Clash / Breach describe exactly one situation: bodies moving through
open ground. The front screens the *crossers*. The back fires on the *crossers*, before they
*arrive*. The survivors *land*. The lines *meet*. The deep blows land *last*.

In round 1 that picture is perfect -- because in round 1 everyone genuinely is in transit. The two
lines are closing, the gap is real, and every beat of the schedule is about the gap.

In round 3, nobody is in transit. And the game runs the gauntlet anyway.

Both models break here, in mirror-image ways:

- **The spec's model** re-rolls positions every round without anyone moving. A rank is a role you
  re-audition for, not a place you are. There is no continuity of place, so the crossing fiction
  has nothing to cross. The formation is "all over the place" because it is re-dealt every round.
- **The product's model** freezes positions forever and *still* runs the crossing every round. Your
  Outrider raids the back in round 1, and then in round 2 it raids the back again -- from where? It
  never came home. The fiction is a loop of identical charges by bodies that never move.

The gap between "the formation" and "what the schedule narrates" is the entire complaint, and it
is structural. **Neither model has any transit after round 1, but the schedule is made of nothing
but transit.**

---

## 3. The razor (take it from the product, and cut harder)

The product already found the right principle and wrote it down at `combat.rs:447`:

> The sub-phase boundary **is where the dead stop fighting, not where wounds close.**
> The Round Reset is **the one deadline in a fight.**

Generalize it into the rule that should govern every future timing question:

> **A sub-phase exists for exactly one reason: so that a death inside it can silence something
> later.** If two effects should *trade* (both land), they go in the *same* sub-phase. If one
> death should *silence* the other, they go in *ordered* sub-phases. Nothing else earns a
> boundary. The **round** is the only deadline: tempo stands up, unapplied damage closes.

This is already §4.6's PRINCIPLE, but stated this way it is a *razor*, and the first thing it
cuts is in the current schedule:

**Intercept and Volley should be one sub-phase.** They silence the *same* thing (the Raid). The
Vanguard's screen does not need to kill the Outrider *before* the Rearguard shoots it -- they only
both need to happen before it lands. Under the razor, two effects that need to precede the same
thing but not each other **should trade in one pile**. Merging them is strictly better: the
screen's damage and the volley's damage *combine* against the crosser (focus fire on the runner,
which is what the fiction wants anyway), and the schedule loses a step for free.

That cut is available *today*, independent of everything below.

---

## 4. The reframe: the three intentions are three relations

You said the intentions are right: support from the rear, hold the line, press the enemy's
support. Agreed -- keep all three. The problem is that they are currently **slots in a two-tier
ladder**, so the only geometry the game can express is "front" and "behind the front."

Make each one a **directed edge** instead:

| Today's rank                       | What it actually asserts              | The edge                                |
| ---------------------------------- | ------------------------------------- | --------------------------------------- |
| **Vanguard** (hold the line)       | my body is between you and them       | **Defend(ally)**                        |
| **Rearguard** (deal from the back) | I act from behind someone else's body | **Support(ally)**, and *being Defended* |
| **Outrider** (break the line)      | I am coming for *that specific one*   | **Press(enemy)**                        |

Now "the back is shielded while the front lives" stops being a global rule about ranks and becomes
a fact you can point at: *the Marksman is shielded because the Bastion declared Defend(Marksman),
and the Bastion is alive.* Kill the Bastion and the link dies with it. Rout it and the link snaps.

And crucially: **the link persists across rounds unless somebody spends something to change it.**
That is the continuity both current models lack.

Three things fall out immediately:

- **The current model is the special case where the defend-graph is a two-level star** (every
  Vanguard defends every Rearguard). We are not replacing the formation; we are letting it have
  more than two layers.
- **"Assassinate" is Press aimed at a *screened* target.** That is what makes it an assassination
  rather than a swing -- the target is behind bodies. The Outrider is *"a Press whose target sits at
  depth 1."* The role is preserved and re-derived rather than declared.
- **Depth becomes gradeable.** If Bastion defends Marksman and Raider defends Bastion, reaching the
  Marksman costs you two screens. A deep formation is a long chain -- and it *pays* for itself,
  because every body spent screening is a body not attacking. That trade is the game.

---

## 5. The model

### 5.1 Regions come from togetherness, not from a map

Space is a **partition of the bodies on the field**. Two characters in the same region are
**together** (in contact -- melee reaches, area strikes catch both, spillover applies). Characters
in different regions are **apart**.

There is no authored map and no coordinate system. Regions are **derived** from what people
declare, not named in the rules. The letters (A, B, C) are a **UI affordance for pointing at a
place on the table -- they must never enter the game state.** (This is not fussiness; §9 shows it
is worth a large constant factor to the solver.)

Consequences, all of them free:

- **A region containing both sides is a melee.** A region containing one side is safe ground.
- **"The back" is not a rank -- it is any region with no enemy in it.** A Marksman is "in the back"
  exactly when no enemy shares its region. The Rearguard's safety becomes *emergent*, which is what
  force-not-fiat has always wanted.
- **Grouping and positioning become the same mechanic.** §4.5's whole group system stops being a
  thing bolted on top of ranks and becomes *what a region is*. (It is unbuilt in the product today,
  so this is a merge, not a migration.)

### 5.2 Each body declares one move and one aim

Hidden, simultaneous, at the Marshal -- which now happens **every round again**, because it is no
longer free.

**Move:** `Hold` (stay), `Join(region)` (go be with someone), or `Peel` (leave alone).
**Aim:** exactly one of
- **Defend(ally)** -- my body is in the way of yours.
- **Support(ally)** -- my buffs/heals/braces land on you.
- **Press(enemy)** -- my violence is aimed at you, and I will cross to reach you.

One aim, not several -- this preserves today's "each intention buys one thing at the price of
another," and it keeps the branching factor sane (§9).

A **Defender is not passive**: it still strikes whoever is in contact with it. Defend does not
mean "do nothing but block"; it means "my body is between you and my ward, and I will hit whatever
comes for it."

### 5.3 The screen is the only spatial rule

> **To land anything on X -- a blow or a shot -- you must first get past everyone who Defends X.**

That is the whole of geometry. Each screen is a Tempo contest (the product's Engage/Evade
threshold, unchanged). And it gives us the melee/ranged distinction **derived rather than
decreed**:

- **Ranged** fires from where it stands: no crossing, so no departure blows and no arrival. But it
  still pays past the screen -- *a body in the way stops an arrow too.* (This is what keeps `R->R`
  gated behind a living front, and now it is motivated instead of stipulated.)
- **Melee** must be co-located to strike. So it must **cross** -- and crossing is the gauntlet.

That single difference replaces the spec's whole targeting matrix.

### 5.4 Defend has two faces, from one physical fact

The same edge does two jobs depending on whether the enemy is *outside* or *inside*:

- **Across regions (on entry):** you must beat me to come in. This is the screen.
- **Within a region:** single-target damage aimed at my ward hits **me** first. This is §4.5's
  spillover / bodyguarding.

And that is what makes **breaching persistent and visible**: the screen is paid *on entry, not
maintained*. Once an enemy is inside your region, it is **past** the screen -- it is in the room
with you. Your Bastion can still bodyguard (soak), but it can no longer *screen* (deny). Your
formation has been penetrated, and that fact **sits on the table** until you spend real actions to
fix it, instead of evaporating at the round boundary.

That is the "much stronger metaphor for rounds beyond the first."

---

## 6. The one law that generates the schedule

Everything below is a corollary of a single sentence, so nobody has to memorize the order:

> **Ground you cross is ground you cross unscreened -- including your own screen.**

The moment you move, you are outside everybody's protection: you turn your back on whoever you
were fighting, you walk into whoever stands in the way, and the people you are coming for watch you
come. Only then do you arrive.

The Outrider's "exposed both ways" is no longer a special property of a role. It is what happens to
**anyone** who crosses -- and the Outrider is simply the body built to survive it.

---

## 7. The schedule: four sub-phases, each earning its boundary

Apply the razor from §3 -- *a sub-phase exists only so a death in it can silence something later* --
and ask of each candidate boundary: **what does a death here silence?**

| Sub-phase      | What happens                                                                                                                                                                                                                                               | A death here silences...                                                                                                                     | Was                                 |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| **1. Cross**   | Everything that punishes movement, all trading in one pile: **parting blows** from the region you are leaving; the **screen contest** with everyone defending your destination; **fire** from the destination's occupants, who see you coming in the open. | ...the crosser's **Arrival**. It never lands.                                                                                                | Intercept **+** Volley (merged, §3) |
| **2. Arrive**  | Survivors of the crossing land in their destination and strike.                                                                                                                                                                                            | ...the victim's **Contact** action. *The raider kills the cannon before it fires.* This is the Outrider's entire purpose, preserved exactly. | Raid                                |
| **3. Contact** | Everyone now co-located with an enemy trades: the arrivals, and everyone who never moved.                                                                                                                                                                  | ...a screener, which **opens the ground behind it**.                                                                                         | Clash                               |
| **4. Breach**  | The blows that were waiting on ground that just opened: a screen chain whose screeners died in 1-3 is now open, and anyone pressing a target behind it **gets through free** -- no crossing cost, because the line broke.                                  | (nothing -- it is the last)                                                                                                                  | Breach                              |

**Four, down from five.** Every one of them is justified by a *named silencing*, and no boundary
exists for any other reason.

Rename note: the old sub-phase **Clash** collides with the §1.0 **Clash** module (the four-card
duel). **Contact** fixes that collision for free.

### 7.1 It reproduces the current model exactly

This is the check that matters. Take the *old* formation shape (Vanguards defending Rearguards,
Outriders pressing Rearguards) in *round 1* (everyone apart, everyone closing):

| New schedule                                                                                                                                                                                                                         | Reduces to                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------- |
| **Cross** -- nobody is in contact yet, so no parting blows. Enemy Vanguards defend their Rearguard, so my Outrider pressing that Rearguard must beat them; the Rearguard, being the destination's occupant, fires on it as it comes. | **Intercept + Volley**, exactly |
| **Arrive** -- my Outrider lands and strikes the Rearguard.                                                                                                                                                                           | **Raid**, exactly               |
| **Contact** -- the fronts are co-located and trade; the Rearguards fire (no crossing) at the front.                                                                                                                                  | **Clash**, exactly              |
| **Breach** -- the front fell, so `V->R` and `R->R` open.                                                                                                                                                                             | **Breach**, exactly             |

The new model *is* the old model in round 1. It is a strict generalization: same fiction, same
sub-phases, same opportunity costs (the Rearguard that spends its shot on the crosser in **Cross**
still has nothing left for the wall in **Contact** -- the glass-cannon dilemma survives untouched).
And unlike the old one, **it keeps working in round 4.**

### 7.2 The round

`Marshal` (hidden, every round -- it is no longer free) -> `Reveal` -> `Ready` (Support auto-lands)
-> `Engage` (Cross -> Arrive -> Contact -> Breach) -> **`Reset`**.

### 7.3 When unapplied damage clears

Keep the product's rule, unchanged, and let it be the *only* deadline:

- **Sub-phase boundary:** deaths finalize. **Nothing else.** A committed blow still lands even if
  its striker died in the same sub-phase (order-free, commit-based batch -- preserved).
- **Round Reset:** tempo stands back up, and **the damage pile closes**. A wound you could not
  finish this round is a wound you did not inflict.

So Grit demands concentration **within a round** -- across four sub-phases, a grain a player can
actually plan at -- and there is exactly one place in the whole game where a number is discarded.
Two levels, two rules, no exceptions.

---

## 8. Round 2, which is the entire point

Party: Raider, Bastion, Marksman, Bombardier. Foes: Ogre, Shaman, two Wolves.

**Round 1.** Heroes in one region, foes in another; everyone apart. Bastion holds, Defend(Marksman).
Marksman holds, Press(Shaman) -- ranged, so no crossing. Raider crosses, Press(Shaman). Ogre crosses,
Press(Bastion). Wolves cross, Press(Marksman).

- **Cross.** Nobody is in contact yet, so no parting blows. The Wolves are coming for the Marksman,
  who is Defended -- so the Bastion contests them; one Wolf is halted, one gets through. The Marksman
  and Bombardier fire on the incoming crossers -- and *that fire is the shot they no longer have for
  the Shaman.* The Raider crosses to the Shaman unscreened (the Shaman declared no Defender, and the
  Ogre and Wolves have left), but the Shaman shoots at it on the way in.
- **Arrive.** Raider lands and strikes the Shaman. Wolf lands and strikes the Marksman. Ogre lands
  and strikes the Bastion.
- **Contact.** The two regions are now melees. Bastion trades with Ogre, etc.
- **Breach.** Nothing opened yet.

**Round 2 -- and here is where the current game falls apart and this one does not.**

The board now says something true and legible: *the enemy is inside your formation, and your Raider
is inside theirs.* The Marksman is sharing a region with a Wolf, which means **it is no longer in
the back at all** -- not by fiat, but because "the back" was only ever "a region with no enemy in
it." Its safety was a fact about the board, and the board changed.

The Marksman's problem is now real, and every option costs:

- **Stay and fight.** It is a cannon in a melee. Bad, but the Bastion is still in-region, so
  Defend(Marksman) still *bodyguards* -- single-target damage aimed at the Marksman hits the Bastion
  first. It just cannot *deny* any more. The wall is inside the wall.
- **Flee** to fresh ground. That is a **Peel** -- so it eats **parting blows** from the Wolf and the
  Ogre on the way out, and it lands alone and unscreened unless somebody comes with it.
- **Be rescued.** The Bastion could Peel with it -- but then the Bastion turns its back on the Ogre
  and eats a parting blow too, and the region it abandons is the region the Bombardier is standing in.

None of that requires a single new rule. It all falls out of "crossing is unscreened" plus "the
screen is paid on entry." And it is *a decision*, made under a real cost, that would have been
completely invisible in both current models -- the spec would have simply re-dealt the ranks, and the
product would not have let anybody move at all.

**This is the answer to "the formation is all over the place."** The formation was all over the place
because the game handed out a free re-formation every round (spec) or forbade movement outright
(product). Charge for movement and the formation acquires **inertia** -- it has history, it has
consequences, and the round-2 picture is a *consequence of* the round-1 picture rather than a fresh
roll.

---

## 9. Tractability -- the part that has to survive

This is the constraint that can kill the design, so it gets an honest section rather than a
reassurance. The good news: the two hardest numbers are already measured in this repo.

### 9.1 The state does not blow up (and probably shrinks)

The solver's memo key is `(per-unit (health, tempo, fallen, pending, rank), round, sub)`
(`solver.rs:642`). **Rank is a `u8`. A region id is also a `u8`.** The key does not change shape.

And the *count* of formation states goes **down**, if regions are canonicalized:

| bodies on field | rank states (`3^n`) | region states (`Bell(n)`) |
| --------------- | ------------------- | ------------------------- |
| 4               | 81                  | **15**                    |
| 6               | 729                 | **203**                   |
| 8               | 6561                | **4140**                  |

`Bell(n) < 3^n` for every `n <= 8`, which covers every real encounter. **The partition is a
cheaper state than the rank assignment.** This is precisely why the region *labels* must never
enter the state -- `{A: Bastion, B: Marksman}` and `{B: Bastion, A: Marksman}` are the same
position, and canonicalizing (sort the partition) is what buys the whole table above. Letters are
UI; partitions are state.

### 9.2 The branching factor is the real bill, and we have a measured baseline

`v2_remarshal` measured the cost of adding a per-round `3^living-heroes` re-declaration: **24x**.
That is the number to beat, and it is a real, in-repo, measured baseline rather than a guess.

The new per-round declaration is `(moves x aims)` per unit, so the budget is:

> keep `moves x aims` per unit at or below roughly **3 to 5 after pruning**, and we are inside the
> envelope that was already paid for and found affordable.

Four levers get there, and three of them are **already canon**:

1. **§4.1 count-adaptivity is already law:** *"a choice is presented iff it has >= 2 legal options."*
   Defend an ally nobody can reach: not offered. Press an enemy you cannot see or afford to reach:
   not offered. Support a target with nothing to gain: not offered. This is not a new solver hack --
   it is an existing *rule* the solver is entitled to lean on.
2. **§4.6's positive-effect rule is already law:** a Tempo spend that cannot change the outcome is
   never committed. A move you cannot pay for is not a branch.
3. **Cap regions per side at 3.** A formation is at most three knots. This caps `Join` targets at ~4.
4. **Most bodies Hold most rounds.** This is the big one, and it is a *design* property, not an
   optimization: movement is the exception because it *costs*. Order the search to try `Hold` first
   and the reachability search (which short-circuits on a win -- `forces_win` already does) finds its
   line without ever expanding the wide branches. Only `map_out` is exhaustive, and it is a *tool*,
   not the in-app oracle.

### 9.3 The safety valve already exists

The doom oracle is **budgeted and restartable**: it returns `Verdict::Evaluating` when it runs out
of nodes, and the one rule it must never break is that *an aborted subtree is not memoized*
(`solver.rs:73`). So the existing architecture already tolerates a bigger tree gracefully:

> **An honest "Evaluating" is allowed. A wrong "Doomed" is never allowed.**

That property is what lets this design be *attempted* rather than merely argued about. Build it,
run `map_out_formation` on the worst case, and read the real number.

### 9.4 The honest risk -- RESOLVED, it passes (see §0)

*Written before the probe:* if, after pruning, `moves x aims` stays large (say `>= 8` per unit), a
4-hero party puts `8^4 = 4096` joint declarations at the top of every round, and five rounds of
that will not exhaustively map out. **This is the single thing to prototype first.**

*Measured:* **836 nodes, 577 worst-case memo states, ~110 ms for all eight encounters.** Nothing.
The predicted mitigations all landed -- canonicalized partitions, count-adaptive aims, and
short-circuit-on-first-win -- and the "move follows from the aim" collapse (§0) removed the
`moves x aims` product entirely: a unit makes **one** declaration, not two, so the branch is
`~7` per unit rather than `~8 x 5`.

The in-app doom oracle can afford this. This section's fear is retired.

---

## 10. What this buys, listed plainly

- **The metaphor survives round 1**, which was the ask.
- **A formation with inertia**, and therefore a *reason* for re-Marshal to exist at all -- it
  resurrects the decision axis the product had to delete for being free and therefore worthless.
- **Four sub-phases instead of five** (and one of those cuts, merging Intercept + Volley, is
  available today regardless).
- **Range is derived, not decreed.** Melee crosses; ranged does not; both pay the screen. This
  deletes the targeting matrix.
- **The shield is derived, not decreed.** "The back" is any region with no enemy in it.
- **Groups and positions collapse into one mechanic** (§4.5 stops being a bolt-on and becomes what a
  region *is*) -- and it is unbuilt in the product, so this is a merge, not a migration.
- **The breach becomes a persistent, visible fact** on the table instead of a per-round flag.
- **It fits the card-table primitives** -- regions are piles, Defend/Support/Press are card
  `association`/`Link` edges. This maps onto the existing UI vocabulary far better than three fixed
  rank-piles do.
- **The RPS triangle survives, and generalizes**: Press-deep beats the supported cannon; the cannon
  beats the screen (only Might cracks Grit, and it never has to cross); the screen beats the crosser
  (screen-and-drain). It is now playable *at any depth of the graph* rather than only between ranks.

## 11. What it costs, and what I could not settle

1. **The branching factor (§9.4).** The one real risk. Prototype it before committing.
2. **Cycles in the defend-graph.** A defends B, B defends A. This is **legal and merely expensive** --
   passing a screen is a *cost*, never an immunity, so a cycle just means paying twice, and the
   chain is bounded by party size. Force-not-fiat holds. It also means the mutual-turtle formation
   exists, is available, and is a trap (both bodies spend the fight screening each other and nobody
   attacks). That is a *good* outcome, not a bug -- but it should be confirmed on the solver.
3. **A crosser now always strikes before the standing melee** (Arrive precedes Contact), where the
   old model gave the Outrider an early slot only against the *back* (`O->V` sat in the Breach).
   This is a **deliberate divergence** and it is the main balance risk: charging may now be too good.
   It is motivated (initiative bought by charging is paid for in exposure) but it is a dial, and it
   is the first thing balance should look at.
4. **What halts a stopped crosser?** Proposal: it does not arrive, it stays where it was, and its
   attack is spent -- but it *still ate* the parting blows and the incoming fire. Brutal, and exactly
   consistent with today's "the Intercept screens and drains it, so it reaches the back empty, or
   dead."
5. **Naming.** "Assassinate" is too narrow to be the general verb (the aim also covers "hit the body
   in front of me"). Proposal: the verb is **Press(X)**, and *assassination* is the name for pressing
   a **screened** target -- which is what makes it an assassination. Your word survives, attached to
   the thing it actually names.
6. **The blind bid.** The spec says the contest is a simultaneous blind bid; the product made it a
   perfect-information threshold (Engage, then Evade *seeing the commitment*). This design is neutral
   on that -- but the divergence should be resolved deliberately, because the spec's entire
   game-theoretic placement (§0.4: "mixed strategies live only in the hidden-simultaneous layer")
   rests on a blind bid the product does not have.

## 12. Decisions I need from you

1. **Prototype the branching factor first?** (§9.4 -- the only thing that can kill this.)
2. **Merge Intercept + Volley today**, independent of this redesign? It is a free cut under the razor.
3. **Support as a declared aim, or auto-derived?** Declared is cleaner; auto-derived is cheaper for
   the solver and is the obvious first thing to sacrifice if §9.4 goes badly.
4. **Reconcile spec and product** on the three divergences in §1 -- and in each case, which one wins?
   (My read: the product wins on the pile-wipe and on armor; the spec wins on groups; the blind bid
   is genuinely open.)
