# Card-Design Audit — does every card serve its role?

**Status:** design pass (suggestions only — 2026-06-21). The card-level companion to the stat-depth
audit, run through the now-canon lens: **#12** (cards *are* the roles' signature mechanics; distinct in
*kind*), **§8.6** (each role *emergently* load-bearing), **emergent-not-fiat**, **thematic naming**.
No code/canon/data edited. Numbers are `booklet.ron`'s and human-tuned; this audits *shape*, not values.

**Scope:** the **25 role-reward cards** (5 Suits × 5 levels, `booklet.ron` `rewards`) — the cards that
*are* the roles (§8.5). Weapons, traits, the standalone pool, and the cast are secondary.

**Headline:** the role sets are **mostly coherent and distinct** — Infiltrator, Controller, and Support
are clean; Wall is clean bar one card. The real findings are **two distinctness violations** (a control
card on Artillery; an augment card on the Wall), **one expression gap** (Artillery has the Pierce stat
but no Pierce *card*), and a **damage-type monoculture** the called-shot system doesn't pay for.

---

## Per-role read

### Iron / Wall — *hold the front* ✅ (one misfit)

| Lvl | Card | Effect | Kind | Verdict |
|---|---|---|---|---|
| 1 | Brace | Guard(+3 Tempo) | Base | ✅ defensive tempo to answer blows — on-role (survive the hold) |
| 2 | Phalanx | +hold | Modifier | ✅ the signature hold mechanic |
| 3 | Bodyguard | intercept for an ally | Modifier | ✅ extends the hold to the backline — distinct |
| 4 | **Rally** | Rally(+4 Resolve to allies) | Base | ⚠️ **an augment — Support's job** (see Cross-cutting #2) |
| 5 | Last Stand (+Taunt) | Lifeline+Steel; Taunt pulls fire | Base+Mod | ✅ the capstone hold; thematic |

Mechanic (hold the line, survive) is **distinct and positional**. Naming is strong (Phalanx, Taunt, Last
Stand). The one blemish is **Rally**: buffing allies' Resolve is an *augment* (Support's axis), and it
**doesn't scale with the Wall** (Inspiration is Salt's) — so a Wall's Rally is a flat, off-axis effect.

### Silver / Infiltrator — *slip & assassinate* ✅ (exemplary)

| Lvl | Card | Effect | Kind | Verdict |
|---|---|---|---|---|
| 1 | Slip Strike | Sharp 3 + Shove | Base | ✅ knocks a foe out of line — on-role |
| 2 | Blitz | first slip free | Modifier | ✅ signature slip mechanic |
| 3 | Shadowstep | win the slip tie | Modifier | ✅ signature slip mechanic |
| 4 | Backstab | bonus vs enemy Reserve | Modifier | ✅ the breakthrough payoff |
| 5 | Assassinate | Sharp 6, executes a Reserve | Base | ✅ the §8.6 "break through" prize |

**Cleanest role.** Every card is the slip-and-reach mechanic; three Modifiers tune the slip, two Bases
deliver the backline kill. Distinct in kind, emergently necessary (only an Infiltrator reaches the
Reserve), thematically named. Nothing to change.

### Brass / Artillery — *ranged fire* ⚠️ (a control card + a missing Pierce card)

| Lvl | Card | Effect | Kind | Verdict |
|---|---|---|---|---|
| 1 | Bolt | Sharp 3, ranged | Base | ✅ on-role |
| 2 | Volley | Sharp 3 ×3, ranged | Base | ✅ AoE — on-role |
| 3 | **Suppress** | Suppress(−3 Tempo) | Base | ❌ **a control effect — not fire** (Cross-cutting #1) |
| 4 | Longshot | reach enemy Reserve | Modifier | ✅ the sniper exception |
| 5 | Bombardment | Sharp 5 ×5 | Base | ✅ the capstone barrage |

Fire and AoE are distinct and on-role, but **L3 Suppress is tempo-denial — Controller's mechanic on the
damage role** (it's a near-duplicate of Bone's Confuse; see #1). And though Brass *owns the Pierce stat*
(its L2/L4 bundle), **no Brass card expresses armor-piercing** — `Sunder` (armor-shear) exists in the
pool but isn't a Brass reward. So Artillery's anti-Wall identity is **stat-only, not card-expressed** —
weak for an emergent armored-foe lock (#3).

### Bone / Controller — *degrade* ✅ (clean)

| Lvl | Card | Effect | Kind | Verdict |
|---|---|---|---|---|
| 1 | Slow | Slow(−2 Speed) | Base | ✅ cheapen the foe — on-role |
| 2 | Terror | Fear 4 + Stagger | Base | ✅ fear (scales off **Dread**) + action-denial |
| 3 | Confuse | Confuse(−3 Tempo) + Disarm | Base | ✅ tempo-denial + card-denial — on-role |
| 4 | Curse | debuffs hit +1 foe | Modifier | ✅ the signature debuff-amplifier |
| 5 | Unmake | Fear 4 ×3 | Base | ✅ mass fear capstone |

Coherent (Slow / Stagger / Disarm / fear), **position-agnostic** (correct per §4.4), Dread-scaled, well
named (Terror, Curse, Unmake). The *only* note is the inverse of #1: **Confuse owns tempo-denial**, so
Artillery's Suppress is the intruder, not this.

### Salt / Support — *augment* ✅ (clean; now Inspiration-scaled)

| Lvl | Card | Effect | Kind | Verdict |
|---|---|---|---|---|
| 1 | Haste | Haste(+2 Tempo) | Base | ✅ +Inspiration now scales it |
| 2 | Empower | Empower(+2 Power) | Base | ✅ +Inspiration scales it |
| 3 | Mend | Mend(+4 Body) | Base | ✅ +Inspiration scales it |
| 4 | Steel | Steel + Recover | Base | ✅ anti-fear / un-flip (binary — no magnitude to scale) |
| 5 | Sanctuary | Empower+Haste+Mend, party | Base | ✅ the mass-amplification capstone |

Coherent augment kit, all magnitude buffs now **scale with Inspiration** (the work we just shipped). All
**Base** cards, no Modifier — Support's signature is *breadth of kit*, not a passive (consistent with it
being the "all-cards" effect role). **Text is now stale**: the `text:` fields still read "Restores 4
Body," "+2 Power," etc. — they don't mention the +Inspiration scaling (Cross-cutting #5).

---

## Cross-cutting findings

**#1 — Suppress (Brass L3) ≈ Confuse (Bone L3): a control card on the damage role.** Both drain **3
Tempo**; Confuse merely adds Disarm. Tempo-denial is **Controller's mechanic** (#12.3: roles differ in
*kind*) — Artillery should *fire*, not control. **Fix:** replace Brass L3 with a damage/Pierce card
(see #3) and let Controller own tempo-denial. *(Case 1 — a re-home, no new mechanic.)*

**#2 — Rally (Iron L4): an augment on the Wall.** Buffing allies' Resolve is *Support's* axis, and it
**doesn't scale for the Wall** (no Inspiration). Either (a) accept it as the Wall's one diegetic
"steady the line" exception (defensible — the anchor inspires), or (b) move Rally to Salt and give Iron
L4 a hold/survival card. **Recommend (a)** *if* it stays flat-by-design and rare; flag if more
augment-on-Wall creeps in. *(Case 3 — touches role-distinctness intent.)*

**#3 — Artillery owns the Pierce stat but has no Pierce card.** `Sunder` (shear −2 Armor) sits unused in
the pool. **Fix:** make Brass L3 (freed by #1) an armor-bypass card — *Sunder* or an armor-piercing shot.
This gives Artillery a *played* anti-Wall answer, which is exactly the mechanic its **emergent armored-foe
lock** (§8.6) needs — closing the gap the stat-audit flagged (Pierce load-bearing only if a lock needs it).

**#4 — Damage-type monoculture.** Almost every attack card is **Sharp** (Bolt, Volley, Bombardment,
Barrage, Cleave, Slip Strike, Assassinate). Live types in the whole set: Sharp, Blunt (weapons), Heat
(Flame only), Fear (Terror/Unmake/Wand). **Cold, Lightning, and the Pierce/Confusion damage types are
unused.** The §2.2 typed-Armor / called-shot system is real complexity the **cards don't pay for** —
either diversify attack types (a Heat/Cold artillery line that punishes specific armors) or revisit
whether 8 damage types earn their slot (the deferred §2.2 case-3 question). *(Surfaced, not resolved.)*

**#5 — Salt card text is stale post-Inspiration.** Mend/Rally/Empower/Haste `text:` describe flat values;
they now scale +Inspiration. Per source-of-truth, card text should be **generated** from Spec keyword
manual lines, not hand-written — so the real fix is an augment keyword line noting "+Inspiration," then
text regenerates. *(Case 1 — a text/generation cleanup.)*

**#6 — Stale "Drive" in tutorial data.** `tutorials` scenario **5 ("Slip & Wall")** blurb still says
"Out-Drive… catch Drive… advance Drive" — a leftover the Drive→Daring rename didn't reach (it's a
scenario blurb, not a stat field). One-line cleanup.

---

## Emergent lock-scenario seeds (feeds the §8.6 residue)

The audit doubles as the five **emergent** per-role locks the reference scenario still needs (§8.6 /
`reference-scenario.md`). Each makes a role the *efficient* key — outpaced, not forbidden:

| Role | Lock foe (emergent) | Why only this role |
|---|---|---|
| **Wall** | a charging line whose front offense overruns you within the round | only a **hold** (Phalanx) stops the break; out-trading loses the race |
| **Infiltrator** | a lethal enemy **Reserve** (caster) behind its line | only a **slip** reaches it; everything else is out of range |
| **Artillery** | a **heavily-armored** front + a back you must out-range | only **Pierce/fire** cracks it efficiently (needs the #3 card) |
| **Controller** | an offense too strong to out-damage | only **Stagger/Slow** survives it; you must *disable*, not trade |
| **Support** | an **attrition** grind that out-lasts raw HP | only **heal/buff** sustains the line past its bare capacity |

The campaign scenarios (Ward the Cannon, Hold & Rain, The Five) already gesture at these — formalizing
them into the A/B/C/Final lattice is what turns §8.6's necessity from **headcount** into **emergent**.

---

## Whole-set verdict & priorities

**The card set is ~85% conformant** — like the stat set, it's a well-chosen role substrate, not arbitrary.
Three roles (Infiltrator, Controller, Support) are clean; Wall is clean bar Rally. The work is a small
punch-list, not a redesign:

1. **Re-home Suppress → give Brass L3 a Pierce card (#1 + #3).** The single highest-value fix: it
   removes the Artillery↔Controller overlap *and* gives Artillery its emergent anti-Wall card in one move.
2. **Regenerate Salt card text for Inspiration (#5)** — cheap, and closes the loop on the Inspiration work.
3. **Decide Rally's home (#2)** — keep-as-Wall-exception (recommended) or move to Salt.
4. **Fix the stale "Drive" tutorial blurb (#6)** — trivial.
5. **Open: damage-type monoculture (#4)** and the **five emergent locks** — the larger design threads,
   shared with the §8.6 residue.

*Suggestions only; numbers and the case-3 calls (Rally's home, damage-type collapse) stay human.*
