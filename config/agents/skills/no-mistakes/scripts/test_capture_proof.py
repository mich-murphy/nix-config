import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("capture-proof.py")


class ProofCapture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)
        for command in [["init", "-q"], ["config", "user.name", "Fixture"],
                        ["config", "user.email", "fixture@example.invalid"]]:
            self.git(*command)
        (self.repo / ".gitignore").write_text(".local/\n")
        (self.repo / "source.txt").write_text("healthy\n")
        self.git("add", ".")
        self.git("commit", "-qm", "baseline")
        (self.repo / ".local").mkdir()

    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True)

    def capture(self, output, code):
        return subprocess.run([sys.executable, str(SCRIPT), "--cwd", str(self.repo),
                               "--output", str(output), "--path", "source.txt", "--",
                               sys.executable, "-c", code], capture_output=True)

    def test_failed_fault_and_restoration_keep_primary_receipts(self):
        fault = self.repo / ".local/fault"
        result = self.capture(fault, "from pathlib import Path; import sys; Path('source.txt').write_text('fault'); print('detected'); sys.exit(7)")
        self.assertEqual(result.returncode, 7)
        receipt = json.loads((fault / "receipt.json").read_text())
        self.assertNotEqual(receipt["before"]["files"], receipt["after"]["files"])
        self.assertEqual((fault / "stdout.log").read_text(), "detected\n")
        restored = self.repo / ".local/restored"
        self.assertEqual(self.capture(restored, "from pathlib import Path; Path('source.txt').write_text('healthy\\n')").returncode, 0)
        final = json.loads((restored / "receipt.json").read_text())
        self.assertEqual(receipt["before"]["files"], final["after"]["files"])
        self.assertGreater(final["elapsed_ns"], 0)
        self.assertEqual((restored / "after.diff").read_bytes(), b"")

    def test_existing_or_unignored_output_never_launches_command(self):
        existing = self.repo / ".local/existing"
        existing.mkdir()
        for output in [existing, self.repo / "tracked-proof"]:
            self.assertNotEqual(self.capture(output, "from pathlib import Path; Path('sentinel').touch()").returncode, 0)
        self.assertFalse((self.repo / "sentinel").exists())

    def test_missing_executable_keeps_intent_without_claiming_completion(self):
        result = subprocess.run([sys.executable, str(SCRIPT), "--cwd", str(self.repo),
                                 "--output", str(self.repo / ".local/missing"), "--path", "source.txt", "--",
                                 str(self.repo / "does-not-exist")], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.repo / ".local/missing/intent.json").exists())
        self.assertFalse((self.repo / ".local/missing/receipt.json").exists())


if __name__ == "__main__":
    unittest.main()
