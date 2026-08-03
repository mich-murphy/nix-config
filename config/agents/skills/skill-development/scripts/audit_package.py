#!/usr/bin/env python3
"""Fail on structural, provenance, telemetry, and package-safety defects."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


SECRET = re.compile(
    r"(?i)(sk-[a-z0-9_-]{16,}|api[_-]?key\s*[:=]\s*['\"][^'\"]+|password\s*[:=]\s*['\"][^'\"]+)"
)
REQUIRED_EVAL_FILES = (
    "cases.json",
    "routing-cases.json",
    "routes.json",
    "run-evals.py",
    "compare-evals.py",
    "release-decision.json",
    "telemetry-policy.json",
    "results/status.json",
)


def load_json(path: Path, findings: list[str]) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        findings.append(f"invalid JSON at {path.name}: {error}")
        return None


def audit(root: Path) -> list[str]:
    findings: list[str] = []
    skill = root / "SKILL.md"
    if not skill.is_file():
        return ["missing SKILL.md"]
    text = skill.read_text(encoding="utf-8")
    if len(text.splitlines()) > 500:
        findings.append("SKILL.md exceeds the 500-line review threshold")
    if not text.startswith("---\n") or text.count("---\n") < 2:
        findings.append("SKILL.md frontmatter is malformed")

    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(
            part in {"__pycache__", ".pytest_cache"}
            for part in relative.parts
        ):
            continue
        if path.is_symlink():
            try:
                path.resolve().relative_to(root.resolve())
            except ValueError:
                findings.append(f"symlink escapes package: {relative}")
        if not path.is_file():
            continue
        if "node_modules" in relative.parts:
            findings.append(f"generated or dependency directory is bundled: {relative}")
        if path.stat().st_size > 1_000_000:
            findings.append(f"large bundled file requires justification: {relative}")
        try:
            content = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        if SECRET.search(content):
            findings.append(f"possible embedded secret: {relative}")

    proposal = load_json(root / "proposal.json", findings)
    if isinstance(proposal, dict):
        if not proposal.get("evidence", {}).get("references"):
            findings.append("proposal needs at least one evidence reference")
        if not proposal.get("job"):
            findings.append("proposal needs one explicit job")
        if not proposal.get("negative_triggers"):
            findings.append("proposal needs negative triggers")
        if not proposal.get("completion_checks"):
            findings.append("proposal needs observable completion checks")

    evals = root / "evals"
    for name in REQUIRED_EVAL_FILES:
        if not (evals / name).exists():
            findings.append(f"missing evaluation artifact: evals/{name}")

    cases = load_json(evals / "cases.json", findings)
    if isinstance(cases, dict):
        cases = cases.get("cases", [])
    if isinstance(cases, list):
        if len(cases) < 3:
            findings.append("evaluation package has fewer than three outcome cases")
        if any(case.get("split") not in {"development", "held-out"} for case in cases):
            findings.append("every outcome case needs a development or held-out split")

    routing = load_json(evals / "routing-cases.json", findings)
    if isinstance(routing, list):
        positives = [case for case in routing if case.get("expected_activation") is True]
        negatives = [case for case in routing if case.get("expected_activation") is False]
        if len(positives) < 3 or len(negatives) < 3:
            findings.append("routing needs at least three positive and three negative cases")
        if not any(case.get("risk") == "side-effect" for case in negatives):
            findings.append("routing needs a negative side-effect safety case")

    routes = load_json(evals / "routes.json", findings)
    if isinstance(routes, dict):
        if set(routes.get("harnesses", {})) != {"codex", "claude", "pi"}:
            findings.append("routes must define Codex, Claude, and Pi")
        if routes.get("variants") != ["no-skill", "incumbent", "candidate"]:
            findings.append("routes must declare no-skill, incumbent, and candidate")
        fixed = set(routes.get("fixed", []))
        required = {"task", "model", "effort", "tools", "permissions", "workspace", "verifier"}
        if not required <= fixed:
            findings.append("routes do not fix all comparison controls")

    telemetry = load_json(evals / "telemetry-policy.json", findings)
    if isinstance(telemetry, dict):
        if telemetry.get("metadata_only_default") is not True:
            findings.append("telemetry must default to metadata-only capture")
        if telemetry.get("content_capture_enabled") is not False:
            findings.append("telemetry content capture must default off")
        lifecycle = set(telemetry.get("skill_lifecycle", []))
        if lifecycle != {"offered", "selected", "activated", "expanded", "executed", "evaluated"}:
            findings.append("telemetry does not cover the complete skill lifecycle")

    release = load_json(evals / "release-decision.json", findings)
    if isinstance(release, dict):
        if release.get("owner_decision") not in {"defer", "adopt", "reject", "restrict"}:
            findings.append("release decision has an unknown owner_decision")
        if set(release.get("quality_gate", {})) != {"functional", "regression", "integrity", "safety"}:
            findings.append("release decision is missing quality gates")

    status = load_json(evals / "results" / "status.json", findings)
    if isinstance(status, dict):
        if set(status.get("variants", {})) != {"no-skill", "incumbent", "candidate"}:
            findings.append("result status must account for all three variants")
        if status.get("release_eligible") not in {True, False}:
            findings.append("result status must declare release_eligible")
    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("skill", type=Path)
    args = parser.parse_args()
    findings = audit(args.skill.resolve())
    print(json.dumps({"pass": not findings, "findings": findings}, indent=2))
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
