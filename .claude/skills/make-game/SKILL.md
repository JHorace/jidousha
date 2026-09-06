---
name: make-game
description: Build a playable game or prototype with the Jidousha engine, or land a module into one that already runs, as the game's author rather than the engine's. Use whenever the user asks for a game, a prototype, a demo, or anything playable built with this engine — "make Pong", "a little arcade game", "try this mechanic" — even if they never say the word "game", and whenever a session's job is a wave or a module of a game that already exists under games/. Owns the whole game-session workflow — where a game lives (a crate under games/, ADR-0038), reading order for docs/api/ and for a game's own canon, writing the game, the --verify mode and its players, the mutation round, the capture, the decision-surface section a handoff must carry, the findings it owes back, and the closing checklist the owner loop runs on. Not for engine work: changing the engine's source, docs, or tools has its own routing in CLAUDE.md.
---

# make-game — the game-session workflow

You are the game's author, not the engine's. Treat the engine as a library you
did not write and do not change.

Two session shapes come through here, and they share a spine:

| You were asked to… | Go to |
|---|---|
| Make a new game or prototype — nothing exists yet | **§A**, then §C, §D and §E |
| Land a wave or a module into a game that already runs (`games/ninjo/`) | **§B**, then §C, §D and §E |

§0 binds both. §C (findings), §D (decision surfaces) and §E (the closing
checklist) are owed by every session of either shape, and §E is not optional: a
session that ends without working its checklist has violated this skill. §D
carries the one rule in this file that is read **before** implementing rather
than after: a handoff missing its decision-surface section stops the session.

---

## 0. What binds every game session

### 0.1 The reading fence

**A new prototype reads `docs/api/` — all four documents — and
`crates/jidousha/examples/`, and nothing else**: not `crates/*/src/`, not
`docs/internal/`, not `docs/adr/`. This binds the *session*, not only the game
file: opening the engine's source to answer a question spends the evidence
this exercise exists to collect, whether or not a line of it reaches the game.
ADR-0038 is the one standing exception — where the crate goes and what it may
depend on — and any other ADR your handoff names by number.

**A module session inherits that fence and adds its own game to the readable
set.** You are extending a crate that already exists, so `games/<name>/` is
yours to read whole; `crates/*/src/`, `docs/internal/` and unnamed ADRs stay
shut. Reading a sibling game (as ninjo read `games/giri/` to port from it) is
reading a game, not the engine, and is allowed — say so in your findings note
so the reading discipline stays auditable.

If a document does not answer a question, work around it in the game and name
the gap (§C). The documents are maintained on exactly that evidence, and a
reported gap gets fixed where a silent workaround hides it.

### 0.2 Trouble

`tools/doctor` before any fix that is not a plain compile error in code you
just changed, and obey its verdict — CLAUDE.md's "When builds/tests fail" is
the whole rule and it applies here unchanged. Same command failing the same
way twice after a fix attempt: stop, run doctor, copy `docs/templates/BLOCKED.md`
to the repo root and fill it in. **A good BLOCKED.md is a successful outcome.**
`target/verify/report.json` is ground truth; the terminal is advisory.

### 0.3 What "done" means for a game

CLAUDE.md's definition of done, plus: `cargo fmt --all` clean, clippy clean,
`tools/check-game-deps` clean, `tools/check-assets` if the game loads art,
`tools/test` green with the report file as the verdict — and the four things
this workflow adds, each with its own step below: the `--verify` mode (§A.4 /
§B.4), the mutation round (§A.6 / §B.5), the pictures a person actually looked
at (§A.7 / §B.6), and the findings (§C). §D binds the handoff itself.

---

## A. A new prototype, end to end

Everything a game needs to *know* is in the four API documents. What this
checklist adds is **order**: each step is cheap at the moment it is listed and
expensive after it, and every one of these orderings was paid for by a session
that met the fact too late.

### A.1 Read in this order, at these moments

| Read | When |
|---|---|
| `docs/api/jidousha-api.md` — Quickstart and Concepts in full; Reference for lookup | before writing anything |
| `docs/api/jidousha-testing.md` — top to bottom once | when the game first runs, before the first check |
| `docs/api/jidousha-controllers.md` — whole, it is short | when the check needs a player that can win — and before tuning any constant |
| `docs/api/jidousha-capture.md` | last, once `--verify` runs and asserts |

Each document names the next; this table is the same order stated once, so
none has to be discovered by needing it.

### A.2 Set up — the decisions that are cheap now and a restructure later

Before the first system:

- [ ] Make the game **its own crate**, at `games/<name>/` (ADR-0038). Nothing
  registers it anywhere: living there is what makes it built, linted, verified
  and published. The layout, and the four files `examples/prototype_kit/` is
  the shape of:

  ```
  games/<name>/Cargo.toml     [package] name = "<name>", the four `.workspace = true`
                              lines, and one dependency: jidousha = { path = "../../crates/jidousha" }
                              plus `[lints] workspace = true`
  games/<name>/src/main.rs    the game, and `#![allow(missing_docs)]` at the top
                              src/checks.rs, src/verify.rs, src/capture.rs beside it
  ```

  Two things about that manifest are load-bearing:

  - **`jidousha` is the only engine crate it may name**, at any depth. Naming
    `jidousha-core` or any other fails `tools/check-game-deps` in CI. When the
    facade does not expose what the game needs, that gap is a finding (§C),
    not a dependency line.
  - **`#![allow(missing_docs)]` at the crate root** is how a game opts out of
    the one workspace lint that is the engine's and not a game's. Cargo refuses
    a manifest that overrides a workspace lint table, so the crate root is where
    the exemption lives. Everything else in that table still applies.
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
  the swept-collision paragraphs). §A.4 holds the game to it.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings` after the
  first hundred lines and keep it clean as you go — the lint section closing
  *Concepts* says which rules a game inherits and why the end is the wrong
  place to meet them.

### A.3 Build until it plays

`cargo check` after every edit. `cargo run -p <name>` and play it when a
display exists; `tools/serve-web <name> --check` drives a browser at the web
build headlessly either way. The honest bar for a prototype is fun for about
thirty seconds.

### A.4 Give it a `--verify` mode

Read `docs/api/jidousha-testing.md` start to finish first — it is ordered the
way a check file is, and skimming it costs more than reading it.

- [ ] The mode's skeleton is the document's closing convention: the
  `verified ` verdict line, failures collected rather than exited on, every
  failure reporting the numbers it judged.
- [ ] First check: nothing drawn outside the camera — and print the clearance
  margin beside it, as the document asks and `examples/prototype_kit` does.
  **If the camera pans or zooms over a world larger than the screen, that
  check does not transfer** — G-010 (`games/ninjo/FINDINGS.md`) works out the
  three-assertion form it becomes, and `games/ninjo/UI.md` §4 states it.
- [ ] Then layout checks stated as the game's *requirements*, not its
  constants — the document's closing passages name the trap and its general
  form. Expect to get one of these wrong anyway; §A.6 is what catches it.
- [ ] Stage the screens the run never reaches, and ask the contracts play
  never exercises directly (both in the document).
- [ ] Assert the schedule order picked in §A.2 (`schedule_debug`, written
  out in the document).

`tools/verify <name>` runs the mode under a timeout;
`target/verify/<name>.json` is ground truth, terminal output is advisory.

### A.5 Give the check a player

Read `docs/api/jidousha-controllers.md` whole before believing — or tuning
against — any number a run reports. `crates/jidousha/examples/slalom/` is the
document worked; `crates/jidousha/examples/pong/controller.rs` is it worked
against an opponent — those stay engine examples and are the ones to read, not
to move.

- [ ] Write the three players the document opens with, one verdict line each —
  its first section is why no single player, and only the middle line, can
  call the game playable.
- [ ] Print the three controller numbers every run (the document's "Three
  numbers" section; `slalom`'s `Report` is the shape) — they are what make
  "suspect the controller first" a suspicion you can settle in one run.
- [ ] When the middle player's line says the game will not play, work the
  document's last two sections **before touching any speed** — sessions that
  tuned first spent the round the document exists to save.

### A.6 Break the game on purpose

- [ ] Commit every file the round will touch **first** — the mutation passage
  in the testing document ("Mutate the game and check the run notices") says
  which revert eats what.
- [ ] Inject one-line faults and demand the run names each one. The same
  passage names the two ways a hand-rolled harness lies about its own score;
  build both as hard errors before trusting a number.
- [ ] **Write the instrument's expectation as a shipped literal**, never as
  arithmetic over the constant under test: a check that derives its expectation
  from the number being mutated cannot see that number move. (ninjo wave 0b,
  GDD §9 — its trait arithmetic and store battery are both written this way.)
- [ ] Run the first round as soon as the first few checks exist, not once at
  the end. Expect it to find a loose check — the document is explicit that its
  own rule does not transfer by being read, and that the round is the
  mechanism.

### A.7 Take the picture

Read `docs/api/jidousha-capture.md` last, once the check runs and asserts.

- [ ] `examples/prototype_kit/capture.rs` is the worked path; the document
  says which of its lines a shapes-only game leaves out.
- [ ] The `capture:` line is a contract with `tools/verify` — word it exactly
  as the document gives it, or the run passes while the report says no picture
  was taken.
- [ ] Open the PNG and name what you see. Then break the game on purpose and
  look again — the document's closing paragraph is the procedure.

### A.8 Ship it

- [ ] Nothing to register — a game under `games/` is picked up by `tools/test`,
  `tools/verify`, `tools/build-web` and the deploy from where it lives
  (ADR-0038). That holds for the production page too: the deploy curates which
  *examples* it serves and never which games (web-publish.md §3a). The step that
  used to be here is the step that kept being missed.
- [ ] The gates of §0.3, all of them, with the report file as the verdict.
- [ ] The PR's preview comment carries a playtest URL for the game, at
  `/<name>/`. Open it, play it there, and say in the PR what it was like — a
  build nobody played is a build nobody checked.
- [ ] In the PR and the commit message: anything a document failed to answer,
  or answered somewhere you did not look, and **every deviation from the
  handoff or from the game's own design docs, with its reason**. A deviation
  nobody was told about is a design change made by silence.

---

## B. A module session into a game that already runs

This is the shape ninjo's wave plan is made of (`games/ninjo/GDD.md` §8): one
session, one handoff, one module — landing into a world that already runs and
must still run when yours is switched off.

### B.1 Read in this order

| Read | Why |
|---|---|
| `CLAUDE.md` | the router, the never-list, the failure protocol |
| the game's **GDD** — its registry row for your module, plus the sections that row's `reads`/`writes` columns point into | the GDD is the game's canon. Where it and any other document disagree about a decided thing, it wins |
| the game's **DESIGN.md** | the substrate's technical doc — the clock, the grid, the scheduler, the verify machinery you are landing on top of |
| the game's **UI.md**, if your module owns a surface | the floors, the screenshot process, and what binds a new surface |
| the game's **content bible**, where it has one (`games/ninjo/CAST.md`) | who the cast are, what the vocabulary words mean, and which of its sections your wave is the one that builds |
| **the module's fences in your handoff**, and its decision-surface section (§D) | what this session may and may not touch — three sessions can be in flight at once — and what the player is meant to be able to decide when yours has landed |
| `docs/api/` | as §A.1 orders them, for any engine question the game has not already answered |

Read the registry row before the prose. It is the contract; the prose is why.

### B.2 The module's contract, from its registry row

A module is a row in the GDD's registry (`games/ninjo/GDD.md` §5) and a set of
systems that obey it. Before writing a system:

- [ ] **Take the row's `requires`, `reads` and `writes` columns as binding.**
  Reading shared state the row does not list, or writing state it does not
  list, is a registry change — which is a GDD edit and an owner decision, not
  a thing a module session does on its way past.
- [ ] **No module reads another module's interior** (GDD §1's standing
  principles). Coupling is through shared state and events only. If you need
  something another module knows, either it belongs in shared state or the
  need is a design question for the owner; a private accessor added "just for
  this" is the coupling this architecture exists to refuse.
- [ ] **Register the disable flag in the module scaffold** —
  `games/ninjo/src/modules.rs`: one `ModuleSpec` row (id, tier, wave,
  `degrades_to`), and `ModuleSet::enabled("<id>")` read wherever the module's
  systems and data are installed. Nothing else changes: the matrix, the stamp
  and the drawer's report line all walk the table.
- [ ] **`degrades_to` is a sentence about behavior, and it is the thing the
  module-off matrix asserts.** Write what the world does with your module off,
  then make the off-pass prove it. A row whose `degrades_to` is empty fails
  the registry's own validation; a row whose `degrades_to` is untested is
  worse, because it reads as checked.
- [ ] **One decision function per question** (GDD §1). Preview and simulation
  call the same function, so a surface cannot say something the sim disagrees
  with. Warnings derive from the numbers that produce the consequence.

### B.3 Events and attention

If your module emits anything the player should notice:

- [ ] **Every occurrence carries time, place and class** (GDD §3). A thing
  that happened nowhere or at no time is not an event in this game.
- [ ] **A new event class registers in the event-class table** — today that is
  `EventClass` in the game's `src/sim.rs`, the five S1 classes plus whatever
  waves have added since. The registry's `reads`/`writes` columns do not have
  an `emits` column; **the class table is where emission is declared**, so a
  class added without a row there is an emission nobody can find.
- [ ] **Give each new class a default attention mode** — ignore / log /
  pause-and-focus (GDD §3, wave 0a). Until wave 0a lands the mode column, say
  the intended default in the class's comment and in your PR, so the attention
  session inherits a decision instead of a guess. The feed is the attention
  budget's ledger; a class with no stated mode spends it silently.

### B.4 Verify — what a module owes the harness

The game's verify mode is the instrument, and the module-off matrix is what
makes "modular" a fact rather than a claim (GDD §9). Your module owes:

- [ ] **A green matrix.** `verify` runs the suite once per module with that
  module alone off, plus the everything-on baseline. The baseline pass asserts
  the authored timeline; an off-pass asserts the world still *runs* — no panic,
  the script consumed, everything at rest, shared-state arithmetic intact —
  because a module being off is *supposed* to change what happens.
- [ ] **The speed-invariance sweep, extended over every new event source.**
  Identical event sequences with identical world-time stamps under every speed
  script. A new occurrence that is scheduled on the one scheduler with a
  world-time address is speed-invariant by construction; one that is not is
  the exact failure the substrate exists to prevent.
- [ ] **A run that adds no event source must leave the transcript identical.**
  That is how wave 0b landed regard drift, and it is the cheapest possible
  proof that a change changed nothing.
- [ ] **Floors and the screenshot process on every surface you add** — the
  game's `UI.md` §4 and §6 own both: every row of content in the `Panel`,
  every string ASCII, every read of the world through the lens. Open every PNG
  and name what you see.
- [ ] **Stamps carry seed, constants and module set** — verify's report and the
  scenario's opening log line both. A recording whose stamp does not say which
  modules were on is a recording of an unknown build.
- [ ] **The mutation round walks every constant**, with shipped literals as the
  expectations (§A.6's third bullet). Report the count: `N of N noticed`.
- [ ] Any sweep the GDD's §9 owes your module (economy bands, distribution over
  a ladder) lands with the module that makes it meaningful, not before.

### B.5 Break it on purpose, then B.6 take the picture

§A.6 and §A.7 apply unchanged; they are the same two steps, and the mutation
round is the one that decides whether anything above is an instrument.

### B.7 Land it

- [ ] The gates of §0.3.
- [ ] **Write the `*Implemented (wN):*` mark** into the GDD sections your wave
  built, in the same commit, naming what landed, where it lives, and **where
  the build bent the shape above it**. That mark discipline is the GDD's own
  (its header states it); a section built without one reads as unbuilt.
- [ ] Play the build at its preview URL and say in the PR what it was like.
- [ ] In the PR: every deviation from the handoff or the GDD, with its reason.

---

## C. Write down what the documents cost you

A game session is a report on the documents as well as a game, and this is the
half that is easy to skip because the build already works. E0 ended (ADR-0036);
the reporting did not.

**File a finding when a document was missing — and when a document misled
you.** The second half is the newer rule and the sharper one. A doc that
answered wrongly is more expensive than a doc that said nothing, because you
acted on it: F-055 is a doc comment that described the wrong method, which the
generator carried faithfully into `docs/api/` and a human paraphrased into the
prose; F-045 and F-088 are worked examples teaching two spellings of one import
in a repo whose first convention is "one way to do everything". Nothing about
either was missing. The ledgers are this repo's rot sensor and they only sense
what gets filed, so:

- [ ] One entry per question the documents did not answer, answered wrongly,
  or answered somewhere you did not look — in the format
  `docs/internal/e0-findings.md` uses: what you were doing, what you expected,
  what happened, and which document owns it. Read one existing entry for the
  shape; do not read the file's analysis sections, which are about runs you
  were not in.
- [ ] **When the fault is a doc that misled you, say what you did on its
  authority** before you found out. That sentence is what tells a maintainer
  whether the fix is a wording change or a check.
- [ ] Findings live in the game's own `FINDINGS.md` (G-numbers continue the
  sequence across games) **and** in the PR description. A workaround shipped
  silently is a gap nobody fixes, and a PR body is not somewhere anybody reads
  twice.
- [ ] `0 findings` is a real answer and worth saying explicitly — say *why* it
  is true (wave 0b's entry is the model: it asked the documents nothing new).
  A finding invented to fill the section is worse than an empty one.
- [ ] Something you learned that is the *game's* decision and not the engine's
  still belongs in `FINDINGS.md`, marked as such — the next wave meets the same
  fact.

---

## D. Decision surfaces, and mockups as references

§C is owed after the build. This section is owed **before** it: it is about
what the handoff says, and the one place in this file where the right move is
to stop and ask rather than to implement.

### D.1 The decision-surface table is a required section of the handoff

**A game handoff that adds or changes a player decision carries a
decision-surface table** — one row per decision, six columns:

| column | what goes in it |
|---|---|
| **decision** | what the *player* chooses, in their words ("which job to send the selected character to") |
| **must know** | the facts the choice turns on (the job's task type, its pot, its duration; this character's fit; the travel) |
| **surface** | where those facts are shown **at the moment of choosing** (the site panel's job row) |
| **action** | the input that commits it (tap the open row with an idle character selected) |
| **one function** | the sim function that both the display and the outcome read (`traits::competence_at`; the pathfinder), so a preview and the sim cannot disagree |
| **asserted by** | the floor or the scripted test that proves those facts are on screen when the action is available |

`docs/templates/DECISIONS.md` is the copyable skeleton and carries that row
worked out in full. **A handoff that adds no decision says so in one explicit
line** — `Decisions: none new; existing surfaces unchanged.` — because an empty
table reads as a section nobody filled in.

**The rule: a game handoff with neither the table nor that line is malformed.
Stop and ask the owner before implementing**, exactly as you would for a
handoff missing its reading fence. Do not infer the surface from the systems,
and do not plan to build the systems now and the surface after.

Why it is a stop rather than a reminder: a build is faithful to its handoff,
which means faithful to its **silence**. Wave 1.1 of ninjo specified its
systems and its data down to the drawer row and its decision surface not at
all, and the owner's playtest of the deployed build filed
`games/ninjo/FINDINGS.md` **G-017** (two selections over one roster, so the
player could not tell whom they were about to send) and **G-018** (one person
drawn twice on the map) — both "the game's own (a wave-1.1 gap)", both
invisible to the checks, because a surface nobody specified is a surface
nobody asserts. The omission was the design session's, so the remedy is a
section of the handoff and a stop, not diligence at build time.
`docs/agent-practices.md` §2.6 is the argument in full. (Owner policy,
2026-09-02.)

Where the handoff has the table, §E closes it out: every row ships with the
test that asserts it, named in the final message.

### D.2 A mockup is a reference the build is checked against

Two kinds come through, and they are not the same object:

- **An architecture mockup** — an interactive HTML slice that runs the real
  data shapes. Rare, and attached when a wave introduces a new *interaction
  model* rather than another surface in an existing one (ninjo's wave 0a
  attention mockup is the precedent: GDD §8, and §8's wave-1.5 calibration is
  still being spent out of it).
- **A surface spec** — the decision table plus a wireframe. The ordinary case,
  and what most waves should attach.

**When a handoff attaches a mockup of either kind, the session opens it.** An
attached mockup nobody opened is the failure mode this rule exists for: it
looks like a specification and functions as decoration.

You have a headless browser and no excuse. It is the one `tools/serve-web
--check` drives — that tool's `chromium()` helper names where it lives
(`/opt/pw-browsers/chromium-*/chrome-linux/chrome`, then `chromium` on PATH).
For each surface the mockup specifies:

- [ ] **Photograph the mockup.** A self-contained HTML mockup needs the plain
  screenshot pass and none of `--check`'s WebGL flags:

  ```
  <chromium> --headless=new --no-sandbox --disable-background-networking \
    --host-resolver-rules='MAP * ~NOTFOUND, EXCLUDE 127.0.0.1' \
    --user-data-dir=target/mockup-shots/profile \
    --window-size=<the surface's reference width,height> \
    --screenshot=target/mockup-shots/<surface>.png \
    file://$PWD/<the mockup>.html
  ```

  (Chromium's dbus complaints on stderr are noise in a container; the line
  that matters is `<n> bytes written to file`.) A mockup that is itself an
  engine build is a page under `dist/`: serve it and let `tools/serve-web
  <name> --check` photograph it (`target/web-check/<page>/check-run.png`).
- [ ] **Photograph the build's same surface with the capture the game already
  has** (§A.7 / §B.6 — ninjo's is `UI.md` §5, eleven PNGs into `screens/`). Do
  not add a second screenshot path. If the mockup specifies a surface the
  capture does not photograph, add that capture: a surface worth mocking up is
  a surface worth a committed picture.
- [ ] **Put the pair in the PR, mockup beside build, one line per difference
  and whether it was deliberate.** A deliberate difference is a decision and
  reads as one; an undeliberate one is a bug you found before the owner did.
  Both are cheap here and expensive after the playtest.

---

## E. The closing checklist — the owner loop

**Every game session's final message enumerates the owner actions it
triggered.** The owner loop only runs on what a session hands back; forgetting
this checklist is a skill violation, not a missed pleasantry. Work all five
lines and state each one, including the ones that do not apply and why.

- [ ] **"Sync now."** Say it whenever the session changed anything the owner's
  sync carries. Note the quirk with it: the Sync button does not always take on
  the first press — press it, then confirm the change actually arrived.
- [ ] **The playtest ask, carrying the wave's question.** When the wave plan
  puts a play judgment after this session, ask for it *and* name the question
  the owner is playing to answer (the GDD's gates state them — wave gates are
  alive-and-correct; the MVP gate is the first fun judgment and has one
  sentence of its own). A playtest without its question comes back as "seemed
  fine". When no playtest is due, say that explicitly — "no playtest this
  time" — so silence is never the message.
- [ ] **"Trigger the sanitation pass"** — when this session closes a wave.
  Wave 1.5 and the asks wave close waves, and so does each wave-final session
  after them. Name the pass type your evidence points at and the FINDINGS
  entries that dispatch it; `docs/templates/SANITATION.md` is the handoff
  template and the four pass types. You do not run the pass and you do not
  create its session — you hand the owner the evidence that one is owed.
- [ ] **Decision surfaces shipped**: one line per row of the handoff's
  decision-surface table (§D.1), naming the test that asserts it — or the one
  line saying the handoff added no decision. A table that was opened and never
  closed out is the same silence one document later.
- [ ] **What the next session inherits**: the deviations, the open findings,
  and anything you decided that the GDD leaves open.

---

This checklist orders the documents; it does not replace them. When this file
and a document disagree, the document is right — fix this file in the same
commit.
