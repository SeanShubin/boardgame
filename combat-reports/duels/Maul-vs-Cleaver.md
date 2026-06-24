# Maul vs Cleaver

## Maul → Cleaver   (kills in 2 round(s))

```
Maul      crush-6 speed-2
Cleaver   health-4 toughness-4 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][ ][ ][ ]
  action 2  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][#][#][ ]
  action 2  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-4  FLIP (waste-8)   [#][#][#][#]
```

## Cleaver → Maul   (kills in 4 round(s))

```
Cleaver   slash-6 speed-2
Maul      health-4 toughness-4 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][ ][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][ ][ ]
  -- end round 2: acc clear
round 3
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][ ][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][ ]
  -- end round 3: acc clear
round 4
  action 1  slash-6 ×½ = damage-3   acc 0+3=3 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash-6 ×½ = damage-3   acc 3+3=6 / toughness-4  FLIP (waste-2)   [#][#][#][#]
```

## Verdict

**Maul** wins — kills in 2 vs 4.
