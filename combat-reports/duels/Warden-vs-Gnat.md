# Warden vs Gnat

## Warden → Gnat   (kills in 3 round(s))

```
Warden    pierce-3 speed-2
Gnat      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Gnat → Warden   (kills in 3 round(s))

```
Gnat      pierce-2 speed-5
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  pierce-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][ ][ ][ ][ ]
  action 3  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 4  pierce-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][ ][ ][ ]
  action 5  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 1: acc 4 WASTED (round reset)
round 2
  action 1  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  pierce-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][ ][ ]
  action 3  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 4  pierce-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][ ]
  action 5  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  pierce-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][#]
```

## Verdict

**Gnat** wins — kills in 3 vs 3.
