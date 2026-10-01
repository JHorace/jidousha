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

---

## Session 2 (W2 + the art item)

**Reading discipline, session 2.** Read: `CLAUDE.md`, the `make-game` skill, the crate whole
(`spec/` included), from `docs/api/` the asset, sprite, `MemorySource`, `FrameRecorder` and
capture passages; from `crates/jidousha/examples/` `prototype_kit` (main, verify, capture) for
the art half of a capture. The handoff named `docs/internal/assets.md` (read; it does not
describe the depot — G-040) and ADR-0040 (read). The depot `jidousha-assets` (README,
LICENSES.md, `tools/`, contact sheets). **Engine source: not opened.** `games/ninjo/` and
`attic/`: not opened. Three touches to disclose: `grep -o "^### G-0nn"` over
`games/ninjo/FINDINGS.md` to continue the G-sequence (G-039's method); a `grep -l depot`
over the repository listed two file *names* under `games/ninjo/`; and an `ls games/*/assets`
listed the file names in `games/ninjo/assets/`. No contents were read. `games/giri/` does
not exist any more (it is in `attic/`), so no sibling game was read.

### G-040 — the depot's import flow lives only inside a game folder

Class: process · Session: keifu 2 · Owner: the depot README §5, `docs/internal/assets.md`,
and the `make-game` skill

**Doing:** importing sprites "through the flow `docs/internal/assets.md` documents", as the
handoff asked.

**Expected:** the flow in `docs/internal/assets.md`, or a tool under `tools/`.

**Happened:** `docs/internal/assets.md` is the engine's asset-loading design; it never
mentions the depot, roles or `CREDITS.md`. The depot's README §5 says the flow is
"`games/<name>/art/import_pack.py`", and the only such script is inside `games/ninjo/` —
the folder this port is fenced from. The depot's LICENSES.md calls it "the established
import flow" and the only way art enters a game tree.

**What I did:** wrote keifu's own minimal step from the README's *description* of the flow
(role-named, PNG-only inside a size envelope, refuses to run without a licence and source,
writes `CREDITS.md`): `games/keifu/art/import_sprites.py`, which borrows the depot's own PNG
reader. The size envelope (8..64 px) is my choice; I could not read ninjo's.

**Fix:** move the import step to `tools/` (it is not any one game's), document it beside
ADR-0040's asset root, and point the depot README at it.

### G-041 — the API document does not say which PNGs the engine can read

Class: docs · Session: keifu 2 · Owner: `docs/api/jidousha-api.md` (Concepts, the assets
paragraphs; `Assets`, `decode_png`)

**Doing:** loading Kenney Tiny Dungeon tiles straight out of the pack.

**Expected:** "a file that is not a readable PNG resolves Failed" to come with what
*readable* means — bit depths and colour types.

**Happened:** every Tiny Dungeon tile is a 4-bit palette PNG with `tRNS`. `decode_png`
refused all ten: "Four bit depth is not supported; re-export as 8-bit". I learned it from
verify's first run; in the window it would have been ten magenta checkerboards and ten
error lines. The depot's README calls 1/2/4/8-bit palette PNGs what Kenney's packs *are*,
so every game that imports from the depot meets this.

**What I did:** the import step re-encodes each sprite as 8-bit RGBA, texel for texel.

**Fix:** one sentence at `asset_source`/`Assets`: the PNG formats the engine decodes (8-bit
only, it appears) — and, while there, the texture sampling filter (the sprites came out
crisp at 2x and 3x, so it is nearest; nothing said so, and I scaled to whole multiples in
case it was not).

### G-042 — `tools/check-assets` passed while checking none of this game's paths

Class: docs (misled) · Session: keifu 2 · Owner: `docs/api/jidousha-api.md` (the assets
paragraphs) and `tools/check-assets`

**Doing:** the definition-of-done gate "`check-assets` green".

**Expected:** a green run to mean the game's ten `load_texture` paths name files that exist.

**Happened:** the tool only checks loads in a file that *also builds* a filesystem-backed
store (its docstring says so; `docs/api/` does not). I built the store in `main.rs`'s
Startup and wrote the loads in `art.rs`, a natural split. `check-assets` exited 0 and
listed five files, none of them keifu's — a green gate that had looked at nothing.

**What I did on its authority:** nearly reported it green. Then noticed keifu was not in
its list, moved the store into `art.rs` beside the loads (`art::store`), and confirmed the
check now lists `games/keifu/src/art.rs` and fails on a mistyped path.

**Fix:** either say the one-file rule where a game author reads about assets, or make the
tool check every literal load in a crate that builds a file store anywhere.

### G-043 — one recorder across sessions makes a capture's texture ids drift, and the GPU panics

Class: docs · Session: keifu 2 · Owner: `docs/api/jidousha-capture.md`

**Doing:** adding the art half of the capture (`prototype_kit/capture.rs`'s pattern).

**Expected:** that replaying "the same art, uploaded the same way" into a fresh backend
reproduces the recorder's ids.

**Happened:** session 1 records every frame on one `FrameRecorder` across many sessions.
Each session has its own store, and `settle_assets` uploads its art again, so by the
capture the plan named backend texture 69 while a replay had uploaded ten. The id check
the document asks for caught it — and then `WgpuBackend::render` *panicked* ("a frame plan
named backend texture 69 which is not uploaded") instead of returning an error.

**What I did:** each picture is staged in its own session and recorded by its own fresh
recorder, and a drifted id now skips the render with a failed check.

**Fix:** say in the capture document that a recorder whose frame you will capture must have
seen exactly the store the replay recreates — one session — and consider `render`
returning an error for an unknown texture rather than panicking.

### The game's own (session 2)

- **The mutation round: 78 of 80 noticed.** 80 one-line faults over every W2 constant,
  table entry and predicate — bonds (rank rule, mirroring, `since`, self-bonds, the change
  guard, steadying), fear (penalty, bonus, refusal, the dread rule's three gates, cap and
  break, breaking's scar, deed and pronoun, conquering, shedding), grief (every branch,
  bearers, shared kinship, whose side of the bond), destinies (every shield, claim, mend,
  the Door's +5, may-still-learn, claimed, speaking) and the power sum and fear line —
  harness-checked as session 1's was (a mutation that matches other than once is an error;
  one that does not build is re-cut until it does). The instrument is `tools/test`'s two
  halves: `cargo test` noticed 78, verify alone 42. **The two escapes are equivalent:**
  `steadying_companion`'s `other != member` is redundant because no hero holds a bond to
  themselves (`form` refuses one, a mutation the round does notice); and `add_dread`
  passing an amount of 0 adds nothing and cannot break, since dread 5 is only ever reached
  by breaking. No loose check was found; the rules were written with their tests.
- **W2 lands the power sum early.** The W2 oracle reads "you bring 4", so `power.rs` sums
  SPEC §6 for a party (lines 1-6, the floor, bonds, patrons) and builds the card's fear
  line. W4 owns the quest model, the line-by-line breakdown and the forecast, and extends
  these two functions rather than writing its own. Line 7 (blessings) needs W3's blessing
  scope; until then a blessed hero in a power sum panics, loudly — nothing blesses anyone
  before W3.
- **No §19.1 reading was stubbed.** Session 1's sheet already read conquered, broken, born
  brave and settled from state; W2 makes that state reachable through the real rules, and
  verify's staged story reads the result off the sheets (Maren broken at the Coast and
  bearing her father's death; Odo conquered; Pip spoken a destiny).
- **The picks** (`screens/w2-picks.png`): Kenney Tiny Dungeon, one 16 px family. Knight:
  full helm. Warrior: horned helm. Ranger (the original's "guard"): green headband. Scholar:
  the robed woman — the only unarmoured, unbearded civilian left; heroes of either pronoun
  wear it, as the original's sprites were worn. Priest: the bald, bearded friar. Sage: the
  wizard. Elder of the fighting callings: the grey-haired veteran. Elder of the learned: the
  hooded hermit (red-eyed — the pick I am least sure of). Child: the plain, unarmed youth,
  drawn at two-thirds size because the pack has no child. Blade: the long sword.
- **The tint rules are unchanged; the stand-in's colour is retired.** Dead grey, elder grey,
  wounded red, as session 1 built them. Session 1's rectangle took its vocation's aptitude
  colour otherwise; multiplied into a coloured sprite that would stain it, so "otherwise"
  is now untinted, which is what §5.4 lists.

---

## Session 3 (W3)

**Reading discipline, session 3.** Read: `CLAUDE.md`, the `make-game` skill, the crate whole
(`SPEC-GAPS.md`, `FINDINGS.md`, `src/`), and from `spec/` MODULES.md, SPEC.md §0-§9,
§12-§16 and §17-§23, CONSTANTS.md's dream and legacy rows, OPEN-QUESTIONS.md's dream
entries, `content/README.md`, and the content files W3 reads (`dreams.json`,
`legacies.json`, `household.json`, `quests.json`'s Barrow templates, the dream, legacy and
bond keys of `lines.json` and `ui-text.json`). From `docs/api/`: the testing document's
mutation passage, for the harness. **Engine source: not opened.** `games/ninjo/` and
`attic/`: not opened; `grep -oh "^### G-0[0-9]*" games/*/FINDINGS.md` read the G-headings
only, to continue the sequence (G-039's method). No sibling game read.

### G-044 — the handoff's mutation harness was never committed

Class: process · Session: keifu 3 · Owner: the `make-game` skill (§A.6, §B.5) and the keifu
handoff

**Doing:** "every W3 constant, table entry and predicate mutated at least once through the
session-1/2 harness".

**Expected:** the harness in the repository — the testing document's two hard errors built
in, and the session's mutation list beside it, so a later session reruns the old faults and
adds its own.

**Happened:** neither session committed it. FINDINGS describes it ("a mutation that matches
other than once is an error; one that does not build is re-cut") and nothing else of it
exists: not under `games/keifu/`, not under `tools/`. The lists of sessions 1 and 2 (78 and
80 faults) are gone with it, so their rounds cannot be rerun against W3's changes.

**What I did:** wrote `games/keifu/mutants/mutate.py` from the testing document's passage and
session 2's description — a find that matches other than once stops the run before
anything is written, a mutation that does not build is reported NOT BUILT and never counted,
each file is restored from the bytes read, the tree is checked clean after — with `--jobs N`
running the list over N worktrees of HEAD (a round of 118 takes about five minutes). The W3
list is `mutants/w3.txt`. Both are committed.

**Fix:** a line in the skill's §A.6: commit the harness and every round's list with the game
— a mutation round that cannot be rerun is evidence for one commit only.

### docs/api: 0 findings

W3 is pure rules over the game's own state. The only engine surfaces it touched — a
`HeadlessSim`'s world, the per-session recorder of the capture path — are the ones sessions 1
and 2 already used, and the capture document's rule as G-043 corrected it (one recorder per
picture's session) carried the fourth picture without change. Nothing was asked of the
documents that they had not already answered.

### The game's own (session 3)

- **"Grave goods" carries the Dark, not the Undead.** The handoff's "a Barrow/Undead quest"
  is two quests in `quests.json`: the year-1 card quest "Grave goods" (Dark only) and "The
  lamps in the Barrow" (Dark, Undead). Garrick's current stage asks only the Barrow and a
  triumph, so both call him ("He must triumph"); his rest adds +2 on the lamps and nothing on
  grave goods. Verify asserts all four, and prints the vector for "Grave goods", the quest
  W4's oracle seats him on.
- **The mutation round: 117 of 118 noticed.** 118 faults over every W3 constant
  (blessing powers, blade count, the Court's renown, the three stage goals and the default),
  every lore table entry W3 reads, every predicate of SPEC §9.1, witnessing (own first, one
  stage, counting, settling, the lines), legacies (dreamer and fulfiller, the three heirlooms,
  the tale titles, the record and the deed), the heir of the blood and its fallback, blessing
  wording, recipients and reach, power line 7, every step of the dream call, the card and
  sheet lines, and dream rivals. Round one: 106 of 117 noticed, one not built (re-cut to build); the final round reran the
  whole list against the tightened checks. Ten
  escapes were loose checks, every one a test whose hero masked the rule — a childless test
  hero who was also wed, a setup that was always the Coast, an owned dream with no " my " in
  its title, a ring made by its own dreamer, no living spouse, no student with full hands, no
  dead dreamer, kin who also held a bond — so the rules were written right and their tests
  were not yet discriminating. One of the ten fixes did not land: rustfmt had rewrapped the
  line my edit searched for, and the next round showed the escape again — the testing
  document's silent-miss warning, met in a hand edit. **The escape left is equivalent:** K7
  (`needed.is_some() || going` to `needed.is_some()`): when the dreamer is seated, step 5's
  absent moment *is* step 4's moment (the party already holds them), so it finds nothing and
  the call is `None` either way. The instruments: `cargo test` alone noticed 115,
  verify alone 48; two only verify saw (a fulfilled dream asked its stage panics there first;
  the sheet's blessing-effect line is read only off the staged sheets).
- **Two heirloom sprites are not imported.** W3's rules can forge a road-book
  (`reward-guidebook`) and a cradle-ring (`reward-ring`); session 2 imported only the blade.
  The sheet panics loudly if it is asked to draw an heirloom with no imported role
  (`summer.rs`), so nothing goes quiet, but no live source makes one before W6-W8. Verify
  reads those sheets as lines, not pictures. The session that wires a source imports the two
  sprites (the depot was not reachable from this session's container).
- **LEAVES shows the latest legacy only.** A hero who fulfils two dreams (verify's Garrick:
  his own, then Brannoc's blade as a burden) records the second; §3.2 gives a hero one
  legacy field. Not a gap — the epitaph's LEFT part (W9) reads the same field.
- **BLESSED shows the title and its effect** ("Garrick's rest" / "+2 against Undead"), the
  way HEIRLOOM shows a name and its effect; presentation only.
- **The capture has a fourth picture,** `keifu-w3-garrick.png`: Garrick after the oracle's
  triumph and the blade — settled (the gold pip on his card), "Fulfilled." twice, Emberwake in
  hand with Thornfall gone to Maren, BLESSED "Garrick's rest". Committed as
  `screens/w3-garrick-settled.png`.
