# Sand vs Reaver

## Sand → Reaver   (kills in 4 round(s))

```
Sand      might-4 crush speed-5 daring-4
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 3  crush might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 4  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 5  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 3  crush might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][ ][ ]
  action 4  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 5  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 2  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  action 3  crush might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][ ]
  action 4  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 5  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  crush might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 2  crush might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  action 3  crush might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][#]
```

## Reaver → Sand   (kills in 2 round(s))

```
Reaver    might-3 slash speed-3 daring-3 persist
Sand      vitality-6 toughness-2 resist-pierce-2 resist-slash-0 resist-crush-0
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

**Reaver** wins — kills in 2 vs 4.
