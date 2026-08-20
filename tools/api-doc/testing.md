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

**A tick is cheap, and thousands of them are not something to budget for.** There
is no frame to wait for, no vsync and no window — a tick is the systems you wrote
and nothing else. A whole `--verify` — a 2,013-tick match, two more
headless runs, three staged screens and a GPU capture — takes about two seconds
in a *debug* build, with a controller rolling the game forward thirteen candidate
futures deep, up to four hundred ticks each, on every decision. So simulate
rather than solve: running the game forward and looking is allowed, and it is
usually both simpler and more honest than a closed form kept in step by hand.
Design for a slow tick and you will design around a cost that is not there.

**"Forward" means your game's own step functions, not a copy of the
simulation.** There is no way to fork a `HeadlessSim` — `World` is not
cloneable, `Recording` replays input rather than state, and rebuilding from
`headless(..)` and replaying to the current tick is quadratic. That is a
boundary rather than something you have missed, and the shape that works instead
is the one *Concepts* asks for while the game is still being written: the ball's
step and the opponent's decision as free functions the game owns, which a check
calls as often as it likes. `examples/slalom`'s controller is thirteen futures
deep and ticks nothing.

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

For "the player is there and doing nothing" — not the same as inserting no
`Input` at all, and what proves a game can be *lost* as well as won — the value is
`Input::new(InputSnapshot::new())`. A `SnapshotBuilder` with nothing recorded
yields the same thing from `first_tick_snapshot()`, so a controller that already
has a builder keeps using it rather than reaching for a second spelling.

**Making that player *good* is a document of its own**, and it is
`docs/api/jidousha-controllers.md`: a blind script never returns the ball, a
controller that plays safe measures its own caution rather than the game, and a
mediocre one reports a plausible wrong number that costs six rounds of tuning the
wrong half of the program. None of it is about this engine, which is why it is
not here. Read it when the check needs a player that can win;
`crates/jidousha/examples/slalom/` is the whole of it worked.

**On the way into tick 1 there is nothing to look at.** `Startup` runs inside
that first `tick()`, so the controller's read at the top of the loop happens
once against an empty world: `find_resource` rather than `resource`, and a query
that yields nothing rather than a `[0]` into an empty `Vec`. It is one tick out
of thousands and it is the first one, so a controller that gets this wrong
panics before it has tested anything.

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
run's last frame **and** build the screens the run never reached.
`recorder.frames()` is the other road to the same place and it *does* borrow the
recorder for as long as the reference lives, so anything taken out of it has to
be `.clone()`d before the next `draw`. `recorder.font_texture()` borrows nothing
and is free to call wherever you like.

**The frame a match ends on is not a picture of the game being played.** `last`
out of that loop is the frame somebody won on, so it carries the end screen
rather than the layout: 88 glyphs, 2 in the score band and 50 in the hint band,
leaves 36 unaccounted and they are the banner. Carry out the last frame drawn
while play was **live**, with the score and positions from that same tick, and
assert the ordinary layout against that. End screens get staged frames instead.

**Those two calls are the whole way in.** `draw` **is** the longer road — draw
the simulation, build a texture table, plan the frame, hand it to a backend —
walked for you and with the result kept: same submissions, same plan, same
arithmetic, and nothing in this surface asks you to walk it yourself.

The recorder keeps **every** frame, oldest first, with no way to forget them: a
six-hundred-tick check holds six hundred frames. That is deliberate and it is
affordable at prototype scale — the history is what a failing assertion reads
backwards, and the tick before the one that broke is usually the interesting one.

The recorder's viewport **overrides** the `Camera` resource's; everything else —
centre, height, clear color — is the game's own. Nothing writes the recorder's
viewport back into the world, so a check that reads bounds from
`world.resource::<Camera>()` and quads from the recorder is comparing against
the wrong rectangle unless the two viewports agree.

Two ways out, and take the first: give the recorder the size the game's camera
already has, and the question stops existing. When it cannot — a headless
viewport smaller than the window's, so a capture is cheap — rebuild the camera
the frame was *drawn* with rather than reading the resource raw. It is one line
at the top of every check that measures against `visible_bounds()`:

```rust
const HEADLESS_VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);
// The recorder's viewport, the game's everything else. Read it back after the
// ticks rather than before: a game may move or zoom its camera as it plays.
let camera = Camera { viewport: HEADLESS_VIEWPORT, ..*sim.world().resource::<Camera>() };
```

`frame.covering(point)` answers "what is at this world position?" with exact
rotated-quad containment, and `frame.quads()` hands you every quad with its
`bounds()` and `tint`.

**Two things are called `transcript` and they are not the same size.**
`frame.transcript()` — on a `FrameRecord` — renders **that one frame** as stable,
diffable text, one line per quad's world-space extent: the closest thing to a
screenshot on a machine with no display. `recorder.transcript()` renders **every
frame it holds**, each headed `frame N:` — the history, for a failure that needs
the ticks before the one that broke. One run printed 1,263 of them as 121,465
lines without noticing. Print the frame; keep the recorder.

**And the order your systems run in is assertable too.** Concepts asks a game
with a swept collision to pick whether the collider counts as pre- or post-move,
and to say so at the site; `sim.schedule_debug()` returns every phase and its
systems in run order as a string, so a check can hold the game to the order it
picked. Assert that the mover you decided goes first appears before the other in
it — system names appear verbatim, one per line, numbered within their phase:

```text
schedule:
  Startup (1)
    0. set_the_scene
  Update (6)
    0. restart_the_match
    1. drive_the_player
```

so `order.find("drive_the_player") < order.find("move_the_ball")` is the check.
Assert that both names were *found* as well: two renamed systems give two
`None`s, which compare equal, and the check then passes while seeing nothing.
Nothing else in this surface sees a swap of two `add_system` calls — the
world ends up in a legal state either way, one tick of a paddle's travel apart,
and every assertion about where things ended up passes.

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

**Which has a consequence about your own frames that is easy to miss: a band is
only visible where it changes the order.** The sort is `(layer, z, submission
index)`, so wherever the bands already agree with the order the game submitted
in, `quads()` *is* the submission order and no assertion over drawn quads can
see a band at all. Move a winner's banner from the UI band down to the field
band, on a screen where it was submitted last and nothing else was drawn where it
was, and no check can tell: it was already last in the order and it still is.
Both spellings inherit this. `covering(p)` needs two bands to cover the same point;
comparing indices in `quads()` does not need the overlap, but still needs a pair
whose order the bands decide rather than the submission sequence.

So arrange the disagreement rather than hoping for it. Two shapes work, and a
game usually wants both. **Draw something that must be behind after the thing in
front** — a field marking submitted after the ball it sits under is a pair whose
order only the band can produce, and one index comparison tests it. And **stage a
frame for the overlaps a played session never produces**: put the ball under the
hint line, put it on a centre-line dash, draw one frame each, and ask
`covering(p)[0]` which won. That is three lines per band boundary, next to the
screens-you-never-reach frames below, and it is the difference between a layer
that is checked and a layer that is merely spelled.

**"A quad the size of the thing is at the thing's position" is how you check a
rectangle, and it is wrong for a circle.** `ctx.circle` submits sixteen wedge
quads, not one square, so nothing the size of the ball is drawn anywhere. What is
true is that all sixteen share the centre as a corner and all sixteen fit inside
the circle's bounding box, so the box around the quads covering the centre is
exactly `2r × 2r`. `find_bounds` is that box — the fold over `quad.bounds()`
that "how big is the thing that was drawn" always comes down to, and `None` when
nothing was drawn there at all:

```rust
let box_of_it = Rect::from_center_size(at, Vec2::splat(radius * 2.0));
let disc = find_bounds(frame.covering(at).into_iter().filter(|quad| {
    // Inside the disc's box, a hair of slack for the rim's arithmetic. Written
    // out rather than as `Rect::contains`, which is half-open and would throw
    // away the one wedge that reaches the far edge. A filter, because a centre
    // line running under the ball covers the same point and is not the ball.
    let drawn = quad.bounds();
    drawn.min.x >= box_of_it.min.x - 1e-3
        && drawn.min.y >= box_of_it.min.y - 1e-3
        && drawn.max.x <= box_of_it.max.x + 1e-3
        && drawn.max.y <= box_of_it.max.y + 1e-3
}));
let size = disc.expect("nothing at all was drawn where the ball is").size();
assert!(
    (size.x - radius * 2.0).abs() < 1e-3 && (size.y - radius * 2.0).abs() < 1e-3,
    "no ball-sized disc at ({at:?}): the quads covering it span {size:?}, want \
     {:?} square",
    radius * 2.0,
);
```

The same call answers the same question for a string — `ctx.text` is one quad
per character, so `find_bounds(quads sampling the font)` is where the score
actually sits and how wide it actually is — and for anything else a game draws
out of several primitives.

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

**And count the expected quads in `chars()`, never `len()`.** `ctx.text` submits
one per *character*; `str::len` is *bytes*. `drawn == HINT.len()` is right for
pure ASCII and wrong for exactly the input the check above exists for — an em
dash is one quad and three bytes — so the two contradict each other on the one
string that matters, and the count fires first with a number unrelated to the
fault.

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

**And a staged frame is not staged until all of it is.** That recipe is additive
— tick, insert, draw — while the frames a game needs are usually *corrective*:
whatever the run left behind is still set. A check parked the ball on a
centre-line dash and asked `covering(p)[0]` which quad won; the answer was a
glyph of the **winning banner**, because the match had ended and the stage
resource still said so. Twenty minutes went into re-reading correct paddle code.
Set every piece of state the frame depends on, including the state you are not
asking about.

**Then check the contracts your run never exercises.** Those screens are the
visible half of a general problem: **a run only tests the states it reaches, and
the safety margins a game is built on are exactly the states a correct game never
reaches.** Write the swept collision test Concepts asks for, then cap the ball at
0.55 units of travel per tick against a paddle 0.7 thick, and the ball *cannot*
tunnel: the sweep never does anything a naive position test would not, and
replacing it with a position-only one passes the entire session — every
assertion, the same 5–0, every drawn frame. The margin is real and a played
session cannot see it. So ask the function its contract directly
rather than hoping play reaches the case: one tick of travel eight units long
across that same paddle, plus the two negative cases — past the end of it, and
leaving through the same face — is three calls and no match at all. It will be
the only check in the file that is not about a played game.

**Mutate the game and check the run notices — and commit before you start.** The
cheapest way to find out whether a `--verify` file is an instrument or a
decoration is to break the game on purpose — one constant, one sign, one swapped
constraint — and see whether the run says so. The natural way back from each
mutation is `git checkout -- <file>`, which also throws away every *uncommitted*
change in that file, including the check you wrote ten minutes ago to catch the
fault you are injecting now. Commit **every** file the harness touches, not just
the one holding the checks: the revert eats an uncommitted fix in the *game* file
just as happily. Two more things the harness itself has to get right, both silent
when it does not: a search-and-replace that matches nothing writes the file back
unchanged and reports success, so make a miss an error rather than a no-op; and a
mutation that does not compile is not a caught fault, so tell a failed build apart
from a failed check before counting it.

It is worth doing, because the answers are not guessable. A file that catches
seventeen of seventeen injected faults usually gets there only after two checks
written carefully and believed thorough turn out to be loose. The swept test
above is one. The other is a paddle drawn half out of position, which passes a
"paddle-sized quad covers this point" check for the reason such a check always
passes — a paddle still covers its own centre when it is displaced. Assert on the
quad's *bounds*, not on the fact that something is there.

**"On screen" is not "in the right place".** The bounds check passes for anything
inside the camera, including a hint line drawn on top of a wall. If a layout has
constants — a field edge, a margin, a band the score lives in — assert quads
against *those* rather than only against the camera, because otherwise the
transcript is the only instrument and reading it means holding a hundred lines of
coordinates in your head.

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
whole difference, and it is worth the dozen lines: an instrument that stops at
the first bad reading costs a cycle per fault. Keep a `Vec` of failures, push
into it, print them all in the four-part shape at the end, and return `FAILURE`
if it is not empty. One deliberate break can produce six reported problems with
the diagnostic one **fourth**; exiting first shows only "no one won the match",
the conclusion rather than the fault. The exception is a reading that makes the
rest meaningless — a missing entity, a frame never recorded. Stop there because
there is nothing left to measure, not because something is wrong.

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

**The picture is yours to take.** `tools/verify` renders nothing: it reads one
line out of what your `--verify` mode printed, and a run that captures nothing
passes. So a frame you can *look* at is one your `--verify` mode drew — worth
doing, because a picture answers what no assertion here reaches: whether it looks
like the game.

**And the frame you already recorded is the one to draw.** You do not replay the
session, and you do not restructure your game to hand it a renderer.
`FrameRecord` carries a `plan` — the finished frame, with the depth sort and the
batching already done — and a renderer built for the purpose will execute it:

```rust
// `PhysicalSize` is not in this list: it is in the prelude, which a game's
// `--verify` file already globs, and taking it from `testing` as well is the
// same item twice. Only the testing-only names belong here.
use jidousha::testing::{
    FONT_TEXTURE, RenderBackend, RenderError, WgpuBackend, create_builtin_textures, encode_png,
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
// inside the plan mean the same thing here — and one assertion that they do,
// which is what separates "a PNG was written" from "a PNG of this game".
let textures = create_builtin_textures(&mut gpu);
assert_eq!(textures.resolve(FONT_TEXTURE), recorder.font_texture());
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
because both counters start empty and are filled by the same call in the same
order. That is true of **a game that loads no assets** — every shape a colour,
every string the built-in font. If yours loads art, the replay has to upload it
too, or the plan names a texture the new renderer lacks. The assertion above is
that step, and it costs one line whether or not you have art: without it, a plan
whose ids drifted renders the wrong texture into a PNG that every other check in
your `--verify` is happy with.

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
requirement rather than the constant, and the pair survives the constant
changing. Written in the first form alone, the clear colour is exactly the fault
that escapes a mutation round.

**Colour is where the trap is easiest to see; layout is where it bites.** A score
drawn at `SCORE_TOP` and checked with `quad.min.y < SCORE_TOP + margin` moves
with its constant — put `SCORE_TOP` in the middle of the court and the check
follows it down, passes, and leaves the score across the play. A game that had
guarded its clear colour correctly walked into this anyway, reading the pairing
as advice about colours. The requirement names no constant the game owns: the
score sits in the **top third of `visible_bounds()`**, one number either side of
the centre line, evenly set.

**And state the requirement where the game actually operates, not at its most
favourable point.** A requirement stated at a boundary is a requirement about a
case that hardly ever happens, so it passes for a game that fails everywhere
else. A winnability check asking whether the player can reach the fastest ball
the game produces — a speed a rally touches only at its very end — passes for
opponents that cannot be scored against inside a minute. Where the arithmetic
needs a precision the game
does not permit, stop deriving and **measure**: "the opponent returns at least
half the balls that reach it" is a number the run already has, and it is about
the game as played rather than about the game at its limit.
