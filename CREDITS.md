# Credits

Third-party work this repository redistributes, and the licence each one
carries. Everything here is **committed**, not fetched at build time: an asset
the build downloads is an asset the store can lie about, and a licence that
lives on somebody else's server is a licence this repository is not honouring
(assets.md §3, ADR-0042).

The engine's own code is this project's. This file is for the things that
travel with it and are not.

## Fonts

### Fira Sans — `assets/fonts/`

`FiraSans-Regular.ttf` and `FiraSans-Bold.ttf`, the two weights the engine's
`examples/text` sets its specimen sheet in and the family a game reaches for
when it wants real type.

- **Copyright** © 2012–2015, The Mozilla Foundation and Telefónica S.A.
- **Licence** SIL Open Font License, Version 1.1 — the full text is committed
  beside the files at `assets/fonts/OFL.txt`, which is what the licence
  requires of a redistribution.
- **Source** the [Google Fonts](https://github.com/google/fonts) repository,
  `ofl/firasans/`. The files are the upstream ones, byte for byte: nothing here
  subsets, re-hints or renames them, so the licence text above describes
  exactly what is in the tree.
- **Why this family** it is legible at the sizes a prototype's UI actually uses,
  covers ASCII and Latin-1 completely, and ships as *static* TTFs. That last
  one is not a preference: the rasterizer this engine uses cannot set the axes
  of a variable font, so a variable-only family would give one weight and call
  it two (ADR-0042).

Reserved Font Name: none. The OFL's Reserved Font Name clause is not invoked by
this family's copyright line, so no renaming obligation attaches to it.

## Rust dependencies

Every crate in the dependency graph carries its own licence, and
`tools/dep-count` lists the graph. They are not vendored here — cargo fetches
them — so this file does not restate them. The one worth naming because a
decision was made about it is **ab_glyph** (Apache-2.0), which parses and
rasterizes the outlines above; ADR-0042 records why it rather than another.
