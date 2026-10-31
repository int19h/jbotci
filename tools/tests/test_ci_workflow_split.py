"""Regression checks for the separation between PR and deployment gates."""

from __future__ import annotations

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
