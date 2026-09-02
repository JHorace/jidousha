#!/usr/bin/env python3
"""games/giri/art/extract.py — stage the owner's picks as role-named PNGs.

The step between curation and import. `kenney-manifest.json`'s `chosen` map says
which pack region fills which role; this cuts each one out of the owner's packs,
lifts any baked-in tile background, and writes it under its **role name** into a
staging directory. `import_pack.py` then copies that directory into `assets/` —
unchanged, still the one door art comes in through.

Two scripts rather than one because they are two jobs: this one knows about
packs and rectangles, that one knows about giri's slots and its credits. Neither
knows about the other's half.

**Nothing here decides anything.** The picks are the owner's, recorded in the
manifest; a role mapped to `null` keeps its generated art and is skipped, which
is how the infamy eye survives a pack that has no eye in it.

Staging is untracked (under `target/`) because it holds pack-derived files that
have not yet passed the import's checks.

Usage:
  art/extract.py --packs <dir of unpacked Kenney packs> [--out <staging dir>]

Writes `<staging>/<role>.png` per chosen role, plus `provenance.json` naming the
pack each file came from, which `import_pack.py --provenance` turns into
`CREDITS.md` rows.

Exit codes: 0 staged · 1 the packs or a pick could not be read · 2 the script
could not run.

Key functions: `resolve`, `main`.
Depends on: `pack_reader.py` and the Python 3.8+ standard library only.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent.parent
MANIFEST = HERE / "kenney-manifest.json"
sys.path.insert(0, str(HERE))

from contact_sheet import png  # noqa: E402
from pack_reader import Image, read_png, strip_background  # noqa: E402


class Surface:
    """The shape `contact_sheet.png` encodes: width, height, RGBA8 bytes."""

    def __init__(self, image: Image) -> None:
        self.width = image.width
        self.height = image.height
        self.pixels = image.pixels


def resolve(entry: dict, manifest: dict, packs: Path) -> Image:
    """The picked sprite, cut from the pack and lifted off its tile background."""
    pack = manifest["packs"][entry["pack"]]
    root = packs / pack["dir"]
    if "file" in entry:
        image = read_png(root / pack["source"] / entry["file"])
    else:
        x, y, width, height = entry["rect"]
        image = read_png(root / pack["source"]).crop(x, y, width, height)
    return strip_background(image)


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(add_help=True, description=__doc__)
    parser.add_argument("--packs", required=True)
    parser.add_argument("--out", default=None)
    args = parser.parse_args(argv[1:])

    packs = Path(args.packs).expanduser().resolve()
    if not packs.is_dir():
        print(f"[giri-art] no packs at {packs}")
        print("  fix: pass --packs <the directory the owner's packs were unpacked into>")
        return 1

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    by_id = {entry["id"]: entry for entry in manifest["entries"]}
    chosen = {
        role: pick for role, pick in manifest["chosen"].items() if not role.startswith("_")
    }

    out = Path(args.out).expanduser().resolve() if args.out else REPO / "target" / "ninjo-art" / "staged"
    out.mkdir(parents=True, exist_ok=True)

    provenance = {}
    staged = 0
    for role, pick in chosen.items():
        if pick is None:
            print(f"  {role:16} generated art kept (no pick)")
            continue
        entry = by_id.get(pick)
        if entry is None:
            print(f"[giri-art] {role} names {pick}, which is not an entry in the manifest")
            print("  likely cause: a pick was edited without adding the entry it names")
            print(f"  fix: add {pick} to 'entries', or correct 'chosen'")
            return 1
        try:
            image = resolve(entry, manifest, packs)
        except (ValueError, OSError, KeyError) as error:
            print(f"[giri-art] {role} ({pick}): {error}")
            print("  likely cause: --packs does not hold the pack this pick names")
            return 1
        (out / f"{role}.png").write_bytes(png(Surface(image)))
        pack = manifest["packs"][entry["pack"]]
        provenance[role] = {
            "pack": pack["name"],
            "url": pack["url"],
            "licence": pack["licence"],
            "pick": pick,
            "texels": [image.width, image.height],
            "description": entry["description"],
        }
        print(f"  {role:16} {image.width:2}x{image.height:<2} <- {pick}")
        staged += 1

    (out / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
    print(f"[giri-art] staged {staged} file(s) in {out}")
    print(f"  next: art/import_pack.py --pack {out} --provenance {out / 'provenance.json'} \\")
    print('          --licence "CC0 1.0" --source "kenney.nl" --confirm-terms')
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Exception as error:  # noqa: BLE001 - a tool reports rather than traces
        print(f"[giri-art] the picks could not be staged\n  {error}")
        sys.exit(2)
