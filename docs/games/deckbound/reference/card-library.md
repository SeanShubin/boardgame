# Deckbound — Card Library

> **GENERATED — do not edit.** Regenerate with `cargo run -p deckbound --example handbook`.
> A projection of `crates/deckbound/data/booklet.ron` (the print master) and the Spec.

## Counts

| Cardset              | Cards   |
| -------------------- | ------: |
| Baseline             | 1       |
| Iron — Wall          | 11      |
| Silver — Infiltrator | 10      |
| Brass — Artillery    | 10      |
| Bone — Controller    | 10      |
| Salt — Support       | 10      |
| Weapons              | 9       |
| Pool                 | 20      |
| Cast                 | 28      |
| **Total**            | **109** |

## Baseline (1 cards)

| Level | Card  | Kind | Effect                                      |
| ----: | ----- | ---- | ------------------------------------------- |
| —     | Human | stat | might 1, vitality 5, toughness 1, cadence 3 |

## Iron — Wall (11 cards)

| Level | Card       | Kind    | Effect                    |
| ----: | ---------- | ------- | ------------------------- |
| 1     | Brace      | action  | brace +3 tough            |
| 1     | Iron L1    | stat    | vitality 2, toughness 1   |
| 2     | Phalanx    | passive | —                         |
| 2     | Iron L2    | stat    | vitality 2, toughness 1   |
| 3     | Aegis      | action  | cover an ally             |
| 3     | Iron L3    | stat    | vitality 3                |
| 4     | Bastion    | action  | cover an ally, x2 targets |
| 4     | Iron L4    | stat    | vitality 2, toughness 2   |
| 5     | Last Stand | action  | cannot fall               |
| 5     | Taunt      | passive | —                         |
| 5     | Iron L5    | stat    | vitality 4, toughness 2   |

## Silver — Infiltrator (10 cards)

| Level | Card          | Kind    | Effect               |
| ----: | ------------- | ------- | -------------------- |
| 1     | Slip Strike   | action  | might 3, shove       |
| 1     | Silver L1     | stat    | cadence 2, finesse 2 |
| 2     | Smoke         | action  | smoke                |
| 2     | Silver L2     | stat    | might 1, cadence 1   |
| 3     | Shadowstep    | passive | —                    |
| 3     | Silver L3     | stat    | cadence 2, finesse 2 |
| 4     | Coiled Strike | action  | charge +3            |
| 4     | Silver L4     | stat    | might 2              |
| 5     | Assassinate   | action  | might 9              |
| 5     | Silver L5     | stat    | might 2, cadence 1   |

## Brass — Artillery (10 cards)

| Level | Card        | Kind    | Effect                                   |
| ----: | ----------- | ------- | ---------------------------------------- |
| 1     | Bolt        | action  | might 3                                  |
| 1     | Brass L1    | stat    | might 2                                  |
| 2     | Volley      | action  | might 3, pin (deny a charge), x3 targets |
| 2     | Brass L2    | stat    | might 2                                  |
| 3     | Incendiary  | action  | burn 3x3                                 |
| 3     | Brass L3    | stat    | might 3                                  |
| 4     | Longshot    | passive | —                                        |
| 4     | Brass L4    | stat    | might 2                                  |
| 5     | Bombardment | action  | might 5, rout, x5 targets                |
| 5     | Brass L5    | stat    | might 2                                  |

## Bone — Controller (10 cards)

| Level | Card    | Kind    | Effect                                       |
| ----: | ------- | ------- | -------------------------------------------- |
| 1     | Sunder  | action  | sunder -2 tough                              |
| 1     | Bone L1 | stat    | might 1                                      |
| 2     | Mire    | action  | mire -2 cadence                              |
| 2     | Bone L2 | stat    | might 1                                      |
| 3     | Hex     | action  | sunder -2 tough, x3 targets                  |
| 3     | Bone L3 | stat    | might 1                                      |
| 4     | Curse   | passive | —                                            |
| 4     | Bone L4 | stat    | might 1                                      |
| 5     | Unmake  | action  | sunder -3 tough, defang -3 might, x3 targets |
| 5     | Bone L5 | stat    | might 1                                      |

## Salt — Support (10 cards)

| Level | Card      | Kind   | Effect                                           |
| ----: | --------- | ------ | ------------------------------------------------ |
| 1     | Haste     | action | haste +3                                         |
| 1     | Salt L1   | stat   | vitality 1                                       |
| 2     | Empower   | action | empower +3 might                                 |
| 2     | Salt L2   | stat   | vitality 1                                       |
| 3     | Thorns    | action | thorns 3                                         |
| 3     | Salt L3   | stat   | vitality 1, cadence 1                            |
| 4     | Mend      | action | mend +4                                          |
| 4     | Salt L4   | stat   | vitality 2                                       |
| 5     | Sanctuary | action | empower +3 might, haste +3, mend +4, x99 targets |
| 5     | Salt L5   | stat   | vitality 2                                       |

## Weapons (9 cards)

| Level | Card    | Kind   | Effect            |
| ----: | ------- | ------ | ----------------- |
| —     | Fist    | weapon | might weapon (+0) |
| —     | Blade   | weapon | might weapon (+0) |
| —     | Maul    | weapon | might weapon (+1) |
| —     | Claw    | weapon | might weapon (+0) |
| —     | Spear   | weapon | might weapon (+0) |
| —     | Staff   | weapon | might weapon (+0) |
| —     | Bow     | weapon | might weapon (+1) |
| —     | Wand    | weapon | might weapon (+2) |
| —     | Boulder | weapon | might weapon (+1) |

## Pool (20 cards)

| Level | Card       | Kind    | Effect              |
| ----: | ---------- | ------- | ------------------- |
| —     | Bash       | action  | might 0             |
| —     | Flame      | action  | might 3             |
| —     | Barrage    | action  | might 3, x3 targets |
| —     | Cleave     | action  | might 0, x3 targets |
| —     | Terror     | action  | stagger             |
| —     | Suppress   | action  | suppress -3 tempo   |
| —     | Slow       | action  | slow -2 cadence     |
| —     | Confuse    | action  | confuse -3 tempo    |
| —     | Ward       | action  | ward (grant melee)  |
| 4     | Mend       | action  | mend +3             |
| 1     | Haste      | action  | haste +3            |
| —     | Bank       | action  | +3 cadence          |
| 1     | Brace      | action  | brace +3 tempo      |
| 2     | Phalanx    | passive | —                   |
| —     | Bulwark    | passive | —                   |
| 5     | Taunt      | passive | —                   |
| —     | Blitz      | passive | —                   |
| 3     | Shadowstep | passive | —                   |
| —     | Backstab   | passive | —                   |
| 4     | Longshot   | passive | —                   |

## Cast (28 cards)

| Level | Card     | Kind      | Effect              |
| ----: | -------- | --------- | ------------------- |
| —     | Novice   | character | Human · Fist        |
| —     | Anvil    | character | Wall · Maul         |
| —     | Wisp     | character | Infiltrator · Blade |
| —     | Sear     | character | Artillery · Bow     |
| —     | Vow      | character | Support ·           |
| —     | Hex      | character | Controller · Wand   |
| —     | Recruit  | character | Militia · Spear     |
| —     | Archer   | character | Bowman · Bow        |
| —     | Husk     | character | Swarmer · Claw      |
| —     | Raider   | character | Infiltrator · Claw  |
| —     | Brute    | character | Bruiser · Maul      |
| —     | Slinger  | character | Skirmisher · Bow    |
| —     | Seer     | character | Hexer · Wand        |
| —     | Ogre     | character | Boss · Boulder      |
| —     | Golem    | character | Bulwark · Maul      |
| —     | Monolith | character | Bulwark · Maul      |
| —     | Mender   | character | Healer · Wand       |
| —     | Sentry   | character | Warden · Spear      |
| —     | Fighter  | character | Fighter · Blade     |
| —     | Assassin | character | Assassin · Claw     |
| —     | Mage     | character | Mage · Staff        |
| —     | Vael     | character | Duelist · Blade     |
| —     | Pell     | character | Charger · Claw      |
| —     | Bron     | character | Leader · Claw       |
| —     | Hollow   | character | Counter · Claw      |
| —     | Maw      | character | Brawler · Claw      |
| —     | Rage     | character | Feinter · Claw      |
| —     | Sable    | character | Duelist · Claw      |

