# ADR-0012: One constructor per type; `Default` only for meaningful defaults

Status: accepted · 2026-08-15

## Context

`World::new()` triggers `clippy::new_without_default`, whose advice is to add a
`Default` impl forwarding to `new`. Rust convention agrees: a type with an
argument-less `new` is expected to implement `Default`.

That advice collides with the engine's first rule: one way to do everything
(agent-practices §5.3). A `Default` impl that only forwards to `new` creates a
second spelling of the same operation. Generated game code would then split
between `World::new()` and `World::default()`, and every example, doc snippet,
and future skill would have to pick one anyway.

The pressure is real in both directions, because the engine *does* want
`Default` in places: `GameConfig::default()` with struct-update syntax is the
documented way to configure a run, and `InputSnapshot::default()` is the neutral
input for a tick (core.md §8).

## Decision

- Every engine type has **exactly one constructor**, named `new` (or a named
  constructor stating what it builds). It is the only documented way to make one.
- **`Default` is implemented only where the default value is itself meaningful**
  to game code — a neutral configuration, an empty input snapshot, a zero vector
  — and not as an alias for a constructor that already exists.
- Where `clippy::new_without_default` fires on a type in the first category, it
  is silenced at the site with `#[allow(clippy::new_without_default)]` and a
  `DELIBERATE:` tag pointing here.

## Rationale

The lint optimizes for library ergonomics in a large ecosystem, where generic
code bounded by `T: Default` is common. This engine's public surface is consumed
almost entirely by generated game code that never writes such bounds, and its
overriding constraint is that there be one obvious spelling for each operation.

The test in practice: does the *value* mean something ("an empty config", "no
input this tick"), or does the impl merely mean "call `new`"? The first earns a
`Default`; the second is a synonym.

## Consequences

- `World::new()` stands alone. `World::default()` does not exist, so code
  written against the examples cannot drift into a second spelling.
- Each suppression is local and tagged, so a future session reading the allow
  finds this decision instead of "fixing" the lint.
- Types whose defaults *are* meaningful still get `Default`, which keeps
  `GameConfig { seed: 42, ..GameConfig::default() }` (core.md §8) intact.

## Alternatives rejected

- **Implement `Default` everywhere the lint asks**: two ways to build a world,
  for the benefit of generic code this engine's users do not write.
- **Drop `new` and keep only `Default`**: `World::default()` is a worse name for
  "create an empty world", and `new` is what an agent greps and guesses first.
- **Allow the lint crate-wide**: silences it for the cases where it is right
  (a type with a genuinely meaningful default), and hides the decision from the
  place it matters.
