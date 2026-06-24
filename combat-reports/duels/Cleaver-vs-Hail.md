# Cleaver vs Hail

## Cleaver → Hail   (kills in 3 round(s))

```
Cleaver   slash-6 speed-2
Hail      health-6 toughness-2 armor-cloth
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

## Hail → Cleaver   (kills in 2 round(s))

```
Hail      slash-2 speed-5
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 3  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 4  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 5  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  action 2  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ]
  action 3  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  action 4  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Hail** wins — kills in 2 vs 3.
