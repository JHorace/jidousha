# Call of Cthulhu (r2) — findings

What the documents cost this build, in the shape `docs/internal/e0-findings.md`
uses. G-numbers continue the sequence on `main` (keifu's last is G-069); other
yakin branches in flight the same night may take the same numbers — renumber at
merge if they collide.

**Reading discipline.** Read: `CLAUDE.md`, the `make-game` skill, the task specs
(`tools/yakin/tasks/call-of-cthulhu.md`, `-r2.md`), all five `docs/api/`
documents (the API reference by section, not cover to cover), from
`crates/jidousha/examples/` the `prototype_kit` files (`main.rs`, `checks.rs`,
`capture.rs`, parts of `verify.rs`). From `games/`: `games/README.md`,
`games/keifu/Cargo.toml` (the manifest's shape), the first block of
`games/keifu/mutants/w3.txt` (the list format) and the head of
`games/keifu/FINDINGS.md` (this file's shape and the G-sequence). The tool
headers of `tools/mutate` and `tools/verify`. **Engine source
(`crates/*/src/`): not opened.** Night one's `call-of-cthulhu` branch, PR #127
and any game it produced: not opened (the r2 spec's comparison hygiene).

---

### G-070 — "in the transcript" in a decision table names an instrument that cannot see words

Class: docs · Owner: `docs/templates/DECISIONS.md` (the *asserted by* column),
and the task spec that copied its wording

**Doing:** writing the check for both decision rows, whose *asserted by* column
says the question, options, sanity and known lore must be "in the transcript"
before answering, and the morning's stated effect "in the transcript at
choosing".

**Expected:** one instrument called the transcript that holds what the screen
says.

**Happened:** `docs/api/jidousha-testing.md` has two things called
`transcript` (`FrameRecord::transcript`, `FrameRecorder::transcript`) and both
are lists of quad extents — the same document says no assertion over drawn
quads can see a wrong character. A text game's facts are words, so "in the
transcript" read literally is a check that cannot fail on the thing it names.

**What I did:** read it as "on the screen": the call and morning screens are
`jidousha::ui::Panel`s, the checks assert on the panel's strings, and
`judge_frame` proves every one of those rows is on the recorded frame. The run's
own event log (`Game::log`) is printed after the verdict as a second, textual
transcript. If the table meant something else, the checks are the wrong shape.

### G-071 — `judge_frame`'s example names a `Flat` mapping nothing defines

Class: docs · Owner: `docs/api/jidousha-ui.md` (`judge_frame`'s example)

**Doing:** judging a screen laid out directly in world units.

**Expected:** `Flat` — used in the example — to be a kit type for "a design
unit is a world unit".

**Happened:** it is not in the Reference; it is the example's own unstated
type. A three-line `impl Mapping` in the game (`screens::Flat`) was the answer.
Cost: a minute; recorded because an example that names an undefined type reads
as a missing export.

### G-072 — the controllers document's three numbers are about aim, and a choice game has none

Class: docs · Owner: `docs/api/jidousha-controllers.md`

**Doing:** writing the three players for a game of discrete choices (a menu in
the morning, one of three answers at night).

**Expected:** the document's three numbers to transfer as "write three players"
did.

**Happened:** "write three players" transferred whole and earned its keep (the
middle line is what showed the first tuning was wrong — G-074). The three
numbers are stated for a paddle (approaches met, landing distance, landing
error) and needed translating: the run prints, per player, answers by kind
(lore / wrong / insult), how many were given with the hint showing, and an aim
error — summed |hinted sanity cost − applied cost|, which is the hint's
promise checked against the exchange. "Suspect the controller first" also
transferred: the first scholar (a fixed heuristic) won 9 of 12 and the fix was
a rollout scholar, not a constant.

### G-073 — the game's own: disrupting the same cult every morning removed a being from the run

Class: game · Owner: this game's `DESIGN.md`

The rollout scholar found that "Disrupt <cult>" every morning keeps that being
off the phone for the whole run at the price of temper it never pays (a being
that never calls never charges its temper). Fixed in the rules: a being at
temper 2 or more cannot be held back, so each cult can be disrupted at most
twice (`rules::UNHELD_TEMPER`).

### G-074 — the game's own: with wrong answers shortening the call, lore was nearly worthless

Class: game · Owner: this game's `DESIGN.md`

First tuning had a wrong answer at −1 interest (lore −3). The first-timer, who
guesses between the two answers that are not rude, then won as often as the
scholar: guessing cost about one extra exchange a call, so studying bought
almost nothing. A wrong answer now *prolongs* the call (+1), which is also what
the brief says ("wrong answers prolong it"); the middle line went from winning all 3 seeds then tried
to about a quarter, and the scholar holds 22 of 24.
