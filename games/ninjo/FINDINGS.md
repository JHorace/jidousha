# ninjo — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). G-numbers continue giri's sequence (its
`FINDINGS.md` ends at G-009); the fork inherits giri's open workarounds —
G-008's `?constants=` location reading rode along in `src/web.rs` unchanged
and is not re-counted here.

**Reading discipline:** this fork was written from `docs/api/` (all four),
`crates/jidousha/examples/`, and `games/giri/` (a game, not the engine). No
file under `crates/*/src/` was opened, and neither was `docs/internal/` nor
any ADR but 0038 and 0041 (both named by the handoff). Wave 0b held the same
line: `games/giri/` (the port source) and this crate, and nothing under
`crates/*/src/`.

One entry from the S1 session. Nothing else was asked of the documents that
they did not answer: the Camera's pan/zoom, `visible_bounds`, the pointer's
scroll, `SnapshotBuilder`'s edge rules, `Time::alpha`'s per-tick value and
the capture path all worked as written.

### G-010 — the bounds check's stated form assumes a camera that does not move

Class: docs · Game: ninjo (as giri-rt) · Documents: `jidousha-testing.md` ("Assert that
nothing is drawn outside `Camera::visible_bounds()`") · Open

The testing document presents the bounds assertion — every quad
`contains_rect`-inside `visible_bounds()` — as "the highest-value check a
game of shapes and text can write", and for every game so far it was. A
game whose camera pans and zooms over a world larger than the screen cannot
pass it: a partially visible tile at the view's edge is *correct* rendering
and still fails `contains_rect`, and per-run text culling (a label is one
`ctx.text` call) means edge glyphs of a half-visible label land fully
outside. The check the situation actually wants is the inverse pair: nothing
submitted that does not *overlap* the view (culling is honest), and the
submitted count dropping when the view shrinks (culling is real).
ninjo ships that pair (`verify.rs::culling_probe`, UI.md §4); the
document could name the adaptation the first time a scrolling game reaches
it, because the naive reading is "skip the check", which drops real
coverage.

Expected: guidance on what the bounds check becomes for a camera that
roams. Happened: worked it out from the check's purpose; the workaround is
three assertions rather than one. Owner: `jidousha-testing.md`.


## Wave 0b (the people substrate) — **0 new findings**

Said explicitly, because `0 findings` is a real answer and an unsaid one
reads as a skipped step.

Wave 0b reached for **no engine API that S1 had not already established**.
The port was game logic — a registry, a vocabulary, three stores, an
arithmetic — and everything it touched of the engine's surface (`Resource`,
`headless`, `SnapshotBuilder`, `FrameRecorder`, the capture path,
`TextStyle::width_of`) was already load-bearing in this crate and answered
by the four documents when S1 asked. Nothing new was asked of them, so
nothing new can be reported about them; a finding invented to fill this
section would be worse than an empty one.

**G-010 stays open.** The bounds check's stated form still assumes a camera
that does not move, and this wave added map-space content — the cast's
figures and names — which is culled the same way the terrain is and would
fail the naive `contains_rect` reading for the same reason.

One thing worth recording that is *not* a documents finding, because it is
this game's decision and not the engine's: **`Sim::at_rest` had to stop
meaning "the queue is empty"** the moment an ambient occurrence started
rescheduling itself forever. The substrate's stopping condition was written
when every occurrence belonged to a party. Any wave that adds a recurring
ambient occurrence — needs ticking is the next one — meets the same fact, so
it is written down here as well as at the site.

## Wave 0a (the attention architecture) — **1 new finding**

Reading discipline held: `docs/api/` (all four), `crates/jidousha/examples/`,
`games/giri/` and this crate. Nothing under `crates/*/src/`, `docs/internal/`
or any ADR was opened.

### G-011 — whether the first-finger-to-pointer mirror applies to a scripted snapshot is not stated

Class: docs · Game: ninjo · Documents: `jidousha-api.md` ("a game written for
a mouse is already playable by touch"), `jidousha-testing.md`
(`InputEvent::Touched`, `SnapshotBuilder`) · Open

The API document states the mirror as a property of the engine — "the engine
puts the first finger down onto the primary pointer", so
`just_pressed(PointerButton::Primary)` is a tap — and the testing document
lists `InputEvent::Touched { finger, phase, screen }` among the events a
`SnapshotBuilder` records. Neither says whether the mirror is applied when a
*check* records a `Touched` event, or only by the platform layer on the way
in. That is the difference between a check that can verify the claim and a
check that cannot: if the mirror lived in the platform crate, a failing
assertion would mean "the harness does not mirror" rather than "the game's
hit-test is wrong", and there is no way to tell those apart from the
documents.

Expected: one sentence in the testing document saying that a recorded
`Touched` produces the mirrored pointer in the snapshot the game reads.
Happened: wrote the check and ran it to find out. It does mirror, and
`verify::touch_selects` now asserts a finger on a character's figure selects
them with no `PointerMoved` and no `ButtonPressed` in the snapshot — so the
answer is recorded here, and the document is the place it belongs.
Owner: `jidousha-testing.md`.

**G-010 stays open** for the third wave running: the bounds check's stated
form still assumes a camera that does not move, and wave 0a added more
map-space content (the selection ring, the focus pulse) culled the same way.
