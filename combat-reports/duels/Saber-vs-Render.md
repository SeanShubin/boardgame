# Saber vs Render

## Saber → Render   (kills in 2 round(s))

```
Saber     might-6 slash speed-2 daring-2
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][ ][ ][ ]
  action 2  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][ ]
  action 2  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][#]
```

## Render → Saber   (kills in 4 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Saber     vitality-4 toughness-4 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 3+3=6 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Saber** wins — kills in 2 vs 4.
