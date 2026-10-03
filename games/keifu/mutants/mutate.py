#!/usr/bin/env python3
"""The mutation round: inject one-line faults into keifu and demand the run names each.

Usage (from the repository root, with every file under games/keifu committed):

    games/keifu/mutants/mutate.py games/keifu/mutants/w3.txt [--only LABEL ...] [--jobs N]

A list file holds one mutation per block of four lines:

    ### label — what the fault is
    @ src/witness.rs
    - the exact text to find (it must occur exactly once in the file)
    + the text that replaces it

("⏎" in either stands for a line break, for a fault that spans lines.)

Per mutation it builds, runs `cargo test -p keifu` and then `keifu --verify`, and
counts the fault as noticed if either fails. The two ways a hand-rolled harness lies
about its own score are hard errors here (docs/api/jidousha-testing.md, "Mutate the
game and check the run notices"): a find that matches other than once stops the run
before anything is written, and a mutation that does not build is reported as
NOT BUILT — never counted as noticed. Each file is written back from the bytes read,
not with `git checkout`, and the tree is checked clean at the end.

With --jobs N the list is split over N git worktrees of HEAD (each with its own
target directory), so the round runs in parallel against exactly the committed tree.

Stdlib only, like every tool in this repository.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

CRATE = Path("games/keifu")


def parse(path: Path) -> list[tuple[str, str, str, str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    out, i = [], 0
    while i < len(lines):
        if not lines[i].strip() or lines[i].startswith("#!"):
            i += 1
            continue
        block = lines[i : i + 4]
        if (
            len(block) < 4
            or not block[0].startswith("### ")
            or not block[1].startswith("@ ")
            or not block[2].startswith("- ")
            or not block[3].startswith("+ ")
        ):
            sys.exit(f"[mutate] {path}:{i + 1}: a mutation is ### label / @ file / - find / + replace")
        # "⏎" stands for a line break, for a fault that spans lines.
        find, replace = (block[2][2:].replace("⏎", "\n"), block[3][2:].replace("⏎", "\n"))
        out.append((block[0][4:].strip(), block[1][2:].strip(), find, replace))
        i += 4
    labels = [m[0] for m in out]
    dupes = {l for l in labels if labels.count(l) > 1}
    if dupes:
        sys.exit(f"[mutate] duplicate labels: {sorted(dupes)}")
    return out


def run(cmd: list[str], cwd: Path, env: dict) -> int:
    # A run that never ends is a run that failed: a fault that hangs the check is noticed,
    # and must not take its worker thread (and the rest of its share of the list) with it.
    try:
        return subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=600).returncode
    except subprocess.TimeoutExpired:
        return 124


def check_all(root: Path, mutations) -> None:
    for label, file, find, _ in mutations:
        text = (root / CRATE / file).read_text(encoding="utf-8")
        hits = text.count(find)
        if hits != 1:
            sys.exit(f"[mutate] {label}: {find!r} occurs {hits} times in {file}; a mutation must match exactly once")


def work(root: Path, mutations, results: list, lock: threading.Lock) -> None:
    env = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"))
    for label, file, find, replace in mutations:
        path = root / CRATE / file
        original = path.read_bytes()
        text = original.decode("utf-8")
        path.write_text(text.replace(find, replace, 1), encoding="utf-8")
        try:
            if run(["cargo", "build", "-q", "-p", "keifu", "--tests"], root, env) != 0 or run(
                ["cargo", "build", "-q", "-p", "keifu"], root, env
            ) != 0:
                verdict = ("NOT BUILT", False, False)
            else:
                tests = run(["cargo", "test", "-q", "-p", "keifu"], root, env) != 0
                verify = run(["cargo", "run", "-q", "-p", "keifu", "--", "--verify"], root, env) != 0
                verdict = ("noticed" if tests or verify else "ESCAPED", tests, verify)
        finally:
            path.write_bytes(original)
        with lock:
            results.append((label, *verdict))
            print(f"[mutate] {verdict[0]:9} tests={'x' if verdict[1] else '.'} verify={'x' if verdict[2] else '.'}  {label}", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("list", type=Path)
    parser.add_argument("--only", nargs="*", default=None)
    parser.add_argument("--jobs", type=int, default=1)
    args = parser.parse_args()
    repo = Path.cwd()
    if subprocess.run(["git", "status", "--porcelain", str(CRATE)], capture_output=True, text=True).stdout.strip():
        sys.exit("[mutate] games/keifu has uncommitted changes; commit every file the round touches first")
    mutations = parse(args.list)
    if args.only is not None:
        mutations = [m for m in mutations if m[0].split(" ")[0] in args.only]
    check_all(repo, mutations)
    results: list = []
    lock = threading.Lock()
    if args.jobs <= 1:
        work(repo, mutations, results, lock)
    else:
        base = Path(tempfile.mkdtemp(prefix="keifu-mutants-"))
        trees = []
        for n in range(args.jobs):
            tree = base / f"w{n}"
            subprocess.run(["git", "worktree", "add", "--detach", str(tree), "HEAD"], check=True, capture_output=True)
            trees.append(tree)
        threads = [
            threading.Thread(target=work, args=(tree, mutations[n :: args.jobs], results, lock))
            for n, tree in enumerate(trees)
        ]
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        for tree in trees:
            dirty = subprocess.run(["git", "status", "--porcelain"], cwd=tree, capture_output=True, text=True).stdout.strip()
            if dirty:
                sys.exit(f"[mutate] {tree} is not clean after the round: {dirty}")
            subprocess.run(["git", "worktree", "remove", "--force", str(tree)], check=True)
    if args.jobs <= 1 and subprocess.run(
        ["git", "status", "--porcelain", str(CRATE)], capture_output=True, text=True
    ).stdout.strip():
        sys.exit("[mutate] games/keifu is not clean after the round")
    built = [r for r in results if r[1] != "NOT BUILT"]
    noticed = [r for r in built if r[1] == "noticed"]
    print()
    print(f"[mutate] {len(noticed)} of {len(built)} noticed; tests alone {sum(r[2] for r in built)}, verify alone {sum(r[3] for r in built)}")
    for r in results:
        if r[1] != "noticed":
            print(f"[mutate] {r[1]}: {r[0]}")


if __name__ == "__main__":
    main()
