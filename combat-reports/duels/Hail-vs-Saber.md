# Hail vs Saber

## Hail → Saber   (kills in 2 round(s))

```
Hail      slash-2 speed-5
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 3  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 4  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 5  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  action 2  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 3  slash-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][#][ ]
  action 4  slash-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Saber → Hail   (kills in 2 round(s))

```
Saber     slash-4 speed-3
Hail      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][ ][ ][ ][ ][ ]
  action 2  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][ ][ ][ ][ ]
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][#][ ][ ]
  action 2  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][#][#][ ]
  action 3  slash-4 ×1 = damage-4   acc 0+4=4 / toughness-2  FLIP (waste-2)   [#][#][#][#][#][#]
```

## Verdict

**Saber** wins — kills in 2 vs 2.
