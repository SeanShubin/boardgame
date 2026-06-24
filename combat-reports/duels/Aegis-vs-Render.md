# Aegis vs Render

## Aegis → Render   (never (∞))

```
Aegis     crush-3 speed-2
Render    health-4 toughness-4 armor-padded
start     [ ][ ][ ][ ]
round 1
  action 1  crush-3 ×½ = damage-1   acc 0+1=1 / toughness-4  no flip   [ ][ ][ ][ ]
  action 2  crush-3 ×½ = damage-1   acc 1+1=2 / toughness-4  no flip   [ ][ ][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
  -- walled: no path to a kill
```

## Render → Aegis   (never (∞))

```
Render    crush-5 speed-2 cleave
Aegis     health-5 toughness-6 armor-padded
start     [ ][ ][ ][ ][ ]
round 1
  action 1  crush-5 ×½ = damage-2   acc 0+2=2 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  action 2  crush-5 ×½ = damage-2   acc 2+2=4 / toughness-6  no flip   [ ][ ][ ][ ][ ]
  -- end round 1: acc 4 WASTED (round reset)
  -- walled: no path to a kill
```

## Verdict

**Draw** — neither closes it out.
