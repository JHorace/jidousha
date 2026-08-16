# E0 findings — what building a game with this engine actually cost

Status: **no runs yet.** The harness is `docs/internal/e0-prompt.md`; the
milestone is implementation-plan.md §3.

E0 is the project's definition of working: a fresh Claude Code session, given
only `docs/api/jidousha-api.md` and `crates/jidousha/examples/`, builds a
playable Pong. This file is where its frictions get root-caused.

---

## 1. The rule

**Every friction is an engine bug or a docs gap until proven otherwise.**

The tempting reading of an E0 failure is "the prompt was unclear" or "the agent
made a mistake". Sometimes that is true. It is the *last* explanation to reach
for, because it is the one that requires nothing to change, and a milestone
whose failures cost nothing is a milestone that measures nothing.

Each finding is classified as exactly one of:

| Class | Meaning | What it costs |
|---|---|---|
| `engine` | The engine is missing something, or does something surprising. | A code change, and usually an ADR if the surprise was deliberate. |
| `docs` | The API document does not say something a game author needs. | A change to the facade's doc comments or `tools/api-doc/` prose, then `tools/gen-api-doc`. |
| `author` | The run made an ordinary mistake the document does cover. | Nothing — but the finding stays, because three `author` findings on the same topic is a `docs` finding wearing a hat. |

A finding classified `author` needs a quote from `docs/api/jidousha-api.md`
showing where the answer already was. Without that quote it is not an `author`
finding.

## 2. The bar

E0 passes when **two consecutive runs produce no new `engine` or `docs`
findings**. Not "no findings" — an `author` finding is allowed in a passing run,
and the second clean run is what distinguishes a fixed engine from a lucky one.

A run whose transcript shows a read under `crates/*/src/`, `docs/internal/` or
`docs/adr/` is void and does not count towards the two. Void runs are logged
below anyway: a restriction that is hard to honor is itself a finding about the
prompt.

## 3. Run log

| Run | Date | Outcome | New `engine` | New `docs` | New `author` | Notes |
|---|---|---|---|---|---|---|
| — | — | no runs yet | — | — | — | — |

## 4. Findings

None yet. Each finding gets a subsection in this shape:

```
### F-001 — <one line, in the game author's words>

Class: engine | docs | author · Run: <n> · Fixed in: <commit or "open">

**What the run did.** The game code or the question, quoted from `E0-NOTES.md`.

**What happened.** The error, the wrong behaviour, or the dead end.

**Root cause.** Why the engine or the document made this the likely outcome.
Not "the agent should have known" — what made not-knowing reasonable.

**Fix.** What changed, and why that is the right change rather than a note
telling future readers to be careful.
```

## 5. What this file feeds

The `make-game` skill (agent-practices §3) is written from E0's findings after
it passes. A friction that recurs across runs and cannot be designed away is
exactly what a skill is for — and one that *was* designed away must not appear
in the skill at all, because a skill that restates a fixed problem is how the
fix gets undone later.
