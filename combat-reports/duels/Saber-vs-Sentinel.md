# Saber vs Sentinel

## Saber → Sentinel   (kills in 5 round(s))

```
Saber     slash-4 speed-3
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  slash-4 ×1 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][ ][ ][ ][ ]  armor-quantity-1
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  -- end round 1: acc 4 WASTED (round reset)
round 2
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 2  slash-4 ×1 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][ ][ ][ ]  armor-quantity-0
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  slash-4 ×1 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][ ][ ]  armor-quantity-0
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 3: acc 4 WASTED (round reset)
round 4
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  slash-4 ×1 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][ ]  armor-quantity-0
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  -- end round 4: acc 4 WASTED (round reset)
round 5
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  slash-4 ×1 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Saber   (kills in 2 round(s))

```
Sentinel  slash-3 speed-2
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][#]
```

## Verdict

**Sentinel** wins — kills in 2 vs 5.
