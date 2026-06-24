# Stiletto vs Sentinel

## Stiletto → Sentinel   (kills in 3 round(s))

```
Stiletto  pierce-3 speed-4 pen
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-1
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 4  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 4  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  -- end round 2: acc clear
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Stiletto   (kills in 2 round(s))

```
Sentinel  slash-3 speed-2
Stiletto  health-3 toughness-2 armor-cloth
start     [ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#]
```

## Verdict

**Sentinel** wins — kills in 2 vs 3.
