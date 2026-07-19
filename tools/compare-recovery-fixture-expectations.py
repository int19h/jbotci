#!/usr/bin/env python3
"""Classify recovery fixture rewrites before applying generated expectations.

The candidate tree is produced outside the worktree with the normal,
facet-aware ``fixture-rewrite`` mode, limited to fixtures that contain a
recovered syntax expectation.  Do not use ``--syntax-only``: that specialized
mode refreshes strict syntax failures but does not regenerate their recovered
expectations.
This comparator only auto-accepts the narrow #526 mechanical improvement:

* all expectation data outside ``expectations.syntax.recovered`` is unchanged;
* existing diagnostics are preserved or narrowed, in order;
* previously valid tokens remain in order and more tokens become valid; and
* a recovered skip is split into more recovery items with strictly less
  positive-width byte coverage.

Everything else is residue for individual manual review.  In particular,
format-only rewrites are not copied into the worktree.
"""

from __future__ import annotations

import argparse
import copy
import json
import shutil
import sys
import tomllib
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Iterable, Sequence


JsonObject = dict[str, Any]
ByteSpan = tuple[int, int]


@dataclass(frozen=True)
class FixtureClassification:
    path: str
    category: str
    reasons: tuple[str, ...]
    old_diagnostic_count: int | None = None
    new_diagnostic_count: int | None = None
    old_valid_token_count: int | None = None
    new_valid_token_count: int | None = None
    old_recovery_item_count: int | None = None
    new_recovery_item_count: int | None = None
    old_recovered_bytes: int | None = None
    new_recovered_bytes: int | None = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-root", type=Path, required=True)
    parser.add_argument("--candidate-root", type=Path, required=True)
    parser.add_argument(
        "--apply",
        action="store_true",
        help="copy only mechanically classified candidate fixtures to the baseline root",
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="write the complete machine-readable classification report as JSON",
    )
    return parser.parse_args()


def load_toml(path: Path) -> JsonObject:
    with path.open("rb") as fixture_file:
        value = tomllib.load(fixture_file)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: fixture root is not a TOML table")
    return value


def fixture_paths(root: Path) -> set[Path]:
    return {path.relative_to(root) for path in root.rglob("*.toml")}


def recovered_expectation(fixture: JsonObject) -> JsonObject | None:
    value: Any = fixture
    for key in ("expectations", "syntax", "recovered"):
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value if isinstance(value, dict) else None


def without_recovered_expectation(fixture: JsonObject) -> JsonObject:
    result = copy.deepcopy(fixture)
    expectations = result.get("expectations")
    if not isinstance(expectations, dict):
        return result
    syntax = expectations.get("syntax")
    if not isinstance(syntax, dict):
        return result
    syntax.pop("recovered", None)
    return result


def is_ordered_subsequence(old: Sequence[Any], new: Sequence[Any]) -> bool:
    new_iterator = iter(new)
    return all(any(candidate == item for candidate in new_iterator) for item in old)


def diagnostic_refines(old: Any, new: Any) -> bool:
    if not isinstance(old, dict) or not isinstance(new, dict):
        return old == new
    old_span = byte_span(old.get("byte-span"))
    new_span = byte_span(new.get("byte-span"))
    if old_span is None or new_span is None:
        return old == new
    old_rest = {key: value for key, value in old.items() if key not in {"byte-span", "source-text"}}
    new_rest = {key: value for key, value in new.items() if key not in {"byte-span", "source-text"}}
    if old_rest != new_rest:
        return False
    if not (old_span[0] <= new_span[0] <= new_span[1] <= old_span[1]):
        return False
    if old_span == new_span:
        return old.get("source-text") == new.get("source-text")
    return True


def diagnostics_preserved_or_refined(old: Sequence[Any], new: Sequence[Any]) -> bool:
    new_index = 0
    for old_diagnostic in old:
        while new_index < len(new) and not diagnostic_refines(
            old_diagnostic, new[new_index]
        ):
            new_index += 1
        if new_index == len(new):
            return False
        new_index += 1
    return True


def byte_span(value: Any) -> ByteSpan | None:
    if (
        isinstance(value, list)
        and len(value) == 2
        and all(
            isinstance(endpoint, int) and not isinstance(endpoint, bool)
            for endpoint in value
        )
        and value[0] <= value[1]
    ):
        return value[0], value[1]
    return None


def recovery_items(tree: JsonObject) -> list[JsonObject] | None:
    items = tree.get("recovery-items")
    if not isinstance(items, list) or not all(isinstance(item, dict) for item in items):
        return None
    return items


def positive_recovery_spans(items: Iterable[JsonObject]) -> list[ByteSpan] | None:
    spans: list[ByteSpan] = []
    for item in items:
        item_spans = item.get("byte-spans")
        if not isinstance(item_spans, list):
            return None
        for value in item_spans:
            span = byte_span(value)
            if span is None:
                return None
            if span[0] < span[1]:
                spans.append(span)
    return spans


def merge_spans(spans: Iterable[ByteSpan]) -> list[ByteSpan]:
    merged: list[ByteSpan] = []
    for start, end in sorted(spans):
        if not merged or start > merged[-1][1]:
            merged.append((start, end))
            continue
        previous_start, previous_end = merged[-1]
        merged[-1] = previous_start, max(previous_end, end)
    return merged


def covered_bytes(spans: Iterable[ByteSpan]) -> int:
    return sum(end - start for start, end in merge_spans(spans))


def spans_are_within(inner: Iterable[ByteSpan], outer: Iterable[ByteSpan]) -> bool:
    outer_merged = merge_spans(outer)
    return all(
        any(outer_start <= start and end <= outer_end for outer_start, outer_end in outer_merged)
        for start, end in inner
    )


def classify_changed_fixture(
    relative_path: Path,
    baseline: JsonObject,
    candidate: JsonObject,
) -> FixtureClassification:
    reasons: list[str] = []
    if without_recovered_expectation(baseline) != without_recovered_expectation(candidate):
        reasons.append("expectation data outside syntax.recovered changed")

    old_recovered = recovered_expectation(baseline)
    new_recovered = recovered_expectation(candidate)
    if old_recovered is None or new_recovered is None:
        reasons.append("both fixtures must contain expectations.syntax.recovered")
        return FixtureClassification(
            path=relative_path.as_posix(), category="residue", reasons=tuple(reasons)
        )

    old_diagnostics = old_recovered.get("diagnostics")
    new_diagnostics = new_recovered.get("diagnostics")
    if not isinstance(old_diagnostics, list) or not isinstance(new_diagnostics, list):
        reasons.append("recovered diagnostics are not both arrays")
        old_diagnostics = [] if not isinstance(old_diagnostics, list) else old_diagnostics
        new_diagnostics = [] if not isinstance(new_diagnostics, list) else new_diagnostics
    elif not diagnostics_preserved_or_refined(old_diagnostics, new_diagnostics):
        reasons.append(
            "existing diagnostics were removed, reordered, or changed rather than narrowed"
        )

    old_tree = old_recovered.get("tree")
    new_tree = new_recovered.get("tree")
    old_valid: list[Any] = []
    new_valid: list[Any] = []
    old_items: list[JsonObject] = []
    new_items: list[JsonObject] = []
    old_spans: list[ByteSpan] = []
    new_spans: list[ByteSpan] = []
    if not isinstance(old_tree, dict) or not isinstance(new_tree, dict):
        reasons.append("recovered trees are not both tables")
    else:
        old_metadata = {
            key: value
            for key, value in old_recovered.items()
            if key not in {"diagnostics", "tree"}
        }
        new_metadata = {
            key: value
            for key, value in new_recovered.items()
            if key not in {"diagnostics", "tree"}
        }
        if old_metadata != new_metadata:
            reasons.append("recovered status, cap, or other metadata changed")
        old_valid_value = old_tree.get("valid-tokens")
        new_valid_value = new_tree.get("valid-tokens")
        if not isinstance(old_valid_value, list) or not isinstance(new_valid_value, list):
            reasons.append("valid-tokens are not both arrays")
        else:
            old_valid = old_valid_value
            new_valid = new_valid_value
            if not is_ordered_subsequence(old_valid, new_valid):
                reasons.append("previously valid tokens are not preserved in order")
            if len(new_valid) <= len(old_valid):
                reasons.append("candidate does not expose additional valid tokens")

        old_items_value = recovery_items(old_tree)
        new_items_value = recovery_items(new_tree)
        if old_items_value is None or new_items_value is None:
            reasons.append("recovery-items are not both arrays of tables")
        else:
            old_items = old_items_value
            new_items = new_items_value
            if len(new_items) <= len(old_items):
                reasons.append("recovered skip was not split into more recovery items")
            old_spans_value = positive_recovery_spans(old_items)
            new_spans_value = positive_recovery_spans(new_items)
            if old_spans_value is None or new_spans_value is None:
                reasons.append("recovery byte-spans are malformed")
            else:
                old_spans = old_spans_value
                new_spans = new_spans_value
                if not old_spans:
                    reasons.append("baseline has no positive-width recovered byte coverage")
                elif not spans_are_within(new_spans, old_spans):
                    reasons.append("candidate recovery extends outside baseline recovered coverage")
                if covered_bytes(new_spans) >= covered_bytes(old_spans):
                    reasons.append("candidate does not strictly reduce recovered byte coverage")

    category = "mechanical-improvement" if not reasons else "residue"
    return FixtureClassification(
        path=relative_path.as_posix(),
        category=category,
        reasons=tuple(reasons),
        old_diagnostic_count=len(old_diagnostics),
        new_diagnostic_count=len(new_diagnostics),
        old_valid_token_count=len(old_valid),
        new_valid_token_count=len(new_valid),
        old_recovery_item_count=len(old_items),
        new_recovery_item_count=len(new_items),
        old_recovered_bytes=covered_bytes(old_spans),
        new_recovered_bytes=covered_bytes(new_spans),
    )


def classify_roots(baseline_root: Path, candidate_root: Path) -> list[FixtureClassification]:
    baseline_paths = fixture_paths(baseline_root)
    candidate_paths = fixture_paths(candidate_root)
    classifications: list[FixtureClassification] = []
    for relative_path in sorted(baseline_paths | candidate_paths):
        if relative_path not in baseline_paths:
            classifications.append(
                FixtureClassification(
                    path=relative_path.as_posix(),
                    category="residue",
                    reasons=("candidate added a fixture",),
                )
            )
            continue
        if relative_path not in candidate_paths:
            classifications.append(
                FixtureClassification(
                    path=relative_path.as_posix(),
                    category="residue",
                    reasons=("candidate removed a fixture",),
                )
            )
            continue
        baseline_path = baseline_root / relative_path
        candidate_path = candidate_root / relative_path
        baseline_bytes = baseline_path.read_bytes()
        candidate_bytes = candidate_path.read_bytes()
        if baseline_bytes == candidate_bytes:
            continue
        baseline = load_toml(baseline_path)
        candidate = load_toml(candidate_path)
        if baseline == candidate:
            classifications.append(
                FixtureClassification(
                    path=relative_path.as_posix(),
                    category="format-only",
                    reasons=("parsed fixture data is unchanged",),
                )
            )
            continue
        classifications.append(classify_changed_fixture(relative_path, baseline, candidate))
    return classifications


def report_value(classifications: Sequence[FixtureClassification], applied: int) -> JsonObject:
    counts = {
        category: sum(item.category == category for item in classifications)
        for category in ("mechanical-improvement", "residue", "format-only")
    }
    counts["changed"] = len(classifications)
    counts["applied"] = applied
    return {
        "summary": counts,
        "classifications": [asdict(item) for item in classifications],
    }


def main() -> int:
    args = parse_args()
    if not args.baseline_root.is_dir():
        raise SystemExit(f"baseline root is not a directory: {args.baseline_root}")
    if not args.candidate_root.is_dir():
        raise SystemExit(f"candidate root is not a directory: {args.candidate_root}")

    classifications = classify_roots(args.baseline_root, args.candidate_root)
    applied = 0
    if args.apply:
        for item in classifications:
            if item.category != "mechanical-improvement":
                continue
            source = args.candidate_root / item.path
            destination = args.baseline_root / item.path
            shutil.copyfile(source, destination)
            applied += 1

    report = report_value(classifications, applied)
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(rendered, encoding="utf-8")
    sys.stdout.write(rendered)
    return 1 if report["summary"]["residue"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
