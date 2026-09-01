# BLOCKED — human intervention needed

CI's `web build` job has failed on **every run since `5f5f0dd`** — on `main` and on
PR #80 — at one step: `tools/serve-web <page> --check`. Nothing else is red. The
investigation below exonerates the code on five independent axes and localizes the
fault to the headless browser on the runner, which cannot be reproduced from this
container. It needs a decision that is not an agent's to make.

**Consequence worth knowing first:** `deploy` has `needs: [… web]`, so a red web
build blocks it. **Production has not deployed since `5f5f0dd`** — that run's
`deploy web` job is `skipped`, and every run since has been too.

## What I need from you

**Decide how CI should handle a browser-side hang in `tools/serve-web --check`.**
Three candidates, in the order I would take them. All are CI/tooling changes, which
CLAUDE.md puts on the human side of the line ("Never … edit CI config … to route
around a build failure"):

1. **Preserve the evidence.** The `web build` job's artifact upload has no
   `if: always()` and uploads only `dist/`. On exactly the runs that fail, the step
   is skipped and `target/web-check/browser-*.log` (the browser's own stderr) and
   `check-*.png` die with the runner. That is why three failures have produced no
   diagnosis. Adding `if: always()` and `target/web-check/` to that step turns the
   next failure into a diagnosable one. **This is the highest-value change and the
   smallest.**
2. **Close the fleet coverage gap** (see "Why the PR passed" below), or accept it
   knowingly.
3. Only then, if the hang is genuinely a slow-page problem rather than a browser
   one, revisit `CHECK_VIRTUAL_MS` / the 120s wall in `tools/serve-web`.

I have not touched any of the three.

## What I was doing

Landing the performance panel (`JIDOUSHA_FRAMETIME=2`) on
`claude/perf-panel-overlay-6tv4l7`, PR #80. The work itself is complete and green;
this blockage is CI's, not the branch's.

The failing command, from `.github/workflows/ci.yml`, job `web build`, step
"browser-check the optimized build":

```
page="$(head -n 1 dist/fleet.txt)"
JIDOUSHA_CHROMIUM="$(command -v google-chrome)" python tools/serve-web "${page}" --check
```

## Why the PR that created `5f5f0dd` passed — the fleet coverage gap

This is the question that started the investigation, and it has a clean answer.

**The merge introduced no code.** `git diff 15fb5fc 5f5f0dd` is empty; the merge
commit's tree is byte-identical to PR #79's head.

**What changed is which page CI browser-checks.** The workflow builds a different
fleet per event and checks exactly one page from it (`head -n 1 dist/fleet.txt`):

| event | fleet | page checked |
|---|---|---|
| pull request | `--all` | **`prototype_kit`** (`HEADLINE_EXAMPLE`, tools/build-web) |
| push to `main` | `--release-fleet` | **`pong`** (`RELEASE_EXAMPLES = ("pong",)`) |

So **no PR ever browser-checks the page `main` will check.** A page-level failure on
`pong` cannot be caught before merge. `pong`'s check was green on `main` at
`aba6afa` and red at `5f5f0dd`, and PR #79's own CI structurally could not have seen
it.

That explains the *shape* of what you observed. It does not, on the evidence below,
mean PR #79 broke anything.

## Doctor verdict

`ENV_OK`, on this container, taken after the investigation:

<details><summary>tools/doctor</summary>

```
  [ok     ] python: Python 3.11.15
  [ok     ] rust-toolchain: rustc 1.94.1 matches the pin
  [ok     ] rust-components: rustfmt and clippy present
  [ok     ] wasm-target: wasm32-unknown-unknown installed
  [ok     ] wasm-bindgen: CLI 0.2.127 matches the crate
  [info   ] wasm-opt: wasm-opt version 124 (version_124)
  [ok     ] serve-web-mime: .wasm served as application/wasm
  [ok     ] cargo: cargo runs and every manifest parses
  [ok     ] crates-io: crates.io sparse index reachable
  [ok     ] disk: 19.5 GiB free
  [ok     ] git: branch claude/perf-panel-overlay-6tv4l7, 0 uncommitted path(s)
  [ok     ] build-dir: target/ writable
  [info   ] graphics: no DISPLAY/WAYLAND_DISPLAY — headless
  [info   ] gpu: vulkan drivers: … lvp_icd.json …
  [ok     ] assets: assets/ readable, 3 entr(ies)

ENV_OK
```

Note `wasm-opt` reads `version_124` because **I installed binaryen during this
investigation**, by the same recipe `ci.yml` uses, so local builds would produce
CI's exact bytes. It is not part of the container image; a fresh session will see
`wasm-opt: not installed` again.

</details>

## Error

Identical on every failing run — `serve-web`'s own timeout, not an assertion:

```
[jidousha] the headless browser did not finish
  it ran for 120s without dumping the page
  likely cause: a software rasterizer spending virtual time on every frame, or a hung module
  fix: lower CHECK_VIRTUAL_MS, or run without --check and look at the page yourself
```

<details><summary>Which pass died, per run — the passes print as they complete, so this is readable off the log</summary>

`serve-web --check` runs three browser passes: **1** the page itself, **2**
`?panic=1`, **3** `?frametime=1`.

| run | ref | page | pass 1 | pass 2 | pass 3 |
|---|---|---|---|---|---|
| [33333033431](https://github.com/JHorace/jidousha/actions/runs/33333033431) PR #79 | `15fb5fc` | `prototype_kit` | ok | ok | ok — whole step 30s |
| [33333997211](https://github.com/JHorace/jidousha/actions/runs/33333997211) main | `5f5f0dd` | `pong` | **11.7s** | **1.9s** | **HANG >120s** |
| [33337817620](https://github.com/JHorace/jidousha/actions/runs/33337817620) PR #80 | `6dd0e5f` | `prototype_kit` | **HANG >120s** | — | — |
| same, re-run | `6dd0e5f` | `prototype_kit` | **HANG >120s** | — | — |

</details>

## Ruled out

Each with the source that killed it. **Do not re-run these.**

1. **The merge brought something in.** `git diff 15fb5fc 5f5f0dd` → empty.

2. **PR #79 (the frame-pacing overlay) made the check slower.** Measured directly:
   `pong` browser-checked three times at `aba6afa` (before #79) and three times at
   `5f5f0dd` (after), same machine, same toolchain, CI-identical optimized bytes.
   All six passed and the timings are indistinguishable:

   | pass | `aba6afa` | `5f5f0dd` |
   |---|---|---|
   | 1 page load | 11.2 / 15.3 / 16.9s | 14.5 / 9.7 / 16.0s |
   | 2 `?panic=1` | 0.9 / 0.9 / 0.9s | 0.9 / 0.9 / 0.9s |
   | 3 `?frametime=1` | 20.6 / 13.9 / 20.3s | 10.8 / 20.8 / 19.8s |

3. **PR #79 changed web presentation.** Its commit claims the web frame path is
   byte-identical; that is true. `wgpu-hal`'s GL backend lists only `Fifo` on
   non-Windows (`src/gles/adapter.rs:1371`), so `get_default_config` already chose
   it and `WANTED_PRESENT_MODE` changed nothing. `presentation()` → `Vsync` →
   `needs_a_cap()` false → `ControlFlow::Poll`, before and after.

4. **PR #80's new GPU timer runs on the web.** It cannot: `wgpu-hal` gates
   `TIMESTAMP_QUERY` on the `GL_ARB_timer_query` desktop extension
   (`src/gles/adapter.rs:530`), absent on WebGL2, so `init::optional_features`
   returns empty and `GpuTimer` is never constructed in a browser. Everything else
   PR #80 adds is inert with the switch off, which is the state all failing passes
   run in.

5. **`wasm-opt` / the optimized bytes.** Installed binaryen `version_124` (CI's pin)
   and reproduced CI's exact sizes — `pong` 3.49→3.54 MB, `prototype_kit` 3.56 MB.
   Still passes locally.

6. **The runner is slow.** Main's own failing log gives pass 1 at **11.7s** and pass
   2 at **1.9s**; this container on the same page and the same bytes gives **11.9s**
   and **0.9s**. The machines are the same speed. Only pass 3 diverges — 20.6s here
   against >120s there.

7. **The runner image changed in the 16 minutes between #79's pass and main's
   fail.** Both jobs report `ubuntu-24.04` version `20260823.283.1`.

8. **`JIDOUSHA_CHROMIUM` stopped resolving**, falling back to the snap shim the
   `ci.yml` comment warns about. Both failing logs print
   `[serve-web] checking with /usr/bin/google-chrome`, so the override held.

## What the evidence points at

**A browser-side hang, with a precedent on record in this repo.** The `ci.yml`
comment beside the `JIDOUSHA_CHROMIUM` line says of the snap `chromium`:

> a snap shim that **hangs headless (observed: 120s, no DOM)**

The same signature — 120 seconds, no DOM — has been seen on this runner before and
was diagnosed as the browser binary, not the page. That workaround is still in force
and something is hanging anyway.

Two supporting observations:

- **Pass 3 is the only pass that opens a second WebGL context.** `probeRenderer()`
  creates one to read `WEBGL_debug_renderer_info`, on top of the engine's, under
  SwiftShader. Pre-existing, from the `?frametime=1` overlay; a suspect worth a look
  if the browser logs (item 1 above) point that way.
- **The check is high-variance.** Pass 3 on quiet machines here ranged **10.8s →
  28.8s** on identical bytes across today's runs, against a 120s wall. The failing
  pass differs by run (main: pass 3; PR #80: pass 1), which is what a wall being hit
  looks like rather than one code path.

## What could not be tested, and why

**CI's browser.** The runner drives the image's `google-chrome`; this container has
bundled `chromium-1194` only. Installing Chrome is blocked by the agent proxy's
network policy:

```
connect_rejected: gateway answered 403 to CONNECT — dl.google.com:443
```

Reproducing the hang almost certainly requires either CI's browser or a runner. That
is why item 1 under "What I need from you" matters: without the artifacts, the next
failure is as opaque as these three.

## State of the work

- **Branch** `claude/perf-panel-overlay-6tv4l7`, pushed, **clean**. The panel work
  is one commit, `6dd0e5f`; this file is the only thing on top of it. Per the
  template, delete it in the commit that resolves the blockage.
- **PR #80** is open. Every check green except `web build`. Two comments on it
  record the standing-down reasoning; the second is superseded by this document on
  one point — it leaned on "`main` is red too", and the fleet gap above is the
  better explanation.
- **No production code was changed by this investigation.** The only edits were a
  temporary `git checkout --detach` to `aba6afa` and `5f5f0dd` to measure them, both
  reverted; the branch is back where it was.
- **Local state a next session inherits:** binaryen `wasm-opt` v124 installed at
  `/usr/local/bin/wasm-opt` (not in the image — reinstall with `ci.yml`'s recipe),
  and `dist/` holding optimized `pong` and `prototype_kit` builds.
- **Safe to resume:** everything. The panel work is done and independently verified;
  only CI's browser check stands between PR #80 and green.

### Reproducing the measurements

```
# binaryen, so local builds are CI's bytes (ci.yml's own recipe)
curl -sSfL https://github.com/WebAssembly/binaryen/releases/download/version_124/binaryen-version_124-x86_64-linux.tar.gz | tar xz
install -m 755 binaryen-version_124/bin/wasm-opt /usr/local/bin/wasm-opt

tools/build-web pong          # or prototype_kit
tools/serve-web pong --check  # prints each pass's browser time as it completes
```
