# Brolf — findings

What the documents and the design cost this build, in the shape
`docs/agent-practices.md` §2.5 gives: class, what I was doing, what I expected, what
happened, what I did on the doc's authority, and who owns it. G-numbers continue the
sequence across games (keifu's last on `main` is G-069); a parallel yakin branch may
have taken the same numbers — if so, renumber on merge.

**Reading discipline.** Read: `CLAUDE.md`, the `make-game` skill, the spec, the design
(`tools/yakin/runs/brolf/DESIGN.md`), `docs/api/` all five documents (the UI kit's only to
confirm the design's Non-goal), and from `crates/jidousha/examples/` the `prototype_kit`
files (main, verify, checks, capture) and `slalom/controller.rs` (head). **Engine source
(`crates/*/src/`): not opened.** No other game read.

Entries marked *design stage* are filed by the implement stage against
`tools/yakin/runs/brolf/DESIGN.md`: it said something about the engine, the spec or the
build that was not true. The designer filed none (its `FINDINGS.md` does not exist), and
its closing line "No FINDINGS entry arises from this design" was true of the engine.

---

### G-070 — "the box around the quads covering the centre is exactly 2r x 2r" holds only at the fan's own bits

Class: docs (misled) · Owner: `docs/api/jidousha-testing.md` ("a quad the size of the thing")

**Doing:** asserting the aim preview's scatter disc (`ctx.circle`, radius 0.6) with the
document's `find_bounds(frame.covering(at).filter(inside the 2r box))` recipe, at the point
where the check *computed* the aim to be (`ball + (4, -3)`).

**Expected:** a 1.2 x 1.2 box, as the document says: "all sixteen share the centre as a
corner ... the box around the quads covering the centre is exactly 2r x 2r".

**Happened:** a 0.6 x 0.459 box. The game draws at the pointer-through-the-camera's float
arithmetic, which differs from the check's literal by about 1e-6; `covering` is exact, and a
point a millionth off a fan's shared corner is inside one or two wedges, not sixteen.

**What I did on the doc's authority:** wrote the check as the document shows, ran it, and lost
one cycle reading the draw code for a clipping bug that did not exist. Fixed by reading the
centre from the preview's own aim point (`aim_landing`) and checking separately that it is
within 0.01 of the literal. The document wants one sentence: the recipe needs the query point
to be the very value the game drew at, not a recomputation of it.

### G-071 — design stage: the strike staging is unreachable (parked bodies win contact)

Class: design (misled) · Owner: DESIGN.md "Session C" C3 and "Staging"

**Doing:** building C3 as written: the human's ball at `hole + (0.5, 0)`, `P1` at `hole + (-0.5, 0)`,
every other rival "parked within 1.2 of the hole".

**Expected:** `npc_intent` rule 3 strikes the human's ball.

**Happened:** the parked bodies are 0.7 from `P1`. `contact_target` prefers a body, so it is a
`Club` on someone holding nothing, rule 3 declines, and the strike never comes. The same
parking also puts a parked body inside the human's reach of `(-2, 0)` for some holes, which
would have changed Session B's cue on a different seed.

**What I did:** staged every session relative to which side of the course the hole is on
(`gates::side`), the human 8 units away on the other side, and put the striker 5 to 9 units
from the parked group. The check numbers (strike length 2.0, shot length 5.25 +/- 12%) are as
the design states them. Cost: one design-vs-reality re-derivation of every coordinate.

### G-072 — design stage: a 3 s daze against a 2 s swing wait is a stunlock

Class: design (misled) · Owner: DESIGN.md "contact", Decision 5

**Doing:** the first full match against the live rivals.

**Expected:** "a temporary disable, never a direct permanent kill" (the spec's row 2).

**Happened:** `DAZE_TICKS 180` against `SWING_COOLDOWN 120`: a clubber swings again 60 ticks
before the victim recovers, and a club on a dazed body refreshes the daze. Two Hunters locked
each other, then everyone else, for ever: 98 dazes in 95 seconds, and `Stats` showed no strike
at all because a body in reach always beat a ball. A human next to a Hunter would be
permanently disabled, which is the thing the spec forbids by name.

**What I did:** `contact_target` skips a body that is already dazed (its ball is then the
target). This is a departure from the design's "nearest live rival body", recorded in the PR's
Deviations. It also made strikes happen (9 in the idle run, 6 in the golfer run). The numbers
are untouched.

### G-073 — design stage: the second status line does not fit at size 0.42

Class: design (misled) · Owner: DESIGN.md "draw", the band text size

**Doing:** laying the four band lines out at the stated 0.42.

**Expected:** every status string fits the 32-unit-wide camera.

**Happened:** at 0.42 a character is 0.327 wide: 96 characters fill the band. The second line's
worst case (two items held, the pad open with a closing time, the hole not yet open) is 117.
The design's own check F1 would have caught the overrun, but only on a frame that reached it.

**What I did:** the band text is 0.33 (`TEXT_SIZE`), the separators two spaces, and
`HOLE {d} away,` is `HOLE {d}`. The worst case is 117 characters = 30.0 units.

### G-074 — design stage: three gate numbers describe a game that was not yet running

Class: design (misled, numbers) · Owner: DESIGN.md E1, E5

- **E1 `Golfer` takes >= 6 shots.** A first-try golfer that lays up to within 1.5 of the hole and
  waits for it to open takes 3 shots in the 95 seconds before the hole opens, then holes out or
  is beaten to it within ten ticks. The gate is `>= 3`; the observation is that the first 95
  seconds of a golfer's match are three shots and a wait, which is the thing to look at when
  this is played.
- **E5 counted on the `Full` run.** `Full` holds two items when the pad opens at tick 2700 and
  extracts at tick ~2820, which ends the match before any rival can extract. E5 is counted on the
  `Golfer` run (the whole match, 5700+ ticks), where a rival extracts once and strikes six balls.
- **Strikes need a dazed owner.** With body-beats-ball and no stunlock a ball is struck only
  when its owner is down or away from it; the Hunters therefore also close on a resting rival
  ball whose owner is more than a swing's reach from it (`intent.rs` rule 4), and swing at
  anything in reach (rule 3). Both are departures from the design's rules 3 and 4.

### G-075 — design stage: staging "after Startup" leaves the rivals' tick-1 shots in the air

Class: design (silent) · Owner: DESIGN.md "Staging"

Every rival's ball is 0.8 from its body, so on tick 1 every Golfer shoots. Teleporting the ball
(the design's recipe) leaves its `Flight`, and `fly` moves the ball back. `gates::place` clears
the flight, the exposure, any extraction and the swing wait. The design should say so; the
engine did nothing wrong.

### docs/api: 1 finding (G-070). The game's own: the rest above.
