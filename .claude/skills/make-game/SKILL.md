---
name: make-game
description: Build a playable game or prototype with the Jidousha engine, as the game's author rather than the engine's. Use whenever the user asks for a game, a prototype, a demo, or anything playable built with this engine — "make Pong", "a little arcade game", "try this mechanic" — even if they never say the word "game". Owns the whole prototype workflow — reading order for docs/api/, writing the game, the --verify mode and its players, the mutation round, the capture, registration in tools/test. Not for engine work: changing the engine's source, docs, or tools has its own routing in CLAUDE.md.
---

# make-game — the prototype workflow

You are the game's author, not the engine's. Treat the engine as a library you
did not write and do not change. Read `docs/api/` — all four documents — and
`crates/jidousha/examples/`, and nothing else: not `crates/*/src/`, not
`docs/internal/`, not `docs/adr/`. If a document does not answer a question,
do not read the source for the answer: work around it in the game and name the
gap in your commit message — the API documents are maintained on exactly that
evidence, and a reported gap gets fixed where a silent workaround hides it.

Everything a game needs to *know* is in those four documents. What this
checklist adds is **order**: each step below is cheap at the moment it is
listed and expensive after it, and every one of these orderings was paid for
by a session that met the fact too late.

## 1. Read in this order, at these moments

| Read | When |
|---|---|
| `docs/api/jidousha-api.md` — Quickstart and Concepts in full; Reference for lookup | before writing anything |
| `docs/api/jidousha-testing.md` — top to bottom once | when the game first runs, before the first check |
| `docs/api/jidousha-controllers.md` — whole, it is short | when the check needs a player that can win — and before tuning any constant |
| `docs/api/jidousha-capture.md` | last, once `--verify` runs and asserts |

Each document names the next; this table is the same order stated once, so
none has to be discovered by needing it.

## 2. Set up — the decisions that are cheap now and a restructure later

Before the first system:

- [ ] Make the game a directory — `crates/jidousha/examples/<name>/` with a
  `main.rs` — because a game with a `--verify` mode wants one (*Concepts*, "An
  example can be a directory"). `examples/prototype_kit/` is the shape:
  `main.rs`, `checks.rs`, `verify.rs`, `capture.rs`.
- [ ] State the layout in constants derived from the window — the three-line
  block in *Concepts* ("A layout in constants"). One line now; a hand-typed
  ratio later, coupled to the window by nothing but an assertion.
- [ ] Write what a check will need to predict — the opponent's decision, the
  collision response — as free functions from the first draft (*Concepts*,
  "Write the two decisions a check will want as free functions, now, while
  they are free"). This is the single most expensive item here to retrofit;
  the paragraph says why.
- [ ] Pick the thinnest collider and the top speed **together** — *Concepts*
  couples them ("the thinnest collider is the ceiling on speed"). The day the
  game plays too slowly, return to that paragraph, not to the speed constant.
- [ ] Decide the sub-tick collision order and say so at the site (*Concepts*,
  the swept-collision paragraphs). Step 4 holds the game to it.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings` after the
  first hundred lines and keep it clean as you go — the lint section closing
  *Concepts* says which rules a game inherits and why the end is the wrong
  place to meet them.

## 3. Build until it plays

`cargo check` after every edit. `cargo run -p jidousha --example <name>` and
play it when a display exists; `tools/serve-web <name> --check` drives a
browser at the web build headlessly either way. The honest bar for a prototype
is fun for about thirty seconds.

## 4. Give it a `--verify` mode

Read `docs/api/jidousha-testing.md` start to finish first — it is ordered the
way a check file is, and skimming it costs more than reading it.

- [ ] The mode's skeleton is the document's closing convention: the
  `verified ` verdict line, failures collected rather than exited on, every
  failure reporting the numbers it judged.
- [ ] First check: nothing drawn outside the camera — and print the clearance
  margin beside it, as the document asks and `examples/prototype_kit` does.
- [ ] Then layout checks stated as the game's *requirements*, not its
  constants — the document's closing passages name the trap and its general
  form. Expect to get one of these wrong anyway; step 6 is what catches it.
- [ ] Stage the screens the run never reaches, and ask the contracts play
  never exercises directly (both in the document).
- [ ] Assert the schedule order picked in step 2 (`schedule_debug`, written
  out in the document).

`tools/verify <name>` runs the mode under a timeout;
`target/verify/report.json` is ground truth, terminal output is advisory.

## 5. Give the check a player

Read `docs/api/jidousha-controllers.md` whole before believing — or tuning
against — any number a run reports. `examples/slalom/` is the document worked;
`examples/pong/controller.rs` is it worked against an opponent.

- [ ] Write the three players the document opens with, one verdict line each —
  its first section is why no single player, and only the middle line, can
  call the game playable.
- [ ] Print the three controller numbers every run (the document's "Three
  numbers" section; `slalom`'s `Report` is the shape) — they are what make
  "suspect the controller first" a suspicion you can settle in one run.
- [ ] When the middle player's line says the game will not play, work the
  document's last two sections **before touching any speed** — sessions that
  tuned first spent the round the document exists to save.

## 6. Break the game on purpose

- [ ] Commit every file the round will touch **first** — the mutation passage
  in the testing document ("Mutate the game and check the run notices") says
  which revert eats what.
- [ ] Inject one-line faults and demand the run names each one. The same
  passage names the two ways a hand-rolled harness lies about its own score;
  build both as hard errors before trusting a number.
- [ ] Run the first round as soon as the first few checks exist, not once at
  the end. Expect it to find a loose check — the document is explicit that its
  own rule does not transfer by being read, and that the round is the
  mechanism.

## 7. Take the picture

Read `docs/api/jidousha-capture.md` last, once the check runs and asserts.

- [ ] `examples/prototype_kit/capture.rs` is the worked path; the document
  says which of its lines a shapes-only game leaves out.
- [ ] The `capture:` line is a contract with `tools/verify` — word it exactly
  as the document gives it, or the run passes while the report says no picture
  was taken.
- [ ] Open the PNG and name what you see. Then break the game on purpose and
  look again — the document's closing paragraph is the procedure.

## 8. Ship it

- [ ] Register the game in `tools/test`: add `<name>` to both
  `WINDOWED_EXAMPLES` and `VERIFIABLE_EXAMPLES`, so it is built and verified
  on every push. (An unregistered `--verify` game is still run, with a loud
  note — the lists are what a reader consults, so take the step.)
- [ ] `cargo fmt --all` clean, clippy clean, `tools/test` green — the report
  file is the verdict. `tools/check-assets` if the game loads art.
- [ ] In the commit message: anything a document failed to answer, or answered
  somewhere you did not look.

---

This checklist orders the documents; it does not replace them. When this file
and a document disagree, the document is right — fix this file in the same
commit.
