#!/usr/bin/env python3
"""games/giri/art/import_pack.py — bring an owner-supplied pack into the library.

The one door owner-supplied art comes in through (DESIGN §7's curation model).
It does three things a hand copy does not: it renames each file to its **role**,
so the game keeps naming what a picture *means* rather than what it is; it
refuses a file that is not a PNG within the size envelope; and it will not run
without a stated licence and source, which it writes into `assets/CREDITS.md`.

**It never downloads anything.** The pack is a directory the owner already has;
this script reads it and nothing else. An asset whose terms forbid
redistribution does not belong in a repository whose visibility allows it to be
redistributed — the script prints that check and requires `--confirm-terms`
before it writes, because the check is a human's to make and not a script's.

Usage:
  art/import_pack.py --pack <dir> --licence "<terms>" --source "<where from>"
                     [--provenance <json>] [--map role=file ...]
                     [--confirm-terms] [--dry-run]

With no `--map`, files are matched to roles by basename: a pack file named
`icon_flame.png` fills the `icon_flame` role. Roles the pack does not fill keep
their current file, so a partial library is fine and says which slots are still
the generated art.

The pack this reads is normally the staging directory `art/extract.py` writes
from the owner's picks, whose `provenance.json` names the pack each file was
curated out of — several packs can fill one library, and `--provenance` is what
lets `CREDITS.md` say which row came from which. A hand-assembled directory of
role-named PNGs still works and needs none of that.

Exit codes: 0 imported (or, under --dry-run, would import) · 1 a file was
rejected or the terms were not confirmed · 2 the script could not run.

Key functions: `roles`, `inspect`, `plan`, `write_credits`, `main`.
Depends on: the Python 3.8+ standard library only.
"""

from __future__ import annotations

import argparse
import json
import shutil
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ASSETS = HERE.parent / "assets"
CREDITS = ASSETS / "CREDITS.md"
MAX_EDGE = 2048

sys.path.insert(0, str(HERE))
from sprite_defs import LIBRARY  # noqa: E402


def roles() -> "list[str]":
    """Every slot the game names, in library order. This is the contract."""
    return [name for name, _, _ in LIBRARY]


def inspect(path: Path) -> "tuple[int, int]":
    """A PNG's size, or a ValueError saying why it is not usable."""
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n") or len(data) < 24:
        raise ValueError(f"{path.name} is not a PNG")
    width, height = struct.unpack(">II", data[16:24])
    if width > MAX_EDGE or height > MAX_EDGE:
        raise ValueError(
            f"{path.name} is {width}x{height}; individual PNGs stay at or under {MAX_EDGE}"
        )
    if width == 0 or height == 0:
        raise ValueError(f"{path.name} is {width}x{height}")
    return (width, height)


def plan(pack: Path, mapping: "dict[str, str]") -> "list[tuple[str, Path]]":
    """(role, source file) for every role the pack fills."""
    out = []
    for role in roles():
        named = mapping.get(role)
        candidate = pack / named if named else pack / f"{role}.png"
        if candidate.is_file():
            out.append((role, candidate))
    return out


def write_credits(
    rows: "list[tuple[str, int, int]]",
    licence: str,
    source: str,
    provenance: "dict[str, dict]",
) -> None:
    """Replace the credits table's body with what is now in the library.

    `provenance` names, per role, the pack a file was curated out of — several
    packs can fill one library, and a table that credited them all to one line
    would be wrong about every row but its own. Roles it does not carry fall
    back to the single `--source`, which is the whole story for a one-pack
    import.
    """
    header = CREDITS.read_text(encoding="utf-8").split("## Current library")[0]
    lines = [
        header.rstrip(),
        "",
        "## Current library — imported",
        "",
        "Each row is a **curated subset**: one individually chosen sprite, named for",
        "the role it fills. No pack is redistributed here, whole or rearranged.",
        "",
        "| File | Size | Pack | Source | Licence | What it is |",
        "|---|---|---|---|---|---|",
    ]
    for role, width, height in rows:
        entry = provenance.get(role, {})
        lines.append(
            f"| `{role}.png` | {width}x{height} "
            f"| {entry.get('pack', '—')} "
            f"| {entry.get('url', source)} "
            f"| {entry.get('licence', licence)} "
            f"| {entry.get('description', '—')} |"
        )
    # Every role the import did not fill still has a committed file, and this
    # document's own rule is that a file with no row should not be here. So the
    # generated remainder is named, not merely alluded to.
    filled = {role for role, _, _ in rows}
    generated = [role for role in roles() if role not in filled]
    if generated:
        lines += [
            "",
            "## Current library — generated",
            "",
            "Slots no pack filled. Original work of this repository, written by",
            "`art/make_art.py` from the grids in `art/sprite_defs.py`.",
            "",
            "| File | Source | Licence |",
            "|---|---|---|",
        ]
        for role in generated:
            lines.append(f"| `{role}.png` | `art/sprite_defs.py` (this repository) | original work |")
    lines += [
        "",
        "The slots are the same whichever way a file arrived (UI.md §9): the role is",
        "the contract, not the picture.",
        "",
        "## Replacing a file",
        "",
        "From a pack the manifest already knows, changing which sprite fills a role is",
        "one line — edit `chosen` in `art/kenney-manifest.json`, then:",
        "",
        "```",
        "art/extract.py --packs <dir>              # cuts the picks, role-named, into target/",
        "art/import_pack.py --pack target/giri-art/staged \\",
        "    --provenance target/giri-art/staged/provenance.json \\",
        '    --licence "CC0 1.0" --source "https://kenney.nl" --confirm-terms',
        "```",
        "",
        "From a pack nothing has looked at yet, see it first — `art/contact_sheet.py`",
        "renders indexed sheets into `target/`, `art/role_sheet.py` renders the",
        "shortlist for one role — then classify what you used into the manifest and run",
        "the two commands above. Contact sheets are never committed.",
        "",
        "Whichever route: check the licence against this repository's visibility",
        "**before** the commit. Art that may not be redistributed does not go in a",
        "repository that redistributes it — not even \"temporarily\".",
        "",
    ]
    CREDITS.write_text("\n".join(lines), encoding="utf-8")


def main(argv: "list[str]") -> int:
    parser = argparse.ArgumentParser(add_help=True, description=__doc__)
    parser.add_argument("--pack", required=True, help="directory of PNGs the owner supplied")
    parser.add_argument("--licence", required=True, help="the terms, verbatim, for CREDITS.md")
    parser.add_argument("--source", required=True, help="where the pack came from")
    parser.add_argument("--map", action="append", default=[], metavar="role=file")
    parser.add_argument(
        "--provenance",
        default=None,
        metavar="JSON",
        help="per-role pack/licence facts, as art/extract.py writes them",
    )
    parser.add_argument("--confirm-terms", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv[1:])

    pack = Path(args.pack).expanduser().resolve()
    if not pack.is_dir():
        print(f"[giri-art] no pack at {pack}")
        print("  likely cause: the path is wrong, or the pack has not been unpacked")
        print("  fix: pass --pack <directory of PNGs>")
        return 1
    mapping = {}
    for entry in args.map:
        role, _, name = entry.partition("=")
        if not role or not name:
            print(f"[giri-art] --map wants role=file, got {entry!r}")
            return 1
        mapping[role] = name

    provenance = {}
    if args.provenance:
        try:
            provenance = json.loads(Path(args.provenance).read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
            print(f"[giri-art] the provenance file could not be read\n  {error}")
            print("  fix: re-run art/extract.py, which writes it beside the staged PNGs")
            return 1

    chosen = plan(pack, mapping)
    if not chosen:
        print(f"[giri-art] {pack} fills none of giri's {len(roles())} roles")
        print(f"  roles: {', '.join(roles())}")
        print("  fix: rename the pack's files to their roles, or pass --map role=file")
        return 1

    rows = []
    for role, source in chosen:
        try:
            width, height = inspect(source)
        except ValueError as error:
            print(f"[giri-art] {error}")
            print("  likely cause: the pack holds an atlas or a non-PNG for this slot")
            print("  fix: export the slot as an individual PNG at or under 2048 on each axis")
            return 1
        rows.append((role, width, height))

    print(f"[giri-art] {len(rows)} of {len(roles())} role(s) filled from {pack}")
    for role, width, height in rows:
        pack_name = provenance.get(role, {}).get("pack", args.source)
        print(f"  {role}.png  {width}x{height}  <- {pack_name}")
    print(f"  licence: {args.licence}")
    print(f"  source:  {args.source}")
    print(
        "  These files will be committed to this repository. Check the licence against\n"
        "  the repository's visibility first: art that may not be redistributed does not\n"
        "  go in a repository that redistributes it."
    )
    if args.dry_run:
        print("[giri-art] --dry-run: nothing written")
        return 0
    if not args.confirm_terms:
        print("[giri-art] refusing to write without --confirm-terms")
        print("  likely cause: the licence check above has not been made by a person")
        print("  fix: re-run with --confirm-terms once the terms allow this repository")
        return 1

    for (role, _, _), (_, source) in zip(rows, chosen):
        shutil.copyfile(source, ASSETS / f"{role}.png")
    write_credits(rows, args.licence, args.source, provenance)
    print(f"[giri-art] wrote {len(rows)} file(s) and rewrote {CREDITS.name}")
    print("  next: cargo check -p giri, then tools/verify giri, then look at the captures")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv))
    except Exception as error:  # noqa: BLE001 - a tool reports rather than traces
        print(f"[giri-art] the import could not run\n  {error}")
        sys.exit(2)
