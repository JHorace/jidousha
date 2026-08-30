# Agent Practices

Practices for keeping this codebase legible, navigable, and safely modifiable by AI agents.
This engine is **agent-developed** and **agent-used**: the practices below serve both the
agents that build the engine and the agents that build games with it.

This document is the *rationale* reference. The enforceable subset lives in `CLAUDE.md`
(always in an agent's context) and in hooks/CI (enforced mechanically). When this document
and CLAUDE.md disagree, CLAUDE.md is wrong — fix it in the same commit.

Status: adopted at project start. Language assumed Rust; formalize in ADR-0001.

---

## The meta-principles

Every practice below derives from these. When a situation isn't covered, derive the answer
from these.

**1. Unenforced scaffolding rots, and rotted scaffolding is worse than none.**
A human treats a stale comment with suspicion; an agent treats it as ground truth. Every
practice therefore names its *enforcement mechanism* — a hook, a CI check, a type, or a
definition-of-done step. A rule that lives only in prose is a rule scheduled for deletion
or mechanization.

**2. Agents read selectively under context pressure.**
A working agent pulls in perhaps 5% of the repo and must pick the right 5%. Everything is
therefore designed for discoverability and summary-first reading: greppable names, fixed-shape
module headers, routing tables, small files. Completeness is worthless if the reader never
finds the file.

**3. The repo is the prompt.**
Every file an agent reads is, functionally, part of its instructions. A human reader
holds a repository at arm's length — they know the comment is old, they can see the
example is from before the refactor, they discount the doc they remember arguing about.
An agent has none of that. It reads the nearest working code and copies it; it reads
prose and acts on it. So the blast radius of rot is *larger* here than in a
human-maintained repo, not smaller, and it points forward: a stale exemplar does not
merely mislead one reader, it seeds the next several files written in this codebase.

Two consequences run through everything below. **Exemplar quality is load-bearing**
(§5.1): the most-copyable code in the repo is the strongest instruction in it, so
"is this still the pattern we want copied?" is a maintenance question with a real
answer, and E0 found the answer to be *no* twice by accident (F-045, F-088 — worked
examples teaching two spellings of one import, in a repo whose first convention is
one way to do everything). And **a document that is wrong is worse than a document
that is missing**, because the agent acts on it: F-055 is a doc comment that
described the wrong method, which the generator carried faithfully into `docs/api/`
and a human then paraphrased into the prose. Nothing was absent at any point in that
chain. §2.5 is how this repo senses that class of failure.

---

## 1. Comments

Comment the things that are **not recoverable from the code**: invariants, cross-file
contracts, units and conventions, and the intent behind surprising decisions. Do not write
narrative "what the code does" comments — they rot fastest and displace useful context.

### 1.1 Structured comment tags

A small, fixed, greppable vocabulary, placed **at the point of relevance** (constraints
stated at the point of temptation get respected; constraints stated elsewhere get missed):

| Tag | Meaning | Must include |
|---|---|---|
| `INVARIANT:` | A condition that must remain true across this code | The condition and *why* |
| `CONTRACT:` | A cross-boundary obligation on callers or callees | Who owes what, when |
| `SAFETY:` | Justification for an `unsafe` block (Rust-idiomatic) | Why the invariants hold |
| `PERF:` | Looks improvable but is deliberate for performance | Benchmark or measurement reference |
| `DELIBERATE:` | Looks wrong/unidiomatic but is intentional | Link to the ADR (`see ADR-00NN`) |

`DELIBERATE:` is the load-bearing tag in an agent-developed codebase: agents have a strong
drive to "clean up" anything surprising. A `DELIBERATE` tag at the site of the surprise is
the effective defense; a rationale buried in a distant doc is not.

No other tags. A growing tag vocabulary is itself scaffolding rot.

*Enforcement:* tag format checked by lint script in CI (`tools/check-tags`); `DELIBERATE:`
without an ADR reference fails CI.

### 1.2 Module headers

Every module (file) opens with a doc comment of fixed shape:

```rust
//! One-sentence purpose.
//!
//! Key types: `Foo`, `BarHandle`.
//! Depends on: `core::alloc`. Must never be depended on by: `platform`.
//! INVARIANT: <any module-wide invariant>
```

This header is what an agent reads when deciding whether the file is relevant. A good header
saves reading the whole file; a missing one forces the read.

*Enforcement:* `#![deny(missing_docs)]` on all crates; header shape spot-checked in review
and by `tools/check-headers`.

### 1.3 Conventions live in types, not comments

Cross-cutting conventions (coordinate handedness, Y direction, angle units, time units,
color space) are stated **once** in `docs/conventions.md` and **echoed in type signatures**
via newtypes. `Radians(f32)` beats a hundred `// in radians` comments and cannot drift.

*Enforcement:* the type system; clippy lint against bare `f32` in public APIs where a
newtype exists (custom lint in `tools/`, aspirational until written).

---

## 2. Documentation

Three layers, organized by *when the content enters an agent's context*:

### 2.1 Always-in-context: `CLAUDE.md`

Ruthlessly small — one to two pages. Contains: build/test/run commands, the five most
important conventions, the routing table ("touching X → read Y first"), the definition of
done, and the never-do list. Its job is **navigation, not knowledge**. The most common
failure mode is CLAUDE.md bloat; anything that can live a layer down, must.

*Enforcement:* hard size cap (150 lines), checked in CI.

### 2.2 Read-on-demand: `docs/internal/` and `docs/adr/`

`docs/internal/<subsystem>.md` — one file per subsystem, written for a reader with **zero
session memory**. Fixed shape: what it does (one paragraph), core data flow, invariants,
how to test it, known sharp edges. Written at subsystem-completion time; updated as part of
the definition of done for any change to that subsystem.

`docs/adr/NNNN-title.md` — short architecture decision records: context, decision,
consequences, alternatives rejected. Written **at decision time**. ADRs are the only durable
defense against a future session re-litigating a settled decision. Every `DELIBERATE:` tag
points at one. ADRs are immutable once accepted; supersede, don't edit. They are
never culled either, so what keeps the pile usable is `docs/adr/INDEX.md` — a row
per record with its current status, landing in the same commit as the record, and
the thing navigation points at (conventions, §Documents).

*Enforcement:* doc-drift hook — if `src/<subsystem>/` changed and `docs/internal/<subsystem>.md`
did not, the hook flags it (warning, not block; the agent must explicitly state "no doc
impact" in the commit message to silence it).

### 2.3 Generated: `docs/api/`

The game-agent-facing public API reference, generated from source doc comments into
**compact files** designed to fit in a game-building agent's context. Never
hand-maintain anything that duplicates code.

It was one file until ADR-0025 split it in two by what the reader is doing:
`jidousha-api.md` is how a game is written and `jidousha-testing.md` is how one is
checked, each with its own budget. The measurement that forced it — 46% of a
game-writing agent's documentation budget spent on verification material — is the
general lesson: a budget protects relevance, not size, so the fix for a full one is
usually a seam rather than a bigger number.

**ADR-0030 applied that lesson a second time**, to the testing document, and the
second application is the one that shows the rule is a rule rather than a story
about one file. `jidousha-controllers.md` is how the *player* inside a check is
written — advice that would be as true of a driving game, and was therefore
spending a seventh of a checking-a-game budget on something that is not checking a
game. Worth knowing before reaching for a bigger number: curation was tried first
and measured, and it recovered 143 tokens on one pass and 20 on the next. At ten
thousand tokens of prose, tightening sentences is noise; the seam is the move.

**ADR-0035 applied it a third time and showed where the rule's edge is.**
`jidousha-capture.md` is how one recorded frame is rendered for real — a distinct
task, done last, by a reader whose check already runs. Two things about the third
application are worth carrying to whatever the fourth one is. It is the first to
move **reference entries** as well as prose, so it needed a rule for which move
(an item goes when no entry outside the moving set names it) and left three
behind that a tidier cluster would have taken, because each is named by an entry
that stays. And it moved the **vocabulary exemption** with the material: the
testing document may no longer name a renderer. Leaving that behind is the half of
a seam that lands unnoticed — the material is gone, the licence to talk about it
is not, and the next paragraph written drifts back into the wrong file.

`docs/api/` is a **product surface** — arguably the most important one. Its quality metric:
can a fresh agent, given only `docs/api/` and `examples/`, produce a working prototype?
It gets evaluated like a product, not proofread like a doc.

*Enforcement:* `tools/gen-api-doc` regenerates them; CI fails if either committed copy
is stale, is over its budget, or names implementation vocabulary.

### 2.4 Two audiences, strict separation

Agents *developing* the engine read `docs/internal/`. Agents *using* the engine read
`docs/api/`. Never mix: internal details leaking into `docs/api/` waste the game agent's
context and invite dependence on non-guaranteed behavior.

### 2.5 The ledgers are the rot sensor

The FINDINGS ledgers — `docs/internal/e0-findings.md` for the engine's own acceptance
runs, `games/<name>/FINDINGS.md` for each game — are not a complaints box. They are the
only instrument this repo has for detecting the failure meta-principle 3 describes, and
an instrument only reads what it is given.

**File a finding when a document misled you, not only when one was missing.** This is
the rule that had to be said out loud, because the reflex is the opposite: a gap feels
like the document's fault and a wrong answer feels like your own. It is the wrong way
round. A missing answer costs a search; a wrong answer costs the search *plus*
everything built on it before it was caught, and it is invisible afterwards because
the workaround looks like a decision. When the fault is a doc that misled you, the
entry says **what you did on its authority** before you found out — that sentence is
what tells a maintainer whether the fix is a wording change or a check that pins the
claim (§5.2's last paragraph is the shape of the check).

`0 findings` is a real answer and is worth stating explicitly, with why it is true. A
finding invented to fill the section is worse than an empty one; a session that reached
for nothing new can say so in a sentence.

**Maintenance is dispatched by the ledger, and it is typed and fenced.** A "let's tidy
up" session is unbounded, and an unbounded session in an agent-developed repo is a
behavior change waiting to be mistaken for housekeeping. So a sanitation pass names one
of four types — **doc-truth audit** (verify a document's checkable claims against the
code) · **exemplar audit** (is the most-copyable code still the pattern we want
copied?) · **dead-weight sweep** (mechanical: clippy, unused deps, orphaned assets and
links) · **history-bleed sweep** (living docs are rewritten state; §Documents in
conventions) — and it names the ledger entries that dispatched it. Judgment passes are
**report-first**: findings written before anything is edited, so what was considered and
left alone is visible. Code-touching passes are **transcript-identical**: replay
determinism (§5.6) makes "this changed nothing" machine-checkable, so it gets checked
rather than claimed. Never judgment and mechanical work in the same session — the diff
stops being reviewable when taste hides inside a lint fix.
`docs/templates/SANITATION.md` is the handoff template; passes close waves, and the
session that closes a wave is the one that asks for the pass.

**No agent approves a pull request, ever.** Approval is the owner's, and an agent
review that ends in an approval is a rubber stamp with a plausible voice. The one
permitted review form is *narrow adversarial verification* — a fenced session that
tries to break one specific claim and whose only output is FINDINGS entries — and it
is warranted only when a wave gate has actually failed. A review nobody's failure
motivated is a review nobody reads.

*Enforcement:* the `make-game` skill's findings step and closing checklist (a session
that skips them violates the skill); the SANITATION template's evidence slot, which
cannot be filled without a ledger entry or a failed gate.

---

## 3. Skills (`.claude/skills/`)

Skills encode **procedures** (multi-step workflows with a known good order). Docs encode
**facts**. Keeping that boundary clean prevents duplication and drift.

**When to write one:** the second time a workflow is performed manually *and* the agent
needed correction either time. Not before — during initial development, workflows aren't
stable, and a premature skill enforces a procedure about to change.

**Expected skills (write when triggered, not preemptively):**

- `add-subsystem` — scaffold module + tests + internal doc + ADR in the canonical shape.
- `run-verification` — run and interpret the headless verification harness.
- `make-game` — the flagship: how a game session uses the engine. Points at
  `docs/api/` and `examples/`; owns both session shapes — a new prototype, and a
  wave or module landing into a game that already runs — plus the findings a
  session owes back (§2.5) and the closing checklist the owner loop runs on.

**Form:** a checklist that points into repo docs rather than restating them. One source of
truth. Skills live in `.claude/skills/` and version with the code, so an engine change and
its skill change land in the same commit.

*Enforcement:* review rule — a skill that restates facts from docs is rejected; it must link.

---

## 4. Repository organization

Everything versioned in the repo. Nothing lives in session memory, chat history, or
external notes.

```
CLAUDE.md            always-in-context router (≤150 lines, CI-enforced)
.claude/
  skills/            procedural workflows (§3)
  hooks/             fmt, clippy, doc-drift, api-doc staleness
  commands/          repeated prompt shortcuts, if any emerge
docs/
  adr/               numbered, immutable decision records
    INDEX.md         what you navigate by; a row per ADR, landing with the ADR
  internal/          per-subsystem contributor docs (fixed shape, §2.2)
  api/               generated game-agent-facing reference (§2.3)
  conventions.md     units, coordinates, naming vocabulary, error style, doc shape
  agent-practices.md this file
  templates/         BLOCKED.md (§6.4) · SANITATION.md handoff templates (§2.5)
examples/            small canonical programs; compiled AND run in CI
tools/
  doctor             environment self-diagnosis (§6.1) — build FIRST
  test               test wrapper: report file, timeouts, failure counter (§6.2)
  verify, gen-api-doc, check-tags, check-headers, …
src/ or crates/      engine code
BLOCKED.md           present only while blocked on a human (§6.4); removed when resolved
```

Hooks are the enforcement layer and deserve real investment: every rule moved from prose
into a hook stops consuming context and starts being unbreakable. Priority order:
format-on-edit, clippy with `-D warnings`, api-doc staleness, doc-drift warning, tag lint.

---

## 5. Code and design practices

### 5.1 Examples are the strongest prompt

Agents pattern-match from working code more reliably than from any documentation. Each
example in `examples/` is minimal, canonical (exactly one way to do each thing), and
CI-tested so it cannot rot. **Shipping any public API includes adding or updating the
example that demonstrates it** — this is in the definition of done.

### 5.2 Tests are the spec

Behavioral tests are how a future session learns what a subsystem is *supposed* to do —
the only intent-description that cannot drift. Test names are sentences
(`sprite_draw_order_follows_z_then_submission`), because test names are what greps well.
Prefer many small tests over few large ones. Property tests where invariants allow.

**Break the thing on purpose and check the tests notice.** A suite is only worth
what it catches, and which checks are vacuous is not reliably guessable by reading
them — E0 run 5 mutated its game seventeen ways, caught all seventeen, and says two
of those only after tightening checks it had written carefully and believed were
thorough (e0-findings.md F-058). The same technique applied to this repository's own
worked example found that "a paddle-sized quad covers this point" passes for a
paddle drawn 45% of its height out of position, because a paddle covers its own
centre wherever it is drawn. That check had survived every review since R3. Two
recurring shapes are worth mutating for specifically: a check that asks whether
*something* is there rather than where its bounds are, and a safety margin that
correct behaviour never exercises — the second is invisible to any amount of
running, so its contract has to be asked directly.

**A sentence in a doc comment is load-bearing and can be asserted.** F-055 is the
first E0 finding caused by a description being *false* rather than absent:
`FrameRecorder::transcript` said "the last frame" and rendered all of them, which
the generator then carried faithfully into `docs/api/` and a human paraphrased into
the prose. No gate over generated summaries catches a true-shaped lie. A test that
pins the claim does, and it is worth writing for any sentence a reader would act on
without checking.

### 5.3 One way to do everything

No convenience overloads, no aliases, no second path "for ergonomics." Every alternative
doubles the ways generated code can diverge from the examples. Fixed verb vocabulary
(recorded in `docs/conventions.md`):

- `create` / `destroy` — object lifetime
- `load` / `unload` — assets
- `get_*` — infallible; `find_*` — returns `Option`; `try_*` — returns `Result`
- No synonyms: never `make`, `new_*` (outside `T::new`), `fetch`, `lookup`, `remove`-vs-`delete` splits

Half the API becomes guessable; guessable is the goal.

### 5.4 Greppability is a design constraint

Unique, searchable symbol names. No macro-generated *public* symbols (grep can't find them;
an API that can't be grepped doesn't exist to an agent). File names match the primary type
they contain. No deep re-export chains that hide a symbol's home.

### 5.5 Errors are written for the repair loop

Agents paste errors back into their own context: error text is documentation delivered at
exactly the right moment. Every engine error states **what happened, the likely cause, and
the fix**:

```
TextureLoad failed: "sprites/player.png" not found under asset root "assets/".
Likely cause: path is relative to the project root, not the asset root.
Fix: use "player.png" if the file is at assets/player.png, or check asset_root config.
```

**Silent failure is banned at the design level.** No no-op fallbacks, no degraded
continues. Debug builds panic loudly; release builds return `Result`. `#[must_use]` on
every `Result`-returning and handle-returning API.

### 5.6 Determinism from day one

Seeded RNG, fixed-timestep option, replayable input recording. Near-impossible to retrofit,
and they are what make agent *self-verification* possible — the property this entire engine
exists to maximize. The verification harness (headless run + structured assertions +
frame/state dumps) is a core subsystem, not tooling.

### 5.7 File sizing

Soft cap ~500 lines per file, so any file is a single cheap read. Split by concept, not by
line count alone. Locality beats maximal DRY: a small amount of duplication that keeps
related behavior in one readable place is preferred over indirection that scatters it.

### 5.8 Dependency budget

Rust dependency trees grow ambiently and each crate costs build time — the core
resource of the agent repair loop. Every **new direct dependency** records, in the
adding commit: what it's for, why not hand-rolled, and its measured `cargo tree`
delta (count of new transitive crates). Preference order: no dependency > tiny
pure-Rust dependency > large pure-Rust dependency > C-linking dependency (the last
needs an ADR, per ADR-0001). CI reports total dependency count so growth is a
visible number in every PR, not an ambient drift.

*Enforcement:* CI dependency-count report; review rule for the commit-message
justification.

### 5.9 Make illegal states unrepresentable

Newtypes for units and IDs, typestate where lifecycle matters (an unloaded asset handle
cannot be drawn), enums over booleans, non-optional fields over "maybe set later." Every
state the type system excludes is a bug class the repair loop never has to enter.

---

## 6. Environment failures and escalation

The root failure mode: an agent cannot natively distinguish *"my code is wrong"* from
*"the world is wrong."* Both arrive as a failed command or unreadable output, and the
agent's prior is that it caused the problem — so it "fixes" code that was never broken,
or flails at infrastructure it cannot repair. The guards: make the distinction detectable
(§6.1–6.2), make stopping a defined behavior (§6.3), make deferring cheap and legitimate
(§6.4), and shrink the failure class structurally (§6.5).

### 6.1 `tools/doctor` — detection

A fast self-diagnosis script, built **before the first engine subsystem**. Checks:
toolchain version vs `rust-toolchain.toml`, required system libraries, network
reachability to crates.io, disk space, git state, headless-graphics capability,
stale lock/target state. Emits a machine-readable verdict:

- `ENV_OK` — environment healthy; the failure is in your code. Go debug it.
- `ENV_FIXABLE: <exact command>` — run the named fix and nothing else.
- `ENV_BROKEN: <description>` — human required. Stop and escalate (§6.4).

Paired rule (in CLAUDE.md): on any build/test failure that is not plainly a compile
error in code just changed, run `tools/doctor` **before attempting any fix**. This
converts the agent's worst guessing game into a lookup.

### 6.2 Two channels for every critical signal

A garbled or truncated terminal must not be able to masquerade as a test failure.
`tools/test` wraps the suite and, in addition to normal output, writes a structured
report — pass/fail counts, failed test names, exit code — to `target/verify/report.json`.

**The report file is ground truth; terminal scrollback is advisory.** If the two
disagree, or the terminal is unreadable, that disagreement *is* the diagnosis: the
tooling channel broke, not the tests.

The wrapper also enforces **timeouts on every phase**, so a hang (e.g. windowing code
trying to open a display in a headless environment) becomes a diagnosable
`TIMEOUT in phase X` instead of a dead terminal.

### 6.3 Circuit breaker — behavior

Detection is not enough; agents are biased toward action. Explicit stop-rules:

1. The same command fails the same way twice after a fix attempt → **stop**. No third
   variation. Run `tools/doctor`, then escalate per §6.4.
2. Infrastructure (non-code) failures get a hard budget of two fix attempts total.
   Code failures get normal debugging.
3. Some failure classes are **never agent-fixable, by decree**: missing system
   dependencies, toolchain installation, network/registry outages, GPU/driver problems,
   permission errors, disk exhaustion. On these the only correct move is escalation.
4. **Forbidden flailing** — never, without explicit human sign-off: delete or
   `#[ignore]` a test to get green; downgrade dependencies; edit `rust-toolchain.toml`
   or CI config to route around a failure; switch to `--offline`; `rm -rf target` more
   than once per incident.

Because prose rules decay under context pressure (meta-principle 1), the stop-rule is
also delivered **in the error channel at the moment of temptation**: `tools/test` counts
consecutive identical failures and on the second prints
*"Second identical failure. Per policy: run tools/doctor, then write BLOCKED.md. Do not
retry."* — the same trick as the `DELIBERATE:` tag. A Claude Code hook may later harden
this further.

### 6.4 `BLOCKED.md` — make deferring cheap and legitimate

Agents resist stopping partly because stopping has no defined form. Give it one: on
escalation, fill `docs/templates/BLOCKED.md` and write it to the repo root — what I was
doing, the exact command, full error text, doctor output, hypotheses ruled out, and the
specific thing a human should do. Then continue any workable unrelated task, or end the
session cleanly.

Stated policy, verbatim in CLAUDE.md: **writing a good BLOCKED.md for an environment
issue is a successful outcome, not a failure.** A five-minute human read of a well-formed
handoff beats an hour of token burn. In an interactive session, also ask the human
directly at this point; BLOCKED.md covers the unattended case and preserves state across
sessions either way. Delete BLOCKED.md in the commit that resolves the blockage.

### 6.5 Shrink the surface structurally

The best environment failure is one that cannot occur.

- Commit `Cargo.lock`; pin the toolchain in `rust-toolchain.toml`.
- Prefer pure-Rust dependencies over anything linking system C libraries. This is a
  standing evaluation criterion in dependency ADRs.
- **Headless-first testing**: the default `cargo test` path never opens a window and
  never touches a real GPU, so the display/driver failure class barely exists. (This is
  the same harness that serves determinism and self-verification, §5.6.)
- Document every unavoidable system requirement in `tools/doctor`, so the doctor's
  knowledge is the single source of truth for "what this repo needs from the machine."

*Enforcement:* doctor and the test wrapper are mechanical. The circuit breaker is policy
plus the in-channel nudge (§6.3); hook-based hardening is a follow-up once the wrapper
exists.

---

## 7. Definition of done

A change is done when all of the following hold. (Mirrored in CLAUDE.md; enforced by hooks
where mechanizable.)

1. `cargo fmt` clean, `cargo clippy -- -D warnings` clean, all tests pass.
2. New/changed behavior has a behavioral test whose name states the behavior.
3. Public API changes: example added/updated, `docs/api/` regenerated.
4. Subsystem docs (`docs/internal/`) updated, or "no doc impact" stated in the commit.
5. Any deliberate oddity carries a `DELIBERATE:` tag pointing at an ADR (write the ADR if
   the decision is new).
6. No new warnings, no `unwrap()` outside tests, no silent fallback paths.
