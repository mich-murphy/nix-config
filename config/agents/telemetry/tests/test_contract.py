from __future__ import annotations

import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT.parent))
SPEC = importlib.util.spec_from_file_location("normalize_fixture", ROOT / "scripts" / "normalize_fixture.py")
NORMALIZER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(NORMALIZER)

HOOK_SPEC = importlib.util.spec_from_file_location("app_agent_hook", ROOT / "hooks" / "app_agent_hook.py")
HOOK = importlib.util.module_from_spec(HOOK_SPEC)
assert HOOK_SPEC.loader
HOOK_SPEC.loader.exec_module(HOOK)


class ContractTests(unittest.TestCase):
    def test_hook_emits_completed_task_without_claiming_acceptance(self) -> None:
        exported = []
        with tempfile.TemporaryDirectory() as temporary:
            state_dir = Path(temporary)
            with mock.patch.dict(os.environ, {"APP_AGENT_HOOK_STATE_DIR": str(state_dir)}), mock.patch(
                "telemetry.task_trace.export_task_trace", side_effect=lambda trace: exported.append(trace) or {"status": "exported"}
            ):
                HOOK.handle({
                    "session_id": "conversation-1", "turn_id": "turn-1",
                    "cwd": temporary, "model": "gpt-test",
                    "hook_event_name": "UserPromptSubmit", "prompt": "Do the task",
                }, "codex")
                HOOK.handle({
                    "session_id": "conversation-1", "turn_id": "turn-1",
                    "cwd": temporary, "model": "gpt-test",
                    "hook_event_name": "Stop", "last_assistant_message": "Done",
                }, "codex")

        self.assertEqual(len(exported), 1)
        spans = exported[0]["resourceSpans"][0]["scopeSpans"][0]["spans"]
        root = next(span for span in spans if span["name"] == "agent.task")
        attributes = {
            item["key"]: next(iter(item["value"].values()))
            for item in root["attributes"]
        }
        self.assertEqual(attributes["session.id"], "conversation-1")
        self.assertEqual(attributes["app.agent.final.status"], "completed")
        self.assertNotEqual(attributes["app.agent.final.status"], "accepted")

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
        module.validate({**valid, "mlflow_trace_id": "tr-" + "a" * 32})
        with self.assertRaises(ValueError):
            module.validate({**valid, "mlflow_trace_id": "trace-a"})
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
        claude_settings = json.loads((ROOT / "claude-observability.settings.json").read_text())
        claude = claude_settings["env"]
        codex = (ROOT / "codex-observability.config.toml").read_text()
        pi = (ROOT / "pi" / "app-agent-otel.ts").read_text()

        self.assertIn(f"Contract {version}", (ROOT / "contract.md").read_text())
        self.assertIn(f"app.agent.schema.version={version}", claude["OTEL_RESOURCE_ATTRIBUTES"])
        self.assertIn(f'const SCHEMA_VERSION = "{version}"', pi)
        self.assertEqual(claude["OTEL_TRACES_EXPORTER"], "none")
        self.assertEqual(claude["OTEL_LOG_USER_PROMPTS"], "0")
        self.assertEqual(claude["OTEL_LOG_TOOL_DETAILS"], "0")
        self.assertEqual(claude["OTEL_LOG_TOOL_CONTENT"], "0")
        self.assertEqual(claude["OTEL_LOG_RAW_API_BODIES"], "0")
        self.assertIn("UserPromptSubmit", claude_settings["hooks"])
        self.assertIn("Stop", claude_settings["hooks"])
        self.assertIn('trace_exporter = "none"', codex)
        self.assertIn('UserPromptSubmit', codex)
        self.assertIn('Stop', codex)

    def test_pi_requires_verifier_provenance_for_acceptance(self) -> None:
        pi = (ROOT / "pi" / "app-agent-otel.ts").read_text()
        self.assertIn('"session.id": sessionId', pi)
        self.assertIn("APP_AGENT_VERIFIER_PROVENANCE", pi)
        self.assertNotIn('task.attributes["app.agent.final.status"] = "accepted"', pi)
        self.assertIn('"app.agent.content.capture": "metadata"', pi)


if __name__ == "__main__":
    unittest.main()
