# Encounter suite — the lever-gated niche ledger (Task 2)

> **Design, staged 2026-06-26.** The curated encounter set the role-weight measurement runs over — and
> simultaneously the **§8.6 lock redesign** the spec-sync is blocked on. **Promotion target:
> `computability-and-balance.md` §10** (par-tooling). Pairs with `role-weight-balance-testing.md`
> (the measurement), `perfect-solver-plan.md` (the policy that validates it), and
> `automated-balance-testing-roadmap.md` (Tier 1).

## One artifact, four uses
Each entry is a **niche-encounter** that does quadruple duty:
1. **§6.1 necessity** — a scenario the keyed role is *required* to win (baseline loses, +role wins).
2. **Closure / dominance check** — *no other role* may clear it (niche-exclusivity).
3. **Role-weight context** — the contexts the marginal-contribution measurement averages over.
4. **Tutorial** — teaches the role's lever with clean credit assignment (you can't win without the insight);
   topological order (simplest first) = test order = tutorial order.

## Core principle — LEVER-gated, not difficulty-gated (the §8.6 lesson)
A niche must gate on a capability the **other roles structurally lack**, not on raw difficulty. The §8.6
erosion proved why: a tanky breaching soloer (3× Wall) **beats any difficulty gate** — reseeding the foe
harder just means the universal soloer wins a harder fight. The fix is to require a lever the Wall (and the
other non-keyed roles) **cannot perform at all**, so they fail *structurally*, not for lack of stats.

Each role's structural gaps (what makes exclusivity possible):
- **Wall** — no ranged reach; can't slip; **no proactive offense** (only the reactive §4.2 trade-back —
  Shield Sweep dropped, so nothing sums across N Walls); no heal; no stat-drop.
- **Infiltrator** — fragile; little AoE; no heal/buff/debuff.
- **Artillery** — fragile; weak melee; no heal/buff/stat-drop.
- **Controller** — **no damage**; fragile; no heal.
- **Support** — **no damage**; fragile.

## The exclusivity matrix
The suite is a **niche × role matrix**: each niche cleared by **exactly one** role (its keyed lever) and
**failed by all others** — especially the Wall. Diagonal = §6.1 necessity; off-diagonal = non-dominance.
The par-solver validates both (see "division of labor").

## The five niches (structural designs)
Numbers are **par-solver-tuned** (below); these fix the *structure* + the lever-gate rationale.

- **Iron / Wall — "escort the glass cannon"** *(Anchor)*. A **fragile, decisive ally** (e.g. the Artillery
  whose fire is the only kill) is under **focused fire that downs it round 1** unless the Wall **holds +
  Cover-redirects** the killing blow to itself. *Lever:* protect (Cover/Guard/Phalanx). *Excludes others:*
  no other role can soak/redirect a killing blow. **Wall offense (decided 2026-06-26):** the Wall has **no
  proactive dedicated attack** — its only damage is the reactive §4.2 **trade-back** (punish whoever crashes
  the line), the one form immune to *both* sins (no AoE; reactive, so it can't sum across N Walls under
  focus-fire). **Shield Sweep is dropped**; the freed L4 becomes another defensive lever. *(Soloability is
  not a criterion — it's power-vs-encounter; the Wall's necessity is this protect niche.)* Implement in the
  balance pass, with the lock redesign — not during the spec-sync.
- **Silver / Infiltrator — "the shielded caster"** *(Striker)*. An enemy **Rearguard caster about to fire**
  (a deferred bomb at the Reckoning / lethal backliner) behind a front **too tough to clear in the rounds
  available**. Must be **killed THIS round**. *Lever:* slip (Smoke, uncontested) **+** one-phase burst
  (Coiled/Assassinate). *Excludes:* the **Wall can't slip and can't crack the front in time**; **Artillery's
  Longshot reaches but can't one-phase-burst** the caster; Controller/Support deal no damage.
- **Brass / Artillery — "the back-rank sniper"** *(Striker, ranged)*. A **durable but non-urgent** back-rank
  threat behind a front that **never falls** (melee can't reach it all game). Killable by **ranged chip over
  rounds**. *Lever:* ranged reach (Longshot/Bolt). *Excludes:* **Wall/Infiltrator melee can't reach**;
  Controller/Support no damage. *(Contrast Silver: there the kill is **urgent + burst**; here it's
  **reach-only + patient** — distinct levers.)*
- **Bone / Controller — "the unbreakable wall"** *(Multiplier)*. A foe whose **Toughness sits above any
  non-Controller party's max *single-phase* burst** (per-phase pile wipes, so you can't chip across phases),
  but **below it after Sunder/Hex** (−Toughness). *Lever:* −Toughness. *Excludes:* no raw-damage party crosses
  the wall in one phase. *(With Shield Sweep dropped the Wall adds no proactive burst here, so the old
  "tune above the 3× Wall sum" worry is gone — tune above the **Strikers'** max single-phase burst.)*
- **Salt / Support — "the war of attrition"** *(Multiplier)*. A threat that **out-lasts the party's
  durability** — steady damage that grinds the party down before it can kill the foe, **unless sustained**
  (Mend/Sanctuary). *Lever:* heal/sustain. *Excludes:* no non-Support party survives the grind (the Wall
  tanks longer but still loses without sustain). *Alt lever:* **Thorns** — a foe the party **can't out-damage**
  but whose **own blows, reflected, kill it** (only Support's reflect).

## Per-niche schema (each entry records)
`id` · `role` + `lever` · `profile` (Anchor/Striker/Multiplier) + `domain` · `encounter` (foe structure) ·
`baseline` (party that should **lose** — missing the lever) · `keyed` (party with the role → should **win**) ·
`exclusivity` (the other roles that must **fail**, Wall first) · `lever-gate rationale` (the structural gap
others have) · `solver-tuning target` (which numbers make the diagonal+off-diagonal hold).

## Division of labor — suite = structure, par-solver = numbers
The suite fixes the **lever-gate structure** (what capability is required, who structurally lacks it). The
**par-solver tunes the exact numbers** so the matrix holds: the keyed role's line **wins**, every other
role's best line **loses** (especially the Wall), and the baseline loses. This is *why reseeding by hand
failed* — exclusivity against an optimal universal soloer can only be **validated under the strong policy**,
not eyeballed under greedy.

## Coverage — per-role now, per-lever later; the cut list
Start with **one niche per role** (the five §8.6 locks, lever-gated). Extend to **one per lever** (Sunder,
Mire, Hex, Defang, …) for full §6.1 mechanic-necessity. **A role/lever with no constructible lever-gated
niche is dead weight** (§6.1 cut) **or** the suite is thin (a suite bug) — the suite-design process *is* the
coverage ledger.

## §8.6 resolution (what unblocks the spec-sync)
This **is** the lock redesign. The eroded Silver/Brass locks become **lever-gated**: the Silver niche
requires a **slip** (the Wall can't), the Brass niche requires **ranged reach** (the Wall can't) — so the
3× Wall fails them **structurally**, not for want of a harder foe. Re-validate Bone (tune Toughness above
the **N-Wall sum**) and Salt (the Wall can't out-tank an attrition built to outlast it). Then re-enable
`each_paired_role_is_necessary_in_its_lock` — green by *construction*, under the par-solver.
