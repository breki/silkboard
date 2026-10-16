"""Behavioral tests for the Stop hook; no Cargo workspace is required."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


HOOK = Path(__file__).resolve().parents[1] / "stop-check.sh"
BASH = shutil.which("bash")
STAGES = ["fmt --all -- --check", "xtask clippy", "xtask doc", "xtask test"]


class StopCheckTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="stop hook tests ")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.repo = self.base / "checkout with spaces"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Hook test")
        self.git("config", "user.email", "hook@example.invalid")
        (self.repo / "main.rs").write_text("fn main() {}\n")
        (self.repo / "Cargo.toml").write_text("# mocked workspace\n")
        self.git("add", ".")
        self.git("commit", "-qm", "initial")
        self.bin = self.base / "bin"
        self.bin.mkdir()
        # An isolated PATH makes missing-runtime cases deterministic.
        for name in ("git", "jq", "cat"):
            (self.bin / name).symlink_to(shutil.which(name))
        self.cargo = self.bin / "cargo"
        self.cargo.write_text(
            "#!/bin/bash\n"
            'printf "%s|%s\\n" "$PWD" "$*" >> "$HOOK_TEST_LOG"\n'
            'if [[ "$*" == "$HOOK_TEST_FAIL" ]]; then\n'
            '  echo "mock gate unavailable or failed" >&2\n'
            "  exit 1\n"
            "fi\n"
        )
        self.cargo.chmod(0o755)
        self.log = self.base / "calls"
        self.env = {
            **os.environ,
            "PATH": str(self.bin),
            "HOOK_TEST_LOG": str(self.log),
            "HOOK_TEST_FAIL": "",
        }

    def git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.repo), *args],
            check=True, capture_output=True, text=True,
        )

    def run_hook(self, payload=None, raw=None):
        if raw is None:
            raw = json.dumps(payload or {"cwd": str(self.repo)})
        return subprocess.run(
            [BASH, str(HOOK)], input=raw, text=True,
            capture_output=True, cwd=self.base, env=self.env,
        )

    def calls(self):
        return self.log.read_text().splitlines() if self.log.exists() else []

    def change_rust(self):
        (self.repo / "main.rs").write_text("fn main() { panic!(); }\n")

    def assert_pass(self, result, stages=()):
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")
        self.assertEqual(self.calls(), [f"{self.repo}|{s}" for s in stages])

    def test_no_rust_changes(self):
        (self.repo / "notes.md").write_text("documentation change\n")
        self.assert_pass(self.run_hook())

    def test_unstaged_rust(self):
        self.change_rust()
        self.assert_pass(self.run_hook(), STAGES)

    def test_staged_rust(self):
        self.change_rust()
        self.git("add", "main.rs")
        self.assert_pass(self.run_hook(), STAGES)

    def test_untracked_rust(self):
        (self.repo / "new file.rs").write_text("// new\n")
        self.assert_pass(self.run_hook(), STAGES)

    def test_unstaged_rust_deletion(self):
        (self.repo / "main.rs").unlink()
        self.assert_pass(self.run_hook(), STAGES)

    def test_staged_rust_deletion(self):
        (self.repo / "main.rs").unlink()
        self.git("add", "-u")
        self.assert_pass(self.run_hook(), STAGES)

    def test_payload_subdirectory_and_false_flag(self):
        self.change_rust()
        subdir = self.repo / "subdir"
        subdir.mkdir()
        self.assert_pass(self.run_hook({
            "cwd": str(subdir), "stop_hook_active": False,
        }), STAGES)

    def test_true_flag_prevents_recursion(self):
        self.change_rust()
        self.assert_pass(self.run_hook({
            "cwd": str(self.repo), "stop_hook_active": True,
        }))

    def test_string_true_does_not_skip(self):
        self.change_rust()
        self.assert_pass(self.run_hook({
            "cwd": str(self.repo), "stop_hook_active": "true",
        }), STAGES)

    def test_each_failed_stage_stops_the_pipeline(self):
        self.change_rust()
        for i, stage in enumerate(STAGES):
            with self.subTest(stage=stage):
                self.log.unlink(missing_ok=True)
                self.env["HOOK_TEST_FAIL"] = stage
                result = self.run_hook()
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn("failed or unavailable", result.stderr)
                self.assertIn("mock gate unavailable or failed", result.stderr)
                self.assertEqual(self.calls(), [
                    f"{self.repo}|{s}" for s in STAGES[:i + 1]
                ])

    def test_missing_workspace(self):
        self.change_rust()
        (self.repo / "Cargo.toml").unlink()
        result = self.run_hook()
        self.assertEqual(result.returncode, 2)
        self.assertIn("Cargo.toml is missing", result.stderr)
        self.assertEqual(self.calls(), [])

    def test_missing_cargo(self):
        self.change_rust()
        self.cargo.unlink()
        result = self.run_hook()
        self.assertEqual(result.returncode, 2)
        self.assertIn("Cargo is required", result.stderr)
        self.assertEqual(self.calls(), [])

    def test_missing_jq(self):
        (self.bin / "jq").unlink()
        result = self.run_hook()
        self.assertEqual(result.returncode, 0)
        warning = json.loads(result.stdout)["systemMessage"]
        self.assertIn("validation unavailable", warning)
        self.assertIn("jq is required", warning)
        self.assertEqual(self.calls(), [])

    def test_missing_jq_does_not_block_continuation(self):
        (self.bin / "jq").unlink()
        result = self.run_hook({
            "cwd": str(self.repo), "stop_hook_active": True,
        })
        self.assertEqual(result.returncode, 0)
        self.assertIn("validation unavailable", json.loads(result.stdout)["systemMessage"])
        self.assertEqual(self.calls(), [])

    def test_invalid_payloads(self):
        for raw in ("invalid", "[]", "{}", '{"cwd": 1}'):
            with self.subTest(raw=raw):
                result = self.run_hook(raw=raw)
                self.assertEqual(result.returncode, 2)
                self.assertIn("unavailable", result.stderr)
                self.assertEqual(self.calls(), [])

    def test_non_repository_payload(self):
        result = self.run_hook({"cwd": str(self.base)})
        self.assertEqual(result.returncode, 2)
        self.assertIn("Git worktree", result.stderr)

    def test_registered_command_from_subdirectory(self):
        # Exercise the exact configured shell command, including path quoting.
        config = json.loads((HOOK.parents[1] / "hooks.json").read_text())
        command = config["hooks"]["Stop"][0]["hooks"][0]["command"]
        destination = self.repo / ".codex/hooks"
        destination.mkdir(parents=True)
        shutil.copyfile(HOOK, destination / HOOK.name)
        subdir = self.repo / "subdir"
        subdir.mkdir()
        (self.bin / "bash").symlink_to(BASH)
        self.change_rust()
        result = subprocess.run(
            [BASH, "-c", command], cwd=subdir, env=self.env,
            input=json.dumps({"cwd": str(self.repo)}),
            text=True, capture_output=True,
        )
        self.assert_pass(result, STAGES)


if __name__ == "__main__":
    unittest.main()
