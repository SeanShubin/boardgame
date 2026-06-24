# Pike vs Saber

## Pike → Saber   (kills in 4 round(s))

```
Pike      pierce-3 speed-3
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 3  pierce-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 3  pierce-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  action 3  pierce-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  action 3  pierce-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][#]
```

## Saber → Pike   (kills in 4 round(s))

```
Saber     slash-4 speed-3
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 3  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][#][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Verdict

**Pike** wins — kills in 4 vs 4.
