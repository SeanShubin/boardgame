# Hail vs Render

## Hail → Render   (kills in 1 round(s))

```
Hail      slash-2 speed-5
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 2  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 3  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-4  FLIP   [#][#][#][ ]
  action 4  slash-2 ×2 = damage-4   acc 0+4=4 / toughness-4  FLIP   [#][#][#][#]
```

## Render → Hail   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Hail      health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP×2 (cleave)   [#][#][ ][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 1+5=6 / toughness-2  FLIP×3 (cleave)   [#][#][#][#][#][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-2  FLIP   [#][#][#][#][#][#]
```

## Verdict

**Hail** wins — kills in 1 vs 2.
