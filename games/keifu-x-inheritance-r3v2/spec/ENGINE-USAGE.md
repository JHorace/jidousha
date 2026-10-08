# Lineage — engine usage

What Lineage needs from its engine, **behaviourally**. The port's engine (Jidousha) only has
to provide these behaviours; how is free. Paths relative to `source/application/game/` unless
they start with `source/`.

Lineage is a turn-based, pointer-driven menu game. It uses almost none of a real-time
engine: no physics, no collision, no animation, no particles, no camera, no events. All game
logic runs on button presses; the only time-based behaviour is the telling's typewriter.

## 1. Frame structure and modes

- One immediate-mode frame: (1) apply any pending scene switch, (2) declare widgets and
  handle input, (3) "simulate" (only the typewriter, and only while no overlay is open),
  (4) draw (`game.jai:77-145`).
- **Scene switches are deferred** to the start of the next frame and reset the UI state
  (hot/active widgets, drags) (`scene/scene.jai:12-27`). A button handler that switches
  scenes therefore finishes its frame in the old scene.
- Overlays (title, pause, guide, family) take input and drawing exclusively while open
  (`game.jai:77-145`).
- Engine modes GAME and DEMO; DEMO (engine demos) is debug-only and irrelevant.

## 2. Session, RNG and reset

- A run is a "session". Starting one: reset all game state, seed the gameplay RNG, call the
  game's `initialize` (`source/core/initialize.jai:70-113`).
- Seed: the monotonic clock at launch, or a command-line `-seed` in developer builds
  (`source/main.jai:57-72`). "Begin another house" requests a reset whose seed is drawn from
  the current gameplay RNG (`source/core/initialize.jai:78-91`).
- RNG API used: a seedable generator object passed explicitly; `uniform [0,1)` float; a
  64-bit draw for reseeding (SPEC §22).
- Needed by the port: one seedable PRNG with uniform floats, plus a way to restart a run
  without restarting the process.

## 3. Rendering

Canvas: fixed 640 x 360 virtual pixels, origin bottom-left, y up, integer-scaled to the
window (`source/platform/window.jai:16-17`). Presentation is free in the port; these are the
primitives the original draws with, for reference:

| Need | Used for | Engine API (original) |
| --- | --- | --- |
| Filled rectangle with optional outline | panels, cards, pips, odds bar | `draw(box, fill, edge)`, `draw_colored_quad` |
| 1-px line | family tree spouse links | `draw_line` |
| Sprite by name, integer scale 1-3, **tint colour override** | every sprite (all are white masks) | `draw_sprite(..., use_color_override, color_override)` (`entity/entities/hero.jai:158-163`) |
| Bitmap text, two fonts (large "monogram" 16 px, small "Kenney Mini" 8 px), left/centre/right alignment, colour | all text | `draw_text(text, x, y, alignment, font, color, progress)` |
| Partial text ("progress" = first N characters) | typewriter | `draw_text(..., progress)` |
| Word wrap to a width; measure text width; measure wrapped size | all paragraphs, paging | `wrap_text`, `text_width`, `measure_text` |
| Clip rectangle stack | cards, sheets, tree | `push_clip/pop_clip` |
| Draw order by z then submission | cards (1), tree (2), dragged card (30), hover notes (40) | `push_z/pop_z` |
| Missing sprite → fallback, no crash | | `sprite_from_name` (returns null; the game skips drawing) |

Text measurement drives **paging** in the telling and turning screens (how many lines fit
on a page) (`lineage/panels.jai:219-248`). The port's paging may differ; only the order of
lines matters.

## 4. Input

The game declares **mouse and touch only** (`source/core/capabilities.jai:10-13`). It never
reads keyboard or gamepad buttons itself except the engine's generic "start" (open/close
pause) and "b" (close pause) (`game.jai:87-98`).

| Need | Used for |
| --- | --- |
| Pointer position in canvas coordinates | hover sheets, hover notes, pointing at lines |
| Is the pointer the active input (vs. directional) | hover notes and hover-to-read only when pointing (`lineage/hover-note.jai:9-13`) |
| Press, hold, release per pointer | buttons, drag and drop |
| Any press anywhere | closing the title screen |

Touch builds get a corner **Pause** button because there is no Start button
(`game.jai:211-221`).

## 5. UI widgets

Immediate-mode widgets keyed by a stable label id (text after `&&` is the id; the visible
text is before it). Needed behaviours:

### 5.1 Button

Click to activate; optional **disabled** state (drawn dimmed, does not activate); "hot"
(hovered) query used to show the hero behind an heir button or tree node
(`scene/scenes/turning.jai:30-37`, `lineage/family-tree.jai:145-152`). Plays the UI click
on activation (§7).

Buttons the game uses: set out / stay home / try the Door; telling next / skip / page
numbers; let the winter pass; turning next / skip / page numbers / heir choices / "No one";
ending: the family / the verdict / begin another house; top bar: how to play / the family;
guide: next page / close; family: back; tree: scroll earlier / later; pause menu rows.

### 5.2 Slot (drag and drop) — the core interaction

`source/core/ui/widgets/slot.jai`, `source/core/ui/slot-group.jai:5-59`,
`lineage/panels.jai:266-298`.

- A slot is a box that may hold one hero. Pressing on an occupied slot **grabs** the hero
  (the slot becomes empty, the hero is "carried" and drawn under the pointer).
- Releasing over any slot **drops**: the carried hero goes there; if that slot held someone,
  they move to the slot the carried hero came from (swap).
- Releasing anywhere else, or losing the pointer, **cancels**: the hero returns to the slot
  they came from.
- One shared "exchange" spans every seat group on a screen (roster, quests, hearth seats), so
  heroes move freely between groups.
- While carrying: the drop target under the pointer is highlighted; hovering a quest card
  previews the odds with the carried hero added (`lineage/quest-card.jai:31-46`).
- Each slot can report "hot" (hovered) to show that hero's sheet.
- Grab and drop each play the UI click.

### 5.3 Slider

Only in the pause menu (sfx and music volume, 0.25 steps). Not gameplay.

### 5.4 Hover note (tooltip)

A boxed paragraph that follows the pointer when it is over a region
(`lineage/hover-note.jai:9-27`), used for rule explanations (renown, Door, aptitude bases,
fear rules, bonds, winter stations, heirloom provenance, legacy promises).

## 6. Save and load

**None.** `save.jai` defines an empty save struct and two functions nothing calls. A run is
lost when the program closes. The engine separately persists **settings** (window scale,
fullscreen, volumes) to a settings file on exit (`source/core/initialize.jai:116-120`); that
is not game state. The port need not save runs (matching the original), though adding it is
a free choice outside the rules.

## 7. Audio (waived for the port)

Lineage ships no sounds or music of its own (`assets/game/sfx/`, `assets/game/music/` are
empty). Cue points, as game events:

| Event | Sound | Source |
| --- | --- | --- |
| Any button activated (except family-tree nodes) | engine `click` | `source/core/ui/widgets/button.jai:49,89` |
| Hero grabbed from a seat | `click` | `source/core/ui/widgets/slot.jai:128` |
| Hero dropped on a seat | `click` | `source/core/ui/widgets/slot.jai:129` |
| Pause closed with Start/B | `click` | `game.jai:91` |
| Pause opened / closed | music paused / resumed over 0.25 s (no music plays) | `modal-screens/pause-menu.jai:7-17` |

No sound marks deaths, births, outcomes or the Door. If the port wants sound, these are
free additions.

## 8. Debug and developer tooling (not ported)

- `debug_print` overlays: entity and event counts, camera (`game.jai:187-190`).
- Editor windows: calendar date with "Advance Day", entity count
  (`source/tools/editor/game/*.jai`). "Advance Day" can move the calendar off the
  summer/winter grid; developer-only.
- **UI scripts** (`-uiscript`, `-uitrace`): the seat labels are deliberately descriptive
  ("The Barrow, Grave goods, Dark, needs Might 9, danger 2, calls Garrick to succeed, seat 1";
  a seated hero's one-line description) so automation can read the board
  (`lineage/quest.jai:72-89`, `entity/entities/hero.jai:110-138`). Useful as a model for the
  port's own playtest automation; not gameplay.
- `-seed N` reproduces a run (developer builds).

## 9. Things the engine provides that the game deliberately does **not** use

Fixed-timestep simulation (nothing is simulated), entities beyond a data holder (heroes do
nothing per frame), events, collision, particles, animation, camera, music, roll tables,
selectors, serialization of game state, data tables, hot reload of game data.
