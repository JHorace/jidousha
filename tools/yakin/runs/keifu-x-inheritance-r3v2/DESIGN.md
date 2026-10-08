# DESIGN — keifu-x-inheritance-r3v2

task: keifu-x-inheritance-r3v2
variant: V2
model-as-configured: claude-fable-5-1
date: 2026-10-07 21:25 PDT

This is a **delta design**: every rule below is stated as "mainline does X; the
variant does Y". Mainline is `games/keifu/` as it stands on `main` (read-only for
this task, every byte of it). The variant is `games/keifu-x-inheritance-r3v2/`,
crate `keifu_x_inheritance_r3v2`, a copy of mainline that diverges by exactly the
rules in **Systems** and nothing else. Where this document names a mainline file,
function or line, it means the copy of it inside the variant's folder.

Reading order for the implementer: the task spec (`tools/yakin/tasks/
keifu-x-inheritance-r3v2.md`, then the base it points at), this file whole, then
mainline's `spec/SPEC.md` §3.2, §7.1, §7.3, §9.6, §11.3, §11.6, §15.1-15.2, §17.2,
§17.3, §18 and `spec/CONSTANTS.md` §8-§11 as the rules being bent. The source
files named in **Systems** are the ones to open; nothing in `crates/**`.

## What the game is

Keifu is a 25-year house of heroes: each year a summer of quests, a telling, a
winter at the hearth and a turning of the year, until the Sealed Door. The
variant makes the **Thorne name** the thing the player keeps: a family member who
goes questing in their own name and fails stains the name with a heritable black
mark; wanderers are outsiders whose deeds are their own until renown lets them
marry in; and a child is born leaning to one parent's aptitude, so what a line is
good at carries forward.

## The player-facing loop

Mainline's loop is unchanged in shape (SPEC §1, §2.1): summer, telling, winter,
turning, then the Door. The variant adds three readings and three consequences:

1. **Summer.** The player drags heroes onto quest cards as before. When a seated
   family member's *own* dream calls them to that quest (the card's "Dream:"
   line already names them), the quest is their **personal quest**. Pointing at
   the card opens the quest sheet in the dock as before; under that dreamer's
   call line the variant adds the stake: "If Garrick fails, a black mark on the
   Thorne name: -2 renown to the house and to him, and -1 a year while it is
   carried." The player reads it and commits with "Set out", as before.
2. **Telling.** If a personal quest came home a Setback or a Disaster, the quest
   page carries a new line: the dreamer "went in his own name and failed", the
   mark, and the renown it costs now (-2 house, -2 personal). A success or a
   triumph marks nothing.
3. **Winter.** The garden works as before. If exactly one of the pair is an
   outsider, the garden's preview reads "renown 1 of 4" instead of "will wed"
   until the outsider's personal renown reaches 4; at 4 it reads "marries in".
   Letting the winter pass weds them as before and, when one married in, adds a
   line saying so; a refused outsider's winter page says what renown they had
   and what the house asked. Two outsiders may wed each other and stay outsiders.
4. **Turning.** After the tales (+1 renown each, mainline), the name's marks:
   -1 renown per black mark carried by living family members, as one line. A
   death page of a hero carrying marks says how many, and how many an heir would
   take (half, rounded down); choosing an heir gives them that half, choosing "No
   one" buries the marks. A birth page says which parent the child takes after
   and which aptitude they lean to. A wanderer's arrival page says they are an
   outsider and what would make them one of the house.
5. **Sheets.** A hero's sheet shows an outsider's standing ("An outsider. Renown
   1 of 4 to marry in."), the hero's lean ("Leans to Wits, after Maren.") and,
   when they carry any, a BLACK MARKS section.

Everything else — the Door, epitaphs, ghosts, legacies, the family screen —
reads and plays exactly as mainline.

## Systems

In build order. Each names what it does, the file it lives in (inside the Fence,
all under `games/keifu-x-inheritance-r3v2/`), and the `docs/api/` surface it
touches by file and item. Most systems touch the engine only through the game's
own modules, which already wrap the surfaces they need; those say "none new".

### S0 — The fork (its own commit, before anything else; the spec's fork rule)

- **What:** `cp -r games/keifu games/keifu-x-inheritance-r3v2`, then rename so the
  two crates coexist and nothing in the copy names mainline. Behaviour identical.
  · `games/keifu-x-inheritance-r3v2/**`, `Cargo.lock` (the new crate's entry only,
  written by `cargo check`, never by hand) · touches none new.
- The renames, each a literal find-and-replace inside the copy (verify with
  `grep -rn keifu games/keifu-x-inheritance-r3v2/src games/keifu-x-inheritance-r3v2/Cargo.toml`
  afterwards: the only hits left must be the SPEC/FINDINGS prose and the
  `screens/` file names, which are history and stay):
  1. `Cargo.toml`: `name = "keifu"` → `name = "keifu_x_inheritance_r3v2"`. The
     binary takes the package name; `tools/verify keifu_x_inheritance_r3v2`,
     `tools/build-web keifu_x_inheritance_r3v2` and `tools/mutate
     keifu_x_inheritance_r3v2 …` all key off it.
  2. `src/main.rs`: `title: "Keifu"` → `title: "Keifu X Inheritance"`; the two
     `cargo run -p keifu` mentions in the module header → `-p
     keifu_x_inheritance_r3v2`; `tools/verify keifu` → `tools/verify
     keifu_x_inheritance_r3v2`. Rewrite the header's first paragraph to say what
     the fork is (one sentence: a variant of Keifu that deepens what a family
     inherits — black marks, outsiders, leans — per this DESIGN.md; mainline
     stays the faithful port). Keep the rest of the header.
  3. `src/art.rs`: `ASSET_ROOT: &str = "games/keifu/assets"` → `"games/keifu-x-inheritance-r3v2/assets"`
     (`tools/check-assets` requires a game under `games/<dir>/` to load from
     `games/<dir>/assets`).
  4. `src/verify.rs`: the verdict line `"verified keifu: …"` → `"verified
     keifu_x_inheritance_r3v2: …"` (the `verified ` prefix is the contract with
     `tools/verify`; what follows is free), `"verify keifu FAILED"` →
     `"verify keifu_x_inheritance_r3v2 FAILED"`, and the message naming
     `games/keifu/assets/{path}` → the new asset root.
  5. `src/checks.rs`: `run \`cargo run -p keifu\`` → `-p keifu_x_inheritance_r3v2`.
  6. Every `[keifu]` error-message prefix across `src/**` (57 of them) →
     `[keifu_x_inheritance_r3v2]`: `grep -rl '\[keifu\]' src | xargs sed -i 's/\[keifu\]/[keifu_x_inheritance_r3v2]/g'`.
  7. `src/capture.rs`: every PNG name in `capture_all`'s table, `"keifu…png"` →
     `"xi…png"` (`"keifu.png"` → `"xi.png"`, `"keifu-w8-death.png"` →
     `"xi-w8-death.png"`, and so on). Both games write into `target/verify/`
     during `tools/test`; same names would overwrite mainline's pictures.
  8. `mutants/*.txt` need no change: their `@ src/…` paths are relative to the
     game's directory. `screens/*.png`, `spec/**`, `SPEC-GAPS.md`, `FINDINGS.md`,
     `art/*.py` are copied as they are — they are the fork's inherited canon.
- Gate for this commit: `cargo check --workspace` (writes the `Cargo.lock` entry),
  the fast gate, `python3 tools/verify keifu_x_inheritance_r3v2` → `pass`, and
  `python3 tools/verify keifu` → `pass`. Commit as
  `yakin(keifu-x-inheritance-r3v2): fork keifu` with nothing else in it.

### S1 — The family name and who is of it

- **What:** mainline has no notion of family; a hero's `house` is a name string
  and every hero is alike. The variant adds `House::family_house: String`
  ("Thorne") and `Hero::is_family: bool`. **Family** = the nine founders (all of
  them, whatever their house name — Brannoc Hale and Odo Fenn stood under the
  roof when the story began) and every child born with at least one family
  parent. **Outsider** = a wanderer (SPEC §17.2) who has not married in, and a
  child of two outsiders. · `src/hero.rs` (field), `src/house.rs` (field),
  `src/household.rs` (read the key; found every founder `is_family = true`),
  `src/newcomers.rs` (`newcomer()` defaults `is_family: false`; every other place that builds a
  `Hero` literal — `household.rs`, any test fixture the compiler names — sets
  the three new fields), `src/births.rs`
  (`child.is_family = heroes[first].is_family || heroes[second].is_family`),
  `src/wanderer.rs` (stays false), `spec/content/household.json` (new top-level
  key `"family_house": "Thorne"`, validated at load to equal some founder's
  `house`, else a `SchemaError` naming the key). · touches none new.
- Founders also get their `lean` here (S4) so the founding is written once.

### S2 — Outsiders: what their deeds do not do

- **Reward (mainline SPEC §7.3: house renown += quest renown (+1 triumph) +
  carriers, whoever went).** Variant: the house gains the quest renown (+1 on a
  triumph) only if **some member of the party is family**; the carriers' +1 each
  is unchanged; every member's personal renown rises as mainline's does. With no family
  member in the party, `lines.quest.reward` is replaced by the new
  `lines.quest.outsiders_reward` ("% came home with it, and the house gained
  nothing: no Thorne went."). · `src/reward.rs`: one gate around
  `house.add_renown(renown + …)` and the line choice. · none new.
- **The long table (mainline §11.3 step 5: the first adult teller gives the house
  +1).** Variant: the first adult teller **who is family** gives the house +1
  (`Teller::House`); an outsider teller is `Teller::Own` wherever they sit (their
  +1 is their own). · `src/plans.rs` `tellers()` (it already has `heroes`): the
  `adults` counter counts family adults only; an outsider adult returns `Own`.
  `src/winter.rs` changes nothing (it reads the part). · none new.
- **Heirs (mainline §15.1: the living other than the dead, by rank).** Variant:
  **outsiders are never offered as heirs** and never nearest kin: `heirs::heirs`
  filters `is_family`. An outsider's own death page still gathers heirs (family
  ones), so their heirloom and dream go to the house — a cheap body's things
  stay. · `src/heirs.rs` `heirs()`: one `.filter(|&id| heroes[id].is_family)`.
  · none new.
- **The black mark (S3) is a family matter**: an outsider's failed personal
  quest marks nobody (they are not of the name).
- **Death or departure** leaves what mainline's does — a death page, grief, an
  epitaph, a crowned outsider's patron — and no mark. The one difference is the
  heir list above.
- **The sheet.** Under the condition line (`hero_sheet.renown*`), an outsider
  gets one Body line: `hero_sheet.outsider` ("An outsider. Renown % of % to marry
  in.") when renown < 4, else `hero_sheet.outsider_proven` ("An outsider with
  renown % of %. A winter in the garden would make % one of the house."). Family
  heroes get no line (mainline's sheet). · `src/sheet.rs` `hero_sheet`, right
  after the `out.push(match hero.fate …)` condition line. · none new.
- **The arrival page.** After `lines.arrival.who`, a new line
  `lines.arrival.outsider`. · `src/wanderer.rs` `arrive`. · none new.

### S3 — The black mark

- **The personal quest.** Mainline has none; its nearest thing is the dream call
  (SPEC §9.6, `calls::dream_call`). Variant: a posted quest is **hero h's
  personal quest** iff all of: h is seated on it; h `is_family`; the quest is a
  template quest (not a ghost's, not a Door lock); and `dream_call(content,
  heroes, h, quest.facts(), party)` returns `Some(call)` with `call.burden ==
  false` and `call.telling != Telling::StayBehind`. One function owns this:
  `marks::personal_quest(content, heroes, hero, quest, party) -> bool` in a new
  `src/marks.rs`. Carrying someone else's dream (a burden) is not going in your
  own name; a ghost is the dead's business; the Door is the house's.
- **Failing one** = the quest resolves **below SUCCESS** (Setback or Disaster)
  with h in the party. A Success when the call needed a Triumph is not a failure.
- **The consequence**, one function the sheet's preview and the resolution both
  call: `marks::consequence(house, hero) -> Consequence { marks_after: i32,
  house_renown: i32, personal_renown: i32, yearly: i32 }` = `{ hero.marks + 1,
  MARK_HOUSE_RENOWN (2), MARK_PERSONAL_RENOWN (2), MARK_YEARLY_RENOWN (1) }`.
  `marks::stain(f: &Afield, house, hero, lines)` applies it: `hero.marks =
  marks_after`; `hero.renown = (hero.renown - personal_renown).max(0)`;
  `house.add_renown(-house_renown)`; pushes `lines.quest.black_mark`; records a
  `DeedKind::BlackMark` deed (new variant, weight = the quest's danger, telling
  `lines.deed.black_mark`). Any exhaustive `match` on `DeedKind` the compiler
  then names gains an arm that ignores the new kind (the epitaph reads no mark;
  Non-goals). · `src/marks.rs` (new), `src/hero.rs` (`marks: i32`,
  `DeedKind::BlackMark`). · none new.
- **Where in the resolution.** Mainline §7.1 steps 1-15 in `resolve::resolve_party`.
  Variant inserts **step 7b**, after step 7 (the disaster's renown line) and
  before step 8 (facing the fear): if `outcome < Outcome::Success`, for each
  member in party order for whom `personal_quest` held **at the moment of
  resolution** (read before step 1, since the party is known then), `stain`. At
  the Door `resolve_party` is called with a lock quest: `personal_quest` is false
  there by construction. · `src/resolve.rs`. · none new.
- **Stacking and the yearly cost.** Marks are an integer on the hero, no cap.
  At the turning, mainline step 8 adds `+1 renown per tale`. Variant adds **step
  8b**: `total = sum of marks over living family heroes`; if `total > 0`,
  `house.add_renown(-total * MARK_YEARLY_RENOWN)` and one line, `lines.turning.mark_one`
  ("The % name carries a black mark: -% renown.") when total == 1, else
  `lines.turning.marks_many` ("The % name carries % black marks: -% renown.").
  The line goes on the "Year N begins" page with the tales' line, so a turning
  that was quiet gets that page. · `src/turning.rs`: a `marks_cost` beside
  `tales`, pushed into `year_lines` right after `tales`'. · none new.
- **Decay across generations = halving at succession.** The heir takes
  `dead.marks / MARK_INHERITED_SHARE` (integer division by 2): 1 mark dies with
  its bearer, 2-3 pass one on, 4-5 two. No other decay: a living hero's marks
  never fade. (S5 owns the death page.)
- **The preview on the quest sheet.** Mainline's `quest_sheet` pushes one
  `call_line` per called hero. Variant: right after each call line whose hero's
  `personal_quest` holds for the *seated* party, push one Body line
  `quest_sheet.mark_stake` filled from `consequence(house, hero)`: "If % fails, a
  black mark on the % name: -% renown to the house and to %, and -% a year while
  it is carried." (name, family house, house_renown, object pronoun, yearly).
  Nothing is added to the quest card. · `src/quest_sheet.rs`. · none new.
- **The sheet.** When `hero.marks > 0`, after the FEAR section and before the
  second column: Heading `hero_sheet.marks` ("BLACK MARKS") and Body
  `hero_sheet.marks_line` ("% on the name. -% renown a year; an heir would take
  %.") with marks, marks × yearly, marks / 2. · `src/sheet.rs`. · none new.

### S4 — Trait inheritance: the lean

- **What passes at birth.** Mainline (SPEC §17.3): each base aptitude =
  `max((F + S) / 4 + U{0,1}, 1)`; a fear tag by the per-parent chain (40% /
  80% broken / 50% born brave); every blessing of both parents. Variant keeps all
  three and adds a **lean**: `Hero::lean: Option<Lean>`, `Lean { aptitude:
  Aptitude, from: Option<HeroId> }`. A newborn's lean is one parent's lean,
  chosen by **one coin** (`chance::index(rng, 2)`: 0 → `first`'s, 1 →
  `second`'s), and the child's base in that aptitude gets `+LEAN_BONUS` (1),
  capped at `APTITUDE_LIMIT`. Roll order inside `born`: pronoun, name, the three
  U{0,1}, **the lean coin**, then mainline's fear rolls. The parent's own lean is
  what passes (not their best aptitude today), so a line's lean can run three
  generations unchanged.
- **One function**, `inheritance::birthright(heroes, first, second, coin: usize)
  -> Lean` in a new `src/inheritance.rs`, called by `births::born`; the birth
  page line `lines.birth.lean` ("% takes after %: % leans to %.") reads the same
  `Lean`. If the chosen parent has no lean (impossible after S1/S2 — every hero
  gets one — but the type allows it) fall to the other parent, then to the
  child's best aptitude with `from: None`.
- **Founders**: `lean = Some(Lean { aptitude: best_aptitude(), from: None })` at
  founding (Garrick Might, Maren Wits, Pip Wits, Ysolde Wits, Brannoc Might, Odo
  Spirit, Wren Might, Elsbeth Wits, Aud Spirit — `best_aptitude` ties go to the
  lower index). **Wanderers**: the gift aptitude, `from: None`.
- **The sheet.** After the three aptitude Note lines: one Note,
  `hero_sheet.lean_after` ("Leans to %, after %.") when `from` is some, else
  `hero_sheet.lean` ("Leans to %."). · `src/sheet.rs`. · none new.
- Fear and blessings: mainline's rules and rolls, restated here only so the
  implementer knows they are the rest of "which traits pass".
- · `src/inheritance.rs` (new), `src/hero.rs`, `src/births.rs`, `src/household.rs`,
  `src/wanderer.rs`, `src/sheet.rs`. · `docs/api/jidousha-api.md`: `Rng` — through
  the game's `chance::index`, nothing new.

### S5 — Succession: what an heir takes

- **Mainline (§15.1-15.2):** a death page waits when the dead holds an heirloom
  or leaves an undone dream; the heir takes both. **Variant:** the page also
  waits when `dead.marks / 2 > 0`, and the heir also takes those marks.
- **One function** both the page's preview and the choice read:
  `inheritance::succession(heroes, dead) -> Succession { marks: i32 }` =
  `dead.marks / MARK_INHERITED_SHARE`. (It lives beside `birthright` in
  `src/inheritance.rs`: the two halves of what a Thorne inherits.)
- **The page** (`death_page::death_page`): after `lines.death.leaves_dream`, when
  `dead.marks >= 2` push `lines.death.leaves_marks` ("% carried % black marks on
  the % name. An heir takes %.") with He/She, marks, family house,
  `succession().marks`; when `dead.marks == 1` push `lines.death.mark_buried`
  ("% carried one black mark on the % name. It is buried with %."). These count
  toward `bequest_end`. `leaves = heirloom || undone || succession().marks > 0`.
- **The choice** (`heirs::choose`): after the heirloom and before the dream, if
  `succession().marks > 0`: with an heir H, `H.marks += that`, line
  `lines.heir.takes_mark` ("% carries one of %'s black marks now.") or
  `lines.heir.takes_marks` ("% carries % of %'s black marks now."); with no one,
  `lines.heir.marks_buried` ("%'s black marks go into the ground with %."). The
  dead's marks are set to 0 either way (they are no longer on a living bearer,
  so step 8b stops counting them).
- **The heir buttons** are mainline's labels, unchanged (the page's line above
  carries the number; `heirs()` already drops outsiders, S2). Pointing at a
  button opens the heir's sheet in the dock as mainline does; that sheet now
  shows their lean, fear and marks (S3, S4) — the "inherited traits" the
  decision needs at the moment of choosing.
- · `src/inheritance.rs`, `src/death_page.rs`, `src/heirs.rs`. · none new.

### S6 — Marrying in

- **Mainline (§11.6)** checks in order: NOBODY, WAITING, KIN, TOO_YOUNG,
  TOO_FAR_APART, RIVALS, WED_ALREADY, else WILL_WED. **Variant** inserts one
  check after WED_ALREADY: if exactly one of the pair is an outsider and that
  outsider's `renown < FAMILY_RENOWN_TO_WED` (4) → `Courtship::Unproven`; if
  exactly one is an outsider at or above 4 → `Courtship::MarriesIn`; else (both
  family, or both outsiders) WILL_WED as mainline. `Unproven` and `MarriesIn`
  are new variants; their notes: `winter.courtship_notes.UNPROVEN` = "renown %
  of %" (the outsider's renown, 4 — the same length as "too far apart", so the
  hearth's note floor holds), `winter.courtship_notes.MARRIES_IN` = "marries in".
  · `src/plans.rs` `courtship`, `Courtship`, `Courtship::note`. · none new.
- **Resolution** (`winter::court`): `MarriesIn` does everything WILL_WED does
  (the spouse bond, the WEDDINGS line, the WED deeds) and then sets the
  outsider's `is_family = true` and pushes `lines.winter.married_in` ("% is of
  the % name now. % renown was enough: the house asked %."). `Unproven` pushes
  `lines.winter.unproven` ("% walked in the garden with %. An outsider with %
  renown may not marry in: the house asks %.") naming the outsider first, in
  place of mainline's `courting_failed`. Both occupants still take the COURT
  moment. `court`'s panic arm for a verdict with an empty seat covers
  `MarriesIn` too. `play::seat_the_winter` (the batteries' wedding) treats
  `MarriesIn` like `WillWed`. `plans_tests.rs` gains two tests: an outsider at
  renown 3 beside Ysolde is `Unproven`, at 4 `MarriesIn`; two outsiders at any
  renown are `WillWed`.
- **What marrying in changes:** from then on their won quests and told tales
  count for the house (S2), they are offered as heirs (S2), their children are
  family (S1), and their failed personal quests mark the name (S3). Their
  personal renown stays theirs: nothing transfers to the house at the wedding.
- **What it does not change:** traits — their lean passes to children by S4
  whether or not they married in (two outsiders' children lean too; they are
  just not family).
- · `src/plans.rs`, `src/winter.rs`, `src/play.rs`, `spec/content/ui-text.json`,
  `spec/content/lines.json`. · none new.

### S7 — Content and words

- New `lines.json` entries (under `"lines"`, each an object with `text`, `args`,
  `when`, `source: "variant: DESIGN.md S<n>"`), exact texts as quoted in S2-S6:
  `quest.outsiders_reward`, `quest.black_mark` ("% went in % own name and failed.
  A black mark on the % name: -% renown to the house, and to %." — name,
  possessive, family house, 2, name), `deed.black_mark` ("failed in % own name at
  %" — possessive, place), `turning.mark_one`, `turning.marks_many`,
  `death.leaves_marks`, `death.mark_buried`, `heir.takes_mark`,
  `heir.takes_marks`, `heir.marks_buried`, `winter.married_in`,
  `winter.unproven`, `arrival.outsider` ("% is an outsider. % renown is % own,
  and the house takes none of it; at % renown % may marry in." — name,
  possessive (capitalised), possessive, 4, subject), `birth.lean`.
- New `ui-text.json` entries: `winter.courtship_notes.UNPROVEN`,
  `winter.courtship_notes.MARRIES_IN`, `hero_sheet.outsider`,
  `hero_sheet.outsider_proven`, `hero_sheet.lean`, `hero_sheet.lean_after`,
  `hero_sheet.marks`, `hero_sheet.marks_line`, `quest_sheet.mark_stake`.
- One `W` variant per key in `src/words.rs` (the loader resolves every key at
  start, so a typo stops the game with the key's path). Pronoun forms come from
  `content.lore.pronouns[...]` as every mainline line takes them.
- The new constants, each with a doc comment citing this file's section, in
  `src/constants.rs`: `FAMILY_RENOWN_TO_WED: i32 = 4`, `MARK_HOUSE_RENOWN: i32 =
  2`, `MARK_PERSONAL_RENOWN: i32 = 2`, `MARK_YEARLY_RENOWN: i32 = 1`,
  `MARK_INHERITED_SHARE: i32 = 2`, `LEAN_BONUS: i32 = 1`.
- A short `games/keifu-x-inheritance-r3v2/VARIANT.md` (≤ 40 lines): what the fork
  is, the six constants, and a pointer to this DESIGN.md for the rules. The
  copied `spec/SPEC.md` stays mainline's; VARIANT.md is where the deltas are
  owned inside the game folder.
- · `spec/content/lines.json`, `spec/content/ui-text.json`, `src/words.rs`,
  `src/constants.rs`, `VARIANT.md`. · none new.

### S8 — The checks (`--verify`), one module per decision row, and the pictures

- `src/xi_marks.rs`, `src/xi_outsiders.rs`, `src/xi_inheritance.rs` (new), wired
  into `verify::run` after `w10_battery` and before `sessions::check_family`, each
  returning a summary line pushed into `summary`. The scripts are in **Gates to
  add**. They reuse the helpers mainline's checks already export: `verify::
  {session, point_at, page_of, hero_named, dock_pages, dock_read}`, `scripted::
  {lines_in, drag, Pointer, center_of}`, `w4::seat_the_oracle`,
  `w7::{stay_home_into_winter, seat_at, group_lines}`, `w8::{stage_garricks_winter,
  go_to_the_choice, heir_labels, page_leaves, stage_turned_year, turn_to_kind}`,
  `resolve::resolve_rolled`, `wanderer::arrive`, `play::read_to_summer`.
- **Mainline oracles the variant deliberately breaks, rewritten** (each one a
  Deviations line in the PR, as the base spec asks): `w8::stirred` — its wanderer
  is an outsider now, so Garrick's stirred heir list is **seven** labels: Maren,
  Pip, Odo, "Ysolde, of the house (not the dream)", "Brannoc, of the house (lays
  one aside)", Wren, "No one. Let it lie."; the `burdened` arm no longer changes
  the list (keep the parameter, assert the same seven both ways, and add one
  more assertion: on a fresh session staged the same way, after `stir` set the
  wanderer's `is_family = true` by hand before letting the winter pass, and
  assert the eighth label is back as mainline had it — `"<name>, of the house"`
  plus `" (not the dream)"` when `burdened`). `w7_battery::check_agreement`'s
  match on the garden's verdict gains one arm, `Courtship::MarriesIn => kind ==
  Some(BondKind::Spouse)`, beside `WillWed`'s; its `all_verdicts` coverage list
  stays as it is (the stirred hearths need not reach the new verdicts). No other mainline check
  moves: founders are all family, so W0-W10's oracles read as before; the
  sheet additions are new lines between the W1 oracle's (it matches a
  subsequence, `oracles::check_w0_and_w1_on`).
- **Pictures** (`capture::capture_all`, three rows added at the end of its
  table): `xi-marks-sheet.png` — the quest sheet with the mark stake, Garrick
  and Brannoc seated on "Grave goods", the card pointed at; `xi-garden-unproven.png`
  — the hearth with the outsider and Ysolde in the garden, "renown 1 of 4"
  showing; `xi-death-marks.png` — Garrick's death page with the marks line and
  the heir buttons. Stage each with the same functions the checks use.
- **Mutants:** a new `mutants/xi.txt`, ~24 faults, one per rule line above: each
  of the six constants nudged; the family gate in `reward` dropped; `tellers`'
  family test dropped; `heirs`' filter dropped; `personal_quest`'s burden
  exclusion, ghost exclusion and `StayBehind` exclusion each dropped;
  `outcome < Success` → `<= Success`; `stain`'s personal floor dropped; step 7b
  moved after the deaths (so a dead dreamer is not marked — the check must see
  the mark count move); step 8b's sign flipped; the lean coin fixed to `first`;
  the lean bonus applied to the other parent's aptitude; `succession`'s division
  → `/ 3`; `leaves` without the marks term; `choose` not zeroing the dead's
  marks; `Unproven` ordered before `WedAlready`; `MarriesIn` not setting
  `is_family`; a newborn's `is_family` as `&&`. Run `tools/mutate
  keifu_x_inheritance_r3v2 mutants/xi.txt --fast` and report `N of 24 noticed`.
  Then, time permitting, `tools/mutate keifu_x_inheritance_r3v2 mutants/*.txt
  --fast --changed-since <the fork commit's sha>` for the mainline lists over the
  files the variant touched; if the window does not allow it, say so in the PR.
- · `docs/api/jidousha-testing.md`: `HeadlessSim`, `FrameRecorder`, `InputScript`,
  `SnapshotBuilder` (through `scripted.rs`); `docs/api/jidousha-capture.md`: the
  `capture:` line (through `capture.rs`). Nothing new is imported.

- Assets: **none.** The variant reuses the cast's twelve sprites under
  `assets/` (copied with the fork); no new figure, tile or sound is assumed.

## Gates to add

Every check is deterministic: seeds are `verify::SEEDS` / `w5::recorded()`, dice
are staged through `resolve::resolve_rolled`, and every expectation is a shipped
literal. Expected strings below use the content texts of S7 filled with the
founders' names and pronouns; copy them into the checks as literals.

- **G1 — the fork is pure** — input: the fork commit · asserts: `git show
  --stat <fork sha>` names only paths under `games/keifu-x-inheritance-r3v2/` and
  `Cargo.lock`; `python3 tools/verify keifu_x_inheritance_r3v2` on that commit
  reports `pass` in `target/verify/keifu_x_inheritance_r3v2.json` · covers
  Done-when: "its first code commit on the branch is the pure copy-and-rename
  with its verify green" and "exists as a workspace member".
- **G2 — mainline untouched** — input: the branch head · asserts: `python3
  tools/verify keifu` reports `pass`; `git diff --stat origin/main...` names no
  path under `games/keifu/` · covers Done-when: "tools/verify keifu still reports
  pass … no path under games/keifu/".
- **G3 — the diff is fenced** — input: the branch head · asserts: `git diff
  --name-only origin/main...HEAD` names only `games/keifu-x-inheritance-r3v2/**`,
  `Cargo.lock` (whose diff is the one new `[[package]] name =
  "keifu_x_inheritance_r3v2"` block) and `tools/yakin/runs/keifu-x-inheritance-r3v2/**`
  · covers Done-when: "names only …".
- **G4 — personal quest, decision row 1** (`xi_marks::check_personal_quest`) —
  input: `session(SEEDS[0])`, `w4::seat_the_oracle` (Garrick and Brannoc on
  "Grave goods"), then `point_at(Target::Quest(0), false)` · asserts, through
  the dock (`dock_pages`/`dock_read`): the sheet holds, in order, "Garrick's
  dream: Win a triumph at the Barrow. He must triumph." and immediately after it
  "If Garrick fails, a black mark on the Thorne name: -2 renown to the house and
  to him, and -1 a year while it is carried."; no mark line follows Ysolde's call
  line (she is not seated). Then, on a clone of the house with the run's `Rng`
  cloned out, `resolve_rolled(content, house, rng, 0, [1, 1])` (power 12, demand
  9..11: margin -2..-4, a Setback on every seed) · asserts: the page's lines
  contain "Garrick went in his own name and failed. A black mark on the Thorne
  name: -2 renown to the house, and to Garrick."; `heroes[garrick].marks == 1`,
  `heroes[garrick].renown == 4` (6 - 2), `house.renown == 13` (15 - 2),
  `heroes[brannoc].marks == 0`; and on a second clone `resolve_rolled(…, [6,
  6])` (margin ≥ +6, a Triumph) · asserts: no "black mark" line, `marks == 0`,
  Garrick settled (mainline W3). Then on the first clone (the Setback): set
  `heroes[garrick].age = 40` (so old age cannot take him and bury the mark —
  CONSTANTS §10 rolls nothing below 55), then, since `resolve_rolled` opens no
  telling for `season::leave_the_telling` to close, do what it does by hand —
  `house.calendar.begin_winter(); house.open_hearth();` — then
  `season::let_the_winter_pass`, `play::choose_every_heir(first_heir)` and
  `season::summer_comes` · asserts: the passage's "Year 2 begins" page (read
  before `summer_comes` clears it) holds "The Thorne name carries a black mark:
  -1 renown." and `house.renown == 12` (13 less the mark's year; no tales). Run
  on every seed of `recorded()`.
  · covers Done-when: "verify … with a check for each decision row" (row 1).
- **G5 — the mark passes only in one's own name** (unit tests, `src/marks.rs`
  `#[cfg(test)]`, each named as a sentence) — input: `testkit::house()` ·
  asserts: `personal_quest` is false for Maren carrying Garrick's dream as a
  burden on a Barrow quest she is seated on; false for a ghost quest (stage one
  with `ghost::ghost_quest`); false for an outsider (a founded hero with
  `is_family = false`) with a calling dream; false for Garrick not seated; true
  for Garrick seated on "Grave goods". `consequence` on a hero with 2 marks
  returns `marks_after 3, 2, 2, 1`; `stain` floors personal renown at 0.
  · covers Done-when: row 1 ("one function computing a failure's mark").
- **G6 — marrying in, decision row 2** (`xi_outsiders::check_marry_in`) — input:
  `session(SEEDS[0])`, `w7::stay_home_into_winter`; `wanderer::arrive` with the
  run's `Rng` cloned out (as `w8::stir` does), then set the wanderer's `age = 30`
  and `renown = 1`; `open_hearth`; `w7::seat_at(<wanderer name>, Seat::Garden(0))`
  and `seat_at("Ysolde", Seat::Garden(1))` through the screen · asserts:
  `group_lines(Group::Garden)` contains "renown 1 of 4"; the wanderer's sheet
  (point at their card, read the dock) contains "An outsider. Renown 1 of 4 to
  marry in."; `point_at(LetWinterPass)`: the winter page holds "<Name> walked in
  the garden with Ysolde. An outsider with 1 renown may not marry in: the house
  asks 4."; `is_family == false`; no spouse bond. Second session, same staging
  with `renown = 4` · asserts: the garden reads "marries in"; after the winter
  the page holds a WEDDINGS line naming both and "<Name> is of the Thorne name
  now. 4 renown was enough: the house asked 4."; `is_family == true`; the spouse
  bond on both sides. In both sessions, set Garrick's `age = 93` before letting
  the winter pass (as `stage_garricks_winter` does), so his death page comes in
  the same turning, after the garden has resolved · asserts: after
  `go_to_the_choice`, the unproven session's `heir_labels` are the W8 oracle's
  seven and never name the wanderer; the married-in session's include
  "<Name>, of the house" as the seventh, before "No one. Let it lie.". Run on
  `recorded()[0]` and `recorded()[1]`. · covers Done-when: row 2 ("a below-threshold outsider
  refused and an at-threshold one accepted, with the threshold and the
  outsider's renown in the transcript").
- **G7 — outsiders confer no renown** (unit tests in `src/reward.rs` and
  `src/plans.rs`) — input: `testkit::house()`, a founded hero flagged
  `is_family = false` · asserts: a 1-seat quest won by that hero alone through
  `resolve_rolled` leaves `house.renown == 15` and the page holds the
  `outsiders_reward` line, while their personal renown rose by the quest's
  renown; the same quest with Garrick beside them raises the house as mainline;
  `tellers([outsider, Garrick])` is `[Own, House]`; `tellers([Garrick,
  outsider])` is `[House, Own]`. · covers Done-when: row 2 ("what marrying in
  changes (renown transfer…)").
- **G8 — succession, decision row 3** (`xi_inheritance::check_succession`) —
  input: `session(SEEDS[0])`, `w8::stage_garricks_winter`, then set
  `heroes[garrick].marks = 3`; `point_at(LetWinterPass)`; `go_to_the_choice` ·
  asserts: `page_leaves(1)[0]` holds "He carried 3 black marks on the Thorne
  name. An heir takes 1." and the seven heir labels of the W8 oracle; `point_at`
  Maren's button without clicking: the dock's first lines are "Maren Thorne" and
  the sheet contains "Leans to Wits." and "Water (deep water)" and no BLACK
  MARKS heading; click it · asserts: the page now holds "Maren carries one of
  Garrick's black marks now."; `heroes[maren].marks == 1`; `heroes[maren].lean
  == Some(Lean { aptitude: Wits, from: None })` (what the sheet showed);
  `heroes[maren].fear.tag == Water`; `heroes[garrick].marks == 0`; Maren holds
  Thornfall and Garrick's dream (mainline). A second session with `marks = 1` ·
  asserts: the page holds "He carried one black mark on the Thorne name. It is
  buried with him." and, after choosing Maren, `heroes[maren].marks == 0`. A
  third with `marks = 2`, Garrick's heirloom and dream removed by hand · asserts:
  the page still waits (heir buttons present), and "No one. Let it lie." yields
  "Garrick's black marks go into the ground with him." and `marks == 0` on
  everyone. Run on every seed of `recorded()`. · covers Done-when: row 3 ("the
  shown traits and marks equal the heir's actual state after choosing").
- **G9 — the lean is born** (`xi_inheritance::check_birth`, plus unit tests) —
  input: `session(recorded()[0])`, `w8::stage_turned_year` (it weds Maren and
  Brannoc and retries the turning until a child is born), `turn_to_kind(Birth)`
  · asserts: the birth page holds a line matching "<child> takes after Maren:
  <he/she> leans to Wits." or "… after Brannoc: … leans to Might."; the child's
  `lean.from` is that parent and `lean.aptitude` that parent's lean's aptitude;
  `child.is_family == true`; `child.base(lean.aptitude) >= (F + S) / 4 + 1`
  (the share's floor plus the bonus) and `<= 9`. Unit tests on `birthright`:
  coin 0 gives `first`'s lean, coin 1 `second`'s; a child of two outsiders has
  `is_family == false`. · covers Done-when: row 3's "one inheritance function
  … the birth/succession use".
- **G10 — the whole battery still runs** — input: `python3 tools/verify
  keifu_x_inheritance_r3v2` · asserts: `pass`, with the W0-W10 oracles as before
  except `w8::stirred` as rewritten in S8, and the three new summary lines and
  three `also captured:` lines present · covers Done-when: "verify … reports
  pass … with a check for each decision row".
- **G11 — the web build** — input: `python3 tools/build-web
  keifu_x_inheritance_r3v2 && python3 tools/serve-web keifu_x_inheritance_r3v2
  --check` · asserts: both exit 0 · covers Done-when: the build-web line.
- **G12 — the PR** — input: the PR · asserts: title `[yakin:V2:r3] Keifu X
  Inheritance: fork Keifu and deepen what a family inherits`; WORKER.md §4's body
  with the three extra lines after `Task:`; Deviations listing every departure
  from this file and the `w8::stirred` rewrite; `Findings:`; `Owner actions:`
  with make-game §E's five lines ("nobody played it" at the preview) · covers
  Done-when: the PR line.

## Non-goals

Cut so the implement stage fits the window (Thursday 04:30), or left out on
purpose. None needs an engine change; there is no FINDINGS entry.

- **No change to the quest card.** The mark stake lives on the quest sheet
  (dock), where it wraps freely; the card's fixed height is a readability-floor
  risk the window cannot afford.
- **Marks do not touch power, odds, the Door, the crown or the epitaph.** They
  cost renown (now and yearly) and pass to heirs; nothing else reads them. An
  epitaph sentence for a stained life is a later wave.
- **No family screen or top-bar change.** Outsiders are not drawn differently on
  the tree; the tally does not count marks.
- **No mark for a burden's failure, a ghost quest, or the Door.** Stated in S3;
  listed here because it is a cut, not an oversight.
- **Nothing transfers at the wedding.** An outsider's personal renown stays
  theirs; the house gains no renown for a good match.
- **Outsiders' children of two outsiders stay outsiders**, and may marry in on
  the same terms. No "born in the house" rule.
- **Genetics stop at the lean.** No heritable scars, destinies (beyond
  mainline's Door promise), vocations or pronoun rules; no mutation; no
  grandparent reach-back.
- **No new art, sound or screen.** All three surfaces are lines on panels that
  already exist.
- **The mutation round is `mutants/xi.txt`**, plus the mainline lists filtered
  to changed files if time allows. The full seven-list round (736 faults, hours
  of machine time) is out of the window.
- **Mainline `games/keifu` is not touched**, not even to share code: the variant
  is a copy, by the owner's ruling.

## Decisions already made

Settled; the implementer does not relitigate them.

1. **The family is the founding nine; outsiders are wanderers.** Reason: it
   keeps every W0-W10 oracle true (all founders act for the house as before) and
   puts the outsider rule where the game already makes strangers — the
   arrival page.
2. **Family passes by either parent; marrying in flips `is_family`.** Reason:
   one boolean, one flip, no house-name arithmetic; "Thorne" is a word in the
   lines, not a rule.
3. **A personal quest is a seated family member's own-dream call, telling not
   "stay behind", on a template quest.** Reason: the call is the one place
   mainline already says "this quest is yours", and `calls::dream_call` already
   computes it for the card.
4. **Failing is Setback or Disaster.** Reason: the band the player already reads
   as failure on the card; a success that falls short of a needed triumph brings
   the quest home.
5. **A mark costs -2 house and -2 personal now, -1 house a year while carried,
   and halves at succession.** Reason: the yearly cost mirrors the tales' +1 a
   year, so the name's shames and glories are one ledger; halving makes a
   single mark die with its bearer and a heavy one outlive them.
6. **The marks line goes on the "Year N begins" page with the tales.** Reason:
   that is where the house's standing is already told each year.
7. **Outsiders are never heirs; an outsider's own death page offers family
   heirs.** Reason: the spec's "cheap bodies" — their things stay with the
   house; the name does not pass to them.
8. **The house gains a quest's renown only if a family member went; carriers'
   +1 is unchanged.** Reason: one gate on the lump sum mainline already pays;
   CARRY_THE_HOUSE is a prophecy about the house and stays as the Seer said.
9. **The threshold is 4 personal renown.** Reason: wanderers arrive with 0..2,
   a won quest pays 1..5, so one or two good summers earn it; it is the same
   bar as "Earn 4 renown" at Court.
10. **Two outsiders may wed each other and stay outsiders.** Reason: "exactly
    one outsider" keeps the rule one line; a house of wanderers is still not
    the Thornes.
11. **The lean is one coin between the parents' leans, +1 in it.** Reason: the
    smallest rule that makes a line visibly favour an aptitude across
    generations and replays on a seed.
12. **Founders' leans are their best aptitude, wanderers' their gift.** Reason:
    deterministic, no content edit, and true to who they already are.
13. **The death page waits on marks ≥ 2 alone.** Reason: half a mark passing is
    a thing the player decides; one mark is not.
14. **The stirred W8 oracle is rewritten and the W7 battery's verdict match
    gains one arm; no other mainline check moves.** Reason: the first is the one
    check whose cast includes an outsider acting as family; the second only
    teaches an existing instrument the new verdict's outcome. Both are
    Deviations lines in the PR.
15. **The stake is told on the quest sheet, not the card.** Reason: Non-goals;
    the sheet already carries the per-dreamer call line it attaches to.
16. **Content keys are objects with `text`/`args`/`when`/`source`, source
    `"variant: DESIGN.md S<n>"`.** Reason: the loader accepts either shape; the
    object shape is what the rest of `lines.json` uses, and the source field is
    how a reader tells variant words from port words.
17. **The fork renames `[keifu]` to `[keifu_x_inheritance_r3v2]` and the
    pictures to `xi-*.png`.** Reason: the spec's "would collide or mislead"
    covers both; `target/verify/` is shared with mainline during `tools/test`.

## Open calls delegated to the implementer

Each with the constraint it must respect.

1. **Exact wrapping and ordering of the new sheet lines inside the dock**, as
   long as the outsider line follows the condition line, the lean note follows
   the three aptitude notes, and BLACK MARKS sits between FEAR and DESTINY. The
   W1 oracle matches a subsequence, so nothing of mainline's order may move.
2. **The unit tests' names and placement** (beside the rule they test, `#[cfg(test)]`
   as mainline does), as long as each is a sentence stating the behaviour and
   G5, G7 and G9's assertions are all made.
3. **Whether `Consequence` carries the family house's name or the line reads it
   off `house`**, as long as `marks::consequence` is the one function the sheet
   and `stain` both call.
4. **If the window runs short:** drop pictures before checks (keep
   `xi-death-marks.png` last of the three), and the `--changed-since` round
   before `mutants/xi.txt`; never drop a decision row's check or weaken a
   mainline check. Say what was dropped in the PR's Deviations.
5. **The PNG names and the three capture stagings' exact pointer paths**, as long
   as each picture shows the line the decision needs and the `capture:` line is
   worded as `docs/api/jidousha-capture.md` gives it (mainline's `capture.rs`
   already does).
6. **VARIANT.md's wording**, within the 40-line cap and naming the six
   constants with their values.
