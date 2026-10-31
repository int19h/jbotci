"""Keep restored Cargo artifacts consistent with normal checkout timestamps."""

from __future__ import annotations

from enum import Enum
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from uuid import uuid4


COAUTHOR = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"


class Version(Enum):
    A = "version-A"
    B = "version-B"


@unittest.skipUnless(sys.platform == "linux", "The CI Cargo cache runs on Linux.")
class CargoCacheTests(unittest.TestCase):
    def setUp(self) -> None:
        target = os.environ.get("CARGO_TARGET_DIR")
        self.assertTrue(target, "Set CARGO_TARGET_DIR before this test.")
        self.target = Path(target).resolve()
        cases = self.target / "tests" / "cargo-cache"
        cases.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix="case-", dir=cases)
        self.addCleanup(temporary.cleanup)
        self.case = Path(temporary.name)
        self.repo = self.case / "source"
        self.repo.mkdir()
        self.origin = self.case / "origin.git"
        identity = uuid4().hex
        # An owned profile isolates snapshots without another Cargo target directory.
        self.profile = f"cache-{identity}"
        self.profile_dir = self.target / self.profile
        self.assertFalse(self.profile_dir.exists())
        self.addCleanup(shutil.rmtree, self.profile_dir, ignore_errors=True)
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

    def _build(self, *selection: str) -> str:
        result = self._run(
            ["cargo", "build", "--offline", "--verbose", "--profile", self.profile, "-j", "8", *selection]
        )
        return result.stdout + result.stderr

    def _binary(self) -> str:
        return self._run([str(self.profile_dir / "cache-audit")]).stdout.strip()

    def _snapshot(self) -> Path:
        destination = self.case / "saved-target"
        shutil.copytree(self.profile_dir, destination)
        return destination

    def _restore(self, snapshot: Path) -> None:
        shutil.rmtree(self.profile_dir)
        shutil.copytree(snapshot, self.profile_dir)

    def test_failed_run_keeps_the_last_successful_target(self) -> None:
        commit_a = self._commit(Version.A)
        self._build()
        self.assertEqual(self._binary(), Version.A.value)
        saved = self._snapshot()

        self._commit(Version.B)
        self._build()
        self.assertEqual(self._binary(), Version.B.value)
        # A later job failure prevents replacement of the saved cache.
        self._restore(saved)
        self._run(["git", "checkout", "--detach", commit_a])
        build = self._build()

        self.assertIn('println!("version-A")', (self.repo / "src/main.rs").read_text())
        self.assertIn("Compiling cache-audit", build)
        self.assertEqual(self._binary(), Version.A.value)

    def test_three_successful_builds_keep_the_selected_feature_current(self) -> None:
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() + "\n[features]\nextra = []\n")
        (self.repo / "build-mode").write_text("extra\n")
        self._commit(Version.A)
        self._build("--features", "extra")
        self.assertEqual(self._binary(), Version.A.value)
        (self.repo / "build-mode").write_text("default\n")
        self._commit(Version.B)
        self._build()
        self.assertEqual(self._binary(), Version.B.value)
        (self.repo / "build-mode").write_text("extra\n")
        self._commit(Version.B)
        self._run(["git", "checkout-index", "--all", "--force"])
        self._build("--features", "extra")
        self.assertEqual(self._binary(), Version.B.value)

    def test_successful_library_build_does_not_refresh_retained_binary(self) -> None:
        (self.repo / "src/lib.rs").write_text("pub fn version() -> &'static str { \"library\" }\n")
        (self.repo / "build-mode").write_text("binary\n")
        self._commit(Version.A)
        self._build("--bin", "cache-audit")
        self.assertEqual(self._binary(), Version.A.value)
        (self.repo / "build-mode").write_text("library\n")
        self._commit(Version.B)
        self._build("--lib")
        (self.repo / "build-mode").write_text("binary\n")
        self._commit(Version.B)
        self._run(["git", "checkout-index", "--all", "--force"])
        self._build("--bin", "cache-audit")
        self.assertEqual(self._binary(), Version.B.value)

    def test_path_dependency_source_is_not_fixed_by_the_lockfile(self) -> None:
        dependency = self.repo / "vendor/local-dependency"
        (dependency / "src").mkdir(parents=True)
        (dependency / "Cargo.toml").write_text(
            '[package]\nname = "local-dependency"\nversion = "0.1.0"\nedition = "2021"\n'
        )
        source = dependency / "src/lib.rs"
        source.write_text("pub fn version() -> &'static str { \"version-A\" }\n")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() +
            '\n[dependencies]\nlocal-dependency = { path = "vendor/local-dependency" }\n')
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text(
            'fn main() { println!("{}", local_dependency::version()); }\n')
        self._build()
        lockfile = (self.repo / "Cargo.lock").read_bytes()
        self.assertEqual(self._binary(), Version.A.value)
        source.write_text("pub fn version() -> &'static str { \"version-B\" }\n")
        self._build()
        self.assertEqual(self._binary(), Version.B.value)
        self.assertEqual((self.repo / "Cargo.lock").read_bytes(), lockfile)

    def test_build_script_regenerates_output_from_changed_input(self) -> None:
        (self.repo / "build.rs").write_text(
            'fn main() {\n'
            ' println!("cargo:rerun-if-changed=version.txt");\n'
            ' let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());\n'
            ' std::fs::write(out.join("version.txt"), std::fs::read("version.txt").unwrap()).unwrap();\n'
            '}\n')
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text(
            'fn main() { println!("{}", include_str!(concat!(env!("OUT_DIR"), "/version.txt"))); }\n')
        source = self.repo / "version.txt"
        source.write_text(Version.A.value)
        self._build()
        self.assertEqual(self._binary(), Version.A.value)
        source.write_text(Version.B.value)
        self._build()
        self.assertEqual(self._binary(), Version.B.value)


if __name__ == "__main__":
    unittest.main()
