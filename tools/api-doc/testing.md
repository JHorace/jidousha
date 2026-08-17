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

Assets are scripted the same way: `MemorySource` lets a test say "this texture
becomes ready at tick 30", so loading behaviour — placeholders, gates, the frame
a sprite appears — is something to assert on rather than a race.

To check what was *drawn*, draw the game into a `jidousha::testing::FrameRecorder`,
which records every frame as structured data. No GPU and no window is involved,
so this runs anywhere:

```rust
let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
for tick in 1..=600 {
    sim.world_mut().insert_resource(Input::new(script.snapshot_at(tick)));
    sim.tick();
    recorder.draw(&mut sim);          // one frame, recorded
}
let frame = recorder.frames().last().expect("600 frames were drawn");
```

The recorder's viewport **overrides** the `Camera` resource's; everything else —
centre, height, clear color — is the game's own. Nothing writes the recorder's
viewport back into the world, so a check that reads bounds from
`world.resource::<Camera>()` and quads from the recorder is comparing against
the wrong rectangle unless the two viewports agree. Give the recorder the size
the game's camera already has, and the question stops existing.

`frame.covering(point)` answers "what is at this world position?" with exact
rotated-quad containment, `frame.quads()` hands you every quad with its
`bounds()` and `tint`, and `recorder.transcript()` renders the last frame as
stable, diffable text — every quad's world-space extent, one per line. That
transcript is the closest thing to a screenshot available on a machine with no
display, and it is good enough to check a layout by eye.

To ask whether any of it was *text*, compare a quad's texture against
`recorder.font_texture()`: the font atlas is a texture like any other, so a quad
sampling it came from `ctx.text` and nothing else could have produced it.

A game with art also calls `recorder.settle_assets(&mut sim, tick)` before each
`draw`, which is what makes a texture that became ready on this tick appear in
this frame. A game of shapes and text never needs it.

**Assert that nothing is drawn outside `Camera::visible_bounds()`.** It is the
highest-value check a game of shapes and text can write, and it is six lines:

```rust
let (top_left, bottom_right) = camera.visible_bounds();
for quad in frame.quads() {
    let bounds = quad.bounds();
    assert!(
        bounds.min.x >= top_left.x && bounds.min.y >= top_left.y
            && bounds.max.x <= bottom_right.x && bounds.max.y <= bottom_right.y,
        "drawn off screen: {bounds:?} against a camera showing {top_left:?}..{bottom_right:?} \
         — text centred by width_of is the usual culprit",
    );
}
```

`TextStyle::width_of` is exact and completely silent: centring by it is the
documented idiom, and a banner one character too long runs off both edges
without a word from anything. A game that shipped exactly that had eight other
assertions passing — glyphs existed, the score was placed, the world was
correct — and only this one would have caught it.

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

**A failing assertion has to report the numbers it judged.** Nobody writing a
game this way can look at it; the assertion is the only instrument there is, so
a message that says only *this is wrong* costs a whole cycle to turn into a
diagnosis. "No one won after a hundred seconds" says nothing. "No one won: score
0–0, longest rally 14 touches, top ball speed 25.6 units/s" says the ball is too
slow for the field, and says it immediately. Print the quantities the condition
looked at, not the conclusion it reached.

`tools/verify <example>` is the whole loop as one command: scripted input, a
fixed number of headless ticks, the example's own assertions, and a captured PNG
if the machine has a GPU.
