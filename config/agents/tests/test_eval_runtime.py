from __future__ import annotations

import json
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import eval_compare  # noqa: E402
import eval_cli  # noqa: E402
import eval_identity  # noqa: E402
import eval_publication  # noqa: E402
import eval_runtime  # noqa: E402
import eval_validation  # noqa: E402
from telemetry import protect_evals, publish_evals, task_trace  # noqa: E402


class EvalRuntimeTests(unittest.TestCase):
    def test_evaluation_identity_is_unique_per_execution_and_binds_skill_and_evaluator(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        configuration = {"skill": "bro", "suite": "smoke", "repetitions": 1}

        first = eval_identity.build_identity("bro", eval_dir, configuration)
        second = eval_identity.build_identity("bro", eval_dir, configuration)
        replay = eval_identity.build_identity(
            "bro", eval_dir, configuration, execution_id=first["execution_id"]
        )

        self.assertNotEqual(first["key"], second["key"])
        self.assertEqual(first["key"], replay["key"])
        self.assertRegex(first["skill_hash"], r"^[0-9a-f]{64}$")
        self.assertRegex(first["evaluator_hash"], r"^[0-9a-f]{64}$")
        self.assertIn("incumbent/SKILL.md", first["definition_hashes"])

    def test_evaluator_provenance_covers_dispatch_policy_trace_and_publication(self) -> None:
        hashes = eval_identity.evaluator_hashes(ROOT / "skills" / "bro" / "evals")

        self.assertTrue({
            "eval_registry.json",
            "evaluation/policy.json",
            "telemetry/task_trace.py",
            "telemetry/publish_evals.py",
            "telemetry/protect_evals.py",
        }.issubset(hashes))

    def test_publication_record_excludes_raw_content_and_is_bounded(self) -> None:
        canary = "privacy-canary-secret-value"
        document = {
            "schema_version": "2.0.0",
            "evaluation_identity": {"key": "a" * 64, "execution_id": "exec-1"},
            "configuration": {"skill": "bro", "suite": "smoke"},
            "summary": {"runs": 1, "accepted": 1},
            "results": [{
                "id": "case-1",
                "harness": "codex",
                "variant": "candidate",
                "mode": "conditional",
                "repetition": 1,
                "state": "succeeded",
                "failure_kind": None,
                "accepted": True,
                "valid": True,
                "assertions_passed": 2,
                "assertions_total": 2,
                "duration_seconds": 0.1,
                "usage": {"input_tokens": 4, "output_tokens": 2},
                "mlflow_trace_id": "tr-abc",
                "telemetry": {"status": "exported"},
                "output": canary,
                "stderr": canary,
                "events": [{"prompt": canary}],
                "environment": {"TOKEN": canary},
            }],
        }

        safe = eval_publication.safe_publication_document(document)
        serialized = json.dumps(safe, sort_keys=True)

        self.assertNotIn(canary, serialized)
        self.assertNotIn("output", safe["results"][0])
        self.assertNotIn("stderr", safe["results"][0])
        self.assertEqual(safe["evaluation_identity"]["key"], "a" * 64)
        self.assertLess(len(serialized), 10_000)

    def test_publication_recursively_filters_forbidden_nested_content(self) -> None:
        canary = "privacy-canary-nested-secret"
        document = {
            "evaluation_identity": {
                "key": "f" * 64,
                "execution_id": "exec",
                "definition_hashes": {},
                "controls": {"environment": {"TOKEN": canary}},
            },
            "configuration": {
                "skill": "bro",
                "suite": "smoke",
                "fixed": ["task", canary],
                "routes": {"codex": {"model": "model", "prompt": canary}},
            },
            "summary": {"runs": 1, "notes": {"response": canary}},
            "results": [{
                "id": "case-1",
                "valid": True,
                "accepted": False,
                "blocking_failures": [canary],
                "failure_kind": canary,
                "state": canary,
                "outcome": canary,
                "source": {"content": canary},
                "usage": {"input_tokens": 1, "environment": canary},
            }],
        }

        safe = eval_publication.safe_publication_document(document)
        serialized = json.dumps(safe, sort_keys=True)

        self.assertNotIn(canary, serialized)
        self.assertNotIn("prompt", serialized)
        self.assertNotIn("blocking_failures", safe["results"][0])
        self.assertEqual(safe["results"][0]["failure_kind"], "other")
        self.assertEqual(safe["results"][0]["state"], "other")
        self.assertEqual(safe["results"][0]["outcome"], "other")
        self.assertEqual(safe["results"][0]["usage"], {"input_tokens": 1})

    def test_publication_content_hash_detects_changed_safe_results(self) -> None:
        document = {
            "evaluation_identity": {"key": "1" * 64},
            "configuration": {"skill": "bro"},
            "results": [{"id": "case-1", "accepted": True, "valid": True}],
        }
        changed = json.loads(json.dumps(document))
        changed["results"][0]["accepted"] = False

        self.assertEqual(
            eval_publication.publication_content_hash(document),
            eval_publication.publication_content_hash(document),
        )
        self.assertNotEqual(
            eval_publication.publication_content_hash(document),
            eval_publication.publication_content_hash(changed),
        )

    def test_trace_result_hash_binds_verified_outcome_fields(self) -> None:
        result = {
            "id": "case-1", "harness": "codex", "variant": "candidate",
            "mode": "conditional", "repetition": 1, "valid": True,
            "accepted": True, "assertions_passed": 2, "assertions_total": 2,
        }
        changed = dict(result, accepted=False)

        self.assertNotEqual(
            eval_publication.trace_result_hash(result),
            eval_publication.trace_result_hash(changed),
        )

    def test_hashes_bind_privacy_safe_specialized_comparator_claims(self) -> None:
        neo = {
            "id": "case-1", "harness": "codex", "variant": "candidate",
            "mode": "end-to-end", "repetition": 1, "valid": True,
            "accepted": True, "assertions_passed": 3, "assertions_total": 3,
            "skill_enabled": True, "returncode": 0, "timed_out": False,
            "deterministic": {"pass": True},
            "steps": [{"kind": "model"}], "duration_seconds": 1.0,
        }
        neo_changed = json.loads(json.dumps(neo))
        neo_changed["steps"].append({"kind": "model"})
        skill_development = {
            "id": "case-2", "harness": "codex", "variant": "candidate",
            "mode": "end-to-end", "repetition": 1, "valid": True,
            "accepted": True, "assertions_passed": 2, "assertions_total": 2,
            "score": 1.0, "blocking_failures": [],
        }
        skill_changed = json.loads(json.dumps(skill_development))
        skill_changed["blocking_failures"] = ["raw private blocker"]

        self.assertNotEqual(
            eval_publication.trace_result_hash(neo),
            eval_publication.trace_result_hash(neo_changed),
        )
        self.assertNotEqual(
            eval_publication.publication_content_hash({"results": [neo]}),
            eval_publication.publication_content_hash({"results": [neo_changed]}),
        )
        self.assertNotEqual(
            eval_publication.publication_content_hash({"results": [skill_development]}),
            eval_publication.publication_content_hash({"results": [skill_changed]}),
        )
        safe = eval_publication.safe_publication_document(
            {"results": [skill_changed]}
        )
        self.assertNotIn("raw private blocker", json.dumps(safe))
        self.assertEqual(
            safe["results"][0]["comparison_claim"]["blocking_failure_count"], 1
        )

    def test_specialized_publication_minimization_is_idempotent(self) -> None:
        document = {
            "evaluation_identity": {"key": "1" * 64},
            "results": [
                {
                    "id": "neo", "harness": "codex", "variant": "candidate",
                    "mode": "end-to-end", "repetition": 1, "valid": True,
                    "accepted": True, "assertions_passed": 1,
                    "assertions_total": 1, "skill_enabled": True,
                    "returncode": 0, "timed_out": False,
                    "deterministic": {"pass": True},
                    "steps": [{"kind": "model"}], "duration_seconds": 1.0,
                    "score": 1.0,
                },
                {
                    "id": "skill-development", "harness": "codex",
                    "variant": "candidate", "mode": "end-to-end",
                    "repetition": 1, "valid": True, "accepted": False,
                    "assertions_passed": 1, "assertions_total": 2,
                    "score": 0.5, "blocking_failures": ["private blocker"],
                },
            ],
        }

        once = eval_publication.safe_publication_document(document)
        twice = eval_publication.safe_publication_document(once)

        self.assertEqual(once, twice)
        self.assertEqual(
            eval_publication.publication_content_hash(document),
            eval_publication.publication_content_hash(once),
        )
        for original, minimized in zip(document["results"], once["results"]):
            self.assertEqual(
                eval_publication.trace_result_hash(original),
                eval_publication.trace_result_hash(minimized),
            )

    def test_normalized_claim_rejects_conflicting_raw_comparator_fields(self) -> None:
        neo = {
            "id": "neo", "harness": "codex", "variant": "candidate",
            "mode": "end-to-end", "repetition": 1, "valid": True,
            "accepted": True, "assertions_passed": 1, "assertions_total": 1,
            "skill_enabled": True, "returncode": 0, "timed_out": False,
            "deterministic": {"pass": True}, "steps": [{"kind": "model"}],
            "duration_seconds": 1.0,
        }
        neo["comparison_claim"] = eval_publication.comparison_claim(neo)
        neo["deterministic"]["pass"] = False
        skill_development = {
            "id": "skill-development", "harness": "codex",
            "variant": "candidate", "mode": "end-to-end", "repetition": 1,
            "valid": True, "accepted": True, "assertions_passed": 2,
            "assertions_total": 2, "score": 1.0, "blocking_failures": [],
        }
        skill_development["comparison_claim"] = eval_publication.comparison_claim(
            skill_development
        )
        skill_development["blocking_failures"] = ["private blocker"]

        with self.assertRaisesRegex(ValueError, "comparison claim conflicts"):
            eval_publication.trace_result_hash(neo)
        with self.assertRaisesRegex(ValueError, "comparison claim conflicts"):
            eval_publication.publication_content_hash(
                {"results": [skill_development]}
            )

    def test_raw_blocker_list_cannot_be_overridden_by_redundant_count(self) -> None:
        clear = {
            "id": "case", "harness": "codex", "variant": "candidate",
            "mode": "end-to-end", "repetition": 1, "valid": True,
            "accepted": True, "assertions_passed": 2, "assertions_total": 2,
            "score": 1.0, "blocking_failures": [], "blocking_failure_count": 0,
        }
        blocked = json.loads(json.dumps(clear))
        blocked["blocking_failures"] = ["private blocker"]

        self.assertNotEqual(
            eval_publication.trace_result_hash(clear),
            eval_publication.trace_result_hash(blocked),
        )
        self.assertNotEqual(
            eval_publication.publication_content_hash({"results": [clear]}),
            eval_publication.publication_content_hash({"results": [blocked]}),
        )

    def test_publisher_rejects_trace_bound_to_different_result_content(self) -> None:
        result = {
            "id": "case-1", "harness": "codex", "variant": "candidate",
            "mode": "conditional", "repetition": 1, "valid": True,
            "accepted": True, "assertions_passed": 2, "assertions_total": 2,
        }
        root = mock.Mock(
            parent_id=None,
            attributes={
                "app.agent.eval.identity": "identity",
                "app.agent.eval.result_hash": eval_publication.trace_result_hash(
                    dict(result, accepted=False)
                ),
            },
        )
        trace = mock.Mock()
        trace.data.spans = [root]

        with self.assertRaisesRegex(RuntimeError, "result content"):
            publish_evals.validate_trace_binding(trace, result, "identity")

    def test_missing_incumbent_trace_binds_the_classified_invalid_result(self) -> None:
        eval_dir = ROOT / "skills" / "code-review" / "evals"
        case = json.loads((eval_dir / "cases.json").read_text())[0]
        route = json.loads((eval_dir / "routes.json").read_text())["harnesses"]["codex"]

        result = eval_runtime.run_once(
            eval_dir, case, "codex", "incumbent", "conditional", route, 1, 1,
            offline=True, evaluation_key="6" * 64,
        )

        spans = result["publication_trace"]["resourceSpans"][0]["scopeSpans"][0]["spans"]
        root = next(span for span in spans if "parentSpanId" not in span)
        attributes = {
            item["key"]: next(iter(item["value"].values()))
            for item in root["attributes"]
        }
        self.assertEqual(
            attributes["app.agent.eval.result_hash"],
            eval_publication.trace_result_hash(result),
        )

    def test_existing_summary_rejects_same_identity_with_changed_content(self) -> None:
        run = mock.Mock()
        run.data.tags = {
            "app.agent.eval.definition_hash": "d" * 64,
            "app.agent.eval.content_hash": "a" * 64,
        }

        with self.assertRaisesRegex(RuntimeError, "content hash conflict"):
            publish_evals.validate_existing_summary(
                [run], "identity", "d" * 64, "b" * 64
            )

    def test_existing_summary_is_incomplete_until_last_marker(self) -> None:
        run = mock.Mock()
        run.info.run_id = "run-1"
        run.data.tags = {
            "app.agent.eval.definition_hash": "d" * 64,
            "app.agent.eval.content_hash": "c" * 64,
            "app.agent.eval.summary_complete": "false",
        }

        self.assertEqual(
            publish_evals.validate_existing_summary(
                [run], "identity", "d" * 64, "c" * 64
            ),
            ("run-1", False),
        )
        run.data.tags["app.agent.eval.summary_complete"] = "true"
        self.assertEqual(
            publish_evals.validate_existing_summary(
                [run], "identity", "d" * 64, "c" * 64
            ),
            ("run-1", True),
        )

    def test_summary_complete_marker_is_written_only_after_metrics_and_artifact(self) -> None:
        mlflow = mock.MagicMock()
        mlflow.start_run.return_value.__enter__.return_value = mock.Mock()
        document = {"results": [{"valid": True, "accepted": True}]}

        publish_evals.write_summary_payload(mlflow, "run-1", document, 1)

        self.assertEqual(
            [call[0] for call in mlflow.method_calls],
            ["start_run", "log_metrics", "log_dict", "set_tag"],
        )
        mlflow.reset_mock()
        mlflow.start_run.return_value.__enter__.return_value = mock.Mock()
        mlflow.log_dict.side_effect = RuntimeError("artifact failed")
        with self.assertRaisesRegex(RuntimeError, "artifact failed"):
            publish_evals.write_summary_payload(mlflow, "run-1", document, 1)
        mlflow.set_tag.assert_not_called()

    def test_existing_assessment_rejects_changed_content_without_summary(self) -> None:
        assessment = mock.Mock()
        assessment.name = "verified_task_outcome"
        assessment.metadata = {
            "app.agent.eval.identity": "identity",
            "app.agent.eval.content_hash": "a" * 64,
        }

        with self.assertRaisesRegex(RuntimeError, "assessment content hash conflict"):
            publish_evals.validate_existing_assessments(
                [assessment], "identity", "b" * 64
            )

    def test_skill_development_document_preserves_evaluation_identity(self) -> None:
        runner = ROOT / "skills" / "skill-development" / "evals" / "run-evals.py"
        spec = importlib.util.spec_from_file_location("skill_development_eval_runner", runner)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)
        identity = {"key": "3" * 64, "execution_id": "execution"}

        document = module.build_evaluation_document(
            {"skill": "skill-development"}, identity, []
        )

        self.assertEqual(document["evaluation_identity"], identity)

    def test_skill_development_trace_binds_final_comparator_claim(self) -> None:
        runner = ROOT / "skills" / "skill-development" / "evals" / "run-evals.py"
        spec = importlib.util.spec_from_file_location(
            "skill_development_eval_runner_trace", runner
        )
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)

        trace, _ = module.export_evaluation_trace(
            case={"id": "case-1", "risk": "normal"},
            harness="codex", variant="candidate", repetition=1,
            skill_name="skill-development", skill_hash="a" * 64,
            prompt="metadata-only prompt", started_ns=1, ended_ns=2,
            evaluation_ended_ns=3, state="task_failure", accepted=False,
            score=0.5, blocking_failure_count=1, passed=1, total=2,
            usage={}, events=[], offline=True, evaluation_key="3" * 64,
        )
        spans = trace["resourceSpans"][0]["scopeSpans"][0]["spans"]
        root = next(span for span in spans if "parentSpanId" not in span)
        attributes = {
            item["key"]: next(iter(item["value"].values()))
            for item in root["attributes"]
        }
        result = {
            "id": "case-1", "harness": "codex", "variant": "candidate",
            "mode": "end-to-end", "repetition": 1, "valid": True,
            "accepted": False, "assertions_passed": 1, "assertions_total": 2,
            "score": 0.5, "blocking_failures": ["private blocker"],
        }

        self.assertEqual(
            attributes["app.agent.eval.result_hash"],
            eval_publication.trace_result_hash(result),
        )

    def test_neo_full_cost_acknowledgement_is_forwarded_explicitly(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            args = eval_cli.build_parser().parse_args([
                "run", "neo", "--output", str(Path(temporary) / "candidate.json"),
                "--suite", "full", "--ack-full-cost",
            ])
            completed = mock.Mock(returncode=0)
            with mock.patch.object(eval_cli.subprocess, "run", return_value=completed) as runner:
                result = eval_cli.run_specialized(args, "neo")

        self.assertEqual(result, 0)
        self.assertIn("--ack-full-cost", runner.call_args.args[0])

    def test_neo_suite_mapping_records_the_suite_actually_executed(self) -> None:
        self.assertEqual(eval_cli.specialized_suite("neo", "development"), "smoke")
        self.assertEqual(eval_cli.specialized_suite("neo", "held-out"), "full")
        self.assertEqual(eval_cli.specialized_suite("neo", "full"), "full")
        self.assertEqual(
            eval_cli.specialized_suite("skill-development", "development"),
            "development",
        )
        self.assertEqual(eval_cli.specialized_repetitions("neo", "smoke", None), 3)
        self.assertEqual(eval_cli.specialized_repetitions("neo", "full", None), 5)
        self.assertEqual(
            eval_cli.specialized_repetitions("skill-development", "smoke", None),
            1,
        )
        self.assertEqual(
            eval_cli.specialized_repetitions("skill-development", "full", None),
            5,
        )
        self.assertEqual(eval_cli.specialized_repetitions("neo", "smoke", 7), 7)

    def test_neo_offline_results_get_retryable_common_evidence_without_export(self) -> None:
        document = {"results": [{
            "case_id": "ambiguous-brownfield-feature",
            "harness": "codex",
            "variant": "candidate",
            "repetition": 1,
            "duration_seconds": 0.1,
            "returncode": 0,
            "deterministic": {"pass": True, "checks": {"process_healthy": True}},
            "model": "gpt-5.6-terra",
            "effort": "low",
        }]}
        identity = {"key": "4" * 64}

        with mock.patch.object(task_trace, "export_task_trace") as exporter:
            eval_cli.attach_neo_evidence(document, identity, offline=True)

        result = document["results"][0]
        exporter.assert_not_called()
        self.assertTrue(result["valid"])
        self.assertTrue(result["accepted"])
        self.assertEqual(result["evidence_state"], "pending")
        self.assertEqual(result["telemetry"]["status"], "disabled")
        self.assertIn("publication_trace", result)
        self.assertTrue(result["mlflow_trace_id"].startswith("tr-"))
        spans = result["publication_trace"]["resourceSpans"][0]["scopeSpans"][0]["spans"]
        root = next(span for span in spans if "parentSpanId" not in span)
        attributes = {
            item["key"]: next(iter(item["value"].values()))
            for item in root["attributes"]
        }
        self.assertEqual(
            attributes["app.agent.eval.result_hash"],
            eval_publication.trace_result_hash(result),
        )

    def test_matrix_validation_rejects_missing_duplicate_and_extra_runs(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        configuration = {
            "suite": "smoke",
            "repetitions": 1,
            "harnesses": ["codex"],
            "variants": ["candidate"],
            "modes": ["conditional"],
        }
        case_id = json.loads((eval_dir / "cases.json").read_text())[0]["id"]
        run = {
            "id": case_id,
            "harness": "codex",
            "variant": "candidate",
            "mode": "conditional",
            "repetition": 1,
            "valid": True,
        }

        complete = eval_validation.validate_result_matrix(
            eval_dir, "generic", {"configuration": configuration, "results": [run]}
        )
        duplicate = eval_validation.validate_result_matrix(
            eval_dir, "generic", {"configuration": configuration, "results": [run, run]}
        )
        extra = dict(run, id="unregistered-case")
        malformed = eval_validation.validate_result_matrix(
            eval_dir, "generic", {"configuration": configuration, "results": [extra]}
        )

        self.assertTrue(complete["valid"])
        self.assertFalse(duplicate["valid"])
        self.assertEqual(duplicate["duplicates"], 1)
        self.assertFalse(malformed["valid"])
        self.assertEqual(malformed["missing"], 1)
        self.assertEqual(malformed["extra"], 1)

    def test_comparison_binds_to_candidate_identity_and_safe_content(self) -> None:
        candidate = {
            "evaluation_identity": {"key": "2" * 64},
            "configuration": {"skill": "bro"},
            "results": [],
        }
        expected_hash = eval_publication.publication_content_hash(candidate)

        comparison = eval_compare.compare(candidate)

        self.assertEqual(comparison["candidate_identity"], "2" * 64)
        self.assertEqual(comparison["candidate_content_hash"], expected_hash)

    def test_release_provenance_recomputes_current_identity_and_binds_publication(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        configuration = {
            "skill": "bro", "suite": "smoke", "repetitions": 1,
            "harnesses": ["codex"], "variants": ["candidate"],
            "modes": ["conditional"],
        }
        identity = eval_identity.build_identity("bro", eval_dir, configuration)
        candidate = {
            "evaluation_identity": identity,
            "configuration": configuration,
            "results": [],
        }
        content_hash = eval_publication.publication_content_hash(candidate)
        comparison = {
            "candidate_identity": identity["key"],
            "candidate_content_hash": content_hash,
        }
        publication = {
            "evaluation_identity": identity["key"],
            "content_hash": content_hash,
        }

        eval_cli.validate_release_provenance(
            "bro", candidate, comparison, publication
        )
        publication["content_hash"] = "0" * 64
        with self.assertRaisesRegex(eval_cli.EvaluationError, "publication.*content"):
            eval_cli.validate_release_provenance(
                "bro", candidate, comparison, publication
            )
        publication["content_hash"] = content_hash
        candidate["evaluation_identity"]["evaluator_hash"] = "0" * 64
        with self.assertRaisesRegex(eval_cli.EvaluationError, "current package"):
            eval_cli.validate_release_provenance(
                "bro", candidate, comparison, publication
            )

    def test_protection_validates_identity_and_content_before_mutation(self) -> None:
        run = mock.Mock()
        run.data.tags = {
            "app.agent.eval.identity": "identity",
            "app.agent.eval.content_hash": "a" * 64,
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
        }

        protect_evals.validate_run_binding(run, "identity", "a" * 64)
        with self.assertRaisesRegex(RuntimeError, "content"):
            protect_evals.validate_run_binding(run, "identity", "b" * 64)
        run.data.tags["app.agent.eval.content_hash"] = "a" * 64
        run.data.tags["app.agent.eval.summary_complete"] = "false"
        with self.assertRaisesRegex(RuntimeError, "not complete"):
            protect_evals.validate_run_binding(run, "identity", "a" * 64)

    def test_protection_preflights_supersession_before_any_tag_mutation(self) -> None:
        run = mock.Mock()
        run.data.tags = {
            "app.agent.eval.identity": "identity",
            "app.agent.eval.content_hash": "a" * 64,
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
        }
        client = mock.Mock()
        client.get_run.return_value = run
        client.get_experiment_by_name.return_value = mock.Mock(experiment_id="eval")
        client.search_runs.return_value = []

        with self.assertRaisesRegex(RuntimeError, "superseded identity"):
            protect_evals.apply_protection(
                client=client,
                experiment="skill-evaluations",
                run_id="new-run",
                identity="identity",
                content_hash="a" * 64,
                decision="adopt",
                supersedes="old-identity",
            )

        client.set_tag.assert_not_called()

        old_run = mock.Mock(info=mock.Mock(run_id="old-run"))
        old_run.data.tags = {
            "app.agent.eval.identity": "old-identity",
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
            "app.agent.eval.protection": "protected",
            "app.agent.eval.owner_decision": "adopt",
            "app.agent.eval.superseded_at": "",
            "app.agent.eval.grace_days": "365",
        }
        client.search_runs.return_value = [old_run]
        protect_evals.apply_protection(
            client=client,
            experiment="skill-evaluations",
            run_id="new-run",
            identity="identity",
            content_hash="a" * 64,
            decision="adopt",
            supersedes="old-identity",
        )
        self.assertEqual(
            [(call.args[0], call.args[1]) for call in client.set_tag.call_args_list],
            [
                ("new-run", "app.agent.eval.protection"),
                ("new-run", "app.agent.eval.grace_days"),
                ("old-run", "app.agent.eval.superseded_by"),
                ("old-run", "app.agent.eval.superseded_at"),
                ("new-run", "app.agent.eval.owner_decision"),
            ],
        )

    def test_protection_rejects_self_or_cross_skill_supersession_before_mutation(self) -> None:
        new_run = mock.Mock()
        new_run.data.tags = {
            "app.agent.eval.identity": "new-identity",
            "app.agent.eval.content_hash": "a" * 64,
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
        }
        old_run = mock.Mock(info=mock.Mock(run_id="old-run"))
        old_run.data.tags = {
            "app.agent.eval.identity": "old-identity",
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "research",
            "app.agent.eval.protection": "protected",
            "app.agent.eval.owner_decision": "adopt",
            "app.agent.eval.superseded_at": "",
            "app.agent.eval.grace_days": "365",
        }
        client = mock.Mock()
        client.get_run.return_value = new_run
        client.get_experiment_by_name.return_value = mock.Mock(experiment_id="eval")
        client.search_runs.return_value = [old_run]

        with self.assertRaisesRegex(RuntimeError, "different release lineage"):
            protect_evals.apply_protection(
                client=client, experiment="skill-evaluations", run_id="new-run",
                identity="new-identity", content_hash="a" * 64,
                decision="adopt", supersedes="old-identity",
            )
        with self.assertRaisesRegex(RuntimeError, "cannot supersede itself"):
            protect_evals.apply_protection(
                client=client, experiment="skill-evaluations", run_id="new-run",
                identity="new-identity", content_hash="a" * 64,
                decision="adopt", supersedes="new-identity",
            )
        client.set_tag.assert_not_called()

    def test_protection_rejects_reactivation_of_a_superseded_current_run(self) -> None:
        run = mock.Mock()
        run.data.tags = {
            "app.agent.eval.identity": "identity",
            "app.agent.eval.content_hash": "a" * 64,
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
            "app.agent.eval.superseded_at": "2025-01-01T00:00:00+00:00",
        }
        client = mock.Mock()
        client.get_run.return_value = run

        with self.assertRaisesRegex(RuntimeError, "already superseded"):
            protect_evals.apply_protection(
                client=client, experiment="skill-evaluations", run_id="run",
                identity="identity", content_hash="a" * 64,
                decision="adopt", supersedes=None,
            )
        client.set_tag.assert_not_called()

    def test_interrupted_supersession_retries_to_the_same_logical_transaction(self) -> None:
        new_run = mock.Mock(info=mock.Mock(run_id="new-run"))
        new_run.data.tags = {
            "app.agent.eval.identity": "new-identity",
            "app.agent.eval.content_hash": "a" * 64,
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
            "app.agent.eval.protection": "ordinary",
            "app.agent.eval.owner_decision": "defer",
            "app.agent.eval.superseded_at": "",
            "app.agent.eval.superseded_by": "",
            "app.agent.eval.grace_days": "365",
        }
        old_run = mock.Mock(info=mock.Mock(run_id="old-run"))
        old_run.data.tags = {
            "app.agent.eval.identity": "old-identity",
            "app.agent.eval.summary_complete": "true",
            "app.agent.eval.skill": "bro",
            "app.agent.eval.protection": "protected",
            "app.agent.eval.owner_decision": "adopt",
            "app.agent.eval.superseded_at": "",
            "app.agent.eval.superseded_by": "",
            "app.agent.eval.grace_days": "365",
        }
        client = mock.Mock()
        client.get_run.return_value = new_run
        client.get_experiment_by_name.return_value = mock.Mock(experiment_id="eval")
        client.search_runs.return_value = [old_run]
        fail_owner_once = True

        def set_tag(run_id, key, value):
            nonlocal fail_owner_once
            if key == "app.agent.eval.owner_decision" and fail_owner_once:
                fail_owner_once = False
                raise RuntimeError("interrupted owner decision")
            target = new_run if run_id == "new-run" else old_run
            target.data.tags[key] = value

        client.set_tag.side_effect = set_tag
        arguments = {
            "client": client, "experiment": "skill-evaluations",
            "run_id": "new-run", "identity": "new-identity",
            "content_hash": "a" * 64, "decision": "adopt",
            "supersedes": "old-identity",
        }

        with self.assertRaisesRegex(RuntimeError, "interrupted owner decision"):
            protect_evals.apply_protection(**arguments)
        self.assertEqual(old_run.data.tags["app.agent.eval.superseded_by"], "new-identity")
        self.assertTrue(old_run.data.tags["app.agent.eval.superseded_at"])
        self.assertEqual(new_run.data.tags["app.agent.eval.owner_decision"], "defer")

        protect_evals.apply_protection(**arguments)

        self.assertEqual(new_run.data.tags["app.agent.eval.owner_decision"], "adopt")

    def test_publication_retries_have_one_deterministic_logical_object_per_boundary(self) -> None:
        result = {
            "id": "case-1",
            "harness": "codex",
            "variant": "candidate",
            "mode": "conditional",
            "repetition": 1,
            "mlflow_trace_id": "tr-abc",
            "telemetry": {"status": "exported"},
        }
        document = {
            "evaluation_identity": {"key": "b" * 64},
            "configuration": {"skill": "bro"},
            "results": [result, dict(result)],
        }

        objects = eval_publication.logical_objects(document)

        self.assertEqual(objects["summary"], ["b" * 64])
        self.assertEqual(len(objects["dataset_records"]), 1)
        self.assertEqual(len(objects["assessments"]), 2)
        self.assertEqual(objects, eval_publication.logical_objects(document))

    def test_retry_spool_keeps_a_redacted_otlp_payload_without_raw_content(self) -> None:
        canary = "privacy-canary-spool-secret"
        document = {
            "evaluation_identity": {"key": "e" * 64},
            "configuration": {"skill": "bro"},
            "results": [{
                "id": "case-1",
                "mlflow_trace_id": "tr-abc",
                "trace_id": "abc",
                "evidence_state": "pending",
                "publication_trace": {"resourceSpans": [{
                    "resource": {"attributes": [
                        {"key": "app.agent.eval.case_id", "value": {"stringValue": "case-1"}},
                        {"key": "gen_ai.input.messages", "value": {"stringValue": canary}},
                    ]},
                    "scopeSpans": [{"scope": {"name": "test"}, "spans": [{
                        "traceId": "abc",
                        "spanId": "def",
                        "name": "agent.task",
                        "attributes": [
                            {"key": "app.agent.outcome.status", "value": {"stringValue": "accepted"}},
                            {"key": "tool.payload", "value": {"stringValue": canary}},
                        ],
                    }]}],
                }]},
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            path = eval_publication.write_spool(document, Path(temporary))
            spooled = json.loads(path.read_text(encoding="utf-8"))

        serialized = json.dumps(spooled, sort_keys=True)
        self.assertNotIn(canary, serialized)
        self.assertEqual(spooled["retry_traces"][0]["mlflow_trace_id"], "tr-abc")
        self.assertIn("app.agent.outcome.status", serialized)
        self.assertNotIn("gen_ai.input.messages", serialized)
        self.assertNotIn("tool.payload", serialized)

    def test_mixed_retry_spool_preserves_already_exported_results(self) -> None:
        exported = {
            "id": "case-exported",
            "mlflow_trace_id": "tr-exported",
            "trace_id": "exported",
            "valid": True,
            "telemetry": {"status": "exported"},
            "evidence_state": "published",
        }
        pending = {
            "id": "case-pending",
            "mlflow_trace_id": "tr-pending",
            "trace_id": "pending",
            "valid": True,
            "telemetry": {"status": "export_failed"},
            "evidence_state": "pending",
            "publication_trace": {"resourceSpans": []},
        }
        document = {
            "evaluation_identity": {"key": "9" * 64},
            "configuration": {"skill": "bro"},
            "results": [exported, pending],
        }

        with tempfile.TemporaryDirectory() as temporary:
            path = eval_publication.write_spool(document, Path(temporary))
            spooled = json.loads(path.read_text(encoding="utf-8"))

        self.assertEqual(spooled["results"][0]["telemetry"]["status"], "exported")
        self.assertEqual(spooled["results"][0]["evidence_state"], "published")
        self.assertEqual(spooled["results"][1]["telemetry"]["status"], "pending")
        self.assertEqual(len(spooled["retry_traces"]), 1)
        self.assertEqual(
            eval_publication.logical_objects(spooled)["dataset_records"],
            ["9" * 64 + ":tr-exported"],
        )
        response = mock.MagicMock()
        response.__enter__.return_value.status = 200
        with mock.patch.object(
            publish_evals.urllib.request, "urlopen", return_value=response
        ):
            publish_evals.retry_pending_traces(
                spooled, "http://collector.invalid/v1/traces"
            )
        self.assertEqual(
            eval_publication.logical_objects(spooled)["dataset_records"],
            ["9" * 64 + ":tr-exported", "9" * 64 + ":tr-pending"],
        )

    def test_publication_content_hash_ignores_retryable_evidence_state(self) -> None:
        pending = {
            "evaluation_identity": {"key": "8" * 64},
            "configuration": {"skill": "bro"},
            "results": [{
                "id": "case-1", "valid": True, "evidence_state": "pending",
                "publication_failure_kind": "offline", "telemetry": {"status": "pending"},
            }],
        }
        published = json.loads(json.dumps(pending))
        published["results"][0]["evidence_state"] = "published"
        published["results"][0]["telemetry"] = {"status": "exported"}
        published["results"][0].pop("publication_failure_kind")

        self.assertEqual(
            eval_publication.publication_content_hash(pending),
            eval_publication.publication_content_hash(published),
        )

    def test_retry_command_updates_the_safe_candidate_for_release(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "spool.json"
            source.write_text(json.dumps({
                "configuration": {"skill": "bro"},
                "results": [],
            }), encoding="utf-8")
            args = eval_cli.build_parser().parse_args([
                "retry", "bro", str(source),
            ])
            completed = mock.Mock(returncode=0, stdout="{}\n", stderr="")
            with mock.patch.object(eval_cli.subprocess, "run", return_value=completed) as runner:
                result = eval_cli.publish_skill(args)

        self.assertEqual(result, 0)
        self.assertIn("--update-candidate", runner.call_args.args[0])

    def test_retry_candidate_replacement_is_safe_and_release_ready(self) -> None:
        canary = "retry-raw-content-canary"
        document = {
            "evaluation_identity": {"key": "7" * 64},
            "configuration": {"skill": "bro"},
            "results": [{
                "id": "case-1",
                "valid": True,
                "accepted": True,
                "telemetry": {"status": "exported"},
                "evidence_state": "published",
                "output": canary,
            }],
            "evidence_state": "published",
        }
        expected_hash = eval_publication.publication_content_hash(document)

        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "candidate.json"
            publish_evals.write_release_candidate(path, document)
            candidate = json.loads(path.read_text(encoding="utf-8"))

        self.assertEqual(candidate["evidence_state"], "published")
        self.assertEqual(candidate["results"][0]["telemetry"], {"status": "exported"})
        self.assertNotIn(canary, json.dumps(candidate))
        self.assertEqual(
            eval_publication.publication_content_hash(candidate), expected_hash
        )

    def test_retry_exports_spooled_trace_without_rerunning_behavior(self) -> None:
        document = {
            "results": [{
                "id": "case-1",
                "mlflow_trace_id": "tr-abc",
                "telemetry": {"status": "pending"},
                "evidence_state": "pending",
            }],
            "retry_traces": [{
                "mlflow_trace_id": "tr-abc",
                "payload": {"resourceSpans": []},
            }],
        }
        response = mock.MagicMock()
        response.__enter__.return_value.status = 200

        with mock.patch.object(publish_evals.urllib.request, "urlopen", return_value=response) as sender:
            retried = publish_evals.retry_pending_traces(
                document, "http://collector.invalid/v1/traces"
            )

        self.assertEqual(retried, ["tr-abc"])
        self.assertEqual(document["results"][0]["telemetry"]["status"], "exported")
        self.assertEqual(document["results"][0]["evidence_state"], "published")
        sender.assert_called_once()

    def test_central_cli_lists_all_skills_and_specialized_adapters(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "eval_cli.py"), "list", "--json"],
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(completed.returncode, 0, completed.stderr)
        listed = json.loads(completed.stdout)
        self.assertEqual(len(listed["skills"]), 17)
        self.assertEqual(
            {item["name"] for item in listed["skills"] if item["adapter"] != "generic"},
            {"neo", "skill-development"},
        )

    def test_registry_packages_use_central_mechanics_and_compact_manifests(self) -> None:
        registry = json.loads((ROOT / "eval_registry.json").read_text(encoding="utf-8"))
        package_names = {
            path.name for path in (ROOT / "skills").iterdir() if path.is_dir()
        }
        self.assertEqual(set(registry["skills"]), package_names)
        for name, entry in registry["skills"].items():
            eval_dir = ROOT / "skills" / name / "evals"
            manifest = json.loads((eval_dir / "release-manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["skill"], name)
            self.assertIn(manifest["owner_decision"], {"defer", "adopt", "reject", "restrict"})
            self.assertIsInstance(manifest["release_eligible"], bool)
            if entry["adapter"] == "generic":
                self.assertFalse((eval_dir / "run-evals.py").exists())
                self.assertFalse((eval_dir / "compare-evals.py").exists())
            else:
                self.assertTrue((eval_dir / "run-evals.py").is_file())
                self.assertTrue((eval_dir / "compare-evals.py").is_file())

    def test_central_validator_checks_registry_manifests_and_definition_hashes(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "eval_cli.py"), "validate", "--json"],
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        report = json.loads(completed.stdout)
        self.assertTrue(report["valid"])
        self.assertEqual(report["skills"], 17)
        self.assertEqual(report["errors"], [])

    def test_central_cli_rejects_an_unregistered_skill(self) -> None:
        completed = subprocess.run(
            [
                sys.executable,
                str(ROOT / "eval_cli.py"),
                "run",
                "missing-skill",
                "--output",
                "/tmp/unused-evaluation.json",
                "--offline",
            ],
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(completed.returncode, 2)
        self.assertIn("unknown skill", completed.stderr)

    def test_central_cli_replays_generic_results_without_a_model_call(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        case = json.loads((eval_dir / "cases.json").read_text())[0]
        source_document = {
            "configuration": {"skill": "bro"},
            "results": [{
                "id": case["id"],
                "mode": "conditional",
                "state": "succeeded",
                "output": "config/worker.toml cargo test -p worker git reset --hard would destroy work; then staging",
                "valid": True,
                "accepted": False,
                "usage": {},
                "duration_seconds": 0,
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source.json"
            target = Path(temporary) / "target.json"
            source.write_text(json.dumps(source_document), encoding="utf-8")
            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "eval_cli.py"),
                    "replay",
                    "bro",
                    str(source),
                    "--output",
                    str(target),
                    "--offline",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            replayed = json.loads(target.read_text(encoding="utf-8")) if target.exists() else {}

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertTrue(replayed["results"][0]["accepted"])
        self.assertEqual(replayed["configuration"]["skill"], "bro")
        self.assertRegex(replayed["evaluation_identity"]["key"], r"^[0-9a-f]{64}$")

    def test_central_cli_replays_specialized_results_through_explicit_adapter(self) -> None:
        source_document = {
            "configuration": {"skill": "neo", "suite": "smoke"},
            "results": [{
                "case_id": "ambiguous-brownfield-feature",
                "variant": "candidate",
                "deterministic": {"pass": True},
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source.json"
            target = Path(temporary) / "target.json"
            source.write_text(json.dumps(source_document), encoding="utf-8")
            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "eval_cli.py"),
                    "replay",
                    "neo",
                    str(source),
                    "--output",
                    str(target),
                    "--offline",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            replayed = json.loads(target.read_text(encoding="utf-8")) if target.exists() else {}

        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertEqual(replayed["adapter_validation"], {"adapter": "neo", "valid": True})
        self.assertEqual(replayed["evidence_state"], "pending")

    def test_every_registered_skill_has_a_no_model_replay_path(self) -> None:
        registry = json.loads((ROOT / "eval_registry.json").read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temporary:
            temporary_root = Path(temporary)
            for name, entry in registry["skills"].items():
                if entry["adapter"] == "generic":
                    case = json.loads(
                        (ROOT / "skills" / name / "evals" / "cases.json").read_text(encoding="utf-8")
                    )[0]
                    result = {
                        "id": case["id"],
                        "mode": "conditional",
                        "state": "succeeded",
                        "output": "",
                        "valid": True,
                        "accepted": False,
                        "usage": {},
                        "duration_seconds": 0,
                    }
                elif entry["adapter"] == "neo":
                    result = {"case_id": "replay", "deterministic": {"pass": True}}
                else:
                    result = {"id": "replay", "valid": True, "accepted": True, "score": 1.0}
                source = temporary_root / f"{name}-source.json"
                output = temporary_root / f"{name}-output.json"
                source.write_text(json.dumps({
                    "configuration": {"skill": name, "suite": "smoke"},
                    "results": [result],
                }), encoding="utf-8")
                completed = subprocess.run(
                    [
                        sys.executable,
                        str(ROOT / "eval_cli.py"),
                        "replay",
                        name,
                        str(source),
                        "--output",
                        str(output),
                        "--offline",
                    ],
                    check=False,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(completed.returncode, 0, f"{name}: {completed.stderr}")
                self.assertTrue(output.is_file(), name)

    def test_central_compare_fails_closed_when_incumbent_evidence_is_missing(self) -> None:
        candidate = {
            "configuration": {"skill": "bro"},
            "results": [{
                "id": "case-1",
                "harness": "codex",
                "variant": "candidate",
                "mode": "conditional",
                "repetition": 1,
                "valid": True,
                "accepted": True,
                "usage": {},
                "duration_seconds": 0,
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "candidate.json"
            output = Path(temporary) / "comparison.json"
            source.write_text(json.dumps(candidate), encoding="utf-8")
            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "eval_cli.py"),
                    "compare",
                    "bro",
                    str(source),
                    "--output",
                    str(output),
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            comparison = json.loads(output.read_text(encoding="utf-8")) if output.exists() else {}

        self.assertEqual(completed.returncode, 1, completed.stderr)
        self.assertEqual(comparison["decision"], "defer")
        self.assertFalse(comparison["checks"]["incumbent_evidence_present"])

    def test_strict_release_fails_closed_without_joined_publication_evidence(self) -> None:
        identity = {"key": "c" * 64, "definition_hashes": {"cases.json": "d" * 64}}
        candidate = {
            "configuration": {"skill": "bro"},
            "evaluation_identity": identity,
            "evidence_state": "pending",
            "results": [{"valid": True, "accepted": True}],
        }
        comparison = {
            "decision": "pass",
            "release_eligible": True,
            "checks": {"complete_valid_matrix": True},
        }
        publication = {"evidence_state": "pending", "evaluation_identity": "c" * 64}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name, value in (
                ("candidate.json", candidate),
                ("comparison.json", comparison),
                ("publication.json", publication),
            ):
                (root / name).write_text(json.dumps(value), encoding="utf-8")
            manifest = root / "release-manifest.json"
            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "eval_cli.py"),
                    "release",
                    "bro",
                    "--candidate",
                    str(root / "candidate.json"),
                    "--comparison",
                    str(root / "comparison.json"),
                    "--publication",
                    str(root / "publication.json"),
                    "--owner",
                    "release-owner",
                    "--decision",
                    "adopt",
                    "--manifest",
                    str(manifest),
                ],
                check=False,
                capture_output=True,
                text=True,
            )

        self.assertEqual(completed.returncode, 2)
        self.assertIn("published evidence", completed.stderr)
        self.assertFalse(manifest.exists())

    def test_task_trace_has_one_root_and_recognized_session(self) -> None:
        trace = task_trace.build_task_trace(
            harness="codex",
            session_id="session-123",
            task_id="task-123",
            started_ns=100,
            ended_ns=200,
            attributes={"app.agent.eval.case_id": "case-1"},
            children=[],
        )

        spans = trace["resourceSpans"][0]["scopeSpans"][0]["spans"]
        roots = [span for span in spans if "parentSpanId" not in span]
        self.assertEqual([span["name"] for span in roots], ["agent.task"])
        root_attributes = {
            item["key"]: next(iter(item["value"].values()))
            for item in roots[0]["attributes"]
        }
        self.assertEqual(root_attributes["session.id"], "session-123")
        self.assertEqual(trace["trace_id"], roots[0]["traceId"])
        self.assertEqual(trace["mlflow_trace_id"], f"tr-{roots[0]['traceId']}")

    def test_evaluation_metadata_is_joinable_and_versioned(self) -> None:
        metadata = task_trace.evaluation_attributes(
            case_id="case-1",
            variant="candidate",
            repetition=2,
            mode="conditional",
            skill_name="bro",
            skill_hash="abc123",
            skill_source="candidate",
            repository_hash="repo-hash",
            base_revision="deadbeef",
            model_requested="model-a",
            model_returned="model-b",
            effort="medium",
            prompt_version="prompt-hash",
            tool_version="tool-hash",
            evaluator_version="eval-hash",
            risk="normal",
            outcome="accepted",
            verifier="assertion-scorer@eval-hash",
        )

        self.assertEqual(metadata["app.agent.eval.case_id"], "case-1")
        self.assertEqual(metadata["app.agent.eval.variant"], "candidate")
        self.assertEqual(metadata["app.agent.eval.repetition"], 2)
        self.assertEqual(metadata["app.agent.skill.package_hash"], "abc123")
        self.assertEqual(metadata["app.agent.repository.base_revision"], "deadbeef")
        self.assertEqual(metadata["app.agent.model.requested"], "model-a")
        self.assertEqual(metadata["app.agent.model.returned"], "model-b")
        self.assertEqual(metadata["app.agent.outcome.status"], "accepted")
        self.assertEqual(metadata["app.agent.outcome.verifier"], "assertion-scorer@eval-hash")
        self.assertEqual(metadata["app.agent.cost.status"], "not_observed")

    def test_claude_parser_retains_returned_model_and_cost(self) -> None:
        event = {
            "type": "result",
            "result": "done",
            "usage": {"input_tokens": 2, "output_tokens": 1},
            "total_cost_usd": 0.012,
            "modelUsage": {
                "provider-versioned-id": {"canonicalModel": "claude-haiku-4-5"},
                "claude-haiku-4-5": {"canonicalModel": "claude-haiku-4-5"},
            },
        }

        output, usage, events = eval_runtime.parse_output("claude", json.dumps(event))

        self.assertEqual(output, "done")
        self.assertEqual(usage["total_cost_usd"], 0.012)
        self.assertEqual(eval_runtime.returned_model_for(events), "claude-haiku-4-5")

    def test_run_result_returns_exported_mlflow_trace_id(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        case = json.loads((eval_dir / "cases.json").read_text())[0]
        route = json.loads((eval_dir / "routes.json").read_text())["harnesses"]["codex"]
        stdout = "\n".join([
            json.dumps({
                "type": "item.completed",
                "item": {"type": "agent_message", "text": "plain response"},
            }),
            json.dumps({"type": "turn.completed", "usage": {"input_tokens": 4, "output_tokens": 2}}),
        ])
        completed = eval_runtime.subprocess.CompletedProcess([], 0, stdout, "")

        with mock.patch.object(eval_runtime.subprocess, "run", return_value=completed), mock.patch.object(
            task_trace, "export_task_trace", return_value={"status": "exported"}
        ) as exporter:
            result = eval_runtime.run_once(
                eval_dir, case, "codex", "candidate", "conditional", route, 1, 30
            )

        self.assertRegex(result["trace_id"], r"^[0-9a-f]{32}$")
        self.assertEqual(result["mlflow_trace_id"], f"tr-{result['trace_id']}")
        self.assertEqual(result["telemetry"]["status"], "exported")
        exported = exporter.call_args.args[0]
        spans = exported["resourceSpans"][0]["scopeSpans"][0]["spans"]
        self.assertEqual(sum(span["name"] == "agent.task" for span in spans), 1)
        self.assertIn("evaluator.run", {span["name"] for span in spans})

    def test_mlflow_dataset_records_link_only_exported_traces(self) -> None:
        records = publish_evals.dataset_records({"results": [
            {
                "id": "case-1", "variant": "candidate", "harness": "codex",
                "repetition": 1, "skill_hash": "abc", "accepted": True,
                "mlflow_trace_id": "tr-a", "telemetry": {"status": "exported"},
            },
            {
                "id": "case-2", "mlflow_trace_id": "tr-b",
                "telemetry": {"status": "export_failed"},
            },
        ]})
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["source"]["source_type"], "TRACE")
        self.assertEqual(records[0]["source"]["source_data"]["trace_id"], "tr-a")
        self.assertEqual(records[0]["inputs"]["case_id"], "case-1")

    def test_telemetry_failure_preserves_successful_behavior_and_marks_publication_pending(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        case = json.loads((eval_dir / "cases.json").read_text())[0]
        route = json.loads((eval_dir / "routes.json").read_text())["harnesses"]["codex"]
        stdout = "\n".join([
            json.dumps({
                "type": "item.completed",
                "item": {"type": "agent_message", "text": "config/worker.toml cargo test -p worker git reset --hard would destroy work; then staging"},
            }),
            json.dumps({"type": "turn.completed", "usage": {}}),
        ])
        completed = eval_runtime.subprocess.CompletedProcess([], 0, stdout, "")
        with mock.patch.object(eval_runtime.subprocess, "run", return_value=completed), mock.patch.object(
            task_trace, "export_task_trace", return_value={"status": "export_failed", "error": "URLError"}
        ):
            result = eval_runtime.run_once(
                eval_dir, case, "codex", "candidate", "conditional", route, 1, 30
            )
        self.assertEqual(result["state"], "succeeded")
        self.assertTrue(result["valid"])
        self.assertTrue(result["accepted"])
        self.assertEqual(result["evidence_state"], "pending")
        self.assertEqual(result["publication_failure_kind"], "URLError")

    def test_offline_run_skips_export_without_changing_behavior(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        case = json.loads((eval_dir / "cases.json").read_text())[0]
        route = json.loads((eval_dir / "routes.json").read_text())["harnesses"]["codex"]
        stdout = "\n".join([
            json.dumps({
                "type": "item.completed",
                "item": {"type": "agent_message", "text": "config/worker.toml cargo test -p worker git reset --hard would destroy work; then staging"},
            }),
            json.dumps({"type": "turn.completed", "usage": {}}),
        ])
        completed = eval_runtime.subprocess.CompletedProcess([], 0, stdout, "")
        with mock.patch.object(eval_runtime.subprocess, "run", return_value=completed), mock.patch.object(
            task_trace, "export_task_trace"
        ) as exporter:
            result = eval_runtime.run_once(
                eval_dir, case, "codex", "candidate", "conditional", route, 1, 30,
                offline=True,
            )

        exporter.assert_not_called()
        self.assertTrue(result["valid"])
        self.assertTrue(result["accepted"])
        self.assertEqual(result["evidence_state"], "pending")
        self.assertEqual(result["publication_failure_kind"], "offline")

    def test_assertion_scorer_supports_packaged_assertion_types(self) -> None:
        output = json.dumps({"ready": True, "mode": "safe", "items": ["first"]})
        assertions = [
            {"type": "json_truthy", "path": "ready"},
            {"type": "json_in", "path": "mode", "values": ["safe", "review"]},
            {"type": "json_equals", "path": "items.0", "value": "first"},
            {"type": "not_regex", "pattern": "secret"},
        ]
        self.assertTrue(all(item["passed"] for item in eval_runtime.assertion_results(output, assertions)))

    def test_fixed_route_is_variant_independent(self) -> None:
        routes = json.loads((ROOT / "skills" / "bro" / "evals" / "routes.json").read_text())
        self.assertEqual(routes["variants"], ["no-skill", "incumbent", "candidate"])
        self.assertIn("model", routes["fixed"])
        self.assertNotIn("variants", routes["harnesses"]["codex"])

    def test_quality_gate_counts_false_activation(self) -> None:
        results = []
        for expected, actual, risk in ((True, True, "normal"), (False, True, "side-effect")):
            results.append({
                "id": f"route-{expected}", "harness": "codex", "variant": "candidate",
                "mode": "routing", "repetition": 1, "valid": True, "accepted": expected == actual,
                "expected_activation": expected, "actual_activation": actual, "risk": risk,
            })
        outcome = eval_compare.compare({"results": results})
        self.assertEqual(outcome["routing"]["precision"], 0.5)
        self.assertEqual(outcome["routing"]["side_effect_false_activations"], 1)
        self.assertEqual(outcome["decision"], "defer")

    def test_replay_rescores_preserved_output_without_model_call(self) -> None:
        eval_dir = ROOT / "skills" / "bro" / "evals"
        case = json.loads((eval_dir / "cases.json").read_text())[0]
        record = {
            "configuration": {"skill": "bro"},
            "results": [{
                "id": case["id"], "mode": "conditional", "state": "succeeded",
                "output": "config/worker.toml cargo test -p worker git reset --hard would destroy work; then staging",
                "valid": True, "accepted": False, "usage": {}, "duration_seconds": 0,
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "source.json"
            target = Path(temporary) / "target.json"
            source.write_text(json.dumps(record))
            eval_runtime.replay(eval_dir, source, target)
            rescored = json.loads(target.read_text())["results"][0]
        self.assertTrue(rescored["accepted"])


if __name__ == "__main__":
    unittest.main()
