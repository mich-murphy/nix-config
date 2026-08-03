#!/usr/bin/env python3
"""Validate and append a content-free outcome annotation to JSONL."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


REQUIRED = {"annotation_id", "task_id", "recorded_at", "kind", "status", "owner", "rubric_version"}
KINDS = {"trace_review", "ci", "review", "merge", "revert", "incident", "owner", "user"}
STATUSES = {"pass", "fail", "defer", "not_observed", "invalid"}


def validate(record: dict) -> None:
    missing = REQUIRED - record.keys()
    if missing:
        raise ValueError(f"missing fields: {sorted(missing)}")
    if record["kind"] not in KINDS or record["status"] not in STATUSES:
        raise ValueError("unknown annotation kind or status")
    if record.get("mlflow_trace_id") is not None and not re.fullmatch(r"tr-[0-9a-f]{32}", record["mlflow_trace_id"]):
        raise ValueError("mlflow_trace_id must be a canonical MLflow trace ID")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("journal", type=Path)
    parser.add_argument("record", help="JSON object")
    parser.add_argument("--publish-mlflow", action="store_true")
    args = parser.parse_args()
    record = json.loads(args.record)
    validate(record)
    existing_ids = set()
    if args.journal.exists():
        existing_ids = {
            json.loads(line)["annotation_id"]
            for line in args.journal.read_text().splitlines()
            if line.strip()
        }
    if record["annotation_id"] in existing_ids:
        raise SystemExit("annotation_id already exists; append a superseding record")
    if record.get("supersedes") and record["supersedes"] not in existing_ids:
        raise SystemExit("supersedes must reference an existing annotation")
    args.journal.parent.mkdir(parents=True, exist_ok=True)
    with args.journal.open("a") as journal:
        journal.write(json.dumps(record, sort_keys=True) + "\n")
    if args.publish_mlflow:
        if not record.get("mlflow_trace_id"):
            raise SystemExit("--publish-mlflow requires mlflow_trace_id")
        subprocess.run([
            "uv", "run", str(Path(__file__).parents[1] / "publish_annotation.py"),
            json.dumps(record, separators=(",", ":")),
        ], check=True)


if __name__ == "__main__":
    main()
