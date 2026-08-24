"""Read PNGs and cut packs into sprites — the input half of the art tooling.

`make_art.py` writes PNGs; this reads them. Curation needs the other direction:
an owner-supplied pack arrives as individual files or as a tilesheet, and
`contact_sheet.py` has to *see* every candidate before anything is chosen.

**Stdlib only, like every tool in this repository** (tooling.md). The decoder
covers exactly what Kenney's packs are — palette at 1/2/4/8 bits with `tRNS`,
and RGBA8 — and raises on anything else rather than guessing, because a decoder
that silently mis-reads a picture produces a contact sheet that lies about the
art. Interlaced PNGs are refused for the same reason.

Nothing here writes to `assets/` and nothing here downloads: a pack is a
directory the owner already has, read and never copied wholesale (DESIGN §7).

Key functions: `read_png`, `slice_grid`, `load_sources`, `is_empty`.
Depends on: the Python 3.8+ standard library only.
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

PNG_MAGIC = b"\x89PNG\r\n\x1a\n"

# Channels per pixel for the colour types this decoder accepts.
CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


class Image:
    """A decoded picture: RGBA8 texels, row-major, `width * height * 4` bytes."""

    def __init__(self, width: int, height: int, pixels: bytes) -> None:
        self.width = width
        self.height = height
        self.pixels = pixels

    def pixel(self, x: int, y: int) -> "tuple[int, int, int, int]":
        offset = (y * self.width + x) * 4
        return tuple(self.pixels[offset : offset + 4])  # type: ignore[return-value]

    def crop(self, x: int, y: int, width: int, height: int) -> "Image":
        """The sub-rectangle at (x, y), clipped to nothing — callers stay in bounds."""
        out = bytearray()
        for row in range(y, y + height):
            start = (row * self.width + x) * 4
            out.extend(self.pixels[start : start + width * 4])
        return Image(width, height, bytes(out))


def _unfilter(raw: bytes, width: int, height: int, bit_depth: int, channels: int) -> bytes:
    """Undo the per-row filters, returning packed scanlines with no filter bytes."""
    stride = (width * channels * bit_depth + 7) // 8
    # Filtering works on whole bytes: one pixel, rounded up, never less than one.
    step = max(1, (channels * bit_depth) // 8)
    out = bytearray()
    previous = bytearray(stride)
    position = 0
    for row in range(height):
        if position >= len(raw):
            raise ValueError(f"the image data ends after {row} of {height} rows")
        kind = raw[position]
        position += 1
        line = bytearray(raw[position : position + stride])
        if len(line) != stride:
            raise ValueError(f"row {row} is {len(line)} bytes, expected {stride}")
        position += stride
        if kind == 0:
            pass
        elif kind == 1:
            for i in range(step, stride):
                line[i] = (line[i] + line[i - step]) & 0xFF
        elif kind == 2:
            for i in range(stride):
                line[i] = (line[i] + previous[i]) & 0xFF
        elif kind == 3:
            for i in range(stride):
                left = line[i - step] if i >= step else 0
                line[i] = (line[i] + ((left + previous[i]) >> 1)) & 0xFF
        elif kind == 4:
            for i in range(stride):
                left = line[i - step] if i >= step else 0
                up = previous[i]
                upper_left = previous[i - step] if i >= step else 0
                estimate = left + up - upper_left
                da, db, dc = (
                    abs(estimate - left),
                    abs(estimate - up),
                    abs(estimate - upper_left),
                )
                nearest = left if (da <= db and da <= dc) else (up if db <= dc else upper_left)
                line[i] = (line[i] + nearest) & 0xFF
        else:
            raise ValueError(f"row {row} uses filter {kind}, which is not a PNG filter")
        out.extend(line)
        previous = line
    return bytes(out)


def _unpack_indices(scanlines: bytes, width: int, height: int, bit_depth: int) -> "list[int]":
    """Palette indices, one per texel, for a sub-byte or 8-bit indexed image."""
    if bit_depth == 8:
        stride = width
        return [scanlines[row * stride + x] for row in range(height) for x in range(width)]
    per_byte = 8 // bit_depth
    mask = (1 << bit_depth) - 1
    stride = (width * bit_depth + 7) // 8
    indices = []
    for row in range(height):
        base = row * stride
        for x in range(width):
            byte = scanlines[base + x // per_byte]
            # Left-to-right within the byte, most significant group first.
            shift = 8 - bit_depth * (x % per_byte + 1)
            indices.append((byte >> shift) & mask)
    return indices


def read_png(path: Path) -> Image:
    """Decode a PNG to RGBA8, or raise a ValueError saying what it is instead."""
    data = path.read_bytes()
    if not data.startswith(PNG_MAGIC):
        raise ValueError(f"{path.name} is not a PNG")
    width = height = bit_depth = colour_type = 0
    palette: "list[tuple[int, int, int]]" = []
    transparency: "list[int]" = []
    idat = bytearray()
    position = len(PNG_MAGIC)
    while position + 8 <= len(data):
        (length,) = struct.unpack(">I", data[position : position + 4])
        kind = data[position + 4 : position + 8]
        body = data[position + 8 : position + 8 + length]
        position += 12 + length
        if kind == b"IHDR":
            width, height, bit_depth, colour_type, _, _, interlace = struct.unpack(
                ">IIBBBBB", body
            )
            if interlace:
                raise ValueError(f"{path.name} is interlaced; this reader wants a plain PNG")
            if colour_type not in CHANNELS:
                raise ValueError(f"{path.name} has colour type {colour_type}")
        elif kind == b"PLTE":
            palette = [tuple(body[i : i + 3]) for i in range(0, len(body), 3)]  # type: ignore[misc]
        elif kind == b"tRNS":
            transparency = list(body)
        elif kind == b"IDAT":
            idat.extend(body)
        elif kind == b"IEND":
            break
    if not width or not height:
        raise ValueError(f"{path.name} has no IHDR")

    scanlines = _unfilter(
        zlib.decompress(bytes(idat)), width, height, bit_depth, CHANNELS[colour_type]
    )
    out = bytearray()
    if colour_type == 3:
        if not palette:
            raise ValueError(f"{path.name} is indexed but carries no palette")
        for index in _unpack_indices(scanlines, width, height, bit_depth):
            if index >= len(palette):
                raise ValueError(f"{path.name} indexes palette entry {index} of {len(palette)}")
            red, green, blue = palette[index]
            alpha = transparency[index] if index < len(transparency) else 255
            out.extend((red, green, blue, alpha))
    elif colour_type == 6 and bit_depth == 8:
        out.extend(scanlines)
    elif colour_type == 2 and bit_depth == 8:
        for i in range(0, len(scanlines), 3):
            out.extend(scanlines[i : i + 3])
            out.append(255)
    else:
        raise ValueError(
            f"{path.name} is colour type {colour_type} at {bit_depth} bits, which this reader "
            "does not decode"
        )
    return Image(width, height, bytes(out))


def is_empty(image: Image) -> bool:
    """True when every texel is fully transparent — a blank cell of a tilesheet."""
    return not any(image.pixels[3::4])


def strip_background(image: Image) -> Image:
    """Lift a tile's baked-in background to transparency, from the edges inward.

    Some packs ship tiles meant for a tilemap, where every texel is opaque and
    the "background" is a flat colour behind the subject — Kenney's Micro
    Roguelike is one, and its icons would draw as dark squares on giri's panels.

    **Edge-connected flood fill, not a colour key.** The two differ exactly
    where it matters: that pack's skull holds its eye sockets, and its chest its
    banding, in the *same* colour as the background. A key erases them and
    leaves a skull with no eyes; a fill started from the border cannot reach
    them and leaves them alone.

    A tile that already carries any transparency is returned untouched — the
    pack has said what it means, and second-guessing it is how art gets damaged.
    """
    if not all(alpha == 255 for alpha in image.pixels[3::4]):
        return image
    background = image.pixel(0, 0)[:3]
    pixels = bytearray(image.pixels)
    seen = set()
    stack = [
        (x, y)
        for x in range(image.width)
        for y in range(image.height)
        if x in (0, image.width - 1) or y in (0, image.height - 1)
    ]
    while stack:
        x, y = stack.pop()
        if (x, y) in seen or not (0 <= x < image.width and 0 <= y < image.height):
            continue
        if image.pixel(x, y)[:3] != background:
            continue
        seen.add((x, y))
        pixels[(y * image.width + x) * 4 + 3] = 0
        stack += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    return Image(image.width, image.height, bytes(pixels))


def slice_grid(sheet: Image, tile: "tuple[int, int]", spacing: int, margin: int) -> "list[Image]":
    """Cut a tilesheet into tiles, row-major, in the pack's own index order.

    The stride is tile plus spacing, and a cell counts when it fits whole: that
    is the arithmetic Kenney's `Tilesheet.txt` describes, and it reproduces the
    stated tile counts on every pack in this session.
    """
    tile_width, tile_height = tile
    columns = (sheet.width - 2 * margin + spacing) // (tile_width + spacing)
    rows = (sheet.height - 2 * margin + spacing) // (tile_height + spacing)
    return [
        sheet.crop(
            margin + column * (tile_width + spacing),
            margin + row * (tile_height + spacing),
            tile_width,
            tile_height,
        )
        for row in range(rows)
        for column in range(columns)
    ]


def load_sources(
    source: Path, tile: "tuple[int, int]", spacing: int, margin: int
) -> "list[tuple[str, Image]]":
    """(label, image) for every candidate in `source`, in a stable order.

    A directory yields its PNGs sorted by name — which is the pack's own order
    for `tile_0000.png`-style packs and alphabetical for descriptive ones,
    either way reproducible. A single file is sliced when `tile` is given and
    taken whole when it is not.
    """
    if source.is_dir():
        return [(path.stem, read_png(path)) for path in sorted(source.glob("*.png"))]
    if not source.is_file():
        raise ValueError(f"{source} is neither a directory of PNGs nor a PNG")
    sheet = read_png(source)
    if tile is None:
        return [(source.stem, sheet)]
    return [(str(index), image) for index, image in enumerate(slice_grid(sheet, tile, spacing, margin))]
