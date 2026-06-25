# Hail vs Paragon

## Hail → Paragon   (kills in 3 round(s))

```
Hail      might-4 slash speed-5 daring-4
Paragon   vitality-5 toughness-4 resist-pierce-2 resist-slash-2 resist-crush-2
start     [ ][ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ][ ]
  action 2  slash might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ][ ]
  action 3  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ][ ]
  action 4  slash might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ][ ]
  action 5  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ][ ]
  action 2  slash might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ][ ]
  action 3  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ][ ]
  action 4  slash might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#][ ]
  action 5  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][#][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  slash might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][#][ ]
  action 2  slash might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#][#]
```

## Paragon → Hail   (kills in 3 round(s))

```
Paragon   might-6 pierce slash crush speed-2 daring-4
Hail      vitality-6 toughness-2 resist-pierce-0 resist-slash-0 resist-crush-2
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][ ][ ][ ][ ][ ]
  action 2  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][ ][ ][ ]
  action 2  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][ ]
  action 2  slash might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][#]
```

## Verdict

**Draw** — neither closes it out.
