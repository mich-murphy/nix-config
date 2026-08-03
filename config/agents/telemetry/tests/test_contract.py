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

    def test_rich_contract_exposes_content_without_credential_fields(self) -> None:
        schema = json.loads((ROOT / "schemas" / "task-record.schema.json").read_text())
        properties = set(schema["properties"])
        self.assertTrue({
            "gen_ai.input.messages",
            "gen_ai.output.messages",
            "gen_ai.tool.call.arguments",
            "gen_ai.tool.call.result",
        }.issubset(properties))
        forbidden = {"authorization", "cookie", "password", "api_key", "access_token", "refresh_token", "private_key"}
        leaf_names = {name.rsplit(".", 1)[-1] for name in properties}
        self.assertTrue(forbidden.isdisjoint(leaf_names))

    def test_harnesses_use_rich_contract_version_and_safe_content_gates(self) -> None:
        schema = json.loads((ROOT / "schemas" / "task-record.schema.json").read_text())
        version = schema["properties"]["app.agent.schema.version"]["const"]
        claude = json.loads((ROOT / "claude-observability.settings.json").read_text())["env"]
        pi = (ROOT / "pi" / "app-agent-otel.ts").read_text()

        self.assertIn(f"Contract {version}", (ROOT / "contract.md").read_text())
        self.assertIn(f"app.agent.schema.version={version}", claude["OTEL_RESOURCE_ATTRIBUTES"])
        self.assertIn(f'const SCHEMA_VERSION = "{version}"', pi)
        self.assertEqual(claude["OTEL_LOG_USER_PROMPTS"], "1")
        self.assertEqual(claude["OTEL_LOG_TOOL_DETAILS"], "1")
        self.assertEqual(claude["OTEL_LOG_TOOL_CONTENT"], "1")
        self.assertEqual(claude["OTEL_LOG_RAW_API_BODIES"], "0")


if __name__ == "__main__":
    unittest.main()
