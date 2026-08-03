from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("normalize_fixture", ROOT / "scripts" / "normalize_fixture.py")
NORMALIZER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(NORMALIZER)


class ContractTests(unittest.TestCase):
    def test_equivalent_harness_fixtures_normalize_identically(self) -> None:
        fixture_dir = ROOT / "fixtures" / "equivalent-task"
        expected = json.loads((fixture_dir / "expected.json").read_text())
        for harness in ("codex", "claude", "pi"):
            with self.subTest(harness=harness):
                self.assertEqual(NORMALIZER.normalize(fixture_dir / f"{harness}.json"), expected)

    def test_annotation_validation_rejects_incomplete_records(self) -> None:
        module_path = ROOT / "scripts" / "append_annotation.py"
        annotation_spec = importlib.util.spec_from_file_location("append_annotation", module_path)
        module = importlib.util.module_from_spec(annotation_spec)
        assert annotation_spec.loader
        annotation_spec.loader.exec_module(module)
        valid = {
            "annotation_id": "annotation-1",
            "task_id": "task-1234",
            "recorded_at": "2026-08-03T00:00:00Z",
            "kind": "owner",
            "status": "pass",
            "owner": "human-owner",
            "rubric_version": "1",
        }
        module.validate(valid)
        with self.assertRaises(ValueError):
            module.validate({"annotation_id": "short"})

    def test_normal_contract_has_no_content_fields(self) -> None:
        schema = json.loads((ROOT / "schemas" / "task-record.schema.json").read_text())
        forbidden = {"prompt", "source", "diff", "command", "path", "payload", "body"}
        properties = {name.rsplit(".", 1)[-1] for name in schema["properties"]}
        self.assertTrue(forbidden.isdisjoint(properties))


if __name__ == "__main__":
    unittest.main()
