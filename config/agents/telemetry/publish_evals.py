#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["mlflow==3.13.0"]
# ///
"""Publish joined evaluation traces, assessments, and a summary run to MLflow."""

from __future__ import annotations

import argparse
import json
import os
import time
from pathlib import Path
from typing import Any


def eligible_results(document: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        result for result in document.get("results", [])
        if result.get("mlflow_trace_id") and result.get("telemetry", {}).get("status") == "exported"
    ]


def dataset_records(document: dict[str, Any]) -> list[dict[str, Any]]:
    records = []
    for result in eligible_results(document):
        records.append({
            "inputs": {
                "case_id": result["id"],
                "variant": result.get("variant", "not_observed"),
                "harness": result.get("harness", "not_observed"),
                "mode": result.get("mode", "end-to-end"),
                "repetition": result.get("repetition", 1),
                "skill_hash": result.get("skill_hash", "not_observed"),
            },
            "expectations": {
                "accepted": bool(result.get("accepted")),
                "score": float(result.get("score", result.get("assertions_passed", 0))),
                "verifier_status": result.get("outcome", "accepted" if result.get("accepted") else "failed"),
            },
            "source": {
                "source_type": "TRACE",
                "source_data": {"trace_id": result["mlflow_trace_id"]},
            },
        })
    return records


def publish(document: dict[str, Any], *, tracking_uri: str, experiment: str, dataset_name: str) -> dict[str, Any]:
    import mlflow
    from mlflow.entities import AssessmentSource, AssessmentSourceType

    mlflow.set_tracking_uri(tracking_uri)
    experiment_info = mlflow.set_experiment(experiment)
    pending = {result["mlflow_trace_id"] for result in eligible_results(document)}
    for _ in range(60):
        pending = {trace_id for trace_id in pending if mlflow.get_trace(trace_id, silent=True) is None}
        if not pending:
            break
        time.sleep(0.5)
    if pending:
        raise RuntimeError(f"exported traces were not queryable in MLflow: {sorted(pending)}")
    existing = mlflow.genai.datasets.search_datasets(
        experiment_ids=[experiment_info.experiment_id], max_results=100,
    )
    dataset = next((item for item in existing if item.name == dataset_name), None)
    if dataset is None:
        dataset = mlflow.genai.datasets.create_dataset(
            name=dataset_name, experiment_id=[experiment_info.experiment_id],
        )
    records = dataset_records(document)
    if records:
        dataset.merge_records(records)

    code_source = AssessmentSource(
        source_type=AssessmentSourceType.CODE,
        source_id="repository-evaluator",
    )
    human_source = AssessmentSource(
        source_type=AssessmentSourceType.HUMAN,
        source_id="repository-case-author",
    )
    assessed = 0
    for result in eligible_results(document):
        trace_id = result["mlflow_trace_id"]
        existing = {assessment.name for assessment in mlflow.get_trace(trace_id).info.assessments}
        if "verified_task_outcome" not in existing:
            mlflow.log_feedback(
                trace_id=trace_id,
                name="verified_task_outcome",
                value=bool(result.get("accepted")),
                rationale=(
                    f"state={result.get('state')}; passed={result.get('assertions_passed', 0)}/"
                    f"{result.get('assertions_total', 0)}; blockers={result.get('blocking_failures', [])}"
                ),
                source=code_source,
            )
        if "case_id" not in existing:
            mlflow.log_expectation(
                trace_id=trace_id,
                name="case_id",
                value=result["id"],
                source=human_source,
            )
        assessed += 1

    valid = [item for item in document.get("results", []) if item.get("valid")]
    with mlflow.start_run(run_name=f"{dataset_name}-publication") as run:
        mlflow.set_tags({
            "app.agent.eval.dataset": dataset_name,
            "app.agent.eval.schema_version": document.get("schema_version", "not_observed"),
            "app.agent.eval.trace_count": str(len(records)),
        })
        mlflow.log_metrics({
            "runs": len(document.get("results", [])),
            "valid_runs": len(valid),
            "accepted": sum(bool(item.get("accepted")) for item in valid),
            "telemetry_joined": len(records),
        })
        mlflow.log_dict(document, "evaluation-results.json")
        run_id = run.info.run_id
    return {"dataset": dataset_name, "records": len(records), "assessments": assessed, "run_id": run_id}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("--tracking-uri", default=os.environ.get("MLFLOW_TRACKING_URI", "http://mlflow:5000"))
    parser.add_argument("--experiment", default="skill-evaluations")
    parser.add_argument("--dataset")
    args = parser.parse_args()
    document = json.loads(args.results.read_text())
    configured_skill = document.get("configuration", {}).get("skill", args.results.stem)
    dataset_name = args.dataset or f"{configured_skill}-evaluation-cases"
    print(json.dumps(publish(
        document, tracking_uri=args.tracking_uri,
        experiment=args.experiment, dataset_name=dataset_name,
    ), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
