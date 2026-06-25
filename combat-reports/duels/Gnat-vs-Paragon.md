# Gnat vs Paragon

## Gnat → Paragon   (kills in 3 round(s))

```
Gnat      might-4 pierce speed-5 daring-4
Paragon   vitality-5 toughness-4 resist-pierce-2 resist-slash-2 resist-crush-2
start     [ ][ ][ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ][ ]
  action 2  pierce might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ][ ]
  action 3  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ][ ]
  action 4  pierce might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ][ ]
  action 5  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ][ ]
  action 2  pierce might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ][ ]
  action 3  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ][ ]
  action 4  pierce might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#][ ]
  action 5  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][#][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  pierce might-4 − resist-2 = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][#][ ]
  action 2  pierce might-4 − resist-2 = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#][#]
```

## Paragon → Gnat   (kills in 3 round(s))

```
Paragon   might-6 pierce slash crush speed-2 daring-4
Gnat      vitality-6 toughness-2 resist-pierce-0 resist-slash-2 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][ ][ ][ ][ ][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][ ][ ][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP (waste-4)   [#][#][#][#][#][#]
```

## Verdict

**Draw** — neither closes it out.
