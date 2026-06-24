# Gnat vs Sentinel

## Gnat → Sentinel   (kills in 5 round(s))

```
Gnat      pierce-2 speed-5
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-1
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  action 4  pierce-2 ×1 = damage-2   acc 3+2=5 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  action 5  pierce-2 ×1 = damage-2   acc 5+2=7 / toughness-6  FLIP (waste-1)   [#][ ][ ][ ][ ]  armor-quantity-0
  -- end round 1: acc clear
round 2
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-0
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 3: acc 4 WASTED (round reset)
round 4
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  -- end round 4: acc 4 WASTED (round reset)
round 5
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Gnat   (kills in 3 round(s))

```
Sentinel  slash-3 speed-2
Gnat      health-6 toughness-2 armor-cloth
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

**Sentinel** wins — kills in 3 vs 5.
