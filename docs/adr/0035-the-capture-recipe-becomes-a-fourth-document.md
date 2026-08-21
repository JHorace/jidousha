# ADR-0035: Taking a picture becomes a fourth document, and reference entries move with it

Status: accepted · 2026-08-20 · **ADR-0025's rule a third time, and the first split to move a reference**

> **`docs/api/jidousha-capture.md`.** The capture recipe and the nine reference
> entries only it reaches leave `jidousha-testing.md`, which drops from 13,842
> tokens to 12,355 — its lowest since ADR-0030. The testing document stops naming
> a renderer at all. Budgets are unchanged; this is the answer ADR-0034 named and
> gave a trigger, brought forward by one run because the machinery and the prompt
> change are the same work whenever they land and both must land *between* runs.

## Context

ADR-0034 wrote the trigger down: when `jidousha-testing.md` crosses 14,000 again,
the capture reference cluster moves. The document sat at 13,843 with novel
findings arriving at three a run and the prose costing roughly 200 tokens each —
so the trigger was one run away, and the run in question is the one that would
have hit it *during* a triage.

**That is the wrong moment for two reasons and both are structural.** The budget
is a CI gate, so overflow turns the build red while somebody is already fixing
something else. And this split changes `e0-prompt.md`'s may-read list, which
`e0-prompt.md`'s own ledger says makes runs incomparable if it happens mid-run.
Doing it now costs nothing that doing it later would not, and it is the last
moment at which it is free.

**The cut is the one ADR-0025's rule gives.** Split by what the reader is doing:
taking a picture is a distinct task, done last, by somebody whose `--verify` mode
already runs and asserts. A game with no capture path passes every check the other
three documents ask for.

## Decision

**Four documents.** `jidousha-api.md` (writing a game, 25k), `jidousha-testing.md`
(checking one, 15k), `jidousha-capture.md` (rendering one frame of it, **4k**) and
`jidousha-controllers.md` (driving the check's player, 5k).

**This is the first split to move reference entries**, and the rule for which move
is stated rather than left to taste: **an item goes to the capture document when
no entry outside that set names it.** Nine qualify — `WgpuBackend`,
`RenderBackend`, `RenderError`, `RawImage`, `TextureTable`, `FONT_TEXTURE`,
`create_builtin_textures`, `upload_ready_textures`, `encode_png`.

**Three that look like they belong and do not**: `BackendTextureId`, `FramePlan`
and `PhysicalSize` are each named by an entry that stays —
`FrameRecorder::font_texture`, `FrameRecord::plan`, `FrameRecorder::new`. Moving
one would leave the testing document naming a type it does not define, which is
F-017 exactly. The capture document borrows them and says so in its header, the
way the controllers document borrows `InputScript` and `SnapshotBuilder`
(ADR-0030).

**The testing document stops naming a renderer.** Its vocabulary exemption was
`("wgpu", "RenderBackend", "FramePlan")` and is now `("FramePlan",)` — a plan's
`clear_color` is read without rendering anything. The capture document takes the
other two. This is the half of a split that is easy to leave behind: move the
recipe out, keep the exemption, and a backend drifts back into the wrong document
with nothing to say so.

## Rationale

**Prose alone would not have been enough.** ADR-0034's rule moved the capture
*path* into `prototype_kit/capture.rs` and recovered 546 tokens; this moves what
is left plus the entries and recovers 1,487 more. The reference was two thirds of
what the capture material cost, and no destination rule reaches it — a reference
entry exists because the facade exports the item, not because a paragraph chose
to include it. That is why the trigger existed.

**It makes a run harder, which is the safe direction.** `e0-prompt.md`'s ledger
requires that of any prompt change: a fourth file is one more thing a run must
find. Both previous splits were argued this way and the streak survived both.

**The risk is specific and worth naming.** The capture is the instrument that
found two faults nothing else could — a banner reading `YOU WINS 5 - 2`, and a
second line drawn through both paddles and well inside the camera. A run that
never opens the fourth file ships those. Three things are aimed at that: the game
document advertises it, the testing document points at it where the capture
material used to be, and the capture document's own first paragraph leads with
what it has caught rather than with what it does. **§6 should watch for it**: if
run 11 writes no capture path, that is a finding about the split rather than about
the run, and the answer is more pointers rather than fewer documents.

## Consequences

- `jidousha-testing.md` at **12,355 of 15,000**, its lowest since ADR-0030 landed;
  `jidousha-capture.md` at 2,035 of 4,000. At the floor rate that is roughly four
  runs of headroom rather than one.
- **`Document` learns to route reference entries**, which it could not before —
  `CAPTURE_ITEMS` plus a complement filter in `render_testing`. ADR-0025's note
  that "a third document costs those checks nothing" holds for a fourth, and now
  holds for one that carries a reference.
- **`check-api-coverage` reads all four documents**, not the testing one. It was
  written a commit earlier against a single file and would have called every moved
  item unreachable the moment this landed — which it did, and which is why the
  reachability question is asked across the surface rather than per file.
- Five self-tests pin the ways this can be landed half-done: an item in both
  documents or in neither, a borrowed type moved by mistake, the vocabulary
  exemption left behind, a document nobody points at.
- `e0-prompt.md` gains a ledger row and a fourth line in its may-read list. The
  run-11 branch has not started, so no run is invalidated.

## Alternatives rejected

- **Wait for the trigger.** ADR-0034 set it at 14,000 and the document was at
  13,843. Waiting buys nothing and spends the one property that makes this cheap:
  that no run is in flight.
- **Move the prose and leave the reference.** What ADR-0034's rule can reach on
  its own, and it recovers a third of the material. The nine entries are two
  thirds of what capture costs and they follow no paragraph.
- **Move `BackendTextureId`, `FramePlan` and `PhysicalSize` too**, for a tidier
  cluster. Each is named by an entry that stays, so it opens an F-017 hole in the
  document that keeps the namer. Tidiness on one side of a split is a hole on the
  other.
- **Raise the testing budget instead.** Foreclosed by ADR-0025, and the reason is
  unchanged: the budget is the point, because the document has to fit in a
  game-writing agent's context beside the game.
