#!/usr/bin/env python3
"""Add the owned evaluation and release skeleton to an initialized skill."""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TEMPLATES = ROOT / "assets" / "templates"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("skill", type=Path)
    parser.add_argument("--proposal", type=Path, required=True)
    args = parser.parse_args()
    skill = args.skill.resolve()
    if not (skill / "SKILL.md").is_file():
        raise SystemExit("run the harness's official skill initializer first")
    proposal = json.loads(args.proposal.read_text())
    if not proposal.get("evidence", {}).get("references"):
        raise SystemExit("proposal needs at least one inspected evidence reference")
    evals = skill / "evals"
    evals.mkdir(exist_ok=True)
    (evals / "results").mkdir(exist_ok=True)
    release = evals / "release-decision.json"
    if not release.exists():
        shutil.copyfile(TEMPLATES / "release-decision.json", release)
    proposal_target = skill / "proposal.json"
    if proposal_target.exists():
        raise SystemExit("proposal.json already exists; refusing to overwrite")
    proposal_target.write_text(json.dumps(proposal, indent=2) + "\n")
    print(skill)


if __name__ == "__main__":
    main()
