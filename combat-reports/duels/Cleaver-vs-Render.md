# Cleaver vs Render

## Cleaver → Render   (kills in 4 round(s))

```
Cleaver   might-8 slash speed-1 daring-2
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  slash might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][#][#][#]
```

## Render → Cleaver   (kills in 3 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Cleaver   vitality-3 toughness-4 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ]
round 1
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][#][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][#][#]
```

## Verdict

**Render** wins — kills in 3 vs 4.
