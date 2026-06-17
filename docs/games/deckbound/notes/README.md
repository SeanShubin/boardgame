# Deckbound — Design Notes

> **Status:** design in progress; no code yet, working title. The systems below are
> settled in *shape* — what remains is numbers, content, and the Spirit aspect.
> Nothing is final.

## Pitch

A **cooperative**, simulation-style fantasy game made **entirely of cards**,
playable **solo or with others** (solo is the design focus; multiplayer is built
in from the start). A character is a collection of decks that represent everything
they can do; the world, the scenarios, the enemies, and the escalating events are
decks too. You explore a generic fantasy world (where anything can be explained by
magic), survive its conflicts, and grow by acquiring new cards — and sometimes
entirely new kinds of decks — racing an event deck that keeps raising the stakes.

## Scope & shape

- **Cooperative, players versus the world** — no player-versus-player in the
  default game.
- **Solo-focused, multiplayer-native** — the solo game is the practical focus, but
  the cooperative structure is core, not an add-on. See
  [turn structure](turn-structure.md).
- **Simultaneous, order-light turns** — to keep length flat as players are added
  and to avoid downtime.
- **30 minutes to 2 hours** per session, at any player count.
- **Tension from an [event deck](world-and-progression.md#the-event-deck)** that
  periodically emits threats, mechanics, and victory/loss conditions.
- **Start small** — build a few systems first and grow from there.

## Design pillars

The canonical north stars live in **[philosophy](../canon/1-charter.md)** — read that
first; it is the charter the rest of these notes must serve. In brief:

1. **Represent and reward human intellect** — minds reason; only nature uses
   decks.
2. **Computable tactics, uncomputable strategy** — the exchange is solvable;
   the campaign is risk/reward, push-your-luck, and opportunity cost.
3. **One hidden choice, simultaneous reveal** — a game of prediction and bluffs.
4. **A character is their decks**, which are **never shuffled** — order is intent,
   and an action is a *chord* of one card per aspect.
5. **Asymmetry by design, balance by scenario** — characters are deliberately
   uneven; fairness comes from environment, objective, and team.
6. **From doom to mastery** — limited reach early, real loss, growth that opens
   the world.
7. **Many systems from few rules.**
8. **Playable by hand, cards only** — no computer required to run the game.
9. **Predictability is a managed resource** — it erodes as cards exhaust; see
   [zones](zones.md).

## Core vocabulary

- **Deck** — an ordered, never-shuffled pile of cards owned by a character. A
  character can have many, of different **deck types**.
- **Aspect** — a way of acting (**Body**, **Mind**, **Spirit**, …; a
  starting set). One action draws one card per aspect, and **Mind** is the tactical
  stance. Capability cards for each aspect live in the **Form** zone. The **outer**
  aspect (Body) is a **pool** (Health cards + the Vitality card); **inner** aspects
  (Mind, Spirit) are **thresholds/capacities** with no Health stack.
- **"Speed swings, Mind reads, toughness endures."** The named split of how an Actor
  removes and survives blows: **Speed** = how many you strike + Runners you drag +
  first-strike; **Mind** = how many you negate by **predicting** (a focus pool, each
  prediction costs the attacker's Speed); **toughness** = how many you absorb without
  predicting.
- **Mind** — prediction-bandwidth **capacity**: a **focus pool** sized to Mind that
  negates blows by anticipating a foe's stance, plus the read/stance job (the tactical
  RPS). It is an **inner** aspect — a capacity, not a depleting Health pool;
  **Confusion** shrinks the focus pool.
- **Resolve** — the **threshold** the Spirit aspect defends: a **standing value Fear
  must exceed** to break the will (Fear > Resolve → panic). It is **not** a card pool
  that depletes — an inner capacity, not an outer Health stack.
- **Ward** — a **passive, typed inner cut** (vs-fear, vs-confusion), the inner twin
  of Armor: applied per-source, never depleting, before the Resolve / Mind-capacity
  bar. It is **not** anti-magic — it guards only fear (Spirit) and confusion (Mind).
- **Numberless card** — represents a quality or effect itself (e.g. *speed*,
  *power*, *precision*, *1 damage*).
- **Modifier card** — **attaches** to another card and changes its value (*+1*,
  *×2*); **attachment order matters**.
- **Passive attribute card** — a card representing a standing trait rather than
  an action (e.g. an *armor* card that changes how physical damage applies).
- **Conflict** — a contest against other characters and/or the environment.
- **Zone** — where a card currently is: **Form** (capabilities + health),
  **Potential** (playable), or **Active** (in play). A card on the table has a
  **facing**: face up = currently in effect; face down = not in effect (used or
  sealed). Form is vitality ([form-and-defeat](form-and-defeat.md)); the rest are
  the tactical layer, where cards **exhaust** and predictability erodes
  ([zones](zones.md)).
- **Lasting / Fleeting** — whether a played card's effect persists (stays Active)
  or happens once (then is turned face down).
- **Stance** — the Mind's rock-paper-scissors choice (Strike / Block / Evade / Scheme);
  winning stances bank **momentum** (see [the Mind](mind-and-stances.md)).
- **Region** — a grouping of locations; players in the same region coordinate
  their turns (see [turn structure](turn-structure.md#regions)).
- **Event deck** — the tension engine that emits threats and victory/loss
  conditions (see
  [world-and-progression](world-and-progression.md#the-event-deck)).
- **Capability / damage / defeat** — your **outer** Form cards (Body) are
  **Health-card pools**; **typed** damage turns them face down in Form. **Inner**
  aspects (Mind, Spirit) are **thresholds**, not pools. An Actor is removed when *any*
  channel breaks — and when your **Body** (keystone) fails you are knocked out →
  retreat (see [form-and-defeat](form-and-defeat.md)).
- **Scenario deck / enemy deck / world deck** — the non-player decks that run
  the game (see [world-and-progression](world-and-progression.md)).

## The design notes

**Foundations**

- [Philosophy](../canon/1-charter.md) — the north stars; read this first.
- [Constraints](constraints.md) — no computer required, cards only, every agent
  bound by the same rules.
- [Entities](entities.md) — the noun taxonomy: **Actor** (the umbrella), and the two
  kinds — **Character** (agency; improvises) and **Creature** (rule-driven; scripted).

**The character**

- [Stats overview](stats.md) — the full stat lineup on one page: the three aspects,
  the cut→bar→pool defense model, the offense/timing stats, and the Tempo/Focus budgets.
- [Decks & aspects](decks-and-aspects.md) — a character as a set of never-shuffled
  decks; aspects, the action *chord*, and card kinds.
- [Zones](zones.md) — the tactical choice cycle (Potential → Active → face down),
  Lasting vs Fleeting, exhaustion as predictability.
- [Form & defeat](form-and-defeat.md) — the Form zone as capabilities + health +
  defenses; typed damage, toughness, knockout.
- [Cards & customization](cards-and-customization.md) — the customization matrix:
  quality axes by aspect, weapons & armor (and their damage-type RPS), magic.

**A conflict**

- [Decision-making](decision-making.md) — the hidden-information core; how human,
  computer stand-in, and environment agents differ.
- [The Mind — stances & momentum](mind-and-stances.md) — the RPS cycle, the uncapped
  momentum it banks, and the misprediction that forfeits it.
- [The duel](the-duel.md) — the stance-game **Marshal / Unleash / Overwhelm /
  Parry** (the strike/throw/block mix-up), the public **Edge** bank, and the
  spend that scales a card's primary effect. Supersedes the RPS stance cycle +
  momentum above.
- [Combos & interactions](combos.md) — the design target: meaningful play as
  **combinations** of effects (aspect chords, multi-effect cards, stance-outcome chains).
- [Combat resolution](combat.md) — the magnitude layer: first strike by tempo,
  pre-emption (drop them / out-predict them), and how a stance becomes damage.
- [Speed & tempo](speed-and-tempo.md) — Speed as a per-round tempo pool: spend it to
  evade/engage/strike, first-strike by tempo, overextending, and the Mind/Speed/Power
  split.
- [The Spirit aspect](spirit.md) — will & morale: Resolve, Rally, Dread, fear, and
  dispositional traits.
- [Coordination & interruption](coordination-and-interruption.md) — the cardless
  positioning layer: front/back lines, Attack/Hold, running as a **gauntlet** whose
  drag (Speed) stops Runners, and prediction-bandwidth as **Mind**.

**Play & world**

- [Turn structure](turn-structure.md) — simultaneous, order-light turns; regions;
  how solo and co-op share one loop.
- [World & progression](world-and-progression.md) — the world, scenario, enemy, and
  event decks; exploration and the doom-to-mastery arc.

**Prototype**

- [Sample combat (4 vs 5)](sample-round.md) — the current card-level play-by-play:
  all three aspects, the gauntlet, fear vs resolve, momentum and combos, tracked
  through the zones. **Start here.**
- [Interactive tutorial](../presentation/tutorial.html) — the sample combat as a narrated,
  Back/Next walkthrough (open in a browser; no build); the board illustrates each beat.
- [Physical representation](physical-representation.md) — the sample combat rendered as
  actual cards and tokens: every card face, its starting zone, and when it moves — a
  pressure test of the cards-only pillar.
- [Skirmish prototype (6 vs 9)](skirmish-prototype.md) — an earlier, larger draft
  (its coverage rule and numbers predate the gauntlet); kept for the bigger roster
  and the one-threat-per-mechanic table.

**Building it**

- [Engine architecture](engine-architecture.md) — the rulebook / appendix / components
  tiers, the keyword model, and one engine (WASM + native) behind three projections
  (HTML, Bevy, and a generated tabletop kit).
- [Resolution procedure](resolution.md) — the deterministic round the engine runs.
- [Keyword vocabulary](keywords.md) — the rulebook glossary: each keyword's engine
  intent + one-line manual text.
- [Tabletop rulebook](rulebook.md) — the human-facing manual, appendix, and card listing
  for the sample combat; the comprehensibility-pressure artifact (hand-written now,
  generated later).

**Reference**

- [Archetypes](archetypes.md) — character archetypes (solo generalist vs co-op
  specialists) and a monster roster where every mechanic is someone's lifeline.
- [Design principles](design-principles.md) — general game-design wisdom imported
  from the seans-arcade research library, adapted to Deckbound.

## Open questions

- The **numbers** — stat scaling, the damage formula, and the bank → effect
  conversions (deferred until there is something to playtest).
- What never-shuffled deck order *means* to play around beyond each card's fixed
  starting zone — foresight, sequencing, manipulation.
- How **locations and connectivity** are represented as cards so a growing map stays
  practical in physical form
  ([world & progression](world-and-progression.md#world-deck)).
- **Death** beyond knockout → retreat (permadeath? attrition?), and what **persists**
  between scenarios.
- How a card — or an entirely new **aspect** — is **acquired**, and the full aspect
  list beyond Body / Mind / Spirit.
- **Spirit's** cards and effects — its identity is set (the will to act; see
  [decks & aspects](decks-and-aspects.md#the-aspects)), the mechanics still to
  build.
