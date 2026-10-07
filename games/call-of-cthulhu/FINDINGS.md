# Call of Cthulhu — findings

What the documents cost this game, in the shape `docs/internal/e0-findings.md` uses. G-numbers
continue the sequence across games (keifu's open entries from `keifu-fixes` are G-070 to G-072).

**Reading discipline.** Read: `CLAUDE.md`, the `make-game` skill, all five `docs/api/` documents,
`crates/jidousha/examples/prototype_kit/{capture,checks,verify}.rs`, `games/README.md`, the root
`Cargo.toml` (for the four `.workspace = true` keys) and `tools/yakin/`. **Engine source
(`crates/*/src/`), `docs/internal/`, ADRs and every other game: not opened.**

### G-073 — `judge_panel`'s `controls` argument is never defined

Class: docs · Session: call-of-cthulhu · Owner: `docs/api/jidousha-ui.md` (*The floors*, `judge_panel`)

**Doing:** judging each screen's rows against the click targets they sit in.

**Expected:** a sentence saying what the `String` of each `(String, Rect)` control is — the label
row's text, an id, a name for a message — and whether rows inside the rectangle are "labels".

**Happened:** the prose says "a row lying across a control it is not the label of (you pass the
controls, named)" and the only example passes `&[]`. I could not tell whether my option rows, which
sit wholly inside their click rectangles, would be read as labels or as intruders, and could not
learn without staging both. **What I did:** passed `&[]`, so that one floor is not exercised here,
and said so; I did not guess a naming scheme and ship it as if it were checked.

**Fix:** state what the string is, and give one example with a control and the row that labels it.

### G-074 — a game with no pictures still has to invent an empty art type

Class: docs · Session: call-of-cthulhu · Owner: `docs/api/jidousha-ui.md` (*A screen is data first*)

**Doing:** building every screen of a text game on `Panel`, as the make-game skill says a game with
chrome should.

**Expected:** that `Panel` can be used with no icons without declaring anything.

**Happened:** `Panel<I>` wants `I: Icon`, so a game of text alone declares `enum NoArt {}` and
`impl Icon for NoArt { fn size_at(self, _) -> Vec2 { match self {} } }`, and draws with
`|icon, _| match icon.art {}`. The only place that spelling appears is the example under
`judge_panel`; the `Panel` entry shows `enum Art { Coin }`. **What I did:** found it by reading the
`judge_panel` example after the `Panel` one would not compile with `Panel::default()` and no `I`.

**Fix:** one sentence under *A screen is data first*: "a game with no pictures declares an empty enum".

### G-075 — a first mutation round fails on an uncommitted `Cargo.lock`

Class: process · Session: call-of-cthulhu · Owner: the make-game skill §A.6 / `tools/mutate`

**Doing:** the first `tools/mutate call-of-cthulhu` of a new crate.

**Expected:** the skill's rule — "commit every file the round will touch first" — covers it.

**Happened:** every fault ran and was scored, then the round ended `w0 is not clean after the round,
so the score is not trusted: M Cargo.lock`: a new crate's own lock entry is written by the first
build and I had not committed it. The score was withheld and nothing else said so before the end.
**What I did:** committed `Cargo.lock` (the crate's entry only, which the task's fence allows) and
reran. **Fix:** §A.6 could add "and `Cargo.lock`, for a game that is new this session".

### The game's own

- **Ignorance must have a floor, or it has no ceiling.** The first draft let a call lengthen on a
  wrong answer and shorten on a right one; a player who guessed at random had zero drift and a call
  that never ended (Nyarlathotep's cost 85 sanity on average and was capped only by my test's limit).
  The shipped rule gives every answer a step toward the end, a right one two, and a wrong one none.
  The three players (`players.rs`) found it; no reading of the rules would have.
- **A lore game must keep ignorance expensive and the unlucky survivable.** The numbers are the
  sweep's: 60 sanity, calls of 4/6/4 exchanges, drains of 2/1/2. Measured over 100 seeds each: the
  reader of lore wins 71, the player who answers the first thing and never studies wins 17, the one
  who only hangs up wins 0.
- **Insults must not be recognisable by tone.** In the first draft the insulting answer was rude and
  a player who skipped the rude one beat the game without lore (90 of 100). The shipped insults are
  polite and presumptuous ("Some kind of sea monster."), so only lore tells them from a guess.
- **Hang up costs eight drains, un-steadied.** Cheaper, it was the best move in the game (200 of 200
  wins); Composure had flattened every other cost to one and made it free.
- **Studying teaches three facts, not one or two,** because a morning spent on lore has to beat a
  morning spent on rest; with two the player who only rested won more often than the one who read.
- **Mutation round** (`mutants/core.txt`, 46 faults over `rules.rs`, `lore.rs`, `screens.rs` and
  `input.rs`): round one 40 of 46 noticed. The six escapes were loose checks, not equivalents:
  Composure's cap, resting one short of full, the last point of sanity, a study carrying into the next
  morning, Yog's call length, and the drain line forgetting Composure. Each now has a check in
  `verify.rs::check_state` that fails under it: **46 of 46**.
- **Pictures** (`screens/`): `morning.png` — day 1 with all eight options, each stating what it does to
  tonight (Appease the Keepers of the Gate: "Yog-Sothoth will not call tonight. Tonight: Nyarlathotep, then
  Dagon"); `call.png` — Dagon on the line with no lore: every answer marked "?", Hang up's price in
  plain view, the right-hand column saying "Nothing. Every answer is a guess." Both opened and read.
- **The web build** runs it (`tools/build-web call-of-cthulhu`, `tools/serve-web call-of-cthulhu --check`).
