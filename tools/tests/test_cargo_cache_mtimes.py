"""Exercise the Cargo cache action with compiled A -> partial B -> A source."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from uuid import uuid4


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
ACTION_SCRIPT = REPOSITORY_ROOT / ".github/actions/cargo-cache-mtimes/run.sh"
COAUTHOR = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"


class Version(Enum):
    A = "version-A"
    B = "version-B"


@dataclass(frozen=True)
class TargetSnapshot:
    artifacts: Path
    source_commit: str

    def __post_init__(self) -> None:
        if not self.artifacts.is_dir():
            raise ValueError("The saved target directory must exist.")
        if re.fullmatch(r"[0-9a-f]{40}", self.source_commit) is None:
            raise ValueError("The source stamp must contain a SHA-1 commit ID.")


@unittest.skipUnless(sys.platform == "linux", "The cache action uses GNU tools on Linux.")
class CargoCacheMtimeTests(unittest.TestCase):
    def setUp(self) -> None:
        target = os.environ.get("CARGO_TARGET_DIR")
        self.assertTrue(target, "Set CARGO_TARGET_DIR before this test.")
        self.target = Path(target).resolve()
        cases = self.target / "tests" / "cargo-cache-mtimes"
        cases.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix="case-", dir=cases)
        self.addCleanup(temporary.cleanup)
        self.case = Path(temporary.name)
        self.repo = self.case / "source"
        self.repo.mkdir()
        self.origin = self.case / "origin.git"
        identity = uuid4().hex
        # An owned profile isolates snapshots without another Cargo target directory.
        self.profile = f"cache-mtime-{identity}"
        self.profile_dir = self.target / self.profile
        self.stamp = self.target / f".cache-commit-{identity}"
        self.assertFalse(self.profile_dir.exists())
        self.assertFalse(self.stamp.exists())
        self.addCleanup(shutil.rmtree, self.profile_dir, ignore_errors=True)
        self.addCleanup(self.stamp.unlink, missing_ok=True)
        global_config = self.case / "gitconfig"
        global_config.write_text("", encoding="utf-8")
        self.env = dict(
            os.environ,
            CARGO_TARGET_DIR=str(self.target),
            CARGO_BUILD_JOBS="8",
            GIT_CONFIG_GLOBAL=str(global_config),
            GIT_CONFIG_NOSYSTEM="1",
        )
        self._run(
            ["git", "init", "--bare", "--initial-branch=main", "--object-format=sha1", str(self.origin)],
            cwd=self.case,
        )
        self._run(["git", "init", "--initial-branch=main", "--object-format=sha1"])
        self._run(["git", "config", "user.name", "Cache regression"])
        self._run(["git", "config", "user.email", "cache-regression@example.invalid"])
        self._run(["git", "remote", "add", "origin", str(self.origin)])
        (self.repo / "src").mkdir()
        (self.repo / "Cargo.toml").write_text(
            '[package]\nname = "cache-audit"\nversion = "0.1.0"\nedition = "2021"\n'
            f'\n[workspace]\n\n[profile.{self.profile}]\ninherits = "dev"\n',
            encoding="utf-8",
        )
        self._write_source(Version.A)
        self._run(["cargo", "generate-lockfile", "--offline"])

    def _run(
        self,
        arguments: list[str],
        *,
        cwd: Path | None = None,
        extra_env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            arguments,
            cwd=cwd or self.repo,
            env=self.env | (extra_env or {}),
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            result.returncode, 0, f"{arguments!r}\n{result.stdout}\n{result.stderr}"
        )
        return result

    def _write_source(self, version: Version) -> None:
        (self.repo / "src/main.rs").write_text(
            f'fn main() {{ println!("{version.value}"); }}\n', encoding="utf-8"
        )

    def _commit(self, version: Version) -> str:
        self._write_source(version)
        self._run(["git", "add", "."])
        self._run(["git", "commit", "-m", f"Build {version.value} for #951", "-m", COAUTHOR])
        commit = self._run(["git", "rev-parse", "HEAD"]).stdout.strip()
        self._run(["git", "push", "origin", "HEAD:refs/heads/main"])
        return commit

    def _action(self, mode: str, commit: str) -> None:
        self._run(
            ["bash", str(ACTION_SCRIPT)],
            extra_env={
                "MODE": mode,
                "TARGET_DIR": str(self.target),
                "STAMP_FILE": self.stamp.name,
                "OLD_TOUCH_TIMESTAMP": "200001010000.00",
                "GITHUB_WORKSPACE": str(self.repo),
                "GITHUB_SHA": commit,
            },
        )

    def _build(self) -> str:
        result = self._run(
            ["cargo", "build", "--offline", "--verbose", "--profile", self.profile, "-j", "8"]
        )
        return result.stdout + result.stderr

    def _binary(self) -> str:
        return self._run([str(self.profile_dir / "cache-audit")]).stdout.strip()

    def _snapshot(self) -> TargetSnapshot:
        destination = self.case / "saved-target"
        shutil.copytree(self.profile_dir, destination)
        return TargetSnapshot(
            artifacts=destination,
            source_commit=self.stamp.read_text(encoding="utf-8").strip(),
        )

    def _restore(self, snapshot: TargetSnapshot) -> None:
        shutil.rmtree(self.profile_dir)
        shutil.copytree(snapshot.artifacts, self.profile_dir)
        self.stamp.write_text(snapshot.source_commit + "\n", encoding="utf-8")

    def test_failed_run_keeps_the_last_successful_target_and_stamp(self) -> None:
        commit_a = self._commit(Version.A)
        self._build()
        self.assertEqual(self._binary(), Version.A.value)
        self._action("stamp", commit_a)
        saved = self._snapshot()

        commit_b = self._commit(Version.B)
        self._action("restore", commit_b)
        self._build()
        self.assertEqual(self._binary(), Version.B.value)
        self.assertEqual(self.stamp.read_text(encoding="utf-8").strip(), commit_a)
        # A later job failure prevents both stamping and replacing the saved cache.
        self._restore(saved)
        self._run(["git", "checkout", "--detach", commit_a])
        self._action("restore", commit_a)
        build = self._build()

        self.assertIn('println!("version-A")', (self.repo / "src/main.rs").read_text())
        self.assertIn("Fresh cache-audit", build)
        self.assertEqual(self._binary(), Version.A.value)
        self.assertEqual(self.stamp.read_text(encoding="utf-8").strip(), commit_a)


if __name__ == "__main__":
    unittest.main()
