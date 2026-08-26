# giri, prototype #1 — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). They are also in the pull request that landed this
crate; they live here as well because a workaround shipped silently is a gap
nobody fixes, and the PR body is not somewhere anybody reads twice.

**Reading discipline:** this game was written from `docs/api/` (all four) and
`crates/jidousha/examples/` only. No file under `crates/*/src/` was opened, and
neither was `docs/internal/` or any ADR but 0038. Each entry below is therefore
a question the four documents were actually asked.

Eight entries over four sessions: four from the first slice, two from the
presentation rebuild, one from the curation session, and one from the tuning
session (2026-08-24). The first four are resolved; G-007 and G-008 are open.

Four from the first slice, then two more from the presentation rebuild
(2026-08-23), which was the first session to give giri art and a scaling
contract. Both new ones are about the same half-hour: a game with its own
pictures has no documented way to get them onto a web page, and the store it
ends up using accepts them and silently draws nothing. Both are resolved as of
2026-08-23 — a game crate has an asset root (ADR-0040) and the store decodes at
the texture-load boundary — and giri consumes both fixes, which is what proves
they close.

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
`Assets`) · Fixed in: ADR-0040, this branch

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

**Resolved.** The web build learned to stage it, which was the first of the two
and is the owner's decision (2026-08-23). `games/giri/assets/` is a real asset
root now: giri loads it with `asset_source("games/giri/assets")` and the
`include_bytes!` table is gone. The rule that makes one string work on both
platforms is **`dist/<name>/` is repository-shaped** — `tools/build-web` stages
an asset root under the page at the path the code names it by, so the native
read and the web fetch resolve the same directory. Two roots reach a page and a
file's position picks its own: the repository's shared `assets/` for the
engine's examples, `games/<name>/assets/` for a game.

The other half of this entry's offer — blessing compiled-in bytes as the answer
— was **declined**, with the reasons in ADR-0040: it does not scale past a
handful of files (the owner's curated library is incoming), it turns every art
tweak into a recompile, and art that cannot travel with its game contradicts
what ADR-0038 put games in this repository to get. The ADR also records why a
game's art is not staged as plain `assets/` beside its page, which would have
been the prettier URL: it would make one string mean two different directories
depending on which crate wrote it.

Where the missing sentence now lives: `asset_source`'s doc comment (so
`jidousha-api.md`'s entry carries it), a new paragraph in the Concepts prose
that says a game's art is its crate's own and that the same string works on the
web, and `web-publish.md` §1a for the pipeline half. `tools/check-assets`
enforces the same two roots from the source side — a root the build does not
stage is now a CI failure rather than a 404 after deploying — and giri's
thirteen paths are checked by it for the first time, because compiled-in bytes
were invisible to it.

### G-006 — `MemorySource::insert` accepts image bytes, reports `Ready`, and draws nothing

Class: api · Game: giri · Documents: `jidousha-api.md` (`Assets`,
`MemorySource`), `jidousha-testing.md` (`decode_png`) · Fixed in: this branch

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

**Resolved.** The store decodes, at the texture-load boundary, whatever the
source. Bytes that a `load_texture` request resolves — from a disk, from a page,
from a `MemorySource` — go through the engine's one `decode_png` (assets.md §3
CONTRACT), so the spelling this entry opens with now works and giri's own art is
loaded exactly that way. Bytes that are not a picture resolve `Failed` with the
§6 decode error naming what the decoder found, reported once at the commit, with
the game's own line in it.

The serious half is closed structurally rather than by a rule: `Ready` is
reachable for a texture only through a decoded payload, and the property is
written as a test in this entry's own words — *a store can never report `Ready`
for a texture it has no texels for* (`jidousha-assets/tests/asset_ops.rs`) —
plus a second reading of it on every handle after every operation of two
thousand random sequences (`tests/asset_model.rs`), and the transcript version a
game would recognise: raw PNG bytes in, `load_texture`, and the sprite draws the
*texture* rather than the placeholder
(`jidousha-render-core/tests/loading_frames.rs`).

The decoder's address was the separable half, and it did **not** move:
`decode_png` stays in `jidousha::testing`, because no shipping game needs it once
the store decodes. giri's `src/sprites.rs` has no decoder and no panic path left
— the file that had to notice does not have to notice any more. The other
direction is now loud rather than silent too: a scripted store's `insert_texture`
asked for with `load_bytes` panics, because there are no bytes to hand back and
`Ready` would be the same lie the other way round.

One thing this cost, recorded because it is the sort of thing that goes
unwritten: every doc example, test fixture and example that scripted a store
with `b"fake png"` and loaded it as a texture was documenting the bug. They now
insert either real PNG bytes or texels, and `examples/loading_gate.rs` says
which is which and why.

## From the curation session (2026-08-23)

This session swapped giri's generated art for a curated subset of the owner's
Kenney packs. It asked `docs/api/` nothing new — no engine call changed, and the
whole swap landed as texel sizes in one table, which is the curation model
working exactly as DESIGN §7 claims. The one gap it did hit is in the repository's
tooling rather than its API, and it is recorded here for the same reason as the
rest: a workaround that ships silently is a gap nobody fixes.

### G-007 — the art tooling can write a PNG and cannot read one

Class: tooling · Game: giri · Documents: `tooling.md` (stdlib-only tools) ·
Fixed in: not fixed; worked around in `games/giri/art/pack_reader.py`

Curating from an owner-supplied pack is a looking problem before it is anything
else: you cannot choose a sprite you have not seen, and seeing several hundred
candidates means rendering them as contact sheets. That needs a PNG **decoder**.

The repository has three PNG encoders and no decoder reachable from a tool. The
engine decodes at the texture-load boundary (G-006's fix), in Rust, inside a
crate a Python tool cannot call. `art/make_art.py` writes PNGs from grids and
never reads one. `tools/` has no image utility at all. Tools are stdlib-only by
policy (tooling.md), and the standard library has `zlib` but no PNG.

So giri hand-rolled one: `art/pack_reader.py` is ~150 lines of chunk walking,
scanline unfiltering and palette expansion, covering the four colour-type and
bit-depth combinations Kenney's packs actually use and raising on everything
else. It is correct for this job and is a liability as a general decoder, which
is precisely why it should not be the second game's problem too.

Two things make this more than an inconvenience worth noting. The decoder is
where a subtle bug hides silently: a mis-read palette produces a contact sheet
that is *plausible* and wrong, and the only check on it is a human recognising
that a sprite looks off. And the reading step immediately grew a second
non-obvious piece — `strip_background`, which lifts a tilemap pack's baked-in
opaque background by an edge-connected flood fill rather than a colour key,
because Micro Roguelike's skull holds its eye sockets in the same colour as its
background and a key erases them. That is a general fact about tilemap packs, not
a giri fact, and the next game to import one will rediscover it by shipping a
skull with no eyes.

Not proposed for a fix now: one consumer is not a case for shared tooling
(second-consumer rule), and giri's copy is scoped to what it verified against.
The finding exists so that the second game to curate a pack promotes this rather
than rewriting it — at which point the natural home is a `tools/` image module,
and `strip_background`'s flood-fill-not-key rule is the part that must survive
the move.

## From the tuning session (2026-08-24)

Same reading discipline: `docs/api/` (all four) and `crates/jidousha/examples/`
only. This session built DESIGN §8a's live tuning drawer, its presets, its
`?constants=` links and §11's instrumentation. One finding, and it is the first
in this file that made giri name a crate other than the facade.

### G-008 — a game cannot read the parameters its own page was opened with

Class: api · Game: giri · Documents: `jidousha-api.md` (`GameConfig`, `run`,
`asset_source`) · Fixed in: not fixed; worked around in `games/giri/src/web.rs`

UI.md §9a makes a tuning configuration a URL: `?constants=k_inf:2,k_kill:6` is a
playtest link that carries its weights and a repro link when a playtester reports
a feel. The parameter has to be read once, before the first beat, because a
constant that arrived after it would be a constant that changed mid-run — which
is the one thing DESIGN §8a exists to prevent.

There is no way to read it. The facade's whole launch surface is
`GameConfig { title, window_size, .. }` and `run(config, setup)`; `App` offers
`add_system` and nothing else, so a game cannot even plant a resource before
`Startup` from the windowed path. `std::env::args()` is the native answer and is
empty on `wasm32-unknown-unknown`. Nothing in the four documents mentions the
page's query string at all.

What makes this more than an absence is that **the engine reads the same query
string already**: `?frametime=1` is documented in `jidousha-api.md`'s own frame
pacing passage, and `?renderscale=` landed beside it. So the mechanism exists,
one layer down, and exposes neither the values nor a way to ask for one. A game
author reading the documents finds a page that already parses parameters and no
parameter of their own.

The workaround is `web-sys`, target-gated to wasm32, four lines in
`src/web.rs`: `window()?.location().search()`. It works, it adds no crate to
`Cargo.lock` (`web-sys`, `js-sys` and `wasm-bindgen` are all already there at
these versions for the engine's own platform layer, so the delta is one edge),
and it is the first time a game under `games/` has named a dependency that is not
the facade. `tools/check-game-deps` is satisfied — the rule it enforces is about
*engine* crates — but the spirit of ADR-0038 is that the facade is the whole API
a game gets, and this is a game reaching around it for something the platform
layer already has.

Two shapes would close it, and they are separable:

- **The small one**: `Launch` (or a field on the existing input resources) — the
  page's query parameters as a `&str` or a key lookup, empty on native. One
  resource, and the engine already parses the string it would come from.
- **The general one**: a documented way for a game to receive *anything* before
  `Startup` on the windowed path. `headless(..)` has this — a harness plants
  resources and `Startup` reads them, which is how giri's own verify mode passes
  a beat index and a constants set — and `run(..)` has no equivalent. The
  asymmetry is invisible until a game wants its scripted path and its played path
  to take the same input, which is exactly what a shareable tuning link is.

Native is deliberately not given an equivalent, and that is a game-side decision
rather than a gap: the drawer is reachable on every platform (UI.md §9a), and a
`--constants` flag would be a second way to do a thing that already has one.

## From the P1 mechanics session (2026-08-26)

Same reading discipline: `docs/api/` (all four) and `crates/jidousha/examples/`
only. This session implemented DESIGN v2's People slice — traits, reputation
marks, explicit wealth, willingness with verdict + margin + reasons — across
the model, the beats, the interim UI and the verify harness.

**0 new findings.** Every question the session asked was answered by the four
documents or by machinery previous sessions already built against them: the
new components are the same ECS shapes G-001's fix serves, the new screens are
the same `Panel`-as-data pattern, the wider tuning set was picked up by the
drawer because it walks the constants module, and no engine call was added or
changed. First time this line has been true.
