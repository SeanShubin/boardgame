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

**Step 3 - DECLARE the crossing bids (two contests, not one).** The vanguard and
the rearguard answer *different questions*, so they are two independent pooled
contests - never one combined pool:

- **Interception - "am I halted?"** The vanguard on the crosser pool their catch
  bids (`tempo x Finesse x bodies`, summed); the crosser bids tempo to slip the
  front. Beat the pool -> through the front untouched; fall short -> **caught**
  (into Step 4's push/halt).
- **Volley - "am I hit?"** The rearguard on the crosser pool their shot bids the
  same way; the crosser bids tempo to dodge. Beat the pool -> dodged; fall short
  -> **hit**. Either way it keeps crossing - a volley *damages*, it does not halt.

The crosser **allocates its evade tempo between the two** - slip the line, dodge
the arrows, or split, and sometimes it can only afford one. REVEAL; resolve both.

**Step 4 - RESOLVE strikes, then DECLARE the answer.** Everyone who connected
lands its **one free opening strike** (Might): each intercepting vanguard (a
melee clash) and each hitting rearguard (a shot). Then, by the engagement rule
(see Global rules):

- **Catchers.** A vanguard may declare **extra** melee strikes, one tempo each; a
  rearguard may loose **extra** shots the same way. Neither can be answered across
  the gap.
- **The caught crosser** declares its posture, and the posture decides its blow:
  - **push** - flee *through*: no strike-back at all (it did not stop to fight).
  - **halt** - *engage*: it stays, and because it is now in a two-way melee it
    earns its own **one free blow** at a catcher of its choice, then may spend
    remaining tempo on more. Its strike-back reaches **only the vanguard in melee
    with it** - never a rearguard it never touched.

REVEAL; land all strikes; check downed. *A crosser taking lethal damage should
halt: as a corpse-in-waiting it should spend its tempo on the bodies that caught
it rather than waste it arriving as an outrider it will not survive to use.*

### Phase 2 - The Raid (the arrivals strike the back)

**Step 5 - DECLARE outrider targets.** Each of **this round's new outriders** (the
crossers that pushed through) declares its target: a rearguard (or any host body
in its region). Prior-round outriders already acted in Phase 0 - here it is only
the fresh arrivals. Vanguard and rearguard declare nothing here; they had their
chance at the crosser in Phase 1. REVEAL.

**Step 6 - DECLARE rearguard evade bids.** A targeted rearguard bids tempo to
evade the outrider on it. REVEAL; resolve.

**Step 7 - RESOLVE the raid.** Outrider strikes land, plus declared extra
strikes. A struck rearguard may declare **retaliation** - as many strikes as it
has tempo for, but **only against bodies that struck it in melee** (an outrider
in its face; never a body firing from across the gap). REVEAL; land; check downed.

### Phase 3 - The Clash (the formations trade)

**Step 8 - DECLARE clash targets.** Each **vanguard and rearguard** declares a
target: an enemy vanguard, an outrider loose in its ranks, or an **unscreened**
rearguard (one whose vanguard has fallen). Outriders already acted in Phase 2;
here they are targets only. REVEAL.

**Step 9 - DECLARE evade bids.** Each targeted body bids tempo to evade. REVEAL;
resolve.

**Step 10 - RESOLVE the clash.** Strikes land, plus extra strikes, plus melee
retaliation along any melee edge. Check downed.

### Round end

Tempo refreshes to Cadence (leftover does not carry). Damage piles close - an
unfinished wound is gone. Deaths finalize; a rearguard whose vanguard just fell
is now **exposed** (directly clashable next round, but it keeps its rank and its
first-shot phase slot).

## Global rules that cut across the steps

- **Engaging melee earns one free blow; ranged is one-way.** Whoever engages in
  melee - the vanguard catching, the crosser that *halts* to fight, anyone who
  clashes - lands one free opening strike (the clash itself) and can be answered
  in kind. A body firing from **range** lands its shot but is **never answered**:
  you cannot strike back at something you never reached. This one rule is behind
  the catcher's free blow, the crosser's free blow *only when it halts* (fleeing
  through earns nothing - it did not engage), and a rearguard's immunity to a
  crosser's strike-back.
- **Area strikes never target and never retaliate.** An area (aoe) body's strike
  is *always* the untargeted regional sweep - it hits every enemy in the tier it
  is aimed at, unevadably, for one tempo. It is never a declared single-target
  strike, never an extra strike poured onto one body, and never a retaliation. An
  area body participates in whichever phase it acts as a **sweep**, nothing else.
- **A crossing is committed.** Once through, an outrider cannot cross back out.
- **The screen is a price, not a wall.** A vanguard can never *stop* a crosser -
  only make it pay, in tempo (evade), in blood (push), or in the ground it gives
  up (halt).

## The declare/reveal steps at a glance

| # | Phase | Who declares | What |
|---|---|---|---|
| I1 | Inner | prior outriders + hosts | point-blank targets (any tier, no screen) |
| I2 | Inner | targeted bodies | evade bids |
| I3 | Inner | attackers + defenders | strikes + extras + retaliation; then dissolve |
| 1 | Crossing | vanguard | cross or not |
| 2 | Crossing | vanguard + rearguard | intercept (melee, halts) or volley (ranged, hits) |
| 3 | Crossing | catchers + crossers | two contests: front (halted?) and back (hit?); evade tempo split between them |
| 4 | Crossing | catchers; crossers | free opening each + extras; crosser push (0) or halt (own free blow + paid, melee catchers only) |
| 5 | Raid | outriders | which rearguard/host to strike |
| 6 | Raid | targeted rearguard | evade bid |
| 7 | Raid | outriders + rearguard | extra strikes; rearguard retaliation |
| 8 | Clash | vanguard + rearguard | clash target |
| 9 | Clash | targeted bodies | evade bid |
| 10 | Clash | attackers + defenders | extra strikes; melee retaliation |

## Implementation status (2026-07-18, after M1)

Where the shipped `rules::combat` model stands against this sequence. "Folded"
means the model already produces the same outcome by carrying the decision on the
up-front act (equivalent against deterministic foes), so only UI theater is
missing; "pending" means a genuine rule is not yet modeled.

| Step | Status | Note |
|---|---|---|
| 0 Inner Ring (prior outriders) | done | resolved first in `play_round`: a distance-zero brawl (both tiers, no screen), then `dissolve` |
| 1 crossings | done | up-front `Cross`, vanguard-only |
| 2 elective catch + volley split | pending | catching is automatic geometry today, and not yet split into melee-interception (halts) vs ranged-volley (hits) (M2) |
| 3 bids (two contests) | partial | M1 runs front and back as separate pooled passes, but the crosser can't split evade between them - it's one `{0, beat-pool}` posture applied to both, and the back pass still gates push/halt rather than damage-only |
| 4 strikes + push/halt + free-blow | partial | push/halt + strike-back allocation shipped (M1); strike-back restricted to **melee catchers** (one-way rule); the crosser's **free blow on halt** now lands; **TODO:** catcher extra strikes; the two-contest split (back = damage only) |
| 5 outrider targets back | different | today the raid target is bundled into `Cross(Some(t))` and resolved in the crossing ring, not a separate post-crossing beat |
| 6 rearguard evade | folded | evade exists but automatic, not a declared bid |
| 7 outrider strike + rearguard retaliation | pending | defender retaliation is only the aborter's today; a ranged back does not yet retaliate at point-blank |
| 8 clash targets | done | Outer Ring clash, up front |
| 9 clash evade | folded | automatic |
| 10 land + downed | done | |
| aoe never targets/retaliates | done | sweep already untargeted; strike-back now excludes aoe (candidates + resolver); pour/clash route aoe to `area_strike` |

The distance left is four structural rules - elective catching split into
interception vs volley (2), the two-contest bid with evade split (3), decoupled
outrider targeting (5), and defender retaliation (7). (The crosser's free blow on
halt, the melee-only strike-back, and the aoe never-target/retaliate invariant are
now done.) Everything else is presentation over a model that already plays it.
