# ADR-0002: Full ECS architecture

Status: accepted · 2026-08-15

## Context

The engine needs a core object model. Rust's ownership rules punish pointer-heavy
scene graphs; agents in particular spiral on borrow-checker errors from shared mutable
object graphs, escaping via `Rc<RefCell<>>`, cloning, or `unsafe`. The object model is
also the primary API surface game-writing agents touch, so uniformity matters more
than expressive variety.

## Decision

**Full ECS**: entities as opaque IDs, components as plain data, logic in systems run
by an explicit schedule. This is the *only* object model — no parallel scene-graph or
inheritance path.

Determinism requirements (non-negotiable, serve agent self-verification):

- Entity iteration order is **defined and stable** for a given world history.
- The system schedule is **explicit and ordered** — no implicit parallel scheduling
  in the default configuration; parallelism, when added, must preserve observable
  ordering.
- Entity IDs are deterministic (generational indices; no randomness, no pointers).

## Rationale

- Dissolves the borrow-checker failure mode: systems borrow disjoint component sets;
  entity references are copyable IDs, never `&mut` chains.
- One uniform pattern: every gameplay feature is "components + a system." Uniformity
  makes generated code predictable and reviewable; agents pattern-match one shape.
- ECS is heavily represented in Rust training data (Bevy), so the *paradigm* costs
  agents nothing even where our API differs in detail.
- Plain-data components serialize trivially → world snapshots, golden-state tests,
  and replay come nearly for free (verification harness).

## Consequences

- The implementation choice (custom vs. existing crate) is a separate decision:
  **ADR-0006**.
- The schedule, not the ECS storage, is where determinism lives; the core design doc
  must specify system ordering rules precisely.
- API design must keep query/system ergonomics simple enough that a game agent never
  needs to understand archetype storage.

## Alternatives rejected

- **Scene graph / OOP objects**: the borrow-checker trap; two ways to model
  everything once components sneak in anyway.
- **Hybrid (ECS + node tree, Godot-style)**: two object models doubles the API
  surface and violates "one way to do everything."
