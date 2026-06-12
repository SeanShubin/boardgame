# Deckbound — Cards & Customization

The card system is a **matrix of customization**: a few orthogonal dimensions
that combine, so characters differ wildly while each card owns a distinct,
meaningful cell. This note collects the dimensions, how they combine into an
action, and a starting set per dimension. Focus is **physical / mental / magical**
— Spirit is deferred.

## The quality axes, and where they come from

Three magnitude axes drive an action, and — crucially — they are **sourced from
different aspects**:

- **Power — Body.** Force → damage; hard to interrupt. Available even **mindless or
  blind**.
- **Speed — Body** (modified by weapon weight). Act earlier; harder to be hit. A
  correct **Mind** read can grant initiative on top.
- **Precision — Mind-gated.** Hit weak spots → bonus damage / bypass armor. With no
  working Mind you can't aim at all — you swing **wild** (Power and Speed only,
  random spot).

Plus the **read (RPS) — Mind**: anticipation, the hidden-information layer (see
[decision-making](decision-making.md)).

**This falls straight out of the aspect-chord.** An action is one card per aspect,
so a **Body card alone** (a mindless beast, or a sealed/blinded Mind) is Power +
Speed with **no Precision and no read** — the wild swing; a **Body + Mind chord**
adds aim and the read. No special "blinded" rule is needed: *blind = Mind sealed*.
It is also why **disabling a Mind is the deepest cut** — it strips Precision, the
read, and [recovery](zones.md) at once. (Magic mirrors this with its own Power;
Precision likely still draws on Mind. Deferred.)

## The dimensions

A character — and each action — is built by combining cards across orthogonal
dimensions:

| Dimension | Aspect / kind | Role |
| --- | --- | --- |
| **Action** | Body — playable (Potential) | the *means*: what you physically do |
| **Tactic** | Mind — playable (Potential) | the *read* (RPS) + meta (recovery) |
| **Spell** | Magic — playable (Potential) | a parallel attack track + status effects |
| **Quality** | modifier | which axis an action pushes (Power / Speed / Precision) |
| **Weapon** | Form (equipment) | enables / reshapes actions; sets damage type & profile |
| **Armor** | Form (defense) | reduces incoming damage *by type* |
| **Damage type** | property | carried by weapons / spells; meets armor in an RPS |

### How an action resolves (the chord)

A physical attack = **Body action** × **quality emphasis** × **weapon** → yields
(Power, Speed, Precision, **damage type**); the **Mind tactic** overlays the read;
it meets the target's **armor** (type reduction) and **Body** toughness, eroding
Body cards (see [form-and-defeat](form-and-defeat.md)). Aspects combine
commutatively; only the Mind tactic is rock-paper-scissors.

> **"Which is which."** *Quickly / hard / precisely* are **not** actions or tactics
> — they are the **quality axes**. "Strike quickly before they can react" is a
> chord: a fast **Body** strike + a **Mind** tactic that reads their timing.

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

**Disarm** sends a weapon Form card to **Dormant**. **Re-equipping** is an action
that leaves you **vulnerable** — unless a character has a maneuver to pick it up
cleanly.

## Armor is Form (defense) — and meets weapons in an RPS

Cloth / Leather / Chainmail / Plate each reduce damage **by type**, so no armor is
universally best; the foe's armor tells you which weapon to bring:

| Damage ↓ \ Armor → | Cloth | Leather | Chainmail | Plate |
| --- | --- | --- | --- | --- |
| **Blunt** (bash) | ok | ok | good | **strong** — concussion through plate |
| **Sharp** (slice) | **strong** | good | weak | weak |
| **Pierce** (poke) | good | ok | **strong** — between the rings | weak |

Profiles are a starting *shape*; numbers TBD. **Entangle** is **control**, not
damage — a Seal / restrain effect. And metal armor (mail / plate), strong against
steel, may be a **liability** against lightning or heat — so the right armor
depends on the threat.

## Magic — a parallel track

Magic deals its own damage **types** (Heat, Cold, Lightning) and carries **status
effects** that hook into existing mechanics:

- **Heat → burn** — a *Lasting* card dealing Body damage each turn.
- **Cold → freeze / slow** — **Seal** cards, or cut Speed.
- **Lightning → shock** — **Seal a Mind tactic** (can't read) or stun.

"As many effects as we have mechanics to interact with," so magic grows with the
rest of the system.

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
  **stagger** — high Speed and just enough Power to interrupt. The natural guard's
  tool: an ideal opportunity-attack / [interrupt](coordination-and-interruption.md).

*(Block and Evade are defensive **reads** — see [the Mind](mind-and-reads.md). A
shield is the Form card that enables both Block and the Shield bash above.)*

## Tactics — the Mind pool

Where the [read / RPS](decision-making.md) lives, plus meta abilities:

- **Anticipate** — read their action to gain advantage / counter.
- **Recover** — rouse Dormant cards back to Potential (see [zones](zones.md)).
- *(feint, bait, focus, … TBD)*

## Open questions

- Do **quality emphases** attach as modifier cards, or are they baked into each
  action / weapon profile?
- Exact **stat profiles** for Body actions and weapons.
- The **armor × damage numbers**, and whether metal armor's magic vulnerability is
  in.
- The full **tactic** list, and how each interacts with the RPS.
- How **Magic** axes / precision work — its own, or shared with Mind.
