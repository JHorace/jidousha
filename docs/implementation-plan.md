# Implementation plan — Jidousha v1

The handoff document for implementing Claude Code sessions. Read this after
CLAUDE.md, before starting any milestone. Update the checklist (§4) in the same
commit that completes a milestone.

## 1. Session-zero (repo initialization) — do once, before M0

1. `git init`; commit the planning docs (this file, `CLAUDE.md`, `docs/`) as the
   first commit.
2. `rust-toolchain.toml` pinning current stable; workspace `Cargo.toml` with the
   crate layout from core doc §1 (crates may start empty).
3. CI provider: GitHub Actions (confirmed — the repo lives on GitHub). Jobs from
   day one: fmt, clippy `-D warnings`, test (Linux + Windows),
   `cargo check --target wasm32-unknown-unknown`, CLAUDE.md line-count, plus
   `tools/doctor` on a healthy runner and a dependency-count report
   (practices §5.8). See `docs/internal/tooling.md`.
4. Then start M0.

## 2. Per-session protocol

- Read: `CLAUDE.md` → this file's checklist → the milestone's design-doc
  sections (linked below). Don't read every doc every session; the routing
  table exists so you don't have to.
- One milestone per session is the intended grain. Finishing early: polish
  tests/docs for the finished milestone rather than starting the next.
- Definition of done: CLAUDE.md. Doc updates land with code (practices §2).
  Dependency additions follow practices §5.8 (justify + `cargo tree` delta).
- Build/test trouble: CLAUDE.md "When builds/tests fail" — doctor first, two
  attempts, then BLOCKED.md. This applies from M0 onward (M0 *builds* doctor;
  until it exists, escalate directly).
- **Precedence when documents disagree**: ADRs > subsystem design docs >
  practices/conventions > CLAUDE.md. A conflict is itself a bug: fix the
  lower-precedence doc in the same commit, or write BLOCKED.md if the conflict
  is substantive.
- Contracts (CONTRACT-marked) change only via ADR. When implementation reality
  argues with a design-doc detail that is *not* contract-marked, implement the
  sensible thing and update the doc, noting the change in the commit message.

## 3. Milestone sequence

Single-agent linear order (dependencies in parentheses; details in each doc):

```
Core (docs/internal/core.md §11)      M0 → M1 → M2 → M3 → M4
Assets (assets.md §8)                 A0 (M3)
Input (input.md §8)                   I0 (M3)
Renderer (renderer.md §11)            R0 (M4)
Platform                              M5 (M4)
                                      R1 (M5, R0) → R2 (R1, A0→A1) → R3 (R2)
                                      I1 (M5, I0, R3-text for echo example)
                                      A2 (A1) → I2 (I1, A0) → R4 (R3, I2)
                                      A3 (A2, R4)
Facade & docs (public-api.md)         F0 (all above): facade crate, prelude,
                                        gen-api-doc + docs/api, check-api-coverage,
                                        quickstart example
Acceptance                            E0 (F0): see below
Web publish (web-publish.md §6)       W0 (R1) → W1 (W0 + owner secrets,
                                        web-publish.md §7) → W2 (W1)
                                      W3 (W2; pairs with the post-E0
                                        make-game skill work)
```

Recommended strict order:
`M0 M1 M2 M3 M4 A0 I0 R0 M5 R1 A1 R2 R3 I1 A2 I2 R4 A3 F0 E0`

A0/I0/R0 are independent of each other (all pure, no platform deps) — natural
parallel work if multiple sessions ever run concurrently (worktrees).

**E0 — acceptance ("agent Pong").** A *fresh* Claude Code session, given only
`docs/api/` + `examples/` (enforced: it may not read `src/` or
`docs/internal/` — this restriction is stated in its prompt and honored on
trust + checked by reviewing its transcript), builds a playable Pong with score
text, verified via `tools/verify` script + a human web playtest. Failures are
treated as engine/docs bugs first, not prompt bugs: each E0 failure gets a
root-cause note in `docs/internal/e0-findings.md` and a fix. E0 repeated until
it passed clean twice in a row — **clean meaning no `engine` finding and no
*novel* `docs` finding** (ADR-0029; a re-tread of an already-recorded shape is
fixed but does not reset the streak). That was the project's definition of
working.

**It never happened, and E0 is closed anyway (ADR-0036).** Eleven runs, a hundred
and forty-one findings, zero consecutive clean runs. The exercise ends because
its own instrumentation stopped measuring the document — `e0-findings.md` §6
establishes that the novel count tracks the author rather than the prose — and
because no run since run 4 has found the engine doing the wrong thing. What is
recorded is that `docs/api/` is sufficient for an author who cannot read the
source to build, check, capture and ship a working **Pong**; that is narrower
than "sufficient for a game", and ADR-0036 says so at length. The harness stays
runnable as a regression check.

**Harness ready** (see `docs/internal/e0-prompt.md` for the prompt and the
before/after procedure, and `e0-findings.md` for the classification rule and the
bar). Two things about running it are worth stating here, because they are what
the milestone's honesty depends on:

- **The session that runs E0 must not be the session that built the engine.**
  Discipline about not *opening* `src/` is not the same as not knowing what is
  in it. An author who already knows the answers measures nothing, and would
  produce a pass — which is worse than a failure, because E0's whole output is
  the findings a failure generates.
- **The run's `docs/e0/run-N.md` is the raw observation; `e0-findings.md` is the
  root cause.** Keeping them separate keeps the maintainer's explanation from
  overwriting what the game author actually hit. **One file per run**, and the
  author writes only their own: run 2 was pointed at run 1's file and read it
  before writing a line of Pong, which handed it three facts it should have had
  to find.
- **The previous run's game is deleted before the next run starts.**
  `crates/jidousha/examples/` is on the run's allowed list, so a finished Pong
  left in it is a worked solution the next author may read entirely within the
  rules — and would find first. Run 2 deleted run 1's unprompted and said so;
  the harness must not depend on an author choosing to (e0-findings.md F-020).
  It is step 2 of `e0-prompt.md`'s before-the-run checklist.

  **With E0 closed there is no next run, so run 11's `pong` stays** — in
  `crates/jidousha/examples/` and registered in `tools/test`, a permanent worked
  example beside `slalom/` and `prototype_kit/`. The step protects the next
  author's reading list, and there is no next author (ADR-0036). It applies again
  the moment E0 is re-run.

After E0: the `make-game` skill (practices §3) was written from what E0 taught
— `.claude/skills/make-game/SKILL.md`, a checklist that orders the four
`docs/api/` documents and points into them. With it, v1 is done and the
deferred lists become the roadmap conversation.

## 4. Progress checklist

Tick in the completing commit. (All unticked at handoff.)

- [x] session-zero  - [x] M0  - [x] M1  - [x] M2  - [x] M3  - [x] M4
- [x] A0  - [x] I0  - [x] R0  - [x] M5  - [x] R1  - [x] A1  - [x] R2
- [x] R3  - [x] I1  - [x] A2  - [x] I2  - [x] R4  - [x] A3  - [x] F0
- [x] E0  - [x] make-game skill
- [x] W0  - [x] W1  - [x] W2  - [ ] W3 (deferred, ADR-0038)

E0 is ticked as **closed after eleven runs, not as passed** (ADR-0036). The
condition §3 states was never met — the streak at closure is zero — and it is
retired rather than lowered. `e0-findings.md` §2 and §6 carry the reasoning and
what the closure does not establish.

`make-game` was the last box of v1: v1 is complete, and per §3 the deferred
lists are now the roadmap conversation rather than pending work. W0–W3 are
post-v1 — the web publish track (ADR-0037, web-publish.md §6).

W2 was observed live on PR #59: one sticky comment
(`pr-59-jidousha.jpsumihiro.workers.dev`, created 2026-08-22T04:57Z), and the
next push edited that same comment in place with the new stamp — no
duplicate. **W1 ticks on 2026-08-22**: the owner observed the production deploy
from `main`, every example served and playable at the production URL. The
condition the previous paragraph set — a `main` push observed serving the whole
fleet, not merely a green deploy job — is the one that was met. That was the
milestone's exit criterion and not a standing policy: since 2026-08-23
production serves a curated release fleet — every game plus an example
allowlist — while PR previews keep the full fleet (web-publish.md §3a).

**W3 is deferred behind a trigger rather than a date** (ADR-0038): prototypes
are workspace members at `games/<name>/` on `main`, so the game-repo template
lands when the first prototype has to leave this repository — one that ships
under its own name, or one whose CI time or churn measurably slows the engine's
own loop. Neither is true of any prototype that exists. `web-publish.md` §6's W3
entry stands unchanged as the design for when it fires.

## 5. Document map

```
CLAUDE.md                        router — always read first
docs/agent-practices.md          why every rule exists; enforcement map
docs/conventions.md              coordinates, units, color, naming
docs/implementation-plan.md      this file
docs/adr/0001..0041              decisions; DELIBERATE tags point here
docs/internal/core.md            ECS, schedule, time, app (M-milestones)
docs/internal/renderer.md        submissions, backend seam (R-milestones)
docs/internal/assets.md          handles, readiness determinism (A-milestones)
docs/internal/input.md           snapshots, replay (I-milestones)
docs/internal/public-api.md      facade inventory, docs/api spec (F0/E0)
docs/internal/tooling.md         tools/ scripts, CI jobs, enforcement (M0)
docs/internal/web-publish.md     web build/serve/deploy pipeline (W-milestones)
docs/internal/frame-pacing.md    the Firefox pacing defect; diagnosed, parked
docs/internal/e0-prompt.md       the acceptance prompt, verbatim (E0 harness)
docs/internal/e0-findings.md     what building a game actually cost (E0)
docs/templates/BLOCKED.md        escalation template
```
