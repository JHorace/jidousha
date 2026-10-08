#!/usr/bin/env python3
"""Import sprites from the asset depot into games/keifu-x-inheritance-r3v1/assets/, role-named, credited.

The depot's README (jidousha-assets §5) names the import flow as
`games/<name>/art/import_pack.py`, and the only such script lives inside a game
folder this port is fenced from. This is keifu's own minimal step, written from
the README's description of that flow rather than from its code:

- every sprite is renamed to the **role** it plays (lowercase snake_case);
- anything that is not a non-interlaced PNG inside the size envelope is refused;
- it will not run without a stated licence and source, and it writes both into
  `games/keifu-x-inheritance-r3v1/assets/CREDITS.md`, one row per file;
- it writes each sprite as 8-bit RGBA, texel for texel — no resizing, no
  recolouring. The engine decodes 8-bit PNGs only ("Four bit depth is not
  supported; re-export as 8-bit"), and Kenney's packs ship palette PNGs at 1, 2
  and 4 bits, so the step re-encodes. The decoder is the depot's own
  (`tools/pack_reader.py`, found by walking up from `--pack`); the encoder is
  copied from the depot's `tools/contact_sheet.py` (`png`).

Usage (from the repository root, the depot checked out sparsely beside it):

    games/keifu-x-inheritance-r3v1/art/import_sprites.py \\
      --pack "../jidousha-assets/kenney-all-in-1-3.7.0/2d/Tiny Dungeon" \\
      --licence "CC0 1.0 Universal (public domain dedication)" \\
      --source "Kenney Game Assets All-in-1 3.7.0 — Tiny Dungeon — https://kenney.nl" \\
      --confirm-terms \\
      --role hero_knight=Tiles/tile_0096.png --role ...

Stdlib only, like every tool in this repository.
"""

from __future__ import annotations

import argparse
import re
import struct
import sys
import zlib
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets"
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
# The size envelope: pixel-art sprites, never a sheet or a background.
SMALLEST, LARGEST = 8, 64
ROLE = re.compile(r"^[a-z][a-z0-9_]*$")
CREDITS_HEAD = """# Keifu — art credits

Every file in this directory, where it came from, and on what terms. Written by
`games/keifu-x-inheritance-r3v1/art/import_sprites.py`; one row per file. The art is presentation
only: the original Lineage's art is not reused (owner's decision, 2026-10-01).

| File | Role | Source file | Source | Licence |
| --- | --- | --- | --- | --- |
"""


def depot_reader(pack: Path):
    """The depot's PNG decoder, from the `tools/` above `pack`."""
    for parent in [pack, *pack.parents]:
        if (parent / "tools" / "pack_reader.py").is_file():
            sys.path.insert(0, str(parent / "tools"))
            import pack_reader  # noqa: E402 — found at run time, by design

            return pack_reader.read_png
    raise SystemExit(
        f"[import] no tools/pack_reader.py above {pack}\n"
        "  likely cause: --pack is not inside a sparse checkout of the depot\n"
        '  fix: git sparse-checkout add "/tools/" in the depot'
    )


def rgba8_png(width: int, height: int, pixels: bytes) -> bytes:
    """RGBA8 texels as a PNG (copied from the depot's tools/contact_sheet.py `png`)."""
    raw = bytearray()
    stride = width * 4
    for y in range(height):
        raw.append(0)
        raw.extend(pixels[y * stride : (y + 1) * stride])

    def chunk(kind: bytes, body: bytes) -> bytes:
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def png_size(data: bytes, name: str) -> "tuple[int, int]":
    """The width and height of a PNG, refusing anything else."""
    if not data.startswith(PNG_MAGIC) or data[12:16] != b"IHDR":
        raise SystemExit(f"[import] {name} is not a PNG\n  fix: pick a .png from the pack")
    width, height, _depth, _colour, _comp, _filter, interlace = struct.unpack(
        ">IIBBBBB", data[16:29]
    )
    if interlace:
        raise SystemExit(f"[import] {name} is interlaced\n  fix: pick a non-interlaced PNG")
    if not (SMALLEST <= width <= LARGEST and SMALLEST <= height <= LARGEST):
        raise SystemExit(
            f"[import] {name} is {width}x{height}, outside the {SMALLEST}..{LARGEST} px envelope\n"
            "  likely cause: a tilesheet or a preview, not a sprite\n"
            "  fix: pick one tile from the pack's Tiles/ directory"
        )
    return width, height


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--pack", required=True, help="the pack directory in the depot")
    parser.add_argument("--licence", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--confirm-terms", action="store_true", required=True)
    parser.add_argument("--role", action="append", required=True, help="role=path/in/pack.png")
    args = parser.parse_args(argv)
    pack = Path(args.pack)
    if not pack.is_dir():
        raise SystemExit(f"[import] no pack at {pack}\n  fix: sparse-checkout it from the depot")
    read_png = depot_reader(pack)
    ASSETS.mkdir(exist_ok=True)
    credits = ASSETS / "CREDITS.md"
    text = credits.read_text(encoding="utf-8") if credits.exists() else CREDITS_HEAD
    for pair in args.role:
        role, _, relative = pair.partition("=")
        if not ROLE.match(role) or not relative:
            raise SystemExit(f"[import] --role {pair!r} is not role=path with a snake_case role")
        data = (pack / relative).read_bytes()
        width, height = png_size(data, relative)
        image = read_png(pack / relative)
        out = ASSETS / f"{role}.png"
        out.write_bytes(rgba8_png(image.width, image.height, image.pixels))
        row = (
            f"| `{out.name}` | {role} ({width}x{height}) | `{pack.name}/{relative}` | "
            f"{args.source} | {args.licence} |\n"
        )
        # One row per file: a re-import replaces the file's row rather than adding one.
        lines = [line for line in text.splitlines(keepends=True) if not line.startswith(f"| `{out.name}` |")]
        text = "".join(lines) + row
        print(f"[import] {relative} -> {out.relative_to(ASSETS.parent.parent.parent)}")
    credits.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
