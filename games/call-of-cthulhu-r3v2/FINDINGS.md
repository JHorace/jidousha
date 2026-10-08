# Call of Cthulhu (r3v2) — findings

What the documents and the design cost this build, in the shape
`docs/internal/e0-findings.md` uses. G-numbers continue the sequence across games
(the last on `main` was keifu's G-069); other round-three runs in flight at the
same time may take the same numbers (G-068's collision, again) — these are this
crate's.

**Reading discipline (implement tick).** Read: `CLAUDE.md`, the `make-game` skill,
`tools/yakin/{WORKER,DOCTRINE}.md`, the task specs (`tasks/call-of-cthulhu-r3v2.md`,
`tasks/call-of-cthulhu.md`), the run folder's `DESIGN.md` and `CHECKPOINT.md`; from
`docs/api/` the Quickstart, Concepts and the reference entries used, of all five
documents; from `crates/jidousha/examples/` the `prototype_kit` files (`checks.rs`,
`capture.rs`, the head of `verify.rs`). From `tools/`: the heads of `tools/verify`
(how a playable is named) and `tools/mutate` (its list format). **Engine source
(`crates/*/src/`), `docs/internal/`, ADRs: not opened.** No other round's run of this
idea, nor its V1 counterpart, was opened.

---

### G-070 — the design's arithmetic said the Guesser dies; it survives

Class: design · Origin: design stage, `tools/yakin/runs/call-of-cthulhu-r3v2/DESIGN.md`
· Owner: the designer routine

**Doing:** building G5's three players from DESIGN.md §Gates and §Decisions ("the
arithmetic").

**Expected:** the Guesser ("morning: `Meditate`; night: always Wrong") "pays at least 108
and dies on night 5", a band G5 asserts as "reaches `End { won: false }` during night 4
or 5".

**Happened:** by the design's own rules a meditating, never-studying player pays
`4 * (drain - 1)` a call over seven calls — 68 to 76 sanity across the rotations, out of
90 — and wins. The 108 matches the same calls without meditation (96 to 104), so the
design subtracted the meditation from nothing. Caught on paper, before any code ran.

**What I did:** kept every number of the design and changed the Guesser's morning to
the Scholar's (study tonight's first caller with fewer than two secrets) while it still
always answers Wrong: a player who does the homework and guesses anyway. It pays 94 on
both seeds and loses on night 4. One line in the PR's Deviations.

**Cost:** about ten minutes of arithmetic; no rebuild.

### G-071 — `tools/mutate` takes the binary's name, and the design gave the folder's

Class: design · Origin: design stage, `DESIGN.md` §Systems "mutants" · Owner: the
designer routine (and `docs/api/jidousha-testing.md`, which says `tools/mutate <game>`)

**Doing:** running the mutation round as DESIGN.md wrote it,
`python3 tools/mutate call-of-cthulhu-r3v2 mutants/r1.txt`.

**Expected:** the round to run; the testing document's spelling is `tools/mutate <game>
<list>...` and the list lives in `games/<game>/mutants/`, which reads as the folder.

**Happened:** `there is no game called 'call-of-cthulhu-r3v2' under games/ (games:
call_of_cthulhu_r3v2, keifu, ninjo)` — the tool takes the binary's name, as `tools/verify`
does. Harmless for a game whose folder and package share a name; the r3 suffix rule
(folder hyphenated, package underscored) is exactly the case where they differ.

**What I did:** ran `python3 tools/mutate call_of_cthulhu_r3v2 mutants/r1.txt`.

**Cost:** one failed invocation.

### docs/api: 0 findings

A turn-based game of text on one screen: `Panel`, `TextRun`, `wrap`, the floors,
`FrameRecorder`, `SnapshotBuilder`, `InputScript`, `schedule_debug` and the
shapes-only capture path. Every question the build asked, the documents answered where
the make-game reading order put them; `Panel::block`'s `leading` being the gap between
rows rather than the line step is shown by its reference example's arithmetic.

### The game's own

- **The scream lives outside `Outcome`.** `answer_outcome` returns the design's four
  fields; whether a reply makes the being scream is `rules::screams(temper, outcome)` and
  what it costs in all is `rules::total_cost`, read by both `flow::answer` and the
  players' prediction, so "predicted X, paid Y" stays one function's answer.
- **A call that ends shows its last reaction on the next call's first screen**, above
  the new caller's opening line; the design's "reaction line, afterwards" left the case
  of a call change open.
- **Invalid presses do nothing.** `play` offers `step` only a digit within the screen's
  option count or Enter on a Dawn or End, and `step` reports (returns `false`) rather
  than acts on anything else.
