# Bulwark vs Reaver

## Bulwark → Reaver   (never (∞))

```
Bulwark   slash-3 speed-2
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Reaver → Bulwark   (kills in 4 round(s))

```
Reaver    slash-3 speed-3 persist
Bulwark   health-5 toughness-6 armor-plate
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][ ][ ][ ][ ]
  -- end round 1: acc 3 carried (persist)
round 2
  action 1  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][ ][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][ ][ ]
  -- end round 2: acc 0 carried (persist)
round 3
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-6  no flip   [#][#][#][#][ ]
  -- end round 3: acc 3 carried (persist)
round 4
  action 1  slash-3 ×1 = damage-3   acc 3+3=6 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Reaver** wins — kills in 4 vs ∞.
