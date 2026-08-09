"""File-text regressions; workflow gates enforce runtime and static behavior."""

from __future__ import annotations

import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PREPARE_WORKFLOW = (
    REPOSITORY_ROOT / ".github" / "workflows" / "prepare-cli-release.yml"
)
TEST_WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "test.yml"
CARGO_COMMAND = (
    'RUSTFLAGS="-C target-feature=-crt-static" cargo rustc --release --locked '
    "-p jbotci --bin jbotci -- -C target-feature=+crt-static"
)
COMBINED_TEST_COMMAND = (
    "python3 -m unittest tools.tests.test_cli_release "
    "tools.tests.test_prepare_cli_release_workflow -v"
)


class PrepareCliReleaseWorkflowTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = PREPARE_WORKFLOW.read_text(encoding="utf-8")
        cls.test_workflow = TEST_WORKFLOW.read_text(encoding="utf-8")
        cls.linux_job = cls.workflow.split("  build-linux:\n", 1)[1].split(
            "  build-macos:\n", 1
        )[0]
        cls.build_step = cls.linux_job.split(
            "      - name: Build in pinned native Alpine toolchain\n", 1
        )[1].split("      - name: Reject dynamically linked ELF output\n", 1)[0]

    def test_matrix_declares_both_native_alpine_bindgen_targets(self) -> None:
        self.assertIn(
            "            target: x86_64-unknown-linux-musl\n"
            "            bindgen_target: x86_64-alpine-linux-musl\n",
            self.linux_job,
        )
        self.assertIn(
            "            target: aarch64-unknown-linux-musl\n"
            "            bindgen_target: aarch64-alpine-linux-musl\n",
            self.linux_job,
        )
        self.assertEqual(self.linux_job.count("            bindgen_target:"), 2)

    def test_bindgen_target_is_passed_explicitly_into_the_container(self) -> None:
        self.assertEqual(
            self.build_step.count(
                '--env EXPECTED_BINDGEN_TARGET="${{ matrix.bindgen_target }}"'
            ),
            1,
        )

    def test_compiler_and_header_probes_precede_the_bindgen_override(self) -> None:
        apk_end = self.build_step.index("                zstd-dev\n")
        compiler_probe = self.build_step.index(
            'test "$(g++ -dumpmachine)" = "${EXPECTED_BINDGEN_TARGET}"'
        )
        header_probe = self.build_step.index(
            'printf "%s\\n" "#include <bits/c++config.h>" | clang++'
        )
        rust_target = self.build_step.index(
            '--target="${EXPECTED_RUST_HOST}"', header_probe
        )
        bindgen_target = self.build_step.index(
            '--target="${EXPECTED_BINDGEN_TARGET}"', rust_target
        )
        language = self.build_step.index("-x c++", bindgen_target)
        syntax_only = self.build_step.index("-fsyntax-only", bindgen_target)
        probe_end = self.build_step.index("                -\n", syntax_only)
        bindgen_override = self.build_step.index(
            'export BINDGEN_EXTRA_CLANG_ARGS="--target=${EXPECTED_BINDGEN_TARGET}"'
        )
        cargo_command = self.build_step.index(CARGO_COMMAND)
        probe_targets = [
            line.strip().removesuffix(" \\")
            for line in self.build_step[header_probe:probe_end].splitlines()
            if line.strip().startswith("--target=")
        ]

        self.assertLess(
            apk_end,
            compiler_probe,
            "g++ must not be queried until build-base has been installed",
        )
        self.assertLess(compiler_probe, header_probe)
        self.assertLess(header_probe, rust_target)
        self.assertLess(rust_target, bindgen_target)
        self.assertLess(bindgen_target, language)
        self.assertLess(language, syntax_only)
        self.assertLess(syntax_only, bindgen_override)
        self.assertLess(bindgen_override, cargo_command)
        self.assertEqual(
            probe_targets,
            [
                '--target="${EXPECTED_RUST_HOST}"',
                '--target="${EXPECTED_BINDGEN_TARGET}"',
            ],
        )

    def test_split_crt_flags_wrap_one_exact_native_cargo_invocation(self) -> None:
        cargo_commands = [
            line.strip()
            for line in self.build_step.splitlines()
            if line.strip().startswith(("cargo ", "RUSTFLAGS="))
        ]
        self.assertEqual(cargo_commands, [CARGO_COMMAND])
        self.assertNotIn("--target", cargo_commands[0])
        self.assertNotIn("CARGO_BUILD_TARGET", self.build_step)
        self.assertLess(
            self.build_step.index("target-feature=-crt-static"),
            self.build_step.index("target-feature=+crt-static"),
        )

    def test_linux_consumers_keep_the_native_target_release_path(self) -> None:
        self.assertEqual(
            self.build_step.count('build_root="${RUNNER_TEMP}/linux-build"'), 1
        )
        self.assertEqual(
            self.build_step.count("--env CARGO_TARGET_DIR=/out/target"), 1
        )
        self.assertEqual(
            self.build_step.count('--volume "${build_root}:/out"'), 1
        )
        self.assertEqual(self.linux_job.count("target/release/jbotci"), 3)
        self.assertIn(
            'test -x "${build_root}/target/release/jbotci"', self.linux_job
        )
        self.assertEqual(
            self.linux_job.count(
                'binary="${RUNNER_TEMP}/linux-build/target/release/jbotci"'
            ),
            2,
        )

    def test_normal_ci_runs_this_module_with_the_release_tool_tests(self) -> None:
        self.assertEqual(self.test_workflow.count(COMBINED_TEST_COMMAND), 1)


if __name__ == "__main__":
    unittest.main()
