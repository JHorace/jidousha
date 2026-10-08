# Restaurant Nemesis (r3v1) — findings

What the documents cost this build, in the shape `docs/internal/e0-findings.md` uses:
what I was doing, what I expected, what happened, and which document owns it.
G-numbers continue the sequence across games. The highest on `main` when this run
started was G-069, and this tick's previous task (`brolf-r3v1`, PR #139) took
G-070..G-073, so these start at G-074. Other round-three branches in flight may
collide; that is a renumbering at merge.

**Reading discipline (yakin task restaurant-nemesis-r3v1, one V1 tick lineage, the
same session that built brolf-r3v1).** Read for this game: the task specs
(`tools/yakin/tasks/restaurant-nemesis-r3v1.md`, `tools/yakin/tasks/restaurant-nemesis.md`)
and `docs/api/jidousha-ui.md` (the prose whole; the Reference entries for `Cell`,
`centered`, `Floors`, `frame_text_floor`, `glyph_run`, `Icon`, `IconRun`, `inside`,
`judge_frame`, `judge_panel`, `Mapping`, `Panel`, `Pause`, `TextRun`, `toggle`, `wrap`,
`Breach`, `Attention`). The other four `docs/api/` documents and
`crates/jidousha/examples/prototype_kit/` were read earlier in the session for brolf.
`checks.rs` and `capture.rs` were copied from this session's own brolf branch.
**Engine source (`crates/*/src/`), `docs/internal/`, `docs/adr/`: not opened.** The
paired run `restaurant-nemesis-r3v2` was not opened (comparison hygiene).

---

### G-074 — no floor sees a row the kit clipped, and on a decision surface that hides the decision

Class: docs / kit · Session: restaurant-nemesis-r3v1 · Owner: `docs/api/jidousha-ui.md`
("Measured text", "The floors")

**Doing:** building the nemesis card — the surface the player reads to choose how to serve
a nemesis — out of `Cell::run` rows, as the kit recommends.

**Expected:** a floor, among the ones `judge_panel` returns, for a row that did not fit:
the kit already measures every row against its width to clip it.

**Happened:** `Cell::run` clips silently with three dots, by design ("so a row that ran
out of room never reads as a rendering fault"), and no floor reports it. The first build's
card ended `CAREFUL (cost 2): DEFEATED...` and `2 overwhelm: quality 7+ vs...` — the two
facts the decision turns on were the clipped halves. `judge_panel`, `judge_frame` and
`frame_text_floor` all passed; my own scripted check that read the card's text is what
failed.

**What I did:** shortened every card and row string to fit, and added a check of my own
over every panel in `--verify`: no string on screen ends in `...`.

**Fix:** either a floor in `judge_panel` ("a row was clipped to fit its cell") that a game
can waive per surface, or one sentence in "Measured text" saying a clipped row on a
surface a decision reads is a defect the floors will not catch.

### G-075 — the game's own: the first economy had no pressure, the second had nothing but

Class: game · Session: restaurant-nemesis-r3v1 · Owner: this game
(`tools/yakin/runs/restaurant-nemesis-r3v1/DESIGN.md`)

**Doing:** the three-player run, from the first build.

**Expected:** a good player who wins while meeting a nemesis or two, a first-timer who
struggles, an idle player who loses.

**Happened:** with DESIGN.md's 4 customers a round, "standard for everyone" served every
order every round, never failed anyone badly, and won with $200 — the first-timer beat
the good player. With 5 a round and a skipped order one level more severe, every skip
spawned and both players snowballed into three nemeses and closed by day 4. Neither was
visible from the numbers on paper.

**What I did:** 5 customers a round plus a seeded 3-customer lunch rush in one round, a
skip one level more severe, the spawn line at severity 4 (so only a skipped level-3
demand spawns), a nemesis starting at 30 followers, refunds at $1 a level. On seed 3 the
good player now wins with $183 after beating one nemesis outright, the first-timer closes
on day 5 with three nemeses at once, and the idle player closes on day 1. Every change is
a Deviation in the PR.
