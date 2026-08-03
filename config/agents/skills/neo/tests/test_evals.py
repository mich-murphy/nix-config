"""Tests for Neo evaluation fixtures and workspace preparation."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


RUNNER_PATH = Path(__file__).parents[1] / "evals" / "run-evals.py"
SPEC = importlib.util.spec_from_file_location("neo_run_evals", RUNNER_PATH)
assert SPEC and SPEC.loader
RUNNER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = RUNNER
SPEC.loader.exec_module(RUNNER)

VALIDATOR_PATH = Path(__file__).parents[1] / "evals" / "validate-judges.py"
VALIDATOR_SPEC = importlib.util.spec_from_file_location(
    "neo_validate_judges", VALIDATOR_PATH
)
assert VALIDATOR_SPEC and VALIDATOR_SPEC.loader
VALIDATOR = importlib.util.module_from_spec(VALIDATOR_SPEC)
sys.modules[VALIDATOR_SPEC.name] = VALIDATOR
VALIDATOR_SPEC.loader.exec_module(VALIDATOR)

COMPARATOR_PATH = Path(__file__).parents[1] / "evals" / "compare-evals.py"
COMPARATOR_SPEC = importlib.util.spec_from_file_location(
    "neo_compare_evals", COMPARATOR_PATH
)
assert COMPARATOR_SPEC and COMPARATOR_SPEC.loader
COMPARATOR = importlib.util.module_from_spec(COMPARATOR_SPEC)
sys.modules[COMPARATOR_SPEC.name] = COMPARATOR
COMPARATOR_SPEC.loader.exec_module(COMPARATOR)


class EvalHarnessTests(unittest.TestCase):
    def test_balanced_case_set_and_single_smoke_case(self) -> None:
        full = RUNNER.load_cases("full")
        smoke = RUNNER.load_cases("smoke")
        self.assertEqual(len(full), 6)
        self.assertEqual([case["id"] for case in smoke], ["ambiguous-brownfield-feature"])
        self.assertEqual(
            {case["expected_route"] for case in full},
            {"direct", "focused", "full"},
        )

    def test_concurrency_is_bounded_at_three(self) -> None:
        self.assertEqual(RUNNER.bounded_jobs("1"), 1)
        self.assertEqual(RUNNER.bounded_jobs("3"), 3)
        with self.assertRaises(RUNNER.argparse.ArgumentTypeError):
            RUNNER.bounded_jobs("4")

    def test_stream_normalization_keeps_message_and_tool_counts(self) -> None:
        stream = "\n".join(
            (
                json.dumps(
                    {
                        "type": "item.completed",
                        "item": {
                            "type": "command_execution",
                            "status": "completed",
                        },
                    }
                ),
                json.dumps(
                    {
                        "type": "item.completed",
                        "item": {"type": "agent_message", "text": "Done."},
                    }
                ),
            )
        )
        normalized = RUNNER.normalize_stream(stream, "")
        self.assertEqual(normalized["final_message"], "Done.")
        self.assertEqual(
            normalized["tool_summary"],
            [{"name": "command_execution", "calls": 1, "failures": 0}],
        )

    def test_stream_normalization_never_falls_back_to_raw_json(self) -> None:
        stream = json.dumps({"type": "unrecognized", "payload": "x" * 1000})
        normalized = RUNNER.normalize_stream(stream, "")
        self.assertEqual(normalized["final_message"], "")

    def test_stream_normalization_caps_plain_messages(self) -> None:
        normalized = RUNNER.normalize_stream(
            "x" * (RUNNER.MAX_FINAL_MESSAGE_CHARS + 10), ""
        )
        self.assertLessEqual(
            len(normalized["final_message"]),
            RUNNER.MAX_FINAL_MESSAGE_CHARS + 100,
        )
        self.assertIn("10 characters omitted", normalized["final_message"])

    def test_confirmed_risks_prompt_starts_at_discovery(self) -> None:
        case = RUNNER.load_cases("smoke")[0]
        prompt = RUNNER.initial_prompt(case, True, "full")
        self.assertIn("Use $neo-discover", prompt)
        self.assertIn("Do not repeat router preflight", prompt)

    def test_skill_case_uses_one_model_call_and_omits_raw_streams(self) -> None:
        case = RUNNER.load_cases("smoke")[0]
        original_run_process = RUNNER.run_process
        model_calls = []

        def fake_run_process(command, workspace, timeout):
            if "neo.py" in command[1]:
                return original_run_process(command, workspace, timeout)
            model_calls.append(command)
            stdout = json.dumps(
                {
                    "type": "item.completed",
                    "item": {"type": "agent_message", "text": "Need ownership."},
                }
            )
            return (0, 0.01, stdout, "", False)

        with mock.patch.object(RUNNER, "run_process", side_effect=fake_run_process):
            result = RUNNER.run_case(
                "codex",
                case,
                skill_enabled=True,
                timeout=30,
                retain_raw=False,
            )

        self.assertEqual(len(model_calls), 1)
        self.assertEqual(
            [(step["stage"], step["kind"]) for step in result.steps],
            [("route", "local"), ("discover", "model")],
        )
        self.assertEqual(result.steps[-1]["final_message"], "Need ownership.")
        self.assertIsNone(result.raw_streams)
        self.assertFalse(
            any("stdout" in step or "stderr" in step for step in result.steps)
        )

    def test_comparator_accepts_quality_preserving_latency_reduction(self) -> None:
        baseline = json.loads(
            (RUNNER.HERE / "results" / "baseline.json").read_text(encoding="utf-8")
        )
        candidate = {
            "cases": 3,
            "deterministic_passed": 3,
            "deterministic_pass_rate": 1.0,
            "completed_mean_seconds": 100.0,
            "model_invocations": 3,
            "model_invocations_per_case": 1.0,
        }
        result = COMPARATOR.compare(baseline, candidate)
        self.assertTrue(result["pass"])

    def test_every_case_has_explicit_context_and_risks(self) -> None:
        for case in RUNNER.load_cases("full"):
            self.assertIn("approved_context", case)
            self.assertIsInstance(case["approved_risk_signals"], list)
            self.assertIn("Do not implement", case["prompt"])

    def test_judges_are_narrow_and_unvalidated(self) -> None:
        judges = json.loads(RUNNER.JUDGES.read_text(encoding="utf-8"))
        self.assertEqual(judges["status"], "unvalidated_until_human_calibration")
        self.assertEqual(len(judges["judges"]), 3)
        self.assertEqual(
            {judge["id"] for judge in judges["judges"]},
            {
                "clarification-discipline",
                "decision-clarity",
                "implementation-readiness",
            },
        )

    def test_skill_workspace_contains_portable_suite(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            RUNNER.prepare_workspace(workspace, skill_enabled=True)
            self.assertTrue((workspace / ".agents/skills/neo/SKILL.md").is_file())
            self.assertTrue((workspace / ".claude/skills/neo").is_symlink())
            self.assertEqual(
                (workspace / ".claude/skills/neo").resolve(),
                (workspace / ".agents/skills/neo").resolve(),
            )

    def test_baseline_workspace_has_no_skill_catalog(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            RUNNER.prepare_workspace(workspace, skill_enabled=False)
            self.assertFalse((workspace / ".agents").exists())

    def test_judge_calibration_requires_balanced_human_labels(self) -> None:
        rows = []
        for judge_id in {
            "clarification-discipline",
            "decision-clarity",
            "implementation-readiness",
        }:
            rows.extend(
                {"judge_id": judge_id, "human": "Pass", "predicted": "Pass"}
                for _ in range(20)
            )
            rows.extend(
                {"judge_id": judge_id, "human": "Fail", "predicted": "Fail"}
                for _ in range(20)
            )
        report = VALIDATOR.calculate(rows)
        self.assertTrue(
            all(item["target_met"] for item in report.values())
        )

    def test_unlabelled_judges_are_not_usable(self) -> None:
        report = VALIDATOR.calculate([])
        self.assertTrue(
            all(not item["minimum_usable"] for item in report.values())
        )


if __name__ == "__main__":
    unittest.main()
