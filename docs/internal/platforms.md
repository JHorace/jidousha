# Platforms — what each target promises, and what it does not

What this engine runs on, at what tier, and what a tier obliges. Written for a
reader who is about to stand somewhere the last reader did not: a developer on a
platform this document names, or an author of a feature that has to decide what
to do where the feature cannot exist.

Owns: the support table below, the per-platform branches in the engine and in
`tools/`, and the promotion rule. Does NOT own: why the web is tier 1 (ADR-0005),
how a native frame is paced (frame-pacing.md §6), what the perf panel reads
(frame-pacing.md §7), or the scripts themselves (tooling.md).

Inherits: the tier-1 set and the web constraints that shape the core (ADR-0005),
the backend boundary (ADR-0003), winit (ADR-0004), and macOS's tier (ADR-0044).

---

## 1. The table

| Platform | Tier | Builds | Runs a game | `tools/test` · `tools/verify` | `tools/doctor` | CI |
|---|---|---|---|---|---|---|
| **Linux** (x86_64) | **1** | yes | yes | yes | `ENV_OK` | every push, every PR |
| **Windows** (x86_64) | **1** | yes | yes | yes | — (not a CI job) | every push, every PR |
| **Web** (wasm32) | **1** | yes | yes, in a browser | `serve-web --check` instead | — | every push, every PR |
| **macOS** (aarch64, x86_64) | **best-effort development platform** | yes | yes | yes | `ENV_OK` | `main` + on demand |
| Android | not built | — | — | — | — | — |
| iOS | not built | — | — | — | — | — |

**Tier 1 is what ADR-0005 decided and this document does not change**: CI-gated
on every merge, and a failure on one of them blocks. **macOS is the third
platform and a different kind of thing** (ADR-0044), and §3 is what its tier
means in obligations rather than in adjectives.

The two rows at the bottom are there so the table is the whole answer. "Not
built" is a real state and worth a row: winit and wgpu both reach further than
this list, and somebody will ask.

## 2. Determinism across platform *and* architecture

**Replay determinism is the contract** — same seed, same inputs, same simulation
— and ADR-0005 §4 states it as a cross-platform obligation. Until macOS, it had
never been *tested* anywhere but Linux and Windows on x86_64, so what the
repository could honestly claim was narrower than what it asserted.

**The recorded expectations are the check, and nothing weaker stands in for
them.** Every game's `--verify` asserts on its own draw transcript, the golden
transcripts are checked-in text, and `tools/test` runs all of it. A run of
`tools/test` on a second architecture either produces byte-identical transcripts
or it does not, and either answer is information. The rule while getting there,
stated because it is the failure mode: **a transcript that differs is a finding,
never a reason to loosen an assertion, a floor or a recorded expectation.** The
usual suspects would be libm, FMA contraction and SIMD codegen — the class
ADR-0009's deterministic trig already exists because of.

**What the first macOS run said, and it is worth reading carefully.** The
job's first real run reported **1041 passed, 10 failed** — and every one of the
ten was a GPU test that died in `wgpu::Instance::new`, not one recorded
expectation (§5, P-006). Nothing in the simulation, the ECS, the math, the
transcripts, the plan model or the record/replay suites moved. So the run is
evidence in both directions at once: it found a real port defect, and it found
**no determinism defect** in the part of the engine determinism is a property
of. The claim the repository may make once the job is green is stated in §6
with what is still owed against it.

**And the two halves of a port are different evidence.** The compile half was
proved by cross-compiling in a Linux container; the run half was not, and could
not have been. P-006 is the case that settles it: `cargo clippy --target
aarch64-apple-darwin` was clean on a tree that panicked at startup on a Mac.

## 3. What "best-effort development platform" obliges

macOS exists in this repository because a second developer builds games on it.
That is the whole of the reason, and the tier is sized to it.

**It obliges:**

- **It builds.** `cargo check` and `cargo build`, native and wasm, clean.
- **It lints.** `cargo clippy -- -D warnings` clean — on macOS, because the
  `#[cfg(target_os = "macos")]` side of the tree is compiled nowhere else.
- **It runs.** A window opens and a game runs, on the Metal backend, selected by
  wgpu with no special-casing anywhere in this engine.
- **It verifies.** `tools/test` green, and every game's `tools/verify` green,
  against the same committed expectations every other platform is held to.
- **`tools/doctor` reaches `ENV_OK`** on a healthy Mac. A doctor that cannot
  pass on a supported platform is a doctor nobody will run (agent-practices
  §6.1), which is why the two Linux-shaped checks got macOS branches (§4)
  rather than being left to a fallthrough that says only "not probed".

**It does not oblige:**

- **Per-PR CI.** A macOS runner bills at roughly ten times a Linux one. The job
  runs on pushes to `main` and on demand — `workflow_dispatch`, or the `macos`
  label on a pull request followed by a push. tooling.md §3a has the job's
  shape and its measured cost.
- **Every instrument reading.** The perf panel's process CPU and RSS read
  `n/a` here (frame-pacing.md §7), and that is a landing rather than a gap. The
  panel already prints `gpu n/a` honestly where a device offers no timestamp
  queries, and the never-lies discipline prefers a blank to a guess. **What is
  not exempt**: the frame breakdown, the busy share and the whole
  engine-tracked accounting tier are all-platform and work here.
- **Golden-image comparison.** The reference comparison is Linux-only and
  deliberately so (renderer.md §9): a reference is a picture *some* rasterizer
  produced, CI blesses on lavapipe, and a Metal device fills edge pixels
  differently enough that a tolerance wide enough to accept both would be wide
  enough to accept a regression. Everything else in that file — the offscreen
  target, capture, row unpadding, the clear colour, render-twice stability —
  runs here like everywhere.
- **A platform-conditional in a game.** Games are platform-agnostic. Every
  branch this document describes is in the engine or in `tools/`, and a game
  under `games/` has none and must not acquire one.

**Promotion has a trigger, not a date: a game shipping on macOS.** Until then a
feature that cannot exist here is allowed to answer `n/a`, and a future session
designing one should read that as permission rather than as a bug to fix.

## 4. Where the branches are

**There are two conditional axes and they are not the same size.** Keeping them
apart is what makes this section short enough to be complete.

**The web/native axis** — `cfg(target_arch = "wasm32")` — is large: 72 sites
across 14 files, most of them in `jidousha-platform`. It is ADR-0005's subject,
not this document's, and macOS adds nothing to it. The lifecycle split in
`lib.rs::run_app` is its head (the browser owns the loop and never gives it
back) and the rest follows from that and from the web having no filesystem, no
process counters and no wall clock.

**The per-operating-system axis is two files.** That is the whole of it in the
engine, verified rather than remembered:

```
$ grep -rn 'target_os\|cfg(windows)\|cfg(unix)' crates/ --include='*.rs'
crates/jidousha-platform/src/driver/overlay/process.rs
crates/jidousha-render-wgpu/tests/golden.rs
```

**And the frame loop is in neither.** `driver/mod.rs::about_to_wait` reads
`RenderBackend::presentation` and decides; `driver/pacing.rs` and
`driver/frame.rs` carry no `cfg` at all. macOS changes nothing about their
shape, and a per-platform fork there is the failure mode the native-pacing work
already named (frame-pacing.md §6.3).

So the complete per-OS inventory, engine and tooling together:

| Where | Branch | Why |
|---|---|---|
| `…/driver/overlay/process.rs` | `target_os = "linux"` · `windows` · else | Two system calls per platform, hand-rolled. macOS takes the `else` arm and reads nothing. |
| `…/driver/overlay/panel.rs` — `cpu_line`, `memory_line` | `process::IMPLEMENTED`, a `const` from the file above | So the two `n/a`s are told apart: "no reading yet" and "no reading here" send a reader in opposite directions. Not a second `cfg` — one condition, stated once. |
| `jidousha-render-wgpu/tests/golden.rs` | `target_os = "linux"` | The reference comparison, and the three helpers and six imports that serve only it (§5). |
| `jidousha-platform/src/lib.rs` — `run` | **no branch**; a `CONTRACT:` comment and a `# Panics` | The event loop is created on the caller's thread, and that thread must be the main one. Listed because a reader looking for the macOS branch should find out there isn't one, and why (§5, P-002). |
| `tools/doctor` — `check_graphics` | `darwin` | A windowed run needs the logged-in session and the main thread; a headless one needs no display at all. |
| `tools/doctor` — `check_gpu` | `darwin` | Metal ships with the OS, so there is no ICD to be missing and nothing to install — and the golden *comparison* is still Linux-only. |
| `tools/serve-web` — `browser_candidates` | macOS bundle paths | `shutil.which` finds nothing on a Mac: browsers install as bundles and put no executable on PATH. |
| `jidousha-render-wgpu/Cargo.toml` — wgpu's feature list | one feature per backend, `metal` among them | **The per-OS condition that is not a `cfg`**, and the one that bit (§5, P-006). A backend absent from this list does not degrade on the platform that needs it; it panics in `Instance::new`. |

**What replaces what, for a headless run.** Linux needs `xvfb` plus `lavapipe`
because a Linux machine with no display server cannot make a surface and a
Linux runner has no GPU. **macOS needs neither and gets no substitute**: Metal
renders offscreen with no window server involved, so `tools/verify` and the
frame capture work on a Mac with nothing installed and nothing wrapped around
them. There is no macOS `xvfb` in this repository because there is nothing for
one to do.

**`tools/test`, `tools/verify`, `tools/check-assets` and `tools/check-game-deps`
needed no branch at all.** All four are standard-library Python and ask
`cargo metadata` what exists rather than assuming a path. One is worth naming as
a strength rather than a survival: `check-assets` walks each path component
against a directory listing instead of asking the filesystem, *because* asking a
case-insensitive filesystem answers yes for the wrong spelling — which is
exactly the bug a developer on macOS's default APFS volume would otherwise ship
to a Linux runner (assets.md §2). That check was built for this platform before
this platform existed.

## 5. What the documents and the code assumed

Findings from the first reading of this repository by a session standing on a
third platform. The ledgers (agent-practices §2.5) have never had this data:
`docs/internal/e0-findings.md` is the E0 exercise's and closed, and a game's
`FINDINGS.md` is its own, so these live here until a general engine ledger
exists — named in ADR-0044 as the thing to build if a second platform session
ever needs one.

**P-001 — three dead-code warnings had been latent on both non-Linux platforms,
and CI could not see them.** `golden.rs` gates its reference-comparison *test*
on `target_os = "linux"` and did not gate the three helpers (`golden_dir`,
`artifact_dir`, `check_against_reference`) or the six imports that serve only
it. On any other target those compile unused. CI runs clippy with `-D warnings`
on `ubuntu-latest` only, so the workspace has never been linted for a target
where the test is absent — and the Windows job runs `tools/test`, which does not
lint. The first `cargo clippy --target aarch64-apple-darwin` of this workspace
found all three. *What was done on the document's authority before finding out:*
renderer.md §9's "Everything else in the file … runs everywhere, including on
Windows" was read as evidence the file was platform-clean, which is a claim
about the tests rather than about the warnings. Fixed here; the durable fix is
the macOS clippy step, which is the only place this class can be caught.

**P-002 — the main-thread rule was satisfied by accident and stated nowhere.**
macOS requires the event loop on the process's main thread, and winit enforces
it on every native platform with a panic. `jidousha-platform::run` has always
been correct — `EventLoop::new()` is called inline on the caller's thread and
nothing in the crate spawns a thread — but nothing said so, nothing tested it,
and the doc comment's `# Errors` section listed three failures that are all
facts about the machine while the fourth, which is a fact about the *call*, was
absent. A future refactor that moved the loop behind a worker would have been
green on two platforms. Now a `CONTRACT:` at the site and a `# Panics` on the
public function.

**P-003 — `tools/serve-web` could not find a browser on macOS, and would have
reported it as the machine's fault.** `chromium()` looked on `PATH` and in the
container's Playwright directory. macOS browsers install as application bundles
and put nothing on `PATH`, so the check reported "no browser" on a machine with
two. The failure would have read as an environment problem rather than as a
tool that had not been told where to look — the class doctor exists to prevent,
in a tool doctor does not cover.

**P-004 — the perf panel's native `n/a` was a disjunction a reader could not
resolve.** `process n/a - no reading yet, or none this platform offers` is true
on Linux, where the first disjunct is the live one for the first two seconds of
every run, and true on macOS, where the second is the only one there will ever
be. A macOS developer reading it cannot tell whether to wait. Not a false
sentence — which is why it survived — but a sentence that costs its reader the
thing it was written to give them. Now two sentences, chosen by a `const` in the
module that knows.

**P-006 — a wgpu backend is a Cargo feature, and macOS had none — so nothing
fell back and nothing reported an absent adapter; `wgpu::Instance::new`
panicked.** `jidousha-render-wgpu`'s manifest listed wgpu's backends
explicitly and said, in as many words, that `metal` was "dropped because macOS
is not a target". Correct when written. What it meant in practice was not the
graceful degradation every other absence in this repository produces: a macOS
build panicked at instance construction with *"No wgpu backend feature that is
implemented for the target platform was enabled"*, taking every GPU test and
every game's capture with it — ten test failures and `game-verify:ninjo`, on
the macOS job's first real run.

Three things about it are worth more than the fix:

- **Cross-compilation cannot find this, and it was the thing that found
  everything else.** `cargo check` and `cargo clippy --target
  aarch64-apple-darwin` were clean before and after: the code compiles and
  lints perfectly well with no backend compiled in for the target it is being
  compiled *for*. Only running it on the platform says otherwise. The compile
  half and the run half of a port are genuinely different evidence, and this is
  the case that proves it.
- **"No adapter is not a failure" did not apply**, because the failure was
  upstream of the adapter question. renderer.md §9's skip path handles
  `request_adapter` returning `None`; there was no instance to ask.
- **It was not caught by review or by a document**, and a document is what
  caused it: the manifest comment was the record of a decision (ADR-0005's
  target list), so it read as a deliberate exclusion rather than as something
  to re-examine when the target list changed. The durable defence is that
  ADR-0044 now exists for a future target-list change to find.

The fix is one word in a feature list, and its measured price (agent-practices
§5.8) is the reason it needs no budget conversation: **+0 crates on Linux,
Windows and the web**, because wgpu-hal's Apple dependencies are
`cfg(target_vendor = "apple")`; **+9 on macOS** — `wgpu-core-deps-apple`,
`raw-window-metal`, `objc2-metal`, `objc2-quartz-core`, `objc2-core-graphics`,
`objc2-core-foundation`, `objc2-foundation`, `objc2` and `block2`, all of them
already reachable through winit's own Apple tree. `tools/dep-count` reads the
lockfile rather than a build, so it reports 258 → 262.

**P-005 — the `tools/` scripts' "standard library only" rule paid off on a
platform nobody wrote it for**, and it is worth recording as a confirmation
rather than a defect (§2.5 asks for both). Four of the six scripts needed no
change. The rule was justified in tooling.md as keeping the tools working "when
the package ecosystem is exactly what is broken"; its larger dividend turns out
to be that a script with no dependencies has no platform.

## 6. What is owed

Two things this platform cannot be signed off without, and neither can be
produced from CI:

1. **A person runs the build on a real Mac and looks at it.** CI cannot see a
   window. This project does not count a platform as working until somebody has
   watched it run, and a screenshot in an artifact bundle is not that check —
   it is a picture a machine took of a frame a machine rendered. The macOS job
   uploads `frames-macos` for diagnosis, and the artifact is explicitly *not*
   the evidence for this line.
2. **Frame-pacing numbers on Metal.** frame-pacing.md §6 grades a platform on
   measured readings — which present mode the surface was actually configured
   with, which branch `presentation()` took, whether `FALLBACK_CAP_HZ` ever
   engages, and what the CPU cost is with and without. None of that is
   measurable without a display, so the macOS row of §6.5 is owed from real
   hardware. What is *known* rather than measured is in frame-pacing.md §6.7,
   marked as what it is.

And one that is owed elsewhere: when the game-repo template exists, its README
gains a line pointing here, so a game author learns what macOS promises them
before they find out.
