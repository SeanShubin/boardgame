# Lance vs Stiletto

## Lance → Stiletto   (kills in 1 round(s))

```
Lance     pierce-5 speed-3
Stiletto  health-3 toughness-2 armor-cloth
start     [ ][ ][ ]
round 1
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP (waste-3)   [#][#][#]
```

## Stiletto → Lance   (kills in 2 round(s))

```
Stiletto  pierce-3 speed-4 pen
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 4  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 4  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Lance** wins — kills in 1 vs 2.
