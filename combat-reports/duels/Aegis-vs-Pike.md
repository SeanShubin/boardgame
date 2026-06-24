# Aegis vs Pike

## Aegis → Pike   (kills in 2 round(s))

```
Aegis     crush-3 speed-2
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][ ]
  action 2  crush-3 ×1 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][#]
```

## Pike → Aegis   (kills in 5 round(s))

```
Pike      pierce-3 speed-3
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]
  -- end round 1: acc 3 WASTED (round reset)
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 2: acc 3 WASTED (round reset)
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]
  -- end round 3: acc 3 WASTED (round reset)
round 4
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]
  action 3  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 4: acc 3 WASTED (round reset)
round 5
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Aegis** wins — kills in 2 vs 5.
