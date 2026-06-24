# Cleaver vs Sentinel

## Cleaver → Sentinel   (kills in 3 round(s))

```
Cleaver   slash-6 speed-2
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-2
  action 2  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-1
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  action 2  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  -- end round 2: acc clear
round 3
  action 1  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Cleaver   (kills in 4 round(s))

```
Sentinel  slash-3 speed-2
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Cleaver** wins — kills in 3 vs 4.
