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

To check what was *drawn*, render into `jidousha::testing::NullBackend`, which
records every frame as structured data. `FrameRecord::covering(point)` answers
"what is at this world position?" with exact rotated-quad containment, and
`transcript()` renders a frame as stable, diffable text. No GPU is involved, so
this runs anywhere.

`tools/verify <example>` is the whole loop as one command: scripted input, a
fixed number of headless ticks, the example's own assertions, and a captured PNG
if the machine has a GPU.
