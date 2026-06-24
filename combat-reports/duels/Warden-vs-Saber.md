# Warden vs Saber

## Warden → Saber   (never (∞))

```
Warden    pierce-3 speed-2
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Saber → Warden   (kills in 5 round(s))

```
Saber     slash-4 speed-3
Warden    health-5 toughness-6 armor-mail
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 3: acc clear
round 4
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 4: acc clear
round 5
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 3  slash-4 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Saber** wins — kills in 5 vs ∞.
