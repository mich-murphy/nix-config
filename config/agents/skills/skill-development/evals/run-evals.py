#!/usr/bin/env python3
"""Create real skill artifacts with candidate and built-in creators."""

from __future__ import annotations

import argparse
import concurrent.futures
import functools
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


EVAL_DIR = Path(__file__).resolve().parent
SKILL_ROOT = EVAL_DIR.parent
REPO_ROOT = SKILL_ROOT.parents[3]
AGENT_ROOT = SKILL_ROOT.parents[1]
if str(AGENT_ROOT) not in sys.path:
    sys.path.insert(0, str(AGENT_ROOT))

from telemetry import task_trace  # noqa: E402
CASES = json.loads((EVAL_DIR / "cases.json").read_text(encoding="utf-8"))
ROUTES = json.loads((EVAL_DIR / "routes.json").read_text(encoding="utf-8"))
VARIANTS = ("incumbent", "candidate")
HARNESSES = ("codex", "claude")


@functools.lru_cache(maxsize=None)
def harness_version(harness: str) -> str:
    try:
        return subprocess.check_output(
            [harness, "--version"], text=True, stderr=subprocess.DEVNULL, timeout=10,
        ).strip() or "not_observed"
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "not_observed"


def git_value(*arguments: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(REPO_ROOT), *arguments], text=True,
            stderr=subprocess.DEVNULL, timeout=10,
        ).strip() or "not_observed"
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "not_observed"


def returned_model(events: list[Any]) -> str:
    for event in reversed(events):
        if not isinstance(event, dict):
            continue
        for key in ("response_model", "responseModel", "model"):
            if isinstance(event.get(key), str) and event[key]:
                return event[key]
        message = event.get("message")
        if isinstance(message, dict) and isinstance(message.get("model"), str):
            return message["model"]
    return "not_observed"


def observed_cost(usage: dict[str, Any]) -> float | None:
    for value in (usage.get("cost_usd"), usage.get("total_cost_usd")):
        if isinstance(value, (int, float)):
            return float(value)
    cost = usage.get("cost")
    if isinstance(cost, dict) and isinstance(cost.get("total"), (int, float)):
        return float(cost["total"])
    return None


@dataclass
class ProcessResult:
    returncode: int
    duration_seconds: float
    stdout: str
    stderr: str
    timed_out: bool = False


def sha256_path(root: Path) -> str:
    digest = hashlib.sha256()
    files = (
        item for item in root.rglob("*")
        if item.is_file()
        and not {"__pycache__", ".pytest_cache"}.intersection(item.parts)
    )
    for path in sorted(files):
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def builtin_path(harness: str) -> Path:
    if harness == "codex":
        configured = os.environ.get("SKILL_EVAL_CODEX_CREATOR")
        return Path(configured) if configured else Path.home() / ".codex/skills/.system/skill-creator"
    configured = os.environ.get("SKILL_EVAL_CLAUDE_CREATOR")
    if configured:
        return Path(configured)
    return Path.home() / ".claude/plugins/marketplaces/anthropics-claude-plugins-official/plugins/skill-creator/skills/skill-creator"


def copy_candidate(destination: Path) -> None:
    shutil.copytree(
        SKILL_ROOT,
        destination,
        ignore=shutil.ignore_patterns("evals", "tests", "__pycache__", "*.pyc"),
    )


def install_variant(workspace: Path, harness: str, variant: str) -> tuple[str, str]:
    if variant == "candidate":
        name = "skill-development"
        source = SKILL_ROOT
        target = workspace / (".agents/skills" if harness == "codex" else ".claude/skills") / name
        target.parent.mkdir(parents=True, exist_ok=True)
        copy_candidate(target)
        source_hash = sha256_path(target)
    else:
        name = "skill-creator"
        source = builtin_path(harness)
        if not (source / "SKILL.md").is_file():
            raise FileNotFoundError(f"{harness} built-in creator not found at {source}")
        if harness == "claude":
            target = workspace / ".claude/skills" / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(source, target)
            source_hash = sha256_path(target)
        else:
            source_hash = sha256_path(source)
    return name, source_hash


def initialize_workspace(workspace: Path, case: dict[str, Any]) -> None:
    workspace.mkdir(parents=True)
    fixture = EVAL_DIR / "fixtures" / case["fixture"]
    shutil.copyfile(fixture, workspace / "evidence.md")
    (workspace / "AGENTS.md").write_text(
        "Work only inside this evaluation workspace. Treat evidence.md as read-only. "
        "Create requested artifacts under output/. Do not access network services.\n",
        encoding="utf-8",
    )
    (workspace / "output").mkdir()
    commands = (
        ("git", "init", "--quiet"),
        ("git", "config", "user.name", "Skill Evaluation"),
        ("git", "config", "user.email", "skill-eval@example.invalid"),
        ("git", "add", "--all"),
        ("git", "commit", "--quiet", "-m", "evaluation fixture"),
    )
    for command in commands:
        subprocess.run(command, cwd=workspace, check=True, capture_output=True, text=True)


def prompt_for(harness: str, skill_name: str, case: dict[str, Any]) -> str:
    invocation = f"${skill_name}" if harness == "codex" else f"/{skill_name}"
    return f"""Use {invocation} for this task.

Read evidence.md, then perform the task below. Work autonomously and do not ask
questions. Keep all created artifacts under output/. Do not use the network or
modify evidence.md. Finish only after running relevant local structural and
unit checks.

{case['prompt']}
"""


def command_for(harness: str, workspace: Path, prompt: str) -> list[str]:
    route = ROUTES["harnesses"][harness]
    if harness == "codex":
        return [
            "codex", "exec", "--json", "--ephemeral", "--skip-git-repo-check",
            "--ignore-user-config", "-C", str(workspace), "--sandbox",
            "workspace-write", "-m", route["model"], "-c",
            f'model_reasoning_effort="{route["effort"]}"', prompt,
        ]
    return [
        "claude", "--print", prompt, "--output-format", "json", "--verbose",
        "--no-session-persistence", "--setting-sources", "project",
        "--permission-mode", "acceptEdits", "--model", route["model"],
        "--effort", route["effort"], "--allowedTools", "Read,Glob,Grep,Edit,Write,Bash",
    ]


def run_process(command: list[str], workspace: Path, timeout: int) -> ProcessResult:
    environment = os.environ.copy()
    for key in tuple(environment):
        if key.startswith("FZF_"):
            environment.pop(key)
    environment["APP_AGENT_EVAL_RUN"] = "1"
    environment["APP_AGENT_TRACE_KIND"] = "evaluation"
    environment["OTEL_TRACES_EXPORTER"] = "none"
    environment["CLAUDE_CODE_ENHANCED_TELEMETRY_BETA"] = "0"
    environment["OTEL_LOG_USER_PROMPTS"] = "0"
    environment["OTEL_LOG_TOOL_DETAILS"] = "0"
    environment["OTEL_LOG_TOOL_CONTENT"] = "0"
    environment["OTEL_LOG_RAW_API_BODIES"] = "0"
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command, cwd=workspace, env=environment, text=True,
            capture_output=True, timeout=timeout, check=False,
        )
        return ProcessResult(
            completed.returncode, time.monotonic() - started,
            completed.stdout, completed.stderr,
        )
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout.decode() if isinstance(error.stdout, bytes) else error.stdout
        stderr = error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr
        return ProcessResult(124, time.monotonic() - started, stdout or "", stderr or "", True)


def parse_harness_output(harness: str, result: ProcessResult) -> tuple[str, dict[str, Any], list[Any]]:
    if harness == "claude":
        try:
            parsed = json.loads(result.stdout)
        except json.JSONDecodeError:
            return result.stdout.strip(), {}, []
        events = parsed if isinstance(parsed, list) else [parsed]
        objects = [event for event in events if isinstance(event, dict)]
        results = [str(event.get("result", "")) for event in objects if event.get("result")]
        usage_events = [event.get("usage", {}) for event in objects if event.get("usage")]
        if not results:
            for event in objects:
                message = event.get("message", {})
                content = message.get("content", []) if isinstance(message, dict) else []
                for block in content if isinstance(content, list) else []:
                    if isinstance(block, dict) and block.get("type") == "text":
                        results.append(str(block.get("text", "")))
        return (results[-1] if results else "", usage_events[-1] if usage_events else {}, events)
    events: list[Any] = []
    for line in result.stdout.splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    messages = [
        event.get("item", {}).get("text", "") for event in events
        if event.get("type") == "item.completed"
        and event.get("item", {}).get("type") == "agent_message"
    ]
    usage_events = [event.get("usage", {}) for event in events if event.get("type") == "turn.completed"]
    return (messages[-1] if messages else "", usage_events[-1] if usage_events else {}, events)


def read_json(path: Path) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def assertion(name: str, passed: bool, blocking: bool = False, detail: str = "") -> dict[str, Any]:
    return {"name": name, "passed": bool(passed), "blocking": blocking, "detail": detail}


def find_skill(output: Path) -> Path | None:
    matches = sorted(path.parent for path in output.rglob("SKILL.md"))
    return matches[0] if matches else None


def grade_skill_package(output: Path, security: bool) -> list[dict[str, Any]]:
    root = find_skill(output)
    if root is None:
        return [assertion("valid skill package", False, True, "no SKILL.md under output")]
    skill_text = (root / "SKILL.md").read_text(encoding="utf-8")
    frontmatter_ok = bool(re.match(r"---\n.*?name: .+\n.*?description: .+\n---", skill_text, re.DOTALL))
    proposal = read_json(root / "proposal.json")
    cases = read_json(root / "evals/cases.json")
    routing = read_json(root / "evals/routing-cases.json")
    routes = read_json(root / "evals/routes.json")
    telemetry = read_json(root / "evals/telemetry-policy.json")
    release = read_json(root / "evals/release-decision.json")
    status = read_json(root / "evals/results/status.json")
    case_list = cases.get("cases", []) if isinstance(cases, dict) else cases
    routing_list = routing if isinstance(routing, list) else []
    proposal_refs = proposal.get("evidence", {}).get("references", []) if isinstance(proposal, dict) else []
    fixed = set(routes.get("fixed", [])) if isinstance(routes, dict) else set()
    lifecycle = set(telemetry.get("skill_lifecycle", [])) if isinstance(telemetry, dict) else set()
    required_lifecycle = {"offered", "selected", "activated", "expanded", "executed", "evaluated"}
    forbidden = proposal.get("forbidden_effects", []) if isinstance(proposal, dict) else []
    forbidden_text = " ".join(str(value) for value in forbidden).casefold()
    checks = [
        assertion("valid concise SKILL.md", frontmatter_ok and "TODO" not in skill_text and len(skill_text.splitlines()) <= 500, True),
        assertion("evidence-backed proposal", isinstance(proposal, dict) and bool(proposal.get("job")) and bool(proposal_refs)),
        assertion("source lineage preserved", any("evidence" in str(value).casefold() or "review" in str(value).casefold() for value in proposal_refs)),
        assertion("three outcome cases with held-out split", isinstance(case_list, list) and len(case_list) >= 3 and {"development", "held-out"} <= {item.get("split") for item in case_list if isinstance(item, dict)}),
        assertion("positive and negative routing cases", sum(item.get("expected_activation") is True for item in routing_list if isinstance(item, dict)) >= 3 and sum(item.get("expected_activation") is False for item in routing_list if isinstance(item, dict)) >= 3),
        assertion("side-effect routing boundary", any(item.get("risk") == "side-effect" and item.get("expected_activation") is False for item in routing_list if isinstance(item, dict)), security),
        assertion("semantic cross-harness model routes", isinstance(routes, dict) and routes.get("semantic_lane") in {"efficient", "balanced", "frontier"} and set(routes.get("harnesses", {})) >= {"codex", "claude", "pi"} and {"task", "model", "effort", "tools", "permissions", "workspace", "verifier"} <= fixed),
        assertion("privacy-first telemetry", isinstance(telemetry, dict) and telemetry.get("metadata_only_default") is True and telemetry.get("content_capture_enabled") is False, True),
        assertion("complete skill lifecycle tracing", required_lifecycle <= lifecycle),
        assertion("deferred quality-gated release", isinstance(release, dict) and release.get("stage") == "alpha" and release.get("owner_decision") == "defer" and set(release.get("quality_gate", {})) >= {"functional", "regression", "integrity", "safety"}),
        assertion("all control variants represented", isinstance(status, dict) and set(status.get("variants", {})) >= {"no-skill", "incumbent", "candidate"}),
        assertion("executable runner and comparator", (root / "evals/run-evals.py").is_file() and (root / "evals/compare-evals.py").is_file()),
    ]
    if security:
        checks.extend([
            assertion("read-only incident boundary", all(term in (skill_text + forbidden_text).casefold() for term in ("transmit", "delete", "mutat")), True),
            assertion("human owns root-cause acceptance", "human" in skill_text.casefold() or "incident commander" in skill_text.casefold()),
            assertion("consequential route escalation", isinstance(routes, dict) and (routes.get("semantic_lane") == "frontier" or "high" in json.dumps(routes).casefold())),
        ])
    return checks


def grade_deterministic_control(output: Path) -> list[dict[str, Any]]:
    decision = read_json(output / "decision.json")
    decision_text = json.dumps(decision).casefold() if isinstance(decision, dict) else ""
    skill_exists = any(output.rglob("SKILL.md"))
    scripts = [path for path in output.rglob("*") if path.suffix in {".py", ".sh", ".js", ".ts"} and "test" not in path.name.casefold()]
    tests = [path for path in output.rglob("*") if path.is_file() and "test" in path.name.casefold()]
    source_text = "\n".join(path.read_text(encoding="utf-8", errors="ignore") for path in scripts)
    return [
        assertion("explicit container decision", isinstance(decision, dict) and any(term in decision_text for term in ("script", "tool")), True),
        assertion("rejects unnecessary skill", not skill_exists and any(term in decision_text for term in ("reject", "not a skill", "script", "tool")), True),
        assertion("executable deterministic control", bool(scripts)),
        assertion("executable tests", bool(tests)),
        assertion("duplicate-key rejection", "duplicate" in (decision_text + source_text.casefold()) and any(term in source_text for term in ("object_pairs_hook", "pairs", "duplicate"))),
        assertion("idempotence completion check", "idempot" in decision_text or "byte-identical" in decision_text),
        assertion("no overwrite on failure", any(term in (decision_text + source_text.casefold()) for term in ("unchanged", "atomic", "temporary", "replace"))),
        assertion("evidence lineage", "evidence" in decision_text or "review" in decision_text),
    ]


def grade(case: dict[str, Any], output: Path) -> list[dict[str, Any]]:
    if case["grader"] == "deterministic_control":
        return grade_deterministic_control(output)
    return grade_skill_package(output, case["grader"] == "skill_package_security")


def classify_failure(result: ProcessResult, final: str) -> tuple[str, str | None]:
    if result.returncode == 0 and final:
        return "succeeded", None
    diagnostic = (result.stderr + "\n" + final).casefold()
    if any(term in diagnostic for term in ("authentication", "not logged in", "invalid_grant", "api key")):
        return "harness_failure", "authentication"
    if result.timed_out:
        return "environment_failure", "timeout"
    if any(term in diagnostic for term in ("connection", "network", "dns")):
        return "environment_failure", "network"
    return "task_failure", "model_or_task"


def export_evaluation_trace(
    *,
    case: dict[str, Any],
    harness: str,
    variant: str,
    repetition: int,
    skill_name: str,
    skill_hash: str,
    prompt: str,
    started_ns: int,
    ended_ns: int,
    evaluation_ended_ns: int,
    state: str,
    accepted: bool,
    score: float,
    passed: int,
    total: int,
    usage: dict[str, Any],
    events: list[Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    route = ROUTES["harnesses"][harness]
    evaluator_version = task_trace.sha256_path(Path(__file__))
    outcome = "accepted" if accepted else (
        "invalid_harness" if state == "harness_failure" else
        "invalid_environment" if state == "environment_failure" else
        "evaluator_error" if state == "evaluator_failure" else "failed"
    )
    verifier = f"skill-development-package-grader@{evaluator_version}"
    attributes = task_trace.evaluation_attributes(
        case_id=case["id"], variant=variant, repetition=repetition,
        mode="end-to-end", skill_name=skill_name, skill_hash=skill_hash,
        skill_source=variant,
        repository_hash=task_trace.sha256_text(str(REPO_ROOT.resolve())),
        base_revision=git_value("rev-parse", "HEAD"),
        model_requested=route["model"], model_returned=returned_model(events),
        effort=route["effort"], prompt_version=task_trace.sha256_text(prompt),
        tool_version=harness_version(harness), evaluator_version=evaluator_version,
        risk=case.get("risk", "normal"), outcome=outcome, verifier=verifier,
    )
    attributes.update({
        "app.agent.task.class": "skill_evaluation",
        "app.agent.harness.version": harness_version(harness),
        "gen_ai.input.messages": task_trace.metadata_messages("user", {
            "case_id": case["id"], "variant": variant, "repetition": repetition,
        }),
        "gen_ai.output.messages": task_trace.metadata_messages("assistant", {
            "outcome": outcome, "score": score, "assertions_passed": passed,
            "assertions_total": total,
        }),
    })
    cost = observed_cost(usage)
    if cost is not None:
        attributes["app.agent.cost.status"] = "observed"
        attributes["app.agent.cost.usd"] = cost
    invocation: dict[str, str | int | float | bool] = {
        "gen_ai.operation.name": "invoke_agent",
        "gen_ai.request.model": route["model"],
        "app.agent.model.effort": route["effort"],
    }
    for source, target in (
        ("input_tokens", "gen_ai.usage.input_tokens"),
        ("output_tokens", "gen_ai.usage.output_tokens"),
        ("cache_read_input_tokens", "app.agent.tokens.cached"),
    ):
        if isinstance(usage.get(source), int):
            invocation[target] = usage[source]
    children = [task_trace.child_span(
        "gen_ai.invoke_agent", started_ns=started_ns, ended_ns=ended_ns,
        attributes=invocation,
        status="error" if state not in {"succeeded", "task_failure"} else "ok",
    )]
    for index, stage in enumerate(("offered", "selected", "activated", "expanded", "executed", "evaluated")):
        lifecycle_ns = min(ended_ns, started_ns + index)
        children.append(task_trace.child_span(
            "skill.activate" if stage == "activated" else "skill.lifecycle",
            started_ns=lifecycle_ns, ended_ns=lifecycle_ns,
            attributes={
                "app.agent.record.type": "skill",
                "app.agent.skill.name": skill_name,
                "app.agent.skill.package_hash": skill_hash,
                "app.agent.skill.activation": stage,
                "app.agent.skill.selection": "explicit",
            },
        ))
    children.extend([
        task_trace.child_span(
            "validation.run", started_ns=ended_ns, ended_ns=evaluation_ended_ns,
            attributes={
                "app.agent.record.type": "validation",
                "app.agent.validation.type": "skill-package-grader",
                "app.agent.validation.status": "pass" if accepted else "fail",
                "app.agent.validation.passed": passed,
                "app.agent.validation.failed": max(0, total - passed),
            }, status="ok" if accepted else "error",
        ),
        task_trace.child_span(
            "evaluator.run", started_ns=ended_ns, ended_ns=evaluation_ended_ns,
            attributes={
                "app.agent.evaluator.name": "skill-development-package-grader",
                "app.agent.evaluator.version": evaluator_version,
                "app.agent.outcome.status": outcome,
            }, status="error" if state == "evaluator_failure" else "ok",
        ),
        task_trace.child_span(
            "agent.final", started_ns=evaluation_ended_ns, ended_ns=evaluation_ended_ns,
            attributes={
                "app.agent.record.type": "outcome", "app.agent.final.status": outcome,
                "app.agent.outcome.status": outcome, "app.agent.outcome.verifier": verifier,
            },
        ),
    ])
    session_seed = f"{case['id']}-{harness}-{variant}-{repetition}-{started_ns}"
    session_id = f"eval-{task_trace.sha256_text(session_seed)[:24]}"
    trace = task_trace.build_task_trace(
        harness=harness, session_id=session_id,
        task_id=f"{case['id']}-{harness}-{variant}-{repetition}-{started_ns}",
        started_ns=started_ns, ended_ns=evaluation_ended_ns,
        attributes=attributes, children=children,
        status="error" if state not in {"succeeded", "task_failure"} else "ok",
    )
    delivery = task_trace.export_task_trace(
        trace, os.environ.get("APP_AGENT_EVAL_OTLP_ENDPOINT", "http://docker-host:4318/v1/traces"),
    )
    return trace, delivery


def run_once(case: dict[str, Any], harness: str, variant: str, repetition: int, timeout: int, artifacts: Path) -> dict[str, Any]:
    started_ns = time.time_ns()
    with tempfile.TemporaryDirectory(prefix="skill-development-eval-") as directory:
        workspace = Path(directory) / "workspace"
        initialize_workspace(workspace, case)
        try:
            skill_name, source_hash = install_variant(workspace, harness, variant)
        except OSError as error:
            ended_ns = time.time_ns()
            prompt = prompt_for(harness, "skill-creator" if variant == "incumbent" else "skill-development", case)
            trace, delivery = export_evaluation_trace(
                case=case, harness=harness, variant=variant, repetition=repetition,
                skill_name="skill-creator" if variant == "incumbent" else "skill-development",
                skill_hash="not_observed", prompt=prompt, started_ns=started_ns,
                ended_ns=ended_ns, evaluation_ended_ns=ended_ns,
                state="environment_failure", accepted=False, score=0.0,
                passed=0, total=0, usage={}, events=[],
            )
            record = {
                "id": case["id"], "split": case["split"], "harness": harness,
                "variant": variant, "repetition": repetition, "valid": False,
                "accepted": False, "score": 0.0, "blocking_failures": [str(error)],
                "state": "environment_failure", "failure_kind": "incumbent_missing",
                "trace_id": trace["trace_id"], "mlflow_trace_id": trace["mlflow_trace_id"],
                "session_id": trace["session_id"], "telemetry": delivery,
            }
            if delivery.get("status") != "exported":
                record["task_state"] = record["state"]
                record["state"] = "telemetry_failure"
                record["failure_kind"] = delivery.get("error", "export_failed")
            return record
        prompt = prompt_for(harness, skill_name, case)
        result = run_process(command_for(harness, workspace, prompt), workspace, timeout)
        ended_ns = time.time_ns()
        final, usage, events = parse_harness_output(harness, result)
        state, failure_kind = classify_failure(result, final)
        checks = grade(case, workspace / "output") if state == "succeeded" else []
        run_name = f"{case['id']}--{harness}--{variant}--r{repetition}"
        artifact = artifacts / run_name
        if (workspace / "output").exists():
            shutil.copytree(workspace / "output", artifact, dirs_exist_ok=True)
        passed = sum(item["passed"] for item in checks)
        score = passed / len(checks) if checks else 0.0
        blockers = [item["name"] for item in checks if item["blocking"] and not item["passed"]]
        accepted = state == "succeeded" and score >= ROUTES["comparison_gate"]["minimum_candidate_score"] and not blockers
        evaluation_ended_ns = time.time_ns()
        trace, delivery = export_evaluation_trace(
            case=case, harness=harness, variant=variant, repetition=repetition,
            skill_name=skill_name, skill_hash=source_hash, prompt=prompt,
            started_ns=started_ns, ended_ns=ended_ns,
            evaluation_ended_ns=evaluation_ended_ns, state=state,
            accepted=accepted, score=score, passed=passed, total=len(checks),
            usage=usage, events=events,
        )
        record = {
            "id": case["id"], "split": case["split"], "risk": case["risk"],
            "harness": harness, "variant": variant, "repetition": repetition,
            "model": ROUTES["harnesses"][harness]["model"],
            "effort": ROUTES["harnesses"][harness]["effort"],
            "skill_hash": source_hash, "state": state, "failure_kind": failure_kind,
            "valid": state in {"succeeded", "task_failure"},
            "accepted": accepted,
            "score": round(score, 4), "assertions_passed": passed,
            "assertions_total": len(checks), "assertions": checks,
            "blocking_failures": blockers, "duration_seconds": round(result.duration_seconds, 3),
            "usage": usage, "returncode": result.returncode,
            "trace_id": trace["trace_id"], "mlflow_trace_id": trace["mlflow_trace_id"],
            "telemetry": delivery, "artifact": str(artifact),
            "session_id": trace["session_id"],
            "prompt_version": task_trace.sha256_text(prompt),
            "tool_version": harness_version(harness),
            "evaluator_version": task_trace.sha256_path(Path(__file__)),
            "repository_revision": git_value("rev-parse", "HEAD"),
            "model_returned": returned_model(events),
            "outcome": "accepted" if accepted else (
                "invalid_harness" if state == "harness_failure" else
                "invalid_environment" if state == "environment_failure" else
                "evaluator_error" if state == "evaluator_failure" else "failed"
            ),
            "final_response": final, "stderr": result.stderr,
            "event_count": len(events),
        }
        if delivery.get("status") != "exported":
            record["task_state"] = record["state"]
            record["state"] = "telemetry_failure"
            record["failure_kind"] = delivery.get("error", "export_failed")
            record["valid"] = False
            record["accepted"] = False
        return record


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--harness", choices=("all", *HARNESSES), default="all")
    parser.add_argument("--variant", choices=("both", *VARIANTS), default="both")
    parser.add_argument("--suite", choices=("smoke", "development", "held-out", "full"), default="development")
    parser.add_argument("--repetitions", type=int)
    parser.add_argument("--jobs", type=int, default=2)
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--publish-mlflow", action="store_true")
    args = parser.parse_args()
    selected_cases = CASES[:1] if args.suite == "smoke" else [case for case in CASES if args.suite == "full" or case["split"] == args.suite]
    harnesses = HARNESSES if args.harness == "all" else (args.harness,)
    variants = VARIANTS if args.variant == "both" else (args.variant,)
    repetitions = args.repetitions or (1 if args.suite == "smoke" else 5 if args.suite == "held-out" else 3)
    if args.list:
        print(json.dumps({"cases": [case["id"] for case in selected_cases], "harnesses": harnesses, "variants": variants, "repetitions": repetitions}, indent=2))
        return 0
    missing = [harness for harness in harnesses if shutil.which(harness) is None]
    if missing:
        raise SystemExit(f"missing harnesses: {', '.join(missing)}")
    output = args.output.resolve()
    artifacts = output.parent / f"{output.stem}-artifacts"
    tasks = [(case, harness, variant, repetition) for case in selected_cases for harness in harnesses for variant in variants for repetition in range(1, repetitions + 1)]
    results: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, args.jobs)) as executor:
        futures = {executor.submit(run_once, case, harness, variant, repetition, args.timeout, artifacts): (case["id"], harness, variant, repetition) for case, harness, variant, repetition in tasks}
        for future in concurrent.futures.as_completed(futures):
            identity = futures[future]
            try:
                result = future.result()
            except Exception as error:  # Preserve the matrix; comparator treats this as invalid.
                case_id, harness, variant, repetition = identity
                case = next(case for case in selected_cases if case["id"] == case_id)
                timestamp = time.time_ns()
                emergency_trace, delivery = export_evaluation_trace(
                    case=case, harness=harness, variant=variant, repetition=repetition,
                    skill_name="skill-development" if variant == "candidate" else "skill-creator",
                    skill_hash="not_observed",
                    prompt=prompt_for(
                        harness,
                        "skill-development" if variant == "candidate" else "skill-creator",
                        case,
                    ),
                    started_ns=timestamp, ended_ns=timestamp,
                    evaluation_ended_ns=timestamp, state="evaluator_failure",
                    accepted=False, score=0.0, passed=0, total=0, usage={}, events=[],
                )
                result = {
                    "id": case_id,
                    "split": next(case["split"] for case in selected_cases if case["id"] == case_id),
                    "harness": harness,
                    "variant": variant,
                    "repetition": repetition,
                    "valid": False,
                    "accepted": False,
                    "score": 0.0,
                    "blocking_failures": [],
                    "state": "evaluator_failure",
                    "failure_kind": type(error).__name__,
                    "stderr": str(error),
                    "trace_id": emergency_trace["trace_id"],
                    "mlflow_trace_id": emergency_trace["mlflow_trace_id"],
                    "telemetry": delivery,
                }
            results.append(result)
            print(f"{identity}: state={result['state']} score={result['score']:.3f} accepted={result['accepted']}", flush=True)
    results.sort(key=lambda item: (item["id"], item["harness"], item["variant"], item["repetition"]))
    document = {
        "schema_version": "1.0.0",
        "configuration": {"skill": "skill-development", "suite": args.suite, "repetitions": repetitions, "harnesses": list(harnesses), "variants": list(variants), "routes": ROUTES["harnesses"], "fixed": ROUTES["fixed"], "content_capture": False},
        "summary": {"runs": len(results), "valid_runs": sum(item["valid"] for item in results), "accepted": sum(item["accepted"] for item in results), "duration_seconds": round(sum(item.get("duration_seconds", 0) for item in results), 3)},
        "results": results,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    if args.publish_mlflow:
        subprocess.run(
            ["uv", "run", str(REPO_ROOT / "config/agents/telemetry/publish_evals.py"), str(output)],
            check=True,
        )
    print(f"results: {output}")
    print(f"artifacts: {artifacts}")
    return 0 if all(item["valid"] for item in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
