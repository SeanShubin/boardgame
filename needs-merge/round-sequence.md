# The round sequence

Status: **the shipped model**, 2026-07-20. The canonical step-by-step procedure
for one combat round, written so a human can run it at a table - and it is what
`rules::combat` actually runs: the two declare waves are the `Game`'s structure,
the rings are `play_round`'s schedule, and the combat log (`scripts/combat.sh` /
`scripts\combat.ps1`, mirrored to `fight-log.txt`) prints these coordinates. The
crossing's bid math lives in [crossing-bid-tree.md](crossing-bid-tree.md); this
document is the *frame* - the order of play, and who may do what when.

## Two principles, stated once

**1. Every step is a simultaneous declare/reveal.** At each step BOTH sides
secretly declare their part, then reveal together, then the step resolves
deterministically from what was revealed. The rulebook is the same for both
sides at every step - there is no step one side has and the other lacks.

**2. Perfect information collapses choices; it does not remove steps.** Hidden
information is the only thing that makes a declaration a gamble. Against a fully
predictable (scripted) enemy there is no hidden information, so most declarations
have exactly one sensible value - the rest are dominated. The **rules keep every
step for both sides**; the **user interface** is what hides a declaration that
cannot change the outcome and auto-advances a step where you have no real choice.
So: nothing in this sequence is ever skipped - only choices that cannot matter
are hidden from the player.

Read the sequence below as the full rules. What a solver or UI *shows* is this
same sequence with the dominated choices pruned.

## Design intent: the two shapes

The sequence exists to produce two recognizable strategic shapes. Every rule is
judged by whether it serves them with a **thematically coherent cost** - not by
being mechanically clever. The mechanics stay flexible; what must hold is that the
costs read true.

**The opening shape (round one).** The vanguard's crossers strike ahead to
**disrupt** the enemy's soft-but-dangerous rearguard - assassinate it if they can,
but at minimum tie it up - buying the main army safety from that fire. The price is
real: a crosser becomes an outrider, exposed inside the enemy ranks and liable to
be wiped out earlier than it would have been in formation. Behind that screen the
vanguards (and whatever tempo the rearguards keep) exchange their damage; and once
an enemy vanguard collapses, its rearguard is exposed and gets cleaned up.

**The ongoing shape (later rounds).** Everyone pays the consequences of where they
stand. Outriders are the sharpest expression of it: loose inside the enemy, they
wreak havoc but are vulnerable to everyone at once.

**Disruption, not a guaranteed kill.** Reaching a rearguard does NOT guarantee a
hit - it may still dodge (the raid strike is evadable, by design). The outrider's
job is to disrupt the rearguard's damage: by killing it, *or* by keeping it
occupied - burning its tempo, threatening it - until the enemy vanguard falls and
the exposed rearguard is reached and finished. The kill is only one form of the
disruption, which is why the raid is not guaranteed.

## Terminology

- **Vanguard** - front rank, strikes in **melee**.
- **Rearguard** - back rank, strikes at **range**; screened while its own
  vanguard stands.
- **Outrider** - a body loose *inside* the enemy formation, having crossed in.
- **Tempo** - each body's pool of action cards, refreshed to Cadence at round
  end. Every bid and every strike spends tempo.
- A **bid** is weighted by **Finesse** (and, for a catch, multiplied by the
  catcher's **body count** - a horde catches with many hands). A **strike** does
  **Might** damage. **Engaging earns one free opening strike** - the clash, or the
  shot - and every extra strike costs one more tempo. A **melee** engagement is
  two-way (it can be answered); a **ranged** shot is one-way (it cannot). See the
  engagement rule under Global rules.

## The round: two declare waves, then the rings resolve

A round is **two declare/reveal waves** (the decisions), then one deterministic
resolution through the three distance **rings**, nearest-first (the schedule).
The combat log prints these exact coordinates - `[round N - declare acts]`,
`[round N - declare catches]`, then `[ring 1] INNER` / `[ring 2] CROSSING` /
`[ring 3] OUTER` with numbered sub-phases - so a transcript line always locates
itself as *round . wave-or-ring . sub-phase*.

### Wave 1 - declare acts

Every living body (hero and foe, through one loop) declares its one **act**:

- **Clash** an enemy across the gap (a vanguard, or an exposed rearguard);
- **Cross** into the enemy line - a raid (with an on-arrival target) or a bare
  slip - carrying its two crossing answers, declared independently: the
  [`Answer`] to the vanguard (slip / push / halt-with-strike-back) and the
  [`Volley`] answer to the rearguard (dodge / eat) - the evade-priority split;
- **Melee** a body in its own region (an intruder, or - as an outrider - a host);
- **Retreat** (outrider only): strike or not, then withdraw at the Inner Ring
  boundary;
- **Hold**.

REVEAL: when the last body declares, every act stands revealed - **the crossings
included**.

### Wave 2 - declare catches (only if somebody crossed)

Catching is **elective, and additive**. Each eligible body - living, holding a
line (**Vanguard or Rearguard**; an outrider holds no line), and not itself
crossing - declares its **catch**:

- **intercept** (a vanguard) or **volley** (a rearguard) ONE named enemy
  crosser - several catchers may gang one crosser;
- or **let them pass**.

A catch is an engagement **in addition to** the body's act, priced in tempo -
never a replacement for its strike (measured: making the catch consume the act
collapsed two balance corners). An **area** catcher's catch sweeps the whole
enemy crossing band (area is width). Scripted foes declare by the catch instinct
(`foe_catch`: always answer a crossing, at the crosser you most disrupt); a
player - and the solver - may catch anyone, or decline and bank the tempo.

### Resolution - the three rings, nearest-first

The ring order *is* the silencing rule: a body killed in an earlier ring is gone
from every later ring's strikes. Deaths finalize at each sub-phase boundary;
damage piles close there too.

**[ring 1] INNER - bodies already point-blank.**
- **1.1 Outriders**: every prior-round outrider and its hosts trade the strikes
  they declared (`Melee`, and a `Retreat`'s optional strike) - one simultaneous
  exchange, no screen, both tiers reachable, melee and ranged together (nobody
  is closing, so no ranged-first). First for a reason: a kill here opens a hole
  in the line before the crossings run. Afterwards an outrider whose host
  formation is wiped **dissolves** - it rejoins its own line at weapon rank.
- **1.2 Withdraw**: every surviving outrider that declared a `Retreat` leaves -
  rejoining its own line at weapon rank, free; the ring it just stood was the
  price. A body felled in 1.1 never leaves.

**[ring 2] CROSSING - this round's crossers close into a formation.** Two
independent contests, then the land, then the raid:
- **2.1 Intercept** - "am I halted?", the only contest that decides
  through-vs-stay. The declared vanguard catchers pool their bids
  (`tempo x Finesse x bodies`, summed); the crosser's `Answer` decides:
  **slip** (out-bid the whole pool: through untouched), **push** (caught - eat
  each catcher's free opening strike, cross anyway), or **halt** (caught - stay,
  earn one free blow at a melee catcher plus the declared paid strike-back;
  melee catchers only, never a rearguard it never reached).
- **2.2 Volley** - "am I hit?", damage only, never halts. The declared rearguard
  catchers pool their shots; the crosser's `Volley` answer decides: **dodge**
  (out-bid the pool, tempo spent) or **eat** (shots land, one-way, never
  answered). A crosser felled in 2.1 is not volleyed - the front kill saves the
  back its shot.
- **2.3 Land** - survivors that pushed or slipped arrive: into an enemy region
  as an **Outrider**; a halted crosser stays home.
- **2.4 Raid** - each arrival strikes the back-line target its `Cross` named,
  before that target can fire in the Outer Ring. Evadable: a reached rearguard
  may still dodge, spending tempo it then cannot fire with - the outrider
  disrupts whether or not the blow lands. No retaliation: the rearguard had its
  shot in 2.2.

**[ring 3] OUTER - the standing formations trade across the gap.**
- **3.1 Fire** - every rearguard's declared `Clash` lands (holding off IS being
  quicker: an arrow lands before a swordsman closes). An exposed rearguard keeps
  this first-shot slot even with its vanguard fallen.
- **3.2 Clash** - every vanguard's declared `Clash` lands. There is no separate
  retaliation anywhere in the round: a body answers an attacker by having
  declared its own strike at it - a mutual clash trades both ways. Fight what
  you declared.

### Round end

Tempo refreshes to Cadence (leftover does not carry). Damage piles close - an
unfinished wound is gone. Deaths finalize; a rearguard whose vanguard just fell
is now **exposed** (directly clashable next round, but it keeps its rank and its
3.1 first-shot slot).

## Global rules that cut across the steps

- **Engaging melee earns one free blow; ranged is one-way.** Whoever engages in
  melee - the vanguard catching, the crosser that *halts* to fight, anyone who
  clashes - lands one free opening strike (the clash itself). "Answered in kind"
  means the other side likewise *engages* (a mutual clash it declared, or a
  crosser that halts) - not a bolt-on retaliation. A body firing from **range**
  lands its shot but is **never answered**: you cannot strike back at something you
  never reached. This one rule is behind the catcher's free blow, the crosser's
  free blow *only when it halts* (fleeing through earns nothing - it did not
  engage), and a rearguard's immunity to a crosser's strike-back.
- **No redundant strike-backs.** Each body's one defensive chance is already spent
  in an earlier phase: the vanguard's catch, the rearguard's volley, everyone's
  declared clash. So a *strike-back* rule is added only where a body has **no
  earlier chance** - and the only such body is the **crosser**, whose entire turn
  is the crossing. That is why the halt strike-back exists and nothing else does; a
  reached rearguard does not answer (it shot during the volley), and a clash is
  answered by declaring your own clash, not by a separate rule.
- **Area strikes never target and never retaliate.** An area (aoe) body's strike
  is *always* the untargeted regional sweep - it hits every enemy in the tier it
  is aimed at, unevadably, for one tempo. It is never a declared single-target
  strike, never an extra strike poured onto one body, and never a retaliation. An
  area body participates in whichever phase it acts as a **sweep**, nothing else.
- **Withdrawal is priced, not banned.** An outrider may withdraw: strike in the
  Inner Ring (or not) and rejoin its own line at the boundary, at weapon rank. The
  rank change itself is FREE - the price is standing one more Inner Ring among the
  hosts, where every body around it had its declared chance to strike. Raids can
  therefore be round-trips; whether one is worth it is a read of the ring you must
  survive to leave. *(Demoted 2026-07-20: "a crossing is committed, no retreat."
  Commitment was a means to simplicity, not a goal - the schedule now prices the
  exit instead of banning it. Measured: the diagonal held 4/4 + 5/5 with
  withdrawal in the search space, so the ring alone is a sufficient price.)*
- **The screen is a price, not a wall.** A vanguard can never *stop* a crosser -
  only make it pay, in tempo (evade), in blood (push), or in the ground it gives
  up (halt).

## The round at a glance

| Coordinate | Who | What |
|---|---|---|
| wave 1 - declare acts | every living body | one act: Clash / Cross (with its two crossing answers) / Melee / Retreat / Hold. Reveal: crossings stand |
| wave 2 - declare catches | each living V/R not itself crossing | intercept (V) / volley (R) ONE named enemy crosser, or let them pass. Additive, tempo-priced; aoe sweeps the band |
| 1.1 Outriders | prior outriders + hosts | point-blank exchange (declared Melee / Retreat strikes); both tiers, no screen; then dissolve |
| 1.2 Withdraw | retreating outriders | survivors rejoin their own line at weapon rank - free; the ring was the price |
| 2.1 Intercept | vanguard catchers vs crossers | pooled catch-bid vs the crosser's Answer: slip / push / halt (free blow + paid strike-back, melee catchers only). Decides through-vs-stay |
| 2.2 Volley | rearguard catchers vs crossers | pooled shot vs the crosser's Volley answer: dodge / eat. Damage only, never halts; independent of 2.1 |
| 2.3 Land | surviving crossers | arrive as Outriders (halted crossers stayed home) |
| 2.4 Raid | the arrivals | strike the named back-line target, before it fires; evadable; no retaliation (it volleyed in 2.2) |
| 3.1 Fire | rearguards | declared Clash lands (an arrow lands before a swordsman closes; an exposed back keeps this slot) |
| 3.2 Clash | vanguards | declared Clash lands; a mutual clash is the only "retaliation" - fight what you declared |

## A worked round (illustrative log)

A two-round fight showing the two waves, the crossing (both contests), the two
shapes, and the Inner Ring - each block tagged with the wave or ring.sub-phase it
realizes, the same coordinates the combat log prints. **Illustrative**: faithful
to the mechanics and the numbers (backed by the `worked_round_example` test), but
a clean rendering, not the app's exact transcript. Every body has **Finesse 1**
and **Grit 1**, so a bid = tempo spent and a flip = one point of Might penetrated
(kept trivial on purpose).

```
The board (party vs foes):
  Raider   me   Might 3  Vit 5  Grit 1  Cadence 4   (melee vanguard)
  Wall     *    Might 1  Vit 3  Grit 1  Cadence 2   (melee vanguard - screens)
  Sniper   *    Might 3  Vit 2  Grit 1  Cadence 2   (ranged rearguard - behind the Wall)

[round 1 - declare acts]                                         [wave 1]
  commit  Raider  -> Raid the Sniper (push through; dodge the arrows)
  commit  *Wall   -> Hold        (holding its ground this round, for clean numbers)
  commit  *Sniper -> Hold
  reveal: the Raider is crossing.

[round 1 - declare catches: Raider crossing]                     [wave 2]
  commit  *Wall   -> intercept the crossing Raider
  commit  *Sniper -> volley the crossing Raider

[ring 2] CROSSING
  2.1 Intercept  ("am I halted?")
    Wall bids 1 to catch (1 tempo x Finesse 1).  Raider would pay
    pool 1 / Finesse 1 + 1 = 2 tempo to slip -> it declared PUSH instead.
    Caught. Wall's opening strike: Might 1 penetrates Grit 1 -> Raider flips 1
    (5 -> 4 hp).  (Push, so no strike-back.)
  2.2 Volley  ("am I hit?" - independent of 2.1)
    Sniper bids 1 to volley.  Raider declared DODGE: pays 2 tempo (4 -> 2) ->
    arrows miss.
    >> the evade-priority split: Raider ATE the trivial line but DODGED the
       deadly volley - a combination one welded answer could not make.
  2.3 Land
    Raider: pushes through the line, now an Outrider beside the Sniper (4 hp).
  2.4 Raid
    The Sniper has only 1 tempo left (it spent one volleying) - not enough to
    dodge the raid, which needs 2 -> it cannot dodge.
    Raider strikes: Might 3 penetrates Grit 1 -> Sniper flips 2 (2 -> 0).
    *** The Sniper is DOWN. ***

  (Round 1 ends: Sniper dead. Raider stands as an Outrider inside the foe
   line, beside the Wall - exposed, but it silenced the dangerous back. That
   is the opening shape.)

[round 2 - declare acts]                                         [wave 1]
  commit  Raider  -> Melee the Wall
  commit  *Wall   -> Melee the Raider
  (nobody crossing -> no catch wave)

[ring 1] INNER
  1.1 Outriders  (the outrider and its host, point-blank)
    Tempo resets. No screen; both declared their melee.
    Raider melees Wall: Might 3 penetrates Grit 1 -> Wall flips 3 (3 -> 0). DOWN.
    Wall melees Raider: at point-blank it spends its whole pool - one opening
    strike plus one poured - Might 1 each -> Raider flips 2 (4 -> 2).
    The Wall's formation is gone -> the Raider DISSOLVES, rejoining its own line.

========================== WIN ==========================
```

**The road not taken (2.1, if the Raider had declared HALT instead of Push):**

```
  2.1 Intercept  (halt)
    Raider HALTS - it engages, so it earns one FREE blow at the Wall:
      Might 3 penetrates Grit 1 -> Wall flips 3 (3 -> 0). Wall DOWN.
    But it STAYS home - it does not become an outrider and never reaches the
    Sniper this round. (Its 2.2 volley answer is still its own choice.)
```

**And the round-trip (round 2, if the Raider had declared Retreat instead):**

```
[ring 1] INNER
  1.1 Outriders
    Raider strikes the Wall on its way out (a Retreat's strike is a normal
    inner-ring melee); the Wall's declared blows land on the Raider - the
    price of leaving.
  1.2 Withdraw
    Raider: withdraws from the enemy ranks, rejoining its line as a Vanguard.
```

That is the crosser's core decision in one line: **push** to advance (reach and
silence the back, at the cost of exposure) versus **halt** to fight the line
(kill the front, but give up the ground) - the two shapes, chosen a body at a
time.

## Implementation status (2026-07-20)

**The sequence above IS the shipped model** (`rules::combat`): the two declare
waves are literally the `Game`'s structure (`Choice::Act` / `Choice::Catch`),
resolution runs the rings in `play_round`, and the combat log prints the same
coordinates. The diagonal gate held 4/4 solos + 5/5 party fights through every
delta (withdrawal; the catch wave; the free blow on halt; the evade-priority
split), with zero re-tuning.

What remains, deliberately deferred:

- **Bid sizing** - every bid (catch, slip, dodge, raid-evade) auto-sizes today
  (`reach_cards` / min-to-beat-the-pool): the *whom* is declared, the *how hard*
  is computed. Freeing the amounts is behavior-card territory (foes) and a
  decision-richness add (party).
- **Decoupled raid targeting** - *presentation only*: the raid target rides the
  `Cross` act; against deterministic foes that is equivalent to naming it on
  arrival. A UI two-beat if wanted; no rule change.
- **Clash evade as a choice** - a clashed body's evade is the automatic greedy
  (`dodges_against`); making it a declared bid is the same fold-out the crossing
  got, if playtesting ever wants it.

## History (how this model settled, 2026-07-20)

The sequence grew from a brainstorm that factored combat into one **Interaction
primitive** (target -> contact bid -> strike with free opening + paid extras ->
resolve) applied over a **rank-pair schedule** - position IS rank, given one
region per side. Its deltas were implemented and measured one at a time against
the diagonal gate:

1. **Withdrawal (O->V)** - landed clean; the "a crossing is committed, no
   retreat" tenet was demoted (a means to simplicity, not a goal). The Inner
   Ring alone proved a sufficient price.
2. **Catch = the clash you declared (one act, one strike)** - REVERTED on
   measurement: the Sweep and Raid corners collapsed to unwinnable.
   **CAVEAT (added 2026-07-20): that experiment was CONFOUNDED.** It bundled
   three changes - (a) the catch consumes the catcher's act, (b) no strike-phase
   extras for catchers (the pile-on was unimplemented, so a cheap catch left the
   rest of the pool STRANDED - an implementation artifact, not a property of the
   act-consuming design), and (c) catch targeting moved from everyone-catches to
   only-your-declared-target. The collapse condemns that bundle; the clean
   comparison (act-consuming WITH the strike-phase pour) was never run. With the
   pour in place, the two models differ only in **split-freedom**: whether one
   body may divide its pool across two targets in the same round.
3. **The catch wave** - a genuine second declaration per body, additive and
   tempo-priced. Landed clean; the solver searches it, foes play the catch
   instinct.

**Canon ruling (2026-07-20): the additive model, with split-freedom.** A body may
be in as many engagements as its tempo funds - its act, a catch, and the pours -
and may split its pool across targets if it can afford to (at high Finesse that
may even be the strong play). The only requirement on creatures is DETERMINISM:
a creature may carry a rule for how it splits, but need not have one.

Open calls were resolved conservatively: ranged fire keeps preceding the melee
clash ("an arrow lands before a swordsman closes"), and the pooled contest is
the crossing's rule. Promote-to-canon criterion was "deltas in, diagonal green" -
both hold; this document now describes the shipped model. (In flight: the
catcher's strike-phase pile-on - implemented with a finish-the-runner default,
currently under balance measurement; see the working tree.)