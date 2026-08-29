# ADR-0043: touch is snapshot data, and the first finger is the cursor

Status: accepted · 2026-08-29

## Context

The owner playtests web builds on a phone and a tablet, and published pages are
this project's distribution. The `InputSnapshot` carried keyboard and pointer
and nothing else, so mobile play was whatever the browser's mouse emulation
happened to produce: a tap became a `click` at a position, a drag became
nothing much, and a second finger did not exist.

That is not merely thin — it is outside the contract. input.md §1 says every
input simulation can observe flows through the snapshot, recorded and
replay-deterministic. Browser-emulated mouse events *were* reaching the snapshot
through winit's mouse path, so mobile sessions were replayable by accident; but
anything the engine wanted to do better than emulation would have had to reach
outside it. A mobile playtest that cannot be recorded is a bug report that
cannot be reproduced, and those are the playtests the owner actually runs.

## Decision

**Touch joins the snapshot, and the first finger down is mirrored onto the
primary pointer.**

1. **Touch is snapshot data.** `InputSnapshot` gains a fixed list of at most
   `MAX_TOUCHES` = 4 touches, each a stable `TouchId` slot, a `TouchPhase`
   (`Began`/`Moved`/`Ended`/`Cancelled`), and a screen position in the same
   space the pointer's is in. It is recorded, replayed and read through the one
   choke point, like everything else. Simulation gets no callback, no event
   queue and no way to ask the platform anything mid-tick.
2. **The mirror.** The first finger to land while nothing is mirrored takes the
   primary pointer: its position, and a `PointerButton::Primary` press for as
   long as it is down. The rule is **first active touch wins, and does not hand
   over** — a second finger never moves the pointer, and when the mirrored
   finger ends the button releases rather than being handed to whatever else is
   on the glass. Every existing game that reads the pointer becomes tappable
   with no change; `examples/scripted_player` plays the same button with a
   mouse and then with a thumb, and the game code between the two is identical.
3. **The mirror is applied where the event is recorded**, in `SnapshotBuilder`,
   not where the snapshot is read. A mirrored press is then an ordinary pointer
   edge obeying the ordinary edge rules — spent once, absent from catch-up
   ticks, released on focus loss — and it is *in* the recording, so a replay
   re-reads the mirror rather than re-deciding it.
4. **Sources**: winit's `WindowEvent::Touch` on native and on the web, where
   winit's own backend routes touch pointers away from the mouse path. The
   browser's compatibility mouse events are suppressed (winit's
   `prevent_default`, stated explicitly by the driver) and the canvas is
   `touch-action: none` so the browser does not take the gesture. Our mirror is
   the only mirror.
5. **Gestures are not engine scope.** Pinch, pan and long-press are what a
   *game* makes of raw touches, and what a swipe means differs between one game
   and the next. A shared helper waits for a second consumer.
6. **The recording format extends additively.** The snapshot's version goes to
   2 with the touch list on the end and nothing before it moved, so a version 1
   snapshot decodes to what it always meant — no touches. A recording written
   by this engine replays only on this engine, refused by number rather than
   misread.

## Consequences

- **Mobile playtests are first-class and replayable.** The owner taps on a
  phone, the session records, and the file replays headlessly like any other.
  That was the point.
- **`Moved` is the resting phase**, not a fifth "still down" one. A finger that
  has not moved reports `Moved`, so `touches()` is what is on the glass rather
  than what changed — a game reading it needs no state of its own to know a
  finger is still there. The alternative was a phase nobody would branch on, in
  every recording forever.
- **A touch that lands and lifts inside one frame reports `Began` on that tick
  and `Ended` on the next.** One entry per touch per tick has room for one
  phase, and losing the second edge is the one thing input.md §2 refuses to do.
  A tap is therefore two ticks of touch data — while the *mirrored* click is a
  press and a release on the same tick, exactly as a mouse tap inside one frame
  already was.
- **Four, and the fifth finger is dropped.** The snapshot is a fixed structure
  written to disk sixty times a second, and the bound is part of the format:
  the decoder refuses a file claiming more. Dropping the fifth is a documented
  boundary of the same kind as a key the `Key` enum does not name.
- **Touch does not become a second pointer**, though `PointerId::touch` has
  been sitting there since I0 waiting for it. A pointer has no phase and no
  bound; a touch has both, and the mirror needs exactly one finger on the
  primary pointer rather than four pointers a game would have to choose
  between. `pointers()` stays what it was — the headroom ADR-0005 asked for,
  still length 1 — and the touch list is the thing that grew.
- **The bound and the slot assignment live above the platform seam**, in
  `jidousha-input`, for the reason the edge rules do: they are contracts, and
  behind winit they would be testable on native only, through a real
  touchscreen, and not at all on wasm CI. What stays platform-side is the
  translation table and the render-scale multiplication.
- **Old recordings keep replaying, and that costs one property.** The byte
  round trip — `encode(decode(b)) == b` — holds for what this build writes;
  reading a version 1 snapshot is an upgrade, and writing it back out produces
  version 2. A recording worth keeping is kept as the file it was written as.
  `tests/old_recordings.rs` replays a file the pre-touch engine actually wrote.
- **Declined: mirror on read rather than on record.** `Input::pointer()` could
  have consulted the touch list and answered from it. It would have kept the
  snapshot smaller by one decision and made replay a re-derivation: a recording
  would carry the fingers and each engine version would re-decide what they
  meant for the cursor, so a change to the rule would silently change what an
  old recording did. The recorded mirror is a fact; a derived one is an opinion
  the replay has to share.
- **Declined: promote the next finger when the mirrored one lifts.** It reads
  as generous and plays as a click nobody made — the cursor teleports to the
  thumb a player has been resting on the glass. "Does not hand over" is also
  the version with nothing to decide when two fingers race.
- **Declined: gesture recognition in the engine.** Every game would get the
  engine's idea of a long press, and the first game that disagreed would have
  to work around it while still paying for it in the snapshot.
