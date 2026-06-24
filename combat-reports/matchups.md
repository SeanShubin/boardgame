# Matchup matrix (iso-level)

Cell = row attacker vs column defender, from the row's view: `W` win, `L` loss, `.` draw.

## Legend

- `0` — Bulwark
- `1` — Warden
- `2` — Aegis
- `3` — Maul
- `4` — Lance
- `5` — Cleaver
- `6` — Gnat
- `7` — Hail
- `8` — Sandstorm
- `9` — Pike
- `10` — Saber
- `11` — Stiletto
- `12` — Reaver
- `13` — Render
- `14` — Sentinel

| vs | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 |
|---|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|
| **0** Bulwark | — | . | . | L | W | L | W | W | L | . | W | W | L | L | W | 6/5/3
| **1** Warden | . | — | . | W | L | L | L | W | W | L | L | L | W | W | W | 6/6/2
| **2** Aegis | . | . | — | W | L | W | W | L | W | W | L | W | L | . | L | 6/5/3
| **3** Maul | W | L | L | — | L | W | L | W | L | L | W | L | W | L | W | 6/8/0
| **4** Lance | L | W | W | W | — | L | L | L | W | W | L | W | L | W | L | 7/7/0
| **5** Cleaver | W | W | L | L | W | — | W | L | L | W | L | L | W | L | W | 7/7/0
| **6** Gnat | L | W | L | W | W | L | — | W | W | W | L | W | W | W | L | 9/5/0
| **7** Hail | L | L | W | L | W | W | L | — | . | L | L | W | L | W | L | 5/8/1
| **8** Sandstorm | W | L | L | W | L | W | L | . | — | L | W | W | . | L | L | 5/7/2
| **9** Pike | . | W | L | W | L | L | L | W | W | — | W | L | W | L | W | 7/6/1
| **10** Saber | L | W | W | L | W | W | W | W | L | L | — | L | L | L | L | 6/8/0
| **11** Stiletto | L | W | L | W | L | W | L | L | L | W | W | — | W | L | L | 6/8/0
| **12** Reaver | W | L | W | L | W | L | L | W | . | L | W | L | — | W | W | 7/6/1
| **13** Render | W | L | . | W | L | W | L | L | W | W | W | W | L | — | W | 8/5/1
| **14** Sentinel | L | L | W | L | W | L | W | W | W | L | W | W | L | L | — | 7/7/0

## Per-pair detail

| A | B | rounds A→B | rounds B→A | winner |
|---|---|--:|--:|---|
| Bulwark | Warden | ∞ | ∞ | draw |
| Bulwark | Aegis | 3 | 3 | draw |
| Bulwark | Maul | ∞ | 3 | Maul |
| Bulwark | Lance | 2 | 5 | Bulwark |
| Bulwark | Cleaver | 4 | 3 | Cleaver |
| Bulwark | Gnat | 3 | ∞ | Bulwark |
| Bulwark | Hail | 3 | 5 | Bulwark |
| Bulwark | Sandstorm | 3 | 3 | Sandstorm |
| Bulwark | Pike | ∞ | ∞ | draw |
| Bulwark | Saber | 2 | 5 | Bulwark |
| Bulwark | Stiletto | 2 | 3 | Bulwark |
| Bulwark | Reaver | ∞ | 4 | Reaver |
| Bulwark | Render | 2 | 2 | Render |
| Bulwark | Sentinel | 5 | 5 | Bulwark |
| Warden | Aegis | 5 | 5 | draw |
| Warden | Maul | 2 | 3 | Warden |
| Warden | Lance | 4 | 2 | Lance |
| Warden | Cleaver | ∞ | 5 | Cleaver |
| Warden | Gnat | 3 | 3 | Gnat |
| Warden | Hail | 3 | ∞ | Warden |
| Warden | Sandstorm | 3 | 5 | Warden |
| Warden | Pike | 2 | 2 | Pike |
| Warden | Saber | ∞ | 5 | Saber |
| Warden | Stiletto | 2 | 2 | Stiletto |
| Warden | Reaver | 2 | 10 | Warden |
| Warden | Render | 4 | 5 | Warden |
| Warden | Sentinel | 7 | ∞ | Warden |
| Aegis | Maul | 4 | 5 | Aegis |
| Aegis | Lance | ∞ | 5 | Lance |
| Aegis | Cleaver | 2 | 3 | Aegis |
| Aegis | Gnat | 3 | 5 | Aegis |
| Aegis | Hail | 3 | 3 | Hail |
| Aegis | Sandstorm | 3 | ∞ | Aegis |
| Aegis | Pike | 2 | 5 | Aegis |
| Aegis | Saber | 2 | 2 | Saber |
| Aegis | Stiletto | 2 | 3 | Aegis |
| Aegis | Reaver | 2 | 2 | Reaver |
| Aegis | Render | ∞ | ∞ | draw |
| Aegis | Sentinel | 4 | 3 | Sentinel |
| Maul | Lance | 4 | 2 | Lance |
| Maul | Cleaver | 2 | 4 | Maul |
| Maul | Gnat | 3 | 1 | Gnat |
| Maul | Hail | 3 | 4 | Maul |
| Maul | Sandstorm | 3 | 2 | Sandstorm |
| Maul | Pike | 2 | 2 | Pike |
| Maul | Saber | 2 | 4 | Maul |
| Maul | Stiletto | 2 | 1 | Stiletto |
| Maul | Reaver | 2 | 6 | Maul |
| Maul | Render | 4 | 2 | Render |
| Maul | Sentinel | 3 | ∞ | Maul |
| Lance | Cleaver | 4 | 2 | Cleaver |
| Lance | Gnat | 2 | 2 | Gnat |
| Lance | Hail | 2 | 1 | Hail |
| Lance | Sandstorm | 2 | 4 | Lance |
| Lance | Pike | 2 | 4 | Lance |
| Lance | Saber | 4 | 2 | Saber |
| Lance | Stiletto | 1 | 2 | Lance |
| Lance | Reaver | 2 | 2 | Reaver |
| Lance | Render | 2 | 4 | Lance |
| Lance | Sentinel | 5 | 2 | Sentinel |
| Cleaver | Gnat | 3 | 4 | Cleaver |
| Cleaver | Hail | 3 | 2 | Hail |
| Cleaver | Sandstorm | 3 | 1 | Sandstorm |
| Cleaver | Pike | 2 | ∞ | Cleaver |
| Cleaver | Saber | 2 | 2 | Saber |
| Cleaver | Stiletto | 2 | 2 | Stiletto |
| Cleaver | Reaver | 2 | 3 | Cleaver |
| Cleaver | Render | 2 | 1 | Render |
| Cleaver | Sentinel | 3 | 4 | Cleaver |
| Gnat | Hail | 2 | 2 | Gnat |
| Gnat | Sandstorm | 2 | 2 | Gnat |
| Gnat | Pike | 1 | 2 | Gnat |
| Gnat | Saber | 4 | 2 | Saber |
| Gnat | Stiletto | 1 | 2 | Gnat |
| Gnat | Reaver | 1 | 2 | Gnat |
| Gnat | Render | 2 | 2 | Gnat |
| Gnat | Sentinel | 5 | 3 | Sentinel |
| Hail | Sandstorm | 2 | 2 | draw |
| Hail | Pike | 4 | 2 | Pike |
| Hail | Saber | 2 | 2 | Saber |
| Hail | Stiletto | 1 | 2 | Hail |
| Hail | Reaver | 4 | 2 | Reaver |
| Hail | Render | 1 | 2 | Hail |
| Hail | Sentinel | 5 | 3 | Sentinel |
| Sandstorm | Pike | 2 | 2 | Pike |
| Sandstorm | Saber | 1 | 2 | Sandstorm |
| Sandstorm | Stiletto | 1 | 2 | Sandstorm |
| Sandstorm | Reaver | 2 | 2 | draw |
| Sandstorm | Render | 4 | 2 | Render |
| Sandstorm | Sentinel | 4 | 3 | Sentinel |
| Pike | Saber | 4 | 4 | Pike |
| Pike | Stiletto | 1 | 1 | Stiletto |
| Pike | Reaver | 2 | 4 | Pike |
| Pike | Render | 4 | 2 | Render |
| Pike | Sentinel | 6 | ∞ | Pike |
| Saber | Stiletto | 1 | 1 | Stiletto |
| Saber | Reaver | 4 | 2 | Reaver |
| Saber | Render | 2 | 1 | Render |
| Saber | Sentinel | 5 | 2 | Sentinel |
| Stiletto | Reaver | 1 | 1 | Stiletto |
| Stiletto | Render | 2 | 1 | Render |
| Stiletto | Sentinel | 3 | 2 | Sentinel |
| Reaver | Render | 2 | 2 | Reaver |
| Reaver | Sentinel | 4 | ∞ | Reaver |
| Render | Sentinel | 2 | 2 | Render |
