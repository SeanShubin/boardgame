# Bulwark vs Lance

## Bulwark → Lance   (kills in 2 round(s))

```
Bulwark   slash-3 speed-2
Lance     health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Lance → Bulwark   (kills in 5 round(s))

```
Lance     pierce-5 speed-3
Bulwark   health-5 toughness-6 armor-plate
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 3  pierce-5 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][ ][ ][ ][ ]
  action 3  pierce-5 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 3  pierce-5 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 3: acc clear
round 4
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][ ][ ]
  action 3  pierce-5 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][ ]
  -- end round 4: acc clear
round 5
  action 1  pierce-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [#][#][#][#][ ]
  action 2  pierce-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [#][#][#][#][ ]
  action 3  pierce-5 ×½ = damage-2   acc 4+2=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Bulwark** wins — kills in 2 vs 5.
