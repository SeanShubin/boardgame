# Warden vs Sentinel

## Warden → Sentinel   (kills in 7 round(s))

```
Warden    pierce-3 speed-2
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-2
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-1
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 1+3=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]  armor-quantity-0
  -- end round 3: acc clear
round 4
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]  armor-quantity-0
  -- end round 4: acc clear
round 5
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]  armor-quantity-0
  -- end round 5: acc clear
round 6
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  -- end round 6: acc clear
round 7
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]  armor-quantity-0
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Warden   (never (∞))

```
Sentinel  slash-3 speed-2
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Warden** wins — kills in 7 vs ∞.
