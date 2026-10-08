# DESIGN — keifu-x-inheritance-r2

task: keifu-x-inheritance-r2
variant: V2
model-as-configured: claude-fable-5-1
date: 2026-10-07 18:19 PDT

This is a **delta design** over mainline Keifu (`games/keifu/`, read-only). Every rule
below is stated as "mainline does X; the variant does Y". Where a rule is not named,
the variant keeps mainline exactly as copied. Section numbers (`§n`) are mainline's
`games/keifu/spec/SPEC.md`; `CONSTANTS §n` its `spec/CONSTANTS.md`. The implementer
reads mainline whole (its spec, its source) — the fork is read-only but in-fence to read.

Comparison hygiene (the r2 spec): this design was written from the spec and mainline
alone; night one's branch, PR #129 and any `games/keifu-x-inheritance/` were not opened.
The implementer must not open them either.

## What the game is

Keifu X Inheritance is Keifu — a house of heroes kept for 25 years until the Sealed Door —
with the family line made to matter: the house's renown is the *family's*, earned only
through family members, and the name they carry can be marked. A family member may swear a
dream's quest in the family's name: kept, it honours the name; failed, it puts a black
mark on it that drains renown every year and follows the name's heir, halving at each
passing, until someone takes it into the ground. Wanderers are outsiders — cheap hands who
earn only personal renown — until they prove themselves and wed into the family, and
children are bred from their parents' traits rather than averaged.

## The player-facing loop

Keifu's loop, with the variant's moments marked **(new)**.

1. **Summer.** The player drags heroes onto the four quest cards as in mainline (§5.3). A
   card that calls a seated family member's own dream shows an oath button **(new)**;
   pressing it swears that hero's quest in the family's name and the card turns over its
   stakes: who swore, what they must do, the chance it fails in 100, what keeping it earns
   and what failing it costs — a mark of the quest's danger on the name. The quest sheet in
   the dock spells the mark out: the renown lost now, the yearly drain while it stands, the
   half-weight it passes to an heir, and that it is buried with the last to carry it.
   Pressing the button again withdraws the oath; lifting the swearer off the card
   withdraws it too. "Set out" commits.
2. **The telling.** A sworn quest's page tells the oath kept or failed after the reward
   **(new)**. A party of outsiders alone wins personal renown but the house gains nothing
   **(new)** — the page says so.
3. **Winter.** The hearth as mainline (§11). In the garden, an outsider seated beside a
   family member previews "unproven, renown R of T" until their personal renown reaches
   the threshold, then "will wed, and take the name X" **(new)**; two outsiders preview
   "neither is family" and cannot wed **(new)**. The garden's help in the dock quotes the
   threshold. At the long table an outsider's telling earns them renown but the house none
   **(new)**. "Let the winter pass" commits.
4. **The turning.** The winter page tells a wedding-in: the outsider joins the family,
   takes their spouse's house name, and brings half their renown to the house **(new)**.
   The "Year N begins" page lists the name's marks and their drain beside the tales
   **(new)**. A newborn's aptitudes come from one parent or the other, per aptitude
   **(new)**. On a death page that leaves an heirloom or a dream, the player points at an
   heir button and the dock shows that heir's sheet ending in WOULD INHERIT — the heirloom,
   the dream, the dead's blessings, and each mark at the weight it would arrive at or
   "struck" **(new)**; choosing the heir gives exactly that; choosing "No one" buries the
   marks with the heirloom. An outsider's death page offers no heir **(new)**: what they
   held is buried, their undone dream walks as a ghost.
5. **The last summer, the Door, the Ending**: mainline, unchanged. Oaths cannot be sworn
   on the Door.

## Systems

All paths are under `games/keifu-x-inheritance-r2/` (the Fence). Build order; the first
item is the spec's own first act. The engine surface named per system is what the mainline
file already touches, by `docs/api/` item; the variant adds no engine use of its own.

- **Copy and rename** — `cp -r games/keifu games/keifu-x-inheritance-r2`; package and binary
  `keifu_x_inheritance_r2`; window title "Keifu X Inheritance"; the `[keifu]` message
  prefix, `cargo run -p keifu` doc lines and `tools/verify keifu` mentions renamed; every
  `capture:` file name prefixed `keifu-x-inheritance-r2-` instead of `keifu-` (the two
  games write into one `target/verify/`); `src/main.rs`'s module doc gains one paragraph
  naming the variant and this DESIGN. Nothing else changes in that commit, and
  `python3 tools/verify keifu_x_inheritance_r2` is green on it · `Cargo.toml`, `src/main.rs`,
  `src/verify.rs`, `src/capture.rs` · touches `jidousha-api.md`: `GameConfig` (`title`,
  `seed`, `window_size`), `run`, `App::add_system`.
- **family** — who is family: `Hero.family: bool` (founders true; `newcomers::newcomer`
  false; `wanderer::arrive` leaves false; `births::born` sets true); `is_family`,
  `marry_in(hero) -> MarryIn { renown, threshold, eligible }` (eligible iff family or
  `renown >= MARRY_IN_RENOWN`), and `join(heroes, outsider, spouse) -> dowry` (family =
  true, house name = the spouse's, returns `renown / DOWRY_SHARE`) · `src/family.rs` (new)
  · touches `jidousha-api.md`: `Resource` (`House`), nothing else.
- **courtship** — mainline `plans::courtship` checks NOBODY, WAITING, KIN, TOO_YOUNG,
  TOO_FAR_APART, RIVALS, WED_ALREADY, WILL_WED; the variant inserts after WED_ALREADY:
  `NeitherFamily` (both outsiders), `Unproven { who, renown, threshold }` (the first
  outsider in seat order below the threshold), then `WillWedIn { outsider }` when exactly
  one is an outsider at or above it, else WILL_WED. `winter::court` carries out
  `WillWedIn` as a wedding (§11.3 step 4) followed by `family::join` and the dowry; the
  hearth's garden note and the garden help quote the same `MarryIn` · `src/plans.rs`,
  `src/winter.rs`, `src/hearth_view.rs`, `src/hearth_help.rs`, `src/words.rs`,
  `spec/content/ui-text.json`, `spec/content/lines.json` · touches nothing new.
- **family renown** — mainline `reward::reward` credits the house `renown + carriers` on
  every win; the variant credits it only when `members` holds a family member, else the
  line `lines.quest.reward_outsiders` and personal renown only. Mainline `plans::tellers`
  makes the first adult teller `House`; the variant makes the first *family* adult `House`
  and every outsider `Own`; `winter.rs`'s table writes `lines.winter.tale_told_outsider`
  for an outsider in the first seat · `src/reward.rs`, `src/plans.rs`, `src/winter.rs`.
- **heirs** — mainline `heirs::heirs` lists every living hero but the dead; the variant
  lists living *family* only, so an outsider is never offered and an outsider dead has an
  empty list (their heirloom is buried, their undone dream raised as a ghost, the page
  decided on making — `death_page::death_page` sets `leaves = false` for an outsider).
  `nearest_kin` (crowning) reads the same list · `src/heirs.rs`, `src/death_page.rs`.
- **marks** — `Mark { title: String, place: Place, year: i32, by: HeroId, weight: i32 }`;
  `Hero.marks: Vec<Mark>`; `house_marks(house) -> Vec<(HeroId, &Mark)>` = marks carried by
  living family in creation order; the turning's step 8 (beside the tales) drains
  `house renown -= house_marks().len() * MARK_DRAIN` with `lines.marks.drain_one` /
  `drain_many`; the death page lists the dead's marks after the heirloom and dream lines
  (`lines.death.leaves_marks`); `heirs::choose` passes them to the heir at
  `weight / MARK_HALVING` (a mark reaching 0 is struck, `lines.heir.mark_struck`; the rest
  `lines.heir.takes_marks`) or buries them with "No one" (`lines.heir.marks_buried`); the
  sheet gets a MARKS section · `src/marks.rs` (new), `src/turning.rs`,
  `src/death_page.rs`, `src/heirs.rs`, `src/sheet.rs`.
- **oath** — `Posted.sworn: Option<HeroId>` (one value: who swore, or no one);
  `oath::may_swear(content, heroes, quest, party, hero) -> Option<Need>`: the quest is a
  `Source::Template` quest, the hero is family, adult, in `party`, and
  `calls::dream_call` gives a call from their *own* dream (not `burden`, not
  `Telling::StayBehind`); `Need` is the outcome the call needs — `Telling::Triumph` →
  TRIUMPH, `Succeed` → SUCCESS, `Go` → SETBACK; `oath::swearer(…) -> Option<HeroId>`: the
  first seated hero in seat order who may swear; `oath::consequence(quest) ->
  Consequence { weight: quest.danger }` — the one function the card, the sheet and the
  resolution read; `oath::fail_ways(forecast, need) -> i32` ways of 36 below the need,
  shown through `forecast::percent`; `Target::Swear(slot)` toggles `sworn` (set to
  `swearer`, or cleared); `House::reseat` clears every `sworn`; after every drop
  (`board::drop_hero`, where refusers are sent back) an oath whose swearer no longer sits on
  that card is withdrawn; `resolve::resolve_party` judges the oath as a new step **5b**,
  after the reward and before the trouble eases: kept (outcome ≥ need) → house
  `+weight`, swearer personal `+weight`, `lines.oath.kept`; failed → house `-weight`,
  swearer personal `-weight` (floored at 0), a `Mark { title: quest.title, place,
  year, by: swearer, weight }` pushed on the swearer, `lines.oath.failed`. The oath is
  judged on the dead too (a swearer killed at step 10 still bears the mark into their
  death page). The card (`quest_card::read_card`) gains `oath: Option<OathReading>`
  (the sworn line, the stakes line, and the button's label) and the sheet
  (`quest_sheet::quest_sheet`) a SWORN BY block · `src/oath.rs` (new), `src/board.rs`,
  `src/house.rs`, `src/screen.rs`, `src/board_view.rs`, `src/pointer.rs`,
  `src/quest_card.rs`, `src/quest_sheet.rs`, `src/resolve.rs` · touches
  `jidousha-api.md`: `Rect`, `Vec2` (the button's rect inside the card), `Input` /
  `PointerButton` (the press, as `pointer.rs` already reads them).
- **inheritance** — `inheritance::conceive(first, second, rng) -> [i32; 3]`: mainline's
  newborn base is `max((F + S) / 4 + U{0,1}, 1)` per aptitude; the variant draws, per
  aptitude in order, one coin (`chance::index(rng, 2)`: 0 = the first-created parent is
  dominant, 1 = the second) then `U{0,1}` (`chance::between(rng, 0, 1)`), and sets
  `max(dominant_base / DOMINANT_SHARE + U, 1)`; then, if both parents' `best_aptitude()`
  are the same aptitude, that aptitude `+BRED_TRUE_BONUS` (cap 9) with
  `lines.birth.bred_true` on the birth page. The CHILD_WILL_SURPASS_YOU +2, the fear
  rolls and the blessings stay mainline (§17.3), in mainline's roll order with the two
  draws above in place of mainline's one. Marks are not inherited at birth.
  `inheritance::succession(content, house, dead, heir) -> Succession { heirloom:
  Option<String>, dream: Option<String>, blessings: Vec<String>, marks: Vec<(Mark,
  Passing)> }` with `Passing::Arrives(weight)` or `Passing::Struck`: what `heir` gets
  if chosen — the heirloom name if the dead holds one, the undone dream's told title if
  `heirs::can_take_dream(heir)`, every blessing of the dead whose title the heir lacks,
  and every mark at `weight / MARK_HALVING`. `heirs::choose` applies exactly `succession`
  (heirloom and dream as mainline, blessings added with `lines.heir.takes_blessings`,
  marks as the marks system says). The dock, with the pointer on an heir button, shows
  that heir's sheet followed by a WOULD INHERIT section built from `succession` · 
  `src/inheritance.rs` (new), `src/births.rs`, `src/heirs.rs`, `src/dock_lines.rs`,
  `src/screen.rs` (what the pointer is on), `src/turning_view.rs` · touches
  `jidousha-api.md`: `Rng` (the draws, through `chance.rs`).
- **sheet** — mainline's condition line "Renown R"; the variant, for a living outsider,
  `hero_sheet.renown_outsider` ("Renown 2, an outsider: 4 to wed into the house"), the
  wounded and settled forms keeping mainline; a MARKS section (one line per mark,
  `hero_sheet.mark`) after SCAR · `src/sheet.rs`, `src/dock_lines.rs`.
- **checks** — `src/xi.rs` (and `src/xi_stages.rs` if the file would pass 500 lines):
  the three decision-row checks, the staged renown and drain checks, and the three
  pictures; registered in `verify::run`'s summary; three stages added to `floors::check`
  and three rows to `capture::capture_all`; `mutants/xi.txt` · touches
  `jidousha-testing.md`: `HeadlessSim`, `headless`, `InputScript`, `SnapshotBuilder`,
  `InputEvent` (the scripted press — through `scripted.rs` and `verify::point_at`),
  `FrameRecorder`, `FrameRecord` (the floors on the three new surfaces);
  `jidousha-capture.md`: `WgpuBackend::offscreen`, `encode_png` (through `capture.rs`).

- Assets: none. The variant reuses every sprite the copy carries (`assets/hero_*.png`,
  `assets/heirloom_*.png`); the oath button is a text button drawn as "Set out" is
  (`summer::button`); marks are lines of type. No third-party art.

`docs/api/jidousha-ui.md` is not opened: mainline draws its own screen (`screen.rs`,
`dock.rs`) and a fork does not adopt the kit mid-stream. `jidousha-controllers.md`: no
new player; the batteries' players in `play.rs` are the controllers and must stay green.

## Gates to add

Every expectation is a shipped literal copied by hand from the content or this design,
never computed from the code under test (mainline's INVARIANT, kept). `recorded()` is
mainline's `w5::recorded` seed set; staging reuses mainline's helpers by name.

- **xi::check_oath** (decision row 1) — input: on every recorded seed, year 1,
  `w4::seat_the_oracle` (Garrick and Brannoc dragged onto "Grave goods" by the scripted
  pointer), then `house.board[0].quest.demand = 10` staged as mainline's W4 tests stage
  it, then `Target::Swear(0)` pressed · asserts: before the press the card shows the
  button "Swear it" and no sworn line; after it the card's lines contain "Sworn by
  Garrick: he must triumph. Fails 72 in 100." (power 12 against 10: triumph 10 of 36,
  CONSTANTS §3) and "Kept: +2 renown, his and the house's. Failed: a mark of 2 on the
  name.", the quest sheet in the dock (pointer on the card) contains "SWORN BY GARRICK"
  and the failed-oath sentence naming -1 a year, half to the heir, buried with the last;
  `house.board[0].sworn == Some(garrick)`; pressing again clears it and the card reads
  "Swear it" again; re-sworn, then the quest resolved by `resolve::resolve_rolled` with
  dice `[1, 1]` (margin -3, SETBACK: below the triumph needed) → `marks::house_marks`
  is exactly `[(garrick, Mark { title: "Grave goods", place: Barrow, year: 1, by:
  garrick, weight: 2 })]`, house renown 13, Garrick's renown 4, the page's lines contain
  the `lines.oath.failed` literal; on a fresh session the same staging with dice `[6, 6]`
  (margin 7, TRIUMPH) → no mark, house renown 19 (15 + 2 reward + 2 oath), Garrick 10
  (6 + 2 + 2), the `lines.oath.kept` literal · covers Done-when: "a check for each
  decision row" (row 1: the mark consequence in the transcript before the commit, the
  failed quest on a fixed seed, the house carrying exactly that mark).
- **xi::check_oath_eligibility** — input: staged boards · asserts: the Door's lock and a
  ghost's quest show no button; an outsider (a staged wanderer) seated on a quest that
  calls them shows none; a family member called only by their burden shows none; a family
  member called "stay behind" shows none; lifting the swearer off the card (a scripted
  drag to the roster) clears `sworn` and the card reads "Swear it" with no one else
  eligible · covers: row 1's "one function" (the button and the resolution read
  `may_swear`/`consequence`).
- **xi::check_marry_in** (decision row 2) — input: on every recorded seed, year 1,
  `w7::stay_home_into_winter`, `wanderer::arrive` on the session's generator (as
  `w8::stir` does), then staged: the wanderer's age 24, renown 2; the wanderer put in
  `Seat::Garden(0)` and Ysolde in `Seat::Garden(1)` (`w7::seat_at`) · asserts: the
  garden's note (`w7::group_lines(Group::Garden)`) contains "unproven, renown 2 of 4";
  the garden's help in the dock (pointer on the group) contains "An outsider weds in only
  at renown 4"; the wanderer's sheet reads "Renown 2, an outsider: 4 to wed into the
  house"; "Let the winter pass" → the winter page contains "<name> is unproven: renown 2
  of 4. The house will not have <him/her> yet.", no SPOUSE bond, `family == false`, house
  renown 11; a second session staged the same with renown 4 → the note reads "will wed,
  and take the name Vane", after the winter the page contains the WEDDINGS line and
  "<name> weds into the house and takes the name Vane: renown +2 to the house.", the
  SPOUSE bond stands both ways, `family == true`, `house == "Vane"`, house renown 13; and
  with Garrick staged to 93 in the same session, his death page's heir list contains
  "<name>, of the house" where the unproven session's list does not · covers Done-when
  row 2 (below-threshold refused, at-threshold accepted, the threshold and the renown in
  the transcript — `verify` prints the note and the page lines as its vector).
- **xi::check_neither_family** — input: two staged outsiders in the garden · asserts:
  the note "neither is family"; after the winter no wedding and the
  `lines.winter.neither_family` line · covers row 2's eligibility function.
- **xi::check_succession** (decision row 3) — input: on every recorded seed,
  `w8::stage_garricks_winter`, then Garrick given `marks = [Mark { title: "Grave goods",
  place: Barrow, year: 1, by: garrick, weight: 4 }, Mark { title: "The bell under the
  tide", place: DrownedCoast, year: 1, by: garrick, weight: 1 }]` and `blessings =
  [Blessing { title: "Garrick's rest", scope: AgainstTag(Undead), power: 2 }]`; "Let the
  winter pass", `w8::go_to_the_choice`; the pointer on "Maren, daughter" (not pressed) ·
  asserts: the dock's lines (`scripted::lines_in(page, summer::SHEET)`, paged with
  `verify::dock_pages`) end with "WOULD INHERIT", "Thornfall (+1 Might)", "The dream: <the
  Barrow dream's told title>", "Garrick's rest (+2 against Undead)", "A mark of 2: Grave
  goods, year 1", "A mark struck: The bell under the tide, year 1"; the death page's
  lines contain "He leaves marks on the name: Grave goods (4) and The bell under the tide
  (1)."; `inheritance::succession(…, garrick, maren)` returned before the press equals
  a `Succession` read back from Maren after Maren is chosen: `heirloom == Thornfall`,
  the Barrow dream now Maren's burden (her own dream stands), "Garrick's rest" in her
  blessings, `marks == [Mark { …, weight: 2 }]` and no bell mark; `marks::house_marks`
  is exactly that one mark; the page's lines contain "Maren takes up the marks: Grave
  goods (2).", "The mark of The bell under the tide is struck from the name." and "Maren
  takes up Garrick's rest."; a second session choosing "No one" → Maren unchanged,
  `house_marks` empty, the page contains "The marks go into the ground with him: the
  name is lighter by 2." · covers Done-when row 3 (the shown traits and marks equal the
  heir's actual state after choosing).
- **xi::check_outsider_death** — input: a staged wanderer holding an heirloom and an
  undone dream, aged to a certain death · asserts: the death page is decided on making
  (no heir buttons, "Go on" live), the heirloom buried (`lines.heir.buried_with`), a
  ghost raised, `house_marks` unchanged · covers: the heirs system.
- **xi::check_family_renown** — input: a staged board, slot 0 seated with one staged
  outsider alone, resolved with dice `[6, 6]` · asserts: the outsider's personal renown
  rose by the quest's renown + 1, house renown did not move, the page contains the
  `reward_outsiders` literal; the same with Garrick beside them → house renown rose ·
  covers: the family renown system.
- **xi::check_drain** — input: a staged house: two marks on one living family member,
  one on a dead one, the winter let pass · asserts: the "Year 2 begins" page contains
  "The name carries 2 marks: -2 renown." and house renown fell by exactly 2 beyond what
  tales and deaths did (stage no tales, no carriers) · covers: the marks system.
- **Unit tests** (`cargo test -p keifu_x_inheritance_r2`), each named as a sentence:
  `a_newborn_takes_each_aptitude_from_one_parent_or_the_other` (parents `[7, 2, 2]` and
  `[3, 6, 2]`: every child base is in the set a dominant draw and a coin allow, and seed
  1 gives one fixed literal child); `a_child_of_two_parents_who_share_a_best_aptitude_is_bred_true`
  (both best Might → Might +1 over the drawn value); `a_mark_halves_at_each_passing_and_is_struck_at_nothing`
  (4 → 2 → 1 → struck); `an_outsider_below_the_threshold_is_unproven_and_at_it_weds_in`
  (`plans::courtship` on staged pairs); `two_outsiders_are_refused_the_garden`;
  `the_first_family_teller_tells_for_the_house` (`plans::tellers` with an outsider first);
  `outsiders_are_never_heirs` (`heirs::heirs` with a staged wanderer).
- **Mainline gates kept green**: `python3 tools/verify keifu` and `cargo test -p keifu`
  unchanged (nothing under `games/keifu/` is written); `tools/verify keifu_x_inheritance_r2`
  green on the copy commit and on the final one; `tools/build-web keifu_x_inheritance_r2`
  and `tools/serve-web keifu_x_inheritance_r2 --check`; `cargo fmt --all`, clippy
  `-D warnings`, `tools/check-game-deps`, `tools/check-assets`, `tools/test` · covers the
  rest of `## Done when`.
- **Floors** — the three new surfaces (the sworn card with the sheet in the dock; the
  garden with an unproven note; the death page with the WOULD INHERIT dock) staged into
  `floors::check` and judged by the same floors as every other screen.
- **Pictures** — three rows in `capture::capture_all`: `keifu-x-inheritance-r2-xi-sworn.png`,
  `…-xi-garden.png`, `…-xi-succession.png`; opened and named in the PR (make-game §A.7).
- **Mutation round** — `mutants/xi.txt`, at least 15 faults: every new constant
  (`MARRY_IN_RENOWN`, `DOWRY_SHARE`, `MARK_DRAIN`, `MARK_HALVING`, `DOMINANT_SHARE`,
  `BRED_TRUE_BONUS`), the need table (Go → SETBACK), the family-only heir filter, the
  outsider reward gate, the table's first-family rule, the mark weight (`danger` → `danger
  + 1`), the oath's sign (`-weight` → `+weight`), the struck-at-zero rule, the coin's
  parent order, `sworn` not cleared on reseat, the dock preview reading a different
  function than `choose`. Run `tools/mutate keifu_x_inheritance_r2 mutants/xi.txt --fast`,
  then mainline's lists filtered: `tools/mutate keifu_x_inheritance_r2 mutants/*.txt --fast
  --changed-since <the copy commit>`. Report `N of N noticed` in the PR; an escape is a
  check to tighten, never a fault to drop.

## Non-goals

Cut so the implement stage fits the window, or left out on purpose; each one that is
cut goes nowhere else — mainline behaviour stands in its place.

- No marks on the top bar, the family screen's tally or the family tree; no outsider tint
  or sprite. The name's marks are read on the "Year N begins" page, on sheets and on
  death pages.
- No new deeds and no epitaph parts for oaths, marks or weddings-in; epitaphs read the
  mainline record unchanged.
- No genetics beyond aptitudes: vocation, destiny, dream and fear keep mainline's rules
  (§17.3, §17.5). Marks are never inherited at birth.
- No renown transfer at a family death; personal renown dies with its bearer as in
  mainline. An outsider's departure (crowning) leaves what mainline's crowning leaves,
  less an heir: the heirloom goes to Court with them.
- Oaths only on template quests: not the Door's locks, not a ghost's quest; only from the
  swearer's own dream, never a burden, never a "stay behind" call.
- No mark redemption beyond burial: no quest, winter or year lightens a mark; it passes
  at half or goes into the ground.
- Two outsiders cannot wed under the roof, so no outsider is ever born; "family" has one
  entry point for the born and one for the wed.
- No change to board generation, dream calls, the Door, the Ending or the guide.
- No engine change is needed: every surface the design rests on is one mainline already
  uses. No FINDINGS entry is owed by the design; the build files its own (make-game §C,
  into the variant's `FINDINGS.md`, G-numbers continuing from mainline's G-069).
- Nobody plays the web build; the PR says so (DOCTRINE §6).

## Decisions already made

1. **Family is a flag, not a house name.** All nine founders are family (four house names
   among them, as authored); a wanderer is an outsider; every child born is family (a
   family parent is guaranteed, see 4); an outsider who weds in becomes family. Reason:
   the founding household is the family the player is given, and a name-equality rule
   would make Odo and Ysolde outsiders in year 1.
2. **The threshold is `MARRY_IN_RENOWN = 4`** personal renown, read by `family::marry_in`.
   Reason: wanderers arrive at 0..2 (§17.2); one won quest or two winters at the table
   proves them — reachable in a year or two, never free.
3. **Marrying in**: the outsider takes the spouse's house name, becomes family, and the
   house gains `renown / DOWRY_SHARE` (`DOWRY_SHARE = 2`, integer division) at the
   wedding; from then they are offered as an heir and their children are family. Reason:
   the name carries forward; the dowry makes the proven outsider worth more than a cheap
   body. The courtship verdict stays one function (`plans::courtship`) read by the
   garden's note and the winter's resolution, so the preview cannot disagree.
4. **Two outsiders may not wed** (`NeitherFamily`). Reason: keeps "born under the roof"
   equal to "family" with no third status.
5. **House renown is family renown**: a won quest credits the house only with a family
   member in the party; the table credits it only through the first family teller. Costs
   (unanswered, disaster, a carrier's death) still fall on the house. Reason: the roof
   answers for its quests; only the family earns for the name.
6. **Outsiders are never heirs** and an outsider's death page gathers none. Reason: an
   outsider leaves nothing to the line; what they held is buried or walks.
7. **A personal quest is a called dream's quest, sworn.** Eligibility is `may_swear`;
   the need is the call's telling (triumph / succeed / go → TRIUMPH / SUCCESS / SETBACK);
   failing is an outcome below the need. Reason: mainline's nearest analogue is the dream
   (§9), and the call already says what the quest must do for it — no new content.
8. **The mark's weight is the quest's danger**, and `oath::consequence` is the one
   function: kept, `+weight` to the house and the swearer; failed, `-weight` to each and
   a mark of `weight` on the swearer. Reason: danger is already on the card as pips, so
   the stake reads off what the player sees.
9. **Marks drain and halve, never fade**: `MARK_DRAIN = 1` house renown per mark on the
   name per turning (beside the tales, §14.3's mirror); at a death page the marks go with
   the heirloom and the dream to the chosen heir at `weight / MARK_HALVING`
   (`MARK_HALVING = 2`; 0 is struck) or into the ground with "No one". A page waits only
   for an heirloom or a dream, as mainline; marks alone are buried without a choice.
   Reason: the choice has teeth — Thornfall comes with Garrick's shame — and a mark of 1
   dies with its bearer while a mark of 4 outlives two heirs.
10. **The oath is one value on the card** (`Posted.sworn`), swearer-chosen by seat order,
    withdrawn when the swearer leaves the card or the roster is reseated; judged at step
    5b of §7.1, after the reward, before the trouble eases, on the living and the dead
    alike. Reason: the kit's "one value, never a flag beside it"; the step placement
    keeps the reward's renown and the oath's separately told.
11. **Genetics**: per aptitude a dominant parent by a coin, then `max(dominant /
    DOMINANT_SHARE + U{0,1}, 1)` (`DOMINANT_SHARE = 2`), and `BRED_TRUE_BONUS = 1` to the
    shared best aptitude; the fear and blessing rules stay mainline's. Reason: two draws
    per aptitude, seeded, so a run replays; distinct from mainline's average and
    assertable at a fixed seed.
12. **Succession is one function** (`inheritance::succession`) read by the dock's WOULD
    INHERIT section and by `heirs::choose`; it also carries the dead's blessings to the
    heir. Reason: row 3's one function; a blessing on the line should follow the name.
13. **Decision rows elaborated, none added or dropped.** Row 1: surface = the quest card
    and the quest sheet in the dock; action = `Target::Swear(slot)`, committed by "Set
    out"; one function = `oath::consequence` (with `oath::fail_ways`); asserted by
    `xi::check_oath`. Row 2: surface = the garden's note on the hearth screen and the
    garden's help in the dock; action = "Let the winter pass" with the pair seated (the
    garden's commit input, §11.3); one function = `plans::courtship` over
    `family::marry_in`; asserted by `xi::check_marry_in`. Row 3: surface = the death
    page's heir buttons with the pointed-at heir's sheet and WOULD INHERIT in the dock;
    action = the heir button (`Target::Heir`); one function =
    `inheritance::succession`; asserted by `xi::check_succession`.
14. **Content lives in the variant's own `spec/content/`** copies under new keys, read
    through `words.rs` as every string is (no literal in source). New keys, by file:
    `ui-text.json` — `quest_card.swear` "Swear it", `quest_card.withdraw` "Withdraw the
    oath", `quest_card.sworn` "Sworn by %: % must %. Fails % in 100.", `quest_card.oath_stakes`
    "Kept: +% renown, % and the house's. Failed: a mark of % on the name.",
    `quest_sheet.sworn_by` "SWORN BY %", `quest_sheet.oath_need` "% must %, or the name is
    marked.", `quest_sheet.oath_fails` "Fails % in 100.", `quest_sheet.oath_kept` "Kept:
    +% renown to % and +% to the house.", `quest_sheet.oath_failed` "Failed: -% renown to
    % and -% to the house, and a mark of % on the name: -1 renown a year while it stands,
    passed to % heir at half its weight, buried with the last to carry it.",
    `hero_sheet.renown_outsider` "Renown %, an outsider: % to wed into the house",
    `hero_sheet.marks` "MARKS", `hero_sheet.mark` "%, %, year %, by % (%)",
    `hero_sheet.would_inherit` "WOULD INHERIT", `hero_sheet.inherit_nothing` "Nothing but
    the name.", `hero_sheet.inherit_heirloom` "% (%)", `hero_sheet.inherit_dream` "The
    dream: %", `hero_sheet.inherit_blessing` "% (%)", `hero_sheet.inherit_mark` "A mark of
    %: %, year %", `hero_sheet.inherit_mark_struck` "A mark struck: %, year %",
    `winter.courtship_notes.NEITHER_FAMILY` "neither is family",
    `winter.courtship_notes.UNPROVEN` "unproven, renown % of %",
    `winter.courtship_notes.WILL_WED_IN` "will wed, and take the name %", and
    `winter.garden_help` extended by " An outsider weds in only at renown %, takes the
    family's name, brings half their renown to the house, and may be named an heir from
    then. Two outsiders cannot wed under this roof.". `lines.json` — `oath.kept` "% swore
    it in the family's name and kept the oath: +% renown to % and +% to the house.",
    `oath.failed` "% swore it in the family's name and failed: -% renown to %, -% to the
    house, and a mark of % on the name.", `marks.drain_one` "The name carries a mark: -1
    renown.", `marks.drain_many` "The name carries % marks: -% renown.",
    `death.leaves_marks` "% leaves marks on the name: %." (a name list of "title (weight)"),
    `heir.takes_marks` "% takes up the marks: %.", `heir.mark_struck` "The mark of % is
    struck from the name.", `heir.marks_buried` "The marks go into the ground with %: the
    name is lighter by %.", `heir.takes_blessings` "% takes up %.", `winter.unproven` "% is
    unproven: renown % of %. The house will not have % yet.", `winter.neither_family` "%
    and % are neither of the house. They may not wed under this roof.", `winter.wed_in` "%
    weds into the house and takes the name %: renown +% to the house.",
    `winter.tale_told_outsider` "% told the tale; an outsider's telling adds nothing to the
    name.", `quest.reward_outsiders` "+% renown to each who went. Outsiders bring the
    house nothing.", `birth.bred_true` "% has the line's gift: +1 %.". Pronoun arguments
    come from `lore.pronouns` as mainline fills them. Reason: mainline's one way.
15. **Mainline oracles the variant's rules break are rewritten to the variant's rule**,
    each named in the PR's Deviations: `w8::stirred` (the wanderer is no longer on
    Garrick's heir list — its `want` drops the wanderer's row), `births_tests.rs`'s
    aptitude expectations (the dominant draw replaces the average), any W8/W10 battery
    rule reading newborn aptitudes or heir membership, and any W6 battery line asserting
    house renown on a win whose party was staged as outsiders (none are expected: the
    batteries play founders and their descendants). Everything else stays as copied, and
    `tools/verify keifu` is never touched.
16. **If the window closes before the build is through**, cut in this order, each cut a
    PR deviation: the long-table rule (5, table half), the bred-true bonus (11), the
    name change at marrying in (3, keep the flag and the dowry), the blessing carry at
    succession (12). The three decision rows, their checks, the mutation round and the
    pictures are never cut.
17. **Fences restated**: write only `games/keifu-x-inheritance-r2/**`, `Cargo.lock` (the
    `keifu_x_inheritance_r2` entry only — the workspace globs `games/*`, no manifest edit),
    and `tools/yakin/runs/keifu-x-inheritance-r2/`; `games/keifu/**` read-only; no
    dependency; the manifest names `jidousha = { path = "../../crates/jidousha" }` alone.
    PR title `[yakin:V2:r2] Keifu X Inheritance: fork Keifu and deepen what a family
    inherits`, WORKER.md body, naming #129 as the comparison baseline, "Owner actions"
    carrying make-game §E.

## Open calls delegated to the implementer

- **How the pointer's heir button reaches the dock.** `UiState` must know which death
  page and which heir the pointer rests on so `dock_lines::hero_lines` can append WOULD
  INHERIT (for example `pointing_heir: Option<(usize, HeroId)>` set where `Target::Heir`
  is hit-tested). Constraint: one value, cleared when the pointer leaves; the section is
  built by `inheritance::succession` and nothing else; a sheet opened any other way
  shows no section.
- **The oath button's place on the card.** Constraint: inside the card's rect, clear of
  the seats and the tags, drawn with `summer::button` (or `greyed_button` when no one
  may swear — or absent; pick one and keep it on every card), and the floors stay clean
  at the native window and both web canvases.
- **The exact wording** of lines beyond the literals in decision 14 (page titles,
  the sheet's order of the SWORN BY block within the quest sheet, the WOULD INHERIT lines'
  order among heirloom, dream, blessings, marks — the order given is the default).
  Constraint: every check's literal is copied by hand from the content, never built from
  it.
- **Where `family: bool` is set for founders**: `household.json` gains no field unless
  the implementer prefers it; setting it in `household::found` for every founder is
  acceptable. Constraint: the content loader's checks still pass and `tools/verify keifu`
  is untouched.
- **The split of `xi.rs`** into two files at the 500-line rule, and which staging helpers
  move to `xi_stages.rs`.
- **The mutation list's exact faults** beyond the named ones, and whether the full
  mainline round is rerun (`--changed-since <copy commit>` is enough unless an escape
  suggests otherwise).
- **Capture staging** for the three pictures (which seed, which heir, the sworn card's
  party) — the staged sessions of the three checks are the intended source.
- **The subject pronoun's case** in "Sworn by Garrick: he must triumph." (lower, as
  mainline's call line sets "He must triumph." after a full stop; here after a colon).
