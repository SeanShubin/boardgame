# Deckbound — Source of Truth & How to Behave

**This is the entry point for the entire Deckbound design. Read this before
changing anything, answering any design question, or making any suggestion.**

Its job is to tell you (human or AI) **where truth lives**, **who wins when two
documents disagree**, and **exactly how you are expected to behave** when you
work on this game.

---

## ⚠️ AI ASSISTANTS — READ THIS FIRST

This section is binding. If you are an AI assistant working on Deckbound, these
are the rules of the house. Follow them literally.

### The one-paragraph version

There are **four** kinds of truth, and they answer **different questions**.
**Intent** lives in the **Charter**. **Rules and what keywords mean** live in the
**Spec** — the Spec is canonical for mechanics. **What pieces exist and their
numbers** (cards, traits, actors, prebuilt decks, scenarios) live in
**`booklet.ron`** — the print master, canonical for components and values. The
**code is not a source of truth** — it is an implementation that may contain
**defects**, and when it disagrees with the Spec or `booklet.ron`, *the code is
wrong by default*. The old `notes/` notes are **frozen exploration** and are
**not authoritative** — many are stale. When you are unsure, **ask the human; do
not invent, and do not promote a note or the code to truth.**

### Motivated rules — the bar every rule must clear

A *simple* rule can still be hard to hold if it is **arbitrary** — nothing explains
its shape, so it can only be memorized. A *complex* rule can be easy to hold if it is
**motivated**: its form follows from its intent, so anyone who grasps the intent can
**re-derive** the parts they forgot. **Always prefer a motivated rule over a merely
short one.** Arbitrary simplicity is a trap; motivated complexity is fine.

The whole-game target is **conceptual integrity**: every rule springs from a few
intents (the Charter's north stars and each rule's WHY), so a reader — human or AI —
who holds the intents can *reconstruct* the mechanics instead of memorizing them. This
is exactly why every Spec rule carries a **WHY** (and **GUARANTEES**): the WHY is the
rule's motivation — the thing you reconstruct from. A rule you cannot motivate is a
smell. This principle is **Charter north star #10 (*conceptual integrity*)**; this
section is how to apply it.

**This is a working directive for AI assistants, not a nicety:**

- When you **write or change** a rule, make it **re-derivable from its intent**, and
  state that intent (the WHY). If you cannot articulate a WHY, stop — the rule is
  probably arbitrary, and an unmotivated rule is a defect-in-waiting.
- When you **answer** a design question, reason **from the intent and reconstruct the
  rule** rather than reciting it. If a recited rule and the intent disagree, the rule
  is the suspect (consistent with *the code is a defect report*).
- **Theme is one engine of motivation:** a rule that falls out of the fiction
  (diegetic) is re-derivable from the world. Prefer diegetic rules where you can.
- **Never simplify a rule by severing it from its WHY** — that trades a motivated rule
  for an arbitrary one. It is a regression even if the text got shorter.

### What is authoritative (and what is not)

| Question you're answering                                     | Source of truth                    | Authoritative?                          |
| ------------------------------------------------------------- | ---------------------------------- | --------------------------------------- |
| *Why* is it designed this way? What is the intent?            | **Charter** (`canon/1-charter.md`) | ✅ yes — for intent                      |
| What does a rule / keyword *mean*? How does resolution work?  | **Spec** (`canon/2-spec/`)         | ✅ yes — for mechanics                   |
| What pieces *exist*? Their values, decks, scenarios, numbers? | **`booklet.ron`** (print master)   | ✅ yes — for components & numbers        |
| What does the running game *currently do*?                    | the code (`crates/deckbound/`)     | ❌ no — implementation; may have defects |
| Exploratory reasoning, history, alternatives                  | `notes/` notes                     | ❌ no — frozen, often stale              |

The Charter, the Spec, and `booklet.ron` are **co-equal** — they don't outrank
each other, they own **different questions**. The code and the notes are **never**
the tie-breaker.

### When two things disagree

Resolve contradictions by **which question is being asked**, then apply the rule:

- **Spec says one thing, the code does another** → **the code is the defect.**
  Report it. Do **not** edit the Spec to match the code. Reconcile by fixing the
  code — or, if the human decides the *intent* changed, by consciously amending
  the Spec first (see change discipline below).
- **A prose number disagrees with `booklet.ron`** → **`booklet.ron` wins.**
  Numbers written in prose are illustrative only; the print master is real.
- **A `notes/` note disagrees with the Spec or Charter** → **the note is stale.**
  Trust the Spec / Charter. Flag the note for a "superseded" banner; do not act on
  it.
- **`booklet.ron` references something the Spec doesn't define** (a keyword,
  effect, trait, or a card/weapon name that doesn't exist) → that is a **defect in
  the data** (the print master isn't shippable). Flag it; don't paper over it.
- **You can't find an authority for the question** → **ask the human.** Do not
  guess, and do not treat the code or a note as the answer.

### How to make a change — match the change to its owner

Different kinds of change have different owners. **Do not blur them.**

| You want to change…                          | Edit                              | Do NOT touch               | Extra rule                                                |
| -------------------------------------------- | --------------------------------- | -------------------------- | --------------------------------------------------------- |
| A **rule / keyword meaning / procedure**     | the **Spec**                      | numbers in `booklet.ron`   | write it as RULE / WHY / GUARANTEES (see Spec)            |
| A **number / balance value**                 | **`booklet.ron`** only            | the Spec                   | **propose to the human; the human tunes.** You seed only. |
| **Add a card** that reuses existing keywords | **`booklet.ron`** only            | the Spec                   | pure data — no rule change                                |
| **Add a brand-new keyword / mechanic**       | the **Spec first**, then the code | —                          | one rulebook change; then it's free vocabulary in data    |
| **Card rules text** (what prints on a card)  | **nothing — it's generated**      | never hand-write card text | compose Spec keywords; text comes from their manual lines |

**The non-negotiable discipline:** a mechanics change is **not done** until the
**Spec is updated in the same change**, and the code is then checked against the
Spec. If you change behavior and leave the Spec untouched, you have created the
exact drift this document exists to prevent.

**AI proposes, human disposes — on numbers.** You may *suggest* values (that's
your seeding role), but you never decide them, never silently re-tune them to make
a rule change "work," and never edit the Spec to chase a number. Number = human.
Rule = Spec. Keep them apart.

### Classify every change you propose — intent vs. mechanics

The human specifically wants to know, for any suggestion: **was the intention
wrong, or were the mechanics wrong?** You must answer that explicitly. Every Spec
rule carries a **WHY** (its intent) and **GUARANTEES** (the invariants it must
preserve). Use them to classify, and **say which case you're in**:

1. **Breaks the RULE, preserves WHY + GUARANTEES** → *"The mechanic is wrong.
   Here is a fix that keeps the intent."* — safe to propose.
2. **Keeps the RULE, but violates a GUARANTEE** → *"This is mechanically legal but
   breaks invariant X."* — flag it; usually don't do it.
3. **Fights the WHY / a Charter north star** → *"This changes the design intent.
   That is the human's call. I will not do it silently."* — stop and ask.

Never collapse case 3 into case 1. Changing what the game is *trying to do* is
always the human's decision, surfaced explicitly — never smuggled in as a
"mechanics fix."

### Hard "never" list

- **Never** edit the Charter or a Spec's WHY/GUARANTEES to make a problem go away.
  Surface the conflict instead.
- **Never** treat the code or a `notes/` note as a source of truth.
- **Never** hand-write rules text onto a card.
- **Never** change a number and a rule in the same breath without telling the
  human you've done both.
- **Never** answer a design question from memory of the old stance system
  (Strike / Block / Evade / Scheme, banked momentum). That system is
  **superseded** (see below). The current tactical core is the **Duel**
  (Marshal / Unleash / Overwhelm / Parry, per-duel Edge).

---

## The sources of truth, in detail

### Charter — *the intent*

**`canon/1-charter.md`** (the ten north stars). Answers *why*. Changes to it
are deliberate design acts, made on purpose and rarely. Every Spec rule should
trace back to a Charter north star.

### Spec — *the mechanics*

**`canon/2-spec/`**. The canonical, precise statement of how every system works. Written
as **RULE / WHY / GUARANTEES** so that mechanics and their intent live together
and cannot drift apart, and so the intent-vs-mechanics classification above is
always possible. The Spec owns the **vocabulary** (what every keyword means) and
the **procedures** (how a round/duel/hit resolves). It does **not** own numbers.

### `booklet.ron` — *the print master (components & numbers)*

**`crates/deckbound/data/booklet.ron`**. The official list of **every card,
trait, actor, prebuilt deck, and scenario**, with their **values** — what you
would send to the printer today. Co-equal with the Spec; owns a different
question. Numbers are **AI-seeded, human-tuned**. It composes **only** Spec-defined
keywords; it never carries hand-written rules text.

- **The seam:** a printed card = *values from `booklet.ron`* × *rules text from the
  Spec's keyword manual lines*. Neither alone is the card.
- **The printability check:** every keyword / effect / trait / card-reference in
  `booklet.ron` must resolve to a Spec definition or a catalog entry. A dangling
  reference means the print master isn't shippable — a defect.
- **Human-readable sheets** (card lists, deck sheets, scenario setup) are
  **generated projections** of `booklet.ron`, never hand-maintained prose.

### Code — *the implementation (not a source of truth)*

**`crates/deckbound/`**. Interprets the Spec's rules and `booklet.ron`'s data. It
is authoritative for **nothing**: where it disagrees with the Spec or the print
master, the **code is the defect**. (Decision recorded by the designer: *Spec is
intent; code is a defect report.*)

### `notes/` notes — *frozen exploration (not authoritative)*

The ~35 documents under **`notes/`** are the working-out of the design:
reasoning, alternatives, history. They are **not** a source of truth, they were
never meant to be, and several are now **stale**. They get exactly one kind of
edit: a "**Superseded by `canon/2-spec/…`**" banner. Do not act on them over the Spec.

---

## Reading order

1. **This document** — how truth and behavior work.
2. **Charter** (`canon/1-charter.md`) — the intent.
3. **Spec** (`canon/2-spec/`) — the mechanics.
4. **`booklet.ron`** — the actual pieces and numbers.

Read `notes/` notes only as *background*, and only after the above, knowing they
are non-authoritative.

---

## Supersession protocol

When a system changes:

1. **Edit the Spec first**, in the RULE / WHY / GUARANTEES format. The change is
   not "real" until this is done.
2. **Update `booklet.ron`** if components/numbers are affected (human tunes
   numbers).
3. **Regenerate** any human-readable component sheets from `booklet.ron`.
4. **Update the code** and check it against the Spec; any gap is a code defect to
   fix.
5. **Banner the superseded `notes/` note(s)** with a pointer to the new Spec
   section. Do **not** rewrite the note to match — it is frozen history.

A change that updates code or data but not the Spec is **incomplete by
definition.**

---

## Current transitional state (read before trusting older notes)

The Spec is **being written** and does not yet cover every system. Until a system
has a Spec section, treat authority as follows:

- **Tactical core (the Duel):** authoritative design is **`notes/the-duel.md`**
  plus this Spec's Duel section. The older stance system — **Strike / Block /
  Evade / Scheme** and **banked momentum** — is **superseded** and must not be
  used. Notes still describing it (`rulebook.md`, `keywords.md`,
  `mind-and-stances.md`, and parts of `decision-making.md`,
  `coordination-and-interruption.md`, `combat.md`, `zones.md`) are **stale on this
  point**.
- **Stats / defense model:** **`notes/stats.md`** and **`notes/form-and-defeat.md`**
  are current (three **channels** — Body / Mind / Spirit; cut → bar → pool; one health track).
  *(The notes call the channels "aspects"; "aspect" is now reserved for the deferred combo layer.)*
- **Everything else:** trust `booklet.ron` for components/numbers and the
  most-recently-reconciled note for shape — but if it predates the Duel switch and
  touches the tactical layer, **distrust it** and ask.

When in doubt, the safe answer is always: **the Spec if it exists, else the
Charter for intent and `booklet.ron` for pieces — and ask the human for the rest.**
