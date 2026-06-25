# Warden vs Render

## Warden → Render   (never (∞))

```
Warden    might-4 pierce speed-2 daring-2
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Render → Warden   (kills in 3 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Warden    vitality-6 toughness-5 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP   [#][ ][ ][ ][ ][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 1+6=7 / toughness-5  FLIP   [#][#][ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP   [#][#][#][ ][ ][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 1+6=7 / toughness-5  FLIP   [#][#][#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP   [#][#][#][#][#][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 1+6=7 / toughness-5  FLIP   [#][#][#][#][#][#]
```

## Verdict

**Render** wins — kills in 3 vs ∞.
