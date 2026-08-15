# ADR-0003: Render backend — wgpu now, ash later

Status: accepted · 2026-08-15

## Context

We need a GPU abstraction that works on Linux, Windows, and web (ADR-0005) today,
with a desired future migration to `ash` (raw Vulkan bindings) on native platforms
for control and performance.

The tension: **ash is Vulkan-only and the web has no Vulkan.** A future ash migration
therefore does not *replace* the portable backend — it adds a second, native-only
backend beside whatever serves the web. "Migrate to ash" really means "become
multi-backend."

## Decision

1. **wgpu is the render backend now**, on all platforms.
2. **The renderer is architected for backend replacement from day one**: all wgpu
   usage lives in one backend crate behind an internal engine-defined interface
   (a compact command/resource API designed for our 2D needs, not a general RHI).
3. **Isolation rule (CI-enforced): no `wgpu` type appears outside the backend
   crate.** Not in the public API, not in other engine crates, not in examples.
   Enforced by forbidding the `wgpu` dependency everywhere else.
4. Core rendering features stay within a **WebGL2-compatible envelope** for now
   (no compute shaders, texture/limit budgets checked against GLES3/WebGL2), so
   wgpu's GL fallback remains viable on hardware without WebGPU. Revisit when
   WebGPU coverage makes the fallback unnecessary.

## Rationale

- wgpu is pure Rust (dependency policy, ADR-0001), runs on Vulkan/DX12/Metal/WebGPU/
  WebGL2, and is the best-represented GPU API in Rust training data.
- The isolation rule is what makes the ash future real: if wgpu types leak, migration
  cost grows monotonically until it never happens. The rule also keeps game agents
  fully insulated from GPU concepts.
- A narrow engine-specific backend interface (sprites, batches, textures, render
  passes we actually use) is achievable; a general-purpose RHI is a tar pit.

## Consequences

- Slight upfront cost: even the first triangle goes through the backend interface.
  Accepted — retrofitting isolation is far costlier.
- The backend interface is internal (`docs/internal/renderer.md`), not public API.
  It may change freely; only the backend crate and renderer core see it.
- When ash lands: native uses ash, web keeps wgpu (or a WebGPU-direct path);
  both implement the same interface. CI runs both.
- Backend crate naming: `jidousha-render-wgpu` (later `jidousha-render-ash`).

## Alternatives rejected

- **wgpu types in public API**: fastest today, forecloses ash, couples games to
  backend churn.
- **ash immediately + separate web renderer**: two backends before one sprite ships.
- **Custom OpenGL/WebGL layer**: abandons modern native APIs and fights wgpu's
  entire value proposition by hand.
