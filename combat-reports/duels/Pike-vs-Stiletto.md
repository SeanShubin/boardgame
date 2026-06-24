# Pike vs Stiletto

## Pike → Stiletto   (kills in 1 round(s))

```
Pike      pierce-3 speed-3
Stiletto  health-3 toughness-2 armor-cloth
start     [ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#]
```

## Stiletto → Pike   (kills in 1 round(s))

```
Stiletto  pierce-3 speed-4 pen
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][ ][ ][ ]
  action 2  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][ ][ ]
  action 3  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][ ]
  action 4  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][#]
```

## Verdict

**Stiletto** wins — kills in 1 vs 1.
