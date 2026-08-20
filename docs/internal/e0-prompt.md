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
| 9 | The may-read list names a **third**, `docs/api/jidousha-controllers.md`, and says when to read it. | The same split one level down (ADR-0030): the controller advice was a seventh of the testing document, is not about this engine, and had taken seven findings across six runs without the file being able to afford an eighth. Same reasoning as run 6's row — the material is unchanged, the file was always inside the allowed `docs/api/`, and naming it is so a run does not have to guess it exists. |

No change alters what the run is asked to *build* or what it may read of
the engine, so runs 1–2 and 3+ remain comparable on the thing being measured:
whether `docs/api/` is enough on its own. The first two changes remove
information the earlier runs had and should not have had, which makes later runs
strictly harder — the safe direction for a bar to move. **Any future change that
makes a run *easier* invalidates the streak and restarts the count.**

**Neither split makes a run easier, and the reasoning is worth writing down
rather than asserting.** Not one sentence of guidance was added, removed or
softened by it — the same prose and the same reference entries, in two files
chosen by task. What changed is that a run must now find a second file, which is
a hazard rather than a help: a run that misses it writes its `--verify` mode
without the testing reference, which is *harder* than run 5 had it. So the streak
stands. The honest risk runs the other way, and §6 should watch for it — if run 6
never opens `jidousha-testing.md`, the three pointers into it are not enough and
that is a finding about the split, not about the run.

**The second split (ADR-0030) inherits that argument whole**, and one risk in it
is worth watching specifically. `jidousha-controllers.md` is the file a run
reaches *last* — it already has a working game and a `--verify` mode that runs —
and it is the one a run under pressure is most likely to skip. If run 9 writes a
blind or naive controller and reports a number about *it* as though it were a
number about the game, which is the failure five runs have now had, check first
whether it opened the file at all. A run that read it and still fell in is a
finding about the prose; a run that never found it is a finding about the split,
and the answer to that one is more pointers rather than more paragraphs.

---

## Before starting a run

1. `git checkout -b e0/attempt-<n>` from the current default branch.
2. **Delete the previous run's game and de-register it**, in one commit:
   `git rm -r crates/jidousha/examples/pong/`, then remove `pong` from
   `WINDOWED_EXAMPLES` and `VERIFIABLE_EXAMPLES` in `tools/test`.

   **Only `pong/`.** `crates/jidousha/examples/slalom/` is a permanent worked
   example — run 4's lever, spent after run 8 (e0-findings.md §6) — and is not a
   previous run's game. It stays. A run reading it is the point of it existing.

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

   **The window half is now doable from the container**, which it was not for
   runs 1–8 (e0-findings.md F-111): the session-start hook installs
   `libxkbcommon-x11-0`, `xvfb`, `xdotool` and `x11-apps`, so

   ```
   Xvfb :99 -screen 0 1280x720x24 &
   DISPLAY=:99 cargo run -p jidousha --example pong &
   DISPLAY=:99 xdotool windowfocus --sync "$(DISPLAY=:99 xdotool search --name pong | tail -1)"
   DISPLAY=:99 xdotool key space; DISPLAY=:99 xwd -root -silent -out frame.xwd
   ```

   is a real playtest — real key events through `winit`, real frames back out of
   `wgpu` on lavapipe. **The `windowfocus` line is not optional**: Xvfb has no
   window manager, so without it every key goes to the root window and the game
   looks deaf. A session that skips it will file an input bug that does not exist.

   **And the browser half is doable from the container too**, since run 9's triage
   (F-124): the hook now installs the `wasm-bindgen` version `Cargo.lock` pins, and
   a Chromium was in the image all along, so

   ```
   tools/serve-web pong --check
   ```

   builds the wasm, drives the browser at it and reports what the canvas drew,
   writing `target/web/check.png`. That is the only check that runs the web target
   as a *program* rather than as the `cargo check` CI has gated since M0.

   Doing either in the container does **not** retire the person. Together they
   answer "do the window and web paths work end to end", which is what F-079,
   F-096 and F-112 were about; neither answers "is this fun", and that is the
   question step 2 exists for.
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
5. E0 passes when two consecutive runs produce **no `engine` finding and no
   *novel* `docs` finding** (ADR-0029, e0-findings.md §2). A `docs` finding whose
   cross-run column names a prior `F-` number is a re-tread: record it, fix it,
   and do not reset the streak for it. An `engine` finding resets the streak
   whatever its history.

   **This changed after run 8 and the ledger above applies to it**, so the streak
   restarted at zero — which cost nothing, because it had been zero since run 1.
   The prompt itself is deliberately unchanged: it is right to ask a run for
   every friction, and the conflict between "do not soften these" and a bar that
   required silence is resolved on the counting side rather than by asking runs
   to report less.
6. Adopt the game: put `pong` back into `WINDOWED_EXAMPLES` and
   `VERIFIABLE_EXAMPLES` in `tools/test`, so it is built and verified on every
   push like every other example. This is deliberately the maintainer's step —
   asking a game author to register their game with the engine's test harness
   would be asking them out of the role the run is measuring.

   **This step is now bookkeeping rather than a gate.** `tools/test` reads each
   example's source for a `--verify` flag and runs anything it finds through
   `tools/verify`, registered or not, printing a note that says what was not
   registered. So a game is checked on every push from the moment it lands, and
   the failure this step used to cause — missed after runs 4, 5 and 7, each time
   surfacing as `RunError::NoDisplay` in an `example:pong` phase (e0-findings.md
   F-094) — cannot recur. Take the step anyway: the lists are what a reader
   consults.

   **A consequence for the prompt below, which is deliberately *not* changed.**
   It tells the author "you do not need `tools/test` to pass … yours opens a
   window, so it fails for a reason that has nothing to do with your game". The
   reason is now out of date — their game is verified rather than run bare — but
   the instruction is still true and still what we want, and the ledger above
   says a prompt that changes between runs makes the runs incomparable. Left
   alone on purpose.

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
> - `docs/api/jidousha-controllers.md` — the third: how to write the *player*
>   inside that check, so what it reports is about the game rather than about
>   itself. Read it last, and only once your `--verify` mode needs a player that
>   can win.
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
