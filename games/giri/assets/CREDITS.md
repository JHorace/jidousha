# giri — asset credits

Every file in this directory, where it came from, and what its terms are.
DESIGN §7's curation model requires one row per file, and an asset with no row
here is an asset that should not have been committed.

**Nothing here was downloaded.** An agent never fetches third-party art
(provenance and licensing); the owner supplies a library, or a committed script
generates it. `art/import_pack.py` is the only door an owner-supplied file
comes in through, and it refuses to run without a stated licence.

**This is giri's art** (owner, 2026-08-23). The set began as the placeholders
the mockup's grids describe, the owner reviewed the captures and kept it, and
the licence question is therefore closed: every file below is original work of
this repository, generated from committed grids, and there is no third-party
pack pending. A change to how giri looks is a change to `art/sprite_defs.py`.

## Current library

| File | Size | Source | Licence |
|---|---|---|---|
| `portrait_alex.png` | 16x16 | `art/make_art.py` (grids in `art/sprite_defs.py`) | Original work of this repository |
| `portrait_bob.png` | 16x16 | as above | Original work of this repository |
| `portrait_steve.png` | 16x16 | as above | Original work of this repository |
| `portrait_tim.png` | 16x16 | as above | Original work of this repository |
| `quest_cave.png` | 12x12 | as above | Original work of this repository |
| `quest_crypt.png` | 12x12 | as above | Original work of this repository |
| `quest_tower.png` | 12x12 | as above | Original work of this repository |
| `quest_vault.png` | 12x12 | as above | Original work of this repository |
| `icon_flame.png` | 8x8 | as above | Original work of this repository |
| `icon_eye.png` | 8x8 | as above | Original work of this repository |
| `icon_coin.png` | 8x8 | as above | Original work of this repository |
| `icon_skull.png` | 10x10 | as above | Original work of this repository |
| `icon_heart.png` | 8x8 | as above | Original work of this repository |

The grids these are drawn from were authored in the UI/UX design session of
2026-08-23, in the approved interactive mockup, and transcribed into
`art/sprite_defs.py`. `art/make_art.py --check` says whether the committed PNGs
are still what the grids produce.

The role naming still earns its keep with the art settled: a sprite is named by
what it *means*, so a later library — or a hand edit to a grid — replaces files
by name and no code changes (UI.md §9).

## Replacing a file, if that day comes

1. `games/giri/art/import_pack.py --pack <dir> --licence "<terms>" --source "<where>"`
2. The script writes the new PNG under its role name and adds a row here.
3. Check the licence against this repository's visibility **before** the
   commit. A purchased pack whose terms forbid redistribution does not go in a
   public repository at all — not even "temporarily".
