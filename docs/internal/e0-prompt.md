# E0 — the acceptance prompt

The text below is what a **fresh** Claude Code session is given for an E0 run
(implementation-plan.md §3). It is checked in so that every repeat uses the same
prompt: if the wording changes between runs, the runs are not comparable and
"passed clean twice in a row" means nothing.

This file lives in `docs/internal/` deliberately — it is maintainers' harness,
and the E0 session must never read it. Paste the block, do not point at the file.

## Prompt revisions

The paragraph above says the prompt is checked in so that repeats are
comparable. It has changed twice, so the claim needs a ledger rather than
assertion — "passed clean twice in a row" only means something if the two runs
were asked the same question.

| From run | What changed | Why |
|---|---|---|
| 1 | — | Original. |
| 3 | Friction log moves from `E0-NOTES.md` at the root to `docs/e0/run-N.md`, chosen by the author; `docs/e0/` added to the may-not-read list. | Run 2 was pointed at run 1's file and read it, so it knew the timestep and three key names before opening the API document (F-020). |
| 3 | *Before starting a run* deletes the previous run's `pong/` and its `tools/test` registrations; step 6 puts the registrations back. | The previous run's finished game sat inside `crates/jidousha/examples/`, which is on the **allowed** list — a complete worked solution the next author could read without breaking a rule (F-020). |
| 6 | The may-read list names two API documents instead of one: `docs/api/jidousha-api.md` and `docs/api/jidousha-testing.md`. | The surface split by what the reader is doing (ADR-0025). The material is unchanged and both files were always inside the `docs/api/` the list already allowed; naming them is so a run does not have to guess that the second exists. |

No change alters what the run is asked to *build* or what it may read of
the engine, so runs 1–2 and 3+ remain comparable on the thing being measured:
whether `docs/api/` is enough on its own. The first two changes remove
information the earlier runs had and should not have had, which makes later runs
strictly harder — the safe direction for a bar to move. **Any future change that
makes a run *easier* invalidates the streak and restarts the count.**

**The split does not make a run easier, and the reasoning is worth writing down
rather than asserting.** Not one sentence of guidance was added, removed or
softened by it — the same prose and the same reference entries, in two files
chosen by task. What changed is that a run must now find a second file, which is
a hazard rather than a help: a run that misses it writes its `--verify` mode
without the testing reference, which is *harder* than run 5 had it. So the streak
stands. The honest risk runs the other way, and §6 should watch for it — if run 6
never opens `jidousha-testing.md`, the three pointers into it are not enough and
that is a finding about the split, not about the run.

---

## Before starting a run

1. `git checkout -b e0/attempt-<n>` from the current default branch.
2. **Delete the previous run's game and de-register it**, in one commit:
   `git rm -r crates/jidousha/examples/pong/`, then remove `pong` from
   `WINDOWED_EXAMPLES` and `VERIFIABLE_EXAMPLES` in `tools/test`.

   The deletion is not tidying, it is the measurement.
   `crates/jidousha/examples/` is on the run's *allowed* list, so a previous
   run's finished Pong sitting in it is a complete worked solution the next
   author may read without breaking a single rule — and the one they would reach
   for first. It stays in the default branch's history for diffing.

   Both halves, because `test_the_windowed_list_names_examples_that_exist`
   fails on a registered example that is not there, and it is right to: a stale
   name would otherwise silently start running a windowed example headlessly.
   Checked — the game alone leaves `tools/test` red, the game and the two
   entries together leave it green.
3. Confirm the working tree is clean and `tools/test` passes, so anything the
   run breaks is the run's.
4. Start a **new session**. Not a continuation, not a compaction — a session
   with no memory of the engine's internals. That is the entire instrument.
5. Paste the prompt below verbatim, filling in nothing: it is self-contained on
   purpose, so that two runs cannot differ by what a maintainer typed around it.

## After the run

1. Read the session's transcript and check the restriction was honored — no
   reads under `crates/*/src/`, `docs/internal/`, or `docs/adr/`. A breach
   invalidates the run; note it and start again.
2. **Play it.** `cargo run -p jidousha --example pong` in a window, and
   `tools/serve-web pong` in a browser. "Playable" is not something a script can
   assert, which is why the milestone asks a person: a Pong whose ball passes
   through the paddle satisfies every assertion an agent would think to write.
3. Take the run's `docs/e0/run-N.md` and root-cause each entry into
   `docs/internal/e0-findings.md`. **Every friction is an engine or docs bug
   until proven otherwise** — that is the rule the milestone turns on.
   **One file per run, and the author writes only their own.** Run 2 was told to
   write into run 1's file, so it read run 1's findings first and knew the
   timestep and three key names before it opened the API document. Its
   conclusions survived, because all three facts had genuinely landed in the
   document — but "the run guessed at nothing" is weaker evidence when the run
   was handed the answers (e0-findings.md F-020).
4. Fix what the findings say to fix. Then run E0 again, fresh.
5. E0 passes when two consecutive runs produce no new findings of the
   engine-bug or docs-gap kind.
6. Adopt the game: put `pong` back into `WINDOWED_EXAMPLES` and
   `VERIFIABLE_EXAMPLES` in `tools/test`, so it is built and verified on every
   push like every other example. This is deliberately the maintainer's step —
   asking a game author to register their game with the engine's test harness
   would be asking them out of the role the run is measuring.

   **`tools/test` now says so if you forget.** An example with a `verify.rs`
   beside its `main.rs` and no entry in either list fails the wrapper before any
   phase runs, naming this step. It was missed after runs 4, 5 and 7, each time
   surfacing as `RunError::NoDisplay` in an `example:pong` phase — a symptom that
   says nothing about its cause (e0-findings.md F-094).

   **Every run, not only the one that passes.** This step used to say "on the
   run that passes", written before anyone had run E0; run 1 registered its game
   immediately and that was both harmless and useful, since a game nobody
   verifies is a game that rots between runs. Registration is **not** evidence
   of a pass — §5 of `e0-findings.md` records where that confusion came from,
   and the checklist in §4 of `implementation-plan.md` is the only thing that
   says whether the milestone is met.

---

## The prompt

> You are writing a game with the Jidousha engine. You are the game's author,
> not the engine's — treat the engine as a library you did not write and cannot
> change.
>
> **Build a playable Pong.** Two paddles, a ball that bounces, a score on
> screen. One player controls the left paddle from the keyboard; the right one
> can be a simple AI or a second set of keys, your choice. It should be fun for
> about thirty seconds, which is the honest bar for a prototype.
>
> **What you may read:**
> - `docs/api/jidousha-api.md` — the engine's API. This is the document.
> - `docs/api/jidousha-testing.md` — its other half: how to check the game you
>   wrote. Headless runs, asserting on what was drawn, the `--verify` convention,
>   and capturing a picture of a frame.
> - `crates/jidousha/examples/` — worked examples, including `quickstart.rs`.
>
> **What you may not read**, at all, for any reason:
> - `crates/*/src/` — the engine's source.
> - `docs/internal/` and `docs/adr/` — the engine's design notes.
> - `docs/agent-practices.md`, `docs/conventions.md` — maintainer docs. The part
>   of `conventions.md` a game needs is already inside the API document.
> - `docs/e0/` — earlier authors' friction logs. They are the record of what
>   this exercise cost people before you, and reading one hands you answers you
>   are here to have to find.
>
> If you catch yourself about to open one of those, stop. Not knowing is the
> point: this run is measuring whether the API document is enough on its own, and
> a run that reaches past it measures nothing. `CLAUDE.md` is in your context
> automatically — it is the engine maintainers' router. The one row that applies
> to you is *"Write a game with the engine → `docs/api/` and `examples/` ONLY —
> never `src/`"*. Ignore the rest of it.
>
> **Where the game goes:** `crates/jidousha/examples/pong.rs`, or a directory
> `crates/jidousha/examples/pong/` if it outgrows one file. Run it with
> `cargo run -p jidousha --example pong`.
>
> **What "done" means:**
> 1. It runs in a window and is playable.
> 2. It has a `--verify` mode: scripted input, a fixed number of headless ticks,
>    and assertions about what the world did and what was drawn — so the game can
>    be checked with nobody watching. `crates/jidousha/examples/prototype_kit/`
>    is a worked example of the shape.
> 3. `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
>    are clean.
>
> You do **not** need `tools/test` to pass. It runs every example in the
> repository headlessly, and yours opens a window, so it fails for a reason that
> has nothing to do with your game. `cargo run -p jidousha --example pong --
> --verify` is your check. Leave the repository's own tooling alone — it is not
> yours to edit, and editing it is not part of writing a game.
>
> **Write down every friction, as it happens, in `docs/e0/run-N.md`.** Create
> the file, choosing the lowest N that does not already exist — you can see the
> names in that directory without opening anything in it, and you must not open
> anything in it. This file is as much the deliverable as the game is. Record:
> - anything the API document did not tell you, that you had to guess at;
> - anything you expected to exist and could not find;
> - anything that behaved differently from what the document implied;
> - anything that took more than one attempt to get right, and why;
> - anything you wanted to look up in the engine's source, and what for.
>
> Do not soften these. "The document does not say what units `Sprite::size` is
> in, so I guessed world units" is exactly what this run is for. A run that
> reports no friction and produces a working Pong is a less useful run than one
> that limps and says why.
>
> **If you get stuck**, and the API document does not have the answer: say so,
> write it down, and either work around it in the game or stop. Do not read the
> engine's source to get unstuck. Being blocked is a result, not a failure — it
> is the most valuable result this run can produce.
