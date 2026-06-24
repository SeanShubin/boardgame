# Gnat vs Saber

## Gnat → Saber   (kills in 4 round(s))

```
Gnat      pierce-2 speed-5
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 4  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 5  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][ ][ ]
  action 4  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 5  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][ ]
  action 4  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 5  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][#]
```

## Saber → Gnat   (kills in 2 round(s))

```
Saber     slash-4 speed-3
Gnat      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][ ][ ][ ][ ][ ]
  action 2  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][ ][ ][ ][ ]
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][#][ ][ ]
  action 2  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][#][#][ ]
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][#][#][#]
```

## Verdict

**Saber** wins — kills in 2 vs 4.
