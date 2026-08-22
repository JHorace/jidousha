# giri, prototype #1 — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). They are also in the pull request that landed this
crate; they live here as well because a workaround shipped silently is a gap
nobody fixes, and the PR body is not somewhere anybody reads twice.

**Reading discipline:** this game was written from `docs/api/` (all four) and
`crates/jidousha/examples/` only. No file under `crates/*/src/` was opened, and
neither was `docs/internal/` or any ADR but 0038. Each entry below is therefore
a question the four documents were actually asked.

Four findings. Two are gaps that cost a workaround in the game; two are prose
that would have saved a reader time and did not exist.

### G-001 — no read-only projection both an Update system and a Draw system can read

Class: api · Game: giri · Documents: `jidousha-api.md` (Concepts; ECS reference)

giri's whole UI is a view of relational state — every stat and every regard edge
— and DESIGN's invariant is that the willingness preview and the simulation must
call *one* function, so the preview cannot say something the resolution
disagrees with. That wants one reader: collect the roster and the edges into a
plain snapshot, and let both the `&mut World` systems and the `&mut DrawCtx`
systems call it.

There is no way to write that reader once. `World::query` and
`WorldView::query` are separate inherent methods; the surface has no trait either
of them implements, no `WorldView::from(&World)`, and no `World::view()`. The
documents describe `WorldView` as "a read-only view of the world, handed to Draw
systems" and say a Draw system "reaches the same values through `ctx.world`" —
which is true of the values and not of the code that reads them.

The workaround is `Social::read(&World)` and `Social::view(&WorldView)`,
character for character identical apart from the receiver, both feeding one
`assemble`. It is eleven duplicated lines and it is not wrong; what it is is
un-DRY in a way no game can avoid, because the duplication is forced by the
surface rather than chosen. Any game with a projection used by both logic and
drawing meets this, and a game whose UI *is* the projection meets it on day one.

Worth noting the shape of the fix is a decision, not an omission: a `Read` trait
would put a trait bound in every game's signature, and `World::view(&self) ->
WorldView` would not (and appears to be what the type already is). Which is the
maintainer's call; this entry is the evidence that something is wanted.

### G-002 — nothing says how to script a pointer at a target the game states in world space

Class: docs · Game: giri · Document: `jidousha-testing.md`

giri is pointer-only, so its `--verify` mode has to click a card and a button
that the game knows as world-space rectangles. `InputScript::pointer_at` takes
**screen** pixels, and its reference entry says so in four words:

```rust
.pointer_at(60, Vec2::new(400.0, 300.0))
.click(PointerButton::Primary, 61)
```

That is the only pointer material in the surface. Every worked example — the
document's prose, `scripted_player.rs`, `slalom`, `pong` — drives keys, so a
game whose input is a mouse has no worked instance of the thing it must do
first, and the two facts it has to put together are in different documents:
`Camera::world_to_screen` is in `jidousha-api.md`'s Render reference, and the
trap that makes it load-bearing is in *this* document, stated about something
else. Under `headless` nothing stamps `Camera::viewport`, so a check that builds
its camera differently from the game's converts every click to the wrong pixel
and the run fails with an empty party and no clue why — which is exactly the
`FrameRecorder` viewport trap the document spells out at length for
`visible_bounds()`, and never mentions for clicks.

Expected: a paragraph beside the `InputScript` material — "a pointer game's
targets are world rectangles; convert with a `Camera` built exactly as the game
builds its own, including the viewport, because nothing stamps it here." Found:
`400.0, 300.0`. The workaround was to derive it, which took one read of the
camera paragraphs in the other document and a guess that turned out right.

The general form is worth stating: **the testing document is written for a game
driven by keys**, and a pointer game reads it a document short.

### G-003 — a game laying generated text into a column reimplements the font's advance

Class: api · Game: giri · Documents: `jidousha-api.md` (Concepts; `TextStyle`)

`ctx.text` does not wrap and `\n` is the only break, which the documents say
plainly and which is the right v1 boundary. So a game that draws a *generated*
string — a dilemma sentence, a report row, a blocked-send reason — into a column
of known width has to answer "how many characters fit", and `TextStyle` measures
the other direction only (`width_of(&str) -> f32`).

The ratio is documented — "each exactly `size` tall and `size * 7 / 9` wide" —
so this is a gap in the API rather than in the prose, and a small one: giri
carries `columns_in(width, size)`, four lines, derived from that sentence. It is
recorded because the sentence is the *only* place the ratio appears, a game that
misses it writes `width / size` instead, and the failure is a line that runs off
the side of the world — which the bounds assertion catches, in the tenth minute
rather than the first. A `TextStyle::columns_in` (or a documented
`ADVANCE_RATIO`) would cost one line and remove a magic 7/9 from every game.

### G-004 — "the recorder keeps every frame" is priced for one session, and a verify mode runs many

Class: docs · Game: giri · Document: `jidousha-testing.md`

The document says the recorder keeps every frame with no way to forget them, and
that this "is deliberate and it is affordable at prototype scale". True of one
session. giri's `--verify` runs 4 beats, and then the mutation round runs all 4
again for each of 10 perturbed constants — 44 sessions in one process, which at
one frame per tick is about 3,700 frames nobody will read.

The document is what makes the mutation round cheap in the first place ("a whole
*game* is cheap to build too, which is what makes a tuning sweep a loop rather
than a shell script"), so the two passages meet in a game that takes both offers
and neither mentions the other. The shape that works is one line — build the
`FrameRecorder` only for the runs that will read frames, `Option<FrameRecorder>`
and a `record: bool` — and it belongs beside the sweep paragraph, which is where
a reader is standing when the multiplication happens.
