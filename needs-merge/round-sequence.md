# The round sequence

Status: design spec, 2026-07-18. The canonical step-by-step procedure for one
combat round, written so a human can run it at a table. The crossing's bid math
lives in [crossing-bid-tree.md](crossing-bid-tree.md); this document is the
*frame* - the order of play, and who may do what when.

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

## The round: the inner ring, then three phases

The round is the three distance **rings**, resolved **nearest-first** - Inner
(distance zero) -> Crossing (closing) -> Outer (across the gap). The ring order
*is* the silencing rule: a body killed in an earlier ring is gone from every
later ring's strikes. The rings are the resolution *order*; the declare/reveal
steps below are the *decisions* layered on each ring.

### Phase 0 - The Inner Ring (bodies already point-blank)

Resolved FIRST, and first for a reason: a body killed here is gone from every
later phase, so an **outrider** still loose inside an enemy formation from a prior
round can open a hole in the line *before* this round's crossings are declared.

The outrider and the host bodies around it are at distance zero - no screen, no
closing, so no ranged-first: one point-blank brawl. It runs the **same
declare/reveal shape as the Clash** (Phase 3): declare targets (each outrider a
host; each host an outrider), declare evade bids, then resolve with strikes, extra
strikes, and retaliation - except that at distance zero *everything* is reachable
(both tiers, no screen) and even a rearguard firing point-blank **can** be
answered (the one-way rule is a *distance* rule; nothing here is out of reach).
Afterwards, an outrider whose host formation is now wiped **dissolves** - it
rejoins its own line at its weapon rank.

A newly-arrived outrider does **not** act here - it reaches distance zero only
after the crossing, so it strikes in Phase 2. **Each outrider acts once per
round:** a prior-round one here, a this-round one in Phase 2; a survivor becomes a
prior-round outrider and acts here *next* round.

An outrider may also **withdraw** here: strike (or not), then rejoin its own line
at the Inner Ring boundary, at weapon rank - free, because the ring it just stood
was the price (see "Withdrawal is priced, not banned" under Global rules).

### Phase 1 - The Crossing (the front closes the gap)

**Step 1 - DECLARE crossings.** Each side's **vanguard** bodies declare whether
they cross (no target yet - a crossing commits to *going*, not yet to *whom* to
hit). REVEAL.

**Step 2 - DECLARE interceptions and volleys.** A crosser is opposed in two
different ways, declared together but resolved apart:

- Each enemy **vanguard** declares which crosser, if any, it **intercepts** - a
  melee catch, trying to *halt* it.
- Each enemy **rearguard** declares which crosser, if any, it **volleys** - a
  ranged shot, trying to *hit* it before it reaches.

Several of either may pick one crosser. REVEAL.

The crossing is now **two independent crossings**, resolved in order - the line,
then the volley. They answer different questions, and the crosser's choice at each
is made **independently** of the other (that is the evade-priority split: slip the
line but eat the arrows, or take the catch but dodge the arrows, whichever the
tempo and the threats favour).

**Step 3 - CROSS THE VANGUARD (the interception - "am I halted?").** The only
crossing that decides through-vs-stay. The vanguard on the crosser pool their catch
bids (`tempo x Finesse x bodies`, summed); the crosser independently bids tempo to
**slip**. REVEAL.

- **Slipped** (beat the pool): through the front, untouched by it.
- **Caught** (fell short): each catcher lands its one free opening strike (Might)
  and may declare **extra** melee strikes; then the crosser declares its posture -
  - **push**: flee through, no strike-back (it did not engage), or
  - **halt**: stay and engage - it earns its own **one free blow** at a catcher of
    its choice plus any paid strike-back, reaching the **melee vanguard only**.

  Land; check downed. *A crosser taking lethal damage should halt: spend its tempo
  on the bodies that caught it rather than waste it arriving as an outrider it will
  not survive to use.*

**Step 4 - CROSS THE REARGUARD (the volley - "am I hit?").** Decided
**independently** of step 3, and it only ever **damages** - a volley never halts.
The rearguard on the crosser pool their shot bids; the crosser independently bids
tempo to **dodge**. REVEAL.

- **Dodged** (beat the pool): no volley damage (but the dodge tempo is spent).
- **Hit** (fell short): each volleying rearguard lands its shot (Might) and may
  loose **extra** shots - one-way, never answered.

A crosser felled by the vanguard in step 3 is already gone and is not volleyed
(the razor: the front kill saves the back its shot). Land; check downed.

### Phase 2 - The Raid (the arrivals strike the back)

**Step 5 - DECLARE outrider targets.** Each of **this round's new outriders** (the
crossers that pushed through) declares its target: a rearguard (or any host body
in its region). Prior-round outriders already acted in Phase 0 - here it is only
the fresh arrivals. Vanguard and rearguard declare nothing here; they had their
chance at the crosser in Phase 1. REVEAL.

**Step 6 - DECLARE rearguard evade bids.** A targeted rearguard bids tempo to
evade the outrider on it. Reaching the back is **not** a guaranteed hit - the
rearguard may dodge, spending tempo it then cannot fire with. That is the point: an
outrider disrupts whether or not it lands the blow (a dodged raid still burned the
rearguard's tempo). REVEAL; resolve.

**Step 7 - RESOLVE the raid.** Outrider strikes land, plus declared extra strikes;
check downed. The rearguard does **not** retaliate: it already had its one shot at
this body - the volley in step 4. Being reached is the raid's whole reward - a bow
is helpless with a blade inside its guard - so the archer's defense was to kill the
crosser *before* it arrived, not after. (No separate defender-retaliation rule is
needed; see "no redundant strike-backs" under Global rules.)

### Phase 3 - The Clash (the formations trade)

**Step 8 - DECLARE clash targets.** Each **vanguard and rearguard** declares a
target: an enemy vanguard, an outrider loose in its ranks, or an **unscreened**
rearguard (one whose vanguard has fallen). Outriders already acted in Phase 2;
here they are targets only. REVEAL.

**Step 9 - DECLARE evade bids.** Each targeted body bids tempo to evade. REVEAL;
resolve.

**Step 10 - RESOLVE the clash.** Strikes land, plus declared extra strikes; check
downed. There is no separate retaliation: a body that wants to answer an attacker
simply **declared a clash at it** (step 8), and a mutual clash already trades both
ways. A body that declared elsewhere, or held, spent its turn there. Fight what you
declared.

### Round end

Tempo refreshes to Cadence (leftover does not carry). Damage piles close - an
unfinished wound is gone. Deaths finalize; a rearguard whose vanguard just fell
is now **exposed** (directly clashable next round, but it keeps its rank and its
first-shot phase slot).

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

## The declare/reveal steps at a glance

| # | Phase | Who declares | What |
|---|---|---|---|
| I1 | Inner | prior outriders + hosts | point-blank targets (any tier, no screen) |
| I2 | Inner | targeted bodies | evade bids |
| I3 | Inner | attackers + defenders | strikes + extras (mutual, both declared); then dissolve |
| I4 | Inner | outriders | withdraw (free rank change at the boundary; the ring was the price) |
| 1 | Crossing | vanguard | cross or not |
| 2 | Crossing | vanguard + rearguard | intercept (melee, halts) or volley (ranged, hits) |
| 3 | Crossing | vanguard + crosser | cross the vanguard: catch-bid vs slip; if caught -> push or halt (free blow + paid, melee only). Decides through vs stay |
| 4 | Crossing | rearguard + crosser | cross the rearguard: volley-bid vs dodge; damage only, never halts. Chosen independently of step 3 |
| 5 | Raid | outriders | which rearguard/host to strike |
| 6 | Raid | targeted rearguard | evade bid |
| 7 | Raid | outriders | extra strikes (rearguard does NOT retaliate - it volleyed in step 4) |
| 8 | Clash | vanguard + rearguard | clash target |
| 9 | Clash | targeted bodies | evade bid |
| 10 | Clash | attackers | extra strikes (a mutual clash is answered by declaring it, step 8 - no separate retaliation) |

## A worked round (illustrative log)

A two-round fight showing the crossing (both contests), the two shapes, and the
Inner Ring - each block tagged with the step above it realizes. **Illustrative**:
faithful to the mechanics and the numbers, but a clean rendering, not the app's
exact transcript. Every body has **Finesse 1** and **Grit 1**, so a bid = tempo
spent and a flip = one point of Might penetrated (kept trivial on purpose).

```
The board (party vs foes):
  Raider   me   Might 3  Vit 5  Grit 1  Cadence 4   (melee vanguard)
  Wall     *    Might 1  Vit 3  Grit 1  Cadence 2   (melee vanguard - screens)
  Sniper   *    Might 3  Vit 2  Grit 1  Cadence 2   (ranged rearguard - behind the Wall)

================================ ROUND 1 ================================

DECLARE
  Raider -> Raid the Sniper                                      [Step 1]
  the line answers: Wall intercepts Raider, Sniper volleys it    [Step 2]

CROSS THE VANGUARD  (the interception - decides through vs stay)  [Step 3]
  Wall bids 1 to catch (1 tempo x Finesse 1).  Raider would pay
  pool 1 / Finesse 1 + 1 = 2 tempo to slip -> declines: PUSH.
  Caught. Wall's opening strike: Might 1 penetrates Grit 1 -> Raider flips 1
  (5 -> 4 hp).  (Push, so no strike-back.)

CROSS THE REARGUARD  (the volley - damage only, chosen independently)  [Step 4]
  Sniper bids 1 to volley.  Raider DODGES: pays 2 tempo (4 -> 2) -> arrows miss.
  >> the evade-priority split: Raider ATE the trivial line but DODGED the
     deadly volley - a combination one welded answer could not make.
  Through the front -> Raider is now an Outrider beside the Sniper (4 hp).

THE RAID  (the arrival strikes the back)                         [Steps 5-7]
  The Sniper has only 1 tempo left (it spent one volleying) - not enough to
  dodge the raid, which needs 2 -> it cannot dodge.                [Step 6]
  Raider strikes: Might 3 penetrates Grit 1 -> Sniper flips 2 (2 -> 0).
  *** The Sniper is DOWN. ***                                    [Step 7]

  (Round 1 ends: Sniper dead. Raider stands as an Outrider inside the foe
   line, beside the Wall - exposed, but it silenced the dangerous back. That
   is the opening shape.)

================================ ROUND 2 ================================

THE INNER RING  (the outrider and its host, point-blank)         [Phase 0]
  Tempo resets. Raider (Outrider) and the Wall trade at distance zero - no
  screen, both declared a melee.
  Raider melees Wall: Might 3 penetrates Grit 1 -> Wall flips 3 (3 -> 0). DOWN.
  Wall melees Raider: at point-blank it spends its whole pool - one opening
  strike plus one poured - Might 1 each -> Raider flips 2 (4 -> 2).
  The Wall's formation is gone -> the Raider DISSOLVES, rejoining its own line.

========================== WIN ==========================
```

**The road not taken (Step 3, if the Raider had HALTED instead of Push):**

```
CROSS THE VANGUARD  (halt)                                       [Step 3]
  Raider HALTS - it engages, so it earns one FREE blow at the Wall:
    Might 3 penetrates Grit 1 -> Wall flips 3 (3 -> 0). Wall DOWN.
  But it STAYS home - it does not become an outrider and never reaches the
  Sniper this round. (Its Step 4 volley answer is still its own choice.)
```

That is the crosser's core decision in one line: **push** to advance (reach and
silence the back, at the cost of exposure) versus **halt** to fight the line
(kill the front, but give up the ground) - the two shapes, chosen a body at a
time.

## Implementation status (2026-07-18, after M1)

Where the shipped `rules::combat` model stands against this sequence. "Folded"
means the model already produces the same outcome by carrying the decision on the
up-front act (equivalent against deterministic foes), so only UI theater is
missing; "pending" means a genuine rule is not yet modeled.

| Step | Status | Note |
|---|---|---|
| 0 Inner Ring (prior outriders) | done | resolved first in `play_round`: a distance-zero brawl (both tiers, no screen), then `dissolve` |
| I4 withdrawal (O->V) | done | `Act::Retreat(Option<target>)`: optional inner-ring strike, then rejoin at the boundary; diagonal held 4/4 + 5/5 with it searchable |
| 1 crossings | done | up-front `Cross`, vanguard-only |
| 2 elective catch + volley split | done | the CATCH WAVE: a second declaration per round - each eligible body (V/R, living, not crossing) names ONE enemy crosser to intercept (vanguard) / volley (rearguard), or declines; tempo-priced, additive to its act. `Choice::Catch`, `foe_catch` instinct, solver searches the party's real catch options; diagonal held |
| 3 cross the vanguard | done | `Act::Cross(_, Answer, _)` - slip/push/halt, free blow, melee-only strike-back; decides through-vs-stay |
| 4 cross the rearguard | done | `Act::Cross(_, _, Volley)` - dodge/eat, damage only, chosen INDEPENDENTLY of the front (the evade-priority split); `legal_acts` enumerates Front x Volley |
| 5 outrider targets back | different | today the raid target is bundled into `Cross(Some(t))` and resolved in the crossing ring, not a separate post-crossing beat |
| 6 rearguard evade | folded (by design) | the raid is evadable - a reached back may dodge (spending firing tempo); the outrider disrupts either way. Evade exists but automatic, not yet a declared bid |
| 7 outrider strike (no retaliation) | done (rule dropped) | the rearguard does not retaliate - it had its shot in the volley (step 4); no defender-retaliation rule needed (see "no redundant strike-backs") |
| 8 clash targets | done | Outer Ring clash, up front |
| 9 clash evade | folded | automatic |
| 10 land + downed | done | |
| aoe never targets/retaliates | done | sweep already untargeted; strike-back now excludes aoe (candidates + resolver); pour/clash route aoe to `area_strike` |

The distance left:

- **Decoupled outrider targeting (5)** - *presentation only*: against deterministic
  foes, declaring the raid target up front (today's bundled `Cross(Some(t))`) is
  equivalent to picking it on arrival. A UI two-beat, no rule change.
- **Catch-bid sizing / behavior cards** - the catch wave declares WHOM to catch;
  how hard (the bid) still auto-sizes (`reach_cards`). Sizing it, and richer foe
  catch policies, is behavior-card territory.

Elective catching (2) is **done** - the catch wave, a genuine second declaration
per round, measured green. Defender retaliation (7/10) is **dropped** (covered by
the volley and the declared clash). The crosser's free blow on halt, the melee-only
strike-back, and the aoe never-target/retaliate invariant are done. Everything else
is presentation over a model that already plays it.

## Brainstorming

*Status 2026-07-20: adopted as the target model, pending balance verification of
each delta. The Interaction primitive matches the engine's existing
engage/evade/land physics; the rank-pair schedule replaces rings+regions (position
IS rank, given one region per side). Deltas being implemented and measured one at
a time: (1) O->V withdrawal - DONE, diagonal held, tenet demoted; open calls
resolved conservatively unless overridden: ranged fire keeps preceding the melee
clash ("an arrow lands before a swordsman closes"), and the pooled contest is the
universal rule. Promote to canon when the deltas are in and the diagonal holds.*

*MEASURED FINDING (2026-07-20), delta 2 attempt - "catch = the clash you
declared, one act one strike": REVERTED. Making the catch CONSUME the catcher's
act (its declared Clash resolves as the interception and never strikes again)
collapsed two corners to unwinnable - Sweep (the area tool itself loses) and Raid
(the full party loses) - because the party's razor-thin wins depended on the
defense's catch/volley being an ADDITIONAL, tempo-priced engagement, not a
replacement for its offense. This is not a tuning miss; it is a semantic
disagreement with the sequence above: steps 2-4 price the catch in TEMPO, and
step 8 is a separate declaration - a body may both catch and clash in one round,
budgeted by its pool. The brainstorm agrees (each step has its own declarations).*

*RESOLVED same day: the per-step declaration surface is BUILT - the round is now
two declare waves (acts, then catches), with the catch a genuine second
declaration per body, tempo-priced and additive. The solver searches the party's
real catch choices (whom, or declining); scripted foes and the greedy baseline
play the catch instinct (`foe_catch`: always answer, at the crosser you most
disrupt). Diagonal held 4/4 + 5/5 with the wave fully searchable. Steps 1-2 of
the sequence are now literally the machine's structure.*
- Interaction
  - Target
    - Everyone declare valid targets, then reveal
  - Contact
    - Tempo bids for evading vs. catch.  Finesse matters, must strictly beat catch.  Then reveal
  - Strike
    - May ignore hit
    - May stop and redirect bit to strike
    - May spend additional tempo for additional strike per temp (contact already established, finesse irrelevant)
  - Resolve
    - Check for downed
- Steps
  - Interactions on Outriders vs Rearguard and Vanguard, an Reguard and Vanguard vs Outriders
  - Outriders may change to vanguard (no cost)
  - Interactions between vanguard and vanguard
  - Vanguard may change to Outriders if they did not strike
  - Interactions from Rearguard to Outriders (one way)
  - Interactions from Outriders to Rearguard (one way)
  - Interactions from Rearguard and Vanguard to Vanguard
  - Interactions from Rearguard and Vanguard to Rearguard (if no enemy vanguard)