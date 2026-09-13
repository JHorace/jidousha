# ADR-0044: macOS is a supported development platform, and not tier 1

Status: accepted · 2026-09-13 · **extends ADR-0005, which does not move — the
tier-1 set is unchanged**

> **macOS builds, runs, verifies and passes `tools/doctor`, and is not tier 1.**
> It gets a CI job on `main` and on demand rather than on every PR, it is absent
> from `deploy`'s gates, and a feature that cannot exist there is allowed to
> answer `n/a`. Promotion has a trigger rather than a date: a game shipping on
> macOS.

## Context

ADR-0005 set the tier-1 set — Linux, Windows, Web — and said of the rest: "Not
targeted: macOS/iOS for now (wgpu/winit keep the door open at near-zero cost)."
That sentence is the one this record answers. It was right about the door: this
work needed no new dependency, no backend selection code, and no change to the
frame loop's shape. It is the sentence's *other* half — "not targeted" — that
has stopped being true, and the reason is not technical.

**A second developer builds games on this engine and works on macOS** (owner
decision, 2026-09-13). That is the whole of the motivation, and it is worth
stating plainly because it is what sizes the answer. Nobody asked for macOS
parity; somebody asked not to be blocked.

**And there is a second thing this buys, which nobody asked for and which is
larger.** Replay determinism is the contract this engine is built on, and
ADR-0005 §4 states it as a cross-platform obligation. It had never been tested
off Linux and Windows on x86_64. A macOS run — on Apple Silicon, a different
architecture as well as a different operating system — is the first evidence for
a claim the repository has been making since its first commit. The engine's
recorded expectations are the instrument: they either reproduce byte-identically
there or they do not.

**Not deciding has a cost that the "not targeted" sentence hides.** Without a
tier, the next feature that cannot exist on macOS has no rule to follow, and the
two available defaults are both bad: block on it, or invent a number. The perf
panel's process counters are the live instance — `n/a` there is correct, and it
is only correct because something says a platform is allowed to answer `n/a`.

## Decision

**macOS is a best-effort development platform.** aarch64 and x86_64. It obliges:

- `cargo check` and `cargo build`, native and wasm, clean;
- `cargo clippy -- -D warnings` clean **on macOS**, because the
  `#[cfg(target_os = "macos")]` side of the tree is compiled nowhere else;
- a window opens and a game runs, on wgpu's Metal backend, with no
  special-casing in this engine;
- `tools/test` green, and every game's `tools/verify` green, against the same
  committed expectations every other platform is held to;
- `tools/doctor` reaching `ENV_OK` on a healthy Mac.

**It does not oblige** per-PR CI, every instrument reading, or the golden-image
reference comparison. The full table and the reasoning for each exemption are
platforms.md §1 and §3 — the obligations live in one place and this record is
what makes them a decision rather than a description.

**A feature that cannot exist on macOS answers `n/a`.** Never a zero, never a
guess, and never a blocked feature. This is the rule the next platform-touching
session needs and the reason this ADR is short rather than absent.

**Games stay platform-agnostic.** Every branch this decision permits is in the
engine or in `tools/`. A `cfg` in a crate under `games/` is out of scope for it.

**Promotion trigger: a game shipping on macOS.** Not a date, not a headcount.
When somebody distributes a build to players on it, the exemptions above stop
being free and this record gets superseded by one that prices them.

## Rationale

**Why a tier rather than just doing the work.** The work is small and mostly
already done — the door ADR-0005 described really was open. What is not small is
the *standing question*: for every future feature, does macOS count? A platform
with no tier gets that question re-litigated each time, and the cheapest answer
in the moment is usually "make it work everywhere", which is how a best-effort
platform silently becomes a tier-1 one without anybody deciding to pay for it.

**Why not tier 1.** Two costs, both real. A macOS runner bills at roughly ten
times a Linux one, and this project has already spent a session on CI wall time;
putting macOS on every PR would be the largest single addition to it. And tier 1
means a red macOS job blocks a merge and blocks the deploy — which for a
platform nobody ships on would trade a real risk (a stuck `main`) for a
hypothetical one.

**Why `n/a` rather than the two system calls.** The perf panel's process CPU and
RSS are reachable on macOS with no new dependency: `task_info` with
`MACH_TASK_BASIC_INFO`, a third hand-declared `extern` block beside the Windows
one this repository already carries. The dependency budget is therefore not what
stopped it. What stopped it is that it could not be *written honestly* from a
machine that cannot compile or run it, and an unsafe FFI block nobody has
executed is worse than a blank. The route is recorded at the site so the next
session with a Mac in front of it does not rediscover it. **If a future session
does want it, the budget question is already answered and the remaining question
is only whether the tier obliges it — it does not.**

**Why the label is not a CI trigger.** Adding `labeled` to the workflow's
`pull_request` types would re-run every job in the file, and cancel the
in-flight run, each time anybody touched any label. On a repository whose CI
file has caused a two-day outage, that is a bad trade for a handle that costs
one push instead.

## Consequences

- **`docs/internal/platforms.md` is new**, and is where the table, the
  obligations, the branch inventory and the findings live. `CLAUDE.md`'s routing
  table gains one row pointing at it.
- **`ci.yml` gains a `macos` job** and a `workflow_dispatch` trigger. The job is
  absent from `deploy`'s `needs`, which is the mechanical form of "not tier 1".
- **ADR-0005 does not move.** Its tier-1 set is unchanged and its web reasoning
  is untouched; what this record changes is the scope of one clause about what
  is *not* targeted. That is an extension rather than a supersession, and the
  index row says so — a reader who arrives at ADR-0005's "Not targeted:
  macOS/iOS for now" finds this record through `docs/adr/INDEX.md`, which is
  what the index is for (conventions, §Documents).
- **Platform findings have a home, and it is a temporary one.**
  platforms.md §5 holds them, because `e0-findings.md` is the E0 exercise's
  ledger and closed, and a game's `FINDINGS.md` is its own. If a second platform
  session ever needs one, a general engine ledger is the thing to build; one
  session's findings do not justify inventing one.
- **Two things are owed and cannot come from CI** (platforms.md §6): a person
  running the build on a real Mac and watching it, and frame-pacing readings on
  Metal. Neither is a blocker for this decision and both are conditions on
  calling the platform *checked*. A CI artifact is not the first of them.
- **A third platform now exists for every future feature to consider**, which is
  the durable effect. "Does this work on macOS, and if not, may it answer `n/a`?"
  has an answer before the feature is designed.

## Alternatives rejected

- **Tier 1.** Priced above: ten-times CI on every PR, and a merge gate for a
  platform nobody ships on. It is the answer *after* the promotion trigger, not
  instead of it.
- **Leave it undecided and just fix what breaks.** What the "not targeted"
  sentence already amounted to. It works until the first feature that cannot
  exist on macOS, and then the decision gets made by whoever is in a hurry.
- **Supersede ADR-0005.** Its tier-1 set and its web reasoning are entirely
  live; marking it superseded would tell a reader that a correct, load-bearing
  record had lost, which is worse rot than the one stale clause.
- **Implement the mach counters now.** No dependency cost, and it would have
  closed the one gap in the panel. Rejected on evidence rather than on budget:
  the code could not be run by the session writing it. Filed as a route rather
  than as a gap.
- **A separate `macos.yml` workflow.** Lower blast radius on the file that has
  hurt this project before, and genuinely tempting. Rejected because two
  workflow files is a second place to keep the cache strategy, the CI-only
  cargo settings and the runner conventions right, and they would drift — the
  same argument `tools/check-game-deps` makes for one script in two call sites.
