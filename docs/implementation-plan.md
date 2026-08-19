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
root-cause note in `docs/internal/e0-findings.md` and a fix. E0 repeats until
it passes clean twice in a row. This is the project's definition of working.

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

After E0: write the `make-game` skill (practices §3) from what E0 taught, then
v1 is done and the deferred lists become the roadmap conversation.

## 4. Progress checklist

Tick in the completing commit. (All unticked at handoff.)

- [x] session-zero  - [x] M0  - [x] M1  - [x] M2  - [x] M3  - [x] M4
- [x] A0  - [x] I0  - [x] R0  - [x] M5  - [x] R1  - [x] A1  - [x] R2
- [x] R3  - [x] I1  - [x] A2  - [x] I2  - [x] R4  - [x] A3  - [x] F0
- [ ] E0  - [ ] make-game skill

## 5. Document map

```
CLAUDE.md                        router — always read first
docs/agent-practices.md          why every rule exists; enforcement map
docs/conventions.md              coordinates, units, color, naming
docs/implementation-plan.md      this file
docs/adr/0001..0017              decisions; DELIBERATE tags point here
docs/internal/core.md            ECS, schedule, time, app (M-milestones)
docs/internal/renderer.md        submissions, backend seam (R-milestones)
docs/internal/assets.md          handles, readiness determinism (A-milestones)
docs/internal/input.md           snapshots, replay (I-milestones)
docs/internal/public-api.md      facade inventory, docs/api spec (F0/E0)
docs/internal/tooling.md         tools/ scripts, CI jobs, enforcement (M0)
docs/internal/e0-prompt.md       the acceptance prompt, verbatim (E0 harness)
docs/internal/e0-findings.md     what building a game actually cost (E0)
docs/templates/BLOCKED.md        escalation template
```
