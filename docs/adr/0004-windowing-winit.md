# ADR-0004: Windowing — winit

Status: accepted · 2026-08-15

## Context

We need window creation, an event loop, and raw input events on Linux, Windows, and
web, with Android later. The user expressed no preference; the constraint set decides.

## Decision

**winit**, wrapped entirely inside the engine's platform crate. The isolation rule
from ADR-0003 applies identically: **no `winit` type appears outside the platform
crate.** Engine input types (ADR'd with the input system design) are our own.

## Rationale

- Only mature pure-Rust option covering all three targets plus Android, and the
  windowing layer wgpu is designed against and tested with.
- Best-represented in training data by a wide margin.
- Web support matters most: winit abstracts the browser's callback-driven main loop
  (`requestAnimationFrame`) behind its `ApplicationHandler` model, which drives the
  core-loop design in ADR-0005.

## Consequences

- The engine's application lifecycle API must be callback/trait-shaped, not an
  owned `loop {}` — this is winit's model and the web's requirement anyway.
  Core design docs must reflect it.
- winit's input events are translated at the platform boundary into engine input
  types; game agents never see winit.
- Known cost: winit has API churn between majors. Isolation confines each upgrade
  to the platform crate.

## Alternatives rejected

- **SDL2/SDL3 bindings**: mature, but C-linking (dependency policy) and a worse
  wasm story.
- **Per-platform hand-rolled**: three platforms' worth of undifferentiated pain.
- **glfw**: C-linking, no web.
