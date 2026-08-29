# ADR-0042: A typeface is an asset, a size is an atlas, and a measurement is part of the API

Status: accepted · 2026-08-29

Owner-approved direction, landed with the corrections contact forced. Those
corrections are listed under *Deviations from the approved draft* at the bottom
rather than folded silently into the decision above them.

## Context

Engine text is the placeholder bitmap path: one compiled-in five-by-seven font,
monospace, printable ASCII, an advance of seven ninths of the line and no way to
have any other (renderer.md §6). It was always described as minimal, and it has
been enough, because what has been drawn with it is scores, debug readouts and
prototype chrome.

The grand-strategy prototype multiplies text everywhere — a larger map, ledgers,
feeds, sheets — and the roadmap has carried "TTF text — likeliest first pull"
since v1. The pull has now happened: readable type at multiple sizes is a
prerequisite for every screen the GDD will specify.

Text rendering sits entirely on the draw side. The simulation never reads a
glyph, so nothing here touches the determinism contract (core.md §7).

There is a second thing in the context, and it is the one that shaped the API.
`games/giri` and `games/giri-rt` assert **readability floors** — no row of text
below a minimum size, nothing running off its rect, nothing lying across a
control it is not the label of (UI.md §7). Every one of those floors measures a
row with `TextStyle::width_of`, which is a character count times a constant.
That constant is a fact about a monospace font. A proportional face does not
have one, and a floor asserted against a number that no longer describes
anything is a floor that passes while the screen is wrong.

## Decision

**1. CPU rasterization into a glyph atlas, drawn as quads through the sprite
path.** No new pipeline, no text shader, no second sort. A glyph is a quad
sampling a texture, which is what a glyph already was — the only new thing is
where the texture comes from. The atlas is uploaded through
`RenderBackend::create_texture` into the same `TextureTable` a loaded sprite
goes into.

**2. One rasterizing crate: `ab_glyph`.** Explicitly not pulled: shaping engines
(harfbuzz, rustybuzz), the cosmic-text/glyphon stacks, layout frameworks.
Coverage is **ASCII plus Latin-1** — 0x20–0x7E and 0xA0–0xFF, skipping the C1
control block, which is not characters anybody sets. Complex scripts, shaping,
bidirectional text and wrapping are out of scope until a prototype needs them,
and each is a subsystem rather than a flag.

**3. The measurement API is part of the feature.** `TextStyle::measure` returns
`TextExtents`; `fits_in` answers "how much of *this* string fits in this width";
`width_of` and `columns_in` keep working and now mean something a proportional
face can honour. The floors move to measured extents. A floor that could not be
asserted against the new metrics would be a regression, not a casualty — and
none was dropped.

**4. Fonts are assets.** One OFL family committed — Fira Sans, regular and bold
— with the upstream `OFL.txt` beside it and a `CREDITS.md` entry. Loaded through
`Assets::load_bytes`, which is what makes the never-lies store contract apply
(assets.md §3): a face is built from bytes the store said it had, so a face
either has a real file behind it or was never created.

**5. No flag day.** The bitmap path keeps working, and keeps working *the same*:
the same cells, the same advance, the same numbers out of `width_of`. A game
opts into a loaded face per text draw, by naming one in a style.

### What the implementation had to decide, and did

**A face is rasterized at one texel per world unit, rounded, clamped to 6–64.**
`TextStyle::size` is a line height in world units, as it always was; the raster
size in texels is `round(size)`. So a game whose world units are reference
pixels — which is what `games/giri-rt`'s chrome already is, and what
`examples/text` sets itself up as — gets type rasterized at exactly the
resolution it is drawn at.

**The atlas's texture id is arithmetic, not an allocation.**
`2 + face_id * 128 + px`, inside the range below `1 << 32` that asset ids can
never reach. So `ctx.text` works out which texture a glyph samples from the face
and the size alone, with no store to consult and nothing to mutate — and can
therefore name an atlas that has not been rasterized yet. The driver rasterizes
what the frame asked for **between the draw and the plan**, which is the only
window in which the set of wanted atlases is known and the plan has not yet
resolved ids. An atlas nobody built resolves to the checkered placeholder, which
is the policy renderer.md §5 already sets for art in flight.

**A weight is a face, not a field on a style.** The approved draft's sketch had
a style of (face, size, weight, colour). Regular and bold are separate files
with separate outlines, so `Weight` as a style field would make
`TextStyle { face: built_in, weight: Bold, .. }` representable — a state with no
honest answer, which would have to be a silent fallback to regular, a panic, or
a lie. Making a weight *be* a face removes the state (practices §5.9).

**Cells are a uniform grid; quads are cut to the glyph.** The atlas is a 16×12
grid of identical cells, so a cell's place in it is arithmetic. A *quad*,
though, spans only the columns its own glyph inks, so an `i` is an `i` wide.
Without that, every glyph would be drawn as wide as the widest one in the face
and a floor asking whether a row fits inside a panel would be measuring an `M`
every time it looked at an `i`.

## Rationale

### `ab_glyph` over `fontdue`

Both are pure Rust, both are rasterizer-only by design, and neither pulls a
shaping stack — which is what the budget's real constraint is. The tiebreak is
the dependency budget itself (practices §5.8), measured rather than guessed:

| | new transitive crates | workspace total after |
|---|---|---|
| `ab_glyph` | **0** | 258 |
| `fontdue` | 4 | 262 |

`ab_glyph`, `ab_glyph_rasterizer`, `owned_ttf_parser` and `ttf-parser` are
already in the graph, pulled in by `winit` through `sctk-adwaita`, which draws
the client-side window decorations on Wayland. The crates are already fetched,
already compiled, already licence-reviewed. Choosing the one that is already
there is the preference order's "no dependency" case as nearly as a dependency
can get to it.

`fontdue`'s four are `fontdue`, `hashbrown`, `foldhash` and `allocator-api2` —
not a large tree, and not a reason to refuse it on its own. It is simply the
more expensive of two adequate answers. Neither is used for layout: this crate
calls `outline_glyph`, `px_bounds`, `draw` and `h_advance_unscaled`, and nothing
else.

The engine is not hand-rolling this. A TrueType parser and a scanline rasterizer
are thousands of lines of well-trodden work with a decade of edge cases in them,
and neither is this project's subject.

### Why the raster size does not come from the camera

The sharper answer is to rasterize at the size the text will actually occupy on
screen: multiply the world size by the camera's pixels-per-world-unit and
rasterize at that. It was declined.

The camera's `viewport` is **driver-maintained**: it describes the window
(renderer.md §4). A raster size derived from it is a raster size derived from
how big somebody's window happens to be — so the same game on two machines would
build different atlases, a golden image could not be compared across viewports,
and `face.atlas_texture(size)`, which is how a check asks *"which of these quads
is my heading?"*, would need the window to answer. That is environmental input
reaching into the draw path, which is the class of thing ADR-0005 exists to keep
out.

Deriving it from `GameConfig::window_size` instead — a game-declared constant —
would be deterministic, and was also declined, because it makes the atlas depend
on a number that describes a window a headless run never opens.

The cost of the rule that was taken is real and is worth writing down: **a game
whose world is twenty units across cannot get crisp type out of a loaded face.**
At `size: 1.6` the raster size clamps to 6 texels and is scaled up nine-fold.
The answer for such a game is the one `games/giri-rt` already takes for its
chrome — map a UI space onto the world and set type in it — and the answer for
this engine, if that stops being enough, is a per-face resolution stated once,
which is a smaller decision than this one and can be made when something needs
it.

### Why `width_of` and `columns_in` survive

"One way to do everything" says a second spelling of a measurement is a mistake.
Three things argued the other way, and they won:

- `width_of` **is** `measure(text).size.x`, and centring is what a game asks for
  far more often than it asks for a block's extents. `at.x - width_of(line) / 2`
  is the documented idiom; spelling it with a field access in the middle makes
  the commonest line of layout code in the repository worse.
- `columns_in` is a different question, not a second spelling of one. It asks
  how many characters of **any** string fit, and it is now answered with the
  face's *widest* advance — which makes its documented promise ("a line of
  `columns_in(w)` characters is never wider than `w`") true for a proportional
  face as well, where before it was true only because every character was the
  same. `fits_in` is the tight answer for a string in hand.
- Every existing floor, in two games and four examples, is written in these two
  words. Changing their spelling to land this feature would be exactly the flag
  day decision 5 refuses.

A fixed-advance face is measured by **multiplication** rather than by adding
character widths, and that is not a fast path — a hundred additions land a
rounding step below the same run multiplied, and `columns_in` would then count
one column short of what `width_of` had just measured. The round trip
`games/giri` G-003 asked for is an equality, and equalities do not survive
accumulation.

### Why a face is never freed

`Fonts` has no `destroy_face`, and a created face lives for the program. This is
the asset store's own v1 lifetime policy (assets.md §1: assets live until
`unload` or exit, no refcounting, no automatic drop), and it buys something
specific: `Face` is a plain `Copy` value, so `TextStyle` stays `Copy`, so a
layout can measure text **anywhere** — in a game's pure layout module, with no
world, no store and no borrow in sight. That is where `games/giri-rt`'s floors
do their measuring, and any design where measurement needs the store is a design
where those floors cannot be written.

The alternative that keeps freeing — a handle into the store, resolved at
measurement time — was declined for exactly that. Prototypes load two faces, not
two thousand.

## Consequences

- Readable type at three sizes and two weights, on native and in a browser over
  WebGL2. `examples/text` is the specimen sheet and `target/verify/text.png` the
  captured proof.
- The floors gained real metrics. `examples/text`'s own floors are asserted
  through `measure` end to end, and caught two genuine collisions the first time
  they ran.
- One new direct dependency, at a measured cost of zero new crates.
- Atlas memory is the renderer's. Eviction is deferred: Latin-1 at a handful of
  sizes is a megabyte or so, and a face is rasterized once per size for the life
  of the program.
- `docs/api/` gains a text section that says how to draw text and how to measure
  it; `docs/internal/renderer.md` §6 gains the loaded-face half.
- `TextStyle` gained a field, so every struct literal of it gained a line. That
  is the one place this reaches into code it did not have to.
- **One finding, filed where ADR-0034 says findings go.** Writing this feature's
  `--verify` turned up a gap that has nothing to do with type: a check that waits
  for an asset by ticking a fixed number of times is a race, because the native
  loader reads on a thread of its own. The same loop here resolved a font in 242
  ticks on a warm cache and had not resolved it in 600 on a cold one — and the
  failure reads as *the asset is wrong* rather than *the wait was short*. It is a
  fact about one API item, so it is in `FrameRecorder::settle_assets`'s doc
  comment with the loop written out, and `examples/text`'s `--verify` is the
  worked case. Nothing in `docs/api`'s prose changed for it.

## Deviations from the approved draft

One line each, as the handoff asked.

- **The draft was not attached to the implementing session at first**; the work
  started from the handoff prompt's constraints and the ADR was read into it
  when it arrived. Nothing in the design had to change.
- **`TextStyle` gained a `face` field, so `TextStyle { size, color, depth }`
  literals had to gain one line** — nine of them, across two games and four
  examples. Decision 5's "Pong and giri stay green untouched" is held to mean no
  redesign and no adoption of TTF, which is what happened: their pictures, their
  numbers and their verifies are unchanged.
- **Weight is a face rather than a style field**, so a single-weight face cannot
  be asked for bold and silently answer in regular.
- **The demonstration is `examples/text` rather than `prototype_kit`**, which
  the handoff allowed: prototype_kit's world is twenty units across, so a loaded
  face there would rasterize at the 6-texel clamp and the screenshot would argue
  against the feature.
- **`FrameRecorder::texture(id)` was added** beside `font_texture()`, because a
  face with one atlas per size makes "which quads are my heading" a question
  that takes an argument.
