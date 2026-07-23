#!/usr/bin/env python3
"""Write or check syntax runtime modules and stubs emitted by the Rust schema consumer."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from jbotci import _native

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
OUTPUTS = {
    PACKAGE_ROOT / "python" / "jbotci" / "syntax" / "strict.py": _native._syntax_STRICT_SOURCE,
    PACKAGE_ROOT / "python" / "jbotci" / "syntax" / "strict.pyi": _native._syntax_STRICT_STUB,
    PACKAGE_ROOT / "python" / "jbotci" / "syntax" / "recovered.py": _native._syntax_RECOVERED_SOURCE,
    PACKAGE_ROOT / "python" / "jbotci" / "syntax" / "recovered.pyi": _native._syntax_RECOVERED_STUB,
}


def parse_args() -> argparse.Namespace:
    """Parse generator arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when an emitted module is stale",
    )
    return parser.parse_args()


def main() -> int:
    """Write all generated syntax files or verify their exact contents."""
    args = parse_args()
    stale: list[Path] = []
    for path, expected in OUTPUTS.items():
        if args.check:
            if not path.is_file() or path.read_text(encoding="utf-8") != expected:
                stale.append(path)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(expected, encoding="utf-8")
    if stale:
        for path in stale:
            print(f"{path} is stale; run {Path(__file__).name}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
