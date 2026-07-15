# Regions combat, reorganized: engagement by geometry

**Status:** exploration, staged for merge. Author: one instance, 2026-07-14.
**Supersedes the *schedule* of** `regions-and-relations-combat.md` (the five-beat crossing
gauntlet). Keeps everything else that document argued: formation declared once, position earned
not declared, slipping as the one movement, the solver/`Game` seam untouched.

**Question:** the current round is a fixed **five-beat timeline** (Intercept -> Volley -> Raid ->
Fire -> Clash) -- a schedule of *when*. It works, but the beats are a flat list you have to
memorize, and a raid's only lasting mark is that the raider ends up standing somewhere. Can the
same fights come out of a smaller set of rules that a player can *derive* rather than memorize,
and can a raid leave a mark that matters for more than one round?

**Answer this document argues:** yes. Stop scheduling by *when* and organize by **where the
attacker and target stand** -- three engagement kinds, each built from one shared resolve loop --
and let a landed raid leave the raider **loose inside the enemy formation**, wreaking havoc every
round until it is dug out. One timing principle then orders the whole round, and it is a
principle a player already has an intuition for: *whatever connects first, goes first.*

---

## 1. The core: engagement is chosen by geometry

A body never picks an "attack type." It picks a **target**, and where that target stands
relative to it decides which of three engagements happens. All three are the same three beats --
the "inner three" the code already has -- so there is really only one combat primitive, run in
three geometries.

| Engagement | Attacker and target are... | Reaches | Because |
|---|---|---|---|
| **Clash** | in **different** regions | the enemy **vanguard** only | you cannot see past their screen from across the gap |
| **Raid** | you go for a **screened rearguard** in another region | that **rearguard** | you physically cross over -- and pay for it on the way in |
| **Melee** | in the **same** region (an intruder is loose inside) | **anyone** in the region | there is no screen between bodies already intermingled |

### The one primitive every engagement runs

1. **Target.** Declare who you are reaching for.
2. **Evade.** Each target answers what reaches it: **Evade** (pay the slip cost, untouched),
   **Push** (pay nothing, eat the blows, act anyway), **Abort** (turn and fight, stay put).
3. **Strike.** The opening blow the reach bought, plus **one extra strike per remaining tempo**
   poured into the same declared target.

**AoE is the *width* of this primitive, never extra *reach*** (the rule the current model already
states, now applied uniformly): an area strike catches **every enemy the single-target attack
could have reached in that same engagement**, and it may pulse **once per tempo**.

| In a... | a single blow hits | a sweep hits |
|---|---|---|
| **Clash** | one enemy vanguard | the **whole enemy vanguard line** (never the rearguard -- still screened) |
| **Raid** | the rearguard you came for | the **whole rearguard line** you are now standing among |
| **Melee** | one body in the region | **every enemy in the region**, front and back alike |

A sweep in a melee catching "both tiers" is not the sweep reaching deeper -- it is that the tiers
stopped protecting anyone the moment a body got *inside* them. Reach is what the screen governs;
width is what a sweep governs; they are different axes and a sweep buys only the second.

---

## 2. The intruder: a raid's lasting mark (the new idea)

When a raider survives the crossing and lands, it does **not** dissolve the formation it entered.
**The host keeps its front and back, its screen, everything.** The raider is simply a **loose
body inside those ranks** -- past the screen, so it strikes **anyone** it likes, and adjacent to
everyone, so **anyone** can strike it. It stays there, a splinter in the formation, until it is
killed or **slips back out**.

That is the whole payoff of a raid, and it is much bigger than "the raider ends up standing
somewhere":

- Even a raid that does not kill the cannon it went for leaves a body **parked next to that
  cannon**, threatening it **every round** thereafter.
- The defender is **taxed**: it must spend actions digging the intruder out -- actions it is not
  spending on the fight across the gap.
- An intruder carrying a **sweep** is devastating, because it is inside the screen: its area
  strike catches the host's front *and* back at once (see the Melee row above). That is the price
  of letting one in.

An intruder is not a fourth rank. It is the old **Outrider** made persistent: a third position,
*in enemy territory, unscreened*, that the geometry -- not a keyword -- creates and destroys. An
intruder has **no post** while it is loose: post describes a zone's own formation, and an intruder
is not part of the formation it invaded.

**Zone promotion -- clearing ground takes it.** When intruders kill **every** body of the
formation that owns a zone, that zone **flips to their side** on the spot -- the resolution step
where the last defender falls (the same "on the spot" as vanguard-promotion, not deferred to the
Reset). The erstwhile intruders are now the zone's formation, taking their weapon-fixed posts
(below). This gives a raid a *territorial* payoff and lets a side be pushed back zone by zone. It
mostly bites when an enemy formation spans **more than one zone**; against a single-zone warband,
clearing the only enemy zone simply *is* the win.

---

## 3. The timing principle: whatever connects first, goes first

The whole round order falls out of one idea -- **resolve engagements in the order their blows
land** -- which is just *distance*:

- An **intruder already in your ranks** is distance zero. It is swinging before anyone can turn.
  **First.**
- An **arrow across the gap** is fast: the shot travels, the body does not.
- A **body that must close** -- a swordsman clashing the gap, an outrider landing its strike -- is
  slowest. **Last.**

And the two orderings that used to look like separate rules are the *same* rule seen from two
directions:

- Something coming **at** your region meets the nearest defender first -- your **front rank** ->
  *spears before bows* (front intercepts a crosser, then the back volleys).
- **You** reaching **across** a gap connect soonest with your **arrow** -> *bows before swords*
  (Fire before Clash).

Both are "whatever connects first goes first." Not two rules -- one rule, two frames. A player who
grasps *that* can re-derive the entire schedule instead of memorizing five beats.

---

## 4. The round schedule

Three distance bands, nearest first. Deaths finalize at each step boundary, so a body killed
early is **silenced** in every later step (the razor: a step earns its place only by letting a
death in it silence something after it).

### Band 1 -- Intruders (distance zero)

Every region that contains a loose enemy body resolves its in-place fight: the intruder's havoc
and the host's answer, the shared primitive with **no screen** between them.

- *A death here silences:* an intruder that kills a host's back-line body silences that body's
  **Fire** in Band 3; the host killing the intruder ends its havoc from the **next** round on.

### Band 2 -- Crossings (distance: closing into a formation)

Every declared raid sends its body across as an **outrider**, running the gauntlet:

1. **Intercept** -- the enemy **front rank** (nearest to the incoming edge) reaches for the
   outriders. *A death here silences the Volley and the outrider's strike.*
2. **Volley** -- the enemy **back rank** looses at whoever the spears did **not** already drop
   (the front rank was first contact, so the bows never fire at a body that never reached them).
   *A death here silences the outrider's strike.*
3. **Land & strike** -- surviving outriders strike the rearguard they came for, and **remain in
   that region as intruders** for Band 1 of the next round.

### Band 3 -- Across the gap (distance: reaching over open ground)

The standing formations trade at each other's **vanguards** (the screen keeps the rearguards out
of reach from across a gap):

1. **Fire** -- every back line looses. *A death here silences the target's Clash.*
2. **Clash** -- every front line closes and trades. Slowest, so last.

**Ordering choice (tweakable):** Band 2 (crossings) resolves before Band 3's Fire -- a body
sprinting into your lines is treated as more urgent than a duel across the field. Chosen for a
clean nearest-to-farthest ladder; easy to swap to "all ranged together" later if the archer math
wants it.

---

## 5. Placement, post, and what a body may declare

**Post is not declared -- it is the weapon.** A **melee** body is always **front**; a **ranged**
body is always **back**; fixed for the whole fight. So **setup declares only which region** a hero
joins (join a friendly region or open a new one), never front/back -- which roughly **halves the
setup branching** and makes the solver faster, and unifies heroes with foes (the foes already take
posts this way). A body that is **both** melee and ranged is the one case this cannot decide; there
are none in the roster, so it is **deferred** (that future case is exactly what would re-introduce a
real post choice). The cost, paid on purpose: no strategic archer-at-the-front, no self-punishing
mis-post, and **screening is now compositional** -- a zone is screened only if it *contains* a melee
body, so protecting your archers is a partition decision, not a post decision.

The per-round declaration is still just an **Act**; the intruder state adds the in-region target
set:

- **Clash(target)** -- a target vanguard in another region. Any body, melee or ranged.
- **Raid(target)** -- a screened rearguard in another region. Melee only (you have to cross).
- **Melee(target)** -- any enemy in your **own** region (i.e. an intruder loose in your ranks, or
  -- if you are the intruder -- any host body). No screen applies in-region.
- **Slip(region)** -- the one movement. **A slip goes only to a region that already holds bodies:**
  an **enemy** zone (raid in, become an intruder) or a friendly zone (rally). **No slipping onto
  empty ground** -- you move to fight or to rally, never to nowhere, which removes the "stall in an
  empty square" option and trims the branching. Three details:
  - *Rallying into a friendly zone* you take your weapon-fixed post (melee front, ranged back).
  - *Enemy-zone to enemy-zone* directly is allowed (still "move to an enemy zone"); not special-cased.
  - *The one exception to "no empty ground":* an **intruder may always fall back to a default home
    zone** even when no friendly zone is occupied -- a stranded raider is never trapped. This is a
    retreat to your own rear, the sole sanctioned move onto open ground.
- **Hold** -- nothing.

Each Raid/Slip/Melee-crossing still carries its Evade/Push/Abort answer, declared up front (in
perfect-information PvE that loses nothing a solver could use).

---

## 6. Preserved / changed

**Preserved:** formation declared once then only earned; slipping the sole movement; the screen as
*force, not fiat* (a price, never a ban); ranged beats melee via first-contact timing; the
"a step exists only to let a death silence a later one" razor; the 5-round cap = Draw; the
`Game`/solver seam (declare-then-resolve is unchanged, so `options`/`apply`/`outcome` and the
generic `Solver`/`PathCounter` need no changes).

**Changed:**
- The **schedule** is reorganized from a flat five-beat timeline into **three distance bands**
  derived from one principle.
- A landed raid now leaves a **persistent intruder** inside the enemy formation, and clearing a
  zone **promotes** it to your side -- territory, not just relocation.
- Same-region fighting (**Melee**) is a first-class engagement, so `legal_acts` offers in-region
  targets whenever a region holds both sides, and `play_round` grows Band 1.
- **Post is derived from the weapon** (melee front, ranged back), not declared -- so setup chooses
  only the region, which nearly halves setup branching.
- **Movement is constrained** to occupied enemy/friendly zones (plus the intruder's retreat-home
  exception); no more slipping to arbitrary open ground.
- AoE is stated once, as width-of-the-engagement, covering all three geometries uniformly.

**What this asks of the code (not done here):** setup `options` drops the front/back dimension
(region only); `legal_acts` gains `Melee` targets for any region holding both sides and restricts
`Slip` destinations to occupied zones (plus the intruder retreat); `play_round` grows the Band-1
intruder step, keeps a landed outrider in the enemy region instead of treating it as a one-shot,
and flips zone ownership when a zone's formation is wiped; `is_screened` already means the right
thing (a back with a living front), and an intruder is simply a body whose side differs from the
region's formation. No change to the `Game` seam, so the solver stays tractable by construction --
and the smaller setup/move branching makes it *faster*.

---

## 7. Decisions & what remains

**Settled this pass:**
- **Band 2 before Band 3-Fire** (§4) -- crossings-first, swappable later if the archer math wants
  all-ranged-together.
- **Intruder AoE hits both host tiers** (§1/§2) -- kept, as the raid's teeth; the crossing you pay
  to get there is the counterweight.
- **How an intruder leaves** (§5) -- `Slip` to a friendly zone, opposed by the zone it leaves (a
  crossing in reverse); or clear the zone and hold it. A stranded intruder always has the
  retreat-home fallback.
- **Post is the weapon** (§5) -- melee front, ranged back, fixed; setup declares region only.
- **Zone flip is on the spot** (§2) -- the step where the last defender dies, not the Reset.

**Still open / deferred:**
- **Dual melee+ranged bodies** -- the one case post-by-weapon cannot decide; deferred until such a
  unit exists, and it is what would re-introduce a genuine post choice.
- **Promotion needs multi-zone enemy formations to do visible work** (§2) -- against a single-zone
  warband it never fires before the win; encounters may want to be authored to span zones.
- **Corner/solo tuning is void** under this schedule -- solos and corners are tuned against the
  five-beat `play_round`; re-derive once Band 1, persistent intruders, and weapon-fixed posts land.
