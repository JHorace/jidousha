# ADR-0015: The draw submission vocabulary lives in core, named by opaque texture ids

Status: accepted · 2026-08-16

## Context

Two rules that were each right on their own met, and disagreed.

- ADR-0008 puts `DrawCtx` in `jidousha-core`: the Draw phase is typed by the
  schedule, and Draw-immutability is enforced by `DrawCtx` exposing no mutating
  method. The phase and its context have to live where the schedule is.
- core.md §1 is contract-marked: **`jidousha-core` has no dependency on any
  other jidousha crate.** That is what makes it compile on every target with
  zero cfg branches, and what keeps `wgpu` and `winit` provably out of it.

`DrawCtx` needs a submission sink. The sink has to speak some vocabulary, and
the central verb is "draw this sprite" — where a sprite names a texture, and
textures belong to `jidousha-assets`. Core cannot name a `TextureHandle`.

Four ways out were considered; the sketch in renderer.md §2 assumed the problem
away by writing `ctx.sprite(&Transform, &Sprite)` without saying where either
type lived.

## Decision

**Core owns the submission sink and a texture-agnostic vocabulary. The renderer
owns everything that knows what a texture is.**

- `jidousha-core` gains `Quad` — four world-space corners, four texture
  coordinates, a tint, a `Depth`, and a `TextureId` — plus `Color`, `Rect`,
  `Depth`, `Transform`, and `DrawCtx::submit(Quad)`.
- `TextureId` is an **opaque `u64` with no meaning in core**. It is not a
  handle, cannot be dereferenced, and answers no questions. `TextureId::WHITE`
  is reserved for untextured shapes.
- `jidousha-assets` mints one from a handle: `TextureHandle::texture_id()`.
  Assets already depends on core, so that edge exists.
- `jidousha-render-core` owns `Sprite`, `Camera`, expansion, sorting, batching,
  `FramePlan`, and the `Submit` extension trait that adds `ctx.sprite(...)` to
  `DrawCtx`. Games get `Submit` from the prelude and never name it.
- The id→texture mapping lives in render-core's `TextureTable`. **An id nobody
  registered draws the placeholder**, which is how renderer.md §5's not-ready
  policy is implemented without anyone asking assets a question.

## Rationale

- **The contract that had to hold, held.** Core still depends on no jidousha
  crate. What crossed the line is a `u64` with a name, not a dependency.
- **Expansion stays above the seam and outside core.** Core never learns what an
  anchor, an atlas region, or a placeholder is; it carries four corners someone
  else computed. All the cleverness is still in render-core, which is what
  renderer.md §1 asks for.
- **The not-ready policy falls out for free.** "Unregistered id → placeholder"
  covers both a texture still loading and one that failed, without a status
  lookup, without assets being consulted at draw time, and without a code path
  that can be wrong about which case it is in.
- One vertex format and one pipeline was already the plan (renderer.md §7).
  Making `Quad` *the* submission — not one of several — means rectangles, lines,
  circles, and glyphs (R3) all arrive through the same door, and the sort and
  batch code never grows a second case.

## Consequences

- A rendering-shaped type sits in the ECS crate. It carries a `DELIBERATE:` tag
  pointing here, because "why is `Quad` in core?" is a fair question with a
  non-obvious answer.
- `ctx.sprite(...)` needs the `Submit` trait in scope. Inside the engine that is
  an import; for games the facade's prelude carries it (F0), so a game agent
  never encounters the seam. Until the facade exists, examples import it.
- `TextureId::from_bits` is public, so anything can mint an id. That is
  deliberate: the id is opaque and the *mapping* is the authority, so a forged
  id is not unsafe — it draws the placeholder, exactly like one whose texture
  has not arrived.
- If a future subsystem needs a second thing in the sink that core cannot name
  (a font, a shader), it gets the same treatment: an opaque id in core, the
  mapping in the crate that owns the concept. If that happens more than once
  more, the pattern deserves its own small abstraction rather than a third
  bespoke id type.

## Alternatives rejected

- **Move `DrawCtx` and `Draw` into render-core.** Cleanest on paper, and it
  requires core's `App` to accept phases it does not know — a plugin system, in
  a codebase that has deliberately avoided one (ADR-0006, ADR-0007). It would
  also make `App` generic over the draw context, which leaks into every game's
  setup closure.
- **A type-erased sink (`&mut dyn Any`) that render-core downcasts.** Adds a
  runtime failure path to the one place that must never have one, and buys
  nothing over an opaque id.
- **Let core depend on `jidousha-assets`.** Breaks a contract-marked rule to
  avoid a `u64`.
- **Put `Sprite` in core.** Same thing by another route: `Sprite` names a
  `TextureHandle`, so core would depend on assets anyway.
