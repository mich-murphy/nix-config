#!/usr/bin/env python3
"""Deterministic state, freshness, and review gates for the ship skill."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "1.0.0"
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
CURRENT_STATES = {
    "intake", "implementation", "verification", "review", "remediation",
    "release-ready", "stopped",
}
REVIEW_VERDICTS = {"pass", "changes-required", "replan"}
FINDING_SEVERITIES = {"blocking", "nonblocking"}
FINDING_ROUTES = {"tdd", "refactor", "replan", "clarify", "accept"}
FINDING_FIELDS = {
    "id", "severity", "category", "location", "finding", "evidence",
    "consequence", "route",
}


class ShipError(ValueError):
    """A user-actionable state or validation error."""


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def validate_slug(value: str) -> str:
    if not SLUG.fullmatch(value):
        raise ShipError("slug must use lowercase letters, digits, and single hyphens")
    return value


def state_path(root: Path, slug: str) -> Path:
    return root / ".ship" / "tasks" / validate_slug(slug) / "state.json"


def sha256_file(path: Path) -> str:
    if not path.is_file():
        raise ShipError(f"missing artifact: {path}")
    return f"sha256:{hashlib.sha256(path.read_bytes()).hexdigest()}"


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise ShipError(f"missing JSON artifact: {path}")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ShipError(f"invalid JSON artifact {path}: {error}") from error
    if not isinstance(value, dict):
        raise ShipError(f"JSON artifact must be an object: {path}")
    return value


def atomic_write(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(prefix=".state-", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            json.dump(value, stream, indent=2)
            stream.write("\n")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def run_git(root: Path, *arguments: str) -> bytes:
    completed = subprocess.run(
        ["git", "-C", str(root), *arguments],
        check=False,
        capture_output=True,
    )
    if completed.returncode:
        diagnostic = completed.stderr.decode("utf-8", errors="replace").strip()
        raise ShipError(f"git {' '.join(arguments)} failed: {diagnostic}")
    return completed.stdout


def repository_head(root: Path) -> str:
    return run_git(root, "rev-parse", "HEAD").decode().strip()


def candidate_hash(root: Path) -> str:
    digest = hashlib.sha256()
    digest.update(repository_head(root).encode())
    digest.update(
        run_git(
            root, "diff", "--binary", "--no-ext-diff", "HEAD", "--", ".",
            ":(exclude).ship",
        )
    )
    untracked = run_git(
        root, "ls-files", "--others", "--exclude-standard", "-z"
    ).split(b"\0")
    for raw_name in sorted(item for item in untracked if item):
        name = raw_name.decode("utf-8", errors="surrogateescape")
        if name == ".ship" or name.startswith(".ship/"):
            continue
        path = root / name
        metadata = path.lstat()
        digest.update(raw_name)
        digest.update(str(stat.S_IMODE(metadata.st_mode)).encode())
        if path.is_symlink():
            digest.update(os.readlink(path).encode())
        elif path.is_file():
            digest.update(path.read_bytes())
    return f"sha256:{digest.hexdigest()}"


def initial_paths(root: Path) -> list[str]:
    entries = [
        item for item in run_git(root, "status", "--porcelain=v1", "-z").split(b"\0")
        if item
    ]
    return [
        item.decode("utf-8", errors="replace")
        for item in entries
        if not item[3:].startswith(".ship/")
    ]


def validate_state(state: dict[str, Any]) -> None:
    required = {
        "schema_version", "slug", "title", "root", "plan", "plan_hash",
        "readiness", "readiness_hash", "base_revision",
        "initial_candidate_hash", "initial_paths", "current",
        "max_review_cycles", "review_attempts", "verified_candidate_hash",
        "evidence", "reviews", "invalidations", "stop_reason", "created_at",
        "updated_at",
    }
    if set(state) != required:
        raise ShipError(
            "state fields mismatch; "
            f"missing={sorted(required - set(state))}, "
            f"unknown={sorted(set(state) - required)}"
        )
    if state["schema_version"] != SCHEMA_VERSION:
        raise ShipError(f"unsupported state schema: {state['schema_version']}")
    if state["current"] not in CURRENT_STATES:
        raise ShipError(f"invalid current state: {state['current']}")
    if not isinstance(state["review_attempts"], int) or state["review_attempts"] < 0:
        raise ShipError("review_attempts must be a non-negative integer")
    if not isinstance(state["max_review_cycles"], int) or state["max_review_cycles"] < 1:
        raise ShipError("max_review_cycles must be a positive integer")


def load_state(root: Path, slug: str) -> dict[str, Any]:
    path = state_path(root, slug)
    if not path.is_file():
        raise ShipError(f"missing ship state: {path}")
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ShipError(f"invalid ship state: {error}") from error
    validate_state(state)
    return state


def save_state(root: Path, state: dict[str, Any]) -> None:
    state["updated_at"] = utc_now()
    validate_state(state)
    atomic_write(state_path(root, state["slug"]), state)


def validate_readiness(path: Path) -> None:
    value = load_json(path)
    required = {
        "objective", "acceptance_criteria", "non_goals", "constraints",
        "verification", "open_questions",
    }
    if set(value) != required:
        raise ShipError(
            "readiness fields mismatch; "
            f"missing={sorted(required - set(value))}, "
            f"unknown={sorted(set(value) - required)}"
        )
    if not isinstance(value["objective"], str) or not value["objective"].strip():
        raise ShipError("readiness objective must be non-empty")
    for name in ("acceptance_criteria", "non_goals", "constraints", "verification"):
        if not isinstance(value[name], list):
            raise ShipError(f"readiness {name} must be a list")
    if not value["acceptance_criteria"]:
        raise ShipError("readiness needs at least one acceptance criterion")
    if not value["verification"]:
        raise ShipError("readiness needs at least one verification item")
    if not isinstance(value["open_questions"], list):
        raise ShipError("readiness open_questions must be a list")
    for question in value["open_questions"]:
        if not isinstance(question, dict) or set(question) != {"question", "blocking"}:
            raise ShipError("each open question needs question and blocking fields")
        if not isinstance(question["question"], str) or not question["question"].strip():
            raise ShipError("open question text must be non-empty")
        if not isinstance(question["blocking"], bool):
            raise ShipError("open question blocking must be boolean")
        if question["blocking"]:
            raise ShipError(f"blocking open question: {question['question']}")


def validate_review(path: Path) -> dict[str, Any]:
    value = load_json(path)
    if set(value) != {"verdict", "summary", "findings"}:
        raise ShipError("review needs exactly verdict, summary, and findings")
    if value["verdict"] not in REVIEW_VERDICTS:
        raise ShipError(f"invalid review verdict: {value['verdict']}")
    if not isinstance(value["summary"], str):
        raise ShipError("review summary must be a string")
    if not isinstance(value["findings"], list):
        raise ShipError("review findings must be a list")
    identifiers: set[str] = set()
    blocking: list[dict[str, Any]] = []
    for finding in value["findings"]:
        if not isinstance(finding, dict) or set(finding) != FINDING_FIELDS:
            raise ShipError(
                "each finding needs exactly " + ", ".join(sorted(FINDING_FIELDS))
            )
        if finding["id"] in identifiers:
            raise ShipError(f"duplicate finding id: {finding['id']}")
        identifiers.add(finding["id"])
        if finding["severity"] not in FINDING_SEVERITIES:
            raise ShipError(f"invalid finding severity: {finding['severity']}")
        if finding["route"] not in FINDING_ROUTES:
            raise ShipError(f"invalid finding route: {finding['route']}")
        for field in FINDING_FIELDS - {"severity", "route"}:
            if not isinstance(finding[field], str) or not finding[field].strip():
                raise ShipError(f"finding {finding['id']} has empty {field}")
        if finding["severity"] == "blocking":
            blocking.append(finding)
    verdict = value["verdict"]
    if verdict == "pass" and blocking:
        raise ShipError("pass review cannot contain blocking findings")
    if verdict == "changes-required":
        if not blocking:
            raise ShipError("changes-required review needs a blocking finding")
        if any(item["route"] not in {"tdd", "refactor"} for item in blocking):
            raise ShipError("changes-required blockers must route to tdd or refactor")
    if verdict == "replan" and (
        not blocking
        or not any(item["route"] in {"replan", "clarify"} for item in blocking)
    ):
        raise ShipError("replan review needs a replan or clarify blocker")
    return value


def evidence_record(stage: str, path: Path, root: Path) -> dict[str, Any]:
    if not path.is_file() or not path.read_bytes().strip():
        raise ShipError(f"evidence must be a non-empty file: {path}")
    return {
        "stage": stage,
        "path": str(path.resolve()),
        "hash": sha256_file(path),
        "candidate_hash": candidate_hash(root),
        "recorded_at": utc_now(),
    }


def stale_reasons(root: Path, state: dict[str, Any]) -> list[str]:
    reasons = []
    try:
        if sha256_file(Path(state["plan"])) != state["plan_hash"]:
            reasons.append("plan_changed")
    except ShipError:
        reasons.append("plan_missing")
    if state["readiness"]:
        try:
            if sha256_file(Path(state["readiness"])) != state["readiness_hash"]:
                reasons.append("readiness_changed")
        except ShipError:
            reasons.append("readiness_missing")
    if state["current"] in {"review", "release-ready"}:
        if candidate_hash(root) != state["verified_candidate_hash"]:
            reasons.append("candidate_changed_after_verification")
    return reasons


def cmd_start(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    repository_head(root)
    path = state_path(root, args.slug)
    if path.exists():
        raise ShipError(f"ship task already exists: {path}")
    plan = args.plan.resolve()
    now = utc_now()
    state = {
        "schema_version": SCHEMA_VERSION,
        "slug": validate_slug(args.slug),
        "title": args.title,
        "root": str(root),
        "plan": str(plan),
        "plan_hash": sha256_file(plan),
        "readiness": None,
        "readiness_hash": None,
        "base_revision": repository_head(root),
        "initial_candidate_hash": candidate_hash(root),
        "initial_paths": initial_paths(root),
        "current": "intake",
        "max_review_cycles": args.max_review_cycles,
        "review_attempts": 0,
        "verified_candidate_hash": None,
        "evidence": [],
        "reviews": [],
        "invalidations": [],
        "stop_reason": None,
        "created_at": now,
        "updated_at": now,
    }
    save_state(root, state)
    return state


def cmd_ready(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    if state["current"] != "intake":
        raise ShipError(f"ready requires intake state, got {state['current']}")
    if sha256_file(Path(state["plan"])) != state["plan_hash"]:
        raise ShipError("plan changed after ship start")
    readiness = args.readiness.resolve()
    validate_readiness(readiness)
    state["readiness"] = str(readiness)
    state["readiness_hash"] = sha256_file(readiness)
    state["current"] = "implementation"
    save_state(root, state)
    return {"current": state["current"], "readiness": state["readiness"]}


def cmd_advance(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    if state["current"] != args.stage:
        raise ShipError(
            f"advance expected {args.stage}, current state is {state['current']}"
        )
    state["evidence"].append(evidence_record(args.stage, args.evidence.resolve(), root))
    if args.stage in {"implementation", "remediation"}:
        state["current"] = "verification"
        state["verified_candidate_hash"] = None
    else:
        state["current"] = "review"
        state["verified_candidate_hash"] = candidate_hash(root)
    save_state(root, state)
    return {
        "current": state["current"],
        "candidate_hash": state["verified_candidate_hash"],
    }


def cmd_record_review(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    if state["current"] != "review":
        raise ShipError(f"record-review requires review state, got {state['current']}")
    current_hash = candidate_hash(root)
    if current_hash != state["verified_candidate_hash"]:
        raise ShipError("candidate changed after verification; verify it again")
    review_path = args.review.resolve()
    review = validate_review(review_path)
    state["review_attempts"] += 1
    state["reviews"].append(
        {
            "path": str(review_path),
            "hash": sha256_file(review_path),
            "candidate_hash": current_hash,
            "verdict": review["verdict"],
            "findings": review["findings"],
            "recorded_at": utc_now(),
        }
    )
    if review["verdict"] == "pass":
        state["current"] = "release-ready"
    elif review["verdict"] == "replan":
        state["current"] = "stopped"
        state["stop_reason"] = "review requires replanning or clarification"
    elif sum(
        item["verdict"] == "changes-required" for item in state["reviews"]
    ) >= state["max_review_cycles"]:
        state["current"] = "stopped"
        state["stop_reason"] = "review remediation cycle limit reached"
    else:
        state["current"] = "remediation"
        state["verified_candidate_hash"] = None
    save_state(root, state)
    return {
        "current": state["current"],
        "verdict": review["verdict"],
        "review_attempts": state["review_attempts"],
        "stop_reason": state["stop_reason"],
    }


def cmd_invalidate(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    if state["current"] == "stopped":
        raise ShipError("cannot invalidate a stopped task")
    state["invalidations"].append(
        {"from": state["current"], "reason": args.reason, "recorded_at": utc_now()}
    )
    state["current"] = "implementation"
    state["verified_candidate_hash"] = None
    save_state(root, state)
    return {"current": state["current"], "reason": args.reason}


def cmd_stop(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    state["current"] = "stopped"
    state["stop_reason"] = args.reason
    save_state(root, state)
    return {"current": state["current"], "reason": args.reason}


def cmd_status(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    return {
        "slug": state["slug"],
        "current": state["current"],
        "base_revision": state["base_revision"],
        "candidate_hash": candidate_hash(root),
        "verified_candidate_hash": state["verified_candidate_hash"],
        "review_attempts": state["review_attempts"],
        "max_review_cycles": state["max_review_cycles"],
        "stale_reasons": stale_reasons(root, state),
        "stop_reason": state["stop_reason"],
    }


def cmd_validate(args: argparse.Namespace) -> dict[str, Any]:
    root = args.root.resolve()
    state = load_state(root, args.slug)
    if state["current"] != "release-ready":
        raise ShipError(f"task is not release-ready: {state['current']}")
    reasons = stale_reasons(root, state)
    if reasons:
        raise ShipError("release-ready evidence is stale: " + ", ".join(reasons))
    if not state["reviews"] or state["reviews"][-1]["verdict"] != "pass":
        raise ShipError("release-ready task needs a final pass review")
    return {
        "valid": True,
        "status": "release-ready",
        "slug": state["slug"],
        "base_revision": state["base_revision"],
        "candidate_hash": candidate_hash(root),
        "review_attempts": state["review_attempts"],
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    commands = parser.add_subparsers(dest="command", required=True)

    command = commands.add_parser("start")
    command.add_argument("slug")
    command.add_argument("--title", required=True)
    command.add_argument("--plan", type=Path, required=True)
    command.add_argument("--max-review-cycles", type=int, default=2)
    command.set_defaults(handler=cmd_start)

    command = commands.add_parser("ready")
    command.add_argument("slug")
    command.add_argument("--readiness", type=Path, required=True)
    command.set_defaults(handler=cmd_ready)

    command = commands.add_parser("advance")
    command.add_argument("slug")
    command.add_argument(
        "--stage",
        choices=("implementation", "verification", "remediation"),
        required=True,
    )
    command.add_argument("--evidence", type=Path, required=True)
    command.set_defaults(handler=cmd_advance)

    command = commands.add_parser("record-review")
    command.add_argument("slug")
    command.add_argument("--review", type=Path, required=True)
    command.set_defaults(handler=cmd_record_review)

    command = commands.add_parser("invalidate")
    command.add_argument("slug")
    command.add_argument("--reason", required=True)
    command.set_defaults(handler=cmd_invalidate)

    command = commands.add_parser("stop")
    command.add_argument("slug")
    command.add_argument("--reason", required=True)
    command.set_defaults(handler=cmd_stop)

    command = commands.add_parser("status")
    command.add_argument("slug")
    command.set_defaults(handler=cmd_status)

    command = commands.add_parser("validate")
    command.add_argument("slug")
    command.set_defaults(handler=cmd_validate)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        result = args.handler(args)
    except ShipError as error:
        print(json.dumps({"error": str(error)}), file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
