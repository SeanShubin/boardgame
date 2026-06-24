# Sandstorm vs Stiletto

## Sandstorm → Stiletto   (kills in 1 round(s))

```
Sandstorm crush-2 speed-5
Stiletto  health-3 toughness-2 armor-cloth
start     [ ][ ][ ]
round 1
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-2  FLIP   [#][ ][ ]
  action 2  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-2  FLIP   [#][#][ ]
  action 3  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-2  FLIP   [#][#][#]
```

## Stiletto → Sandstorm   (kills in 2 round(s))

```
Stiletto  pierce-3 speed-4 pen
Sandstorm health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 4  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Verdict

**Sandstorm** wins — kills in 1 vs 2.
