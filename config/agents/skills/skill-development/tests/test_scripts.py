from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def load(relative: str, name: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class PackageScriptTests(unittest.TestCase):
    def test_package_hash_changes_with_content(self) -> None:
        module = load("scripts/package_hash.py", "package_hash")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "SKILL.md").write_text("one", encoding="utf-8")
            first = module.package_hash(root)
            (root / "SKILL.md").write_text("two", encoding="utf-8")
            self.assertNotEqual(first, module.package_hash(root))

    def test_scaffold_requires_evidence_and_refuses_overwrite(self) -> None:
        module = load("scripts/scaffold_package.py", "scaffold_package")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            skill = root / "example"
            skill.mkdir()
            (skill / "SKILL.md").write_text("---\nname: example\ndescription: example\n---\n", encoding="utf-8")
            proposal = root / "proposal.json"
            proposal.write_text(json.dumps({"job": "x", "evidence": {"references": ["trace-1"]}}), encoding="utf-8")
            created = module.scaffold(skill, proposal)
            self.assertTrue(skill / "evals/telemetry-policy.json" in created)
            with self.assertRaisesRegex(ValueError, "refusing to overwrite"):
                module.scaffold(skill, proposal)

    def test_audit_rejects_missing_evaluation_and_unsafe_telemetry(self) -> None:
        module = load("scripts/audit_package.py", "audit_package")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "SKILL.md").write_text("---\nname: x\ndescription: x\n---\n", encoding="utf-8")
            findings = module.audit(root)
            self.assertTrue(any("missing evaluation artifact" in item for item in findings))
            self.assertTrue(any("proposal" in item for item in findings))


class ArtifactGraderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.runner = load("evals/run-evals.py", "skill_development_run_evals")

    def test_deterministic_grader_blocks_unnecessary_skill(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            skill = output / "wrong"
            skill.mkdir()
            (skill / "SKILL.md").write_text("---\nname: wrong\ndescription: wrong\n---\n", encoding="utf-8")
            (output / "decision.json").write_text(json.dumps({"container": "skill"}), encoding="utf-8")
            checks = self.runner.grade_deterministic_control(output)
            blockers = [item for item in checks if item["blocking"] and not item["passed"]]
            self.assertTrue(blockers)

    def test_claude_parser_accepts_event_lists(self) -> None:
        result = self.runner.ProcessResult(
            0,
            1.0,
            json.dumps([{"type": "result", "result": "done", "usage": {"input_tokens": 4}}]),
            "",
        )
        output, usage, events = self.runner.parse_harness_output("claude", result)
        self.assertEqual(output, "done")
        self.assertEqual(usage["input_tokens"], 4)
        self.assertEqual(len(events), 1)

    def test_skill_grader_blocks_content_capture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            skill = output / "example"
            (skill / "evals/results").mkdir(parents=True)
            (skill / "SKILL.md").write_text("---\nname: example\ndescription: example\n---\n", encoding="utf-8")
            (skill / "proposal.json").write_text(json.dumps({"job": "x", "evidence": {"references": ["evidence.md"]}}), encoding="utf-8")
            (skill / "evals/cases.json").write_text("[]", encoding="utf-8")
            (skill / "evals/routing-cases.json").write_text("[]", encoding="utf-8")
            (skill / "evals/routes.json").write_text("{}", encoding="utf-8")
            (skill / "evals/telemetry-policy.json").write_text(json.dumps({"metadata_only_default": False, "content_capture_enabled": True}), encoding="utf-8")
            (skill / "evals/release-decision.json").write_text("{}", encoding="utf-8")
            (skill / "evals/results/status.json").write_text("{}", encoding="utf-8")
            checks = self.runner.grade_skill_package(output, False)
            privacy = next(item for item in checks if item["name"] == "privacy-first telemetry")
            self.assertTrue(privacy["blocking"])
            self.assertFalse(privacy["passed"])


if __name__ == "__main__":
    unittest.main()
