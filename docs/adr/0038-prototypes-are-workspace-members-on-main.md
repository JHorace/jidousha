# ADR-0038: Prototypes are workspace members on `main`

Status: accepted · 2026-08-22

## Context

Twelve prototypes have now been written against this engine, and every one of
them lived at `crates/jidousha/examples/<name>/`. That was the right home while
E0 was running: the exercise asked whether `docs/api/` alone is enough for an
author who cannot read the source, and an example of the facade crate is the
cheapest thing that compiles against exactly the facade. ADR-0036 closed E0 and
kept three of them — `prototype_kit/`, `slalom/`, `pong/` — as permanent worked
examples.

What comes next is not E0. The next prototype is written to find out whether a
mechanic is fun, not whether a document is sufficient, and it wants three things
an engine example cannot give it:

**A dependency edge that is checkable.** An example of `crates/jidousha` inherits
that crate's dependency table. Every internal crate — `jidousha-core`,
`jidousha-render-wgpu` — is a resolved `extern crate` away, and nothing fails if
an author reaches one. `public-api.md` §4 CONTRACT says games go through the
facade; for examples that is enforced by `tools/check-api-coverage` scanning
their text for the names. Text-scanning is the enforcement available when the
edge is not in a manifest. When a prototype is its own crate, the edge *is* in a
manifest, and a manifest is checkable exactly.

**Lints that fit a prototype.** The workspace lint table denies `missing_docs`,
because `docs/api/` is generated from the engine's doc comments (practices §2.3).
A prototype has no public API and generates no document. It should still get
`fmt`, `clippy -D warnings` and the ADR-0009 determinism bans — those catch real
bugs in a game — but a prototype that must document a private helper to compile
is paying an engine's tax for a game's work.

**A name that says what it is.** `crates/jidousha/examples/` is the engine's
canonical-example directory: `tools/check-api-coverage` reads it as the proof
that every public item is shown somewhere, and `docs/api/` points at it by name.
Prototypes accumulating there make that directory two things at once, and the
one thing it must stay is small enough to read.

The alternative already on the books is W3 of the web-publish track (ADR-0037,
web-publish.md §6): a `templates/game-web-publish/` the `make-game` skill copies
into a *separate repository* per game. That is a real answer to all three, and it
costs a repository, two Cloudflare secrets (web-publish.md §7, owner-only), a CI
pipeline and a copy of the engine version per prototype — paid on the first
prototype, when nothing yet says a prototype needs to leave.

## Decision

**A prototype is a workspace member of this repository, at `games/<name>/`, on
`main`.** It depends on the engine through the `jidousha` facade crate and
through nothing else. It is not a separate repository, and it is not an example
of the engine crate.

- `games/<name>/` is an ordinary crate: `Cargo.toml`, `src/main.rs`, and the
  files its `--verify` mode needs. `crates/jidousha/examples/prototype_kit/` is
  the shape to copy — `main.rs`, `checks.rs`, `verify.rs`, `capture.rs` — moved
  from an example directory into a crate's `src/`.
- **Facade only.** Every crate under `games/*` may name `jidousha` among its
  dependencies and no other `jidousha-*` crate, directly or transitively.
  Mechanized by `tools/check-game-deps`, which fails naming the offending
  dependency and this ADR.
- **`attic/` holds retired prototypes.** Excluded from the workspace and from
  every tooling glob: read, never compiled, never deployed. A prototype that
  stops earning its build time moves there instead of being deleted, because the
  thing worth keeping about a dead prototype is what it looked like.
- **The three surviving E0 games do not move.** `prototype_kit/`, `slalom/` and
  `pong/` stay at `crates/jidousha/examples/` where ADR-0036 and the
  implementation plan put them. They are the engine's worked examples and carry
  `check-api-coverage`; `games/` starts empty and takes what comes next.

### Lint layering

A game gets the checks that catch bugs in a game and is exempt from the ones
that exist to keep an engine's public surface honest.

| Check | Games | Why |
|---|---|---|
| `cargo fmt --all` | yes | one diff shape across the repo |
| `clippy -D warnings` | yes | it finds real bugs; a prototype is not exempt from bugs |
| ADR-0009 determinism bans (`clippy.toml` `disallowed-methods`) | yes | a game that drifts across platforms is a game whose `--verify` mode lies |
| `unwrap_used` / `expect_used` | yes, per-file opt-out | same rule examples already have (`clippy.toml`) |
| `tools/check-assets` | yes | a mistyped asset path is a runtime placeholder hunt either way |
| `missing_docs` | **no** | a prototype has no public surface and generates no document |
| module-header shape (practices §1.2) | **no** | the shape is for a reader navigating a subsystem, not a game |
| `tools/check-api-coverage` | **no** | it proves the *engine's* items are shown; a game shows nothing |
| `tools/gen-api-doc` | **no** | `docs/api/` is generated from the facade; games are not in it |

### Tooling

Every tool that enumerates something playable picks up `games/*` with no list to
maintain:

- `tools/test` runs each game through `tools/verify`, and its
  `tools/check-game-deps` phase enforces the facade edge.
- `tools/verify <name>` resolves a name to an example or to a game.
- `tools/build-web <name>` and `--all` build games alongside examples;
  `tools/serve-web` serves whatever `dist/` holds.
- `tools/check-assets` reads `games/*/src/` as well as the engine's crates.
- The deploy workflow's fleet is `tools/build-web --all`, so the root index
  lists games in their own section beside the examples — a playtest URL per
  prototype, on the first push, with no per-game setup.

**A game needs no registration anywhere.** The registration step that
`tools/test` demands of an example — adding a name to `WINDOWED_EXAMPLES` and
`VERIFIABLE_EXAMPLES` — was missed after E0 runs 4, 5 and 7 (F-094) and does not
exist for a game: living under `games/` is what makes it windowed and verified.

## Rationale

**The dependency edge is the whole point, and only a crate has one.** The rule
"a game goes through the facade" is the load-bearing claim of `public-api.md`:
it is what makes the facade's surface *the* surface, and what E0 spent eleven
runs testing. As long as a prototype is an example of the facade crate, that rule
is enforced by grepping for names in source text — which cannot see a reach made
through a re-export, and cannot see a transitive one at all. A `games/*` crate
declares its dependencies in a manifest, `cargo metadata` resolves them
transitively, and the check becomes exact. This ADR does not add the rule; it
moves it somewhere it can be checked.

**`main` is where the engine's own gates already run.** A prototype in this
workspace is compiled by the same `cargo check`, linted by the same clippy
invocation, and deployed by the same `tools/build-web --all` as everything else.
An engine change that breaks a prototype fails on the PR that makes it, which is
the entire value of dogfooding and is exactly what a separate repository gives
up: there, the breakage surfaces the next time somebody bumps a version, with the
cause several commits behind.

**The cost of a repository is paid per prototype; the cost of a directory is
paid once.** W3's template is right for a game that ships — its own name, its own
release cadence, possibly its own collaborators. It is wrong for the third
prototype of a mechanic somebody wants to feel for thirty seconds. Making the
cheap case cheap is what decides how many prototypes get written, and how many
get written is the thing this engine is optimizing.

**`attic/` costs nothing and answers a question that recurs.** "What did that
one look like?" is asked about dead prototypes constantly, and the answers are
otherwise in a branch nobody can name. Excluding the directory from the workspace
means the cost of keeping it is disk, not build time.

## Consequences

- **W3 is deferred**, behind a stated trigger rather than a date: it lands when
  the first prototype has to leave this repository — one that ships under its own
  name, or one whose CI time or churn measurably slows the engine's own loop.
  Neither is true of any prototype that exists. `implementation-plan.md` §4
  records the deferral; `web-publish.md` §6's W3 entry stands unchanged as the
  design for when the trigger fires.
- **`tools/check-game-deps` is a CI gate and a `tools/test` phase**, in that
  order of discovery and the same list either way: a check that runs in one place
  and not the other is a check whose result depends on where you stood.
- **The `games/*` workspace glob must match something.** Cargo resolves a
  members glob that matches nothing to a literal path and fails to read its
  manifest, so `games/README.md` is load-bearing as well as documentary — it is
  what makes the glob non-empty while no prototype exists. The README says so at
  the site.
- **`docs/api/` does not change.** Games read it; nothing in it reads games. This
  decision touches no public item, and `tools/gen-api-doc --check` produces no
  diff.
- **The `make-game` skill owns the `games/` workflow** — scaffolding, the
  docs/api-only reading discipline, the `--verify` requirement, and a
  findings-capture step in the `e0-findings.md` format. A prototype is still an
  author's report on the documents as well as a game; E0 ended, the reporting
  did not.
- A prototype that reaches an internal crate now fails CI instead of compiling.
  That is a new way to be stopped, and it is the one this ADR exists to add.

## Alternatives rejected

- **Keep prototypes in `crates/jidousha/examples/`.** Zero new machinery, and
  the facade rule stays a text scan while the engine's canonical-example
  directory grows without bound. The directory `check-api-coverage` reads as
  "every public item is shown here" would become mostly games, which is how a
  gate that reads it stops meaning anything.
- **A separate repository per prototype now (W3 first).** The right end state
  for a game that ships and the wrong entry cost for a game that might not
  survive an afternoon. It also puts every prototype behind owner-only secrets
  (web-publish.md §7), which makes "start a prototype" a task an agent cannot
  finish alone.
- **A `games/` directory outside the workspace, built by its own scripts.** Keeps
  the engine's build time flat, and gives up the one property that makes living
  in this repo worth it: an engine change that breaks a prototype fails on its
  own PR.
- **Enforce the facade edge by review rather than a tool.** Practices' first
  meta-principle: a rule that lives only in prose is a rule scheduled for
  deletion. This one already existed in prose, in `public-api.md` §4, and this
  ADR is what happens when a manifest finally makes it checkable.
- **Delete retired prototypes instead of an `attic/`.** git keeps them, in the
  sense that a commit somebody can name keeps them. Nobody can name it.
