# Warden vs Render

## Warden → Render   (kills in 4 round(s))

```
Warden    pierce-3 speed-2
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Render → Warden   (kills in 5 round(s))

```
Render    crush-5 speed-2 cleave
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  -- end round 1: acc 4 WASTED (round reset)
round 2
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 3: acc 4 WASTED (round reset)
round 4
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 4: acc 4 WASTED (round reset)
round 5
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  crush-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Warden** wins — kills in 4 vs 5.
