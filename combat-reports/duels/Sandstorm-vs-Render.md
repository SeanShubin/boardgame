# Sandstorm vs Render

## Sandstorm → Render   (kills in 4 round(s))

```
Sandstorm crush-2 speed-5
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  crush-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 3  crush-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 4  crush-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 5  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 1 WASTED (round reset)
round 2
  action 1  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  crush-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 3  crush-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 4  crush-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 5  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 1 WASTED (round reset)
round 3
  action 1  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  action 2  crush-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][ ][ ]
  action 3  crush-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][ ][ ]
  action 4  crush-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][ ]
  action 5  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 1 WASTED (round reset)
round 4
  action 1  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  action 2  crush-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][#][ ]
  action 3  crush-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][#][ ]
  action 4  crush-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][#]
```

## Render → Sandstorm   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Sandstorm health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP×2 (cleave)   [#][#][ ][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 1+5=6 / toughness-2  FLIP×3 (cleave)   [#][#][#][#][#][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP   [#][#][#][#][#][#]
```

## Verdict

**Render** wins — kills in 2 vs 4.
