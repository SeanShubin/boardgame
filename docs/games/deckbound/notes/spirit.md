# Deckbound — The Spirit Aspect: Will & Morale

Spirit is the **will to act** — the inner aspect that lands no physical blow but
reaches the *capacity to act* (see [the aspects](decks-and-aspects.md#the-aspects)).
It works **only if you let it**, through your own response: face it with **resolve**
and it washes over you; fall short and you undo yourself. This note is its card list,
now that the stat mechanics underneath are settled.

## Resolve — the threshold

Resolve is **not a pool of Health cards** — it is a **threshold**: a single standing
value (e.g. "Resolve 5"), set by your Form and **modified by effects**, that fear must
**exceed** to break you. Spirit is an [inner aspect](decks-and-aspects.md#the-aspects),
and **inner aspects are thresholds, outer aspects are pools** (see
[form & defeat](form-and-defeat.md#how-damage-resolves--the-vitality-card-and-health-cards)):
there is **no Resolve stack to chip away** — Resolve itself **never depletes**.

What accumulates is **Fear**, the inner attack. Fear **piles up within a round, not
between** — a steady nerve recovers each round (accumulated fear **clears at round's
end**), a sustained terror wears you down inside one. Composure and resolve are the
**only** shield against the inner aspects; armor does nothing.

## Ward (vs-fear) — the inner cut

Before Fear ever reaches the Resolve bar, it is shaved by **Ward (vs-fear)** — the inner
**cut**. Ward is a **passive, typed, per-source, never-depleting** value (a warding charm
or discipline): a **number on a card, not a meter or track**. It shaves **each** fright
**per source** before that fright joins the round's pile, exactly like Armor / Toughness
shaves each physical hit — so it is **strong against many small frights and weak against
one big one**. Ward is **distinct from innate nerve**: Resolve is your baseline grit (set
by your **Form** / **Resolute**), while Ward is a separate guard layered on top. Ward is
**not anti-magic** (Magic is a Body delivery stopped by Armor vs-heat); Ward guards **only
fear** here (and confusion, in [Mind](mind-and-stances.md)).

**The Fear pipeline.** A fright resolves in order: **raw Fear → −Ward (per source, by
type) → add to the round's pile → if the pile exceeds Resolve, the channel breaks this
round.** The cut applies per incoming source; the pile is the accumulated total tested
against the bar.

When the round's accumulated fear **exceeds** your current Resolve, it breaks through to
your own panic: you **freeze, flee, or — in the extreme — are scared to death**. Breaks
are **this-round events that clear at round's end**, except **scared to death**, which
**bleeds permanently into the Body pool** (your body's response turning *Body* cards face
down). Effects move the *threshold*, not a stack: **Rally / Steel raise** your Resolve,
**debuffs / Coward lower** it, and a **Resolute** character is **immune** — fear simply
never crosses. A **Coward** crumbles.

## The card list (first-pass)

**Actions** (played from Potential):

- **Dread** — a Spirit attack: **Fear X** against a target's Resolve. It **bypasses
  armor entirely** (resolve, not armor, is the only shield), so it threatens even the
  most heavily armored foe. *Aggressive → exhausts.*
- **Terror** — an **area** Fear (a banshee's wail): lower X, many targets — the line-
  breaker that routs a whole front by morale.
- **Steel** — clear the round's accumulated fear (and/or raise your Resolve threshold):
  fear recedes, your nerve holds. *Defensive → self-returns.*

**Collective** (live in the [party zone](zones.md#zones-at-every-scope)):

- **Rally** — raise allies' Resolve, and **every Rally in the zone boosts every other**:
  morale compounds with the number of people standing firm together. *Lasting.*
- **Inspire** — a one-shot surge: a big Resolve lift to all, or a cleanse of fear.

**Dispositional traits** (passive, in Form):

- **Resolute / Conviction** — high baseline Resolve.
- **Fanatic** — **immune to fear** (no check breaks it) — but **cannot retreat** and is
  compelled to press. Courage with no brakes.
- **Coward** — low Resolve; **flees** when pressed (a creature's conditional behavior; a
  hero's liability).
- **Bloodlust** — **must Attack** (cannot Hold or defend): all offense, no guard.

## Offense, defense, and the Warden

- **Offense:** Dread / Terror break enemy will — rout a line, shatter morale, set up a
  "scared to death."
- **Defense:** Rally / Inspire / Steel raise Resolve so inner attacks wash over you;
  Resolute / Fanatic bake it in.
- The **Warden** archetype is the home of all this — the will-specialist who rallies the
  party and breaks the enemy's nerve.

## Open questions (numbers)

- **Rally's compounding** — flat +1 Resolve per ally rallying, or a multiplier? Scaling
  is [uncapped in principle](world-and-progression.md#power-scaling-and-the-balance-budget),
  but unbounded *compounding* could explode — does Rally need its own curve?
- **Fear → effect** — how much overcoming Resolve makes you freeze vs flee vs the lethal
  "scared to death," and the thresholds for each.
- *(Deferred)* the **incorporeal** trait and what bites it (Spirit only, or all inner
  aspects) are parked with the [special-card library](form-and-defeat.md#bespoke-traits-are-a-feature)
  until the core is solid.
- **Fanatic / Bloodlust** — the exact "can't retreat" / "must attack" rules, and what
  they buy in exchange.
