#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["mlflow==3.13.0"]
# ///
"""Attach a content-free delayed outcome assessment to an MLflow trace."""

from __future__ import annotations

import argparse
import hashlib
import json
import os

import mlflow
from mlflow.entities import AssessmentSource, AssessmentSourceType


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("record", help="validated annotation JSON")
    parser.add_argument("--tracking-uri", default=os.environ.get("MLFLOW_TRACKING_URI", "http://mlflow:5000"))
    args = parser.parse_args()
    record = json.loads(args.record)
    trace_id = record.get("mlflow_trace_id")
    if not isinstance(trace_id, str) or not trace_id.startswith("tr-"):
        raise SystemExit("annotation requires an MLflow trace ID")
    mlflow.set_tracking_uri(args.tracking_uri)
    source_type = (
        AssessmentSourceType.CODE
        if record["kind"] in {"ci", "merge", "revert"}
        else AssessmentSourceType.HUMAN
    )
    source_id = "sha256:" + hashlib.sha256(str(record["owner"]).encode()).hexdigest()
    assessment = mlflow.log_feedback(
        trace_id=trace_id,
        name=f"delayed_{record['kind']}_outcome",
        value=record["status"],
        rationale=f"rubric={record['rubric_version']}; annotation={record['annotation_id']}",
        source=AssessmentSource(source_type=source_type, source_id=source_id),
        metadata={
            "task_id": record["task_id"],
            "recorded_at": record["recorded_at"],
            "outcome_reference": str(record.get("outcome_reference") or "not_observed"),
            "supersedes": str(record.get("supersedes") or "none"),
        },
    )
    print(json.dumps({"trace_id": trace_id, "assessment_id": assessment.assessment_id}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
