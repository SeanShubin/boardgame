# Lance vs Render

## Lance → Render   (kills in 2 round(s))

```
Lance     pierce-5 speed-3
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-4  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-4  FLIP (waste-1)   [#][#][ ][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-4  FLIP (waste-1)   [#][#][#][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-4  FLIP (waste-1)   [#][#][#][#]
```

## Render → Lance   (kills in 4 round(s))

```
Render    crush-5 speed-2 cleave
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  crush-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  crush-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  crush-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  crush-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  action 2  crush-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  crush-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  action 2  crush-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Lance** wins — kills in 2 vs 4.
