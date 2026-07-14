#!/usr/bin/env python3
"""Verify issue #368 CLI compatibility and the new default.

The probes intentionally include the default gentufa web text, whose semantic
builder failure is part of the compatibility contract just as much as success
output is.
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
from dataclasses import dataclass
from pathlib import Path


PROBES = (
    "mi klama",
    "mi nitcu lo tanxe",
    "naku ro da poi mlatu cu klama",
    ".ui mi klama",
    "mi pu klama",
    "do nelci mi .ibabo mi nelci do",
    "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa ro lo jmive kei ta'i lo racli",
)
UNSUPPORTED_PROBE = PROBES[-1]
UNSUPPORTED_DIAGNOSTIC = (
    b"semantic error: generated semantic builder does not yet support scoped connected tanru unit\n"
)


@dataclass(frozen=True)
class Outcome:
    returncode: int
    stdout: bytes
    stderr: bytes


def run(binary: Path, text: str, format_name: str | None) -> Outcome:
    command = [str(binary), "tersmu"]
    if format_name is not None:
        command.extend(("--format", format_name))
    command.append(text)
    result = subprocess.run(command, check=False, capture_output=True)
    return Outcome(result.returncode, result.stdout, result.stderr)


def require_equal(label: str, left: Outcome, right: Outcome) -> None:
    if left != right:
        raise ValueError(
            f"{label} differs: "
            f"status {left.returncode}/{right.returncode}, "
            f"stdout {len(left.stdout)}/{len(right.stdout)} bytes, "
            f"stderr {len(left.stderr)}/{len(right.stderr)} bytes"
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    args = parser.parse_args()

    for format_name in ("json", "tree"):
        for index, text in enumerate(PROBES, start=1):
            require_equal(
                f"{format_name} probe {index}",
                run(args.base_binary, text, format_name),
                run(args.current_binary, text, format_name),
            )
        print(f"{format_name} main-vs-PR byte identity: {len(PROBES)}/{len(PROBES)}")

    for index, text in enumerate(PROBES, start=1):
        require_equal(
            f"default probe {index}",
            run(args.current_binary, text, None),
            run(args.current_binary, text, "tree+proj"),
        )
    print(
        "current default matches explicit tree+proj: "
        f"{len(PROBES)}/{len(PROBES)}"
    )

    unsupported = run(args.base_binary, UNSUPPORTED_PROBE, "json")
    if unsupported.returncode == 0 or unsupported.stdout:
        raise ValueError("unsupported probe no longer fails without stdout")
    if unsupported.stderr != UNSUPPORTED_DIAGNOSTIC:
        raise ValueError("unsupported probe diagnostic text changed")
    for label, outcome in (
        ("base tree", run(args.base_binary, UNSUPPORTED_PROBE, "tree")),
        ("current json", run(args.current_binary, UNSUPPORTED_PROBE, "json")),
        ("current tree", run(args.current_binary, UNSUPPORTED_PROBE, "tree")),
        ("current tree+proj", run(args.current_binary, UNSUPPORTED_PROBE, "tree+proj")),
        ("current default", run(args.current_binary, UNSUPPORTED_PROBE, None)),
    ):
        require_equal(f"unsupported diagnostic ({label})", unsupported, outcome)
    digest = hashlib.sha256(unsupported.stderr).hexdigest()
    print(
        "unsupported diagnostic byte identity: "
        f"status={unsupported.returncode}, stdout=0, stderr={len(unsupported.stderr)}, "
        f"sha256={digest}"
    )


if __name__ == "__main__":
    main()
