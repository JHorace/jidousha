# UI kit — design and contracts

Status: **living doc for `jidousha-ui`; landed whole with ADR-0046.** Owns:
the crate, its facade module `jidousha::ui`, the fifth API document
(`docs/api/jidousha-ui.md`, generated from `tools/api-doc/ui.md` and the
crate) and `examples/ui_kit.rs`. Does NOT own: any game's bands, palette,
layout, formatters or floors' numbers — those are the game's, and the kit
holds no number of its own.

Inherits: the draw vocabulary and `TextStyle` (renderer.md §2, §6), the
facade's curation rule (public-api.md §1), the documentation split
(ADR-0025), the extraction rule (conventions §Extraction).

---

## 1. What it does

A screen hands back a `Panel` — every row of text and every icon, with its
position — and three readers take the same panel: `Panel::draw` turns it into
quads through a `Mapping`, `judge_panel` judges it against the game's `Floors`,
and `judge_frame` finds every row and icon on the recorded frame. Around that
substrate sit the one-value carriers (`Chip`, `toggle`, the at-most-one-overlay
floor), measured text (`wrap`, `clipped`, `Cell`, `centered`, `Panel::hint`),
the attention architecture (`ClassSpec`, `Attention`, `Pause`, `feed`,
`reason_line`) and the meters mechanism (`MeterSpec`, `faces`, `count`).

Every part was produced three or more times by `games/ninjo` and promoted by
the wave-1 exemplar audit's report (PR #115 §3) and the owner's verdict; what
was left out, and its trigger, is in ADR-0046 §4.

## 2. Core data flow

```
game screen fn ──► Panel<Art> ──┬─► Panel::draw(ctx, &map, sprite_of)   (quads)
   (lens, flow)                 ├─► judge_panel(&panel, &floors, controls, overlays) ─► Vec<Breach>
                                └─► judge_frame(&panel, &frame, font, &map, view)    ─► Vec<Breach>
game event log ──► feed(classes, &attention, show_ignored, cap) ─► Vec<FeedEntry>
game roll ───────► faces(roll, |who| (spec.asks)(..)) ─► Vec<(who, reason)>
```

The game owns the context on both ends: its `Icon` enum and the closure that
turns an `IconRun` into a `Sprite`; its `Mapping` (ninjo's `camera::UiMap`);
its `Floors`; its `Checks`-shaped accumulator the breaches go into.

## 3. Invariants

- **CONTRACT: the kit decides nothing a game simulates.** A panel is
  strings and icons; a chip is which id is lit; a feed is indices into the
  game's log; a meter's count is the length of the list it opens into. Every
  sentence and number is the game's.
- **CONTRACT: no number of its own.** Thresholds come in through `Floors`;
  bands come in on the `TextStyle`/`IconRun`; leading is an argument. A kit
  constant would be a second place a game's number lives.
- **One value, never a flag beside it.** `Chip<Id>` is one `Option<Id>`;
  `toggle` is the one tap idiom; the overlay floor refuses two overlays in one
  frame by title. These carry the audit's closing judgement (ADR-0046).
- **A breach is asserted by name.** `Breach::what` is a stable sentence a check
  stages a violation against; `detail` carries the numbers. A floor passes
  silently.
- **`Attention::mode` panics for a class with no row** (a contract violation,
  core.md §9 shape); `class_faults` reports a malformed table before any
  world exists.
- **Behaviour-free on landing.** ninjo's `tools/verify` output was
  byte-identical before and after the swap (ADR-0046 §7); any future change
  to a floor's arithmetic is a ninjo transcript change and is reviewed as one.

## 4. How to test it

- `cargo test -p jidousha-ui`: doctests on every public item, plus
  `tests/floors_bite.rs` (one staged violation per floor, by name, and the
  frame half through a real `FrameRecorder`) and `tests/one_value.rs` (the
  chip's line derived at draw time, the feed as a view, a meter's set against
  the act). `tools/test` runs them with the workspace.
- `examples/ui_kit.rs` names every item and is run by `tools/test`.
- `games/ninjo` is the living exemplar: `floors::floors_bite` and
  `floors::moved_floors_bite` stage every moved floor through ninjo's own
  wrappers, silently, in every `tools/verify ninjo`.
- `tools/gen-api-doc --check`, `tools/check-api-coverage` and
  `tools/check-api-prose` hold the document: one entry per item, every item in
  an example, every prose block compiling.

## 5. Known sharp edges

- **A game's clip style is the game's.** ninjo clips every row at its small
  size, including rows drawn at its heading size; `Cell::run` clips at the
  row's own style. ninjo therefore keeps a one-line `panels::clipped` binding
  and its sites unchanged — adopting `Cell` at a heading row would move the clip
  point, a transcript change.
- **`judge_panel`'s overlay floor counts by title.** An overlay whose title
  row is clipped or absent is invisible to it; the game's own floor over the
  title row (ninjo's drawer floors) is what keeps the title there.
- **`frame_text_floor` measures world units.** On a frame drawn at a zoom the
  chrome's glyphs scale with the mapping and map labels do not; state the floor
  for the frame you judge (`examples/ui_kit.rs` shows both answers).
- **`feed` wants a double-ended, exact-size iterator** of classes — a slice
  iterator mapped, which is what a log gives; a filtered iterator does not
  qualify, and the compiler says so.
