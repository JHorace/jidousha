# E0 — the acceptance prompt

The text below is what a **fresh** Claude Code session is given for an E0 run
(implementation-plan.md §3). It is checked in so that every repeat uses the same
prompt: if the wording changes between runs, the runs are not comparable and
"passed clean twice in a row" means nothing.

This file lives in `docs/internal/` deliberately — it is maintainers' harness,
and the E0 session must never read it. Paste the block, do not point at the file.

---

## Before starting a run

1. `git checkout -b e0/attempt-<n>` from the current default branch.
2. Confirm the working tree is clean and `tools/test` passes, so anything the
   run breaks is the run's.
3. Start a **new session**. Not a continuation, not a compaction — a session
   with no memory of the engine's internals. That is the entire instrument.
4. Paste the prompt below verbatim.

## After the run

1. Read the session's transcript and check the restriction was honored — no
   reads under `crates/*/src/`, `docs/internal/`, or `docs/adr/`. A breach
   invalidates the run; note it and start again.
2. **Play it.** `cargo run -p jidousha --example pong` in a window, and
   `tools/serve-web pong` in a browser. "Playable" is not something a script can
   assert, which is why the milestone asks a person: a Pong whose ball passes
   through the paddle satisfies every assertion an agent would think to write.
3. Take the run's `E0-NOTES.md` and root-cause each entry into
   `docs/internal/e0-findings.md`. **Every friction is an engine or docs bug
   until proven otherwise** — that is the rule the milestone turns on.
4. Fix what the findings say to fix. Then run E0 again, fresh.
5. E0 passes when two consecutive runs produce no new findings of the
   engine-bug or docs-gap kind.
6. On the run that passes, adopt the game: add `pong` to `WINDOWED_EXAMPLES` and
   `VERIFIABLE_EXAMPLES` in `tools/test`, so it is built and verified on every
   push like every other example. This is deliberately the maintainer's step —
   asking a game author to register their game with the engine's test harness
   would be asking them out of the role the run is measuring.

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
> - `crates/jidousha/examples/` — worked examples, including `quickstart.rs`.
>
> **What you may not read**, at all, for any reason:
> - `crates/*/src/` — the engine's source.
> - `docs/internal/` and `docs/adr/` — the engine's design notes.
> - `docs/agent-practices.md`, `docs/conventions.md` — maintainer docs. The part
>   of `conventions.md` a game needs is already inside the API document.
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
> **Write down every friction, as it happens, in `E0-NOTES.md` at the repository
> root.** This file is as much the deliverable as the game is. Record:
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
