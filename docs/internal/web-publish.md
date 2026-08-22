# Web publish — design and contracts

Status: **design draft, pre-implementation.** Becomes the living internal doc
for the web build/publish tooling. **CONTRACT** items binding as elsewhere.

Inherits: web as tier-1 + single-threaded/no-COOP-COEP (ADR-0005), Cloudflare
Workers static assets decision (ADR-00NN), error taxonomy (core §9), recording
format (input §5), dependency budget (practices §5.8).

In scope: `tools/build-web`, `tools/serve-web`, the playtest page shell, CI
deploy (production + PR previews), the game-repo workflow template.
Out of scope (deferred): itch.io release channel, analytics, multiplayer/
server anything, asset CDN tricks, threads (would reopen COOP/COEP).

---

## 1. The pipeline

```
tools/build-web <example-or-game> [--release]
  cargo build --target wasm32-unknown-unknown (release profile by default)
  wasm-bindgen --target web --out-dir dist/<name>/
  wasm-opt -Os if available (optional; skipped with a log line if absent)
  stage: index.html (from tools/web-template/), assets/, build stamp
tools/serve-web [<name>]
  local static server for dist/; MUST serve application/wasm correctly
  (implementation free; doctor verifies by fetching a .wasm and checking the
  Content-Type)
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
- Build stamp footer (§1).
- Reserved hook (deferred, do not build yet): "download recording" button
  wired to the input recording buffer (input §5) once I2 lands — turns remote
  playtesting into remote deterministic repro. Tracked here so the template
  leaves an obvious seam for it.

## 3. Deploy targets and layout

- **Engine repo**: `dist/` root is a generated index page listing every built
  example, each at `/<example>/`. Production URL serves latest `main`;
  dogfoods the whole pipeline. Examples built: all of them; `prototype_kit`
  is the headline.
- **Game repos**: single game at site root. Same scripts, same template,
  simpler layout.
- Wrangler config: `wrangler.toml` with `assets.directory = "dist"`, no worker
  script, no bindings. Preview URLs enabled (default).

## 4. CI workflow (engine repo; template mirrors it)

- Trigger: push to `main` → production deploy; `pull_request` → preview deploy.
- Deploy job runs only after build+test jobs pass in the same workflow run.
  Concurrency group per-branch, cancel-in-progress (stale pushes don't race).
- PR preview posts a **sticky comment** (created once, updated on subsequent
  pushes — never one comment per push) with the preview URL + build stamp.
- Fork PRs: secrets absent → deploy job skips with a neutral notice. Not
  worked around (ADR-00NN).
- CI-only deps: node + wrangler live in the workflow, never in rust-toolchain
  or doctor's local requirements.

## 5. Toolchain checks (doctor additions)

- `wasm-bindgen-cli` present AND version-identical to the workspace's
  `wasm-bindgen` crate version (mismatch is the classic silent runtime
  breakage → `ENV_FIXABLE: cargo install wasm-bindgen-cli --version <x>`).
- `wasm32-unknown-unknown` target installed (already listed, ADR-0005).
- `wasm-opt`: optional; absence reported as info, not failure.
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

## 7. Owner setup (human-only; agents never do these)

1. Cloudflare account; note the account ID.
2. API token scoped to Workers Scripts:Edit (+ Workers Builds if used).
3. GitHub repo secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`.
4. (Per game repo later: same two secrets, or a shared org-level secret.)

CONTRACT: missing/invalid secrets at W1+ is BLOCKED-class (write BLOCKED.md
naming which secret; do not attempt alternative hosts or token creation).

## 8. Deferred

itch.io release channel (butler; revisit post-v1) · recording-download button
(after I2; seam reserved in §2) · custom domain · analytics/telemetry ·
password-gated playtests (Cloudflare Access would do it if ever needed).
