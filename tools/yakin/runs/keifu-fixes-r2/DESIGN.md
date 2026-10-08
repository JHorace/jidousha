# DESIGN — keifu-fixes-r2 (V1, inline)

Built fresh from `tools/yakin/tasks/keifu-fixes.md` with the r2 overrides; the night-one
branch/PR (#126) was not opened. Read: the spec, `games/keifu/spec/SPEC.md` §5-§7, §14.4,
§18, `ui-text.json`, and the keifu source the three fixes touch.

## What the port calls "the tutorial"

The port never built Lineage's paged guide (`ui.guide` is not in `words.rs`). What
teaches the player is: the summer dock's idle help (`summer.help`) and the quest sheet's
dice explanation — "Two dice, less 7, are added to that." and the four outcome lines
(`quest_sheet.dice/triumph/success/setback/disaster/unanswered`) plus the trouble stakes
(`quest_sheet.trouble_stakes`). Those are the lines rewritten.

## 1. Tutorial language

- Rewrite those strings in `spec/content/ui-text.json` (the fence grants it), voice: a
  person teaching a friend. Every number stays a placeholder filled from the constants the
  sim reads (`DICE_MIDPOINT`, `TRIUMPH_MARGIN`, `SETBACK_MARGIN`, the quest's renown,
  danger, `death_percent`), so no stated number can drift from the sim. The dice line
  gains the dice's swing (-5 to +5), computed from `DICE_SIDES`/`DICE_MIDPOINT`.
- The help does not mention NEXT SUMMER: the last summer has no such label and shares the help.
- Assertions quoting the old words are updated: `quest_sheet.rs` unit tests, `w4.rs`
  (the four outcome lines), `w4_rules.rs` (dice line, trouble stakes). Each listed in the PR.

## 2. Foresight — deliberately partial

Next summer's board cannot be shown exactly: it is drawn after this summer resolves, from
the run's one RNG stream, and reads state the resolution changes (trouble, the living,
ghosts). Drawing it early would move every later draw (the spec's forbidden fix). So the
foresight shows **what is already determined or bounded by state**:

- per questing place, its trouble range next summer *as the heroes are seated now*
  (unseated quest -> rises by 1, capped; seated -> 0 on a win, one less on a loss; not on
  the board -> unchanged) — read from the same `risen_trouble`/`eased_trouble` functions
  the resolution now calls;
- the first ghost, which planning always posts (§5.2 step 3), unless its quest is answered
  now (it may be laid);
- the year's demand step when next year crosses one (`(year-1)/6`);
- in the year before the Door: that no quests are posted.

Surface: a "NEXT SUMMER" label in the summer screen's left column, under the yard; pointing
at it opens the reading in the dock (the game's idiom — the Door lines work the same way).
The idle dock stays the help alone (an existing floor asserts exactly that). Not a separate
screen; updates live as heroes are seated. Files: new `src/outlook.rs` (the readings),
`summer.rs` (label + target), `screen.rs`/`pointer.rs`/`dock.rs`/`dock_lines.rs`/`board.rs`
(the new target and dock subject), `resolve.rs` (the two trouble functions), `words.rs`
+ `ui-text.json` (new strings).

## 3. Telegraph

`outlook::telegraph(content, house, slot)`: the place's trouble if left
(`risen_trouble`), and over the place's templates *other than the one the template memory
holds there* (the memory next summer's planning reads), evaluated through `quest::stakes`
at that trouble and next year: danger, seats and renown ranges, and the most it can need
(`base_demand` + wobble). Easing (§5.2) can only lower a demand, so the demand is shown as
"at most N" — a promise the sim keeps. A ghost's quest is not a template quest; it is
foresighted separately (above).

Surfaces: on the card, while nobody is seated (the state in which it would be left), one
line "Left: danger 3-4, room 1-2"; on the quest sheet, a full line under UNANSWERED with
trouble, danger, room, need and pay.

## Checks (`src/r2_checks.rs`, wired into `verify.rs`)

- Foresight: a seating summer, point at NEXT SUMMER, assert the per-place lines, ghost and
  step lines in the dock transcript; stay home, play through to next summer, assert every
  posted quest's trouble is inside the shown range for its place (all recorded seeds), and
  a staged seated run (Garrick+Brannoc on Grave goods) whose Barrow range is 0-0/0 honoured.
- Telegraph: at seating, the card line and the sheet line in the transcript; stay home,
  next summer each template quest at a place that was on the board lies inside its shown
  danger/seats/renown ranges and its demand + eased <= shown max.
- Mutation round on the new checks: `games/keifu/mutants/r2.txt` via `tools/mutate`.
- Captures: two new PNGs (`r2-outlook.png`, `r2-telegraph.png`) and recapture of changed
  screens.

## Done-when mapping

Each line of keifu-fixes.md's Done-when is checked by: the branch diff (`git diff --stat`),
`tools/verify keifu` report, the two decision rows' checks above, build-web + serve-web
--check, and the PR body.
