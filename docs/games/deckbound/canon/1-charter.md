# Deckbound — Design Philosophy

> The north stars. When a design decision is unclear, resolve it in favour of
> these. They are *why* the game is the way it is; the other notes are *how*.
> Keep this short and honest — if a principle stops being true, change it here on
> purpose rather than letting the design drift.

## 1. Represent and reward human intellect

The game exists to make human cleverness matter. Wherever a human mind is
represented — a player, or a computer standing in for one — choices come from
**reasoning**, not from a shuffled deck. Decks are for non-player nature, and even
those are built to be **readable**, so that beating them is the player's
achievement. See [decision-making](decision-making.md).

## 2. Computable tactics, uncomputable strategy

The **tactical** layer — a single hidden, simultaneous exchange — is deliberately
**constrained enough to be computable**: a well-defined rock-paper-scissors-plus-
magnitude game with a solvable optimal (mixed) strategy. That is what lets a
computer stand-in, or a sharp human, play it game-theoretically, and what makes
predictions and bluffs meaningful.

The **strategic** layer — which conflicts to enter, when to push your luck, what
to spend and exhaust, which capabilities to chase, when to retreat — is governed
by **risk/reward, push-your-luck, and opportunity cost**, and is deliberately
**not** reducible to a clean computation. Tactics are a near-solvable skill;
strategy is judgment.

**Design consequence.** Keep the tactical exchange **small and well-defined**
(bounded options, crisp resolution) so it stays computable; push richness and
open-endedness into the **strategic / meta** layer (scenarios, acquisition,
exhaustion, routing), where it should *not* be solvable.

- If the moment-to-moment turn is becoming too complex to reason about cleanly —
  that's a smell. Simplify the tactics.
- If the overall campaign is collapsing into one solvable optimization — that's
  also a smell. Add a real risk/reward or opportunity-cost fork.

**Relation to #11.** "Not reducible to a clean computation" is the *player's*
experience, created by the Clash and the world's hidden information. Beneath them sits a
**deterministic skeleton that _is_ computable on purpose** — the designer's instrument for
proving the game is beatable and for *checking* that the strategic layer has not collapsed
into one dominant line. The two are consistent: see [#11](#11-computable-by-construction--so-the-other-promises-can-be-proven).

## 3. Hidden information, simultaneous reveal

Every contest is a hidden, simultaneous commitment resolved at once — a game of
predictions and bluffs, not of reacting to a revealed move. See
[decision-making](decision-making.md).

## 4. Asymmetry by design, balance by scenario

Characters are **deliberately unbalanced**; fairness and challenge come from the
environment, the objective, and the team — not from evening out the roster. See
[world-and-progression](world-and-progression.md) and
[decks-and-aspects](decks-and-aspects.md).

## 5. From doom to mastery

Reach is limited early; loss is real; some places mean certain doom until the
character has grown. Progress is **earned** through exploration and combat, and
the world reacts to it. See [world-and-progression](world-and-progression.md);
for the borrowed craft behind this, see
[design-principles](design-principles.md).

## 6. Many systems from few rules

Depth comes from a **few consistent systems interacting**, not from many scripted
features. Prefer rules that generate emergent interactions over one-off content.
See [design-principles](design-principles.md#emergence--systems).

## 7. Playable by hand, cards only

No computer is required to *run* the game; cards and shuffling only; resources are
represented as cards. A computer is an optional convenience, never a dependency.
See [constraints](constraints.md).

## 8. Deliberate cards; predictability is a resource

A player's own cards carry **no luck** — no shuffle, no random draw; what you can do is
whatever sits **ready**, and the **order you commit is intent**. A player's
unpredictability is a **managed resource** that **erodes as cards exhaust** (Spend →
face-down) and is **restored only at a tempo cost** (Recover). The mechanism is **zone
state**, not deck order — see Spec [§5 (Zones / exhaustion)](2-spec/README.md#5-zones--exhaustion--the-card-state-machine-).

## 9. Every rule rides on a solid metaphor

The game must be **remembered without a rulebook**. So each mechanic rests on a clear
physical image: you block a path by *keeping pace* with someone; running past a guard
*gets you hit*; more guards *cover more angles*; aggression *spends you*. If a rule
can't be stated as an intuitive picture, it is too abstract to hold at the table —
rework the rule, or the metaphor, until it can. Mechanics serve the metaphor, not the
other way around.

## 10. Conceptual integrity — every rule re-derivable from its intent

The game must be **reconstructable, not memorised.** A *simple* rule is still bad if it
is **arbitrary** — its shape has no reason, so it can only be held by rote. A *complex*
rule is fine if it is **motivated** — its form follows from its intent, so anyone who
holds the intent can **re-derive** the parts they forget. Prefer a motivated rule over a
merely short one; arbitrary simplicity is the trap.

The whole design should spring from a **few intents** — these north stars, and each
mechanic's stated reason — so a reader (player, designer, or AI) who grasps the intents
can rebuild the mechanics rather than recall them. That property is **conceptual
integrity**: it is what lets a large game still be held in one head.

This is the general principle of which **#9 (every rule rides on a metaphor) is the
fiction engine.** A metaphor motivates a rule by tying it to a physical image; a rule
can also be motivated by its **consequence** (Edge is per-duel because a fight-long
meter would snowball — re-derivable, but no picture). Either way the test is one
question: *could someone who forgot this rule rebuild it from why it exists?*

- If a rule can only be stated as a bare fact — no metaphor, no consequence, no intent —
  it is **arbitrary**. Rework it until it carries its own reason, or cut it.
- "Simplifying" a rule by **severing it from its reason** is a regression, even if the
  text got shorter.

## 11. Computable by construction — so the other promises can be proven

There is a **canonical analysis mode** — the optional Clash module **off**, creature
draw decks and locations **open** — in which the whole game is **deterministic,
perfect-information, single-agent, and bounded**. (Single-agent because the creatures are
a *fixed* environment, not an adversary searching against your plan.) In that mode the
campaign is a finite planning problem, so **par is computable**: the fewest days to win,
and whether a given build clears a given place, can be *computed*, not guessed.

This mode is the **designer's instrument and a correctness floor**, not the player's
default. The played game keeps the Clash's hidden, simultaneous exchange (#2, #3) and the
world's fog and push-your-luck (#5) on top, so the player's strategy stays **judgment under
uncertainty**. The determinism underneath is what lets us *prove* the game is beatable and
*measure* whether the design's promises hold.

**It is the instrument that enforces #2 and #4.** #2 warns that the campaign must not
collapse into one solvable line; #4 promises balance by scenario, not by roster. A
computable par lets us **check both objectively**: that *many* interesting strategies tie
near par, and that **no** unnamed strategy dominates them. So computability does not fight
#2 — it is how we keep #2's promise.

**Therefore the canonical mode must stay computable.** Guard these invariants; a mechanic
that breaks one belongs in an optional mode (Clash, Versus) or must be explicitly bounded:

- **No live randomness or hidden information** in the canonical mode — confine it to the
  Clash, or make it open.
- **Foes are a fixed environment, not an adaptive adversary** — scripted / stat-driven,
  never searching against the player's specific plan (that is the jump to two-player
  minimax, the chess cliff).
- **Battles stay near-stateless functions of `(build, place)`** — no carried wounds or
  buffs; or, if any, kept small, discrete, and bounded, so combat remains a memoizable
  oracle.
- **Builds stay monotone, additive, order-independent — no path-dependent budget** — owned assets
  only accrue and combine commutatively, with **no resource refund** (sell-back / oscillation) or
  multiplicative combo. (Rearranging *owned* assets is fine; a history-dependent *budget* is what
  explodes the search — see Spec §0.1.)
- **Bounded horizon and modest branching** — a day cap, few legal actions per state,
  terminating combat.

The discipline is concrete: the par solver is to be wired as a **regression test** — if a
change makes the reference scenario stop solving within its state / time budget, that change
has broken this north star and fails the build. "Feasibly computable" is an *enforced
budget*, not a hope.

The full reasoning — the structural facts that make it computable, the design-review
checklist, par's policy-relativity, and the objective balancing method (interesting beats
boring, plus the closure check that no unnamed strategy dominates) — lives in
[computability-and-balance](../computability-and-balance.md). **Read it before adding a
mechanic that touches randomness, foe behaviour, carried state, build growth, or the day
clock.**

## 12. Roles are the spine; stats serve the roles

The five **Suits** are the unit of design. Each Suit names a **Role** — its function in combat — and the
**Role, not the stat, is what the game is built around.** Three things follow, and they are binding:

- **Stats are instruments, not first-class.** Every stat exists to give some Role its teeth — Vitality /
  Toughness let the **Wall** hold; Speed / Daring let the **Infiltrator** slip and dodge; **Might** arms
  the **Artillery**; the **Controller** turns the foe's own stats against it (lowering Might / Toughness /
  Daring, draining Tempo); the **Support** line raises those same dials. A stat **no Role needs is a
  defect** — cut it or re-home it. The Roles are **fixed**; the stats are **negotiable** — refactor the
  stat layer freely whenever it serves the Roles better. After the 2026 collapse the stat layer is a
  small **shared chassis** — **Might · Vitality · Toughness · Speed · Daring** — with **no role-exclusive
  signature stat**: the effect Roles (Controller, Support) bend these shared dials rather than owning a
  private one, and Roles are told apart by their **card mechanics**, not by a signature stat.
- **Each Role is uniquely valuable and load-bearing.** For every Role there is a challenge that **cannot
  be met without it**, so a party whose collective coverage **omits a Role is doomed**. The measure is
  the **campaign**, not a single fight: an individual conflict may be winnable by one Role alone — indeed
  a single-Role conflict is a **tutorial** in what that Role can and cannot do (#4 *balance by scenario*;
  Spec §8.4). No Role is decorative; none is redundant with another. **That necessity must be *earned*,
  not *granted*:** a Role is needed because the situation's natural pressures make its mechanic the
  effective answer — **never** because a foe is **arbitrarily immune** to the other Roles or a keyword
  **bans** them. The other Roles are **outpaced, not forbidden** — without R they still act, they just
  cannot clear the challenge within par. Manufacturing need by fiat-immunity is a design cheat: it fails
  **#6** (emergence over scripted exceptions) and **#10** (a memorized exception, not a re-derivable
  system) — and it is exactly why a foe **arbitrarily immune to damage so that only debuffs "work"** is
  **not** how the Controller earns its slot.
- **The Roles differ in kind, not degree.** Each owns a **distinct decision and mechanic** — hold the
  front / break through / fire from safety / **degrade** the foe / **augment** the ally — never a
  stat-reskin of another. This is also what makes a **god's role-combos** worth their concentration risk:
  combining Roles in one round (the per-role play cap, Spec §4.4) only pays if the Roles genuinely
  differ. If two Roles play the same and differ only in numbers, one of them has failed.

This is the operational sharpening of **#4** (*asymmetry by design, balance by scenario*): #4 says
fairness comes from the team and the scenario; #12 says **how** — each Role is a *necessary key* that
some scenario is the *lock* for, and the Roles stay maximally distinct so covering them is a real
composition problem, not a stat-shopping list. It rests on **#11** (*computable by construction*):
role-necessity is only a slogan unless the solver can **measure** it — remove a Role's coverage and some
reference scenario must become unwinnable. And it subordinates the stats to the Roles so the game stays
**reconstructable from a few intents** (#10): learn five Roles, and the stats fall out as their
implementation. See Spec §8.5 / §8.6.

*(The solver measures **structural** necessity — that some scenario is unwinnable without a Role. Whether
a Role *feels* distinct or *fun* stays the human's ratification, as with every balance claim — #11.)*

## 13. Damage is the triangle's; control and augment are the effect Roles'

**Direct damage belongs to the three §4-triangle Roles — Wall, Infiltrator, Artillery.** The two effect
Roles never deal it: the **Controller degrades** (a round-scoped status or stat-drop — never
damage) and the **Support augments** (buff / heal). A Controller or Support card that dealt direct damage
is a **defect**.

This is the hard edge of **#12**'s "differ in kind, not degree": the cleanest possible separation of the
`3 + 2` is that one axis — the triangle — **removes Body**, and the other — the effect pair — **never
does**; it bends the fight instead. It keeps each effect Role un-blurrable (a Controller cannot quietly
become a fifth damage-dealer), gives the game a **single, legible kill-condition** (#7 *playable by hand*:
you die exactly one way — your health pool empties), and makes "what does this Suit *do*?" answerable
without numbers: Iron / Silver / Brass **kill**, Bone **disables**, Salt **sustains**. Fear that *killed*
(the old scared-to-death) violated this and is gone; the separate fear/Dread **channel** was then
**collapsed out entirely** (2026), and the Controller now **degrades the foe's own stats directly**
(lowering Might / Toughness / Daring, draining Tempo) and hangs round-scoped statuses (Stagger → Shove →
Rout), never health loss. See **Spec §2.2** (one channel; control is stat-drop, not damage) and **§8.6**
(the damage-separation GUARANTEE).

---

**Using this document.** Every other design note should trace back to one or more
of these north stars. If a proposed mechanic doesn't serve any of them — or
actively fights one (e.g. making tactics uncomputable, making the **canonical mode**
uncomputable (#11), or balancing characters against each other) — that is the signal to
stop and reconsider, here, on purpose.
