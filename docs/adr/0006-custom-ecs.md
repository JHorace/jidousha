# ADR-0006: ECS implementation — custom, from scratch

Status: accepted · 2026-08-15

## Context

ADR-0002 fixed the architecture as full ECS but left the implementation open:
custom, `hecs`-based, or `bevy_ecs`. The ECS is the engine's heart and the largest
surface game agents touch; the properties this engine exists for — guaranteed
determinism, agent-grade error messages, a small controlled API — all live or die
here.

## Decision

**Write a custom ECS**, minimal and archetype-based, designed to the requirements of
ADR-0002 (stable iteration order, explicit ordered schedule, generational entity IDs)
rather than to feature parity with existing crates.

Deliberately minimal initial feature set: entities, plain-data components, queries
over component sets, an explicit system schedule, and commands (deferred structural
changes). No change detection, no events-in-ECS, no parallel execution in v1 — each
is added only when a real engine/game need demands it, via its own design note.

## Rationale

- **Determinism as a guarantee, not an observation.** Existing crates make iteration
  order an implementation detail; we make it a documented, tested contract. This
  underwrites the entire verification story (replay, golden-state tests).
- **Error messages are ours.** Query conflicts, missing components, and misuse can
  fail with what-happened/likely-cause/fix text (§5.5) instead of a generic panic
  or an opaque trait-bound error.
- **Small surface.** bevy_ecs's power comes with an enormous API that contradicts
  "one way to do everything"; hecs would still need a scheduler, change tracking,
  and an error layer wrapped around types we don't control.
- **Fits the project thesis.** The engine is agent-developed; an ECS is well-trodden,
  highly testable (property tests over world operations), and exactly the kind of
  component the build process is meant to prove out.

## Consequences

- Largest single chunk of pre-sprite work. Accepted; the core design doc will slice
  it into small, independently testable milestones.
- The detailed design (storage layout, query semantics, schedule rules, command
  buffers, ID recycling) belongs to the engine-core design doc, not this ADR.
- Property/model-based tests are mandatory from the start: a reference "naive world"
  implementation checked against the real one under random operation sequences.
- We accept re-deriving known solutions (archetype moves, borrow rules). Prior art
  (hecs, bevy_ecs, flecs docs) is fair to study; code is written fresh against our
  contracts.

## Alternatives rejected

- **hecs + custom scheduler**: fastest respectable path, but determinism and error
  text remain wrapped, not owned; two idioms (hecs's and ours) leak into one API.
- **bevy_ecs standalone**: best features and training-data familiarity, but huge
  surface, no by-guarantee determinism, real major-version churn.
