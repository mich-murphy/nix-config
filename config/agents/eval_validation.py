"""Structural and referential integrity checks for the evaluation registry."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from eval_identity import definition_hashes


REQUIRED_MANIFEST_FIELDS = {
    "schema_version",
    "skill",
    "git_revision",
    "definition_hashes",
    "evaluation_identity",
    "mlflow",
    "owner_decision",
    "owner",
    "limitations",
    "supersedes",
    "protection",
    "release_eligible",
}


def _cases(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    return value.get("cases", []) if isinstance(value, dict) else value


def _selected_cases(eval_dir: Path, adapter: str, suite: str, mode: str) -> list[dict[str, Any]]:
    cases = _cases(eval_dir / ("routing-cases.json" if mode == "routing" else "cases.json"))
    if adapter == "neo":
        return [case for case in cases if case.get("smoke")] if suite == "smoke" else cases
    if suite == "smoke":
        return cases[:1]
    if suite in {"development", "held-out"}:
        return [case for case in cases if case.get("split", "development") == suite]
    return cases


def validate_result_matrix(
    eval_dir: Path, adapter: str, document: dict[str, Any]
) -> dict[str, Any]:
    """Require exactly the configured case/harness/variant/mode/repetition matrix."""
    configuration = document.get("configuration", {})
    suite = configuration.get("suite")
    repetitions = configuration.get("repetitions")
    harnesses = configuration.get("harnesses")
    variants = configuration.get("variants")
    modes = configuration.get("modes")
    if not (
        suite in {"smoke", "development", "held-out", "full"}
        and isinstance(repetitions, int) and repetitions > 0
        and isinstance(harnesses, list) and harnesses
        and isinstance(variants, list) and variants
    ):
        return {"valid": False, "expected": 0, "observed": len(document.get("results", [])), "missing": 0, "extra": 0, "duplicates": 0, "invalid": 0, "error": "incomplete configuration"}
    if adapter == "generic":
        matrix_modes = modes if isinstance(modes, list) and modes else []
    else:
        matrix_modes = ["end-to-end"]
    expected = {
        (case["id"], harness, variant, mode, repetition)
        for mode in matrix_modes
        for case in _selected_cases(eval_dir, adapter, suite, mode)
        for harness in harnesses
        for variant in variants
        for repetition in range(1, repetitions + 1)
    }
    observed_list = []
    invalid = 0
    for result in document.get("results", []):
        key = (
            result.get("id", result.get("case_id")),
            result.get("harness"),
            result.get("variant"),
            result.get("mode", "end-to-end"),
            result.get("repetition"),
        )
        observed_list.append(key)
        if result.get("valid") is not True:
            invalid += 1
    observed = set(observed_list)
    duplicates = len(observed_list) - len(observed)
    missing = expected - observed
    extra = observed - expected
    return {
        "valid": bool(expected) and not missing and not extra and not duplicates and not invalid,
        "expected": len(expected),
        "observed": len(observed_list),
        "missing": len(missing),
        "extra": len(extra),
        "duplicates": duplicates,
        "invalid": invalid,
    }


def validate(agent_root: Path) -> dict[str, Any]:
    errors: list[str] = []
    registry = json.loads((agent_root / "eval_registry.json").read_text(encoding="utf-8"))
    registered = registry.get("skills", {})
    package_names = {
        path.name for path in (agent_root / "skills").iterdir() if path.is_dir()
    }
    if set(registered) != package_names:
        errors.append(
            f"registry/package mismatch: registered={sorted(registered)} packages={sorted(package_names)}"
        )
    policy = json.loads((agent_root / "evaluation" / "policy.json").read_text(encoding="utf-8"))
    if policy.get("metadata_only_default") is not True or policy.get("content_capture_enabled") is not False:
        errors.append("central publication policy is not privacy-first")
    if policy.get("ordinary_retention_days") != 365 or policy.get("protected_supersession_grace_days") != 365:
        errors.append("central retention policy is not 365 days")
    for name, entry in sorted(registered.items()):
        eval_dir = agent_root / "skills" / name / "evals"
        for definition in ("cases.json", "routing-cases.json", "routes.json"):
            if not (eval_dir / definition).is_file():
                errors.append(f"{name}: missing {definition}")
        manifest_path = eval_dir / "release-manifest.json"
        if not manifest_path.is_file():
            errors.append(f"{name}: missing release-manifest.json")
            continue
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        missing = REQUIRED_MANIFEST_FIELDS - set(manifest)
        if missing:
            errors.append(f"{name}: manifest missing {sorted(missing)}")
        if manifest.get("skill") != name:
            errors.append(f"{name}: manifest skill mismatch")
        if manifest.get("owner_decision") not in {"defer", "adopt", "reject", "restrict"}:
            errors.append(f"{name}: invalid owner decision")
        observed_hashes = definition_hashes(eval_dir)
        if manifest.get("definition_hashes") != observed_hashes:
            errors.append(f"{name}: definition hashes do not match the package")
        if manifest.get("release_eligible") and (
            not manifest.get("evaluation_identity") or not manifest.get("mlflow")
        ):
            errors.append(f"{name}: release eligibility lacks joined evidence")
        generic = entry.get("adapter") == "generic"
        for facade in ("run-evals.py", "compare-evals.py"):
            exists = (eval_dir / facade).exists()
            if generic and exists:
                errors.append(f"{name}: obsolete generic facade remains: {facade}")
            if not generic and not exists:
                errors.append(f"{name}: specialized adapter asset is missing: {facade}")
        result_files = list((eval_dir / "results").glob("*.json")) if (eval_dir / "results").exists() else []
        if result_files:
            errors.append(f"{name}: generated result blobs remain in Git")
    return {"valid": not errors, "skills": len(registered), "errors": errors}
