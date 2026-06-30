# Phases

A combat round is five steps. The middle step (Resolve Engagements) walks a fixed
schedule of role-vs-role engagements, each running the same three sub-phases. The
ordering of that schedule is what replaces the old interception / pre-empt /
Reckoning phases — a unit struck early in the schedule may be gone before it would
have acted later, so "resolve last / fizzle if dead" falls out of the order itself.

## Round overview
1. **Declare Intentions**
2. **Reveal Intentions**
3. **Pre-Battle Effects**
4. **Resolve Engagements** — loop the schedule a–e; each engagement runs three
   sub-phases: **Declare Effects → Resolve Effects → Apply Effects**
5. **Reset**

Declare Intentions, Reveal Intentions, Pre-Battle Effects, Resolve Engagements (Intercept, Volley, Raid, Clash, Breach), and Reset

## Tempo
- One Tempo pool per character, refreshed at Reset, **shared across the entire
  round** (all of step 4).
- Spent to declare attacks, to avoid incoming attacks (beat Finesse), and to
  strike back.
- A character may act in **as many engagements, and as many times within an
  engagement, as its Tempo allows** — so spending early (attacking or defending in
  an early engagement) starves it later in the schedule.

## Information model
- **Tabletop:** declarations are made openly and resolved as you go; Tempo is the
  bookkeeping token. The hidden 1-D deck below is the formal model, not how a table
  actually plays.
- **PvE:** the opponent follows scripted rules — no hidden reveal. This is the
  perfect-information, finite, computable case, and the **balance oracle**.
- **PvP:** genuinely simultaneous and hidden, so **not strictly computable**;
  its balance is **approximated by PvE**.

## Declare Intentions
There are 3 intention cards: **Vanguard**, **Outrider**, **Rearguard**.
Build a 1-dimensional deck: an intention card first, then each character taking
that intention, repeated for each intention. **Hidden information.**
Use **Join** cards to mark two adjacent characters as part of the same group.

## Reveal Intentions
Lay the intention and character cards out 2-dimensionally. **Revealed information.**
One row per intention: the intention card on the left, its characters in order to
the right; cards in the same group flush against each other, separate groups
spaced apart.

## Pre-Battle Effects
Apply immediately; independent of character reactions or later phases, so they
**must be order-independent by design**. They do **not** interact with a damage
pile, though they may directly flip cards or attach temporary cards. Usually for
buffing your own party, or attaching a card to be interacted with in a later phase.

## Resolve Engagements
Walk the schedule a–e in order. Each engagement runs:

- **Declare Effects**
  - Attacks are limited by the intentions of the initiating and target characters
    (per the schedule) — **except a strike-back is always allowed against a melee
    attacker**, if you have the Tempo.
  - As many attacks as Tempo allows may be declared.
  - **All attacks on both sides are declared before any are resolved.**
  - An attack is composed of **might**, **finesse**, and **range** (melee, ranged,
    melee-AoE, ranged-AoE).
- **Resolve Effects**
  - Damage accumulates into the per-phase pile. For each incoming attack:
    - spend enough Tempo to **strictly beat its Finesse** to avoid it, or
    - **eat it** — and if melee, may spend one Tempo to **strike back**.
    - abilities may be used to change accumulated damage.
- **Apply Effects**
  - Health cards flip to resolve damage; the pile clears the target's Toughness to
    flip a card. A character with all health cards flipped is removed.
  - Excess damage insufficient to flip a card is discarded.

### The engagement schedule
Resolved in order. Every legal attacker→target role-pair appears exactly once
(Rearguard→Rearguard is the only illegal pair — it needs an enabling effect).

| Step | Engagements |
|------|-------------|
| **Intercept** | Vanguard → Outrider |
| **Volley**    | Rearguard → Outrider |
| **Raid**      | Outrider → Rearguard |
| **Clash**     | Rearguard → Vanguard, Vanguard → Vanguard |
| **Breach**    | Vanguard → Rearguard, Outrider → Vanguard, Outrider → Outrider |

Because Outriders are *targets* in a/b before they are *attackers* in c/e, a
flanker cut down crossing never delivers its strike — interception and
"resolve-last fizzle" are encoded by position in this order, not by separate phases.

## Reset
- Discard temporary cards.
- Refresh Tempo.
- Repeat from Declare Intentions.
