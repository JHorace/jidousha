# E0 run 7 — Pong

Written as I went. Entries are in the order I hit them, not sorted by
severity. Nothing here was smoothed over afterwards.

## Before writing a line: what the two documents left me guessing

**F1. `Camera::viewport` under `run` is undefined behaviour from the game's
side.** The API document says the camera is `height` world units tall "and as
wide as the window's aspect makes it", and `Camera` has a `viewport:
PhysicalSize` field with a `Default` of 1280x720. What it never says is *who
writes that field during a windowed run*. If the driver overwrites it on
resize, my `Startup` value is a placeholder and `visible_bounds()` is honest
after the first frame. If it does not, then a resized window silently draws
against a stale aspect and every layout I compute from `visible_bounds()` is
wrong. I guessed "the driver owns it, my value is the opening one", set it to
match `GameConfig::window_size`, and then made the game not depend on the
answer: every layout constant is my own `COURT` half-extents, and the camera is
only ever asked whether things are inside it. That is a fine outcome, but I
chose it to route around not knowing, not because it was the better design.

**F2. Nothing says whether `Input` exists during `Startup`.** The resource
table says `Input` is inserted by `run` "before every Update tick" and is
absent "not before the first tick"; Concepts says `Startup` runs *inside* the
first tick, before `Update`. Composing those two gives "Startup never sees
Input", which is almost certainly right, but it is an inference across two
sections rather than a sentence. I did not need it in the end, but I spent a
minute on it.

**F3. There is no circle *outline*.** `ctx.circle` fills. A Pong centre circle
is an outline, and the drawing vocabulary has no stroke mode, so it is either a
filled disc at low alpha or sixteen `ctx.line` calls I write myself. The API
document is explicit that the vocabulary is closed, so this is a boundary
rather than a gap — but I looked for `Depth`-style "there is no X and here is
why" prose next to `circle` the way there is next to `Rect::sweep`, and there
is none. I drew a dashed centre line out of `ctx.rect` calls instead.
