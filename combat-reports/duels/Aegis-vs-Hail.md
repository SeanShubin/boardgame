# Aegis vs Hail

## Aegis → Hail   (kills in 3 round(s))

```
Aegis     crush-3 speed-2
Hail      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 2  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Hail → Aegis   (kills in 3 round(s))

```
Hail      slash-2 speed-5
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][ ][ ][ ][ ]
  action 3  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 4  slash-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][ ][ ][ ]
  action 5  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 1: acc 4 WASTED (round reset)
round 2
  action 1  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  slash-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][ ][ ]
  action 3  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 4  slash-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][ ]
  action 5  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  slash-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][#]
```

## Verdict

**Hail** wins — kills in 3 vs 3.
