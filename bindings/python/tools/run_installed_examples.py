#!/usr/bin/env python3
"""Run every public example and optionally require a wheel-only import."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = tuple(sorted((PACKAGE_ROOT / "examples").glob("*.py")))


def parse_args() -> argparse.Namespace:
    """Parse example-runner arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--require-wheel",
        action="store_true",
        help="reject an import whose package files reside in this checkout",
    )
    return parser.parse_args()


def main() -> int:
    """Run examples with an outside-tree working directory."""
    if not EXAMPLES:
        raise RuntimeError("no executable Python examples were found")
    with tempfile.TemporaryDirectory(prefix="jbotci-installed-examples-") as directory:
        outside = Path(directory)
        environment = os.environ.copy()
        environment.pop("PYTHONPATH", None)
        if parse_args().require_wheel:
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
        for example in EXAMPLES:
            subprocess.run(
                [sys.executable, str(example)],
                cwd=outside,
                env=environment,
                check=True,
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
