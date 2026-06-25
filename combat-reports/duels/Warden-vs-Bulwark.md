# Warden vs Bulwark

## Warden → Bulwark   (kills in 6 round(s))

```
Warden    might-4 pierce speed-2 daring-2
Bulwark   vitality-6 toughness-5 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-5  no flip   [ ][ ][ ][ ][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 4+4=8 / toughness-5  FLIP (waste-3)   [#][ ][ ][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-5  no flip   [#][ ][ ][ ][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 4+4=8 / toughness-5  FLIP (waste-3)   [#][#][ ][ ][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-5  no flip   [#][#][ ][ ][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 4+4=8 / toughness-5  FLIP (waste-3)   [#][#][#][ ][ ][ ]
  -- end round 3: acc clear
round 4
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-5  no flip   [#][#][#][ ][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 4+4=8 / toughness-5  FLIP (waste-3)   [#][#][#][#][ ][ ]
  -- end round 4: acc clear
round 5
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-5  no flip   [#][#][#][#][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 4+4=8 / toughness-5  FLIP (waste-3)   [#][#][#][#][#][ ]
  -- end round 5: acc clear
round 6
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-5  no flip   [#][#][#][#][#][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 4+4=8 / toughness-5  FLIP (waste-3)   [#][#][#][#][#][#]
```

## Bulwark → Warden   (never (∞))

```
Bulwark   might-4 slash speed-2 daring-2
Warden    vitality-6 toughness-5 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash might-4 − resist-3 = damage-1   acc 0+1=1 / toughness-5  no flip   [ ][ ][ ][ ][ ][ ]
  action 2  slash might-4 − resist-3 = damage-1   acc 1+1=2 / toughness-5  no flip   [ ][ ][ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Warden** wins — kills in 6 vs ∞.
