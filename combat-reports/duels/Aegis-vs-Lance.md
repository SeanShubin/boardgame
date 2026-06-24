# Aegis vs Lance

## Aegis → Lance   (never (∞))

```
Aegis     crush-3 speed-2
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  crush-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  crush-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Lance → Aegis   (kills in 5 round(s))

```
Lance     pierce-5 speed-3
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][ ][ ][ ][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][ ][ ][ ][ ]
  -- end round 1: acc 5 WASTED (round reset)
round 2
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][ ][ ][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][ ][ ][ ]
  -- end round 2: acc 5 WASTED (round reset)
round 3
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][#][ ][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][ ][ ]
  -- end round 3: acc 5 WASTED (round reset)
round 4
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][#][#][ ]
  action 3  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 4: acc 5 WASTED (round reset)
round 5
  action 1  pierce-5 ×1 = damage-5   acc 0+5=5 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  pierce-5 ×1 = damage-5   acc 5+5=10 / toughness-6  FLIP (waste-4)   [#][#][#][#][#]
```

## Verdict

**Lance** wins — kills in 5 vs ∞.
