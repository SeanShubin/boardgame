# Sandstorm vs Sentinel

## Sandstorm → Sentinel   (kills in 4 round(s))

```
Sandstorm crush-2 speed-5
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  crush-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][ ][ ][ ][ ]  armor-quantity-1
  action 3  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 4  crush-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-0
  action 5  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 3  crush-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  action 4  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 5  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 3  crush-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  action 4  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 5  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  -- end round 3: acc 4 WASTED (round reset)
round 4
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 3  crush-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Sandstorm   (kills in 3 round(s))

```
Sentinel  slash-3 speed-2
Sandstorm health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Verdict

**Sentinel** wins — kills in 3 vs 4.
