# ADR-0007: Systems are plain `fn(&mut World)` — no parameter extraction

Status: accepted · 2026-08-15

## Context

The dominant Rust ECS idiom (Bevy) declares systems as functions whose parameters
(`Query<...>`, `Res<...>`) are extracted by trait machinery at schedule time. Agents
know this idiom well from training data, so adopting it has real familiarity value.
The question is whether that value survives contact with this engine's goals.

## Decision

Systems are ordinary functions taking `&mut World`. Queries and resource accesses
are constructed inline inside the function body. No extraction traits, no
all-tuples macros, no derive-based system registration.

Aliasing is enforced at runtime by column-level borrow flags with agent-grade
panic messages (core design doc §5, §9).

## Rationale

- **Parameter extraction buys parallel scheduling** — access sets declared in
  signatures let a scheduler run non-conflicting systems concurrently. We
  deliberately have no parallel scheduling (ADR-0002/0006), so we'd pay the
  idiom's costs for a benefit we've excluded.
- **Its failure mode is our anti-goal.** Trait-bound errors from extraction
  machinery ("the trait `SystemParam` is not implemented for...") are among the
  worst diagnostics in the Rust ecosystem — precisely the illegible-error class
  this engine exists to eliminate. Plain functions fail with our own messages.
- **Greppability and zero magic.** A system is a function; its name, body, and
  every access are plain text a grep finds. Nothing is generated.
- Bevy-familiarity mostly transfers anyway: the concepts (queries, resources,
  systems, commands) are identical; only the signature shape differs, and the
  examples directory teaches that shape in seconds.

## Consequences

- Aliasing violations surface at runtime, not compile time. Accepted: Bevy's are
  runtime too (at extraction); sequential execution makes conflicts rare (nested
  queries only); and the panic message names both sides of the conflict.
- If parallelism ever arrives (own ADR), access declarations get revisited —
  likely as explicit registration metadata, still not signature magic.
- `DELIBERATE:` tags at the schedule/system-registration code point here.

## Alternatives rejected

- **Bevy-style extraction**: see above — cost without the enabling benefit.
- **Macro-registered systems** (`#[system]`): hides registration from grep;
  violates the no-macro-generated-public-symbols rule.
