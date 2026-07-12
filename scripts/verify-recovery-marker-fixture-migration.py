#!/usr/bin/env python3
"""Verify that fixture changes only collapse equal missing-marker runs."""

from __future__ import annotations

import argparse
import copy
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


TREE_PATH = ("expectations", "syntax", "recovered", "tree", "recovery-items")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, text=True, stdout=subprocess.PIPE
    ).stdout


def nested(document: dict[str, Any]) -> list[dict[str, Any]] | None:
    value: Any = document
    for key in TREE_PATH:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    if not isinstance(value, list):
        raise ValueError(f"{'.'.join(TREE_PATH)} is not an array")
    return value


def replace_nested(document: dict[str, Any], items: list[dict[str, Any]]) -> None:
    value: Any = document
    for key in TREE_PATH[:-1]:
        value = value[key]
    value[TREE_PATH[-1]] = items


def is_zero_width_missing(item: dict[str, Any]) -> bool:
    spans = item.get("byte-spans")
    return (
        item.get("kind") == "missing"
        and item.get("error-index") is not None
        and isinstance(spans, list)
        and bool(spans)
        and all(
            isinstance(span, list) and len(span) == 2 and span[0] == span[1]
            for span in spans
        )
    )


def collapse_equal_missing_runs(
    items: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    collapsed: list[dict[str, Any]] = []
    for item in items:
        if (
            collapsed
            and is_zero_width_missing(item)
            and is_zero_width_missing(collapsed[-1])
            and item == collapsed[-1]
        ):
            continue
        collapsed.append(item)
    return collapsed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="main", help="git revision with old fixtures")
    args = parser.parse_args()

    root = Path(git("rev-parse", "--show-toplevel").strip())
    changes = git(
        "diff", "--name-status", args.base, "--", "tests/fixtures/**/*.toml"
    )
    migrated = 0
    removed = 0
    failures: list[str] = []

    for change in changes.splitlines():
        status, relative = change.split("\t", 1)
        if status != "M":
            failures.append(f"{relative}: fixture status is {status}, expected M")
            continue
        current_path = root / relative
        old_text = git("show", f"{args.base}:{relative}")
        current_text = current_path.read_text(encoding="utf-8")
        if old_text == current_text:
            continue
        old = tomllib.loads(old_text)
        current = tomllib.loads(current_text)
        old_items = nested(old)
        current_items = nested(current)
        if old_items is None or current_items is None:
            failures.append(f"{relative}: changed outside a recovered tree expectation")
            continue
        expected_items = collapse_equal_missing_runs(old_items)
        expected = copy.deepcopy(old)
        replace_nested(expected, expected_items)
        if expected != current:
            failures.append(
                f"{relative}: parsed fixture differs by more than equal missing-marker collapse"
            )
            continue
        if expected_items == old_items:
            failures.append(f"{relative}: changed without collapsing an eligible run")
            continue
        migrated += 1
        removed += len(old_items) - len(expected_items)

    if failures:
        print("fixture migration verification failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    if migrated == 0:
        print("fixture migration verification found no migrated fixtures", file=sys.stderr)
        return 1
    print(
        f"verified {migrated} migrated fixtures; removed {removed} redundant markers; "
        "zero other parsed differences"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
