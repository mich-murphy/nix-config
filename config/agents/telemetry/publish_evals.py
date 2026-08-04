#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["mlflow==3.13.0"]
# ///
"""Publish joined evaluation traces, assessments, and a summary run to MLflow."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
import time
import urllib.request
from pathlib import Path
from typing import Any

import sys


AGENT_ROOT = Path(__file__).resolve().parents[1]
if str(AGENT_ROOT) not in sys.path:
    sys.path.insert(0, str(AGENT_ROOT))

from eval_publication import (  # noqa: E402
    logical_objects,
    publication_content_hash,
    safe_publication_document,
    trace_result_hash,
)


def write_release_candidate(path: Path, document: dict[str, Any]) -> None:
    if not document.get("results") or any(
        result.get("telemetry", {}).get("status") != "exported"
        for result in document["results"]
    ):
        raise RuntimeError("release candidate evidence is not completely published")
    candidate = safe_publication_document(document)
    candidate["evidence_state"] = "published"
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(prefix=".eval-candidate-", dir=path.parent)
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            json.dump(candidate, stream, indent=2)
            stream.write("\n")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def write_summary_payload(
    mlflow: Any, run_id: str, safe_document: dict[str, Any], record_count: int
) -> None:
    valid = [item for item in safe_document.get("results", []) if item.get("valid")]
    with mlflow.start_run(run_id=run_id):
        mlflow.log_metrics({
            "runs": len(safe_document.get("results", [])),
            "valid_runs": len(valid),
            "accepted": sum(bool(item.get("accepted")) for item in valid),
            "telemetry_joined": record_count,
        })
        mlflow.log_dict(safe_document, "evaluation-summary.json")
        mlflow.set_tag("app.agent.eval.summary_complete", "true")


def validate_existing_summary(
    runs: list[Any], identity: str, definition_hash: str, content_hash: str
) -> tuple[str | None, bool]:
    if len(runs) > 1:
        raise RuntimeError(f"conflicting summary runs for evaluation identity {identity}")
    if not runs:
        return None, False
    run = runs[0]
    observed_definition = run.data.tags.get("app.agent.eval.definition_hash")
    if observed_definition != definition_hash:
        raise RuntimeError(f"definition hash conflict for evaluation identity {identity}")
    observed_content = run.data.tags.get("app.agent.eval.content_hash")
    if observed_content != content_hash:
        raise RuntimeError(f"content hash conflict for evaluation identity {identity}")
    return (
        run.info.run_id,
        run.data.tags.get("app.agent.eval.summary_complete") == "true",
    )


def validate_existing_assessments(
    assessments: list[Any], identity: str, content_hash: str
) -> set[str]:
    names = set()
    for assessment in assessments:
        if assessment.name not in {"verified_task_outcome", "case_id"}:
            continue
        names.add(assessment.name)
        metadata = assessment.metadata or {}
        if metadata.get("app.agent.eval.identity") != identity:
            raise RuntimeError(
                f"assessment identity conflict for evaluation identity {identity}"
            )
        if metadata.get("app.agent.eval.content_hash") != content_hash:
            raise RuntimeError(
                f"assessment content hash conflict for evaluation identity {identity}"
            )
    return names


def validate_trace_binding(trace: Any, result: dict[str, Any], identity: str) -> None:
    roots = [span for span in trace.data.spans if span.parent_id is None]
    if len(roots) != 1:
        raise RuntimeError(f"trace {result.get('mlflow_trace_id')} lacks one task root")
    attributes = roots[0].attributes
    if attributes.get("app.agent.eval.identity") != identity:
        raise RuntimeError("trace identity does not match the evaluation identity")
    if attributes.get("app.agent.eval.result_hash") != trace_result_hash(result):
        raise RuntimeError("trace result content does not match the candidate result")


def eligible_results(document: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        result for result in document.get("results", [])
        if result.get("mlflow_trace_id") and result.get("telemetry", {}).get("status") == "exported"
    ]


def dataset_records(document: dict[str, Any]) -> list[dict[str, Any]]:
    records = []
    seen: set[str] = set()
    identity_key = document.get("evaluation_identity", {}).get("key", "legacy")
    safe_results = {
        result.get("mlflow_trace_id"): result
        for result in safe_publication_document(document).get("results", [])
    }
    for result in eligible_results(document):
        result = safe_results.get(result["mlflow_trace_id"], {})
        record_key = f"{identity_key}:{result['mlflow_trace_id']}"
        if record_key in seen:
            continue
        seen.add(record_key)
        records.append({
            "inputs": {
                "evaluation_record_key": record_key,
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


def retry_pending_traces(document: dict[str, Any], endpoint: str) -> list[str]:
    retried = []
    by_trace = {
        item.get("mlflow_trace_id"): item
        for item in document.get("results", [])
        if isinstance(item, dict)
    }
    for retry in document.get("retry_traces", []):
        request = urllib.request.Request(
            endpoint,
            data=json.dumps(retry["payload"], separators=(",", ":")).encode("utf-8"),
            headers={"content-type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            if response.status >= 300:
                raise RuntimeError(
                    f"OTLP retry returned {response.status} for {retry['mlflow_trace_id']}"
                )
        trace_id = retry["mlflow_trace_id"]
        retried.append(trace_id)
        result = by_trace.get(trace_id)
        if result is not None:
            result["telemetry"] = {"status": "exported"}
            result["evidence_state"] = "published"
    return retried


def publish(document: dict[str, Any], *, tracking_uri: str, experiment: str, dataset_name: str, otlp_endpoint: str | None = None) -> dict[str, Any]:
    import mlflow
    from mlflow.entities import AssessmentSource, AssessmentSourceType

    mlflow.set_tracking_uri(tracking_uri)
    experiment_info = mlflow.set_experiment(experiment)
    safe_document = safe_publication_document(document)
    identity = safe_document["evaluation_identity"]["key"]
    definition_hash = hashlib.sha256(json.dumps(
        safe_document["evaluation_identity"].get("definition_hashes", {}),
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")).hexdigest()
    content_hash = publication_content_hash(document)
    existing_runs = mlflow.search_runs(
        experiment_ids=[experiment_info.experiment_id],
        filter_string=f"tags.`app.agent.eval.identity` = '{identity}'",
        max_results=2,
        output_format="list",
    )
    existing_run_id, summary_complete = validate_existing_summary(
        existing_runs, identity, definition_hash, content_hash
    )
    retried = retry_pending_traces(
        document,
        otlp_endpoint or os.environ.get(
            "APP_AGENT_EVAL_OTLP_ENDPOINT", "http://docker-host:4318/v1/traces"
        ),
    ) if document.get("retry_traces") else []
    incomplete = [
        result.get("id", "unknown")
        for result in document.get("results", [])
        if result.get("telemetry", {}).get("status") != "exported"
    ]
    if incomplete:
        raise RuntimeError(
            f"evaluation evidence remains pending after retry: {sorted(incomplete)}"
        )
    document["evidence_state"] = "published"
    document.pop("retry_traces", None)
    safe_document = safe_publication_document(document)
    expected = logical_objects(document)
    pending = {result["mlflow_trace_id"] for result in eligible_results(document)}
    for _ in range(60):
        pending = {trace_id for trace_id in pending if mlflow.get_trace(trace_id, silent=True) is None}
        if not pending:
            break
        time.sleep(0.5)
    if pending:
        raise RuntimeError(f"exported traces were not queryable in MLflow: {sorted(pending)}")
    code_source = AssessmentSource(
        source_type=AssessmentSourceType.CODE,
        source_id="repository-evaluator",
    )
    human_source = AssessmentSource(
        source_type=AssessmentSourceType.HUMAN,
        source_id="repository-case-author",
    )
    assessments_by_trace: dict[str, set[str]] = {}
    for result in eligible_results(document):
        trace_id = result["mlflow_trace_id"]
        trace = mlflow.get_trace(trace_id)
        validate_trace_binding(trace, result, identity)
        assessments_by_trace[trace_id] = validate_existing_assessments(
            trace.info.assessments, identity, content_hash
        )

    assessed = 0
    safe_results = {
        result.get("mlflow_trace_id"): result
        for result in safe_document.get("results", [])
    }
    for result in eligible_results(document):
        trace_id = result["mlflow_trace_id"]
        safe_result = safe_results[trace_id]
        failure_kind = safe_result.get("failure_kind")
        if failure_kind not in {
            None, "authentication", "harness_not_found", "timeout", "network",
            "model_or_task", "incumbent_missing", "harness_failure",
        }:
            failure_kind = "other"
        existing = assessments_by_trace[trace_id]
        assessment_metadata = {
            "app.agent.eval.identity": identity,
            "app.agent.eval.content_hash": content_hash,
        }
        if "verified_task_outcome" not in existing:
            mlflow.log_feedback(
                trace_id=trace_id,
                name="verified_task_outcome",
                value=bool(safe_result.get("accepted")),
                rationale=(
                    f"state={safe_result.get('state')}; passed={safe_result.get('assertions_passed', 0)}/"
                    f"{safe_result.get('assertions_total', 0)}; failure_kind={failure_kind}"
                ),
                source=code_source,
                metadata=assessment_metadata,
            )
        if "case_id" not in existing:
            mlflow.log_expectation(
                trace_id=trace_id,
                name="case_id",
                value=safe_result["id"],
                source=human_source,
                metadata=assessment_metadata,
            )
        assessed += 1

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

    if not existing_run_id:
        with mlflow.start_run(run_name=f"evaluation-{identity[:12]}") as run:
            mlflow.set_tags({
                "app.agent.eval.identity": identity,
                "app.agent.eval.skill": safe_document.get("configuration", {}).get(
                    "skill", "not_observed"
                ),
                "app.agent.eval.definition_hash": definition_hash,
                "app.agent.eval.content_hash": content_hash,
                "app.agent.eval.protection": "ordinary",
                "app.agent.eval.owner_decision": "defer",
                "app.agent.eval.superseded_at": "",
                "app.agent.eval.superseded_by": "",
                "app.agent.eval.grace_days": "365",
                "app.agent.eval.dataset": dataset_name,
                "app.agent.eval.schema_version": document.get("schema_version", "not_observed"),
                "app.agent.eval.trace_count": str(len(records)),
                "app.agent.eval.summary_complete": "false",
            })
            run_id = run.info.run_id
    else:
        run_id = existing_run_id
    if not summary_complete:
        write_summary_payload(mlflow, run_id, safe_document, len(records))
    return {
        "evaluation_identity": identity,
        "content_hash": content_hash,
        "evidence_state": "published",
        "tracking_uri": tracking_uri,
        "experiment": experiment,
        "dataset": dataset_name,
        "records": len(records),
        "assessments": assessed,
        "run_id": run_id,
        "expected": expected,
        "retried_traces": retried,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    parser.add_argument("--tracking-uri", default=os.environ.get("MLFLOW_TRACKING_URI", "http://mlflow:5000"))
    parser.add_argument("--experiment", default="skill-evaluations")
    parser.add_argument("--dataset")
    parser.add_argument("--otlp-endpoint")
    parser.add_argument(
        "--update-candidate",
        action="store_true",
        help="replace the retry spool with the safe release-ready candidate",
    )
    args = parser.parse_args()
    document = json.loads(args.results.read_text())
    configured_skill = document.get("configuration", {}).get("skill", args.results.stem)
    dataset_name = args.dataset or f"{configured_skill}-evaluation-cases"
    publication = publish(
        document, tracking_uri=args.tracking_uri,
        experiment=args.experiment, dataset_name=dataset_name,
        otlp_endpoint=args.otlp_endpoint,
    )
    if args.update_candidate:
        write_release_candidate(args.results, document)
        publication["candidate"] = str(args.results)
    print(json.dumps(publication, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
