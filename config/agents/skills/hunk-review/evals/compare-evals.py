#!/usr/bin/env python3
"""Compare this skill's candidate eval with its recorded baseline."""

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
    quality_maintained = (
        candidate["accepted"] >= baseline["accepted"]
        and candidate["assertions_passed"] >= baseline["assertions_passed"]
    )
    context_reduced = candidate_context < baseline_context
    passing = quality_maintained and context_reduced
    result = {
        "pass": passing,
        "baseline_accepted": f"{baseline['accepted']}/{baseline['cases']}",
        "candidate_accepted": f"{candidate['accepted']}/{candidate['cases']}",
        "baseline_context_tokens": baseline_context,
        "candidate_context_tokens": candidate_context,
        "context_change_percent": round(
            100 * (candidate_context - baseline_context) / baseline_context, 2
        ),
    }
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if passing else 1)


if __name__ == "__main__":
    main()
