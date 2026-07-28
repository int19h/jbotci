#!/usr/bin/env python3
"""Run every public example and optionally require a wheel-only import."""

from __future__ import annotations

import argparse
import ast
import os
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
if str(PACKAGE_ROOT) not in sys.path:
    # Direct script execution exposes tools/, while package imports expose its
    # parent. Normalize both entry points onto the same namespace import.
    sys.path.insert(0, str(PACKAGE_ROOT))

from tools.installed_package import assert_installed_package

EXAMPLES = tuple(sorted((PACKAGE_ROOT / "examples").glob("*.py")))


def parse_args() -> argparse.Namespace:
    """Parse example-runner arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require-wheel",
        action="store_true",
        help="reject an import whose package files reside in this checkout",
    )
    parser.add_argument(
        "--artifact-checks",
        action="store_true",
        help="run metadata, inventory, package-data, and strict typing checks",
    )
    return parser.parse_args()


def assert_examples_are_public_and_self_contained() -> None:
    """Reject private imports and repository-relative data dependencies."""
    for example in EXAMPLES:
        tree = ast.parse(
            example.read_text(encoding="utf-8"),
            filename=str(example),
        )
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                assert all(
                    not alias.name.startswith("jbotci._native")
                    for alias in node.names
                ), example
            elif isinstance(node, ast.ImportFrom):
                assert not (
                    node.module is not None
                    and node.module.startswith("jbotci._native")
                ), example
                assert not (
                    node.module == "jbotci"
                    and any(alias.name == "_native" for alias in node.names)
                ), example
            elif isinstance(node, ast.Name):
                assert node.id != "__file__", example


def main() -> int:
    """Run examples with an outside-tree working directory."""
    if not EXAMPLES:
        raise RuntimeError("no executable Python examples were found")
    args = parse_args()
    assert_examples_are_public_and_self_contained()
    with tempfile.TemporaryDirectory(prefix="jbotci-installed-examples-") as directory:
        outside = Path(directory)
        environment = os.environ.copy()
        for name in ("PYTHONHOME", "PYTHONPATH", "MYPYPATH"):
            environment.pop(name, None)
        if args.require_wheel:
            check = (
                "from pathlib import Path; import jbotci; "
                f"root=Path({str(PACKAGE_ROOT)!r}).resolve(); "
                "installed=Path(jbotci.__file__).resolve(); "
                "assert not installed.is_relative_to(root), (installed, root)"
            )
            subprocess.run(
                [sys.executable, "-c", check],
                cwd=outside,
                env=environment,
                check=True,
            )
        if args.artifact_checks:
            workspace_manifest = PACKAGE_ROOT.parents[1] / "Cargo.toml"
            workspace = tomllib.loads(
                workspace_manifest.read_text(encoding="utf-8")
            )
            expected_version = workspace["workspace"]["package"]["version"]
            assert isinstance(expected_version, str)
            assert_installed_package(
                source_package_root=PACKAGE_ROOT,
                expected_version=expected_version,
            )
        for example in EXAMPLES:
            subprocess.run(
                [sys.executable, str(example)],
                cwd=outside,
                env=environment,
                check=True,
            )
        if args.artifact_checks:
            subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "mypy",
                    "--strict",
                    "--no-incremental",
                    "--cache-dir",
                    str(outside / "mypy-cache"),
                    str(PACKAGE_ROOT / "tests" / "typecheck.py"),
                    str(PACKAGE_ROOT / "examples"),
                ],
                cwd=outside,
                env=environment,
                check=True,
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
