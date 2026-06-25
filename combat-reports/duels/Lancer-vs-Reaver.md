# Lancer vs Reaver

## Lancer → Reaver   (kills in 2 round(s))

```
Lancer    might-6 pierce speed-2 daring-2
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][ ][ ][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-3  FLIP (waste-3)   [#][#][#][#]
```

## Reaver → Lancer   (never (∞))

```
Reaver    might-3 slash speed-3 daring-3 persist
Lancer    vitality-4 toughness-4 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ][ ]
  action 2  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ][ ]
  action 3  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
  -- walled: no path to a kill
```

## Verdict

**Lancer** wins — kills in 2 vs ∞.
