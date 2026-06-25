# Lancer vs Hail

## Lancer → Hail   (kills in 3 round(s))

```
Lancer    might-6 pierce speed-2 daring-2
Hail      vitality-6 toughness-2 resist-pierce-0 resist-slash-0 resist-crush-2
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][ ][ ][ ][ ][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][ ][ ][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][#]
```

## Hail → Lancer   (kills in 4 round(s))

```
Hail      might-4 slash speed-5 daring-4
Lancer    vitality-4 toughness-4 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 3  slash might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 4  slash might-4 − resist-3 = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 5  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 1 WASTED (round reset)
round 2
  action 1  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  slash might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 3  slash might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 4  slash might-4 − resist-3 = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 5  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 1 WASTED (round reset)
round 3
  action 1  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][ ][ ]
  action 2  slash might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][ ][ ]
  action 3  slash might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][ ][ ]
  action 4  slash might-4 − resist-3 = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][ ]
  action 5  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 1 WASTED (round reset)
round 4
  action 1  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-4  no flip   [#][#][#][ ]
  action 3  slash might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-4  no flip   [#][#][#][ ]
  action 4  slash might-4 − resist-3 = damage-1   acc 3+1=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Lancer** wins — kills in 3 vs 4.
