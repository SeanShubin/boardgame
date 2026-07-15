# Combat: two formations, engagement by geometry

**Status: SHIPPED** (implemented in `crates/rules/src/combat`, balanced, verified). Author: one
instance, updated 2026-07-15. This is the current combat model; it supersedes the multi-region
"regions" exploration this file used to describe, and the earlier five-beat rank schedule.

**The shape in one breath.** Two formations face off -- the party and the foes -- each a single
group with a **front** (melee) and a **back** (ranged). There is no setup and no formation to
choose: a fight opens on round 1. Every round, every body declares one action; the round resolves
by *how fast each blow lands*, nearest first, through three **rings** of **strikes**. The whole
thing is derivable from a handful of rules, not a memorized timeline.

---

## 1. Standing: one formation per side, posts by weapon

- Each side occupies **one region** -- the party on its ground, the foes on theirs, with open
  ground between. (The region field survives only to track who is *home* vs. *intruding*; nobody
  ever chooses a partition. Multiple regions were removed once the [removal test](#8-what-was-cut-and-why)
  proved splitting never earns a win.)
- **Post is the weapon, fixed for the fight:** a **melee** body stands **front**, a **ranged** body
  stands **back**. Not chosen, never changed. (A body carrying both would need a real post choice --
  none exist, so it is deferred.)
- **Screening is compositional and automatic:** a back is *screened* exactly while its side still
  has a **living front**. No positioning decision -- protecting your back line is a matter of *having*
  a front line.

## 2. The core: engagement is chosen by geometry

A body never picks an "attack type." It picks a **target**, and where that target stands decides
which of three engagements happens. All three run the same little primitive.

| Engagement | The target is... | Reaches | Who may |
|---|---|---|---|
| **Clash** | an enemy **front** (vanguard), across the gap | that front | any weapon |
| **Raid** | an enemy **back** (rearguard), across the gap | that back -- you cross in | **melee** only |
| **Melee** | an enemy in your **own** region (an intruder is loose) | anyone in-region | any weapon |

**The one primitive** (the "inner three"):

1. **Target** -- declare who you reach for.
2. **Evade** -- the target answers: **Evade** (pay the slip cost, untouched), **Push** (pay nothing,
   eat the blows, act anyway), or **Abort** (turn and fight, give up the ground).
3. **Strike** -- the opening blow the reach bought, plus **one extra strike per remaining tempo**,
   poured into the declared target.

**AoE is the *width* of that reach, never extra reach.** A sweep catches every enemy the same
single-target attack could have reached: a **Clash** sweeps the whole enemy **front line**; a
**Raid** sweeps the whole **back line** you now stand among; a **Melee** sweeps **every** enemy in
the region, front and back, because a body that got *inside* is past the screen entirely.

## 3. Reach, and why unscreened is no advantage

Reach follows the target's post and screen:

- An enemy **front** -> **Clash** (across the gap, any weapon).
- A **screened back** (its side still has a living front) -> **Raid** only. The front **intercepts**
  the raid on the way in -- that is the whole worth of a screen.
- An **exposed back** (its front has fallen) -> it is **clashable** by anyone (always targetable, so
  standing unscreened is never *shelter*) **and raidable** by a melee body (a raider reaches it in
  the Crossing ring, *before* it would fire, so being unscreened is never an *advantage* either). A
  screen is what buys a back its first shot.
- An enemy in your **own** region (an intruder) -> **Melee** (no screen between intermingled bodies).

## 4. The intruder: a raid's lasting mark

A raider that survives the crossing and lands does **not** dissolve the formation it entered -- the
host keeps its front, back, and screen. The raider is a **loose body inside those ranks**: past the
screen, so it strikes anyone; adjacent to everyone, so anyone strikes it. It stays -- a splinter --
until it is killed or **slips back out**.

That is the payoff of a raid, bigger than "the raider ends up somewhere":

- Even a raid that does not finish the cannon it went for leaves a body **parked next to it**,
  threatening it **every round** until dug out.
- The defender is **taxed** -- actions spent digging the intruder out are actions not spent across
  the gap.
- An intruder with a **sweep** is devastating: inside the screen, its area strike catches the host's
  front *and* back at once. That is the price of letting one in.

**Promotion -- clear a formation and the ground is yours.** If intruders kill *every* body of the
formation that owns a region, that region flips to their side (they become its formation, at their
weapon posts), settled once at the end of the Inner ring. With one region per side this mostly
coincides with the win; it earns its keep the day formations span more than one region again.

## 5. The timing principle: whatever connects first, goes first

The round order is one idea -- **resolve blows in the order they land**, i.e. by distance:

- An **intruder already in your ranks** is distance zero -- swinging before anyone turns. **First.**
- An **arrow across the gap** is fast (the shot travels, the body does not).
- A **body that must close** -- a swordsman clashing, an outrider landing -- is slowest. **Last.**

And the two orderings that look like separate rules are one rule, two frames: something coming *at*
you meets your **front rank first** (spears before bows -- intercept, then volley); *you* reaching
*across* connect soonest with your **arrow** (bows before swords -- Fire before Clash).

## 6. The round: three rings of strikes

The **Reset** opens the round (tempo stands back up -- Cadence, or a horde's living body count). Then
three distance **rings**, nearest first. Each ring resolves in **strikes**; a strike is a
commit-batch (all its blows land together, so a blow lands even if its striker dies to a simultaneous
one), and each strike ends in a **death check** that finalizes the fallen. A body killed in a strike
is **silenced** in every later strike -- the razor: a strike earns its place only by letting a death
in it silence something after it.

- **Inner ring -- Intruders** (distance 0). Every region holding both a formation and enemy intruders
  fights in place, no screen. It is **one simultaneous strike** -- melee and ranged together, because
  nobody is closing (the crossing happened a round ago; everyone is point-blank). Then promotion.
- **Crossing ring** -- every declared raid crosses as an outrider and runs the gauntlet:
  1. **Intercept** -- the enemy **front** reaches for it (a death here silences the volley *and* the
     strike).
  2. **Volley** -- the enemy **back** looses at survivors the spears did not drop (a death silences
     the strike). Front before back, so a corpse is never shot twice.
  3. **Land & strike** -- survivors strike the back they came for and **remain as intruders** for the
     next round's Inner ring.
- **Outer ring -- across the gap.** **Fire** (every back looses -- a death silences the target's
  Clash) then **Clash** (every front closes and trades, slowest, last).

Crossings resolve before the Outer ring's Fire (a body sprinting into your lines is more urgent than
a duel across the field).

**Damage and Grit.** Each strike banks `max(0, Might - armor)` into a pile; a Health card flips every
time the pile crosses the target's **Grit**. **The pile closes at every death check** (every strike),
not just the Reset -- so only blows within *one* strike combine, and Grit is a real gate that a blow
must cross *there*, not wear down over a round. (This is why a Wall's Bulwark needs one overwhelming
strike.) A fight undecided in **5 rounds** is a Draw, which counts as a loss.

## 7. Movement: raid across, or retreat home

**Slip** is the one movement, and with one region per side it has just two destinations: cross into
the enemy region (a raid, becoming an intruder) or, as an intruder, fall back **home**. A slip is
opposed at both ends it touches -- the enemy formation reaches for you as you cross in *and* as you
pull out (a crossing in reverse) -- which is where a pushed slipper's damage comes from. A rally
between friendly bodies is unopposed.

## 8. What was cut, and why

Three things were removed by the **removal test** (does it ever change a win?), keeping the core
solid rather than carrying variety on spec:

- **Multi-region partitioning + the setup phase.** Splitting a formation across regions only helps as
  *AoE defense* (spread out so one sweep catches fewer), and no creature has AoE. A `OneRegion`-vs-free
  comparison confirmed splitting never earns a win, so the whole partition mechanic and its setup
  phase are gone -- and the search got ~4x faster with no formation tree. A standing diagnostic will
  flag the day an AoE creature makes splitting load-bearing again.
- **The collapsed-vanguard advantage.** An exposed back used to *fire first and be unraidable* -- a
  reward for being unscreened. Now it is raidable (see [§3](#3-reach-and-why-unscreened-is-no-advantage)).
- **The "screen necessary" lesson.** It could not be expressed: with two backs and generalist melee
  fronts, no single body is ever decisively necessary. Replaced by the CombinedArms capstone below.

## 9. The Game seam and the solver

Combat is a pure state machine (`crates/rules/src/combat/game.rs`) behind the generic `Game` trait:
`options` / `apply` / `outcome` over a `Clone`able `State`. **Everyone declares through one loop** --
a hero's options are its real choices; a foe's is a single scripted option (its instinct's pick), so
a foe "declares" too and a driver auto-advances it. That keeps it **single-agent reachability** (a
foe multiplies branching by one), so the generic `Solver` (winnable / evaluating / doomed) and
`PathCounter` search it unchanged. The memo key is per-body `(health, fallen, post, intruder)` +
canonical regions + round + declare cursor + pending declarations; **tempo and the damage pile are
excluded** -- both are transient scratch inside `play_round`, never a state variable. The one control
newtype is **`ClashOnly`** (the party may not raid), used to prove a raid load-bearing.

## 10. The balance ladder (all stats <= 7)

**Stats** `[Might, Vitality, Grit, Cadence, Finesse]`: Raider `[6,6,1,2,2]` · Marksman `[5,2,1,2,2]` ·
Bastion `[1,3,3,1,2]` · Bombardier `[3,3,1,1,2]` -- Wall `[1,4,6,1,2]` · Duelist `[6,5,1,2,2]` ·
Swarm `[1,7,1,1,1]` (ranged horde) · Storm `[3,4,1,2,1]` (melee horde).

**Solos** -- each creature soloable by exactly its counter kit: Duelist->Marksman, Wall->Raider,
Swarm->Bastion, Storm->Bombardier.

**Corners** -- three single-mechanic lessons plus a capstone, each scored by control comparison:

| Corner | Behavior | Passes iff (full party wins, and...) | Warband |
|---|---|---|---|
| Emberfall Hollow | **VanguardCarries** | melee-only wins, ranged-only loses | `Wall x2` |
| Greywater Ford | **RearguardCarries** | ranged-only wins, melee-only loses | `Duelist x3` |
| The Hollow Rampart | **RaidNecessary** | `ClashOnly` loses | `Wall x3, Swarm x1` |
| Ninefold Deep | **CombinedArms** | melee-only loses AND ranged-only loses AND `ClashOnly` loses | `Wall x2, Swarm x2, Storm x1` |

CombinedArms is the graduation exam: it demands ranged *and* melee damage *and* a raid all at once --
reachable where "screen necessary" was not, because capabilities are not redundant the way bodies
are. Verified: `regions_diagonal` reads **4/4 solos + 4/4 corners in ~39 ms**.

---

*Canonical-doc note: this file lives in `needs-merge/` for history; the shipped rules are the source
of truth in `crates/rules/src/combat/`. The older `docs/games/deckbound/notes/` combat notes
(turn-structure, zones) still describe the rank/zone model and are a separate follow-up to reconcile.*
