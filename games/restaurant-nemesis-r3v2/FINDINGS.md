# Restaurant Nemesis (r3v2) — findings

What the documents (and the design) cost this build, in the shape
`docs/internal/e0-findings.md` uses. G-numbers continue the sequence across
games: the highest on `main` was G-069, and this tick's other open PR (#136,
`games/stack-card-game-r3v1/`) takes G-070..G-074, so this file starts at
G-075.

**Reading discipline (yakin V2 worker tick).** Read: `CLAUDE.md`, the
`make-game` skill, `tools/yakin/{WORKER,DOCTRINE}.md`, the task specs
(`tools/yakin/tasks/restaurant-nemesis{,-r3v2}.md`), this run's `DESIGN.md` and
`FINDINGS.md` whole, the five `docs/api/` documents (read earlier in the same
tick for #136), and `crates/jidousha/examples/prototype_kit/`. **Engine source
(`crates/*/src/`), `docs/internal/`, `docs/adr/`: not opened.** The paired V1
run (`restaurant-nemesis-r3v1`) was not opened.

---

### G-075 — `judge_frame`'s worked example names a `Flat` mapping the Reference never defines

Origin: design stage, `tools/yakin/runs/restaurant-nemesis-r3v2/FINDINGS.md` (D-1).

Class: docs · Owner: `docs/api/jidousha-ui.md` (the `judge_frame` example)

**Doing:** stating the screen's mapping from design units to world units.
**Expected:** every name an example uses to be in the Reference, or marked as
the reader's own. **Happened:** `judge_frame`'s example passes `&Flat`; `Flat`
appears nowhere else in the five documents. **What I did:** the design had the
game define its own identity `Mapping` (`screen::Flat`), and the build does.
The implement stage met the same gap independently in #136's game.

### G-076 — `judge_panel`'s `overlays` parameter does not say what its two strings are

Origin: design stage, `tools/yakin/runs/restaurant-nemesis-r3v2/FINDINGS.md` (D-2).

Class: docs · Owner: `docs/api/jidousha-ui.md` (`judge_panel`)

**Doing:** naming the nemesis card as an overlay so the two-overlays floor
covers it. **Expected:** the pair defined. **Happened:** only inferable from
one example (`("FEED", "FEED - what happened")`). **What I did:** read it as
(name, exact title row) and shipped `OVERLAYS = [("NEMESIS", "NEMESIS CARD")]`;
every staged screen judges clean against it. The implement stage could not
stage a breach of *this* floor without a second overlay, so the reading is
consistent with the floor but not proven by it.

### G-077 — the design's `SEED_SPAWN` precondition has no seed

Origin: implement stage, against the design (a design that misled).

Class: design · Owner: `tools/yakin/runs/restaurant-nemesis-r3v2/DESIGN.md`
(§Open calls, "Seeds") — the designer routine's process

**Doing:** scanning for `SEED_SPAWN`: "the greedy chef's day 1 leaves unmet an
order with severity ≥ 4, exactly one nemesis spawns on day 1, and that nemesis
is alone through day 5 under the tally policy".

**Expected:** a seed in 0..256 (or 0..1024, as the design allows).

**Happened:** none in 0..1024. Under the scenario player (no spends) the
nemesis's followers carry `need + 1` in its theme, so their severities reach 4
and they spawn a second nemesis by day 3 or 4 on every seed where the first
spawn happens (174 seeds of 1024 spawn exactly one on day 1; 0 stay alone).
The design's own numbers make its precondition unreachable; it was not run.

**What I did:** the conservative reading. The precondition is now
"exactly one nemesis after day 1, money ≥ $50 on night 1, and that nemesis
first in the queue on days 3 and 5", so others may spawn. The G6 runs track
the first nemesis by id rather than asserting the active list is empty.
`SEED_SPAWN = 9` (Mustard Monster). The precondition is itself checked on
every run (`spawn_seed_holds_now`). Cost: about ten minutes, plus a deviation.

### G-078 — the capture size the design names is too small to read

Origin: implement stage, against the design and `docs/api/jidousha-capture.md`.

Class: docs · Owner: `docs/api/jidousha-capture.md` (Capture at the
recorder's aspect ratio)

**Doing:** the G10 capture at the design's 480x270. **Happened:** this game
is all 12-unit text, which is ~6 pixels at 480x270. Same finding as #136's
G-072. **What I did:** captured at 960x540 (same 16:9, asserted).

### G-079 — the game's own: two of the design's row strings overflow the room at the floor size

Class: game · Owner: this game (DESIGN.md §Surfaces and inputs)

**Doing:** judging the service screen with a follower and a nemesis at the cap.
**Happened:** the design's line 2 for a follower — `served: ... | unmet: ...
-> feeds <title> +<f> fol, -$8 -<t> rep` — plus the follower's "+5 to the
leader" ran to 1037 units at 12px, past the 960 rect; `judge_panel` caught it.
The design's ledger row `[n] <TITLE> - <tier>, <f> fol - tomorrow <share> of 6
customers are followers (<theme>, +1 need)` is ~115 characters, which is also
past the rect.
**What I did:** the leader growth moved to line 1 (`... need 3 +5 fol if
unmet`). The ledger row is split into three: `[n] <TITLE> - <tier>, <f> fol,
<k> spends to ratio` / the tier's post / `tomorrow <share> of 6 customers are
followers (<theme>, +1 need)`. The spend line is unchanged. Both are listed as
deviations.
