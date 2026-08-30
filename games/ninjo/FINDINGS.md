# ninjo — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). G-numbers continue giri's sequence (its
`FINDINGS.md` ends at G-009); the fork inherits giri's open workarounds —
G-008's `?constants=` location reading rode along in `src/web.rs` unchanged
and is not re-counted here.

**Reading discipline:** this fork was written from `docs/api/` (all four),
`crates/jidousha/examples/`, and `games/giri/` (a game, not the engine). No
file under `crates/*/src/` was opened, and neither was `docs/internal/` nor
any ADR but 0038 and 0041 (both named by the handoff).

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
