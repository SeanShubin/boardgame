# Gnat vs Render

## Gnat → Render   (kills in 4 round(s))

```
Gnat      might-4 pierce speed-5 daring-4
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 3  pierce might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 4  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 5  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][ ][ ][ ]
  action 2  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 3  pierce might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][ ][ ]
  action 4  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 5  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][ ][ ]
  action 2  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][ ][ ]
  action 3  pierce might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][ ]
  action 4  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 5  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  pierce might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-3  no flip   [#][#][#][ ]
  action 2  pierce might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-3  no flip   [#][#][#][ ]
  action 3  pierce might-4 − resist-3 = damage-1   acc 2+1=3 / toughness-3  FLIP   [#][#][#][#]
```

## Render → Gnat   (kills in 1 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Gnat      vitality-6 toughness-2 resist-pierce-0 resist-slash-2 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP×3 (cleave)   [#][#][#][ ][ ][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-2  FLIP×3 (cleave)   [#][#][#][#][#][#]
```

## Verdict

**Render** wins — kills in 1 vs 4.
