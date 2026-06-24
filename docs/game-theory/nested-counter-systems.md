# Nested Counter Systems

Complex games do not have a single counter system — they have counter systems at multiple levels simultaneously. The hierarchy of concerns applies independently at each level, but the *design goal* at each level is different. Conflating these levels produces both bad game design and bad analysis.

---

## The Three Levels

### Macro Level: Faction or Archetype Identity

At the highest level, a game offers a choice of faction, race, deck archetype, or character class. This is the choice that precedes all gameplay.

**Correct design goal: equality, not a counter cycle.**

A macro-level RPS (Zerg beats Terran beats Protoss beats Zerg) would be a design failure. If faction choice has systematic win-rate implications, then optimal play is "pick the faction that counters the current meta" — a decision made before the game begins. Skill expression is diminished. Faction becomes a strategic calculation rather than a personal identity.

The correct goal is that faction choice is **strategically irrelevant to expected outcome at equal skill**. All faction matchups converge to 50% win rates. The Nash equilibrium at the faction level is uniform: you should be indifferent between factions. Players choose factions based on playstyle preference, not strategic advantage.

Formally: the faction-level tournament should satisfy the first three concerns from the hierarchy (no Condorcet winner, regular, uniform Nash), but the Hamiltonian cycle is neither required nor desirable — there is no intended counter relationship between factions at all.

### Mid Level: Strategic and Build-Order Counter Cycles

Within any given faction matchup, there is a space of strategic choices: which build order to follow, when to attack, whether to expand economy or build army, which tech path to pursue. These choices constitute a mid-level counter system.

**Correct design goal: an RPS cycle within each matchup.**

This is where the RPS structure is intentional and valuable. The standard form is a 3-cycle:
- Aggressive opener counters fast expansion (punishes greediness before defenses are up)
- Fast expansion counters defensive tech (out-economies the defender over time)
- Defensive tech counters aggressive opener (walls off and survives, then wins late)

This cycle is what creates the "meta-game" — the ongoing strategic arms race within a matchup. It is the source of variety, because each game begins with both players making a strategic-level bet that the other player will counter in a particular way. Correct reads are rewarded; incorrect reads are punished.

The key property: this RPS cycle exists *inside* a single matchup (e.g., ZvT), not between factions. A Zerg player faces this cycle when playing against Terran and a different, equally rich cycle when playing against Protoss.

### Micro Level: Unit Counter Cycles

At the finest level, individual units counter each other directly, creating a moment-to-moment tactical puzzle during combat.

**Correct design goal: explicit RPS counters that reward tactical reads.**

Unit counters are the most visible counter system in an RTS and the most obviously RPS-structured. Marines counter Zerglings (range, kiting), Zerglings counter Siege Tanks (swarm before they siege), Siege Tanks counter Marines (splash), and so on. These create a continuous stream of decisions within combat: what to produce, how to position, what to target.

Unlike the macro level (where equality is the goal) or the strategic level (where a 3-cycle is the goal), the unit level can accommodate rich, complex counter webs without violating the hierarchy — because the decisions are fast and reversible. A wrong unit production choice costs one cycle of production; a wrong faction choice costs the game.

---

## Why the Levels Must Not Be Confused

**Treating factions as macro-level RPS** is the most common design mistake in complex games. It feels intuitive: "Zerg swarms should counter Terran's expensive mechanized units" is a compelling narrative. But this narrative, if encoded as a systematic win-rate advantage, makes faction selection a strategic calculation rather than a stylistic one. The result is a tournament meta where everyone plays the faction that counters the current dominant faction — a single macro-level RPS game played before any in-game decision matters.

**Treating unit counters as sufficient for faction balance** is the complementary mistake. A game where every unit has a counter but no faction-level equality still produces dominant factions — because some factions may have more efficient access to the units at the top of various counter chains.

The correct design separates these concerns explicitly:
1. Factions are equal (no faction matchup has a systematic win-rate advantage)
2. Within each matchup, strategic RPS cycles create variety
3. Within each strategic phase, unit RPS cycles create depth

Depth and variety are both maximized, and neither comes at the cost of faction equality.

---

## StarCraft: The Exemplar of Nested Counter Systems

StarCraft (both Brood War and SC2) is the clearest example of this design working correctly.

### Faction Level: Equality

StarCraft's three factions are designed to have approximately 50% win rates across all matchups at high levels of play. This is the explicit patching target: when ZvT deviates from 50%, patches adjust unit stats, costs, or abilities until it returns.

This is **not** a macro-level RPS. Blizzard does not intend for Zerg to beat Terran, Terran to beat Protoss, or Protoss to beat Zerg. The narrative framing ("Zerg swarms pressure Terran's mechanical army") describes thematic flavor and the *source* of strategic tension, not a systematic win-rate advantage.

Historical competitive data from Korean Brood War (the most extensively studied competitive RTS dataset) shows all three matchup win rates hovering around 49–51% across most of the game's competitive life — a near-perfect uniform Nash equilibrium at the faction level.

### Strategic Level: Nested RPS Within Each Matchup

Within ZvT, the strategic space forms a rich counter cycle. One simplified framing:
```
Fast Zerg expansion  counters  Terran turtle-and-push  (out-economies over time)
Terran early push    counters  Fast Zerg expansion     (attacks before Zerg stabilizes)
Zerg defensive build counters  Terran early push       (holds with Sunken Colonies, then swings)
```
A Terran player who correctly reads "Zerg is going for a fast expand" wins by executing an early push. A Zerg player who correctly reads "Terran is pushing early" wins by building defensively. Neither choice is universally optimal — the advantage belongs to whoever correctly anticipated the other's choice. This is RPS working as intended, at the strategic level.

ZvP and TvP have their own, distinct strategic counter cycles. A player who masters ZvT learns a different strategic vocabulary than one who masters TvP — this is what makes playing different matchups feel like learning a new game within the same game.

### Unit Level: Explicit Counter Chains

Within each strategic phase, unit counters drive moment-to-moment play:

**ZvT unit counter examples:**
- Zerglings overwhelm unsieged Siege Tanks; Siege Tanks decimate Zerglings when sieged
- Lurkers destroy clumped Marines; Siege Tanks and Vultures shut down Lurkers
- Mutalisks harass mineral lines; Goliaths and Missile Turrets shut down Mutalisks
- Ultralisks absorb fire and crush massed Goliaths; Irradiate and Firebats handle Ultralisks
- Defiler Dark Swarm neutralizes all ranged attacks including Tanks; Science Vessel Irradiate punishes Defilers

**The depth mechanism:** These counter chains are long enough that no single unit is universally optimal, but short enough that the correct counter is inferable. A Terran player seeing mass Mutalisks knows to build Goliaths. A Zerg player seeing Goliaths knows to switch off Mutalisks. The tactical decisions flow from the unit counter structure.

### Why Three Factions Maximizes This Design

With three factions:
- There are 3 matchups (ZvT, ZvP, TvP), each with its own strategic RPS cycle
- Each matchup can receive dedicated design and balance investment
- Each faction has a coherent unit pool that expresses a consistent strategic identity
- The calibration surface (3 matchups) is small enough to tune with high precision

With four or more factions, the inter-faction calibration budget grows as n(n-1)/2 while the per-faction design budget shrinks as 1/n. Strategic depth per matchup decreases. Unit pools become shallower. The nested counter systems at the micro level become less rich because fewer design resources were available per faction.

Three is not just the minimum odd number — it is the number where the calibration surface and the per-faction depth budget are both optimal for deep, nested counter system design.

---

## Warcraft III: Added Complexity, Added Problems

Warcraft III attempts a similar nested design (faction equality + in-game counter systems) but compounds the structural even-n problem with an additional parallel counter layer: the hero system.

### The Hero Layer

Heroes in Warcraft III are powerful units that level up during the game, providing a mid-game power curve that units alone do not create. Hero selection, sequencing, and level timing constitute a fourth level of counter system that the base framework does not have:

```
Faction level       (4 factions — structurally irregular, even n)
Hero selection      (faction-specific heroes with matchup-dependent strengths)
Strategic level     (build order, creep timing, tech choices)
Unit level          (unit counters, similar to StarCraft)
```

The hero level interacts with all other levels in ways that are difficult to measure:
- A strong hero player can overcome faction-level disadvantages by winning hero-vs-hero trades
- Certain hero combinations create near-dominant compositions regardless of faction
- Creep timing (when to kill neutral camps for hero experience) becomes a parallel optimization problem

The even-n structural problem means faction-level Copeland scores cannot all be zero. The hero layer adds a second imbalance vector that cross-cuts the faction structure. Balance attempts that adjust unit stats may fail because hero interactions dominate the matchup. The design space has too many coupled degrees of freedom.

In practice, Warcraft III patches never fully equalized all six matchup win rates. Human and Orc remained slightly favored over Night Elf and Undead across most of the game's competitive life — the structural floor imposed by even n.

---

## Dawn of War: Soulstorm — Budget Dilution

Soulstorm has 9 factions. Nine is odd, so a perfectly regular faction-level tournament is mathematically achievable. The failure is economic, not structural.

### The Budget Dilution Problem

Every faction added to a game divides the available design and balance budget:

```
calibration_work = n × (n-1) / 2  matchups to tune
design_work      = n × (unit_pool_depth + strategic_options + balance_tuning)

budget_per_matchup = total_budget / calibration_work  → shrinks as n grows
budget_per_faction = total_budget / n                 → also shrinks as n grows
```

For Soulstorm's 9 factions:
- 36 matchups to calibrate, versus StarCraft's 3
- 9 faction identities to develop versus StarCraft's 3

The result is not just weak inter-faction balance. It is shallow intra-faction depth. Each faction has fewer viable units, fewer strategic options, fewer nested counter cycles within matchups — because the design budget was spread across 9 factions rather than concentrated on 3.

**The double failure:** Soulstorm's factions are not just poorly balanced against each other (macro level failure) — they also provide less strategic variety per matchup (mid level shallowness) and thinner unit counter webs (micro level shallowness). Every level of the nested system is degraded simultaneously by the faction count.

**Specific evidence:** Dark Eldar and Sisters of Battle, the two factions added specifically for Soulstorm (by Iron Lore, not Relic's original team), received the least development investment and are considered the weakest factions. Factions carried over from earlier expansions are generally considered stronger — because they had been refined over more development time. Balance is not just a mathematical property; it requires sustained investment proportional to the number of inter-faction relationships.

---

## Design Principle Summary

| Level | Goal | If Done Right | If Done Wrong |
|---|---|---|---|
| Macro (faction) | Equality — uniform Nash, no Condorcet | Faction is personal expression; skill dominates | Faction becomes a strategic calculation; meta collapses |
| Mid (strategy) | RPS cycle within each matchup | Every game feels fresh; reads are rewarded | Dominant build exists; games become rote |
| Micro (unit) | Explicit counter chains | Tactical decisions are meaningful | Either trivially clear or chaotic; micro skill doesn't matter |

The key principle: **the RPS structure belongs at the strategic and unit levels, not the faction level.** Factions should be different, not countering. The depth comes from what happens inside the game, not from which faction you chose before it started.

---

## Implications for the Hierarchy of Concerns

The hierarchy of concerns document describes properties for a single-level counter system. When applying those properties to complex, multi-level games, the first question is always: *which level are we analyzing?*

- Apply the full hierarchy (Condorcet, regularity, Nash, connectivity, Hamiltonian, cognitive load) to unit counters and strategic cycles — the RPS properties should hold at these levels
- Apply only the first three concerns (no Condorcet, regularity, uniform Nash) to the faction level — the goal is equality, not a cycle
- Measure each level independently, because a game can have perfect faction-level balance alongside broken unit counters, or vice versa
