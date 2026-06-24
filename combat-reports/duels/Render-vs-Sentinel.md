# Render vs Sentinel

## Render → Sentinel   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-5 ×2 = damage-10   acc 0+10=10 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-2
  action 2  crush-5 ×2 = damage-10   acc 4+10=14 / toughness-6  FLIP×2 (cleave)   [#][#][#][ ][ ]  armor-quantity-1
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush-5 ×2 = damage-10   acc 0+10=10 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  action 2  crush-5 ×1 = damage-5   acc 4+5=9 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Render   (kills in 2 round(s))

```
Sentinel  slash-3 speed-2
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Render** wins — kills in 2 vs 2.
