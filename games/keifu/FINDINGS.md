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
