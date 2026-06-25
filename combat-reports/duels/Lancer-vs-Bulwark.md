# Lancer vs Bulwark

## Lancer → Bulwark   (kills in 3 round(s))

```
Lancer    might-6 pierce speed-2 daring-2
Bulwark   vitality-6 toughness-5 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP (waste-1)   [#][#][#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 2  pierce might-6 − resist-0 = damage-6   acc 0+6=6 / toughness-5  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Bulwark → Lancer   (never (∞))

```
Bulwark   might-4 slash speed-2 daring-2
Lancer    vitality-4 toughness-4 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Lancer** wins — kills in 3 vs ∞.
