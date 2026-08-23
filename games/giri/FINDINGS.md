# giri, prototype #1 — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). They are also in the pull request that landed this
crate; they live here as well because a workaround shipped silently is a gap
nobody fixes, and the PR body is not somewhere anybody reads twice.

**Reading discipline:** this game was written from `docs/api/` (all four) and
`crates/jidousha/examples/` only. No file under `crates/*/src/` was opened, and
neither was `docs/internal/` or any ADR but 0038. Each entry below is therefore
a question the four documents were actually asked.

Four from the first slice, then two more from the presentation rebuild
(2026-08-23), which was the first session to give giri art and a scaling
contract. Both new ones are about the same half-hour: a game with its own
pictures has no documented way to get them onto a web page, and the store it
ends up using accepts them and silently draws nothing.

### G-001 — no read-only projection both an Update system and a Draw system can read

Class: api · Game: giri · Documents: `jidousha-api.md` (Concepts; ECS reference) ·
Fixed in: ADR-0039, `43e4e35`

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

**Resolved.** `World::view(&self) -> WorldView<'_>` landed as ADR-0039 — the
shape this entry proposed, with the `Read` trait declined for the reason it
gives. ADR-0008 is unweakened and pinned by
`crates/jidousha-core/tests/compile-fail/view_query_cannot_write.rs`: a `&mut T`
query through a view is the same compile error it is through `ctx.world`.
`Social::read` and `Social::view` are now one `Social::read(&WorldView<'_>)`,
called with `&world.view()` from Update and `&ctx.world` from Draw; every beat
outcome is byte-identical, which is the evidence the two collectors were the
same function. `examples/headless_sim.rs` is the worked version.

### G-002 — nothing says how to script a pointer at a target the game states in world space

Class: docs · Game: giri · Document: `jidousha-testing.md` · Fixed in: `43e4e35`

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

**Resolved.** `tools/api-doc/testing.md` (which generates `jidousha-testing.md`)
now carries the worked pointer sequence beside the `InputScript` material — a
world rectangle converted with `Camera::world_to_screen` through a camera built
exactly as the game builds its own — and states the `viewport` trap there rather
than only at `visible_bounds()`. `examples/scripted_player.rs` gained a third
section that clicks a world rectangle and asserts on the world *and* on the
recorded frame's transcript, so the document's claim has a worked instance.

### G-003 — a game laying generated text into a column reimplements the font's advance

Class: api · Game: giri · Documents: `jidousha-api.md` (Concepts; `TextStyle`) ·
Fixed in: `43e4e35`

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

**Resolved.** `TextStyle::columns_in(width) -> usize` landed — one of the two
this entry offered, not both: a documented `ADVANCE_RATIO` was declined because
a game would still write the division, which is the thing to remove. Its doc
comment and `width_of`'s name each other, and a round-trip test keeps the stated
ratio and the new API in step. giri's local `columns_in` and its 7/9 are gone,
and `examples/input_echo.rs` clips its generated readout with the new call.

### G-004 — "the recorder keeps every frame" is priced for one session, and a verify mode runs many

Class: docs · Game: giri · Document: `jidousha-testing.md` · Fixed in: `43e4e35`

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

---

## From the presentation rebuild (2026-08-23)

Same reading discipline: `docs/api/` (all four) and `crates/jidousha/examples/`
only. This session added sprites, a scaling contract, and a capture set, so the
questions it asked were about assets and about the camera.

### G-005 — a game crate has no way to ship its own art to the web

Class: tooling · Game: giri · Documents: `jidousha-api.md` (`asset_source`,
`Assets`) · Open

ADR-0038 says a game is a crate under `games/` and that living there is what
makes it built, linted, verified and published. giri's art is giri's — four
portraits, four dungeon icons, five stat icons, generated by a script committed
beside them — so it belongs at `games/giri/assets/`.

It cannot load from there on the web. `asset_source(root)` fetches relative to
the page, and what a web build puts beside the page is the **repository root's**
`assets/` directory and nothing else. So a game that owns its art either puts it
in the repository's shared asset root — where it is no longer travelling with
the game, and where two games' `icon_coin.png` collide — or it does not use the
asset source at all.

The documents do not say this. `asset_source`'s entry is four lines and an
example with `"assets"` in it; nothing in the four documents says what that root
means on the web, and the fact that it means "one fixed directory per
repository" is a fact about the build tool rather than about the API, which is
exactly why a game author does not find it.

The workaround is `include_bytes!` and a `MemorySource`, which is fine and is
what giri ships: native and web get the identical store and the loading path is
the same `Assets` one either way. It is a workaround rather than a choice,
though — a fifty-file library would be fifty `include_bytes!` and a binary
carrying all of it — and the general form is worth stating: **a game crate
cannot have an asset root.** Either the web build learns to stage a game's own
directory, or the documents should say that compiled-in bytes are the answer for
a game under `games/`, and say it where `asset_source` is.

### G-006 — `MemorySource::insert` accepts image bytes, reports `Ready`, and draws nothing

Class: api · Game: giri · Documents: `jidousha-api.md` (`Assets`,
`MemorySource`), `jidousha-testing.md` (`decode_png`) · Open

Following G-005's workaround, giri built its store the obvious way:

```rust
source.insert("icon_coin.png", include_bytes!("../assets/icon_coin.png").to_vec());
let handle = assets.load_texture("icon_coin.png");
```

Every sprite drew the engine's magenta placeholder. `Assets::all_ready()` was
`true`, `Assets::status(handle)` was `Ready`, `commit` returned no
`AssetFailure`, and the recorded frame's batches named the placeholder texture.
Nothing anywhere said no.

The working spelling is `MemorySource::insert_texture` with an already-decoded
`TextureData`, and the only decoder the facade exposes is
`jidousha::testing::decode_png` — so a *shipping game* imports the testing
module to load its own art. `examples/prototype_kit/verify.rs` does exactly
that, and it is a verify file, so a reader takes it for a testing convenience
rather than for the only path there is.

Two things follow, and they are separable:

- **The silent half is the serious one.** "No silent failure" is the engine's
  own rule, and this is a fallback that does nothing and says nothing:
  `insert` of bytes that a `load_texture` will ask for as an image should either
  decode them or resolve `Failed` with an `AssetError::Decode`. The symptom —
  every sprite magenta — is indistinguishable from a wrong path, and no
  assertion over drawn quads can see it, which is how it survived a green
  verify run in this session until a person looked at the PNG.
- **The decoder's address is the other.** If a game is expected to decode its
  own bytes, `decode_png` belongs in the prelude beside `Assets`, not in
  `jidousha::testing`; if it is not, `Assets` should do it. Either way the four
  documents currently describe a path that compiles, runs, reports success, and
  shows the placeholder.

giri's `src/sprites.rs` decodes with `decode_png` and panics with a four-part
message if a file stops decoding, because that is the only place the game could
notice.
