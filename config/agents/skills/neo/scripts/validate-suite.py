#!/usr/bin/env python3
"""Validate the Neo skill suite and its cross-harness entry points."""

from __future__ import annotations

import re
import sys
from pathlib import Path


SKILLS = (
    "neo",
    "neo-discover",
    "neo-product",
    "neo-architecture",
    "neo-program",
    "neo-delivery",
    "neo-finalize",
)
SKILL_ROOT = Path(__file__).resolve().parents[2]
NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def parse_frontmatter(path: Path) -> tuple[dict[str, str], list[str]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "---":
        return {}, [f"{path}: missing opening frontmatter"]
    try:
        closing = lines.index("---", 1)
    except ValueError:
        return {}, [f"{path}: missing closing frontmatter"]
    fields = {}
    errors = []
    for number, line in enumerate(lines[1:closing], start=2):
        if ": " not in line:
            errors.append(f"{path}:{number}: malformed frontmatter")
            continue
        key, value = line.split(": ", 1)
        if key in fields:
            errors.append(f"{path}:{number}: duplicate field {key}")
        fields[key] = value.strip("\"'")
    return fields, errors


def validate_skill(skill_root: Path, name: str) -> list[str]:
    errors = []
    directory = skill_root / name
    skill_file = directory / "SKILL.md"
    metadata_file = directory / "agents" / "openai.yaml"
    if not skill_file.is_file():
        return [f"{skill_file}: missing"]
    fields, frontmatter_errors = parse_frontmatter(skill_file)
    errors.extend(frontmatter_errors)
    if set(fields) != {"name", "description"}:
        errors.append(f"{skill_file}: frontmatter must contain name and description")
    if fields.get("name") != name or not NAME_RE.fullmatch(name):
        errors.append(f"{skill_file}: name must match its kebab-case directory")
    description = fields.get("description", "").lower()
    if "use " not in description or "do not use" not in description:
        errors.append(f"{skill_file}: description needs positive and negative routing")
    if not metadata_file.is_file():
        errors.append(f"{metadata_file}: missing")
    else:
        metadata = metadata_file.read_text(encoding="utf-8")
        for field in ("display_name", "short_description", "default_prompt"):
            if not re.search(rf'^  {field}: "[^"]+"$', metadata, re.MULTILINE):
                errors.append(f"{metadata_file}: missing quoted {field}")
        if f"${name}" not in metadata:
            errors.append(f"{metadata_file}: default prompt must name ${name}")
        if "allow_implicit_invocation: false" not in metadata:
            errors.append(f"{metadata_file}: implicit invocation must be disabled")
    for target in LINK_RE.findall(skill_file.read_text(encoding="utf-8")):
        if "://" in target or target.startswith("#"):
            continue
        resolved = (skill_file.parent / target.split("#", 1)[0]).resolve()
        if not resolved.exists():
            errors.append(f"{skill_file}: broken local link {target}")
    return errors


def validate_suite(skill_root: Path = SKILL_ROOT) -> list[str]:
    errors = []
    for name in SKILLS:
        errors.extend(validate_skill(skill_root, name))
    neo = skill_root / "neo"
    for relative in (
        "scripts/neo.py",
        "scripts/validate-suite.py",
        "references/risk-routing.md",
        "references/interaction.md",
        "references/discovery.md",
        "references/product-design.md",
        "references/architecture.md",
        "references/program-design.md",
        "references/delivery.md",
        "references/prototype.md",
        "references/implementation-brief.md",
        "references/evaluation.md",
        "evals/cases.json",
        "evals/judges.json",
        "evals/run-evals.py",
        "evals/compare-evals.py",
        "evals/baseline.json",
        "evals/validate-judges.py",
    ):
        if not (neo / relative).is_file():
            errors.append(f"{neo / relative}: missing suite resource")
    return errors


def main() -> int:
    errors = validate_suite()
    for error in errors:
        print(error)
    if errors:
        print(f"{len(errors)} Neo suite error(s).")
        return 1
    print("Neo skill suite is valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
