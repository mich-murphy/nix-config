"""Tests for Neo deterministic routing, revision, and approval behavior."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "neo.py"


STAGE_TEXT = {
    "discover": "# Discovery\n\nCurrent state\n\nFact\n\nUnknown\n\nSuccess\n",
    "product": "# Product\n\nOutcome\n\nScenario\n\nNon-goal\n\nAssumption\n",
    "architecture": (
        "# Architecture\n\nQuality scenario\n\nData flow\n\nFailure\n\nAlternative\n"
    ),
    "program": "# Program\n\nInterface\n\nInvariant\n\nCall path\n\nOwnership\n",
    "delivery": "# Delivery\n\nSlice\n\nVerifier\n\nDependency\n\nReplan\n",
}

FINAL_TEXT = """# Implementation Brief

## Intent

Intent.

## Requirements and Non-goals

Requirements.

## Current-state Evidence

Evidence.

## Product Scenarios

Scenarios.

## Architecture

Architecture.

## Program Design

Interface, invariant, and Call path.

## Delivery Slices

Slices.

## Verification

Verifier.

## Compatibility, Rollout, and Recovery

Recovery.

## Assumptions, Risks, and Replan Triggers

Risks.
"""


class NeoCliTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.cli("init", "example", "--title", "Example")

    def tearDown(self) -> None:
        self.temporary.cleanup()

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
            msg=f"stdout={completed.stdout}\nstderr={completed.stderr}",
        )
        if completed.stdout:
            return json.loads(completed.stdout)
        return {}

    def artifact(self, stage: str) -> Path:
        path = self.root / f"{stage}.md"
        path.write_text(STAGE_TEXT[stage], encoding="utf-8")
        return path

    def assess_full(self) -> dict:
        return self.cli(
            "assess",
            "example",
            "--signals",
            "problem-uncertain,system-boundary,new-abstraction",
        )

    def gate(self, stage: str) -> dict:
        return self.cli(
            "gate",
            "example",
            "--stage",
            stage,
            "--artifact",
            str(self.artifact(stage)),
        )

    def build_to_finalize(self) -> Path:
        self.assess_full()
        for stage in ("discover", "product", "architecture", "program", "delivery"):
            self.gate(stage)
        final = self.root / "implementation-brief.md"
        final.write_text(FINAL_TEXT, encoding="utf-8")
        self.cli("finalize", "example", "--artifact", str(final))
        return final

    def test_zero_risk_route_bypasses_all_neo_stages(self) -> None:
        state = self.cli("assess", "example", "--signals", "")
        self.assertEqual(state["route"], "direct")
        required = [
            stage for stage, item in state["stages"].items() if item["required"]
        ]
        self.assertEqual(required, [])
        self.assertIsNone(self.cli("status", "example")["current_stage"])

    def test_start_routes_confirmed_signals_in_one_operation(self) -> None:
        result = self.cli(
            "start",
            "second-example",
            "--title",
            "Second example",
            "--signals",
            "persistent-data,new-abstraction",
        )
        self.assertEqual(result["route"], "focused")
        self.assertEqual(result["current_stage"], "discover")

    def test_start_rejects_invalid_signals_without_creating_state(self) -> None:
        self.cli(
            "start",
            "invalid-example",
            "--title",
            "Invalid example",
            "--signals",
            "invented-risk",
            expected=2,
        )
        self.assertFalse(
            (self.root / ".neo/tasks/invalid-example/state.json").exists()
        )

    def test_full_route_requires_every_stage(self) -> None:
        state = self.assess_full()
        self.assertEqual(state["route"], "full")
        self.assertTrue(all(item["required"] for item in state["stages"].values()))

    def test_out_of_order_gate_is_rejected(self) -> None:
        self.assess_full()
        self.cli(
            "gate",
            "example",
            "--stage",
            "architecture",
            "--artifact",
            str(self.artifact("architecture")),
            expected=2,
        )

    def test_blocking_unknown_prevents_gate_until_resolved(self) -> None:
        self.assess_full()
        self.cli(
            "record-unknown",
            "example",
            "--stage",
            "discover",
            "--id",
            "audience",
            "--question",
            "Who is affected?",
        )
        status = self.cli("status", "example")
        self.assertEqual(status["current_stage"], "discover")
        self.assertIsNone(status["stages"]["discover"]["artifact"])
        self.cli(
            "gate",
            "example",
            "--stage",
            "discover",
            "--artifact",
            str(self.artifact("discover")),
            expected=2,
        )
        self.cli(
            "resolve-unknown",
            "example",
            "--id",
            "audience",
            "--resolution",
            "Operators",
        )
        self.gate("discover")

    def test_clarification_does_not_invalidate(self) -> None:
        self.assess_full()
        self.gate("discover")
        result = self.cli(
            "record-feedback",
            "example",
            "--kind",
            "clarify",
            "--message",
            "Explain the current-state evidence.",
            "--stage",
            "discover",
        )
        self.assertEqual(result["invalidated"], [])
        status = self.cli("status", "example")
        self.assertEqual(status["stages"]["discover"]["status"], "approved")

    def test_change_invalidates_owning_and_downstream_stages(self) -> None:
        self.assess_full()
        self.gate("discover")
        self.cli(
            "record-decision",
            "example",
            "--stage",
            "product",
            "--id",
            "product-audience",
            "--summary",
            "Primary audience",
            "--choice",
            "Operators",
            "--rationale",
            "Observed workflow",
        )
        self.gate("product")
        result = self.cli(
            "record-feedback",
            "example",
            "--kind",
            "change",
            "--message",
            "Include administrators.",
            "--decisions",
            "product-audience",
        )
        self.assertEqual(
            result["invalidated"],
            ["product", "architecture", "program", "delivery", "finalize"],
        )
        status = self.cli("status", "example")
        self.assertEqual(status["stages"]["product"]["status"], "pending")
        self.assertEqual(status["stages"]["architecture"]["status"], "stale")

    def test_replacement_links_to_superseded_decision(self) -> None:
        self.assess_full()
        self.gate("discover")
        self.cli(
            "record-decision",
            "example",
            "--stage",
            "product",
            "--id",
            "old-audience",
            "--summary",
            "Audience",
            "--choice",
            "Operators",
            "--rationale",
            "Initial evidence",
        )
        self.cli(
            "record-feedback",
            "example",
            "--kind",
            "change",
            "--message",
            "Expand the audience.",
            "--decisions",
            "old-audience",
        )
        self.cli("revise", "example", "--stage", "product")
        self.cli(
            "record-decision",
            "example",
            "--stage",
            "product",
            "--id",
            "new-audience",
            "--summary",
            "Audience",
            "--choice",
            "Operators and administrators",
            "--rationale",
            "User correction",
            "--supersedes",
            "old-audience",
        )
        state = json.loads(
            (self.root / ".neo/tasks/example/state.json").read_text(encoding="utf-8")
        )
        old = next(item for item in state["decisions"] if item["id"] == "old-audience")
        self.assertEqual(old["superseded_by"], "new-audience")

    def test_visual_prototype_requires_evidence(self) -> None:
        self.cli(
            "record-prototype",
            "example",
            "--kind",
            "visual",
            "--question",
            "Can users find the action?",
            expected=2,
        )
        result = self.cli(
            "record-prototype",
            "example",
            "--kind",
            "visual",
            "--question",
            "Can users find the action?",
            "--evidence",
            "Five observed walkthroughs",
            "--disposition",
            "Retain the chosen flow; discard prototype code",
        )
        self.assertEqual(result["kind"], "visual")

    def test_non_disposable_prototype_routes_record_without_fake_evidence(self) -> None:
        for kind in ("none", "tracer"):
            result = self.cli(
                "record-prototype",
                "example",
                "--kind",
                kind,
            )
            self.assertEqual(result["kind"], kind)

    def test_rejection_reopens_stage_and_dependents(self) -> None:
        self.assess_full()
        self.gate("discover")
        self.gate("product")
        result = self.cli(
            "record-feedback",
            "example",
            "--kind",
            "reject",
            "--message",
            "The product direction is wrong.",
            "--stage",
            "product",
        )
        self.assertEqual(
            result["invalidated"],
            ["product", "architecture", "program", "delivery", "finalize"],
        )

    def test_decision_card_validator_rejects_missing_structure(self) -> None:
        card = self.root / "card.md"
        card.write_text("## Decision\n\nChoose one.\n", encoding="utf-8")
        result = self.cli("validate-card", str(card))
        self.assertFalse(result["valid"])
        self.assertIn(
            "missing decision-card heading: Approval question",
            result["errors"],
        )

    def test_final_approval_is_bound_to_artifact_hash(self) -> None:
        final = self.build_to_finalize()
        final.write_text(FINAL_TEXT + "\nChanged after review.\n", encoding="utf-8")
        self.cli("approve", "example", expected=2)
        status = self.cli("status", "example")
        self.assertEqual(status["final"]["status"], "stale")

    def test_unchanged_final_brief_can_be_approved(self) -> None:
        self.build_to_finalize()
        result = self.cli("approve", "example")
        self.assertEqual(result["status"], "approved")
        self.cli("validate", "example")
        state = json.loads(
            (self.root / ".neo/tasks/example/state.json").read_text(encoding="utf-8")
        )
        retention = {
            item["stage"]: item["retention"] for item in state["artifacts"]
        }
        self.assertEqual(retention["discover"], "regenerable")
        self.assertEqual(retention["finalize"], "durable")

    def test_unknown_state_field_is_rejected(self) -> None:
        path = self.root / ".neo/tasks/example/state.json"
        state = json.loads(path.read_text(encoding="utf-8"))
        state["invented"] = True
        path.write_text(json.dumps(state), encoding="utf-8")
        self.cli("status", "example", expected=2)


if __name__ == "__main__":
    unittest.main()
