# ADR-0030: The controller advice becomes a third document

Status: accepted · 2026-08-20 · **extends ADR-0025**

> **`docs/api/jidousha-controllers.md`, prose only, budget 5,000.** The material
> about driving a game you cannot watch leaves `jidousha-testing.md`, which drops
> from 14,665 tokens of 15,000 to 12,455. ADR-0025's rule is unchanged and this
> is its second application, one level down.

## Context

ADR-0025 split one document in two when the testing half reached 46% of it, on
the rule that **a document's budget belongs to its reader** and everything in it
has to be relevant to what that reader is doing. The same pressure has now built
inside the testing half.

`jidousha-testing.md` stood at **14,665 tokens of 15,000** after run 8's triage —
335 of headroom against a document that has absorbed six findings a run. The
controller material inside it was ~2,200 tokens, about a seventh of the file, and
it is the part that keeps growing: F-037, F-047, F-056, F-074, F-080, F-082 and
F-100, seven findings across six acceptance runs, every one of them answered with
another paragraph in the same place.

**Curation was tried first and does not work at this scale.** Run 8's triage
tightened four passages to pay for its additions and recovered 143 tokens; a
second pass over three more passages recovered **20**. The file is ten thousand
tokens of prose, and sentence-level economy is noise against it. The only moves
left were structural.

**And the material fails ADR-0025's own test.** Nothing in it is about this
engine. "A driver that brakes for every corner never finds the top speed" is as
true of a driving game as of Pong, and `crates/jidousha/examples/slalom/` — run
4's lever, spent after run 8 — now demonstrates the whole of it in a game with no
opponent, no bounce and no rally. A reader checking whether their game drew the
right quads is not the reader deciding whether their rollout controller is
optimising over positions its paddle cannot occupy.

## Decision

**A third generated document, `docs/api/jidousha-controllers.md`, split by what
its reader is doing.** Three readers, three files, three budgets:

| document | the reader | budget |
|---|---|---|
| `jidousha-api.md` | writing a game that does not exist yet | 25,000 |
| `jidousha-testing.md` | checking a game that does | 15,000 |
| `jidousha-controllers.md` | making that check's player good enough to measure the game | **5,000** |

- **Prose only, no reference section.** A controller is written with the game's
  own vocabulary plus `InputScript` and `SnapshotBuilder`, and both already have
  reference entries in the testing document. A second copy would be a second
  place to keep right, which is the failure `Document` exists to prevent.
- **The mechanism stays where its reference is.** How to *send* a keypress —
  `InputScript`, `SnapshotBuilder`, events not states, the empty snapshot for an
  idle player — remains in the testing document. What moves is the strategy:
  how to make the player good. That seam is the difference between "how do I
  press a key" and "which key should I press".
- **Budget 5,000 against 2,661 of content.** Deliberately not generous. This is
  where controller findings land now, and a budget that started with room for
  everything would not be a budget.
- **`allowed=()`**, unlike the testing document: a controller decides what to
  press and never draws, so no renderer is nameable here.

## Rationale

The alternative readings were considered and each fails on something concrete.

Raising `TESTING_BUDGET` treats the symptom. The budget is not a quota, it is the
claim that everything in the file is relevant to one reader — and the controller
material demonstrably is not, so a bigger number would make the document worse in
exactly the way ADR-0025 was written to prevent.

Moving it to the `make-game` skill is where §7 of `e0-findings.md` eventually
sends it, and it cannot go there yet: the skill is written after E0 passes, and an
E0 run may not read it. A run needs this advice *now* to write its `--verify`
controller, so removing it from the readable surface would make the next run worse
in a way that would look like a docs finding and would not be one.

Deleting it is not available. Seven findings say it is load-bearing.

## Consequences

- `jidousha-testing.md` drops to **12,455 tokens of 15,000**, and
  `jidousha-controllers.md` sits at 2,661 of 5,000. Between them there is room
  for several runs' worth of findings for the first time since run 6.
- **Discoverability is the price, and it is paid explicitly.** The game document
  names the third file twice, the testing document once, and the controllers
  document names both of the others so a reader landing on it first can leave.
  `test_every_document_is_reachable_from_the_game_document` asserts all of it.
- `test_the_controllers_document_holds_the_controller_material` asserts the
  material **moved** rather than being copied — four load-bearing phrases must be
  in the new file and absent from the old one. A split that duplicates is a split
  that will drift.
- **The E0 prompt's may-read list gains a third file, and its ledger gains a
  row.** By the ledger's own argument this makes a run *harder* rather than
  easier — one more file to find — so it does not invalidate the streak. The
  streak was restarted to zero by ADR-0029 in the same commit series anyway,
  which is why both changes land together.
- `CLAUDE.md`'s routing table, its `tools/gen-api-doc` line and
  `docs/internal/public-api.md` §4 all say "two documents" and all change.

## Alternatives rejected

- **Raise `TESTING_BUDGET` to 18,000.** Cheapest, and it concedes the argument
  the budget exists to make. It also does not stop: the controller material would
  keep growing and the next conversation would be 20,000.
- **Split by *when* rather than by *what*** — a "first `--verify`" file and an
  "advanced" one. Sounds tidier and cuts across the actual seam: the advanced
  half would still mix engine-specific assertions with genre-neutral advice.
- **Fold the material into `examples/slalom` alone** and delete the prose. The
  example is a worked instance and a reader who has not been told *why* reads it
  as one game's arbitrary choices. Run 8's lattice finding (F-100) is exactly
  what happens when a technique is presented without its condition.
