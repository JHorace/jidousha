# Conventions

Cross-cutting conventions, stated once. Where a convention can live in a type, it
does (practices §1.3) — this file is the human/agent-readable index, the types are
the enforcement. Every entry here is assumed by all subsystem docs.

## Coordinates and space

- **World space: X right, Y down, right-handed (+Z into screen) — matches
  Vulkan NDC. Positive rotation is clockwise on screen** (right-hand rule about
  +Z). DELIBERATE: screen-natural over math-canonical — see ADR-0010; do not
  "fix" this to Y-up. Gravity is `+y`; jumping is `-y`; the 3D story is covered
  in ADR-0010's consequences.
- World units are abstract (not pixels). The camera defines the world↔pixel
  relationship (`docs/internal/renderer.md` §5).
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
  key, NOT a coordinate on the spatial +Z axis, which points into the screen —
  ADR-0010). NaN `z` is a contract violation (debug-checked).
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
  use `jidousha::math::{sin_cos, atan2, ...}` (ADR-0009).

## Error messages

- Format and taxonomy: core doc §9 (`[jidousha]` prefix, what/specifics/likely
  cause/fix, system name included). Applies to every crate, compile-time
  diagnostics (`on_unimplemented`) included.
