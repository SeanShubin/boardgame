# Deckbound — Future Possibilities (design backlog)

> **Status: living, speculative, non-authoritative.** This is a parking lot for design
> changes that are too big or too drastic to decide from the armchair — they need
> **playtest of the current model first**. Nothing here binds.
>
> It is deliberately **not** `canon/` (it doesn't govern anything) and **not** `notes/`
> (those are *frozen* history; this one is *living* and forward-looking). Acting on any
> entry means going through the normal change discipline in
> [`canon/0-source-of-truth.md`](canon/0-source-of-truth.md): **spec first**, then code,
> then tests — and a number change is the human's call.

## How to use this doc

- One entry per candidate change. Append; don't rewrite history.
- Each entry carries: **Idea**, **Why explore it**, **Analysis**, **Risks**,
  **Current lean**, **Open questions for playtest**, **Status**.
- When an entry is decided, either promote it to `canon/` (supersession protocol) or
  record that it was rejected and why.

---

## 1. Simplify the breadth economy (Speed / Tempo / Focus)

- **Status:** Parked — revisit after feeling out the current model in play (raised
  2026-06-17).
- **Scope:** the *breadth* layer only (§3 Tempo/Focus + §4 gauntlet). **Does not touch
  the Clash.**

### The two candidate simplifications

- **P1 — counts instead of Speed.** Remove Speed as a magnitude. Tempo becomes the
  *number of attacks you can initiate*; Focus becomes the *number of attacks you can
  defend against*. Every engage/cover costs **1**, regardless of the foe.
- **P2 — combine the pools.** One "actions" pool spent on offense *or* defense, the
  player's choice each round.

### Framing (why these are lower-risk than they look)

Both changes touch only the **breadth** economy. The hidden-commit mix-up that is the
core fun — the **Clash** — is untouched by either. So the question is not "do we keep the
fun" but "what does the strategic economy around the fun lose or gain, and does balance
get easier?"

### What the current Speed economy buys (the levers we'd be trading)

Speed currently does quadruple duty, each job a balance lever:

1. **Tempo pool** = Speed (how many duels you can *start*).
2. **Per-target cost** — engaging/covering a foe costs *that foe's* Speed (fast foes are
   expensive, slow foes cheap).
3. **Drag** — the gauntlet is sum-of-Speeds vs runner Speed.
4. **Focus** is the parallel *defensive* pool, derived from **Mind**, separate from Speed.

The **separation** of (1) Tempo and (4) Focus is what creates the **glass-cannon ↔ wall**
axis, and the god-tier set is built on it (Blur = Speed 10 / Mind 2, overextends and gets
free-hit; Sage = Mind 10 / Power 2, covers everything but can't kill).

### P1 analysis — counts instead of Speed

Essence: drop per-target cost and drag-as-Speed; Tempo/Focus become flat counts. This
mostly *renames* Speed→Tempo-count, Mind→Focus-count and flattens cost to 1.

- **Party scaling:** breadth becomes pure addition — offense = Σ Tempo, defense = Σ Focus.
  Round survivability = "incoming attacks ≤ total Focus." Trivially computable.
- **God scaling:** dominance = "my action count ≥ the threat count." The Blur lesson
  *survives* — Focus 2 still means "cover only 2 of the swarm, the rest free-hit,"
  regardless of foe Speed. Arguably clearer than today.
- **Balance verdict: easier.** Removes a cost table and the Speed-vs-Speed drag math;
  scaling goes linear (no compounding, no per-target thresholds).
- **Cost:** loses the fast/slow-foe distinction (skirmisher and brute cost the same to
  engage); the **gauntlet must be re-keyed** off counts (it loses the combined-Speed tax).
  The swarm dynamic also inverts: today a high-Mind defender is taxed *more* by fast foes;
  under counts a high-Focus defender covers many foes regardless, so swarms win only by
  **out-numbering the Focus count** (e.g. "Slay the Titan" needs the Titan's Focus set
  low, not relying on recruit Speed). Numbers change; the design still works.

### P2 analysis — combine the pools

- **One genuine win:** it *clarifies* the free-hit rule to something clean — *"each foe
  needs one of your actions aimed at it (duel it or cover it), or it free-hits you."*
- **Party scaling:** still additive, **but role identity collapses** — the cannon ↔ wall
  axis *is* the pool separation; merge it and every hero is "N flexible actions." Co-op
  "lock and key" differentiation must move onto Power / armor / damage-type / reach /
  decks (which still exist, but breadth stops defining roles).
- **God scaling:** a combined pool makes a god *strictly more dominant* — always allocates
  optimally, never caught with the wrong pool. **This deletes the overextension lesson**
  (greed only bites when offense and defense are separate). The Blur scenario as designed
  stops working.
- **Balance verdict: easier arithmetic, harder *intentional* balance.** You lose the
  biggest lever for punishing greed and for balancing gods by something other than raw
  attrition; against a god you're left with one blunt knob — pile on more foes than its
  action count.

### Both together

The simplest possible economy: *"each actor has N actions/round; spend 1 to attack or
defend; any foe you didn't point an action at free-hits you."* One integer per actor;
balance is trivially computable for parties (Σ actions vs threats) and gods (actions ≥
threats). Cost: the entire breadth layer becomes one dial, and all texture (fast/slow,
cannon/wall, the gauntlet, greed-punishment) must be carried by Power, armor, damage type,
reach, and the Clash decks.

### Synthesis & current lean

General law: **simplification makes *coarse* balance easier and *fine* balance harder** —
you trade tuning resolution for legibility. For powerscaling, coarse legibility ("god's
count beats the swarm's count") communicates the fantasy well; the loss of fine knobs
mainly hurts making many *distinct* foe/hero feels.

- **P1 is the good trade** — legibly linear scaling; it *keeps* the cannon/wall axis and
  the overextension lesson (those live in the *separation*, not in Speed-as-magnitude);
  only real costs are re-keying the gauntlet and losing fast/slow flavor.
- **P2 is the risky one** — attractive free-hit clarification, but it removes the lever the
  god-tier and co-op designs are built on, and homogenizes roles.
- **Middle path (current lean):** **do P1, keep the pools separate.** Count-based
  legibility and trivially computable scaling, while the cannon/wall axis and
  greed-punishment survive. Most likely to "preserve the core fun with simpler mechanics."
- If P2 is ever wanted, soften it with a per-round **allocation cap** (e.g. "at most X of
  your actions may be defensive") to retain the commitment tension under one pool.

### Open questions to resolve in playtest (before deciding)

1. In actual play, does the **fast/slow-foe** texture (per-target Speed cost) carry its
   weight, or is it complexity nobody feels?
2. Does the **gauntlet's** Speed-vs-Speed tax read as fun, or as fiddly arithmetic? (It's
   the system that most depends on Speed-as-magnitude.)
3. How much does **role identity** actually come from the Tempo/Focus split versus from
   Power / damage type / reach / decks? (Determines how much P2 would really cost co-op.)
4. Do players experience the **overextension** lesson as a highlight (keep separation) or
   as a feel-bad (maybe combine)?

---

## 2. Commitment-order battle system (replace front/back formation)

- **Status:** **Graduated to canon (§4)** on 2026-06-17, then **refined to the lane model** —
  see `canon/2-spec/README.md` §4 for the authoritative version. The text below is the
  *exploration that led there* (the speed-pairing form with interposition); it is kept as
  history, **not** as a live proposal. Where it differs from §4, §4 wins.
- **Scope:** the breadth/positioning layer. Does **not** touch the Clash.

### The idea

Replace spatial lines with a **commitment order**. Roles become *when you commit*, not
*where you stand* — which is what finally earns the Vanguard/Skirmisher/Reserve names (an
**information gradient**: commit blind → choose with partial info → choose with full info).

Each round:

1. **Declare Vanguard, in order, secretly** — a face-down stack with **bluff/decoy cards**,
   so the opponent can't read how many Vanguard you committed or in what order.
2. **Anyone not Vanguard is Reserve.**
3. **Pair off the Vanguard by Speed** — the two front lines clash (tough units meeting tough
   units).
4. **Unpaired Vanguard, or Vanguard who refuse their pairing (take a free hit), become
   Skirmishers.**
5. **Skirmishers choose targets** (they slip past to hit the vulnerable).
6. **Reserve choose targets** *with knowledge of the Skirmishers' choices.*
7. All duels are now fixed → **resolve order-independently** (§1.9 property preserved).

So targets are chosen in info order: **Vanguard (blind) → Skirmishers → Reserve (informed).**

### Hard invariant — keep the gradient at the *round* scale

The gradient must never leak reveal-first into the **Clash**. "Later = more info" may only
mean *more of the already-public, resolved board* — never "I saw the opponent's choice this
phase before making mine." Keep each phase **cross-side simultaneous** (both sides' Vanguard
declare at once; both sides' Skirmishers at once; both sides' Reserve at once). Within a side
your Reserve naturally acts after your Skirmishers — that's the point — but between opponents
no phase reveals first. The Clash itself stays hidden-simultaneous.

### Q1 resolution — keep Tempo / Focus **split**

This system makes the split *cleaner*, with new crisp meanings:

- **Tempo** = engagements you **initiate** — target-picks spent across
  Vanguard/Skirmisher/Reserve (offense breadth, ordered along the gradient).
- **Focus** = incoming targetings you can **answer** as a real (defensive, survive-only)
  duel; overflow resolves as **free hits** (the "refuse → take a hit" valve generalized).

The decisive argument is information-theoretic: in this design **Tempo is hidden and
bluffable** (the face-down Vanguard stack) while **Focus is public** (a defensive capacity
the Reserve's informed choice depends on being known). You cannot merge a hidden stat with a
public one without incoherence — so **keep them split.** Merge only if you also flatten the
initiate-vs-answer asymmetry (make every answered duel able to *kill*, not just survive);
today defensive duels are survive-only, so the asymmetry — and the split — stays.

### Q2 resolution — "extra actions" = engagements from the Tempo pool

Model an extra action as an extra engagement, allocated across the three phases. A grunt has
one; a god has many, and the gradient is the **value curve**:

- Vanguard action — cheap, blind, but *forces* a Speed-pairing on the enemy.
- Skirmisher action — mid, partial info, free target choice.
- Reserve action — premium, full info, but only leftover/exposed targets remain.

A single fast hero can flow through phases (commit one blow blind, then read the board and
pick more targets later). The "lone god engages the swarm" fantasy falls out of the
structure, and stays balanced because the god still needs **Focus** to survive everything
that targets it back — big Tempo *and* big Focus = Kael's current profile. Powerscaling is
preserved without a new subsystem.

### Open question (under active discussion) — *why be Vanguard? how does Vanguard protect Reserve?*

Reserve has strictly more information, so without a forcing function everyone bluffs an
empty Vanguard and holds for Reserve. The intended fiction resolves this: **the Vanguard
protects the Reserve.** Reserve are the high-impact, fragile pieces (support buffers,
debuffers, ranged glass cannons — *vulnerable but decisive*); Skirmishers are what slips in
to assassinate them; Vanguard is what stops the Skirmishers. A party that surrenders target
choice — too few/too fragile Vanguard — lets the opponent freely pick which Reserve to
assassinate and is annihilated; a party that fields **durable units who can take a hit to
protect the Reserve** prevails. The mechanical representation of "Vanguard protects Reserve"
is the part still being designed (see chat: candidate is *pairing occupies enemy attackers*
+ *interposition redirects a Skirmisher's blow onto a durable Vanguard*, paid in Focus).

---

## 3. Deterministic base mode + the Clash as an optional module

- **Status:** Strong lean (likely architecture). Raised 2026-06-17.
- **Scope:** how a same-range engagement resolves, and the Clash's (§1.0) relationship to the
  rest of the game.

### The idea

Make the **canonical floor deterministic** — *no Clash*. A **same-range** engagement resolves as
a **trade** (both deal their base through armor/toughness, §2); a **range mismatch** is the
**auto-hit** already in §4.2. The **Clash (§1.0) becomes an optional tactical module** layered
onto same-range engagements for groups who want the per-beat mix-up and Force.

### Why

- **Depth without RPS is proven.** The strategic layer stands alone: hidden lane allocation
  (**Colonel Blotto**), the Tempo/Focus economy, the protect-the-specialist **coordination
  graph**, and **card combos** — and the **role triangle is itself a strategic RPS** resolved by
  *commitment*, not a coin-flip. The Clash adds tactical texture and the "lucky read," not the
  core depth.
- **Determinism makes card-exceptions predictable.** With a board that is **computable at every
  phase boundary**, an extreme-but-named, *local* card exception composes cleanly. This is the
  safest substrate for "wild cards that break core rules in crazy ways."
- **Accessibility + option.** A clean deterministic base game, with the Clash as opt-in depth.

### The invariant to keep regardless

Even if the Clash stays mandatory: **the strategic layer must be rich without RPS.** Never let
the game's depth *depend* on the Clash — it's a module, not a load-bearing wall.

### Open

- Same-range base resolution: pure simultaneous **trade**, or higher **effective Power** wins, or
  a Speed tiebreak? (Trade is simplest and keeps "offense is lethal.")
- Does **Force/escalation** exist in base mode, or is it Clash-only? (Likely Clash-only; base =
  flat base damage.)
- Default posture: deterministic base as default with the Clash **opt-in** (lean), or Clash on by
  default.

---

## 4. Aspects & the chord — the multi-deck combo system *(RETIRED)*

- **Status:** **Retired 2026-06-21** — removed from the live backlog; this is no longer a candidate.
  The single-deck core + the §4.4 per-suit-per-round play already deliver its "combine capabilities"
  intent, and a fused-action chord runs against the simplification trajectory and Charter #2 (small,
  computable tactics). The idea, the full rationale, and **the bar it must clear to return** are parked
  in **[retired-ideas.md](retired-ideas.md)**.

*(Heading kept as a stable anchor: this was "entry 4", still referenced from the Spec and other notes.)*

---

## 5. Role-card redesign — a scarce, shared, level-gated pool

- **Status:** **Graduated to canon as intent (2026-06-19)** — now Spec §8.3 / §8.5 / §5.6 / §4.4
  (`🟡 migration pending`); the code migration (Phases 1–4) has not started. Full design + plan:
  **[`role-card-redesign.md`](role-card-redesign.md).** *(This entry is kept as history; the live
  authority is the Spec.)*
- **Scope:** how role identity and progression rewards are structured — re-types rewards from stat
  Upgrades (§8.3) into a **25-card role pool** (5 roles × 5 levels). Touches §8.3 / §8.5 / §5; must
  clear the Spec §0 computability invariants.

### Idea (three constraints)

One copy of each role card, **one per (role, level)** → exactly **25 effects**, unlocked by clearing
levels, with the **party assigning** each scarce reward. **One role card per role per turn** (the
god-vs-party lever). Unlocks may be **multi-card sets** for richer high-level effects (">25 cards, 25
effects").

### Why explore it

The bet: *the right constraints maximise interesting options* — scarcity (no stacking), the
per-role-per-turn cap (no spamming), and atomic sets (no combo-multiplication) each remove a dominant
pattern, and that is where real choice lives. It also **regularises** today's uneven 1–3 / 0–3 / 2
patchwork into a flat 5 × 5 grid (easier to balance) and maps cleanly onto the physical format.

### Current lean / open questions

Promising; the load-bearing decisions are in the tracking doc §6 — **permanent vs reassignable card
ownership** (the computability hinge), whether role cards carry **stat growth** or just effects,
whether this **replaces the currency-buy step**, and the per-level **set-complexity curve**. See
[`role-card-redesign.md`](role-card-redesign.md) §3 (consequences), §5 (computability check), §6
(decisions).

---

## 6. The card-table UI — a rigorous physical metaphor

- **Status:** **Future direction, not scheduled** (recorded 2026-06-20). **Tuning the role-card model
  comes first.** Full vision: **[`presentation/card-table-ui.md`](presentation/card-table-ui.md).**
  This is the **north star for the renderer** — new UI should move toward it, not away.
- **Scope:** presentation only (the `tabletop` renderer); governs *how the rules are shown*, no rules
  change.

### Idea

Make the UI a faithful image of the tabletop game: **every card always has a physical place and is
always on screen** — shown as a card, or **collapsed into a deck** (a labelled, counted pile) when not
attended to. Two primitives: **cards** (exist) and **decks** (new). **Click a deck to fan it out and
focus the camera; click the table to zoom out one level** (recursive; dead-zone around cards).
Focusing one set **collapses everything else into decks**, so nothing is lost and nothing crowds.
Several decks fan at once when an action spans them (e.g. placing a character card into Vanguard /
Reserve shows hand + both zone cards + others' placements). A **damage deck** of 1-damage cards piles
onto a target through a phase and resolves **once at the phase boundary** (the physical image of the
order-independent `tally`).

### Why explore it

It directly serves Charter **#7** (cards only) and **#9 / #10** (rules ride on a metaphor, are
re-derivable). It also unifies several ad-hoc affordances we've added (the "Next" hint, the
suggested-action highlight, the event feed, assemble-as-placement) into one coherent model, and gives a
clean answer to "where do all the cards go" as the game grows.

### Current lean / open questions

Endorsed as the *direction*; deferred until after tuning. **Rendering approach (flexbox UI vs a 3D
table — thickness, stacking, isometric, full camera orbit) is an explicit open question** (doc §7),
leaning undecided. Other open questions (doc §8): deck identity/count rendering, how many zoom levels,
multi-deck fan layout, the "perspective" convention for a single player driving several characters, and
the visual language for "a legal move lives here" vs. "you may only look." The assemble-as-placement and
zone-visuals steps (label-card-left, fan, hover-pop) should be built so they **generalise into** this
deck/zoom model, not as one-offs.

## 7. Gear system — a third treasure, and the reward-structure expansion

- **Status:** **Wanted, deferred** (recorded 2026-06-21). **A working game comes first** — this is the
  designated home for the gear idea and the reward-structure rescale it implies, so they have a place to
  live until scheduled.
- **Scope:** the **reward / progression** layer (§8.3 / §8.5) and the **damage-type / armor** model
  (§2.2). Does not touch the Clash or the gauntlet core.

### Idea

Add **gear** as a **third treasure** per `(suit, level)`. Today a reward is **two** things — one **suit
(role) card** + one **stat boost**; gear makes it **three** — **suit card + stat boost + a piece of
gear**. Gear is the missing **player-managed weapons/armor layer**: a weapon deals a **damage type**, a
piece of armor **resists** types — so equipping is a real choice.

**Five is doing heavy lifting.** Five suits, and — conveniently — **five gear slots**: *weaponset, head,
legs, arms, body*. The 5×5 lattice gains a natural third axis.

**Reward-structure consequences (the rescale).** Three treasures per `(suit, level)` invites splitting
them across locations:
- **3 rewards → 3 locations:** the current **25** reward-locations become **75** (3× the map), OR
- **a 5-stage campaign:** one **level** at a time — each stage is **15 locations** (5 suits × 3 rewards),
  cleared before the next stage opens. A cleaner arc than one sprawling 75-node map.

### Why explore it

It **lights up four dormant systems at once** (the reason it keeps surfacing):
- the **damage types** earn their slot — called-shots (§2.2) become a real *gear* choice (the type×armor
  lattice is currently inert because weapons are fixed and foes are mostly unarmored);
- the **Wall's** Armor / mitigation gains depth (and the **Shield Wall** card, which grants temporary
  Armor, becomes a preview of this axis);
- the **Artillery's** Pierce stat + the **Sunder** card become meaningful (something to pierce);
- the **§8.6 emergent locks** get their content — an *armored-foe* lock is exactly what makes Pierce /
  Sunder / type-choice the efficient key. **(Partly realized 2026-06-21:** the Artillery necessity lock
  in `crates/deckbound/src/balance.rs` (`check_role_necessity` / `lock_encounter`) is already this — a
  Heavy-Plate Brute front that blunt Wall fists cannot crack and only Brass's sharp + precision defeats.
  So the gear payoff has a **working witness in the balance harness, ahead of gear itself**; building
  gear is what would let a *player* make that called-shot choice rather than it being a fixed lock.)

### Risks / open questions (for playtest)

- **Complexity budget.** Gear + 3 treasures + a rescaled map is a lot of new surface; it must not bury
  the core. Sequence it *after* the role-card model is tuned in play.
- **75 vs 5-stage.** Which reward-structure — a 3× map, or staged 15-location levels? (Leaning **staged**:
  it keeps each stage legible and bounds the par search.)
- **Build computability (§0.1).** Gear is *owned, monotone, additive* assets — keep it so (no sell-back /
  swap-oscillation) or it breaks the no-path-dependent-budget invariant.
- **Does gear scale with a stat, or is it flat?** (Cf. the signature-stat principle, Charter #12.)
- **Damage-type set.** Gear is what would justify keeping all **6 outer types** (Blunt / Sharp / Pierce /
  Heat / Cold / Lightning); absent it, `Pierce` / `Cold` / `Lightning` sit **dormant — produced by no
  card, answered by no armor** — and the pure-physical Blunt/Sharp/Pierce distinctions are tabling
  candidates. **Anti-rediscovery note:** these dormant types *look* like dead code and have been flagged
  as "cleanup" more than once; they are not — they are this feature's scaffolding, and the `DamageType`
  enum (`crates/deckbound/src/stats.rs`) carries a pointer back to this section so the next reader stops
  here instead of re-deriving the whole analysis. Do **not** prune them while gear is still wanted.
  *(`Confusion` was a separate, already-closed case — the Mind channel was cut 2026-06-20, and the type
  is already gone; it is not part of this dormant set.)*

### Current lean

**Build it — but not yet.** The current priority is a complete, tuned, working game on the existing
2-treasure / 25-location model. Gear is the first major *expansion* after that floor holds. Until then,
this entry is where gear-shaped ideas accumulate.

---

## 8. Static ranks + ordered resolution — simplify the gauntlet

- **Status:** **Exploring** (raised 2026-06-21). A candidate **simplification** of the §4
  charge-and-gauntlet that would replace the secret-charge + threading gauntlet (the current
  resolver-of-record). **Decision 2026-06-21: promote to Spec §4** (via spec-sync) — the two knobs
  below are now locked. Descends from **entry 2** (which became §4) but drops its *information
  gradient* for simultaneous declaration.
- **Scope:** the §4 battle's positioning + resolution. Does **not** touch the Clash (§1) or the stat
  model (§2) beyond *reading* Speed / Drive / Power.

### The idea

Declare ranks **simultaneously and hidden** — each living character → **Front / Flank / Back** —
reveal together, **nobody moves**. Resolve the round as a fixed sequence of windows ("be precise about
the order"), in **two tiers**:

- **Tier 1 — at the line** (from start-of-round snapshots): Fronts may **strike** the opposing Front
  (a card per blow); each crossing Flank runs an **infiltration contest** vs the enemy front; deaths
  tally at the tier boundary (a Front killed at the line still landed every blow it committed, §1.3).
- **Tier 2 — the breakthrough**: slipped Flanks strike the enemy **Back**; both Backs volley the enemy
  Front + Flank; tally; refresh.

Roles are **declared, not emergent**. This deletes the threading machinery (column pairing, Taunt sort,
surplus loops, Bodyguard-across, the `Cross` enum, ~150 lines of `combat.rs`) in favor of per-contest
comparisons.

### The contest — priced summation (force, not fiat)

A contest (slip vs catch) is a **single simultaneous hidden Tempo bid**: each side commits *k* of its
Tempo cards; committed Drive = **k × grade**; higher total wins; committed cards are spent. So the three
stats finally separate:

- **Speed** (card count) = how many actions / contests / defenses you get — *volume*.
- **Drive** (card grade) = how *cheaply* you win a contest — *efficiency* (one-card the common case,
  keep the rest of your hand).
- **Power** = strike weight (Drive is inert in a strike).

Quantity *can* substitute for quality, but at a worse exchange rate (you pay extra cards), so Drive
stays valuable as **tempo-efficiency** and the depth/breadth build fork is real (Charter #2). This is
**force, not fiat**: a low-Drive Flank is *outpaced* by a high-Drive wall (it must overspend), never
*forbidden* (Charter #12 — outpaced, not forbidden). A **single simultaneous bid** (not an iterated
raise-war) keeps it computable (#11) and a hidden, simultaneous bet (#3). *(This is the "escalating
Drive auction" the v1 gauntlet deferred — adopted in its single-bid, computable form.)*

### "Act while you have Tempo"

No per-phase action cap. A unit strikes / catches / strikes back while it has cards (1 each). A
high-Speed god's hand is therefore **breadth**: slip, carve several backliners, and still punish
focus-fire — it cannot win a duel *harder* (Drive caps that) or hit *harder* (Power caps that), only do
*more*. Aggregate party Tempo (5 bodies) exceeds one god's, so breadth opposes the god **with force**,
diminishing returns, never a hard cap.

### Re-homed powers

Threading-specific powers become clean stat-modifiers: **Phalanx** = +catch Drive; **Bodyguard** = catch
one extra Flank (one more catch-bid); **Taunt** = must be assigned the first catch; **Blitz** = a free
slip card; **Shadowstep** = win a tied contest. All **accelerators, not keys** — a no-skill unit can
still do everything by raw stats (see the invariant below).

### Demise — protection comes from the line

Each rank's vulnerability is just *how much line stands between it and the enemy*, which gives one
thematic rule for who dies how:

| Rank | Dies to | Safe from |
| --- | --- | --- |
| **Front** (is the line) | direct engagement (enemy Front clash); a slipped enemy Flank striking from behind | being flanked while it holds — it faces forward |
| **Flank** (left the line) | the wall's **catch / parting hit** at the line · the enemy **Back's** ranged fire · an enemy **Flank** in the open | the committed enemy **Front** — a holding line cannot wheel and chase |
| **Back** (behind the line) | a Flank that **slips past** its wall · its own **Front wiped**, then enemy Fronts pour through | everything, *while its line holds* |

So the **Flank is the exposed rank**: it bought reach by giving up cover, and dies in the open. Two
consequences: (1) **Drive is read in exactly one interaction** — Flank vs the Front's catch; the
front-clash, parting hits, gap-fights, and volleys are all **Power**. (2) the Flank is **both spear and
screen** — you field Skirmishers to assassinate the enemy Back *and* to kill the enemy's Skirmishers in
the open before they reach yours, giving the Back a third line of defense (hold the wall · screen with
flanks · shoot the crossers).

### The "force, not fiat" invariant (proposed test)

**A single character with *no skills* but *infinite stats* must wipe any *finite-stat* party in one
round.** Infinite Speed = unlimited actions, infinite Drive = win every contest, infinite Power =
one-shot, infinite Body = survive. If it *cannot*, some rule forbids by **fiat** — a hard cap, an
immunity, a skill-gate, a permanently-unreachable rank — which is exactly the bug to catch. The
*no-skills* clause forces the win to come from **stats**, so a skill can never be a load-bearing key.
Implementation: a large-but-finite god vs several adversarial finite parties (deep wall, swarm,
hide-in-back); assert a **round-1 wipe**. The test also *pressures the targeting matrix* to keep every
rank reachable by enough force (no permanent safe rank). Its mirror — a same-treasure balanced party
matches or beats a god — is the existing BI-1 direction.

### Computability (does the bidding break the solver?)

No. In **analysis mode** (Clash off, creatures a *fixed environment*) a creature's bid is a
deterministic function of state, so the hero **best-responds by optimization**, not game-theoretic
search — single-agent, exactly as today's deterministic creature charge. The bids add only **bounded**
branching (*k* ∈ 0..Speed per contest), and because each character bids from **its own** Tempo pool,
contests are **independent per character** — linear in roster, not a joint product — so no combinatorial
blow-up; a single PvE combat stays a finite, exactly-solvable tree (#11). A genuine simultaneous-move
game appears only in **PvP / Clash-on**, where the bid is the intended small RPS-plus-magnitude tactical
layer (#2), solvable for a mixed-strategy optimum.

### What it preserves / sheds / changes

- **Preserves:** the Front/Flank/Back triangle and Iron/Silver/Brass identities; the hidden-formation
  bluff (#3); parting free hits; the merged Tempo economy paying offense *and* defense; the metaphors
  (keep pace to block, run-past-gets-hit, more guards cover more angles, aggression spends you).
- **Sheds:** the threading algorithm; emergent roles; "concentrate your charge" becomes the
  surplus-Flank-slips-free-when-the-wall-runs-out-of-catch-cards rule.
- **Changes:** outcomes are **more predictable** (no pairing lottery — a plus for #2/#11); drops entry
  2's information gradient in favor of simultaneous declaration. (The gradient was elegant but is the
  source of the "why be Vanguard?" tension; static ranks answer it structurally — the Front is the only
  thing that can spend catch-bids to stop Flanks.)

### Decisions to pin

1. **[LOCKED] Tempo is the currency of aggression; the defender directs the catch.** Standing in a
   rank and **absorbing** hits are free; every **strike** (a front blow, a strike-back) and every
   **contest** (slip / catch) costs a card — so an out-of-cards Front still holds and soaks (it just
   can't hit back), and a passive Front lets Flanks slip by free. The verbs reduce to **Strike**
   (reads Power) and **Contest** (reads Drive). And the **defender freely assigns which Front catches
   which Flank**, committed in the hidden reveal so the round stays simultaneous.
2. **[OPEN] Tie-breaker:** equal totals → held (catcher), unless Shadowstep; optionally equal →
   higher-Speed slips (a small, bounded role for volume at the margin).
3. **[OPEN] Held Flank:** trades with its catcher, or just stalls? (Stall is simplest.)
4. **Reachability — settled by the invariant.** A **slipped Flank is behind the line and may strike any
   enemy rank** (Back, Front-from-behind, or enemy Flank), and **a wiped Front no longer protects its
   Back** (enemy Fronts pour through the break). No rank is ever permanently safe — every unit is
   reachable by enough force.
5. **Re-ratification cost:** this replaces the v1 resolver-of-record, so re-tune the balance harness, the
   reference combat bands, and the transcript.

### Current lean

**Promising — the strongest simplification on the table.** Same game, far less machine, and it makes
Speed / Drive / Power finally orthogonal. Wants playtest plus the force-not-fiat invariant wired before
graduating to §4; hold spec-first until the decisions above are pinned.
