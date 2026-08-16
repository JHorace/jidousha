# Tooling — scripts, CI, and enforcement

The repo's enforcement layer: the `tools/` scripts an agent runs, and the CI jobs
that run them for every change. Written for a reader with zero session memory.

Owns: `tools/`, `.github/workflows/ci.yml`, `rust-toolchain.toml`, `clippy.toml`,
the workspace lint tables in the root `Cargo.toml`. Does NOT own: engine
subsystems (`docs/internal/<subsystem>.md`) or the decisions behind the rules
(`docs/agent-practices.md`, which is the rationale for everything here).

---

## 1. What it does

Six scripts, each answering one question (plus `tools/serve-web`, which drives a
browser — §3). All are Python 3.8+, standard library only — no third-party
import, so they keep working when the package ecosystem is exactly what broke.

| Script | Question | Exit codes |
|---|---|---|
| `tools/doctor` | Is my code wrong, or is the world wrong? | 0 `ENV_OK` · 1 `ENV_FIXABLE` · 2 `ENV_BROKEN` |
| `tools/test` | What actually passed? | 0 pass · 1 tests failed · 2 tooling/env fault |
| `tools/check-claude-md` | Is the always-in-context router still small? | 0 ok · 1 over cap · 2 missing |
| `tools/dep-count` | How big is the dependency graph? | always 0 (reports only) |
| `tools/check-compile-fail` | Do the errors that must be compile errors still say the right thing? | 0 ok · 1 drifted · 2 harness broke |
| `tools/verify` | What did the game actually do, with nobody watching? | 0 verified · 1 the example's assertions failed · 2 tooling/env fault |

Not built yet (later milestones): `tools/gen-api-doc` (F0), `tools/check-tags`,
`tools/check-headers`.

## 2. Core data flow

```
agent ──> tools/test ──> phase: tool-selftest  (python -m unittest, tools/tests/)
                         phase: build          (cargo test --no-run)
                         phase: test           (cargo test --all-targets)
                         phase: doc-test       (cargo test --doc)
                         phase: compile-fail   (tools/check-compile-fail)
                         phase: example:<name> (cargo run --example, one per example)
                              │
                              ├─> terminal (advisory)
                              └─> target/verify/report.json     (GROUND TRUTH)
                                  target/verify/failure-streak.json (circuit breaker)

agent ──> tools/doctor ──> ten checks ──> verdict line + target/verify/doctor.json

agent ──> tools/verify <example> ──> cargo run --example <name> -- --verify
                              │
                              ├─> terminal: the verdict line and its summary
                              └─> target/verify/<example>.json  (verdict, summary,
                                  and the whole draw transcript)
```

Phase order is load-bearing. The wrapper's own tests run first: a broken wrapper
cannot be trusted to report on the engine. The build is separated from the run so
a compile error and a hanging test are different, individually timed-out phases —
`TIMEOUT in phase <name>` instead of a dead terminal.

Counts from the Rust tests and from the tools' own unittest run land in the same
totals: one run, one set of numbers.

Examples are discovered from `cargo metadata` and **run**, not merely compiled
(practices §5.1) — each asserts its own results, so a broken example fails here
rather than in a game agent's face. One phase per example keeps the report
specific about which one broke, and the discovered list is printed and recorded
so a vanished example cannot pass as silence.

A windowed example named in `VERIFIABLE_EXAMPLES` is the exception to the
exception: instead of `example-build:<name>` it gets `example-verify:<name>`,
which runs `tools/verify <name>`. "Needs a person to look at it" is a reason to
script the looking, not to skip it. Every name in that set must also be windowed
(tested) — a headless example already asserts in its normal mode, and giving it a
second mode would be a second way to do one thing.

## 3. Invariants

- **The report file is ground truth; terminal scrollback is advisory.** If they
  disagree, the tooling broke, not the tests (agent-practices §6.2). `tools/test`
  writes `target/verify/report.json` on every exit path; an absent report means
  the wrapper itself died.
- **One tool, one report file.** `tools/verify` writes
  `target/verify/<example>.json`, never `report.json`. Two tools writing one
  ground-truth file is how ground truth stops being true, and the second writer
  would be the one whose result you were not looking at.
- **A verify run that verified nothing is not a pass.** An example opts into
  verification by handling `--verify`; one that ignores the flag runs normally
  and exits 0 having asserted nothing. `tools/verify` therefore requires a line
  beginning `verified ` in the output and reports `unverified` (exit 2) without
  it — the single failure mode this script exists to avoid is reporting silence
  as success.
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
  `#![allow(clippy::unwrap_used)]` at the top of the file. The exemption also
  covers only `#[test]` functions themselves, **not helper functions beside
  them** in an integration test — a helper that unwraps fails clippy while the
  test calling it would not.
- **Compile-fail snippets are checked by substring, not by snapshot.** Each
  `crates/*/tests/compile-fail/<name>.rs` has a `<name>.expected` listing
  sentences the compiler's output must contain. rustc's framing (line numbers,
  carets, note ordering) changes between releases; the engine's own sentences
  are what is being guarded. DELIBERATE: hand-rolled instead of `trybuild`,
  which measured 27 transitive dev-dependencies against a budget that prefers
  none (practices §5.8) — revisit if managing the snippets ever gets painful.
- **Not every wrong shape reaches our own error text.** Registering a
  Draw-shaped function in Update is caught by rustc's own signature mismatch
  (E0631) before `IntoSystem`'s `on_unimplemented` message fires, so those
  snippets lock rustc's wording plus the `IntoSystem<Phase>` mention. The
  `&mut T`-in-a-Draw-query case — the mistake ADR-0008 actually predicts — does
  show the engine's sentence.
- **`tools/verify <example>` is the headless half of "did it work?".**
  `serve-web --check` asks whether a picture appeared in a browser;
  `tools/verify` asks what the game *did*, with no display anywhere — scripted
  input, a fixed number of ticks, and the example's own assertions on world
  state and on the draw transcript (input.md §5). The report keeps the whole
  transcript, so the evidence for a failure is still there after the scrollback
  is gone. Adding a verify mode to an example is two things: a `--verify` branch
  in `main`, and the example's name in `tools/test`'s `VERIFIABLE_EXAMPLES`.
- **`tools/serve-web <example>` is the web target's other half.** `cargo check
  --target wasm32-unknown-unknown` has gated every merge since M0 and proves the
  engine compiles for the web; this builds an example, runs `wasm-bindgen`,
  writes `tools/web/index.html` with the example's name substituted in, and
  serves it. `--check` drives a headless Chromium at the page, screenshots it,
  decodes the PNG, and asserts the canvas was drawn on.
  Stdlib only, including the PNG decoder — forty lines of `zlib` and
  un-filtering, for the same reason the input codec is hand-written (ADR-0014).
- **"Was the canvas drawn on" takes two questions, not one.** The original check
  asked only whether the canvas differed from the page's own background, and I1
  found its blind spot: `input_echo` clears to rgb(15, 18, 26) against a page of
  rgb(16, 16, 20), so a correct, fully-drawn readout registered as 1% different
  and failed. It now also accepts a canvas that is *not one flat colour* — if
  anything was drawn over the clear, there is more than one colour up there.
  Either piece of evidence passes; a blank canvas has neither, because it is the
  page background, uniformly. Both directions are tested, including the one that
  matters most: a page that merely cleared to something near the background and
  drew nothing must still fail.
- **`serve-web` stages the asset root next to the page.** A2's web loader
  fetches `assets/...` *relative to the page*, so the served directory has to
  contain them — which is exactly what deploying a web build involves. Copying
  them into `target/web/` rather than teaching the server to reach back into the
  repository keeps the served tree honest about what a deployment needs.
- **A page can start and then fail, and the check now notices.** The page styles
  its status line `failed` for a real failure and leaves it alone for the
  engine's own §9 reports, which are handled problems — a missing asset draws a
  placeholder and the game carries on. `--check` reads that class, so a page
  that loaded and then threw is no longer a silent pass. The two are told apart
  by the `[jidousha] ` prefix, which is the first time the §9 format has been
  load-bearing for something other than reading.
- **The `wasm-bindgen` CLI must match the `wasm-bindgen` crate exactly.** They
  generate two halves of one interface, and a skew produces glue that fails at
  run time with a message about nothing in particular. `serve-web` reads the
  version from Cargo.lock, compares it to the installed CLI, and prints the
  exact `cargo install` line when they differ. This is the single most likely
  thing to go wrong for someone running the web target for the first time.
- **Two browser-flag traps, both found the hard way.** `--use-gl=swiftshader`
  is wrong for current Chromium — it reports "Requested GL implementation not
  found" and the GPU process exits during initialization, leaving a page that
  looks like it merely failed to draw. The right flag is
  `--use-angle=swiftshader` with `--enable-unsafe-swiftshader`. And
  `--virtual-time-budget` is *page* time: the engine draws every frame, so a
  large budget never finishes under a software rasterizer. Four seconds is
  enough to load, negotiate a GPU, and draw.
- **Serving wasm needs the right MIME type, set in the right place.**
  `SimpleHTTPRequestHandler` has already sent a `Content-Type` by `end_headers`,
  so adding a second one there is ignored; override `guess_type` instead. The
  symptom of getting this wrong is a browser warning about falling back from
  `instantiateStreaming` — mild, and a lie about what is wrong.
- **Almost every example is run; windowed ones are built and not run.**
  Resolved in M5, which is when it first mattered. `tools/test` carries a
  `WINDOWED_EXAMPLES` set (`window_blank` from M5, `window_clear` from R1,
  `sprites` from R2, `prototype_kit` from R3, `input_echo` from I1); a
  name in it gets `cargo build --example` under a
  phase called `example-build:<name>` instead of `cargo run`, and the runner
  prints which examples it built rather than ran. Three reasons for a list over
  the alternatives: a headless flag would put test-runner concerns into engine
  code, a separate phase would still need to know which examples belong in it,
  and a name in a set is greppable. Building still catches every compile error,
  and CI's wasm job builds it too — what is lost is only the assertion that it
  *runs*, which for a window means "a person looked at it". Two self-tests guard
  the list: that a windowed example is built and not run, and that every name in
  it is an example that exists, so a rename cannot silently start running a
  window or keep skipping something deleted.
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
