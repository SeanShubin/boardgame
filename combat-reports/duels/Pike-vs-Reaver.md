# Pike vs Reaver

## Pike → Reaver   (kills in 2 round(s))

```
Pike      pierce-3 speed-3
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][ ][ ][ ]
  action 2  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][ ][ ]
  action 3  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][#]
```

## Reaver → Pike   (kills in 4 round(s))

```
Reaver    slash-3 speed-3 persist
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
round 2
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 2: acc 0 carried (persist)
round 3
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][ ]
  -- end round 3: acc 0 carried (persist)
round 4
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][#]
```

## Verdict

**Pike** wins — kills in 2 vs 4.
