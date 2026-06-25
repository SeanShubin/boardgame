# Gnat vs Reaver

## Gnat → Reaver   (kills in 1 round(s))

```
Gnat      might-4 pierce speed-5 daring-4
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 3  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 4  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Reaver → Gnat   (kills in 4 round(s))

```
Reaver    might-3 slash speed-3 daring-3 persist
Gnat      vitality-6 toughness-2 resist-pierce-0 resist-slash-2 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash might-3 − resist-2 = damage-1   acc 0+1=1 / toughness-2  no flip   [ ][ ][ ][ ][ ][ ]
  action 2  slash might-3 − resist-2 = damage-1   acc 1+1=2 / toughness-2  FLIP   [#][ ][ ][ ][ ][ ]
  action 3  slash might-3 − resist-2 = damage-1   acc 0+1=1 / toughness-2  no flip   [#][ ][ ][ ][ ][ ]
  -- end round 1: acc 1 carried (persist)
round 2
  action 1  slash might-3 − resist-2 = damage-1   acc 1+1=2 / toughness-2  FLIP   [#][#][ ][ ][ ][ ]
  action 2  slash might-3 − resist-2 = damage-1   acc 0+1=1 / toughness-2  no flip   [#][#][ ][ ][ ][ ]
  action 3  slash might-3 − resist-2 = damage-1   acc 1+1=2 / toughness-2  FLIP   [#][#][#][ ][ ][ ]
  -- end round 2: acc 0 carried (persist)
round 3
  action 1  slash might-3 − resist-2 = damage-1   acc 0+1=1 / toughness-2  no flip   [#][#][#][ ][ ][ ]
  action 2  slash might-3 − resist-2 = damage-1   acc 1+1=2 / toughness-2  FLIP   [#][#][#][#][ ][ ]
  action 3  slash might-3 − resist-2 = damage-1   acc 0+1=1 / toughness-2  no flip   [#][#][#][#][ ][ ]
  -- end round 3: acc 1 carried (persist)
round 4
  action 1  slash might-3 − resist-2 = damage-1   acc 1+1=2 / toughness-2  FLIP   [#][#][#][#][#][ ]
  action 2  slash might-3 − resist-2 = damage-1   acc 0+1=1 / toughness-2  no flip   [#][#][#][#][#][ ]
  action 3  slash might-3 − resist-2 = damage-1   acc 1+1=2 / toughness-2  FLIP   [#][#][#][#][#][#]
```

## Verdict

**Gnat** wins — kills in 1 vs 4.
