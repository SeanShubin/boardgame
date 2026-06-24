# Lance vs Sentinel

## Lance → Sentinel   (kills in 5 round(s))

```
Lance     pierce-5 speed-3
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-1
  action 3  pierce-5 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-0
  -- end round 1: acc clear
round 2
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][ ][ ][ ]  armor-quantity-0
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 2: acc 5 WASTED (round reset)
round 3
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][#][ ][ ]  armor-quantity-0
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 3: acc 5 WASTED (round reset)
round 4
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][#][#][ ]  armor-quantity-0
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  -- end round 4: acc 5 WASTED (round reset)
round 5
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Lance   (kills in 2 round(s))

```
Sentinel  slash-3 speed-2
Lance     health-4 toughness-4 armor-padded
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

**Sentinel** wins — kills in 2 vs 5.
