# Bestiary / encounter verification (solver)

Verification of the encounter build (creatures-as-cards, `catalog::CREATURES` + the Bestiary + physical
foe instances). Run against the real `deckbound` resolver, seed 1. **Solos pass; the corners, as authored
on the cardtable, do not — and cannot be fixed by retuning the shared creatures. The project already holds
the tuned answer (`region-locks.ron`); reconciling the two is a design decision left for the user.**

## Solo encounters — CLEAN ✓

Each of the four inn-adjacent locations stations a single duel-locks creature. The existing instrument
proves the diagonal directly:

```
cargo run -p deckbound --example balance -- crates/deckbound/data/balance/duel-locks.ron

  kit \ foe        Anvil    Swarm     Coil   Mirage
  Executioner        WIN        ·        ·        ·
  Broadsider           ·      WIN        ·        ·
  Marksman             ·        ·      WIN        ·
  Phantom              ·        ·        ·      WIN
  clean diagonal — every creature is beaten by exactly its one key kit.
```

So every solo encounter is soloable by exactly one kit — Cinderwatch Keep (Coil→Marksman), The Sundered
Vault (Anvil→Executioner), Thornmarch Gate (Swarm→Broadsider), The Salt Barrows (Mirage→Phantom).

## Corner encounters — the cardtable composition does NOT verify ✗

The cardtable corners field **all four raw creatures with the keystone doubled** (`catalog::encounter_foes`).
Expressed in the region-locks format (`crates/deckbound/data/balance/bestiary-corners.ron`) and run:

```
cargo run -p deckbound --example balance -- crates/deckbound/data/balance/bestiary-corners.ron

  The Hollow Rampart (Broadsider)   full: winnable   — but Executioner REDUNDANT (win without it)
  Greywater Ford     (Marksman)     full: UNWINNABLE
  Emberfall Hollow   (Executioner)  full: UNWINNABLE
  Ninefold Deep      (Phantom)      full: winnable   — all four kits needed ✓  (the only clean corner)
```

Only 1 of 4 is clean. **Root cause:** the duel-locks creatures are tuned as *1v1 locks*. As party-corner
foes they misbehave — the raw Swarm (Vitality 45, doubled to 90) dwarfs the Broadsider's per-round AoE, and
the doubled T5 Anvil / M6 Coil overwhelm a four-body party inside the five-round horizon. Two corners become
strictly unwinnable; one no longer needs its signature kit.

### Why "retune once" can't save it here

The stopping rule was: retune once toward the intended result, else stop and report. **The corners share
their creatures with the solos**, so any stat change that eases a corner (e.g. shrinking the Swarm's
Vitality) simultaneously changes the *solo* Thorn Swarm and breaks the proven 1v1 diagonal. A single shared
stat-set cannot satisfy both "1v1 lock" and "party-necessity corner." No composition-only tweak fixes it
either — the oversized hoard is the dominant term in every corner.

### The tuned answer already exists

`region-locks.ron` is exactly the corner instrument, and it verifies **clean** — four regions, full party
wins each, every kit strictly needed (leave-one-out fails for all four):

```
cargo run -p deckbound --example balance -- crates/deckbound/data/balance/region-locks.ron
  The Sundered Vault / Overrun Warren / Riposte Bastion / Feint Hollow
  → full: winnable; without <each kit>: NEEDED (unwinnable without it), for all four, in every region.
```

It achieves this with **party-scaled foe variants** (Bulwark V4, Swarmling V10–28, Warden, Wisp) that are
*distinct from* the 1v1 locks — precisely the separation the raw-creature corners lack.

## Recommendation (a design decision, not done in this run)

The corners want **party-scaled foes**, separate from the solo locks. Options, for the user to pick:

1. **Adopt the region-locks foes for corners.** Give the Bestiary a second tier of party-scaled creatures
   (Bulwark/Swarmling/Warden/Wisp) and build corners from those; keep the four locks for solos. Proven clean
   today; costs a slightly larger Bestiary (two tiers of foe).
2. **Per-corner tuning in the duel-locks spirit**, mirroring what region-locks already did, folded into the
   cardtable catalog so the corners on the table match a passing balance file.
3. **Leave corners as a thematic preview** until combat is wired. No fight resolution exists on the cardtable
   yet, so the corners don't *play* — they only display. Defer the balance reconciliation to the combat slice
   (with fight/retire and kill-conservation), when it can be verified end-to-end.

Given combat isn't wired, **option 3 is the low-risk default**: the corners read correctly (all four foes,
keystone bulk, one signature threat) and the balance gap is documented here, to be closed alongside fight
resolution. Solos are already correct and proven.
