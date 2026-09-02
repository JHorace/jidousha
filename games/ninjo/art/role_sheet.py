#!/usr/bin/env python3
"""games/giri/art/role_sheet.py — one candidate sheet per asset slot, for the owner.

`contact_sheet.py` renders a whole pack so somebody can find things in it. This
renders the *shortlist*: the classified candidates for one role, side by side at
a size worth judging, so the owner can pick a slot's art by naming an id.

**The picking is the owner's** (DESIGN §7's curation model). This script's whole
job is to put three to five real options in front of a person; it has no notion
of a best one and writes nothing into `assets/`.

Candidates come from `kenney-manifest.json` — every entry whose `candidate_for`
names the role — so adding an option is a manifest edit, not a code edit.

Sheets land beside the contact sheets, under `target/`, and are never committed.

Usage:
  art/role_sheet.py --packs <dir of unpacked Kenney packs> [--role <name>]
                    [--out <dir>] [--scale N]

Exit codes: 0 sheets written · 1 the packs or the manifest could not be read ·
2 the script could not run.

Key functions: `candidates`, `resolve`, `main`.
Depends on: `pack_reader.py`, `contact_sheet.py`, and the standard library only.
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

from contact_sheet import compose, png  # noqa: E402
from pack_reader import Image, read_png, strip_background  # noqa: E402


def candidates(manifest: dict, role: str) -> "list[dict]":
    """Every classified entry offered for `role`, in manifest order."""
    return [entry for entry in manifest["entries"] if role in entry.get("candidate_for", [])]


def resolve(entry: dict, manifest: dict, packs: Path) -> Image:
    """The candidate's picture, read out of the pack on the owner's disk.

    Two shapes, because packs come in two: a named file inside the pack's source
    directory, or a rectangle of a tilesheet. Both are stated in the manifest,
    so nothing here has to re-derive a grid.
    """
    pack = manifest["packs"][entry["pack"]]
    root = packs / pack["dir"]
    if "file" in entry:
        image = read_png(root / pack["source"] / entry["file"])
    else:
        x, y, width, height = entry["rect"]
        image = read_png(root / pack["source"]).crop(x, y, width, height)
    # What the owner judges has to be what the import would commit, background
    # and all — a candidate shown on its pack's opaque tile backing is a
    # candidate shown as it will never appear in the game.
    return strip_background(image)


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(add_help=True, description=__doc__)
    parser.add_argument("--packs", required=True, help="directory holding the unpacked packs")
    parser.add_argument("--role", default=None, help="one role; default is every role offered")
    parser.add_argument("--out", default=None)
    parser.add_argument("--scale", type=int, default=10)
    parser.add_argument("--columns", type=int, default=6)
    args = parser.parse_args(argv[1:])

    packs = Path(args.packs).expanduser().resolve()
    if not packs.is_dir():
        print(f"[giri-art] no packs at {packs}")
        print("  fix: pass --packs <the directory the owner's packs were unpacked into>")
        return 1
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))

    offered = []
    for entry in manifest["entries"]:
        for role in entry.get("candidate_for", []):
            if role not in offered:
                offered.append(role)
    roles = [args.role] if args.role else offered
    unknown = [role for role in roles if role not in offered]
    if unknown:
        print(f"[giri-art] no candidates classified for {', '.join(unknown)}")
        print(f"  roles with candidates: {', '.join(offered)}")
        print("  fix: classify some in kenney-manifest.json, or ask for one of the above")
        return 1

    out = Path(args.out).expanduser().resolve() if args.out else REPO / "target" / "ninjo-art" / "roles"
    out.mkdir(parents=True, exist_ok=True)

    for role in roles:
        chosen = candidates(manifest, role)
        entries = []
        for entry in chosen:
            try:
                entries.append((entry["id"].replace(":", "_"), resolve(entry, manifest, packs)))
            except (ValueError, OSError) as error:
                print(f"[giri-art] {entry['id']}: {error}")
                print("  likely cause: --packs does not hold the pack this entry names")
                return 1
        canvas = compose(entries, args.scale, min(args.columns, len(entries)))
        path = out / f"{role}.png"
        path.write_bytes(png(canvas))
        print(f"  {path.relative_to(REPO)}  {len(entries)} candidate(s)")
        for entry in chosen:
            print(f"      {entry['id']:24} {entry['description']}")
    print(f"[giri-art] {len(roles)} role sheet(s) in {out} — show these to the owner")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Exception as error:  # noqa: BLE001 - a tool reports rather than traces
        print(f"[giri-art] the role sheets could not be rendered\n  {error}")
        sys.exit(2)
