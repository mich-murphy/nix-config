#!/usr/bin/env python3
"""Compare a Neo candidate with the recorded pre-optimization baseline."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent


def candidate_summary(payload: dict[str, Any]) -> dict[str, Any]:
    results = [item for item in payload["results"] if item["skill_enabled"]]
    if not results:
        raise ValueError("candidate contains no skill-enabled Neo results")
    completed = [
        item
        for item in results
        if item["returncode"] == 0 and not item["timed_out"]
    ]
    deterministic_passed = sum(
        bool(item["deterministic"]["pass"]) for item in results
    )
    model_invocations = sum(
        step.get("kind") == "model"
        for item in results
        for step in item.get("steps", [])
    )
    return {
        "cases": len(results),
        "deterministic_passed": deterministic_passed,
        "deterministic_pass_rate": deterministic_passed / len(results),
        "completed_mean_seconds": (
            sum(item["duration_seconds"] for item in completed) / len(completed)
            if completed
            else None
        ),
        "model_invocations": model_invocations,
        "model_invocations_per_case": model_invocations / len(results),
    }


def compare(baseline: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any]:
    baseline_summary = baseline["summary"]
    candidate_latency = candidate["completed_mean_seconds"]
    quality_maintained = (
        candidate["deterministic_pass_rate"]
        >= baseline_summary["deterministic_pass_rate"]
    )
    invocation_reduced = (
        candidate["model_invocations_per_case"]
        < baseline_summary["model_invocations_per_case"]
    )
    latency_reduced = (
        candidate_latency is not None
        and candidate_latency < baseline_summary["completed_mean_seconds"]
    )
    return {
        "pass": quality_maintained and invocation_reduced and latency_reduced,
        "quality_maintained": quality_maintained,
        "model_invocations_reduced": invocation_reduced,
        "latency_reduced": latency_reduced,
        "baseline": baseline_summary,
        "candidate": candidate,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument(
        "--baseline",
        type=Path,
        default=HERE / "results" / "baseline.json",
    )
    args = parser.parse_args()
    baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
    payload = json.loads(args.candidate.read_text(encoding="utf-8"))
    try:
        result = compare(baseline, candidate_summary(payload))
    except (KeyError, TypeError, ValueError) as error:
        parser.error(str(error))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
