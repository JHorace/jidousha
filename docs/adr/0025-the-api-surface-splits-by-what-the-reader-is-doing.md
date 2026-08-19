# ADR-0025: The generated API surface splits by what the reader is doing

Status: accepted · 2026-08-19

> **Two documents, not one bigger budget.** `docs/api/jidousha-api.md` is how a
> game is written; `docs/api/jidousha-testing.md` is how one is checked. Nothing
> was added to or removed from the public surface — the same items are
> documented, in two files chosen by the reader's task.

## Context

`docs/api/` had one file and one budget: 25,000 tokens, whose stated purpose in
public-api.md §4 is that "the whole surface fits in a game-writing agent's
context alongside the game itself". After F-066 the file stood at ~24,130 tokens
and the budget had become the binding constraint on ordinary work.

Measuring what was in it is what settled this:

| Part | ~tokens | Share |
|---|---:|---:|
| Reference — the six game groups (App, ECS, Math, Render, Assets, Input) | 8,250 | 34% |
| Reference — `jidousha::testing` | 4,598 | 19% |
| *Testing your game* prose | 6,470 | 27% |
| Concepts · Quickstart · Conventions | 4,703 | 20% |

**46% of the document was about verifying a game rather than writing one**, and
the two halves were not even adjacent — the testing reference sat inside
`## Reference` and the prose was a top-level section three thousand lines away.
The `jidousha::testing` block alone was larger than App, ECS, Render and Assets
put together.

Three things were already blocked behind the ceiling, all of them wanted:

- **The "tiny example" third of §4's own spec.** Entries carry a signature and a
  one-liner and no example. §4 estimates the ~39 existing doctests at about 5k
  tokens, which did not fit.
- **The summary-completeness check**, which §4 calls the top of the generator's
  queue and blames for four of E0 run 4's sixteen findings. Its fix means
  printing *more* of a doc comment when the body carries a fact the summary
  drops.
- **Every deferred subsystem.** All milestones in all four subsystem docs are
  green, so ~24k is v1 *complete*, not v1 partway. Audio, gamepads, structured
  data, atlas packing, render-to-texture and 3D are all still to come, at roughly
  1,100 tokens per subsystem group on the going rate.

There was also a smaller, sharper signal. `gen-api-doc` cut the entire
`### Testing (jidousha::testing)` block out of its forbidden-vocabulary check,
because "a golden image has to be drawn by something" and the entry has to name
the renderer. That carve-out worked for reference entries and failed for prose:
F-066 needed a capture recipe in *Testing your game*, could not write one without
naming `WgpuBackend` and importing `RenderBackend`, and shipped words and a
pointer to an example instead. **The seam was already there, discovered by
necessity and papered over inside one file.**

## Decision

Generate two documents, split by what the reader is doing.

- **`docs/api/jidousha-api.md`** — Quickstart, Concepts, Reference (the six game
  groups), Conventions digest. Read while the game is being written. Budget
  25,000 tokens; currently ~13,300.
- **`docs/api/jidousha-testing.md`** — *Testing your game*, then the
  `jidousha::testing` reference. Read once there is a game to check. Budget
  15,000 tokens; currently ~11,600.

The budgets differ because the readers are in different states. The game
document competes for context with a game that does not exist yet. The testing
document is read beside a game that is finished and already understood — a later
and much cheaper moment.

**The vocabulary CONTRACT becomes per-document, and gets tighter.** The game
document is checked entire, with no exemption of any kind. The testing document
may use exactly three words the game document may not — `wgpu`, `RenderBackend`,
`FramePlan` (`TESTING_VOCABULARY`) — because a picture has to be drawn by
something. Everything else in `FORBIDDEN` applies to both: an internal crate
name or a pointer into `docs/internal/` is as refused in one as in the other.
That is strictly stronger than what it replaces, which excused a whole section
from every rule at once.

**What did not change.** The public surface: the same facade `pub use` lists
generate the same entries, `tools/check-api-coverage` is untouched, and no item
gained or lost documentation. This is where the material sits, not what it says.

## Consequences

- The capture recipe F-066 could not write is now in *Testing your game* as
  compiling code rather than as prose and a pointer.
- Both queued generator improvements fit. Tiny examples (~5k) land inside the
  game document's ~11.7k of headroom; richer summaries have room in both.
- `Document` carries a path, a budget and a vocabulary exception, and the budget,
  vocabulary and staleness checks are each written once and applied to a list. A
  second copy of the staleness check is exactly the drift F-016 was.
- **The cost is discoverability**, and it is the only real one: an agent that
  does not know the second file exists will not find it. Paid three ways — the
  game document names it in its header, in the Reference group where the testing
  signatures used to be, and in the `## Testing your game` section where the
  prose used to be; `docs/api/` was already a *directory* in CLAUDE.md's routing
  table, so no instruction changed; and `e0-prompt.md`'s may-read list names both
  files. `test_the_game_document_points_at_the_testing_document` pins all three
  pointers.
- **E0 measures whether this worked.** Run 6 is the first run to meet a split
  surface, and "did it find the testing document" is now a question the friction
  log will answer. A run that writes a `--verify` mode without ever opening
  `jidousha-testing.md` is evidence the pointers are not enough.

## Alternatives considered

**Raise the budget to 40k and keep one file.** Rejected as treating the symptom.
By the budget's literal wording — fitting in context beside the game — 25k is
conservative today and could be raised. But the property that makes this document
work is that everything in it is relevant to what the reader is doing, which is
what F-001 cost the project when it was false in the other direction. A bigger
number keeps a game-writing agent spending 46% of its documentation budget on a
job it is not doing, and removes the pressure that produced this measurement.
Raising a ceiling is still available later, deliberately, now that each number
protects one reader.

**Split the Reference by subsystem**, one file per group. Rejected: it fragments
exactly what agents most need to scan, and the groups are small (755–2,657
tokens). Size is not a seam; task is.

**Trim the testing prose.** It is the second-largest item and it grew ~900 tokens
in the commit before this one. Rejected as the primary answer — F-058, F-061 and
F-066 all landed there and it is earned — but the split puts the trimming
pressure on the document that owns the prose instead of on the game surface.
