# ADR-0046: The UI kit — extracted from a game by the rule of three, and documented as a fifth surface

Status: accepted · 2026-10-03 · supersedes nothing · extends ADR-0025 (a fifth document on its rule)

> **A crate, `jidousha-ui`, reached as `jidousha::ui`, holds the parts of a
> game's chrome that one game produced three or more times: a screen as data,
> measured text, the one-value chip and toggle, the data-registered class
> table and its feed, the meters-to-faces mechanism, and the readability
> floors. Its spec is an audit's report plus the owner's verdict on it, and two
> parts were left out with their triggers recorded.**

## Context

`games/ninjo` built a UI grammar — panels of text and icons over a map,
drawers, chips whose explanation opens on a tap, a feed of what happened, a
row of meters — and the wave-1 exemplar audit (PR #115, `SANITATION.md`'s
exemplar-audit type, dispatched by `FINDINGS.md` G-031) judged the grammar
worth copying. Two things were about to copy it: the game-repo template W3
points authors at the engine's examples, and `games/keifu` was building beside
it. Copying it meant re-typing it, and re-typing a grammar is how two games
come to disagree about one thing.

The audit's report §3 listed thirteen parts (P-1 to P-13) with, for each, the
call sites, the generic half, the game's half, and a recommendation. The owner
decided it on 2026-10-03: extract P-2, P-5, P-7, P-8's generic half, P-9 with
`clipped`, P-10, P-11, P-12 and P-13; document P-1, P-4 and P-8's conductor
as patterns rather than types; defer P-3 and P-6 with a recorded trigger each.

The audit's closing judgement is the kit's reason for carrying types rather
than only helpers: **every parallel-state defect in that game was a surface
that grew a flag beside an `Option`** (G-017, G-023, G-026, G-027, G-060,
G-063) — two representations of one fact, kept in step by a comment and broken
by the next wave. A type cannot be left off a clearing list.

The game document, `docs/api/jidousha-api.md`, sat at ~24.5k of its 25k budget
when the kit landed. ADR-0025's rule for a full budget is a seam, not a bigger
number, and a screen is built at a different moment from the simulation it
shows.

## Decision

1. **The kit is its own crate**, `crates/jidousha-ui`, depending on
   `jidousha-core` and `jidousha-render-core` and on nothing new. The facade
   re-exports it as `jidousha::ui`, a module beside `testing` and **not in the
   prelude**: a game with no chrome never names it. Games reach it only through
   the facade (`tools/check-game-deps` is the proof, unchanged). A crate rather
   than a module of the facade because the facade carries no code of its own
   by its invariant, and because `tools/gen-api-doc` and `tools/check-api-coverage`
   read the surface from `crates/jidousha-*/src`.

2. **What is in it**, by the audit's numbering:
   - **P-8's substrate**: `Panel<Icon>`, `TextRun` (a position, a string and
     its `TextStyle`), `IconRun<Icon>`, the `Icon` trait a game's art enum
     implements, the `Mapping` trait chrome rides the camera through, and
     `Panel::draw`. Two spaces — chrome in design units, labels in world units.
   - **P-8's generic floors**: `Floors` (the game's numbers), `Breach`,
     `judge_panel` (text ≥ floor, inside the rect, not across a control it
     does not label, chrome-vs-chrome on one band, map labels apart, integer
     icon scale, at most one overlay by title), `judge_frame` (every row and
     icon the panel named, on the frame), `frame_text_floor`, `glyph_run`,
     `inside`.
   - **P-9 with `clipped`**: `wrap`, `clipped(&style, ..)`; **P-10**: `Cell` and
     `centered`; **P-11**: `Panel::hint`; **P-12**: `Panel::lifted`; **P-13**:
     `toggle`.
   - **P-2**: `Chip<Id> { lit: Option<Id> }` — `toggle`, `showing`, `shut`, and
     `line`, which derives the explanation from the game's rows at draw time.
   - **P-5**: `Mode`, `ClassSpec<Class, Icon>`, `find_class`, `Attention<Class,
     Icon>` (a config that is simulation state), `Pause<Class>`, `FeedEntry`,
     `feed` (a view over the game's log), `reason_line` (the one sentence), and
     `class_faults`.
   - **P-7**: `MeterSpec<Icon, Ask>`, `faces`, `count`, `meter_faults`.

3. **What stays in the game**: every formatter, every sort key, `where_drawn`,
   the four meter predicates and the lens they ask, the shot judges, the
   conductor, the offsets, the palette and the bands. The kit's own crate
   invariant states it: nothing in it decides anything a game simulates.

4. **What was deliberately left out, and what brings it in.**
   - **P-3, the one-drawer discipline** (an `Overlay` trait with `all()`, a
     handle and a title, and the one `Option` the render, the click routing
     and the control set each `match` on). One instance exists. **Trigger: a
     second instance.** The first candidate is ninjo's left column — the
     `Option<Column>` collapse the audit's Q3 proposes — which rides the next
     ninjo UI session, not this one. Until then the at-most-one-overlay floor
     carries the discipline as a check.
   - **P-6, timer bars against a world-time deadline** (a `0..=1` share with
     an ember inside its last day). Two call sites, below the rule.
     **Trigger: the next wave that adds a deadline.**
   - **P-1 and P-4** are documented as patterns in the kit's guide and not
     typed: a `Vec` with one ordering function and per-row equality
     assertions, and one `Option` selection every door writes. A `RowList<T>`
     would add nothing over the `Vec`.

5. **The kit's reference is a fifth document**, `docs/api/jidousha-ui.md`,
   on ADR-0025's rule with its own budget (11k, sized just above what it cost
   on landing) and the game document's vocabulary rule. The game document
   signposts it twice. `jidousha-testing.md` gains the conductor pattern
   (P-8's photo schedule), which the engine already has `FrameRecorder` for.

6. **How a part gets in**, so the next session knows: a part produced
   **three or more times** by a game is a promotion candidate; promotion is
   proposed by an exemplar audit's report and decided by the owner; below
   three, the trigger is recorded and the part waits. `docs/conventions.md`
   §Extraction states it.

7. **The swap is behaviour-free by construction and by proof.** ninjo consumes
   the kit and its own copies are deleted; `tools/verify ninjo` and `tools/verify
   keifu` are byte-identical before and after, reports (two wall-clock fields
   stripped), stdout and PNGs alike. One deliberately staged violation per
   moved floor, judged through ninjo's own wrappers, shows each floor still
   bites from its new home (`floors::moved_floors_bite`).

## Consequences

- A second game with chrome imports `jidousha::ui` beside the prelude and
  writes its bands, its palette, its formatters and its floors' numbers; the
  shape — and the one-value discipline — comes with the import.
- The generic floors' thresholds live in the game (`Floors`); the kit holds
  no number. ninjo's offsets stay ninjo's: unifying the button baselines the
  audit found inconsistent (10, 11, 12, 8) would move pixels, which this
  session may not, so `centered` is the rule and the offsets are the game's
  constants until a ninjo UI session chooses to move them.
- `TextRun` carries a `TextStyle` rather than a size, a colour and a band,
  so a game with a loaded typeface (ADR-0042) measures its rows in that face;
  ninjo binds its bands in `ui::row`, `ui::over` and `ui::icon`.
- The kit's `Attention::mode` panics for a class with no row, where ninjo's
  silently read the first row; `class_faults` and every game's vocabulary
  check catch the fault before a world exists, and the path is unreachable in
  a checked game.
- `tools/gen-api-doc`, `tools/check-api-coverage` and `tools/check-api-prose`
  know the module: every kit item has exactly one entry, in the kit document,
  and is shown in `examples/ui_kit.rs`.

## Alternatives rejected

- **A module of the facade crate.** The facade is a curation and carries no
  code; the generator and the coverage check would not have seen it.
- **Items in the prelude.** A game of shapes and a score would import a chip
  it never draws, and the game document had no budget for the reference.
- **Extracting P-3 from one instance** because it is already understood. The
  rule is three; one instance generalises nothing, and the second — the
  column collapse — is a known, scheduled piece of work.
- **A `Checks` accumulator in the kit.** Every game has its own; the floors
  hand back `Breach`es and the game keeps them the way it keeps everything else.
- **Unifying ninjo's baselines on the way past.** A transcript change inside a
  behaviour-free swap, which is the thing the fence exists to refuse.
