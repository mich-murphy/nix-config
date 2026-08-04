"""Privacy-first publication records and retry spool mechanics."""

from __future__ import annotations

import json
import os
import tempfile
import hashlib
from pathlib import Path
from typing import Any


SCHEMA_VERSION = "1.0.0"
CONFIGURATION_FIELDS = (
    "skill",
    "suite",
    "repetitions",
    "harnesses",
    "variants",
    "modes",
    "routes",
    "fixed",
)
RESULT_FIELDS = (
    "id",
    "split",
    "risk",
    "harness",
    "variant",
    "mode",
    "repetition",
    "model",
    "effort",
    "model_returned",
    "state",
    "failure_kind",
    "publication_failure_kind",
    "valid",
    "accepted",
    "expected_activation",
    "actual_activation",
    "assertions_passed",
    "assertions_total",
    "score",
    "duration_seconds",
    "usage",
    "returncode",
    "telemetry_warning",
    "trace_id",
    "mlflow_trace_id",
    "session_id",
    "evidence_state",
    "skill_hash",
    "prompt_version",
    "tool_version",
    "evaluator_version",
    "repository_revision",
    "outcome",
)
IDENTITY_FIELDS = (
    "schema_version",
    "skill",
    "definition_hashes",
    "controls",
    "key",
    "execution_id",
    "skill_hash",
    "evaluator_hash",
)
TRACE_RESULT_FIELDS = (
    "id", "harness", "variant", "mode", "repetition", "valid", "accepted",
    "assertions_passed", "assertions_total",
)

SAFE_FIXED_CONTROLS = {
    "task", "model", "effort", "tools", "permissions", "timeout", "workspace", "verifier",
}
SAFE_USAGE_FIELDS = {
    "input_tokens", "output_tokens", "cached_input_tokens", "cache_read_input_tokens",
    "cache_creation_input_tokens", "reasoning_tokens", "total_tokens", "cost_usd",
}
FORBIDDEN_KEYS = (
    "prompt", "response", "content", "payload", "argument", "result", "environment",
    "credential", "secret", "source", "token_value", "stderr", "stdout", "output", "message",
)


def _bounded(value: Any, *, depth: int = 0) -> Any:
    if depth > 5:
        return "truncated"
    if isinstance(value, str):
        return value[:512]
    if isinstance(value, (bool, int, float)) or value is None:
        return value
    if isinstance(value, list):
        return [_bounded(item, depth=depth + 1) for item in value[:100]]
    if isinstance(value, dict):
        return {
            str(key)[:128]: _bounded(item, depth=depth + 1)
            for key, item in list(sorted(value.items()))[:100]
            if not any(term in str(key).casefold() for term in FORBIDDEN_KEYS)
        }
    return str(value)[:512]


def _safe_routes(value: Any) -> dict[str, dict[str, Any]]:
    if not isinstance(value, dict):
        return {}
    return {
        str(harness)[:64]: {
            key: _bounded(route[key])
            for key in ("model", "effort")
            if isinstance(route, dict) and key in route
        }
        for harness, route in list(sorted(value.items()))[:32]
        if isinstance(route, dict)
    }


def _safe_configuration(configuration: Any) -> dict[str, Any]:
    if not isinstance(configuration, dict):
        return {}
    safe = {
        field: _bounded(configuration[field])
        for field in ("skill", "suite", "repetitions", "harnesses", "variants", "modes")
        if field in configuration
    }
    if "routes" in configuration:
        safe["routes"] = _safe_routes(configuration["routes"])
    if isinstance(configuration.get("fixed"), list):
        safe["fixed"] = [
            item for item in configuration["fixed"][:32]
            if isinstance(item, str) and item in SAFE_FIXED_CONTROLS
        ]
    return safe


def _safe_metrics(value: Any, *, depth: int = 0) -> Any:
    if depth > 4:
        return None
    if isinstance(value, (bool, int, float)) or value is None:
        return value
    if isinstance(value, dict):
        return {
            str(key)[:128]: safe
            for key, item in list(sorted(value.items()))[:100]
            if not any(term in str(key).casefold() for term in FORBIDDEN_KEYS)
            and (safe := _safe_metrics(item, depth=depth + 1)) is not None
        }
    return None


def _safe_usage(value: Any) -> dict[str, int | float]:
    if not isinstance(value, dict):
        return {}
    return {
        key: item
        for key, item in value.items()
        if key in SAFE_USAGE_FIELDS and isinstance(item, (int, float)) and not isinstance(item, bool)
    }


def _safe_failure_kind(value: Any) -> str | None:
    return value if value in {
        None, "authentication", "harness_not_found", "timeout", "network",
        "model_or_task", "incumbent_missing", "harness_failure", "offline",
        "export_failed",
    } else "other"


def _safe_result_enum(field: str, value: Any) -> str:
    allowed = {
        "state": {
            "succeeded", "task_failure", "harness_failure", "environment_failure",
            "evaluator_failure",
        },
        "outcome": {
            "accepted", "failed", "invalid_harness", "invalid_environment",
            "evaluator_error",
        },
    }
    return value if value in allowed[field] else "other"


def comparison_claim(result: dict[str, Any]) -> dict[str, Any]:
    """Return only privacy-safe fields that can affect the selected comparator."""
    existing = result.get("comparison_claim")
    if isinstance(existing, dict):
        adapter = existing.get("adapter")
        try:
            if adapter == "neo":
                normalized = {
                    "adapter": "neo",
                    "skill_enabled": bool(existing.get("skill_enabled")),
                    "returncode": int(existing.get("returncode", -1)),
                    "timed_out": bool(existing.get("timed_out")),
                    "deterministic_pass": bool(existing.get("deterministic_pass")),
                    "model_invocations": int(existing.get("model_invocations", 0)),
                    "duration_seconds": float(existing.get("duration_seconds", 0)),
                }
            elif adapter == "skill-development":
                normalized = {
                    "adapter": "skill-development",
                    "score": float(existing.get("score", 0)),
                    "blocking_failure_count": int(
                        existing.get("blocking_failure_count", 0)
                    ),
                }
            elif adapter == "generic":
                normalized = {
                    "adapter": "generic",
                    "risk": existing.get("risk", "normal"),
                    "expected_activation": existing.get("expected_activation"),
                    "actual_activation": existing.get("actual_activation"),
                    "duration_seconds": float(existing.get("duration_seconds", 0)),
                    "usage": _safe_usage(existing.get("usage", {})),
                }
            else:
                raise ValueError("unknown comparison adapter")
        except (TypeError, ValueError) as error:
            raise ValueError("comparison claim conflicts with raw result") from error

        neo_raw = any(
            field in result
            for field in ("skill_enabled", "deterministic", "steps", "timed_out")
        )
        skill_development_raw = any(
            field in result for field in ("blocking_failures", "blocking_failure_count")
        )
        if neo_raw:
            raw = dict(result)
            raw.pop("comparison_claim", None)
            if adapter != "neo" or comparison_claim(raw) != normalized:
                raise ValueError("comparison claim conflicts with raw result")
        elif skill_development_raw:
            raw = dict(result)
            raw.pop("comparison_claim", None)
            if adapter != "skill-development" or comparison_claim(raw) != normalized:
                raise ValueError("comparison claim conflicts with raw result")
        elif adapter == "generic":
            raw = dict(result)
            raw.pop("comparison_claim", None)
            if comparison_claim(raw) != normalized:
                raise ValueError("comparison claim conflicts with raw result")
        elif adapter == "neo" and (
            ("returncode" in result and int(result["returncode"]) != normalized["returncode"])
            or (
                "duration_seconds" in result
                and float(result["duration_seconds"]) != normalized["duration_seconds"]
            )
        ):
            raise ValueError("comparison claim conflicts with raw result")
        elif adapter == "skill-development" and (
            "score" in result and float(result["score"]) != normalized["score"]
        ):
            raise ValueError("comparison claim conflicts with raw result")
        return normalized
    if any(field in result for field in ("skill_enabled", "deterministic", "steps", "timed_out")):
        deterministic = result.get("deterministic", {})
        return {
            "adapter": "neo",
            "skill_enabled": bool(result.get("skill_enabled")),
            "returncode": int(result.get("returncode", -1)),
            "timed_out": bool(result.get("timed_out")),
            "deterministic_pass": bool(
                deterministic.get("pass") if isinstance(deterministic, dict) else False
            ),
            "model_invocations": sum(
                item.get("kind") == "model"
                for item in result.get("steps", [])
                if isinstance(item, dict)
            ),
            "duration_seconds": float(result.get("duration_seconds", 0)),
        }
    if "score" in result or "blocking_failures" in result or "blocking_failure_count" in result:
        blockers = result.get("blocking_failures", [])
        return {
            "adapter": "skill-development",
            "score": float(result.get("score", 0)),
            "blocking_failure_count": int(
                len(blockers)
                if "blocking_failures" in result and isinstance(blockers, list)
                else result.get("blocking_failure_count", 0)
            ),
        }
    usage = _safe_usage(result.get("usage", {}))
    return {
        "adapter": "generic",
        "risk": result.get("risk", "normal"),
        "expected_activation": result.get("expected_activation"),
        "actual_activation": result.get("actual_activation"),
        "duration_seconds": float(result.get("duration_seconds", 0)),
        "usage": usage,
    }


def safe_publication_document(document: dict[str, Any]) -> dict[str, Any]:
    """Return only bounded metadata permitted on default MLflow surfaces."""
    identity = document.get("evaluation_identity", {})
    configuration = _safe_configuration(document.get("configuration", {}))
    safe_identity = {
        field: _bounded(identity[field])
        for field in IDENTITY_FIELDS
        if field in identity and field != "controls"
    }
    if "controls" in identity:
        safe_identity["controls"] = _safe_configuration(identity["controls"])
    safe_results = []
    for result in document.get("results", [])[:10_000]:
        safe_result = {
            field: _bounded(result[field])
            for field in RESULT_FIELDS
            if field in result and field not in {
                "usage", "failure_kind", "publication_failure_kind", "state", "outcome",
            }
        }
        for field in ("state", "outcome"):
            if field in result:
                safe_result[field] = _safe_result_enum(field, result[field])
        for field in ("failure_kind", "publication_failure_kind"):
            if field in result:
                safe_result[field] = _safe_failure_kind(result[field])
        if "usage" in result:
            safe_result["usage"] = _safe_usage(result["usage"])
        # This structure is assembled from controlled enums, booleans, and numbers
        # above. Keep its controlled string fields: comparator selection and risk
        # are both material to the published decision.
        safe_result["comparison_claim"] = _bounded(comparison_claim(result))
        telemetry_status = result.get("telemetry", {}).get("status")
        if telemetry_status:
            safe_result["telemetry"] = {
                "status": "exported" if telemetry_status == "exported" else "pending"
            }
        safe_results.append(safe_result)
    return {
        "schema_version": SCHEMA_VERSION,
        "evaluation_identity": safe_identity,
        "configuration": configuration,
        "summary": _safe_metrics(document.get("summary", {})) or {},
        "results": safe_results,
    }


def publication_content_hash(document: dict[str, Any]) -> str:
    safe = safe_publication_document(document)
    for result in safe.get("results", []):
        for field in (
            "evidence_state", "publication_failure_kind", "telemetry",
        ):
            result.pop(field, None)
    return hashlib.sha256(
        json.dumps(safe, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def trace_result_hash(result: dict[str, Any]) -> str:
    payload = {
        "result": {
            field: _bounded(result.get(field))
            for field in TRACE_RESULT_FIELDS
        },
        "comparison_claim": comparison_claim(result),
    }
    return hashlib.sha256(
        json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def logical_objects(document: dict[str, Any]) -> dict[str, list[str]]:
    """Return deterministic logical keys used to reconcile partial retries."""
    identity_key = document.get("evaluation_identity", {}).get("key")
    if not isinstance(identity_key, str) or len(identity_key) != 64:
        raise ValueError("evaluation identity key is required for publication")
    traces = {
        result["mlflow_trace_id"]
        for result in document.get("results", [])
        if result.get("mlflow_trace_id")
        and result.get("telemetry", {}).get("status") == "exported"
    }
    dataset_records = {
        f"{identity_key}:{trace_id}"
        for trace_id in traces
    }
    assessments = {
        f"{trace_id}:{name}"
        for trace_id in traces
        for name in ("verified_task_outcome", "case_id")
    }
    return {
        "summary": [identity_key],
        "dataset_records": sorted(dataset_records),
        "assessments": sorted(assessments),
    }


def spool_path(spool_root: Path, identity_key: str) -> Path:
    return spool_root / f"{identity_key}.json"


def _safe_attributes(attributes: Any) -> list[dict[str, Any]]:
    safe = []
    forbidden = (
        "prompt",
        "response",
        "content",
        "tool.payload",
        "tool.arguments",
        "tool.result",
        "environment",
        "credential",
        "secret",
        "gen_ai.input.messages",
        "gen_ai.output.messages",
        "user.",
        "source",
    )
    for attribute in attributes if isinstance(attributes, list) else []:
        if not isinstance(attribute, dict) or not isinstance(attribute.get("key"), str):
            continue
        key = attribute["key"]
        if any(term in key.casefold() for term in forbidden):
            continue
        if not (
            key.startswith("app.agent.")
            or key.startswith("gen_ai.usage.")
            or key in {"gen_ai.operation.name", "gen_ai.request.model", "session.id", "service.name", "deployment.environment.name"}
        ):
            continue
        safe.append({"key": key[:256], "value": _bounded(attribute.get("value"))})
    return safe


def safe_retry_trace(trace: dict[str, Any]) -> dict[str, Any]:
    resource_spans = []
    for resource_span in trace.get("resourceSpans", []):
        scopes = []
        for scope_span in resource_span.get("scopeSpans", []):
            spans = []
            for span in scope_span.get("spans", []):
                spans.append({
                    key: _bounded(span[key])
                    for key in (
                        "traceId", "spanId", "parentSpanId", "name", "kind",
                        "startTimeUnixNano", "endTimeUnixNano", "status",
                    )
                    if key in span
                } | {"attributes": _safe_attributes(span.get("attributes"))})
            scope = scope_span.get("scope", {})
            scopes.append({
                "scope": {
                    key: _bounded(scope[key])
                    for key in ("name", "version")
                    if key in scope
                },
                "spans": spans,
            })
        resource_spans.append({
            "resource": {
                "attributes": _safe_attributes(
                    resource_span.get("resource", {}).get("attributes")
                )
            },
            "scopeSpans": scopes,
        })
    return {"resourceSpans": resource_spans}


def write_spool(document: dict[str, Any], spool_root: Path) -> Path:
    safe = safe_publication_document(document)
    identity_key = safe.get("evaluation_identity", {}).get("key")
    if not isinstance(identity_key, str) or len(identity_key) != 64:
        raise ValueError("evaluation identity key is required before spooling")
    retry_traces = []
    for result in document.get("results", []):
        trace = result.get("publication_trace")
        if not isinstance(trace, dict) or not result.get("mlflow_trace_id"):
            continue
        retry_traces.append({
            "trace_id": _bounded(result.get("trace_id")),
            "mlflow_trace_id": _bounded(result["mlflow_trace_id"]),
            "payload": safe_retry_trace(trace),
        })
    safe["retry_traces"] = retry_traces
    for original, result in zip(document.get("results", []), safe["results"]):
        exported = original.get("telemetry", {}).get("status") == "exported"
        result["telemetry"] = {"status": "exported" if exported else "pending"}
        result["evidence_state"] = "published" if exported else "pending"
    spool_root.mkdir(parents=True, exist_ok=True)
    target = spool_path(spool_root, identity_key)
    handle, temporary = tempfile.mkstemp(prefix=".eval-", dir=spool_root)
    try:
        with os.fdopen(handle, "w", encoding="utf-8") as stream:
            json.dump(safe, stream, indent=2)
            stream.write("\n")
        os.replace(temporary, target)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return target
