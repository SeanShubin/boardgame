# Reaver vs Render

## Reaver → Render   (kills in 2 round(s))

```
Reaver    slash-3 speed-3 persist
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  action 3  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 1: acc 0 carried (persist)
round 2
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Render → Reaver   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-3  FLIP   [#][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 2+5=7 / toughness-3  FLIP×2 (cleave)   [#][#][#][ ]
  -- end round 1: acc 1 WASTED (round reset)
round 2
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-3  FLIP   [#][#][#][#]
```

## Verdict

**Reaver** wins — kills in 2 vs 2.
