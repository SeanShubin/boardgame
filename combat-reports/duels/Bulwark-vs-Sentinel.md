# Bulwark vs Sentinel

## Bulwark → Sentinel   (kills in 5 round(s))

```
Bulwark   slash-3 speed-2
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-1
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 2: acc clear
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 3: acc clear
round 4
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  -- end round 4: acc clear
round 5
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Bulwark   (kills in 5 round(s))

```
Sentinel  slash-3 speed-2
Bulwark   health-5 toughness-6 armor-plate
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 3: acc clear
round 4
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 4: acc clear
round 5
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Bulwark** wins — kills in 5 vs 5.
