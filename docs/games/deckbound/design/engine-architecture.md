# Deckbound — Engine Architecture

How one set of rules drives a digital game (two front-ends) **and** a printable
tabletop game, and what can change without a rebuild. This is the *implementation*
plan; the game's rules themselves live in the rest of these notes.

## The board-game metaphor — three tiers

| Tier           | Is                                                                                                            | Answers                          | Form     | Change =     |
| -------------- | ------------------------------------------------------------------------------------------------------------- | -------------------------------- | -------- | ------------ |
| **Rulebook**   | the engine: procedures, keyword handlers, the menu of options                                                 | *how it works*                   | **code** | **redeploy** |
| **Appendix**   | global tunables + distributions: drag aggregate, momentum / fear numbers, caps, counts, pools, scenario setup | *how much · how many · what mix* | **data** | reload       |
| **Components** | the catalog of card / creature / token **definitions** (each a bag of keywords + values)                      | *what the pieces are*            | **data** | reload       |

**Which tier?** A procedure or a brand-new keyword's meaning → **rulebook**. A specific
piece's definition → **component**. A number or mix that spans pieces → **appendix**.

**Deploy boundary:** only a **rulebook** change needs a rebuild. Appendix and components
are pure data — rebalance and expand freely. A component that needs a *new* rule pays
**one** rulebook update (the new keyword); thereafter that rule is free vocabulary in
data.

## Keywords — data-composed, engine-interpreted

Cards and creatures are **bags of keywords + parameters** (components, data). The engine
holds, per keyword, **two things**:

- an **executable handler** — what it *does*, and
- a **one-line manual text** — what the rulebook *says*.

That dual nature is the join point: the engine runs the handler; the **rulebook
generator** prints the text. Digital and printed rules therefore **cannot drift** — they
come from the same definition. (And "can I write a clean one-line manual entry?" is a
design test for whether a keyword is too fiddly — [philosophy §9](philosophy.md).)

Not a scripting DSL — a **fixed vocabulary** the engine knows. Composing known keywords
onto a new card is data; a genuinely new mechanic is a new handler (a redeploy).

## One engine, three projections

```
 rules data  =  appendix (tunables + distributions)  +  components (keyword bags)
 rulebook    =  the engine (keyword handlers + resolution procedure), in the pure `engine` crate
                 │ compiled to WASM                    │ compiled native
            HTML / JS front-end                   Bevy tabletop front-end          generated TABLETOP KIT
            (renders TableView,                   (renders TableView,              · rulebook  ← procedure + keyword glossary
             calls legal-moves / apply)            same engine)                    · reference/setup sheets ← appendix
                                                                                   · card sheets ← components
```

- **One interpreter.** The pure, Bevy-free `engine` crate compiles to **WASM** (for the
  HTML/JS UI) and **native** (for Bevy). No second engine in JS — no drift.
- Both digital front-ends are **thin renderers** of the engine's `TableView`, calling
  **legal-moves** and **apply-move**; the engine enforces legality.
- The **tabletop kit** is generated from the same source, so hand-played = on-screen.

## Computer enhancements over the bare tabletop

The UI *is* the physical table (move cards between zones), plus three things a table
can't do:

- **Illegal moves are prevented** — the engine's legal-move check gates every action.
- **Rules are discoverable in context** — a keyword's manual text surfaces on hover.
- **Options are shown** — legal targets / plays are highlighted.

## The rulebook spine (next)

Two notes define the rulebook: the **[resolution procedure](resolution.md)** (the
deterministic round the engine runs) and the **[keyword vocabulary](keywords.md)** (each
keyword's handler intent + manual text). Together they fix the shape of the data file,
the engine, and the generated manual at once. The appendix's actual numbers are
first-pass / TBD until the balance phase.
