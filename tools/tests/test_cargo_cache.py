"""Keep every retained Cargo unit tied to the stamped source."""

from __future__ import annotations

from enum import Enum
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import json
import textwrap
import unittest
from uuid import uuid4


from tools import cargo_cache

ROOT = Path(__file__).resolve().parents[2]

COAUTHOR = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"


class Version(Enum):
    A = "version-A"
    B = "version-B"


@unittest.skipUnless(sys.platform == "linux", "The CI Cargo cache runs on Linux.")
class CargoCacheTests(unittest.TestCase):
    native_watch = "native"
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
        self._run(["git", "init", "--initial-branch=main", "--object-format=sha1"])
        self._run(["git", "config", "user.name", "Cache regression"])
        self._run(["git", "config", "user.email", "cache-regression@example.invalid"])
        (self.repo / "src").mkdir()
        (self.repo / "Cargo.toml").write_text(
            '[package]\nname = "cache-audit"\nversion = "0.1.0"\nedition = "2021"\n'
            f'\n[workspace]\n\n[profile.{self.profile}]\ninherits = "dev"\n',
            encoding="utf-8",
        )
        self._write_source(Version.A)
        self._run(["cargo", "generate-lockfile", "--offline"])
        action = (ROOT / ".github/actions/cargo-cache-mtimes/action.yml").read_text()
        self.restore_body = textwrap.dedent(action.split("      run: |\n", 1)[1])
        self._run(["git", "remote", "add", "origin", str(self.repo)])
        self.states: list[Path] = []

    def _run(self, arguments: list[str]) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            arguments,
            cwd=self.repo,
            env=self.env,
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
        return commit

    def _build(self, *selection: str, subcommand: str = "build") -> str:
        if (self.profile_dir / ".cache-commit").exists():
            sha = self._run(["git", "rev-parse", "HEAD"]).stdout.strip()
            environment = dict(self.env, MODE="restore", TARGET_DIR=str(self.profile_dir),
                               STAMP_FILE=".cache-commit", OLD_TOUCH_TIMESTAMP="200001010000.00",
                               GITHUB_WORKSPACE=str(self.repo), GITHUB_SHA=sha)
            restored = subprocess.run(["bash", "-c", self.restore_body], cwd=self.repo, env=environment,
                                      capture_output=True, text=True)
            self.assertEqual(restored.returncode, 0, restored.stdout + restored.stderr)
        state = self.case / f"state-{len(self.states)}"
        environment = cargo_cache.install(state)
        self.states.append(state)
        result = subprocess.run(
            [environment["CARGO"], subcommand, "--offline", "--verbose", "--profile", self.profile, "-j", "8", *selection],
            cwd=self.repo, env=dict(self.env, **environment), capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        units, aliases, _ = cargo_cache.selected_units(self.target, state)
        cargo_cache.prune_profiles([self.profile_dir], units, aliases)
        sha = self._run(["git", "rev-parse", "HEAD"]).stdout.strip()
        (self.profile_dir / ".cache-commit").write_text(sha + "\n")
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
        self.assertIn("Fresh cache-audit", build)
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

    def test_default_features_revert_with_current_source(self) -> None:
        manifest = self.repo / "Cargo.toml"
        base = manifest.read_text()
        manifest.write_text(base + '\n[features]\ndefault = ["extra"]\nextra = []\n')
        self._commit(Version.A)
        self._build()
        self.assertEqual(self._binary(), Version.A.value)

        manifest.write_text(base + '\n[features]\ndefault = []\nextra = []\n')
        self._commit(Version.B)
        self._build()
        self.assertEqual(self._binary(), Version.B.value)

        manifest.write_text(base + '\n[features]\ndefault = ["extra"]\nextra = []\n')
        self._commit(Version.B)
        self._run(["git", "checkout-index", "--all", "--force"])
        self._build()
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
        self._run(["git", "add", "."])
        self._run(["git", "commit", "-m", "Use path dependency for #951", "-m", COAUTHOR])
        self._build()
        lockfile = (self.repo / "Cargo.lock").read_bytes()
        self.assertEqual(self._binary(), Version.A.value)
        source.write_text("pub fn version() -> &'static str { \"version-B\" }\n")
        self._run(["git", "commit", "-am", "Change path input for #951", "-m", COAUTHOR])
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
        self._run(["git", "add", "."])
        self._run(["git", "commit", "-m", "Add build inputs for #951", "-m", COAUTHOR])
        self._build()
        self.assertEqual(self._binary(), Version.A.value)
        source.write_text(Version.B.value)
        self._run(["git", "commit", "-am", "Change build input for #951", "-m", COAUTHOR])
        self._build()
        self.assertEqual(self._binary(), Version.B.value)

    def test_fresh_build_script_runs_and_compile_units_are_reported(self) -> None:
        (self.repo / "version.txt").write_text("unchanged")
        (self.repo / "build.rs").write_text(
            'fn main() { println!("cargo:rerun-if-changed=version.txt"); }\n')
        self._commit(Version.A)
        self._build()
        self._build()
        records = list(self.states[-1].glob("invocation-*.json"))
        messages = json.loads(records[0].read_text())["messages"]
        compiled = [m for m in messages if m["reason"] == "compiler-artifact"]
        self.assertTrue(compiled)
        self.assertTrue(all(m["fresh"] for m in compiled), compiled)
        self.assertTrue(any(m["target"]["kind"] == ["custom-build"] for m in compiled))
        self.assertEqual(sum(m["reason"] == "build-script-executed" for m in messages), 1)

    def test_native_cpp_edit_after_restore_rebuilds_the_native_crate(self) -> None:
        native = self.repo / "native"
        native.mkdir()
        (native / "CMakeLists.txt").write_text("# Constant across the source edit\n")
        cpp = native / "version.cpp"
        cpp.write_text('extern "C" int version() { return 1; }\n')
        # The fixture runs the same copy function as the vendored native crate.
        (self.repo / "build_inputs.rs").write_bytes(
            (ROOT / "crates/vendor/llama-cpp-sys-4/build_inputs.rs").read_bytes())
        (self.repo / "build.rs").write_text("""
mod build_inputs;
fn main() {
    println!("cargo:rerun-if-changed=native");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    build_inputs::prepare_native_source(std::path::Path::new("native"), &out.join("native"), &out);
    assert!(std::process::Command::new("c++").args(["-c", "-o"]).arg(out.join("version.o"))
        .arg(out.join("native/version.cpp")).status().unwrap().success());
    assert!(std::process::Command::new("ar").arg("crs").arg(out.join("libversion.a"))
        .arg(out.join("version.o")).status().unwrap().success());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=version");
}
""")
        build_script = self.repo / "build.rs"
        build_script.write_text(build_script.read_text().replace(
            "rerun-if-changed=native", f"rerun-if-changed={self.native_watch}"))
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text(
            'extern "C" { fn version() -> i32; } fn main() { println!("{}", unsafe { version() }); }\n')
        self._run(["git", "add", "."])
        self._run(["git", "commit", "-m", "Add native fixture for #951", "-m", COAUTHOR])
        self._build()
        self.assertEqual(self._binary(), "1")
        saved = self._snapshot()
        cpp.write_text('extern "C" int version() { return 2; }\n')
        self._run(["git", "commit", "-am", "Edit native source for #951", "-m", COAUTHOR])
        self._restore(saved)
        self._build()
        self.assertEqual(self._binary(), "2")


    def test_nested_cargo_uses_the_proxy(self) -> None:
        (self.repo / "build.rs").write_text("""
fn main() {
    let result = std::process::Command::new(std::env::var_os("CARGO").unwrap())
        .arg("--version").output().unwrap();
    assert!(result.status.success());
    println!("cargo:rerun-if-changed=build.rs");
}
""")
        self._commit(Version.A)
        self._build()
        reports = [json.loads(path.read_text()) for path in self.states[-1].glob("invocation-*.json")]
        self.assertEqual(len(reports), 2)
        self.assertTrue(any(report["arguments"] == ["--version"] for report in reports))

    def test_json_passthrough_keeps_fresh_unit_messages(self) -> None:
        self._commit(Version.A)
        self._build()
        output = self._build("--message-format=json")
        messages = [json.loads(line) for line in output.splitlines() if line.startswith("{")]
        self.assertTrue(any(message.get("reason") == "compiler-artifact" and message["fresh"] for message in messages))


    def test_cargo_install_reports_its_selected_units(self) -> None:
        self._commit(Version.A)
        destination = self.case / "installed"
        self._build("--path", str(self.repo), "--root", str(destination), subcommand="install")
        self.assertEqual(self._run([str(destination / "bin/cache-audit")]).stdout.strip(), Version.A.value)

    def test_direct_cargo_bypass_rejects_the_cache_save(self) -> None:
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() + "\n[features]\nextra = []\n")
        self._commit(Version.A)
        self._build()
        state = self.case / "bypass"
        environment = cargo_cache.install(state)
        result = subprocess.run(
            [environment["JBOTCI_CARGO_REAL"], "build", "--offline", "--profile", self.profile,
             "--features", "extra", "-j", "8"], cwd=self.repo,
            env=dict(self.env, **environment), capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        with self.assertRaisesRegex(cargo_cache.CacheError, "Compiler parent"):
            cargo_cache.selected_units(self.target, state)



if __name__ == "__main__":
    unittest.main()
