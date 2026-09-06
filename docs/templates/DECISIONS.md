# DECISIONS — the decision-surface table a game handoff carries

Copy the table into the handoff, one row per decision the wave **adds or
changes**, and delete this guidance. A game handoff that adds no decision says
so in the one explicit line §3 gives. A handoff with neither the table nor that
line is malformed: the session stops and asks the owner before implementing
(`make-game` §D.1).

**Why this exists.** Wave 1.1 of ninjo specified its systems and its data
thoroughly and the surface on which the player makes the wave's decision not at
all. The build was faithful to the handoff, so it was faithful to the silence,
and the decision was unmakeable in the deployed build — the owner's playtest
filed `games/ninjo/FINDINGS.md` G-017 and G-018, both classed "the game's own
(a wave-1.1 gap)", both a surface the spec never named. The omission was the
design session's, not the build's, which is why the fix is a section of the
handoff rather than advice to the implementer. (Owner policy, 2026-09-02.)

---

## 1. The table

**Decisions this wave adds or changes:**

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| … | … | … | … | … | … |

## 2. The columns

- **decision** — what the *player* chooses, in their words. "Which job to send
  the selected character to", not "dispatch".
- **must know** — the facts the choice turns on. If the player cannot tell two
  options apart without a fact, the fact is in this cell.
- **surface** — where each of those facts is shown **at the moment of
  choosing**. A fact on another screen, or one tap away, is not on this
  surface; say which surface it is on and whether that is good enough.
- **action** — the input that commits the choice, exactly as a player performs
  it, including whatever must already be true (a selection, an idle character).
- **one function** — the sim function that both the display and the outcome
  read, so a preview and the simulation cannot disagree (ninjo's GDD §1, "one
  decision function per question"). Two names here is fine when the decision
  turns on two questions; two *different* functions for the same question is
  the bug this column exists to prevent.
- **asserted by** — the readability floor or the scripted test that proves
  those facts are on screen at the tick the action is available. A surface
  nothing asserts is a surface the next wave can quietly move off screen.

## 3. When the wave adds no decision

One line, in the handoff, in these words:

```
Decisions: none new; existing surfaces unchanged.
```

An empty table is not the same statement — it reads as a section nobody filled
in. Say it in the line.

---

## 4. A worked row

This one is real: it is the decision ninjo's site board makes, written the way
the wave that builds it carries it. Today the map draws a marker, a label and
an open-quest count per site (`games/ninjo/UI.md` §3) — the task type, the pot
and the duration of a job are nowhere on screen, and the dispatch that commits
the choice is a click on the marker. That gap is exactly what a filled row
turns into work.

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Which job to send the selected character to | the job's task type, its pot and its duration; how well *this* character fits that task; how far the site is from where they stand | the site panel's job row — one row per open job, carrying the task icon, the pot with its coin, the duration, this character's fit and the travel time, all readable while the panel is open | tap the open row with an idle character selected (DESIGN §5's two clicks, the second refined from the marker to the row) | `traits::competence_at` for the fit and `path::route` for the travel — the row and `sim::dispatch` call the same two, so the row cannot promise what the order then refuses | `floors::controls_for` for the row's 32x32 target and the overlap floor (UI.md §4), and a scripted `verify` check that opens the panel and asserts every fact above is in the `Panel` on the tick the row is tappable |

Six columns, one row per decision. That is the whole instrument; a heavier one
does not get filled in.
