"""Stable identities and definition provenance for skill evaluations."""

from __future__ import annotations

import hashlib
import json
import secrets
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "1.0.0"
EXCLUDED_PARTS = {"__pycache__", "results"}
EXCLUDED_FILES = {"release-manifest.json"}
EVALUATOR_FILES = (
    "eval_cli.py",
    "eval_compare.py",
    "eval_identity.py",
    "eval_publication.py",
    "eval_registry.json",
    "eval_runtime.py",
    "eval_validation.py",
    "evaluation/policy.json",
    "evaluation/release-manifest.schema.json",
    "telemetry/task_trace.py",
    "telemetry/publish_evals.py",
    "telemetry/protect_evals.py",
)


def sha256_path(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def definition_hashes(eval_dir: Path) -> dict[str, str]:
    return _tree_hashes(eval_dir)


def _tree_hashes(root: Path) -> dict[str, str]:
    return {
        path.relative_to(root).as_posix(): sha256_path(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
        and path.name not in EXCLUDED_FILES
        and not any(part in EXCLUDED_PARTS for part in path.relative_to(root).parts)
    }


def _aggregate_hash(hashes: dict[str, str]) -> str:
    return hashlib.sha256(
        json.dumps(hashes, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def skill_hash(eval_dir: Path) -> str:
    return _aggregate_hash(_tree_hashes(eval_dir.parent))


def evaluator_hashes(eval_dir: Path) -> dict[str, str]:
    agent_root = Path(__file__).resolve().parent
    hashes = {
        name: sha256_path(agent_root / name)
        for name in EVALUATOR_FILES
        if (agent_root / name).is_file()
    }
    for name in ("run-evals.py", "compare-evals.py"):
        path = eval_dir / name
        if path.is_file():
            hashes[f"specialized/{name}"] = sha256_path(path)
    return hashes


def evaluator_hash(eval_dir: Path) -> str:
    return _aggregate_hash(evaluator_hashes(eval_dir))


def build_identity(
    skill: str,
    eval_dir: Path,
    configuration: dict[str, Any],
    *,
    execution_id: str | None = None,
) -> dict[str, Any]:
    definitions = definition_hashes(eval_dir)
    execution = execution_id or secrets.token_hex(16)
    controls = {
        key: configuration.get(key)
        for key in ("suite", "repetitions", "harnesses", "variants", "modes", "routes", "fixed")
        if key in configuration
    }
    stable = {
        "schema_version": SCHEMA_VERSION,
        "skill": skill,
        "definition_hashes": definitions,
        "skill_hash": skill_hash(eval_dir),
        "evaluator_hash": evaluator_hash(eval_dir),
        "controls": controls,
        "execution_id": execution,
    }
    key = hashlib.sha256(
        json.dumps(stable, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return {
        **stable,
        "key": key,
        "execution_id": execution,
    }
