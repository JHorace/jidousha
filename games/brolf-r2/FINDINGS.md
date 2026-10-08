# Brolf (r2) — findings

What the documents and the design cost this build, in the shape
`docs/internal/e0-findings.md` uses (keifu's entries were read for the shape):
what I was doing, what I expected, what happened, and who owns it. G-numbers
continue the sequence across games: the largest heading under `games/` was
G-069 (keifu) when this file was written. Other yakin ticks were in flight the
same night, so a fork at G-070 is possible — G-068's fault, unfixed, because
there is still no neutral place to claim a number.

**Reading discipline (implement stage).** Read: `CLAUDE.md`, the `make-game`
skill, `tools/yakin/{WORKER,DOCTRINE}.md`, the task specs
(`tools/yakin/tasks/brolf.md`, `brolf-r2.md`) and the design
(`tools/yakin/runs/brolf-r2/DESIGN.md`); all five `docs/api/` documents; from
`crates/jidousha/examples/` the `prototype_kit` files (checks, capture, the top
of verify), the head of `slalom/controller.rs` and `ui_kit.rs`. From `games/`:
`keifu/Cargo.toml` and the head of `keifu/src/main.rs` for the crate shape, one
`keifu/mutants/` list for the format, and `keifu/FINDINGS.md`'s shape and last
headings. The usage header of `tools/mutate`. **Engine source
(`crates/*/src/`): not opened. Night one's `claude/yakin-brolf`, PR #128 and
`games/brolf/`: not opened** (the spec's comparison hygiene).

---

### G-070 — the overlay floor counts overlays, not title rows

Class: docs · Session: brolf-r2 implement · Owner: `docs/api/jidousha-ui.md`
(*One value, never a flag beside it*, and `judge_panel`)

**Doing:** staging the screen the two-overlays floor was written for, so the
floor is seen to bite (the document's own rule, *The floors*).

**Expected:** "`judge_panel` refuses a frame with two overlays' content in it,
counted by the title row each one draws" — so the result panel absorbed twice,
two `RESULT` title rows on one frame, breaches it. The design stage read the
sentence the same way and wrote the gate that way.

**Happened:** the only breaches were `two rows of chrome text overlap` (the two
copies sit on each other). Two *different* overlays — the result plus a staged
`PAUSED` panel, both named in the `overlays` list — breach
`two overlays' content is in one frame`. So the floor counts distinct entries of
the `overlays` list whose title is present, and one overlay drawn twice is not
two overlays. **What I did:** staged the second overlay as its own panel and
list entry, and assert the breach by that name.

**Fix:** one clause in the doc: "two *different* overlays — a title drawn twice
is one overlay, and the overlap floor is what catches it."

### G-071 — a long recorded run has no way to forget frames, so it swaps recorders

Class: docs · Session: brolf-r2 implement · Owner: `docs/api/jidousha-testing.md`
(the recorder paragraphs)

**Doing:** keeping the last *live* frame of a 2,000-6,000-tick match (the
document asks for it: "carry out the last frame drawn while play was live"),
which means drawing every tick, because nothing says in advance which tick is
the last live one.

**Expected:** either a way to drop old frames, or a sentence on how a long run
gets its last live frame without holding thousands.

**Happened:** the document says the recorder keeps every frame "with no way to
forget them" and prices it per session; it does not say whether two recorders
in one session agree on texture ids — the font's id in particular, which
`judge_frame` and `frame_text_floor` take. **What I did:** replaced the
recorder every 200 draws (`verify.rs`, `Drawer`), kept the first recorder's
`font_texture()` for the session, and let the floors over frames drawn by later
recorders be the evidence that the ids agree (they pass, and a glyph drawn on
another texture would fail `judge_frame`).

**Fix:** state that `FrameRecorder::new` always lands the built-ins on the same
ids (or that it does not), and name the pattern for a long run's last live
frame.

### G-072 — the design's staging leaned on NPCs standing still

Class: design · Session: brolf-r2 implement (origin: design stage, DESIGN.md
"Gates to add") · Owner: the yakin design stage

**Doing:** run A's sledge gate: "speed == 13.5 (30/60 x 18 x 1.5 Driver x 1.0)".

**Expected:** N1 holding nothing at tick 340, since its Spikes were clubbed
off at tick 5.

**Happened:** between its stun ending (185) and the sledge, N1 looted a Heavy
under its own policy (rule 5), so the strike went out at half speed and the
gate failed with the ball moving 0.11 a tick instead of 0.22. **What I did on
the design's authority:** first believed the sledge itself was wrong; the
moved distance said Heavy. Added `held.clear()` for N1 to the stage after tick
300. Cost: one run.

**Fix:** a staged gate sets every piece of state the asserted number depends
on — the testing document already says so ("a staged frame is not staged until
all of it is"); a design writing stages should apply it to the NPCs too.

### G-073 — the design's end rule let the idle player win

Class: design (the game's own) · Session: brolf-r2 implement (origin: design
stage, DESIGN.md Systems `end_match` and Gates "three players rank the game") ·
Owner: the yakin design stage

**Doing:** the gate "Idle is Eliminated at exactly the tick zone_at predicts
… and banks 0".

**Expected:** the closing zone kills a player who never moves.

**Happened:** under the design's `end_match` — alive count 1 and the others all
`Extracted | Eliminated` makes the last one `Survived` with the bonus — the
three NPCs all extracted soon after the pad opened (tick ~2000-2500), and the
idle player was crowned survivor with 6. The design's own decision ("standing
still is never a win") and its own gate both contradict its end rule. **What I
did:** a golfer survives only when every rival was *eliminated*; a rival that
extracted is not one it outlasted, so it plays on alone and must extract or
meet the zone. Idle is now eliminated on the predicted tick; mutant M19 holds
the rule. A deviation in the PR.

**Fix:** none needed in the engine; the next design stage should run its end
rule against its NPCs' extraction thresholds before fixing a gate on it.

### G-074 — "touches jidousha-testing.md: headless, HeadlessSim"

Class: design · Session: brolf-r2 implement (origin: design stage, DESIGN.md
Systems "verify") · Owner: the yakin design stage

**Doing:** importing what the design listed under `jidousha-testing.md`.

**Expected:** `headless` and `HeadlessSim` in `jidousha::testing`.

**Happened:** they are prelude names (the testing document says so in its
opening lines, and the api document lists both under *App and lifecycle*); the
import failed to compile. **What I did:** took them from the prelude. Cost: one
compile.

**Fix:** none in the docs — they were right; the design attributed the names to
the document that uses them rather than the one that defines them.

### G-075 — the design's item lines did not fit its own 27 columns

Class: design · Session: brolf-r2 implement (origin: design stage, DESIGN.md
Systems "rules" and "the screen") · Owner: the yakin design stage

**Doing:** the pickup rows, `  <ItemSpec.line>` in a 260-unit cell of size-12
text (27 columns, the design's own count).

**Happened:** `  a club stuns 1.0s not 3.0s` is 28 characters and
`  rivals strike your ball 0.5x` 30, so the cell would clip both to `...` and
the gate asserting the Helmet line would fail. The design delegated exactly
this ("any row wording that the 27-column clip forces shorter"). **What I
did:** `club stuns 1.0s not 3.0s` and `rivals hit your ball 0.5x`.

### docs/api: two findings (G-070, G-071)

Everything else the build needed — the closed-form-friendly fixed timestep,
`world.view()` for one snapshot both phases read, the kit's `Panel`/`Cell`/
floors, `schedule_debug`, `find_bounds` for discs and rings, the shapes-only
capture path — the documents answered where they said they would.

### The game's own

- **Every decision row's one function is in `rules`**: `zone_at` (row 1),
  `in_reach` (row 2), `effects` (row 3), `banked` (row 4). The panel, the draw,
  the NPCs, the sim and the gates all call them.
- **Matches end early.** Against seed 7 the NPCs extract between ticks ~2000
  and ~2500 with 8-24 banked, so phases 2-4 of the zone are only ever met by a
  golfer who stays: the Chaser (eliminated at 5917) and the Idle. Whether the
  NPCs should be greedier — higher `EXTRACT_AT` — is a tuning question for a
  playtest, not one this build answered.
- **Cups are easy.** The Chaser, which only putts at the nearest cup, holed 58
  in one match: a cup of radius 0.5 against a 60-step charge is close to a
  sure thing from any distance. Holing pays 2, the same as a sledge; if holes
  should be rarer, the cup radius or the charge resolution is the lever.
