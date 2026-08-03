"""Build and export metadata-first OpenTelemetry task traces."""

from __future__ import annotations

import hashlib
import json
import os
import secrets
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "1.2.0"


def evaluation_attributes(
    *,
    case_id: str,
    variant: str,
    repetition: int,
    mode: str,
    skill_name: str,
    skill_hash: str,
    skill_source: str,
    repository_hash: str,
    base_revision: str,
    model_requested: str,
    model_returned: str,
    effort: str,
    prompt_version: str,
    tool_version: str,
    evaluator_version: str,
    risk: str,
    outcome: str,
    verifier: str,
) -> dict[str, str | int | float | bool]:
    """Return the immutable join and reproduction fields for an eval task."""
    return {
        "app.agent.trace.kind": "evaluation",
        "app.agent.eval.case_id": case_id,
        "app.agent.eval.variant": variant,
        "app.agent.eval.repetition": repetition,
        "app.agent.eval.mode": mode,
        "app.agent.skill.name": skill_name,
        "app.agent.skill.package_hash": skill_hash,
        "app.agent.skill.source": skill_source,
        "app.agent.repository.hash": repository_hash,
        "app.agent.repository.base_revision": base_revision,
        "app.agent.model.requested": model_requested,
        "app.agent.model.returned": model_returned,
        "app.agent.model.effort": effort,
        "app.agent.prompt.version": prompt_version,
        "app.agent.tool.version": tool_version,
        "app.agent.evaluator.version": evaluator_version,
        "app.agent.risk.class": risk,
        "app.agent.outcome.status": outcome,
        "app.agent.outcome.verifier": verifier,
        "app.agent.cost.status": "not_observed",
        "app.agent.final.status": outcome,
        "app.agent.content.capture": "metadata",
    }


def _otel_value(value: str | int | float | bool) -> dict[str, Any]:
    if isinstance(value, bool):
        return {"boolValue": value}
    if isinstance(value, int):
        return {"intValue": str(value)}
    if isinstance(value, float):
        return {"doubleValue": value}
    return {"stringValue": value}


def _attributes(values: dict[str, str | int | float | bool]) -> list[dict[str, Any]]:
    return [
        {"key": key, "value": _otel_value(value)}
        for key, value in sorted(values.items())
    ]


def sha256_text(value: str) -> str:
    return hashlib.sha256(value.encode()).hexdigest()


def sha256_path(root: Path) -> str:
    digest = hashlib.sha256()
    if not root.exists():
        return "not_observed"
    files = [root] if root.is_file() else sorted(path for path in root.rglob("*") if path.is_file())
    for path in files:
        if {"__pycache__", ".pytest_cache"}.intersection(path.parts) or path.suffix == ".pyc":
            continue
        relative = path.name if root.is_file() else path.relative_to(root).as_posix()
        digest.update(relative.encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def metadata_messages(role: str, values: dict[str, str | int | bool]) -> str:
    """Encode non-content task references in the GenAI message shape MLflow maps."""
    return json.dumps([{
        "role": role,
        "parts": [{"type": "text", "content": json.dumps(values, sort_keys=True)}],
    }], separators=(",", ":"))


def child_span(
    name: str,
    *,
    started_ns: int,
    ended_ns: int,
    attributes: dict[str, str | int | float | bool],
    status: str = "ok",
    parent_span_id: str | None = None,
) -> dict[str, Any]:
    return {
        "spanId": secrets.token_hex(8),
        **({"parentSpanId": parent_span_id} if parent_span_id else {}),
        "name": name,
        "kind": 1,
        "startTimeUnixNano": str(started_ns),
        "endTimeUnixNano": str(max(started_ns, ended_ns)),
        "attributes": _attributes(attributes),
        "status": {"code": 2 if status == "error" else 1},
    }


def build_task_trace(
    *,
    harness: str,
    session_id: str,
    task_id: str,
    started_ns: int,
    ended_ns: int,
    attributes: dict[str, str | int | float | bool],
    children: list[dict[str, Any]],
    trace_id: str | None = None,
    status: str = "ok",
) -> dict[str, Any]:
    """Return one OTLP/JSON trace rooted at ``agent.task``."""
    resolved_trace_id = trace_id or secrets.token_hex(16)
    root_span_id = secrets.token_hex(8)
    root_attributes = {
        "app.agent.schema.version": SCHEMA_VERSION,
        "app.agent.record.type": "task",
        "app.agent.task.id": task_id,
        "app.agent.harness.name": harness,
        "app.agent.harness.version": "not_observed",
        "app.agent.repository.hash": "not_observed",
        "app.agent.repository.base_revision": "not_observed",
        "app.agent.task.class": "not_observed",
        "app.agent.risk.class": "not_observed",
        "app.agent.skill.catalogue_hash": "not_observed",
        "session.id": session_id,
        "app.agent.session.id": session_id,
        **attributes,
    }
    root = {
        "traceId": resolved_trace_id,
        "spanId": root_span_id,
        "name": "agent.task",
        "kind": 1,
        "startTimeUnixNano": str(started_ns),
        "endTimeUnixNano": str(ended_ns),
        "attributes": _attributes(root_attributes),
        "status": {"code": 2 if status == "error" else 1},
    }
    normalized_children = []
    for child in children:
        normalized = dict(child)
        normalized.setdefault("traceId", resolved_trace_id)
        normalized.setdefault("spanId", secrets.token_hex(8))
        normalized.setdefault("parentSpanId", root_span_id)
        normalized_children.append(normalized)
    resource_attributes = {
        "service.name": f"{harness}-coding-agent",
        "app.agent.schema.version": SCHEMA_VERSION,
        "app.agent.harness.name": harness,
        "deployment.environment.name": (
            "evaluation" if attributes.get("app.agent.trace.kind") == "evaluation" else "local"
        ),
        "app.agent.trace.kind": str(attributes.get("app.agent.trace.kind", "operational")),
    }
    for key in (
        "app.agent.repository.hash",
        "app.agent.repository.base_revision",
        "app.agent.skill.catalogue_hash",
    ):
        if key in attributes:
            resource_attributes[key] = attributes[key]
    payload = {
        "resourceSpans": [{
            "resource": {"attributes": _attributes(resource_attributes)},
            "scopeSpans": [{
                "scope": {"name": "app.agent.task", "version": SCHEMA_VERSION},
                "spans": [root, *normalized_children],
            }],
        }],
        "trace_id": resolved_trace_id,
        "mlflow_trace_id": f"tr-{resolved_trace_id}",
        "session_id": session_id,
    }
    return payload


def export_task_trace(
    trace: dict[str, Any],
    endpoint: str | None = None,
    *,
    timeout: float = 5.0,
) -> dict[str, str | int]:
    """Export OTLP/JSON without making telemetry failure change task behavior."""
    destination = endpoint or os.environ.get(
        "APP_AGENT_OTLP_ENDPOINT",
        os.environ.get("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", "http://docker-host:4318/v1/traces"),
    )
    payload = {"resourceSpans": trace["resourceSpans"]}
    request = urllib.request.Request(
        destination,
        data=json.dumps(payload, separators=(",", ":")).encode(),
        headers={"content-type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return {"status": "exported", "http_status": response.status}
    except (OSError, urllib.error.HTTPError, urllib.error.URLError) as error:
        return {"status": "export_failed", "error": type(error).__name__}
