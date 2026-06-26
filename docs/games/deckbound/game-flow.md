# Deckbound — Game Flow & Structure (the complete map)

> **Re-synced (2026-06-26) to the §4.6 six-phase model.** The stat set is **Might · Vitality ·
> Toughness · Cadence · Finesse**, combat is **one channel** (untyped Might → health; no
> Fear / Dread / Resolve / Ward, no Armor), and ranged attacks are **evadable** (Spec §2.2 / §3.1 / §4.2).
> The round *cycle* was **replaced** by the §4.6 six phases — the old Charge / Muster / Gauntlet /
> Outrider / Rearguard model is retired. Canon (`canon/2-spec` §4 / §4.6) + the generated reference are
> authoritative; this is a map, not truth.

> **One place to find every cycle and phase in the game, largest to smallest.** This is a
> **map / index, not a source of truth** — each level points to where it is *authoritatively*
> defined: the **Spec** (`canon/2-spec`) for combat, the **design docs** for the still-non-canonical
> strategic layer. When those move, this map follows them.

## The nesting (largest → smallest)

```
Run                     the whole scenario (many Days); goal = clear the final location
└─ Day                  the strategic clock tick ("1 day passes"); FULL reset at its end
   └─ Encounter         one fight — one per character per Day; a sequence of Rounds
      └─ Round  (§4.6)   one combat cycle — six phases, one shared per-round Tempo pool:
         ├─ Standoff                   reveal the blind bid; lock positions (Vanguard / Rearguard);
         │                             cast Standing buffs/braces (auto-land)
         ├─ Fray                       the fronts engage — melee + instant ranged + defenses resolve
         │                             simultaneously; deaths here fix the breach list (per-unit lock)
         ├─ Volley                     free Vanguards charge the enemy Rearguard (or flank a survivor);
         │                             the rear answers FIRST (pre-empt)
         ├─ Breach                     chargers who survived the Volley land their blows; a kill here
         │                             disrupts a deferred (Reckoning) spell
         ├─ Reckoning                  deferred (slow) spells from survivors resolve last
         └─ Lull                       Refresh: Tempo refills (= Cadence × Finesse); Health persists; round++
```

Inside the **Fray / Volley / Breach**, a same-range engagement resolves as one of:

```
• a Trade   — the deterministic base: both deal base damage at once  (§4.2)
• a Clash   — [optional module] a 1v1 mix-up that replaces the Trade  (§1.0)
   └─ Beat  — one simultaneous card-reveal (RPS): Strike / Anticipate / Gather / Evade;
              the duel ENDS the instant a strike connects (ends-on-strike)
```

So the **single RPS matchup is a Beat**, the **Clash** is the run of Beats, and they exist only when
the optional module is on; otherwise a same-range engagement is a single Trade.

## Each level — what it is and who owns it

| Level         | What happens                                                                                                                                                      | Authoritative source                                 |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| **Run**       | the whole scenario; **win = clear the final location** (placeholder golf goal); run victory/defeat undefined                                                      | `progression §6`, `reference-scenario.md`; Spec §8 ⬜ |
| **Day**       | each character may **move 1 space**, use a **per-day ability** (deferred), and attempt **one Encounter**; all act in parallel; **full reset at the Day boundary** | `progression §6`                                     |
| **Encounter** | one fight; a sequence of **Rounds** until clear or retreat; foes drawn from the **threat deck** (a deck recipe scaled by level)                                   | `progression §2 / §4.1`                              |
| **Round**     | one **Standoff → Fray → Volley → Breach → Reckoning** pass, ending in the **Lull** (refresh)                                                                       | **Spec §4 / §4.6**                                   |
| **Phase**     | Standoff (reveal) · Fray · Volley · Breach · Reckoning · Lull — order-independent *within* each                                                                    | **Spec §4.6** (TERM *Phase 1 / Phase 2*)             |
| **Trade**     | a same-range engagement's deterministic resolution: simultaneous mutual base damage                                                                               | **Spec §4.2**                                        |
| **Clash**     | the **optional** 1v1 mix-up that replaces a Trade; a sequence of **Beats**; ends-on-strike; builds **Force**                                                      | **Spec §1.0**                                        |
| **Beat**      | the single RPS matchup: both pick a card, reveal at once, resolve                                                                                                 | **Spec §1.0**                                        |

## What resets at each boundary

| Boundary            | What resets                                                                                                                            |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| **Beat**            | the duel continues, or **ends on a connecting strike**; **Force** builds during the non-connecting dance (§1.0)                        |
| **Round → Lull**    | **Tempo refills *fully*** (= Cadence × Finesse); downs finalize; charge/breach state clears; Health persists (§4.6)                    |
| **Encounter end**   | **win →** Health restored (§2.1); **retreat →** state carried to the Day reset                                                         |
| **Day boundary**    | **everything**: Health and all Resource pools, all Action cards Recover to their start zones (`zones-exhaustion §7`; `progression §6`) |
| **Run**             | victory / defeat — **undefined** (placeholder: clear the final location in the fewest Days)                                            |

## Two things that are *not* part of the flow

- **No turn order.** Within any phase, all sides **commit simultaneously** and resolve
  **order-independently** — Cadence sizes the Tempo pool, never initiative (§3). **"Turn" is not a
  unit** in this game.
- **Card zones are *state*, not a cycle.** Hand / Active / Down describe where a card *is*, not when
  things happen — see [`zones-exhaustion-design.md`](zones-exhaustion-design.md).

## Status of each layer

- **Combat flow** (Round · Phase · Trade · Clash · Beat) — **specced** (Spec §1, §4).
- **Strategic flow** (Run · Day · Encounter) — **design in progress**, non-canonical
  ([`progression-design.md`](progression-design.md)); on the spec-first path to Spec §8.
