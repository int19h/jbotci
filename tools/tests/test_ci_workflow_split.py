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

    def test_target_cache_saves_require_a_successful_job_and_stamp(self) -> None:
        self.assertIn(
            "      - name: Stamp test target cache\n"
            "        id: target-cache-stamp\n"
            "        if: ${{ success() && github.event_name != 'pull_request' }}\n",
            self.test_workflow,
        )
        self.assertIn(
            "      - name: Save test target cache\n"
            "        if: ${{ success() && github.event_name != 'pull_request' "
            "&& steps.target-cache-stamp.outcome == 'success' }}\n"
            "        uses: actions/cache/save@v5\n",
            self.test_workflow,
        )
        self.assertIn(
            "      - name: Stamp render target cache\n"
            "        id: target-cache-stamp\n"
            "        if: ${{ success() }}\n",
            self.render_workflow,
        )

        self.assertIn(
            "      - name: Save render target cache\n"
            "        if: ${{ success() && steps.target-cache-stamp.outcome == 'success' }}\n"
            "        uses: actions/cache/save@v5\n",
            self.render_workflow,
        )

    def test_target_cache_keys_exclude_caches_saved_before_the_fix(self) -> None:
        for workflow, name in (
            (self.test_workflow, "test"),
            (self.render_workflow, "render"),
        ):
            self.assertEqual(workflow.count(f"jbotci-{name}-target-v2-"), 3)
            self.assertNotIn(f"jbotci-{name}-target-${{{{", workflow)


if __name__ == "__main__":
    unittest.main()
