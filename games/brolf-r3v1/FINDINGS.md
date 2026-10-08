# Brolf (r3v1) — findings

What the documents cost this build, in the shape `docs/internal/e0-findings.md` uses:
what I was doing, what I expected, what happened, and which document owns it.
G-numbers continue the sequence across games (the highest on `main` when this run
started was G-069). Other round-three runs were in flight on other branches at the
same time and may also have started at G-070 — a collision is a renumbering at merge,
not a conflict of substance.

**Reading discipline (yakin task brolf-r3v1, one V1 tick lineage).** Read: `CLAUDE.md`,
the `make-game` skill, `tools/yakin/{WORKER,DOCTRINE}.md`, the task specs
(`tools/yakin/tasks/brolf-r3v1.md`, `tools/yakin/tasks/brolf.md`), the design template,
`docs/api/jidousha-api.md` (Quickstart, Concepts, the Reference entries used),
`docs/api/jidousha-testing.md` (prose and the Reference entries used),
`docs/api/jidousha-controllers.md` and `docs/api/jidousha-capture.md` whole, and
`crates/jidousha/examples/prototype_kit/` (main, checks, capture; verify's summary
block). From `games/keifu/`: its `Cargo.toml`, one mutants list and the head of its
`FINDINGS.md`, for shape; its G-numbers were grepped. `tools/mutate`'s docstring.
`docs/api/jidousha-ui.md`: not opened — the chrome here is two status lines and a
result card, which the kit is not for. **Engine source (`crates/*/src/`),
`docs/internal/`, `docs/adr/`: not opened.** No other round's brolf branch, PR or
folder was opened (comparison hygiene).

---

### G-070 — a ring drawn over the play has no recipe that works

Class: docs · Session: brolf-r3v1 · Owner: `docs/api/jidousha-api.md` (Concepts, "Those
five verbs are the whole vocabulary")

**Doing:** drawing the battle-royale zone — a circle the size of most of the course, with
golfers, balls and cups both inside and outside it — and the next zone, dashed.

**Expected:** the ring recipe Concepts gives to hold.

**Happened:** Concepts says "a ring is two circles with the inner one in the background
colour". That paints a disc the size of the zone over everything under it; it works only
for a ring with nothing inside it, and a zone exists to have the game inside it. It is
also sixteen wedges per circle, so a ring of radius 24 is visibly a hexadecagon. The
sentence is right for a small marker and silently wrong for the common big case.

**What I did:** read it before writing the ring, saw the paint-over, and did not use it.
Wrote the ring as 64 `ctx.line` chords, dropped every chord with an end off the course
(so a ring larger than the screen draws only its on-screen arc and the bounds check
holds), and every other chord for the dashed next ring (`draw.rs`, `ring_segments`). The
verify run finds the ring by asking for a ring-tinted quad on a chord's midpoint.

**Fix:** one sentence after the two-circles line: for a ring over content, draw chords
with `ctx.line`, and clip them to the screen yourself.

### G-071 — the margin print is always 0.00 for a game with a full-bleed band

Class: docs · Session: brolf-r3v1 · Owner: `docs/api/jidousha-testing.md` ("Then print
the margin it passed by")

**Doing:** printing the closest-quad-to-the-edge margin, as the testing document asks.

**Expected:** a number that turns the bounds cliff into a gradient.

**Happened:** the status bar sits on a backing band that spans the full camera width and
starts at its top edge — flush on three sides by design — so the fold over every quad
printed `0.00` on every run, which says nothing about the text, labels and rings the
check exists for.

**What I did:** kept the bounds check over every quad (flush is on screen; the band
passes `contains_rect`), and excluded quads in the band's own tint from the margin fold,
saying so in the summary line (`closest quad to the edge, the full-bleed status band
aside: 0.35`).

**Fix:** a sentence beside the fold: a deliberately flush background passes the check
and should be left out of the margin, or the margin reads zero for ever.

### G-072 — `manual_is_multiple_of` is a stock lint games now meet

Class: docs · Session: brolf-r3v1 · Owner: `docs/api/jidousha-api.md` (the lint list
closing Concepts)

**Doing:** sampling one frame a second in the verify loop, `tick % 60 == 0`.

**Expected:** the list of stock lints games have tripped to include it, since every game
that samples on a cadence writes that line.

**Happened:** `cargo clippy -- -D warnings` on toolchain 1.94 rejects it
(`manual_is_multiple_of`, wanting `tick.is_multiple_of(60)`); the list does not name it.
The list says it is a sample and the tool is the authority, which held — one edit.

**What I did:** `tick.is_multiple_of(60)`.

**Fix:** add it to the sample; it is the cadence idiom.

### G-073 — the game's own: the first build's cups made champion a seven-second win

Class: game · Session: brolf-r3v1 · Owner: this game (`tools/yakin/runs/brolf-r3v1/DESIGN.md`)

**Doing:** the first full match of the three-player run.

**Expected:** a two-minute match where the zone, contact and extraction matter.

**Happened:** with cups sinkable once per golfer, a perfect landing function and no shot
error, both the good player and the chaser sank three cups in under seven seconds; on
the second build NPC brutes, free to club anyone not *currently* stunned, kept two
golfers stunned for the rest of the match. Both were design faults the design stage
could not see without a run.

**What I did:** cups open one at a time (0, 12, 24, 36, 48 s) and take one ball each;
shots carry a seeded error (8° and 12% at full power, scaled by power) and the aim draws
the landing area; a clubbed golfer cannot be clubbed again until two seconds after the
stun ends. All are listed as deviations from DESIGN.md in the PR.
