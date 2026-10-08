# FINDINGS — restaurant-nemesis-r3v2 (design stage)

Entries in the `docs/agent-practices.md` §2.5 / make-game §C shape. The
implement stage carries these into `games/restaurant-nemesis-r3v2/FINDINGS.md`
(continuing the G-number sequence) and the PR body.

## D-1 — `judge_frame`'s worked example names a `Flat` mapping the Reference never defines

- **Class**: doc gap (an example references an item the document does not list).
- **Doing**: designing the screen's mapping from design units to world units, from `docs/api/jidousha-ui.md`, so the floors and `judge_frame` could be stated for the implementer.
- **Expected**: every name an example uses to be in the Reference, or the example to say it is the reader's own type.
- **Happened**: the `judge_frame` entry's example calls `judge_frame(&screen(), &frame, recorder.font_texture(), &Flat, view)`. `Flat` appears nowhere else in the five documents and is not in the `jidousha::ui` Reference; the `Mapping` entry's own example defines a `UiMap` instead.
- **What I did**: on the document's authority I assumed `Flat` is a kit item for a moment, then searched the Reference, found nothing, and had the design define its own identity `Mapping` (`Flat` in `src/screen.rs`, `to_world(ui) = ui`, `scale() = 1.0`). The implementer should not look for it in the kit.
- **Owner**: `docs/api/jidousha-ui.md` (the `judge_frame` example in `tools/api-doc/` or the facade's doc comment).

## D-2 — `judge_panel`'s `overlays` parameter does not say what its two strings are

- **Class**: doc gap (a parameter whose meaning is only inferable from one example).
- **Doing**: naming the nemesis card as an overlay so the "two overlays in one frame" floor applies to it.
- **Expected**: the `judge_panel` entry to say what `overlays: &[(&str, &str)]` holds.
- **Happened**: the signature says `overlays: &[(&str, &str)]`; the prose says overlays are "counted by the title row each one draws"; the only example is `DRAWERS: &[(&str, &str)] = &[("FEED", "FEED - what happened"), ...]`. The design reads the pair as (a short name, the exact title-row text) and fixes the card's title row to the constant string `NEMESIS CARD` so the second element is a literal.
- **What I did**: designed to the inference and told the implementer to stage the card and assert the floor by name, so a wrong reading fails a check rather than passing silently.
- **Owner**: `docs/api/jidousha-ui.md` (`judge_panel`'s doc comment).
