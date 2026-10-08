#!/usr/bin/env python3
"""Compose the picks sheet: each imported role's sprite at 1x and at 6x, labelled.

The owner approves or vetoes the picks from this picture, so it shows exactly the
files in `games/keifu-x-inheritance-r3v1/assets/`, on the game's own dark ground. It borrows the
depot's PNG reader and canvas (`tools/pack_reader.py`, `tools/contact_sheet.py`)
rather than carrying a second copy of either.

Usage: games/keifu-x-inheritance-r3v1/art/picks_sheet.py --depot ../jidousha-assets --out games/keifu-x-inheritance-r3v1/screens/w2-picks.png
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets"
# Role file, the original's sprite name it plays, and who wears it at founding.
PICKS = [
    ("hero_knight", "character-knight", "knights"),
    ("hero_warrior", "character-warrior", "warriors - Brannoc"),
    ("hero_ranger", "character-guard", "rangers - Maren"),
    ("hero_scholar", "character-scholar", "scholars - Ysolde"),
    ("hero_priest", "character-priest", "priests - Odo"),
    ("hero_sage", "character-sage", "sages"),
    ("hero_elder", "character-grandpa", "elder knight warrior ranger - Garrick"),
    ("hero_hermit", "character-hermit", "elder scholar priest sage"),
    ("hero_child", "character-boy", "every child - Pip, Wren"),
    ("heirloom_blade", "reward-sword", "a blade heirloom - Thornfall"),
]
ZOOM = 6
GROUND = (15, 15, 20, 255)
CELL = (28, 28, 36, 255)
INK = (235, 235, 240, 255)
NOTE = (158, 163, 178, 255)


def scaled(image, k, Image):
    out = bytearray(image.width * k * image.height * k * 4)
    for y in range(image.height * k):
        for x in range(image.width * k):
            source = ((y // k) * image.width + x // k) * 4
            target = (y * image.width * k + x) * 4
            out[target : target + 4] = image.pixels[source : source + 4]
    return Image(image.width * k, image.height * k, bytes(out))


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--depot", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    sys.path.insert(0, str(Path(args.depot) / "tools"))
    from contact_sheet import Canvas, png  # noqa: E402 — the depot's, by design
    from pack_reader import Image, read_png  # noqa: E402

    width, row = 620, 16 * ZOOM + 24
    canvas = Canvas(width, 40 + row * len(PICKS), GROUND)
    canvas.text("KEIFU PICKS - TINY DUNGEON (KENNEY, CC0) - 1X AND 6X ON THE GAME'S GROUND", 12, 14, INK)
    for index, (role, original, worn) in enumerate(PICKS):
        image = read_png(ASSETS / f"{role}.png")
        top = 36 + index * row
        canvas.rect(8, top, width - 16, row - 6, CELL)
        canvas.blit(image, 20, top + (row - 6 - image.height) // 2)
        canvas.blit(scaled(image, ZOOM, Image), 52, top + 6)
        x = 52 + 16 * ZOOM + 20
        canvas.text(f"{role}.png", x, top + 20, INK)
        canvas.text(f"plays {original}", x, top + 40, NOTE)
        canvas.text(worn, x, top + 60, NOTE)
    Path(args.out).write_bytes(png(canvas))
    print(f"[picks] {len(PICKS)} roles written to {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
