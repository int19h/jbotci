"""Negative strict-typing coverage for capabilities absent at runtime."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]


def test_dictionary_misuse_is_rejected_by_strict_mypy() -> None:
    """Keep native protocol stubs from claiming missing runtime operations."""
    fixture = PACKAGE_ROOT / "tests" / "typing_failures" / "dictionary_misuse.py"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            "--show-error-codes",
            str(fixture),
        ],
        cwd=PACKAGE_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    diagnostics = tuple(
        line for line in result.stdout.splitlines() if ": error:" in line
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert len(diagnostics) == 7, result.stdout
    assert sum("[attr-defined]" in line for line in diagnostics) == 2
    assert sum("[call-arg]" in line for line in diagnostics) == 5


def test_unordered_collection_inputs_are_rejected_by_strict_mypy() -> None:
    """Collection-bearing domain APIs require ordered Sequence inputs."""
    fixture = PACKAGE_ROOT / "tests" / "typing_failures" / "sequence_misuse.py"
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            "--show-error-codes",
            str(fixture),
        ],
        cwd=PACKAGE_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    diagnostics = tuple(
        line for line in result.stdout.splitlines() if ": error:" in line
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert len(diagnostics) == 4, result.stdout
    assert all("[arg-type]" in line for line in diagnostics)


def test_returned_only_domain_classes_reject_construction_in_strict_mypy() -> None:
    """Rust-owned domain products require an impossible sentinel argument."""
    fixture = (
        PACKAGE_ROOT
        / "tests"
        / "typing_failures"
        / "morphology_returned_only.py"
    )
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            "--show-error-codes",
            str(fixture),
        ],
        cwd=PACKAGE_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    diagnostics = tuple(
        line for line in result.stdout.splitlines() if ": error:" in line
    )
    assert result.returncode == 1, result.stdout + result.stderr
    assert len(diagnostics) == 8, result.stdout
    assert all("[call-arg]" in line for line in diagnostics)
