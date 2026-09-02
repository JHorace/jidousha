#!/usr/bin/env python3
"""games/ninjo/art/picks_sheet.py — the one sheet that gets committed.

`contact_sheet.py` renders a whole pack and `role_sheet.py` renders a role's
shortlist; both write under `target/` and are never committed, because a pack
rearranged into sheets is still the pack. **This renders the picks** — the
curated subset that is already in `assets/`, and nothing else — so committing it
is inside the curation model rather than an exception to it.

It exists because the owner approves from a tablet, on GitHub, away from the
machine that holds the packs (CAST.md §9's one-session dispensation). A veto is
one line: edit `chosen` in `kenney-manifest.json` and any later session re-runs
`extract.py` and `import_pack.py`.

**Every picture is shown at the size it is actually drawn, and again at four
times that**, because the two questions are different ones. At the drawn size a
portrait has to be tellable from the other nine on the map and a chip has to read
at all; at 4x you can see what the picture *is*. A sheet that only showed 4x
would pass art that vanishes at the size the game uses.

Portraits get three bands rather than two: 1x, which is the texel-level
comparison the distinctness criterion is stated at; 2x, which is what the map and
the party strip actually draw (32 units at 16 texels, `layout::HOME`); and 4x.
Chips get two: the 16-unit chip (`attention::CHIP`, 8 texels at scale 2) and 4x.

Portraits sit on the map's ground colour and chips on the panel colour — the
grounds they are drawn on (UI.md §2), never white.

Usage:
  art/picks_sheet.py [--out <png>]

Exit codes: 0 written · 1 a role's file is missing or unreadable · 2 the script
could not run.

Key functions: `band`, `compose_sheet`, `main`.
Depends on: `contact_sheet.py`, `pack_reader.py`, and the standard library only.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ASSETS = HERE.parent / "assets"
REPO = HERE.parent.parent.parent
sys.path.insert(0, str(HERE))

from contact_sheet import BORDER, DIGIT_HEIGHT, GROUND, LABEL, PANEL, Canvas, png  # noqa: E402
from pack_reader import read_png  # noqa: E402

GOLD = (0xE0, 0xB3, 0x4A, 255)
INK = (0xE8, 0xDD, 0xC4, 255)

# The roster, in CAST.md §4's order: the four wave-0b founders, then the six
# this session picked. `new` is what the gold label marks.
CAST = [
    ("bob", False), ("steve", False), ("alex", False), ("tim", False),
    ("rin", True), ("goro", True), ("hana", True), ("ludo", True),
    ("ines", True), ("odd", True),
]

# CAST.md §3's two families, in its order. The blank is the gap drawn between
# them, so the two families are two groups on the sheet as well as in the data.
CHIPS = ["fight", "labor", "scout", "craft", None,
         "indebted", "renown", "caring", "restless", "maker"]

CELL = 76          # one column, wide enough for a 16-texel portrait at 4x
PAD = 6
LABEL_BAND = DIGIT_HEIGHT + 5
MARGIN = 12


def load(role: str):
    """One role's committed picture, or a ValueError naming the file."""
    path = ASSETS / f"{role}.png"
    if not path.is_file():
        raise ValueError(f"{path.relative_to(REPO)} does not exist")
    return read_png(path)


def band(canvas: Canvas, top: int, items, scale: int, fill) -> int:
    """One row of cells at `scale`, each labelled; returns the row's height.

    `items` is (label, image, highlight) or None for a spacer column, which is
    how the two chip families are separated without a caption between them.
    """
    art_height = max(image.height for _, image, _ in (i for i in items if i)) * scale
    height = art_height + LABEL_BAND + PAD * 2
    for column, item in enumerate(items):
        if item is None:
            continue
        label, image, highlight = item
        x = MARGIN + column * CELL
        canvas.rect(x, top, CELL - 2, height - 2, fill)
        canvas.rect(x, top, CELL - 2, 1, BORDER)
        canvas.rect(x, top + height - 3, CELL - 2, 1, BORDER)
        canvas.rect(x, top, 1, height - 2, BORDER)
        canvas.rect(x + CELL - 3, top, 1, height - 2, BORDER)
        canvas.blit_scaled(
            image,
            x + (CELL - 2 - image.width * scale) // 2,
            top + PAD + (art_height - image.height * scale) // 2,
            scale,
        )
        canvas.text(label, x + PAD, top + PAD + art_height + 4, GOLD if highlight else LABEL)
    return height


def compose_sheet() -> Canvas:
    """The whole sheet: the ten at 1x, 2x and 4x, then the nine at chip size and 4x."""
    portraits = [(name, load(f"portrait_{name}"), new) for name, new in CAST]
    chips = [None if t is None else (t, load(f"icon_{t}"), True) for t in CHIPS]
    width = MARGIN * 2 + CELL * len(CAST)

    def lay(canvas: Canvas) -> int:
        """Draw the whole sheet onto `canvas`; return the bottom it reached.

        One function, run twice — once on a canvas tall enough for anything to
        find the height, then on the real one. Two hand-kept passes is how a
        sheet ends up with its last line half off the bottom.
        """
        canvas.text("NINJO - THE FOUNDING CAST - PICKS 2026-09", MARGIN, MARGIN, INK)
        y = MARGIN + 20
        canvas.text(
            "PORTRAIT ROLES - 16 TEXELS - 1X THEN 2X (THE MAP TOKEN) THEN 4X"
            " - GOLD LABEL IS NEW",
            MARGIN, y - 8, LABEL,
        )
        y += 6
        for scale in (1, 2, 4):
            y += band(canvas, y, portraits, scale, GROUND) + 4
        y += 18
        canvas.text(
            "TRAIT CHIPS AT THE 16 UNIT CHIP - 8 TEXELS AT SCALE 2 - PANEL GROUND",
            MARGIN, y - 8, LABEL,
        )
        y += 6
        for scale in (2, 8):
            y += band(canvas, y, chips, scale, PANEL) + 4
        y += 4
        canvas.text(
            "APTITUDES LEFT - MOTIVATORS RIGHT"
            " - VETO IS ONE LINE IN ART KENNEY-MANIFEST JSON",
            MARGIN, y, LABEL,
        )
        return y + DIGIT_HEIGHT

    canvas = Canvas(width, lay(Canvas(width, 4000, GROUND)) + MARGIN, GROUND)
    lay(canvas)
    return canvas


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(add_help=True, description=__doc__)
    parser.add_argument("--out", default=None, help="default: art/picks/cast-2026-09.png")
    args = parser.parse_args(argv[1:])

    out = Path(args.out).resolve() if args.out else HERE / "picks" / "cast-2026-09.png"
    try:
        canvas = compose_sheet()
    except (ValueError, OSError) as error:
        print(f"[ninjo-art] the picks sheet could not be composed\n  {error}")
        print("  likely cause: a role in CAST or CHIPS has no file in assets/")
        print("  fix: run art/extract.py then art/import_pack.py, then this again")
        return 1
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(png(canvas))
    print(f"[ninjo-art] {out.relative_to(REPO)}  {canvas.width}x{canvas.height}")
    print("  look at it at both scales before calling this done, then put it in the PR")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Exception as error:  # noqa: BLE001 - a tool reports rather than traces
        print(f"[ninjo-art] the picks sheet could not be written\n  {error}")
        sys.exit(2)
