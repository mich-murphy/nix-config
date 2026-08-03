#!/usr/bin/env python3
"""Validate human-labelled Neo judge predictions as binary classifiers."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


JUDGE_IDS = {
    "clarification-discipline",
    "decision-clarity",
    "implementation-readiness",
}


def calculate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        if row.get("judge_id") not in JUDGE_IDS:
            raise ValueError(f"unknown judge_id: {row.get('judge_id')}")
        if row.get("human") not in {"Pass", "Fail"}:
            raise ValueError("human labels must be Pass or Fail")
        if row.get("predicted") not in {"Pass", "Fail"}:
            raise ValueError("predicted labels must be Pass or Fail")
        grouped[row["judge_id"]].append(row)

    report = {}
    for judge_id in sorted(JUDGE_IDS):
        items = grouped[judge_id]
        human_pass = [item for item in items if item["human"] == "Pass"]
        human_fail = [item for item in items if item["human"] == "Fail"]
        true_positive = sum(item["predicted"] == "Pass" for item in human_pass)
        true_negative = sum(item["predicted"] == "Fail" for item in human_fail)
        tpr = true_positive / len(human_pass) if human_pass else None
        tnr = true_negative / len(human_fail) if human_fail else None
        calibrated = (
            len(human_pass) >= 20
            and len(human_fail) >= 20
            and tpr is not None
            and tnr is not None
            and tpr >= 0.8
            and tnr >= 0.8
        )
        report[judge_id] = {
            "human_pass": len(human_pass),
            "human_fail": len(human_fail),
            "true_positive_rate": tpr,
            "true_negative_rate": tnr,
            "minimum_usable": calibrated,
            "target_met": calibrated and tpr >= 0.9 and tnr >= 0.9,
        }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("labels", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    rows = json.loads(args.labels.read_text(encoding="utf-8"))
    if not isinstance(rows, list):
        parser.error("labels file must contain a JSON list")
    try:
        report = calculate(rows)
    except ValueError as error:
        parser.error(str(error))
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
