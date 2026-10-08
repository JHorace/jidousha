# FINDINGS — stack-card-game-r3v2 (design stage)

Entries in the `docs/internal/e0-findings.md` shape: Class · Doing · Expected ·
Happened · What I did · Owner. The implement stage appends its own below and
copies the game's findings into `games/stack-card-game-r3v2/FINDINGS.md` per
make-game §C.

## 1. `Flat` is used by the UI doc and defined nowhere in `docs/api/`

- Class: doc misled (an example names an item the surface does not export).
- Doing: designing the frame check for the kit-built screen; reading
  `docs/api/jidousha-ui.md`, `judge_frame`'s example.
- Expected: every name in a Reference example to be either the kit's or the
  prelude's, or defined in the example itself.
- Happened: `judge_frame(&screen(), &frame, recorder.font_texture(), &Flat,
  view)` passes `&Flat`, and `Flat` appears in no Reference entry of any of the
  five API files and in no example in the same entry. A reader takes it for a
  kit-provided identity `Mapping` and writes `use jidousha::ui::Flat`, which
  does not exist.
- What I did (on the doc's authority): nothing was built on it at design
  stage; DESIGN.md tells the implementer to define its own identity `Mapping`
  (`to_world(ui) = ui`, `scale() = 1.0`) in `src/screen.rs` and names this
  entry. If `jidousha::ui::Flat` turns out to exist and is only missing from
  the generated doc, the implementer uses it and says so in Deviations.
- Owner: `docs/api/jidousha-ui.md` (generated from `tools/api-doc/`; the
  `judge_frame` doc example should define `Flat` inline or the kit should
  export it).

## Design-stage reading note

Zero other findings: the five API files answered every question this design
asked (the kit's `Panel`/`judge_panel`/`judge_frame`/`glyph_run` for the
surfaces, `SnapshotBuilder` for a key-driven check player, `Rng::below` for
the shuffle, the staging-before-tick-1 pattern for the decision gates).
