#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["mlflow==3.13.0"]
# ///
"""Protect adopted or restricted evaluation evidence in MLflow."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone


def validate_run_binding(run: object, identity: str, content_hash: str) -> None:
    if run.data.tags.get("app.agent.eval.identity") != identity:
        raise RuntimeError("summary run identity does not match the release candidate")
    if run.data.tags.get("app.agent.eval.content_hash") != content_hash:
        raise RuntimeError("summary run content does not match the release candidate")
    if run.data.tags.get("app.agent.eval.summary_complete") != "true":
        raise RuntimeError("summary run is not complete")


def apply_protection(
    *,
    client: object,
    experiment: str,
    run_id: str,
    identity: str,
    content_hash: str,
    decision: str,
    supersedes: str | None,
) -> dict[str, str | None]:
    """Preflight every reference before applying fail-safe ordered tags."""
    run = client.get_run(run_id)
    validate_run_binding(run, identity, content_hash)
    skill = run.data.tags.get("app.agent.eval.skill")
    if not isinstance(skill, str) or not skill:
        raise RuntimeError("summary run has no release lineage")
    if (
        run.data.tags.get("app.agent.eval.superseded_at") not in {None, ""}
        or run.data.tags.get("app.agent.eval.superseded_by") not in {None, ""}
    ):
        raise RuntimeError("summary run is already superseded")
    superseded_run_id = None
    if supersedes:
        if supersedes == identity:
            raise RuntimeError("release cannot supersede itself")
        experiment_info = client.get_experiment_by_name(experiment)
        if experiment_info is None:
            raise RuntimeError(f"missing experiment: {experiment}")
        runs = client.search_runs(
            [experiment_info.experiment_id],
            filter_string=f"tags.`app.agent.eval.identity` = '{supersedes}'",
            max_results=2,
        )
        if len(runs) != 1:
            raise RuntimeError(
                f"superseded identity must resolve to exactly one summary run: {supersedes}"
            )
        superseded_run = runs[0]
        superseded_run_id = superseded_run.info.run_id
        tags = superseded_run.data.tags
        if superseded_run_id == run_id:
            raise RuntimeError("release cannot supersede itself")
        if (
            tags.get("app.agent.eval.identity") != supersedes
            or tags.get("app.agent.eval.summary_complete") != "true"
            or tags.get("app.agent.eval.skill") != skill
        ):
            raise RuntimeError("superseded run belongs to a different release lineage")
        superseded_by = tags.get("app.agent.eval.superseded_by")
        superseded_at = tags.get("app.agent.eval.superseded_at")
        active_predecessor = (
            superseded_by in {None, ""} and superseded_at in {None, ""}
        )
        retrying_same_transaction = superseded_by == identity
        if (
            tags.get("app.agent.eval.protection") != "protected"
            or tags.get("app.agent.eval.owner_decision") not in {"adopt", "restrict"}
            or tags.get("app.agent.eval.grace_days") != "365"
            or not (active_predecessor or retrying_same_transaction)
        ):
            raise RuntimeError("superseded run is not an active protected release")

    client.set_tag(run_id, "app.agent.eval.protection", "protected")
    client.set_tag(run_id, "app.agent.eval.grace_days", "365")
    if superseded_run_id:
        if superseded_by in {None, ""}:
            client.set_tag(
                superseded_run_id, "app.agent.eval.superseded_by", identity
            )
        if superseded_at in {None, ""}:
            client.set_tag(
                superseded_run_id,
                "app.agent.eval.superseded_at",
                datetime.now(timezone.utc).isoformat(),
            )
    client.set_tag(run_id, "app.agent.eval.owner_decision", decision)
    return {
        "run_id": run_id,
        "identity": identity,
        "decision": decision,
        "superseded_run_id": superseded_run_id,
    }


def protect(
    *,
    tracking_uri: str,
    experiment: str,
    run_id: str,
    identity: str,
    content_hash: str,
    decision: str,
    supersedes: str | None,
) -> dict[str, str | None]:
    import mlflow
    from mlflow import MlflowClient

    mlflow.set_tracking_uri(tracking_uri)
    client = MlflowClient()
    return apply_protection(
        client=client,
        experiment=experiment,
        run_id=run_id,
        identity=identity,
        content_hash=content_hash,
        decision=decision,
        supersedes=supersedes,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tracking-uri", required=True)
    parser.add_argument("--experiment", default="skill-evaluations")
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--identity", required=True)
    parser.add_argument("--content-hash", required=True)
    parser.add_argument("--decision", choices=("adopt", "restrict"), required=True)
    parser.add_argument("--supersedes")
    args = parser.parse_args()
    import json

    print(json.dumps(protect(
        tracking_uri=args.tracking_uri,
        experiment=args.experiment,
        run_id=args.run_id,
        identity=args.identity,
        content_hash=args.content_hash,
        decision=args.decision,
        supersedes=args.supersedes,
    ), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
