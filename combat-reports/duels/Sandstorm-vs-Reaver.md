# Sandstorm vs Reaver

## Sandstorm → Reaver   (kills in 2 round(s))

```
Sandstorm crush-2 speed-5
Reaver    health-4 toughness-3 armor-mail
start     [ ][ ][ ][ ]
round 1
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [ ][ ][ ][ ]
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][ ][ ][ ]
  action 3  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][ ][ ][ ]
  action 4  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][ ][ ]
  action 5  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  -- end round 1: acc 2 WASTED (round reset)
round 2
  action 1  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][ ][ ]
  action 2  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][ ]
  action 3  crush-2 ×1 = damage-2   acc 0+2=2 / toughness-3  no flip   [#][#][#][ ]
  action 4  crush-2 ×1 = damage-2   acc 2+2=4 / toughness-3  FLIP (waste-1)   [#][#][#][#]
```

## Reaver → Sandstorm   (kills in 2 round(s))

```
Reaver    slash-3 speed-3 persist
Sandstorm health-6 toughness-2 armor-cloth
start     [ ][ ][ ][ ][ ][ ]
round 1
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][ ][ ][ ][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][ ][ ][ ][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][ ][ ][ ]
  -- end round 1: acc 0 carried (persist)
round 2
  action 1  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][ ][ ]
  action 2  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][ ]
  action 3  slash-3 ×1 = damage-3   acc 0+3=3 / toughness-2  FLIP (waste-1)   [#][#][#][#][#][#]
```

## Verdict

**Draw** — neither closes it out.
