# Bulwark vs Render

## Bulwark → Render   (kills in 2 round(s))

```
Bulwark   might-4 slash speed-2 daring-2
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 2  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Render → Bulwark   (kills in 6 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Bulwark   vitality-6 toughness-5 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-5  no flip   [ ][ ][ ][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-5  FLIP   [#][ ][ ][ ][ ][ ]
  -- end round 1: acc 1 WASTED (round reset)
round 2
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-5  no flip   [#][ ][ ][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-5  FLIP   [#][#][ ][ ][ ][ ]
  -- end round 2: acc 1 WASTED (round reset)
round 3
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-5  no flip   [#][#][ ][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-5  FLIP   [#][#][#][ ][ ][ ]
  -- end round 3: acc 1 WASTED (round reset)
round 4
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-5  no flip   [#][#][#][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-5  FLIP   [#][#][#][#][ ][ ]
  -- end round 4: acc 1 WASTED (round reset)
round 5
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-5  no flip   [#][#][#][#][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-5  FLIP   [#][#][#][#][#][ ]
  -- end round 5: acc 1 WASTED (round reset)
round 6
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-5  no flip   [#][#][#][#][#][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-5  FLIP   [#][#][#][#][#][#]
```

## Verdict

**Bulwark** wins — kills in 2 vs 6.
