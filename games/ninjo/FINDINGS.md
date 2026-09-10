# ninjo — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). G-numbers continue giri's sequence (its
`FINDINGS.md` ends at G-009); the fork inherits giri's open workarounds —
G-008's `?constants=` location reading rode along in `src/web.rs` unchanged
and is not re-counted here.

**Reading discipline:** this fork was written from `docs/api/` (all four),
`crates/jidousha/examples/`, and giri (a game, not the engine — at
`games/giri/` then, `attic/giri/` since wave 1.1). No
file under `crates/*/src/` was opened, and neither was `docs/internal/` nor
any ADR but 0038 and 0041 (both named by the handoff). Wave 0b held the same
line: `games/giri/` (the port source) and this crate, and nothing under
`crates/*/src/`.

**The selection-bugfix session (2026-09-06) read** `CLAUDE.md`, the
`make-game` skill, this game's `UI.md`, `DESIGN.md`, `GDD.md`, `FINDINGS.md`
and its whole `src/`, and nothing else: no file under `crates/*/src/`, no
`docs/internal/`, no ADR, and it asked `docs/api/` nothing — every question it
had was about this game's own UI state, and the two entries it files below are
about this repository rather than about the engine's documents.

**The double-drawn-cast session (2026-09-08) read** `CLAUDE.md`, the
`make-game` skill, this game's `UI.md`, `FINDINGS.md`, `screens/README.md` and
its whole `src/`, plus `docs/api/jidousha-api.md`'s `Vec2` tour (which of
`distance`/`length_squared` to reach for, and whether `normalize` on a zero
vector is safe — it is not, and the placement never normalizes) and
`docs/api/jidousha-testing.md`'s `DrawnQuad` entry. Nothing under
`crates/*/src/`, no `docs/internal/`, no ADR but 0041, which the handoff names.
Its one new entry is about this repository's own documents.

**The legibility session (2026-09-09) read** `CLAUDE.md`, the `make-game`
skill, this game's `UI.md`, `GDD.md`, `FINDINGS.md` and its whole `src/`.
Nothing under `crates/*/src/`, no `docs/internal/`, and no ADR. It asked
`docs/api/` **one** thing it had not asked before — whether the input snapshot
carries a wheel event a scripted run can record, so a picture could be taken
one notch of the wheel out (it does: `InputEvent::Scrolled`, and the game's
conductor now has an `Act::Scroll` beside its clicks). The rest of the session
is arithmetic over the game's own data drawn into `Panel`s the floors already
judge. It closed **G-022** with its post-fix measurement and files **two new
entries**, both about *this game* and one of them about a handoff.

**The job-board session (2026-09-06) read** `CLAUDE.md`, the `make-game`
skill, this game's `UI.md`, `DESIGN.md`, `GDD.md`, `CAST.md`, `FINDINGS.md`
and its whole `src/`, plus one line of `docs/api/jidousha-testing.md` (the
`DrawnQuad` fields, checking whether a recorded quad carries the draw band it
was submitted on — it does not, which is why the answer below is a rule about
what the game draws rather than a smarter judge). Nothing under
`crates/*/src/`, no `docs/internal/`, no ADR. Its two entries are about this
game.

**Wave 1.1 read** `CLAUDE.md`, the `make-game` skill, this game's own
`GDD.md`, `DESIGN.md`, `UI.md`, `CAST.md` and `FINDINGS.md`, and its whole
`src/`. It opened no file under `crates/*/src/`, no `docs/internal/`, and no
ADR. It asked `docs/api/` nothing new — the scorer is arithmetic over the
game's own data and the two surfaces it added are `Panel`s like every other,
so its two entries below are about *this repository* rather than about the
engine's documents.

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


## Wave 1.2 (the asks module) — **2 new findings**, both the game's own

**Wave 1.2 read** `CLAUDE.md`, the `make-game` skill, this game's own
`GDD.md`, `DESIGN.md`, `UI.md`, `CAST.md` and `FINDINGS.md`, and its whole
`src/`. It opened no file under `crates/*/src/`, no `docs/internal/`, and no
ADR. **It asked `docs/api/` nothing new**, and that is a real answer rather
than an empty section: the module is arithmetic over the game's own data, its
three new surfaces are `Panel`s like every other, its two new inputs are
clicks on rectangles `layout.rs` already knew how to state, and its
occurrences ride the one scheduler the substrate landed in S1. The engine's
documents were not the cost of this wave; the two entries below are about
*this game*, and both are numbers the next playtest is owed.

### G-021 — the game's own: the standing rate is a coarse lever at the rate it ships at

Class: **the game's own** (a tuning fact, found by the check that exists to
find it) · Game: ninjo · Files: `games/ninjo/src/compliance.rs`,
`src/asks.rs`, `GDD.md` §9 · **Open — a question for the playtest**

The amendment's policy test asks that "raising fight pay by one drawer step
shifts at least one fighter's next choice". Expected: one tap of the
standing-rates panel moves somebody. Happened: at the shipped fight rate of
24g, **three** taps of the panel's 4g step are needed before anybody's *work*
changes, and the first person to move is Hana at 36g.

The arithmetic is not a bug and the check reports it rather than hiding it. A
wage's pull is `(pot_affinity + desperation) x wage / 10`, so 4g is worth one
or two points to most of the band, while the aptitude term that holds somebody
in their own trade is six. Walking the rate across the panel's whole range
shows the lever working exactly as designed — somebody's work changes at 12g,
16g, 36g and 56g, and five of the ten drift into fight work as it rises — so
what was wrong was the *test's* framing, not the mechanism.

**What was done:** the battery asserts the whole walk (the four turning
points, as shipped literals) **and** pins the distance from the shipped rate
to the first drift at three steps, so a moved weight moves a literal and the
mutation round notices. Nothing was tuned to make a one-step test pass: moving
the opening fight rate to sit just under a wall would have been tuning to the
instrument.

**What the owner is asked:** is a 4g step the right size for a policy control,
and is 24g the right price for fight work when 36g is what it takes to pull a
scout off her own trade? The answer is a content decision (`asks::RATES` and
`asks::RATE_STEP`), not a code one.

### G-022 — the game's own: the crowding at the settlement is the labels, not the zoom

Class: **the game's own** (a rider, answered by measuring it) · Game: ninjo ·
Files: `games/ninjo/src/floors.rs`, `src/camera.rs`, `UI.md` §4 ·
**Open — the next UI session's**

The wave's rider asked for the default camera to step out one level "so the
ten figures are not on top of each other", with the readability floors
re-asserted at that zoom and the note that if names stop being legible, "the
zoom is the thing that yields".

Expected: a wider view separates the figures. Happened: **zooming out cannot
separate them, because the overlap is scale-invariant** — the map is drawn in
world units and pulling the camera back shrinks the figures and the gaps
together. What it *does* change is legibility: a name is drawn at twelve world
units, which is exactly the twelve-reference-pixel floor at the default
camera, and one notch of the wheel out puts it at 10.7. So the floor refuses
the step, which is what the rider said should happen, and the default stays at
540.

The crowding the owner saw is real and has a different cause: the homes are
two rows of tents two tiles apart (`CAST.md` §4), a figure is 32 world units
tall (two tiles), and a name is drawn *under* its figure — so the northern
row's names land on the southern row's heads. The fix is a layout one — the
label above the figure for one row, or a name only for the selected character,
or three tiles between the rows — and each of those is a UI or content
decision this session did not have a mandate for.

**Reopened 2026-09-08, on its subject rather than its measurements.** This
finding measured the settlement's crowding and concluded the cause was label
placement against row spacing. The measurements are not wrong and none of them
changes. **Its subject was**: the map it measured was drawing twenty figures
for ten people (G-023), so "the ten figures are on top of each other" was a
report about a duplicate and not about the rows. The crowding question is to be
**re-judged by the owner** at ten figures before anybody moves a row or a
label, and the layout suggestions below — the label above the figure for one
row, a name only for the selected character, three tiles between the rows —
are not to be acted on until they are. The fix session deliberately did not
touch them.

**What was done (2026-09-08):** `floors::map_legibility` states the numbers as
a floor and the verify report prints them (`map labels 12.0px at the default
zoom, 10.7px one notch out`), so the next session inherits a measured fact
instead of a disagreement.

**Closed 2026-09-09 by the legibility session, and here is what it did and
what it then measured.** The reopening was right and the fix is neither of the
layout suggestions: **the floor was being used as a veto and it should have
been a governor.** A name was going to be drawn whatever the camera did, so the
twelve-pixel floor could only forbid the zoom; now the *label* yields — a
label is drawn only where it clears the floor and lands on nothing already
drawn (a figure, a marker, an earlier label, or a chrome surface that is up),
in one fixed order, and below the floor none is drawn at all. So:

- **the crowding is gone at the default camera without moving a row or a
  home.** Of the nineteen words the settlement can say, **twelve survive**;
  the seven that drop are the ones that were landing on the southern row's
  heads, which is exactly what this finding measured. `screens/ninjo-settlement-reference.png`
  is the after picture, and the seven that dropped are the *northern* row's
  names, so two of the ten figures now stand anonymous at the default zoom.
  **That is the residual, and it is the owner's to judge**: it is a real cost,
  it is the price of never drawing a name across a face, and a name only for
  the selected character (this finding's own second suggestion) is what the
  chrome label now gives on top of it.
- **zooming out is permitted**, which is what the wave-1.2 rider wanted and
  could not have: one notch out drops every map word and keeps every figure
  (`screens/ninjo-zoomed-reference.png`), and the selected character stays
  named because that name is chrome. **The default camera was deliberately not
  moved** — where it should sit is a play judgement, and the point of the rule
  is that it is now a judgement somebody can make by making it.
- `floors::map_legibility` keeps stating the two numbers, and
  `verify::map_labels_are_governed` asserts the rule itself: every drawn label
  clears the floor and touches nothing, none is drawn below it, the selected
  name survives both, and the same frame drops the same labels twice running.

**What the owner is asked for**, and the reason this closes rather than
reopens again: play it, zoom out one notch, and say whether the default should
move and whether two unnamed figures at the settlement is a price worth
paying. Both are content judgements now rather than measurements.

## The legibility session (2026-09-09) — **2 new findings**, both the game's own

It closed G-022 above with the measurement that finding asked for. Its two new
entries are a pattern the owner has now paid for three times, and a handoff
that forbade in one section the thing it required in another.

### G-024 — the game's own: three surfaces have now outlived the world they were built for, and nobody was looking for the fourth

Class: **the game's own** (a pattern, not an incident) · Game: ninjo ·
Files: `games/ninjo/UI.md`, `src/flow.rs`, `src/screens.rs`, `GDD.md` §8 ·
**Open — the wave-1 close's**

Three sessions running have retired a piece of S1 presentation that was
correct when it was written and wrong by the time it was found, and each was
found by an owner playtest rather than by a check:

| the residue | what it was for | what made it wrong | found by |
|---|---|---|---|
| the **double selection** (G-017) | S1's dispatch pick, beside wave 0a's character selection | wave 1.1 made a party a character, so one roster had two indices over it | the wave-1.1 playtest |
| the **double figure** (G-023) | three party tokens stacked on a town tile | wave 0b gave every person their own figure, so the token was a second picture of the same person | the wave-1.1 playtest |
| the **party strip** (this session) | the only way to see and select a one-person band | wave 0a's meters, wave 1.1's roster and the map's own figures took its four jobs one at a time | the wave-1.2 playtest |

**The shape is the same every time**: a surface built for a world where it was
the *only* answer to a question, kept through the waves that gave that question
better answers, and retired only when somebody played the build and said it
felt cluttered. Nothing in this repository asks "what is this surface still
for?" — the floors ask whether a surface is legible, the content floors ask
whether it says what it means, and both pass with flying colours on a surface
nobody needs. **A check cannot find this**; a reading can.

**Where the rest should be swept for**: the wave-1 close's exemplar audit
(`GDD.md` §8). The audit walks the surfaces against what the built world
actually needs, and this is the class it should be walking for — the
candidates a reader should start from being every surface whose justification
in `UI.md` names a wave earlier than the one that last touched the question it
answers. Two are visible from here without judging them: the **notices band**
(the last two things the player did, in a drawer whose feed now carries a `?`
on every decision) and the **meters band's two chips** (`idle` and `away`,
which the roster's ten rows now state per person). Neither is wrong today;
both are surfaces whose question moved.

**What this session did about it:** retired the strip, listed its four jobs and
where each now lives (`UI.md` §3), and filed this. It did not touch the two
candidates above — naming them is the finding, judging them is the audit's.

### G-025 — the game's own: a handoff forbade in its fences what it required in its verification

Class: **the game's own** (a handoff defect, and the session deviated) ·
Game: ninjo · Files: the legibility session's handoff, `games/ninjo/src/sim.rs`
· **Open — the owner's to rule on**

The legibility handoff's fences say **"no sim state"** twice — in its opening
line and again in its "You may NOT" list. Its **Verify** section requires that
"the feed breakdown equals the `Judged` **recorded with that decision**", and
its work item 1 requires the same breakdown for "a choice already made" so
that "why did Ludo go there" is answerable *after the fact*.

Expected: to build both halves inside the fence. Happened: **they cannot both
be met.** The arithmetic behind a decision cannot be recovered later without
re-running the scorer against a world that has moved — the board has been
claimed, the wage stepped, the regard drifted — and a breakdown that disagrees
with the decision it explains is the failure mode the same handoff names first.
The job row's half needs no record at all (it is a preview of *now*); the
feed's half needs the terms kept at the moment they were weighed.

**What was done, and on whose authority:** the session kept them, on the
`Event::gold` precedent — a derived fact recorded on the occurrence because
recomputing it later would lie, which `sim.rs` already states in those words
for money. `Event::judged` is a record and not an input: nothing in the
simulation reads it, no arithmetic depends on it, no transcript prints it, and
the invariance sweep and every replay are byte-identical with it in place. It
is nonetheless **a change to a sim struct in a session told not to make one**,
and it is called out here and in the PR rather than shipped quietly.

**What the owner is asked to rule:** whether "no sim state" in a UI handoff
means "change no state the world decides from" (which this obeys) or "add no
field to a sim struct" (which this breaks). The second reading makes the
feed's half of a breakdown unbuildable by any session that is also forbidden
to compute a second answer, so the wording is worth settling before the next
UI handoff inherits it.

**Two smaller things about the same handoff**, recorded because they cost
minutes rather than decisions: it names `reason_for` as the function that
collapses a `Judged` to its loudest term — the function is `autonomy::words`,
and there is no `reason_for` in the crate; and it asks for the party strip's
jobs to be listed "in the PR", which is where they are, but the durable place
for them turned out to be `UI.md` §3, since a PR body is not somewhere anybody
reads twice (`make-game` §C says exactly that about findings).

## The job-board session (2026-09-06) — **2 new findings**, both the game's own

The documents were asked nothing they did not answer. `jidousha-testing.md`
was opened once, for `DrawnQuad`'s fields, and it was accurate; the panel /
floors / frames machinery this game already had absorbed a whole new surface
without a new engine question, which is the thing that made the session cheap.

### G-019 — the game's own: a site marker showed a count and took an order, and the work was invisible

Class: **the game's own** (a wave-1.1 gap the owner played into) · Game: ninjo ·
Files: `games/ninjo/src/flow.rs`, `src/board.rs`, `src/sim.rs`, `UI.md`,
`DESIGN.md` · **Fixed here**

Reported by the owner from the wave-1.1 playtest of the deployed build: a site
marker shows only a quest count, so there is no way to see the work or to
choose it.

Expected, from `CAST.md` §2 and the wave-1.1 board: the player picks a person
for a *kind of work*, because the whole trait vocabulary is built to make
"whom do I send, and why" a decision. Happened: dispatch was "this person to
this site", the site handed out its first open row, and the marker's `6
quests` was the only thing on screen about what stood there. Wave 1.1 had
grown the board from seven jobs to twenty-four and put a task type on every
one of them — which made the gap much larger than it had been, because a
count of six now stood in for six *different* pieces of work.

**What made it invisible to the checks:** every assertion about dispatch was
about the site. `expected_events` pinned arrival minutes, `judge_one_path`
compared movement classes, and the scorer's own battery weighed
`SeekWork { site }` — so a build in which the UI named one job and the sim
claimed another would have passed all of them. The fix carries its own
instrument: `autonomy::judge_named_claims` walks every spent row on every
board and requires that the departure which claimed it named it, which is a
check that could not have been written while a claim was a count.

**What the next session inherits:** the board and the faces list share the
left of the screen and are never open together, so the base screen is now two
control sets (`floors::targets` and `floors::board_targets`) and
`controls_for` picks between them. A third over-the-map surface has to pick a
side or take a third set.

### G-020 — the game's own: a recorded quad does not say which draw band it came from

Class: **the game's own**, though it starts at an engine boundary · Game: ninjo ·
Files: `games/ninjo/src/screens.rs`, `src/frames.rs` · **Worked around here,
and the workaround is a rule**

Found while adding the job board. `frames::judge_chrome` finds a row of chrome
on the recorded frame by counting glyph quads inside the row's own box; the
board's footer sits at the same world *y* as the Old Crypt's `4 quests` label
and overlaps it in *x*, so a glyph of the map label landed inside the footer's
box and the count came out one too high. The row was drawn correctly and the
panel covers the label on screen — the judge was counting a glyph the reader
cannot see.

Expected: a way to ask the frame for the chrome's own glyphs. Happened:
`DrawnQuad` carries `batch`, `texture`, `corners` and `tint`, and no depth or
layer (`jidousha-testing.md` is accurate about this; nothing misled). The
document is not wrong — it is a gap, and a small one, because the honest fix
turned out to be a rule about the game rather than a smarter judge.

**What we did on that basis:** extended UI.md §3's standing law — *under an
open drawer the map says nothing* — to the job board, which is a panel over
the map in exactly the same sense. While a board is open the map draws no
words at all; the board carries the site's name and its open count in full,
and the labels it hides are the ones it is standing on. That is a better rule
than the one it replaced anyway. It is filed here so the next surface over the
map meets the fact rather than the symptom, and so that a `layer` on
`DrawnQuad` — if the engine ever wants one — has a use case on record.

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

### G-015 — the sweep's `games/giri/` paths in the engine's own comments

Class: docs · Game: ninjo · Files: `crates/*/src/`, `docs/api/` · Open

Wave 1.1 retired giri to `attic/giri/` (the handoff's last item). Every path in
`docs/internal/` that pointed at one of its documents was repointed in this
commit; the ones inside `crates/*/src/` doc comments were not, because engine
source is outside a game session's fence — and `docs/api/` carries them
because it is generated from those comments. The dangling references are:
`crates/jidousha-render-core/src/font/mod.rs` and `style.rs` (G-003),
`crates/jidousha-platform/src/driver/mod.rs`, `src/web/render_scale.rs` and
`src/lib.rs` (`games/giri/UI.md §6`, and `"games/giri/assets"` as the worked
example of a game's own asset root, which reaches `docs/api/jidousha-api.md`
and `tools/api-doc/concepts.md`).

Expected: a move that is mechanical in the game's own tree to be mechanical
everywhere. Happened: four engine files and one generated document now name a
directory that does not exist. None of them can mislead an *action* — they are
citations and an illustrative path — but the asset-root example is the sharper
half: it teaches `asset_source("games/giri/assets")` for "a game at
`games/giri/`", and the game it names is in the attic. A sanitation pass or an
engine session can repoint them at `games/ninjo/` and at `attic/giri/`; a game
session may not.

### G-016 — the game's own: an order cannot be addressed to a world-minute a fast clock never visits

Class: **the game's own**, recorded here because the next wave meets it ·
Game: ninjo · Open

Wave 1.1 retuned 1x to 24 world-minutes a real second (the owner's 0a
playtest). At 4x the clock then carries **1.6 world-minutes a tick**, which
has two consequences the substrate's shape did not anticipate and neither is
a bug:

1. **The clock skips minutes.** `floor(1.6t)` never lands on a minute whose
   residue mod 8 is 2, 5 or 7, so a scripted order addressed at such a minute
   fires at the next one the clock does visit — and at a *different* next one
   under each speed script, which breaks the invariance sweep at the
   conductor rather than in the world. The sweep's four orders were moved to
   minutes eight apart (8, 16, 24, 360) for this reason, and the reason is
   worth a sentence because the next content change will want to move them
   again.
2. **A multi-tick input sequence lands later than it starts.** An order is
   four ticks of clicking, which at 4x is six world-minutes. `When::Approaching`
   is the answer: the conductor simulates the clock forward the ticks the
   sequence takes and starts it early enough to *land* on the minute it names.

The design rule that survives both: **the invariance claim is about the
world's occurrences, not about the player's inputs**. Two speed scripts can
issue the same order at the same world-minute only where the clock visits
that minute at both speeds, and the sweep's scripts are authored to that.

**Amended by the job-board session (2026-09-06), and the rule is now a
check.** An order became three clicks (person, marker, job row), so its lead
went from three ticks to five and every one of the sweep's order minutes had
to be re-derived: they are now `sweep::ORDER_MINUTES` — 16, 32, 48 and 360,
multiples of eight sixteen apart, because six ticks of clicking at 4x is nine
world-minutes and 8 is where `floor(8k/5)` lands. The paragraph above said
"the next content change will want to move them again", and it did; so
`sweep::visited_minutes` now enumerates the minutes each speed's clock
actually reads and `sweep::orders_are_addressable` asserts every order minute
is in all three sets. **The entry stays open** because the rule still binds
the next change — but it now fails loudly instead of quietly, which is the
half of it that was missing.

### G-017 — the game's own: wave 1.1 made a party a character and left two selections over one roster

Class: **the game's own** (a wave-1.1 gap) · Game: ninjo ·
Files: `games/ninjo/src/flow.rs`, `src/screens.rs`, `UI.md` · **Fixed here**

Reported by the owner from a playtest of the deployed build: select a character
in the party strip, then click a *different* character's map sprite, and both
are ringed gold on the map.

Expected: one selection, because after wave 1.1 a party *is* a character — ten
parties, ten people, same order, `party.member == index`. Happened: S1's
dispatch pick (`Flow::selected`, an index over parties) and wave 0a's character
selection (`Flow::selected_person`, an index over people) survived wave 1.1 as
two independent fields over what had become one list, and each drew its own
ring — the token's at 36 units and the figure's at 38. Nothing was wrong with
either field on its own; what was wrong is that wave 1.1 unified the two lists
and did not unify the two states over them. Neither field's own tests could see
it: each asserted its own index, and no check counted rings.

The fix is a deletion, not a bridge: one field, written by every select path
and read by everything that highlights (UI.md §3b). The reproduction is
`verify::one_selection` — chip-select one, sprite-select another, count the
gold marker-sized quads on the photographed frame, expect the shipped literal
**one** — and `screens/ninjo-selection-reference.png` is that frame.

**What the next wave inherits:** the character panel's *body* is no longer a
click target (its close button and trait chips still are). It had to stop being
one: the panel now opens on any selection, so it is up for the whole of a
two-click dispatch, and at the reference camera it lies across the Watchtower's
and the Black Vault's markers — a swallowing body would have made two of the
four sites unorderable. `dispatch_reads_the_selection` orders to all four with
the panel open, so a later layout change cannot take that back quietly.

### G-018 — the game's own: an idle character is drawn twice, and the second one drifts further with every index

Class: **the game's own** (a wave-1.1 gap) · Game: ninjo ·
Files: `games/ninjo/src/screens.rs` ·
**Fixed 2026-09-08** — see G-023, which is the owner's report of it from the
deployed build and carries the fix, the floors and the doc-truth half

Found while working out which of a person's two portraits the one selection
ring belongs on, and left alone because this session's fence is the selection.

Expected: one picture of a person on the map. Happened: two. `content` draws a
**figure** at each at-home character's own tile (wave 0b's cast-at-home) and
`draw_map` draws a **party token** for every party, always — and after wave 1.1
those are the same ten people. The token carries S1's two-parties-on-one-tile
nudge, `(index * 4, index * -4)` world units, which was written when there were
three parties: at index 9 it is 36 units away, nearly two tiles, so the tenth
character's second portrait is not even overlapping the first.
`screens/ninjo-settlement-reference.png` shows all ten pairs at world-minute 0.

It is the same 1.1 gap as G-017 one layer down — two renderings of one roster
rather than two selections over it — and it is why `screens::selection_ring`
rings the *figure* when somebody is home and the *token* when they are on the
road: those are the two places a person is actually drawn. Fixing it is a
decision about what the map should show (a figure and no token while somebody
is idle, presumably, and the nudge retired or restated for ten) and it changes
five committed screenshots, which is an owner call rather than a bugfix
session's.

**G-010 stays open** for the fifth wave running, untouched here: this session
added map-space content (ten figures and their names) but no *new kind* of it,
and the culling pair it describes is unchanged.

### G-023 — the game's own: the map drew twenty figures for ten people, and the exception it hid behind was never written down

Class: **the game's own** (a wave-1.1 gap) and **a document that said less than
the code did** · Game: ninjo ·
Files: `games/ninjo/src/screens.rs`, `src/floors.rs`, `src/flow.rs`,
`src/layout.rs`, `src/shots.rs`, `src/verify.rs`, `UI.md` §3, §4, §6 ·
**Fixed here.** Closes G-018, which is the same defect found from the inside
two days earlier and left open as an owner call.

Reported by the owner from a playtest of the deployed build b48216a: character
sprites appear duplicated on the map, offset diagonally up-and-right; the whole
southern row of tents shows doubles, and in the northern row only Rin does.

Expected: one picture of a person. Happened: two, on every idle character.
`screens::content` drew a figure per person where `Lens::at_home`, at their
doorstep; `screens::draw_map` drew a sprite per party at `token_rect`. Since
wave 1.1 a party *is* a character and stands at that character's own doorstep,
so both paths drew the same person in the same place. `token_rect`'s per-party
nudge — `(index * 4, index * -4)` world units, S1 residue from three parties
stacked on the town tile — is what made the second copy visible and diagonal,
and it predicts the report exactly: index 0's duplicate hid perfectly, 1 to 3
offset 4 to 12 units and read as thickened sprites, index 4 (Rin) offset a
whole 16-unit tile and separated, and 5 to 9 offset 20 to 36 units per axis —
one to over two tiles. Rin is the fifth row of the northern row of tents and
the only one in it far enough out to read as two people, which is what the
owner saw.

**The second defect inside the first**, and the more expensive one: the nudge
put a token up to three tiles from the tile its party was actually standing on,
and the error grew with the roster. A figure that lies about where somebody is
lies to a click and to a camera focus too.

**Why nothing caught it, which is the half that is about a document.**
`UI.md` §6 said, without qualification, that every surface owes "every row of
its content in the `Panel`" — that is what makes `floors.rs` able to judge what
was meant and `frames.rs` able to find it on the frame. `screens.rs` took an
exception for the party tokens anyway, on the grounds that their between-tile
position is derived at draw time (ADR-0041), and recorded it **in its own
module header** — where nobody reading the rule in §6 would ever meet it. So:

- `floors.rs` judges the `Panel`, and the tokens were not in it;
- `frames::judge_chrome` asks only whether what the screen said is on the
  frame, never whether anything else is;
- the map-label overlap check covers `world_runs`, and labels were drawn once;
- `shots.rs`'s settlement check asserts *sim* truth — ten people, all home —
  which held perfectly.

The duplicate lived exactly in the gap the undeclared exception carved, on all
sixteen photographs, for two waves. **What was done on the document's
authority** was to leave the tokens outside the `Panel` when the figures moved
into it in wave 0b: §6 read as satisfied because the figures were content and
the tokens were "the drawing", and the sentence that would have said otherwise
was in a file the rule does not point at.

The exception was also not needed. Interpolation is a reading of the clock; a
`Panel` icon takes a position like any other, and `ui::draw` culls a world icon
to the camera exactly as the token loop did.

**The fix is one function.** `screens::where_drawn(lens, who, now)` is the
single answer to "where is this person drawn right now" — their doorstep while
`at_home`, their party's interpolated position otherwise, never both — and the
figure, the name under it, the selection ring and the map's hit-test all read
it and compute nothing of their own. The nudge became a **placement** rather
than a formula over an index: everybody takes the place they are standing if it
is clear and steps out a ring only because somebody else is standing close
enough to read as them, so a person alone is drawn on their own tile and two
people together are two figures.

**And the floor gap is closed, in the direction it was open.**
`floors::judge_cast` reads the `Panel` on every screen state the content floors
judge; `floors::judge_figures` reads the frame on every photograph, asking the
question `judge_chrome` never asked — whether the frame carries anything the
screen did not say it would draw. `verify::one_figure_each` is the
reproduction, and it failed before the fix on both photographed frames with the
owner's own numbers (`the settlement: 8 figure-sized quad(s) landed at corners
the screen never named ... the screen says 25 of them and the frame carries
35`). §6 now carries the whole list of what is exempt from the `Panel` and why,
and says that an exemption not in that list is a defect rather than an
exemption.

**What the next session inherits:** a site marker now takes a map click ahead
of a person's figure. It had to: a party working at a site stands on that
site's marker, and now that figures answer clicks on the road, a figure taking
it there would make the site unorderable while anybody worked it — the same
refusal `UI.md` §3a already makes for the character panel's body. A person
standing on a site is still selectable from the strip, the roster and a faces
list. And **G-022 is reopened on its subject**: it measured the settlement's
crowding against a map drawing twenty figures for ten people, so whether ten is
still crowded is the owner's to say before anybody moves a row or a label.

**G-010 stays open** for the sixth wave running, untouched by the
double-drawn-cast session: the party tokens moved from `ctx.sprite` into the
`Panel`'s world icons, which is a change of *which list* map-space content
lives in and not a new kind of it, and `ui::draw` culls a world icon on the
same bounds the token loop did. The culling pair G-010 describes is unchanged.
