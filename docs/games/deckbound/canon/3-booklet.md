# Booklet — the print master (pointer)

> **This is a pointer, not the document.** The print master itself is data, and it
> lives in the crate because the game loads it at build time
> (`include_str!` in `crates/deckbound/src/scenarios.rs`):
>
> **`crates/deckbound/data/booklet.ron`**

`booklet.ron` is the third canonical source of truth, co-equal with the
[Charter](1-charter.md) (intent) and the [Spec](2-spec/README.md) (mechanics). It owns a
different question: **what pieces exist and what their numbers are** — every card, trait,
actor, prebuilt deck, and scenario, with their values. See
[the governing rules](0-source-of-truth.md) for how it relates to the others.

It is kept in the crate (rather than physically beside the other canon files) only because
it is a **compile-time build input**, not because it ranks lower. When you change it:

- It composes **only** keywords the Spec defines — never hand-written rules text.
- **Numbers are AI-seeded, human-tuned:** propose values; the human decides them.
- Every keyword / effect / trait / card-reference in it must resolve to a Spec definition
  or a catalog entry, or the print master isn't shippable (a defect).
