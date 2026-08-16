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

`tools/verify <example>` is the whole loop as one command: scripted input, a
fixed number of headless ticks, the example's own assertions, and a captured PNG
if the machine has a GPU.
