#!/usr/bin/env python3
"""Run isolated Neo evaluations across Codex, Claude, and Pi."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
CASES = HERE / "cases.json"
JUDGES = HERE / "judges.json"
ROUTES = json.loads((HERE / "routes.json").read_text(encoding="utf-8"))
HARNESSES = ("codex", "claude", "pi")
MAX_FINAL_MESSAGE_CHARS = 100_000


@dataclass
class RunResult:
    harness: str
    case_id: str
    skill_enabled: bool
    returncode: int
    duration_seconds: float
    timed_out: bool
    state: dict[str, Any] | None
    artifacts: dict[str, str]
    steps: list[dict[str, Any]]
    deterministic: dict[str, Any]
    raw_streams: list[dict[str, str]] | None


def json_events(stream: str) -> list[dict[str, Any]]:
    events = []
    for line in stream.splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            events.append(value)
    return events


def content_text(value: Any) -> list[str]:
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [text for item in value for text in content_text(item)]
    if not isinstance(value, dict):
        return []
    if value.get("type") in {"text", "output_text"} and isinstance(
        value.get("text"), str
    ):
        return [value["text"]]
    return content_text(value.get("content"))


def assistant_messages(events: list[dict[str, Any]]) -> list[str]:
    messages = []
    for event in events:
        item = event.get("item")
        if isinstance(item, dict) and item.get("type") == "agent_message":
            messages.extend(content_text(item.get("text") or item.get("content")))
            continue
        message = event.get("message")
        if isinstance(message, dict) and (
            event.get("type") == "assistant" or message.get("role") == "assistant"
        ):
            messages.extend(content_text(message.get("content")))
            continue
        if event.get("type") == "result" and isinstance(event.get("result"), str):
            messages.append(event["result"])
    deduplicated = []
    for message in messages:
        message = message.strip()
        if message and (not deduplicated or deduplicated[-1] != message):
            deduplicated.append(message)
    return deduplicated


def tool_counts(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    calls: Counter[str] = Counter()
    failures: Counter[str] = Counter()

    def visit(value: Any) -> None:
        if isinstance(value, list):
            for item in value:
                visit(item)
            return
        if not isinstance(value, dict):
            return
        event_type = str(value.get("type", ""))
        if event_type in {
            "tool_use",
            "tool_call",
            "function_call",
            "command_execution",
            "mcp_tool_call",
        }:
            name = str(value.get("name") or event_type)
            calls[name] += 1
            if value.get("is_error") or value.get("status") in {"failed", "error"}:
                failures[name] += 1
        elif event_type == "tool_result" and value.get("is_error"):
            failures[str(value.get("name") or "tool_result")] += 1
        for item in value.values():
            visit(item)

    visit(events)
    return [
        {"name": name, "calls": calls[name], "failures": failures[name]}
        for name in sorted(calls.keys() | failures.keys())
    ]


def normalize_stream(stdout: str, stderr: str) -> dict[str, Any]:
    events = json_events(stdout)
    messages = assistant_messages(events)
    final_message = messages[-1] if messages else "" if events else stdout.strip()
    if len(final_message) > MAX_FINAL_MESSAGE_CHARS:
        omitted = len(final_message) - MAX_FINAL_MESSAGE_CHARS
        final_message = (
            final_message[:MAX_FINAL_MESSAGE_CHARS]
            + f"\n[normalized message truncated; {omitted} characters omitted]"
        )
    error = ""
    if stderr.strip():
        error = stderr.strip()[-2000:]
    return {
        "final_message": final_message,
        "tool_summary": tool_counts(events),
        "error": error,
    }


def bounded_jobs(value: str) -> int:
    jobs = int(value)
    if not 1 <= jobs <= 3:
        raise argparse.ArgumentTypeError("jobs must be between 1 and 3")
    return jobs


def load_cases(suite: str) -> list[dict[str, Any]]:
    data = json.loads(CASES.read_text(encoding="utf-8"))
    cases = data["cases"]
    if suite == "smoke":
        cases = [case for case in cases if case.get("smoke")]
    return cases


def command_for(
    harness: str, workspace: Path, prompt: str, skill_enabled: bool
) -> list[str]:
    route = ROUTES["harnesses"][harness]
    if harness == "codex":
        return [
            "codex",
            "exec",
            "--json",
            "--ephemeral",
            "--skip-git-repo-check",
            "--ignore-user-config",
            "-C",
            str(workspace),
            "--sandbox",
            "workspace-write",
            "-m",
            route["model"],
            "-c",
            f'model_reasoning_effort="{route["effort"]}"',
            prompt,
        ]
    if harness == "claude":
        command = [
            "claude",
            "--print",
            prompt,
            "--output-format",
            "stream-json",
            "--verbose",
            "--no-session-persistence",
            "--setting-sources",
            "project",
            "--permission-mode",
            "acceptEdits",
            "--allowedTools",
            "Read",
            "Glob",
            "Grep",
            "Edit",
            "Write",
            "Bash(python3 .agents/skills/neo/scripts/neo.py *)",
            "--model",
            route["model"],
            "--effort",
            route["effort"],
        ]
        if not skill_enabled:
            command.append("--disable-slash-commands")
        return command
    if harness == "pi":
        command = [
            "pi",
            "--print",
            "--mode",
            "json",
            "--no-session",
            "--offline",
            "--approve",
            "--no-extensions",
            "--no-prompt-templates",
            "--no-themes",
            "--tools",
            "read,bash,edit,write,grep,find,ls",
            "--model",
            route["model"],
            "--thinking",
            route["effort"],
        ]
        if not skill_enabled:
            command.append("--no-skills")
        command.append(prompt)
        return command
    raise ValueError(f"unsupported harness: {harness}")


def run_process(command: list[str], workspace: Path, timeout: int) -> tuple:
    environment = os.environ.copy()
    for key in tuple(environment):
        if key.startswith("FZF_"):
            environment.pop(key)
    started = time.monotonic()
    try:
        result = subprocess.run(
            command,
            cwd=workspace,
            env=environment,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        return (
            result.returncode,
            time.monotonic() - started,
            result.stdout,
            result.stderr,
            False,
        )
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout.decode() if isinstance(error.stdout, bytes) else error.stdout
        stderr = error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr
        return (124, time.monotonic() - started, stdout or "", stderr or "", True)


def prepare_workspace(destination: Path, skill_enabled: bool) -> None:
    (destination / "src").mkdir(parents=True)
    (destination / "src" / "service.py").write_text(
        "\"\"\"Synthetic service fixture for Neo evaluation.\"\"\"\n",
        encoding="utf-8",
    )
    (destination / "tests").mkdir()
    (destination / "tests" / "test_service.py").write_text(
        "\"\"\"Synthetic test fixture for Neo evaluation.\"\"\"\n",
        encoding="utf-8",
    )
    (destination / "AGENTS.md").write_text(
        "# Evaluation Repository\n\nPlan only. Do not implement the requested change.\n",
        encoding="utf-8",
    )
    if not skill_enabled:
        return
    agents = destination / ".agents" / "skills"
    agents.mkdir(parents=True)
    source_skills = SUITE.parent
    for source in source_skills.glob("neo*"):
        if source.is_dir():
            shutil.copytree(source, agents / source.name)
    claude = destination / ".claude" / "skills"
    claude.mkdir(parents=True)
    for source in agents.glob("neo*"):
        (claude / source.name).symlink_to(
            Path("../../.agents/skills") / source.name
        )


def initial_prompt(
    case: dict[str, Any], skill_enabled: bool, route: str | None = None
) -> str:
    if skill_enabled and route == "direct":
        prefix = (
            "Neo's deterministic preflight selected the direct route. Plan this "
            "software change normally without invoking a Neo stage or creating "
            "Neo artifacts. "
        )
    elif skill_enabled:
        prefix = (
            f"Use $neo-discover for the existing task eval-{case['id']}. "
            "Routing and the discover handoff are already recorded. Do not repeat "
            "router preflight. Continue consecutive Neo stages in this session "
            "until consequential user input is required. "
        )
    else:
        prefix = "Plan this software change without using any project skill. "
    signals = ", ".join(case["approved_risk_signals"]) or "none"
    return (
        f"{prefix}{case['prompt']}\n\n"
        f"The user explicitly confirms these risk signals: {signals}.\n"
        f"Approved context: {case['approved_context']}\n"
        "Do not claim approval for any other decision. Stop when user input is required."
    )


def bootstrap_neo(
    workspace: Path, case: dict[str, Any], timeout: int
) -> tuple[list[str], tuple]:
    command = [
        sys.executable,
        str(workspace / ".agents/skills/neo/scripts/neo.py"),
        "start",
        f"eval-{case['id']}",
        "--title",
        case["prompt"],
        "--signals",
        ",".join(case["approved_risk_signals"]),
    ]
    return command, run_process(command, workspace, min(timeout, 30))


def read_state(workspace: Path, case_id: str) -> dict[str, Any] | None:
    path = workspace / ".neo" / "tasks" / f"eval-{case_id}" / "state.json"
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None


def collect_artifacts(workspace: Path, case_id: str) -> dict[str, str]:
    directory = workspace / ".neo" / "tasks" / f"eval-{case_id}"
    if not directory.is_dir():
        return {}
    artifacts = {}
    for path in sorted(directory.rglob("*")):
        if path.is_file() and path.name != "state.json":
            try:
                artifacts[str(path.relative_to(directory))] = path.read_text(
                    encoding="utf-8"
                )
            except UnicodeDecodeError:
                artifacts[str(path.relative_to(directory))] = "<binary artifact>"
    return artifacts


def deterministic_result(
    result_code: int,
    timed_out: bool,
    state: dict[str, Any] | None,
    expected_route: str,
    skill_enabled: bool,
) -> dict[str, Any]:
    checks = {
        "process_healthy": result_code == 0 and not timed_out,
        "state_created": state is not None if skill_enabled else True,
        "route_matches": (
            state is not None and state.get("route") == expected_route
            if skill_enabled
            else True
        ),
    }
    return {"checks": checks, "pass": all(checks.values())}


def run_judges(
    judge_harness: str,
    case: dict[str, Any],
    candidate: RunResult,
    timeout: int,
    retain_raw: bool,
) -> list[dict[str, Any]]:
    definitions = json.loads(JUDGES.read_text(encoding="utf-8"))
    judgments = []
    for judge in definitions["judges"]:
        prompt = (
            "Act as a narrow binary evaluator. Return JSON only with keys "
            '"critique", "verdict", and "evidence". Verdict must be Pass or Fail.\n\n'
            f"Criterion: {judge['criterion']}\n"
            f"Pass definition: {judge['pass']}\n"
            f"Fail definition: {judge['fail']}\n\n"
            f"Task: {case['prompt']}\n\n"
            "Candidate final messages:\n"
            f"{json.dumps([step.get('final_message', '') for step in candidate.steps])}"
            "\n\n"
            f"Candidate state:\n{json.dumps(candidate.state, sort_keys=True)}\n\n"
            f"Candidate artifacts:\n{json.dumps(candidate.artifacts, sort_keys=True)}"
        )
        with tempfile.TemporaryDirectory(prefix="neo-judge-") as temporary:
            workspace = Path(temporary)
            prepare_workspace(workspace, skill_enabled=False)
            command = command_for(judge_harness, workspace, prompt, False)
            code, duration, stdout, stderr, timed_out = run_process(
                command, workspace, timeout
            )
        judgment = {
            "judge_id": judge["id"],
            "status": definitions["status"],
            "harness": judge_harness,
            "command": command,
            "returncode": code,
            "duration_seconds": duration,
            "timed_out": timed_out,
            **normalize_stream(stdout, stderr),
        }
        if retain_raw:
            judgment["raw_streams"] = {"stdout": stdout, "stderr": stderr}
        judgments.append(judgment)
    return judgments


def run_case(
    harness: str,
    case: dict[str, Any],
    skill_enabled: bool,
    timeout: int,
    retain_raw: bool,
) -> RunResult:
    with tempfile.TemporaryDirectory(prefix="neo-eval-") as temporary:
        workspace = Path(temporary)
        prepare_workspace(workspace, skill_enabled)
        state = None
        route = None
        steps = []
        raw_streams = [] if retain_raw else None
        total_duration = 0.0
        if skill_enabled:
            route_command, route_result = bootstrap_neo(workspace, case, timeout)
            route_code, route_duration, route_stdout, route_stderr, route_timed_out = (
                route_result
            )
            total_duration += route_duration
            route_normalized = normalize_stream(route_stdout, route_stderr)
            steps.append(
                {
                    "stage": "route",
                    "kind": "local",
                    "command": route_command,
                    "returncode": route_code,
                    "duration_seconds": route_duration,
                    "timed_out": route_timed_out,
                    **route_normalized,
                }
            )
            if raw_streams is not None:
                raw_streams.append(
                    {"stage": "route", "stdout": route_stdout, "stderr": route_stderr}
                )
            state = read_state(workspace, case["id"])
            route = state.get("route") if state else None
            if route_code != 0 or route_timed_out:
                deterministic = deterministic_result(
                    route_code,
                    route_timed_out,
                    state,
                    case["expected_route"],
                    skill_enabled,
                )
                return RunResult(
                    harness=harness,
                    case_id=case["id"],
                    skill_enabled=skill_enabled,
                    returncode=route_code,
                    duration_seconds=total_duration,
                    timed_out=route_timed_out,
                    state=state,
                    artifacts={},
                    steps=steps,
                    deterministic=deterministic,
                    raw_streams=raw_streams,
                )

        prompt = initial_prompt(case, skill_enabled, route)
        command = command_for(harness, workspace, prompt, skill_enabled)
        code, duration, stdout, stderr, timed_out = run_process(
            command, workspace, timeout
        )
        total_duration += duration
        state = read_state(workspace, case["id"])
        first_stage = (
            "direct"
            if skill_enabled and route == "direct"
            else "discover"
            if skill_enabled
            else "baseline"
        )
        steps.append(
            {
                "stage": first_stage,
                "kind": "model",
                "command": command,
                "returncode": code,
                "duration_seconds": duration,
                "timed_out": timed_out,
                **normalize_stream(stdout, stderr),
            }
        )
        if raw_streams is not None:
            raw_streams.append(
                {"stage": first_stage, "stdout": stdout, "stderr": stderr}
            )
        artifacts = collect_artifacts(workspace, case["id"])
        deterministic = deterministic_result(
            code, timed_out, state, case["expected_route"], skill_enabled
        )
        return RunResult(
            harness=harness,
            case_id=case["id"],
            skill_enabled=skill_enabled,
            returncode=code,
            duration_seconds=total_duration,
            timed_out=timed_out,
            state=state,
            artifacts=artifacts,
            steps=steps,
            deterministic=deterministic,
            raw_streams=raw_streams,
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--harness", choices=(*HARNESSES, "all"), default="all")
    parser.add_argument("--suite", choices=("smoke", "full"), default="smoke")
    parser.add_argument("--mode", choices=("skill", "baseline", "both"), default="skill")
    parser.add_argument(
        "--variant",
        choices=("no-skill", "incumbent", "candidate", "all"),
        help="Use explicit release variants; overrides the legacy --mode flag.",
    )
    parser.add_argument(
        "--repetitions",
        type=int,
        help="Defaults to three for smoke/development and five for full/release.",
    )
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--judge-harness", choices=HARNESSES)
    parser.add_argument(
        "--jobs",
        type=bounded_jobs,
        default=3,
        help="Run up to three cases concurrently (default: 3).",
    )
    parser.add_argument(
        "--raw-streams",
        action="store_true",
        help="Retain raw harness streams; omitted by default to keep results compact.",
    )
    parser.add_argument(
        "--ack-full-cost",
        action="store_true",
        help="Acknowledge the model usage of the full paired suite.",
    )
    args = parser.parse_args(argv)
    if args.suite == "full" and not args.ack_full_cost:
        parser.error("the full suite requires --ack-full-cost")
    harnesses = HARNESSES if args.harness == "all" else (args.harness,)
    if args.variant:
        names = ("no-skill", "incumbent", "candidate") if args.variant == "all" else (args.variant,)
        modes = tuple((name, name != "no-skill") for name in names)
    else:
        modes = (
            (("candidate", True),)
            if args.mode == "skill"
            else (("no-skill", False),)
            if args.mode == "baseline"
            else (("candidate", True), ("no-skill", False))
        )
    repetitions = args.repetitions or (5 if args.suite == "full" else 3)
    jobs = [
        (case, harness, variant, skill_enabled, repetition)
        for case in load_cases(args.suite)
        for harness in harnesses
        for variant, skill_enabled in modes
        for repetition in range(1, repetitions + 1)
    ]

    def execute(job: tuple[dict[str, Any], str, str, bool, int]) -> tuple:
        case, harness, variant, skill_enabled, repetition = job
        result = run_case(
            harness,
            case,
            skill_enabled,
            args.timeout,
            retain_raw=args.raw_streams,
        )
        rendered_result = asdict(result)
        if rendered_result["raw_streams"] is None:
            rendered_result.pop("raw_streams")
        rendered_result["variant"] = variant
        rendered_result["repetition"] = repetition
        rendered_result["model"] = ROUTES["harnesses"][harness]["model"]
        rendered_result["effort"] = ROUTES["harnesses"][harness]["effort"]
        rendered_result["judgments"] = (
            run_judges(
                args.judge_harness,
                case,
                result,
                args.timeout,
                args.raw_streams,
            )
            if args.judge_harness
            else []
        )
        return case, result, rendered_result

    results = []
    with ThreadPoolExecutor(max_workers=args.jobs) as executor:
        for case, result, rendered_result in executor.map(execute, jobs):
            results.append(rendered_result)
            label = "skill" if result.skill_enabled else "baseline"
            outcome = "PASS" if result.deterministic["pass"] else "FAIL"
            print(
                f"{outcome} {result.harness} {case['id']} {label}",
                file=sys.stderr,
            )
    payload = {
        "judge_status": json.loads(JUDGES.read_text(encoding="utf-8"))["status"],
        "results": results,
    }
    rendered = json.dumps(payload, indent=2, sort_keys=True)
    if args.output:
        args.output.write_text(rendered + "\n", encoding="utf-8")
    else:
        print(rendered)
    return 0 if all(item["deterministic"]["pass"] for item in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
