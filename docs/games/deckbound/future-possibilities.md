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
