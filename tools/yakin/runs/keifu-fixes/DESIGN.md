# DESIGN — keifu-fixes (V1, inline)

Read: SPEC §5.2/§5.4/§7.2, ui-text.json, generation.rs, quest.rs, quest_card.rs,
quest_sheet.rs, board_view.rs, dock_lines.rs, resolve.rs.

Fact driving every call below: next summer's board is drawn from the run's one RNG
stream at the start of next summer (generation.rs `plan`), and depends on who is alive
then. Dealing it early would move every later draw and change every existing oracle
(the spec's fence forbids that). So foresight and telegraph are **derived from state that
already exists** — place trouble, the remembered template per place, the ghost list, the
current seating — never from a drawn board.

## One new module: `outlook.rs` (the shared function)
- `trouble_if_unanswered(t)` = `min(t+1, TROUBLE_LIMIT)`; `resolve::unanswered` now calls it.
- `pool(content, place, remembered)` = the place's templates minus the remembered one
  (the same set `generation::plan` draws uniformly from).
- `next_quest_range(content, place, remembered, trouble_lo..=hi, next_year)` → ranges of
  seats / danger / renown / demand over the pool, calm seats and wobble, computed by
  calling `quest::stakes` (the function `post` calls) with `DEMAND_WOBBLE`. A ghost's
  quest is the point `ghost::ghost_quest` returns (no wobble).

## Fix 2 — Telegraph (on the card, and in full on the sheet)
- Card: one new line under "Renown +R / unanswered -C": `Left alone, next time: danger 3-4,
  needs 7-14`. Sheet: an `IF LEFT ALONE` block: the place's trouble rising x → y, room,
  danger, renown, needs ranges, and that it is only posted if the place is drawn.
- Not shown in the last summer (the Door) nor when next summer is the Door's.
- Function: `outlook::next_quest_range` at `trouble_if_unanswered(q.trouble)`; the unanswered
  cost is `Quest::unanswered_cost` as today.
- Checked by: unit tests of the ranges against `post` over many seeds (always inside),
  and a `--verify` scene: seat, read the card text in the frame transcript, leave the quest
  unanswered, end the summer, deal next summer, assert the dealt quest at that place (if any)
  lies in the range shown.

## Fix 1 — Foresight (dock, when nothing is pointed at; not a separate screen)
- Deliberately **partial**: which four of six places post next summer is an RNG draw that
  does not exist yet, so it is not shown. Shown: a "NEXT SUMMER" block under the summer help:
  "Four of these six places will post a quest." Then per place: its trouble next summer
  *given the current seating* (empty seats on its quest = left alone = exactly +1; seated =
  eases to 0 on a win or 1 less on a loss; no quest there = unchanged) and what it would
  post: the template titles it can draw (the pool, exactly known: this summer's template
  becomes the remembered one) and the danger range. Plus the first ghost's place, which
  *is* certain unless the ghost is laid.
- It re-reads the seating each frame, so the player sees consequences while dragging.
- Function: `outlook::next_quest_range` and `outlook::pool`, the same as the telegraph; the
  pool is the set `generation::plan` draws from.
- Checked by: a `--verify` scene reaching a seating summer and asserting the block is in the
  dock transcript; and one that plays on to next summer and asserts the quest dealt at each
  place has a template in the previewed pool and stakes inside the previewed ranges when its
  trouble equals a previewed trouble.

## Fix 3 — Tutorial language
- Rewrite in `ui-text.json`: summer.help, quest_sheet.{needs? no, dice, triumph, success,
  setback, disaster, unanswered, trouble_stakes, fewer_seats}, telling.roll and margins,
  top_bar.renown_help, winter.help. Dice explained from `DICE_MIDPOINT`/margins via
  placeholders (range 2-12 less 7 = -5..+5); numbers come from constants as today.
- Tests quoting old wording updated to the new literal; each listed in the PR.

## Order / commits
1. outlook.rs + tests; 2. telegraph (card, sheet); 3. foresight (dock); 4. tutorial text;
5. verify checks, mutants, screens, FINDINGS. Gates per commit: fast gate.
