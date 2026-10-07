# Stack Duel — findings

What the documents cost this build (yakin task `stack-card-game`, V1). G-numbers continue
the sequence across games (the last in `games/*/FINDINGS.md` is G-069).

**Reading discipline.** Read: `CLAUDE.md`, the `make-game` skill, `docs/api/jidousha-api.md`
(Quickstart, Concepts, Input, Text, Rect/Camera references), `docs/api/jidousha-testing.md`
(input scripts, recorder, closing convention), `docs/api/jidousha-controllers.md` whole, and
`crates/jidousha/examples/prototype_kit/` (main, verify, checks, capture). Not opened:
`crates/*/src/`, `docs/internal/`, `docs/adr/`, any other game, `docs/api/jidousha-ui.md`
(the game's chrome is plain rectangles and text; the kit was not used — see G-070).

---

### G-070 — the UI kit was skipped on a game with a lot of chrome

Class: game's own · Session: stack-card-game 1 · Owner: this game

**Doing:** building a panel, a log feed and a hand of cards. `make-game` A.2 says a game with
chrome builds on `jidousha::ui` from the first screen.

**Expected:** n/a — this was a choice.

**Happened:** the yakin tick drew every panel with `ctx.rect`/`ctx.text` through its own
`ui::build` screen model, which is checked by `layout.rs` at three aspect ratios. The kit's
floors and `judge_frame` were not applied. The next session on this game should port the
three panels onto the kit; the model/draw split makes that a change inside `ui.rs`.

### G-071 — `HeadlessSim` is not under `jidousha::testing`

Class: docs · Session: stack-card-game 1 · Owner: `docs/api/jidousha-testing.md`

**Doing:** a `Session` struct holding the sim, importing from `jidousha::testing::{..}`
alongside `FrameRecorder`.

**Expected:** the type that `headless(..)` returns, which the testing document is about, sits
in the same module as the rest of the testing vocabulary.

**Happened:** `no HeadlessSim in testing`; it is in the prelude. One compile error, one
edit. **On the doc's authority:** I wrote the import from the document's section order, not
from a signature.

### Not a finding — the shapes-only capture path

`capture.md` and `prototype_kit/capture.rs` carry the shapes-only path; it worked as written
(built-in textures only, font id checked). No finding — noted so the next reader does not
look for a gap.

### Game-design notes the next session inherits (the game's decisions)

- Balance is by `--verify`'s sweep: blind 0/40, power-only 0/40, skilled (one-ply search over
  `preview`) 28/40 at the time of writing. The power-only player never wins, so the game
  has no "naive player nearly wins" tier; a human who plays only damage will lose every
  match. If that is too harsh, soften the NPC (`npc.rs`) before touching card numbers.
- The NPC has no randomness and holds mana open; it can be read. A second NPC deck is the
  obvious replay lever.
- Nobody played this build. `tools/serve-web --check` drove it in a browser headlessly.
