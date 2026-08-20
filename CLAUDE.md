# CLAUDE.md

<!-- Router file. Navigation, not knowledge. Hard cap 150 lines (CI-enforced).
     Rationale for every rule here: docs/agent-practices.md -->

**Jidousha** — agent-first game engine in Rust. 2D now; 3D planned — don't preclude it.
Agent-developed and agent-used. Full ECS (ADR-0002) · wgpu behind a swappable backend
boundary (ADR-0003) · winit (ADR-0004) · targets Linux/Windows/Web (ADR-0005).

## Commands

```
cargo check                 # fast validation — run after every edit
cargo check --target wasm32-unknown-unknown  # web target — CI-gated, keep it green
tools/test                  # test wrapper — ALWAYS use this, not bare cargo test
tools/doctor                # environment self-diagnosis
cargo clippy -- -D warnings # lint; warnings are errors
cargo fmt                   # format (also runs via hook)
cargo run -p jidousha --example <name>   # run a canonical example
tools/verify <example>      # headless deterministic run + assertions + a captured PNG
tools/check-assets          # every asset path in the code names a file that exists
tools/serve-web <example>   # build for web, serve it; --check drives a browser
tools/gen-api-doc           # regenerate docs/api/ — 3 docs (CI fails if stale)
tools/check-api-coverage    # every public item is shown in an example
```

## When builds/tests fail

1. Plain compile error in code you just changed → debug normally.
2. Anything else (weird output, hang, tool crash, unreadable terminal) → run
   `tools/doctor` BEFORE attempting any fix. Obey its verdict:
   `ENV_OK` = it's your code · `ENV_FIXABLE` = run the named fix only ·
   `ENV_BROKEN` = stop and escalate.
3. Test results: `target/verify/report.json` is ground truth. Terminal output is
   advisory. If they disagree or the terminal is garbled, the tooling broke — not the tests.
4. Same command fails the same way twice after a fix attempt → STOP. No third variation.
   Run doctor, then copy `docs/templates/BLOCKED.md` to repo root, fill it in, and ask
   the human (or end the session if unattended).
5. Never agent-fixable — escalate immediately: missing system deps, toolchain install,
   network/registry outages, GPU/driver issues, permissions, disk full.

Writing a good BLOCKED.md for an environment issue is a successful outcome, not a
failure. Delete it in the commit that resolves the blockage.

## Routing — read before touching

| You are about to… | Read first |
|---|---|
| Start any implementation session | `docs/implementation-plan.md` (protocol + checklist) |
| Modify any subsystem | `docs/internal/<subsystem>.md` |
| Make or change a design decision | `docs/adr/` (search it — the decision may exist) |
| Add/change public API | `docs/conventions.md`, then the matching `examples/` file |
| Write a game with the engine | `docs/api/` (all three files) and `examples/` ONLY — never `src/` |
| Wonder why code looks wrong | The `DELIBERATE:` tag near it → linked ADR |

## Top conventions (full list: docs/conventions.md)

1. **One way to do everything.** No overloads, aliases, or convenience variants.
   Verbs: `create/destroy`, `load/unload`; `get_*` infallible, `find_*` → `Option`,
   `try_*` → `Result`.
2. **Units live in types.** `Radians`, `Seconds`, typed handles — never bare `f32`/`u32`
   in public APIs.
3. **No silent failure.** No no-op fallbacks. Debug: panic loudly. Release: `Result`.
   Error messages state what happened, likely cause, and fix.
4. **Determinism is sacred.** Seeded RNG, fixed timestep, replayable input. Never
   introduce wall-clock, thread-order, or iteration-order dependence into simulation.
5. **Files ≤ ~500 lines**, module doc header of fixed shape at the top of every file.

## Comment tags (grep for them; full spec: docs/agent-practices.md §1)

`INVARIANT:` `CONTRACT:` `SAFETY:` `PERF:` `DELIBERATE: (see ADR-00NN)` — no others.
Never "clean up" code carrying a `DELIBERATE:` tag without reading its ADR.

## Definition of done

1. fmt + clippy (`-D warnings`) + tests clean.
2. New behavior → behavioral test named as a sentence stating the behavior.
3. Public API touched → example added/updated + `tools/gen-api-doc` rerun.
4. `docs/internal/<subsystem>.md` updated, or commit says "no doc impact".
5. New deliberate oddity → ADR written + `DELIBERATE:` tag at the site.

## Never

- Never add a second way to do something that already has one.
- Never let a failure path do nothing.
- Never edit an accepted ADR — supersede it.
- Never put engine internals in `docs/api/` (bar ADR-0025's three testing-doc words) or make examples depend on `src/` internals.
- Never use `unwrap()`/`expect()` outside tests and examples.
- Never delete or `#[ignore]` a test to get a green run.
- Never downgrade deps, edit `rust-toolchain.toml`/CI config, or go `--offline` to
  route around a build failure — those are human decisions.
- Never let `wgpu` or `winit` types (or deps) escape their backend/platform crates —
  the ash migration and web support depend on this (ADR-0003, ADR-0004).
- Never use `std::time::Instant` or wall-clock time in engine/simulation code —
  frame clock only (ADR-0005).
- Never add a dependency without recording justification + `cargo tree` delta in
  the commit (agent-practices §5.8).
