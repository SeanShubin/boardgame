# Aegis vs Sentinel

## Aegis → Sentinel   (kills in 4 round(s))

```
Aegis     crush-3 speed-2
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-2
  action 2  crush-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-1
  -- end round 1: acc clear
round 2
  action 1  crush-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  action 2  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 2: acc 3 WASTED (round reset)
round 3
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  crush-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  -- end round 3: acc clear
round 4
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  crush-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Aegis   (kills in 3 round(s))

```
Sentinel  slash-3 speed-2
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Sentinel** wins — kills in 3 vs 4.
