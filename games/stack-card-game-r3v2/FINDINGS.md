# FINDINGS — stack-card-game-r3v2

Entries in the shape of `docs/internal/e0-findings.md` (make-game §C). G-numbers continue the sequence
across games from the largest heading on `main` (G-069, `games/keifu/FINDINGS.md`). G-068's fork risk
applies: this run's V1 counterpart may take the same numbers, and there is still no neutral place to
claim one.

Reading note: this session read `docs/api/` (all five) and `crates/jidousha/examples/prototype_kit/`,
plus `games/keifu/Cargo.toml` and `games/keifu/mutants/dock.txt` for the manifest and the mutant-list
shape (a sibling game, which make-game §0.1 allows). Nothing under `crates/*/src/`, `docs/internal/` or
`docs/adr/`.

### G-070 — `Flat` is used by the UI doc and defined nowhere in `docs/api/`

Class: doc misled · Session: stack-card-game-r3v2, design stage (origin: `tools/yakin/runs/stack-card-game-r3v2/FINDINGS.md` entry 1) · Owner: `docs/api/jidousha-ui.md` (`tools/api-doc/`)

**Doing:** designing, then building, the frame check for the kit-built screen from `judge_frame`'s example.

**Expected:** every name in a Reference example to be the kit's, the prelude's, or defined in the example.

**Happened:** `judge_frame(&screen(), &frame, recorder.font_texture(), &Flat, view)` passes a `&Flat` that no
Reference entry of the five files defines; a reader takes it for a kit-provided identity `Mapping`.
**What I did:** on the design's instruction, defined the identity mapping in `src/screen.rs` (`to_world(ui) = ui`,
`scale() = 1.0`). I did not probe whether `jidousha::ui::Flat` exists, which would have meant reading past the
documents. **Fix:** define `Flat` inline in the example, or export it from the kit and give it an entry.

### G-071 — the design checked every cell's width against its strings but not the hand title's

Class: design misled · Session: stack-card-game-r3v2, implement stage · Owner: the design stage (yakin V2)

**Doing:** building the hand panel exactly as DESIGN.md "Layout, in constants" gives it.

**Expected:** the layout to fit: the design's done-so-far line says it re-checked "cell widths vs string lengths".

**Happened:** the hand title `HAND - 1-6 plays, Space passes` at the `TITLE` size (18, so 14 units a character)
is 420 units wide from x 598 and runs 58 units past the 960 design rect — the first verify run's floors and
bounds checks both caught it on every frame. **What I did:** shortened the title to `HAND - 1-6 plays` (the
controls line already says Space passes), kept the size, and listed it under the PR's Deviations. Cost: one
verify cycle. **Fix:** a design's layout pass should measure titles as well as cells — or leave widths to the
floors, which catch them in one run.

### G-072 — the sequencer clears the bar by exactly one seed (the game's own)

Class: the game's own · Session: stack-card-game-r3v2, implement stage · Owner: the next session on this game

**Doing:** the three-player sweep (G7), seeds 1–12 against the Rival.

**Expected:** a margin above DESIGN.md's bar of 7 of 12.

**Happened:** nothing 0 of 12, brute 0 of 12, sequencer exactly 7 of 12 (142 plays, 121 of them responses). The
ordering the brief asks for holds with room — raw power wins nothing — but the sequencer's one-ply look-ahead
is at the bar, so a change to the Rival's rule or to a card may tip the sweep red without the game getting
worse. **What I did:** nothing; the sequencer and the Rival are as DESIGN.md wrote them. **Next:** if the
sweep goes red after an unrelated change, deepen the sequencer (a second reply, then its own best answer)
before touching any card — DESIGN.md forbids changing the pool to meet the bar.
