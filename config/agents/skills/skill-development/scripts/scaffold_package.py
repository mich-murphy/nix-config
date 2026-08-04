#!/usr/bin/env python3
"""Add an evidence, evaluation, telemetry, and release skeleton to a skill."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TEMPLATES = ROOT / "assets" / "templates"


def write_if_missing(source: Path, target: Path) -> bool:
    if target.exists():
        return False
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)
    return True


def scaffold(skill: Path, proposal_path: Path) -> list[Path]:
    if not (skill / "SKILL.md").is_file():
        raise ValueError("run the harness's official skill initializer first")
    proposal = json.loads(proposal_path.read_text(encoding="utf-8"))
    references = proposal.get("evidence", {}).get("references", [])
    if not references:
        raise ValueError("proposal needs at least one inspected evidence reference")
    if not proposal.get("job"):
        raise ValueError("proposal needs one explicit job")

    created: list[Path] = []
    proposal_target = skill / "proposal.json"
    if proposal_target.exists():
        raise ValueError("proposal.json already exists; refusing to overwrite")
    proposal_target.write_text(
        json.dumps(proposal, indent=2) + "\n", encoding="utf-8"
    )
    created.append(proposal_target)

    mapping = {
        "routes.json": skill / "evals" / "routes.json",
        "release-manifest.json": skill / "evals" / "release-manifest.json",
    }
    for source_name, target in mapping.items():
        if write_if_missing(TEMPLATES / source_name, target):
            created.append(target)

    release_manifest = skill / "evals" / "release-manifest.json"
    if release_manifest in created:
        manifest = json.loads(release_manifest.read_text(encoding="utf-8"))
        manifest["skill"] = skill.name
        release_manifest.write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )

    cases = skill / "evals" / "cases.json"
    if not cases.exists():
        cases.parent.mkdir(parents=True, exist_ok=True)
        cases.write_text("[]\n", encoding="utf-8")
        created.append(cases)
    routing = skill / "evals" / "routing-cases.json"
    if not routing.exists():
        routing.write_text("[]\n", encoding="utf-8")
        created.append(routing)
    return created


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("skill", type=Path)
    parser.add_argument("--proposal", type=Path, required=True)
    args = parser.parse_args()
    try:
        created = scaffold(args.skill.resolve(), args.proposal.resolve())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit(str(error)) from error
    for path in created:
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
