# Maul vs Render

## Maul → Render   (kills in 4 round(s))

```
Maul      crush-6 speed-2
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  crush-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  crush-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  crush-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  crush-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  crush-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  crush-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  crush-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Render → Maul   (kills in 2 round(s))

```
Render    crush-5 speed-2 cleave
Maul      health-4 toughness-4 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-4  FLIP   [#][ ][ ][ ]
  action 2  crush-5 ×1 = damage-5   acc 1+5=6 / toughness-4  FLIP   [#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush-5 ×1 = damage-5   acc 0+5=5 / toughness-4  FLIP   [#][#][#][ ]
  action 2  crush-5 ×1 = damage-5   acc 1+5=6 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Render** wins — kills in 2 vs 4.
