# Maul vs Sentinel

## Maul → Sentinel   (kills in 3 round(s))

```
Maul      crush-6 speed-2
Sentinel  health-5 toughness-6 armor-plate armor-quantity-3 brittle
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-6  FLIP (waste-6)   [#][ ][ ][ ][ ]  armor-quantity-2
  action 2  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-6  FLIP (waste-6)   [#][#][ ][ ][ ]  armor-quantity-1
  -- end round 1: acc clear
round 2
  action 1  crush-6 ×2 = damage-12   acc 0+12=12 / toughness-6  FLIP (waste-6)   [#][#][#][ ][ ]  armor-quantity-0
  action 2  crush-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][#][ ]  armor-quantity-0
  -- end round 2: acc clear
round 3
  action 1  crush-6 ×1 = damage-6   acc 0+6=6 / toughness-6  FLIP   [#][#][#][#][#]  armor-quantity-0
```

## Sentinel → Maul   (never (∞))

```
Sentinel  slash-3 speed-2
Maul      health-4 toughness-4 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  slash-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  slash-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Maul** wins — kills in 3 vs ∞.
