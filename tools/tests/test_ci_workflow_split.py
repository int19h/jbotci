"""Regression checks for the separation between PR and deployment gates."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

from markdown_it import MarkdownIt

from tools.cargo_inputs_trace import dioxus_cli_version, shared_rustup_home
from tools.release_compiler import toolchain_channel
from tools.workflow_images import (
    dockerfile_rust_versions, named_dockerfiles, render_dockerfiles, require_channel,
    workflow_dockerfiles, workflow_rust_versions,
)
from tools.workflow_pins import PinError, exactly_one, job_env_values


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = REPOSITORY_ROOT / ".github/workflows"
TEST_WORKFLOW = WORKFLOWS / "test.yml"
RENDER_WORKFLOW = WORKFLOWS / "render-image.yml"


_DOCUMENTED_VERSION = re.compile(r"\b(?:Rust|Cargo)\s+v?([0-9][A-Za-z0-9.+-]*)")


def documented_versions(markdown: str) -> set[str]:
    """Read version mentions from Markdown text and inline code."""
    versions = set()
    for block in MarkdownIt("commonmark").parse(markdown):
        if block.type != "inline":
            continue
        line = "".join(
            token.content if token.type in ("text", "code_inline") else
            " " if token.type in ("softbreak", "hardbreak") else ""
            for token in block.children or []
        )
        versions.update(version.removesuffix(".") for version in _DOCUMENTED_VERSION.findall(line))
    return versions


class CiWorkflowSplitTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.test_workflow = TEST_WORKFLOW.read_text(encoding="utf-8")
        cls.render_workflow = RENDER_WORKFLOW.read_text(encoding="utf-8")
        cls.channel = toolchain_channel((REPOSITORY_ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))

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

    def test_release_tooling_uses_the_pinned_build_image(self) -> None:
        job = self.test_workflow.split("  cli-release-tool:\n", 1)[1].split("\n  build-image:\n", 1)[0]
        self.assertIn("needs: build-image\n", job)
        self.assertIn("image: ${{ needs.build-image.outputs.image }}\n", job)
        self.assertNotIn("sudo apt-get", job)
        self.assertIn("shellcheck", job)
        self.assertIn("g++", job)
        self.assertIn("jq", job)

    def test_cache_regression_target_is_set_in_the_runner_step(self) -> None:
        self.assertIn(
            "      - name: Run release tooling tests\n"
            "        env:\n"
            "          CARGO_TARGET_DIR: ${{ runner.temp }}/cargo-cache\n",
            self.test_workflow,
        )

    def test_workflow_rust_pins_match_the_toolchain_channel(self) -> None:
        version = self.channel
        self.assertRegex(version, r"^[0-9]+\.[0-9]+\.[0-9]+$")
        for path in sorted(WORKFLOWS.iterdir()):
            if not path.is_file() or path.suffix not in (".yml", ".yaml"):
                continue
            with self.subTest(file=path.name):
                for pin in workflow_rust_versions(path.read_text(encoding="utf-8")):
                    self.assertEqual(pin, version)

        prepare = (WORKFLOWS / "prepare-cli-release.yml").read_text(encoding="utf-8")
        self.assertEqual(job_env_values(self.test_workflow, "RUST_BUILD_IMAGE"), [
            ("build-image", f"rust:{version}-bookworm"),
        ])
        self.assertEqual(job_env_values(prepare, "RUST_BUILD_IMAGE"), [
            ("build-linux", f"rust:{version}-alpine3.24"),
        ])

    def test_documented_rust_and_cargo_versions_match_the_toolchain_channel(self) -> None:
        for name in ("README.md", "docs/ci/cargo-cache.md"):
            with self.subTest(file=name):
                text = (REPOSITORY_ROOT / name).read_text(encoding="utf-8")
                self.assertEqual(documented_versions(text), {self.channel})

    def test_formatted_stale_version_is_detected_beside_current_version(self) -> None:
        for mention in ("**Rust** 1.99.0", "Rust v1.99.0", "[Rust](https://www.rust-lang.org/) `v1.99.0`"):
            with self.subTest(mention=mention):
                text = f"CI uses Rust 1.100.0.\n\nUse {mention} for the manual job.\n"
                self.assertEqual(documented_versions(text), {"1.99.0", "1.100.0"})

    def test_render_image_disables_rustup_auto_install(self) -> None:
        version = exactly_one(
            self.render_workflow,
            "DIOXUS_CLI_VERSION",
            re.compile(r"[0-9]+(?:\.[0-9]+)+"),
        )
        self.assertEqual(version, dioxus_cli_version())
        homes = [
            exactly_one(workflow, "RUSTUP_HOME", re.compile(r"/[^\s]+"))
            for workflow in (self.test_workflow, self.render_workflow)
        ]
        self.assertEqual(homes, [shared_rustup_home()] * 2)
        self.assertIn('RUSTUP_AUTO_INSTALL: "0"\n', self.render_workflow)

    def test_pinned_keys_reject_every_hidden_declaration(self) -> None:
        pins = (
            ("DIOXUS_CLI_VERSION", "0.7.9", "0.8.0", re.compile(r"[0-9]+(?:\.[0-9]+)+")),
            ("RUSTUP_HOME", "/usr/local/rustup", "/tmp/other-rustup", re.compile(r"/[^\s]+")),
            ("RUSTUP_TOOLCHAIN", "1.100.0", "1.99.0", re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")),
            ("CARGO_TARGET_DIR", "/tmp/cargo-target/test", "/tmp/other", re.compile(r"/[^\s]+")),
        )
        for key, good, bad, pattern in pins:
            with self.subTest(key=key):
                block = f"env:\n  {key}: {good} # pin\n"
                flow = f"env: {{{key}: {good}}}\n"
                quoted = f"env: {{\"{key}\": \"{good}\"}}\n"
                nested = (
                    f"env:\n  {key}: {good}\n"
                    f"jobs:\n  build:\n    env: {{\"{key}\": \"{bad}\"}}\n"
                )
                malformed = f"env:\n  {key}: garbage\n"
                self.assertEqual(exactly_one(block, key, pattern), good)
                self.assertEqual(exactly_one(flow, key, pattern), good)
                self.assertEqual(exactly_one(quoted, key, pattern), good)
                with self.assertRaises(PinError):
                    exactly_one(nested, key, pattern)
                with self.assertRaises(PinError):
                    exactly_one(malformed, key, pattern)

    def test_yaml_pins_include_image_tags_and_rust_versions(self) -> None:
        workflow = (
            "env: {RUST_BASE: 'rust:1.100.0-trixie', RUST_VERSION: '1.99.0', NODE_VERSION: '24.14.0'}\n"
            "jobs:\n"
            "  build:\n"
            "    container: {image: 'docker.io/library/rust:1.99.0-slim-bookworm'}\n"
            "    env: {OTHER_BASE: 'registry.example:5000/mirror/rust:1.100.0-bookworm'}\n"
            "  release:\n"
            "    container: 'rust:1.100.0-slim-bookworm'\n"
            "    env: {RUSTUP_TOOLCHAIN: '1.99.1'}\n"
        )
        self.assertEqual(
            workflow_rust_versions(workflow),
            ["1.100.0", "1.99.0", "1.100.0", "1.99.0", "1.99.1", "1.100.0"],
        )
        with self.assertRaisesRegex(
            PinError,
            "job build container image value 'docker.io/library/rust:latest' is not a full Rust version. "
            "The tag must start with a full Rust version.",
        ):
            workflow_rust_versions(workflow.replace("rust:1.99.0-slim-bookworm", "rust:latest"))
        with self.assertRaisesRegex(PinError, "Declare a Rust version in RUSTUP_TOOLCHAIN"):
            workflow_rust_versions(workflow.replace("1.99.1", "stable"))

    def test_only_workflow_and_job_pin_fields_supply_versions(self) -> None:
        workflow = (
            "name: 'Example image rust:1.99.0-bookworm'\n"
            "jobs:\n"
            "  build:\n"
            "    container: {image: '${{ needs.image.outputs.image }}'}\n"
            "    services: {builder: {image: 'rust:1.99.0-bookworm'}}\n"
            "    env: {RUST_BUILD_IMAGE: 'rust:1.100.0-bookworm'}\n"
            "    steps: [{env: {RUSTUP_TOOLCHAIN: '1.99.0'}}]\n"
        )
        self.assertEqual(workflow_rust_versions(workflow), ["1.100.0"])

    def test_yaml_rust_image_pins_reject_literal_digests(self) -> None:
        digest = "sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6"
        for image in ("docker.io/library/rust:1.100.0-bookworm@" + digest, "rust@" + digest):
            for workflow in (
                f"jobs:\n  build:\n    container: {image}\n",
                f"env: {{RUST_BASE: '{image}'}}\njobs: {{build: {{}}}}\n",
                f"jobs:\n  build:\n    env: {{RUST_BASE: '{image}'}}\n",
            ):
                with self.subTest(workflow=workflow):
                    with self.assertRaisesRegex(PinError, "Use the image resolver"):
                        workflow_rust_versions(workflow)

    def test_a_registry_port_does_not_make_the_repository_rust(self) -> None:
        workflow = (
            "jobs:\n"
            "  build:\n"
            "    container: 'rust:5000/team/alpine:3.24'\n"
            "    env:\n"
            "      OTHER: myrust:1.100.0\n"
            "      BASE: not-rust:1.100.0\n"
        )
        self.assertEqual(workflow_rust_versions(workflow), [])
        broken = "jobs:\n  build:\n    env: {LANGUAGE: rust}\n"
        with self.assertRaisesRegex(
            PinError,
            "job build env LANGUAGE value 'rust' is not a full Rust version. "
            "The tag must start with a full Rust version.",
        ):
            workflow_rust_versions(broken)

    def test_named_dockerfiles_match_the_toolchain_channel(self) -> None:
        paths = named_dockerfiles(REPOSITORY_ROOT)
        self.assertTrue(paths)
        for path in paths:
            source = path.relative_to(REPOSITORY_ROOT).as_posix()
            with self.subTest(file=source):
                versions = dockerfile_rust_versions(path.read_text(encoding="utf-8"), source)
                require_channel(versions, self.channel, source)

    def test_a_stale_dockerfile_rust_image_fails(self) -> None:
        source = "deploy/render/Dockerfile"
        for text in (
            "FROM rust:1.99.0-bookworm\n",
            "FROM rust:1.99.0-bookworm AS build\n",
        ):
            with self.subTest(text=text):
                versions = dockerfile_rust_versions(text, source)
                self.assertEqual(versions, ["1.99.0"])
                with self.assertRaisesRegex(PinError, "deploy/render/Dockerfile pins Rust 1.99.0"):
                    require_channel(versions, "1.100.0", source)

    def test_render_manifest_names_a_dockerfile(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            (repo / "render.yaml").write_text(
                "services:\n  - dockerfilePath: ./deploy/sample/Dockerfile\n",
                encoding="utf-8",
            )
            paths = render_dockerfiles(repo)
        self.assertEqual(
            [path.relative_to(repo).as_posix() for path in paths],
            ["deploy/sample/Dockerfile"],
        )

    def test_declared_dockerfile_paths_exclude_shell_commands(self) -> None:
        text = (
            "jobs:\n"
            "  build:\n"
            "    steps:\n"
            "      - run: docker build -f deploy/stale/Dockerfile .\n"
            "      - uses: docker/build-push-action@v7\n"
            "        with:\n"
            "          file: deploy/render/Dockerfile.runtime\n"
        )
        paths = workflow_dockerfiles(REPOSITORY_ROOT, text)
        self.assertEqual(
            [path.relative_to(REPOSITORY_ROOT).as_posix() for path in paths],
            ["deploy/render/Dockerfile.runtime"],
        )


class TracedTargetTests(unittest.TestCase):
    def test_traced_steps_use_the_session_target(self) -> None:
        install = RENDER_WORKFLOW.read_text(encoding="utf-8").split(
            "      - name: Install Dioxus CLI\n", 1,
        )[1].split("\n      - name:", 1)[0]
        self.assertNotIn("tools.cargo_inputs trace", install)
        self.assertIn("cargo install dioxus-cli", install)
        workflows = {
            "test": (TEST_WORKFLOW.read_text(encoding="utf-8"), "test"),
            "render": (RENDER_WORKFLOW.read_text(encoding="utf-8"), "deploy"),
        }
        for name, (workflow, lane) in workflows.items():
            with self.subTest(workflow=name):
                needle = f"printf 'CARGO_TARGET_DIR=%s/cargo-target/{lane}\\n'"
                self.assertEqual(workflow.count(needle), 1)
                other = "deploy" if lane == "test" else "test"
                other_needle = f"printf 'CARGO_TARGET_DIR=%s/cargo-target/{other}\\n'"
                self.assertEqual(workflow.count(other_needle), 0)


if __name__ == "__main__":
    unittest.main()
