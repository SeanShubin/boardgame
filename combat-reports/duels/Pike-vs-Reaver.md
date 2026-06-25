# Pike vs Reaver

## Pike → Reaver   (kills in 4 round(s))

```
Pike      might-8 pierce speed-1 daring-2
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  pierce might-8 − resist-0 = damage-8   acc 0+8=8 / toughness-3  FLIP (waste-5)   [#][#][#][#]
```

## Reaver → Pike   (never (∞))

```
Reaver    might-3 slash speed-3 daring-3 persist
Pike      vitality-3 toughness-4 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ]
round 1
  action 1  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ]
  action 2  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ]
  action 3  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
  -- walled: no path to a kill
```

## Verdict

**Pike** wins — kills in 4 vs ∞.
