#!/usr/bin/env python3
"""Validate a tasks graph and its Ship task-plan files."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections.abc import Callable
from pathlib import Path
from typing import Any


TASK_ID = re.compile(r"^[a-z0-9][a-z0-9-]*$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REQUIRED_HEADINGS = (
    "## Outcome",
    "## Acceptance Criteria",
    "## Non-goals",
    "## Preserved Decisions and Invariants",
    "## Repository Evidence and Scope",
    "## Dependencies and Preconditions",
    "## Implementation Guidance",
    "## Verification",
    "## Compatibility, Rollout, and Recovery",
    "## Context Budget",
    "## Replan Triggers",
)
TASK_LIST_FIELDS = (
    "plan_refs",
    "requirements",
    "non_goals",
    "acceptance_criteria",
    "repo_evidence",
    "replan_triggers",
)


def nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def nonempty_strings(value: Any) -> bool:
    return (
        isinstance(value, list)
        and bool(value)
        and all(nonempty_string(item) for item in value)
    )


def load_json(path: Path, findings: list[str]) -> dict[str, Any] | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        findings.append(f"cannot read graph: {error}")
        return None
    if not isinstance(value, dict):
        findings.append("graph root must be an object")
        return None
    return value


def validate_source_plan(
    graph: dict[str, Any], graph_path: Path, findings: list[str]
) -> None:
    source = graph.get("source_plan")
    if not isinstance(source, dict):
        findings.append("source_plan must be an object")
        return
    source_path = source.get("path")
    digest = source.get("sha256")
    if not nonempty_string(source_path):
        findings.append("source_plan.path must be non-empty")
    if not isinstance(digest, str) or not SHA256.fullmatch(digest):
        findings.append("source_plan.sha256 must be a lowercase SHA-256")
        return
    if not nonempty_string(source_path):
        return
    candidate = Path(source_path)
    if not candidate.is_absolute():
        candidate = (graph_path.parent / candidate).resolve()
    if not candidate.is_file():
        findings.append(f"source plan does not exist: {source_path}")
        return
    actual = hashlib.sha256(candidate.read_bytes()).hexdigest()
    if actual != digest:
        findings.append("source plan content does not match source_plan.sha256")


def validate_context_policy(graph: dict[str, Any], findings: list[str]) -> None:
    policy = graph.get("context_policy")
    if not isinstance(policy, dict):
        findings.append("context_policy must be an object")
        return
    window = policy.get("window_tokens")
    warning = policy.get("warning_tokens")
    reserve = policy.get("reserve_percent")
    if not isinstance(window, int) or isinstance(window, bool) or window <= 0:
        findings.append("context_policy.window_tokens must be a positive integer")
    if not isinstance(warning, int) or isinstance(warning, bool) or warning <= 0:
        findings.append("context_policy.warning_tokens must be a positive integer")
    if (
        not isinstance(reserve, int)
        or isinstance(reserve, bool)
        or not 0 <= reserve < 100
    ):
        findings.append(
            "context_policy.reserve_percent must be an integer from 0 to 99"
        )
    if isinstance(window, int) and isinstance(warning, int) and warning > window:
        findings.append("context warning cannot exceed the window")
    if (
        isinstance(window, int)
        and not isinstance(window, bool)
        and isinstance(warning, int)
        and not isinstance(warning, bool)
        and isinstance(reserve, int)
        and not isinstance(reserve, bool)
        and 0 <= reserve < 100
        and warning > window * (100 - reserve) / 100
    ):
        findings.append("context warning violates the declared reserve")
    if not nonempty_string(policy.get("basis")):
        findings.append("context_policy.basis must explain the policy source")


def validate_dependencies(
    task: dict[str, Any], task_id: Any, label: str, ids: set[str],
    findings: list[str]
) -> None:
    dependencies = task.get("depends_on")
    if not isinstance(dependencies, list) or not all(
        isinstance(item, str) for item in dependencies
    ):
        findings.append(f"task {label}.depends_on must be a string list")
    else:
        for dependency in dependencies:
            if dependency not in ids:
                findings.append(f"task {label} depends on unknown task {dependency}")
            if dependency == task_id:
                findings.append(f"task {label} cannot depend on itself")


def validate_task_section(
    task: dict[str, Any], label: str, section_name: str,
    fields: tuple[str, ...], field_is_valid: Callable[[Any], bool],
    findings: list[str]
) -> None:
    section = task.get(section_name)
    if not isinstance(section, dict):
        findings.append(f"task {label}.{section_name} must be an object")
        return
    for field in fields:
        if not field_is_valid(section.get(field)):
            findings.append(f"task {label}.{section_name}.{field} must be non-empty")


def validate_context_budget(
    task: dict[str, Any], label: str, policy_warning: int | None,
    findings: list[str]
) -> None:
    budget = task.get("context_budget")
    if not isinstance(budget, dict):
        findings.append(f"task {label}.context_budget must be an object")
        return
    warning = budget.get("warning_tokens")
    if not isinstance(warning, int) or isinstance(warning, bool) or warning <= 0:
        findings.append(f"task {label} needs a positive context warning")
    elif policy_warning is not None and warning != policy_warning:
        findings.append(f"task {label} context warning differs from graph policy")
    if budget.get("assessment") not in {"well-below-warning", "within-warning"}:
        findings.append(f"task {label} is not assessed within the warning")
    if budget.get("confidence") not in {"high", "medium"}:
        findings.append(f"task {label} needs high or medium sizing confidence")
    for field in ("drivers", "split_triggers"):
        if not nonempty_strings(budget.get(field)):
            findings.append(f"task {label}.context_budget.{field} must be non-empty")


def validate_plan_file(
    task: dict[str, Any], label: str, graph_path: Path, findings: list[str]
) -> None:
    plan_file = task.get("plan_file")
    if not nonempty_string(plan_file):
        findings.append(f"task {label}.plan_file must be non-empty")
        return
    candidate = Path(plan_file)
    if candidate.is_absolute():
        findings.append(f"task {label}.plan_file must be relative to the graph")
        return
    candidate = (graph_path.parent / candidate).resolve()
    try:
        candidate.relative_to(graph_path.parent.resolve())
    except ValueError:
        findings.append(f"task {label}.plan_file escapes the graph directory")
        return
    if not candidate.is_file():
        findings.append(f"task {label}.plan_file does not exist: {plan_file}")
        return
    text = candidate.read_text(encoding="utf-8")
    for heading in REQUIRED_HEADINGS:
        if heading not in text:
            findings.append(f"task {label}.plan_file is missing heading: {heading}")


def validate_task_shape(
    task: dict[str, Any], ids: set[str], policy_warning: int | None,
    graph_path: Path, findings: list[str]
) -> None:
    task_id = task.get("id")
    label: str = task_id if nonempty_string(task_id) else "<unknown>"
    if not isinstance(task_id, str) or not TASK_ID.fullmatch(task_id):
        findings.append(f"task {label} has an invalid id")
    if not nonempty_string(task.get("title")):
        findings.append(f"task {label} needs a title")
    if not nonempty_string(task.get("outcome")):
        findings.append(f"task {label} needs one observable outcome")
    for field in TASK_LIST_FIELDS:
        if not nonempty_strings(task.get(field)):
            findings.append(f"task {label}.{field} must be a non-empty string list")

    validate_dependencies(task, task_id, label, ids, findings)
    validate_task_section(
        task, label, "scope", ("likely_touch", "must_not_touch"),
        nonempty_strings, findings,
    )
    validate_task_section(
        task, label, "verification", ("focused", "broader", "real_interface"),
        nonempty_strings, findings,
    )
    validate_task_section(
        task, label, "compatibility",
        ("migration", "rollout", "rollback", "documentation"),
        nonempty_string, findings,
    )
    validate_context_budget(task, label, policy_warning, findings)
    validate_plan_file(task, label, graph_path, findings)


def validate_coverage(
    graph: dict[str, Any], tasks: list[dict[str, Any]], task_ids: set[str],
    findings: list[str]
) -> None:
    coverage = graph.get("coverage")
    if not isinstance(coverage, list) or not coverage:
        findings.append("coverage must contain every requirement and non-goal")
        return
    seen: set[tuple[str, str]] = set()
    ownership: set[tuple[str, str, str]] = set()
    for index, item in enumerate(coverage):
        if not isinstance(item, dict):
            findings.append(f"coverage[{index}] must be an object")
            continue
        kind = item.get("kind")
        text = item.get("text")
        owners = item.get("tasks")
        if kind not in {"requirement", "non-goal", "verification", "integration"}:
            findings.append(f"coverage[{index}].kind is invalid")
        if not nonempty_string(text):
            findings.append(f"coverage[{index}].text must be non-empty")
        elif isinstance(kind, str):
            key = (kind, text.strip())
            if key in seen:
                findings.append(f"duplicate coverage entry: {kind} {text.strip()}")
            seen.add(key)
        if not isinstance(owners, list) or not owners:
            findings.append(f"coverage[{index}].tasks must own the item")
        elif any(owner not in task_ids for owner in owners):
            findings.append(f"coverage[{index}] refers to an unknown task")
        elif isinstance(kind, str) and nonempty_string(text):
            ownership.update((kind, text.strip(), owner) for owner in owners)

    for task in tasks:
        task_id = task.get("id")
        if not isinstance(task_id, str):
            continue
        for kind, field in (("requirement", "requirements"), ("non-goal", "non_goals")):
            values = task.get(field)
            if not isinstance(values, list):
                continue
            for value in values:
                if (
                    isinstance(value, str)
                    and (kind, value.strip(), task_id) not in ownership
                ):
                    findings.append(
                        f"task {task_id}.{field} item lacks matching coverage: {value}"
                    )


def validate_acyclic(tasks: list[dict[str, Any]], findings: list[str]) -> None:
    dependencies = {
        task.get("id"): task.get("depends_on", [])
        for task in tasks
        if isinstance(task.get("id"), str)
        and isinstance(task.get("depends_on"), list)
    }
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(task_id: str) -> None:
        if task_id in visited:
            return
        if task_id in visiting:
            findings.append(f"dependency cycle includes task {task_id}")
            return
        visiting.add(task_id)
        for dependency in dependencies.get(task_id, []):
            if dependency in dependencies:
                visit(dependency)
        visiting.remove(task_id)
        visited.add(task_id)

    for task_id in dependencies:
        visit(task_id)


def validate(graph_path: Path) -> list[str]:
    findings: list[str] = []
    graph = load_json(graph_path, findings)
    if graph is None:
        return findings
    if graph.get("schema_version") != "1.0.0":
        findings.append("schema_version must be 1.0.0")
    validate_source_plan(graph, graph_path, findings)
    validate_context_policy(graph, findings)

    tasks = graph.get("tasks")
    if not isinstance(tasks, list) or not tasks:
        findings.append("tasks must be a non-empty list")
        return findings
    if not all(isinstance(task, dict) for task in tasks):
        findings.append("every task must be an object")
        return findings
    task_ids = [task.get("id") for task in tasks if isinstance(task.get("id"), str)]
    ids = set(task_ids)
    if len(ids) != len(task_ids):
        findings.append("task ids must be unique")
    policy = graph.get("context_policy")
    policy_warning = policy.get("warning_tokens") if isinstance(policy, dict) else None
    if not isinstance(policy_warning, int) or isinstance(policy_warning, bool):
        policy_warning = None
    for task in tasks:
        validate_task_shape(task, ids, policy_warning, graph_path, findings)
    validate_coverage(graph, tasks, ids, findings)
    validate_acyclic(tasks, findings)
    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("graph", type=Path)
    args = parser.parse_args()
    graph_path = args.graph.resolve()
    findings = validate(graph_path)
    print(json.dumps({"pass": not findings, "findings": findings}, indent=2))
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
