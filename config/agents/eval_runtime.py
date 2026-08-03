#!/usr/bin/env python3
"""Cross-harness skill evaluation runtime.

Per-skill packages own their cases, routes, wrappers, and results. This module
owns only repeatable execution and scoring mechanics so variants are compared
with the same harness route and verifier.
"""

from __future__ import annotations

import argparse
import copy
import functools
import json
import os
import re
import secrets
import shutil
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

from telemetry import task_trace


HARNESS_COMMANDS = {"codex": "codex", "claude": "claude", "pi": "pi"}
VARIANTS = ("no-skill", "incumbent", "candidate")
MODES = ("routing", "conditional", "end-to-end")
USAGE_KEYS = (
    "input_tokens",
    "cached_input_tokens",
    "cache_write_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
)


@functools.lru_cache(maxsize=None)
def harness_version(harness: str) -> str:
    try:
        return subprocess.check_output(
            [HARNESS_COMMANDS[harness], "--version"], text=True,
            stderr=subprocess.DEVNULL, timeout=10,
        ).strip() or "not_observed"
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return "not_observed"


def read_json(path: Path) -> Any:
    return json.loads(path.read_text())


def nested_value(value: Any, path: str) -> Any:
    for part in path.split("."):
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def score_assertion(output: str, assertion: dict[str, Any]) -> tuple[bool, str]:
    kind = assertion["type"]
    if kind == "contains":
        expected = assertion["value"]
        return expected.casefold() in output.casefold(), f"contains {expected!r}"
    if kind == "contains_any":
        values = assertion["values"]
        return any(v.casefold() in output.casefold() for v in values), (
            f"contains any of {values!r}"
        )
    if kind == "not_contains":
        expected = assertion["value"]
        return expected.casefold() not in output.casefold(), f"omits {expected!r}"
    if kind in {"regex", "not_regex"}:
        pattern = assertion["pattern"]
        matched = re.search(pattern, output, re.MULTILINE | re.IGNORECASE) is not None
        passed = matched if kind == "regex" else not matched
        return passed, f"{kind} /{pattern}/"
    if kind == "max_words":
        limit = assertion["value"]
        count = len(re.findall(r"\b[\w'-]+\b", output))
        return count <= limit, f"word count {count} <= {limit}"
    parsed = json.loads(output)
    actual = nested_value(parsed, assertion["path"])
    if kind == "json_equals":
        expected = assertion["value"]
        return actual == expected, f"{assertion['path']} == {expected!r} (got {actual!r})"
    if kind == "json_in":
        expected = assertion["values"]
        return actual in expected, f"{assertion['path']} in {expected!r} (got {actual!r})"
    if kind == "json_truthy":
        return bool(actual), f"{assertion['path']} is truthy (got {actual!r})"
    raise ValueError(f"unknown assertion type: {kind}")


def assertion_results(output: str, assertions: list[dict[str, Any]]) -> list[dict[str, Any]]:
    scored = []
    for assertion in assertions:
        try:
            passed, detail = score_assertion(output, assertion)
        except (json.JSONDecodeError, KeyError, IndexError, TypeError, ValueError) as error:
            passed, detail = False, f"assertion error: {error}"
        scored.append({"passed": passed, "detail": detail})
    return scored


def skill_context(eval_dir: Path, case: dict[str, Any], variant: str) -> str | None:
    if variant == "no-skill":
        return None
    root = eval_dir.parent if variant == "candidate" else eval_dir / "incumbent"
    skill_file = root / "SKILL.md"
    if not skill_file.exists():
        return None
    sections = [skill_file.read_text()]
    for name in case.get("references", []):
        reference = root / "references" / name
        if reference.exists():
            sections.append(reference.read_text())
    return "\n\n".join(sections)


def catalogue_metadata(context: str | None) -> str:
    if not context:
        return "No matching skill is available."
    match = re.match(r"---\n(.*?)\n---", context, re.DOTALL)
    return match.group(1).strip() if match else context.split("\n\n", 1)[0]


def prompt_for(
    eval_dir: Path,
    case: dict[str, Any],
    variant: str,
    mode: str,
) -> str:
    context = skill_context(eval_dir, case, variant)
    if mode == "routing":
        return f"""Decide whether the catalogue entry should activate for the request.
Return only JSON with keys `activate` (boolean) and `reason` (short string).

<catalogue_entry>
{catalogue_metadata(context)}
</catalogue_entry>

<request>
{case['prompt']}
</request>
"""
    if mode == "conditional" and variant != "no-skill" and context:
        instruction = f"Use the following skill instructions.\n\n<skill>\n{context}\n</skill>"
    elif mode == "end-to-end" and variant != "no-skill" and context:
        instruction = (
            "Choose whether the catalogue entry applies, and if it does, follow the package. "
            f"Do not mention that routing choice.\n\n<catalogue_entry>\n"
            f"{catalogue_metadata(context)}\n</catalogue_entry>\n\n<skill>\n{context}\n</skill>"
        )
    else:
        instruction = "Answer using normal software-engineering judgment."
    return f"""{instruction}

<task>
{case['prompt']}
</task>

Do not call tools or modify files. Return only the requested answer.
"""


def copy_auth(source: Path, destination: Path) -> None:
    if source.is_file():
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        destination.chmod(0o600)


def isolated_environment(harness: str, temp_root: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["APP_AGENT_EVAL_RUN"] = "1"
    env["APP_AGENT_TRACE_KIND"] = "evaluation"
    if harness == "codex":
        codex_home = temp_root / "codex"
        codex_home.mkdir(mode=0o700)
        source_home = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))
        copy_auth(source_home / "auth.json", codex_home / "auth.json")
        (codex_home / "config.toml").write_text(
            """[otel]
environment = "evaluation"
log_user_prompt = false
exporter = "none"
metrics_exporter = "none"
trace_exporter = "none"
"""
        )
        env["CODEX_HOME"] = str(codex_home)
    elif harness == "claude":
        settings = temp_root / "claude-settings.json"
        settings.write_text(
            json.dumps(
                {
                    "env": {
                        "CLAUDE_CODE_ENABLE_TELEMETRY": "1",
                        "OTEL_METRICS_EXPORTER": "none",
                        "OTEL_LOGS_EXPORTER": "none",
                        "OTEL_TRACES_EXPORTER": "none",
                        "OTEL_LOG_USER_PROMPTS": "0",
                        "OTEL_LOG_TOOL_DETAILS": "0",
                        "OTEL_LOG_TOOL_CONTENT": "0",
                        "OTEL_LOG_RAW_API_BODIES": "0",
                        "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH": "61440",
                    }
                }
            )
            + "\n"
        )
        env["APP_AGENT_CLAUDE_SETTINGS"] = str(settings)
    elif harness == "pi":
        pi_home = temp_root / "pi"
        pi_home.mkdir(mode=0o700)
        source_home = Path(
            os.environ.get("PI_CODING_AGENT_DIR", Path.home() / ".pi" / "agent")
        )
        copy_auth(source_home / "auth.json", pi_home / "auth.json")
        env["PI_CODING_AGENT_DIR"] = str(pi_home)
    return env


def command_for(
    harness: str,
    route: dict[str, str],
    prompt: str,
    temp_root: Path,
    eval_dir: Path,
) -> list[str]:
    model, effort = route["model"], route["effort"]
    if harness == "codex":
        return [
            "codex", "exec", "--ephemeral", "--skip-git-repo-check", "--sandbox",
            "read-only", "--json", "-m", model, "-c",
            f'model_reasoning_effort="{effort}"', prompt,
        ]
    if harness == "claude":
        return [
            "claude", "-p", "--output-format", "json", "--no-session-persistence",
            "--tools", "", "--permission-mode", "dontAsk", "--settings",
            str(temp_root / "claude-settings.json"), "--setting-sources", "project",
            "--model", model, "--effort", effort, prompt,
        ]
    return [
        "pi", "--print", "--no-session", "--no-tools", "--no-extensions",
        "--no-skills", "--no-context-files",
        "--model", model, "--thinking", effort, prompt,
    ]


def parse_output(harness: str, stdout: str) -> tuple[str, dict[str, int], list[Any]]:
    if harness == "codex":
        events = []
        for line in stdout.splitlines():
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                continue
        messages = [
            event["item"]["text"]
            for event in events
            if event.get("type") == "item.completed"
            and event.get("item", {}).get("type") == "agent_message"
        ]
        usage_events = [event.get("usage", {}) for event in events if event.get("type") == "turn.completed"]
        return (messages[-1] if messages else "", usage_events[-1] if usage_events else {}, events)
    if harness == "claude":
        try:
            event = json.loads(stdout)
        except json.JSONDecodeError:
            return stdout.strip(), {}, []
        usage = event.get("usage", {})
        return str(event.get("result", "")), usage, [event]
    return stdout.strip(), {}, []


def classify_failure(returncode: int, stderr: str, output: str) -> str:
    if returncode == 0 and output:
        return "succeeded"
    diagnostic = stderr.casefold()
    if any(word in diagnostic for word in ("login", "authentication", "api key", "not found")):
        return "harness_failure"
    if any(word in diagnostic for word in ("network", "dns", "connection", "timed out")):
        return "environment_failure"
    return "task_failure"


def failure_kind_for(state: str, stderr: str, returncode: int) -> str | None:
    if state == "succeeded":
        return None
    diagnostic = stderr.casefold()
    if any(word in diagnostic for word in ("login", "authentication", "oauth", "api key", "invalid_grant")):
        return "authentication"
    if returncode == 127 or "not found" in diagnostic:
        return "harness_not_found"
    if returncode == 124 or "timed out" in diagnostic:
        return "timeout"
    if any(word in diagnostic for word in ("network", "dns", "connection")):
        return "network"
    return "model_or_task"


def git_value(repository: Path, *arguments: str) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(repository), *arguments],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip() or "not_observed"
    except (OSError, subprocess.CalledProcessError):
        return "not_observed"


def returned_model_for(events: list[Any]) -> str:
    for event in reversed(events):
        if not isinstance(event, dict):
            continue
        for key in ("response_model", "responseModel", "model"):
            value = event.get(key)
            if isinstance(value, str) and value:
                return value
        message = event.get("message")
        if isinstance(message, dict):
            value = message.get("model")
            if isinstance(value, str) and value:
                return value
    return "not_observed"


def observed_cost(usage: dict[str, Any]) -> float | None:
    for value in (usage.get("cost_usd"), usage.get("total_cost_usd")):
        if isinstance(value, (int, float)):
            return float(value)
    cost = usage.get("cost")
    if isinstance(cost, dict) and isinstance(cost.get("total"), (int, float)):
        return float(cost["total"])
    return None


def outcome_for(state: str, accepted: bool) -> str:
    if accepted:
        return "accepted"
    return {
        "succeeded": "failed",
        "task_failure": "failed",
        "harness_failure": "invalid_harness",
        "environment_failure": "invalid_environment",
        "evaluator_failure": "evaluator_error",
    }.get(state, "not_observed")


def evaluation_children(
    *,
    process_started_ns: int,
    process_ended_ns: int,
    evaluation_ended_ns: int,
    route: dict[str, str],
    usage: dict[str, Any],
    skill_name: str,
    skill_hash: str,
    variant: str,
    mode: str,
    assertions_passed: int,
    assertions_total: int,
    outcome: str,
    evaluator_version: str,
) -> list[dict[str, Any]]:
    invocation_attributes: dict[str, str | int | float | bool] = {
        "gen_ai.operation.name": "invoke_agent",
        "gen_ai.request.model": route["model"],
        "app.agent.model.effort": route["effort"],
    }
    for source, target in (
        ("input_tokens", "gen_ai.usage.input_tokens"),
        ("output_tokens", "gen_ai.usage.output_tokens"),
        ("cached_input_tokens", "app.agent.tokens.cached"),
        ("reasoning_output_tokens", "app.agent.tokens.reasoning"),
    ):
        value = usage.get(source)
        if isinstance(value, int):
            invocation_attributes[target] = value
    children = [task_trace.child_span(
        "gen_ai.invoke_agent",
        started_ns=process_started_ns,
        ended_ns=process_ended_ns,
        attributes=invocation_attributes,
    )]
    if variant != "no-skill":
        stages = ["offered", "evaluated"]
        if mode in {"conditional", "end-to-end"}:
            stages[1:1] = ["selected", "activated", "expanded", "executed"]
        for index, stage in enumerate(stages):
            timestamp = min(process_ended_ns, process_started_ns + index)
            children.append(task_trace.child_span(
                "skill.activate" if stage == "activated" else "skill.lifecycle",
                started_ns=timestamp,
                ended_ns=timestamp,
                attributes={
                    "app.agent.record.type": "skill",
                    "app.agent.skill.name": skill_name,
                    "app.agent.skill.package_hash": skill_hash,
                    "app.agent.skill.activation": stage,
                    "app.agent.skill.selection": "user" if mode == "conditional" else "router",
                },
            ))
    children.extend([
        task_trace.child_span(
            "validation.run",
            started_ns=process_ended_ns,
            ended_ns=evaluation_ended_ns,
            attributes={
                "app.agent.record.type": "validation",
                "app.agent.validation.type": "packaged_assertions",
                "app.agent.validation.status": "pass" if outcome == "accepted" else "fail",
                "app.agent.validation.passed": assertions_passed,
                "app.agent.validation.failed": max(0, assertions_total - assertions_passed),
            },
            status="ok" if outcome == "accepted" else "error",
        ),
        task_trace.child_span(
            "evaluator.run",
            started_ns=process_ended_ns,
            ended_ns=evaluation_ended_ns,
            attributes={
                "app.agent.evaluator.name": "packaged-assertion-scorer",
                "app.agent.evaluator.version": evaluator_version,
                "app.agent.outcome.status": outcome,
            },
            status="error" if outcome == "evaluator_error" else "ok",
        ),
        task_trace.child_span(
            "agent.final",
            started_ns=evaluation_ended_ns,
            ended_ns=evaluation_ended_ns,
            attributes={
                "app.agent.record.type": "outcome",
                "app.agent.final.status": outcome,
                "app.agent.outcome.verifier": f"packaged-assertion-scorer@{evaluator_version}",
            },
        ),
    ])
    return children


def run_once(
    eval_dir: Path,
    case: dict[str, Any],
    harness: str,
    variant: str,
    mode: str,
    route: dict[str, str],
    repetition: int,
    timeout: int,
) -> dict[str, Any]:
    run_started_ns = time.time_ns()
    context = skill_context(eval_dir, case, variant)
    if variant == "incumbent" and context is None:
        prompt = prompt_for(eval_dir, case, variant, mode)
        evaluator_version = task_trace.sha256_path(Path(__file__))
        repository = eval_dir.parents[4]
        attributes = task_trace.evaluation_attributes(
            case_id=case["id"], variant=variant, repetition=repetition, mode=mode,
            skill_name=eval_dir.parent.name, skill_hash="not_observed",
            skill_source=variant,
            repository_hash=task_trace.sha256_text(str(repository.resolve())),
            base_revision=git_value(repository, "rev-parse", "HEAD"),
            model_requested=route["model"], model_returned="not_observed",
            effort=route["effort"], prompt_version=task_trace.sha256_text(prompt),
            tool_version=route.get("harness_version", "not_observed"),
            evaluator_version=evaluator_version, risk=case.get("risk", "normal"),
            outcome="invalid_environment",
            verifier=f"packaged-assertion-scorer@{evaluator_version}",
        )
        trace = task_trace.build_task_trace(
            harness=harness, session_id=f"eval-{secrets.token_hex(16)}",
            task_id=f"{case['id']}-{harness}-{variant}-{mode}-{repetition}",
            started_ns=run_started_ns, ended_ns=time.time_ns(), attributes=attributes,
            children=[task_trace.child_span(
                "agent.final", started_ns=run_started_ns, ended_ns=time.time_ns(),
                attributes={
                    "app.agent.record.type": "outcome",
                    "app.agent.final.status": "invalid_environment",
                    "app.agent.outcome.verifier": f"packaged-assertion-scorer@{evaluator_version}",
                }, status="error",
            )], status="error",
        )
        delivery = task_trace.export_task_trace(
            trace, os.environ.get("APP_AGENT_EVAL_OTLP_ENDPOINT", "http://docker-host:4318/v1/traces"),
        )
        result = {
            "id": case["id"], "harness": harness, "variant": variant, "mode": mode,
            "repetition": repetition, "state": "environment_failure",
            "failure_kind": "incumbent_missing", "accepted": False, "valid": False,
            "assertions": [], "assertions_passed": 0, "assertions_total": 0,
            "output": "", "usage": {}, "duration_seconds": 0.0,
            "trace_id": trace["trace_id"], "mlflow_trace_id": trace["mlflow_trace_id"],
            "telemetry": delivery,
        }
        if delivery.get("status") != "exported":
            result["task_state"] = result["state"]
            result["state"] = "telemetry_failure"
            result["failure_kind"] = delivery.get("error", "export_failed")
        return result
    with tempfile.TemporaryDirectory(prefix="skill-eval-") as temp:
        root = Path(temp)
        workspace = root / "workspace"
        workspace.mkdir()
        env = isolated_environment(harness, root)
        prompt = prompt_for(eval_dir, case, variant, mode)
        command = command_for(harness, route, prompt, root, eval_dir)
        process_started_ns = time.time_ns()
        started = time.monotonic()
        try:
            completed = subprocess.run(
                command, cwd=workspace, env=env, check=False, capture_output=True,
                text=True, timeout=timeout,
            )
            duration = time.monotonic() - started
            output, usage, events = parse_output(harness, completed.stdout)
            state = classify_failure(completed.returncode, completed.stderr, output)
        except FileNotFoundError as error:
            duration, output, usage, events = time.monotonic() - started, "", {}, []
            completed = subprocess.CompletedProcess(command, 127, "", str(error))
            state = "harness_failure"
        except subprocess.TimeoutExpired as error:
            duration, output, usage, events = time.monotonic() - started, "", {}, []
            completed = subprocess.CompletedProcess(command, 124, "", str(error))
            state = "environment_failure"
        process_ended_ns = time.time_ns()

    if mode == "routing":
        expected = bool(case["expected_activation"])
        assertions = [{"type": "json_equals", "path": "activate", "value": expected}]
    else:
        assertions = case["assertions"]
    scored = assertion_results(output, assertions) if state == "succeeded" else []
    telemetry_warning = any(
        word in completed.stderr.casefold() for word in ("otel", "telemetry", "exporter")
    )
    accepted = state == "succeeded" and all(item["passed"] for item in scored)
    actual_activation = None
    if mode == "routing" and state == "succeeded":
        try:
            actual_activation = bool(json.loads(output)["activate"])
        except (json.JSONDecodeError, KeyError, TypeError):
            pass
    evaluation_ended_ns = time.time_ns()
    outcome = outcome_for(state, accepted)
    repository = eval_dir.parents[4]
    skill_root = eval_dir.parent if variant == "candidate" else eval_dir / "incumbent"
    skill_hash = "none" if variant == "no-skill" else task_trace.sha256_path(skill_root)
    evaluator_version = task_trace.sha256_path(Path(__file__))
    prompt_version = task_trace.sha256_text(prompt)
    tool_version = route.get("harness_version", "not_observed")
    trace_attributes = task_trace.evaluation_attributes(
        case_id=case["id"],
        variant=variant,
        repetition=repetition,
        mode=mode,
        skill_name=eval_dir.parent.name if variant != "no-skill" else "none",
        skill_hash=skill_hash,
        skill_source=variant,
        repository_hash=task_trace.sha256_text(str(repository.resolve())),
        base_revision=git_value(repository, "rev-parse", "HEAD"),
        model_requested=route["model"],
        model_returned=returned_model_for(events),
        effort=route["effort"],
        prompt_version=prompt_version,
        tool_version=tool_version,
        evaluator_version=evaluator_version,
        risk=case.get("risk", "normal"),
        outcome=outcome,
        verifier=f"packaged-assertion-scorer@{evaluator_version}",
    )
    trace_attributes.update({
        "app.agent.harness.version": tool_version,
        "app.agent.task.class": "skill_evaluation",
        "gen_ai.input.messages": task_trace.metadata_messages("user", {
            "case_id": case["id"], "variant": variant, "mode": mode,
        }),
        "gen_ai.output.messages": task_trace.metadata_messages("assistant", {
            "outcome": outcome, "assertions_passed": sum(item["passed"] for item in scored),
            "assertions_total": len(scored),
        }),
    })
    cost = observed_cost(usage)
    if cost is not None:
        trace_attributes["app.agent.cost.status"] = "observed"
        trace_attributes["app.agent.cost.usd"] = cost
    session_id = f"eval-{secrets.token_hex(16)}"
    task_id = f"{case['id']}-{harness}-{variant}-{mode}-{repetition}-{secrets.token_hex(4)}"
    trace = task_trace.build_task_trace(
        harness=harness,
        session_id=session_id,
        task_id=task_id,
        started_ns=process_started_ns,
        ended_ns=evaluation_ended_ns,
        attributes=trace_attributes,
        children=evaluation_children(
            process_started_ns=process_started_ns,
            process_ended_ns=process_ended_ns,
            evaluation_ended_ns=evaluation_ended_ns,
            route=route,
            usage=usage,
            skill_name=eval_dir.parent.name if variant != "no-skill" else "none",
            skill_hash=skill_hash,
            variant=variant,
            mode=mode,
            assertions_passed=sum(item["passed"] for item in scored),
            assertions_total=len(scored),
            outcome=outcome,
            evaluator_version=evaluator_version,
        ),
        status="error" if state not in {"succeeded", "task_failure"} else "ok",
    )
    delivery = task_trace.export_task_trace(
        trace,
        os.environ.get("APP_AGENT_EVAL_OTLP_ENDPOINT", "http://docker-host:4318/v1/traces"),
    )
    result = {
        "id": case["id"], "split": case.get("split", "development"),
        "risk": case.get("risk", "normal"), "harness": harness, "variant": variant,
        "mode": mode, "repetition": repetition, "model": route["model"],
        "effort": route["effort"], "state": state,
        "failure_kind": failure_kind_for(state, completed.stderr, completed.returncode),
        "valid": state in {"succeeded", "task_failure"}, "accepted": accepted,
        "expected_activation": case.get("expected_activation"),
        "actual_activation": actual_activation,
        "assertions_passed": sum(item["passed"] for item in scored),
        "assertions_total": len(scored), "assertions": scored, "output": output,
        "usage": usage, "duration_seconds": round(duration, 3),
        "returncode": completed.returncode, "stderr": completed.stderr,
        "events": events, "telemetry_warning": telemetry_warning,
        "trace_id": trace["trace_id"], "mlflow_trace_id": trace["mlflow_trace_id"],
        "session_id": session_id, "telemetry": delivery,
        "skill_hash": skill_hash, "prompt_version": prompt_version,
        "tool_version": tool_version, "evaluator_version": evaluator_version,
        "repository_revision": trace_attributes["app.agent.repository.base_revision"],
        "model_returned": trace_attributes["app.agent.model.returned"],
        "outcome": outcome,
    }
    if delivery.get("status") != "exported":
        result["task_state"] = result["state"]
        result["state"] = "telemetry_failure"
        result["failure_kind"] = delivery.get("error", "export_failed")
        result["valid"] = False
        result["accepted"] = False
    return result


def replay(eval_dir: Path, source: Path, output: Path) -> None:
    document = read_json(source)
    outcome_cases = {case["id"]: case for case in read_json(eval_dir / "cases.json")}
    routing_cases = {case["id"]: case for case in read_json(eval_dir / "routing-cases.json")}
    results = []
    for original in document["results"]:
        result = copy.deepcopy(original)
        case = routing_cases[result["id"]] if result["mode"] == "routing" else outcome_cases[result["id"]]
        assertions = (
            [{"type": "json_equals", "path": "activate", "value": bool(case["expected_activation"])}]
            if result["mode"] == "routing" else case["assertions"]
        )
        scored = assertion_results(result.get("output", ""), assertions)
        actual_activation = result.get("actual_activation")
        if result["mode"] == "routing":
            try:
                actual_activation = bool(json.loads(result.get("output", ""))["activate"])
            except (json.JSONDecodeError, KeyError, TypeError):
                actual_activation = None
        result.update(
            accepted=result.get("state") == "succeeded" and all(item["passed"] for item in scored),
            assertions=scored,
            assertions_passed=sum(item["passed"] for item in scored),
            assertions_total=len(scored),
            actual_activation=actual_activation,
            failure_kind=result.get("failure_kind") or failure_kind_for(
                result.get("state", "task_failure"),
                result.get("stderr", ""),
                result.get("returncode", 1),
            ),
        )
        results.append(result)
    write_results(output, document.get("configuration", {}), results)


def write_results(output: Path, configuration: dict[str, Any], results: list[dict[str, Any]]) -> None:
    valid = [result for result in results if result.get("valid")]
    usage = {key: sum(result.get("usage", {}).get(key, 0) or 0 for result in valid) for key in USAGE_KEYS}
    durations = sorted(result.get("duration_seconds", 0) for result in valid)

    def percentile(fraction: float) -> float | None:
        if not durations:
            return None
        index = min(len(durations) - 1, round((len(durations) - 1) * fraction))
        return round(durations[index], 3)

    summary = {
        "runs": len(results), "valid_runs": len(valid),
        "accepted": sum(result.get("accepted", False) for result in valid),
        "invalid_by_state": {
            state: sum(result.get("state") == state for result in results)
            for state in ("harness_failure", "environment_failure", "task_failure")
        },
        "telemetry_warnings": sum(result.get("telemetry_warning", False) for result in results),
        "duration_seconds": round(sum(result.get("duration_seconds", 0) for result in results), 3),
        "latency_p50_seconds": percentile(0.50),
        "latency_p95_seconds": percentile(0.95),
        "usage": usage,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({"schema_version": "2.0.0", "configuration": configuration, "summary": summary, "results": results}, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


def main(eval_dir: Path) -> None:
    parser = argparse.ArgumentParser(description=f"Run {eval_dir.parent.name} evaluations")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--harness", choices=(*HARNESS_COMMANDS, "all"), default="all")
    parser.add_argument("--variant", choices=(*VARIANTS, "all"), default="all")
    parser.add_argument("--mode", choices=(*MODES, "all"), default="all")
    parser.add_argument("--suite", choices=("development", "held-out", "full", "smoke"), default="development")
    parser.add_argument("--repetitions", type=int)
    parser.add_argument("--timeout", type=int, default=240)
    parser.add_argument("--replay", type=Path)
    parser.add_argument("--publish-mlflow", action="store_true")
    args = parser.parse_args()
    if args.replay:
        replay(eval_dir, args.replay, args.output)
        return
    routes = read_json(eval_dir / "routes.json")
    harnesses = list(HARNESS_COMMANDS) if args.harness == "all" else [args.harness]
    variants = list(VARIANTS) if args.variant == "all" else [args.variant]
    modes = list(MODES) if args.mode == "all" else [args.mode]
    repetitions = args.repetitions or (1 if args.suite == "smoke" else 5 if args.suite == "held-out" else 3)
    results = []
    for mode in modes:
        cases = read_json(eval_dir / ("routing-cases.json" if mode == "routing" else "cases.json"))
        if args.suite not in {"full", "smoke"}:
            cases = [case for case in cases if case.get("split", "development") == args.suite]
        elif args.suite == "smoke":
            cases = cases[:1]
        for case in cases:
            for harness in harnesses:
                route = dict(routes["harnesses"][harness])
                route["harness_version"] = harness_version(harness)
                for variant in variants:
                    for repetition in range(1, repetitions + 1):
                        print(f"running {case['id']} ({mode}/{harness}/{variant} #{repetition})", flush=True)
                        results.append(run_once(eval_dir, case, harness, variant, mode, route, repetition, args.timeout))
    configuration = {
        "skill": eval_dir.parent.name, "suite": args.suite, "repetitions": repetitions,
        "harnesses": harnesses, "variants": variants, "modes": modes,
        "routes": routes["harnesses"], "fixed": routes.get("fixed", []),
    }
    write_results(args.output, configuration, results)
    if args.publish_mlflow:
        subprocess.run(
            ["uv", "run", str(Path(__file__).parent / "telemetry" / "publish_evals.py"), str(args.output)],
            check=True,
        )
