# Bulwark vs Render

## Bulwark → Render   (kills in 2 round(s))

```
Bulwark   slash-3 speed-2
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  action 2  slash-3 ×2 = damage-6   acc 0+6=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Render → Bulwark   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Bulwark   health-5 toughness-6 armor-plate
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-5 ×2 = damage-10   acc 0+10=10 / toughness-6  FLIP   [#][ ][ ][ ][ ]
  action 2  crush-5 ×2 = damage-10   acc 4+10=14 / toughness-6  FLIP×2 (cleave)   [#][#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush-5 ×2 = damage-10   acc 0+10=10 / toughness-6  FLIP   [#][#][#][#][ ]
  action 2  crush-5 ×2 = damage-10   acc 4+10=14 / toughness-6  FLIP   [#][#][#][#][#]
```

## Verdict

**Render** wins — kills in 2 vs 2.
