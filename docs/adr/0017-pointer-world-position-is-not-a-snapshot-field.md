# ADR-0017: The pointer's world position is not a snapshot field

Status: accepted · 2026-08-16

## Context

input.md §3 sketched `PointerState` with two positions:

```rust
pub screen: Vec2,   // pixels, origin top-left
pub world: Vec2,    // via the Camera resource
```

and a CONTRACT under it: *`world` is derived, not recorded* — the recorded
snapshot stores raw screen position only, and `world` is computed at the tick
commit from that tick's `Camera`.

I1 built the pointer without the field and wrote down why: deriving it needs a
`Camera`, which `jidousha-input` must not name, so the driver would have to fill
it in after building the snapshot — and that runs into the codec, which either
encodes the field (contradicting the CONTRACT) or excludes it from the value's
own identity. Both questions were deferred to I2, which owns the stream format.

I2 has now built that format, so the deferral is due. The recording is a header
plus a sequence of `TickRecord`s, each holding an `InputSnapshot` encoded with
the I0 codec — the same bytes, the same strictness, the same two round trips.
The snapshot is the unit of the recording, which is what makes the question
answerable rather than a matter of taste.

## Decision

**`PointerState` carries `screen` and no world position.** A game that wants
world coordinates writes:

```rust
let aim = camera.screen_to_world(input.pointer().screen);
```

which is the sanctioned conversion (renderer.md §4) and the only one.

input.md §3's sketch is corrected to match. The CONTRACT it carried is kept and
strengthened: world position is derived from the tick's camera, and now there is
no second place it could come from.

## Rationale

- **The snapshot is the recording's unit, and a value must be its bytes.** The
  I0 codec's tests assert both round trips — bytes→value→bytes as well as
  value→bytes→value (ADR-0014). A field excluded from encoding breaks the first
  one: two snapshots that encode identically would compare unequal. A field
  included in encoding breaks the CONTRACT that a recording stores raw platform
  input, and makes a replay dishonest — the recorded world position would
  override what the replayed code's camera actually says, which is precisely the
  case where a camera bug hides from its own repro.
- **`jidousha-input` cannot name a `Camera`.** It lives in `jidousha-render-core`,
  which depends on input's sibling, not the other way round. The field could
  therefore only be filled by the driver, after the snapshot was built — so a
  snapshot from `InputScript`, which has no driver, would carry a different value
  from a snapshot from a window. One type meaning two things depending on who
  made it is worse than no field.
- **One way to do everything.** The conversion exists, is one line, and is what
  `examples/input_echo.rs` already does. A field would be a second way, and the
  two would disagree the moment a system moved the camera mid-tick — the field
  would hold the camera's position at snapshot time, the call would hold it now.
- **Multi-pointer makes it worse, not better.** ADR-0005 keeps touch headroom:
  `pointers()` is a list. A derived field on each entry is N conversions done
  eagerly, every tick, for a game that reads one of them — and every one of them
  wrong for any camera that is not the one the driver happened to see.

## Consequences

- Games written against pointer position write one more line, at the point where
  they know which camera they mean. For a split-screen or minimap game, that is
  the only place the question is even answerable.
- `PointerState` carries a `DELIBERATE:` tag pointing here, because "why is
  there no `world` field?" is a fair question with a non-obvious answer, and
  input.md §3 sketched one for four milestones.
- Recording stays raw and minimal: what is written down is what the platform
  said. Nothing derived is ever in the stream, so there is no class of bug where
  a recording and the code it replays disagree about a derived value.
- If pointer-heavy games later make the conversion tiresome, the sanctioned
  place for sugar is a method on `Camera` — which is where the camera is — not a
  field on the snapshot.

## Alternatives rejected

- **Add the field; fill it in the driver; exclude it from the codec.** The
  original sketch. It breaks the bytes→value→bytes round trip, which is the test
  that catches a decoder reading a field it should not; weakening it to admit
  one exception weakens it for everything.
- **Add the field and encode it.** Keeps the codec honest and makes replay
  dishonest instead: the replayed session would use a world position derived
  from the *recording's* camera rather than the code's. A camera change is
  exactly the kind of change a replayed session is used to test.
- **Give `Input` a camera reference so `pointer().world` can compute on demand.**
  `Input` is a resource inserted before the tick, and a resource cannot borrow
  another resource for its lifetime. It would also compute against whatever the
  camera was when asked, which is the right answer — and is exactly what calling
  `screen_to_world` does, without the indirection.
- **Put the conversion on `Input` as `pointer_world(&camera)`.** A third spelling
  of `camera.screen_to_world(...)`, on the type that is furthest from the camera.
