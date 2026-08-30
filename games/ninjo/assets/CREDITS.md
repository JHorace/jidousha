# ninjo — asset credits

This directory is a verbatim copy of `games/giri/assets/` — the fork
carries giri's curated library whole (ninjo's VARIANT.md; the fork step
of the S1 handoff). Every row below applies file for file; giri's art
tooling rode along under `art/` for the same reason. The original header
follows.

# giri — asset credits

Every file in this directory, where it came from, and what its terms are.
DESIGN §7's curation model requires one row per file, and an asset with no row
here is an asset that should not have been committed.

**Nothing here was downloaded.** An agent never fetches third-party art
(provenance and licensing); the owner supplies a library, or a committed script
generates it. `art/import_pack.py` is the only door an owner-supplied file
comes in through, and it refuses to run without a stated licence.

**This is giri's art** (owner, 2026-08-23). Twelve of the thirteen slots are a
**curated subset of the owner's Kenney packs**, chosen sprite by sprite from
contact sheets and named for the roles they fill; the thirteenth, the infamy
eye, is still the generated icon, because no eye glyph exists in any of the
packs. A change to how giri looks is now a change to `art/kenney-manifest.json`
(which pack region fills which role) or to `art/sprite_defs.py` (the generated
remainder). Both paths stay in the repository, and the role names mean either
can be swapped without touching code.

**The licence question is closed.** Every pack is Creative Commons Zero 1.0 —
quoted from each pack's own `License.txt`, which states "free to use in
personal, educational and commercial projects" — so no obligation attaches to
any file here. Kenney *requests* that whole packs not be redistributed. That is
not a licence term, and this repository honours it by construction: only the
individually chosen sprites below are committed, and the contact sheets that
made choosing possible are written to `target/` and never committed, because a
whole pack rearranged into sheets is still the whole pack.

The pack directories themselves live on the owner's machine and have never been
in this repository.

## Current library — imported

Each row is a **curated subset**: one individually chosen sprite, named for
the role it fills. No pack is redistributed here, whole or rearranged.

| File | Size | Pack | Source | Licence | What it is |
|---|---|---|---|---|---|
| `portrait_alex.png` | 16x16 | Tiny Dungeon (1.0) | https://kenney.nl | CC0 1.0 | A wizard: purple pointed hat and robe, long white beard. |
| `portrait_bob.png` | 16x16 | Tiny Dungeon (1.0) | https://kenney.nl | CC0 1.0 | A horned-helmed warrior with a heavy brown beard. |
| `portrait_steve.png` | 16x16 | Tiny Dungeon (1.0) | https://kenney.nl | CC0 1.0 | A bearded ranger in a green headband. |
| `portrait_tim.png` | 16x16 | Tiny Dungeon (1.0) | https://kenney.nl | CC0 1.0 | A knight in a closed grey helm, face fully hidden. |
| `quest_cave.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A cave mouth: a red-brown hillside with a dark arch cut into it. |
| `quest_crypt.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A rounded grey headstone with two carved lines, standing on a base. |
| `quest_tower.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A squat grey tower with three battlements and a dark doorway. |
| `quest_vault.png` | 16x16 | Tiny Dungeon (1.0) | https://kenney.nl | CC0 1.0 | An open chest showing a pale gold interior. |
| `icon_flame.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A flame: red outer tongues over a yellow core. |
| `icon_coin.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A thick gold coin seen face-on, darker rim at the lower left. |
| `icon_skull.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A bone-white skull, two dark sockets, jaw implied by one notch. |
| `icon_heart.png` | 8x8 | Micro Roguelike (1.3) | https://kenney.nl | CC0 1.0 | A filled red heart, flat -- no highlight. |

## Current library — generated

Slots no pack filled. Original work of this repository, written by
`art/make_art.py` from the grids in `art/sprite_defs.py`.

| File | Source | Licence |
|---|---|---|
| `icon_eye.png` | `art/sprite_defs.py` (this repository) | original work |

The slots are the same whichever way a file arrived (UI.md §9): the role is
the contract, not the picture.

## Replacing a file

From a pack the manifest already knows, changing which sprite fills a role is
one line — edit `chosen` in `art/kenney-manifest.json`, then:

```
art/extract.py --packs <dir>              # cuts the picks, role-named, into target/
art/import_pack.py --pack target/giri-art/staged \
    --provenance target/giri-art/staged/provenance.json \
    --licence "CC0 1.0" --source "https://kenney.nl" --confirm-terms
```

From a pack nothing has looked at yet, see it first — `art/contact_sheet.py`
renders indexed sheets into `target/`, `art/role_sheet.py` renders the
shortlist for one role — then classify what you used into the manifest and run
the two commands above. Contact sheets are never committed.

Whichever route: check the licence against this repository's visibility
**before** the commit. Art that may not be redistributed does not go in a
repository that redistributes it — not even "temporarily".
