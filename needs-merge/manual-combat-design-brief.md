# Manual Combat — Design Brief

> **Implementation status (2026-07-08):** the entire **headless brain is built + test-proven** (commits
> `cdaacd0` → `981960a`) — Stages 1–5 of the build plan. Foes instantiate as real cards; the deckbound
> resolver is resumable at every decision (`combat::step_manual`, greedy-parity across all existing tests +
> 40 seeds); `ManualCombat` in `cardtable-combat` owns the card↔actor map, drives the fight diffing
> `CardMutation`s, and folds it back. **The interactive arena (step 2) is BUILT** (`688742c`→`012f51b`):
> rank-lanes view, phase pile, tempo, per-decision prompts, log panel. The sections below are the design of
> record; **Arena v2 (below) is the next evolution, from the user's narrative vision (2026-07-09).**

---

## Arena v2 — plan-then-commit, two mini-phases (2026-07-09, from the user's narrative)

The built arena (v1) prompts **one decision at a time** and auto-paces. The user's vision is a **plan-then-
commit board**: per sub-phase you see the whole board, allocate all your strikes + tempo, and commit — with
persistent **Reset**/**Commit**, everything expressible in **single clicks**, and **default = ignore/hold**.

### Each combat sub-phase splits into THREE mini-phases (one-way, no back-and-forth)
1. **Catch / Targeting** (offense): pick enemy targets and bid tempo to **reach contact** (finesse-gated —
   `cards × F_att ≥ F_target`; may over-flip to tax the escape).
   - Two-step, single-click: tap a legal enemy target (schedule-gated — Intercept → enemy Outriders —
     highlighted, help text "Select outriders to intercept") → your legal strikers light up → tap a striker
     to spend a Tempo card on that catch; **keep tapping to add tempo, cycle up and back to zero**. Freely
     re-target / re-allocate.
2. **React** (defense): for each incoming catch, **evade / strike-back / eat-it**.
   - Each **incoming catch** is a single-click chip cycling `eat-it → evade → strike-back → eat-it` (offering
     each only when legal + affordable). **Evade** flips until `cards × F_def >` the attacker's spent bid —
     the hit misses *and* **breaks contact** (so the evader is out of mini-phase 3). **Strike-back** = 1 card
     (adjacent, unevadable), take the hit + counter. **Eat-it** = free, the **default**.
3. **Extra strikes** (finesse-free free-for-all): contact is paid for, so finesse no longer matters — every
   unit **still in contact**, both sides, flips its **remaining** Tempo cards for extra hits (1 card = 1
   strike), simultaneously. Unevadable; you tap which contacted enemy each hits.

**Why exactly three, no ping-pong:** phases 1→2→3 are a strict one-way pipeline. Catch → the only reaction is
in React → Extra strikes are **unevadable** (already in contact, no finesse gate to invoke), so phase 3 is
**terminal** — nothing to react to, no loop back. Phases 1–2 map onto the resolver's **declare → apply**;
phase 3 is a second finesse-free declare→apply among contacted units.

**The design payoff:** the split cleaves *reaching contact* (the finesse mind-game, ph.1–2) from *raw
follow-up once in contact* (finesse-free, ph.3). So (a) tempo burned over-bidding to catch/tax-evade is tempo
you **don't** have for the phase-3 rain — a real budget split between aggression and follow-through; and (b)
**evade breaks contact**, pulling you out of the whole phase-3 exchange — which is why it's worth its steep
price (you escape the follow-up, not just one blow).

### The rule this pins down
Reactions are decided against the **revealed pre-damage board**, then all strikes + reactions apply as one
**order-free batch** → a **committed strike still lands even if its unit dies**, and a **doomed soaker can
still strike back** (commit-based, not "who's alive post-cascade"). This is *simpler* than the engine's
current post-cascade strike-back check and is the recommended change. (Aligns with the "cards are truth /
`apply(all_cards, intentions)->all_cards`, order-free" direction.)

### Rank declaration up front
Combat opens with the player assigning each hero a rank (V/O/R) — the Marshal step (v1 uses default
stat-derived ranks; v2 makes declaration step one), then a **Reveal** (foe ranks shown; fog-of-war is a
future toggle).

### Cost model — RESOLVED 2026-07-09 (the tempo economy)
**Tempo = the hit/not-hit currency; Might = the damage. They never interact** — tempo never makes a hit
*harder*, only makes it *land* (and, later, buys *another* strike).

**Finesse is a multiplier on tempo cards** — the exchange rate from cards to **bid-value**: one flipped
tempo card is worth `finesse`, so a unit's bid = `cards_flipped × finesse`. **Tempo cards do double duty** —
flipping a card *is* the action (the physical strike/evade), *and* that flip contributes its `finesse` to
the bid. One resource, not two. Per single strike:
1. **Catch** (attacker, Targeting): flip cards until `cards × F_attacker ≥ F_target` — the target's finesse
   is how hard it is to pin (F2 catching F3 = 2 cards: 2×2=4 ≥ 3). May flip **extra** to raise the bar the
   defender must clear.
2. **Defender responds** (Reaction, having seen the bid):
   - **Evade** — flip until `cards × F_defender >` the attacker's **spent** bid (strictly more *value*). The
     hit misses. A deliberately losing trade (aggressor sets the price); the "I *cannot* eat this" button.
   - **Strike back** — **1 card** (no catch bar: the attacker came to *you*, adjacent, so finesse doesn't
     gate it): take the hit **and** land a counter. Always available.
   - **Eat it** — free, take the hit. **Default.**
3. **Damage = Might**, applied to whoever got hit, in the order-free batch.

*Confirmed 2026-07-09:* evade must exceed the attacker's **spent value** (`cards_atk × F_atk`), not just the
catch bar (so over-flipping to tax evasion does something); catch bar = the target's finesse
(`cards × F_att ≥ F_target`).

**The finesse contest is strictly commit-then-resolve** (the blind-bid seam): the attacker locks
`cards × F_att` **first**, and only then does the target's finesse resolve the catch (`≥ F_target`) and power
any evade counter — the defender never answers simultaneously, always with the committed bid in hand. **Fog
off:** inert (F_target is pre-readable, so bid precisely). **Fog on:** load-bearing (you commit into
uncertainty about what sticks). So the attacker→defender order isn't UI convenience — it's the information
order that defines the bluff, and the reason to promote target→react to rules phases when fog/PvP arrive.

**Multi-strike = the 3rd mini-phase (in v2, not deferred).** Once contact is paid for, finesse stops
mattering: in **Extra strikes** (mini-phase 3) every unit still in contact, both sides, flips its remaining
Tempo for extra hits (1 card = 1 strike, unevadable). This is the clean solution — extra damage lives in its
own finesse-free phase, gated by leftover tempo, so it never entangles the phase-1 catch bid. See the
three-mini-phase section above.

This cost model *replaces* the engine's current tempo/finesse economy (flat-1 to strike; `⌊Fa/Fd⌋+1`
defender-evade via `policy::avoid_cost`; a can-crack gate on strike-back), and adds
finesse-as-tempo-multiplier + the extra-strike phase — so **v2 is a mechanics change to the resolver**
(engine rework + balance re-validation), not just UI: the rules-adjacent half of v2, aligned with the
order-free-batch direction.

### Open design decisions
- **Mini-phases: rules concept or UI artifact? — RESOLVED 2026-07-09: UI-first, promote later.** Present the
  three mini-phases (Catch / React / Extra strikes) in the **UI for now** (ships the solo arena fast); keep
  mechanics an order-free batch; promote them to first-class rules phases when PvP/fog arrive (the internal
  `CyclePhase` cursor makes it an upgrade, not a rewrite). Rationale below.
- **Mini-phases: rules concept or UI artifact?** Explored 2026-07-09. The split is two things: the
  **resolution mechanics** (order-free batch — already settled, UI-agnostic) and the **information timing**
  (commit targets → reveal → react). The discriminator is **hidden information**: with none (solo vs.
  visible AI, fog off) the mini-phases are **UI pacing**; with **PvP / fog** the commit-reveal boundary is a
  genuine rules mechanic. **Recommendation:** split along the seam the balance philosophy already draws
  (§5.1 fidelity rule — solver measures *raw balance*, blind to the *blind-bid / mind-game* layer): keep the
  **mechanics as an order-free batch** (solver's perfect-info domain, unchanged), and model the
  **target→react reveal as a thin rules-level information protocol** that is explicitly part of the blind-bid
  layer the solver doesn't touch. The two mini-phases become the *observable expression* of that protocol —
  not a UI invention, not baked-in interaction cadence.
  - **Pros of rules-level:** PvP + fog fall out for free; one source of truth (no UI↔engine drift);
    save/spectate/replay; a hook for future cards ("at the start of the Reaction phase…"); shared vocabulary.
  - **Cons of rules-level:** ~2× named sub-phases + restructuring the balance-validated resolver (risk);
    and hidden-info phases make combat **imperfect-information → the exact solver's perfect-info assumption
    breaks** (mitigated by keeping it in the blind-bid layer the solver already excludes).
  - **Fast-ship alternative:** if shipping the *solo* arena first and deferring PvP/fog, keep mechanics
    order-free, present two mini-phases in the **UI only**, and promote them to rules when PvP arrives (the
    internal `CyclePhase` cursor makes that a clean upgrade, not a rewrite).

---


**Status:** preparation for a large design task. This is the verified ground-truth map + scope + open
questions the design must resolve. Nothing here is built yet. All `file:line` references were confirmed by
reconnaissance on 2026-07-08 against the current tree (after commit `62d764c`, foes-virtual).

Owned by the instance that wrote it (per `.claude/CLAUDE.md` needs-merge convention); folds into `docs/`
once the design settles.

---

## 1. Goal

Add a **manual combat** mode alongside the existing **auto combat**, letting the player choose at a
combat-ready location. Manual combat must **capture every intermediate observable state** — card flips,
targeting, damage accumulating — modelled as a transformation function:

> `(current game state, card manipulation) -> (new game state)`

Auto combat stays exactly as-is (one click → headless playout → result folded onto the table). The two
paths differ only in whether the fight is *instantiated and stepped* on the table or *resolved in one call*.

## 2. The foe lifecycle (settled with the user)

Foes have three phases; the Bestiary `×4` stacks (commit `62d764c`) exist precisely to make phase 2 possible.

1. **At rest (location):** *virtual*. The place holds only its `encounter` header, which lists the foes
   (`Foes: The Anvil ×2, …`). No foe cards there — nothing to track.
2. **During combat:** *real*. The arena instantiates the encounter roster by drawing actual foe cards from
   the Bestiary stacks (split a `×1` off the `×N` supply — conservation-clean, same split recruit uses).
   These real cards carry combat state: health piles, rank lane, face-up/down flips, damage.
3. **After combat:** instances return to the Bestiary (merge back into the stack); on a **win** the
   encounter header is also cleared. On a **loss** the header stays (retriable).

## 3. Ground-truth API map (verified)

### 3.1 deckbound combat engine — `crates/deckbound/src/`

**The single most important finding:** deckbound already has a **serializable step machine**, but its
smallest *resting* step is a whole **sub-phase-cycle**, not an individual flip.

- `combat.rs:701` — `pub fn step(state: &mut State) -> bool` — one atomic transition (one `Stage::Cycle` or
  one `Stage::Boundary`); returns `false` when the round's schedule is exhausted. Holds its cursor in
  `State::resolution` so it round-trips through RON and can be observed one step at a time. **Takes no
  card-manipulation argument** — all decisions are pulled from `crate::policy` (greedy).
- `combat.rs:759` — `pub fn resolve_round(state: &mut State)` — `Resolution::start()` then `while step {}`.
- `solver.rs:56` — `pub fn resolve_logged(heroes: Vec<Actor>, foes: Vec<Actor>, seed) -> (Option<bool>, Vec<String>)`
  — the headless playout auto combat uses; returns win + full log.
- `solver.rs:25` `auto_resolve`, `solver.rs:668` `winnable` — related headless entry points.

**State types (all `Serialize/Deserialize/Clone`):**
- `state.rs:123` `State` — whole battle: `round`, `heroes: Vec<Actor>`, `creatures: Vec<Actor>`, `phase`,
  `resolution: Option<Resolution>`, `plan: Round`, `log: Vec<String>`, `rng`, `seed`, `outcome`, …
- `state.rs:84` `Round` — the per-round plan: `hero_intent/foe_intent: Vec<Intention>` (per-unit declared
  rank), `hero_group/foe_group`, `hero_acted/foe_acted`, `deferred`.
- `combat.rs:665` `Resolution { step: usize, stage: Stage, cycle: u32 }` — the in-flight cursor.
- `combat.rs:646` `Stage { Cycle, Boundary }`.

**Sub-phase / rank model:**
- `combat.rs:605` `SCHEDULE` — the fixed `(attacker-role, target-role)` schedule; the single source of
  truth. 5 sub-phases: `combat.rs:625` `SUB_PHASE_NAMES = ["Intercept","Volley","Raid","Clash","Breach"]`.
  Intercept `V→O`, Volley `R→O`, Raid `O→R`, Clash `R→V,V→V`, Breach `V→R,O→V,O→O,R→R` (last pair gated by
  `policy::can_reach` — a Rearguard reaches the back-line only once the enemy Vanguard has fallen).
- `actor.rs:29` `Intention { Vanguard, Outrider, Rearguard }` (Hold/Break/Deal). Prey cycle
  `actor.rs:42` V→O→R→V; `policy.rs:32` `priorities`.
- Rank targeting per sub-phase = `SCHEDULE`, enforced via `policy.rs:167 step_of`, `policy.rs:136
  governing_target`, `policy.rs:52 can_reach`.

**Health / damage model (`stats.rs`, all `Serialize/Deserialize/Clone`) — maps directly to cards:**
- `stats.rs:14` `HealthCard { toughness: u32, down: bool }` — one physical health card, face-up/down.
- `stats.rs:26` `Health { cards: Vec<HealthCard> }` — Vitality = face-up count; front card's `toughness` =
  the bar. Flips front-first. **This is literally a pile of health cards that flip down as damage lands.**
- `stats.rs:144` `PendingDamage { aoe, targeted: u32 }` — the per-phase damage pile; `targeted` drives flips.
- `stats.rs:208` `Defense::take_with_toughness(raw, bar)` — accumulates `raw` into the pile, flips one card
  each time the pile ≥ `bar`. **The pile wipes at every sub-phase boundary** (`clear_phase_piles`) — so
  sub-threshold damage never carries forward; only flipped health cards persist.
- `combat.rs:88` `apply_strike(target, Strike{raw}, attacker, log)` — one strike, narrated to `state.log`.
- No life total: the only states are health cards flipping face-down, and at a boundary all-down → `fallen`
  (decided in `tally`, `combat.rs:201`).

**Unit type & the authoring→Actor bridge:**
- `actor.rs:229` `Actor` — the internal mid-combat unit (offense/defense/attack, tokens, tempo, `fallen`…).
- `balance.rs:935` `DuelUnit { name, ability, stats: Stat5, ranged, aoe, count, hoard, pos }` — the
  authoring template (`Serialize/Deserialize/Clone`).
- `balance.rs:1000` `pub fn build_duel_unit(u: &DuelUnit) -> Actor` — the sole DuelUnit→Actor path.
  `balance.rs:1024` `build_duel_creatures` expands a hoard into Vitality-many one-Health bodies.

### 3.1a deckbound DECISION surface — where every choice lives (verified 2026-07-08)

There are **two decision layers**, architecturally separate. "Player makes all decisions" = take over both.

**Layer 1 — Game-level actions (`contract::Game` on `Deckbound`, `game.rs`). Already player-driven, already
rest between actions.** `game.rs:24 enum Action` (derives `Copy`). Combat-relevant variants:
`SetVanguard/SetOutrider/SetRearguard(usize)` (declare a unit's rank), `PlayCard(unit, card)` (cast a
*Standing* buff/brace — offensive abilities are NOT wired), `Pass(unit)`, `Deploy` (finish declaring →
resolve), `Play(duel::Move)` (Clash module). `game.rs:619 legal_actions` dispatches by `state.phase`;
**`Phase::Marshal`** (`game.rs:671`) offers, **for the first pending unit only**, its two non-current
`Set*` + castable `PlayCard`s + `Pass`, always + `Deploy`. Declaration is **sequential, one unit at a
time**. This phase is a resting, serializable state and is **already two-sided** (see below).

**Layer 2 — in-resolution choices (the greedy `policy.rs` module, consulted by `combat.rs`). Zero player
input today — this is the surface to redirect.** There is no `policy::greedy` fn; the whole module is "the
predictable human stand-in" (`policy.rs:11`). The 5 consult sites, all in `combat.rs`, run synchronously
inside `apply(Deploy)` with **no resting point**:

| `combat.rs` site           | policy call                        | Decision                                                                           |
| -------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------- |
| `:394` (in `declare_side`) | `governing_target(...)`            | **which enemy each unit aims at + whether to hold tempo** (the richest decision)   |
| folded into `:394`         | `team_can_crack` / `choose_target` | focus-fire viability; finish-lowest-Health-first                                   |
| `:488` (in `apply_side`)   | `role_evades(role)`                | endure-vs-evade role gate (Vanguard never evades)                                  |
| `:491`                     | `should_avoid(...)`                | **does this defender evade this blow** (only if it would flip a card + affordable) |
| `:567`                     | `should_strike_back(...)`          | **does this soaker retaliate**                                                     |

`step(&mut State)` (`combat.rs:701`) takes **no** decision argument — it pulls everything from `policy`.
Injection strategy (per the resolver's own header `combat.rs:293`): thread a decision-source into
`declare_side`/`apply_side` (both already take `&mut State`), **or** pre-compute a decision table keyed by
`(step_idx, unit)` that the resolver reads instead of calling `policy::`.

**Decision vs. deterministic-mechanic split (the resolver's explicit contract):**
- **Decisions** (→ player): target + hold-tempo, evade, strike-back, rank/intention, buff-cast.
- **Deterministic consequences** (stay engine-computed, then animated): the `SCHEDULE` pair-walk, cycle-to-
  exhaustion, tempo/crossing arithmetic, spillover `cascade`, thorns, AoE, `tally` deaths, phase-pile wipes,
  and all damage math / which health card flips.

**Two-sided substrate to reuse (PvP pass-and-play, already built):** `state.rs:148 pvp: bool`,
`plan.committing: u8` (0=heroes/1=creatures), side-generic `s_pool`/`s_intent`/`s_group`/`s_acted` (+`_mut`)
accessors (`state.rs:168-221`), `game.rs:605 current_player` seat alternation, and `Deploy`'s side-handoff
(`game.rs:906`). Marshal is **fully two-sided already**; `resolve_sub_phase_cycle` already calls
`declare_side(…,0,…)` **and** `declare_side(…,1,…)` symmetrically (`combat.rs:587`). **What's missing:** the
`pvp` flag never diverts the Layer-2 `policy` calls — inside Engage, *both* sides are still the greedy
stand-in. The one interactive per-beat precedent is the Clash module (`game.rs:460`, hero move only).

**Latent / not-yet-wired (so, out of scope for a v1 manual mode):**
- **Offensive ability selection does not exist** — every strike is `base_strike`; wiring `cast: Strike`
  abilities into `resolve_round` is a noted follow-on (`game.rs:368-377`).
- **Reckoning deferral** — `resolve_reckoning` (`combat.rs:805`) is defined but **not called from
  `resolve_round`**; deferred-spell resolution isn't in the live sub-phase walk yet.

**Definitive takeover checklist:** items already player-routed — rank (heroes), buff-cast, pass, deploy,
Clash hero move. Items to take over from `policy` — **(8) target + hold-tempo, (9) focus-fire, (10) evade,
(11) strike-back** (the core in-resolution surface). Latent — offensive abilities (12), Reckoning defer (13).

### 3.2 card-table model primitives — `crates/cardtable-model/src/model.rs`

Everything is a `Tableau` method; every mutating op is atomic (error ⇒ no mutation). `CardId`/`PileId` are
`pub u64` newtypes.

**Stack split/merge under conservation (PC.2) — ⚠ both PRIVATE today:**
- `model.rs:972` `fn draw_named_from(&mut self, bank, dest, name) -> Result<CardId,_>` **(private)** — splits
  a `×1` twin off a named `×N` stack (decrements the stack, mints a twin copying only `face`+`card_type`+
  `detail`; twin's `kind` resets to `Regular`, no `panel`/`recipe`). Whole-stack move if `quantity<=1`.
  Matches by `name()`, which is `""` for face-down cards → bank cards must be face-up.
- `model.rs:1004` `fn return_one(&mut self, card, bank) -> Result<(),_>` **(private)** — merges `card` into a
  same-`name()` stack in `bank` (`quantity += q`, removes `card`), else just moves it. **Merge is by name
  only** — normalize facing (`flip_up`) + kind before returning, as `unequip_character` does.
- `model.rs:1308` `pub fn set_card_quantity(card, quantity)` — setup-time stack sizing.
- `model.rs:183` `pub fn quantity(&self) -> u32` (on `Card`).

> **Design note:** the arena's split/merge must either be added as new `pub` `Tableau` methods beside
> `draw_named_from`/`return_one` (the way `equip_character` wraps them), or the two helpers made
> `pub(crate)`/`pub`. An arena in `cardtable-combat` or `cardtable` cannot call them as-is.

**Card lifecycle / flip / piles:**
- `model.rs:638` `add_card(pile, face, actionable)`, `model.rs:764` `move_card(card, to_deck, at)`,
  `model.rs:802` `remove_card(card)`, `model.rs:748` `reorder`. `home: PileId` tracked, `home()` @177.
- `model.rs:69` `Face { Up{title}, Down{title} }` (a down card *remembers its front*). `model.rs:1330`
  `flip_down`, `model.rs:1345` `flip_up`, `model.rs:211` `is_face_down`, `model.rs:204` `front_title`
  (identifies a flipped card). The day clock (`mark_moved`/`advance_day`) is the working flip precedent;
  the model doc explicitly names "damage landing on a Health card" as the flip-down use case.
- `model.rs:604` `add_pile`, `model.rs:1159` `remove_pile` (recursive), `model.rs:441` `Pile`
  (`label`, `reflects: Option<CardId>`, subpiles/cards). `model.rs:896` `set_projection` (show other piles'
  cards without moving them — **not** double-counted physically). `model.rs:886` `set_layout`,
  `model.rs:402` `Arrangement { List, Grid{columns}, Free, Rows }`.
- `model.rs:98` `CardKind { Regular, Zone, Utility(_), Header, Virtual }`; `model.rs:264` `is_physical`
  (false for `Utility`/`Virtual`); `model.rs:1552` `physical_card_count`. `card_type`/`set_card_type`
  (@231/@1266); `detail`/`set_card_detail` (@216/@1235); `panel`/`set_card_panel` (@221/@1248, Large).

**The assemble-from-banks precedent (the template for foe instantiation/return):**
- `model.rs:1031` `pub fn equip_character(hero_name, recipe, heroes, stats_bank, numbers_bank,
  abilities_bank, home_location, progress) -> Result<PileId,_>` — builds a deck by `draw_named_from`-ing
  stat/number/ability cards + hero copies. **This is the exact shape a "deal foes from Bestiary" step
  mirrors.**
- `model.rs:1077` `pub fn unequip_character(deck, heroes, stats_bank, numbers_bank, abilities_bank)` — the
  inverse: `return_one` by `card_type()`, gather strays by `front_title()`, `remove_pile`. **The exact shape
  a "return foes to Bestiary" teardown mirrors.**
- catalog: `catalog.rs:317 encounter_for(location)`, `catalog.rs:324 encounter_foes(e) -> Vec<(&Creature,
  u32)>`, `catalog.rs:104 Creature { name, ability, stats:[u8;5], ranged, aoe, hoard, pos }`,
  `fixtures.rs:57 creature_card` (the per-foe card shape), `fixtures.rs:529` the Bestiary `×4` block.

### 3.3 combat bridge — `crates/cardtable-combat/src/lib.rs`

- `lib.rs:34` `pub fn resolve_encounter(table, place, seed) -> CombatOutcome` — **the only public fn.** All
  helpers are private to the crate.
- `lib.rs:66` `hero_units` — physical `hero` cards at place → `character_recipe` → `ability_shape` →
  `DuelUnit`. Reusable *in shape* for a manual path but private.
- `lib.rs:95` `foe_units` — reads foes **virtually** (place label → `encounter_for` → `encounter_foes` →
  ephemeral `DuelUnit`s, no `CardId` backing). **Not** directly reusable for real foe cards; the per-creature
  `DuelUnit` construction block (`lib.rs:105-114`) is the reusable kernel — only the *source* changes.
- `lib.rs:53` `stat5`, imports `deckbound::balance::{DuelUnit, Stat5, build_duel_unit}`.
- `lib.rs:125` `apply_consequences` — win clears `encounter` header (nothing to Bestiary, foes were virtual);
  loss keeps it; one virtual `log` card per place; `advance_day` once. **The win-branch hard-codes the
  virtual-foe assumption** — a real-foe manual path must add explicit foe removal/return-to-Bestiary.
- Invariants the tests lock (a manual path must preserve): win → encounter cleared, `physical_card_count`
  unchanged by virtual log; loss → encounter stays, hero doesn't retreat; one log per place.

### 3.4 renderer — `crates/cardtable/src/lib.rs`

- `lib.rs:991` `location_ready_for_combat(tree, zone) -> bool` — place is a `Locations` subpile with both a
  `hero` and an `encounter` card. **The gate for offering combat buttons.**
- Trigger is record-only: `CombatCard` marker (`lib.rs:191`) → `on_click` writes focused place into
  `CombatRequest` resource (`lib.rs:262`, `init` @95) → consumed in **`crates/boardgame/src/main.rs:74`
  `resolve_combat`** (computes `day_seed`, calls `resolve_encounter`, sets rebuild). Combat is atomic today —
  **no interactive turn-state exists anywhere.**
- **Overlay-button pattern (template for Auto/Manual buttons):** marker near `lib.rs:181-196`; add to the
  `on_click` query tuple (@488) + destructure (@508) + an `else if` branch (@513-528); spawn via
  `spawn_nav_card` (`lib.rs:2219`) in `build_ui`'s overlay section (~@2154) under a condition; new resource
  ⇒ `init_resource` @95 + consumer in `main.rs`. ⚠ **Collision:** Combat + Advance Day + rail all anchor
  `right: 8px` — new buttons need a shared row container (like the rail @2170) or a distinct anchor.
- Zone/nav: `top_deck(label)` (`lib.rs:658`), `focus_id()` = current zone, `focus`/`zoom_out` on click.
- Drag/drop: `can_drop_on_pile` (`lib.rs:978`), `on_node_drag_end` (`lib.rs:1264`, geometry, cursor-follow
  tiles), `on_drop` (`lib.rs:589`, picking, non-cursor-follow), the **"exactly one valid drop target"** rule
  via `exactly_one` + `boxes_overlap` (@1251/@1244); cue glow `update_card_cues` (@1074). No hover anywhere.
- Rendering: `spawn_card` dispatch by size (@2459); type tint `type_accent` (@1576); face-down look
  (amber `FACE_DOWN_EDGE` + foot bar, @2539); `×N` sub-line (@2584); **Large+Virtual → scrollable
  `ScrollPanel`, not `Movable`** (@2652/@2671) — exactly how the combat log renders.
- **Bespoke-zone precedent:** the Locations **map grid** is a hardcoded `build_ui` branch matching a specific
  pile identity (`lib.rs:1946`), rendering place cells with cascaded tokens + explicit `PileDropZone`s. A
  manual arena is a *new* `build_ui` dispatch branch modelled on this (there is no "arena" `Arrangement`).
  No separate window/camera — the whole felt is one rebuilt UI root; "full-screen" = the focused zone's
  content region below the 52px overlay band.

## 4. The core design tensions (surfaced by recon — the design must resolve these)

1. **Step granularity vs. "every flip observable." — DECIDED 2026-07-08 (user).** deckbound's finest
   *resting* step is a whole sub-phase-cycle (both sides declare + apply together, incl.
   AoE→spillover→thorns→strike-backs in one shot). Individual strikes/flips are *narrated* to `state.log`
   but are not individually steppable states.
   - **Chosen: (A) Declare-then-resolve, animating the diff.** The player's card manipulation = declaring
     each unit's rank/intention (drag its card into a Vanguard/Outrider/Rearguard lane), writing
     `Round::hero_intent`. Pressing resolve advances **exactly one sub-phase-cycle** through the real engine
     (`combat::step` at `Stage::Cycle` granularity, or a thin wrapper that runs one cycle), and the table
     then *replays the resulting flips/damage card-by-card* by diffing the pre/post `Actor` state (health
     cards flipped, `PendingDamage.targeted`, `fallen`). Highest reuse of the validated resolver; the player
     makes the choices (see refinement below), the engine only *computes the deterministic mechanics*;
     observability comes from replaying the state diff, not from a finer engine granularity.
   - **The transformation function is therefore two-part:** (i) *declare* — a player card move settles
     `Round::*_intent` (permit-then-settle onto a legal lane); (ii) *resolve* — one `step` cycle produces a
     new `State`, and a **diff→animation** layer turns the `Actor` delta into a sequence of card flips on the
     table. The saved combat state is the deckbound `State` (RON-serializable), plus the card↔actor map.
   - Rejected: **(B)** drive below `step` (per-strike) — too much resolver surgery, risks diverging from the
     validated algorithm; **(C)** player performs every flip with the model as oracle — most tactile but
     most surgery and divergence risk.

   **REFINEMENT 2026-07-08 (user): the player makes ALL decisions, not just rank.** Manual combat replaces
   deckbound's greedy `policy` with player input at *every* decision point; the engine keeps only the
   deterministic mechanical resolution (damage math, which health cards flip), which is animated. So the
   "card manipulation" input generalizes from *rank declaration* to *every choice the AI would make*.

   **Consequence — the engine must rest at every decision point.** For the player to supply each choice, the
   fight must pause at a resting, serializable state at every decision point and accept a player manipulation
   to advance. This is the literal machine form of `(state, manipulation) -> new state`.

   **RESOLVED by the decision-surface scout (§3.1a): the invasive fork — but bounded.** The genuine
   per-strike decisions (target + hold-tempo, evade, strike-back) live *inside* the round walk as 5 direct
   `policy::` calls in `combat.rs` with no resting points — NOT as Game-level actions. So manual mode needs
   **new resting points injected inside the resolver**, not just a menu-level policy swap. What bounds it:
   - The surface is **small and enumerated** — 5 call sites, ~4 decision types (§3.1a). No hidden sprawl.
   - The resolver is **already symmetric** (`declare_side(…,0/1,…)`) with a PvP side-generic substrate
     (`committing`, `s_*` accessors, `pvp`) to build on; Layer-1 Marshal is already two-sided.
   - **The cadence batches, then goes per-blow.** Resolve unit = one sub-phase-cycle; both sides declare
     against the *same pre-apply board*, so all targeting/hold choices in a cycle collect into **one resting
     point at the cycle start** (a per-cycle "Marshal"). The reactive evade/strike-back choices are finer:
     **DECIDED — the fight rests and prompts at every one** (as each blow lands), no stance auto-apply. So
     `apply_side`'s `:488/:491/:567` sites each become a genuine resting point, not just a policy read.
   - **Injection**: thread a decision-source into `declare_side`/`apply_side`, or have them read a
     decision table keyed by `(step_idx, unit)` instead of calling `policy::` (resolver header sanctions
     this, `combat.rs:293`).
   - **v1 scope**: decide targeting/hold, evade, strike-back, rank. Offensive-ability selection and Reckoning
     deferral are latent/unwired in the engine (§3.1a) — later layers, not v1.

2. **Who owns the interactive state?** There is no turn-state today. A manual arena needs a new
   `Option<State>`-style resource (deckbound `State` is `Serialize/Deserialize` and RON-round-trips, so it
   can *be* the saved combat state) plus stepping wired in `main.rs`, mirroring `CombatRequest`.

3. **Real foe cards ↔ deckbound `Actor`s must stay in sync.** The arena holds real cards (health piles,
   rank lanes); deckbound holds `Actor`s. The design needs a stable identity mapping (card ↔ actor index)
   so a resolved cycle's `Actor` health/`fallen` changes drive the right cards' flips, and so a player's
   rank-card placement writes the right `Round::hero_intent[i]`.

4. **`draw_named_from`/`return_one` are private.** Decide: new `pub` `Tableau` methods (`instantiate_foes`
   / `return_foes`, paralleling equip/unequip) vs. exposing the helpers. The `equip/unequip` pair is the
   proven template — favor a matched pair of new model methods.

## 5. Arena layout requirements (from prior discussion — refine in design)

- **Two views** the user approved earlier: a **Battlefield overview** (both sides by rank lane) and a
  **Strike/Reckoning single-attack** focus.
- **Rank lanes:** three lanes per side (Vanguard / Outrider / Rearguard) — a unit's card sits in its lane;
  moving a rank card between lanes = declaring intention (writes `Round::*_intent`). Permit any placement,
  then settle to a legal declaration.
- **Health as piles:** each combatant has a pile of `HealthCard`s (bar = `toughness`, count = Vitality);
  damage flips them front-first, matching `stats.rs` exactly.
- **Damage / AoE piles:** the per-phase `PendingDamage.targeted` pile, shown as a (virtual?) accumulating
  pile that wipes at each sub-phase boundary.
- **Log:** the existing Large+Virtual `ScrollPanel` card, already the combat-log renderer.

## 6. Binding constraints (non-negotiable — from memory + CLAUDE.md)

- **Clicks/drags/piles only:** the only inputs are single-click + drag; every element is a pile/card — even
  buttons, labels, actions, logs. No chrome. Meaning comes from what you click. `[[ui-clicks-drags-piles-only]]`
- **Permit-then-settle:** permit any input (don't block/clamp mid-gesture), then *visibly* settle to a legal
  state. Applies to every arena interaction (e.g. dropping a rank card anywhere, then snapping to a lane).
  `[[ui-permit-then-settle]]`
- **Strict PC↔iPad parity:** no hover or PC-only input; every reveal/examine works with tap+drag. iPad is a
  real deploy target. `[[pc-ipad-parity]]`
- **Conservation (PC.2):** cards are moved/split/merged, never minted. Foe instantiation splits from the
  Bestiary `×N`; return merges back. Card count is conserved across a whole fight (heroes untouched, foes
  round-trip, encounter header removed only on a win).
- **Determinism:** all combat randomness flows from the seed (today `day_seed` in `main.rs`). No wall-clock.
- **Auto combat is preserved** and selectable; manual is additive.

## 7. Suggested integration points (concrete)

1. Two overlay cards **"Auto Combat"** / **"Manual Combat"** via `spawn_nav_card`, gated by
   `location_ready_for_combat`, in a shared right-edge **row** (avoid the `right:8px` collision). Auto reuses
   today's `CombatRequest` path; Manual writes a new request/state resource.
2. A **new `pub` model method pair** on `Tableau` (`instantiate_encounter_foes` / `return_foes_to_bestiary`)
   modelled on `equip_character`/`unequip_character`, wrapping the private split/merge.
3. A **manual-combat state resource** wrapping deckbound `State` (RON-serializable so it persists), driven
   by a `main.rs` system, stepping the engine and diffing `Actor` state onto the arena cards.
4. A **new `build_ui` dispatch branch** for the arena zone, modelled on the Locations-map special case, with
   rank-lane `PileDropZone`s, health piles, and the Virtual log card.
5. A **card ↔ actor identity map** kept in the arena state.

## 8. Suggested staging (each compiles + tests green before the next)

1. **Model:** `instantiate_encounter_foes` / `return_foes_to_bestiary` (+ conservation tests) — real foe
   cards appear in an arena pile and round-trip back to the Bestiary. No UI yet.
2. **Engine — decision-source injection (the invasive part; §3.1a).** Give `combat.rs`'s `declare_side` /
   `apply_side` a decision-source seam so the 5 `policy::` call sites (`:394` target+hold, `:488`/`:491`
   evade, `:567` strike-back) can be answered by an injected provider instead of `policy`. The greedy policy
   stays the default provider (auto combat + all existing tests unchanged); a "manual" provider yields a
   resting `State` at each decision and reads the player's choice. Prove it with a test that drives a fight
   through a scripted provider and matches a greedy run when the script echoes greedy. **Keep the validated
   resolver's mechanics untouched** — only *who answers the choice* changes.
3. **Bridge/state:** the manual combat state resource wrapping deckbound `State` (RON-serializable) + the
   card↔actor map; reuse `build_duel_unit`/`resolve_logged` internals (likely make `hero_units`/foe-mapping
   `pub(crate)`). Auto path untouched.
4. **Renderer:** Auto/Manual buttons; the arena `build_ui` branch; rank-lane drag → intention (per-round);
   per-cycle target declaration (the batched resting point); **a live prompt at every reactive decision**
   (evade / strike-back — each a resting point showing the incoming blow); health-pile flips animated from
   the state diff; the log card.
5. **Integration:** `main.rs` consumer wiring; persistence of an in-progress fight; win/loss teardown
   (return foes, clear encounter, advance day) reconciled with `apply_consequences`.

## 9. Decisions & remaining open questions

**Decided 2026-07-08 (user):**
- **Flip fidelity:** approach **(A) declare-then-resolve, animating the diff** (see §4.1). The engine
  computes the deterministic mechanics; the table replays the flips from the state diff.
- **Player makes ALL decisions:** manual combat replaces the greedy policy with player input at every
  decision point — not just rank. The engine must **rest at a serializable state at each decision point**.
  Decision-surface scout complete (§3.1a): the real per-strike decisions (target+hold, evade, strike-back)
  live *inside* the round walk (5 `policy::` call sites), so this is a **resolver decision-source injection**
  (staging §8.2), not a menu-level swap — bounded (5 sites, ~4 decision types, already-symmetric resolver).
- **v1 decision scope:** targeting/hold, evade, strike-back, rank. Offensive-ability selection and Reckoning
  deferral are latent/unwired in the engine (§3.1a) → later layers, not v1.
- **Resolve unit:** **one sub-phase-cycle** is the mechanical resolution step (the engine's natural atomic
  resting step). Targeting choices batch into one resting point at each cycle's start. **Reactive decisions
  (evade, strike-back) each rest — DECIDED 2026-07-08 (user): prompt every one.** As each blow lands the
  fight pauses at a serializable state, shows the incoming blow on the table, and awaits the player's choice
  (evade for its tempo cost / endure; strike back / hold). No per-unit stance auto-apply — every reactive
  decision is a live player choice. The board settles after each cycle; ranks are re-declared per round.

**Still open (resolve during the design task, low-risk defaults noted):**
- **Token/status layer surfacing:** how much of deckbound's token/status layer (Guard/Cover/Mark/tempo/
  Charge…) appears as cards on the table vs. stays internal and is reflected only via health flips + the
  log. *Default:* keep internal for v1; surface only rank lanes, health piles, the damage pile, and the log.
- **Re-declaration cadence:** intentions are per-round in the engine — confirm the player re-declares ranks
  at each round boundary (not mid-round between cycles). *Default:* yes, declare once per round.
- **Card↔actor identity:** the concrete mapping key (arena pile order vs. a stored id on each card) — an
  implementation detail for stage 2.
