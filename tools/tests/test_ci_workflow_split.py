"""Regression checks for the separation between PR and deployment gates."""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
TEST_WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "test.yml"
RENDER_WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "render-image.yml"


class CiWorkflowSplitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.test_workflow = TEST_WORKFLOW.read_text(encoding="utf-8")
        cls.render_workflow = RENDER_WORKFLOW.read_text(encoding="utf-8")

    @unittest.skipUnless(shutil.which("actionlint"), "actionlint is not installed")
    def test_every_workflow_passes_actionlint(self) -> None:
        self.assertIsNotNone(shutil.which("shellcheck"), "actionlint requires ShellCheck")
        result = subprocess.run(
            ["actionlint", "-shellcheck=shellcheck", "-oneline"],
            cwd=REPOSITORY_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    @unittest.skipUnless(shutil.which("actionlint"), "actionlint is not installed")
    def test_actionlint_rejects_a_runner_context_in_job_environment(self) -> None:
        invalid_workflow = self.test_workflow.replace(
            '      CARGO_INCREMENTAL: "0"\n',
            '      CARGO_INCREMENTAL: "0"\n'
            '      JBOTCI_INPUT_SESSION: ${{ runner.temp }}/cargo-inputs\n',
            1,
        )
        self.assertNotEqual(invalid_workflow, self.test_workflow)
        result = subprocess.run(
            ["actionlint", "-shellcheck=shellcheck", "-oneline", "-"],
            input=invalid_workflow,
            cwd=REPOSITORY_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('context "runner" is not allowed here', result.stdout)

    def test_input_session_is_exported_before_cargo_work(self) -> None:
        for workflow in (self.test_workflow, self.render_workflow):
            with self.subTest(workflow=workflow.splitlines()[0]):
                setup_name = "      - name: Set the input session path\n"
                self.assertLess(workflow.index(setup_name), workflow.index("tools.cargo_inputs begin"))
                setup = workflow.split(setup_name, 1)[1].split("\n      - name:", 1)[0]
                command = setup.split("        run: ", 1)[1].strip()
                with tempfile.TemporaryDirectory() as temporary:
                    runner_temp = Path(temporary) / "runner with spaces"
                    environment_file = Path(temporary) / "github-env"
                    subprocess.run(
                        ["bash", "-e", "-c", command],
                        env={**os.environ, "RUNNER_TEMP": str(runner_temp), "GITHUB_ENV": str(environment_file)},
                        check=True,
                    )
                    self.assertEqual(
                        environment_file.read_text(),
                        f"JBOTCI_INPUT_SESSION={runner_temp}/cargo-inputs\n",
                    )

    def test_cached_container_paths_follow_runtime_temp_directory(self) -> None:
        for workflow, lane in ((self.test_workflow, "test"), (self.render_workflow, "deploy")):
            with self.subTest(lane=lane), tempfile.TemporaryDirectory() as temporary:
                step = workflow.split("      - name: Set container build paths\n", 1)[1].split("\n      - name:", 1)[0]
                command = textwrap.dedent(step.split("        run: |\n", 1)[1])
                workspace = Path(temporary) / "container workspace"
                environment_file = Path(temporary) / "github-env"
                subprocess.run(
                    ["bash", "-e", "-c", command],
                    env={**os.environ, "GITHUB_WORKSPACE": str(workspace), "RUNNER_TEMP": temporary,
                         "GITHUB_ENV": str(environment_file)},
                    check=True,
                )
                exported = dict(line.split("=", 1) for line in environment_file.read_text().splitlines())
                self.assertEqual(exported["CARGO_HOME"], str(Path(temporary) / "cargo-home"))
                self.assertEqual(exported["CARGO_TARGET_DIR"], str(Path(temporary) / f"cargo-target/{lane}"))
                self.assertEqual(exported["GIT_CONFIG_GLOBAL"], str(Path(temporary) / "jbotci-gitconfig"))
                for variable in ("CARGO_HOME", "CARGO_TARGET_DIR"):
                    self.assertFalse(Path(exported[variable]).is_relative_to(workspace))
                    self.assertEqual(workflow.count(f"path: ${{{{ env.{variable} }}}}"), 2)
                if lane == "deploy":
                    self.assertEqual(exported["DIOXUS_HOME"], str(Path(temporary) / "dioxus-home"))
                    self.assertEqual(exported["DIOXUS_CLI_ROOT"], str(Path(temporary) / "dioxus-cli/0.7.9"))
                    self.assertEqual(workflow.count("path: ${{ env.DIOXUS_CLI_ROOT }}"), 2)

    def test_native_cmake_cannot_discover_the_checkout_from_cache_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "checkout"
            runner_temp = root / "runner-temp"
            runner_temp.mkdir()
            environment = {**os.environ, "GIT_CONFIG_GLOBAL": str(root / "gitconfig"), "GIT_CONFIG_NOSYSTEM": "1"}
            subprocess.run(["git", "init", "-q", str(workspace)], env=environment, check=True)
            (workspace / "input").write_text("tracked source")
            subprocess.run(["git", "-C", str(workspace), "add", "input"], env=environment, check=True)
            subprocess.run(
                ["git", "-C", str(workspace), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "commit", "-qm", "fixture"], env=environment, check=True,
            )
            script = root / "probe.cmake"
            script.write_text('include("${INPUT_SCRIPT}")\nmessage(STATUS "NATIVE_COMMIT=${BUILD_COMMIT}")\n')
            command = ["cmake", "-DCMAKE_DISABLE_FIND_PACKAGE_Git=ON",
                       f"-DINPUT_SCRIPT={REPOSITORY_ROOT}/crates/vendor/llama-cpp-sys-4/llama.cpp/cmake/build-info.cmake",
                       "-P", str(script)]
            old_cache = workspace / ".github-cache/native-source"
            old_cache.mkdir(parents=True)
            old = subprocess.run(command, cwd=old_cache, env=environment, capture_output=True, text=True, check=True)
            self.assertNotIn("NATIVE_COMMIT=unknown", old.stdout)
            for workflow in (self.test_workflow, self.render_workflow):
                with self.subTest(workflow=workflow.splitlines()[0]):
                    step = workflow.split("      - name: Set container build paths\n", 1)[1].split("\n      - name:", 1)[0]
                    setup = textwrap.dedent(step.split("        run: |\n", 1)[1])
                    environment_file = root / "github-env"
                    environment_file.write_text("")
                    subprocess.run(
                        ["bash", "-e", "-c", setup],
                        env={**environment, "GITHUB_WORKSPACE": str(workspace), "RUNNER_TEMP": str(runner_temp),
                             "GITHUB_ENV": str(environment_file)}, check=True,
                    )
                    exported = dict(line.split("=", 1) for line in environment_file.read_text().splitlines())
                    native_source = Path(exported["CARGO_TARGET_DIR"]) / "native-source"
                    native_source.mkdir(parents=True)
                    result = subprocess.run(command, cwd=native_source, env={**environment, **exported},
                                            capture_output=True, text=True, check=True)
                    self.assertIn("NATIVE_COMMIT=unknown", result.stdout)

    def test_git_trust_covers_checkout_and_submodule_after_home_change(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkout = root / "checkout with spaces"
            source = root / "submodule-source"
            environment = {**os.environ, "GIT_CONFIG_GLOBAL": str(root / "gitconfig"), "GIT_CONFIG_NOSYSTEM": "1"}
            for repository in (source, checkout):
                subprocess.run(["git", "init", "-q", str(repository)], env=environment, check=True)
                (repository / "input").write_text("tracked input")
                subprocess.run(["git", "-C", str(repository), "add", "input"], env=environment, check=True)
                subprocess.run(
                    ["git", "-C", str(repository), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                     "commit", "-qm", "fixture"], env=environment, check=True,
                )
            subprocess.run(
                ["git", "-C", str(checkout), "-c", "protocol.file.allow=always", "submodule", "add", "-q",
                 str(source), "vendor/cll"], env=environment, check=True,
            )
            environment.update({"GIT_TEST_ASSUME_DIFFERENT_OWNER": "1", "GITHUB_WORKSPACE": str(checkout)})
            rejected = subprocess.run(
                ["git", "-C", str(checkout), "ls-files", "--recurse-submodules"], env=environment,
                capture_output=True, text=True, check=False,
            )
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("dubious ownership", rejected.stderr)
            for workflow in (self.test_workflow, self.render_workflow):
                step_name = "      - name: Trust the checked-out repositories\n"
                self.assertLess(workflow.index(step_name), workflow.index("tools.cargo_inputs begin"))
                step = workflow.split(step_name, 1)[1].split("\n      - name:", 1)[0]
                command = textwrap.dedent(step.split("        run: |\n", 1)[1])
                subprocess.run(["bash", "-e", "-c", command], env=environment, capture_output=True, check=True)
            changed_home = {**environment, "HOME": str(root / "other-home")}
            subprocess.run(
                ["git", "-C", str(checkout), "ls-files", "--recurse-submodules"],
                env=changed_home, capture_output=True, check=True,
            )
            trusted = subprocess.check_output(["git", "config", "--global", "--get-all", "safe.directory"],
                                              env=changed_home, text=True).splitlines()
            self.assertEqual(set(trusted), {str(checkout), str(checkout / "vendor/cll")})

    def test_pr_workflow_keeps_regular_tests_without_deployment_gates(self) -> None:
        self.assertIn("CARGO_BUILD_JOBS: 1", self.test_workflow)
        self.assertIn("NODE_VERSION: 24.14.0", self.test_workflow)
        self.assertIn("uses: actions/setup-node@v6", self.test_workflow)
        self.assertIn("node-version: ${{ env.NODE_VERSION }}", self.test_workflow)
        self.assertIn("node --version", self.test_workflow)
        self.assertIn("npm --version", self.test_workflow)
        self.assertIn("cargo test -r --workspace", self.test_workflow)
        self.assertIn("fixture-test --profile all", self.test_workflow)
        for removed in (
            "wasm-stack-test",
            "f2llm-native-lavapipe",
            "DIOXUS_CLI_VERSION",
            "rustup target add wasm32-unknown-unknown",
            ".github-cache/home/.dx",
        ):
            self.assertNotIn(removed, self.test_workflow)

    def test_render_bundle_probes_staged_public_directory_before_archive(self) -> None:
        build_bundle = self.render_workflow.split("  build-bundle:\n", 1)[1].split(
            "  f2llm-native-lavapipe:\n", 1
        )[0]
        build = build_bundle.index("cargo run --release -p xtask -- dist-server")
        probe = build_bundle.index("cargo run --release -p xtask-full -- wasm-stack-test")
        archive = build_bundle.index("tar -C .jbotci-build/render")
        self.assertLess(build, probe)
        self.assertLess(probe, archive)
        self.assertIn("NODE_VERSION: 24.14.0", build_bundle)
        self.assertIn("uses: actions/setup-node@v6", build_bundle)
        self.assertIn("node-version: ${{ env.NODE_VERSION }}", build_bundle)
        build_dependencies = build_bundle.split(
            "      - name: Install build dependencies\n", 1
        )[1].split("      - name: Check out source\n", 1)[0]
        self.assertNotIn(" nodejs", build_dependencies)
        self.assertNotIn(" npm", build_dependencies)
        self.assertIn("--no-build", build_bundle)
        self.assertIn("--public-dir .jbotci-build/render/public", build_bundle)

    def test_image_publication_waits_for_bundle_and_native_goldens(self) -> None:
        self.assertIn("  f2llm-native-lavapipe:\n", self.render_workflow)
        self.assertIn(
            "cargo test -r -p jbotci-f2llm-runtime --features native",
            self.render_workflow,
        )
        publish = self.render_workflow.split("  publish-image:\n", 1)[1]
        self.assertIn(
            "needs:\n      - build-bundle\n      - f2llm-native-lavapipe",
            publish,
        )

    def test_target_cache_saves_require_a_successful_job(self) -> None:
        for workflow, name, condition in (
            (self.test_workflow, "test", "success() && github.event_name != 'pull_request'"),
            (self.render_workflow, "render", "success()"),
        ):
            self.assertIn(
                f"      - name: Save {name} target cache\n"
                f"        if: ${{{{ {condition} }}}}\n"
                "        uses: actions/cache/save@v5\n",
                workflow,
            )

    def test_target_cache_keys_exclude_earlier_designs(self) -> None:
        for workflow, name in (
            (self.test_workflow, "test"),
            (self.render_workflow, "render"),
        ):
            self.assertEqual(workflow.count(f"jbotci-{name}-target-v5-"), 2)
            self.assertNotIn(f"jbotci-{name}-target-v4-", workflow)
            self.assertNotIn(f"jbotci-{name}-target-${{{{", workflow)

    def test_each_cached_job_records_content_before_saving(self) -> None:
        for workflow in (self.test_workflow, self.render_workflow):
            marker = workflow.index("tools.cargo_inputs begin")
            restore = workflow.index("tools.cargo_inputs restore")
            record = workflow.index("tools.cargo_inputs save")
            save = workflow.index("uses: actions/cache/save@v5", record)
            self.assertLess(marker, restore)
            self.assertLess(restore, record)
            self.assertLess(record, save)
            self.assertNotIn("cargo-cache-mtimes", workflow)
            self.assertNotIn("cargo-provenance", workflow)
            self.assertIn("needs: build-image", workflow)
            self.assertIn("image: ${{ needs.build-image.outputs.image }}", workflow)
            self.assertIn("tools.cargo_cache_identity identity", workflow)
            self.assertIn("python3 strace zstd", workflow)
            self.assertIn("      JBOTCI_GIT_COMMIT: ${{ github.sha }}", workflow)
        self.assertNotIn(".github-cache/home/.dx", self.render_workflow)
        release = REPOSITORY_ROOT / ".github/workflows/prepare-cli-release.yml"
        self.assertNotIn("actions/cache", release.read_text())
        self.assertFalse((REPOSITORY_ROOT / ".github/actions/cargo-cache-mtimes").exists())

    def test_cache_regression_target_is_set_in_the_runner_step(self) -> None:
        self.assertIn(
            "      - name: Run release tooling tests\n"
            "        env:\n"
            "          CARGO_TARGET_DIR: ${{ runner.temp }}/cargo-cache\n",
            self.test_workflow,
        )


if __name__ == "__main__":
    unittest.main()
