---
description: Bring crates/deckbound into conformance with the Spec, with human gates and downstream re-sync
argument-hint: "[system/§ — e.g. §5 or \"zones/exhaustion\"; omit = ALL systems, aggressively]"
---

Update `crates/deckbound` to conform to the Spec. First read
`docs/games/deckbound/canon/0-source-of-truth.md`: Spec = truth, code = defect report,
numbers = human-tuned (`booklet.ron`). Work in phases; do NOT code until the gates clear.

1. PENDING EDITS (do this FIRST) — run `git status`. If the working tree is dirty,
   STOP and remind me to commit (or stash) before we start, so the automated work begins
   from a clean tree. Wait for me.

2. SCOPE — Target: **$ARGUMENTS**
   - If a system/§ is given, scope everything below to that Spec section.
   - If empty, target ALL systems, aggressively: sync every Spec section the code diverges
     from. Apply the completeness gate (3) per section — sync the ready ones; list the
     not-ready ones and do NOT code them.

3. COMPLETENESS GATE (push back on me) — confirm each target § has full RULE/WHY/
   GUARANTEES, no blocking ⬜ stub or unresolved (OPEN) dial, no internal contradiction,
   and the `booklet.ron` values it needs. If anything is underspecified or ambiguous,
   STOP and list exactly what's missing — do NOT code.

4. DECISIONS UP FRONT — in ONE batch, surface every human call: numbers to set, intent
   (case-3) conflicts, ambiguities. Classify each (mechanics-fix / invariant-risk /
   intent-change). Wait for my answers; don't drip questions out later.

5. HANDOFF GATE — once all decisions are in:
   a. Re-run `git status`. If anything changed since step 1, ask me to commit those too,
      and wait.
   b. Ask for my EXPLICIT confirmation to begin the automated portion, and WAIT for it.
   c. Only after I confirm, state: "Decisions collected, tree clean — starting automated
      implementation now." Nothing after this line is interactive.

6. IMPLEMENT — code to match the Spec (never the reverse; never soften a WHY/GUARANTEE
   to make code pass). Order: `booklet.ron` numbers → code → tests.

7. RE-SYNC DOWNSTREAM (aggressively) — bring everything to the new state:
   • in-game docs / encyclopedia (generated from Spec `TERM` lines + `booklet.ron` —
     regenerate, never hand-write)
   • scenarios + tutorials (`booklet.ron` content, `scenario-plan.md`)
   • `reference-scenario.md` and its invariants
   • any generated projections (card lists, etc.)
   • `game-flow.md` if the cycle/phase structure changed
   Flag whatever drifted.

8. VERIFY — run `scripts/verify` (fmt + clippy + tests + build); report honestly —
   failures with output, anything skipped.
