# Showcase fonts (drop-in)

The `fontshow` example (`cargo run -p cardtable --example fontshow`) renders the same card-like sample
text in each candidate typeface. It loads any `.ttf` it finds here at the **exact filenames** below.
All candidates are free and **SIL Open Font License (OFL)**, so they're safe to bundle later too.

Drop the regular weight here with these names, then re-run:

| Candidate             | Save as                            | Where to get it (free / OFL)                    |
| --------------------- | ---------------------------------- | ----------------------------------------------- |
| Atkinson Hyperlegible | `AtkinsonHyperlegible-Regular.ttf` | fonts.google.com/specimen/Atkinson+Hyperlegible |
| IBM Plex Sans         | `IBMPlexSans-Regular.ttf`          | fonts.google.com/specimen/IBM+Plex+Sans         |
| IBM Plex Mono         | `IBMPlexMono-Regular.ttf`          | fonts.google.com/specimen/IBM+Plex+Mono         |
| Source Sans 3         | `SourceSans3-Regular.ttf`          | fonts.google.com/specimen/Source+Sans+3         |
| Figtree               | `Figtree-Regular.ttf`              | fonts.google.com/specimen/Figtree               |
| Nunito Sans           | `NunitoSans-Regular.ttf`           | fonts.google.com/specimen/Nunito+Sans           |

The `.ttf`s here are **git-ignored** (see root `.gitignore`) — throwaway evaluation assets, re-fetchable
from the sources above. The six candidates were fetched from the `google/fonts` repo:
`https://raw.githubusercontent.com/google/fonts/main/ofl/<dir>/<File>.ttf` (the Plex/Source/Figtree/
Nunito ones are **variable** fonts, saved under a `-Regular.ttf` name; they render at their default
instance). Inter is already bundled at `crates/cardtable/fonts/Inter-Regular.ttf`.

This folder is for the **dev showcase only**. Whatever font we pick gets vendored into the `cardtable`
crate proper (via `include_bytes!`, like Inter) so it ships embedded — and *then* we add that one
font's `OFL.txt` alongside it. (Nothing here is committed, so no license files are needed yet.)
