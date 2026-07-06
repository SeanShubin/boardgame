# Deckbound — Cards & Customization

The card system is a **matrix of customization**: a few orthogonal dimensions
that combine, so characters differ wildly while each card owns a distinct,
meaningful cell. This note collects the dimensions, how they combine into an
action, and a starting set per dimension. Focus is **Body / Mind** — Spirit is
deferred. (Magic is not a fourth axis: it is one way the **Body** delivers a
physical effect — see [§Magic](#magic--a-source-and-delivery-not-a-bypass).)

## The quality axes, and where they come from

Three magnitude axes drive an action, and — crucially — they are **sourced from
different aspects**:

- **Power — Body.** Force → damage and **dropping** a foe; pure magnitude, no separate
  interrupt job. Available even **mindless or blind**.
- **Speed — Body** (modified by weapon weight). Act earlier; harder to be hit. A
  correct **Mind** stance can grant initiative on top.
- **Precision — Mind-gated.** Hit weak spots → bonus damage / bypass armor. With no
  working Mind you can't aim at all — you swing **wild** (Power and Speed only,
  random spot).

Plus the **stance (RPS) — Mind**: prediction, the hidden-information layer (see
[decision-making](decision-making.md)).

**This falls straight out of the aspect-chord.** An action is one card per aspect,
so a **Body card alone** (a mindless beast, or a sealed/blinded Mind) is Power +
Speed with **no Precision and no stance** — the wild swing; a **Body + Mind chord**
adds aim and the stance. No special "blinded" rule is needed: *blind = Mind sealed*.
It is also why **disabling a Mind is the deepest cut** — it strips Precision, the
stance, and [recovery](zones.md) at once. (A cast is no exception: it uses the
**shared Power** for magnitude and **Mind's Precision** for weak-spots — no private
axes of its own.)

## The dimensions

A character — and each action — is built by combining cards across orthogonal
dimensions:

| Dimension       | Aspect / kind               | Role                                                                   |
| --------------- | --------------------------- | ---------------------------------------------------------------------- |
| **Action**      | Body — playable (Potential) | the *means*: what you physically do                                    |
| **Tactic**      | Mind — playable (Potential) | the *stance* (RPS) + meta (recovery)                                   |
| **Spell**       | Body — playable (Potential) | a physical delivery: elemental damage-type, ranged/AoE, status effects |
| **Quality**     | modifier                    | which axis an action pushes (Power / Speed / Precision)                |
| **Weapon**      | Form (equipment)            | enables / reshapes actions; sets damage type & profile                 |
| **Armor**       | Form (defense)              | reduces incoming damage *by type*                                      |
| **Damage type** | property                    | carried by weapons / spells; meets armor in an RPS                     |

### How an action resolves (the chord)

A physical attack = **Body action** × **quality emphasis** × **weapon** → yields
(Power, Speed, Precision, **damage type**); the **Mind tactic** overlays the stance;
it meets the target's **armor** (type reduction) and **Body** toughness, eroding
Body cards (see [form-and-defeat](form-and-defeat.md)). Aspects combine
commutatively; only the Mind tactic is rock-paper-scissors.

> **"Which is which."** *Quickly / hard / precisely* are **not** actions or tactics
> — they are the **quality axes**. "Strike quickly before they can react" is a
> chord: a fast **Body** strike + a **Mind** tactic that predicts their timing.

## Weapons are Form (equipment)

A weapon is a **Form card** you wield. It:

- **enables** certain Body actions — a bow enables ranged strikes; a **shield**
  enables both a strong **Block** *and* a **Shield bash** (one Form card, a defense
  and an attack),
- sets a **damage type** — bash → blunt, slice → sharp, poke → pierce, **entangle →
  control**, and
- shifts the **Power / Speed** profile (a maul: high Power, low Speed).

**Handedness gates the loadout:** two-handed forbids a shield; one-handed frees a
hand; ranged needs distance; **preloaded** (crossbow / gun) is a burst that then
needs reloading — a tempo mechanic.

**Disarm** turns a weapon Form card **face down**. **Re-equipping** is an action
that leaves you **vulnerable** — unless a character has a maneuver to pick it up
cleanly.

## Armor is Form (defense) — and meets weapons in an RPS

Cloth / Leather / Chainmail / Plate each reduce damage **by type**, so no armor is
universally best; the foe's armor tells you which weapon to bring:

| Damage ↓ \ Armor → | Cloth      | Leather | Chainmail                      | Plate                                 |
| ------------------ | ---------- | ------- | ------------------------------ | ------------------------------------- |
| **Blunt** (bash)   | ok         | ok      | good                           | **strong** — concussion through plate |
| **Sharp** (slice)  | **strong** | good    | weak                           | weak                                  |
| **Pierce** (poke)  | good       | ok      | **strong** — between the rings | weak                                  |

Profiles are a starting *shape*; numbers TBD. **Entangle** is **control**, not
damage — a Seal / restrain effect. And metal armor (mail / plate), strong against
steel, may be a **liability** against lightning or heat — so the right armor
depends on the threat.

## Artifacts — acquired modifiers that scale the numbers

Growth doesn't mean a fatter stack of cards. An **artifact** is a single acquired card
that **tunes a value** on your rules or capability cards — you get stronger by **scaling
a number, not adding to the pile** (the same principle that lets a 100-Body creature be
ten cards; see [form & defeat](form-and-defeat.md#example--a-vitality-card)):

- *Aegis* — **+1 to the toughness of every Body card** (each now absorbs one more
  damage): a real durability gain from **one card**.
- *Whetstone* — **+1 Power**; *Boots of the Hart* — **+1 Speed**; *Wardplate* — **+1
  armor vs a damage type**.
- Bigger finds add a whole **option** instead of a number — a new stance, a new Body
  action, even a new **aspect** (a whole new deck).

Artifacts live in **Form** (persistent), or **attach** as modifier cards — and per the
modifier rule, **attachment order can matter** (+1 then ×2 ≠ ×2 then +1). They are the
physical face of [acquisition](world-and-progression.md#exploration--acquisition): you
explore, you find one, you slot it — and the table shows your growth as a **few telling
cards**, never a bloated hand. (Every card you hold still earns its cell.)

## Magic — a source and delivery, not a bypass

Magic is **not an aspect** — it is a **source / delivery** of **Body** effects: the
contrivance that explains how a physical effect manifests, but **magic alone touches
no one**. It must **manifest a physical effect** (heat, cold, force,
lightning), and that *physical* effect, with its physical properties, is what does the
affecting. A fire spell manifests **heat** damage, a frost spell **cold**, a storm
spell **lightning** — and that damage is **typed** and meets armor
through the **same matrix** as any weapon (so the metal armor that turns a blade is a
*liability* against heat or lightning). **There is no "magic ignores armor."** What
makes magic distinct is its **effects** — status, **targets** (how many), and **reach**
(how far) — never a bypass:

- **Heat → burn** — a *Lasting* card dealing damage each turn.
- **Cold → freeze / slow** — **Seal** cards, or cut Speed.
- **Lightning → shock** — **Seal a Mind tactic** (no stances, no recovery) or stun.

"As many effects as we have mechanics to interact with," so magic grows with the rest
of the system — but always as **typed, physical** effects, however they're explained.

Bypassing physical constraints altogether is the province of the **Spirit** aspect,
*not* Magic. Spirit has **no physical effects** of its own; it reaches the
**will to act** — fear, morale, resolve, disposition — and
works *through your own response*, so a fearless character ignores it while a fearful
one is undone by their own panic. See
[the aspects](decks-and-aspects.md).

## Targets — breadth, the second damage axis

"AoE" is just a special case of a cleaner property: **number of targets.** An attack or
spell names how many **distinct** entities it may hit — *the same entity can't be
targeted twice* — and its **magnitude** lands on each. So damage scales on **two
independent axes**:

- **Magnitude** — how hard each hit lands (Power vs toughness).
- **Targets** — how many entities it strikes at once.

A plain blow is **1 target**; a cleave or **Firestorm** hits **several**; true "area" is
simply **targets ≥ everything in range**. Against a
[swarm](physical-representation.md#swarms--a-hundred-as-one-card-and-a-count), an
N-target effect removes **N bodies** (each taking the magnitude) — so a Firestorm that
hits **5** clears five of a six-Husk pack and the sixth overflows. Scale a spell up
either way: **harder** (magnitude) or **wider** (targets).

### How targets, reach, and the stance interact

A few rules keep multi-target attacks coherent — especially against the
[stance](mind-and-stances.md):

- **Targets are drawn from within reach.** Reach sets *where* you can hit; the target
  count is *how many* of the entities there you strike. A melee `[1,1]` cleave hits
  several in the **adjacent rank only**, never beyond its reach.
- **Targeting is enemy-directional.** You choose targets from the *enemy* entities in
  reach; your own ranks are never auto-hit. Friendly fire is a special trait, not the
  default.
- **Breadth ≠ bandwidth.** A multi-target attack is **one ability, paid once** (its
  magnitude to each target). Making *separate* attacks instead is **bandwidth**, paid in
  [tempo](speed-and-tempo.md) per sub-phase. Two different ways to hit many — don't
  conflate them.
- **It resolves *pairwise* against the stance.** A multi-target attack commits **one** stance
  (a Strike) to all its targets; each target *engaging* the attacker
  ([Holding or mutual](coordination-and-interruption.md#the-coherence-principle)) predicts
  back, and the cycle settles **per pair** — so a cleave can be **partly foiled** (whoever
  predicts it right negates it *for themselves*; the rest eat it). A stance protects only its
  owner; a target not engaging the attacker **auto-takes** it.
- **Breadth forgoes prediction.** You can't out-predict several foes with one stance — going
  wide means committing **blind** to a single stance against all of them. Tailoring a prediction
  to a specific foe is what **single-target** sub-phases are for. *Width trades the prediction
  advantage.*

*(Open number: when one multi-target Strike beats several defenders' stances, **cap** the
momentum it banks — width already paid for itself in damage.)*

## Body actions — a starting set

Each action earns its cell with a **signature effect**, not just stats
(Power / Speed / Precision profiles TBD):

- **Hand strike** — versatile; the weapon carrier.
- **Foot strike** — reach + Speed; no weapon.
- **Headbutt** — needs no weapon; small **stun**; slight self-risk. The desperate
  option.
- **Ram** — max Power; **exposes you** (counters land harder). All-in.
- **Grapple** — **locks an opponent's capability** (disables an action). The
  control specialist.
- **Tackle** — close distance **and** knock down. Repositioning.
- **Sweep** — low damage; **knockdown** (tempo swing).
- **Shield bash** — needs a **shield**; blunt and fast with low damage but a strong
  **stagger** — high Speed plus the **stagger** keyword: land first and the target loses
  its action. The natural guard's tool: an ideal opportunity-attack /
  [pre-emption](coordination-and-interruption.md#pre-emption--stopping-a-foes-blow).

*(Block and Evade are defensive **Stances** — see [the Mind](mind-and-stances.md). A
shield is the Form card that enables both Block and the Shield bash above.)*

## Tactics — the Mind pool

Where the [stance / RPS](decision-making.md) lives, plus meta abilities:

- **Predict** — predict their action to gain advantage / counter.
- **Recover** — turn face-down cards back up (or return them to hand) (see [zones](zones.md)).
- *(feint, bait, focus, … TBD)*

## Open questions

- Do **quality emphases** attach as modifier cards, or are they baked into each
  action / weapon profile?
- Exact **stat profiles** for Body actions and weapons.
- The **armor × damage numbers**, and whether metal armor's magic vulnerability is
  in.
- The full **tactic** list, and how each interacts with the RPS.
- **Magic axes / precision (RESOLVED).** A cast has **no private axes**: its
  magnitude is the **shared Power** and its weak-spot hit is **Mind's Precision** —
  exactly like any physical delivery. (A warding charm could exist as **gear**, but
  it is *not* anti-magic — a fire-resist is simply an **Armor** type against the heat
  damage-type, alongside the passive [Ward](form-and-defeat.md#ward--the-inner-cut) inner cut.)
