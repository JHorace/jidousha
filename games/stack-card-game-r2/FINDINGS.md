# Stack duel (r2) — findings

What the documents cost this build, in the shape `docs/internal/e0-findings.md`
uses. G-numbers continue the sequence across games (the last on `main` when
this session began was G-069; other yakin branches in flight the same night
may have taken the same numbers — renumber on merge if they collide).

**Reading discipline.** Read: `CLAUDE.md`, the `make-game` skill, the task
specs (`tools/yakin/tasks/stack-card-game.md` and `-r2.md`), all five
`docs/api/` documents, and from `crates/jidousha/examples/` the
`prototype_kit` files (main, checks, capture) and `ui_kit.rs`. From `tools/`:
the usage headers of `tools/verify` and `tools/mutate`, to learn how each names
a game. `games/keifu/` only for its manifest, one mutant list and the first
entry of its `FINDINGS.md` (for this file's shape and the G-sequence).
**Engine source (`crates/*/src/`): not opened.** Night one's
`stack-card-game` branch and PR #125: not opened (the r2 spec's comparison
hygiene).

---

### G-070 — "a row lying across a control it is not the label of" does not say what *across* is

Class: docs · Owner: `docs/api/jidousha-ui.md` (*The floors*, `judge_panel`)

**Doing:** passing the screen's tappable rectangles to `judge_panel` as its
`controls` — six card boxes, each holding three to five rows of its own text
(name, cost, wrapped rule), the PASS button, and one rectangle per stack row.

**Expected:** a sentence saying whether a row wholly *inside* a control counts
as lying across it, and how a control with several rows of its own (a card)
names which rows are its label — the reference entry gives the signature
`controls: &[(String, Rect)]` and the prose says only "a row lying across a
control it is not the label of (you pass the controls, named)".

**Happened:** neither is said. The name is a free string, so there is no way
to tell the kit which rows belong to which control; a card's rule text would
plausibly read as "not its label".

**What I did:** passed only the PASS button (whose one row is centred inside
it by `centered`) and left the cards and stack rows out of `controls`, so the
floor guards the one control whose label is a single row. The cards' rows are
still judged by every other floor and found on the frame by `judge_frame`.
Nothing in the kit was opened to find out the real rule.

### G-071 — the capture size the documents give is too small for a screen of 12-unit text

Class: docs · Owner: `docs/api/jidousha-capture.md` (the aspect bullet), and
`crates/jidousha/examples/prototype_kit/capture.rs` (`CAPTURE_SIZE`)

**Doing:** copying the capture path for a game whose chrome is laid out in a
960x540 design space with a 12-unit text floor.

**Expected:** the size guidance to name the other constraint besides the
aspect — that the picture is only worth looking at if the smallest text is a
legible number of pixels in it.

**Happened:** the document says "480x270 for a 1280x720 recorder" and the
example's reason for the size is cost and aspect only. At 480x270 a 12-unit
glyph on a 540-unit-tall camera is 6 pixels tall — the stack panel's rows, the
thing this game's picture exists to show, would be a smear.

**What I did:** captured at 960x540 (same 16:9, asserted against the window
in `capture.rs`), where the floor's 12 units are 12 pixels, and looked at it.
A sentence in the aspect bullet — "and large enough that your text floor is
legible in pixels" — would have saved the reasoning.

### G-072 — `tools/mutate <game>` wants the binary's name, while its lists live under the game's folder

Class: docs/tooling · Owner: the `make-game` skill §A.6, `tools/mutate`'s usage text

**Doing:** starting the mutation round for a game whose folder is
`games/stack-card-game-r2/` and whose package and binary are
`stack_card_game_r2` (the r2 spec names them that way).

**Expected:** `<game>` in `tools/mutate <game> <list>...` to be the folder,
since the skill says the lists are "kept beside the game,
`games/<game>/mutants/*.txt`" — the same `<game>` in both places.

**Happened:** the tool refused (`there is no game called
'stack-card-game-r2'`) and named the binary as the thing to pass. Its usage
text does say "name a game's binary" in the error, and the list path is
resolved against the game's directory, so the two `<game>`s are different
things that the skill spells the same. One wasted invocation; it only bites a
game whose folder and package differ.

**What I did:** passed the binary name; the list path `mutants/r1.txt`
resolved against the folder as documented.

---

**The game's own decisions** (not engine findings, recorded so a later session
meets them): every card is instant speed (any card may be played whenever its
player holds priority), energy refills for both players every turn, and a
player with no legal play passes automatically after a beat. All three are in
`DESIGN.md`; the last is a convenience the spec did not ask for, and it never
fires while a choice exists.
