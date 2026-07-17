# Combat rules -- current state (snapshot)

Concise, precise reference for where the combat model stands **now**. Implemented in
`crates/rules/src/combat/` (pure, `Game`-seam + resolver). ASCII only. `x` = multiply.

Verified: **4/4 solos + 5/5 party fights** (`regions_diagonal`); 52 rules tests.

---

## 1. Shape

- **Two formations**, one per side (party vs foes). No setup; a fight opens on round 1.
- **Rank** (one enum, three values) is a body's place:
  - **Vanguard** -- the melee front. Fixed by weapon (a melee body).
  - **Rearguard** -- the ranged back. Fixed by weapon (a ranged body).
  - **Outrider** -- the only rank you *earn*, by crossing into the enemy line. Loose among their ranks.
- **Screening is automatic**: a Rearguard is *screened* exactly while its side has a **living Vanguard**.
  (No positioning choice -- protecting your back is a matter of having a front.)

## 2. Stats, pools, flows

- Five stats, one chassis: **`[Might, Vitality, Grit, Cadence, Finesse]`** (`M V G C F`).
- **Two pools** (`count x strength`, both refill each round):
  - **Health** = Vitality cards, each **Grit** strong.
  - **Tempo** = Cadence cards, each **Finesse** strong.
- **Two flows** (`strength x count`, what you produce by spending):
  - **damage** = `Might x strikes`, vs Grit (a Health card flips per Grit).
  - **reach** = `Finesse x tempo cards`, vs the opponent's reach (higher wins the contest).
- Everything is a **product, never a quotient**. `Might` and `Grit` are different stats (offence vs
  defence); on the Tempo side there is only `Finesse` -- so the reach contest is symmetric.

## 3. Acts and engagement (geometry, not menus)

Each round every body -- heroes and foes (a foe's is one scripted instinct) -- declares one **Act**:
**Clash**, **Raid**, **Melee**, **Slip**, or **Hold**. You pick a *target*; where it stands decides which:

| Target is...                                            | Engagement                                        | Who            |
| ------------------------------------------------------- | ------------------------------------------------- | -------------- |
| enemy **Vanguard**, across the gap                      | **Clash**                                         | any weapon     |
| **screened Rearguard** (its side has a living vanguard) | **Raid** (cross in; the front intercepts)         | **melee** only |
| **exposed Rearguard** (its vanguard has fallen)         | **Clash** by anyone, **and** raidable by melee    | --             |
| enemy in your **own** line (an Outrider)                | **Melee** (no screen between intermingled bodies) | any weapon     |

- **AoE = width, never extra reach**: a Clash sweeps the whole front line, a Raid the whole back line,
  a Melee the whole region (past the screen), a Volley the whole crossing. A body you could not
  single-target, you cannot sweep.

## 4. The reach / dodge contest (the "inner three")

1. **Target** -- declare who you reach for.
2. **Answer** -- the target Evades / Pushes / Aborts (for a crossing), or Stands / Slips (an exchange).
3. **Strike** -- the opening blow the reach bought, plus **one extra strike per remaining tempo** poured.

- **reach (a bid)** = `tempo x Finesse` (`x body count` for a **horde** attacker).
- **dodge** = `tempo x Finesse` (a horde gets **no** body multiplier here -- great catcher, poor evader).
- To get through / evade, your value must **strictly beat** the other's; the **reacher / screener wins
  ties**. (The sim auto-spends the minimum tempo to win; the rule is value > value.)

## 5. The round: three rings, nearest-first

Resolve by **how fast each blow lands** ("whatever connects first, goes first"). **Death check after
every strike** silences the dead downstream. **The damage pile closes at every sub-phase boundary** (a
blow must cross Grit *there*; unfinished damage is discarded).

0. **Reset** -- tempo stands back up to **Cadence** (hordes included); the pile closes.
1. **Declaration** -- every body declares one Act.
2. **Inner Ring** (Outriders, distance 0) -- one **simultaneous** strike, melee and ranged together, no
   screen (a melee sweep here catches the whole region). Death check. Then **dissolution** (see 7).
3. **Crossing Ring** (declared Raid/Slip): **Intercept** (front reaches; you answer) -> **Volley** (back
   fires; an area weapon volleys as a sweep) -> **Land** (survivors arrive as Outriders) -> **Raid
   strike** (a landed raider hits its target before it fires). Death check at each.
4. **Outer Ring** (across the gap): **Fire** (Rearguards, ranged first) -> **Clash** (Vanguards). Death
   check at each.

Undecided in **5 rounds = Draw = loss**.

## 6. Damage

- **Normal body**: each blow banks `max(0, Might - armor)` into the pile; a **Health card flips each
  time the pile reaches Grit**. (Armor = 0 for the current roster.)
- **Horde defence -- each body is its OWN Grit-strong pool, NO SPILL**:
  - **Aimed** blow fells **at most one body**, and only if it **penetrates** (`Might - armor >= Grit`).
    Overkill and sub-Grit both waste -- to fell another body you spend another blow (**tempo per body**).
  - **Sweep** hits every body at once: clears the **whole pack** for one card, iff it penetrates.
  - So **width, not power, is the cheap answer to a swarm**. Grit is a pure **penetration gate**.
- **Horde offence**: one **volley** = `body count x Might` (armor per body, no pour). Its size is
  damage + reach, not tempo (tempo still refreshes to Cadence).

## 7. Movement and the Outrider

- **Slip**: only a **Vanguard** crosses, into the enemy line, becoming an **Outrider**. **One-way -- no
  retreat.** (A Raid is a melee crossing that also strikes a rearguard; same gauntlet.)
- Answers to a crossing: **Evade** (pay the slip, untouched), **Push** (pay nothing, eat the blows),
  **Abort** (turn and fight, give up the ground).
- **Dissolution** (at the Inner Ring boundary): an Outrider whose host formation is wiped is "an
  outrider of nothing" -- it reverts to its weapon rank and **rejoins its own line**, or becomes the
  formation where it stands if it is the last of its side. (Replaced the old zone "promotion".)

## 8. Roster (stats `[M V G C F]`)

**Kits** (party of 4): Raider `[6 6 1 2 2]` Jab (melee single); Marksman `[5 2 1 2 2]` Shot (ranged
single); Bastion `[1 3 3 1 2]` Sweep (melee area, tanky); Bombardier `[3 3 1 1 2]` Salvo (ranged area).

**Creatures**: The Wall `[1 4 6 1 2]` (melee single, high Grit); The Duelist `[6 5 1 2 2]` (melee single,
front-fixed); The Swarm `[3 4 1 2 2]` (melee **horde**, front); The Brood `[1 7 1 1 1]` (ranged
**horde**, back); The Sniper `[5 1 1 2 3]` (ranged single, corner-only priority threat).

- Each creature is soloable by **exactly one** kit as a consequence of the numbers (Wall->Raider,
  Duelist->Marksman, Swarm->Bombardier, Brood->Bastion). Sniper has no solo.

## 9. Encounters and scoring

- **9 encounters**: 4 **solos** (one kit each) + 4 **strategy corners** (each teaches one orthogonal
  lesson: Concentration / Range / Sweep / Raid) + 1 **CombinedArms capstone** (needs all at once).
  Exact compositions in `crates/deckbound-content/src/catalog.rs`.
- **Best-route score** `Nd/Mr/Khp` = the cheapest winning line's cost to YOUR party; all **lower is
  better**, ranked lexicographically: **win -> fewest heroes downed (`d`) -> fewest rounds (`r`) ->
  least Health lost (`hp`)**. `<=` = provisional; `no win` = doomed.

## How to run

- Play one encounter (clickable): `cargo run --release -p boardgame --example fight -- N`
  (mirrors `fight-screen.txt` + `fight-log.txt`).
- Balance ladder (4/4 + 5/5): `cargo run --release -p deckbound-board --example regions_diagonal`
  (add `scores` for best routes; slow).
- Text decision-tree: `--example explore`. Tests: `cargo test -p rules`.

Full narrative spec: `needs-merge/regions-engagement-by-geometry.md` (longer; its horde section predates
the no-spill rework in section 6 above -- trust this snapshot for hordes).
