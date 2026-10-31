"""Keep every retained Cargo unit tied to its own input content."""

from __future__ import annotations

from enum import Enum
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import json
import unittest
from uuid import uuid4


from tools import cargo_inputs
from tools.cargo_inputs_trace import trace

ROOT = Path(__file__).resolve().parents[2]

COAUTHOR = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"


class Version(Enum):
    A = "version-A"
    B = "version-B"


@unittest.skipUnless(sys.platform == "linux", "The CI Cargo cache runs on Linux.")
class CargoCacheTests(unittest.TestCase):
    watch_native_tree = True
    def setUp(self) -> None:
        target = os.environ.get("CARGO_TARGET_DIR")
        self.assertTrue(target, "Set CARGO_TARGET_DIR before this test.")
        self.target = Path(target).resolve()
        cases = self.target / "tests" / "cargo-cache"
        cases.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix="cargo-inputs-", dir=os.environ.get("TMPDIR"))
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
        self.states: list[Path] = []
        self.layout = cargo_inputs.Layout(self.repo, self.target, (self.profile_dir,))
        self.last_detection: dict[str, bool] = {}

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
        state = self.case / f"state-{len(self.states)}"
        self.states.append(state)
        cargo_inputs.begin(self.layout, state)
        cargo_inputs.restore(self.layout)
        environment = dict(self.env, PYTHONPATH=str(ROOT))
        result = subprocess.run(
            [sys.executable, "-m", "tools.cargo_inputs", "--workspace", str(self.repo),
             "--target", str(self.target), "--session", str(state), "trace", "--",
             "cargo", subcommand, "--offline", "--verbose", "--profile", self.profile, "-j", "8", *selection],
            cwd=self.repo, env=environment, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        marker = json.loads((state / "session.json").read_text())["marker_ns"]
        self.last_detection = {key: unit.built_after(marker) for key, unit in cargo_inputs.inventory(self.layout).items()}
        cargo_inputs.save(self.layout, state, scrub=False)
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
        self._run(["cargo", "generate-lockfile", "--offline"])
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

    def test_unselected_consumer_keeps_its_original_dependency_record(self) -> None:
        library = self.repo / "src/lib.rs"
        library.write_text("pub fn version() -> &'static str { \"version-A\" }\n")
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text('fn main() { println!("{}", cache_audit::version()); }\n')
        self._run(["git", "commit", "-am", "Use the library for #951", "-m", COAUTHOR])
        self._build("--bin", "cache-audit")
        self.assertEqual(self._binary(), Version.A.value)
        library.write_text("pub fn version() -> &'static str { \"version-B\" }\n")
        self._run(["git", "commit", "-am", "Edit the library for #951", "-m", COAUTHOR])
        self._build("--lib")
        self._build("--bin", "cache-audit")
        self.assertEqual(self._binary(), Version.B.value)

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

    def test_fresh_units_keep_their_fingerprint_mtimes(self) -> None:
        (self.repo / "version.txt").write_text("unchanged")
        (self.repo / "build.rs").write_text(
            'fn main() { println!("cargo:rerun-if-changed=version.txt"); }\n')
        self._commit(Version.A)
        self._build()
        self.assertEqual(len(self.last_detection), 3)
        self.assertTrue(all(self.last_detection.values()))
        before = {key: (unit.directory / unit.stem).stat().st_mtime_ns
                  for key, unit in cargo_inputs.inventory(self.layout).items()}
        self._build()
        self.assertFalse(any(self.last_detection.values()))
        after = {key: (unit.directory / unit.stem).stat().st_mtime_ns
                 for key, unit in cargo_inputs.inventory(self.layout).items()}
        self.assertEqual(before, after)

    def test_rebuilt_build_script_updates_its_compile_and_run_fingerprints(self) -> None:
        script = self.repo / "build.rs"
        script.write_text('fn main() { println!("cargo:rerun-if-changed=build.rs"); }\n')
        self._commit(Version.A)
        self._build()
        script.write_text(script.read_text() + "// source edit\n")
        self._run(["git", "commit", "-am", "Edit build script for #951", "-m", COAUTHOR])
        self._build()
        self.assertEqual(len(self.last_detection), 3)
        self.assertTrue(all(self.last_detection.values()))

    def test_undeclared_proc_macro_read_fails_the_contract_check(self) -> None:
        macro = self.repo / "macro"
        (macro / "src").mkdir(parents=True)
        (macro / "Cargo.toml").write_text(
            '[package]\nname="input-macro"\nversion="0.1.0"\nedition="2021"\n[lib]\nproc-macro=true\n')
        (macro / "src/lib.rs").write_text(
            '#[proc_macro] pub fn read_input(_: proc_macro::TokenStream) -> proc_macro::TokenStream {'
            ' std::fs::read("hidden.txt").unwrap(); "fn main() {}".parse().unwrap() }\n')
        (self.repo / "hidden.txt").write_text("undeclared")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[dependencies]\ninput-macro={path="macro"}\n')
        self._run(["cargo", "generate-lockfile", "--offline"])
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text("input_macro::read_input!();\n")
        self._run(["git", "commit", "-am", "Read macro input for #951", "-m", COAUTHOR])
        with self.assertRaisesRegex(cargo_inputs.InputError, "Undeclared input.*hidden.txt"):
            self._build()

    def test_native_cpp_edit_after_restore_rebuilds_the_native_crate(self) -> None:
        native = self.repo / "native"
        native.mkdir()
        (native / "CMakeLists.txt").write_text("# Constant across the source edit\n")
        cpp = native / "version.cpp"
        cpp.write_text('extern "C" int version() { return 1; }\n')
        # The fixture runs the same copy function as the vendored native crate.
        helper = (ROOT / "crates/vendor/llama-cpp-sys-4/build_inputs.rs").read_text()
        if not self.watch_native_tree:
            helper = helper.replace("source.display()", 'source.join("CMakeLists.txt").display()')
        (self.repo / "build_inputs.rs").write_text(helper)
        (self.repo / "build.rs").write_text("""
mod build_inputs;
fn main() {
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



    def test_nested_cargo_builds_receive_input_records(self) -> None:
        child = self.repo / "child"
        (child / "src").mkdir(parents=True)
        (child / "Cargo.toml").write_text(
            '[package]\nname="nested-input-child"\nversion="0.1.0"\nedition="2021"\n[workspace]\n'
            f'[profile.{self.profile}]\ninherits="dev"\n')
        (child / "src/main.rs").write_text('fn main() {}\n')
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text("""
fn main() {
    assert!(std::process::Command::new("cargo").args(["build", "--offline", "--manifest-path", "child/Cargo.toml", "--profile"])
        .arg(std::env::var("NESTED_PROFILE").unwrap()).args(["-j", "8"]).status().unwrap().success());
}
""")
        self._run(["git", "add", "."])
        self._run(["git", "commit", "-m", "Add nested build for #951", "-m", COAUTHOR])
        self.env["NESTED_PROFILE"] = self.profile
        self._build(subcommand="run")
        units = cargo_inputs.inventory(self.layout)
        child_units = [unit for unit in units.values() if unit.directory.name.startswith("nested-input-child-")]
        self.assertTrue(child_units)
        self.assertTrue(all((unit.directory / cargo_inputs.RECORD).exists() for unit in child_units))

    def test_damaged_record_is_deleted_and_rebuilt(self) -> None:
        self._commit(Version.A)
        self._build()
        unit = next(unit for unit in cargo_inputs.inventory(self.layout).values() if unit.kind == cargo_inputs.Kind.COMPILE)
        (unit.directory / cargo_inputs.RECORD).write_text('{"damaged":true}')
        output = self._build()
        self.assertIn("Compiling cache-audit", output)
        self.assertEqual(self._binary(), Version.A.value)
        self.assertTrue((unit.directory / cargo_inputs.RECORD).exists())

    def test_undeclared_build_script_read_fails_the_contract_check(self) -> None:
        (self.repo / "secret.txt").write_text("secret")
        (self.repo / "build.rs").write_text(
            'fn main() { println!("cargo:rerun-if-changed=build.rs"); std::fs::read_to_string("secret.txt").unwrap(); }\n')
        self._commit(Version.A)
        with self.assertRaisesRegex(cargo_inputs.InputError, "Undeclared input.*secret.txt"):
            self._build()

    def test_option_env_is_a_rustc_environment_dependency(self) -> None:
        self._commit(Version.A)
        (self.repo / "src/main.rs").write_text(
            'fn main() { println!("{}", option_env!("JBOTCI_WEB_EMBEDDINGS_BASE_URL").unwrap_or("unset")); }\n')
        self._run(["git", "commit", "-am", "Read the optional URL for #951", "-m", COAUTHOR])
        self._build()
        self.assertEqual(self._binary(), "unset")
        unit = next(iter(cargo_inputs.inventory(self.layout).values()))
        record = cargo_inputs.load_record(unit)
        self.assertIn("JBOTCI_WEB_EMBEDDINGS_BASE_URL", record["cargo_env"])
        self.assertIsNone(record["cargo_env"]["JBOTCI_WEB_EMBEDDINGS_BASE_URL"])
        self.env["JBOTCI_WEB_EMBEDDINGS_BASE_URL"] = "https://example.invalid/B"
        output = self._build()
        self.assertIn("Compiling cache-audit", output)
        self.assertEqual(self._binary(), "https://example.invalid/B")


if __name__ == "__main__":
    unittest.main()
