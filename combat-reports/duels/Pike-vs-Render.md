# Pike vs Render

## Pike → Render   (kills in 4 round(s))

```
Pike      might-8 pierce speed-1 daring-2
Render    vitality-4 toughness-3 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-8 − resist-3 = damage-5   acc 0+5=5 / toughness-3  FLIP (waste-2)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-8 − resist-3 = damage-5   acc 0+5=5 / toughness-3  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce might-8 − resist-3 = damage-5   acc 0+5=5 / toughness-3  FLIP (waste-2)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  pierce might-8 − resist-3 = damage-5   acc 0+5=5 / toughness-3  FLIP (waste-2)   [#][#][#][#]
```

## Render → Pike   (kills in 1 round(s))

```
Render    might-6 crush speed-2 daring-2 cleave
Pike      vitality-3 toughness-4 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ]
round 1
  action 1  crush might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-4  FLIP   [#][ ][ ]
  action 2  crush might-6 − resist-0 = damage-6   acc 2+6=8 / toughness-4  FLIP×2 (cleave)   [#][#][#]
```

## Verdict

**Render** wins — kills in 1 vs 4.
