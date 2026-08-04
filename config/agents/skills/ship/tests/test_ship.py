"""Tests for ship state ordering, freshness, review routing, and release gates."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "ship.py"


class ShipCliTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)
        self.root = self.base / "repo"
        self.root.mkdir()
        self.git("init")
        self.git("config", "user.email", "ship@example.test")
        self.git("config", "user.name", "Ship Test")
        (self.root / "app.py").write_text("VALUE = 1\n")
        self.git("add", "app.py")
        self.git("commit", "-m", "initial")
        self.plan = self.base / "plan.md"
        self.plan.write_text("# Plan\n\nChange VALUE with a regression test.\n")
        self.readiness = self.base / "readiness.json"
        self.readiness.write_text(
            json.dumps(
                {
                    "objective": "Change observable value",
                    "acceptance_criteria": ["VALUE equals 2"],
                    "non_goals": [],
                    "constraints": ["Preserve module API"],
                    "verification": ["Run focused test"],
                    "open_questions": [],
                }
            )
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def git(self, *arguments: str) -> None:
        completed = subprocess.run(
            ["git", "-C", str(self.root), *arguments],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def cli(self, *arguments: str, expected: int = 0) -> dict:
        completed = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *arguments],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(
            completed.returncode,
            expected,
            f"stdout={completed.stdout}\nstderr={completed.stderr}",
        )
        stream = completed.stdout if completed.stdout else completed.stderr
        return json.loads(stream)

    def artifact(self, name: str) -> Path:
        path = self.base / name
        path.write_text("evidence\n")
        return path

    def review(self, verdict: str, findings: list[dict] | None = None) -> Path:
        count = len(list(self.base.glob("review-*.json")))
        path = self.base / f"review-{count}.json"
        path.write_text(
            json.dumps(
                {
                    "verdict": verdict,
                    "summary": "Independent review",
                    "findings": findings or [],
                }
            )
        )
        return path

    @staticmethod
    def finding(route: str) -> dict:
        return {
            "id": "R1",
            "severity": "blocking",
            "category": "correctness",
            "location": "app.py:1",
            "finding": "Candidate does not meet the plan",
            "evidence": "VALUE is incorrect",
            "consequence": "Caller sees the wrong value",
            "route": route,
        }

    def start_ready(self, max_cycles: int = 2) -> None:
        self.cli(
            "start", "change-value", "--title", "Change value",
            "--plan", str(self.plan), "--max-review-cycles", str(max_cycles),
        )
        self.cli("ready", "change-value", "--readiness", str(self.readiness))

    def reach_review(self) -> None:
        (self.root / "app.py").write_text("VALUE = 2\n")
        self.cli(
            "advance", "change-value", "--stage", "implementation",
            "--evidence", str(self.artifact("implementation.md")),
        )
        self.cli(
            "advance", "change-value", "--stage", "verification",
            "--evidence", str(self.artifact("verification.md")),
        )

    def test_happy_path_reaches_release_ready(self) -> None:
        self.start_ready()
        self.reach_review()
        result = self.cli(
            "record-review", "change-value", "--review", str(self.review("pass"))
        )
        self.assertEqual(result["current"], "release-ready")
        self.assertTrue(self.cli("validate", "change-value")["valid"])

    def test_start_records_dirty_paths_and_excludes_ship_state(self) -> None:
        (self.root / "draft.py").write_text("VALUE = 2\n")
        ship_state = self.root / ".ship" / "tasks" / "existing"
        ship_state.mkdir(parents=True)
        (ship_state / "state.json").write_text("{}\n")

        result = self.cli(
            "start", "change-value", "--title", "Change value",
            "--plan", str(self.plan),
        )

        self.assertEqual(result["initial_paths"], ["?? draft.py"])

    def test_candidate_change_invalidates_verified_review_input(self) -> None:
        self.start_ready()
        self.reach_review()
        (self.root / "app.py").write_text("VALUE = 3\n")
        error = self.cli(
            "record-review", "change-value", "--review", str(self.review("pass")),
            expected=2,
        )
        self.assertIn("changed after verification", error["error"])

    def test_behavior_finding_requires_remediation_and_new_review(self) -> None:
        self.start_ready(max_cycles=3)
        self.reach_review()
        result = self.cli(
            "record-review", "change-value",
            "--review", str(self.review("changes-required", [self.finding("tdd")])),
        )
        self.assertEqual(result["current"], "remediation")
        (self.root / "app.py").write_text("VALUE = 4\n")
        self.cli(
            "advance", "change-value", "--stage", "remediation",
            "--evidence", str(self.artifact("remediation.md")),
        )
        self.cli(
            "advance", "change-value", "--stage", "verification",
            "--evidence", str(self.artifact("verification-2.md")),
        )
        result = self.cli(
            "record-review", "change-value", "--review", str(self.review("pass"))
        )
        self.assertEqual(result["current"], "release-ready")

    def test_replan_finding_stops_without_remediation(self) -> None:
        self.start_ready()
        self.reach_review()
        result = self.cli(
            "record-review", "change-value",
            "--review", str(self.review("replan", [self.finding("replan")])),
        )
        self.assertEqual(result["current"], "stopped")
        self.assertIn("replanning", result["stop_reason"])

    def test_blocking_open_question_rejects_readiness(self) -> None:
        self.cli(
            "start", "change-value", "--title", "Change value",
            "--plan", str(self.plan),
        )
        value = json.loads(self.readiness.read_text())
        value["open_questions"] = [
            {"question": "What value should callers see?", "blocking": True}
        ]
        self.readiness.write_text(json.dumps(value))
        error = self.cli(
            "ready", "change-value", "--readiness", str(self.readiness), expected=2
        )
        self.assertIn("blocking open question", error["error"])

    def test_plan_change_makes_release_ready_stale(self) -> None:
        self.start_ready()
        self.reach_review()
        self.cli(
            "record-review", "change-value", "--review", str(self.review("pass"))
        )
        self.plan.write_text("# Changed plan\n")
        error = self.cli("validate", "change-value", expected=2)
        self.assertIn("plan_changed", error["error"])

    def test_prior_pass_does_not_consume_later_remediation_budget(self) -> None:
        self.start_ready(max_cycles=2)
        self.reach_review()
        self.cli(
            "record-review", "change-value", "--review", str(self.review("pass"))
        )
        self.cli(
            "invalidate", "change-value", "--reason", "PR feedback changed code"
        )
        (self.root / "app.py").write_text("VALUE = 5\n")
        self.cli(
            "advance", "change-value", "--stage", "implementation",
            "--evidence", str(self.artifact("implementation-2.md")),
        )
        self.cli(
            "advance", "change-value", "--stage", "verification",
            "--evidence", str(self.artifact("verification-3.md")),
        )
        result = self.cli(
            "record-review", "change-value",
            "--review", str(self.review("changes-required", [self.finding("tdd")])),
        )
        self.assertEqual(result["current"], "remediation")


if __name__ == "__main__":
    unittest.main()
