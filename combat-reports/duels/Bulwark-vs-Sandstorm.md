# Bulwark vs Sandstorm

## Bulwark → Sandstorm   (kills in 3 round(s))

```
Bulwark   slash-3 speed-2
Sandstorm health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Sandstorm → Bulwark   (kills in 3 round(s))

```
Sandstorm crush-2 speed-5
Bulwark   health-5 toughness-6 armor-plate
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  crush-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][ ][ ][ ][ ]
  action 3  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 4  crush-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][ ][ ][ ]
  action 5  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 1: acc 4 WASTED (round reset)
round 2
  action 1  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  crush-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][ ][ ]
  action 3  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 4  crush-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][ ]
  action 5  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  crush-2 ×2 = damage-4   acc 0+4=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  crush-2 ×2 = damage-4   acc 4+4=8 / toughness-6  FLIP (waste-2)   [#][#][#][#][#]
```

## Verdict

**Sandstorm** wins — kills in 3 vs 3.
