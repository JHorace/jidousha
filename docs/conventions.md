# Conventions

Cross-cutting conventions, stated once. Where a convention can live in a type, it
does (practices §1.3) — this file is the human/agent-readable index, the types are
the enforcement. Every entry here is assumed by all subsystem docs.

## Coordinates and space

- **World space: X right, Y down, right-handed (+Z into screen) — matches
  Vulkan NDC. Positive rotation is clockwise on screen** (right-hand rule about
  +Z). DELIBERATE: screen-natural over math-canonical; do not "fix" this to
  Y-up. Gravity is `+y`; jumping is `-y`, and the 3D story follows from the same
  choice (ADR-0010).
- World units are abstract (not pixels). The camera defines the world↔pixel
  relationship (renderer.md §5).
- **Screen space: pixels, origin top-left** — same orientation as world space,
  differing only in units and camera offset. It exists only at the platform
  boundary (raw pointer events, window sizes); the ONLY sanctioned conversion
  is through camera methods (`world_to_screen` / `screen_to_world`).
- Angles: **radians, always**, as the `Radians` newtype in public APIs. Degrees
  appear nowhere in the engine. `Radians::from_degrees` exists for humans.

## Time

- Simulation time: `tick: u64` (canonical) and `Seconds` newtype (derived,
  `tick * fixed_dt`). Wall-clock time is banned outside `jidousha-platform`
  (ADR-0005; CI-checked).
- Durations in public APIs are `Seconds(f32)`, never milliseconds, never bare f32.
- **Speeds and rates are per second, multiplied by `Time::fixed_dt` where they
  are applied — never per tick.** A per-tick constant is the same arithmetic
  with the timestep baked into it, so it silently means something else the day
  `GameConfig::fixed_dt` changes, and it cannot be read against a number a
  person quotes in seconds. Counting *ticks* is the exception, because the tick
  is the canonical timeline above: a serve pause is 45 ticks, not 0.75 seconds
  converted twice. Every example follows this — a game author reads two of them
  and infers the rule, so two that disagree teach that there is no rule.

## Color

- `Color` = f32 RGBA, **sRGB-encoded, 0.0–1.0**, straight (non-premultiplied)
  alpha. What agents and humans mean by "0.5 gray" — linearization happens inside
  the render backend, invisibly.
- **Alpha is the exception to "invisibly": it reads brighter than the number
  looks.** Blending happens in *linear* light, where it is physically right, so
  a low alpha over a dark background lands much higher than the figure suggests
  — 0.06 white on near-black reads as solid grey, not as a hint. Pick faint
  overlays (grid lines, field markings, dimmers) by eye from a capture rather
  than by arithmetic, and start lower than feels right.
- Constructors: `Color::rgb(r, g, b)`, `Color::rgba(...)`, plus a small named set
  (`Color::WHITE`, `Color::MAGENTA`, …). No 0–255 constructors in v1 (one way).

## Draw ordering

- Sort key: (`layer: i16`, then `z: f32`, then submission order; stable).
  `layer` is the coarse tool (background/world/UI bands); `z` orders within a
  layer. **Higher `z` draws on top** (z-index semantics; `z` is a draw-order
  key, NOT a coordinate on the spatial +Z axis, which points into the screen).
  NaN `z` is a contract violation (debug-checked). The two senses of "z" are
  the same choice seen twice (ADR-0010).
- Within-frame determinism: identical submissions → identical order, always.

## Naming vocabulary (practices §5.3)

- `create` / `destroy` — object lifetime · `load` / `unload` — assets
- `get`-class (`component`, `resource`) — infallible, panics per §9 taxonomy
- `find_*` — returns `Option` · `try_*` — returns `Result`
- `insert` / `remove` — components and resources on an entity or world
  (`core.md` §2, §6). `remove` is banned only as a *synonym for `destroy`*:
  destroying an object is `destroy`, taking a component off a live entity is
  `remove`.
- Banned synonyms: `make`, `fetch`, `lookup`, `obtain`, `new_*` functions
  outside `T::new`. One constructor per type; `Default` only where the default
  value is itself meaningful (ADR-0012).
- Files are snake_case and named for the primary type they contain.

## Math

- glam types (`Vec2`, `Vec3`, `Mat4`) with `scalar-math`; engine newtypes for
  units (`Radians`, `Seconds`). Std float trig is clippy-banned engine-wide —
  use `sin_cos`, `atan2` and `rotate` from `jidousha::math` (ADR-0009).
- **An angle a game states once is a `const`, written in degrees.**
  `Radians::from_degrees` is a `const fn`, so
  `const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);` compiles. The
  alternatives are both worse: `Radians(1.0471976)` is rejected by clippy as an
  approximation of `FRAC_PI_3`, and `Radians(core::f32::consts::FRAC_PI_3)`
  stops being writable at fifty degrees. Constructors and accessors of the
  plain-data types — `Radians`, `Seconds`, `Color`, `Depth`, `PhysicalSize`, the
  typed handles — are `const fn` for this reason, and a new one follows the same
  rule. `from_degrees` was the one that was not, and E0 run 6 found it the only
  way this is findable: by trying to write the constant (e0-findings.md F-069).
  **Then `PhysicalSize::aspect` was the one that was not**, five runs later and
  by the identical method — a game deriving its half-width from the window it
  opens at (e0-findings.md F-137), so a layout's
  `const HALF_W: f32 = HALF_H * WINDOW.aspect();` compiles too. `Rect` is
  deliberately absent from that list: its accessors are glam `Vec2` arithmetic,
  which is not `const fn` upstream, so a layout in constants states its extents
  as numbers and builds the `Rect` where it is used.
- **A game spells them from the prelude and nowhere else.** `jidousha::prelude`
  re-exports every name in `math`, so `use jidousha::prelude::*;` is the whole
  import and a second `use jidousha::math::sin_cos;` beside it is the same item
  twice. Engine-internal code has no facade to reach through and names its own
  module path; that spelling is the engine's and the prelude is the game's, which
  is what "one way to do everything" means here —
  E0 run 4 found two worked examples disagreeing about which (e0-findings.md
  F-045).
- **And that holds for any name the prelude has, not only `math`'s.** A few are
  in the prelude *and* in `jidousha::testing` — `PhysicalSize` is the one that
  bites, because `FrameRecorder::new` takes one — since the testing surface has
  to define what its own signatures name (F-017's rule). A game globs the
  prelude, so it takes them from there and lists only the testing-*only* names
  in its `use jidousha::testing::{..}`. E0 run 7 copied the other spelling out of
  `prototype_kit`, which had it wrong, and the document said the class of thing
  was settled (e0-findings.md F-088).

## Error messages

- Format and taxonomy: core doc §9 (`[jidousha]` prefix, what/specifics/likely
  cause/fix, system name included). Applies to every crate, compile-time
  diagnostics (`on_unimplemented`) included.
