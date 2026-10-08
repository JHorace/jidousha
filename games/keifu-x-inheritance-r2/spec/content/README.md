# Lineage content

Hand-authored content, extracted **verbatim** from the Jai source into flat JSON. Every string
was copied mechanically from the source literal at the cited line (not retyped), so spelling,
punctuation and spacing (including leading/trailing spaces in fragments) are exact. These
files are the "content-identical" half of the port contract; the rules that assemble them at
runtime are in `../SPEC.md`.

Keys starting with `_` are provenance or notes, not content. `source` / `_source` fields cite
`file:line` relative to `source/application/game/`.

## Format conventions

### Jai print placeholders

Strings that take arguments use the Jai `print` convention, kept verbatim:

- `%` — the next argument, in order.
- `%1`, `%2`, ... — argument N (1-based), may repeat or appear out of order.
- `%%` — **two consecutive arguments** (not an escaped percent). Example: `"% wanted %%, and
  did it."` takes three arguments: pronoun, dream title, and an optional suffix (empty or
  `", as X had before him"`).
- `\%` (a backslash then a percent, stored in JSON as `"\\%"`) — a literal percent sign. Only
  in `ui-text.json`.
- Arguments are described per entry in `args` (lines, epitaph) or in the notes. Unless an
  entry says otherwise, "name" means a hero's first name and "full name" means "Name House".
- Numeric arguments that are constants (for example `CONQUERED_FEAR_BONUS (2)`) are named
  with their value; the value is in `../CONSTANTS.md`.

### Brace placeholders

A few strings use `{...}` placeholders replaced by string substitution, not by `print`:

- Phase notes (`lore.json` `phases[].note`): `{He}` = capitalised subject pronoun, `{his}` =
  possessive pronoun.
- Ghost texts (`ghost.json`): `{name}` = the dead hero's first name, `{task}` = the told
  current stage task of the ghost's dream (SPEC §9.2), `{He}`, `{he}`, `{him}` = the dead
  hero's pronouns. Ghost endings additionally take `%1` = the party's names.

### Rendered vs template

Where the source builds a string at compile time from a constant (`#run sprint(...)`), the
JSON holds the rendered text in the main field and the raw template in a `*_template` field
(destiny dooms/gifts, the Court dream's first task, guide entries).

### Enumerations

Ids are the Jai enum names. Order matters wherever it is used as an index; the canonical
orders are:

- Aptitudes: `MIGHT, WITS, SPIRIT`.
- Tags: `DARK, UNDEAD, WATER, BEASTS, HEIGHTS, COLD, FIRE, CROWDS`.
- Places: `BARROW, DROWNED_COAST, HIGH_PASS, EMBERFALL, KINGS_COURT, DEEPWOOD, SEALED_DOOR`
  (the first six are questing places; board order is this order).
- Phases: `CHILD, YOUTH, PRIME, VETERAN, ELDER`.
- Vocations: `KNIGHT, WARRIOR, RANGER, SCHOLAR, PRIEST, SAGE`.
- Outcomes: `DISASTER, SETBACK, SUCCESS, TRIUMPH` (comparisons "at least" use this order).
- Destinies: `UNSPOKEN, FIRE_WILL_END_YOU, OUTLIVE_THOSE_YOU_LOVE, WEAR_A_CROWN,
  OPEN_THE_SEALED_DOOR, CHILD_WILL_SURPASS_YOU, BREAK_AND_BE_MENDED, DIE_IN_YOUR_BED,
  CARRY_THE_HOUSE, TEACH_A_GREATER`.
- Bonds: `COMPANION, FRIEND, RIVAL, SPOUSE, MENTOR, STUDENT, PARENT, CHILD`.
- Dreams: `SEE_THE_SEA, ROOF_OF_THE_WORLD, FORGE_A_BLADE, WORTHY_STUDENT, AVENGE_THE_LOST,
  KNOWN_AT_COURT, WALK_EVERY_ROAD, QUIET_THE_BARROW, SEE_A_CHILD_GROWN`.
- Winter actions: `REST, TRAIN, TEACH, COURT, MIND_A_CHILD, TELL_THE_TALE`.

## Files

### `lore.json` — the world's fixed vocabulary

| Key | Schema |
| --- | --- |
| `aptitudes[]` | `{id, title}` |
| `tags[]` | `{id, title, noun}` — `noun` is the lower-case phrase used in sentences ("the walking dead") |
| `places[]` | `{id, questing, title, name, sprite, tags[], tint, trouble_line}` — `title` capitalised ("The Barrow"), `name` for mid-sentence ("the Barrow"); `trouble_line` takes `%1` = a `year_counts` entry |
| `phases[]` | `{id, title, telling, note, from_age, adjustment{MIGHT,WITS,SPIRIT}}` — `telling` used as "is <telling> now"; `note` has `{He}`/`{his}` |
| `phase_effect_fragments` | pieces of the phase-effect sentence (SPEC; `lineage/lore.jai:125-157`) |
| `vocations[]` | `{id, title, aptitude, sprite, elder_sprite}` |
| `child_sprite` | sprite for every child |
| `pronouns` | `{HE|SHE: {subject, object, possessive}}` |
| `outcomes[]` | `{id, title}` |
| `seasons[]` | indexed by calendar month (only 1 Summer and 3 Winter are used) |
| `year_counts[]` | indexed by trouble (0..3) for trouble lines |
| `count_words[]`, `count_words_beyond` | 0..12 as words, then "% times" |
| `year_tellings` | before year 1 / after year 25 / "in year %" |
| `age_at_death` | "% was %." (He/She, age) |
| `party_of_no_one`, `name_list_*` | name-list fragments (SPEC §21) |
| `colors` | presentation only |

### `destinies.json`

`destinies[]`: `{id, speakable, prophecy, doom, gift, doom_template?, gift_template?}`.
`speakable_order[]`: the order used by the unclaimed roll (index order matters for nothing but
is kept). `blood_of_prophecy` ("Blood of %: %" = promised-to name, lowered prophecy).
`shielded_bed`, `shielded_fire`: lines when a destiny shields a death roll.

### `bonds.json`

`kinds[]`: `{id, title, pair, rank, power, grief, mirror, shown}` — `pair` is the phrase in
power lines ("% and %, <pair>"); `shown` false only for COMPANION. `gendered_titles` and
`kinship_tellings` are chosen by the pronoun of the **other** hero in the bond.

### `dreams.json`

`dreams[]`: `{id, title, legacy, ghost_place, wanderer_dream, stages[3], ...}`.
Each stage: `{task, goal, requirement}`; a requirement is either
`{predicate, parameter?}` or `{all: [requirement, ...]}`. Predicates and their meaning are in
SPEC §9.1. Special parameter values: `SETUP_PLACE` / `SETUP_TAG` = the dream's setup (only
AVENGE_THE_LOST). Special `ghost_place` values: `SETUP_PLACE`, `WHERE_THE_DREAMER_DIED` (the
place of a questing death not at the Door, else the Barrow). AVENGE_THE_LOST's `title` and two
tasks are templates (`title_argument`, `task_argument`). KNOWN_AT_COURT's first task is
rendered with 4 (`task_template` kept).
Also: `wanderer_dream_order` (index order for the unclaimed roll), `call_tellings`,
`title_pronoun_swap`, `task_object_swap`, `progress_format`, `progress_tellings` (epitaph
"had not yet begun it" etc., indexed by min(current stage, 2)).

### `legacies.json`

`blade_names[]` (in forging order, cycling), `heirlooms` by dream (`name` template, sprite,
aptitude, bonus, provenance template with argument notes), `blessings` by dream (title
template, scope, tag/place, power), `blessing_effects` by scope, `tale_titles` by dream,
`ghost_tale_title`, `promises` by legacy kind (sheet/arrival "If it is ever done..."),
`legacy_nouns` by legacy kind, `heirloom_effect` ("+% % on quests." = bonus, aptitude title).

### `quests.json`

`templates[]` (24, four per place, in source order): `{place, title, aptitude, tags[],
seats_low, seats_high, danger, premise, endings{TRIUMPH,SUCCESS,SETBACK,DISASTER}}`. Endings
take `%1` = the party's names as a name list. `opening_quests[]`: the two titles forced onto
year 1's board.

### `ghost.json`

The ghost quest: `title` ("Lay %'s ghost" = dead first name), `aptitude`, calm `seats` and
`danger`, `premise`, `endings` (brace placeholders + `%1`).

### `door.json`

`years`, `seats`, `danger`, `renown`; `locks[]` in order Might, Wits, Spirit: `{aptitude,
demand, title, name, premise, endings{...}}` — lock endings take `%1` = standing party names
and `%2` = the bearer's name. `verdict_titles[]` and `verdicts[]` indexed by locks opened
(0..3). `closed_title`, `closed_verdict` for a house that closed.

### `writing.json`

`pools{NAME: {lines[], args[]}}` — the twelve line pools. A pick never repeats the previous
pick from the same pool (SPEC §21).

### `names.json`

`names_for_him[32]`, `names_for_her[32]`, `houses[16]` — drawn as bags without replacement,
refilled when empty (SPEC §17.1).

### `wanderers.json`

`age_low`, `age_high`, `standings[]` `{renown_at_least, gift_low, gift_high, other_low,
other_high, telling}`, `dreams[]` (the rollable dreams, in index order),
`teacher_dream_minimum_age`.

### `household.json`

The founding household in **creation order** (`heroes[]`), each with every field the source
sets (unset fields are their defaults: dread 0, renown 0, no heirloom, not wounded/settled,
no scars, no deeds, no burden). `dream.stage` is the current stage index (0-based; earlier
stages count as complete). `bonds[]` in formation order with `{hero, other, kind, since}`
meaning "hero's bond to other has this kind" (the mirror is implied). `born_year`,
`fate_year`, `since` are absolute years (year 1 = the first summer).

### `lines.json`

`lines{key: {text, args[], when, source}}` — every narrative line the rules emit in the
telling and the turning, plus fate tellings, scars and deed tellings. SPEC refers to them as
`lines.<key>`. `deed.*` entries are recorded but never displayed (OQ-28).

### `epitaph.json`

`parts[]`, `frames[3][9]`, `priorities[]`, `sentence_limit`, and `templates{key: {text, args,
when, source}}` for every epitaph sentence. The selection logic is SPEC §20. Argument
shorthand: "He/She" = capitalised subject pronoun, "His/Her" = capitalised possessive,
"subject/object/possessive" = lower-case pronouns, all of the hero the epitaph is about.

### `ui-text.json`

Presentation text: title screen, the 16 guide entries (rendered), top bar, quest card and
sheet, Door card and sheet, hero sheet labels, telling/winter/turning/ending/family texts,
seat notes, lesson excuses (with their wasted-winter lines), courtship verdict notes, button
labels. The port is visually free; whether these words are binding is OQ-25.
