# Pike vs Render

## Pike → Render   (kills in 4 round(s))

```
Pike      pierce-3 speed-3
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 3 WASTED (round reset)
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 3 WASTED (round reset)
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 3 WASTED (round reset)
round 4
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Render → Pike   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 2+5=7 / toughness-3  FLIP×2 (cleave)   [#][#][#][ ]
  -- end round 1: acc 1 WASTED (round reset)
round 2
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-3  FLIP   [#][#][#][#]
```

## Verdict

**Render** wins — kills in 2 vs 4.
