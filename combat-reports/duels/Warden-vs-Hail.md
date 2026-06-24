# Warden vs Hail

## Warden → Hail   (kills in 3 round(s))

```
Warden    pierce-3 speed-2
Hail      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  pierce-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Hail → Warden   (never (∞))

```
Hail      slash-2 speed-5
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-2 ×½ = damage-1   acc 0+1=1 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-2 ×½ = damage-1   acc 1+1=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  slash-2 ×½ = damage-1   acc 2+1=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 4  slash-2 ×½ = damage-1   acc 3+1=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 5  slash-2 ×½ = damage-1   acc 4+1=5 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  -- end round 1: acc 5 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Warden** wins — kills in 3 vs ∞.
