#!/usr/bin/env python3
"""Compare a refactor-skill candidate with the recorded no-skill baseline."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


EVAL_DIR = Path(__file__).resolve().parent


def context_tokens(summary: dict) -> int:
    usage = summary["usage"]
    return (
        usage.get("input_tokens", 0)
        + usage.get("output_tokens", 0)
        + usage.get("reasoning_output_tokens", 0)
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    parser.add_argument(
        "--baseline",
        type=Path,
        default=EVAL_DIR / "results" / "baseline.json",
    )
    args = parser.parse_args()

    baseline = json.loads(args.baseline.read_text())["summary"]
    candidate = json.loads(args.candidate.read_text())["summary"]
    baseline_context = context_tokens(baseline)
    candidate_context = context_tokens(candidate)
    accepted_delta = candidate["accepted"] - baseline["accepted"]
    assertion_delta = candidate["assertions_passed"] - baseline["assertions_passed"]
    complete_candidate = candidate["accepted"] == candidate["cases"]
    no_quality_regression = accepted_delta >= 0 and assertion_delta >= 0
    passing = complete_candidate and no_quality_regression
    result = {
        "pass": passing,
        "quality_uplift": accepted_delta > 0 or assertion_delta > 0,
        "baseline_accepted": f"{baseline['accepted']}/{baseline['cases']}",
        "candidate_accepted": f"{candidate['accepted']}/{candidate['cases']}",
        "accepted_delta": accepted_delta,
        "baseline_assertions": (
            f"{baseline['assertions_passed']}/{baseline['assertions_total']}"
        ),
        "candidate_assertions": (
            f"{candidate['assertions_passed']}/{candidate['assertions_total']}"
        ),
        "assertion_delta": assertion_delta,
        "baseline_context_tokens": baseline_context,
        "candidate_context_tokens": candidate_context,
        "context_change_percent": round(
            100 * (candidate_context - baseline_context) / baseline_context, 2
        )
        if baseline_context
        else None,
    }
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if passing else 1)


if __name__ == "__main__":
    main()
