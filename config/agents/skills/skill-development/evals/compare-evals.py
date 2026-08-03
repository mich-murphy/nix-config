#!/usr/bin/env python3
"""Compare candidate skill-development behavior with built-in creators."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


EVAL_DIR = Path(__file__).resolve().parent
ROUTES = json.loads((EVAL_DIR / "routes.json").read_text(encoding="utf-8"))


def mean(values: list[float]) -> float | None:
    return sum(values) / len(values) if values else None


def compare(document: dict[str, Any]) -> dict[str, Any]:
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for result in document["results"]:
        if result.get("valid"):
            grouped[(result["harness"], result["variant"])].append(result)
    by_harness: dict[str, Any] = {}
    gate = ROUTES["comparison_gate"]
    checks: dict[str, bool] = {}
    for harness in document["configuration"]["harnesses"]:
        incumbent = grouped[(harness, "incumbent")]
        candidate = grouped[(harness, "candidate")]
        incumbent_score = mean([item["score"] for item in incumbent])
        candidate_score = mean([item["score"] for item in candidate])
        delta = candidate_score - incumbent_score if candidate_score is not None and incumbent_score is not None else None
        candidate_blockers = sum(len(item.get("blocking_failures", [])) for item in candidate)
        harness_pass = bool(
            incumbent and candidate
            and candidate_score is not None
            and candidate_score >= gate["minimum_candidate_score"]
            and delta is not None
            and delta >= gate["minimum_score_gain_over_incumbent"]
            and candidate_blockers <= gate["blocking_regressions_allowed"]
        )
        checks[f"{harness}_outperforms_builtin"] = harness_pass
        by_harness[harness] = {
            "incumbent_mean_score": incumbent_score,
            "candidate_mean_score": candidate_score,
            "candidate_delta": delta,
            "incumbent_accepted": sum(item["accepted"] for item in incumbent),
            "candidate_accepted": sum(item["accepted"] for item in candidate),
            "valid_runs_each": {"incumbent": len(incumbent), "candidate": len(candidate)},
            "candidate_blocking_failures": candidate_blockers,
            "pass": harness_pass,
        }
    invalid = [
        {key: item.get(key) for key in ("id", "harness", "variant", "repetition", "state", "failure_kind")}
        for item in document["results"] if not item.get("valid")
    ]
    development_pass = bool(checks) and all(checks.values()) and not invalid
    config = document["configuration"]
    held_out_release_evidence = config.get("suite") in {"held-out", "full"} and config.get("repetitions", 0) >= 5
    return {
        "claim": "candidate outperforms each harness's installed built-in skill creator on this artifact and container-choice evaluation suite",
        "decision": "development-pass" if development_pass else "defer",
        "release_eligible": development_pass and held_out_release_evidence,
        "checks": checks,
        "by_harness": by_harness,
        "invalid_runs": invalid,
        "limitations": [
            "The comparison supports only the tested cases, harness versions, models, effort, tools, and acceptance grader.",
            "Artifact criteria reflect this repository's evidence, telemetry, model-routing, safety, and release requirements; they are not a universal skill-quality score.",
            "A development pass is not a stable release. Held-out repetitions, automatic routing, production activation traces, and owner review remain separate gates.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()
    result = compare(json.loads(args.candidate.read_text(encoding="utf-8")))
    rendered = json.dumps(result, indent=2) + "\n"
    if args.json_output:
        args.json_output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if result["decision"] == "development-pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
