# ADR-0019: A closed-loop test builds input with `SnapshotBuilder`, not a second `InputSnapshot` constructor

Status: accepted · 2026-08-17

## Context

`InputSnapshot::new()` is "the player did nothing" and every other method on the
type is a reader. Until now the only route to a *populated* snapshot from outside
the platform crate was `InputScript::hold(key, range).snapshot_at(tick)` — a
script built in advance and then read per tick.

That is exactly right for a fixed session and no use at all for a controller that
must look at the game before deciding what to press. E0 run 2 needed one: a
scripted session proves the controls and the drawing but says nothing about
whether the game is *playable*, because a blind script never returns a ball. The
check that proved it was a closed-loop player, and with no way to build one tick
of input it built a throwaway one-tick script every single tick:

```rust
InputScript::new().hold(Key::S, tick..tick + 1).snapshot_at(tick)
```

It works and it is deterministic. It is also wrong in a way that is easy to miss:
`hold(k, tick..tick + 1)` puts a **press edge on every tick**, because every tick
is the start of its own range. A game keyed on `just_pressed` sees a key tapped
sixty times a second. Run 2's Pong happened not to read `just_pressed` on the
key it drove, so nothing caught it.

The run's own suggestion was `InputSnapshot::with_keys(&[Key])`, and flagged the
tension: conventions §1 says one way to do everything, and `InputSnapshot`
already carries a `DELIBERATE:` refusing a second spelling of catch-up
derivation for the same reason.

## Decision

**No new constructor on `InputSnapshot`.** `jidousha::testing` exports
[`SnapshotBuilder`] and [`InputEvent`] instead, which already existed and are
already the engine's single home for the edge rules:

```rust
let mut keyboard = SnapshotBuilder::new();
for tick in 1..=TICKS {
    // Look at the game, then decide.
    let want = paddle.y < ball.y;
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

## Rationale

`with_keys` would be a second way to make a snapshot, and a worse one. It would
have to answer the edge question — does naming a key produce a press edge? — and
whatever it answered would be a *second* answer, sitting beside
`SnapshotBuilder`'s. The two would drift, and the one a test used would not be
the one a player's keyboard goes through.

Routing closed-loop input through the builder makes a controller exercise the
driver's own path: press and release are events, edges are recorded rather than
derived from a difference (input.md §2 INVARIANT), and a key held across three
ticks presses once. A test that finds an edge bug this way has found it in the
code the window uses.

The cost is that a controller must track what it is already holding, since it
sends events rather than states. That is what a keyboard is, and it is four
lines.

## Consequences

- `InputSnapshot` stays read-only from outside the crate: `new`, the readers,
  and the codec. The `DELIBERATE:` block in `snapshot.rs` names this ADR.
- Two ways to drive a headless run, and they answer different questions:
  `InputScript` for a session written in advance, `SnapshotBuilder` for one
  decided as it goes. Neither can express the other's case, so this is not a
  second spelling of one thing.
- `examples/scripted_player.rs` demonstrates both, and *Testing your game* says
  which to reach for.
- `SnapshotBuilder::first_tick_snapshot` is named for the driver's frame model.
  A controller running one tick per frame calls it every tick, which is exactly
  what its contract describes — the edges are spent once.

## Alternatives rejected

- **`InputSnapshot::with_keys(&[Key])`**: a second constructor, and one that
  cannot produce a press edge without duplicating the builder's rules or a
  release edge at all. A controller using it could never test `just_pressed`.
- **`InputScript::at(tick, &[Key])`, mutating a script as the run goes**: makes
  a script stop being a pure function of the tick, which is the INVARIANT that
  lets a test seek, replay and bisect (input.md §5).
- **Leave it and document the one-tick-script idiom**: it would mean documenting
  a spurious press edge as the recommended way to drive a game.

[`SnapshotBuilder`]: ../../crates/jidousha-input/src/builder.rs
[`InputEvent`]: ../../crates/jidousha-input/src/builder.rs
