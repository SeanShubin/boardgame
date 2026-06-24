# Lance vs Gnat

## Lance → Gnat   (kills in 2 round(s))

```
Lance     pierce-5 speed-3
Gnat      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][ ][ ][ ][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][ ][ ][ ][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][#][#][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][#][#][#][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][#][#][#][#]
```

## Gnat → Lance   (kills in 2 round(s))

```
Gnat      pierce-2 speed-5
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 3  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 4  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 5  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ]
  action 3  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  action 4  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Gnat** wins — kills in 2 vs 2.
