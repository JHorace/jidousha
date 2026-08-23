# Tooling — scripts, CI, and enforcement

The repo's enforcement layer: the `tools/` scripts an agent runs, and the CI jobs
that run them for every change. Written for a reader with zero session memory.

Owns: `tools/`, `.github/workflows/ci.yml`, `rust-toolchain.toml`, `clippy.toml`,
the workspace lint tables in the root `Cargo.toml`. Does NOT own: engine
subsystems (`docs/internal/<subsystem>.md`) or the decisions behind the rules
(`docs/agent-practices.md`, which is the rationale for everything here).

---

## 1. What it does

Ten scripts, each answering one question (plus the web pipeline pair,
`tools/build-web` and `tools/serve-web` — §3 and web-publish.md). All are
Python 3.8+, standard library only — no third-party import, so they keep
working when the package ecosystem is exactly what broke.

| Script | Question | Exit codes |
|---|---|---|
| `tools/doctor` | Is my code wrong, or is the world wrong? | 0 `ENV_OK` · 1 `ENV_FIXABLE` · 2 `ENV_BROKEN` |
| `tools/test` | What actually passed? | 0 pass · 1 tests failed · 2 tooling/env fault |
| `tools/check-claude-md` | Is the always-in-context router still small? | 0 ok · 1 over cap · 2 missing |
| `tools/dep-count` | How big is the dependency graph? | always 0 (reports only) |
| `tools/check-compile-fail` | Do the errors that must be compile errors still say the right thing? | 0 ok · 1 drifted · 2 harness broke |
| `tools/verify` | What did the game actually do, with nobody watching? | 0 verified · 1 its own assertions failed · 2 tooling/env fault |
| `tools/check-assets` | Does every asset path in the code name a file that exists, under the root that file is allowed to use? | 0 all resolve · 1 a reference is broken · 2 the check could not run |
| `tools/check-game-deps` | Does every game reach the engine through the facade only? | 0 facade-only · 1 a game reaches past it, or a game is not a workspace member · 2 could not run |
| `tools/gen-api-doc` | Is `docs/api/` what the facade actually says? | 0 written/current · 1 stale, over budget, leaking vocabulary, or naming a test or example that is not there · 2 could not run |
| `tools/check-api-coverage` | Is every public item shown in an example — and can anything reach each `testing` export? | 0 covered · 1 a gap, an unreachable entry, or a breach · 2 could not run |
| `tools/check-api-prose` | Does the hand-written half of `docs/api/` contain code that compiles? | 0 every block compiles · 1 one does not · 2 could not build the facade |

Not built yet: `tools/check-tags`, `tools/check-headers`.

## 2. Core data flow

```
agent ──> tools/test ──> phase: tool-selftest  (python -m unittest, tools/tests/)
                         phase: build          (cargo test --no-run)
                         phase: test           (cargo test --all-targets)
                         phase: doc-test       (cargo test --doc)
                         phase: compile-fail   (tools/check-compile-fail)
                         phase: check-assets   (tools/check-assets)
                         phase: check-game-deps (tools/check-game-deps)
                         phase: check-api-coverage (tools/check-api-coverage)
                         phase: check-api-prose (tools/check-api-prose)
                         phase: api-doc        (tools/gen-api-doc --check)
                         phase: example:<name> (cargo run --example, one per example)
                         phase: game-verify:<name> (tools/verify, one per game)
                              │
                              ├─> terminal (advisory)
                              └─> target/verify/report.json     (GROUND TRUTH)
                                  target/verify/failure-streak.json (circuit breaker)

agent ──> tools/doctor ──> fifteen checks ──> verdict line + target/verify/doctor.json

agent ──> tools/verify <name> ──> cargo run [--example <name>] -- --verify
                              │      (a game is its crate's binary and takes no
                              │       --example selector — ADR-0038)
                              ├─> terminal: the verdict line and its summary
                              └─> target/verify/<name>.json  (verdict, summary,
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

Both of cargo's example layouts count: `examples/<name>.rs`, and
`examples/<name>/main.rs` for one big enough that the game and the check on the
game are two reads. `prototype_kit` is the first of the second kind.

A windowed example named in `VERIFIABLE_EXAMPLES` — `prototype_kit`, and each
E0 run's `pong` while it is in the tree — is the exception to the exception:
instead of `example-build:<name>` it gets `example-verify:<name>`, which runs
`tools/verify <name>`. "Needs a person to look at it" is a reason to
script the looking, not to skip it. Every name in that set must also be windowed
(tested) — a headless example already asserts in its normal mode, and giving it a
second mode would be a second way to do one thing.

## 3. Invariants

- **The report file is ground truth; terminal scrollback is advisory.** If they
  disagree, the tooling broke, not the tests (agent-practices §6.2). `tools/test`
  writes `target/verify/report.json` on every exit path; an absent report means
  the wrapper itself died.
- **One tool, one report file.** `tools/verify` writes
  `target/verify/<name>.json`, never `report.json`. Two tools writing one
  ground-truth file is how ground truth stops being true, and the second writer
  would be the one whose result you were not looking at.
- **A verify run that verified nothing is not a pass.** An example opts into
  verification by handling `--verify`; one that ignores the flag runs normally
  and exits 0 having asserted nothing. `tools/verify` therefore requires a line
  beginning `verified ` in the output and reports `unverified` (exit 2) without
  it — the single failure mode this script exists to avoid is reporting silence
  as success.

  **The convention is now on the public side too**, since E0 run 4
  (e0-findings.md F-046): `docs/api/`'s *Testing your game* says the mode is the
  game's, that the flag is spelled `--verify`, that `main` branches on it before
  calling `run`, and that the verdict line must begin `verified `. Until then the
  document's closing line named `tools/verify <example>` without saying that the
  loop it runs is a mode the example has to implement, so a reader of the document
  alone got the whole testing section and no way to wire it to a command line. The
  two statements move together — in particular the `verified ` prefix, which a
  game author cannot discover from anywhere else and whose absence is reported as
  a *tooling* fault rather than as their bug.

  **A verify mode collects its failures rather than exiting on the first**, since
  E0 run 5 (e0-findings.md F-061). `verify::run` returns an `ExitCode`; a `Checks`
  accumulator records every failed reading and prints them all in the four-part
  message shape at the end. The reason is the same one behind "report the numbers
  it judged": an instrument that halts at the first bad reading costs a cycle per
  fault, and run 5 measured it — one deliberate break produced six problems and
  the diagnostic one was fourth. `process::exit` survives only for a reading that
  makes the rest meaningless (a missing entity, a frame never recorded), which is
  a different thing from a failure. `prototype_kit` was the worked example
  teaching the opposite and now teaches this; the document's skeleton shows the
  `ExitCode` return.
- **A game's own directory is where its art lives, and two tools agree on
  that** (ADR-0040). `tools/check-assets` refuses any root but the repository's
  shared `assets/` and, for a file under `games/<name>/`, that crate's own
  `assets/`; `tools/build-web` stages exactly those two under a page, at the
  paths the code names them by. The pair is the whole reason a game's art works
  on the deployed page: one tool would only ever be checking or staging its own
  half.
- **A game is a game because of where it lives.** Every tool that enumerates
  something playable asks `cargo metadata`: a workspace member under `games/`
  with a binary is a game, an example target is an example (ADR-0038). There is
  no registration list for games and so none to fall out of date — the step that
  was missed after E0 runs 4, 5 and 7 (F-094) does not exist for them. `attic/`
  is outside the workspace and outside every glob here: retired prototypes are
  read, never built. One consequence worth knowing: a `games/*` members glob that
  matches nothing makes cargo fail on a literal path, which is why
  `games/README.md` exists while no prototype does.
- **The facade check runs in both places from one script.** `tools/check-game-deps`
  is a `tools/test` phase *and* its own CI job. A check enforced in one place and
  skippable in the other is a check whose result depends on where you stood; a
  self-test asserts both call sites still name it.
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
- **`docs/api/` is generated, never written.** `tools/gen-api-doc` builds it
  from the facade's own `pub use` lists plus the prose in `tools/api-doc/`, so an
  item that is not re-exported cannot appear in the documentation and one that is
  cannot be forgotten. CI runs `--check`, which fails when a committed file
  differs — stale documentation is worse than none, because an agent believes it.

  **Four documents**, split by what the reader is doing: `jidousha-api.md`
  (writing a game, 25k) since ADR-0025, `jidousha-testing.md` (checking one, 15k)
  since the same, `jidousha-capture.md` (rendering one frame of it, 4k) since
  ADR-0035, and `jidousha-controllers.md` (driving the check's player, 5k) since
  ADR-0030 — each with its own token budget and its own vocabulary rule.

  **The capture split is the first to move reference entries rather than only
  prose**, which is a shape the checks had never been asked for: an item can now
  be in two documents or in none, and both look fine in a diff. `CAPTURE_ITEMS`
  is the routed set and the rule for it is stated — an item goes there when *no
  entry outside the set names it*. `BackendTextureId`, `FramePlan` and
  `PhysicalSize` stay behind for exactly that reason: each is named by an entry
  that stays, so moving one would leave the testing document naming a type it
  does not define (F-017). The vocabulary exemption moved with the recipe, so the
  testing document no longer names a renderer at all — leaving the exemption
  behind is the half of a split that is easy to land without noticing, and a
  self-test refuses it.
  `Document` carries path, budget and vocabulary exception, so the budget,
  vocabulary and staleness checks are each written once and applied to a list;
  two copies of the staleness check is the drift F-016 was, and a third document
  costs those checks nothing because of it. The controllers document is prose
  only: its reader writes a controller with the other two documents' vocabulary,
  so a reference section here would be a second place to keep the same entries
  right.
  **Both halves of that document are now compiled, and only one of them used to
  be.** The reference comes from doc comments, whose examples are doctests —
  `tools/test`'s `doc-test` phase runs forty-nine of them. The prose in
  `tools/api-doc/` was hand-written and its code blocks were compiled by nothing:
  every gate checked that document's formatting, vocabulary, example pointers and
  token budget, and none checked whether its code was code. `tools/check-api-prose`
  closes that, and found three defects in the eighteen blocks on its first run —
  a `?` in a function returning `String`, two `expect` calls in a document that
  denies `expect_used` two sections earlier, and a `.map` on a `Vec` in a snippet
  added an hour before.

  Blocks are *fragments*, so they take context from rustdoc's `# ` hidden-line
  convention: compiled, never rendered, and therefore free of the token budget.
  `gen-api-doc::visible_prose` drops them on the way to `docs/api/` and
  `check-api-prose::unhide` reveals them on the way to `rustc`; the two are tested
  against each other, because a hidden line that reached the page would put a test
  fixture into the document and one that did not compile would make the check a
  formality. The game-shaped half of the context — `Score`, `my_system`, a
  `played()` fixture — lives in the tool rather than in eighteen copies, since it
  is the same fiction each time and the prose already calls it the reader's own.

  Nothing is *run*: a fragment's value is the sentence beside it, and what a
  fragment can be wrong about is whether it compiles.

  **`<!-- asserted-by: … -->` links a claim to the test that holds it true**, and
  `gen-api-doc` refuses a marker naming a test that does not exist. Three
  sentences in ten E0 runs have been *false* — a document claim contradicting the
  code it described (F-055, F-068, F-097) — each found by a run leaning on it and
  each fixed with a test written afterwards. Be exact about which half this is:
  it does **not** check that a claim is true, because nothing mechanical can. It
  checks that a claim which names its proof still has one, so the linkage rots
  loudly. Same bargain as `dangling_examples` one level up — there a pointer at a
  worked example, here a pointer at a proof.

  Scoped to claims a game's `--verify` leans on: draw order and its
  submission-order tie-break, `covering`'s boundary rule, quads per primitive,
  the text metrics, tick numbering, registration order, `contains_rect` versus
  `contains`. A falsehood there makes every game's check quietly wrong, which is
  what justifies the ceremony. Claims about game design (F-080) and about the
  document's own coverage (F-068) are assertable by nothing and are not asked to
  be — saying so is part of the mechanism rather than an apology for it. Markers
  are dropped on the way to `docs/api/` like hidden lines, so one costs no
  tokens and can go on every claim that deserves it.

- **`check-api-coverage` reads `jidousha::testing` too, and did not until run
  10's triage.** `facade_items` stops at the prelude, so the verification
  vocabulary — a third of the testing document's budget — was checked by nothing:
  ADR-0028 found six items exported for a road only `prototype_kit` walked and
  removed them, and nothing would have said so a second time. **Reachability has
  two forms and a naive check gets the second wrong.** An item may be *used* by
  an example, or *named in another entry's signature* — which is why it has an
  entry at all, F-017 being the finding that a type named in a signature and
  defined nowhere is a hole. `Batch` (F-036), `RawImage` and `DecodeError` are
  all in the second class and none is written by a game. An entry does not make
  itself reachable: the heading carries the item's own name, so it is removed
  with the entry or every entry looks reachable from itself and the check reports
  nothing, ever.

  What is left is an item nobody uses and nothing mentions, in a document with a
  hard token budget. It found one, `ReplaySource`, and the answer was to keep it
  — it is the asset half of the replay story that `TickRecord::readiness` is the
  other half of, and removing it would document a timeline with no way to replay
  it. The exemption carries that reasoning, which is the point of the gate: the
  question gets asked and the answer gets written down.

  Not rustdoc JSON, which needs nightly while `rust-toolchain.toml` pins stable;
  summaries are lifted from the `///` line above each definition, which is a
  bounded text problem with tests rather than a second toolchain.
- **A doctest keeps its indentation on the way into the document.** Every `///`
  line used to be `.strip()`ped at both ends, which is right for wrapped prose
  and wrong for the code block inside it: an `if` inside an `fn` came out flush
  against the margin, in three documents whose whole purpose is to be copied
  from. Only the one space rustdoc puts after the slashes comes off now. Nine E0
  runs read the flattened form and none reported it — which is the argument for
  the maintainer looking at the artifact rather than only at the diff, since the
  runs that copy it write `cargo fmt`-shaped code anyway and never notice they
  reformatted what they copied (e0-findings.md F-114's fix surfaced it).
- **The sources are scanned twice: declarations, then `impl` blocks.** A type's
  blocks are not obliged to live in the file that declares it, and a single pass
  in path order attached members only to types it had already seen. That deleted
  `World`'s whole resource API from the reference — `resource.rs` sorts before
  `world.rs` — and `InputSnapshot::encode`/`try_decode` with it, silently, for
  as long as the generator has existed (e0-findings.md F-016). Two passes make
  path order irrelevant; members are then ordered so the declaring file's block
  comes first, so a rename cannot reshuffle the page either. **The census line
  the generator prints on every run** (`N groups · N signatures · N fields · N
  variant lines`) is the guard a human has against the next silent shrink: a
  parser regression is obvious in those numbers and invisible in the diff of a
  1,700-line file.
- **Foreign re-exports are documented by an embedded example, not a copied
  list.** `Vec2` and `Vec3` come from glam and there is nothing here to generate
  an entry from, but "documented there" points at a crate whose docs may not be
  in the reader's container (F-018). `TOURS` maps the module to an example file
  that is embedded verbatim, exactly as the Quickstart is, so cargo compiles the
  list and `tools/test` runs it. A hand-written list would be the one thing in
  this document that could go stale without CI noticing.
- **The API document has a token budget, and it is enforced.** 25k
  (public-api.md §4), counted at roughly four characters a token because there is
  no tokenizer in the standard library and the budget is an order-of-magnitude
  guard. Growth past it is a curation conversation, not a bigger doc. It also
  refuses implementation vocabulary — internal crate names, the backend seam,
  "archetype" — in every section except the testing reference, which is allowed
  to name a backend because a golden image has to be drawn by something.
- **Script entry points get end-to-end tests of their failure paths.** Four
  milestones running, the surviving mutation was the same one: the judgement at
  the top of a script — "is this a pass?", "should this exit 1?" — is the part
  that unit tests over its helpers never touch. `tools/verify` (I2) reported
  silence as success, `verdict_status` (R4) was unreachable, `check-assets` (A3)
  could find problems and return 0, and `gen-api-doc` (F0) could skip its
  staleness check entirely. Write the test that runs `main` and asserts the
  non-zero exit, at the same time as the script.
- **A mistyped asset path fails before anything runs.** `tools/check-assets`
  extracts the string literals from `load_texture`/`load_bytes` call sites and
  checks each against the asset root, case-strictly, walking each path component
  against a directory listing rather than asking the filesystem — because on a
  case-insensitive filesystem asking answers yes for the wrong spelling, which
  is the exact bug being hunted (assets.md §2). It runs as a `tools/test` phase
  *and* as its own CI job: it needs no toolchain, so it answers in seconds on a
  runner that is still compiling. Two escape hatches, both marker comments and
  both requiring a reason: `check-assets: deliberately missing` for the examples
  that demonstrate failure, and `check-assets: computed path` for §2's
  sanctioned interpolated directory. It also checks the *root* before the paths:
  a file under `games/<name>/` may load only from that crate's own `assets/`,
  everything else only from the repository's shared one, because those are the
  two directories `tools/build-web` stages under a page (ADR-0040).
- **The golden tier needs a rasterizer, and CI installs one.** A runner has no
  GPU, so `mesa-vulkan-drivers` (lavapipe, Mesa's CPU rasterizer) is installed on
  the Linux test job. Without it the golden tests skip and say so and the job
  still passes — this turns a skipped tier into a running one rather than
  routing around a failure. `tools/doctor`'s `gpu` check reports which Vulkan
  drivers are present, so "the golden tier skipped" is a diagnosable fact rather
  than a silence; it is INFO in both directions, because a machine with no GPU
  runs every other test and a doctor that cried wolf here would be ignored when
  it mattered.
- **Rendered frames are CI artifacts.** `target/verify/*.png` and
  `target/verify/golden/*.png` are uploaded on every run, pass or fail: the
  captured frame from `tools/verify prototype_kit`, and the actual/diff pair a
  failing golden test leaves behind. "What did it actually draw?" is then
  answerable from a CI run rather than only on a machine with a GPU.
- **`tools/verify <example>` is the headless half of "did it work?".**
  `serve-web --check` asks whether a picture appeared in a browser;
  `tools/verify` asks what the game *did*, with no display anywhere — scripted
  input, a fixed number of ticks, and the example's own assertions on world
  state and on the draw transcript (input.md §5). The report keeps the whole
  transcript, so the evidence for a failure is still there after the scrollback
  is gone. Adding a verify mode to an example is two things: a `--verify` branch
  in `main`, and the example's name in `tools/test`'s `VERIFIABLE_EXAMPLES`.
  When a run captures a frame, the report carries its path in `artifact` — lifted
  out of the summary prose so an agent looking for the picture does not have to
  parse English to find it, and `null` on a machine that captured nothing.
- **`tools/build-web` + `tools/serve-web` are the web target's other half**
  (W0; design and contracts in web-publish.md). `cargo check --target
  wasm32-unknown-unknown` has gated every merge since M0 and proves the engine
  compiles for the web; `build-web <example>` turns that into something a
  browser loads — cargo (release by default), `wasm-bindgen`, `wasm-opt -Os`
  when installed, then the playtest page from `tools/web-template/` staged into
  `dist/<example>/` with the name and build stamp substituted. CONTRACT
  (web-publish.md §1): it is the ONLY web build path — CI, local dev, and game
  repos all call it. `serve-web [<example>]` serves `dist/` and nothing else,
  so what works locally is what works deployed. `--check` drives a headless
  Chromium at the page twice: once to assert the module started and the canvas
  was drawn on (screenshot → PNG decode), once with `?panic=1` to assert the
  panic overlay rendered the full §9 text (web-publish.md §2). Its working
  files go to `target/web-check/`, never into `dist/` — dist is what deploys.
  Stdlib only, including the PNG decoder — forty lines of `zlib` and
  un-filtering, for the same reason the input codec is hand-written (ADR-0014).
- **The deploy is two CI jobs and nothing local** (W1–W2, web-publish.md §4).
  `web` runs `tools/build-web --all` — the fleet is the facade crate's
  examples, minus the native-only ones build-web names and skips aloud — with
  the `wasm-bindgen` CLI installed at the version Cargo.lock pins (the session
  hook's recipe) and a **pinned binaryen release** so the deploy ships
  optimized modules (never Ubuntu's binaryen 108, which damages the externref
  table — web-publish.md §5; build-web refuses a wasm-opt below its pinned
  minimum, so a runner change cannot silently reintroduce it), then
  browser-checks the optimized bytes (`tools/serve-web sprites --check`)
  before uploading `dist/` as an artifact. `deploy` runs only after every other gate in the same
  run: `wrangler deploy` on a `main` push (production), `wrangler versions
  upload --preview-alias pr-<number>` on a PR (stable preview URL per PR), with
  ONE sticky comment per PR updated in place on each push — never a comment per
  push. Fork PRs have no secrets, so the job skips neutrally (ADR-0037). node
  and wrangler are CI-only dependencies; they appear in the workflow and
  nowhere else.
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
- **`build-web` stages the asset root next to the page.** A2's web loader
  fetches `assets/...` *relative to the page*, so the served directory has to
  contain them — which is exactly what deploying a web build involves. Copying
  them into `dist/<example>/` rather than teaching the server to reach back into
  the repository keeps the served tree honest about what a deployment needs.
- **A page can start and then fail, and the check now notices.** The page styles
  its status line `failed` for a real failure and leaves it alone for the
  engine's own §9 reports, which are handled problems — a missing asset draws a
  placeholder and the game carries on. `--check` reads that class, so a page
  that loaded and then threw is no longer a silent pass. The two are told apart
  by the `[jidousha] ` prefix, which is the first time the §9 format has been
  load-bearing for something other than reading.
- **The `wasm-bindgen` CLI must match the `wasm-bindgen` crate exactly.** They
  generate two halves of one interface, and a skew produces glue that fails at
  run time with a message about nothing in particular. `build-web` reads the
  version from Cargo.lock, compares it to the installed CLI, and prints the
  exact `cargo install` line when they differ — and `tools/doctor` runs the
  same comparison (mismatch `ENV_FIXABLE`; absence info only, since build-web
  gates it and a runner that never builds web is healthy — practices §6.1),
  plus a MIME self-check against
  serve-web's real handler and a wasm-opt presence line (info only), by loading
  the tools as modules rather than re-implementing them (web-publish.md §5).
  This is the single most likely
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
  `sprites` from R2, `prototype_kit` from R3, `input_echo` from I1,
  `quickstart` from F0, and each E0 run's `pong` — which comes back out with the
  game at the start of the next run, `e0-prompt.md` step 2, and goes back in when
  the maintainer adopts the new one at step 6. **That step has now been missed
  three times**, after runs 4, 5 and 7, each time leaving `tools/test`
  failing on a Pong it ran as an ordinary example and watched open a window.
  Run 6's triage recorded the miss as not having recurred; it had not, because a
  maintainer took the step, which is not the same as the trap being closed
  (e0-findings.md F-094). The
  symptom is unmistakable once seen — `RunError::NoDisplay` in an
  `example:pong` phase — and it is worth naming here because the two halves live
  in different commits by design, so nothing structural connects them); a
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
  window or keep skipping something deleted. **And the lists stopped being the
  mechanism**, which is the part worth knowing: `unregistered_verify_modes`
  reads each example's source for the `--verify` flag — both cargo layouts — and
  anything it finds outside the lists is treated as verifiable-and-windowed for
  that run, with a note naming what was not registered. So a game added to
  `examples/` is verified on the next push whether or not a maintainer took the
  adoption step, and the failure mode F-094 describes — the wrapper running a
  windowed game bare and dying on `NoDisplay` — cannot recur.

  Two consequences follow, and the second is why the note is a note.

  - **The E0 author is never told to edit this file.** Their prompt says the
    engine's tooling is not theirs, so the note addresses both readers by name
    and tells the game's author that nothing in it is theirs to fix. An earlier
    version of this check exited before any phase ran and printed a `fix:` line
    instructing them to edit these lists, which is worse than the bug it
    replaced: it contradicted the prompt *and* denied them the rest of the suite.
  - **Nothing fails on an unregistered example**, because nothing is broken by
    one. The lists are now what a reader consults rather than what the wrapper
    obeys, so leaving a name out is bookkeeping. The self-test that remains
    checks the other direction — that a name *in* `VERIFIABLE_EXAMPLES` really
    takes the flag — which is the half that can go stale silently.
- **A session-start hook installs the software rasterizer, and it is the CI line
  moved one directory.** `.claude/hooks/session-start.sh` runs on `SessionStart`
  in a remote session and installs `mesa-vulkan-drivers`, so `tools/verify`
  captures a PNG and the golden tier runs instead of skipping. Five E0 runs built
  a game on a machine that could not render one (e0-findings.md F-054); the fix was
  known and one apt line long from run 4 onward, and it stayed un-taken because
  installing system packages is a human decision under CLAUDE.md's escalation
  rule. It is a hook rather than a note in this file because the one other thing
  the E0 checklist asks a maintainer to remember was missed twice.

  **It installs `wasm-bindgen` too, since run 9's triage** (e0-findings.md
  F-124). `tools/build-web` needs the CLI at exactly the version
  `Cargo.lock` pins and refuses rather than guessing; everything else it wants —
  the wasm32 target, a Chromium under `/opt/pw-browsers` — was already in the
  image, so eight runs of "no session has driven its game in a browser" was one
  absent binary. It comes from the release page as a musl binary rather than from
  `cargo install`, which is 1.3 seconds against several minutes for the same
  program, and **the version is read out of `Cargo.lock`** rather than written
  into the script: two copies of a version drift on the first `cargo update`, and
  a mismatch here fails at run time with a message about nothing in particular.

  It is a **no-op outside a remote session** (`$CLAUDE_CODE_REMOTE`), idempotent
  (the container image is cached after it runs, so a warm start does nothing), and
  it **never fails the session**: an unreachable archive prints the four-part
  message and exits 0, degrading to the no-adapter state the tests already handle.

  **It supplies a `DISPLAY` too, since run 8.** That line used to end "what it
  cannot supply is a `DISPLAY` — a windowed example still needs a person", and
  four triages repeated it. It was wrong. `xvfb-run` was installed the whole time;
  what was missing was `libxkbcommon-x11.so`, which winit's X11 backend dlopens
  and the image did not carry, so a windowed example panicked inside `xkbcommon-dl`
  rather than failing for want of a display (e0-findings.md F-111). The hook now
  installs `libxkbcommon-x11-0`, `xvfb`, `xdotool` and `x11-apps`, which together
  make a windowed example openable, drivable by real key events, and readable back
  as pixels — the whole of `e0-prompt.md`'s after-the-run step 2 bar the browser.

  **One thing about Xvfb is worth carrying here rather than rediscovering.** It has
  no window manager, so nothing sets the input focus and every key event goes to
  the root window: the game looks deaf and is not. `xdotool windowfocus --sync
  <id>` once, after the window appears, is the fix. Measured: a paddle under a
  1.5-second `keydown s` moves zero pixels without it and 286 with it. A session
  that does not know this files an input bug that does not exist.
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
