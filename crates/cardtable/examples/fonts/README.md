# Showcase fonts (drop-in)

The `fontshow` example (`cargo run -p cardtable --example fontshow`) renders the same card-like sample
text in each candidate typeface. It loads any `.ttf` it finds here at the **exact filenames** below.
All candidates are free and **SIL Open Font License (OFL)**, so they're safe to bundle later too.

Drop the regular weight here with these names, then re-run:

| Candidate              | Save as                              | Where to get it (free / OFL)                          |
| ---------------------- | ------------------------------------ | ----------------------------------------------------- |
| Atkinson Hyperlegible  | `AtkinsonHyperlegible-Regular.ttf`   | fonts.google.com/specimen/Atkinson+Hyperlegible       |
| IBM Plex Sans          | `IBMPlexSans-Regular.ttf`            | fonts.google.com/specimen/IBM+Plex+Sans               |
| IBM Plex Mono          | `IBMPlexMono-Regular.ttf`            | fonts.google.com/specimen/IBM+Plex+Mono               |
| Source Sans 3          | `SourceSans3-Regular.ttf`            | fonts.google.com/specimen/Source+Sans+3               |
| Figtree                | `Figtree-Regular.ttf`                | fonts.google.com/specimen/Figtree                     |
| Nunito Sans            | `NunitoSans-Regular.ttf`             | fonts.google.com/specimen/Nunito+Sans                 |

Already available without dropping anything:

- **Inter** — loaded from `crates/cardtable/fonts/Inter-Regular.ttf` (already bundled).
- **IBM Plex Mono row** — falls back to `C:/Windows/Fonts/CascadiaMono.ttf` (an OFL mono usually
  installed on Windows) so you can see number alignment until you add Plex Mono.

This folder is for the **dev showcase only**. Whatever font we pick gets bundled into the `cardtable`
crate proper (via `include_bytes!`, like Inter is now) so it ships embedded — see the note on web below.
Keep each font's `OFL.txt` / license alongside the `.ttf` if you commit them.
