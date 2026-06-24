# Cleaver vs Reaver

## Cleaver → Reaver   (kills in 2 round(s))

```
Cleaver   slash-6 speed-2
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][ ]
  action 2  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][#]
```

## Reaver → Cleaver   (kills in 3 round(s))

```
Reaver    slash-3 speed-3 persist
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 3 carried (persist)
round 2
  action 1  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 2: acc 0 carried (persist)
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Cleaver** wins — kills in 2 vs 3.
