from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import eval_compare
import eval_runtime


class EvalRuntimeTests(unittest.TestCase):
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
                "output": "/srv/app/config.toml cargo test -p worker git reset --hard would destroy work; then staging",
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
