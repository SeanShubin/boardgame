# Gnat vs Reaver

## Gnat → Reaver   (kills in 1 round(s))

```
Gnat      pierce-2 speed-5
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 3  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 4  pierce-2 ×2 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Reaver → Gnat   (kills in 2 round(s))

```
Reaver    slash-3 speed-3 persist
Gnat      health-6 toughness-2 armor-cloth
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

**Gnat** wins — kills in 1 vs 2.
