# Warden vs Reaver

## Warden → Reaver   (kills in 2 round(s))

```
Warden    pierce-3 speed-2
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][ ][ ][ ]
  action 2  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][ ]
  action 2  pierce-3 ×2 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][#]
```

## Reaver → Warden   (kills in 10 round(s))

```
Reaver    slash-3 speed-3 persist
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  -- end round 1: acc 3 carried (persist)
round 2
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 5+1=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  -- end round 2: acc 0 carried (persist)
round 3
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [#][ ][ ][ ][ ]
  -- end round 3: acc 3 carried (persist)
round 4
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 5+1=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 4: acc 0 carried (persist)
round 5
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 5: acc 3 carried (persist)
round 6
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 5+1=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 6: acc 0 carried (persist)
round 7
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [#][#][#][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [#][#][#][ ][ ]
  -- end round 7: acc 3 carried (persist)
round 8
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [#][#][#][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 5+1=6 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 8: acc 0 carried (persist)
round 9
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [#][#][#][#][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 9: acc 3 carried (persist)
round 10
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  slash-3 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [#][#][#][#][ ]
  action 3  slash-3 ×½ = damage-1   acc 5+1=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Warden** wins — kills in 2 vs 10.
