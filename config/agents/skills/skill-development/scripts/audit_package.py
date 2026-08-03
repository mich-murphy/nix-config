#!/usr/bin/env python3
"""Fail on structural, provenance, and common package-safety defects."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


SECRET = re.compile(r"(?i)(sk-[a-z0-9]{16,}|api[_-]?key\s*[:=]\s*['\"][^'\"]+|password\s*[:=]\s*['\"][^'\"]+)")


def audit(root: Path) -> list[str]:
    findings: list[str] = []
    skill = root / "SKILL.md"
    if not skill.is_file():
        return ["missing SKILL.md"]
    text = skill.read_text()
    if len(text.splitlines()) > 500:
        findings.append("SKILL.md exceeds the 500-line review threshold")
    if not text.startswith("---\n") or text.count("---\n") < 2:
        findings.append("SKILL.md frontmatter is malformed")
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if path.is_symlink():
            findings.append(f"symlink requires manual provenance review: {relative}")
        if path.is_file():
            if any(part in {"__pycache__", ".pytest_cache", "node_modules"} for part in relative.parts):
                findings.append(f"generated or dependency directory is bundled: {relative}")
            if path.stat().st_size > 1_000_000:
                findings.append(f"large bundled file requires justification: {relative}")
            try:
                content = path.read_text()
            except UnicodeDecodeError:
                continue
            if SECRET.search(content):
                findings.append(f"possible embedded secret: {relative}")
    evals = root / "evals"
    for name in ("cases.json", "routing-cases.json", "routes.json", "run-evals.py", "compare-evals.py", "release-decision.json", "results/status.json"):
        if not (evals / name).exists():
            findings.append(f"missing evaluation artifact: evals/{name}")
    try:
        cases_value = json.loads((evals / "cases.json").read_text())
        cases = cases_value.get("cases", []) if isinstance(cases_value, dict) else cases_value
        if len(cases) < 3:
            findings.append("evaluation package has fewer than three outcome cases")
        if any(case.get("split") not in {"development", "held-out"} for case in cases):
            findings.append("every outcome case must declare a development or held-out split")
    except (FileNotFoundError, json.JSONDecodeError, TypeError):
        findings.append("evaluation outcome cases are not valid JSON")
    try:
        routing = json.loads((evals / "routing-cases.json").read_text())
        positives = [case for case in routing if case.get("expected_activation") is True]
        negatives = [case for case in routing if case.get("expected_activation") is False]
        if len(positives) < 3 or len(negatives) < 3:
            findings.append("routing package requires at least three positive and three negative cases")
        if not any(case.get("risk") == "side-effect" for case in negatives):
            findings.append("routing package requires a negative side-effect safety case")
    except (FileNotFoundError, json.JSONDecodeError, TypeError):
        findings.append("evaluation routing cases are not valid JSON")
    try:
        routes = json.loads((evals / "routes.json").read_text())
        if set(routes.get("harnesses", {})) != {"codex", "claude", "pi"}:
            findings.append("routes must define Codex, Claude, and Pi")
        if routes.get("variants") != ["no-skill", "incumbent", "candidate"]:
            findings.append("routes must declare no-skill, incumbent, and candidate variants")
        fixed = set(routes.get("fixed", []))
        if not {"model", "effort", "tools", "permissions", "workspace", "verifier"} <= fixed:
            findings.append("routes do not fix all comparison controls")
        if routes.get("incumbent_available") and not (evals / "incumbent" / "SKILL.md").is_file():
            findings.append("incumbent route is enabled without an incumbent snapshot")
    except (FileNotFoundError, json.JSONDecodeError, TypeError):
        findings.append("evaluation routes are not valid JSON")
    try:
        result_status = json.loads((evals / "results" / "status.json").read_text())
        if set(result_status.get("variants", {})) != {"no-skill", "incumbent", "candidate"}:
            findings.append("result status must account for no-skill, incumbent, and candidate")
        if result_status.get("release_eligible") not in {True, False}:
            findings.append("result status must declare release_eligible")
    except (FileNotFoundError, json.JSONDecodeError, TypeError):
        findings.append("evaluation result status is not valid JSON")
    return findings


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("skill", type=Path)
    args = parser.parse_args()
    result = audit(args.skill.resolve())
    print(json.dumps({"pass": not result, "findings": result}, indent=2))
    raise SystemExit(1 if result else 0)
