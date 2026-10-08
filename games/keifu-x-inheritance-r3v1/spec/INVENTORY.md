# Lineage — repository inventory

Branch `claude/lineage-spec-extraction-1ub4mh` at `f530f3a` ("Lineage M3: build notes, the
run table, and where this stands"). The repository is the **Dayne** engine (Jai) with the
Lineage prototype built on top of it. Game code and engine code are cleanly separable: the
game is everything under `source/application/game/` plus `assets/game/` and two engine
settings it chose (canvas size and input capabilities). Everything else is engine, tooling,
demos or publishing.

Total game source: 48 files, about 390 KB of Jai.

## 1. Game code — `source/application/game/`

### 1.1 Shell and baseline (from the engine's prototype kit, adapted)

| File | Purpose |
| --- | --- |
| `game.jai` | The `Game` struct (all run state), per-frame input/simulate/draw for GAME mode, modal routing, run initialization (`found_household`, first summer). |
| `calendar.jai` | Date/season calendar (28-day months as seasons, 4 per year); `advance_calendar_to_season`. Holidays and day-advance are kit leftovers used only by the editor window. |
| `requirement.jai` | `Requirement(Subject)` predicate trees (PREDICATE / ALL / ANY) used by dream stages. |
| `save.jai` | Empty `Save_Data` and `save_game`/`load_game` — **never called** (no saving). |
| `event/event.jai` | Entity event queue (only `BUMP`); no game code emits events. Kit leftover. |
| `entity/entity.jai`, `entity/entity-discovery.jai` | Compile-time entity discovery and dispatch (kit machinery). |
| `entity/entities/hero.jai` | `Hero` — the only entity: all hero state, phase/aptitude helpers, the hero card drawing, the one-line hero description used as a slot label. |
| `scene/scene.jai`, `scene/scene-discovery.jai` | Compile-time scene discovery/dispatch; `switch_scene` takes effect next frame. |

### 1.2 Scenes — `scene/scenes/`

| File | Purpose |
| --- | --- |
| `summer.jai` | The board: roster, yard, quest cards, sheets, drag-and-drop seating, "Set out". |
| `telling.jai` | Paged report of the summer (prologue, quest pages with typewriter story, meanwhile). |
| `winter.jai` | The hearth: ten winter seats plus hall and yard, previews, "Let the winter pass". |
| `turning.jai` | Paged turn of the year; heir choice buttons on death pages. |
| `ending.jai` | Door verdict / house-closed verdict, family tree, "Begin another house"; final epitaphs. |

### 1.3 Modal screens — `modal-screens/`

| File | Purpose |
| --- | --- |
| `modal-screens.jai` | Load list. |
| `title-screen.jai` | Title and subtitle; release builds only. |
| `pause-menu.jai` | Engine settings menu (resume, scale, fullscreen, volumes, demos in debug, quit). |
| `guide.jai` | "How to play": 16 authored entries on 2 pages. |
| `family.jai` | The family tree as an overlay during play. |

### 1.4 Rules — `lineage/`

| File | Purpose |
| --- | --- |
| `details.jai` | Load list. |
| `lore.jai` | Aptitudes, tags, places, phases and their adjustments, vocations, pronouns, outcomes, colours. |
| `chance.jai` | RNG primitives (`roll_between`, `roll_chance`, fresh/unclaimed index) and dice constants. |
| `writing.jai` | Twelve line pools, trouble lines, tag nouns, year/age/party tellings. |
| `fear.jai` | Fear penalty, dread, courage, conquering, breaking, facing a fear. |
| `destiny.jai` | The ten destinies (lore text), speaking, shields and claims. |
| `bond.jai` | Bond kinds/ranks/power/grief, forming, sharing the road, dream rivals, grief. |
| `deed.jai` | Deed kinds and records; heirloom struct and power. |
| `dream.jai` | Dreams, stages, predicates, moments, progress, dream calls. |
| `legacy.jai` | Legacies (heirlooms, tales, blessings), heir-of-the-blood rule, house tales. |
| `ghost.jai` | Ghosts of unfinished dreams and their quests. |
| `epitaph.jai` | Epitaph wording, parts and composition. |
| `quest.jai` | Quest struct, stakes formula, trouble, forecast/odds, likely parties. |
| `tale.jai` | Summer resolution: rewards, wounds, deaths, mending, burning, crowns, dream witnessing. |
| `hearth.jai` | Winter: seating, lessons, courtship, tales, rest. |
| `passage.jai` | Turn of the year: ageing, old age, death pages, heirs, births, comings of age, wanderers. |
| `household.jai` | Roster/limits, `current_year`, household gathering, the authored founding household. |
| `door.jai` | The Sealed Door: locks, best-four outlook, resolution, prologue text. |
| `hover-note.jai` | Hover tooltip drawing (presentation). |
| `panels.jai` | Screen layout, text cursor, paging, seats, top bar (presentation + the seat drag glue). |
| `sheet.jai` | Hero sheet (presentation). |
| `quest-card.jai` | Quest card, quest sheet, Door card and sheet, drag preview (presentation). |
| `family-tree.jai` | Family tree layout and remembrance panel (presentation). |

### 1.5 Generation — `generation/`

| File | Purpose |
| --- | --- |
| `generation.jai` | Generation context (hero + quest). |
| `hero-context/hero-context.jai` | Name/house bags, wanderers, newborns, coming of age (calling, dream, destiny). |
| `hero-context/templates-hero.jai` | Name pools, houses, wanderer dreams, ages, standings, inheritance chances. |
| `quest-context/quest-context.jai` | Board planning, reading, choosing, easing; quest generation. |
| `quest-context/templates-quest.jai` | The 24 quest templates (four per place). |

### 1.6 Game hooks into the engine

| File | What the game set |
| --- | --- |
| `source/platform/window.jai:16-17` | Canvas 640 x 360. |
| `source/core/capabilities.jai:10-15` | Game capabilities: mouse and touch (no keyboard, no gamepad); legibility scale 3. |
| `build.jai:40` | Program title "Lineage". |
| `source/tools/editor/game/calendar.jai`, `game-inspector.jai` | Debug editor windows (calendar date, entity count). Developer tools only. |

## 2. Engine code (not ported; behaviour summarised in ENGINE-USAGE.md)

| Area | Files | Purpose |
| --- | --- | --- |
| Entry | `source/main.jai` | Command line (`-seed`, `-uiscript`, `-automation`, ...), frame loop. |
| Core | `source/core/{core,state,initialize,mode,memory,log,render,capabilities,geometry}.jai` | Program state, session init/reset and seeding, modes, memory regions, logging. |
| Distribution | `source/core/distribution/{choice,roll-table,selector}.jai` | Bag draws (used), weighted tables and selectors (unused by the game). |
| UI | `source/core/ui/*.jai`, `source/core/ui/widgets/*.jai` | Immediate-mode UI: button, slot (drag-and-drop), slider, list, panel, text box/input, tooltip; slot groups; layout cuts. |
| Graphics | `source/core/graphics/*.jai`, `backend/{opengl,metal,webgpu}/*.jai` | Batch renderer, sprites, text, colours, particles, fades, backends. |
| Text/fonts | `source/core/fonts/*.jai`, `source/core/graphics/text.jai` | Baked bitmap fonts (monogram 16 px "ENGINE_FONT", Kenney Mini 8 px "ENGINE_SMALL_FONT"). |
| Atlas | `source/core/atlas/*.jai` | Sprite atlas baking. |
| Audio | `source/core/audio/audio.jai` | SFX and music mixer. |
| Files | `source/core/files/*.jai`, `source/core/serialization/*.jai` | Assets/packages, settings file, user data folder, reflection serializer, tables. |
| Collision, animation | `source/core/collision/*`, `source/core/animation/*` | Unused by Lineage. |
| Platform | `source/platform/**` | Window, input (desktop, iOS, web, touch), frame timing. |
| Tools | `source/tools/**` | Editor (ImGui), profiler, screenshots, ui-script harness, debug overlays. |
| Publishing | `source/publishing/**`, `publishing/**`, `build.jai` | Build, packaging, icons, itch, iOS/macOS/web/Windows bundles. |
| Demos | `source/application/demo/*.jai`, `assets/demo/**`, `data/demo/traits.txt` | Engine demos reachable from the pause menu in debug builds only. Not part of the game. |
| Vendored modules | `modules/Enumerated_Array`, `modules/ctags` | Third-party Jai modules. |
| Docs | `.claude/**` | Engine docs and agent skills. |

## 3. Data files

| File | Purpose | Game? |
| --- | --- | --- |
| `data/schema_versions.txt` | Autogenerated serializer schema hashes. | No |
| `data/demo/traits.txt` | Content-demo sample table. | No |
| `data/game/.gitkeep` | Empty; Lineage reads no data files. | — |
| `assets/metadata/font_sizes.txt` | Font bake sizes (monogram 16, Kenney Mini 8, demo fonts). | Engine |
| `assets/*/metadata/animation_sheets.txt` | Sprite strip slicing (engine cursor, gamepad, keyboard, panel). | Engine |
| `PROTOTYPE.md` | The design brief and build notes for Lineage (see FINDINGS.md). | Doc |

**All Lineage content is baked into Jai source**: names, quests, dreams, destinies, lines,
the founding household. There are no game data files. Everything is extracted to
`port-spec/content/`.

## 4. Assets

All game sprites are 16 x 16 RGBA PNG holding a single near-white colour (249, 250, 251) on
transparent pixels (verified for all 20 used sprites), drawn **tinted** by the game (place tints, hero tints, heirloom colour) and scaled 1x-3x. They are
referenced **by name string** (file name without `.png`). Engine assets used by the game:
the cursor, `panel.png` (9-slice panel), `placeholder.png` (fallback), `click.wav` (UI
click), and the two fonts.

### 4.1 Game sprites that are used (`assets/game/sprites/`)

| Sprite | Size | Used for | Referenced at |
| --- | --- | --- | --- |
| `dungeon-cemetary` | 16x16 | The Barrow | `lineage/lore.jai:60` |
| `dungeon-sea` | 16x16 | The Drowned Coast | `lineage/lore.jai:61` |
| `dungeon-pyramid` | 16x16 | The High Pass | `lineage/lore.jai:62` |
| `dungeon-campout` | 16x16 | Emberfall | `lineage/lore.jai:63` |
| `dungeon-palace` | 16x16 | The King's Court | `lineage/lore.jai:64` |
| `dungeon-nest` | 16x16 | The Deepwood | `lineage/lore.jai:65` |
| `dungeon-gate` | 16x16 | The Sealed Door (also the top bar, the verdict) | `lineage/lore.jai:66` |
| `character-knight` | 16x16 | Knight | `lineage/lore.jai:176` |
| `character-warrior` | 16x16 | Warrior | `lineage/lore.jai:177` |
| `character-guard` | 16x16 | Ranger | `lineage/lore.jai:178` |
| `character-scholar` | 16x16 | Scholar | `lineage/lore.jai:179` |
| `character-priest` | 16x16 | Priest | `lineage/lore.jai:180` |
| `character-sage` | 16x16 | Sage | `lineage/lore.jai:181` |
| `character-grandpa` | 16x16 | Elder Knight/Warrior/Ranger | `lineage/lore.jai:176-178` |
| `character-hermit` | 16x16 | Elder Scholar/Priest/Sage | `lineage/lore.jai:179-181` |
| `character-boy` | 16x16 | Any child | `lineage/lore.jai:184` |
| `reward-sword` | 16x16 | Thornfall; forged blades | `lineage/household.jai:94`, `lineage/legacy.jai:206` |
| `reward-guidebook` | 16x16 | Road-book heirloom | `lineage/legacy.jai:212` |
| `reward-ring` | 16x16 | Cradle-ring heirloom | `lineage/legacy.jai:218` |
| `skull` | 16x16 | Death page portrait | `scene/scenes/turning.jai:231` |

Sprite choice for a hero (`entity/entities/hero.jai:97-104`): child → `character-boy`; elder →
the vocation's elder sprite; otherwise the vocation sprite.

### 4.2 Game sprites never referenced

All 16x16. `ghost`, `hourglass`; characters `barbarian, bear, brute, butcher, champion, chef,
croc, default, demolitionist, devil, dog, farmer, gentleman, gnome, goblin, gremlin, hero,
horse, juggernaut, king, maiden, ninja, pig, rat, selkie, skeleton, spelunker, thief`;
dungeons `arcade, bank, barangrill, bath, bathroom, belfry, camp, castle, catacombs, cellar,
chasm, cone, default, desert, dungeon, fort, foyer, hut, inn, library, mines, outpost, pit,
reliquary, sewer, shrine, spaceship, temple, well, witch`; rewards `amulet, anger,
armor, backpack, bone, boots, bracer, buckler, character, cheese, chest, cloak, coat, crown,
crystal-1..3, default, delight, despair, gloves, hat, hock, hood, joy, loaf, map, phial,
pickaxe, potion-1, potion-2, rosary, sack, scroll, shovel, sombrero, sparkles`.

These are kit stock. `ghost` and `crown` look like they were meant for ghost quests and the
crown but are unused.

### 4.3 Audio and music

`assets/game/sfx/` and `assets/game/music/` are empty. The only sound the game makes is the
engine's `click` on UI interactions (ENGINE-USAGE.md §6). No music is started by the game.

### 4.4 Other assets

Demo fonts, music, sfx, particles and sprites under `assets/demo/` belong to the engine
demos. `publishing/icon.png` (and the iOS icon set generated from it) is the app icon.

## 5. Dead or unreachable code and data (in the game)

| What | Where | Note |
| --- | --- | --- |
| Saving/loading | `save.jai` | Defined, never called; empty payload. |
| Event system | `event/event.jai`; `Hero.react/update/draw` | No events emitted; hero entity callbacks are empty. Heroes are drawn by the scenes, not the entity loop. |
| Calendar holidays, day advance, days remaining | `calendar.jai:7-14,43-45,74-91` | Used only by the debug editor window. |
| `Deed.telling` | `lineage/deed.jai:34`, every `record_deed` call | Written, never displayed or read (OQ-28). Deed kinds WOUNDED, SURVIVED_DISASTER, DREAM_STEP, DREAM_FULFILLED, WED, CHILD_BORN, TAUGHT, TOLD_THE_TALE, CAME_OF_AGE, MENDED, CROWNED, BEFRIENDED, LEFT_LEGACY, INHERITED, TOOK_UP_DREAM, LAID_GHOST are recorded but never read. Read kinds: FIRST_QUEST, TRIUMPH, ARRIVED, CONQUERED_FEAR, BROKEN, OPENED_A_LOCK, STOOD_AT_THE_DOOR. |
| `House_Tale.since` | `lineage/legacy.jai:30` | Set, never read. |
| `Bond.since` except for SPOUSE | `lineage/bond.jai:15` | Only the spouse bond's `since` is read (births). |
| Winter actions REST and COURT as dream moments | `lineage/hearth.jai:216,265-266` | Fired, but no dream stage tests them. |
| `Destiny.fulfilled` for OPEN_THE_SEALED_DOOR | `lineage/passage.jai:314` | Never set; the guard is inert (OQ-32). |
| "A quiet summer" telling page | `scene/scenes/telling.jai:171,220-225` | Only reachable with an empty board; boards always hold four quests. |
| `plan_lesson` self-teaching branch for a child | `lineage/hearth.jai:54-58` | Unreachable through the seats (yard and bench paths filter children first). |
| `Requirement` ANY node | `requirement.jai:5,24-26` | No dream uses ANY. |
| `tell_quest_briefly`, `tell_hero_briefly` | `lineage/quest.jai:72-89`, `entity/entities/hero.jai:110-138` | Accessibility/automation labels for seats (read by the ui-script tracer). Not shown as text. |
| Unused kit APIs | `source/core/distribution/roll-table.jai`, `selector.jai`; `take_occupant`, `count_occupants` | Not called by the game. |
