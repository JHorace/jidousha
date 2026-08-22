# Web publish — design and contracts

Status: **living — W0, W1 and W2 done (W1 ticked 2026-08-22 on the owner's
observation of the production deploy); W3 deferred behind ADR-0038's trigger and
still design.** The internal doc for the web
build/publish tooling. **CONTRACT** items binding as elsewhere.

Inherits: web as tier-1 + single-threaded/no-COOP-COEP (ADR-0005), Cloudflare
Workers static assets decision (ADR-0037), error taxonomy (core §9), recording
format (input §5), dependency budget (practices §5.8).

In scope: `tools/build-web`, `tools/serve-web`, the playtest page shell, CI
deploy (production + PR previews), the game-repo workflow template.
Out of scope: itch.io (dropped — §8), analytics, multiplayer/server anything,
asset CDN tricks, threads (would reopen COOP/COEP).

---

## 1. The pipeline

```
tools/build-web <example-or-game> [--debug]
  cargo build --target wasm32-unknown-unknown (release profile by default;
  --debug for fast iteration — what ships is what gets tested)
  wasm-bindgen --target web --out-dir dist/<name>/
  wasm-opt -Os if available (optional; skipped with a log line if absent)
  stage: index.html (from tools/web-template/), assets/, build stamp
tools/serve-web [<name>] [--check]
  local static server for dist/; MUST serve application/wasm correctly
  (implementation free; doctor verifies by fetching a .wasm and checking the
  Content-Type). --check drives a headless browser at /<name>/: once
  asserting the page ran and drew, once at ?panic=1 asserting the panic
  overlay rendered the full §9 text. Check artifacts go to target/web-check/,
  never into dist/ — dist is what deploys.
```

- CONTRACT: `tools/build-web` is the ONLY web build path — CI, local dev, and
  game repos all call the same script. No inline build steps in workflows.
- Build stamp: short git sha + build date, injected into the page (visible
  footer + `console.log`). A playtester's bug report always identifies its
  build. Sha comes from git at build time (allowed: this is tooling, not
  simulation — the wall-clock ban applies to engine code, not build scripts).
- wasm size: log final .wasm size in CI; warn (not fail) over 5 MB — drift
  visibility per practices §5.8's spirit.

## 2. The playtest page shell (`tools/web-template/`)

One `index.html` template, self-contained (no external CDN dependencies):

- Canvas + loading state (spinner until wasm instantiates; distinct message if
  instantiation itself fails, naming likely causes: old browser, MIME).
- **Panic overlay** (CONTRACT): the wasm panic hook renders the full §9 panic
  message as a styled in-page overlay with a copy button — NOT console-only.
  Remote playtesters don't open devtools; "screenshot the red box" is the bug
  report. This is the §9 error discipline extended to its last mile.
  Mechanics: the hook (jidousha-platform `web/panic.rs`, installed by `run`
  before anything that can panic) writes the message to `console.error`
  behind a first-line marker (`[jidousha panic]`); the page renders whatever
  follows the marker. Engine panics pass through verbatim (their payload is
  already the full §9 text); arbitrary game panics are wrapped in the §9
  shape. The `[jidousha] `-prefixed handled reports stay on the status line —
  a missing asset is not a panic.
- **Forced test panic**: loading any game with `?panic=1` panics at startup
  with a §9-formatted test message (checked in the platform's `run`, web
  only). It exists so the overlay contract is verifiable — manually on any
  deployed build, and by `serve-web --check`'s second pass — and it ships in
  real games because a bug-reporting path nobody can test is a path that rots.
- Build stamp footer (§1).
- Reserved hook (deferred, do not build yet): "download recording" button
  wired to the input recording buffer (input §5) once I2 lands — turns remote
  playtesting into remote deterministic repro. Tracked here so the template
  leaves an obvious seam for it.

## 3. Deploy targets and layout

- **Engine repo**: `dist/` root is a generated index page listing every built
  example and game — games in their own section (ADR-0038) — each at
  `/<name>/`, plus `stamp.txt` (the build stamp, read by
  the deploy workflow for its PR comment). Production URL serves latest
  `main`; dogfoods the whole pipeline. Examples built (`tools/build-web
  --all`): the **facade crate's** — what a game author sees — minus the
  native-only ones build-web names and skips aloud (`load_from_disk` reads a
  real disk; its wasm main is a printed stub, so wasm-bindgen has nothing to
  bind). Internal crates' examples are engine documentation, not playtest
  material. `prototype_kit` is the headline and leads the examples. Every crate under
  `games/` is built too, whole: a prototype exists to be played, and a playtest
  URL on its first push is most of why it lives in this repo (ADR-0038). Stale
  `dist/` subdirectories are pruned on every `--all` build — dist deploys
  verbatim, so a renamed example, or a prototype moved to `attic/`, must not
  stay playable.
- **Game repos**: single game at site root. Same scripts, same template,
  simpler layout.
- Wrangler config: `wrangler.toml` with `assets.directory = "dist"`, no worker
  script, no bindings. Preview URLs enabled (default).

## 4. CI workflow (engine repo; template mirrors it)

- Trigger: push to `main` → production deploy (`wrangler deploy`);
  `pull_request` → preview deploy (`wrangler versions upload --preview-alias
  pr-<number>`, so the preview URL is stable across pushes while its content
  tracks the branch head). Both live in `.github/workflows/ci.yml` (`web` +
  `deploy` jobs) — same workflow run as the gates, per the next line.
- Deploy job runs only after build+test jobs pass in the same workflow run.
  Concurrency group per-branch, cancel-in-progress (stale pushes don't race).
- Bootstrap: previews are versions of the production Worker, so the first
  preview in a fresh account has no Worker to attach to. The preview step
  catches exactly that wrangler error, deploys once to create the Worker
  (production serves nothing until the first `main` push, which replaces
  it), and retries the aliased upload. Any other upload failure stays a
  failure.
- PR preview posts a **sticky comment** (created once, found again by an HTML
  marker and updated on subsequent pushes — never one comment per push) with
  the preview URL + build stamp (from `dist/stamp.txt`). On previews the
  stamp's sha names the PR's **merge commit** (CI checks out
  `refs/pull/N/merge`), not the branch head — what deployed is what would
  merge, and the stamp says so honestly.
- Fork PRs: secrets absent → deploy job skips with a neutral notice. Not
  worked around (ADR-0037).
- CI-only deps: node + wrangler live in the workflow, never in rust-toolchain
  or doctor's local requirements.

## 5. Toolchain checks (doctor additions)

- `wasm-bindgen-cli`, when present, version-identical to the workspace's
  `wasm-bindgen` crate version (mismatch is the classic silent runtime
  breakage → `ENV_FIXABLE: cargo install wasm-bindgen-cli --version <x>`).
  Absence is info, not a fault: it cannot break silently — `build-web` gates
  every build with that same command — and a machine that never builds for
  the web is healthy without it, which the CI doctor job requires
  (practices §6.1: a healthy runner must produce ENV_OK).
- `wasm32-unknown-unknown` target installed (already listed, ADR-0005).
- `wasm-opt`: optional; absence reported as info, not failure. A version
  older than build-web's pinned minimum (124) is *refused* by build-web —
  skipped with a log line — and doctor says so: binaryen 108 (Ubuntu 24.04's
  package, and the runner's) clamps the externref table wasm-bindgen's glue
  grows from JS, and every optimized module then dies at startup in every
  browser (`RangeError: WebAssembly.Table.grow`; found by playtesting PR
  #59's preview on iPad Safari and Android Firefox/Chrome, reproduced in
  desktop Chromium, gone at binaryen 124). CI installs a pinned binaryen
  release and browser-checks the optimized bytes before they can deploy.
- `tools/serve-web` MIME self-check (§1).

## 6. Milestones

- **W0 — build + serve + shell** (needs R1: something renders on web).
  `build-web`, `serve-web`, template with loading/panic-overlay/stamp, doctor
  additions. Exit: `tools/build-web sprites && tools/serve-web sprites` gives
  a playable local page; a forced panic shows the overlay with the §9 text.
- **W1 — production deploy** (needs W0 + owner setup §7). Workflow, wrangler
  config, root index generation. Exit: `main` push updates the live URL;
  every example playable remotely.
- **W2 — PR previews** (needs W1). Preview deploys + sticky comment.
  Exit: a test PR shows its own URL in a comment; second push updates the
  same comment; close/merge cleans up per Workers defaults.
- **W3 — game-repo template** (needs W2). `templates/game-web-publish/`
  (workflow + wrangler.toml + README-snippet including the §7 checklist for
  the game repo's own secrets), consumed by the `make-game` skill (note added
  to the skill's spec in practices §3 / implementation-plan post-E0 item).
  Exit: a scratch game repo, template applied, deploys end-to-end.

Sequencing within the master plan: W0 slots after R1; W1–W2 any time after
(independent of R2+); W3 pairs with the post-E0 `make-game` skill work.
Add W0–W3 to the implementation-plan checklist when landing this doc.

**W3's trigger is ADR-0038**, which put prototypes in this workspace at
`games/<name>/` instead of in repositories of their own: the entry above is the
design, and it is built when the first prototype has to leave — one that ships
under its own name, or one whose CI time or churn measurably slows the engine's
own loop. Stated here, in `implementation-plan.md` §4 and in the ADR, and the
three move together.

## 7. Owner setup (human-only; agents never do these)

1. Cloudflare account; note the account ID.
2. API token scoped to Workers Scripts:Edit (+ Workers Builds if used).
3. GitHub repo secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`.
4. (Per game repo later: same two secrets, or a shared org-level secret.)

CONTRACT: missing/invalid secrets at W1+ is BLOCKED-class (write BLOCKED.md
naming which secret; do not attempt alternative hosts or token creation).

## 8. Deferred

Recording-download button (after I2; seam reserved in §2) · custom domain ·
analytics/telemetry · password-gated playtests (Cloudflare Access would do it if
ever needed).

**itch.io is dropped, not deferred** (owner decision, 2026-08-22): it is not a
goal for this project, and the path this document builds already serves the
purpose it was being kept for — a URL somebody can play. ADR-0037's rationale
mentions it as a possible later release channel; that ADR is accepted and stays
as written (ADRs are superseded, not edited), and this line is where the
decision lives. Nothing in the tooling ever referenced it.
