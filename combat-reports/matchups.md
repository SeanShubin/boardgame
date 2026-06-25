# Matchup matrix (iso-level)

Cell = row attacker vs column defender, from the row's view: `W` win, `L` loss, `.` draw.

## Legend

- `0` — Lancer
- `1` — Saber
- `2` — Maul
- `3` — Pike
- `4` — Cleaver
- `5` — Hammer
- `6` — Gnat
- `7` — Hail
- `8` — Sand
- `9` — Warden
- `10` — Bulwark
- `11` — Reaver
- `12` — Render
- `13` — Paragon

| vs  | 0   | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **0** Lancer | — | W | L | W | W | W | L | W | L | L | W | W | . | L | 7/5/1
| **1** Saber | L | — | W | W | W | W | L | L | W | L | L | W | W | L | 7/6/0
| **2** Maul | W | L | — | W | W | W | W | L | L | W | L | W | . | L | 7/5/1
| **3** Pike | L | L | L | — | . | . | L | L | L | L | W | W | L | L | 2/9/2
| **4** Cleaver | L | L | L | . | — | . | L | L | L | L | L | L | L | L | 0/11/2
| **5** Hammer | L | L | L | . | . | — | L | L | L | W | L | L | L | L | 1/10/2
| **6** Gnat | W | W | L | W | W | W | — | . | . | W | W | W | L | . | 8/2/3
| **7** Hail | L | W | W | W | W | W | . | — | . | L | W | W | W | . | 8/2/3
| **8** Sand | W | L | W | W | W | W | . | . | — | W | L | L | W | . | 7/3/3
| **9** Warden | W | W | L | W | W | L | L | W | L | — | W | W | L | L | 7/6/0
| **10** Bulwark | L | W | W | L | W | W | L | L | W | L | — | W | W | L | 7/6/0
| **11** Reaver | L | L | L | L | W | W | L | L | W | L | L | — | W | L | 4/9/0
| **12** Render | . | L | . | W | W | W | W | L | L | W | L | L | — | L | 5/6/2
| **13** Paragon | W | W | W | W | W | W | . | . | . | W | W | W | W | — | 10/0/3

## Per-pair detail

| A       | B       | rounds A→B | rounds B→A | winner  |
| ------- | ------- | ---------: | ---------: | ------- |
| Lancer  | Saber   | 2          | 4          | Lancer  |
| Lancer  | Maul    | 4          | 2          | Maul    |
| Lancer  | Pike    | 2          | 4          | Lancer  |
| Lancer  | Cleaver | 2          | 4          | Lancer  |
| Lancer  | Hammer  | 3          | 4          | Lancer  |
| Lancer  | Gnat    | 3          | 1          | Gnat    |
| Lancer  | Hail    | 3          | 4          | Lancer  |
| Lancer  | Sand    | 3          | 1          | Sand    |
| Lancer  | Warden  | 3          | 2          | Warden  |
| Lancer  | Bulwark | 3          | ∞          | Lancer  |
| Lancer  | Reaver  | 2          | ∞          | Lancer  |
| Lancer  | Render  | 2          | 2          | draw    |
| Lancer  | Paragon | 3          | 2          | Paragon |
| Saber   | Maul    | 2          | 4          | Saber   |
| Saber   | Pike    | 3          | 4          | Saber   |
| Saber   | Cleaver | 2          | 4          | Saber   |
| Saber   | Hammer  | 2          | 4          | Saber   |
| Saber   | Gnat    | 3          | 1          | Gnat    |
| Saber   | Hail    | 3          | 1          | Hail    |
| Saber   | Sand    | 3          | 4          | Saber   |
| Saber   | Warden  | 6          | 2          | Warden  |
| Saber   | Bulwark | 3          | 2          | Bulwark |
| Saber   | Reaver  | 2          | 3          | Saber   |
| Saber   | Render  | 2          | 4          | Saber   |
| Saber   | Paragon | 3          | 2          | Paragon |
| Maul    | Pike    | 2          | 4          | Maul    |
| Maul    | Cleaver | 3          | 4          | Maul    |
| Maul    | Hammer  | 2          | 4          | Maul    |
| Maul    | Gnat    | 3          | 4          | Maul    |
| Maul    | Hail    | 3          | 1          | Hail    |
| Maul    | Sand    | 3          | 1          | Sand    |
| Maul    | Warden  | 3          | ∞          | Maul    |
| Maul    | Bulwark | 6          | 2          | Bulwark |
| Maul    | Reaver  | 2          | 3          | Maul    |
| Maul    | Render  | 2          | 2          | draw    |
| Maul    | Paragon | 3          | 2          | Paragon |
| Pike    | Cleaver | 3          | 3          | draw    |
| Pike    | Hammer  | 3          | 3          | draw    |
| Pike    | Gnat    | 6          | 1          | Gnat    |
| Pike    | Hail    | 6          | 3          | Hail    |
| Pike    | Sand    | 6          | 1          | Sand    |
| Pike    | Warden  | 6          | 2          | Warden  |
| Pike    | Bulwark | 6          | ∞          | Pike    |
| Pike    | Reaver  | 4          | ∞          | Pike    |
| Pike    | Render  | 4          | 1          | Render  |
| Pike    | Paragon | 5          | 2          | Paragon |
| Cleaver | Hammer  | 3          | 3          | draw    |
| Cleaver | Gnat    | 6          | 1          | Gnat    |
| Cleaver | Hail    | 6          | 1          | Hail    |
| Cleaver | Sand    | 6          | 3          | Sand    |
| Cleaver | Warden  | 6          | 2          | Warden  |
| Cleaver | Bulwark | 6          | 2          | Bulwark |
| Cleaver | Reaver  | 4          | 2          | Reaver  |
| Cleaver | Render  | 4          | 3          | Render  |
| Cleaver | Paragon | 5          | 2          | Paragon |
| Hammer  | Gnat    | 6          | 3          | Gnat    |
| Hammer  | Hail    | 6          | 1          | Hail    |
| Hammer  | Sand    | 6          | 1          | Sand    |
| Hammer  | Warden  | 6          | ∞          | Hammer  |
| Hammer  | Bulwark | 6          | 2          | Bulwark |
| Hammer  | Reaver  | 4          | 2          | Reaver  |
| Hammer  | Render  | 4          | 1          | Render  |
| Hammer  | Paragon | 5          | 2          | Paragon |
| Gnat    | Hail    | 2          | 2          | draw    |
| Gnat    | Sand    | 2          | 2          | draw    |
| Gnat    | Warden  | 3          | 3          | Gnat    |
| Gnat    | Bulwark | 3          | 3          | Gnat    |
| Gnat    | Reaver  | 1          | 4          | Gnat    |
| Gnat    | Render  | 4          | 1          | Render  |
| Gnat    | Paragon | 3          | 3          | draw    |
| Hail    | Sand    | 2          | 2          | draw    |
| Hail    | Warden  | 6          | 3          | Warden  |
| Hail    | Bulwark | 3          | 3          | Hail    |
| Hail    | Reaver  | 1          | 2          | Hail    |
| Hail    | Render  | 1          | 2          | Hail    |
| Hail    | Paragon | 3          | 3          | draw    |
| Sand    | Warden  | 3          | 3          | Sand    |
| Sand    | Bulwark | 6          | 3          | Bulwark |
| Sand    | Reaver  | 4          | 2          | Reaver  |
| Sand    | Render  | 1          | 1          | Sand    |
| Sand    | Paragon | 3          | 3          | draw    |
| Warden  | Bulwark | 6          | ∞          | Warden  |
| Warden  | Reaver  | 2          | ∞          | Warden  |
| Warden  | Render  | ∞          | 3          | Render  |
| Warden  | Paragon | 5          | 3          | Paragon |
| Bulwark | Reaver  | 2          | 4          | Bulwark |
| Bulwark | Render  | 2          | 6          | Bulwark |
| Bulwark | Paragon | 5          | 3          | Paragon |
| Reaver  | Render  | 2          | 2          | Reaver  |
| Reaver  | Paragon | 7          | 2          | Paragon |
| Render  | Paragon | 3          | 2          | Paragon |
