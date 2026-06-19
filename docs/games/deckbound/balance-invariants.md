# Deckbound — Balance Invariants

> A **living registry** of the balance properties the tuned numbers must satisfy.
>
> These are **not** Spec GUARANTEES. A Spec GUARANTEE is *structural* — true by construction,
> independent of numbers. A **balance invariant** is a *target*: it holds or fails depending on the
> values in [`booklet.ron`](../../../crates/deckbound/data/booklet.ron), and is what the **balance
> harness / par solver** verifies (Spec [§0.3](canon/2-spec/README.md#03-separable-balance);
> [computability-and-balance.md](computability-and-balance.md) §6). Every entry is an instance of the
> §6 method — *interesting strategies tie near par; interesting beats boring* — made concrete.
>
> **Rules of this list.** State each invariant precisely enough to be **checked** (ideally by the
> solver). Don't delete a satisfied one — a satisfied invariant is a **regression guard**. When you
> add a mechanic or retune a number, this is the list the harness re-checks.
>
> **Status:** ✅ checked by a test · 🟡 partially evidenced · ⬜ stated, not yet verified.
>
> *Measurement is on the **deterministic core** (Spec §0.1) and against a fixed combat resolver —
> par is policy-relative (computability-and-balance.md §5). "Outcome" of a party on a scenario means
> the lexicographic pair **(clears the objective?, then fewest Days)**.*

---

## BI-1 — Role diversity dominates monotony 🟡

**Invariant.** At party size = the number of roles, the party with **one of each role** has a
**strictly better outcome** than **every** same-size party made of a **single repeated role**.
Strictly better = it clears content the mono party cannot, or clears the objective in **fewer
Days** (better par). Formally, for the all-distinct party `D` and each mono party `Mᵣ` (role `r`
repeated to fill the party), `D` beats `Mᵣ` in the `(clears?, −Days)` ordering, for every `r`.

**Why.** This is the teeth of Charter **#4** (asymmetry by design; coverage comes from the **team**,
not an evened-out roster) and **#8.5** (the five roles are the §4 triangle's splits — a balanced
team answers gates a mono team can't). It is the role-level instance of the §6 rule **interesting
(diverse) beats boring (mono)**: a single-role party is a degenerate composition that *should* be
dominated, by design — not a viable line we accidentally left on the table.

**Check.** **Partially evidenced today:** the reference scenario's final gate is tuned to **require
the full roster** — a mono party fails it while the specialist roster clears it
([`reference.rs::check_combat_bands`](../../../crates/deckbound/src/reference.rs)). **Full
verification** — that the diverse party beats *every* mono party in `(clears?, −Days)` across the
whole run — is **pending the par solver** (§0.3).

**Scope & notes.** Anchored to the reference scenario (the balance instrument); it should generalise
to any scenario authored to demand coverage. It compares the **all-distinct vs all-same** endpoints;
the partial-diversity spectrum in between (e.g. 3 distinct + 2 repeats) is the subject of future
invariants. It does **not** claim diversity is *strictly* best among *interesting* builds — only
that it dominates monotony; parity *among* diverse strategies is the separate §6 target.

---

## BI-2 — Solo viability is ordered, and inverse to synergy ⬜

**Invariant.** The single-role ("mono") parties are ranked by outcome in a fixed **solo-friendliness
order** — **Wall ≥ Artillery ≥ Infiltrator ≥ Controller ≥ Support** — and the one-of-each-role party
beats every one of them. Formally, on the reference scenario, in the `(clears?, −Days)` ordering:
`outcome(mono-Wall) ≥ outcome(mono-Artillery) ≥ outcome(mono-Infiltrator) ≥ outcome(mono-Controller)
≥ outcome(mono-Support)`, and `outcome(all-distinct) >` each. This is the **ordered sharpening of
BI-1** (which only asserts the last clause).

**Why.** The operational form of Charter **#4** (roles unbalanced *by design*; balance is in the
team). Solo viability tracks **how much of a role's kit fires without allies**: Wall is self-contained
(hold + sustain), the glass cannons (Artillery, then Infiltrator — whose slip-keyed kit is almost
entirely *relational*) work alone but fragile, and the effect roles produce *modifiers* with no base
of their own — Support is **power-0 / attack Neither**, pure amplification. Crucially this order is
**inverse to synergy**: the role weakest alone (Support) is the strongest *multiplier*, which is
precisely *why* a team exceeds its parts (Spec §8.5's `3 + 2`). Pinning the order keeps each role's
identity — "doomed alone, devastating combined" for Support/Controller — intact under tuning, and is
the role-level engine of the depth/breadth fork (#2).

**Check.** **Pending the par solver** — it is an ordering over six parties (five mono + the diverse
one) on the reference scenario. Today: directionally consistent with
[`reference.rs::check_combat_bands`](../../../crates/deckbound/src/reference.rs) (the objective needs
the full roster; a clean-slate "bare" character loses its gate). The **inverse-synergy** half — that
Support contributes the *most* as a team complement and Wall the *least* — is measurable as **marginal
team contribution** once the solver exists; until then it is the stated rationale, not a hard check.

**Scope & notes.** The **relative order** is the designer's stated gradient and the firm claim; the
**margins** are tuned, not fixed, so adjacent ties are allowed (`≥`, not `>`). Only monotonicity and
the endpoints are asserted (Wall = best mono, Support = worst mono, diverse beats all). **Subsumes
BI-1.** Anchored to the reference scenario; generalises to any coverage-demanding scenario.

---

## Adding an invariant

Copy the BI-1 shape: **Invariant** (precise enough to check), **Why** (the Charter north star /
Spec section it serves, and which §6 case — "interesting beats boring" or "interesting on par" — it
instances), **Check** (how it is or will be verified, honestly stating what's pending), and
**Scope & notes**. Give it the next `BI-N` id. When the par solver lands, each invariant should
become an assertion it runs, so a retune that breaks one fails the build (computability-and-balance.md
§4).

**See also:** [computability-and-balance.md](computability-and-balance.md) (the discipline) ·
[Spec §0](canon/2-spec/README.md#0-the-deterministic-core--separable-balance-) (separable balance) ·
[reference-scenario.md](reference-scenario.md) (the current harness) ·
[Charter](canon/1-charter.md) (#4, #8 via Spec §8.5).
