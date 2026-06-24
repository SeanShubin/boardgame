# Lance vs Cleaver

## Lance → Cleaver   (kills in 4 round(s))

```
Lance     pierce-5 speed-3
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 3  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 3  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ]
  action 3  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#]
```

## Cleaver → Lance   (kills in 2 round(s))

```
Cleaver   slash-6 speed-2
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  slash-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][ ][ ][ ]
  action 2  slash-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][#][#][ ]
  action 2  slash-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][#][#][#]
```

## Verdict

**Cleaver** wins — kills in 2 vs 4.
