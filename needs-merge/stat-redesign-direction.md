# Stat redesign direction — through the lens of Charter #12

**Status:** design direction + test plan for ratification (2026-06-21). Suggestions only; no code or
canon edited (staged in `needs-merge/`). Builds on the now-canon **Charter #12** (*roles are the spine;
stats serve the roles*) and **Spec §8.6** (*the role set is necessary-and-sufficient*). Companion to
`needs-merge/stat-depth-audit.md`.

---

## 1. The reframe — not a from-scratch redesign, a conformance pass

The stat-depth audit judged each stat on **intrinsic depth** ("deep, or just bigger=better?"). #12
changes the criterion to **role-service** ("does it give some role its distinct teeth, and is every role
load-bearing?"). Re-running the audit through that lens **flips two verdicts and hardens two**:

| Stat | Audit verdict (intrinsic) | #12 verdict (role-service) | Action |
|---|---|---|---|
| **Drive** | shallow — "matters in one place" | **rehabilitated** — that one place is the **Infiltrator's** slip auction; a stat serving exactly one role distinctly is the ideal | **Keep** |
| **Pierce** | strictly dominated by Strike | **rehabilitated** — it's the **Artillery's anti-Wall key** (the triangle made stat-real); value is *being the lock-key*, not raw DPS | **Keep**, iff an armored lock scenario needs it (T2) |
| **Spirit** | unwired / dead | **hard fail** — the **Controller's** signature offense, read by nothing → fails distinctness + no-redundant-stat (T3) | **Wire — mandatory** |
| **Keystone** | unexercised | **hard fail** — no role needs it; never set in data | **Home it to a role, or cut** |

The rest of the set is the role substrate already and stays: Body/Toughness/Armor (Wall + chassis),
Speed (Infiltrator + chassis), Strike/Resolve (chassis). So: **the set is ~80% conformant; #12 demands a
punch-list, not a rewrite.**

### The punch-list (ordered)

1. **Wire Spirit** — inner (Fear) attack force = attacker's Spirit, mirroring Strike↔Power. Unblocks the
   Controller. *(Case 1 mechanics-fix; resulting magnitudes are human-tuned.)*
2. **Decide the Controller's identity** (§2.A) — attrition vs status/spike. Settles **Resolve** (pool or
   thin bar) **and Keystone** in one stroke.
3. **Decide Support's scaling** (§2.B) — card-only, or mint a signature stat. The one genuinely open
   case-3.
4. **Confirm Pierce and Drive each have a lock scenario** (T2 below) — else fold.
5. **Land the no-redundant-stat test (T3)** so the above can't silently regress — implementable now.

---

## 2. The two case-3 forks (for the human to settle)

### A. The Controller's identity — *attrition or status?*

This single choice resolves three loose ends (the inner-pool gap, Resolve's depth, Keystone):

- **Spike + status (recommended default).** The Controller's teeth are its **round-scoped statuses**
  (Stagger / Slow / Shove / Disarm) plus a **fear spike**; fear is a per-round event that clears, never an
  attrition meter. Then the **missing inner pool is a feature, not a gap** — it's guarded by §2.1's
  single-maintained-meter rule — and **Resolve stays a thin bar**. **Keystone** finds its home here: an
  **incorporeal foe** whose keystone is **Spirit** can only be killed by breaking its will → the
  Controller's fear becomes the *only* key → its **lock scenario** (satisfies T2, and activates Keystone).
- **Attrition.** The Controller grinds the inner channel like the Wall grinds Body. Then Resolve needs
  **depth** (a small inner pool, or the round-end fear-decay tweak — a Case-1 clock change inside Resolve),
  and that depth is a **second tracked quantity** brushing against §2.1. Heavier; only worth it if the
  spike+status identity proves too thin in play.

> **Recommendation:** spike+status. It keeps §2.1 intact, gives Keystone a real job, and makes the
> Controller mechanically distinct from the Wall (status-bender, not a second tank).

### B. Support's missing signature stat — *card-only, or a Potency stat?*

#12 exposes what the stat-only audit could not: **four of five roles scale their primary output through a
stat** — Artillery via Strike, Controller via Spirit (once wired), Wall via Armor/Toughness, Infiltrator
via Drive — **but Support's buffs/heals are flat card values** (Mend, Rally, Empower). A deeply-invested
Support heals the *same amount*; it grows only by acquiring better **Salt cards**, not by a stat.

- **Card-only (Support is "the all-cards role").** Distinctive identity: the one role whose power is
  entirely in its cards, scaling by *breadth of kit* not by a magnitude stat. Clean, no new stat.
  Risk: it sits oddly against **#5 doom-to-mastery** (every other role has a stat dial that pours into the
  power fantasy; Support's caps at its card values).
- **Mint a Potency stat (a Salt·Power).** A signature stat that scales Support's augment magnitude
  (heal/buff amount), exactly as Strike scales attack. Gives Support the same depth/breadth shape as the
  others and a clean #5 curve. Cost: one new stat — but **#12.1 explicitly licenses minting a stat to
  serve a role**, and it would be the symmetric partner the set is missing on the augment axis.

> **Recommendation (weaker — genuinely open):** lean **Potency stat**, because it makes Support scale
> like its peers (#5) and gives the augment axis a magnitude dial mirroring the others. But "all-cards
> role" is a legitimate identity; this is the human's taste call. *(Either way: a stat that exists must be
> consumed — T3 — so if Potency is minted it must be wired from day one, not parked like Spirit was.)*

---

## 3. Enforcement — the load-bearing-roles invariant test (plan)

**This is the artifact #12 / §8.6 hangs on:** without a measure, "each role is load-bearing" is a slogan.
Two tests, both grounded in the **existing** harness (`crates/deckbound/src/balance.rs`), which already
assembles an arbitrary party and resolves win/loss via `auto_resolve(party, foes, seed)`.

### The gap to close

`balance.rs` already has a *Tutorial / necessity* property — **but it is the wrong shape** for #12:

| | Current (`balance.rs`) | §8.6 needs (T2) |
|---|---|---|
| Construction | **equipped** party vs **unequipped** party (all rewards vs none) | **complete-coverage** party vs **complete-minus-one-role** party |
| Claim it proves | "you need *some* kit" | "you need *this specific role* — its absence dooms the party" |

The current check can't fail when one role is missing but the other four carry the fight. **The
leave-one-out construction is what captures "the absence of a single role dooms the party."**

### T2 — role-necessity (leave-one-out)

**Test subject: the god** (one character carrying all five suits = the tightest "complete coverage" unit).
For each role R, with R's **designated lock scenario** `L_R`:

- **Sufficiency:** `auto_resolve(god_all_five, L_R) == win` for every R. *(A complete build clears every
  lock.)*
- **Necessity:** `auto_resolve(god_minus_R, L_R) == loss` — the same god with **only R's suit removed**
  fails R's lock. *(If even a max-power god can't clear `L_R` without R, R is the irreplaceable key — raw
  power can't substitute.)*

Shape (using helpers that already exist — `god_rewards`, `rewards_up_to`, `REWARD_SUITS`,
`build_character`, `auto_resolve`):

```text
for R in REWARD_SUITS:
    L_R   = lock_encounter(R)                      // the per-role lock (see "authoring" below)
    full  = build_character("Novice", god_rewards(5))
    minus = build_character("Novice", god_rewards_except(R, 5))   // new one-line helper
    assert auto_resolve([full],  foes(L_R)) == Some(true)         // sufficiency
    assert auto_resolve([minus], foes(L_R)) == Some(false)        // NECESSITY — "absence dooms"
```

- **Ready now:** the party-assembly + resolve machinery, `god_rewards`, the per-suit `grind_encounter`.
- **Needs authoring:** the **five lock scenarios** `L_R`, one per role, each designed so that **only R's
  mechanic solves it** (e.g. a heavy-armor foe only **Pierce/Artillery** cracks; an **incorporeal** foe
  only **Controller**-fear kills; a swarm only **Support**-sustain / AoE endures; a backfield only an
  **Infiltrator** reaches; a frontal grind only a **Wall** holds). These **double as the role tutorials**
  (§2 of the principle / §8.4). Today's per-suit `grind_encounter(suit, 5)` is a first approximation but
  is **not yet tuned to be R-exclusive** — that tuning *is* the authoring task.
- **One-line helper to add:** `god_rewards_except(suit, k)` = `god_rewards(k)` minus `rewards_up_to(suit, k)`.

### T3 — no-redundant-stat (stat-necessity by zero-and-flip)

The runtime analog of T2 at **stat granularity** (no Rust reflection needed): a stat is load-bearing iff
**zeroing it flips some reference outcome.**

```text
for each stat field F in Offense/Defense:
    base    = reference outcomes (the grind ladder)
    zeroed  = same, but F forced to 0 (or Keystone forced Body) on every actor
    assert  zeroed differs from base in at least one encounter      // F is consumed somewhere
```

- **Catches the defects now:** **Spirit** zeroed changes **nothing** (it's never read) → **fails**.
  **Keystone** is never non-Body in data → forcing it changes nothing → **fails / untested** until a
  Spirit-keystone foe exists (which fork §2.A would add).
- **Cheap and immediate** — reuses `check_grind_balance`'s loop; needs no par solver. **Recommend landing
  it as an `#[ignore]` probe first** (so it documents the Spirit/Keystone defects as a red report without
  breaking CI), then promote to a hard assert once the punch-list closes.
- This is the test that makes "**stats serve the roles**" mechanically enforced: a stat no resolution path
  consumes is a *failing state, not a latent one* (§8.6 GUARANTEE).

### What stays deferred

- **Sufficiency over the *campaign*** (not a single encounter) — "a complete party clears the reference
  campaign" — needs **par over the world**, i.e. the par solver (§0.3, deferred). T2 above is the
  *encounter-level* proxy that runs today.
- **Distinctness (T4)** — "each role's lock falls to a *different* mechanic; a god comboing two roles beats
  either alone." Encoded once the lock scenarios exist; lower priority than T2/T3.

---

## 4. Disposition for the human

1. **Settle fork A** (Controller identity) — recommended *spike + status* → fixes Resolve + Keystone.
2. **Settle fork B** (Support scaling) — lean *Potency stat*, but genuinely your taste call.
3. **Approve the punch-list** (§1) — Spirit wiring is the only mandatory item; the rest follow A/B.
4. **Approve the test plan** (§3): land **T3 as an ignored probe now** (documents Spirit/Keystone as red),
   add the **`god_rewards_except` helper + T2 skeleton**, and queue the **five lock scenarios** as the
   authoring task (they double as tutorials). Promote T3 to a hard assert when the punch-list closes.

*Numbers, lock-scenario rosters, and the A/B taste calls stay human-authored; I seed and implement once
ratified.*
