#!/usr/bin/env python3
"""Run this skill's deterministic eval cases through Codex."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import time
from pathlib import Path
from typing import Any


EVAL_DIR = Path(__file__).resolve().parent
SKILL_DIR = EVAL_DIR.parent
SKILL_NAME = SKILL_DIR.name
USAGE_KEYS = (
    "input_tokens",
    "cached_input_tokens",
    "cache_write_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
)


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
        passed = any(value.casefold() in output.casefold() for value in values)
        return passed, f"contains any of {values!r}"
    if kind == "not_contains":
        expected = assertion["value"]
        return expected.casefold() not in output.casefold(), f"omits {expected!r}"
    if kind == "regex":
        pattern = assertion["pattern"]
        passed = re.search(pattern, output, re.MULTILINE | re.IGNORECASE) is not None
        return passed, f"matches /{pattern}/"
    if kind == "not_regex":
        pattern = assertion["pattern"]
        passed = re.search(pattern, output, re.MULTILINE | re.IGNORECASE) is None
        return passed, f"does not match /{pattern}/"
    if kind == "max_words":
        limit = assertion["value"]
        count = len(re.findall(r"\b[\w'-]+\b", output))
        return count <= limit, f"word count {count} <= {limit}"
    if kind == "json_equals":
        actual = nested_value(json.loads(output), assertion["path"])
        expected = assertion["value"]
        return actual == expected, f"{assertion['path']} == {expected!r} (got {actual!r})"
    if kind == "json_truthy":
        actual = nested_value(json.loads(output), assertion["path"])
        return bool(actual), f"{assertion['path']} is truthy (got {actual!r})"
    raise ValueError(f"unknown assertion type: {kind}")


def run_case(
    case: dict[str, Any], skill_text: str, model: str, effort: str, timeout: int
) -> dict[str, Any]:
    prompt = f"""Use these skill instructions to answer the task.

<skill>
{skill_text}
</skill>

<task>
{case['prompt']}
</task>

Do not call tools or modify files. Return only the requested answer.
"""
    command = [
        "codex",
        "exec",
        "--ephemeral",
        "--ignore-user-config",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--json",
        "-m",
        model,
        "-c",
        f'model_reasoning_effort="{effort}"',
        prompt,
    ]
    started = time.monotonic()
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    duration = time.monotonic() - started
    events = [
        json.loads(line) for line in completed.stdout.splitlines() if line.startswith("{")
    ]
    messages = [
        event["item"]["text"]
        for event in events
        if event.get("type") == "item.completed"
        and event.get("item", {}).get("type") == "agent_message"
    ]
    usage_events = [
        event["usage"] for event in events if event.get("type") == "turn.completed"
    ]
    output = messages[-1] if messages else ""
    assertion_results = []
    for assertion in case["assertions"]:
        try:
            passed, detail = score_assertion(output, assertion)
        except (json.JSONDecodeError, KeyError, IndexError, TypeError) as error:
            passed, detail = False, f"assertion error: {error}"
        assertion_results.append({"passed": passed, "detail": detail})
    return {
        "id": case["id"],
        "skill": SKILL_NAME,
        "model": model,
        "effort": effort,
        "accepted": completed.returncode == 0
        and all(item["passed"] for item in assertion_results),
        "assertions_passed": sum(item["passed"] for item in assertion_results),
        "assertions_total": len(assertion_results),
        "assertions": assertion_results,
        "output": output,
        "usage": usage_events[-1] if usage_events else {},
        "duration_seconds": round(duration, 3),
        "returncode": completed.returncode,
        "stderr": completed.stderr,
        "events": events,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description=f"Run the packaged {SKILL_NAME} eval suite."
    )
    parser.add_argument(
        "--route-set", choices=("baseline", "candidate"), default="candidate"
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=180)
    args = parser.parse_args()

    cases = json.loads((EVAL_DIR / "cases.json").read_text())
    routes = json.loads((EVAL_DIR / "routes.json").read_text())[args.route_set]
    skill_text = (SKILL_DIR / "SKILL.md").read_text()
    results = []
    for case in cases:
        route = routes.get(case["id"], routes.get("default"))
        if route is None:
            raise KeyError(
                f"no {args.route_set} route for case {case['id']} or default route"
            )
        print(
            f"running {case['id']} ({route['model']}/{route['effort']})", flush=True
        )
        results.append(
            run_case(case, skill_text, route["model"], route["effort"], args.timeout)
        )

    usage = {
        key: sum(result["usage"].get(key, 0) for result in results)
        for key in USAGE_KEYS
    }
    skill_summary = {
        "cases": len(results),
        "accepted": sum(result["accepted"] for result in results),
        "assertions_passed": sum(result["assertions_passed"] for result in results),
        "assertions_total": sum(result["assertions_total"] for result in results),
        "usage": usage,
    }
    summary = {
        **skill_summary,
        "duration_seconds": round(
            sum(result["duration_seconds"] for result in results), 3
        ),
        "by_skill": {SKILL_NAME: skill_summary},
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"summary": summary, "results": results}, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
