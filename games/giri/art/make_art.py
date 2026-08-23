#!/usr/bin/env python3
"""games/giri/art/make_art.py — write giri's art.

Reads the grids in `sprite_defs.py` and writes one PNG per asset slot into
`games/giri/assets/`, role-named lowercase snake_case (DESIGN §7's curation
model). Original art, generated deterministically by a committed script:
nothing here is downloaded, and the same run on any machine writes the same
bytes.

**This is giri's art, not a stand-in for it** (owner, 2026-08-23). It began as
the placeholder set the mockup's grids describe and the owner kept it, so the
grids in `sprite_defs.py` are where a change to how giri looks is made — edit a
grid, run this, look at the captures. `import_pack.py` remains the door a
different library would come in through; nothing is waiting on it.

Usage:  games/giri/art/make_art.py [--check]

`--check` writes nothing and reports whether the committed PNGs are what this
script would produce, which is how a hand edit to a grid gets noticed.

Exit codes: 0 written (or, under --check, in step) · 1 under --check, a file is
missing or stale · 2 the script could not run.

Depends on: the Python 3.8+ standard library only.
"""

from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ASSETS = HERE.parent / "assets"
sys.path.insert(0, str(HERE))

from sprite_defs import LIBRARY  # noqa: E402


def rgba(hex_color: str) -> "tuple[int, int, int, int]":
    """`#rrggbb` as opaque RGBA bytes."""
    text = hex_color.lstrip("#")
    if len(text) != 6:
        raise ValueError(f"a palette colour must be #rrggbb, got {hex_color!r}")
    return (int(text[0:2], 16), int(text[2:4], 16), int(text[4:6], 16), 255)


def texels(grid: "list[str]", palette: "dict[str, str]") -> bytes:
    """The grid as RGBA8 rows. A key the palette does not carry is transparent."""
    width = len(grid[0])
    if any(len(row) != width for row in grid):
        raise ValueError("every row of a sprite grid is the same width")
    resolved = {key: rgba(value) for key, value in palette.items()}
    out = bytearray()
    for row in grid:
        for key in row:
            out.extend(resolved.get(key, (0, 0, 0, 0)))
    return bytes(out)


def png(width: int, height: int, pixels: bytes) -> bytes:
    """RGBA8 texels as PNG bytes.

    Hand-rolled over `zlib` because this repository's tools are stdlib-only
    (tooling.md) and the alternative is a dependency for nine small pictures.
    Filter byte 0 on every row: no prediction, so the bytes are a function of
    the grid and of nothing else.
    """
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


def render(name: str, grid: "list[str]", palette: "dict[str, str]") -> "tuple[Path, bytes]":
    """One library entry as (path, PNG bytes)."""
    width, height = len(grid[0]), len(grid)
    # The envelope DESIGN §7's curation model states for an individual file.
    # These are 8-16 texels across; the check is here for the day the owner's
    # library arrives through the same door.
    if width > 2048 or height > 2048:
        raise ValueError(f"{name} is {width}x{height}; individual PNGs stay at or under 2048")
    return (ASSETS / f"{name}.png", png(width, height, texels(grid, palette)))


def main(argv: "list[str]") -> int:
    checking = "--check" in argv[1:]
    ASSETS.mkdir(parents=True, exist_ok=True)
    stale: "list[str]" = []
    for name, grid, palette in LIBRARY:
        path, bytes_out = render(name, grid, palette)
        if checking:
            if not path.exists() or path.read_bytes() != bytes_out:
                stale.append(path.name)
            continue
        path.write_bytes(bytes_out)
        print(f"[giri-art] {path.relative_to(HERE.parent)}  {len(grid[0])}x{len(grid)}")
    if checking:
        if stale:
            print(f"[giri-art] out of date: {', '.join(stale)}")
            print("  likely cause: a grid in sprite_defs.py changed and the PNGs were not rewritten")
            print("  fix: run games/giri/art/make_art.py")
            return 1
        print(f"[giri-art] {len(LIBRARY)} file(s) match the committed grids")
        return 0
    print(f"[giri-art] wrote {len(LIBRARY)} file(s) to {ASSETS}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Exception as error:  # noqa: BLE001 - a tool reports rather than traces
        print(f"[giri-art] the art could not be generated\n  {error}")
        sys.exit(2)
