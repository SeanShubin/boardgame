# Lance vs Pike

## Lance → Pike   (kills in 2 round(s))

```
Lance     pierce-5 speed-3
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×2 = damage-10   acc 0+10=10 / toughness-3  FLIP (waste-7)   [#][ ][ ][ ]
  action 2  pierce-5 ×2 = damage-10   acc 0+10=10 / toughness-3  FLIP (waste-7)   [#][#][ ][ ]
  action 3  pierce-5 ×2 = damage-10   acc 0+10=10 / toughness-3  FLIP (waste-7)   [#][#][#][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-5 ×2 = damage-10   acc 0+10=10 / toughness-3  FLIP (waste-7)   [#][#][#][#]
```

## Pike → Lance   (kills in 4 round(s))

```
Pike      pierce-3 speed-3
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 3 WASTED (round reset)
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 3 WASTED (round reset)
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 3 WASTED (round reset)
round 4
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Lance** wins — kills in 2 vs 4.
