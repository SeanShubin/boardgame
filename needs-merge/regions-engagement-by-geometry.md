# Combat: two formations, engagement by geometry

**Status: SHIPPED** (implemented in `crates/rules/src/combat`, balanced, verified). Author: one
instance, updated 2026-07-16. This is the current combat model; it supersedes the multi-region
"regions" exploration this file used to describe, and the earlier five-beat rank schedule.

*2026-07-16 revisions folded in:* Slip and Raid merged into one **Cross** act ([§8](#8-movement-the-one-way-cross-slip-and-raid-are-one-act));
the horde **defence** reworked to **separate per-body Grit pools, no spill** ([§5](#5-hordes-one-body-many-bodies));
the roster is Swarm (melee front) / Brood (ranged back) / Sniper, no "Storm" ([§11](#11-the-balance-ladder-all-stats--7)).

**The shape in one breath.** Two formations face off -- the party and the foes -- each a single
group with a **Vanguard** (its melee front) and a **Rearguard** (its ranged back). There is no setup
and no formation to choose: a fight opens on round 1. Every round, every body declares one action;
the round resolves by *how fast each blow lands*, nearest first, through three **rings** of
**strikes**. The whole thing is derivable from a handful of rules, not a memorized timeline.

**Vocabulary.** A body's position is its **[`Rank`]** -- one enum, three values. `Vanguard` (front,
melee) and `Rearguard` (back, ranged) are **fixed by the weapon**; **`Outrider`** is the third and
the only one you *earn*, by crossing into the enemy's ground. "Front"/"back" below are just the
Vanguard/Rearguard read spatially.

---

## 0. Vocabulary: the two pools, the two flows

Combat runs on two resources that share **one shape**, and two outputs that share **another**. Naming
them in parallel is what keeps the log (and the rules) legible.

**Five stats, one shared chassis.** Every body reads from the same `[Might, Vitality, Grit, Cadence,
Finesse]`; roles differ by *card mechanics*, never by which stats they own.

**Two pools -- `count x strength`.** A pool is a stack of cards. Both refill each round (the Reset):

| Pool       | how many cards (count) | strength of each card |
| ---------- | ---------------------- | --------------------- |
| **Health** | **Vitality**           | **Grit**              |
| **Tempo**  | **Cadence**            | **Finesse**           |

*Health is Vitality cards, each Grit strong; Tempo is Cadence cards, each Finesse strong.* Every
initial is distinct (`M V G C F`, plus the pools `H T`), so a one-letter tile is never ambiguous.

**Two flows -- `strength x count`.** What you *produce by spending* is also a product. The strike side
deals **damage**; the contest side (reach/dodge) produces **reach**:

| Flow       | = per-unit strength x count   | measured against                            |
| ---------- | ----------------------------- | ------------------------------------------- |
| **damage** | **Might** x **strikes**       | **Grit** -- a Health card flips per Grit    |
| **reach**  | **Finesse** x **tempo cards** | the opponent's **reach** -- the higher wins |

- A **strike** is one blow. Banked **damage** piles up; a **Health card flips** each time the pile
  reaches **Grit**, and any remainder clears when the sub-phase closes.
- **reach** is the contest value on *both* sides: the attacker's is its **bid**, the defender's is
  its **dodge**. The side trying to get through must **strictly beat** the other; the
  reacher/screener **wins ties** (section 3). The first strike is bought by the reach that lands the
  contact; each extra **tempo card** poured after it is `+1` strike.

**Always multiply, never divide.** Every number the log shows is a product -- `3 tempo x Finesse 5 =
15 reach`, `5 Might x 2 strikes = 10 damage`. Division only ever appeared inside a solver shortcut
(the fewest cards needed to win a contest); it is kept out of the rules and out of the log.

**The one asymmetry.** On the Health side, offense and defense are *different* stats: **Might** (the
attacker's per-strike output) vs. **Grit** (the defender's per-card strength). On the Tempo side there
is only **Finesse** -- a tempo card *is* both the pool card and what you spend into the contest, so
Finesse does the job of *both* Grit (card strength) and Might (per-unit output). That is why the
contest is symmetric (reach vs. reach) while the damage exchange is not (Might vs. Grit). A **horde**
tilts it once more: its reach carries the body-count multiplier (`tempo x Finesse x bodies`) but its
**dodge does not** -- a fearsome catcher, a poor evader.

---

## 1. Standing: one formation per side, rank by weapon

- Each side occupies **one region** -- the party on its ground, the foes on theirs, with open
  ground between. (The region field survives only to track who is *home* vs. *loose* (an outrider);
  nobody ever chooses a partition. Multiple regions were removed once the
  [removal test](#9-what-was-cut-and-why) proved splitting never earns a win.)
- **Rank is the weapon, fixed for the fight:** a **melee** body is a **Vanguard** (front), a
  **ranged** body a **Rearguard** (back). Not chosen, never changed. (A body carrying both would need
  a real choice -- none exist, so it is deferred to Vanguard.)
- **Screening is compositional and automatic:** a rearguard is *screened* exactly while its side
  still has a **living vanguard**. No positioning decision -- protecting your back line is a matter of
  *having* a front line.

## 2. The core: engagement is chosen by geometry

A body never picks an "attack type." It picks a **target**, and where that target stands decides
which of three engagements happens. All three run the same little primitive.

| Engagement | The target is...                                       | Reaches                   | Who may        |
| ---------- | ------------------------------------------------------ | ------------------------- | -------------- |
| **Clash**  | an enemy **Vanguard**, across the gap                  | that front                | any weapon     |
| **Raid**   | an enemy **Rearguard**, across the gap                 | that back -- you cross in | **melee** only |
| **Melee**  | an enemy in your **own** region (an outrider is loose) | anyone in-region          | any weapon     |

A **Raid** and a plain **Slip** (a cross with no target) are the same act -- `Act::Cross(Option<target>,
Answer)` -- since both cross the gap and land you as an outrider; see [§8](#8-movement-the-one-way-cross-slip-and-raid-are-one-act).

**The one primitive** (the "inner three"):

1. **Target** -- declare who you reach for.
2. **Evade** -- the target answers: **Evade** (pay the slip cost, untouched), **Push** (pay nothing,
   eat the blows, act anyway), or **Abort** (turn and fight, give up the ground).
3. **Strike** -- the opening blow the reach bought, plus **one extra strike per remaining tempo**,
   poured into the declared target. (A **horde** is the exception -- it strikes as one volley; see
   [§5](#5-hordes-one-body-many-bodies).)

**AoE is the *width* of that reach, never extra reach.** A sweep catches every enemy the same
single-target attack could have reached: a **Clash** sweeps the whole enemy **front line**; a
**Raid** sweeps the whole **back line** you now stand among; a **Melee** sweeps **every** enemy in
the region, front and back, because a body that got *inside* is past the screen entirely; and a
**Volley** (an area weapon catching a crossing body) sweeps the whole **charge** -- every crosser it
is catching, **unevadably**. An area weapon reaches with the same shape on *defence* as on *offence*,
so a lone Salvo scatters a charging horde exactly as it would sweep a standing one.

## 3. Reach, and why unscreened is no advantage

Reach follows the target's rank and screen:

- An enemy **Vanguard** -> **Clash** (across the gap, any weapon).
- A **screened Rearguard** (its side still has a living vanguard) -> **Raid** only. The vanguard
  **intercepts** the raid on the way in -- that is the whole worth of a screen.
- An **exposed Rearguard** (its vanguard has fallen) -> it is **clashable** by anyone (always
  targetable, so standing unscreened is never *shelter*) **and raidable** by a melee body (a raider
  reaches it in the Crossing ring, *before* it would fire, so being unscreened is never an *advantage*
  either). A screen is what buys a back its first shot.
- An enemy in your **own** region (an outrider) -> **Melee** (no screen between intermingled bodies).

## 4. The outrider: a raid's lasting mark

A raider that survives the crossing and lands does **not** dissolve the formation it entered -- the
host keeps its vanguard, rearguard, and screen. The raider becomes an **Outrider**: a **loose body
inside those ranks**, past the screen so it strikes anyone, adjacent to everyone so anyone strikes
it. It is a **one-way commitment** -- once inside, there is no slipping back out. It stays a splinter
until it is killed, or until the formation it is loose among is gone and its state dissolves (it
reverts and rejoins its own line).

That is the payoff of a raid, bigger than "the raider ends up somewhere":

- Even a raid that does not finish the cannon it went for leaves a body **parked next to it**,
  threatening it **every round** until dug out.
- The defender is **taxed** -- actions spent digging the outrider out are actions not spent across
  the gap.
- An outrider with a **sweep** is devastating: inside the screen, its area strike catches the host's
  front *and* back at once. That is the price of letting one in.

**Dissolution -- an outrider without a host is an outrider of nothing.** "Outrider" is a *relation* --
loose among an enemy formation, past their screen. If outriders kill *every* body of the formation
they were loose among, the relation ends (settled once at the end of the Inner ring): each reverts to
its weapon rank and **rejoins its own line** if it still holds one, else simply becomes the formation
where it stands (the last of its side). There is no ground to garrison in a two-formation fight, only a
line. (This replaced *promotion* -- "clearing a zone takes the ground" -- a multi-region leftover that,
with one region per side, mostly coincided with the win and mishandled a mutual raid.)

## 5. Hordes: one body, many bodies

A **horde** (the Swarm, the Brood) is a single body whose **Health is a body count** -- one-Health
members swinging together. Its size is spent as **damage** and **reach**, never as tempo (it
refreshes to Cadence like anyone).

- **Attack: one volley.** A horde strikes as a single blow worth **body-count x (Might - armor)** --
  every living body landing at once. Armour bites **per body**, so a swarm of Might-1 bodies does
  nothing to an armoured target however many there are. It does **not** pour extra strikes: the whole
  swarm is that one volley.
- **Reach is amplified, evasion is not.** A single tempo card's reach is multiplied by the body count
  (bid = `cards x Finesse x bodies`), so a horde is **hard to slip** -- a fearsome catcher, and its
  own attacks pin their target even on a small Cadence. But its **evade** stays the ordinary flat cost
  (x1, never x bodies): numbers never help it hide -- missing one body just hits another. **A horde is
  a great catcher and a poor evader.**
- **As a target: separate per-body Grit pools, no spill** (reworked; supersedes the old "aimed fells
  one, ungated / sweep clears if Might > armour"). Each body is its own one-Health, **Grit-strong**
  pool. An **aimed** blow fells at most **one** body, and only if it **penetrates** that body's Grit
  (`Might - armor >= Grit`); overkill and sub-Grit both waste, so **to fell another body you spend
  another blow** (tempo per body). A **sweep** hits every body at once and clears the **whole pack**
  for one card -- but only if it penetrates. So **width, not power, is the cheap answer to a swarm**,
  and **Grit is a pure penetration gate** (who can dent it at all). A sweep and the horde's own volley
  share the **commit-batch**, so the sweep does not shrink that round's volley: it hits full-strength
  as it dies (Spec 1.9).

This is why the Swarm (a **melee front** horde) answers to the **Bombardier** -- a ranged sweep that
first-strikes it from outside, before it can clash -- and the Brood (a **ranged back** horde) to the
**Bastion** -- a tanky melee sweep that goes *in* after it -- while a single-target attacker, capped at
one body per blow, cannot out-race either.

## 6. The timing principle: whatever connects first, goes first

The round order is one idea -- **resolve blows in the order they land**, i.e. by distance:

- An **outrider already in your ranks** is distance zero -- swinging before anyone turns. **First.**
- An **arrow across the gap** is fast (the shot travels, the body does not).
- A **body that must close** -- a swordsman clashing, an outrider landing -- is slowest. **Last.**

And the two orderings that look like separate rules are one rule, two frames: something coming *at*
you meets your **vanguard first** (spears before bows -- intercept, then volley); *you* reaching
*across* connect soonest with your **arrow** (bows before swords -- Fire before Clash).

## 7. The round: three rings of strikes

The **Reset** opens the round (tempo stands back up to **Cadence** -- hordes included; a horde's size
is damage and reach, not tempo). Then three distance **rings**, nearest first. Each ring resolves in
**strikes**; a strike is a commit-batch (all its blows land together, so a blow lands even if its
striker dies to a simultaneous one), and each strike ends in a **death check** that finalizes the
fallen. A body killed in a strike is **silenced** in every later strike -- the razor: a strike earns
its place only by letting a death in it silence something after it.

- **Inner ring -- Outriders** (distance 0). Every region holding both a formation and enemy outriders
  fights in place, no screen. It is **one simultaneous strike** -- melee and ranged together, because
  nobody is closing (the crossing happened a round ago; everyone is point-blank). Then dissolution.
- **Crossing ring** -- every declared raid crosses and runs the gauntlet:
  1. **Intercept** -- the enemy **vanguard** reaches for it (a death here silences the volley *and*
     the strike).
  2. **Volley** -- the enemy **rearguard** looses at survivors the spears did not drop (a death
     silences the strike). Front before back, so a corpse is never shot twice. An **area** rearguard
     volleys as a **sweep** -- one card scatters the whole charge (a horde crosser cleared outright),
     so a lone artillery piece defends a charge from range instead of being overrun.
  3. **Land & strike** -- survivors strike the back they came for and **remain as outriders** for the
     next round's Inner ring.
- **Outer ring -- across the gap.** **Fire** (every rearguard looses -- a death silences the target's
  Clash) then **Clash** (every vanguard closes and trades, slowest, last).

Crossings resolve before the Outer ring's Fire (a body sprinting into your lines is more urgent than
a duel across the field).

**Damage and Grit.** Each strike banks `max(0, Might - armor)` into a pile; a Health card flips every
time the pile crosses the target's **Grit**. **The pile closes at every death check** (every strike),
not just the Reset -- so only blows within *one* strike combine, and Grit is a real gate that a blow
must cross *there*, not wear down over a round. (This is why a Wall's Bulwark needs one overwhelming
strike.) A fight undecided in **5 rounds** is a Draw, which counts as a loss.

## 8. Movement: the one-way Cross (Slip and Raid are one act)

**Cross** is the only movement, and it is a **one-way commitment**. A body crosses the gap into the
enemy line and becomes an **Outrider**; there is **no retreat** back out. Reaching a screened back and
going loose in the enemy ranks are the **same mechanic**, so Slip and Raid are **one act** --
`Act::Cross(Option<target>, Answer)`:

- `Cross(Some(rearguard))` = a **Raid**: it strikes that back-line body on arrival (Crossing ring,
  before the body fires). The on-arrival target is the **back line only** -- a front body you
  **Clash** -- so a raid never gives melee a Crossing-ring pre-emption of the Outer Clash.
- `Cross(None)` = a plain **Slip**: cross and stand, striking nobody this round.

(Merging them is balance-neutral: under perfect information a target chosen on arrival is equivalent to
one declared up front, so the solver already searched it.) The crossing is opposed by the enemy
formation reaching for you as you cross in -- which is where a pushed crosser's damage comes from. An
outrider leaves enemy ground only when the formation it
was loose among is destroyed (its state **dissolves** and it rejoins its own line) or by dying.
(Retreat and the both-ends "crossing in reverse" were removed with the retreat itself -- another rule
that did not change a win.)

## 9. What was cut, and why

Removed by the **removal test** (does it ever change a win?), keeping the core solid rather than
carrying variety on spec:

- **Multi-region partitioning + the setup phase.** Splitting a formation across regions only helps as
  *AoE defense* (spread out so one sweep catches fewer), and no creature has AoE. A `OneRegion`-vs-free
  comparison confirmed splitting never earns a win, so the whole partition mechanic and its setup
  phase are gone -- and the search got ~4x faster with no formation tree. A standing diagnostic will
  flag the day an AoE creature makes splitting load-bearing again.
- **The collapsed-vanguard advantage.** An exposed rearguard used to *fire first and be unraidable* --
  a reward for being unscreened. Now it is raidable (see [§3](#3-reach-and-why-unscreened-is-no-advantage)).
- **Intruder retreat.** An outrider could slip back home; nothing ever needed it, so a raid is now a
  one-way commitment (see [§8](#8-movement-the-one-way-cross-slip-and-raid-are-one-act)).
- **The "screen necessary" lesson.** It could not be expressed: with two rearguards and generalist
  melee vanguards, no single body is ever decisively necessary. Replaced by the CombinedArms capstone
  below.

## 10. The Game seam and the solver

Combat is a pure state machine (`crates/rules/src/combat/game.rs`) behind the generic `Game` trait:
`options` / `apply` / `outcome` over a `Clone`able `State`. **Everyone declares through one loop** --
a hero's options are its real choices; a foe's is a single scripted option (its instinct's pick), so
a foe "declares" too and a driver auto-advances it. That keeps it **single-agent reachability** (a
foe multiplies branching by one), so the generic `Solver` (winnable / evaluating / doomed) and
`PathCounter` search it unchanged. The memo key is per-body **`(health, fallen, rank)`** + canonical
regions + round + declare cursor + pending declarations (the `Rank` carries the outrider state, so no
separate flag); **tempo and the damage pile are excluded** -- both are transient scratch inside
`play_round`, never a state variable. The one control newtype is **`ClashOnly`** (the party may not
Cross -- no raid, no slip), used to prove a raid load-bearing.

## 11. The balance ladder (all stats <= 7)

**Stats** `[Might, Vitality, Grit, Cadence, Finesse]`: Raider `[6,6,1,2,2]` · Marksman `[5,2,1,2,2]` ·
Bastion `[1,3,3,1,2]` · Bombardier `[3,3,1,1,2]` -- Wall `[1,4,6,1,2]` · Duelist `[6,5,1,2,2]` ·
Swarm `[3,4,1,2,2]` (**melee front** horde) · Brood `[1,7,1,1,1]` (**ranged back** horde) · Sniper
`[5,1,1,2,3]` (ranged single, corner-only). For a horde, Vitality is the body count, and its volley is
that count x Might (see [§5](#5-hordes-one-body-many-bodies)).

**Solos** -- each creature soloable by exactly its counter kit: Duelist->Marksman, Wall->Raider,
**Swarm->Bombardier** (clear the front horde from range), **Brood->Bastion** (go in and sweep the back
horde). The Sniper has no solo -- it is a corner-only priority threat.

**Nine encounters** -- a 3x3 map: the 4 solos, 4 **orthogonal strategy corners**, and the
**CombinedArms capstone** at the centre. Each corner needs the full party to win under `Combat` AND its
own necessity test (a different control failing while its own strategy suffices, so none subsumes
another); scored by search. Exact warbands live in `crates/deckbound-content/src/catalog.rs`.

| Encounter          | Behavior          | Passes iff (full party wins, and...)                                     | Warband                        |
| ------------------ | ----------------- | ------------------------------------------------------------------------ | ------------------------------ |
| Emberfall Hollow   | **Concentration** | single-only wins, area-only loses                                        | `Wall x2`                      |
| Greywater Ford     | **Range**         | ranged-only wins, melee-only loses                                       | `Duelist x3`                   |
| Ninefold Deep      | **Sweep**         | area-only wins, single-only loses                                        | `Swarm x2`                     |
| The Hollow Rampart | **Raid**          | `ClashOnly` loses (the raid is load-bearing)                             | `Wall x4, Sniper x1`           |
| Ashfen Crossing    | **CombinedArms**  | melee-only, ranged-only, AND single-only all lose, AND `ClashOnly` loses | `Swarm x1, Wall x2, Sniper x1` |

CombinedArms (the capstone) is the graduation exam: ranged *and* melee damage *and* a raid all at once,
so it *deliberately subsumes* the four corners. Verified: `regions_diagonal` reads **4/4 solos + 5/5
party fights in ~25 ms**.

---

*Canonical-doc note: this file lives in `needs-merge/` for history; the shipped rules are the source
of truth in `crates/rules/src/combat/`. The older `docs/games/deckbound/notes/` combat notes
(turn-structure, zones) still describe the rank/zone model and are a separate follow-up to reconcile.*
