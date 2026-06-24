# Aegis vs Gnat

## Aegis → Gnat   (kills in 3 round(s))

```
Aegis     crush-3 speed-2
Gnat      health-6 toughness-2 armor-cloth
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

## Gnat → Aegis   (kills in 5 round(s))

```
Gnat      pierce-2 speed-5
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  -- end round 1: acc 4 WASTED (round reset)
round 2
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 2: acc 4 WASTED (round reset)
round 3
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]
  -- end round 3: acc 4 WASTED (round reset)
round 4
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][ ]
  action 4  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]
  action 5  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 4: acc 4 WASTED (round reset)
round 5
  action 1  pierce-2 ×1 = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  pierce-2 ×1 = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 3  pierce-2 ×1 = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Aegis** wins — kills in 3 vs 5.
