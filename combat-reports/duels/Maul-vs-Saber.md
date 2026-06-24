# Maul vs Saber

## Maul → Saber   (kills in 2 round(s))

```
Maul      crush-6 speed-2
Saber     health-4 toughness-3 armor-plate
start     [ ][ ][ ][ ]
round 1
  action 1  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-3  FLIP (waste-9)   [#][ ][ ][ ]
  action 2  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-3  FLIP (waste-9)   [#][#][ ][ ]
  -- end round 1: acc clear
round 2
  action 1  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-3  FLIP (waste-9)   [#][#][#][ ]
  action 2  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-3  FLIP (waste-9)   [#][#][#][#]
```

## Saber → Maul   (kills in 4 round(s))

```
Saber     slash-4 speed-3
Maul      health-4 toughness-4 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][ ][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][ ][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][ ][ ]
  action 3  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  -- end round 2: acc 2 WASTED (round reset)
round 3
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][ ][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][ ]
  action 3  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  -- end round 3: acc 2 WASTED (round reset)
round 4
  action 1  slash-4 ×½ = damage-2   acc 0+2=2 / toughness-4  no flip   [#][#][#][ ]
  action 2  slash-4 ×½ = damage-2   acc 2+2=4 / toughness-4  FLIP   [#][#][#][#]
```

## Verdict

**Maul** wins — kills in 2 vs 4.
