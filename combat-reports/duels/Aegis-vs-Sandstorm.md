# Aegis vs Sandstorm

## Aegis → Sandstorm   (kills in 3 round(s))

```
Aegis     crush-3 speed-2
Sandstorm health-6 toughness-2 armor-cloth
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

## Sandstorm → Aegis   (never (∞))

```
Sandstorm crush-2 speed-5
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-2 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  crush-2 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  crush-2 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 4  crush-2 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 5  crush-2 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  -- end round 1: acc 5 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Aegis** wins — kills in 3 vs ∞.
