# Tooling — scripts, CI, and enforcement

The repo's enforcement layer: the `tools/` scripts an agent runs, and the CI jobs
that run them for every change. Written for a reader with zero session memory.

Owns: `tools/`, `.github/workflows/ci.yml`, `rust-toolchain.toml`, `clippy.toml`,
the workspace lint tables in the root `Cargo.toml`. Does NOT own: engine
subsystems (`docs/internal/<subsystem>.md`) or the decisions behind the rules
(`docs/agent-practices.md`, which is the rationale for everything here).

---

## 1. What it does

Four scripts, each answering one question. All are Python 3.8+, standard library
only — no third-party import, so they keep working when the package ecosystem is
exactly what broke.

| Script | Question | Exit codes |
|---|---|---|
| `tools/doctor` | Is my code wrong, or is the world wrong? | 0 `ENV_OK` · 1 `ENV_FIXABLE` · 2 `ENV_BROKEN` |
| `tools/test` | What actually passed? | 0 pass · 1 tests failed · 2 tooling/env fault |
| `tools/check-claude-md` | Is the always-in-context router still small? | 0 ok · 1 over cap · 2 missing |
| `tools/dep-count` | How big is the dependency graph? | always 0 (reports only) |

Not built yet (later milestones): `tools/verify` (headless deterministic run,
lands with M4/E0), `tools/gen-api-doc` (F0), `tools/check-tags`,
`tools/check-headers`.

## 2. Core data flow

```
agent ──> tools/test ──> phase: tool-selftest  (python -m unittest, tools/tests/)
                         phase: build          (cargo test --no-run)
                         phase: test           (cargo test --all-targets)
                         phase: doc-test       (cargo test --doc)
                              │
                              ├─> terminal (advisory)
                              └─> target/verify/report.json     (GROUND TRUTH)
                                  target/verify/failure-streak.json (circuit breaker)

agent ──> tools/doctor ──> ten checks ──> verdict line + target/verify/doctor.json
```

Phase order is load-bearing. The wrapper's own tests run first: a broken wrapper
cannot be trusted to report on the engine. The build is separated from the run so
a compile error and a hanging test are different, individually timed-out phases —
`TIMEOUT in phase <name>` instead of a dead terminal.

Counts from the Rust tests and from the tools' own unittest run land in the same
totals: one run, one set of numbers.

## 3. Invariants

- **The report file is ground truth; terminal scrollback is advisory.** If they
  disagree, the tooling broke, not the tests (agent-practices §6.2). `tools/test`
  writes `target/verify/report.json` on every exit path; an absent report means
  the wrapper itself died.
- **Doctor never hangs.** Every subprocess and network call is bounded by a
  timeout — doctor is what you run when something else hangs.
- **`fix` is non-empty exactly when a check is `FIXABLE`** (tested), and
  `ENV_BROKEN` outranks `ENV_FIXABLE`: a human-required problem is never hidden
  behind a command the agent could run instead.
- **The failure fingerprint ignores noise.** It is (status, failing phase, failed
  test names); when no test is named — compile error, harness crash — the tail of
  the failing phase with digits normalized. Timings must not defeat the counter,
  or the circuit breaker never trips. On the second identical failure the wrapper
  prints the §6.3 stop rule into the error channel itself.
- **CI installs no toolchain.** rustup honors `rust-toolchain.toml`, so CI and a
  developer machine run the same pinned channel, components, and targets. Editing
  that file to route around a failure is a human decision (CLAUDE.md "Never").

## 4. How to test it

`tools/tests/test_tools.py` holds behavioral tests for the parsing, verdict, and
counter logic — the parts that silently rot. `tools/test` runs them as its first
phase, so they run on every test invocation and in CI on Linux and Windows.

The scripts are extensionless executables; the tests load them by path
(`load_tool("doctor")`) rather than importing by name.

End-to-end paths worth re-checking by hand after changing the wrapper: a failing
Rust test (report `status: fail`, `failed_tests` naming it, exit 1), a compile
error (build phase fails, later phases `skipped`), and two identical failures in
a row (stop rule printed, `failure-streak.json` count 2).

## 5. Known sharp edges

- **`missing_docs` applies to integration tests too.** A new file under
  `crates/*/tests/` needs a `//!` header or the build fails with "missing
  documentation for the crate". This is the module-header rule (practices §1.2)
  reaching test files — intended, but surprising the first time.
- **`unwrap`/`expect` are clippy-denied workspace-wide.** `clippy.toml` exempts
  test code (`allow-unwrap-in-tests`); examples are separate compilation targets
  and are not exempt, so an example that unwraps puts
  `#![allow(clippy::unwrap_used)]` at the top of the file.
- **`cargo clean` deletes the reports.** `target/verify/` lives under `target/`,
  so a clean also resets the failure streak.
- **CI invokes `python tools/<name>`, not `tools/<name>`.** The shebang path is
  Unix-only; `setup-python` guarantees `python` on every runner OS.
- **Doctor's registry probe hits `https://index.crates.io/config.json`** — the
  sparse index cargo itself uses. A proxy that allows the crates.io web front end
  but not the index would still fail a build, and doctor would still catch it.
- **Tool output is not the first line of tool output.** The first cargo/rustc
  call on a machine that has not yet materialized the pinned toolchain is
  preceded by rustup's `info: syncing channel updates…` chatter on the same
  merged stream. Doctor's version check searches for the `rustc <version>` line
  rather than matching at position zero; any new parsing does the same.
- **Doctor treats a `cargo metadata` manifest error as `INFO`, not an
  environment fault**: a malformed `Cargo.toml` is the agent's own code, so the
  verdict stays `ENV_OK` ("go debug it") with the parse error quoted.
