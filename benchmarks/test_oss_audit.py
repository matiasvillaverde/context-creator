import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import oss_audit


class AuditBoundaryTests(unittest.TestCase):
    def test_manifest_paths_cannot_escape_the_source_cache(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "cache"
            root.mkdir()
            for relative in ["../outside", "/absolute", "nested/../../outside", "nested\\..\\outside"]:
                with self.subTest(relative=relative), self.assertRaises(ValueError):
                    oss_audit.confined_path(root, relative)
            self.assertEqual(oss_audit.confined_path(root, "src/main.rs"), root / "src/main.rs")
            if os.name != "nt":
                (root / "linked").symlink_to(root.parent, target_is_directory=True)
                with self.assertRaises(ValueError):
                    oss_audit.confined_path(root, "linked/outside")

    def test_timeout_restores_the_original_sample_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            cache = Path(temporary)
            source = cache / "project" / "sample.py"
            source.parent.mkdir()
            original = b"def sample(): return 1\n"
            source.write_bytes(original)
            project = {"cache_key": "project", "language": "Python", "samples": [{
                "path": "sample.py", "sha256": hashlib.sha256(original).hexdigest(),
            }]}
            arguments = argparse.Namespace(cache=cache, probe=cache / "probe", manifest=cache / "manifest.json",
                                           output=cache / "results", mode="sessions", environment={})
            arguments.output.mkdir()
            stale_result = arguments.output / "python-0.sessions.json"
            stale_result.write_text('{"stale_revision_rejected": true}')

            def timeout(command, **_):
                source.write_text("temporary edit")
                raise subprocess.TimeoutExpired(command, 180, stderr=b"timed out")

            with patch.object(oss_audit.subprocess, "run", side_effect=timeout):
                oss_audit.probe_project(arguments, project)
            self.assertEqual(source.read_bytes(), original)
            self.assertFalse(stale_result.exists())
            receipt = json.loads((arguments.output / "project.sessions.probe.json").read_text())
            self.assertEqual(receipt["error"], "180 second timeout")

    def test_neutral_git_configuration_is_child_only(self):
        with tempfile.TemporaryDirectory() as temporary:
            before = dict(os.environ)
            arguments = argparse.Namespace(cache=Path(temporary), user_git_config=False)
            environment = oss_audit.audit_environment(arguments)
            self.assertEqual(dict(os.environ), before)
            self.assertIn("excludesFile", Path(environment["GIT_CONFIG_GLOBAL"]).read_text())
            self.assertTrue((Path(temporary) / ".audit-environment" / "empty-ignore").is_file())

    def test_partial_report_does_not_count_unmeasured_symbols_as_omissions(self):
        with tempfile.TemporaryDirectory() as temporary:
            raw = Path(temporary)
            (raw / "rust-0.semantic.json").write_text('{"error": "probe failed"}')
            (raw / "rust-1.cli.json").write_text('{"map": {"exit_code": 0}}')
            output = raw / "summary.json"
            subprocess.run([sys.executable, str(Path(__file__).with_name("oss_report.py")),
                            "--raw", str(raw), "--output", str(output)], check=True, capture_output=True)
            report = json.loads(output.read_text())
            self.assertEqual(report["summary"]["map_attempts"], 1)
            self.assertEqual(report["summary"]["map_successes"], 1)
            self.assertEqual(report["summary"]["completed_semantic_cases"], 0)
            self.assertEqual(report["summary"]["samples_with_checked_symbol_omissions"], 0)
            failed = next(row for row in report["records"] if row["id"] == "rust-0")
            self.assertEqual(failed["semantic_error"], "probe failed")
            self.assertIsNone(failed["comparison"]["missing_symbols"])


if __name__ == "__main__":
    unittest.main()
