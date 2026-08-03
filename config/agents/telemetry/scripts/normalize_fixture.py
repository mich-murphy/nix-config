#!/usr/bin/env python3
"""Normalize content-free Codex, Claude, and Pi conformance fixtures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def normalize_codex(raw: dict) -> dict:
    parent = raw["task"]["span_id"]
    return {
        "task": {"id": raw["task"]["id"], "class": raw["task"]["class"], "risk": raw["task"]["risk"], "status": raw["task"]["status"]},
        "model": {"lane": raw["task"]["lane"], "effort": raw["task"]["effort"]},
        "skill": {key: raw["skill"][key] for key in ("name", "package_hash", "selection", "activation")},
        "tool": {"type": raw["tool"]["kind"], "status": raw["tool"]["status"]},
        "permission": {"decision": raw["permission"]["decision"]},
        "validation": {"type": raw["validation"]["kind"], "status": raw["validation"]["status"]},
        "outcome": {key: raw["outcome"][key] for key in ("status", "reference")},
        "repository": {"hash": raw["resource"]["repository_hash"], "base_revision": raw["resource"]["base_revision"]},
        "parentage": {name: raw[name]["parent_span_id"] == parent for name in ("skill", "tool", "permission", "validation", "outcome")},
    }


def normalize_claude(raw: dict) -> dict:
    task, skill = raw["interaction"], raw["skill_event"]
    return {
        "task": {"id": task["task_id"], "class": task["task_type"], "risk": task["risk_class"], "status": task["final_status"]},
        "model": {"lane": task["lane"], "effort": task["effort"]},
        "skill": {"name": skill["skill.name"], "package_hash": skill["skill.hash"], "selection": skill["source"], "activation": skill["state"]},
        "tool": {"type": raw["tool_event"]["tool_type"], "status": raw["tool_event"]["result"]},
        "permission": {"decision": raw["permission_event"]["result"]},
        "validation": {"type": raw["validation_event"]["validation_type"], "status": raw["validation_event"]["result"]},
        "outcome": {"status": raw["outcome_event"]["result"], "reference": raw["outcome_event"]["ref"]},
        "repository": {"hash": raw["resource"]["repo"], "base_revision": raw["resource"]["revision"]},
        "parentage": {name: raw[f"{name}_event"]["parent"] == task["span_id"] for name in ("skill", "tool", "permission", "validation", "outcome")},
    }


def span(raw: dict, name: str) -> dict:
    return next(item["attributes"] for item in raw["spans"] if item["name"] == name)


def parented(raw: dict, name: str) -> bool:
    root = next(item for item in raw["spans"] if item["name"] == "agent.task")
    child = next(item for item in raw["spans"] if item["name"] == name)
    return child.get("parent_span_id") == root["span_id"]


def normalize_pi(raw: dict) -> dict:
    task, skill = span(raw, "agent.task"), span(raw, "skill.activate")
    tool, validation, outcome = span(raw, "tool.execute"), span(raw, "validation.run"), span(raw, "outcome.record")
    permission = span(raw, "permission.wait")
    return {
        "task": {"id": task["app.agent.task.id"], "class": task["app.agent.task.class"], "risk": task["app.agent.risk.class"], "status": task["app.agent.final.status"]},
        "model": {"lane": task["app.agent.model.lane"], "effort": task["app.agent.model.effort"]},
        "skill": {"name": skill["app.agent.skill.name"], "package_hash": skill["app.agent.skill.package_hash"], "selection": skill["app.agent.skill.selection"], "activation": skill["app.agent.skill.activation"]},
        "tool": {"type": tool["app.agent.tool.type"], "status": tool["app.agent.tool.status"]},
        "permission": {"decision": permission["app.agent.permission.decision"]},
        "validation": {"type": validation["app.agent.validation.type"], "status": validation["app.agent.validation.status"]},
        "outcome": {"status": outcome["app.agent.outcome.status"], "reference": outcome["app.agent.outcome.reference"]},
        "repository": {"hash": raw["resource"]["app.agent.repository.hash"], "base_revision": raw["resource"]["app.agent.repository.base_revision"]},
        "parentage": {"skill": parented(raw, "skill.activate"), "tool": parented(raw, "tool.execute"), "permission": parented(raw, "permission.wait"), "validation": parented(raw, "validation.run"), "outcome": parented(raw, "outcome.record")},
    }


NORMALIZERS = {"codex": normalize_codex, "claude": normalize_claude, "pi": normalize_pi}


def normalize(path: Path) -> dict:
    raw = json.loads(path.read_text())
    return NORMALIZERS[raw["harness"]](raw)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("fixture", type=Path)
    args = parser.parse_args()
    print(json.dumps(normalize(args.fixture), indent=2, sort_keys=True))
