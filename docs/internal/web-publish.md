# Web publish — design and contracts

Status: **living — W0, W1 and W2 done (W1 ticked 2026-08-22 on the owner's
observation of the production deploy; what production serves since 2026-08-23 is
§3a's release fleet); W3 deferred behind ADR-0038's trigger and
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
tools/build-web <example-or-game> [--debug]   # one page
tools/build-web --all | --release-fleet       # a whole fleet + the root index (§3a)
  cargo build --target wasm32-unknown-unknown (release profile by default;
  --debug for fast iteration — what ships is what gets tested)
  wasm-bindgen --target web --out-dir dist/<name>/
  wasm-opt -Os if available (optional; skipped with a log line if absent)
  stage: index.html (from tools/web-template/), every asset root the page's
  code can name (§1a), build stamp
tools/serve-web [<name>] [--check]
  local static server for dist/; MUST serve application/wasm correctly
  (implementation free; doctor verifies by fetching a .wasm and checking the
  Content-Type). --check drives a headless browser at /<name>/: once
  asserting the page ran and drew, once at ?panic=1 asserting the panic
  overlay rendered the full §9 text, once at ?frametime=1 asserting the
  frame-pacing overlay came up and classified the renderer (§2). Check
  artifacts go to target/web-check/, never into dist/ — dist is what deploys.
```

- CONTRACT: `tools/build-web` is the ONLY web build path — CI, local dev, and
  game repos all call the same script. No inline build steps in workflows. It
  builds one named page, or one of the two fleets §3a defines; the workflow
  chooses a fleet by name and never names a fleet's members.
- Build stamp: short git sha + build date, injected into the page (visible
  footer + `console.log`). A playtester's bug report always identifies its
  build. Sha comes from git at build time (allowed: this is tooling, not
  simulation — the wall-clock ban applies to engine code, not build scripts).
- wasm size: log final .wasm size in CI; warn (not fail) over 5 MB — drift
  visibility per practices §5.8's spirit.

## 1a. Asset roots: `dist/<name>/` is repository-shaped

Decided by the owner, 2026-08-23, and recorded as **ADR-0040** — which also
records the alternative that was declined (blessing `include_bytes!` as the way
a game ships art). This section is the pipeline half of it.

- CONTRACT: **an asset root is staged under the page at the path the code names
  it by.** `asset_source("assets")` and `asset_source("games/giri/assets")` are
  both paths from the top of the repository; the native loader reads them from
  there and the web loader fetches them relative to the page, and the build puts
  the directory at the same relative path under the page so one string means one
  set of pictures (assets.md §2's identical-paths CONTRACT).
- **Two roots, and a file's position picks which one it may use:**
  - the repository's shared `assets/` — what the engine's examples load from,
    staged for **every** page, unchanged from W0;
  - `games/<name>/assets/` — a game crate's own art, staged for **that game's
    page only**, so a prototype's art travels with it and two prototypes'
    `icon_coin.png` cannot collide (ADR-0038).
- The URL a game's picture ends up at is therefore
  `/giri/games/giri/assets/icon_coin.png`, which is redundant and is the price
  of the rule. ADR-0040's rationale is why the shorter `/giri/assets/…` is not
  available: it would make one string mean two different directories depending
  on which crate wrote it.
- `tools/check-assets` enforces the same two roots from the source side, so a
  root the build does not stage fails CI rather than 404ing on the deployed
  page. The game crate's directory comes from `cargo metadata` — a crate's
  directory and its binary's name are two different things and only one of them
  names the page.
- **The deploy needs nothing for this.** The workflow runs a fleet build and
  uploads `dist/` verbatim (§4), so a staged game directory rides along with the
  page it belongs to. Verified rather than assumed, on the PR that landed
  ADR-0040. Neither fleet changes it: games are in both (§3a), and a page that
  is not built stages nothing because it does not exist.

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
- **Frame-pacing overlay** (`?frametime=1`): a page-side instrument, on every
  deployed build, reachable by query parameter alone. It answers the one
  question a remote playtest cannot otherwise answer — "the ball is jumpy in my
  browser and smooth in yours, why" — with the two facts that settle it:
  - a rolling histogram of `requestAnimationFrame` deltas (one-millisecond
    buckets, which is the resolution that separates a 120Hz cadence from a 60Hz
    one and shows a quantised clock piling its deltas onto whole numbers),
    the estimated display refresh from their median, and their spread;
  - **ticks per rendered frame** — 0, 1, 2, 3+ — which is the *symptom*: the
    ball appears to jump forward exactly when one frame runs two ticks;
  - the **WebGL renderer string** (`WEBGL_debug_renderer_info`), classified.
    llvmpipe / lavapipe / SwiftShader / swrast / "software" / Mesa offscreen /
    "basic render" ⇒ a visible warning, "this browser is software-rendering;
    expect jank". Absent under `privacy.resistFingerprinting`, and reported as
    absent rather than guessed at;
  - whether WebGPU is present, and the browser's `performance.now()`
    resolution — Firefox clamps to ~1ms by default, Chrome to ~5µs, and a 1ms
    quantum against a 16.67ms cadence produces periodic 0-tick and 2-tick
    frames with nothing actually dropped.
- CONTRACT: **the overlay is presentation-side and never feeds the
  simulation.** It reads `performance.now()` and `requestAnimationFrame` and
  never calls into the wasm module; real time reaches the engine through
  `jidousha-platform`'s clock and the `Simulation::advance` argument, and
  nowhere else (ADR-0005, core.md §7). The consequence, stated on the panel
  itself: **ticks-per-frame is modelled, not read from the engine** — the page
  runs the same accumulator (60 Hz, 0.25s ceiling — the one place two constants
  agree by hand) over the deltas it measured. The alternative, exporting a
  counter from the wasm side, would put a page-side reader on the simulation's
  timeline for a diagnostic, and that is the door this contract keeps shut.
- The panel takes **no pointer events**, so it can never shadow the game's own
  input — which is also why its readings are kept short enough to fit without
  scrolling rather than relying on a scrollbar nobody can reach.
- Why it exists and what its readings mean, hypothesis by hypothesis:
  `docs/internal/frame-pacing.md`, which is where the owner's reading of it gets
  written down.
- `serve-web --check` has a third pass for it (§1). That pass can require more
  than "the overlay appeared": the check browser is deliberately
  software-rendering (`--use-angle=swiftshader`, because a runner has no GPU),
  so an overlay that fails to *notice* fails CI. A detector nothing exercises is
  a detector that rots, and this one is exercised on every run.
- Build stamp footer (§1).
- Reserved hook (deferred, do not build yet): "download recording" button
  wired to the input recording buffer (input §5) once I2 lands — turns remote
  playtesting into remote deterministic repro. Tracked here so the template
  leaves an obvious seam for it.

## 3. Deploy targets and layout

- **Engine repo**: `dist/` root is a generated index page listing every page
  this build produced — games in their own section (ADR-0038) — each at
  `/<name>/`, plus `stamp.txt` (the build stamp, read by
  the deploy workflow for its PR comment) and `fleet.txt` (the page names, the
  index's first link first, read by the workflow's browser check so it need not
  name a page — §3a, §4). Production URL serves latest
  `main`; dogfoods the whole pipeline. Examples built (`tools/build-web --all`,
  the full fleet): the **facade crate's** — what a game author sees — minus the
  native-only ones build-web names and skips aloud (`load_from_disk` reads a
  real disk; its wasm main is a printed stub, so wasm-bindgen has nothing to
  bind). Internal crates' examples are engine documentation, not playtest
  material. `prototype_kit` is the headline and leads the examples. Every crate under
  `games/` is built too, whole: a prototype exists to be played, and a playtest
  URL on its first push is most of why it lives in this repo (ADR-0038). Stale
  `dist/` subdirectories are pruned on every fleet build — dist deploys
  verbatim, so a renamed example, or a prototype moved to `attic/`, must not
  stay playable. Which of these pages the *production* deploy serves is §3a; a
  PR preview serves all of them.
- **Game repos**: single game at site root. Same scripts, same template,
  simpler layout — and the same §1a rule reads simpler too: the game *is* the
  repository, so its art is at `assets/` and its root string is `"assets"`
  (ADR-0040 says which line changes on the day a prototype moves out).
- Wrangler config: `wrangler.toml` with `assets.directory = "dist"`, no worker
  script, no bindings. Preview URLs enabled (default).

## 3a. Two fleets: production is curated, previews are the whole fleet

Decided by the owner, **2026-08-23**. Production deploys from `main` serve
**every game under `games/*` plus an explicit example allowlist**, which
currently holds exactly `pong`. **PR previews keep the full fleet** — every
example and every game.

- **The allowlist is data, in exactly one place**: `RELEASE_EXAMPLES` in
  `tools/build-web`. Nothing else may hold a copy of it. The workflow chooses a
  fleet by name (`--release-fleet` on a `main` push, `--all` on a PR) and never
  names a fleet's members; `wrangler.toml` uploads whichever `dist/` it is
  handed. A test asserts the workflow contains no page name, because the
  predictable way this rots is a second copy of the list living in CI.
- **Games are never allowlisted.** They are enumerated by the `games/*` glob,
  through `cargo metadata`, in *both* fleets — so a new prototype is on the
  production page with zero configuration, which is ADR-0038's no-registration
  property and is not up for negotiation here. A game name written into
  `RELEASE_EXAMPLES`, the workflow, or the index generator would be that
  property's first breach. Only *examples* are curated.
- **The allowlist rots loudly.** An entry naming an example the workspace no
  longer builds for the web fails the release build with the §9 message rather
  than quietly shrinking the page — the same discipline as
  `NATIVE_ONLY_EXAMPLES`.
- **The production index has two sections**: *games* (from the glob) first,
  because the prototypes are the thing to play, then *worked example* (the
  allowlist) as the reference a game author reads. The preview index keeps the
  shape it has always had — the whole example fleet first, `prototype_kit`
  leading, then games — because a preview is read by whoever is reviewing an
  engine change, and the examples are where that change shows up.
- **The browser check follows the fleet.** `build-web` writes `dist/fleet.txt`
  (the page names, the index's first link first) and the workflow checks its
  first line, so neither fleet needs a page the other lacks. That is
  `prototype_kit` on a preview and the allowlist's first example on production.

**Rationale.** W1's exit criterion — "every example playable remotely" (§6) —
was a *milestone's* exit criterion, met and observed on 2026-08-22, not a
standing policy. Production is the curated face of the project: a visitor should
find the games and one worked example, not a dozen engine test pages. Previews
are the diagnostic surface, where engine PRs get eyeballed and where device bugs
have historically been found — the binaryen-108 externref defect (§5) was caught
by playtesting PR #59's preview on an iPad and two Android browsers, and a
preview that had shipped only the curated fleet would have had far less surface
to find it on.

**Declined: keep publishing everything to production.** It is simpler by exactly
one flag, and it is what W1 built. It was declined because the production URL is
what the project shows people, and a page mostly made of `headless_sim`,
`input_echo`, `loading_gate`, `spawn_and_reap`, `vec2_tour` and `window_clear`
reads as a test harness rather than as an engine with games on it. The
diagnostic value those pages carry is real, which is exactly why they stay on
previews rather than being deleted: the split keeps both properties instead of
trading one for the other.

**No ADR.** The decision reverses no CONTRACT — searched before landing: §1's
one-build-path CONTRACT holds (both fleets are `tools/build-web`), §1a's
asset-root CONTRACT holds, and ADR-0038's guarantee is about a game needing no
registration, which is preserved exactly. ADR-0038's consequence list mentions
`tools/build-web --all` as the deploy's fleet; that names the mechanism of the
day, not the guarantee, and the ADR stands as written (ADRs are superseded, not
edited). This section is where the decision lives.

## 4. CI workflow (engine repo; template mirrors it)

- Trigger: push to `main` → production deploy (`wrangler deploy`);
  `pull_request` → preview deploy (`wrangler versions upload --preview-alias
  pr-<number>`, so the preview URL is stable across pushes while its content
  tracks the branch head). Both live in `.github/workflows/ci.yml` (`web` +
  `deploy` jobs) — same workflow run as the gates, per the next line.
- The `web` job builds the fleet the trigger calls for: `tools/build-web
  --release-fleet` on a `main` push, `--all` on a PR (§3a). That one expression
  is the whole of CI's knowledge about fleets — it names neither an example nor
  a game. It then browser-checks the first line of `dist/fleet.txt`, so the
  check runs against a page the built fleet actually contains.
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
  every example playable remotely. (Met and observed 2026-08-22. That exit
  criterion was this milestone's, not a standing policy: what production serves
  from then on is §3a's decision, taken 2026-08-23.)
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
