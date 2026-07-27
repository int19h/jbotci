"""Executable public documentation examples."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from tools.run_installed_examples import EXAMPLES

PACKAGE_ROOT = Path(__file__).resolve().parents[1]


def test_every_public_example_runs_outside_the_source_tree() -> None:
    """Execute the same discovery-based runner used by the wheel gate."""
    result = subprocess.run(
        [
            sys.executable,
            str(PACKAGE_ROOT / "tools" / "run_installed_examples.py"),
        ],
        cwd=PACKAGE_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_every_public_example_passes_strict_mypy() -> None:
    """Keep executable examples aligned with the packaged typing contract."""
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            str(PACKAGE_ROOT / "examples"),
        ],
        cwd=PACKAGE_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_public_docs_link_every_executable_example_without_private_imports() -> None:
    """Keep documentation links and executable example discovery aligned."""
    documents = (
        PACKAGE_ROOT / "README.md",
        *sorted((PACKAGE_ROOT / "docs").glob("*.md")),
    )
    text = "\n".join(path.read_text(encoding="utf-8") for path in documents)
    mentioned_examples = {
        example.name for example in EXAMPLES if example.name in text
    }
    assert mentioned_examples == {example.name for example in EXAMPLES}
    assert "jbotci._native" not in text
    assert "```python" not in text
    assert ">>>" not in text
