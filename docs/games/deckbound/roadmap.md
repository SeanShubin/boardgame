# Deckbound — Roadmap (undefined mechanics & planned features)

> **Status: living, non-authoritative.** This tracks what **doesn't exist yet** — design
> gaps to flesh out, and tooling/features to build. It's a to-do list, not a source of
> truth.
>
> Sibling to [`future-possibilities.md`](future-possibilities.md) — that doc holds
> speculative *changes to mechanics that already exist*; **this** doc lists *things not yet
> defined or built*. The **canonical** spec-status of each mechanic is the **Coverage
> table** in [`canon/2-spec/README.md`](canon/2-spec/README.md); when an item here is
> fleshed out, it goes through the change discipline in
> [`canon/0-source-of-truth.md`](canon/0-source-of-truth.md) (**spec first**, then code,
> then tests) and graduates onto that Coverage table.

---

## A. Undefined mechanics (design gaps — each needs a spec section)

Most of these are the **strategic / run layer** — the game outside a single combat. The
tactical core (the Clash) and the combat breadth layer (Formation/Tempo/Focus) are the
parts that *are* specced.

- **Geography** — the structure of the world/map: places, adjacency, what a "location"
  is. No spec; little prior thinking. *(see if any survives in
  `notes/world-and-progression.md`.)*
- **Travel** — moving through geography between conflicts: cost, risk, encounters,
  whether it consumes a resource or triggers world events. No spec.
- **Loot** — rewards from conflicts/exploration: what drops, how it's acquired, and how it
  becomes new **cards / aspects** (ties into `notes/cards-and-customization.md` and
  `notes/decks-and-aspects.md`). No spec.
- **Progression** — how a character/party grows between fights (acquiring cards, raising
  aspects, the solo "play tall" vs co-op "play broad" split). Prior thinking in
  `notes/world-and-progression.md` and `notes/archetypes.md`; **no spec**.
- **World events** — the strategic event/world decks that drive the run. Tracked on the
  Spec Coverage table as **"Strategic layer (world/event decks) — ⬜ stub"**; design
  thinking in `notes/world-and-progression.md`. Needs a real spec.
- **Victory / defeat conditions** — **partly defined, partly not.** *Skirmish*-scope is
  defined and implemented (every foe down = win; the party falling = loss — see
  `notes/form-and-defeat.md` and `check_outcome` in code). **Undefined:** *run-level*
  victory and defeat — a game is **many skirmishes**, and how the run as a whole is won or
  lost (objectives, failure states, what ending a run means, non-elimination wins) has no
  definition.

> Note: these gaps are now represented truthfully on the **Spec Coverage table**
> (`canon/2-spec/README.md`) as ⬜ stub rows — skirmish victory/defeat as 🟡 seeded, and
> run victory/defeat, geography & travel, loot, and progression as ⬜ stubs.

## B. Planned features (tooling & implementation — not mechanics)

These are software/artifacts, not rules, so they live outside the Spec.

- **Human-emulating combat AI.** A driver that plays an opponent like a **Character**
  (theory of mind, bluffing, mixed strategy) rather than the current `Creature` drivers
  (`Instinct::Deck` random lean, or deterministic tutorial `Script`s). The *intent* for
  what a Character is exists in `notes/entities.md` and `notes/decision-making.md`; the
  actual emulator does not. This is what would make a non-human opponent feel like a real
  duelist instead of a deck.
- **In-game encyclopedia.** In-app rules lookup — search the Spec keywords/procedures and
  read them in context while playing. (Pairs with the keyword **MANUAL** lines the Spec
  already authors for exactly this "digital and printed rules can't drift" reason.)
- **Detailed card lists / interaction reference.** A per-card reference describing how each
  card interacts with the rules. The source-of-truth model already anticipates this as a
  **generated projection** of `booklet.ron` × the Spec's keyword manual lines (a printed
  card = values × rules text). The generator doesn't exist yet — see
  [`canon/3-booklet.md`](canon/3-booklet.md).
- **Print-export.** Dump the cards from `booklet.ron` into the format a print-on-demand
  company needs for **custom physical cards** (likely CSV/JSON + art/layout fields). Same
  generation seam as the card lists; it's the "send it to the printer today" output the
  print-master is meant to feed.
- **Card presentation polish (look & feel).** The bar for representing physical cards
  digitally — the `tabletop` crate should make cards feel like tactile objects, not flat
  sprites: hover-tilt toward the cursor, lift + drop-shadow, a satisfying "thunk" on play,
  and a particle/sound/animation triad on every meaningful state change (flip, deal,
  damage, defeat). Juice over expensive art — easing curves, screen-shake, and audio carry
  most of the feel and map cleanly onto Bevy systems.
  - **Standards to aspire to:** **Marvel Snap** (the modern ceiling for flashy reveals,
    foils, and animated/parallax cards), **Hearthstone** (the fundamentals of *physical*
    card feel — weight, tilt, shadow, thunk, a board you can poke), **Legends of Runeterra**
    (animated card art + reactive board theater), and **Balatro** (the achievable-in-code
    target: exaggerated springy motion + screen-shake + sound make a minimal art style feel
    incredibly juicy — its juice is curves and audio, not art budget).
  - Ties into the **cards-only** pillar in `notes/physical-representation.md`: every
    card-state change is meaningful, so every one deserves a felt animation. Generic and
    over `Game` — keep it in `tabletop`, never reference a specific game (see
    `.claude/CLAUDE.md` architecture rules).

## C. See also

- **Spec Coverage table** — `canon/2-spec/README.md` (canonical spec-status of mechanics).
- **Speculative changes** to *existing* mechanics — `future-possibilities.md`.
