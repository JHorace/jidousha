# Call of Cthulhu (r3v1) — findings

What the documents cost this game, in the shape `docs/internal/e0-findings.md`
uses. G-numbers continue the sequence across games (the largest heading in
`games/*/FINDINGS.md` at this branch's base was G-069; other rounds in flight
may take the same numbers — G-068's fork, unfixed).

**Reading discipline.** Read: `CLAUDE.md`, the `make-game` skill, the yakin
task specs, all five `docs/api/` documents (api: Quickstart, Concepts and the
Reference entries used; testing, controllers, capture, ui whole or nearly),
and from `crates/jidousha/examples/` the `prototype_kit` files `checks.rs`,
`capture.rs` and the head of `verify.rs`. `games/keifu/` only for its
manifest, a `mutants/` list's shape, and the G-heading grep. **Engine source
(`crates/*/src/`), `docs/internal/`, `docs/adr/`: not opened.** No other
round's call-of-cthulhu output opened (comparison hygiene).

---

### G-070 — Concepts names `just_pressed(PointerButton::Primary)` as if `Input` had it

Class: doc misled · Session: call-of-cthulhu-r3v1 · Owner: `docs/api/jidousha-api.md`, Concepts (touch paragraph)

**Doing:** routing a tap on an answer row to the same command a digit key sends.

**Expected:** "`just_pressed(PointerButton::Primary)` is a tap" — read beside
`input.held(Key::D)` in the Quickstart — to be a method on `Input`.

**Happened:** `Input::just_pressed` takes a `Key`; the button form lives on
`PointerState` (`input.pointer().just_pressed(..)`), which the Reference says
and the Concepts sentence does not. **What I did:** wrote
`input.just_pressed(PointerButton::Primary)` on the paragraph's authority; the
compiler refused it; one edit. Cheap, but the sentence is the only place the
tap idiom is spelled.

**Fix:** spell the receiver in that sentence: `input.pointer().just_pressed(PointerButton::Primary)`.

### G-071 — a text game's decision check has no frame-side "what does it say"

Class: doc gap · Session: call-of-cthulhu-r3v1 · Owner: `docs/api/jidousha-testing.md` / `jidousha-ui.md`

**Doing:** the handoff's row-1 check — "the question, options, sanity and
known lore are in the transcript before answering".

**Expected:** a way to read the words a frame drew back out of it.

**Happened:** `FrameRecord::transcript()` is one line per quad extent; a glyph
quad does not say which character it is, so no frame-side reading recovers a
sentence. The kit's answer is indirect: the panel names the strings, and
`judge_frame` proves every panel row is on the frame at the right width.
**What I did:** asserted the facts against `Panel::all_strings()` (joined, so
a wrapped paragraph reads whole) and required `judge_frame` clean on the same
frame — two halves the documents give separately and never put together for
"is this sentence on screen".

**Fix:** one sentence in the ui doc's *The floors*: "is this fact on screen"
is `all_strings` for the words plus `judge_frame` for the frame.

### G-072 — tapping a chrome row needs the inverse of `Mapping`, which the kit does not carry

Class: doc gap · Session: call-of-cthulhu-r3v1 · Owner: `docs/api/jidousha-ui.md`, *A screen is data first*

**Doing:** hit-testing a tap against the numbered rows, which live in design units.

**Expected:** the kit's `Mapping` to go both ways, or the doc to say how the
pointer reaches design space.

**Happened:** `Mapping` has `to_world` and `scale` only. **What I did:** gave
the game's `UiMap` a `to_design` (`(world - origin) / scale`) and fed it
`camera.screen_to_world(pointer)`; verify taps a row's centre through
`world_to_screen(map.to_world(..))` and requires that row chosen.

**Fix:** the doc's `UiMap` sample could carry the inverse, or *Two patterns
the kit does not type* could name "a tap on a chrome row" beside the sorted
row list. Not an extraction ask — one instance.

### G-073 — the first sanity arithmetic lost every run for every player (the game's own)

Class: game decision · Session: call-of-cthulhu-r3v1 · Owner: this game's DESIGN.md

**Doing:** the three players over the design's first numbers.

**Expected:** a scholar that wins.

**Happened:** every player lost every seed by day 2-4: a blind answer is an
insult one time in three, and an insult that also lengthened the call
compounded into wrath. The scholar was also weak (a fixed study rule), and
the controllers doc's "suspect the controller first" was right about half of
it. **What I did:** gave the scholar a look-ahead over `answer_outcome`
(expected cost of tonight per option), then retuned: insults no longer
lengthen the line, surcharge 4 -> 2, wrath 20 -> 10, drains 3/3/4 -> 2/2/3.
Result over forty seeds: scholar 38, novice 11, mute 0. DESIGN.md records both
sets of numbers.

### G-074 — round 1 found three checks that read the game's own answer back (the game's own)

Class: game decision · Session: call-of-cthulhu-r3v1 · Owner: this game's `src/verify/rows.rs`

**Doing:** `tools/mutate call_of_cthulhu_r3v1 mutants/r1.txt` — 21 faults.

**Happened:** 18 of 21 noticed. Escaped: C4 (wrath one anger late — no check
drove a caller to its temper), C13 (the sanity readout frozen — row 1 compared
the screen against `sanity_line`, the function under test; the testing doc's
"a check that reads the game's own answer back" exactly), C17 (lost only
below zero — no run is staged at exactly zero). **What I did:** row 1 now
insults Cthulhu to wrath and requires the shipped costs `[6, 18]` and a wrath
ending at anger 3; requires the literal `sanity 98/100` after the lore
answer; `zero_is_lost` stages a call with exactly the lore answer's cost left.
Rerun of the three: 3 of 3 noticed.

### docs/api: 3 findings above

G-070 (misled), G-071 and G-072 (gaps). Everything else this game needed —
headless sessions, keys and a pointer by script, the panel and its floors,
the capture path for a text-only game — the five documents answered where
the skill said they would.
