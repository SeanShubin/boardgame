# Maul vs Reaver

## Maul → Reaver   (kills in 2 round(s))

```
Maul      crush-6 speed-2
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-6 ×1 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][ ][ ][ ]
  action 2  crush-6 ×1 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-6 ×1 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][ ]
  action 2  crush-6 ×1 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][#]
```

## Reaver → Maul   (kills in 6 round(s))

```
Reaver    slash-3 speed-3 persist
Maul      health-4 toughness-4 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 3 carried (persist)
round 2
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 2: acc 2 carried (persist)
round 3
  action 1  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 3: acc 1 carried (persist)
round 4
  action 1  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][ ][ ]
  action 3  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][ ]
  -- end round 4: acc 0 carried (persist)
round 5
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][#][ ]
  action 3  slash-3 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][#][ ]
  -- end round 5: acc 3 carried (persist)
round 6
  action 1  slash-3 ×½ = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Maul** wins — kills in 2 vs 6.
