# Cleaver vs Gnat

## Cleaver → Gnat   (kills in 3 round(s))

```
Cleaver   slash-6 speed-2
Gnat      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][ ][ ][ ][ ][ ]
  action 2  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][ ][ ][ ]
  action 2  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][ ]
  action 2  slash-6 ×1 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][#]
```

## Gnat → Cleaver   (kills in 4 round(s))

```
Gnat      pierce-2 speed-5
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 4  pierce-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 5  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 1 WASTED (round reset)
round 2
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 4  pierce-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 5  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 1 WASTED (round reset)
round 3
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][ ][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][ ][ ]
  action 4  pierce-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][ ]
  action 5  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 1 WASTED (round reset)
round 4
  action 1  pierce-2 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  action 2  pierce-2 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][#][ ]
  action 3  pierce-2 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][#][ ]
  action 4  pierce-2 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Cleaver** wins — kills in 3 vs 4.
