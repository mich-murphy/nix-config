#!/usr/bin/env python3
"""Central operator interface for every packaged skill evaluation."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import eval_compare
import eval_runtime
from eval_identity import build_identity
from eval_publication import publication_content_hash, trace_result_hash, write_spool
from eval_validation import validate as validate_registry, validate_result_matrix
from telemetry import task_trace


ROOT = Path(__file__).resolve().parent
REGISTRY = ROOT / "eval_registry.json"


class EvaluationError(ValueError):
    """A user-actionable central evaluation error."""


def attach_neo_evidence(
    document: dict[str, Any], identity: dict[str, Any], *, offline: bool
) -> None:
    """Normalize Neo results and attach the common metadata-only task trace."""
    for result in document.get("results", []):
        deterministic = result.get("deterministic", {})
        checks = deterministic.get("checks", {}) if isinstance(deterministic, dict) else {}
        valid = bool(checks.get("process_healthy"))
        accepted = valid and deterministic.get("pass") is True
        ended_ns = time.time_ns()
        started_ns = ended_ns - int(float(result.get("duration_seconds", 0)) * 1_000_000_000)
        case_id = result.get("case_id", result.get("id", "not_observed"))
        variant = result.get("variant", "candidate")
        repetition = int(result.get("repetition", 1))
        total = len(checks)
        passed = sum(value is True for value in checks.values())
        result_binding = {
            **result,
            "id": case_id,
            "harness": result.get("harness", "not_observed"),
            "variant": variant,
            "mode": "end-to-end",
            "repetition": repetition,
            "valid": valid,
            "accepted": accepted,
            "assertions_passed": passed,
            "assertions_total": total,
        }
        trace = task_trace.build_task_trace(
            harness=result.get("harness", "not_observed"),
            session_id=f"neo-eval-{identity.get('execution_id', identity['key'][:16])}",
            task_id=f"{case_id}-{variant}-{repetition}",
            started_ns=started_ns,
            ended_ns=ended_ns,
            attributes={
                "app.agent.trace.kind": "evaluation",
                "app.agent.eval.identity": identity["key"],
                "app.agent.eval.result_hash": trace_result_hash(result_binding),
                "app.agent.eval.case_id": str(case_id),
                "app.agent.eval.variant": str(variant),
                "app.agent.eval.repetition": repetition,
                "app.agent.eval.mode": "end-to-end",
                "app.agent.skill.name": "neo" if variant != "no-skill" else "none",
                "app.agent.skill.package_hash": identity.get("skill_hash", "not_observed"),
                "app.agent.model.requested": result.get("model", "not_observed"),
                "app.agent.model.effort": result.get("effort", "not_observed"),
                "app.agent.outcome.status": "accepted" if accepted else "failed",
                "app.agent.content.capture": "metadata",
            },
            children=[],
            status="ok" if valid else "error",
        )
        delivery = (
            {"status": "disabled", "reason": "offline"}
            if offline
            else task_trace.export_task_trace(
                trace,
                os.environ.get(
                    "APP_AGENT_EVAL_OTLP_ENDPOINT", "http://docker-host:4318/v1/traces"
                ),
            )
        )
        result.update({
            "id": case_id,
            "mode": "end-to-end",
            "valid": valid,
            "accepted": accepted,
            "state": "succeeded" if valid else "harness_failure",
            "failure_kind": None if valid else "harness_failure",
            "assertions_passed": passed,
            "assertions_total": total,
            "score": passed / total if total else 0.0,
            "trace_id": trace["trace_id"],
            "mlflow_trace_id": trace["mlflow_trace_id"],
            "session_id": trace["session_id"],
            "telemetry": delivery,
            "skill_hash": identity.get("skill_hash", "not_observed"),
            "outcome": "accepted" if accepted else "failed",
            "evidence_state": "published" if delivery.get("status") == "exported" else "pending",
        })
        if delivery.get("status") != "exported":
            result["publication_failure_kind"] = (
                "offline" if offline else delivery.get("error", "export_failed")
            )
            result["publication_trace"] = {"resourceSpans": trace["resourceSpans"]}


def load_registry() -> dict[str, Any]:
    return json.loads(REGISTRY.read_text(encoding="utf-8"))


def list_skills(*, as_json: bool, skill_name: str | None = None) -> int:
    registry = load_registry()
    if skill_name:
        configuration = registered_skill(skill_name)
        eval_dir = ROOT / "skills" / skill_name / "evals"
        cases_value = json.loads((eval_dir / "cases.json").read_text(encoding="utf-8"))
        cases = cases_value.get("cases", []) if isinstance(cases_value, dict) else cases_value
        routing = json.loads((eval_dir / "routing-cases.json").read_text(encoding="utf-8"))
        routes = json.loads((eval_dir / "routes.json").read_text(encoding="utf-8"))
        payload = {
            "name": skill_name,
            **configuration,
            "cases": [case.get("id") for case in cases],
            "routing_cases": [case.get("id") for case in routing],
            "harnesses": sorted(routes.get("harnesses", {})),
        }
        if as_json:
            print(json.dumps(payload, indent=2))
        else:
            print(f"{skill_name}\t{configuration['adapter']}")
            for case in payload["cases"]:
                print(f"case\t{case}")
            for case in payload["routing_cases"]:
                print(f"routing\t{case}")
        return 0
    skills = [
        {"name": name, **configuration}
        for name, configuration in sorted(registry["skills"].items())
    ]
    if as_json:
        print(json.dumps({"schema_version": registry["schema_version"], "skills": skills}, indent=2))
    else:
        for skill in skills:
            print(f"{skill['name']}\t{skill['adapter']}")
    return 0


def registered_skill(name: str) -> dict[str, Any]:
    configuration = load_registry()["skills"].get(name)
    if configuration is None:
        raise EvaluationError(f"unknown skill: {name}")
    return configuration


def specialized_suite(adapter: str, requested_suite: str) -> str:
    if adapter != "neo":
        return requested_suite
    return "full" if requested_suite in {"held-out", "full"} else "smoke"


def specialized_repetitions(
    adapter: str, effective_suite: str, requested_repetitions: int | None
) -> int:
    if requested_repetitions is not None:
        return requested_repetitions
    if adapter == "neo":
        return 5 if effective_suite == "full" else 3
    if effective_suite == "smoke":
        return 1
    return 5 if effective_suite in {"held-out", "full"} else 3


def validate_release_provenance(
    skill: str,
    candidate: dict[str, Any],
    comparison: dict[str, Any],
    publication: dict[str, Any],
) -> None:
    identity = candidate.get("evaluation_identity", {})
    execution_id = identity.get("execution_id")
    if not isinstance(execution_id, str) or not execution_id:
        raise EvaluationError("release candidate lacks a unique execution identity")
    current = build_identity(
        skill,
        ROOT / "skills" / skill / "evals",
        candidate.get("configuration", {}),
        execution_id=execution_id,
    )
    provenance_fields = (
        "schema_version", "skill", "key", "definition_hashes", "skill_hash",
        "evaluator_hash", "controls",
    )
    if any(identity.get(field) != current.get(field) for field in provenance_fields):
        raise EvaluationError(
            "release candidate does not match the current package and evaluator provenance"
        )
    content_hash = publication_content_hash(candidate)
    if comparison.get("candidate_identity") != identity["key"]:
        raise EvaluationError("comparison does not join to the candidate identity")
    if comparison.get("candidate_content_hash") != content_hash:
        raise EvaluationError("comparison does not bind to the candidate content")
    if publication.get("evaluation_identity") != identity["key"]:
        raise EvaluationError("published evidence does not join to the candidate identity")
    if publication.get("content_hash") != content_hash:
        raise EvaluationError("publication does not bind to the candidate content")


def run_skill(args: argparse.Namespace) -> int:
    configuration = registered_skill(args.skill)
    if args.strict and args.offline:
        raise EvaluationError("strict runs cannot be offline")
    if configuration["adapter"] != "generic":
        return run_specialized(args, configuration["adapter"])
    eval_dir = ROOT / "skills" / args.skill / "evals"
    runtime_arguments = [
        "--output", str(args.output),
        "--harness", args.harness,
        "--variant", args.variant,
        "--mode", args.mode,
        "--suite", args.suite,
        "--timeout", str(args.timeout),
    ]
    if args.repetitions is not None:
        runtime_arguments.extend(("--repetitions", str(args.repetitions)))
    if args.offline:
        runtime_arguments.append("--offline")
    if args.strict:
        runtime_arguments.append("--strict")
    if args.spool:
        runtime_arguments.extend(("--spool", str(args.spool)))
    return eval_runtime.main(eval_dir, runtime_arguments)


def run_specialized(args: argparse.Namespace, adapter: str) -> int:
    if args.mode not in {"all", "end-to-end"}:
        raise EvaluationError(f"{args.skill} specialized adapter supports end-to-end mode only")
    runner = ROOT / "skills" / args.skill / "evals" / "run-evals.py"
    command = [sys.executable, str(runner), "--output", str(args.output)]
    if adapter == "neo":
        neo_suite = specialized_suite(adapter, args.suite)
        command.extend(("--harness", args.harness, "--suite", neo_suite))
        neo_variant = "all" if args.variant == "all" else args.variant
        command.extend(("--variant", neo_variant, "--timeout", str(args.timeout)))
        if args.repetitions is not None:
            command.extend(("--repetitions", str(args.repetitions)))
        if args.ack_full_cost:
            command.append("--ack-full-cost")
    elif adapter == "skill-development":
        skill_variant = "both" if args.variant == "all" else args.variant
        if skill_variant == "no-skill":
            raise EvaluationError("skill-development has no no-skill specialized variant; use incumbent")
        command.extend((
            "--harness", args.harness,
            "--variant", skill_variant,
            "--suite", args.suite,
            "--timeout", str(args.timeout),
        ))
        if args.repetitions is not None:
            command.extend(("--repetitions", str(args.repetitions)))
    else:
        raise EvaluationError(f"unknown adapter: {adapter}")
    if adapter == "skill-development":
        if args.offline:
            command.append("--offline")
        if args.strict:
            command.append("--strict")
        if args.spool:
            command.extend(("--spool", str(args.spool)))
    completed = subprocess.run(command, check=False)
    if not args.output.is_file():
        return completed.returncode
    document = json.loads(args.output.read_text(encoding="utf-8"))
    eval_dir = ROOT / "skills" / args.skill / "evals"
    effective_suite = specialized_suite(adapter, args.suite)
    effective_repetitions = specialized_repetitions(
        adapter, effective_suite, args.repetitions
    )
    document.setdefault("configuration", {
        "skill": args.skill,
        "suite": effective_suite,
        "repetitions": effective_repetitions,
        "harnesses": ["codex", "claude", "pi"] if args.harness == "all" else [args.harness],
        "variants": ["no-skill", "incumbent", "candidate"] if args.variant == "all" else [args.variant],
        "modes": ["end-to-end"] if adapter == "neo" else [args.mode],
    })
    if "evaluation_identity" not in document:
        document["evaluation_identity"] = build_identity(
            args.skill, eval_dir, document.get("configuration", {})
        )
    if adapter == "neo":
        attach_neo_evidence(
            document, document["evaluation_identity"], offline=args.offline
        )
        states = {item.get("evidence_state") for item in document.get("results", [])}
        document["evidence_state"] = "published" if states == {"published"} else "pending"
    else:
        document.setdefault("evidence_state", "pending")
    if document["evidence_state"] != "published":
        spool_root = args.spool or ROOT / ".eval-spool"
        document["publication_spool"] = str(write_spool(document, spool_root))
    args.output.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    if args.strict and document["evidence_state"] != "published":
        return 1
    return completed.returncode


def replay_skill(args: argparse.Namespace) -> int:
    configuration = registered_skill(args.skill)
    eval_dir = ROOT / "skills" / args.skill / "evals"
    if configuration["adapter"] == "generic":
        eval_runtime.replay(eval_dir, args.source, args.output)
        document = json.loads(args.output.read_text(encoding="utf-8"))
    else:
        document = json.loads(args.source.read_text(encoding="utf-8"))
        results = document.get("results")
        if not isinstance(results, list):
            raise EvaluationError("specialized replay source needs a results list")
        if configuration["adapter"] == "neo":
            valid = all(
                isinstance(item, dict)
                and isinstance(item.get("deterministic"), dict)
                and isinstance(item["deterministic"].get("pass"), bool)
                for item in results
            )
        else:
            valid = all(
                isinstance(item, dict)
                and isinstance(item.get("valid"), bool)
                and isinstance(item.get("accepted"), bool)
                and isinstance(item.get("score"), (int, float))
                for item in results
            )
        document = dict(document)
        document["adapter_validation"] = {
            "adapter": configuration["adapter"],
            "valid": valid,
        }
        if not valid:
            raise EvaluationError(f"{args.skill} specialized replay validation failed")
    document["evaluation_identity"] = build_identity(
        args.skill,
        eval_dir,
        document.get("configuration", {}),
        execution_id=document.get("evaluation_identity", {}).get("execution_id"),
    )
    document["evidence_state"] = "pending" if args.offline else "not_published"
    args.output.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    return 0


def compare_skill(args: argparse.Namespace) -> int:
    configuration = registered_skill(args.skill)
    document = json.loads(args.candidate.read_text(encoding="utf-8"))
    eval_dir = ROOT / "skills" / args.skill / "evals"
    matrix = validate_result_matrix(eval_dir, configuration["adapter"], document)
    if configuration["adapter"] == "generic":
        comparison = eval_compare.compare(document)
        comparison["matrix"] = matrix
        comparison["checks"]["complete_valid_matrix"] = matrix["valid"]
        comparison["decision"] = "pass" if all(comparison["checks"].values()) else "defer"
        comparison["release_eligible"] = bool(
            comparison["decision"] == "pass"
            and document.get("configuration", {}).get("suite") in {"held-out", "full"}
            and document.get("configuration", {}).get("repetitions", 0) >= 5
        )
        rendered = json.dumps(comparison, indent=2) + "\n"
        args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 0 if comparison["decision"] == "pass" else 1
    comparator = ROOT / "skills" / args.skill / "evals" / "compare-evals.py"
    completed = subprocess.run(
        [sys.executable, str(comparator), str(args.candidate)],
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.stdout:
        comparison = json.loads(completed.stdout)
        comparison["candidate_identity"] = document.get("evaluation_identity", {}).get("key")
        comparison["candidate_content_hash"] = publication_content_hash(document)
        comparison["matrix"] = matrix
        if configuration["adapter"] == "neo":
            config = document.get("configuration", {})
            comparison["decision"] = "pass" if comparison.get("pass") else "defer"
            comparison["release_eligible"] = bool(
                comparison.get("pass")
                and config.get("suite") == "full"
                and config.get("repetitions", 0) >= 5
            )
            comparison["checks"] = {
                "complete_valid_matrix": matrix["valid"],
                "quality_maintained": bool(comparison.get("quality_maintained")),
                "model_invocations_reduced": bool(comparison.get("model_invocations_reduced")),
                "latency_reduced": bool(comparison.get("latency_reduced")),
            }
            if not all(comparison["checks"].values()):
                comparison["decision"] = "defer"
                comparison["release_eligible"] = False
        else:
            comparison.setdefault("checks", {})["complete_valid_matrix"] = matrix["valid"]
            if not matrix["valid"]:
                comparison["decision"] = "defer"
                comparison["release_eligible"] = False
        rendered = json.dumps(comparison, indent=2) + "\n"
        args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    if not completed.stdout:
        return completed.returncode or 1
    return 0 if comparison.get("decision") in {"pass", "development-pass"} else 1


def publish_skill(args: argparse.Namespace) -> int:
    registered_skill(args.skill)
    document = json.loads(args.results.read_text(encoding="utf-8"))
    configured_skill = document.get("configuration", {}).get("skill")
    if configured_skill not in {None, args.skill}:
        raise EvaluationError(
            f"publication skill mismatch: requested {args.skill}, document has {configured_skill}"
        )
    command = ["uv", "run", str(ROOT / "telemetry" / "publish_evals.py"), str(args.results)]
    if args.tracking_uri:
        command.extend(("--tracking-uri", args.tracking_uri))
    if args.experiment:
        command.extend(("--experiment", args.experiment))
    if args.dataset:
        command.extend(("--dataset", args.dataset))
    if args.otlp_endpoint:
        command.extend(("--otlp-endpoint", args.otlp_endpoint))
    if args.command == "retry":
        command.append("--update-candidate")
    completed = subprocess.run(command, check=False, capture_output=True, text=True)
    if completed.stdout:
        print(completed.stdout, end="")
        if args.output:
            args.output.write_text(completed.stdout, encoding="utf-8")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    return completed.returncode


def release_skill(args: argparse.Namespace) -> int:
    registered_skill(args.skill)
    candidate = json.loads(args.candidate.read_text(encoding="utf-8"))
    comparison = json.loads(args.comparison.read_text(encoding="utf-8"))
    publication = json.loads(args.publication.read_text(encoding="utf-8"))
    identity = candidate.get("evaluation_identity", {})
    identity_key = identity.get("key")
    if candidate.get("configuration", {}).get("skill") != args.skill:
        raise EvaluationError("candidate skill does not match the requested release")
    release_eligible = args.decision in {"adopt", "restrict"}
    if release_eligible:
        matrix = validate_result_matrix(
            ROOT / "skills" / args.skill / "evals",
            registered_skill(args.skill)["adapter"],
            candidate,
        )
        if (
            comparison.get("decision") not in {"pass", "development-pass"}
            or comparison.get("release_eligible") is not True
            or not all(comparison.get("checks", {}).values())
        ):
            raise EvaluationError("strict release requires a passing complete comparison")
        if candidate.get("evidence_state") != "published" or publication.get("evidence_state") != "published":
            raise EvaluationError("strict release requires complete published evidence")
        validate_release_provenance(args.skill, candidate, comparison, publication)
        candidate_configuration = candidate.get("configuration", {})
        required_suites = {"full"} if registered_skill(args.skill)["adapter"] == "neo" else {"held-out", "full"}
        if (
            candidate_configuration.get("suite") not in required_suites
            or candidate_configuration.get("repetitions", 0) < 5
        ):
            raise EvaluationError("strict release requires a held-out/full five-repetition candidate")
        if not matrix["valid"]:
            raise EvaluationError("strict release requires the exact configured result matrix")
        if not candidate.get("results") or not all(item.get("valid") for item in candidate["results"]):
            raise EvaluationError("strict release requires a complete valid matrix")
        tracking_uri = args.tracking_uri or publication.get("tracking_uri")
        experiment = publication.get("experiment", "skill-evaluations")
        run_id = publication.get("run_id")
        if not tracking_uri or not run_id:
            raise EvaluationError("strict release requires queryable MLflow summary references")
        protection = subprocess.run(
            [
                "uv", "run", str(ROOT / "telemetry" / "protect_evals.py"),
                "--tracking-uri", tracking_uri,
                "--experiment", experiment,
                "--run-id", run_id,
                "--identity", identity_key,
                "--content-hash", publication["content_hash"],
                "--decision", args.decision,
                *(["--supersedes", args.supersedes] if args.supersedes else []),
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        if protection.returncode:
            raise EvaluationError(
                "MLflow protection failed: " + (protection.stderr.strip() or protection.stdout.strip())
            )
    revision = subprocess.check_output(
        ["git", "-C", str(ROOT.parents[1]), "rev-parse", "HEAD"], text=True
    ).strip()
    manifest = {
        "schema_version": "1.0.0",
        "skill": args.skill,
        "git_revision": revision,
        "definition_hashes": identity.get("definition_hashes", {}),
        "evaluation_identity": identity_key,
        "mlflow": {
            key: publication.get(key)
            for key in (
                "experiment", "dataset", "run_id", "content_hash", "records",
                "assessments",
            )
            if key in publication
        },
        "owner_decision": args.decision,
        "owner": args.owner,
        "limitations": args.limitation,
        "supersedes": args.supersedes,
        "protection": {
            "state": "protected" if release_eligible else "ordinary",
            "protected_until_superseded": release_eligible,
            "superseded_at": None,
            "grace_days": 365,
        },
        "release_eligible": release_eligible,
        "recorded_at": datetime.now(timezone.utc).isoformat(),
    }
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))
    return 0


def validate_all(*, as_json: bool) -> int:
    report = validate_registry(ROOT)
    if as_json:
        print(json.dumps(report, indent=2))
    else:
        print("evaluation registry is valid" if report["valid"] else "evaluation registry is invalid")
        for error in report["errors"]:
            print(error)
    return 0 if report["valid"] else 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    command = commands.add_parser("list", help="list registered skill evaluators")
    command.add_argument("skill", nargs="?")
    command.add_argument("--json", action="store_true")
    command = commands.add_parser("run", help="run a registered skill evaluator")
    command.add_argument("skill")
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--harness", choices=("codex", "claude", "pi", "all"), default="all")
    command.add_argument("--variant", choices=("no-skill", "incumbent", "candidate", "all"), default="all")
    command.add_argument("--mode", choices=("routing", "conditional", "end-to-end", "all"), default="all")
    command.add_argument("--suite", choices=("development", "held-out", "full", "smoke"), default="development")
    command.add_argument("--repetitions", type=int)
    command.add_argument("--timeout", type=int, default=240)
    command.add_argument("--offline", action="store_true")
    command.add_argument("--strict", action="store_true")
    command.add_argument("--spool", type=Path)
    command.add_argument(
        "--ack-full-cost",
        action="store_true",
        help="explicitly acknowledge Neo full-suite model usage",
    )
    command = commands.add_parser("replay", help="re-score a preserved run without calling a model")
    command.add_argument("skill")
    command.add_argument("source", type=Path)
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--offline", action="store_true")
    command = commands.add_parser("compare", help="compare a candidate with its controls")
    command.add_argument("skill")
    command.add_argument("candidate", type=Path)
    command.add_argument("--output", type=Path, required=True)
    for name in ("publish", "retry"):
        command = commands.add_parser(name, help="publish or reconcile evaluation evidence")
        command.add_argument("skill")
        command.add_argument("results", type=Path)
        command.add_argument("--output", type=Path)
        command.add_argument("--tracking-uri")
        command.add_argument("--experiment")
        command.add_argument("--dataset")
        command.add_argument("--otlp-endpoint")
    command = commands.add_parser("release", help="write a compact strict release manifest")
    command.add_argument("skill")
    command.add_argument("--candidate", type=Path, required=True)
    command.add_argument("--comparison", type=Path, required=True)
    command.add_argument("--publication", type=Path, required=True)
    command.add_argument("--owner", required=True)
    command.add_argument("--decision", choices=("defer", "adopt", "reject", "restrict"), required=True)
    command.add_argument("--manifest", type=Path, required=True)
    command.add_argument("--limitation", action="append", default=[])
    command.add_argument("--supersedes")
    command.add_argument("--tracking-uri")
    command = commands.add_parser("validate", help="validate registry and release-manifest integrity")
    command.add_argument("--json", action="store_true")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if args.command == "list":
            return list_skills(as_json=args.json, skill_name=args.skill)
        if args.command == "run":
            return run_skill(args)
        if args.command == "replay":
            return replay_skill(args)
        if args.command == "compare":
            return compare_skill(args)
        if args.command in {"publish", "retry"}:
            return publish_skill(args)
        if args.command == "release":
            return release_skill(args)
        if args.command == "validate":
            return validate_all(as_json=args.json)
        raise AssertionError(args.command)
    except EvaluationError as error:
        print(f"evaluation error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
