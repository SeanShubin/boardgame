# Hail vs Reaver

## Hail → Reaver   (kills in 4 round(s))

```
Hail      slash-2 speed-5
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 3  slash-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 4  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 5  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 3  slash-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][ ][ ]
  action 4  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 5  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 2  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  action 3  slash-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][ ]
  action 4  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 5  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 2  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  action 3  slash-2 ×½ = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][#]
```

## Reaver → Hail   (kills in 2 round(s))

```
Reaver    slash-3 speed-3 persist
Hail      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Verdict

**Reaver** wins — kills in 2 vs 4.
