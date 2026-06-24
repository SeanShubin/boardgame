# Counter Systems Done Right

Each example is analyzed against the hierarchy of concerns: Condorcet winner, regularity, Nash equilibrium, strong connectivity, Hamiltonian cycle, and cognitive load.

---

## Rock Paper Scissors

**The platonic ideal of a counter system.**

**Structure:**
```
Rock     beats  Scissors
Scissors beats  Paper
Paper    beats  Rock
```

**Win matrix:**
```
        Rock  Paper  Scissors
Rock  [  -     0       1   ]
Paper [  1     -       0   ]
Sciss [  0     1       -   ]
```

**Analysis:**
- n = 3, out-degree = 1 for all elements → perfectly regular
- Copeland scores: Rock=0, Paper=0, Scissors=0 → no Condorcet winner
- Nash equilibrium: (1/3, 1/3, 1/3) — provably optimal to play each with equal probability
- SCC count: 1 (the graph is one 3-cycle)
- Hamiltonian cycle: Rock→Paper→Scissors→Rock (the game *is* the cycle)
- Cognitive load: 3 matchups, reducible to 1 sentence

**Why it achieves perfection:** The minimum number of elements (3) that can form a tournament with no Condorcet winner. With 2 elements you get either a draw or one dominates. With 3, the intransitivity (A > B > C > A) is the simplest non-trivial case. The game exhausts all its structural properties in one cycle.

**The key insight:** RPS is not just balanced — it is *maximally simple while remaining balanced*. Every additional element adds complexity. RPS finds the floor.

---

## Rock Paper Scissors Lizard Spock (Sam Kass & Karen Bryla, 1998)

**The correct 5-element expansion.**

**Structure:**
```
Scissors decapitates Lizard     Lizard poisons Spock
Scissors cuts Paper             Spock smashes Scissors
Paper covers Rock               Scissors cuts Paper (already listed)
Rock crushes Lizard             Lizard eats Paper
Lizard poisons Spock            Spock vaporizes Rock
Rock crushes Scissors           Rock crushes Scissors (already listed)
Spock smashes Scissors          Paper disproves Spock
Paper disproves Spock           Spock vaporizes Rock
```

More clearly, each element beats exactly 2 others:
```
Scissors beats Paper, Lizard
Rock     beats Scissors, Lizard
Paper    beats Rock, Spock
Lizard   beats Spock, Paper
Spock    beats Scissors, Rock
```

**Win matrix:**
```
          Sciss  Rock  Paper  Lizard  Spock
Scissors [  -     0      1      1      0  ]
Rock     [  1     -      0      1      0  ]
Paper    [  0     1      -      0      1  ]
Lizard   [  0     0      1      -      1  ]
Spock    [  1     1      0      0      -  ]
```

**Analysis:**
- n = 5, out-degree = 2 for all elements → perfectly regular
- Copeland scores: all 0 → no Condorcet winner
- Nash equilibrium: (1/5, 1/5, 1/5, 1/5, 1/5) exactly
- SCC count: 1
- Hamiltonian cycle: Scissors → Rock → Spock → Paper → Lizard → Scissors (verify: Scissors beats Rock? No. Let me use the correct one.)

Correct Hamiltonian cycle: Scissors → Lizard → Rock → Spock → Paper → Scissors
- Scissors beats Lizard ✓
- Lizard beats Rock? No, Rock beats Lizard.

The Hamiltonian cycle is: Rock → Scissors → Paper → Lizard → Spock → Rock
- Rock beats Scissors ✓
- Scissors beats Paper ✓
- Paper beats Lizard ✓
- Lizard beats Spock ✓
- Spock beats Rock ✓

This is a valid Hamiltonian cycle. Multiple exist.

- Cognitive load: 10 matchups, but reducible to the Hamiltonian cycle (5 facts) plus the rule "each element also beats the element two positions ahead of it in the cycle."

**Why Kass solved this correctly:** He did not simply add two elements at random. Each new element (Lizard and Spock) was assigned exactly 2 wins from the existing 3 elements and 2 losses, maintaining regularity. The fact that Lizard and Spock also beat/lose to each other closes all the required matchups at out-degree 2. This is the unique structure (up to isomorphism) of a 5-regular tournament.

---

## Fire Emblem Weapon Triangle (Sword / Lance / Axe)

**RPS translated into a tactical RPG, done cleanly.**

**Structure:** Introduced in Fire Emblem: Genealogy of the Holy War (1996), standardized through the series:
```
Sword beats Axe
Axe beats Lance
Lance beats Sword
```

**Implementation detail:** In most games, the triangle grants a ±1 or ±5 hit/avoid/damage bonus rather than a guaranteed win. This makes it a *soft* counter system (probability-weighted advantage) rather than a deterministic one — but the structure is identical to RPS.

**Analysis:**
- n = 3, out-degree = 1 for all → perfectly regular
- Soft counters preserve the Nash equilibrium property: if the bonus is symmetric (winner gets +x, loser gets -x), the equilibrium is still uniform
- Hamiltonian cycle: Sword→Lance→Axe→Sword (or reverse)
- Cognitive load: 3 facts, zero cognitive overhead

**The Fates extension (2015):** Fire Emblem Fates added a second triangle (Bow/Shuriken/Tome) operating in parallel with the first. This is an interesting case:
- Each triangle independently satisfies all properties
- The two triangles do not interact with each other (no cross-triangle matchups)
- Result: the weapon system is two independent 3-element counter systems, not one 6-element system
- This is the correct way to expand when you cannot achieve regularity in a combined system — partition rather than merge

**Why it works:** The bonus/penalty is symmetric. The game does not grant immunity (0-damage) — it grants a proportional advantage. This preserves tournament structure.

---

## StarCraft Faction Balance (Equality Model, Empirical)

**Balance achieved not through a faction counter cycle, but through faction equality with counter cycles nested inside each matchup.**

**The design goal:** Blizzard's explicit patching target is that all three faction matchups converge to 50% win rates. ZvT, ZvP, and TvP should each be equally winnable regardless of which side you play. This is **not** a macro-level RPS (Zerg beats Terran beats Protoss) — it is faction equality. The narrative framing ("Zerg swarms pressure Terran's mechanical army") describes thematic flavor and the source of tension within a matchup, not a systematic win-rate advantage between factions.

A macro-level RPS between factions would be a design failure: it would make faction selection a strategic calculation ("play the faction that counters the current meta") rather than a stylistic preference, diminishing skill expression before the game begins.

**Measurement approach:**
```
win_rate[i][j] = historical win rate of race i vs. race j 
                 across professional matches (large sample)

Empirical Copeland score for race i:
    = (win_rate[i][j] > 0.5 for how many j?) - (win_rate[i][j] < 0.5 for how many j?)
```

Historical data from Korean Brood War shows all three matchup win rates clustering around 49–51% at the highest levels of play. All Copeland scores hover near 0. This is a near-perfect uniform Nash equilibrium at the faction level — and it was achieved through sustained patching against that specific numerical target.

**Where the RPS structure actually lives:** The counter cycles in StarCraft are nested inside each matchup, not between factions. Within ZvT, a strategic-level 3-cycle governs build-order decisions:
```
Aggressive opener  counters  Fast expansion    (attacks before defenses are up)
Fast expansion     counters  Defensive tech    (out-economies over time)
Defensive tech     counters  Aggressive opener (holds the attack, wins the late game)
```
Within each strategic phase, a unit-level counter web creates moment-to-moment tactical decisions: Siege Tanks decimate Zerglings, Zerglings overwhelm unsieged Tanks; Lurkers destroy clumped Marines, Tanks and Vultures shut down Lurkers; Mutalisks harass mineral lines, Goliaths and Turrets shut down Mutalisks. These counter chains are what generate strategic depth — not any relationship between factions.

ZvP and TvP each have their own distinct strategic counter cycles and unit counter webs. This is why mastering ZvT and mastering ZvP feel like learning different games despite using the same faction: the nested counter systems differ per matchup.

**What makes this "done right":**
1. Faction-level equality (Nash equilibrium = uniform) means faction choice is personal, not strategic
2. Strategic-level RPS cycles within each matchup create variety — the outcome depends on reads and adaptation, not just execution
3. Unit-level counter webs create depth — every combat decision has a correct answer that rewards game knowledge
4. Three factions means 3 matchups, a calibration surface small enough to tune precisely; the design budget concentrates into deep, rich per-matchup counter systems rather than spreading thin across many factions

**Contrast with Warcraft III:** Warcraft III has 4 factions. Four cannot form a regular tournament — Copeland scores cannot all be zero. The designers were targeting an unreachable state. Night Elf and Undead historically had worse matchup spreads regardless of patch attempts, because the structural constraint is mathematical, not tunable.

---

## Magic: The Gathering Archetype Triangle (Aggro / Control / Combo)

**A counter cycle at the deck-archetype level, emergent from thousands of individual card interactions.**

**The structure:**
```
Aggro beats Control   — aggro kills before control stabilizes
Control beats Combo   — control disrupts combo pieces before assembly
Combo beats Aggro     — combo wins faster than aggro can close
```

**Why this is notable:** No Magic card was designed with "this card counters Control" written on it. The triangle emerges from how entire deck philosophies interact. This is game design encoding a counter system at a macro level, not a micro level.

**Measurement:** Wizards of the Coast uses metagame share and win-rate data across the player base:
```
format_health_score = diversity of top-8 deck archetypes at major tournaments
```
Healthy formats show all three archetypes represented roughly equally at top levels. A healthy format is Nash equilibrium in action — rational players are indifferent between archetypes at optimized skill.

**What makes this "done right":**
1. The 3-cycle structure allows any format imbalance to be fixed by printing cards that strengthen the weakest archetype (e.g., printing faster combo pieces when aggro is too dominant, printing more disruption when combo is too strong)
2. The triangle gives designers a model: "if aggro is overrepresented, we need better Control-vs-Aggro tools"
3. Metagame data functions as continuous Nash equilibrium measurement

**Caveat:** The triangle is approximate. Some decks (midrange, prison) sit between archetypes and have matchup spreads that don't fit the cycle. The Magic metagame is not a formal tournament graph — it is a much messier space that the triangle helps designers navigate. Its value is as a design heuristic, not a mathematical guarantee.

---

## Common Properties of the Done-Right Examples

| Property | RPS | RPSLS | Fire Emblem | StarCraft | Magic |
|---|---|---|---|---|---|
| No Condorcet Winner | ✓ | ✓ | ✓ | ✓ (empirical) | ✓ (approximate) |
| Regular | ✓ | ✓ | ✓ | ✓ (empirical) | ✓ (approximate) |
| Uniform Nash | ✓ exact | ✓ exact | ✓ (symmetric bonus) | ✓ (empirical) | ✓ (approximate) |
| Odd n | ✓ (3) | ✓ (5) | ✓ (3) | ✓ (3) | ✓ (3) |
| Hamiltonian Cycle | ✓ | ✓ | ✓ | N/A — goal is equality, not a cycle | at archetype level only |
| Nested counter systems | n/a | n/a | partial | ✓ (strategic + unit levels) | ✓ (archetype + deck levels) |

The odd-n requirement shows up in every clean example. The two empirical examples (StarCraft, Magic) both use 3 archetypes — designers converging on the same mathematical constraint independently. Notably, StarCraft and Magic do not aim for a Hamiltonian cycle at the top level; their RPS structure lives at nested levels beneath faction/archetype identity, where equality (not a counter cycle) is the correct goal. See `nested-counter-systems.md` for the full analysis of this distinction.
