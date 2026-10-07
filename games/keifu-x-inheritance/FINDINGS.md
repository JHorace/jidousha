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

---

## Session 4 (W4)

**Reading discipline, session 4.** Read: `CLAUDE.md`, the `make-game` skill,
`docs/templates/DECISIONS.md`, the crate whole (`SPEC-GAPS.md`, `FINDINGS.md`, `src/`,
`mutants/`), and from `spec/` MODULES.md, SPEC.md §0-§8 and §14.4, CONSTANTS.md whole, and
the content W4 reads (`quests.json`, `ui-text.json`'s `quest_card`, `quest_sheet` and
`summer`, `bonds.json`'s pairs, `lore.json`'s places and counts, `household.json`). From
`docs/api/`: the API document's input passages (`Input`, the touch mirror, `PointerState`)
and the testing document's scripted-input passages (`InputScript`, `SnapshotBuilder`,
`InputEvent`, `FingerId`, the pointer-click example). **Engine source: not opened.**
`games/ninjo/` and `attic/`: not opened; `grep -oh "^### G-0[0-9]*" games/*/FINDINGS.md`
read the G-headings only (G-039's method). No sibling game read.

### G-045 — a drag cannot be written with the scripted-input type the testing document teaches

Class: docs · Session: keifu 4 · Owner: `docs/api/jidousha-testing.md` (scripted input)

**Doing:** "drive the drag itself through the scripted pointer" — a press on a card, moves
with the button held, a release over a seat.

**Expected:** `InputScript` to hold a pointer button over a range of ticks, the way `hold`
holds a key; or the document to say how a drag is scripted.

**Happened:** `InputScript` has `pointer_at` and `click`, and `click` is a tap ("pressed,
held, and released, all on that one tick"); there is no pointer `hold`. The document's
pointer passage shows only a click. `SnapshotBuilder` is presented for a keyboard controller
("send events, not states"), and nothing says that `InputEvent::ButtonPressed` and
`PointerMoved` fed through it are how a held pointer is scripted.

**What I did:** read `SnapshotBuilder` and `InputEvent` in the reference, inferred that the
builder is the general path, and wrote `src/scripted.rs` on it: press (move + button down),
moves on later ticks, release. It worked first time, so the gap cost reading rather than
debugging.

**Fix:** a short paragraph after the pointer-click example: "a drag is a press, moves and a
release over several ticks — feed `InputEvent`s through `SnapshotBuilder`", with five lines
of code; or `InputScript::hold_button(button, ticks)`.

### G-046 — where the touch mirror and focus loss happen, as a test sees them

Class: docs · Session: keifu 4 · Owner: `docs/api/jidousha-api.md` (the touch paragraphs)
and `docs/api/jidousha-testing.md`

**Doing:** asserting the handoff's "the touch mirror makes it tappable for free" by dragging
with a finger in verify, and undoing a drag the system takes away.

**Expected:** to know whether `Touched` events fed through `SnapshotBuilder` move the primary
pointer (the mirror), or only the windowed driver does; whether losing focus releases held
buttons; and what `InputSnapshot::new()` reports for `window_focused` (only
`SnapshotBuilder::new` is documented as focused).

**Happened:** the API document says "the engine puts the first finger down onto the primary
pointer" without saying which layer does it, and is silent on focus and held buttons.

**What I did:** tried it: a finger fed through the builder drags exactly as the mouse does,
and a `Cancelled` touch reaches `touches()`. The game treats `!window_focused()` or a
cancelled touch, mid-drag, as the drag taken away, and only reads focus while something is in
hand — so whatever `InputSnapshot::new()` reports cannot matter. Verify drives both
(`w4::check_drags`).

**Fix:** one sentence each: the mirror is the builder's (so tests get it), what focus loss
does to held buttons, and `InputSnapshot::new()`'s focus.

### G-047 — the W4 handoff carried no decision-surface table

Class: process · Session: keifu 4 · Owner: the keifu handoff template, and `make-game` §D.1

**Doing:** starting W4, whose live drag is the port's first player decision.

**Expected:** the handoff's decision-surface table, or the line "Decisions: none new".

**Happened:** neither. The handoff specified the card's contents and the drag closely, which
nearly is the table, but §D.1 makes the omission a stop.

**What I did:** drafted the one-row table from the handoff and SPEC §5.3-§6 (which heroes to
send on which quest; the card, previewed live, and the quest sheet; drag and release; one
function `power::party_power` + the forecast; asserted by the W4 oracle's drags and the
floors) and asked the owner, who approved it as drafted. The PR carries it.

**Fix:** the keifu handoffs for W5-W10 carry the table from the start; W6 ("Set out") and W7
(the hearth) each add a decision.

### docs/api: 2 findings

G-045 and G-046 above. Everything else W4 asked the documents (pointer positions through the
camera, the per-session recorder of G-043, the floors' text measurement) they had answered.

### The game's own (session 4)

- **W5 scaffold.** Year 1's board is the two forced opening quests, each posted from its own
  template by the real stakes formula at its place's trouble, in place order
  (`House::post_board`, marked `W5 SCAFFOLD`). It draws four numbers from the run's generator
  at founding (calm seats, then the wobble, for each), so session 1's recorded
  "another house" seed was re-recorded (`sessions.rs` `SEED_AFTER_SEVEN`). No other year has
  a board; W5 replaces the function whole.
- **The board fills the right panel and the sheets are raised over it.** The hero sheet needs
  about 520 px of height and the board two rows of 300; there is not room for both. A sheet
  is raised while a hero or a quest is pointed at and nothing is in hand, and lowered by a
  drag, so the card a hero is held over is always on screen (the original's sheets are
  pop-ups too). One consequence: "<watched> will not go." can only be read while the hero is
  in hand, because pointing at a hero raises their sheet over the board.
- **Pictures** (`screens/`): `w4-drag.png` — Garrick seated on "Grave goods", Brannoc in
  hand over the card, which previews him in the second seat and reads "Needs Might 9", "you
  bring 12", "Succeed 92%", "triumph 42%" (CONSTANTS §3 at +3); `w4-quest-sheet.png` — the
  sheet behind it, the §6 lines with Thornfall's +1, each outcome's odds, the Barrow's
  history panel; `w4-worst-sheet.png` — the staged worst case (trouble 2, three seated, two
  patrons), whose sheet runs on into the right column above its history panel.
- **The help line lives in the board's empty slots.** Year 1 posts two quests, so it fits
  below them. W5's full board of four leaves no room for it — W5 needs to give it a home.
- **The preview and the drop read one function.** `House::landing` answers "where would
  they land" for both, so the card cannot preview a seat the release will not give
  (SPEC-GAPS KG-28 records what that settles).
- **The breakdown and the sum are asserted equal.** `power_lines::party_lines` builds §6
  line by line from the same pieces `power::member_power` sums, and panics if the two ever
  disagree, rather than rewriting session 2's sum.
- **The mutation rounds.** W3's list rerun against W4's code (`93411c7`): 117 of 118
  noticed (tests alone 115, verify alone 48), the same equivalent escape as session 3 (K7).
  W4's own list, `mutants/w4.txt`, is 134 faults: every W4 constant (CONSTANTS §3 and §4),
  the banding, the dice, the rounding and the card's four sums; the stakes formula line by
  line; the board, its seats, landing, swap and refusal; the card, the sheet, the breakdown
  and the history; the pointer and the drawn card; the content W4 reads; and CONSTANTS §3's
  table entry by entry — each of its 21 rows, and the oracle's three, as verify ships them,
  so a row the instrument stopped comparing would show. Round one: 131 of 134 noticed, none
  unbuilt. Two escapes were loose checks, both a state the founding never reaches: no
  founding party changes a Dream: line on the opening quests (K12 — now a staged
  stay-behind call, Odo and the student he taught), and nothing read Garrick's breakdown
  away from Might, where Thornfall must not count (P2 — now W2's bell read line by line).
  Round two, the whole list again: **133 of 134 noticed** (tests alone 85, verify alone
  120). **The escape left is equivalent:** K8, "Room for N" told with the calm seats —
  that line shows only on an untroubled quest, where the seats are the calm seats.

---

## W4 review feedback: the sheet dock (owner, 2026-10-01)

Presentation only, outside the spec's jurisdiction: no rule, constant, content string or
spec file moved, no SPEC-GAPS entry and no new G-number. **Reading discipline:** `CLAUDE.md`,
the `make-game` skill, the crate whole; from `docs/api/` the `Camera` reference, the
"layout in constants" passage, `PointerState::scroll` and `InputEvent::Scrolled`. Outside
the fence, disclosed: `tools/serve-web` and `tools/web-template/index.html`, read to learn
what canvas the web check gives the page, and a headless Chromium dump of the built page's
DOM that measured it (640x329 in the check's 640x480 window). **Engine source: not opened.**

### The game's own

- **The sheets have a dock; nothing is raised over anything.** This supersedes session 4's
  "the board fills the right panel and the sheets are raised over it". The summer screen is
  four regions that never overlap: the household and yard (cards 84x84, the figure over the
  name over the dread pips), the board (2x2 cards of 314x319), the top bar over those two
  (58 px), and the dock — the screen's full height at its right edge, 344 px wide
  (`summer::SHEET`, `dock.rs`). The dock holds the sheet of the hero pointed at, of the
  hero *in hand* mid-drag, or of the quest pointed at with its history; with none, the help
  (which had been squeezed into the board's empty slots and had no home in a full board).
- **Right, not bottom, and why.** Mid-drag is the critical state, and the card being held
  over grows during a drag (the live preview adds "you bring", fear and refusal lines) — a
  bottom dock takes its room from exactly those cards. Both sheets are also a single column
  of lines, so a tall narrow dock reads and scrolls as a document; a short wide one would
  have had to flow the sheet into columns that then scroll. Measured before choosing: the
  longest founding sheet (Garrick) is 50 wrapped rows at 314 px and 42 at 408, so a wider
  dock buys little.
- **A long sheet scrolls by whole lines.** `UiState::dock_first` is the first sheet line
  drawn; a line is drawn whole or not at all, a scrollbar shows above and below, and the
  wheel, a mouse grab or a finger moves it. Resting on the dock keeps its sheet open; a new
  subject opens at its top; a hero dropped on the dock returns. A sheet line taller than the
  dock panics rather than vanish. **The wheel's sign is assumed** (positive lines = wheel
  away from the player = toward the top); the API document does not say which way
  `PointerState::scroll` runs. Owner: please confirm in the playtest.
- **The W1 oracle was layout-coupled.** It read Garrick's whole sheet off one screen, which
  only held while the sheet fitted its panel. It now pages through the dock with the wheel
  (`verify::dock_pages`, `dock_read`: each sheet line once, on the page that drew it) and
  asserts the same twenty lines, the same order, the same bonds and pips, each drawn. No
  other oracle changed.
- **Floors at three sizes.** Native 1280x720, the web check's 640x329 canvas, and a 4:3
  window (1024x768), each with every long sheet paged through and judged per page. The
  camera now fits the page to the window's shape (`screen::fitted`), so a window narrower
  than 16:9 shows the whole page instead of cutting the household and dock off its sides.
  **The floor is in world units.** At 640x329 the 14-unit type is 6.4 device px — session
  1's "Small windows" finding, unchanged by this work and still a later session's.
- **Pictures** (`screens/`): `dock-idle.png` (the help in the dock), `w0-w1-garrick.png`
  (a hero sheet open, scrollbar showing), `w4-quest-sheet.png` (a quest sheet with its
  history), `w4-drag.png` (Brannoc in hand over "Grave goods": the card's "you bring 12" and
  his previewed tile, the hand, and his sheet in the dock), `dock-scrolled.png` (Garrick's
  sheet scrolled to its end); the other W1-W4 screens re-taken on the new layout.
- **The mutation round.** `mutants/dock.txt`, 22 faults over the dock, its scroll, the
  pointer's new paths, the layout regions, the camera fit and the redrawn cards. Round one:
  21 of 22 noticed. The escape, K19 (the dock's text rect widened under the scrollbar), was
  a loose check of the very kind §A.6 warns of: the floor judged rows against
  `dock::text_rect()`, the rect the fault moved. It now measures against the scrollbar's
  lane, which is set from the dock's edge; K19 rerun: noticed — **22 of 22**. W4's pointer
  and board-view entries (U1-U5, V1-V5; V4 and V5 now in `dock_lines.rs`) rerun against
  the new code: 10 of 10.

---

## Session 5 (W5)

**Reading discipline, session 5.** Read: `CLAUDE.md`, the `make-game` skill, the crate whole
(`SPEC-GAPS.md`, `FINDINGS.md`, `mutants/`, `src/`), and from `spec/` MODULES.md, SPEC.md
§0, §5-§6, §9.6, §14.4 and §22, CONSTANTS.md whole, OPEN-QUESTIONS.md's OQ-29 and OQ-33,
`content/README.md`'s brace placeholders, and the content W5 reads (`quests.json`,
`ghost.json`, `lore.json`'s places, `ui-text.json`'s `quest_card.ghost` and `summer.help`).
`docs/api/`: nothing new opened — W5 used only engine surfaces earlier sessions had read.
**Engine source: not opened.** `games/ninjo/` and `attic/`: not opened, and not grepped
either — this session files no G-number, so the heading grep (G-039's method) was not needed.
No sibling game read.

### docs/api: 0 findings

W5 is rules over the game's own state — planning, reading, easing, memory — plus one more
card line and one more picture. Every engine surface it touched (a `HeadlessSim`'s world,
the page's rows and targets, the per-session recorder of G-043, the floors' measurement)
was one sessions 1-4 had already read and used. The documents were asked nothing new.

### The game's own (session 5)

- **The scaffold is gone; every summer has a board.** `House::post_board` is now board
  generation (`generation.rs` — plan, choose, finish; `reading.rs` — likely parties, the
  pair reading, the dreamer who could go; `easing.rs`). A quest now carries its source
  (template or ghost), title and premise, so a ghost's quest reads on the card and sheet
  like any other (`quest.rs` `Source`, `stakes`, which the template and the ghost share).
  What generation decided is kept as `House::board_report` for the checks; nothing in play
  reads it. The Door's summer refuses to generate a board, loudly — it is W10's.
- **The recorded seed, re-recorded.** The founding now generates year 1's board — up to
  sixteen planned boards, each drawing place, template, seats and wobble per drawn quest,
  seats and wobble per forced one — so the draw "another house" makes after a house founded
  on seed 7 moved again: `SEED_AFTER_SEVEN` 0x5504_a624_1e60_5676 → 0x52ec_97b2_4d27_2e97
  (`sessions.rs`). It is the only recorded draw in the crate.
- **"Its summers", before W6 and W8.** Nothing resolves a quest or turns the year yet, so
  the shape battery runs years 1-25 on the founding household *as founded*: each summer
  advances the calendar and prepares the board from the run's own generator, the template
  memory carried. Demand creeps with the year as in play; the household does not grow, age
  or die. That is a harder house than the original's (whose heroes learn), and the numbers
  should be read that way.
- **The shape, and its bound.** The spec gives no rate for "most", so verify asserts the
  conservative reading — strictly more than half of the battery's summers — fixed before
  the battery first ran. It measured 568 of 600 (94.7%): every summer of years 1-14 carries a
  fair-chance Dream: mark on a card, and the rate falls to 71-92% in years 19-25, where the
  founded household meets demand +3 and +4 per seat. The mark is read off the card's own
  "Dream:" line and each named dreamer asked the board reader's could-go test, so it is the
  card the player reads, after easing. The reader's own call on the kept board, before
  easing, is printed beside it (516 of 600).
- **The loop's rules, asserted over the battery** and printed: sort violations 0, template
  repeats 0 (of 2,256 drawn places with a remembered template), memory mismatches 0,
  welcome-rule breaks 0 (the loop stops at its first score-3 plan, else runs 16; the kept
  plan is the first at the best score; the score matches the formula written as literals),
  easing out of rule 0 (it engages exactly when the kept answerability is under 18 of 36,
  touches only the remembered pair, and stops for the reason it says). Easing engaged in
  253 summers and always stopped with both answerable; the floor and the limit never
  occur on a founded household, so a staged all-wounded house in year 25 floors on every
  board in verify, and the limit is asserted on crafted boards in `easing.rs`'s tests.
- **The W4 oracle now reads "lower only if the board was eased".** It asserts the demand
  plus what easing took off Grave goods is 9, 10 or 11. Across its 27 seeds easing never
  touched Grave goods: the founding household answers year 1 easily.
- **The ghost slot, staged.** Two dead heroes' ghosts at Emberfall and the Deepwood, year 2:
  the first is offered on every board as "Lay Garrick's ghost" (Spirit, two seats, danger 2,
  demand 10, the card's "The ghost of Garrick" in place of a Dream: line), the second never;
  a ghost at a place a forced quest holds waits; a ghost's quest writes no template memory.
  Pictured: `screens/w5-ghost.png`.
- **The help stays in the dock.** A full board left no room for session 4's empty-slot help
  and the dock fix had already moved it; the floors now assert, on every board they judge,
  that the idle dock reads the help alone, all four cards are drawn, and no row outside the
  dock carries the help. `screens/dock-idle.png` is year 1's full board.
- **Floors over the full board, at the three sizes** (native 1280x720, the web check's
  640x329, 4:3 1024x768): four cards seated and every quest sheet; every one of the 24
  templates on a card, as eight staged boards of four, with each sheet; the ghost's card
  and sheet. 77 surfaces per size.
- **Picking a template "excluding the remembered one" is `fresh_index`.** SPEC §22.1's
  primitive is exactly that distribution (uniform over the others); the spec asks the port
  for matching distributions, not sequences.
- **The mutation rounds.** Round one ran all four lists against W5's first commit
  (`b4191e4`) in one pass, 333 faults: **325 noticed** (tests alone 249, verify alone 233),
  none unbuilt. `w3.txt` 117 of 118 and `w4.txt` 133 of 134 — the escapes are their
  sessions' known equivalents, W3's K7 and W4's K8 — and `dock.txt` 22 of 22. Ten `w4.txt`
  entries named code W5 moved (the stakes formula now in `quest::stakes`, the scaffold's
  board in `generation.rs`, the card's dream line, the sheet's premise); each was re-cut to
  the same fault at its new site and is marked "(re-cut s5)" — B1, which posted the
  scaffold's board in year 2, is now "the opening quests are forced in year 2", the
  nearest fault that survives every year having a board. `w5.txt` is 59 faults: every W5
  constant (both thresholds, the attempts, the easing limit, the score's two weights, the
  ghost's seats, danger and aptitude), the score's cap and scale, every planning step
  (forced quests, the ghost's slot, place and template draws, the memory), the keep rule,
  the welcome stop and the welcome rule, the memory's writing, the sort, every line of the
  likely party, the pair reading, the dreamer who could go, every easing step, the ghost
  quest and its card line, and the Door's refusal. Round one: 53 of 59. Five escapes were
  loose checks, all in `reading.rs`, all found by the round and not by drafting: a
  refuser the fear's own penalty already kept out of the party (R2), nothing asking that a
  pair's second party excludes the first's (R8), no refusing dreamer (R14), no dreamer
  whose own party is weaker than the house's best (R15), and no board whose calling quest
  is not the first (R17); each now has a test that fails under its fault. Round two, the
  whole `w5.txt` against the tightened checks: **58 of 59 noticed** (tests alone 51,
  verify alone 38). **The escape left is equivalent:** R12, a "stay behind" call counted
  as one who could go. The board reader asks a call with an empty party, and with nobody
  seated no call can be "stay behind": the only predicate an absent hero meets is a
  student faring alone (§9.1), which needs the student in the party, and the predicates
  that hold at any moment are excluded earlier, at step 2 of §9.6.

---

## Session 6 (W6)

**Reading discipline, session 6.** Read: `CLAUDE.md`, the `make-game` skill,
`docs/templates/DECISIONS.md`, the crate whole (`SPEC-GAPS.md`, `FINDINGS.md`, `mutants/`,
`src/`), and from `spec/` MODULES.md, SPEC.md §0-§9, §14-§15 and §18-§23, CONSTANTS.md whole,
OPEN-QUESTIONS.md whole, ENGINE-USAGE.md §1-§5, INVENTORY.md's telling rows, `content/README.md`,
and the content W6 reads (`lines.json`'s summer, quest, fate, death, crown, fear, bond, ghost,
destiny and deed keys; `ui-text.json`'s `telling`, `summer` and `ending`; `writing.json`;
`quests.json`'s endings; `ghost.json`; `door.json`'s closed verdict; `lore.json`'s places,
outcomes and `age_at_death`; `legacies.json`'s `ghost_tale_title`). From `docs/api/`: the API
document's `Time`, `Seconds` and fixed-timestep passages, for the typewriter. The depot
`jidousha-assets` (cloned sparse: `tools/`, Tiny Dungeon, Tiny Town, Tiny Battle, Tiny Farm,
Micro Roguelike), its contact sheets rendered with its own `tools/contact_sheet.py`. **Engine
source: not opened.** `games/ninjo/` and `attic/`: not opened; `grep -oh "^### G-0[0-9]*"
games/*/FINDINGS.md` read the G-headings only (G-039's method). Two incidental touches, disclosed:
`tools/test` and `tools/check-assets` print other games' file names in their reports (ninjo's
captures, `games/ninjo/src/sprites.rs`), and `docs/templates/DECISIONS.md`'s worked row is
ninjo's. No sibling game read.

### G-050 — the W6 handoff's decision table has four of the template's six columns

Class: process · Session: keifu 6 · Owner: the keifu handoff template, `make-game` §D.1

**Doing:** closing out the handoff's decision-surface table (§D.1, §E).

**Expected:** `docs/templates/DECISIONS.md`'s six columns: decision, must know, surface, action,
one function, asserted by.

**Happened:** the table has four — decision, surface, mechanics, asserted by. "Must know",
"action" and "one function" are not there; "mechanics" names the rules instead. §D.1 stops a
session only for a handoff with *neither* the table nor the "none new" line, so this one is not
malformed, and its one row ("Set out") is clear enough to build from.

**What I did:** built from it, and filled the three missing cells from the spec and the build in
the PR's copy of the table: must know — every card's facts as W4 and W5 show them, and the
control's own label ("Set out" / "Stay home"); action — a press on the control in the top bar;
one function — `resolve::set_out`, whose dice go through the forecast's `margin` and `outcome`.

**Fix:** the keifu handoffs copy `DECISIONS.md`'s header row as it stands.

### G-051 — "rerun all five lists" where four exist

Class: process (misled) · Session: keifu 6 · Owner: the keifu handoff

**Doing:** the mutation bar: "rerun all five lists, then your own round".

**Expected:** five committed lists under `mutants/`.

**Happened:** there are four — `dock.txt`, `w3.txt`, `w4.txt`, `w5.txt` (sessions 1 and 2's lists
were never committed, G-044). **What I did on its authority:** looked for a fifth before reading
G-044 again; then reran the four, with W6's own as the fifth list of the round.

**Fix:** count the lists in the handoff from `ls mutants/`, or name them.

### docs/api: 0 findings

The one new engine question W6 asked — how render-side pacing reads time without the sim
seeing it — `Time` answers: `tick` and `fixed_dt` are on the resource every system and every
`HeadlessSim` has, so the typewriter is ticks since its leaf opened times `fixed_dt`, read in the
page projection and nowhere in the house. Everything else W6 touched (the page, its rows and
targets, the per-session recorder of G-043, the scripted pointer of G-045) earlier sessions had
read and used.

### The game's own (session 6)

- **The scene is a projection of the house.** The summer, the telling and the closed house are
  not a state machine of their own: the page shows the telling while `House::telling` is set, the
  closed verdict once `House::closed`, the summer otherwise (`screen::page`). So the screen can
  never disagree with the house, and a replay of the same presses reaches the same screen.
- **The typewriter is pacing only.** `UiState::typing_from` (the tick a leaf came on screen) and
  `revealed` are presentation state; the house never reads them, and the sim is the same whether
  a story typed for a second or a minute. Verify asserts the constant as a literal: nothing on
  the tick the leaf opens, 45 letters 30 ticks later.
- **W7/W8 SCAFFOLD.** Leaving the telling of an open house runs `season::pass_the_year`: winter
  begins and ends, the year counts on, the next summer's board is generated and the household
  reseated. Nobody ages, rests, learns, dies of age, is born or arrives, and `House::mourned`
  keeps W6's dead for W8's death pages. W7 and W8 replace the function whole. A house played
  this way only shrinks — the battery's houses close within six summers more often than not,
  which is the scaffold's and not the rules'.
- **W10 SCAFFOLD.** A house whose renown is spent closes when its telling is left; the closed
  screen (`ending_view.rs`) shows `door.closed_title`, `door.closed_verdict`, "It was year Y,
  with the Door still R years off." and "Begin another house", so a closed house is not a dead
  end. W10 replaces it whole. The Door's summer still refuses to generate, loudly.
- **The sheet dock reads the telling.** Pointing at a member's card, or at a line that names
  someone, opens their sheet beside the page — the help line `ui.telling.help` promises both.
  Which hero a line is "about" is the first of the house's heroes the line names
  (`telling_view::about`), presentation only; the members' cards show them as they are *now*,
  after the summer (a settled pip, a wound's tint), as "what they are now" says.
- **The set-out control sits in the top bar,** left of "The family". The first draft crowded
  the Door's countdown line; the floors now refuse type within 4 px of any control, and the
  mutation round carries the crowding as a fault (L4).
- **Heirloom sprites: imported, not from the blade's family.** Tiny Dungeon (the cast's pack) has
  no book and no ring, and neither has Tiny Town, Tiny Battle or Tiny Farm. Both came from
  Kenney's Micro Roguelike, 8 px, coloured: the road-book is its scroll (`tile_0077`), the
  cradle-ring its gold ring with a pink stone (`tile_0089`). On the sheet they draw at 4x in the
  blade's 32 px box, so their pixels are twice the blade's — the one visible seam in the art.
  The import flow is G-040's (`art/import_sprites.py`); `CREDITS.md` has both rows;
  `check-assets` names `art.rs` and is green. The sheet's panic for an unmapped heirloom stays as
  a guard, and can no longer be reached from the content: the cast check now asks every heirloom
  `legacies.json` can forge for an imported role. A road-book can only be forged at a winter's
  hearth (W7), so no summer here draws one; the ring is pictured (`screens/w6-ring.png`).
- **`lines.fear.steadied` is unreachable** [emergent]: facing's dread amount is 0 or less only
  with a companion on a won quest, and that case takes the courage branch first. Written where
  the spec puts it all the same; a fault on it would be equivalent.
- **The resolution battery's death roll, read once and checked.** On the 400-house battery
  the roll killed 14/107 at danger 1, 171/702 at 2, 262/548 at 3 and 40/69 at 4, against
  CONSTANTS §3's 15/30/45/60%. Danger 2 sits 3.3 standard deviations low, inside the 4σ bound
  but not comfortably, so I checked it before trusting it. The rule alone, 4,000 staged
  disasters at danger 2, killed 30.4%. The engine's generator showed no lean after low dice
  (30.0/30.0/31.4% for a 0.3 chance after low, middle and high sums). The other 1,200 houses
  of a 1,600-house run read 29.7%. It is a draw, and a fixed one: the battery's seeds are
  recorded, so the number cannot flake. A later change that moves it past 4σ moved the rule.
- **The mutation rounds.** `mutants/w6.txt` is 100 faults: the ten new constants; set out,
  the unanswered and healing; every step of §7.1 (the dice, the margin, the patrons, history,
  quests faced, the first quest, the win, trouble easing both ways, disaster renown, facing,
  the unlucky pick, the fire on others and on the unlucky one, each member's disaster, the
  road among the living, witnessing and its party, roads, the crown's place, the ghost, the
  story); the reward and both lessons; §7.4 (shields, wounds, the fire before the roll,
  mending, the roll's chance, the dead's deeds, mending's cap, seats, mourning, the fallen,
  the carrier, grief, the death's tag, patrons); the heir ranks and nearest kin; facing;
  the road; the ghost laid; the telling's typing, order, closing line, margin telling and
  roll; "Go on", the leaf buttons, "Skip ahead", lifting a card on the telling, the control's
  label; leaving, the year and the board; and the control crowding the bar. Round one
  (`025d9e5`): **71 of 100**, none unbuilt. All 29 escapes were loose checks, mostly a staged
  hero who masked the rule. Garrick is both the lowest base and unable to learn, so neither
  the lowest-base rule nor "may still learn" was ever the deciding one. Every founding child
  is also a descendant, so a parent ranking as a child changed no list. A one-member party
  makes "the unlucky one" and "each member" the same hero. No test had patrons, carriers, a
  base of 9, a lost ghost, a crowned member on a ghost's quest, a rival who only succeeded,
  a settled or conquered hero facing a loss, a steadied loss, a prime adult below 5, a disaster
  on a Fire quest, a two-tag death, or a wounded hero the roll spared. Each now has a test that
  fails under its fault (`resolve_tests.rs`, `harm_tests.rs`, `road_tests.rs`, `telling_tests.rs`,
  `w6.rs`, the battery's death roll). The battery's death-roll rate was added for H5, the roll
  made a coin; the staged death test noticed H5 anyway. Round two, all five lists against
  `2da9faf`: **430 of 433 noticed**, none unbuilt — `w6.txt` **100 of 100** (tests alone 84,
  verify alone 30), `dock.txt` 22 of 22, `w3.txt` 117 of 118, `w4.txt` 133 of 134, `w5.txt`
  58 of 59. **The three escapes are the known equivalents** of sessions 3-5: K7, K8 and R12.
  Five earlier faults named code W6 moved and were re-cut to the same fault at the new site,
  marked "(re-cut s6)": dock K14 and K15, w4 Q11, Q12 and U1.

---

## Session 7 (W7)

**Reading discipline, session 7.** Read: `CLAUDE.md`, the `make-game` skill, the crate whole
(`SPEC-GAPS.md`, `FINDINGS.md`, `mutants/` — the harness whole and each list's sites, `src/`), and
from `spec/` MODULES.md, SPEC.md whole, CONSTANTS.md §6-§10 and §14, and the content W7 reads
(`ui-text.json`'s `winter` and `turning`, `lines.json`'s winter, deed, dream, legacy and turning keys,
`writing.json`'s RESTS, WEDDINGS and TALES_TOLD, `dreams.json`, `legacies.json`'s heirlooms,
`household.json`, `lore.json`'s vocations and seasons, `bonds.json`'s MENTOR and STUDENT). `docs/api/`:
nothing new opened — W7 used only engine surfaces earlier sessions had read. **Engine source: not
opened.** `games/ninjo/` and `attic/`: not opened; `grep -oh "^### G-0[0-9]*" games/*/FINDINGS.md`
read the G-headings only (G-039's method). Incidental, disclosed: `tools/test`'s report prints ninjo's
verify lines. No sibling game read.

### G-056 — the handoff's "SPEC-GAPS (to KG-37)" where the file runs to KG-40

Class: process (misled) · Session: keifu 7 · Owner: the keifu handoff

**Doing:** reading `SPEC-GAPS.md` in full before starting, as the handoff asks, and numbering this
session's entries.

**Expected:** the last entry to be KG-37, as the handoff says.

**Happened:** the file runs to KG-40 — session 6 filed seven, KG-34 to KG-40, and its PR says so.
**What I did on its authority:** nothing beyond a second look: the file is the record, so I read it to
its end and numbered this session's from KG-41.

**Fix:** count the entries in the handoff from the file (`grep -c "^## KG-"`), as G-051 asked for the
mutation lists.

### docs/api: 0 findings

W7 is rules over the game's own state — the hearth, its plans, the winter — plus one more seating
screen and a scaffold page. Every engine surface it touched (the scripted pointer's drag of G-045, the
page's rows and targets, the per-session recorder of G-043, the floors' measurement, `Rect::contains_rect`)
was one earlier sessions had read and used. The documents were asked nothing new.

### The game's own (session 7)

- **The scaffold's winter half is retired; its turning half stands.** Leaving the telling of an open
  house begins winter and opens the hearth (§11.1: the hall, the yard's children, the first two wounded
  by the fire). "Let the winter pass" resolves the winter (§11.3, `winter.rs`) and then turns the year
  — still a **W8 SCAFFOLD** (`season.rs` `turn_the_year`, `passage.rs`, `turning_view.rs`): the turning
  is the winter page alone ("What the winter did", "The winter of year N", its lines or "A quiet
  winter."), then "Summer comes" advances the calendar and prepares the board. Nobody ages, dies of age,
  is born, comes of age or arrives; `House::mourned` still carries the summer's dead for W8. The scaffold
  shows `ui.turning.help` in the dock, whose "Everyone is a year older" W8's ageing makes true — the one
  sentence on screen the scaffold does not keep. W6's played checks now walk through the hearth to year 2.
- **One source for the preview and the winter.** `plans.rs` holds every seat's plan — the yard's and a
  bench's lesson with every excuse in §11.4's order, the courtship verdict (§11.6), the rest, the
  teller's part — and nothing else decides them. `winter::plan` reads the hearth into a `WinterPlan`;
  the hearth screen draws every note from it (`hearth_view::notes`); the resolution asks the same
  functions at each step's moment and records the plan it carried out on the turning's `Passage`, so a
  check holds one against the other — the oracle's mid-drag and seated previews, the unit tests' `pass`
  helper (every staged winter), and the agreement battery: 3000 founded houses stirred (wounds, dread,
  broken fears, ages 4-70, aptitudes, callings, TEACH_A_GREATER, spouses, rivals), every living hero in
  a random hall or hearth seat, the previewed plan required equal to the plan carried out and each plan's
  effect read off the house. It met every excuse but the general plan's "needs a teacher", which no seat
  reaches (a child in the yard belongs on a bench; a bench has its own "needs a teacher"), every verdict,
  every rest and every teller.
- **The rank-limited mentor bond is `bonds::form`**, W2's Form under the rank rule, called with the
  learner first (L→T MENTOR): it replaces companion, friend and rival and never spouse, parent or child,
  and `taught` is set on the teacher's side whatever the bond's kind. Teaching a rival turns the rivalry to
  mentorship and says no "new student"; a parent teaching their child sets `taught` and forms nothing
  (OQ-10) — both unit-tested.
- **The road-book's forging path is live.** Ysolde's last stage, "Tell the tale at the hearth", is met by
  the TELL_THE_TALE moment at the long table, and the road-book is forged and given at the hearth. No
  first winter reaches it by the rules (her first two stages want three new roads and every road), so
  verify stages her dream at its last stage and seats her by the scripted pointer; the sprite imported in
  session 6 draws on her sheet (`screens/w7-road-book.png`).
- **"Let the winter pass" sits under the benches**, the right column's width. The top bar has no room for
  its label: at 14 px it measures 207 px, and between the Door's countdown and "The family" there are 142.
- **Mid-drag the hearth shows the seating the release would make** (SPEC-GAPS KG-41): the hand at its
  landing, a swap's displaced hero where the hand came from — and every note is that seating's plan. A
  group's panel is not a drop target; only seats and seated heroes are.
- **Twelve seats, "ten" in the help** (SPEC-GAPS KG-46). The help is shipped as written.
- **Pictures** (`screens/`): `w7-drag.png` — Odo in hand over Pip's bench, "+1 Spirit" beside it, Odo's
  sheet in the dock; `w7-hearth.png` — the played winter seated, every preview showing (calms, house +,
  +1 Spirit, needs a teacher, will wed); `w7-winter.png` — the oracle's winter page; `w7-pip.png` and
  `w7-odo.png` — the sheets pointed at from it (Spirit 3; "[>] Teach the young two winters (1/2)",
  "Student Pip +1"); `w7-road-book.png` — the played winter's page with Ysolde's road-book on her sheet.
- **Floors over the hearth and the turning at the three sizes** (native 1280x720, web 640x329, 4:3
  1024x768): the hearth idle and each group's help, every hero's sheet beside it, the oracle mid-drag
  (judged on its first page: the wheel cannot page a sheet while the hand is over a seat — the held hero's
  sheet is paged where it is pointed at), six stirred hearths with every seat filled, the winter pages, a
  quiet winter and a winter long enough to continue: 117 surfaces per size in all.
- **The W6 battery's numbers moved, by the rules.** Its houses now winter between summers with the hearth
  as it opens — the wounded rest by the fire and heal — so the battery's deaths and closings are not
  session 6's. The death roll read 24/130, 280/975, 359/800 and 44/62 at danger 1-4, inside its bounds.
- **The mutation rounds.** `mutants/w7.txt` is 115 faults: every W7 constant (the seat counts, the
  self-taught and child limits, the teachable age, the gain cap, the lesson and its three bonuses, the
  greater teacher's 9, the tale's renown, the marrying age and gap, the yard, the shed, the garden help's
  two W8 numbers); every excuse and their order; every amount and cap; the wasted lines; teacher credit
  (both seats' rules, OQ-20 and OQ-21), the rank-limited mentor bond and `taught`, the first TAUGHT deed,
  "new student"; every courtship step and its order; the fire's three outcomes and its note; §11.3's
  order; the wedding, the peace, the failed courting; the tellers and the house's once; the hearth's
  opening; the preview/resolve agreement from both sides; the notes; the seat targets, the drag and the
  pairs' layout; the controls and the winter page. Round one (`2dc8257`): **104 of 113 noticed, two not
  built** (Y1 and B2 cut as match guards that left the match non-exhaustive; re-cut as the wrong line).
  Eight escapes were loose checks: the garden judged only with the older or the younger first (K2, K3),
  no child holding a dream a winter moment could move (T4), nothing asking which side of a pair a seat is
  drawn (S4), nothing lifting a hero from a hearth seat (S1), nothing asking the bench's note beside its
  own bench (V2), no teacher alone in the yard (V1); O1 was cut wrong — it moved a binding, not the fire —
  and is re-cut to resolve the fire after the yard. Each now has a test that fails under it
  (`plans_tests.rs`, `winter_tests.rs`, `hearth_view.rs`, `w7_controls.rs`). Round two, all six lists
  (`w7.txt` against `1d13583`'s tightened checks, the five earlier against `c880b51`, the same code):
  **544 of 548 noticed**, none unbuilt — `w7.txt` **114 of 115** (tests alone 95, verify alone 67),
  `dock.txt` 22 of 22, `w3.txt` 117 of 118, `w4.txt` 133 of 134, `w5.txt` 58 of 59, `w6.txt` 100 of
  100. **The four escapes are equivalent:** W3's K7, W4's K8 and W5's R12, as before, and W7's E21 — the
  cap "at most 9 - known" can never bind, because what a teacher counts as knowing is at most 9 (a base,
  or TEACH_A_GREATER's 9), so "at most taught - known" is always the smaller. Three earlier faults named
  code W7 moved and were re-cut to the same fault at the new site, marked "(re-cut s7)": dock K12 (the
  dock's subject now matches a fourth field) and w4 U1 and w6 T11 (the drag now begins on any seating
  screen, `seating`, not only the summer's).

---

## Session 8 (W8)

**Reading discipline, session 8.** Read: `CLAUDE.md`, the `make-game` skill, the crate whole
(`SPEC-GAPS.md` to its end — 46 entries counted from the file, `FINDINGS.md`, `mutants/` — the harness
and each list's sites, `src/`), and from `spec/` MODULES.md, SPEC.md whole, CONSTANTS.md whole,
OPEN-QUESTIONS.md whole, `content/README.md`, and the content W8 reads (`lines.json`'s turning, death,
heir, promise, birth, age, arrival, tales, phase, ghost and deed keys; `ui-text.json`'s `turning`;
`writing.json`'s SLEEP_DEATHS, BIRTHS and ARRIVALS; `names.json`; `wanderers.json`; `destinies.json`;
`dreams.json`'s ghost places and wanderer flags; `bonds.json`'s kinship tellings; `lore.json`'s phases,
vocations and phase-effect fragments; `household.json`). `docs/api/`: nothing new opened — W8 used only
engine surfaces earlier sessions had read. **Engine source: not opened.** `games/ninjo/` and `attic/`: not
opened; `grep -oh "^### G-0[0-9]*" games/*/FINDINGS.md` read the G-headings only (G-039's method).
Incidental, disclosed: `tools/test`'s report prints ninjo's verify lines. No sibling game read.

### G-060 — "the W5/W6 batteries' numbers will move" where W5's cannot

Class: process (misled) · Session: keifu 8 · Owner: the keifu handoff

**Doing:** the handoff's whole-year properties: "The W5/W6 batteries' numbers will move again by the
rules (houses now grow as well as shrink); note the new baselines beside the old."

**Expected:** both batteries to run through W8's turning.

**Happened:** W5's shape battery is defined over the founding household *as founded* — each summer only
advances the calendar and prepares the board (`w5_shape.rs`, session 5's "Its summers, before W6 and
W8"); nothing in it resolves a quest or turns a year, so no W8 rule reaches it and its numbers are
session 5's to the count. **What I did on its authority:** weighed rewriting it to play its summers, and
did not — it is W5's oracle as W5 framed it, and a played house is the W8 battery's subject. Instead the
W8 whole-year battery measures W5's mark on *played* summers and prints it beside W5's founded baseline.
W6's battery does run through the turning, and its numbers moved.

**Fix:** a handoff naming batteries to move could say which of them read the code it changes.

### docs/api: 0 findings

W8 is rules over the game's own state — the turning, its pages, the heirs — plus one more screen of pages
and buttons. Every engine surface it touched (the scripted pointer of G-045, the page's rows and targets,
the per-session recorder of G-043, the floors' measurement, `Rng`'s draws) was one earlier sessions had
read and used. The documents were asked nothing new.

### The game's own (session 8)

- **The scaffold is retired whole.** `season::turn_the_year`'s scaffold, `passage.rs` and `turning_view.rs`
  are replaced: the turning runs every step of SPEC §18 in its stated order (`turning.rs`), writes its pages
  (`passage.rs` — a page per kind, a death page's bequest), and the screen shows them (`turning_view.rs`).
  `House::mourned`'s carried dead get their pages, the summer's first. `ui.turning.help`'s "Everyone is a year
  older" is true. The Door's summer still refuses to generate a board, loudly — W10's.
- **One owner for the heir decision.** `heirs.rs` holds the ranking (moved whole from `harm.rs`, where W6
  had it for the crowned's nearest kin), the kinship words, the buttons with their marks, and `choose`,
  which applies the bequest. The buttons' "(not the dream)" asks `can_take_dream`, the function `choose`
  asks, read when the page is drawn (SPEC-GAPS KG-47), so a button never promises what its press will not do.
- **The year refuses to turn, three ways.** The screen will not go past the first waiting page ("Go on"
  greyed and inert there, leaf buttons beyond it inert, "Skip ahead" lands on it — KG-51); `season::may_turn`
  gates the press; and `season::summer_comes` panics if anything else asks it to turn the year early.
- **Deviation: the epitaph's wording roll is W9's.** The original rolls an epitaph wording on each death
  page (§15.1 step 1, §22.2) and composes the epitaph there and after the choice. Both are W9's (MODULES W9,
  session 1's stated deviation), so the page shows the dead's condition line where the epitaph will go,
  and the wording's draw is not taken. That shifts the generator's sequence after a death page, not any
  distribution (§22.1, §22.3); W9 inserts the draw at the marked site.
- **Deviation: no skull portrait.** §18.1's death page has a skull sprite; the cast has none, so the page
  shows the dead's own card, greyed as the dead are on every screen. Presentation only.
- **The heir buttons** are two columns of five (CONSTANTS §14), filled down the first column then the
  second; the prompt sits above them at the page's foot. A page whose lines leave no room for the choice
  continues to a leaf of its own. The numbered leaf buttons show a window of twelve around the leaf on
  screen when a turning has more.
- **Pointing at an heir opens their sheet in the dock** — the decision's "each candidate's sheet in the
  dock". The death page's own lines (the heirloom left, the dream left undone) name the dead's legacy.
- **Earlier waves' checks, walked through the turning.** W6's and W7's played checks read the turning to
  summer through the screen, choosing the first heir where a page waits (`play::read_to_summer`) — Garrick,
  63 after year 1's turning, dies of age on about a quarter of the seeds. W7's agreement battery now reads
  each seat's effect between the winter and the turning (the turning ages, grieves and teaches at a coming
  of age); the plan the turning records is still required equal to the preview.
- **The batteries' new baselines.** W6's battery (400 houses, up to 6 summers) now turns real years:
  2133 summers, 5844 quests, 251 houses closed, 928 died questing (session 7) → 2136 summers, 7251
  quests, 207 closed, 1226 died questing (the count now restricted to deaths with a place, since old age
  also kills — unrestricted it read 1470). Houses that grow send more parties and close less. The death
  roll read 55/338, 446/1517, 441/943 and 42/59 at danger 1-4, inside CONSTANTS §3's bounds. W5's founded battery cannot move (G-060): 568 of 600 (94.7%) as before; on played summers the
  W8 battery reads the mark in 3093 of 3962 (78.1%) — houses that grow meet demand +3 and +4 a seat with
  more mouths but no more power than their founders had, and fewer callers are free to go.
- **The mutation rounds.** `mutants/w8.txt` is 188 faults: every W8 constant (CONSTANTS §8 and §10's old
  age, births, wanderers, standings, the teaching that shows); the turning's step order (ageing before old
  age, births before comings of age before the wanderer, the tales before the phase lines, the mourned
  cleared, the year page and the closing line); the heir ranking line by line (each rank, each way a parent
  is known, the sort, the dead, the cut at eight); every kinship word; both marks and `can_take_dream`; each
  bequest (the heirloom laid aside, given, deeded, buried with a capital, kept by the dead; the dream passed
  or raised, its fate, its owner, the burden, rivals; the choice's lines and its record); the ghost's
  owner and place; the death page and the Door's promise; old age's chance, factors, carrier and record;
  the bags' no-repeat draw; every birth rule and the fear and blessing inheritance; every coming-of-age rule
  and §9.5's priority; the wanderer's odds, standing, gift, dates, calling, Seer and rivals; the rolled
  dream's claims; the phase sentence; and the screen's refusal (Go on, the leaves beyond, Skip ahead, the
  view after a choice, the dock, the greyed control, the choice's room, `may_turn`). Round one (`ec79c03`):
  one worker's thread died when a fault (B13, the choice never recorded) hung `--verify` in the checks' own
  heir loop, so 36 faults went unjudged and the first pass read **134 of 151**, one not built (M8, cut
  without its closing parenthesis; re-cut). The harness now counts a run that does not end within ten
  minutes as failed — a hang is noticed — and the heir loop fails loudly when a choice makes no progress.
  Round one's rest (the 36, the escapes, M8; `15e27fc`): 46 of 54. Escapes, all loose checks
  but two: no child created after a grandchild (R1), no parent known by the bond alone (R5) or the
  parents field alone (R6), no companion among the heirs (K3), no "(not the dream)" asked with no dream
  to leave (M2), no dead hero leaving both dreams undone (B1), no lower-case heirloom buried (B7), no dream
  told about another (P3, D2), no choice beside a Door's promise (D3), a dead child in the Door test who was
  the younger (D12), no weak parents for the newborn's floor (C15) or roll (Y11), no conquered parent
  failing their roll beside a broken one (Y15), a ghost list too short to tell a swap from a removal
  (A17), no carried dream at a coming of age (A11), no held dream beside a parent lost questing (A20), no
  coming-of-age rival (A7), no wanderer rival (W12), no turning without a year page (T11), and no death
  page long enough for "Skip ahead" and the choice's room to matter (S4, S10). Each now has a test that
  fails under it. **Round two, all seven lists against `c41a21c`: 730 of 736 noticed**, none unbuilt
  (tests alone 609, verify alone 398) — `w8.txt` **186 of 188** (tests 175, verify 63), `dock.txt` 22 of
  22, `w3.txt` 117 of 118, `w4.txt` 133 of 134, `w5.txt` 58 of 59, `w6.txt` 100 of 100, `w7.txt` 114 of
  115. **The six escapes are equivalent:** K7, K8, R12 and E21 as sessions 3-7 classified them, and W8's
  two — B3 (the heir's old heirloom cloned rather than taken is overwritten two lines later by the one
  inherited) and Y3 (a full house skipping one pair rather than stopping: no birth follows a full check,
  so every later pair meets the same full house). Seven earlier faults named code W8 moved and were re-cut
  to the same fault at the new site, marked "(re-cut s8)": w6 H16-H19 (the heir ranking, from `harm.rs`
  to `heirs.rs`) and w7 W2, W4 and W5 (the turning's controls and winter page). The round ran as one
  combined list of the seven, labels prefixed by their list, over four worktrees: about two faults a
  minute on this machine's four cores.

---

## Session 9 (W9)

**Reading discipline, session 9.** Read: `CLAUDE.md`, the `make-game` skill, the crate whole
(`SPEC-GAPS.md` to its end — 56 entries counted from the file, `FINDINGS.md`, `mutants/` — the harness
whole and each list's headers and re-cut sites, `src/`), and from `spec/` MODULES.md, SPEC.md §0-§4, §7.4,
§9.5, §14.4, §15-§23, CONSTANTS.md §13-§14, OPEN-QUESTIONS.md's OQ-2 to OQ-5 and OQ-12/13,
`content/README.md`, and the content W9 reads (`epitaph.json` whole; `dreams.json`'s `progress_tellings`;
`legacies.json`'s `legacy_nouns` and heirloom names; `door.json`'s lock names; `household.json`'s dead;
`lore.json`'s tags, places and pronouns; `writing.json`'s fate pools; `ui-text.json`'s `family` and
`turning`; `lines.json`'s `fate.*`). `docs/api/`: nothing new opened — W9 used only engine surfaces earlier
sessions had read. **Engine source: not opened.** `games/ninjo/` and `attic/`: not opened;
`grep -oh "^### G-0[0-9]*" games/*/FINDINGS.md` read the G-headings only (G-039's method). Incidental,
disclosed: `tools/test`'s report prints ninjo's verify lines. No sibling game read.

### G-066 — "W5's founded battery reaches no death and cannot move", and it moved

Class: process (misled) · Session: keifu 9 · Owner: the keifu handoff

**Doing:** re-baselining the batteries the wording draw shifts — "the W6 and W8 batteries pass death pages,
so their sequences shift ...; W5's founded battery reaches no death and cannot move" (G-060's own fix,
applied).

**Expected:** W5's numbers unchanged, as G-060 found them in session 8.

**Happened:** they moved — 568 of 600 (94.7%) became 566 (94.3%), and the year-1 draws and the loop's
counts with them. SPEC §22.2 puts **two** wording rolls at the founding ("two epitaph wordings ... Then board
generation for year 1"), before the first board, and every battery founds its houses through
`House::found`. The handoff counted only the death page's roll, the one session 8 marked; the founding's two
(session 1's, never marked in code or named as a deviation) and the crowning's (session 6's, marked at its
site in `harm.rs`) were untaken too. **What I did on its authority:** took W5 as fixed until the first verify
run printed 566; then re-read §22.2, took all three draws, and noted W5's new baseline beside the others
(the W8 battery's printed reference to it now reads 566).

**Fix:** a handoff that names a shift's reach could list the draws from §22.2 rather than from the marked
sites; and a session that skips a draw the spec lists should mark the site and say so in FINDINGS, as
session 8 did for the death page — session 1 did neither for the founding's two.

### docs/api: 0 findings

W9 is rules over the game's own state — the wording, the parts, the composition — plus one more line on
two screens that already existed. Every engine surface it touched (the page's rows and targets, the
per-session recorder of G-043, the floors' measurement, `Rng`'s draws) earlier sessions had read and used.
The documents were asked nothing new.

### The game's own (session 9)

- **Both deviations are retired.** The death page rolls its wording at step 1 (the site session 8 marked)
  and composes at step 8; the page sets the epitaph under the dead's card, read off the hero, so the heir
  choice's recomposition shows at once. The family's remembrance shows the dead's and the crowned's
  epitaphs where session 1's condition line stood. The condition line stays only for a summer's dead before
  their page (SPEC-GAPS KG-57), and on the hero sheet itself, where §19.1 puts it.
- **Every wording draw the spec lists is taken**: the founding's two (Elsbeth, then Aud, before the board),
  each death page's, and each crowning's (G-066). The Ending's — un-mourned dead and every living hero — are
  W10's; nothing living carries a wording or an epitaph yet, and the battery holds that.
- **One source for the epitaph.** `epitaph.rs` rolls and composes; `epitaph_parts.rs` and `epitaph_ends.rs`
  hold the nine parts' rules; `epitaph_lore.rs` reads `epitaph.json` by key, with every list checked against
  the canonical part order and the budget and frame count against CONSTANTS §13. Six callers compose — the
  founding, a crowning, the death page, the heir choice, a ghost laid, a ghost taken up — and nothing else
  writes an epitaph. A recomposition with no wording ever rolled panics, loudly.
- **Elsbeth's epitaph, by hand.** ORIGIN is empty (dead before the first year), END is two sentences, DREAM
  ("died before it was done": her dream's fate is UNDECIDED at founding, so the rule falls to `dream.died`)
  and LOVE one each: four. FEAR's coin decides the rest — the two-sentence wording fills the budget at six
  and ROADS's "went out eleven times in all" is skipped; the one-sentence wording leaves room for it. The
  oracle holds the remembrance to one of the six resulting strings, shipped as literals, and part by part.
  Over the 27 recorded seeds: frames 10/8/9, the one-sentence fear on 17.
- **Every battery epitaph is exactly six sentences.** 44,532 readings over the W8 battery's dead, all at
  six: a dead hero always has more than six sentences' worth of parts, so the priority order decides what is
  said, every time — which makes it the rule most worth the owner's eye (below).
- **The W8 oracle reads Garrick's page across its leaves.** Six sentences under the card push the heir
  choice to a continued leaf on most seeds; the oracle reads the heading, the title and the epitaph off the
  first leaf and the prompt and heirs off the choice's, and "stays on the page" now means a leaf of page 2
  (the view returns to the page's first leaf after the choice, as before).
- **Recorded seeds and baselines.** `SEED_AFTER_SEVEN` re-recorded: 0x52ec_97b2_4d27_2e97 →
  0x0213_ec98_7493_1ee4. The batteries move by the sequence, inside their bounds:
  W5 founded 568/600 (94.7%) → 566/600 (94.3%), the reader's own call 516 → 521;
  W6 2136 summers, 7251 quests, 207 closed, 1226 died questing → 2137, 7246, 218, 1289, the death roll
  55/338, 446/1517, 441/943, 42/59 → 44/313, 487/1550, 442/975, 52/80 at danger 1-4;
  W6's played summer on 27 seeds (setback 4, success 12, triumph 11 → 3, 14, 10);
  W8 3957 turnings, 5 closed, old age 427, quests 99, 525 death pages, 1028 births, 1061 comings of age,
  289 wanderers → 3973, 4, 439, 90, 529, 994, 1023, 325; W5's mark on played summers 78.1% → 78.7%.
  One W6 floors staging was seed-bound — a played page "told five times over" stopped overflowing a leaf
  when seed 1's summer became a one-line success — and is now told thirty times, whatever the dice.
- **Pictures** (`screens/`): `w9-elsbeth.png` — the family in year 1, Elsbeth pointed at, her name and
  epitaph on the remembrance panel (frame 2, the two-sentence fear); `w9-questing-death.png` — Brannoc,
  fallen at the Barrow in year 1, his page's epitaph above "He leaves a dream undone" and the heirs;
  `w9-old-age.png` — Odo, dead in his sleep at 94, his epitaph (his friend Garrick, the Seer's "outlive")
  above the grief it caused.
- **Voice, as shipped.** Content is identical by contract, and some joins read oddly; none was smoothed.
  A SLEEP_DEATHS fate in `end.dead_0`: "He fell asleep over a book in the last week of winter in year 1,
  aged 94." Aud's authored fate in `end.before_first_year`: "Aud Hale died of a fever, the winter Wren was
  two, before the first year." A dead TEACH_A_GREATER teacher with no greater student: "... and he never
  learned another thing."
- **The mutation rounds.** `mutants/w9.txt` is 120 faults: both W9 constants; the wording roll (the
  run's first frame, the no-repeat frame, the run's memory, one coin per part and its part, the draws'
  order, a recomposition's frame); the priority walk (a skipped part ending it, the parts' order for the
  priorities'), the budget (strict, uncounted, empty parts chosen, sentences counted by ". "), the frame's
  order (the priorities', frame 0's) and the join; the naming rule (none, every part, the first name, the
  lost "'s", every and no " he " inside, the possessive unlooked-for, the object pronoun); the lore read
  (priorities, frames, lock names); every part's selection rule branch by branch, and each variant pair;
  every composition point (the founding's order and its composing, the death page's roll and its step 8,
  the heir choice, a ghost laid, a ghost taken up, a crowning, a crowning rolled twice, a choice rolling a
  new wording); and the two surfaces (the remembrance and the death page, and the leaves' room for the
  epitaph). Round one (`e4c7783`): **109 of 117 noticed, three not built** — M6 and V8 were cut as match
  guards that left the match non-exhaustive, and W4 (an extra composition before the bequest lines) did
  not borrow-check; W4 was also equivalent as cut, since steps 2-7 of the page change nothing an epitaph
  reads, so it is re-cut to a real fault: only a page with nothing to leave composes. All three are marked
  "(re-cut s9, round 2)". The eight escapes were loose checks, each a staging that never put the rule on
  its edge: no birth in year 1 itself (O5), no fear faced exactly once (F5), no conquest beside a break
  (F9), no borrowed dream whose " my " tells for an owner of the other pronoun (M7), no greater student
  who was never taught (P8), no death in year 1 (E5), no legacy beside a buried heirloom (X3), no made
  heirloom with no heir (X6). Each now has an assertion that fails under it (`epitaph_tests.rs`,
  `epitaph_ends_tests.rs`). One earlier fault named code W9 moved and was re-cut to the same fault at the
  new site, marked "(re-cut s9)": w8 S5 (the turning's leaves now read the heroes, for the epitaph's
  height). **Round two, all eight lists against `4f52dad`: 850 of 856 noticed**, none unbuilt (tests alone
  719, verify alone 457) — `w9.txt` **120 of 120** (tests 110, verify 48), `dock.txt` 22 of 22, `w3.txt` 117
  of 118, `w4.txt` 133 of 134, `w5.txt` 58 of 59, `w6.txt` 100 of 100, `w7.txt` 114 of 115, `w8.txt` 186 of
  188. **The six escapes are the known equivalents** — K7, K8, R12, E21, B3 and Y3, as sessions 3-8
  classified them. The round ran as one combined list of the eight, labels prefixed by their list, over four
  worktrees: 856 faults in about two hours and fifty minutes.

---

## Session 10 (W10)

**Reading discipline, session 10.** Read: `CLAUDE.md`, the `make-game` skill, `docs/templates/DECISIONS.md`'s
header row (the handoff copies it), the crate whole (`SPEC-GAPS.md` to its end — 64 entries counted from the
file, `FINDINGS.md`, `mutants/` — the harness whole and each list's headers and re-cut sites, `src/`), and from
`spec/` MODULES.md, SPEC.md §0-§8, §9.3-§10, §12.2, §12.5-§16, §19-§23, CONSTANTS.md §1, §3, §12-§14,
OPEN-QUESTIONS.md's OQ-14, OQ-15, OQ-23, OQ-24, OQ-32 and OQ-33, `content/README.md`, and the content W10
reads (`door.json` whole; `lines.json`'s door, deed and fate keys; `ui-text.json`'s `top_bar`, `summer`,
`door_sheet`, `telling`, `ending` and `family`; `bonds.json`'s kinship tellings; `lore.json`'s tags, places and
pronouns; `household.json`; `names.json`'s longest name, for the tree's narrowest node). `docs/api/`: nothing
new opened — W10 used only engine surfaces earlier sessions had read. **Engine source: not opened.**
`games/ninjo/` and `attic/`: not opened; `grep -oh "^### G-0[0-9]*" games/*/FINDINGS.md` read the G-headings
only (G-039's method), and `grep -l`/`grep -c` on that heading named the two files that hold G-066 (G-067).
Incidental, disclosed: `tools/test`'s report prints ninjo's verify lines. No sibling game read.

### G-067 — "the living's are six sentences too", and they are not

Class: process (misled) · Session: keifu 10 · Owner: the keifu handoff

**Doing:** the Ending's acceptance — "every living hero with an epitaph (W9's battery property extends: the
living's are six sentences too)".

**Expected:** every living hero's epitaph at exactly six sentences, as every dead one's is (W9: 44,532 of
44,532).

**Happened:** a living child's parts are mostly empty — no roads, no triumph, no love of their own, an unspoken
destiny, `end.child` — so the priority walk runs out of parts before the budget. Over the full-dynasty battery
the living's epitaphs read by sentences 0..6 `[0, 0, 0, 384, 24, 10, 1690]` (the keeper) and `[0, 0, 0, 331, 63,
21, 2132]` (the teacher); the staged Ending's two children read three and four: "Cerys Thorne was born in the
house in year 20, to Pip and Clove. Even small, she was afraid of the dark. She was 5 when the story ended, and
not yet grown." **What I did on its authority:** planned an exact-six assertion; the battery's first run showed
it false by the rules, so the checks hold the rule SPEC §20 states — at most six, every one naming its subject —
and print the distribution.

**Fix:** a handoff that extends a battery property could say which half of it is a rule (the budget) and which
an observation of one population (the dead always overflow it).

### G-068 — the G-sequence forked at G-066

Class: process · Session: keifu 10 · Owner: G-039's fix (a neutral home for the last G-number)

**Doing:** numbering this session's first finding by the heading grep.

**Expected:** one largest heading.

**Happened:** `### G-066` heads an entry in both `games/keifu/FINDINGS.md` (session 9) and
`games/ninjo/FINDINGS.md` — two sessions in flight at once each took the next number from the files as they
stood. **What I did:** numbered from G-067 and read nothing of ninjo's entry; the two G-066s stay as filed.

**Fix:** G-039's — keep the last-used G-number in one neutral place a session claims it in (`games/README.md`).

### G-069 — "the transcript records it" names nothing in this crate

Class: process · Session: keifu 10 · Owner: the keifu handoff

**Doing:** "Begin another house … The transcript records it; a replay reproduces it."

**Expected:** a transcript this port keeps — a record of the run's presses or houses.

**Happened:** nothing in the crate or its docs is called a transcript (the word appears once in `src/`, for a
frame's textual dump). **What I did:** built the nearest thing that answers both halves: `house::Chronicle`, the
run's record of every house founded in this process — its seed, and how it ended once another is begun — kept
outside the house so a reset cannot erase it, and a verify replay that presses the same buttons on the same seed
in a second session and requires the same chronicle and the same new house.

**Fix:** a handoff naming an artifact could point at its file, or say "build one".

### docs/api: 0 findings

W10 is rules over the game's own state — the Door, the Ending — plus a card, a sheet, a verdict page and a tree
that grows. Every engine surface it touched (the scripted pointer of G-045, the page's rows and targets, the
per-session recorder of G-043, the floors' measurement, `Rng`'s draws, `world_mut().insert_resource` for the
reset) earlier sessions had read and used. The documents were asked nothing new.

### The game's own (session 10)

- **The refusal and the scaffold are retired whole.** The last summer posts the Door's Might lock alone
  (`board::post_board`, `door::door_board`); `resolve::set_out` tries the Door there (§7 step 2) and nowhere
  else; generation still refuses the last summer, loudly — it is never asked now. `ending_view.rs` is the real
  verdict page, `ending.rs` what entering the Ending does.
- **One function owns §16.2.** `door::try_the_door`: the prologue, then each lock in order — the standing read
  off the house after the last lock changed it, the bearer (`door::bearer`), the lock resolved by
  `resolve::resolve_party` (the quest's own fifteen steps, factored out of `resolve_rolled` so both read them
  once, with the Door's exceptions inside it and inside `reward`), the opened lock's deed — then the stood deed.
  The card, the sheet and the top bar read `door::outlook` over `door::lock_quest`, the quest the locks are
  resolved as: one source for the numbers and the roll.
- **The top bar has four rows.** The outlook ("Your best four today bring …") needs a row of its own; `TOP_H`
  58 → 76 moves every screen under the bar down 18 px, and the floors at the three sizes hold on all of it.
  "The family" narrowed 140 → 124 px and the set-out control widened 112 → 140 for "Try the Door"; the telling's
  wide buttons 140 → 164 for "After the Door" (both found by the floors, not by eye).
- **The Door's help.** The original's Door hover names the best four; pointing at the top bar's Door lines opens
  `ui.top_bar.door_help` in the dock, as session 1 made every hover inline.
- **The tree holds a big house.** The Ending's family must show everyone who ever lived; a generation wider than
  eleven nodes wraps onto another line (never between a hero and the spouse placed after them), and the lines
  step closer to stay above the remembrance (`tree::tree_lines`, `node_rects`). Seen and left: a spouse from
  another generation (a wanderer wed to a grandchild) is linked by a line that runs under the nodes between them
  — the tree's rows have always put a spouse beside a hero only within one generation (`screens/w10-played-family.png`).
- **"Try the Door" with nobody waits, greyed** (`summer::greyed_button`), and a press does nothing.
- **Two players for the full-dynasty battery.** The W8 battery's player never teaches, and with it the Door
  almost never gives (2 of 233 houses opened one lock): the battery would only say "a house that never trains
  fails". So it plays every house twice: that player ("the keeper"), and the same player teaching every winter
  ("the teacher": the training yard's best pair, each bench's best child and teacher, by the plans the winter
  carries out). The teacher's houses spread across all five verdicts — the number the owner reads.
- **The full-dynasty battery's numbers** (`w10_battery.rs`, 240 houses from `0xa_0000`, every one founded and
  played to its Ending). The keeper: 7 closed before the Door, 233 tried it; stayed shut 231, a hand's breadth 2.
  The teacher: 1 closed, 239 tried; stayed shut 75, a hand's breadth 41, half open 57, is open 66, closed 1. The
  locks against CONSTANTS §3 at the powers rolled: iron 131/239 (forecast 134.1, 1.0σ), riddles 122/238 (129.5,
  2.1σ), breath 100/225 (97.9, 0.7σ) — all inside the 4σ bound fixed before the first run. The outlook's "all three
  open" averaged 25.8 in 100 and all three gave in 66 of 239 (27.6%). The living's epitaphs by sentences 0..6:
  keeper `[0,0,0,384,24,10,1690]`, teacher `[0,0,0,331,63,21,2132]` (G-067). No house broke, and none came to the
  Door with no one to send.
- **The mutation rounds.** `mutants/w10.txt` is 146 faults: every §16 constant (the seats, danger, renown, the
  locks' demands, the Door's destiny power, the best four's divisor); the lore read (titles, names, verdicts, the
  closed house's); the lock as a quest (aptitude, place, tags, danger, renown, demand, seats, `is_door_lock`, the
  Door's promise); the outlook and best four (each lock's chance, all three, patrons, triumph against success, the
  combinations, ties, the wounded and refusers kept, the score); the bearer rule (ties, the weakest, the first who
  stands, the dead); the locks' order and carry-over (reversed, the dice, the dead standing, the opened and stood
  deeds, their weights, year and place); the Door's exceptions (reward, disaster lines, renown, the road, lessons,
  triumph deeds); the control (the label, greyed, inert, the press); the top bar, the help, the card and the
  sheet; the prologue fragment by fragment; every §23 rule (the remembering's order, the bequest, the heirloom and
  heir, the dream's fate, mourning cleared, entered once, the verdict by locks given, the closed year, the verdict
  lines and "too"); the reset and chronicle (only after an Ending, the seed's source, the new house, how the old
  ended); and the tree (the wrap, a spouse kept beside, the lines closing up, the Ending's heading and way back).
  Round one (`9e2d0b6`): **142 of 146 noticed**, none unbuilt (tests alone 121, verify alone 74). The four escapes
  were loose checks: no disaster's story read at a lock (L1), no outlook whose best four are not the first four
  (F4), the Door's sheet never read with nobody before it (V12), no patron in a prologue's summary (D22). Each now
  has a check that fails under it (`door_tests.rs`, `w10.rs`'s best-not-first and empty-sheet literals,
  `door_prologue_tests.rs`). Seven earlier faults named code W10 moved and were re-cut to the same fault at the new
  site, marked "(re-cut s10)": w4 K12, B4 (the resolution factored into `resolve_party`); w5 P14 (the board posted
  through `post_board`); w6 L4, W5, T4, T5 (the telling's nav strip moved to `telling_nav.rs`, set out's gate).
  **Round two, all nine lists against `e2e1c21`: 996 of 1002 noticed**, none unbuilt (tests alone 843, verify alone
  537) — `w10.txt` **146 of 146**, `dock.txt` 22 of 22, `w3.txt` 117 of 118, `w4.txt` 133 of 134, `w5.txt` 58 of
  59, `w6.txt` 100 of 100, `w7.txt` 114 of 115, `w8.txt` 186 of 188, `w9.txt` 120 of 120. **The six escapes are the
  known equivalents** — K7, K8, R12, E21, B3 and Y3, as sessions 3-9 classified them. One combined list over four
  worktrees, as session 9 ran it.
- **The pictures** (`screens/w10-*.png`, fifteen, each opened): the Door's card empty, mid-drag with its odds live,
  three seated and four; its help in the dock; its sheet; the telling's prologue and a lock's page; the four
  verdicts and the closed house's; the family at the Ending on a staged house and on a battery-played one (the big
  tree wrapping, the cross-generation spouse line noted above); and another house begun at year 1. Every earlier
  screen re-taken under the four-row top bar. The web build (`tools/build-web keifu`, `tools/serve-web keifu
  --check`) draws year 1's summer in the browser with the outlook row, at the small 640 canvas.
- **The harness** (engine tooling session, 2026-10-04, not a keifu session): `mutants/mutate.py` is retired and
  `tools/mutate keifu mutants/*.txt` runs the same lists, unchanged. Both scored `w9.txt` on the same tree, 120 of
  120 with every per-fault verdict and both columns identical, and `--fast` agreed on every verdict. On warm
  worktrees the full pair took 47m45s and `--fast` 9m22s on this machine's four cores; the old harness took 54m41s.

## Keifu X Inheritance

(The variant's implement stage, `tools/yakin/runs/keifu-x-inheritance/`. G-numbers continue from mainline's G-069.)

### G-070 — DESIGN.md says "new `src/family.rs`", and `family.rs` is the family screen

Class: doc (misled) · Session: keifu-x-inheritance (implement) · Owner: the designer routine / DESIGN.md template

**Doing:** adding the one eligibility function `may_marry_in` and the `is_family` predicate, in the file DESIGN.md names.
**Expected:** a new file. **Happened:** `src/family.rs` exists (the top bar and the family screen). **What I did on the doc's
authority:** planned the module list around it; found the collision when adding `mod`. Put both in `src/outsiders.rs`.
Cost: a minute; the fix is a design stage that greps the crate for a module name before naming one.

### G-071 — DESIGN.md's G6 roll counts are not what the rule consumes

Class: doc (misled) · Session: keifu-x-inheritance (implement) · Owner: the designer routine

**Doing:** writing the trait-branch unit tests from "the roll count per branch (0, 1, 1, 1 or 2)".
**Happened:** a trait only one parent holds takes 1 roll if it passes, else 2 (the spring roll missed) or 3 (it sprang);
neither parent takes 1 or 2. **What I did:** asserted the counts the rule gives.

### G-072 — two mainline checks assume a sheet fits the dock; the variant's lines break that

Class: process (gap) · Session: keifu-x-inheritance (implement) · Owner: the keifu handoff

The PERSONAL line (a quest sheet) and the TRAITS and MARKS sections (a hero sheet) make sheets longer than the dock. W4's
history panel (at the sheet's foot) fell out of view, and `floors::look`'s wheel scroll over a held hero lets go of the hero.
DESIGN.md said no surface needed a change; these two checks did. W4 now reads the history with the wheel at the end;
`look` does not scroll a held hero's sheet (the same sheet is scrolled whole when pointed at).

### G-073 — a rename after a death changes an epitaph the W9 battery says only triggers change

Class: doc (misled) · Session: keifu-x-inheritance (implement) · Owner: the designer routine

DESIGN.md: marrying in changes the full name "everywhere it is drawn". The W9 watcher holds that an epitaph changes only at
a trigger and equals its composition. **What I did:** recomposed the dead's epitaphs at the wedding and taught the watcher a
rename is a reason to change. The alternative, freezing the epitaph, would also have needed the watcher changed.

### G-074 — the dead's marks are not weighed until an heir takes them

Class: process (decision) · Session: keifu-x-inheritance (implement) · Owner: the owner

Step 8b counts marks on the *living* family; the death pages are made at step 4, before it, and choose later. A hero who dies
carrying the only mark leaves the house unweighed that year and the heir pays from the next. Kept as designed; named so the
owner can decide.
