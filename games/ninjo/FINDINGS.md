# ninjo — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). G-numbers continue giri's sequence (its
`FINDINGS.md` ends at G-009); the fork inherits giri's open workarounds —
G-008's `?constants=` location reading rode along in `src/web.rs` unchanged
and is not re-counted here.

**Reading discipline:** this fork was written from `docs/api/` (all four),
`crates/jidousha/examples/`, and `games/giri/` (a game, not the engine). No
file under `crates/*/src/` was opened, and neither was `docs/internal/` nor
any ADR but 0038 and 0041 (both named by the handoff). Wave 0b held the same
line: `games/giri/` (the port source) and this crate, and nothing under
`crates/*/src/`.

One entry from the S1 session. Nothing else was asked of the documents that
they did not answer: the Camera's pan/zoom, `visible_bounds`, the pointer's
scroll, `SnapshotBuilder`'s edge rules, `Time::alpha`'s per-tick value and
the capture path all worked as written.

### G-010 — the bounds check's stated form assumes a camera that does not move

Class: docs · Game: ninjo (as giri-rt) · Documents: `jidousha-testing.md` ("Assert that
nothing is drawn outside `Camera::visible_bounds()`") · Open

The testing document presents the bounds assertion — every quad
`contains_rect`-inside `visible_bounds()` — as "the highest-value check a
game of shapes and text can write", and for every game so far it was. A
game whose camera pans and zooms over a world larger than the screen cannot
pass it: a partially visible tile at the view's edge is *correct* rendering
and still fails `contains_rect`, and per-run text culling (a label is one
`ctx.text` call) means edge glyphs of a half-visible label land fully
outside. The check the situation actually wants is the inverse pair: nothing
submitted that does not *overlap* the view (culling is honest), and the
submitted count dropping when the view shrinks (culling is real).
ninjo ships that pair (`verify.rs::culling_probe`, UI.md §4); the
document could name the adaptation the first time a scrolling game reaches
it, because the naive reading is "skip the check", which drops real
coverage.

Expected: guidance on what the bounds check becomes for a camera that
roams. Happened: worked it out from the check's purpose; the workaround is
three assertions rather than one. Owner: `jidousha-testing.md`.


## Wave 0b (the people substrate) — **0 new findings**

Said explicitly, because `0 findings` is a real answer and an unsaid one
reads as a skipped step.

Wave 0b reached for **no engine API that S1 had not already established**.
The port was game logic — a registry, a vocabulary, three stores, an
arithmetic — and everything it touched of the engine's surface (`Resource`,
`headless`, `SnapshotBuilder`, `FrameRecorder`, the capture path,
`TextStyle::width_of`) was already load-bearing in this crate and answered
by the four documents when S1 asked. Nothing new was asked of them, so
nothing new can be reported about them; a finding invented to fill this
section would be worse than an empty one.

**G-010 stays open.** The bounds check's stated form still assumes a camera
that does not move, and this wave added map-space content — the cast's
figures and names — which is culled the same way the terrain is and would
fail the naive `contains_rect` reading for the same reason.

One thing worth recording that is *not* a documents finding, because it is
this game's decision and not the engine's: **`Sim::at_rest` had to stop
meaning "the queue is empty"** the moment an ambient occurrence started
rescheduling itself forever. The substrate's stopping condition was written
when every occurrence belonged to a party. Any wave that adds a recurring
ambient occurrence — needs ticking is the next one — meets the same fact, so
it is written down here as well as at the site.

## Wave 0a (the attention architecture) — **1 new finding**

Reading discipline held: `docs/api/` (all four), `crates/jidousha/examples/`,
`games/giri/` and this crate. Nothing under `crates/*/src/`, `docs/internal/`
or any ADR was opened.

### G-011 — whether the first-finger-to-pointer mirror applies to a scripted snapshot is not stated

Class: docs · Game: ninjo · Documents: `jidousha-api.md` ("a game written for
a mouse is already playable by touch"), `jidousha-testing.md`
(`InputEvent::Touched`, `SnapshotBuilder`) · Open

The API document states the mirror as a property of the engine — "the engine
puts the first finger down onto the primary pointer", so
`just_pressed(PointerButton::Primary)` is a tap — and the testing document
lists `InputEvent::Touched { finger, phase, screen }` among the events a
`SnapshotBuilder` records. Neither says whether the mirror is applied when a
*check* records a `Touched` event, or only by the platform layer on the way
in. That is the difference between a check that can verify the claim and a
check that cannot: if the mirror lived in the platform crate, a failing
assertion would mean "the harness does not mirror" rather than "the game's
hit-test is wrong", and there is no way to tell those apart from the
documents.

Expected: one sentence in the testing document saying that a recorded
`Touched` produces the mirrored pointer in the snapshot the game reads.
Happened: wrote the check and ran it to find out. It does mirror, and
`verify::touch_selects` now asserts a finger on a character's figure selects
them with no `PointerMoved` and no `ButtonPressed` in the snapshot — so the
answer is recorded here, and the document is the place it belongs.
Owner: `jidousha-testing.md`.

**G-010 stays open** for the third wave running: the bounds check's stated
form still assumes a camera that does not move, and wave 0a added more
map-space content (the selection ring, the focus pulse) culled the same way.

## The cast-art session (2026-09-02) — **3 new findings**

Reading discipline: `CLAUDE.md`, `.claude/skills/make-game/SKILL.md`,
`docs/internal/assets.md` (named by the handoff — the one document outside the
fence this session was told to read), and this crate whole. Nothing under
`crates/*/src/`, no ADR, and `docs/api/` was not opened, because the session
asked the engine nothing: it added fifteen rows to a table of file names.

All three are the *game's* tooling rather than the engine's, and all three are
about the same fact — `art/` was written for an owner sitting at the keyboard
with the packs, and this session was an agent standing in for one.

### G-012 — the import tool drops a staged role it has no grid for, silently

Class: tooling · Game: ninjo · Files: `games/ninjo/art/import_pack.py`
(`roles`, `plan`), `games/ninjo/art/sprite_defs.py` · **Fixed in this session
for ninjo; still open in `games/giri/art/`**

`import_pack.py` is the one door art comes in through, and it takes "the roles"
to be the names in `sprite_defs.LIBRARY` — the *generated-art* table. `plan()`
walks that list and looks for a file per role, so a role-named PNG staged for a
role the table does not carry is never looked at. Reproduced deliberately after
the fact, with `icon_maker` removed from `LIBRARY` and the fifteen staged:

```
[giri-art] 26 of 27 role(s) filled from target/ninjo-art/staged
exit 0
```

No mention of `icon_maker` anywhere in the output, and a zero exit. The count
reads exactly like a legitimate partial library, which is a documented and
normal state ("Roles the pack does not fill keep their current file"), so
nothing about the run says a file was ignored. This is the no-silent-failure
rule (CLAUDE.md) failing in the tool that enforces the curation model.

Expected: a role list that is the *game's* roles, and a refusal — or at least a
line — for a staged file that matches none of them. Happened: the session read
`plan()` before running it and added all fifteen roles to `sprite_defs.LIBRARY`
first, so the import worked; had it not, fifteen files would have stayed out of
`assets/` and the only symptom would have been `tools/check-assets` failing
later with fifteen missing files and no hint why.

The fix taken here is the honest half of it: every role now has a grid, which
also keeps `make_art.py --restore` meaningful — the way back if a pack is ever
withdrawn is not a way back for a role with no grid. The other half is the
tool's, and is not this session's to change in giri: a staged file matching no
role should be named and refused.

### G-013 — nothing in `art/` shows a candidate at the size it will be drawn

Class: tooling · Game: ninjo · Files: `games/ninjo/art/role_sheet.py`,
`games/ninjo/art/contact_sheet.py` · **Open**

`role_sheet.py` renders a role's shortlist at `--scale` (default 10) and
`contact_sheet.py` a whole pack at `--scale` (default 4). Both upscale the
*art*, so every sheet shows a candidate four to ten times larger than the game
draws it. For an owner at a screen that is fine — a person holds the slot's
real size in their head. An agent does not, and the sheets are the only thing
it sees.

What was done on the sheets' authority: the four aptitude icons were picked as
one family of steel implements — sword (`microrl:70`), pick (`71`), bow (`72`),
hammer (`74`) — off role sheets at scale 12, where all four are unambiguous and
the family cue is obvious. Then a throwaway script composed the same four at
scale 2 (the 16-unit chip, `attention::CHIP`) and magnified the *composed
sheet* rather than the art, which is the only way to see honest 16-pixel
pixels. At that size the pick and the hammer are the same picture — a dim
diagonal with a pale head — and the bow is a smear. Two of the four picks
changed (`71` → `91` ladder, `72` → `39` lantern), and the family cue changed
with them, from "steel implement" to "line-work against filled mass".

Expected: a mode that says "show me this shortlist at the size the slot is
drawn". Happened: wrote one by hand, twice, and would have shipped two
illegible chips without it. `art/picks_sheet.py` (added here) does it for the
landed set — every picture at its drawn size and again at 4x — but it reads
`assets/`, so it can only judge art that has already been imported. The
shortlist half is still missing: `role_sheet.py` wants a `--at-size N` that
composes at the slot's drawn scale and magnifies the sheet.

This is the entry the handoff asked for by name: what an agent-picker needed
that the manifest tooling, written for an owner at the keyboard, does not have.

### G-014 — the fork's art tooling still calls itself giri

Class: docs · Game: ninjo · Files: `games/ninjo/art/*.py` · **Partly fixed**

`art/` rode along whole in the fork (VARIANT.md), and every script still opens
`"""games/giri/art/<name>.py"""`, prints `[giri-art]`, and — until this session
— defaulted its output to `target/giri-art/` and told the reader to run
`cargo check -p giri`. The last of those is the expensive kind: `giri` is a real
crate that really builds, so following the instruction succeeds and checks the
wrong game. `import_pack.py` also wrote `(UI.md §9)` into `CREDITS.md`, which is
giri's asset-slot section; ninjo's is §7, and giri's §9 says something else.

Fixed here: the output paths, the `cargo check -p giri` line, and the `§9` that
went into a committed file. Left alone: the docstring headers and the
`[giri-art]` console prefix, which are wrong but cannot mislead an action — and
a rename of five files was not this session's to make.

Expected: a forked tool says which game it belongs to. Happened: the session
followed `extract.py`'s printed next-step verbatim and staged into
`target/giri-art/`, which is where a giri session would look for it.

**G-010 stays open** for the fourth wave running, untouched here: this session
added no drawn content and no map-space content.
