#!/usr/bin/env python3
"""Deterministic state and validation engine for the Neo planning suite."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
from dataclasses import dataclass
from datetime import UTC, datetime
from enum import Enum
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
STAGES = (
    "discover",
    "product",
    "architecture",
    "program",
    "delivery",
    "finalize",
)

RISK_TO_STAGE = {
    "problem-uncertain": "product",
    "outcome-uncertain": "product",
    "user-uncertain": "product",
    "interaction-uncertain": "product",
    "system-boundary": "architecture",
    "trust-boundary": "architecture",
    "public-contract": "architecture",
    "persistent-data": "architecture",
    "security": "architecture",
    "reliability": "architecture",
    "deployment": "architecture",
    "compatibility": "architecture",
    "expensive-reversal": "architecture",
    "new-abstraction": "program",
    "domain-invariant": "program",
    "state-machine": "program",
    "concurrency": "program",
    "consequential-interface": "program",
    "data-structure": "program",
    "multi-module-call-path": "program",
}

FINAL_HEADINGS = (
    "Intent",
    "Requirements and Non-goals",
    "Current-state Evidence",
    "Product Scenarios",
    "Architecture",
    "Program Design",
    "Delivery Slices",
    "Verification",
    "Compatibility, Rollout, and Recovery",
    "Assumptions, Risks, and Replan Triggers",
)

STAGE_REQUIRED_TERMS = {
    "discover": ("Fact", "Unknown", "Current state", "Success"),
    "product": ("Outcome", "Scenario", "Non-goal", "Assumption"),
    "architecture": ("Quality scenario", "Data flow", "Failure", "Alternative"),
    "program": ("Interface", "Invariant", "Call path", "Ownership"),
    "delivery": ("Slice", "Verifier", "Dependency", "Replan"),
}


class NeoError(ValueError):
    """A user-actionable Neo state or validation error."""


class StageStatus(str, Enum):
    SKIPPED = "skipped"
    PENDING = "pending"
    ACTIVE = "active"
    APPROVED = "approved"
    STALE = "stale"
    REVIEW = "review"


class FeedbackKind(str, Enum):
    APPROVE = "approve"
    CLARIFY = "clarify"
    CHANGE = "change"
    REJECT = "reject"


class PrototypeKind(str, Enum):
    VISUAL = "visual"
    LOGICAL = "logical"
    NONE = "none"
    TRACER = "tracer"


@dataclass(frozen=True)
class TaskLocation:
    root: Path
    slug: str

    @property
    def directory(self) -> Path:
        return self.root / ".neo" / "tasks" / self.slug

    @property
    def state_file(self) -> Path:
        return self.directory / "state.json"


def utc_now() -> str:
    return datetime.now(UTC).replace(microsecond=0).isoformat()


def slug_value(value: str) -> str:
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", value):
        raise NeoError("task slug must use lowercase kebab-case")
    return value


def split_csv(value: str | None) -> list[str]:
    if not value:
        return []
    return [item.strip() for item in value.split(",") if item.strip()]


def validated_signals(value: str | None) -> list[str]:
    signals = sorted(set(split_csv(value)))
    invalid = sorted(set(signals) - set(RISK_TO_STAGE))
    if invalid:
        raise NeoError(f"unknown risk signals: {', '.join(invalid)}")
    return signals


def atomic_write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary_path = Path(temporary)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, path)
    finally:
        if temporary_path.exists():
            temporary_path.unlink()


def load_state(location: TaskLocation) -> dict[str, Any]:
    if not location.state_file.is_file():
        raise NeoError(f"missing Neo task state: {location.state_file}")
    try:
        state = json.loads(location.state_file.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise NeoError(f"invalid JSON state: {error}") from error
    validate_state(state)
    return state


def save_state(location: TaskLocation, state: dict[str, Any]) -> None:
    state["updated_at"] = utc_now()
    validate_state(state)
    atomic_write_json(location.state_file, state)


def validate_state(state: dict[str, Any]) -> None:
    required = {
        "schema_version",
        "task",
        "risk_signals",
        "route",
        "stages",
        "decisions",
        "unknowns",
        "prototypes",
        "feedback",
        "artifacts",
        "final",
        "created_at",
        "updated_at",
    }
    unknown = set(state) - required
    missing = required - set(state)
    if missing or unknown:
        raise NeoError(
            f"state fields mismatch; missing={sorted(missing)}, unknown={sorted(unknown)}"
        )
    if state["schema_version"] != SCHEMA_VERSION:
        raise NeoError(
            f"unsupported state schema {state['schema_version']}; "
            f"expected {SCHEMA_VERSION}"
        )
    if state["route"] not in {"unassessed", "direct", "focused", "full"}:
        raise NeoError(f"invalid route: {state['route']}")
    if set(state["stages"]) != set(STAGES):
        raise NeoError("state must contain every Neo stage exactly once")
    if state["route"] == "direct" and any(
        item["required"] for item in state["stages"].values()
    ):
        raise NeoError("direct route cannot require Neo stages")
    if state["route"] in {"focused", "full"} and not all(
        state["stages"][stage]["required"]
        for stage in ("discover", "delivery", "finalize")
    ):
        raise NeoError("planned routes require discovery, delivery, and finalization")
    valid_statuses = {item.value for item in StageStatus}
    for stage, item in state["stages"].items():
        if set(item) != {"required", "status", "artifact", "approved_version"}:
            raise NeoError(f"invalid stage record: {stage}")
        if not isinstance(item["required"], bool) or item["status"] not in valid_statuses:
            raise NeoError(f"invalid stage status: {stage}")
    decision_ids = set()
    for decision in state["decisions"]:
        decision_id = decision.get("id")
        if not decision_id or decision_id in decision_ids:
            raise NeoError(f"duplicate or missing decision id: {decision_id}")
        decision_ids.add(decision_id)
        if decision.get("stage") not in STAGES:
            raise NeoError(f"invalid decision stage: {decision.get('stage')}")
        if decision.get("status") not in {"approved", "superseded"}:
            raise NeoError(f"invalid decision status: {decision.get('status')}")
    for unknown_item in state["unknowns"]:
        if unknown_item.get("stage") not in STAGES:
            raise NeoError("unknown item has invalid stage")
        if unknown_item.get("status") not in {"open", "resolved"}:
            raise NeoError("unknown item has invalid status")
    final = state["final"]
    if set(final) != {"status", "artifact", "sha256", "approved_at"}:
        raise NeoError("invalid final record")
    if final["status"] not in {"absent", "review", "stale", "approved"}:
        raise NeoError("invalid final status")


def required_stages(state: dict[str, Any]) -> list[str]:
    return [stage for stage in STAGES if state["stages"][stage]["required"]]


def next_stage(state: dict[str, Any]) -> str | None:
    for stage in required_stages(state):
        status = state["stages"][stage]["status"]
        if status in {
            StageStatus.PENDING,
            StageStatus.ACTIVE,
            StageStatus.STALE,
            StageStatus.REVIEW,
        }:
            return stage
    return None


def stage_index(stage: str) -> int:
    try:
        return STAGES.index(stage)
    except ValueError as error:
        raise NeoError(f"unknown Neo stage: {stage}") from error


def ensure_current_stage(state: dict[str, Any], stage: str) -> None:
    if not state["stages"][stage]["required"]:
        raise NeoError(f"stage is not required by the confirmed route: {stage}")
    current = next_stage(state)
    if current != stage:
        raise NeoError(f"expected current stage {current!r}, received {stage!r}")


def artifact_hash(path: Path) -> str:
    if not path.is_file():
        raise NeoError(f"artifact does not exist: {path}")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact_version(path: Path) -> str:
    return f"sha256:{artifact_hash(path)}"


def word_count(text: str) -> int:
    return len(re.findall(r"\b[\w'-]+\b", text))


def validate_decision_card(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    errors = []
    for heading in (
        "Decision",
        "Why now",
        "Evidence",
        "Affected interface or flow",
        "Options",
        "Recommendation",
        "Approval question",
    ):
        if not re.search(rf"^##? {re.escape(heading)}\s*$", text, re.MULTILINE):
            errors.append(f"missing decision-card heading: {heading}")
    if word_count(text) > 250:
        errors.append("decision card exceeds 250 words")
    return errors


def validate_stage_artifact(stage: str, path: Path) -> list[str]:
    if not path.is_file():
        return [f"artifact does not exist: {path}"]
    text = path.read_text(encoding="utf-8")
    errors = []
    for term in STAGE_REQUIRED_TERMS.get(stage, ()):
        if term.casefold() not in text.casefold():
            errors.append(f"{stage} artifact does not mention required concept: {term}")
    return errors


def validate_final_brief(path: Path) -> list[str]:
    if not path.is_file():
        return [f"artifact does not exist: {path}"]
    text = path.read_text(encoding="utf-8")
    errors = []
    for heading in FINAL_HEADINGS:
        if not re.search(rf"^## {re.escape(heading)}\s*$", text, re.MULTILINE):
            errors.append(f"missing final-brief heading: {heading}")
    if "Verifier" not in text:
        errors.append("final brief must name slice verifiers")
    if "Call path" not in text and "call path" not in text:
        errors.append("final brief must describe a call path")
    return errors


def initial_state(location: TaskLocation, title: str) -> dict[str, Any]:
    if location.state_file.exists():
        raise NeoError(f"Neo task already exists: {location.slug}")
    now = utc_now()
    return {
        "schema_version": SCHEMA_VERSION,
        "task": {"slug": location.slug, "title": title},
        "risk_signals": [],
        "route": "unassessed",
        "stages": {
            stage: {
                "required": stage in {"discover", "delivery", "finalize"},
                "status": (
                    StageStatus.PENDING
                    if stage == "discover"
                    else StageStatus.SKIPPED
                ),
                "artifact": None,
                "approved_version": None,
            }
            for stage in STAGES
        },
        "decisions": [],
        "unknowns": [],
        "prototypes": [],
        "feedback": [],
        "artifacts": [],
        "final": {
            "status": "absent",
            "artifact": None,
            "sha256": None,
            "approved_at": None,
        },
        "created_at": now,
        "updated_at": now,
    }


def apply_assessment(state: dict[str, Any], signals: list[str]) -> None:
    design_stages = {RISK_TO_STAGE[signal] for signal in signals}
    required = (
        {"discover", "delivery", "finalize"} | design_stages
        if signals
        else set()
    )
    for stage in STAGES:
        record = state["stages"][stage]
        record["required"] = stage in required
        if stage not in required:
            record["status"] = StageStatus.SKIPPED
        elif record["status"] == StageStatus.SKIPPED:
            record["status"] = StageStatus.PENDING
    state["risk_signals"] = signals
    if not signals:
        state["route"] = "direct"
    elif design_stages == {"product", "architecture", "program"}:
        state["route"] = "full"
    else:
        state["route"] = "focused"


def cmd_init(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = initial_state(location, args.title)
    save_state(location, state)
    return state


def cmd_assess(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    apply_assessment(state, validated_signals(args.signals))
    save_state(location, state)
    return state


def cmd_start(args: argparse.Namespace) -> dict[str, Any]:
    """Initialize, assess, and produce the first handoff in one local operation."""
    location = TaskLocation(args.root, slug_value(args.slug))
    signals = validated_signals(args.signals)
    state = initial_state(location, args.title)
    apply_assessment(state, signals)
    save_state(location, state)
    return {
        "task": state["task"],
        "route": state["route"],
        "risk_signals": state["risk_signals"],
        "current_stage": next_stage(state),
        "state_file": str(location.state_file),
    }


def cmd_status(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    return {
        "task": state["task"],
        "route": state["route"],
        "risk_signals": state["risk_signals"],
        "current_stage": next_stage(state),
        "stages": state["stages"],
        "open_unknowns": [
            item for item in state["unknowns"] if item["status"] == "open"
        ],
        "final": state["final"],
    }


def cmd_record_decision(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    ensure_current_stage(state, args.stage)
    existing = {item["id"] for item in state["decisions"]}
    if args.id in existing:
        raise NeoError(f"decision id already exists: {args.id}")
    dependencies = split_csv(args.depends_on)
    missing = sorted(set(dependencies) - existing)
    if missing:
        raise NeoError(f"unknown decision dependencies: {', '.join(missing)}")
    superseded = None
    if args.supersedes:
        superseded = next(
            (item for item in state["decisions"] if item["id"] == args.supersedes),
            None,
        )
        if not superseded:
            raise NeoError(f"superseded decision does not exist: {args.supersedes}")
        if superseded["status"] != "superseded":
            raise NeoError(
                f"replacement target is still active: {args.supersedes}"
            )
    decision = {
        "id": args.id,
        "stage": args.stage,
        "summary": args.summary,
        "choice": args.choice,
        "rationale": args.rationale,
        "depends_on": dependencies,
        "status": "approved",
        "superseded_by": None,
        "recorded_at": utc_now(),
    }
    state["decisions"].append(decision)
    if superseded:
        superseded["superseded_by"] = args.id
    save_state(location, state)
    return decision


def cmd_record_unknown(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    if any(item["id"] == args.id for item in state["unknowns"]):
        raise NeoError(f"unknown id already exists: {args.id}")
    item = {
        "id": args.id,
        "stage": args.stage,
        "question": args.question,
        "blocking": args.blocking,
        "status": "open",
        "resolution": None,
    }
    state["unknowns"].append(item)
    save_state(location, state)
    return item


def cmd_resolve_unknown(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    for item in state["unknowns"]:
        if item["id"] == args.id:
            item["status"] = "resolved"
            item["resolution"] = args.resolution
            save_state(location, state)
            return item
    raise NeoError(f"unknown item does not exist: {args.id}")


def cmd_record_prototype(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    kind = PrototypeKind(args.kind)
    if kind in {PrototypeKind.VISUAL, PrototypeKind.LOGICAL}:
        if not args.question or not args.evidence or not args.disposition:
            raise NeoError(
                "visual and logical prototypes require question, evidence, and disposition"
            )
    prototype = {
        "kind": kind,
        "question": args.question,
        "evidence": args.evidence,
        "disposition": args.disposition,
        "recorded_at": utc_now(),
    }
    state["prototypes"].append(prototype)
    save_state(location, state)
    return prototype


def open_blockers(state: dict[str, Any], stage: str) -> list[dict[str, Any]]:
    limit = stage_index(stage)
    return [
        item
        for item in state["unknowns"]
        if item["status"] == "open"
        and item["blocking"]
        and stage_index(item["stage"]) <= limit
    ]


def cmd_gate(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    ensure_current_stage(state, args.stage)
    if args.stage == "finalize":
        raise NeoError("use finalize and approve for the final stage")
    blockers = open_blockers(state, args.stage)
    if blockers:
        raise NeoError(
            "blocking unknowns remain: " + ", ".join(item["id"] for item in blockers)
        )
    artifact = Path(args.artifact).resolve()
    errors = validate_stage_artifact(args.stage, artifact)
    if errors:
        raise NeoError("; ".join(errors))
    record = state["stages"][args.stage]
    record["status"] = StageStatus.APPROVED
    record["artifact"] = str(artifact)
    record["approved_version"] = artifact_version(artifact)
    state["artifacts"].append(
        {
            "stage": args.stage,
            "path": str(artifact),
            "version": record["approved_version"],
            "retention": "regenerable",
        }
    )
    following = next_stage(state)
    if following:
        state["stages"][following]["status"] = StageStatus.ACTIVE
    save_state(location, state)
    return {"approved": args.stage, "next_stage": following}


def invalidate_from(state: dict[str, Any], stage: str) -> list[str]:
    invalidated = []
    start = stage_index(stage)
    for affected in STAGES[start:]:
        record = state["stages"][affected]
        if not record["required"]:
            continue
        record["status"] = (
            StageStatus.PENDING if affected == stage else StageStatus.STALE
        )
        record["approved_version"] = None
        invalidated.append(affected)
    if state["final"]["status"] != "absent":
        state["final"]["status"] = "stale"
        state["final"]["approved_at"] = None
    return invalidated


def cmd_record_feedback(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    kind = FeedbackKind(args.kind)
    decision_ids = split_csv(args.decisions)
    feedback = {
        "kind": kind,
        "message": args.message,
        "stage": args.stage,
        "decisions": decision_ids,
        "recorded_at": utc_now(),
        "invalidated": [],
    }
    if kind == FeedbackKind.CHANGE:
        if not decision_ids:
            raise NeoError("change feedback requires affected decision ids")
        decisions = {
            item["id"]: item for item in state["decisions"] if item["status"] == "approved"
        }
        missing = sorted(set(decision_ids) - set(decisions))
        if missing:
            raise NeoError(f"unknown active decisions: {', '.join(missing)}")
        earliest = min(
            (decisions[decision_id]["stage"] for decision_id in decision_ids),
            key=stage_index,
        )
        for decision_id in decision_ids:
            decisions[decision_id]["status"] = "superseded"
        feedback["invalidated"] = invalidate_from(state, earliest)
    elif kind == FeedbackKind.REJECT:
        if not args.stage:
            raise NeoError("reject feedback requires a stage")
        feedback["invalidated"] = invalidate_from(state, args.stage)
    elif kind == FeedbackKind.APPROVE:
        if args.stage == "finalize":
            raise NeoError("use approve to approve the versioned final brief")
    state["feedback"].append(feedback)
    save_state(location, state)
    return feedback


def cmd_revise(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    ensure_current_stage(state, args.stage)
    state["stages"][args.stage]["status"] = StageStatus.ACTIVE
    save_state(location, state)
    return {"active_stage": args.stage}


def cmd_handoff(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    current = next_stage(state)
    if args.expect and current != args.expect:
        raise NeoError(f"handoff expected {args.expect!r}, current stage is {current!r}")
    return {
        "task": state["task"],
        "route": state["route"],
        "current_stage": current,
        "state_file": str(location.state_file),
        "approved_artifacts": [
            {
                "stage": stage,
                "path": item["artifact"],
                "version": item["approved_version"],
            }
            for stage, item in state["stages"].items()
            if item["status"] == StageStatus.APPROVED
        ],
        "open_unknowns": [
            item for item in state["unknowns"] if item["status"] == "open"
        ],
    }


def cmd_finalize(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    ensure_current_stage(state, "finalize")
    blockers = [item for item in state["unknowns"] if item["status"] == "open" and item["blocking"]]
    if blockers:
        raise NeoError(
            "blocking unknowns remain: " + ", ".join(item["id"] for item in blockers)
        )
    for stage in required_stages(state):
        if stage == "finalize":
            continue
        if state["stages"][stage]["status"] != StageStatus.APPROVED:
            raise NeoError(f"required stage is not approved: {stage}")
    artifact = Path(args.artifact).resolve()
    errors = validate_final_brief(artifact)
    if errors:
        raise NeoError("; ".join(errors))
    digest = artifact_hash(artifact)
    state["stages"]["finalize"]["status"] = StageStatus.REVIEW
    state["stages"]["finalize"]["artifact"] = str(artifact)
    state["final"] = {
        "status": "review",
        "artifact": str(artifact),
        "sha256": digest,
        "approved_at": None,
    }
    state["artifacts"].append(
        {
            "stage": "finalize",
            "path": str(artifact),
            "version": f"sha256:{digest}",
            "retention": "durable",
        }
    )
    save_state(location, state)
    return state["final"]


def cmd_approve(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    final = state["final"]
    if final["status"] != "review":
        raise NeoError("final brief is not awaiting review")
    artifact = Path(final["artifact"])
    digest = artifact_hash(artifact)
    if digest != final["sha256"]:
        state["final"]["status"] = "stale"
        state["stages"]["finalize"]["status"] = StageStatus.STALE
        save_state(location, state)
        raise NeoError("final brief changed after review began; finalize it again")
    final["status"] = "approved"
    final["approved_at"] = utc_now()
    state["stages"]["finalize"]["status"] = StageStatus.APPROVED
    state["stages"]["finalize"]["approved_version"] = f"sha256:{digest}"
    save_state(location, state)
    return final


def cmd_validate(args: argparse.Namespace) -> dict[str, Any]:
    location = TaskLocation(args.root, slug_value(args.slug))
    state = load_state(location)
    artifact_errors = []
    for stage, item in state["stages"].items():
        if item["status"] == StageStatus.APPROVED and item["artifact"]:
            path = Path(item["artifact"])
            if not path.is_file():
                artifact_errors.append(f"missing approved artifact: {path}")
            elif item["approved_version"] != artifact_version(path):
                artifact_errors.append(f"approved artifact changed: {path}")
    if artifact_errors:
        raise NeoError("; ".join(artifact_errors))
    return {"valid": True, "task": state["task"]["slug"]}


def add_task_argument(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("slug", type=slug_value)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    subparsers = parser.add_subparsers(dest="command", required=True)

    command = subparsers.add_parser("init")
    add_task_argument(command)
    command.add_argument("--title", required=True)
    command.set_defaults(handler=cmd_init)

    command = subparsers.add_parser("start")
    add_task_argument(command)
    command.add_argument("--title", required=True)
    command.add_argument("--signals", default="")
    command.set_defaults(handler=cmd_start)

    command = subparsers.add_parser("assess")
    add_task_argument(command)
    command.add_argument("--signals", default="")
    command.set_defaults(handler=cmd_assess)

    command = subparsers.add_parser("status")
    add_task_argument(command)
    command.set_defaults(handler=cmd_status)

    command = subparsers.add_parser("record-decision")
    add_task_argument(command)
    command.add_argument("--stage", choices=STAGES, required=True)
    command.add_argument("--id", required=True)
    command.add_argument("--summary", required=True)
    command.add_argument("--choice", required=True)
    command.add_argument("--rationale", required=True)
    command.add_argument("--depends-on", default="")
    command.add_argument("--supersedes")
    command.set_defaults(handler=cmd_record_decision)

    command = subparsers.add_parser("record-unknown")
    add_task_argument(command)
    command.add_argument("--stage", choices=STAGES, required=True)
    command.add_argument("--id", required=True)
    command.add_argument("--question", required=True)
    command.add_argument("--blocking", action=argparse.BooleanOptionalAction, default=True)
    command.set_defaults(handler=cmd_record_unknown)

    command = subparsers.add_parser("resolve-unknown")
    add_task_argument(command)
    command.add_argument("--id", required=True)
    command.add_argument("--resolution", required=True)
    command.set_defaults(handler=cmd_resolve_unknown)

    command = subparsers.add_parser("record-prototype")
    add_task_argument(command)
    command.add_argument("--kind", choices=[item.value for item in PrototypeKind], required=True)
    command.add_argument("--question")
    command.add_argument("--evidence")
    command.add_argument("--disposition")
    command.set_defaults(handler=cmd_record_prototype)

    command = subparsers.add_parser("gate")
    add_task_argument(command)
    command.add_argument("--stage", choices=STAGES, required=True)
    command.add_argument("--artifact", required=True)
    command.set_defaults(handler=cmd_gate)

    command = subparsers.add_parser("record-feedback")
    add_task_argument(command)
    command.add_argument("--kind", choices=[item.value for item in FeedbackKind], required=True)
    command.add_argument("--message", required=True)
    command.add_argument("--stage", choices=STAGES)
    command.add_argument("--decisions", default="")
    command.set_defaults(handler=cmd_record_feedback)

    command = subparsers.add_parser("revise")
    add_task_argument(command)
    command.add_argument("--stage", choices=STAGES, required=True)
    command.set_defaults(handler=cmd_revise)

    command = subparsers.add_parser("handoff")
    add_task_argument(command)
    command.add_argument("--expect", choices=STAGES)
    command.set_defaults(handler=cmd_handoff)

    command = subparsers.add_parser("finalize")
    add_task_argument(command)
    command.add_argument("--artifact", required=True)
    command.set_defaults(handler=cmd_finalize)

    command = subparsers.add_parser("approve")
    add_task_argument(command)
    command.set_defaults(handler=cmd_approve)

    command = subparsers.add_parser("validate")
    add_task_argument(command)
    command.set_defaults(handler=cmd_validate)

    command = subparsers.add_parser("validate-card")
    command.add_argument("artifact", type=Path)
    command.set_defaults(
        handler=lambda args: {
            "valid": not validate_decision_card(args.artifact),
            "errors": validate_decision_card(args.artifact),
        }
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    args.root = args.root.resolve()
    try:
        result = args.handler(args)
    except (NeoError, OSError) as error:
        print(f"neo: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
