#!/usr/bin/env python3
"""Classify epoch-4 fixture deltas against the pre-regeneration baseline.

Only the two transformations approved by the epoch-4 plan are accepted as
mechanical. Every other changed fixture is printed for the manual ledger.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import tomllib
from typing import Any, Callable, Iterator


LeafPath = tuple[str, ...]
Leaf = tuple[LeafPath, Any]
Normalizer = Callable[[LeafPath, Any, bool], Any]

EXPECTED_MECHANICAL = {
    "sumti-connective": {
        "tests/fixtures/corpus/camxes/17625.toml",
        "tests/fixtures/corpus/camxes/18858.toml",
    },
    "relative-continuation": {
        "tests/fixtures/adhoc/v0/warnings/experimental/"
        "simpler-joi-relative-clause-connective.toml",
        "tests/fixtures/adhoc/v0/warnings/experimental/"
        "simpler-relative-clause-connective.toml",
    },
}
EXPECTED_MANUAL_COUNT = 65


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, text=True, stdout=subprocess.PIPE
    ).stdout


def leaves(value: Any, path: LeafPath = ()) -> Iterator[Leaf]:
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, (*path, key))
    else:
        yield path, value


def normalize_sumti_connective(path: LeafPath, value: Any, old: bool) -> Any:
    if not isinstance(value, str):
        return value
    if old:
        return (
            value.replace(
                "VuhuNonlogicalConnectiveSyntax", "SUMTI_CONNECTIVE_SYNTAX"
            )
            .replace("VuhuNonlogicalConnective", "SUMTI_CONNECTIVE")
            .replace("argument connective", "sumti connective")
        )
    return value.replace(
        "ExperimentalVuhuSumtiConnectiveSyntax", "SUMTI_CONNECTIVE_SYNTAX"
    ).replace("ExperimentalVuhuSumtiConnective", "SUMTI_CONNECTIVE")


def transform_relative_json(value: Any) -> Any:
    if isinstance(value, list):
        return [transform_relative_json(child) for child in value]
    if not isinstance(value, dict):
        return value
    transformed: dict[str, Any] = {}
    for key, child in value.items():
        if key != "ConnectedRelativeClauseTail":
            transformed[key] = transform_relative_json(child)
            continue
        connective = child["connective"]
        if "JekConnective" in connective:
            head = connective["JekConnective"]["ja"]
        else:
            head = connective["JoikConnective"]["JoiConnective"]["joi"]
        transformed["RelativeClauseExpContinuation"] = {
            "connective": {"head": transform_relative_json(head)},
            "inner": transform_relative_json(child["inner"]),
        }
    return transformed


def transform_relative_raw(value: str) -> str:
    value = value.replace(
        "ConnectedRelativeClauseTail(ConnectedRelativeClauseTailSyntax { "
        "connective: JekConnective(JekConnectiveSyntax { na: None, se: None, ja:",
        "RelativeClauseExpContinuation(RelativeClauseExpContinuationSyntax("
        "ExpRelativeContinuationSyntax { connective: "
        "ExpRelativeClauseConnectiveSyntax { na: None, se: None, head:",
    )
    value = value.replace(
        "ConnectedRelativeClauseTail(ConnectedRelativeClauseTailSyntax { "
        "connective: JoikConnective(JoiConnective(JoiConnectiveSyntax { "
        "se: None, joi:",
        "RelativeClauseExpContinuation(RelativeClauseExpContinuationSyntax("
        "ExpRelativeContinuationSyntax { connective: "
        "ExpRelativeClauseConnectiveSyntax { na: None, se: None, head:",
    )
    value = value.replace(
        ", nai: None }), inner: BridiRelativeClause",
        ", nai: None }, inner: BridiRelativeClause",
    )
    value = value.replace(
        ", nai: None })), inner: BridiRelativeClause",
        ", nai: None }, inner: BridiRelativeClause",
    )
    return value.replace("kuho: None })) })]", "kuho: None })) }))]")


def normalize_relative(path: LeafPath, value: Any, old: bool) -> Any:
    if not old or not isinstance(value, str):
        return value
    if path[-2:] == ("gentufa", "json"):
        return json.dumps(transform_relative_json(json.loads(value)), sort_keys=True)
    if path[-2:] == ("gentufa", "tree"):
        return value.replace(
            "ConnectedRelativeClauseTail", "ExpRelativeContinuation"
        )
    if path[-2:] == ("syntax", "raw"):
        return transform_relative_raw(value)
    return value


def normalized_leaves(
    document: dict[str, Any], normalizer: Normalizer, old: bool
) -> dict[LeafPath, Any]:
    result: dict[LeafPath, Any] = {}
    for path, value in leaves(document):
        normalized = normalizer(path, value, old)
        if not old and normalizer is normalize_relative and path[-2:] == (
            "gentufa",
            "json",
        ):
            normalized = json.dumps(json.loads(value), sort_keys=True)
        result[path] = normalized
    return result


def classify(
    old: dict[str, Any], current: dict[str, Any]
) -> str | None:
    for name, normalizer in (
        ("sumti-connective", normalize_sumti_connective),
        ("relative-continuation", normalize_relative),
    ):
        normalized_old = normalized_leaves(old, normalizer, True)
        normalized_current = normalized_leaves(current, normalizer, False)
        if normalized_old == normalized_current and old != current:
            return name
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", default="HEAD")
    args = parser.parse_args()

    paths = git("diff", "--name-only", args.baseline, "--", "tests/fixtures").splitlines()
    mechanical: dict[str, list[str]] = {
        "sumti-connective": [],
        "relative-continuation": [],
    }
    manual: list[str] = []
    for path_text in paths:
        path = Path(path_text)
        old = tomllib.loads(git("show", f"{args.baseline}:{path_text}"))
        current = tomllib.loads(path.read_text())
        classification = classify(old, current)
        if classification is None:
            manual.append(path_text)
        else:
            mechanical[classification].append(path_text)

    for name, classified_paths in mechanical.items():
        print(f"mechanical {name}: {len(classified_paths)}")
        for path in classified_paths:
            print(f"  {path}")
    print(f"manual: {len(manual)}")
    for path in manual:
        print(f"  {path}")
    actual_mechanical = {
        name: set(classified_paths)
        for name, classified_paths in mechanical.items()
    }
    if actual_mechanical != EXPECTED_MECHANICAL:
        print("error: mechanical fixture set differs from the reviewed C3 set")
        return 1
    if len(manual) != EXPECTED_MANUAL_COUNT:
        print(
            "error: manual fixture count differs from the reviewed C3 ledger "
            f"({EXPECTED_MANUAL_COUNT})"
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
