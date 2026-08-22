# games/

Prototypes live here — one crate per game, on `main` (ADR-0038).

A prototype is an ordinary workspace member: `games/<name>/Cargo.toml`,
`games/<name>/src/main.rs`, and the files its `--verify` mode needs.
`crates/jidousha/examples/prototype_kit/` is the shape to copy.

Two rules, both mechanized:

- **Facade only.** A game depends on `jidousha` and on no other `jidousha-*`
  crate, directly or transitively. `tools/check-game-deps` fails otherwise.
- **It verifies itself.** A game takes `--verify`, scripts its own input, ticks
  headless and asserts. `tools/test` runs every game through `tools/verify`;
  there is no list to register it in.

Games get `cargo fmt`, `clippy -D warnings`, the ADR-0009 determinism bans and
`tools/check-assets`. They are exempt from `missing_docs`, the module-header
shape, and `tools/check-api-coverage` — the table in ADR-0038 says why.

Write one with the `make-game` skill, which owns this workflow. Game code reads
`docs/api/` and `crates/jidousha/examples/` — never `crates/*/src/`.

Retired prototypes move to `attic/`, not to `rm`.

DELIBERATE: this file is load-bearing, not only documentary (see ADR-0038).
Cargo resolves a `members` glob that matches nothing to a literal path and then
fails to read `games/*/Cargo.toml`. While no prototype exists, this README is
what `games/*` matches. Do not delete it to tidy an empty directory.
