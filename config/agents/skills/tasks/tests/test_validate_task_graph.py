import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate_task_graph.py"
SPEC = importlib.util.spec_from_file_location("validate_task_graph", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


HEADINGS = "\n\n".join(MODULE.REQUIRED_HEADINGS)


class TaskGraphValidationTests(unittest.TestCase):
    @staticmethod
    def read_graph(graph_path: Path) -> dict:
        return json.loads(graph_path.read_text(encoding="utf-8"))

    @staticmethod
    def write_graph(graph_path: Path, graph: dict) -> None:
        graph_path.write_text(json.dumps(graph), encoding="utf-8")

    def make_graph(self, root: Path) -> Path:
        plan = root / "approved-plan.md"
        plan.write_text("approved plan\n", encoding="utf-8")
        task_plan = root / "task-01.md"
        task_plan.write_text(f"# Task\n\n{HEADINGS}\n", encoding="utf-8")
        graph = {
            "schema_version": "1.0.0",
            "source_plan": {
                "path": "approved-plan.md",
                "sha256": hashlib.sha256(plan.read_bytes()).hexdigest(),
            },
            "context_policy": {
                "window_tokens": 200000,
                "warning_tokens": 100000,
                "reserve_percent": 50,
                "basis": "default-local-trial",
            },
            "coverage": [
                {
                    "kind": "requirement",
                    "text": "observable behavior",
                    "tasks": ["task-01"],
                },
                {"kind": "non-goal", "text": "no deployment", "tasks": ["task-01"]},
            ],
            "tasks": [
                {
                    "id": "task-01",
                    "title": "Prove behavior",
                    "outcome": "Caller observes the approved result.",
                    "plan_file": "task-01.md",
                    "plan_refs": ["Requirements"],
                    "requirements": ["observable behavior"],
                    "non_goals": ["no deployment"],
                    "acceptance_criteria": ["focused test proves result"],
                    "repo_evidence": ["tests/test_behavior.py"],
                    "depends_on": [],
                    "scope": {
                        "likely_touch": ["src/behavior.py"],
                        "must_not_touch": ["deployment"],
                    },
                    "verification": {
                        "focused": ["pytest tests/test_behavior.py"],
                        "broader": ["pytest"],
                        "real_interface": ["call public API"],
                    },
                    "compatibility": {
                        "migration": "not applicable: no stored data",
                        "rollout": "not applicable: local candidate only",
                        "rollback": "revert the isolated change",
                        "documentation": "update public behavior docs",
                    },
                    "replan_triggers": ["public API differs from approved plan"],
                    "context_budget": {
                        "warning_tokens": 100000,
                        "assessment": "well-below-warning",
                        "confidence": "high",
                        "drivers": ["one public seam and focused test"],
                        "split_triggers": ["a second independent behavior appears"],
                    },
                }
            ],
        }
        graph_path = root / "task-graph.json"
        self.write_graph(graph_path, graph)
        return graph_path

    def test_accepts_complete_graph(self):
        with tempfile.TemporaryDirectory() as directory:
            graph_path = self.make_graph(Path(directory))
            self.assertEqual([], MODULE.validate(graph_path))

    def test_rejects_cycle_and_over_warning_task(self):
        with tempfile.TemporaryDirectory() as directory:
            graph_path = self.make_graph(Path(directory))
            graph = self.read_graph(graph_path)
            graph["tasks"][0]["depends_on"] = ["task-01"]
            graph["tasks"][0]["context_budget"]["assessment"] = "over-warning"
            self.write_graph(graph_path, graph)
            findings = MODULE.validate(graph_path)
            self.assertTrue(any("depend on itself" in item for item in findings))
            self.assertTrue(any("not assessed within" in item for item in findings))

    def test_rejects_escaping_plan_file_and_stale_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            graph_path = self.make_graph(root)
            graph = self.read_graph(graph_path)
            graph["tasks"][0]["plan_file"] = "../outside.md"
            (root / "approved-plan.md").write_text("changed\n", encoding="utf-8")
            self.write_graph(graph_path, graph)
            findings = MODULE.validate(graph_path)
            self.assertTrue(any("does not match" in item for item in findings))
            self.assertTrue(any("escapes" in item for item in findings))

    def test_reports_each_task_section_without_mutating_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            graph_path = self.make_graph(root)
            graph = self.read_graph(graph_path)
            task = graph["tasks"][0]
            task["title"] = ""
            task["depends_on"] = ["missing-task"]
            task["scope"]["likely_touch"] = []
            task["verification"]["broader"] = []
            task["compatibility"]["rollout"] = ""
            task["context_budget"]["warning_tokens"] = 90000
            task["context_budget"]["drivers"] = []
            graph["coverage"][0]["tasks"] = ["missing-task"]
            self.write_graph(graph_path, graph)
            task_plan = root / "task-01.md"
            task_plan.write_text(
                task_plan.read_text(encoding="utf-8").replace(
                    "## Replan Triggers", "## Removed Replan Heading"
                ),
                encoding="utf-8",
            )
            graph_before = graph_path.read_bytes()
            plan_before = task_plan.read_bytes()

            findings = MODULE.validate(graph_path)

            expected = {
                "task task-01 needs a title",
                "task task-01 depends on unknown task missing-task",
                "task task-01.scope.likely_touch must be non-empty",
                "task task-01.verification.broader must be non-empty",
                "task task-01.compatibility.rollout must be non-empty",
                "task task-01 context warning differs from graph policy",
                "task task-01.context_budget.drivers must be non-empty",
                "task task-01.plan_file is missing heading: ## Replan Triggers",
                "coverage[0] refers to an unknown task",
                (
                    "task task-01.requirements item lacks matching coverage: "
                    "observable behavior"
                ),
            }
            self.assertTrue(expected.issubset(findings))
            self.assertEqual(graph_before, graph_path.read_bytes())
            self.assertEqual(plan_before, task_plan.read_bytes())

    def test_cli_returns_json_and_nonzero_for_invalid_graph(self):
        with tempfile.TemporaryDirectory() as directory:
            graph_path = self.make_graph(Path(directory))
            graph = self.read_graph(graph_path)
            graph["tasks"][0]["depends_on"] = ["task-01"]
            self.write_graph(graph_path, graph)

            completed = subprocess.run(
                [sys.executable, str(SCRIPT), str(graph_path)],
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(1, completed.returncode)
            self.assertEqual("", completed.stderr)
            payload = json.loads(completed.stdout)
            self.assertFalse(payload["pass"])
            self.assertIn(
                "task task-01 cannot depend on itself", payload["findings"]
            )


if __name__ == "__main__":
    unittest.main()
