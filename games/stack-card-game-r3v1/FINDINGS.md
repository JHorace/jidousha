# Stack card game (r3v1) — findings

What the documents cost this build, in the shape `docs/internal/e0-findings.md`
uses. G-numbers continue the sequence across games (the highest under `games/`
at the time was G-069).

**Reading discipline, session 1 (yakin tick, V1).** Read: `CLAUDE.md`, the
`make-game` skill, `tools/yakin/{WORKER,DOCTRINE}.md`, the task specs
(`tools/yakin/tasks/stack-card-game{,-r3v1}.md`), all five `docs/api/`
documents (`jidousha-api.md` Quickstart, Concepts, the Reference entries used;
`jidousha-testing.md` prose and Reference; `jidousha-controllers.md` whole;
`jidousha-capture.md` whole; `jidousha-ui.md` "Building a screen", "The
floors" and the Reference entries used), and
`crates/jidousha/examples/prototype_kit/` (main, checks, verify, capture).
Reading a sibling game: `games/keifu/Cargo.toml`, the header of
`games/keifu/FINDINGS.md` and `games/keifu/mutants/dock.txt` for the shapes of
a manifest, a findings file and a mutation list. **Engine source
(`crates/*/src/`), `docs/internal/`, `docs/adr/`: not opened.** No other
round's run of this idea was opened (the task's comparison hygiene).

---

### G-070 — `judge_panel`'s `controls` names a control, and nothing says by what

Class: docs · Session: r3v1 · Owner: `docs/api/jidousha-ui.md` (The floors;
`judge_panel` Reference)

**Doing:** passing the PASS button to `judge_panel` so the floor that refuses
"a row lying across a control it is not the label of" can see it.

**Expected:** the `String` in `controls: &[(String, Rect)]` defined — the
label's exact text, an id, or a name the floor only prints.

**Happened:** the prose says "you pass the controls, named" and the Reference
example passes `&[]`. Nothing says what the name is matched against, so a game
cannot tell whether its button's own label is exempted.

**What I did:** passed the label's exact text, `("PASS (space)", PASS_BUTTON)`;
the floor reports no breach on every screen, which is consistent with the name
being the label's text but does not prove it. No staged breach was written
against it (I could not state what should bite).

### G-071 — the clearance margin reads 0.00 on any screen with a full-view dimmer

Class: docs · Session: r3v1 · Owner: `docs/api/jidousha-testing.md` ("Then
print the margin it passed by")

**Doing:** printing the closest-quad-to-the-edge margin over every frame the
run judges, as the document asks.

**Expected:** a number that turns the bounds cliff into a gradient.

**Happened:** the result screen draws a dimmer the size of the whole view, so
the fold over every quad returns exactly 0.00 — honest, and useless: the
number no longer says anything about the layout the dimmer covers.

**What I did:** folded the margin over the play screens only and said so in the
summary line (`closest quad to the edge (play screens; the result dimmer is
flush by design)`). The bounds check itself still runs on the result screens.
A sentence in the document — exclude deliberate full-view quads from the margin,
or report it per screen — would have saved the detour.

### G-072 — the capture size the document gives is too small to read a text-heavy screen

Class: docs · Session: r3v1 · Owner: `docs/api/jidousha-capture.md` (Capture at
the recorder's aspect ratio)

**Doing:** opening the captured PNG and naming what I saw, as the document asks.

**Expected:** the suggested 480x270 to be a picture someone can judge.

**Happened:** at 480x270 a 12-unit row of a 540-unit-tall design is about six
pixels tall; the layout was visible, the words were not, and the words are the
whole of this game's decision surface.

**What I did:** captured at 960x540 (the same 16:9, asserted in `capture.rs`).
The document's rule — same aspect — is right; its number is a shapes-game
number. "Pick a size at which the smallest text is legible" would generalise.

### G-073 — the three controller numbers are written for an aimed shot; a turn-based game has to translate them

Class: docs · Session: r3v1 · Owner: `docs/api/jidousha-controllers.md` (Three
numbers, printed every run)

**Doing:** printing the three numbers for the three players of a card duel.

**Expected:** the genre-neutral framing the document promises ("Every claim in
it would be true of a driving game or a fighting game").

**Happened:** the three players transferred cleanly (reader / raw power /
idle). The three numbers are stated in shots, approaches and landing distance,
and only `examples/slalom` and `pong` are worked; nothing worked is turn-based.

**What I did:** translated them: *answered the stack in N of M windows* (did
it engage when a rival item was up and it could afford a reply), *planned
margin* (the life margin its moves were previewed to resolve to), *landed Y off
plan* (how far the margin the stack actually emptied at fell from that plan —
the rival's replies are the error). The reader lands 0.2 off its plan; raw
power 3.8 — the number that says the raw player's plans are fictions, not that
it cannot act. Recorded so the next turn-based game has a precedent.

### G-074 — the game's own: a legality list and a marked-row list came out in different orders

Class: game · Session: r3v1 · Owner: this game

**Doing:** asserting that the stack rows marked on the frame equal the slots
`legal_targets` names.

**Happened:** both said `{A, C}`, one as `[0, 2]` and one as `[2, 0]`
(`legal_targets` walks the stack bottom first; the screen counts from the top),
and the first verify run failed on the order alone.

**What I did:** `screen::aim_targets` sorts its slots, so the one list the
marks, the click hit-test and the check read is top-first by construction.
EOF