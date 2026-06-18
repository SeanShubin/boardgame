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
