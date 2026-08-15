# ADR-0013: Query access through the borrow checker, not runtime borrow flags

Status: accepted · 2026-08-15

## Context

The core design doc (§5) specified two things that cannot both hold:

1. "Queries take `&World`; column-level borrow flags (RefCell-style, per
   component type) enforce aliasing rules at runtime."
2. "Point access: `world.component::<T>(e) -> &T` … plus `_mut` variants."

If a query can hand out `&mut T` from a `&World`, then `component::<T>()` —
which also takes `&World` and returns a bare `&T` — can produce a shared
reference to the very value the query is writing through. Nothing catches it:
both calls borrow the world immutably, so the compiler is satisfied, and a
borrow flag released when `component` returns cannot protect a reference that
outlives it. It is undefined behaviour reachable from entirely safe game code.

Every sound resolution gives something up. This ADR records which.

## Decision

**Mutable iteration takes `&mut World`; read-only iteration takes `&World`.**

- `world.query::<Q>()` — `&self`, parts restricted to `&T`, `With<T>`,
  `Without<T>` by the `ReadOnlyQuery` bound.
- `world.query_mut::<Q>()` — `&mut self`, parts may be `&mut T`.

Point access keeps its M1 shape: `component`/`find_component` return `&T`,
`component_mut`/`find_component_mut` return `&mut T` from `&mut self`. The
naming mirrors the pairs that already exist.

There are no runtime borrow flags and no `unsafe` in `jidousha-core`. Overlapping
access is a **compile error**, not a panic. One runtime check remains, for the
case the type system cannot see: a query naming the same component twice
(`(&mut Position, &Position)`) panics with the §9 message format.

Using `&mut T` in `query` is rejected by a `#[diagnostic::on_unimplemented]`
message in the §9 style, naming the cause and the fix — the same mechanism
ADR-0008 requires for the Draw phase.

## Rationale

- **ADR-0001 makes statically-caught wrongness the dominant criterion.** This
  turns a class of runtime panic into a compile error, which the repair loop
  sees without running the game.
- **No `unsafe` in the engine core.** Interior mutability would put raw pointer
  aliasing at the heart of the most-exercised code in the engine, defended only
  by hand-written flags — the highest-risk code we could write, needing tooling
  (Miri) the project does not yet run.
- **The M1 point-access API survives unchanged.** The alternative would have
  replaced `&T` with guard objects everywhere, which leaks lifetime machinery
  into the API game agents touch most.
- **ADR-0007's headroom is kept.** The access set a query needs is a value
  (`QueryAccess`), not a predicate hidden in a trait, so a future parallel
  scheduler can still collect and compare per-system access sets.
- **ADR-0008 fits.** A read-only Draw world view is exactly `&World` plus the
  `ReadOnlyQuery` bound, with the compile-time rejection already in place.

## Consequences

- **The sharp edge**: a mutable query holds the whole world, so game code cannot
  point-read another entity while iterating one. The workaround is to collect
  what is needed in a read-only pass first (`query` then `query_mut`), or, from
  M3, to defer the work through commands. This is documented on `query_mut` and
  in `docs/internal/core.md`.
- Conflicting queries can never be nested, so the panic core.md §5 described —
  naming both queries and the running system — has no cases left to report. The
  system name still lands in M4 for the panics that remain.
- Two entry points instead of one, distinguished by access exactly as
  `component`/`component_mut` already are. Not a second way to do one thing: a
  read-only query cannot write, and a writing query cannot run against a shared
  world.
- `docs/internal/core.md` §5 is corrected in the same commit; this ADR is the
  record of why it changed.

## Alternatives rejected

- **`&World` + interior mutability + guard-returning point access** (the hecs
  shape): keeps one `query` entry point, but requires `unsafe` in core, demotes
  compile errors to runtime panics, and changes `component::<T>() -> &T` into a
  guard type across the whole public API.
- **`&World` + `for_each` closures**: safe and single-entry, but abandons the
  `for` loop that both the design doc and every ECS an agent has seen use, and
  *still* needs guards on point access to be sound.
- **`&mut World` for every query, read-only included**: one entry point, but a
  read-only pass would then lock the world, breaking cross-entity reads that are
  safe by construction — and leaving ADR-0008's shared Draw view with no query.
