# Deckbound — The Card-Table UI (a rigorous physical metaphor)

> **Status: future direction, not scheduled (recorded 2026-06-20).** We are **not building this
> now** — tuning the role-card model against the reference scenario comes first. This document is the
> **north star for the renderer**: when we add or change UI, it should move *toward* this model, not
> away from it. Nothing here is canon (it governs no rules); it governs *how the rules are shown*.
>
> Origin: a designer's vision (the human) for making the on-screen game a faithful image of the
> tabletop game it secretly is. The framing, the scenario, and the primitives below are theirs; the
> inferences that fill the gaps are marked *(inferred)* and are for the human to confirm or correct.

---

## 0. The principle — every card has a physical place, always

The game is **playable by hand, cards only** (Charter #7), and every rule **rides on a physical
metaphor** (#9). The UI should honour that literally: **at every moment we know where each card
physically is, and every card is on screen at all times** — either shown as a card, or *collapsed
into a deck* among other cards. Nothing is invented for the screen that couldn't sit on a real table;
nothing on the table is hidden from the screen.

The corollary that makes this tractable: **we attend to different sets of cards at different times.**
The whole design problem is *representing attention* — showing the set you're working with in full
while keeping everything else present-but-compact. That representation is what a human designer is
*for*; this doc records the human's answer.

---

## 1. The primitives — **cards** and **decks**

- **Card** — already exists (a [`CardView`]). A single face-up or face-down card.
- **Deck** — *new primitive:* **a pile of cards stacked on top of each other.** A deck shows **what
  kind of deck it is** and **how many cards are in it**. A deck is the compact form a set of cards
  takes when you are *not* attending to it.

Everything on the table is therefore one of: a loose card, or a deck. The screen is never cluttered
with everything at full size, and never drops anything — the unattended simply **collapse into
decks**.

### Focus by zoom (the camera follows attention)

- **Click a deck → it fans out** and the camera focuses there (you're "picking it up to look").
- **Click the table → zoom one level out** (put it back down).
- This is **recursive** — a deck can contain decks; you can drill in and back out level by level.
- **Dead zone around a card** so a click meant for a deck doesn't accidentally hit the table and zoom
  out. *(We are not worried about the reverse — an accidental deck-click is self-evidently what
  happened, and easy to back out of.)*

### Collapse-the-unattended

When you focus one set, **everything else collapses into decks** automatically. The set you're
looking at is **fanned out and face-up** — as if you picked those cards up into your hand and are
flipping through them — while the rest of the table compresses into labelled, counted decks. So the
screen always reads as *"here is what I'm working with; everything else is still here, just stacked."*

---

## 2. The typical table layout

- **Location cards** sit in the **centre of the table** (the world map, §8.1).
- Each player's **character card** sits **on top of a location card** (its position in the world).
- Each player's **Form cards** lie **face-up in front of them** (the stat block — fundamental +
  attachments, §2.3 stats-as-deck; auto-laid on entering an area).
- Each player's **other cards** (the Action/role kit not in use) sit **face-down in a deck, set
  aside**.

The video-game version always renders **all** of this: every location card, and **which character
card is sitting on which location** — even when you're "only looking at the map."

---

## 3. Attention shifts by phase (the scenario the human gave)

The set of cards in focus changes with what you're doing. The renderer's job is to **fan the relevant
set and collapse the rest** for that moment, and to make clear *where to look*.

- **Moving (world phase).** You look at the **location cards only**. The map is fanned; each
  player's kit and Form collapse into decks sitting with their character. *(inferred: the focused
  layer is the location lattice with character cards on top; everything else is a deck.)*
- **Looking through my own kit.** Say I want to flip through my **face-down deck** instead of the map:
  the **locations pile into a single deck**, **each player's cards become an individual deck**, **my
  Form cards become a deck**, and **only my face-down deck is fanned out, shown face-up** (I "picked
  it up"). Every other card is still on screen — collapsed — so nothing vanished.
- **Muster / placement.** Several decks are fanned at once because I'm **moving cards between zones**:
  I pick my **character card** up into my hand and place it into the **Vanguard** or **Reserve** zone.
  So I see, together: **what's in my hand**, the **Vanguard zone card**, the **Reserve zone card**, and
  the **character cards other players have already placed** there. *(This is the §4.4 "place the
  character card into a zone" model the current muster already moves toward — see
  [`game-flow`](../game-flow.md) and the role-card play rule.)*

The general rule: **multiple decks fan out whenever an action spans them** (moving a card from one
place to another needs the source, the destination, and the neighbours already there all visible).

---

## 4. Free viewing vs. legal moves (the indicators)

- The player can **view any card at any time** — switch perspective at will, drill into any deck,
  even ones that aren't theirs or aren't actionable right now.
- But **most areas have no legal move** in a given phase. So the UI must signal:
  1. **Which decks/zones are actionable** — where clicking yields a *legal move*, not just a look.
  2. **Which phase you're in** — so the *context* tells you where to expect moves.
- Together these let a player **explore freely without getting lost**: look anywhere, but always know
  where the game currently wants their decision. *(This generalises the work already done — the
  "Next" hint line, the teal suggested-action highlight, the phase prompt — into a consistent
  "actionable here" affordance across the whole table.)*

---

## 5. The damage deck (physicalising accumulated damage)

A **deck of 1-damage cards**: as damage is dealt across a combat phase, **1-damage cards accumulate**
on the target, and are **applied at the end of the phase**. This makes the **order-independent
resolution** physical and legible — you literally watch the damage cards pile onto a target during the
phase, and the fall is resolved **once, at the phase boundary**, when the pile is totalled against the
body pool (cut → bar → pool, §2). It is the tabletop image of the rule we already enforce in code
(`tally` finalises a fall once, after the phase's damage has accumulated).

*(inferred: this dovetails with the combat feed — the feed narrates each 1-damage card as it lands,
and "X falls" when the pile is totalled at phase end. The damage deck is the *visual* of the same
event the feed describes in text.)*

### 5.1 The combat log as a **card-transition ledger**

> Raised 2026-06-20. **Direction; the full version depends on the card/deck state model below.**

The natural endpoint of "represent all state with cards" is that **the combat log becomes a ledger of
card state transitions** — the textual twin of the physical table. Every line is a card *moving* or
*changing face*, not an abstract event. The transition vocabulary:

- a card moves **between zones** (Hand → board, board → Spend/Down, deck → in play);
- a **character card** moves into the **Vanguard / Reserve** zone (muster);
- a **health card turns face down** (damage); a creature is **defeated** when all its health cards are
  face down (resolved at the phase boundary);
- a **damage card moves from the damage deck onto a creature's deck** (§5), and the pile is totalled at
  phase end;
- a **reward card moves from the treasure deck into a character's deck** when a location level is
  cleared — so *which locations are cleared, and to what level, is read off **what is missing from the
  treasure deck / present in a character's deck***, not a separate "cleared" marker. *(The data model
  already works this way: rewards are assigned to members at unlock and the unclaimed pool shrinks —
  see [`role-card-redesign.md`](../role-card-redesign.md) §8.3 / the campaign's `unassigned` queue. The
  UI just needs to **show** it as physical card movement.)*

There is deliberately **no "life total"** anywhere: a creature is never at "1/2 life"; it has *some
health cards face up and some face down*, and that discrete state is all there is.

**Done now (a first step):** the strike narration already speaks this language — "turns a health card
face down", "damage accumulates", "turned aside by its armor", and at the boundary "all its health
cards are face down; defeated" — with no life fraction. **Still future:** a *complete* ledger (zone
moves, hand↔board, the literal damage-deck and treasure-deck transfers) needs the card/deck **state
model** in §1 to exist first, so the log can name real card moves rather than describe combat math.

---

## 6. How this relates to what exists

- **Canon it serves:** Charter **#7** (cards only), **#9 / #10** (rules ride on a metaphor / are
  re-derivable), the **§5 zone machine** (Hand / Active / Down — a card's physical state), **§4**
  positions (Vanguard / Reserve as zones you place into), **§2.3** stats-as-deck (the Form cards), and
  the **order-independent resolution** (§4 / `tally`).
- **Code today:** the renderer already has cards ([`CardView`]), zones ([`ZoneView`]), a world
  [`MapView`], the event feed, the suggested-action highlight, and per-card hover/animation — the raw
  materials. What's missing is the **deck primitive**, the **zoom/focus camera**, the
  **collapse-the-unattended** behaviour, and the **"actionable here" affordance**.
- **Adjacent backlog:** the muster-as-placement and the zone-visuals direction (label-card-on-left,
  fan when crowded, hover-pop) in [`future-possibilities`](../future-possibilities.md) are *early steps
  on this path* — they should be built so they generalise into the deck/zoom model, not as one-offs.

---

## 7. Rendering approach — flexbox UI vs a 3D table — **OPEN, undecided**

> **Status: open question (raised 2026-06-20). Leaning unsure — the human "is not sure I will ever go
> with 3D."** Recorded so the tradeoff is on the table when UI work resumes; **not** a commitment
> either way.

The vision above (decks, zoom-to-focus, collapse-the-unattended) is achievable in **either** approach;
this question is only about *how the table is drawn*. Note the current renderer is **not** Bevy sprite
2D — it is **`bevy_ui`** (flexbox `Node`s, `Text`, `Interaction`, `UiTransform`) over a `Camera2d`. So
the real fork is **flexbox UI vs a 3D mesh scene**, not "2D vs 3D sprites."

**What only 3D can give** (these are *native* to a 3D scene, *faked at best* in UI):
- **Card thickness + realistic stacking** — a card is a thin box mesh; stacks occlude via the depth
  buffer, so you literally *see* how many are piled up.
- **Isometric view** — an orthographic 3D camera at a fixed angle.
- **Full camera orbit (pitch / yaw / roll)** around the table.
- **Honest deals / flips** — animating a real `Transform` rotation, not a `UiTransform` lift trick.

If we never want to *rotate* the table, UI + offset/shadow fakery (already used for identical-card
stacks via `STACK_PEEK`) may be enough, and 3D's costs aren't worth it.

**What 3D costs** (the price of leaving `bevy_ui`):
1. **No layout engine.** `bevy_ui`/taffy positions every card, zone, and panel for free (rows, wrap,
   gaps, scroll). A 3D scene places each mesh by explicit world `Transform` — fanning, stacking, and
   zone spacing become **hand-computed layout**. This is the bulk of the work.
2. **Text on card faces → a texture pipeline (the crux).** On a card that can rotate, text must live on
   the card's *surface*, so screen-space `Text` won't do. The realistic path is **rasterising each card
   face to an `Image` and using it as the card's material**, regenerated when the card's content
   changes — new infrastructure, and Deckbound is text-heavy (stat blocks, card bodies).
3. **Mesh picking** (`MeshPickingPlugin`, ray-cast) instead of automatic UI `Interaction` hit-testing.
4. **The full 3D pipeline** — `Camera3d`, lighting, `StandardMaterial`/PBR, shadows, depth. Trivial GPU
   cost for a card table, but more moving parts and look-tuning than flat UI quads.
5. **Heavier on the web** (wasm / WebGL2) than the current flat UI.

**What stays free either way:** the `engine` / `deckbound` logic and the **`TableView` seam** are
renderer-agnostic, so the rules and tests don't move — the cost is **confined to the `tabletop`
crate's view-building**. A 3D renderer could even be built **in parallel against the same `TableView`**
while the UI renderer keeps working.

**If we ever do go 3D, the recommended shape is hybrid, not all-3D:** keep `bevy_ui` for the
**HUD** (action buttons, the event feed, status, encyclopedia, card-detail reading panes — documents
and lists, text-heavy, miserable as meshes) and use **3D only for the physical table** (locations,
character cards, decks, the Vanguard / Reserve zones). That keeps the text crisp where it's read and
makes the table physical where it's handled — and it is exactly what the deck/zoom/orbit vision wants.

**Decision criteria to settle it later:** do we actually want to *rotate/orbit* the table (→ 3D), or is
a fixed top-down/iso-fake view with offset-stacking enough (→ stay UI)? Is the **text-to-texture**
infrastructure worth it for the metaphor gain? Does the **web build** weight against 3D?

## 8. Open questions (for the human, when we pick this up)

- **Deck identity & count rendering** — how a collapsed deck shows its kind and size at a glance
  (icon + numeral? a labelled spine?).
- **Zoom levels** — how many levels deep does recursion realistically go (table → player area → deck →
  card), and what's the back-out gesture besides clicking the table?
- **Multi-deck fan layout** — when several decks fan at once (muster), how they share the screen
  without crowding (the same crowding problem as the zone-visuals fan).
- **Perspective in a single-player game** — the human noted the metaphor blends a multiplayer table
  with one player controlling several characters; what "in front of me" means when *I am all the
  players* needs a convention.
- **Actionable affordance** — the exact visual language for "a legal move lives in this deck/zone"
  vs. "you may only look here."

---

**Using this document.** When a UI feature is proposed, ask: *does it move us toward "every card has a
place, always on screen, attention shown by fanning one set and collapsing the rest"?* If yes, build
it so it composes with the deck/zoom model. If it invents a screen-only abstraction with no physical
referent, that's the smell to stop and reconsider — here, on purpose.

[`CardView`]: ../../../crates/engine/src/view.rs
[`ZoneView`]: ../../../crates/engine/src/view.rs
[`MapView`]: ../../../crates/engine/src/view.rs
