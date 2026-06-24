# Pike vs Sentinel

## Pike → Sentinel   (kills in 6 round(s))

```
Pike      pierce-3 speed-3
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-1
  action 3  pierce-3 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  -- end round 1: acc 3 WASTED (round reset)
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-0
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  -- end round 2: acc 3 WASTED (round reset)
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-0
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 3: acc 3 WASTED (round reset)
round 4
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 4: acc 3 WASTED (round reset)
round 5
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  -- end round 5: acc 3 WASTED (round reset)
round 6
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Pike   (never (∞))

```
Sentinel  slash-3 speed-2
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Pike** wins — kills in 6 vs ∞.
