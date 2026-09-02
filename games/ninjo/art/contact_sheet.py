#!/usr/bin/env python3
"""games/giri/art/contact_sheet.py — render a pack so a person can look at it.

Curation has one step no script can take: **somebody has to see the art.** This
renders an owner-supplied pack as indexed contact sheets — every candidate
upscaled, on giri's own ground colour, under the index that names it — so the
screenshot process (UI.md §8) can be pointed at the *inputs* rather than only
at the finished screens.

The index under each sprite is the coordinate the manifest and `import_pack.py`
use: for a directory of PNGs it is the file's stem, and for a sliced tilesheet
it is the tile's row-major number in the pack's own order. Read a sheet, note
the numbers, and the picking is done in the manifest.

**Sheets are never committed.** A whole pack rearranged into sheets is still
the whole pack, and Kenney asks that packs not be redistributed (a request, not
a term of CC0 — but the curation model honours it by construction: only the
individually chosen, role-named sprites are committed). This script therefore
refuses to write anywhere but an untracked directory, and defaults to one under
`target/`.

Usage:
  art/contact_sheet.py --pack <dir> --source <relative path> [--tile W H]
                       [--spacing N] [--margin N] [--out <dir>]
                       [--per-sheet N] [--scale N] [--keep-empty]

`--source` is a directory of individual PNGs (preferred — most Kenney packs
ship one) or a single tilesheet, which needs `--tile` and usually
`--spacing 1`; the pack's own `Tilesheet.txt` states both.

Exit codes: 0 sheets written · 1 the pack or source could not be read · 2 the
script could not run.

Key functions: `digits`, `compose`, `main`.
Depends on: `pack_reader.py` and the Python 3.8+ standard library only.
"""

from __future__ import annotations

import argparse
import struct
import sys
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent.parent
sys.path.insert(0, str(HERE))

from pack_reader import Image, is_empty, load_sources  # noqa: E402

# UI.md §2's colour roles, so a candidate is judged against the ground it will
# actually be drawn on rather than against white.
GROUND = (0x14, 0x12, 0x1D, 255)
PANEL = (0x1E, 0x1B, 0x2B, 255)
BORDER = (0x36, 0x30, 0x50, 255)
LABEL = (0x8D, 0x84, 0xA0, 255)

# A 3x5 digit font, one string per row, so an index can be read off a sheet
# without a font asset existing (DESIGN §7: no font asset is created).
DIGITS = {
    "0": ["111", "101", "101", "101", "111"],
    "1": ["010", "110", "010", "010", "111"],
    "2": ["111", "001", "111", "100", "111"],
    "3": ["111", "001", "111", "001", "111"],
    "4": ["101", "101", "111", "001", "001"],
    "5": ["111", "100", "111", "001", "111"],
    "6": ["111", "100", "111", "101", "111"],
    "7": ["111", "001", "001", "001", "001"],
    "8": ["111", "101", "111", "101", "111"],
    "9": ["111", "101", "111", "001", "111"],
    "-": ["000", "000", "111", "000", "000"],
    "_": ["000", "000", "000", "000", "111"],
    ".": ["000", "000", "000", "000", "010"],
    ":": ["000", "010", "000", "010", "000"],
    "A": ["111", "101", "111", "101", "101"],
    "B": ["110", "101", "110", "101", "110"],
    "C": ["111", "100", "100", "100", "111"],
    "D": ["110", "101", "101", "101", "110"],
    "E": ["111", "100", "111", "100", "111"],
    "F": ["111", "100", "111", "100", "100"],
    "G": ["111", "100", "101", "101", "111"],
    "H": ["101", "101", "111", "101", "101"],
    "I": ["111", "010", "010", "010", "111"],
    "J": ["001", "001", "001", "101", "111"],
    "K": ["101", "101", "110", "101", "101"],
    "L": ["100", "100", "100", "100", "111"],
    "M": ["101", "111", "111", "101", "101"],
    "N": ["110", "101", "101", "101", "101"],
    "O": ["111", "101", "101", "101", "111"],
    "P": ["111", "101", "111", "100", "100"],
    "Q": ["111", "101", "101", "111", "001"],
    "R": ["111", "101", "110", "101", "101"],
    "S": ["111", "100", "111", "001", "111"],
    "T": ["111", "010", "010", "010", "010"],
    "U": ["101", "101", "101", "101", "111"],
    "V": ["101", "101", "101", "101", "010"],
    "W": ["101", "101", "111", "111", "101"],
    "X": ["101", "101", "010", "101", "101"],
    "Y": ["101", "101", "010", "010", "010"],
    "Z": ["111", "001", "010", "100", "111"],
}
DIGIT_WIDTH, DIGIT_HEIGHT = 3, 5


class Canvas:
    """A mutable RGBA8 surface. Small enough that a bytearray is the whole story."""

    def __init__(self, width: int, height: int, fill: "tuple[int, int, int, int]") -> None:
        self.width = width
        self.height = height
        self.pixels = bytearray(bytes(fill) * (width * height))

    def set(self, x: int, y: int, colour: "tuple[int, int, int, int]") -> None:
        if 0 <= x < self.width and 0 <= y < self.height:
            offset = (y * self.width + x) * 4
            self.pixels[offset : offset + 4] = bytes(colour)

    def rect(self, x: int, y: int, width: int, height: int, colour) -> None:
        for row in range(y, y + height):
            for column in range(x, x + width):
                self.set(column, row, colour)

    def blit_scaled(self, image: Image, x: int, y: int, scale: int) -> None:
        """Nearest-neighbour, over the existing background — the engine's sampling.

        Source alpha composites rather than replaces, so a half-transparent
        edge texel reads the way it will read in the game instead of punching a
        hole in the sheet.
        """
        for row in range(image.height):
            for column in range(image.width):
                red, green, blue, alpha = image.pixel(column, row)
                if not alpha:
                    continue
                for dy in range(scale):
                    for dx in range(scale):
                        px, py = x + column * scale + dx, y + row * scale + dy
                        if alpha == 255:
                            self.set(px, py, (red, green, blue, 255))
                        else:
                            offset = (py * self.width + px) * 4
                            if not (0 <= px < self.width and 0 <= py < self.height):
                                continue
                            back = self.pixels[offset : offset + 3]
                            mix = [
                                (channel * alpha + back[i] * (255 - alpha)) // 255
                                for i, channel in enumerate((red, green, blue))
                            ]
                            self.set(px, py, (mix[0], mix[1], mix[2], 255))

    def text(self, label: str, x: int, y: int, colour) -> int:
        """Draw `label` in the 3x5 font; returns the width drawn."""
        cursor = x
        for character in label:
            # Upper and lower case share one glyph: the font is for reading an
            # index off a sheet, not for setting type.
            glyph = DIGITS.get(character.upper())
            if glyph is not None:
                for row, bits in enumerate(glyph):
                    for column, bit in enumerate(bits):
                        if bit == "1":
                            self.set(cursor + column, y + row, colour)
            cursor += DIGIT_WIDTH + 1
        return cursor - x


def png(canvas: Canvas) -> bytes:
    """The canvas as PNG bytes — the same hand-rolled encoder `make_art.py` uses."""
    raw = bytearray()
    stride = canvas.width * 4
    for y in range(canvas.height):
        raw.append(0)
        raw.extend(canvas.pixels[y * stride : (y + 1) * stride])

    def chunk(kind: bytes, body: bytes) -> bytes:
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", canvas.width, canvas.height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 6))
        + chunk(b"IEND", b"")
    )


def compose(entries: "list[tuple[str, Image]]", scale: int, columns: int) -> Canvas:
    """One sheet: a grid of upscaled sprites, each under its index."""
    cell_art = max(image.width for _, image in entries) * scale
    cell_art_height = max(image.height for _, image in entries) * scale
    label_band = DIGIT_HEIGHT + 4
    pad = 6
    cell_width = cell_art + pad * 2
    cell_height = cell_art_height + label_band + pad * 2
    rows = (len(entries) + columns - 1) // columns
    canvas = Canvas(columns * cell_width, rows * cell_height, GROUND)

    for index, (label, image) in enumerate(entries):
        column, row = index % columns, index // columns
        x, y = column * cell_width, row * cell_height
        # A panel behind every cell, so the sprite's own extent is visible and
        # a sprite that does not fill its tile says so.
        canvas.rect(x + 1, y + 1, cell_width - 2, cell_height - 2, PANEL)
        canvas.rect(x + 1, y + 1, cell_width - 2, 1, BORDER)
        canvas.rect(x + 1, y + cell_height - 2, cell_width - 2, 1, BORDER)
        canvas.rect(x + 1, y + 1, 1, cell_height - 2, BORDER)
        canvas.rect(x + cell_width - 2, y + 1, 1, cell_height - 2, BORDER)
        art_x = x + pad + (cell_art - image.width * scale) // 2
        art_y = y + pad
        canvas.blit_scaled(image, art_x, art_y, scale)
        canvas.text(label, x + pad, y + pad + cell_art_height + 3, LABEL)
    return canvas


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(add_help=True, description=__doc__)
    parser.add_argument("--pack", required=True, help="the pack directory the owner supplied")
    parser.add_argument("--source", required=True, help="path within the pack: a dir or a sheet")
    parser.add_argument("--tile", nargs=2, type=int, metavar=("W", "H"))
    parser.add_argument("--spacing", type=int, default=0)
    parser.add_argument("--margin", type=int, default=0)
    parser.add_argument("--out", default=None, help="untracked output dir (default: under target/)")
    parser.add_argument("--per-sheet", type=int, default=100)
    parser.add_argument("--scale", type=int, default=4)
    parser.add_argument("--columns", type=int, default=10)
    parser.add_argument("--keep-empty", action="store_true", help="keep fully transparent cells")
    parser.add_argument("--name", default=None, help="stem for the written sheets")
    parser.add_argument(
        "--only",
        default=None,
        metavar="A-B,C",
        help="render just these cells, by trailing index: ranges and singles",
    )
    args = parser.parse_args(argv[1:])

    pack = Path(args.pack).expanduser().resolve()
    if not pack.is_dir():
        print(f"[giri-art] no pack at {pack}")
        print("  likely cause: the path is wrong, or the pack has not been unpacked")
        print("  fix: pass --pack <the directory the owner gave you>")
        return 1
    source = (pack / args.source).resolve()

    name = args.name or source.stem.replace(" ", "-").lower()
    out = Path(args.out).expanduser().resolve() if args.out else REPO / "target" / "ninjo-art" / name
    # The one hard rule: a pack rendered into sheets is still the pack, so the
    # sheets cannot land anywhere git will pick them up.
    try:
        inside = out.relative_to(REPO)
        if inside.parts and inside.parts[0] not in ("target", "dist"):
            print(f"[giri-art] {out} is inside the repository and is not ignored")
            print("  likely cause: contact sheets are a whole pack rearranged; they are never")
            print("    committed (Kenney asks that packs not be redistributed)")
            print("  fix: leave --out unset, or point it under target/")
            return 1
    except ValueError:
        pass  # Outside the repository entirely, which is fine.
    out.mkdir(parents=True, exist_ok=True)

    tile = tuple(args.tile) if args.tile else None
    try:
        entries = load_sources(source, tile, args.spacing, args.margin)
    except (ValueError, OSError) as error:
        print(f"[giri-art] {error}")
        print("  likely cause: --source names neither a directory of PNGs nor a readable sheet")
        print(f"  fix: check the path against `ls {pack}`")
        return 1

    if args.only:
        # Labels end in their index either way — `tile_0136` and `136` both
        # select on 136 — so a range is stated once and means the same thing
        # for a directory of PNGs and for a sliced sheet.
        wanted: "set[int]" = set()
        for piece in args.only.split(","):
            low, _, high = piece.strip().partition("-")
            wanted.update(range(int(low), int(high or low) + 1))
        entries = [
            (label, image)
            for label, image in entries
            if any(character.isdigit() for character in label)
            and int("".join(character for character in label if character.isdigit())) in wanted
        ]

    total = len(entries)
    if not args.keep_empty:
        entries = [(label, image) for label, image in entries if not is_empty(image)]
    if not entries:
        print(f"[giri-art] {source} yielded no non-empty sprites of {total}")
        return 1

    sheets = [
        entries[start : start + args.per_sheet] for start in range(0, len(entries), args.per_sheet)
    ]
    print(f"[giri-art] {source.relative_to(pack)}: {total} cell(s), {len(entries)} non-empty")
    for number, batch in enumerate(sheets):
        canvas = compose(batch, args.scale, args.columns)
        path = out / f"{name}-{number:02d}.png"
        path.write_bytes(png(canvas))
        first, last = batch[0][0], batch[-1][0]
        print(f"  {path.relative_to(REPO)}  {canvas.width}x{canvas.height}  [{first} .. {last}]")
    print(f"[giri-art] {len(sheets)} sheet(s) in {out} — untracked; look at them, then classify")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Exception as error:  # noqa: BLE001 - a tool reports rather than traces
        print(f"[giri-art] the contact sheets could not be rendered\n  {error}")
        sys.exit(2)
