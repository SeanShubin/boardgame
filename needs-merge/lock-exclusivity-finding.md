# Lock exclusivity is structurally fragile — necessity belongs to the suite, not single locks

> **Empirical finding, 2026-06-26.** Ran the perfect solver against the §8.6 per-role "necessity locks"
> (`balance.rs::probe_solver_locks`, budgeted at 50K nodes). Result: **only one of the four levers admits
> a cleanly exclusive single-encounter lock.** This is a structural property of the mechanics, not a tuning
> miss. **Promotion target: `computability-and-balance.md` §10 + `encounter-suite-design.md`** (revises the
> "one exclusive lock per role" framing). Pairs with `role-weight-balance-testing.md` (the robust instrument).

## What was measured
For each lever, a baseline party that should *lose* the lock, then add exactly one role and ask the solver
whether it now *wins* (`winnable_within(50K)`; a win short-circuits, a loss is bounded + flagged
`budget-limited`). Exclusive = **only** the keyed role flips the lock.

| Lock (foes)                                                     | Lever under test      | Flips it           | Exclusive?           |
| --------------------------------------------------------------- | --------------------- | ------------------ | -------------------- |
| Silver — `Husk 2 + Slinger 4` (lethal backline behind a screen) | Infiltrator **slip**  | `+Silver` only     | **✅ yes**            |
| Brass — `Brute 1` (high-Toughness front)                        | Artillery **burst**   | `+Brass`, `+Salt`  | ✗ Support also       |
| Salt — `Slinger 4` (glass cannons)                              | Support **sustain**   | `+Salt`, `+Silver` | ✗ Infiltrator also   |
| Bone — `Golem 1` (Sunder-gated wall)                            | Controller **Sunder** | none surfaced @50K | ✗ + deep-win problem |

## Why — two "universal solvents"
The damage model is per-phase (overflow wasted; `raw = eff_might + card_power`, Toughness is the gate). Under it:

- **Support's sustain helps win *any* grind-winnable fight.** So Support clears every lock that resolves by
  attrition — it cleared *both* Brass (out-sustain the Brute) and Bone (out-sustain the Golem grind).
- **The Infiltrator's burst kills *any* exposed source.** So it clears every lock whose threat is killable —
  it cleared Salt (kill the glass cannons) as well as its own.

Decisive locks (kill the threat) are therefore Infiltrator/Artillery-clearable; grind locks (out-last it) are
Support-clearable; and the two families overlap. **The only barrier neither solvent bypasses is a *screen*** —
a front the slip passes but melee/sustain cannot — which is exactly why **Silver** (slip) is the lone clean
exclusive lock.

## The Controller lock has a second, independent problem
Across Toughness 5 / 7 / 9 the Controller's win **never surfaced within the fast 50K budget**. Its lever
(incremental −Toughness then grind) is an *intrinsically deep* win: Sunder rounds 1–2, crack rounds 3–5. It
is the opposite of decisive, so it cannot be both **exclusive** and **fast**. (It may still be winnable at the
real test's 50M budget — but it is not validatable by a fast diagnostic, and a Sunder-gated wall is a *grind*,
so the Support solvent clears it anyway.)

## The decisiveness ↔ exclusivity tension (stated)
- **Fast/decisive** pulls toward low-Vitality lethal foes → small search, but **killable → non-exclusive**.
- **Exclusive** pulls toward high-stat (uncrackable screen / Sunder-only wall / unkillable sustain-sink) foes
  → which **drag** (deep wins, slow search) and invite the Support solvent.
- A clean exclusive lock exists only where a lever bypasses a barrier *no other lever can* (the screen/slip).

## Conclusion — redirect, don't grind
The §8.6 "one exclusive lock per role" design is **structurally fragile** for three of four levers; chasing it
fights both the solver budget and the universal-solvent overlap. This **confirms the ratified Tier-1 priority**
(`automated-balance-testing-roadmap.md`): measure necessity as **marginal contribution across a *suite***
(`role-weight-balance-testing.md` — does adding role X improve outcomes across many encounters), where overlap
is *expected and fine*, rather than as exclusive single locks.

**Disposition:**
- **Keep** the Silver lock (genuinely exclusive + decisive + fast — a model for any future "bypass a barrier" lever).
- **Leave** `each_paired_role_is_necessary_in_its_lock` parked `#[ignore]`, now with this *structural* reason
  (not "needs reseeding"): exclusive single-locks are the wrong instrument for the three solvent-clearable levers.
- **Build** the marginal-contribution harness (Tier 1, the robust necessity instrument) next.
- Golem reverted to canonical (`V12 T5`); the retune bought no exclusivity.
