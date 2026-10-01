# Keifu — findings

What the documents cost this port, in the shape `docs/internal/e0-findings.md` uses: what
I was doing, what I expected, what happened, and which document owns it. G-numbers
continue the sequence across games (ninjo's last is G-035).

Spec gaps — questions about *Lineage's* rules — are not here; they are in `SPEC-GAPS.md`.

**Reading discipline, session 1.** Read: `CLAUDE.md`, the `make-game` skill,
`games/keifu/spec/` whole, the four `docs/api/` documents, and from
`crates/jidousha/examples/` the `prototype_kit` files (main, verify, checks, capture) and
`text` (main, verify) for the readability floors. One `docs/internal/e0-findings.md` entry
(F-141) for this file's shape, as the skill directs. **Engine source (`crates/*/src/`):
not opened.** `games/ninjo/` and `attic/`: not opened, except that the heading numbers of
`games/ninjo/FINDINGS.md` were grepped to continue the G-sequence — see G-039.

---

### G-036 — a game whose content is data has no documented way to read it

Class: docs · Session: keifu 1 · Owner: `docs/api/jidousha-api.md` (Concepts, the assets
paragraphs)

**Doing:** loading `spec/content/*.json`, fifteen files of hand-authored content the port
must consume rather than retype, with a loud failure on a schema mismatch.

**Expected:** a sentence saying how a game reads structured data: either "parse it
yourself" or "a game may depend on crate X", and whether it should arrive through
`Assets::load_bytes` or be compiled in.

**Happened:** the API document stops at `load_bytes` ("anything the engine does not decode
itself") and is silent on everything after the bytes. `CLAUDE.md` forbids adding a
dependency without a justification and `cargo tree` delta, and `games/README.md` says a
game "reaches the engine via the facade only", which does not say whether a third-party
crate such as `serde_json` is in bounds for a game. I could not tell whether proposing one
was a human decision.

**What I did:** compiled the files in with `include_str!` (the same bytes on native and
web, nothing to wait for, deterministic) and wrote a JSON reader in the game (~300 lines plus tests)
(`src/json.rs`), with every accessor naming the path that reached it. No dependency added.
That is a defensible answer, and it was still a guess about policy.

**Fix:** one paragraph in Concepts: whether a game under `games/` may take third-party
crates (and the bar for doing so), and the recommended way to ship authored data —
`include_str!` versus `load_bytes` — with the web consequence of each.

### G-037 — "readability floors" are required and are defined nowhere a game can read

Class: docs · Session: keifu 1 · Owner: `docs/api/jidousha-testing.md`

**Doing:** meeting the handoff's "readability floors over every surface" and CLAUDE.md's
definition of done for a game.

**Expected:** the floors stated in the testing document — the minimum type size, and what
"readable" is checked against.

**Happened:** `docs/api/` uses the phrase once, in passing ("`examples/text` ... with its
`--verify` asserting the readability floors"). The floors themselves live in
`crates/jidousha/examples/text/verify.rs`, which cites ADR-0042 §3 — an ADR a game session
is fenced from. The example sets its own minimum (`MIN_TEXT = 11.0`) as a constant of that
specimen sheet, so it is not clear whether 11 world units is the engine's floor or the
example's.

**What I did:** adopted the example's four floors (minimum size, inside its panel, no two
rows overlapping, built-in face ASCII only) plus the camera-bounds check, with a minimum of
14 px (above the example's 11, at one world unit per pixel). All asserted in
`src/floors.rs` over 18 surfaces.

**Fix:** a short section in the testing document naming the floors and the minimum size in
pixels, so a game asserts the engine's number rather than copying an example's constant.

### G-038 — a windowed game cannot hand its own state to `Startup`

Class: docs · Session: keifu 1 · Owner: `docs/api/jidousha-api.md` (`run`, `App`)

**Doing:** making the run seed explicit, recorded state: `cargo run -p keifu -- --seed N`
should found the house on seed N.

**Expected:** a way to put a resource in the world before `Startup` runs under `run`, the
way `headless` allows through `world_mut()`; or a statement that `add_system` accepts a
closure that could capture it.

**Happened:** `run(config, setup)` exposes only `GameConfig`, `IntoSystem` is not exported
and no document says whether a capturing closure is a system. `GameConfig::seed` reaches the
`Rng` resource but nothing else.

**What I did:** the seed rides `GameConfig::seed`; `Startup` reads a `RunSeed` resource if a
check inserted one, and otherwise draws the run seed from the engine `Rng` and records it on
the house — which matches the spec's own "a new house reseeds from a draw of the previous
generator". Works, and the recorded seed is not the number typed on the command line, which
a player reproducing a run will trip on.

**Fix:** say in the `run`/`App` reference whether a closure is a system, or how a windowed
game passes start-up data in.

### G-039 — the G-sequence lives in a game the handoff fences off

Class: process · Session: keifu 1 · Owner: the `make-game` skill (§C) and the keifu handoff

**Doing:** numbering this file's first entry.

**Expected:** the next G-number to be findable without reading another game.

**Happened:** the skill says "G-numbers continue the sequence across games"; the handoff
says "do not open `games/ninjo/`". The only record of the sequence is
`games/ninjo/FINDINGS.md`.

**What I did:** grepped that file's `### G-0nn` headings for the largest number (G-035) and
read nothing else in it. Disclosed here so the fence stays auditable.

**Fix:** keep the last-used G-number somewhere neutral (`games/README.md`, or the skill).

### The game's own

- **No art ships with the spec.** `lore.json` names the original's sprites
  (`character-knight`, `reward-sword`, ...) but no image is in the repository. The engine
  does tint at draw time (`Sprite::tint`, `Color::modulate`, documented), so the
  handoff's tint question is answered yes; with nothing to tint, the hero card draws a
  stand-in figure (a rectangle) coloured by the original's tint rules — dead grey, elder
  grey, wounded red — and otherwise by the vocation's aptitude. A later session that adds
  single-colour sprites tints them with the same function (`summer::figure_tint`).
- **Hover-only facts are shown inline.** The sheet's aptitude notes ("Might 7 of 9, -2 for
  being an elder."), the dream's legacy promise, the heirloom's provenance and the fear's
  effect line are hover text in the original; here they are always on the sheet, because a
  pointer cannot hover over a hover. Presentation is free; the information is all present.
- **The mutation round: 74 of 78 noticed.** 78 one-line faults across every constant
  and every derived quantity, harness-checked (a replace that matches nothing is an error,
  a build that fails is not counted). Of the four the run does not notice, three are
  equivalent (`id < other` to `<=` over distinct ids; a no-op match arm; `rfind` for
  `find` over hit targets that never overlap). One is a real escape: narrowing the
  sheet's column gutter by 6 px, which no founding sheet's text comes close enough to the
  edge to show. Rounds one to three found eleven loose checks first — among them a
  calendar check that a +2-years-per-summer fault passed (it only read odd years) and a
  pip check that returned silently when its row was missing.
- **Confirmation of `jidousha-testing.md`'s mutation passage.** Its warning that a
  search-and-replace matching nothing "writes the file back unchanged and reports
  success" was exact: my own editing script dropped two staged checks that way, and only
  the next mutation round showed it.
- **Small windows.** The layout is in constants for 1280x720 (the API document's
  prototype answer). `tools/serve-web keifu --check`'s 640x480 browser draws it at half
  size, where the 14 px type is 7 px and not readable (`screens/web-640x480.png`). The
  floors hold at the window the game opens at; a phone-width layout is a later session's.
- **0 findings against the capture document.** F-141's paragraph (shapes-and-text games
  need only `create_builtin_textures`) was exactly what this game needed.
