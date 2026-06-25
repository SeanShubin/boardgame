# Hail vs Reaver

## Hail → Reaver   (kills in 1 round(s))

```
Hail      might-4 slash speed-5 daring-4
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 3  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 4  slash might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Reaver → Hail   (kills in 2 round(s))

```
Reaver    might-3 slash speed-3 daring-3 persist
Hail      vitality-6 toughness-2 resist-pierce-0 resist-slash-0 resist-crush-2
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  action 3  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
round 2
  action 1  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  action 2  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 3  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Verdict

**Hail** wins — kills in 1 vs 2.
