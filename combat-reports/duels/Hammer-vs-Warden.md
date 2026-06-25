# Hammer vs Warden

## Hammer → Warden   (kills in 6 round(s))

```
Hammer    might-8 crush speed-1 daring-2
Warden    vitality-6 toughness-5 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-5  FLIP (waste-3)   [#][ ][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-5  FLIP (waste-3)   [#][#][ ][ ][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  crush might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-5  FLIP (waste-3)   [#][#][#][ ][ ][ ]
  -- end round 3: acc clear
round 4
  action 1  crush might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-5  FLIP (waste-3)   [#][#][#][#][ ][ ]
  -- end round 4: acc clear
round 5
  action 1  crush might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-5  FLIP (waste-3)   [#][#][#][#][#][ ]
  -- end round 5: acc clear
round 6
  action 1  crush might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-5  FLIP (waste-3)   [#][#][#][#][#][#]
```

## Warden → Hammer   (never (∞))

```
Warden    might-4 pierce speed-2 daring-2
Hammer    vitality-3 toughness-4 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ]
  action 2  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Hammer** wins — kills in 6 vs ∞.
