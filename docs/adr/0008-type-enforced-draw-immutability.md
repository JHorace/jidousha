# ADR-0008: Draw-phase immutability is type-enforced

Status: accepted · 2026-08-15

## Context

The determinism contract (core doc §7) makes simulation state a pure function of
(seed, systems, per-tick inputs). Update runs a fixed number of times per unit of
game time; Draw runs once per rendered frame, and frame rate varies by machine. A
Draw system that mutates simulation state therefore makes game state depend on the
player's hardware, breaks replay, and silently diverges from headless mode (which
never runs Draw). Draw must be read-only with respect to the world.

Candidate enforcement: type-enforced (read-only Draw context; violations are
compile errors), convention plus a debug-build world-hash check (violations are
runtime panics when exercised), or the hash check now with a later upgrade path.

## Decision

**Type-enforced.** Draw systems have a different signature from Update systems:

```rust
fn draw_sprites(ctx: &mut DrawCtx) {
    for (e, pos, sprite) in ctx.world.query::<(&Position, &Sprite)>() {
        ctx.draw(...);   // submission sink — the renderer doc owns its shape
    }
}
```

- `ctx.world` is a **read-only world view**: it exposes `query` constrained to
  read-only access tuples (`&T` yes, `&mut T` no), `component`/`find_component`,
  and read-only `resource`. Mutation methods do not exist on the type.
- The read-only constraint is a trait bound (`ReadOnlyAccess` on query tuples)
  carrying a `#[diagnostic::on_unimplemented]` message written to our §9 error
  standard — e.g. *"Draw systems cannot take `&mut Position`; Draw runs per
  rendered frame, not per tick, so mutations here break determinism. Read with
  `&Position`, or move this logic to an Update system."* The compile error itself
  teaches the rule.
- Phases are types, and each phase names its system signature
  (`Update` → `fn(&mut World)`, `Draw` → `fn(&mut DrawCtx)`), so registering the
  wrong shape in the wrong phase is also a compile error.
- The verification harness keeps a cheap world-hash check across Draw as
  defense-in-depth against interior-mutability escapes (`unsafe`, atomics/cells
  inside resources) that no type can see.

## Rationale

- A compile error is a better teacher than a debug panic, and game-facing code is
  written by agents: the repair loop fixes a compile error without ever running
  the game. Draw is exactly where a game agent will casually clamp a `Position`.
- "Make illegal states unrepresentable" (practices §5.9) is the engine's own
  principle; this is its clearest application.
- The "second API surface" cost is mitigated: the read-only view reuses the same
  query implementation and syntax, restricted by one trait bound — one query API
  at two capability levels, not two APIs.

## Alternatives rejected

- **Convention + debug hash check alone**: near-zero API cost, but violations
  surface at runtime and only on exercised paths; weaker teacher for agents.
- **Middle path (hash now, view later)**: defers the signature decision the
  renderer doc needs today; retrofitting `DrawCtx` later breaks every game's
  draw systems — the one thing the middle path was meant to avoid.
