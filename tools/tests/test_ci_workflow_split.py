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

    def test_target_cache_keys_exclude_both_backdating_designs(self) -> None:
        for workflow, name in (
            (self.test_workflow, "test"),
            (self.render_workflow, "render"),
        ):
            self.assertEqual(workflow.count(f"jbotci-{name}-target-v3-"), 3)
            self.assertNotIn(f"jbotci-{name}-target-v2-", workflow)
            self.assertNotIn(f"jbotci-{name}-target-${{{{", workflow)

    def test_workflows_keep_checkout_timestamps(self) -> None:
        self.assertFalse((REPOSITORY_ROOT / ".github/actions/cargo-cache-mtimes").exists())
        for path in (REPOSITORY_ROOT / ".github/workflows").glob("*.yml"):
            workflow = path.read_text(encoding="utf-8")
            self.assertNotIn("cargo-cache-mtimes", workflow, path.name)
            self.assertNotIn("target-cache-stamp", workflow, path.name)
            self.assertNotIn("touch -", workflow, path.name)
        self.assertNotIn(".github-cache/home/.dx", self.render_workflow)
        for name in ("prepare-cli-release.yml",):
            path = REPOSITORY_ROOT / ".github/workflows" / name
            self.assertNotIn("actions/cache", path.read_text(encoding="utf-8"))

    def test_cache_regression_target_is_set_in_the_runner_step(self) -> None:
        self.assertIn(
            "      - name: Run release tooling tests\n"
            "        env:\n"
            "          CARGO_TARGET_DIR: ${{ runner.temp }}/cargo-cache\n",
            self.test_workflow,
        )


if __name__ == "__main__":
    unittest.main()
