# Maul vs Reaver

## Maul → Reaver   (kills in 2 round(s))

```
Maul      might-6 crush speed-2 daring-2
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][ ]
  action 2  crush might-6 − resist-3 = damage-3   acc 0+3=3 / toughness-3  FLIP   [#][#][#][#]
```

## Reaver → Maul   (kills in 3 round(s))

```
Reaver    might-3 slash speed-3 daring-3 persist
Maul      vitality-4 toughness-4 resist-pierce-3 resist-slash-0 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash might-3 − resist-0 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 3  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 3 carried (persist)
round 2
  action 1  slash might-3 − resist-0 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  action 2  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 3  slash might-3 − resist-0 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 2: acc 0 carried (persist)
round 3
  action 1  slash might-3 − resist-0 = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash might-3 − resist-0 = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Maul** wins — kills in 2 vs 3.
