# Input system — design and contracts

Status: **living doc for `jidousha-input`; I0 implemented, I1–I2 still design.**
Sections carry `Implemented (I0)` notes where code exists; everything else is
design ahead of the code. **CONTRACT** items are binding and tested.

Inherits: the per-tick snapshot choke point and replay contract (core §7), winit
translation confined to the platform crate (ADR-0004), pointer-not-mouse headroom
(ADR-0005), world-space conversion through the camera only (renderer §4,
conventions), error taxonomy (core §9).

In scope (v1): keyboard (physical keys), pointer (position, buttons, scroll,
multi-pointer-shaped), edge semantics, focus-loss policy, recording/replay,
verify scripting. Out of scope (deferred): gamepads (needs a dep — own decision
later), text/IME input, action mapping / rebinding, cursor capture & relative
mouse mode, clipboard.

---

## 1. The model

All input reaches simulation through **one value**: the per-tick `InputSnapshot`,
exposed to systems as the `Input` resource. No events, no callbacks, no polling
the platform mid-tick. This single choke point is what makes recording and replay
trivial (core §7) — record the snapshots, replay the snapshots, done.

```rust
fn player_control(world: &mut World) {
    let input = world.resource::<Input>();
    if input.just_pressed(Key::Space) { /* jump */ }
    if input.held(Key::A)            { /* move left */ }
    let aim: Vec2 = input.pointer().world;   // world space, Y-down
    if input.pointer().just_pressed(PointerButton::Primary) { /* fire */ }
}
```

CONTRACT: `Input` is read-only for games (no mutation API exists). The snapshot
is plain serializable data; serialize→deserialize round-trips to an identical
value (tested).

Implemented (I0):

- `Input::new(snapshot)` is the only constructor, and there is no setter: the
  driver replaces the whole resource each tick. A system that could edit input
  could edit the recording, which would make replay a story rather than a
  guarantee.
- `InputSnapshot`'s key and button lists are **canonical** — sorted, no
  duplicates — so two snapshots meaning the same input *are* equal, and equal
  snapshots encode to equal bytes. Order within a tick is not observable to
  simulation (every query is by key), so canonicalizing loses nothing.
- `pointer()` always has something to return: a snapshot carries the primary
  pointer even on a machine with no pointer at all, where it sits at the origin
  with nothing pressed. Games never handle an `Option` for the common case.
- Serialization is hand-written rather than derived — see ADR-0014.

## 2. Keyboard

- **Physical key codes only** (`Key` enum, engine-owned, translated from winit's
  physical codes at the platform boundary). WASD is WASD on AZERTY — layout
  independence is what games want, and one key model is one way to do it.
  Logical/text input is a deferred, separate facility (typing a name into a
  text box is a different problem from "is the jump key down").
- `Key` covers the common physical set (letters, digits, arrows, space, enter,
  escape, modifiers, F-keys, common punctuation). Unmapped platform keys are
  dropped, documented as such — not silent failure, a documented boundary of
  the v1 enum; extending `Key` is a normal additive change.
- Queries: `held(Key)`, `just_pressed(Key)`, `just_released(Key)`.

**Edge semantics** (the subtle part):

- CONTRACT: edges are **explicit recorded data, not state diffs**. The snapshot
  carries `pressed: Vec<Key>` and `released: Vec<Key>` for the tick plus the
  held set. A tap that begins and ends between two slow frames still produces
  its press edge and release edge (same tick — pressed, held-for-one-tick,
  released); state-diffing would lose it entirely.
- CONTRACT: each physical event produces exactly one edge, on exactly one tick.
  When a frame runs multiple catch-up Update ticks, the frame's accumulated
  events apply to the **first** tick; subsequent catch-up ticks see held state
  with no new edges.

Implemented (I0):

- The two rules are two methods: `SnapshotBuilder::first_tick_snapshot` (spends
  the frame's edges) and `catch_up_snapshot` (state only). A driver that calls
  the wrong one is the likeliest way to break §2, so the names say which is
  which rather than leaving it to a flag.
- A frame that runs **zero** ticks does not lose its events: they stay pending
  and belong to whichever tick runs next. This falls out of edges being spent at
  emission rather than at the frame boundary, and it is what makes a slow first
  frame harmless.
- Scroll is spent the same way an edge is. Three catch-up ticks must not scroll
  three times, so `catch_up_snapshot` reports zero scroll.
- A tap is `held` on its one tick, even though it is also released on it —
  otherwise `just_pressed(k) && !held(k)` would be reachable, which no game
  expects. A key released *without* a press that frame is not held: it went up
  during the tick.
- **Documented boundary**: two presses of the same key inside one frame collapse
  to one edge. A tick is the resolution of the recorded timeline and cannot
  express two presses of one key; at 60 Hz this is a double-tap inside 16 ms.
  The property test encodes the collapse rule rather than papering over it.

## 3. Pointer

Modeled as **pointers, plural** (ADR-0005 touch headroom), with a primary-pointer
sugar that is all v1 games touch:

```rust
pub struct PointerState {
    pub id: PointerId,           // Primary = the mouse / first touch
    pub screen: Vec2,            // pixels, origin top-left (same orientation as world)
    pub world: Vec2,             // via the Camera resource — see below
    pub held / just_pressed / just_released (PointerButton),  // Primary/Secondary/Middle
    pub scroll: f32,             // lines this tick; normalized by the platform layer
}
```

- `input.pointer()` → primary pointer. `input.pointers()` → all (length 1 until
  touch platforms arrive; game code written against `pointer()` never breaks).
- **`world` is derived, not recorded**: the recorded snapshot stores raw screen
  position only; `world` is computed at the tick commit from that tick's
  `Camera` resource. Camera state is deterministic simulation state, so replay
  re-derives identical world positions — recording stays raw and minimal, and
  a replayed session with a code-modified camera stays *honest* (world pos
  reflects the camera the replayed code actually has). CONTRACT.
- Scroll normalization (line-mode vs pixel-mode deltas differ across
  platforms/browsers): the platform layer normalizes to "lines" before the
  snapshot; whatever it produces is what's recorded — replay is exact even if
  normalization heuristics later improve.

## 4. Focus loss and window edge cases

- CONTRACT: on focus loss (alt-tab, browser tab switch), the engine
  **synthesizes release edges for every held key and button** on the next
  snapshot. The stuck-key-after-alt-tab bug is designed out. Synthesized
  releases are recorded exactly like real ones — replay doesn't care why a key
  released.
- Window resize, close requests, and focus state are **not input** — they route
  through the app lifecycle (core §8) and renderer (surface resize), never
  through `Input`. Focus state is however *readable* (`input.window_focused()`)
  since pause-on-unfocus is a legitimate gameplay concern — and it too is
  recorded (it's observable by simulation, so it must replay; core §7).

Implemented (I0): the synthesis lives in `SnapshotBuilder`, not in the platform
crate as this section originally said. It is pure logic and a CONTRACT; behind
the winit seam it would be testable on native only, through a real window, and
not at all on wasm CI. Regaining focus re-presses nothing — the player is not
still holding a key the engine released for them.

## 5. Recording, replay, scripting

- The recorded stream (one format shared with asset readiness, assets §4) is a
  per-tick sequence of snapshot deltas. Format details land at implementation;
  CONTRACTs: versioned header, append-only writing (a crashed session's
  recording is valid up to the crash — that's precisely the repro you want),
  and byte-stable across platforms.
Implemented (I0): the snapshot codec, which is what a stream will be made of.
Magic number, version, fixed-width little-endian fields, canonical lists, and a
decoder strict enough that both round trips hold — bytes→value→bytes as well as
value→bytes→value. Hand-written, not derived: ADR-0014 says why, and it is the
reason "byte-stable across platforms" is a property we own rather than one we
hope a format crate preserves. The stream *around* the snapshots — the
append-only header, the asset-readiness interleave — is I2.

- `tools/verify` scripting is a builder over the same types:

```rust
InputScript::new()
    .hold(Key::D, 10..120)
    .press(Key::Space, 30)
    .pointer_at(60, Vec2::new(400.0, 300.0))
    .click(PointerButton::Primary, 61)
```

- The full loop this enables (the engine's thesis in one sentence): an agent
  scripts input, runs N headless ticks, asserts on world state and the draw
  transcript, and never opens a window.

Implemented (I0): `InputScript` with `hold`, `press`, `pointer_at`, and `click`,
and `snapshot_at(tick)` — a pure function of the tick, with no cursor, so a test
can seek, replay, and bisect freely. `examples/scripted_player.rs` is the loop
end to end, minus the draw transcript, which needs a renderer. Contradictory
directives panic at the point they are added, naming both directives and both
source lines; the check runs on add, so it fires whichever order they are
written in. A hold that *ends* where another begins is not a contradiction —
that is a real thing a player does.
- Record-on-native → replay-headless is the bug-repro workflow: a human (or
  playtest session) records a session; the agent replays it deterministically
  in tests, bisecting freely.

## 6. Internals

```
jidousha-input       Key/PointerButton/InputSnapshot/Input types; edge logic;
                     recording format; InputScript. Pure, no platform deps,
                     wasm-clean — tests run everywhere.
jidousha-platform    winit event loop → event accumulator → snapshot builder
                     (translation tables, scroll normalization, focus policy).
```

- The accumulator collects winit events between frames; at frame start it emits
  the tick snapshot (per §2 edge rules) and resets. ~No allocation in the
  steady state (edge Vecs are usually empty; SmallVec-class storage).

Implemented (I0), with one deliberate move across the seam: the accumulator —
`SnapshotBuilder` — lives in `jidousha-input`, not in `jidousha-platform` as the
box diagram above has it, and it consumes `InputEvent` values in the engine's own
vocabulary. What stays platform-side is exactly what needs a platform: the winit
translation tables and scroll normalization. The reason is testability — the edge
rules and the focus policy are the CONTRACTs of this subsystem, and behind the
winit seam none of them could be tested on wasm CI. PERF: edge lists are plain
`Vec`s that allocate on first use per frame; SmallVec-class storage is a
measurable-win question for I1, when there is a real event loop to measure.
- CONTRACT (ADR-0004 discipline): no winit type crosses out of
  `jidousha-platform`; the translation table is the only place both vocabularies
  appear.

## 7. Errors

Input has no environmental failures; the §9 surface here is contract violations
and documented boundaries: querying an unknown/unmapped key is impossible by
construction (the `Key` enum is the vocabulary); `InputScript` tick ranges that
overlap contradictorily (hold 5..10 + release at 7) are a debug panic with the
§9 message naming both directives. Malformed recording files → `try_`-class
`Result` (environmental: the file came from outside).

## 8. Milestones

- **I0 — types + semantics, pure.** ✅ Snapshot/Input/edge logic, serialization
  round-trip, `InputScript` builder, catch-up-tick and tap-within-one-frame
  edge tests (property tests over random event streams: every event → exactly
  one edge, exactly once). Runs on wasm CI. Can start any time after core M3.

  Delivered: `tests/edge_semantics.rs` (2000 random event streams against a
  naive model, plus an independent edge *count* check that groups the stream
  into windows and asks how many windows mention each key — a tick-by-tick model
  can agree and still lose an edge at a frame boundary), `tests/codec_round_trip.rs`
  (both round trips over every snapshot those streams produce, plus every
  truncated prefix of every one of them), and `examples/scripted_player.rs`.

  What the mutation checks said. Fourteen deliberate breakages; twelve were
  caught immediately. The two that escaped were both in `catch_up_snapshot` —
  leaking the frame's edges, and re-applying its scroll — and they escaped for
  an interesting reason: under the correct driver protocol `first_tick_snapshot`
  has already spent both, so a leak has nothing left to leak. The fix was not a
  cleverer property test but a plainer one: the contract says a catch-up tick
  carries no edges *unconditionally*, so the test now asserts that with events
  still pending. Both mutants die. The lesson generalizes — when a mutant
  survives, check whether the contract is weaker than it sounds before assuming
  the test is.
- **I1 — platform translation.** winit tables, accumulator, scroll
  normalization, focus-loss synthesis. `examples/input_echo.rs` (draws pressed
  keys + pointer state as text — also exercises renderer text).
  Exit: manual check on all three targets; translation-table unit tests.
- **I2 — recording + verify.** Stream format (with asset-readiness interleave),
  record/replay round-trip test (record scripted session → replay → identical
  world hash per tick), `tools/verify` wiring.
  Exit: the §5 full loop demonstrated on `prototype_kit`.

## 9. Deferred (tracked, not designed)

Gamepads (dep decision + ADR when taken) · text/IME · action mapping & rebinding
· cursor capture / relative mode / pointer lock (web) · multi-touch gestures ·
clipboard.
