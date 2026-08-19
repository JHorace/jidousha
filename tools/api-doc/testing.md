A game written this way is testable without a window. `headless(config, setup)`
builds the same game the window runs and gives you `HeadlessSim`: call `tick()`
as many times as you like and assert on the world afterwards.

```rust
let mut sim = headless(GameConfig::default(), |app| {
    app.add_system(Update, my_system);
});
for _ in 0..60 { sim.tick(); }
assert_eq!(sim.world().resource::<Score>().0, 3);
```

Input comes from `jidousha::testing::InputScript`, which is a pure function of
the tick — no cursor, so a test can seek, replay and bisect freely:

```rust
let script = InputScript::new().hold(Key::D, 10..120).press(Key::Space, 30);
sim.world_mut().insert_resource(Input::new(script.snapshot_at(tick)));
```

A script says what the *player* does, fixed before the run starts. When the
input has to respond to the game — and it does the moment you want to know
whether the game is **playable**, because a blind script never returns a ball —
use `jidousha::testing::SnapshotBuilder` instead. It is the driver's own
accumulator, so a controller written with it goes through the same edge rules a
real keyboard does:

```rust
let mut keyboard = SnapshotBuilder::new();
let mut holding = false;
for _tick in 1..=TICKS {
    let want = /* look at the world, then decide */ true;
    if want != holding {
        keyboard.record(if want {
            InputEvent::KeyPressed(Key::S)
        } else {
            InputEvent::KeyReleased(Key::S)
        });
        holding = want;
    }
    sim.world_mut().insert_resource(Input::new(keyboard.first_tick_snapshot()));
    sim.tick();
}
```

Send events, not states — that is why the controller tracks `holding`, and it is
what makes a key held for a hundred ticks press exactly once. Building a
one-tick script per tick instead (`hold(key, tick..tick + 1)`) puts a press edge
on *every* tick, because every tick is the start of its own range.
`examples/scripted_player.rs` runs both shapes side by side.

For "the player is there and doing nothing" — which is not the same as inserting
no `Input` at all, and is what proves a game can be *lost* as well as won — the
value is `Input::new(InputSnapshot::new())`. A `SnapshotBuilder` with nothing
recorded yields the same thing from `first_tick_snapshot()`, so a controller that
already has a builder keeps using it rather than reaching for a second spelling:
that call is the builder's own first step, not a second way to say "idle".

**On the way into tick 1 there is nothing to look at.** `Startup` runs inside
that first `tick()`, so the controller's read at the top of the loop happens
once against an empty world: `find_resource` rather than `resource`, and a query
that yields nothing rather than a `[0]` into an empty `Vec`. It is one tick out
of thousands and it is the first one, so a controller that gets this wrong
panics before it has tested anything.

**A controller that plays it safe is not a playability test.** A blind script
never returns the ball; a controller that tracks the ball perfectly returns it
*dead flat*, straight back down the middle, and if the opponent tracks too the
rally has nowhere to go — both sides hold a groove neither can lose, and the run
ends 0–0 with a 78-touch rally and a report that the game is unplayable. The
game is fine; the controller made it degenerate. Play to **win**: aim the return
away from where the opponent is standing, meet the ball with the half of the
paddle that sends it off-centre, take the shot a person would take. The same
trap wears other clothes — a driver that brakes for every corner never finds the
top speed, a fighter that blocks everything never tests a combo — and in each
case the thing being measured is the controller's caution, not the game.

**And the bill does not stop at one bad verdict.** A merely mediocre controller
does not report "unplayable" — it reports a plausible wrong number, and then you
tune the game to fix it. One run aimed its returns away from wherever the
opponent was *currently* standing, which is worthless against an opponent that
drifts back to the middle between shots, and reported a correct game taking 79
seconds a match. Six tuning runs went into the game's speed constants before the
fault was found in the driver; replacing the aim with "try every return this
paddle can produce, take the one that lands furthest from the middle" took the
match to 43 seconds **with the game byte-identical**. So the controller is not
just an instrument that can under-read. It is an instrument that will send you
off to change the thing you are measuring. Get it playing to win *before* you
believe any number it prints, and when a number looks wrong, suspect the
controller first — it is the newer and worse-tested of the two.

**And "take the best shot available" will lose you the match, because the best
shot is on the edge of what the paddle can do.** The next run wrote exactly that
search — thirteen contact points across the paddle, each pushed through the
game's own bounce function, take the one landing furthest from the opponent —
and went down **0–5**, making six returns in a whole minute. The cause is
structural rather than a bug: the sharpest return a paddle can produce is always
the one struck at its very tip, because that is where the bounce angle is widest,
so "the best shot available" resolves every single time to "stand so the ball
hits your last millimetre". The optimum sits on the *boundary* of the feasible
set, and on that boundary any error at all — half a tick of overshoot, a dead
band — is a clean miss rather than a worse return.

So **constrain first, then optimise**. Score only the positions that (a) really
make contact, with margin — a fixed fraction of the paddle's half-length, so the
tip is not on the menu — and (b) can be reached before the ball arrives.
Optimise inside what survives both, and when nothing does, run at the ball. That
is three lines of set arithmetic in front of the search, and it is the whole
difference between a controller that reports your game is unwinnable and one
that wins 5–0 with the game byte-identical.

**Reading this is not the same as it working, so make it something the run does
rather than something you remember.** The run that lost 0–5 had read the
suspect-the-controller paragraph above the same morning; it still went and
changed three of the game's speed constants and added a whole new difficulty
knob before finding the fault in its own planner. Prose has now failed at this
three times. What does not fail is an assertion: a controller is code with a
contract like any other, so check the contract on the numbers it actually picked,
every tick. "My aim missed the ball on 94% of returns" is a controller reporting
its own fault; "the game is unwinnable" is the same fault reported as a fact
about your game, and only one of those sends you into the constants.

Assets are scripted the same way: `MemorySource` lets a test say "this texture
becomes ready at tick 30", so loading behaviour — placeholders, gates, the frame
a sprite appears — is something to assert on rather than a race.

To check what was *drawn*, draw the game into a `jidousha::testing::FrameRecorder`,
which records every frame as structured data. No GPU and no window is involved,
so this runs anywhere:

```rust
let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
let mut last = None;
for tick in 1..=600 {
    sim.world_mut().insert_resource(Input::new(script.snapshot_at(tick)));
    sim.tick();
    last = Some(recorder.draw(&mut sim));   // one frame, recorded and handed back
}
let frame = last.expect("600 frames were drawn");
```

**`draw` hands back the frame itself, so keep the one you want.** It stays
readable however many more you draw, which is what lets one function inspect the
run's last frame **and** build the screens the run never reached — the shape
recommended twice below. `recorder.frames()` is the other road to the same place
and it *does* borrow the recorder for as long as the reference lives, so anything
taken out of it has to be `.clone()`d before the next `draw`.
`recorder.font_texture()` hands back a plain id and borrows nothing, so it is
free to call wherever you like; reading it out once before the loop just keeps
the assertions below it short.

The recorder keeps **every** frame, oldest first, with no way to forget them: a
six-hundred-tick check holds six hundred frames. That is deliberate and it is
affordable at prototype scale — the history is what a failing assertion reads
backwards, and the tick before the one that broke is usually the interesting one.

The recorder's viewport **overrides** the `Camera` resource's; everything else —
centre, height, clear color — is the game's own. Nothing writes the recorder's
viewport back into the world, so a check that reads bounds from
`world.resource::<Camera>()` and quads from the recorder is comparing against
the wrong rectangle unless the two viewports agree. Give the recorder the size
the game's camera already has, and the question stops existing.

`frame.covering(point)` answers "what is at this world position?" with exact
rotated-quad containment, and `frame.quads()` hands you every quad with its
`bounds()` and `tint`.

**Two things are called `transcript` and they are not the same size.**
`frame.transcript()` — on a `FrameRecord` — renders **that one frame** as stable,
diffable text, every quad's world-space extent one per line. That is the closest
thing to a screenshot available on a machine with no display, and it is good
enough to check a layout by eye. `recorder.transcript()` renders **every frame
the recorder holds**, each headed `frame N:`; it is the history, for a failure
that needs the ticks before the one that broke. A six-hundred-tick check has six
hundred frames in it, and one run printed 1,263 of them as 121,465 lines without
noticing, because the `--verify` convention below keeps the transcript as
evidence rather than showing it. Print the frame; keep the recorder.

**A recorded frame does show draw order, and that is how you check a layer.**
`quads()` comes back in the depth sort — `layer`, then `z`, then submission order
as the tie-break — so a quad's index in it is its place in the painter's
sequence, and the later of two indices is the one drawn over the other.
`covering(point)` is that same order read backwards, so `covering(p)[0]` is what
a player looking at `p` actually sees. Both spellings answer "is the score behind
the ball?", which is otherwise the failure nothing catches: move the score from
the table band to the UI band and it paints over the ball, in the right place, at
the right size, with every geometric assertion still passing. What a frame does
*not* carry is the `Depth` that produced the order, deliberately — a `layer`
number read back only says the game submitted what the game submitted, and would
pass just as happily for a `mod layers` whose constants are in the wrong order.

**"A quad the size of the thing is at the thing's position" is how you check a
rectangle, and it is wrong for a circle.** `ctx.circle` submits sixteen wedge
quads, not one square, so nothing the size of the ball is drawn anywhere. What is
true is that all sixteen share the centre as a corner and all sixteen fit inside
the circle's bounding box, so the union of the quads covering the centre is
exactly `2r × 2r`. Ask about the union:

```rust
let box_of_it = Rect::from_center_size(at, Vec2::splat(radius * 2.0));
let mut union: Option<Rect> = None;
for quad in frame.covering(at) {
    let drawn = quad.bounds();
    // Inside the disc's box, a hair of slack for the rim's arithmetic. Written
    // out rather than as `Rect::contains`, which is half-open and would throw
    // away the one wedge that reaches the far edge.
    let inside = drawn.min.x >= box_of_it.min.x - 1e-3
        && drawn.min.y >= box_of_it.min.y - 1e-3
        && drawn.max.x <= box_of_it.max.x + 1e-3
        && drawn.max.y <= box_of_it.max.y + 1e-3;
    if !inside {
        continue;                       // the field behind the ball, not the ball
    }
    union = Some(match union {
        None => drawn,
        Some(so_far) => Rect { min: so_far.min.min(drawn.min), max: so_far.max.max(drawn.max) },
    });
}
let size = union.expect("nothing at all was drawn where the ball is").size();
assert!(
    (size.x - radius * 2.0).abs() < 1e-3 && (size.y - radius * 2.0).abs() < 1e-3,
    "no ball-sized disc at ({at:?}): the quads covering it span {size:?}, want \
     {:?} square",
    radius * 2.0,
);
```

`covering` counts a quad whose edge or corner passes exactly through the point,
which is what makes asking about the centre work at all — every wedge touches it.
`Rect::contains` is the other way round, half-open so that adjacent rectangles
never both claim a point, which is why the box test above is spelled out.

To ask whether any of it was *text*, compare a quad's texture against
`recorder.font_texture()`: the font atlas is a texture like any other, so a quad
sampling it came from `ctx.text` and nothing else could have produced it.

A game with art also calls `recorder.settle_assets(&mut sim, tick)` before each
`draw`, which is what makes a texture that became ready on this tick appear in
this frame. A game of shapes and text never needs it.

**Assert that nothing is drawn outside `Camera::visible_bounds()`.** It is the
highest-value check a game of shapes and text can write, and it is three lines:

```rust
let view = camera.visible_bounds();          // a Rect: min top-left, max bottom-right
for quad in frame.quads() {
    let bounds = quad.bounds();
    assert!(
        view.contains_rect(bounds),
        "drawn off screen: {bounds:?} against a camera showing {view:?} \
         — text centred by width_of is the usual culprit",
    );
}
```

`contains_rect` is closed on all four sides, because a quad flush against the
camera's edge is on screen. `Rect::contains`, which takes a point, is half-open
instead — it partitions space so adjacent rectangles never both claim a point,
which is a different question and the wrong rule here.

`TextStyle::width_of` is exact and completely silent: centring by it is the
documented idiom, and a banner one character too long runs off both edges
without a word from anything. A game that shipped exactly that had eight other
assertions passing — glyphs existed, the score was placed, the world was
correct — and only this one would have caught it.

**And centring a multi-line block by `width_of` centres only its longest line.**
`width_of` is the width of the widest line, and `ctx.text` lays a block out from
one top-left corner — so subtracting half of it puts the longest line in the
middle and hangs every shorter line off to the left. A two-line banner of uneven
lengths draws visibly crooked while staying on screen, at the right size, with
the bounds check, the glyph count and the printable-ASCII check all passing.
This is a different failure from the overrun and no assertion over drawn quads
distinguishes it from a layout that meant it. One `ctx.text` call per line, each
centred by its own width, is the fix and it is three lines.

**No assertion over drawn quads can see a wrong character.** The font covers
space through `~` and draws everything else as a box, at exactly the advance of a
letter — so a stray em dash, curly quote or middle dot produces a quad the right
size in the right place, and glyph counts, `width_of` centring and the bounds
check above all pass identically. The geometry is correct; the picture is not. The
check has to look at the string rather than at the frame, and it is one line:

```rust
assert!(
    HINT.chars().all(|c| (' '..='~').contains(&c)),
    "unprintable character in {HINT:?} — the font draws a box, and no assertion \
     over what was drawn can tell the difference",
);
```

Worth running over every literal a game draws, because the habit that produces
one is typing prose. `—`, `’` and `·` are the three that arrive uninvited.

**Then check the screens your run never reaches.** The bounds assertion above
only judges frames that were drawn, and a controller good enough to finish the
game is a controller that never loses it: a run that wins 5–0 draws the winning
banner five thousand times and the losing one never, so the longest string in
the game is the one string nothing measured. Build those screens by hand — one
tick so `Startup` has run, set the resource that selects the screen, draw one
frame, and run the same check over it:

```rust
sim.tick();                                        // Startup, so the world exists
sim.world_mut().insert_resource(Scoreboard { left: 0, right: 5 });
recorder.draw(&mut sim);                           // one frame of the screen nobody reached
```

Three lines per screen, and it is the losing banner, the timeout banner and the
paused overlay that need it.

**Then check the contracts your run never exercises.** Those screens are the
visible half of a general problem: **a run only tests the states it reaches, and
the safety margins a game is built on are exactly the states a correct game never
reaches.** One run wrote the swept collision test Concepts asks for, and also
capped its ball at 0.55 units of travel per tick against a paddle 0.7 thick — so
the ball *could not* tunnel, the sweep never did anything a naive position test
would not, and replacing the swept test with a position-only one passed the
entire session: every assertion, the same 5–0, every drawn frame. The margin was
real and the run could not see it. So ask the function its contract directly
rather than hoping play reaches the case: one tick of travel eight units long
across that same paddle, plus the two negative cases — past the end of it, and
leaving through the same face — is three calls and no match at all. It will be
the only check in the file that is not about a played game.

**Mutate the game and check the run notices.** The cheapest way to find out
whether a `--verify` file is an instrument or a decoration is to break the game
on purpose — one constant, one sign, one swapped constraint — and see whether the
run says so. It is worth doing, because the answers are not guessable: one run
broke its own game seventeen ways and caught all seventeen, but only after
tightening two checks it had written carefully and believed were thorough. The
swept test above was one. The other was a paddle drawn half out of position,
which passed a "paddle-sized quad covers this point" check for the reason such a
check always passes — a paddle still covers its own centre when it is displaced.
Assert on the quad's *bounds*, not on the fact that something is there.

**"On screen" is not "in the right place".** The bounds check passes for anything
inside the camera, including a hint line drawn on top of a wall. If a layout has
constants — a field edge, a margin, a band the score lives in — assert quads
against *those* rather than only against the camera, because otherwise the
transcript is the only instrument and reading it means holding a hundred lines of
coordinates in your head. One run found exactly that bug that way, after every
assertion it had passed.

**A failing assertion has to report the numbers it judged.** Nobody writing a
game this way can look at it; the assertion is the only instrument there is, so
a message that says only *this is wrong* costs a whole cycle to turn into a
diagnosis. "No one won after a hundred seconds" says nothing. "No one won: score
0–0, longest rally 14 touches, top ball speed 25.6 units/s" says the ball is too
slow for the field, and says it immediately. Print the quantities the condition
looked at, not the conclusion it reached.

**The loop has a name, and it is a mode your game implements.** By convention a
game takes a `--verify` flag: with it, `main` skips `run` entirely and does the
headless thing instead — script or drive the input, tick a fixed number of times,
run every assertion above, print a one-line verdict and then the frame
transcript. Without it, `main` opens a window as usual. Nothing in the engine
enforces this; it is the shape the tooling expects:

```rust
fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--verify") {
        return verify::run();          // ticks, asserts, prints "verified ...";
    }                                  // SUCCESS, or FAILURE if a check failed
    match run(GameConfig::default(), setup) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");      // Display, not Debug — see the Quickstart
            ExitCode::FAILURE
        }
    }
}
```

**Collect the failures; do not exit on the first one.** `verify::run` returning
an `ExitCode` rather than calling `process::exit` on the first bad check is the
whole of the difference, and it is worth the dozen lines: an instrument that
stops at the first bad reading costs a cycle per fault, for exactly the reason a
message that reports a conclusion instead of numbers does. Keep a `Vec` of
failures, push into it, print all of them in the four-part shape at the end, and
return `FAILURE` if it is not empty. One run measured this: a single deliberate
break produced six reported problems, and the precisely diagnostic one — "a ball
that misses the paddle is counted as a hit" — was **fourth**. Exiting first would
have shown only "no one won the match", which is the conclusion rather than the
fault. The exception is a reading that makes the rest meaningless: a missing
entity, a frame that was never recorded. Stop there, because there is nothing
left to measure — not because something is wrong.

The verdict line must begin with `verified ` — that is the token the wrapper
looks for, so an example that quietly ignored the flag and opened a window is
reported as a tooling fault rather than as a pass. Indented lines immediately
after it are the summary and are shown; everything after that is kept as
evidence rather than reprinted, which is where the transcript goes —
`frame.transcript()`, one frame, unless a failure genuinely wants the history.

`tools/verify <example>` is then the whole loop as one command: it runs that mode
under a timeout, parses the verdict, writes a report, and lifts the path of any
picture the run captured into a field of its own in that report.
`cargo run -p <crate> --example <name> -- --verify` is the same thing by hand.

**The picture is yours to take.** `tools/verify` renders nothing — it has no game
and no renderer, and all it does about pictures is read one line out of what your
`--verify` mode printed. A run that captures nothing is reported as capturing
nothing, and passes. So if you want a frame you can *look* at rather than only
assert on, your `--verify` mode has to draw one — and it is worth doing, because a
picture answers what no assertion here can reach: whether it looks like the
game.

**And the frame you already recorded is the one to draw.** You do not replay the
session, and you do not restructure your game to hand it a renderer.
`FrameRecord` carries a `plan` — the finished frame, with the depth sort and the
batching already done — and a renderer built for the purpose will execute it:

```rust
use jidousha::testing::{
    PhysicalSize, RenderBackend, RenderError, WgpuBackend, create_builtin_textures, encode_png,
};

// Same 16:9 shape as the recorder's viewport — see the first trap below.
let mut gpu = WgpuBackend::offscreen(PhysicalSize::new(480, 270));
for _ in 0..10_000 {                    // the renderer is poll-based, and a
    match gpu.poll() {                  // `--verify` run has no frame loop
        Ok(()) if gpu.is_ready() => break,
        Ok(()) => {}
        // No adapter is a fact about the machine, not a failure. Every other
        // error here is a fault — see the third trap below.
        Err(error @ RenderError::NoAdapter { .. }) => {
            return format!("skipped, no GPU on this machine ({error})");
        }
        Err(error) => return format!("skipped, the handshake failed ({error})"),
    }
}
// The built-in textures, in the order your recorder created them, so the ids
// inside the plan mean the same thing here. A game of shapes and text needs
// nothing else; the table it returns is not used.
let _ = create_builtin_textures(&mut gpu);
gpu.render(&frame.plan).expect("the plan the recorder already accepted");
let image = gpu.capture().expect("an offscreen renderer reads its own target");
std::fs::write("target/verify/mygame.png", encode_png(&image))?;
```

`examples/prototype_kit/capture.rs` is that with its reasoning written down. It
also shows the other shape available to you: because its own `play` is handed the
renderer, it can run the whole session twice and check that the world did the
same thing both times. Replaying the recorded plan is the cheaper road and the
one that works whatever shape your game is.

That "the ids mean the same thing" step is the load-bearing one, and it holds
because both counters start empty and both are filled by the same call in the same
fixed order. It holds for **a game that loads no assets** — every shape a colour,
every string the built-in font. If yours loads art, the replay has to upload that
art too, or the plan names a texture the new renderer does not have. Check the ids
rather than assuming them; the example does.

Three things are easy to get wrong here, and silent when you do:

- **Capture at the recorder's aspect ratio.** The projection was computed from the
  viewport you handed `FrameRecorder::new` and is baked into every plan; nothing
  downstream can recompute it. A capture of another shape stretches the picture
  while every assertion you wrote goes on passing, because none of them look at
  pixels. 480x270 for a 1280x720 recorder — and assert the ratio rather than
  remembering it.
- **Print the path in the line the tool reads.** `tools/verify` takes the first
  line whose text starts with `capture:` and contains ` written to `, and puts
  what follows into the report. Word it differently and the run still passes while
  the report says no picture was taken.
- **A machine with no GPU must still pass, and a broken one must not.** Every
  runner is headless and some have no graphics stack at all, so
  `RenderError::NoAdapter` is a fact about the machine: say the capture was
  skipped, put that in the summary, keep the run green — and do not skip in
  silence either. Every *other* handshake error is a fault, and reporting one of
  those as "no GPU here" files a real problem as a property of the hardware, on
  every machine, for ever. Match on the variant; it is in the Testing reference
  with the rest.

**Then open the file and look at it.** A capture path that writes a PNG is worth
nothing on its own; the question is whether it writes *your game's* PNG, and a
path wired to the wrong frame or to a stale plan passes every check that does not
ask. So look — name what you see — then break the game on purpose and look again:
move a paddle, stop drawing the score, change the clear colour, and confirm the
picture follows.

**The clear colour is the one part of the picture that leaves no quad behind, and
it is still assertable.** A frame drawn on the wrong background is byte-identical
under every check above, because none of them look at the background — but
`FrameRecord` carries the `plan` it was drawn into, and a `FramePlan` carries the
`clear_color` the camera asked for. So it is one line, and it needs no capture and
no GPU:

```rust
assert_eq!(frame.plan.clear_color, palette::COURT);
```

The capture and that assertion answer different questions and both are cheap: the
picture says whether the frame *looks* right, and the plan says whether it cleared
to the colour the camera asked for.

**That assertion in its naive form is a trap, and the shape of the trap is
general.** Comparing what was drawn against the game's own constant does not
survive somebody changing the constant — the check and the thing it checks move
together, and a mutation walks straight through. It is still worth writing, because
it catches a camera set from the *wrong* constant. What bites is a second check
the constant cannot move: one that states the game's own requirement in numbers.
Here that is "the court has to be dark enough for a white ball to read against":

```rust
let cleared = frame.plan.clear_color;
let brightness = cleared.r.max(cleared.g).max(cleared.b);
assert!(
    brightness < 0.25 && cleared.a > 0.99,
    "the court is not dark enough to see a white ball on: brightest channel \
     {brightness:.3} at alpha {:.2}",
    cleared.a,
);
```

Any check spelled `assert_eq!(what_was_drawn, the_constant_that_drew_it)` has this
shape — a size, a position, a speed cap, a colour. Pair it with one that names the
requirement rather than the constant, and the pair survives the constant changing.
One run wrote only the first form and reported it: of seventeen deliberate faults
it injected, the clear colour was the one that escaped.
