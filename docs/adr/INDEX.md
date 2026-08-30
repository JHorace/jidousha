# ADR index

Every architecture decision record, current status first. **This is what to
navigate by** — `docs/adr/` is a pile of forty-plus filenames and not a table of
contents (conventions, §Documents).

ADRs are never culled. An accepted one is immutable; a decision that moves is
**superseded** by a new record, and the two carry matching status marks. So a
superseded ADR stays in this index, marked — it is the reason the current
decision reads the way it does, and deleting it is how a settled question gets
re-litigated two years later.

**A new ADR's row lands in the same commit as the ADR.** That is a
definition-of-done step and it is not mechanized, which makes this file the
repo's most rot-prone artifact by its own meta-principle: if a row here
disagrees with the file it names, the file is right and this index is a bug.


## By topic

- **Language, scope and platform** — 0001 · 0004 · 0005 · 0037
- **Architecture and the ECS** — 0002 · 0006 · 0007 · 0013 · 0039
- **Rendering and the backend boundary** — 0003 · 0008 · 0015 · 0016 · 0018 · 0020 · 0021 · 0024
- **Coordinates, math and units** — 0009 · 0010 · 0012
- **Assets and text** — 0011 · 0040 · 0042
- **Input, snapshots and recording** — 0014 · 0017 · 0019 · 0023 · 0043
- **Frame pacing** — 0041
- **Checking a game — the verification surface** — 0022 · 0026 · 0027 · 0028 · 0031 · 0032 · 0033
- **The documentation product** — 0025 · 0030 · 0034 · 0035
- **Milestones and prototypes** — 0029 · 0036 · 0038

## Every record

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-language-and-scope.md) | Language and scope | accepted · 2026-08-15 |
| [0002](0002-full-ecs-architecture.md) | Full ECS architecture | accepted · 2026-08-15 |
| [0003](0003-render-backend-wgpu-then-ash.md) | Render backend — wgpu now, ash later | accepted · 2026-08-15 |
| [0004](0004-windowing-winit.md) | Windowing — winit | accepted · 2026-08-15 |
| [0005](0005-platform-targets.md) | Platform targets — Linux, Windows, Web; Android later | accepted · 2026-08-15 |
| [0006](0006-custom-ecs.md) | ECS implementation — custom, from scratch | accepted · 2026-08-15 |
| [0007](0007-plain-function-systems.md) | Systems are plain `fn(&mut World)` — no parameter extraction | accepted · 2026-08-15 |
| [0008](0008-type-enforced-draw-immutability.md) | Draw-phase immutability is type-enforced | accepted · 2026-08-15 |
| [0009](0009-math-glam-deterministic-trig.md) | Math — glam, with engine-owned deterministic trig | accepted · 2026-08-15 |
| [0010](0010-y-down-right-handed-coordinates.md) | World coordinates are Y-down, right-handed (Vulkan-NDC-aligned) | accepted · 2026-08-15 |
| [0011](0011-poll-based-assets-no-async-runtime.md) | Asset API is poll-based — no async runtime, no async/await in public API | accepted · 2026-08-15 |
| [0012](0012-one-constructor-no-convenience-default.md) | One constructor per type; `Default` only for meaningful defaults | accepted · 2026-08-15 |
| [0013](0013-query-access-through-exclusive-borrow.md) | Query access through the borrow checker, not runtime borrow flags | accepted · 2026-08-15 |
| [0014](0014-hand-written-snapshot-encoding-no-serde.md) | Recording formats are hand-written byte encodings, not `serde` | accepted · 2026-08-16 |
| [0015](0015-draw-submission-vocabulary-in-core.md) | The draw submission vocabulary lives in core, named by opaque texture ids | accepted · 2026-08-16 |
| [0016](0016-texels-move-to-the-gpu.md) | Decoded texels move to the GPU and leave the asset store | accepted · 2026-08-16 |
| [0017](0017-pointer-world-position-is-not-a-snapshot-field.md) | The pointer's world position is not a snapshot field | accepted · 2026-08-16 |
| [0018](0018-text-carries-its-depth-in-its-style.md) | Text carries its depth in its style, not as a trailing argument | accepted · 2026-08-16 |
| [0019](0019-closed-loop-input-goes-through-the-builder.md) | A closed-loop test builds input with `SnapshotBuilder`, not a second `InputSnapshot` constructor | accepted · 2026-08-17 |
| [0020](0020-circle-expansion-is-documented-not-queryable.md) | A circle's expansion is documented, not queryable | accepted · 2026-08-18 |
| [0021](0021-visible-bounds-returns-a-rect.md) | `Camera::visible_bounds` returns a `Rect` | accepted · 2026-08-18 |
| [0022](0022-sweep-and-inflate-stay-out-of-v1.md) | Swept collision and `Rect::inflate` stay out of v1 | accepted · 2026-08-18 |
| [0023](0023-frame-recorder-retention-and-borrows.md) | What `FrameRecorder` hands back, and how long it keeps it | accepted · 2026-08-18 |
| [0024](0024-draw-order-is-observable-depth-is-not.md) | A recorded frame shows draw *order*, not the depth that produced it | accepted · 2026-08-18 |
| [0025](0025-the-api-surface-splits-by-what-the-reader-is-doing.md) | The generated API surface splits by what the reader is doing | accepted · 2026-08-19 |
| [0026](0026-prototype-kit-drives-its-own-backend.md) | `prototype_kit`'s verify drives its own backend; `FrameRecorder` is still the one way | **superseded by ADR-0028** · 2026-08-19 |
| [0027](0027-the-controller-self-check-is-a-shape-not-a-type.md) | The controller self-check is a shape the document names, not a type the engine ships | accepted · 2026-08-19 |
| [0028](0028-the-two-backend-comparison-moves-out-of-the-example.md) | The two-backend comparison moves into a test; `prototype_kit` uses `FrameRecorder` | accepted · 2026-08-19 · **supersedes ADR-0026** |
| [0029](0029-the-e0-bar-counts-novel-findings.md) | E0's bar counts *novel* findings; a re-tread does not reset the streak | **superseded by ADR-0036** · 2026-08-20 |
| [0030](0030-the-controller-advice-becomes-a-third-document.md) | The controller advice becomes a third document | accepted · 2026-08-20 · **extends ADR-0025** |
| [0031](0031-headless-sim-does-not-fork.md) | A `HeadlessSim` does not fork; a check rolls the game's own functions forward | accepted · 2026-08-20 · **extends ADR-0022's reasoning to the check surface** |
| [0032](0032-the-check-surface-gets-a-fold.md) | The check surface gets `find_bounds`; the game surface does not get `Rect::union` | accepted · 2026-08-20 · **the other side of ADR-0022's line** |
| [0033](0033-tunable-numbers-live-in-a-resource.md) | A game that expects to be tuned puts its numbers in a resource; nothing is added to configure one from outside | accepted · 2026-08-20 · **extends ADR-0031's "simulate rather than solve" to the sweep** |
| [0034](0034-a-findings-default-home-is-not-the-prose.md) | A docs finding's default home is the reference or a worked example; prose is for rules | accepted · 2026-08-20 · **the budget answer ADR-0025 forecloses raising and ADR-0030 bought one split's worth of time for** |
| [0035](0035-the-capture-recipe-becomes-a-fourth-document.md) | Taking a picture becomes a fourth document, and reference entries move with it | accepted · 2026-08-20 · **ADR-0025's rule a third time, and the first split to move a reference** |
| [0036](0036-e0-closes-on-substance-rather-than-on-its-bar.md) | E0 closes on substance rather than on its bar | accepted · 2026-08-21 · **supersedes ADR-0029; the milestone ends without meeting the condition it stated** |
| [0037](0037-web-publish-cloudflare-workers.md) | Web builds auto-publish to Cloudflare Workers static assets | accepted · 2026-08-16 |
| [0038](0038-prototypes-are-workspace-members-on-main.md) | Prototypes are workspace members on `main` | accepted · 2026-08-22 |
| [0039](0039-a-read-only-projection-both-phases-can-read.md) | `World::view` — one reader for a projection both phases need | accepted · 2026-08-22 |
| [0040](0040-a-game-crate-owns-an-asset-root.md) | A game crate owns an asset root, and `dist/<name>/` is repository-shaped | accepted · 2026-08-23 |
| [0041](0041-a-per-tick-driver-reports-a-full-alpha.md) | a driver that draws once per tick reports `alpha == 1.0` | accepted · 2026-08-23 |
| [0042](0042-a-typeface-is-an-asset-a-size-is-an-atlas-and-a-measurement-is-an-api.md) | A typeface is an asset, a size is an atlas, and a measurement is part of the API | accepted · 2026-08-29 |
| [0043](0043-touch-is-snapshot-data-and-the-first-finger-is-the-cursor.md) | touch is snapshot data, and the first finger is the cursor | accepted · 2026-08-29 |

## The two superseded records, and by what

- **ADR-0026** → **ADR-0028**. `prototype_kit`'s verify drove its own backend;
  the two-backend comparison moved into a test and the worked example went back
  to `FrameRecorder`, so the example teaches the one way again.
- **ADR-0029** → **ADR-0036**. E0's bar counted novel findings and would not
  have closed on that condition; the milestone closes on substance instead.

Neither original is wrong about the moment it was written, which is why both are
still here.
