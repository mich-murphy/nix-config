from __future__ import annotations

import importlib.util
import tempfile
import unittest
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader
    spec.loader.exec_module(module)
    return module


class ScriptTests(unittest.TestCase):
    def test_package_hash_changes_with_content(self) -> None:
        module = load("package_hash")
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "SKILL.md").write_text("one")
            first = module.package_hash(root)
            (root / "SKILL.md").write_text("two")
            self.assertNotEqual(first, module.package_hash(root))

    def test_audit_requires_evaluation_artifacts(self) -> None:
        module = load("audit_package")
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "SKILL.md").write_text("---\nname: x\ndescription: x\n---\n")
            self.assertTrue(any("missing evaluation artifact" in item for item in module.audit(root)))

    def test_audit_rejects_confounded_or_undersized_evals(self) -> None:
        module = load("audit_package")
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / "SKILL.md").write_text("---\nname: x\ndescription: x\n---\n")
            evals = root / "evals"
            evals.mkdir()
            (evals / "cases.json").write_text(json.dumps([{"id": "one"}]))
            (evals / "routing-cases.json").write_text("[]")
            (evals / "routes.json").write_text(json.dumps({"harnesses": {"codex": {}}}))
            for artifact in ("run-evals.py", "compare-evals.py", "release-decision.json"):
                (evals / artifact).write_text("{}")
            findings = module.audit(root)
            self.assertTrue(any("fewer than three" in item for item in findings))
            self.assertTrue(any("Codex, Claude, and Pi" in item for item in findings))
            self.assertTrue(any("comparison controls" in item for item in findings))


if __name__ == "__main__":
    unittest.main()
