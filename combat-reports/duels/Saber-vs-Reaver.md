# Saber vs Reaver

## Saber → Reaver   (kills in 4 round(s))

```
Saber     slash-4 speed-3
Reaver    health-4 toughness-3 armor-mail
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

## Reaver → Saber   (kills in 2 round(s))

```
Reaver    slash-3 speed-3 persist
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][ ]
  -- end round 1: acc 0 carried (persist)
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][#]
```

## Verdict

**Reaver** wins — kills in 2 vs 4.
