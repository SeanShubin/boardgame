# Sandstorm vs Pike

## Sandstorm → Pike   (kills in 2 round(s))

```
Sandstorm crush-2 speed-5
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 3  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 4  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 5  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 3  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][#][ ]
  action 4  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Pike → Sandstorm   (kills in 2 round(s))

```
Pike      pierce-3 speed-3
Sandstorm health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Verdict

**Pike** wins — kills in 2 vs 2.
