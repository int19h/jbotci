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
    assert len(diagnostics) == 17, result.stdout
    assert sum("[attr-defined]" in line for line in diagnostics) == 2
    assert sum("[call-arg]" in line for line in diagnostics) == 5
    assert sum("[arg-type]" in line for line in diagnostics) == 3
    assert sum("[misc]" in line for line in diagnostics) == 7


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


def test_omitted_syntax_variant_fails_exhaustive_match_in_strict_mypy() -> None:
    """The closed generated union reports an unhandled concrete variant."""
    fixture = (
        PACKAGE_ROOT
        / "tests"
        / "typing_failures"
        / "syntax_non_exhaustive.py"
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
    assert len(diagnostics) == 1, result.stdout
    assert "[arg-type]" in diagnostics[0]
    assert "LinkedSumtiSyntaxEmptyLinkedSumti" in diagnostics[0]


def test_omitted_parser_payload_fails_exhaustive_match_in_strict_mypy() -> None:
    """The closed completion union reports its unhandled named-token variant."""
    fixture = (
        PACKAGE_ROOT
        / "tests"
        / "typing_failures"
        / "syntax_parser_non_exhaustive.py"
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
    assert len(diagnostics) == 1, result.stdout
    assert "[arg-type]" in diagnostics[0]
    assert "SyntaxExpectedTokenNamed" in diagnostics[0]


def test_jvozba_misuse_is_rejected_by_strict_mypy() -> None:
    """Typed inputs reject strings, unordered iterables, generators, and enum text."""

    fixture = PACKAGE_ROOT / "tests" / "typing_failures" / "jvozba_misuse.py"
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
    assert diagnostics
    assert all(
        "[arg-type]" in line or "[list-item]" in line
        for line in diagnostics
    )
    assert any(
        "Sequence[" in line and "FixedRafsi" in line
        for line in diagnostics
    )
    assert any("JvozbaMode" in line for line in diagnostics)


def test_omitted_jvozba_error_fails_closed_union_narrowing() -> None:
    """The closed error union reports its unhandled compound-failure variant."""

    fixture = (
        PACKAGE_ROOT
        / "tests"
        / "typing_failures"
        / "jvozba_non_exhaustive.py"
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
    assert diagnostics
    assert all("[arg-type]" in line for line in diagnostics)
    assert all("CouldNotBuildCompound" in line for line in diagnostics)


def test_omitted_reference_target_fails_closed_union_narrowing() -> None:
    """The closed target union reports its unhandled vague variant."""

    fixture = (
        PACKAGE_ROOT
        / "tests"
        / "typing_failures"
        / "references_non_exhaustive.py"
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
    assert len(diagnostics) == 1, result.stdout
    assert "[arg-type]" in diagnostics[0]
    assert "VagueReferenceTarget" in diagnostics[0]
