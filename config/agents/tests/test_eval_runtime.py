from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import eval_compare  # noqa: E402
import eval_runtime  # noqa: E402
from telemetry import publish_evals, task_trace  # noqa: E402


class EvalRuntimeTests(unittest.TestCase):
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

    def test_telemetry_failure_is_separate_from_successful_task(self) -> None:
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
        self.assertEqual(result["state"], "telemetry_failure")
        self.assertEqual(result["task_state"], "succeeded")
        self.assertFalse(result["valid"])
        self.assertFalse(result["accepted"])

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
