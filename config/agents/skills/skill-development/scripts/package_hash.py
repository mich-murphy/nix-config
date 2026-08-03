#!/usr/bin/env python3
"""Print a deterministic SHA-256 hash of a skill package."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path


IGNORED_PARTS = {"__pycache__", ".pytest_cache"}
IGNORED_MUTABLE = {"evals/release-decision.json"}


def package_hash(root: Path) -> str:
    digest = hashlib.sha256()
    files = []
    for path in root.rglob("*"):
        if not path.is_file() or IGNORED_PARTS.intersection(path.parts):
            continue
        relative = path.relative_to(root).as_posix()
        if relative in IGNORED_MUTABLE or relative.startswith("evals/results/"):
            continue
        files.append(path)
    for path in sorted(files):
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("skill", type=Path)
    args = parser.parse_args()
    print(package_hash(args.skill.resolve()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
