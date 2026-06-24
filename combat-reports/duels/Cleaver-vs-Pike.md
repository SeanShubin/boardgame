# Cleaver vs Pike

## Cleaver → Pike   (kills in 2 round(s))

```
Cleaver   slash-6 speed-2
Pike      health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][ ]
  action 2  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][#]
```

## Pike → Cleaver   (never (∞))

```
Pike      pierce-3 speed-3
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  pierce-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  pierce-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 3  pierce-3 ×½ = damage-1   acc 2+1=3 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 3 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Cleaver** wins — kills in 2 vs ∞.
