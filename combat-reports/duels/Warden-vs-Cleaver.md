# Warden vs Cleaver

## Warden → Cleaver   (never (∞))

```
Warden    pierce-3 speed-2
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Cleaver → Warden   (kills in 5 round(s))

```
Cleaver   slash-6 speed-2
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 3: acc clear
round 4
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 4: acc clear
round 5
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Cleaver** wins — kills in 5 vs ∞.
