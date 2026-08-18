# Run 4 — Pong

Friction log, written as it happened. I read `docs/api/jidousha-api.md` and
`crates/jidousha/examples/` (quickstart, prototype_kit, scripted_player,
vec2_tour) and nothing else — no `src/`, no `docs/internal/`, no `docs/adr/`,
no other run's log.

The game is `crates/jidousha/examples/pong/` (`main.rs` + `verify.rs`).
`cargo run -p jidousha --example pong -- --verify` passes; `cargo fmt --all` and
`cargo clippy --workspace --all-targets -- -D warnings` are clean.

Entries are roughly in the order I hit them. Nothing here is softened.

---

## Before writing a line

**F1 — `sin_cos` has two import paths and the examples disagree about which.**
`prototype_kit/main.rs` opens with `use jidousha::math::sin_cos;` *alongside*
`use jidousha::prelude::*`, which reads as "the prelude does not have it".
`vec2_tour.rs` imports only the prelude and calls `sin_cos`, `rotate` and
`atan2` and constructs `Radians`, so the prelude does have it. For an engine
whose first convention is "one way to do everything", two working spellings of
the same import in two example files is exactly what a game author copies
wrongly. The reference's entry for `math` is `pub mod math` and one line of
prose; it does not say whether the contents are re-exported. I used the prelude.

**F2 — nothing says what `Depth::layer` numbers mean.** "Draw ordering" says
`layer` is "the coarse tool (background/world/UI bands)" and stops. Every game
will invent its own numbering. I copied `prototype_kit`'s `mod layers` wholesale
because it is the only worked example of the idea, which means the convention is
propagating by imitation rather than by being written down.

**F3 — the API document never mentions `--verify`.** Its last line says
`tools/verify <example>` "is the whole loop as one command", but nothing says
that the loop is a mode the *example itself* has to implement, or that the
switch is spelled `--verify`. I knew because the task told me and because
`prototype_kit/main.rs` sniffs `std::env::args()`. A game author working from
the document alone gets the whole "Testing your game" section — which is
excellent — and no idea that there is a convention for wiring it to a command
line.

---

## Writing the simulation

**F4 — nothing sweeps, and there is no primitive to sweep with.** The Concepts
section is unusually honest here: it names tick-boundary collision, says a fast
small ball is "the first thing that bites", and says the fix is the game's. Good
— but then the entire collision vocabulary is `Rect::overlaps` and
`Rect::contains`, and neither answers the question a ball and a paddle actually
pose. I wrote a plane-crossing test by hand (`advance` in `main.rs`): where
along this tick's travel did the ball's leading edge cross the paddle's face,
and was the paddle there when it did. Every Pong written against this engine
will write that same forty lines. There is no segment-versus-rect helper, no
`Rect::sweep`, and no `Rect::inflate` either — expanding a paddle by the ball's
radius is `PADDLE_SIZE.y * 0.5 + BALL_RADIUS` spelled out at three call sites.

The advice to assert the tunnelling margin against the `fixed_dt` the engine
hands you rather than against 1/60 is good and I took it
(`assert_the_ball_cannot_tunnel`). It is also the only thing standing between
this game and a silent bug the day someone changes the timestep, which is a lot
of weight for an `assert!` in a game to carry.

**F5 — a game's own "which screen are we on" enum cannot be called `Phase`.**
The prelude exports a `Phase` trait. Mine is called `Stage`. Trivial, but the
obvious name for a very common game-side concept is taken by an engine concept
a game never names directly.

**F6 — `Seconds` is a newtype you immediately leave.** Every integration step is
`something * world.resource::<Time>().fixed_dt.as_f32()`. `Seconds` has `Add`
and `Sub` and nothing that multiplies a rate, so `as_f32()` appears in every
system that moves anything. The examples all do this too, so it is the intended
shape — but "units live in types" ends at the first multiplication.

**F7 — one sign error cost a whole cycle, and the *assertion message* is what
found it.** I wrote `-facing` where I meant `facing` in the bounce, so both
paddles knocked the ball out on their own side of the field. Every structural
assertion passed — the world had two paddles and a ball, frames were drawn,
somebody won 5–0. The check that caught it was the one that reported a
quantity: *"no rally ever came back — the longest rally was 1 paddle touch over
502 ticks"*. The document's insistence on printing the numbers a condition
looked at rather than the conclusion it reached is the single most useful
sentence in it, and it paid for itself on the first failure. Worth saying
plainly because it is easy to read that paragraph as style advice.

---

## Drawing

**F8 — `ctx.circle` is not one quad. It is sixteen, and nothing says so.**
This is the biggest gap I hit. `Submit::circle` is documented as "Fill a
circle". `Quad` is "everything the engine draws, after expansion". The worked
verification in `prototype_kit/verify.rs` checks that a paddle was drawn by
looking for a quad *the size of the paddle* at the paddle's position — the
obvious thing to copy for the ball, and the thing I copied. It fails, silently
and confusingly, because what covers the ball's centre is sixteen wedges of
0.450×0.172, 0.416×0.318, 0.318×0.416, 0.172×0.450 and so on. I only found out
by making the assertion dump what it had actually found, which is a full debug
cycle spent on an undocumented implementation detail of the one primitive a ball
is made of.

The fix in `verify.rs` is `disc_drawn`: union the bounds of every quad covering
the point that fits inside the disc's box, and check the union. That is a
reasonable check, but I had to invent it, and it is not the check the only
worked example teaches.

Two things the document should probably say: a circle expands to a fan, and a
circle costs sixteen quads. The second is a budget fact — my ball is 16 % of the
frame's quad count all by itself.

**F9 — `Camera::visible_bounds` returns `(Vec2, Vec2)` and not a `Rect`.**
`Rect { min, max }` is documented as "min: top-left, max: bottom-right", which
is precisely the pair this returns. The consequence is visible in the document's
own recommended assertion: six lines of hand-written `>=`/`<=` comparisons that
would be one call on a `Rect`. I wrote those six lines, twice (once per frame in
the match, once per staged screen), and factored them into `assert_on_screen`.

**F10 — text has no vertical measurement.** `TextStyle::width_of` is exact and
the document is right to push it. There is no `height_of`, and `size` is "the
height of one line, in world units — *including the gap below it*", so how much
of that a glyph actually occupies is unstated. I placed everything by its top
edge and then read the draw transcript to find out where the glyph quads landed
— they span exactly `size` top to bottom, with the advance being 7/9 of `size`
horizontally as documented. Nothing told me that; I measured it off the output.

**F11 — the em dash is not printable.** The font is "the ninety-five printable
ASCII characters, space through `~`". I had typed `"W / S — move"` in the hint
line out of habit; that would have drawn a box. The document does say this, in
the `TextStyle` entry, and I caught it by re-reading rather than by any check
firing — the bounds assertion cannot see it, because a box glyph is exactly the
same size as a letter. A `debug_assert` in `ctx.text` on unprintable input would
have caught it for free.

**F12 — I found a layout bug by reading 101 lines of quad transcript.** I had
hung the hint line off `bottom_right.y - 1.3`, i.e. off the camera, and the
field's bottom wall is drawn at `FIELD_BOTTOM`, i.e. inside it. The text sat on
top of the wall. Nothing failed: it was on screen, so the bounds assertion was
happy. The document claims the transcript is "good enough to check a layout by
eye" and that is true — but it is also the *only* way, and "by eye" means
reading a hundred lines of coordinates and holding the picture in your head.

---

## The check

**F13 — `FrameRecorder::frames()` and `FrameRecorder::draw()` cannot be used in
the same function without a workaround.** The document gives two snippets a page
apart: draw a frame per tick and then look at `recorder.frames().last()`; and
then "check the screens your run never reaches" with another `recorder.draw()`.
Doing both is a borrow error — `frames()` holds the recorder immutably for as
long as the frame reference lives, and `draw()` wants it mutably. I ended up
doing *both* workarounds: `.cloned()` for the match's last frame, and a second
`FrameRecorder` for the staged screens (which also keeps the printed transcript
pointing at the real last frame of the match rather than at a synthetic screen —
a trap I walked into first).

Related: the recorder keeps every frame. My run records 2598 of them to look at
one. There is no "record only the last" and no `clear()` on `FrameRecorder`
(there is one on `NullBackend`, which is the lower-level path).

**F14 — the closed-loop controller has to re-implement the game's own bounce.**
`best_offset` in `verify.rs` duplicates the paddle-reflection maths from
`advance` in `main.rs`, because to aim you must predict, and there is nothing to
ask. That is probably unavoidable — the simulation is the game's, not the
engine's — but it means the check can agree with a wrong prediction of a wrong
game and still pass. Worth knowing that the shape allows it.

**F15 — "a controller that plays it safe is not a playability test" is sharper
than it reads.** I wrote the naive version first: predict the intercept, stand
so the ball meets the paddle off-centre, aim away from wherever the opponent is
standing. It won, so I believed it. The match took **79 seconds** with rallies
of 20 touches, and I spent six runs — twenty to forty seconds each — retuning
`AI_SPEED`, `SPEED_GAIN` and `MAX_BALL_SPEED`, watching the summary line and
guessing, because the summary line is the only instrument there is.

None of the tuning was the problem. The controller was. Aiming "away from where
the opponent is standing" is worthless against an opponent that drifts back to
the middle between shots — by the time the ball arrives they are not there any
more. Replacing it with "try every return this paddle can produce, work out
where each would reach the far side, take the one that lands furthest from the
middle" took the match from 79 s to 43 s **with the game unchanged**.

So the warning in the document is not just "a timid controller under-reports".
It is "a mediocre controller will make you retune a game that was fine", and
that is a much more expensive failure. I would put that sentence in the
document.

**F16 — the staged-screens advice is exactly right and I would not have thought
of it.** The controller wins 5–0 every time, so the losing banner is drawn
zero times in the real run. Three lines per screen (tick once, set the
resource, draw) covers the winner's banner for both sides, both ends of the
serve countdown, and — separately — the one control that only exists on the
screen the run stops at: pressing Enter to start a new match. Nothing else in
the run could have pressed it.

---

## Things I wanted to look up in the source and did not

- Whether `ctx.circle`'s sixteen-wedge tessellation is fixed or scales with
  radius. My `disc_drawn` unions whatever it finds, so the check does not
  depend on the answer — but I wanted to know, and the answer is not anywhere I
  am allowed to read.
- Whether `FrameRecord::covering(point)` counts a quad whose edge passes exactly
  through the point. I sidestepped it by only ever asking about centres.
- What `Time::alpha` is *for*. It is defined ("how far into the next tick the
  last rendered frame fell") and nothing in the API consumes it — there is no
  interpolation helper, and `Draw` reads the world's committed state. So a
  fast-moving ball judders at the fixed timestep and the field for fixing that
  exists but has nothing to plug into. I ignored it.
- Whether `Rect::overlaps`'s "touching edges do not count" would bite a ball
  resting exactly on a wall. I avoided `overlaps` entirely, so I never found out.

## Things I expected to exist and could not find

- A `Rect` from `visible_bounds` (F9).
- Any sweep or segment-intersection primitive, and `Rect::inflate` (F4).
- `TextStyle::height_of`, or any statement of where glyphs sit inside `size`
  (F10).
- **Sound.** There is none in the document, so presumably none in v1. Pong
  without the blip is noticeably less of a game, and this is the one absence I
  felt as an author rather than as a programmer.
- A way to quit. The document is explicit that this is a v1 boundary rather than
  an omission, which is the right way to document an absence — I list it only
  because a Pong wants an Escape key and I could not give it one.

## What I could not check at all

I never saw the game. This machine has no display (`run` returns
`RunError::NoDisplay`, with a genuinely good four-part message) and no Vulkan
ICD, so `WgpuBackend::offscreen` has nothing to talk to either. I deliberately
did **not** add the PNG capture that `prototype_kit` has, because here it could
only ever print "skipped, no GPU on this machine" and I would be shipping a code
path I had never executed.

Everything I claim about how this game *looks* comes from
`FrameRecord::transcript()`. Everything I claim about how it *plays* comes from
one summary line: `LEFT beat RIGHT 5-0 in 43.3s of play, longest rally 6, ball
topped out at 30.2 units/s`. That is a lot better than nothing, and it is not
the same as having played it.
