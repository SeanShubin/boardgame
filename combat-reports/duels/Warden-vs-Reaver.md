# Warden vs Reaver

## Warden → Reaver   (kills in 2 round(s))

```
Warden    might-4 pierce speed-2 daring-2
Reaver    vitality-4 toughness-3 resist-pierce-0 resist-slash-0 resist-crush-3
start     [ ][ ][ ][ ]
round 1
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 2  pierce might-4 − resist-0 = damage-4   acc 0+4=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Reaver → Warden   (never (∞))

```
Reaver    might-3 slash speed-3 daring-3 persist
Warden    vitality-6 toughness-5 resist-pierce-0 resist-slash-3 resist-crush-0
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ][ ][ ][ ]
  action 2  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ][ ][ ][ ]
  action 3  slash might-3 − resist-3 = damage-0   bounced — wasted   [ ][ ][ ][ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
  -- walled: no path to a kill
```

## Verdict

**Warden** wins — kills in 2 vs ∞.
