# The generic screen-description file

Status: staged design, 2026-07-22. NOT scheduled yet (user: "not necessarily now") - captured so the
requirements are pinned while the first cut (`ui-scene.txt`, fight-only) is fresh.

## The requirement (user-stated)

A screen description file that works on **every screen**, not just the fight - with **abstraction
layers in place so that nothing is rendered, on any screen, that is not also described in the
file**. The description is the guarantee, not a best-effort log.

- **No visual artifacts.** Text wrapping, font metrics, pixel jitter - out of scope. What matters is
  the text we ATTEMPTED to render, in full.
- **Bounding boxes.** Cards have well-defined bounding boxes, so the file must carry them - and with
  them, **overlap is detectable** from the file alone.
- **Effects, not animations.** The file does not record animation frames (the marching dots of the
  target ring); it records **which effect is applied to which card** ("targeting ring: The Wall,
  The Sniper"). The animation is presentation; the *assignment* of the effect is state.

## What exists today (the seeds)

- `ui-scene.txt` (2026-07-22) - the current-screen snapshot, **fight scene only**: every tile with
  its named attention state (which already satisfies the effects rule: `TARGETED (ringed...)` is the
  effect-assignment, no frames), every choice with its status, controls, prompt, log. Rewritten on
  change.
- `ui-state.log` - the append history: view changes, the settled layout of each card (position,
  size, zoom) per view, every pick/drop/click. Already the overlap-audit channel (`grep ERROR
  ui-state.log`).
- The geometry tenet: `cardtable-model` owns ALL layout in integer 2-space (footprints,
  `place_clear`), renderer converts at `Val::Px` - so bounding boxes are already model facts, not
  render facts. This is what makes "detect overlap from the file" possible without describing
  pixels.

## The design direction (the abstraction layer)

The guarantee "nothing on screen that is not in the file" cannot be achieved by logging harder - a
log observes the renderer and can miss what it does not know to look for. It is achieved by
**inversion**: the renderer draws FROM a single description value, and the file is a serialization
of that same value. One producer, two consumers - the screen and the file - so they cannot
disagree by construction (the same shape as `narrate`: one storyteller, every surface reads it).

Sketch:

- A `ScreenDescription` (cardtable-model, render-free): the view identity (which zone / the felt /
  the modal scene), and a flat list of **elements**: `{ id, kind (card / pile chip / control /
  overlay text), text lines attempted, bbox (integer 2-space), effects: [named effect] }`.
- The renderer's draw systems consume ONLY this value. Anything the renderer wants to draw must
  first exist as an element - which is exactly the abstraction-layer discipline requested.
- The file: serialize the value on change (the `ui-scene.txt` discipline, generalized). Overlap
  detection is then a pure function over the element bboxes - runnable in tests, not just eyeballs.
- Animations stay renderer-side, keyed by the named effects (`targeting-ring`, marching arrows), so
  the file records the assignment and never the frames.

## The migration seam

Today the felt path renders from `Board` + layout queries scattered through `cardtable`, and the
modal path from `Scene`. The migration is to make both paths produce a `ScreenDescription` first
(felt: from the existing integer-2-space layout model; modal: from `Scene` - `ui-scene.txt`'s
serializer is the prototype of that half), then move drawing over to it view by view, deleting each
direct Board-to-draw read as it goes. The moment the last direct read dies, the guarantee holds.
