# Hail vs Render

## Hail → Render   (kills in 1 round(s))

```
Hail      might-4 slash speed-5 daring-4
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 3  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 4  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Render → Hail   (kills in 2 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Hail      vitality-6 toughness-2 resist-pierce-0 resist-slash-0 resist-crush-2
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-2 = damage-4   acc 0+4=4 / toughness-2  FLIP×2 (cleave)   [#][#][ ][ ][ ][ ]
  action 2  crush might-6 − resist-2 = damage-4   acc 0+4=4 / toughness-2  FLIP×2 (cleave)   [#][#][#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush might-6 − resist-2 = damage-4   acc 0+4=4 / toughness-2  FLIP×2 (cleave)   [#][#][#][#][#][#]
```

## Verdict

**Hail** wins — kills in 1 vs 2.
