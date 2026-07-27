#!/usr/bin/env python3
"""Install one wheel into a clean outside-tree venv and test the artifact."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tomllib
import venv
from pathlib import Path


def parse_args() -> argparse.Namespace:
    """Parse wheel-test arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--package-root", required=True, type=Path)
    parser.add_argument("--workspace-root", required=True, type=Path)
    parser.add_argument("--venv", required=True, type=Path)
    return parser.parse_args()


def venv_python(environment: Path) -> Path:
    """Return the interpreter created by :mod:`venv` on this platform."""
    if os.name == "nt":
        return environment / "Scripts" / "python.exe"
    return environment / "bin" / "python"


def main() -> int:
    """Create the clean environment and delegate to the shared installed test."""
    args = parse_args()
    wheel = args.wheel.resolve()
    package_root = args.package_root.resolve()
    workspace_root = args.workspace_root.resolve()
    environment = args.venv.resolve()
    assert wheel.is_file(), wheel
    assert not environment.is_relative_to(workspace_root), (
        environment,
        workspace_root,
    )
    project_file = package_root / "pyproject.toml"
    if not project_file.is_file():
        # Maturin relocates the binding pyproject to the sdist root while
        # preserving package-only tests and tools under bindings/python.
        project_file = workspace_root / "pyproject.toml"
    assert project_file.is_file(), project_file

    if environment.exists():
        assert environment.is_dir(), environment
        assert (environment / "pyvenv.cfg").is_file(), environment
        shutil.rmtree(environment)
    environment.parent.mkdir(parents=True, exist_ok=True)
    venv.EnvBuilder(with_pip=True).create(environment)
    python = venv_python(environment)

    project = tomllib.loads(
        project_file.read_text(encoding="utf-8")
    )
    test_dependencies = project["dependency-groups"]["dev"]
    assert isinstance(test_dependencies, list)
    assert test_dependencies and all(
        isinstance(value, str) for value in test_dependencies
    )

    clean_environment = os.environ.copy()
    for name in ("PYTHONHOME", "PYTHONPATH", "MYPYPATH", "VIRTUAL_ENV"):
        clean_environment.pop(name, None)
    subprocess.run(
        [
            str(python),
            "-m",
            "pip",
            "--disable-pip-version-check",
            "install",
            *test_dependencies,
        ],
        cwd=environment.parent,
        env=clean_environment,
        check=True,
    )
    subprocess.run(
        [
            str(python),
            "-m",
            "pip",
            "--disable-pip-version-check",
            "install",
            "--no-deps",
            str(wheel),
        ],
        cwd=environment.parent,
        env=clean_environment,
        check=True,
    )

    harness = package_root / "tools" / "run_installed_examples.py"
    subprocess.run(
        [
            str(python),
            str(harness),
            "--require-wheel",
            "--artifact-checks",
        ],
        cwd=environment.parent,
        env=clean_environment,
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
