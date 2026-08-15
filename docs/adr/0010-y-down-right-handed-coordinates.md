# ADR-0010: World coordinates are Y-down, right-handed (Vulkan-NDC-aligned)

Status: accepted · 2026-08-15
(Planning-phase revision: an earlier same-day draft of this ADR said left-handed;
revised to right-handed before any code existed. ADR immutability applies from
acceptance in the initialized repo onward.)

## Context

The initial conventions draft chose Y-up right-handed (math-natural). Revisited
at the owner's direction in favor of screen-natural Y-down; refined to
right-handed to match **Vulkan NDC** (X right, Y down, Z into screen), which is
where the render stack ultimately points (ash future, ADR-0003).

## Decision

- **World space: X right, Y down, right-handed — +Z into the screen.**
  Positive rotation about +Z is, by the right-hand rule, **clockwise on
  screen**.
- World space, screen space, and Vulkan NDC all share orientation; spaces differ
  only in units and camera offset/scale. Conversions still go exclusively
  through the camera methods.
- Draw order: **higher `z` draws on top** (z-index semantics). DELIBERATE:
  draw-order `z` is a sort key, NOT a position on the spatial +Z axis — under
  this ADR spatial +Z points *into* the screen, so "spatially closer" would be
  *smaller* z. We keep z-index semantics anyway because it is what the 2D corpus
  (web, Godot `z_index`) trained agents to expect; the renderer maps sort order
  to whatever the backend needs. If 3D arrives, true depth uses the spatial
  axis and its own conventions.

## Rationale

- Screen-natural: "down" in code is down on screen — gravity is `+y`, UI reads
  top-to-bottom. Matches the 2D corpus (Godot, canvas, pixel-art thinking) and
  removes a class of sign errors from generated gameplay code.
- Vulkan NDC alignment: world → clip space involves no Y-flip anywhere in the
  stack — fewer sign surprises at the backend seam today (wgpu) and a cleaner
  path to ash (ADR-0003). (wgpu/WebGPU NDC is Y-up; the wgpu backend owns that
  one flip inside its projection matrix, invisibly — CONTRACT: no flip logic
  outside a backend.)
- Right-handed keeps standard math conventions intact: rotation matrices,
  cross-product identities, and glam behave textbook-style; only the visual
  reading ("clockwise") differs from blackboard habit, and that follows from
  the axes, not from modified math.

## Consequences

- Positive angles are clockwise on screen; comments and docs say "clockwise",
  never "CCW".
- Camera `world_to_screen` is scale + translate, no flip.
- 3D headroom improves relative to the left-handed draft: Y-down right-handed
  extends directly to a Vulkan-style 3D space if we want it; the 3D ADR can
  also still choose a distinct 3D convention with an explicit mapping. Either
  way, a future agent must NOT "fix" 2D to Y-up — that is what the
  `DELIBERATE:` tag at the conventions definition site prevents.

## Alternatives rejected

- **Y-up right-handed**: math-canonical, but every generated platformer pays a
  sign-flip tax against 2D-corpus habits, and screen↔world reasoning carries a
  permanent flip.
- **Y-down left-handed**: identical on-screen behavior in 2D, but breaks
  standard right-hand-rule identities for no benefit and diverges from Vulkan
  NDC's usual right-handed reading.
