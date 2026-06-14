# Deckbound — Imported Design Principles

> General game-design principles mined from the research library in the sibling
> **seans-arcade** project (`docs/research/`, plus a couple of its game design
> docs). These are borrowed wisdom adapted to Deckbound — each notes its source
> and how it applies here. They *inform* the design; they are not themselves
> rules. For Deckbound's own non-negotiable north stars, see
> [philosophy](philosophy.md).

## Meaningful choice

- **Choices need visible, irrevocable, remembered consequences.** A choice
  matters only when the world changes, the decision can't be undone, and later
  play remembers it. *(design-philosophy.md)* — **Deckbound:** a simultaneous
  reveal is irrevocable, and the cards it exhausts stay spent, shaping every later
  exchange.
- **Dilemmas, not optimal plays.** The best choices sacrifice something real;
  there is no obviously correct option. *(meaningful-choice-analysis.md)* —
  **Deckbound:** the Strike/Block/Evade/Scheme cycle makes every option beat one
  and lose to another, and exhaustion adds "use it now vs stay unpredictable
  later."
- **Trust in choice is cumulative.** Early choices that matter teach players to
  engage; early choices that don't train them to disengage.
  *(meaningful-choice-analysis.md)* — **Deckbound:** the first exchanges must
  visibly reward predictions and punish predictability.

## Progression & difficulty

- **Multiple, exchangeable progression axes — power, skill, knowledge.** Every
  challenge should be solvable through persistence, mastery, *or* curiosity, no
  one axis invalidating the others. *(design-philosophy.md,
  emergent-gameplay-and-progression.md)* — **Deckbound:** acquire new
  cards/aspects (power), predict opponents better (skill), or discover card
  interactions (knowledge).
- **Soft gates over hard gates.** Prefer obstacles that are much harder without
  the intended tool but still possible; reserve hard gates for major milestones.
  *(progression-and-difficulty-design.md)* — **Deckbound:** lacking an entire
  aspect is a hard gate; a known opponent pattern you can out-predict is a soft one.
- **Player-ordered challenges = self-chosen difficulty.** If players pick the
  order, they pick their own curve, so encounters shouldn't auto-scale; optional
  power that helps but isn't required is the elegant middle.
  *(progression-and-difficulty-design.md; Mega Man, Breath of the Wild)* —
  **Deckbound:** deliberately unbalanced, player-chosen scenarios — retreat,
  acquire, return.
- **The world reacts to progression.** Acquisitions visibly change what's
  possible; old areas reveal new options on return. *(design-philosophy.md)* —
  **Deckbound:** a scenario that was certain doom becomes winnable once the right
  aspect is acquired.

## Risk, loss & relief

- **Pain before relief.** A solution's value is proportional to the felt problem;
  time the relief to land after the pain but before resentment.
  *(pain-before-relief.md)* — **Deckbound:** exhaustion is the pain, recovery the
  relief — a loop tuned per scenario (see [zones](zones.md)).
- **Scarcity forces committed choices.** Limited resources make "use now or save?"
  decisions under incomplete information. *(design-topics-to-explore.md)* —
  **Deckbound:** each hand card is a once-per-cycle resource; spending it costs
  future unpredictability.
- **Real, fair, foreshadowed loss validates progress.** Loss must be possible but
  readable in advance. *(design-philosophy.md, "fairness contract")* —
  **Deckbound:** scenarios telegraph threat so "come back stronger" is a valid,
  satisfying call.

## Emergence & systems

- **Few systems with real rules beat many scripted ones.** Consistent rules
  produce emergent interactions you never hand-authored.
  *(design-philosophy.md)* — **Deckbound:** aspects × zones × the four actions ×
  magnitude is a large space from few parts, and the three deciders share one rule
  set. (This is [philosophy §6](philosophy.md#6-many-systems-from-few-rules).)
- **Interconnected systems reward experimentation.** Mechanics interact in ways
  the game doesn't explicitly teach. *(design-philosophy.md,
  emergent-gameplay-and-progression.md)* — **Deckbound:** let aspect combinations
  and exhaustion-prediction interact without spelling them out.

## Discovery & learning

- **Rules themselves can be discovery content.** Players learn how the game works
  through play, and each discovered rule re-frames every future choice.
  *(design-philosophy.md; Tunic)* — **Deckbound:** let the cycle and the magnitude
  math be learned by playing; early opponents use simple patterns so variables can
  be isolated.
- **Foreshadow rewards; let the player make the connection.** Place depth before
  it's needed; never announce it. *(design-philosophy.md)* — **Deckbound:** cards
  hint at interactions through naming and iconography rather than rules text.

## Authored vs procedural

- **Author pacing; procedural can't.** Algorithms handle connectivity; humans
  handle tension/release, teachable moments, and "aha" beats.
  *(procedural-vs-authored-design.md)* — **Deckbound:** hand-author scenario
  sequencing even if individual decks are shuffled; introduce one mechanic at a
  time at low stakes before it becomes consequential.

## Framing & player respect

- **Contrast safety with danger.** Home/rest only means something against danger,
  and vice-versa. *(design-topics-to-explore.md)* — **Deckbound:** safe-haven
  scenarios where cards recover, set between high-threat ones.
- **Loadout as identity through tradeoffs.** No objectively correct build; players
  define themselves by what they give up. *(design-topics-to-explore.md)* —
  **Deckbound:** which aspects you invest in is your identity and your matchup
  chart.
- **Respect the player's time.** Grinding optional, backtracking reveals something
  new, resolution fast. *(design-philosophy.md)* — **Deckbound:** simultaneous
  reveal keeps turns quick; replaying for cards is a choice, not a tax.
- **Prefer diegetic interaction.** Interactions in the world beat menus.
  *(design-philosophy.md)* — **Deckbound:** laying down a physical card is
  inherently diegetic — lean into it.

## Source library & coverage

Mined from `../seans-arcade/docs/research/`: `design-philosophy`,
`meaningful-choice-analysis`, `emergent-gameplay-and-progression`,
`progression-and-difficulty-design`, `pain-before-relief`,
`procedural-vs-authored-design`, `reference-games`, `zelda-case-study`,
`design-topics-to-explore`.

**Reviewed but not pulled in** (off-topic for a hand-played card game):
`classic-game-candidates`, `maze-key-gate-design`, `non-programming-skills`,
`games/9-keys/*`, `games/battle-arena/*` — these are arcade/action/maze or
production-craft specific. Worth revisiting `reference-games` and `9-keys` if we
ever add procedural scenario generation.
