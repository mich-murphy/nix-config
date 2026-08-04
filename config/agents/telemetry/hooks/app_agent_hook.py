#!/usr/bin/env python3
"""Create one metadata-only task trace from Codex or Claude lifecycle hooks."""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import secrets
import subprocess
import sys
import tempfile
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator

AGENT_ROOT = Path(__file__).resolve().parents[2]
if str(AGENT_ROOT) not in sys.path:
    sys.path.insert(0, str(AGENT_ROOT))

from telemetry import task_trace  # noqa: E402


def state_directory() -> Path:
    configured = os.environ.get("APP_AGENT_HOOK_STATE_DIR")
    directory = Path(configured) if configured else Path(tempfile.gettempdir()) / f"app-agent-otel-{os.getuid()}"
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    directory.chmod(0o700)
    return directory


def state_path(payload: dict[str, Any], harness: str) -> Path:
    identity = "\0".join((
        harness,
        str(payload.get("session_id", "not_observed")),
        str(payload.get("turn_id", "current")),
    ))
    return state_directory() / f"{task_trace.sha256_text(identity)}.json"


@contextmanager
def locked_state(path: Path) -> Iterator[dict[str, Any]]:
    lock_path = path.with_suffix(".lock")
    with lock_path.open("a+") as lock:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        try:
            state = json.loads(path.read_text()) if path.exists() else {}
            yield state
            temporary = path.with_suffix(f".{secrets.token_hex(4)}.tmp")
            temporary.write_text(json.dumps(state, separators=(",", ":")))
            temporary.chmod(0o600)
            temporary.replace(path)
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def git_revision(cwd: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", cwd, "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
            timeout=2,
        ).strip() or "not_observed"
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "not_observed"


def harness_version(harness: str) -> str:
    try:
        return subprocess.check_output(
            [harness, "--version"], text=True, stderr=subprocess.DEVNULL, timeout=2,
        ).strip() or "not_observed"
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "not_observed"


def tool_type(name: str) -> str:
    folded = name.casefold()
    if folded in {"bash", "exec_command", "write_stdin"}:
        return "shell"
    if folded in {"read", "edit", "write", "apply_patch"}:
        return "filesystem"
    if folded in {"grep", "glob", "find", "ls", "search"}:
        return "search"
    if folded.startswith("mcp"):
        return "mcp"
    return "other"


def validation_type(tool_name: str, tool_input: Any) -> str | None:
    if tool_name.casefold() not in {"bash", "exec_command"} or not isinstance(tool_input, dict):
        return None
    command = str(tool_input.get("command", tool_input.get("cmd", ""))).casefold()
    for kind, markers in (
        ("test", (" test", "pytest", "unittest", "cargo test", "go test", "nix flake check")),
        ("lint", ("lint", "ruff", "clippy", "shellcheck", "markdownlint")),
        ("build", (" build", "darwin-rebuild build", "docker compose config")),
    ):
        if any(marker in f" {command}" for marker in markers):
            return kind
    return None


def explicit_skills(prompt: str) -> list[str]:
    names = set(re.findall(r"(?<!\w)[$/]([a-z][a-z0-9-]{1,80})\b", prompt))
    names.update(re.findall(r'<skill\s+name="([a-z][a-z0-9-]{1,80})"', prompt))
    return sorted(names)


def available_skill(name: str, harness: str, cwd: str) -> tuple[str, str] | None:
    roots = [
        Path(cwd) / ".agents" / "skills",
        Path(cwd) / ".claude" / "skills",
        Path.home() / ".codex" / "skills",
        Path.home() / ".claude" / "skills",
    ]
    for root in roots:
        for candidate in (root / name, root / ".system" / name):
            if (candidate / "SKILL.md").is_file():
                return name, task_trace.sha256_path(candidate)
    return None


def tool_reference(tool_input: Any) -> str:
    if not isinstance(tool_input, dict):
        return ""
    return " ".join(str(tool_input.get(key, "")) for key in ("path", "file_path", "command", "cmd"))


def lifecycle_from_tool(state: dict[str, Any], tool_name: str, tool_input: Any) -> None:
    reference = tool_reference(tool_input)
    match = re.search(r"(?:^|/)skills/([a-z][a-z0-9-]{1,80})/(SKILL\.md|references/[^\s'\"]+|scripts/[^\s'\"]+)", reference)
    if not match:
        return
    name, resource = match.groups()
    if isinstance(tool_input, dict):
        raw_path = str(tool_input.get("path", tool_input.get("file_path", "")))
        path_match = re.search(rf"(.*/skills/{re.escape(name)})(?:/|$)", raw_path)
        if path_match:
            state.setdefault("skill_hashes", {})[name] = task_trace.sha256_path(Path(path_match.group(1)))
    lifecycle = state.setdefault("skills", {}).setdefault(name, [])
    for stage in ("offered", "selected"):
        if stage not in lifecycle:
            lifecycle.append(stage)
    if resource == "SKILL.md":
        for stage in ("activated",):
            if stage not in lifecycle:
                lifecycle.append(stage)
    elif resource.startswith("references/"):
        for stage in ("activated", "expanded"):
            if stage not in lifecycle:
                lifecycle.append(stage)
    elif resource.startswith("scripts/"):
        for stage in ("activated", "expanded", "executed"):
            if stage not in lifecycle:
                lifecycle.append(stage)


def verified_outcome() -> tuple[str, str]:
    outcome = os.environ.get("APP_AGENT_VERIFIED_OUTCOME", "completed")
    verifier = os.environ.get("APP_AGENT_VERIFIER_PROVENANCE", "not_observed")
    if outcome in {"accepted", "failed"} and verifier == "not_observed":
        return "completed", verifier
    if outcome not in {"accepted", "failed", "delayed", "completed", "cancelled"}:
        return "completed", verifier
    return outcome, verifier


def start_task(payload: dict[str, Any], harness: str, state: dict[str, Any]) -> None:
    timestamp = time.time_ns()
    session_id = str(payload.get("session_id", secrets.token_hex(16)))
    prompt = str(payload.get("prompt", ""))
    cwd = str(payload.get("cwd", "."))
    invoked = [
        match for name in explicit_skills(prompt)
        if (match := available_skill(name, harness, cwd)) is not None
    ]
    state.clear()
    state.update({
        "trace_id": secrets.token_hex(16),
        "task_id": str(payload.get("turn_id", f"task-{secrets.token_hex(8)}")),
        "session_id": session_id,
        "harness": harness,
        "harness_version": harness_version(harness),
        "started_ns": timestamp,
        "model": str(payload.get("model", "not_observed")),
        "effort": str((payload.get("effort") or {}).get("level", os.environ.get("APP_AGENT_MODEL_EFFORT", "not_observed"))),
        "cwd_hash": task_trace.sha256_text(str(Path(cwd).resolve())),
        "base_revision": git_revision(cwd),
        "prompt_hash": task_trace.sha256_text(prompt),
        "tools": [],
        "permissions": [],
        "skills": {name: ["offered", "selected", "activated", "expanded"] for name, _ in invoked},
        "skill_hashes": {name: package_hash for name, package_hash in invoked},
    })


def _tool_spans(state: dict[str, Any], ended_ns: int) -> list[dict[str, Any]]:
    """Build tool and verifier spans in their original execution order."""
    spans = []
    for item in state.get("tools", []):
        attributes: dict[str, str | int | float | bool] = {
            "app.agent.record.type": "tool",
            "gen_ai.operation.name": "execute_tool",
            "gen_ai.tool.name": item["name"],
            "gen_ai.tool.call.id": item["id"],
            "app.agent.tool.type": tool_type(item["name"]),
            "app.agent.tool.status": item.get("status", "not_observed"),
            "app.agent.tool.input_hash": item.get("input_hash", "not_observed"),
            "app.agent.tool.output_hash": item.get("output_hash", "not_observed"),
        }
        spans.append(task_trace.child_span(
            "tool.execute",
            started_ns=int(item["started_ns"]),
            ended_ns=int(item.get("ended_ns", ended_ns)),
            attributes=attributes,
            status="error" if item.get("status") == "error" else "ok",
        ))
        if item.get("validation_type"):
            spans.append(task_trace.child_span(
                "validation.run",
                started_ns=int(item["started_ns"]),
                ended_ns=int(item.get("ended_ns", ended_ns)),
                attributes={
                    "app.agent.record.type": "validation",
                    "app.agent.validation.type": item["validation_type"],
                    "app.agent.validation.status": (
                        "fail" if item.get("status") == "error" else "pass"
                    ),
                    "app.agent.validation.provenance": f"tool:{item['id']}",
                },
                status="error" if item.get("status") == "error" else "ok",
            ))
    return spans


def _permission_spans(state: dict[str, Any]) -> list[dict[str, Any]]:
    """Build the observed permission-wait spans for a task."""
    return [
        task_trace.child_span(
            "permission.wait",
            started_ns=int(permission["started_ns"]),
            ended_ns=int(permission["started_ns"]),
            attributes={
                "app.agent.record.type": "permission",
                "app.agent.permission.decision": "not_observed",
                "app.agent.permission.policy": str(
                    permission.get("mode", "not_observed")
                ),
                "gen_ai.tool.name": str(
                    permission.get("tool_name", "not_observed")
                ),
            },
        )
        for permission in state.get("permissions", [])
    ]


def _skill_spans(state: dict[str, Any]) -> list[dict[str, Any]]:
    """Complete and render each observed skill lifecycle."""
    spans = []
    for name, stages in state.get("skills", {}).items():
        if "activated" in stages and "executed" not in stages:
            stages.append("executed")
        if "evaluated" not in stages:
            stages.append("evaluated")
        package_hash = state.get("skill_hashes", {}).get(name, "not_observed")
        for stage in stages:
            spans.append(task_trace.child_span(
                "skill.activate" if stage == "activated" else "skill.lifecycle",
                started_ns=int(state["started_ns"]),
                ended_ns=int(state["started_ns"]),
                attributes={
                    "app.agent.record.type": "skill",
                    "app.agent.skill.name": name,
                    "app.agent.skill.package_hash": package_hash,
                    "app.agent.skill.activation": stage,
                    "app.agent.skill.selection": (
                        "user" if stage in {"offered", "selected"} else "model"
                    ),
                },
            ))
    return spans


def finish_task(state: dict[str, Any], payload: dict[str, Any]) -> dict[str, Any] | None:
    if not state.get("trace_id"):
        return None
    ended_ns = time.time_ns()
    outcome, verifier = verified_outcome()
    children = [task_trace.child_span(
        "gen_ai.invoke_agent",
        started_ns=int(state["started_ns"]),
        ended_ns=ended_ns,
        attributes={
            "gen_ai.operation.name": "invoke_agent",
            "gen_ai.request.model": state.get("model", "not_observed"),
            "app.agent.model.effort": state.get("effort", "not_observed"),
        },
    )]
    children.extend(_tool_spans(state, ended_ns))
    children.extend(_permission_spans(state))
    children.extend(_skill_spans(state))
    children.append(task_trace.child_span(
        "agent.final",
        started_ns=ended_ns,
        ended_ns=ended_ns,
        attributes={
            "app.agent.record.type": "outcome",
            "app.agent.final.status": outcome,
            "app.agent.outcome.status": outcome,
            "app.agent.outcome.verifier": verifier,
        },
    ))
    final_hash = task_trace.sha256_text(str(payload.get("last_assistant_message", "")))
    attributes: dict[str, str | int | float | bool] = {
        "app.agent.trace.kind": os.environ.get("APP_AGENT_TRACE_KIND", "operational"),
        "app.agent.harness.version": state.get("harness_version", "not_observed"),
        "app.agent.repository.hash": state["cwd_hash"],
        "app.agent.repository.base_revision": state["base_revision"],
        "app.agent.task.class": os.environ.get("APP_AGENT_TASK_CLASS", "interactive"),
        "app.agent.risk.class": os.environ.get("APP_AGENT_RISK_CLASS", "not_observed"),
        "app.agent.model.requested": state["model"],
        "app.agent.model.returned": str(payload.get("model", state["model"])),
        "app.agent.model.effort": state["effort"],
        "app.agent.prompt.version": state["prompt_hash"],
        "app.agent.tool.version": task_trace.sha256_path(Path(__file__)),
        "app.agent.final.status": outcome,
        "app.agent.outcome.status": outcome,
        "app.agent.outcome.verifier": verifier,
        "app.agent.cost.status": "not_observed",
        "app.agent.content.capture": "metadata",
        "gen_ai.input.messages": task_trace.metadata_messages("user", {"prompt_hash": state["prompt_hash"]}),
        "gen_ai.output.messages": task_trace.metadata_messages("assistant", {"output_hash": final_hash, "outcome": outcome}),
    }
    return task_trace.build_task_trace(
        harness=state["harness"],
        session_id=state["session_id"],
        task_id=state["task_id"],
        started_ns=int(state["started_ns"]),
        ended_ns=ended_ns,
        attributes=attributes,
        children=children,
        trace_id=state["trace_id"],
        status="error" if outcome in {"failed", "cancelled"} else "ok",
    )


def handle(payload: dict[str, Any], harness: str) -> dict[str, Any]:
    path = state_path(payload, harness)
    event = str(payload.get("hook_event_name", ""))
    exported: dict[str, Any] = {"status": "not_exported"}
    with locked_state(path) as state:
        if event == "UserPromptSubmit":
            start_task(payload, harness, state)
        elif event in {"PreToolUse", "PermissionRequest", "PostToolUse", "PostToolUseFailure"} and state:
            tool_name = str(payload.get("tool_name", "not_observed"))
            tool_input = payload.get("tool_input", {})
            lifecycle_from_tool(state, tool_name, tool_input)
            if event == "PreToolUse":
                state.setdefault("tools", []).append({
                    "id": str(payload.get("tool_use_id", secrets.token_hex(8))),
                    "name": tool_name,
                    "started_ns": time.time_ns(),
                    "input_hash": task_trace.sha256_text(json.dumps(tool_input, sort_keys=True, default=str)),
                    "validation_type": validation_type(tool_name, tool_input),
                })
            elif event == "PermissionRequest":
                state.setdefault("permissions", []).append({
                    "started_ns": time.time_ns(),
                    "tool_name": tool_name,
                    "mode": payload.get("permission_mode", "not_observed"),
                })
            else:
                tool_id = str(payload.get("tool_use_id", ""))
                pending = next((item for item in reversed(state.get("tools", [])) if item["id"] == tool_id), None)
                if pending is None:
                    pending = {
                        "id": tool_id or secrets.token_hex(8), "name": tool_name,
                        "started_ns": time.time_ns(), "input_hash": "not_observed",
                        "validation_type": validation_type(tool_name, tool_input),
                    }
                    state.setdefault("tools", []).append(pending)
                pending["ended_ns"] = time.time_ns()
                pending["status"] = "error" if event == "PostToolUseFailure" else "ok"
                output = payload.get("tool_response", payload.get("error", ""))
                pending["output_hash"] = task_trace.sha256_text(json.dumps(output, sort_keys=True, default=str))
        elif event == "Stop":
            trace = finish_task(state, payload)
            if trace:
                exported = task_trace.export_task_trace(trace)
                state.clear()
    return exported


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--harness", choices=("codex", "claude"), required=True)
    args = parser.parse_args()
    try:
        payload = json.load(sys.stdin)
        handle(payload, args.harness)
    except Exception as error:
        print(f"app-agent-otel hook failed without affecting task: {type(error).__name__}", file=sys.stderr)
    print("{}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
