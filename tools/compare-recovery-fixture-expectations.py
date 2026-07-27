#!/usr/bin/env python3
"""Classify recovery fixture rewrites before applying generated expectations.

The candidate tree is produced outside the worktree with the normal,
facet-aware ``fixture-rewrite`` mode, limited to fixtures that contain a
recovered syntax expectation.  Do not use ``--syntax-only``: that specialized
mode refreshes strict syntax failures but does not regenerate their recovered
expectations.
This comparator only auto-accepts the narrow #517 boundary-unwind improvement:

* all expectation data outside ``expectations.syntax.recovered`` is unchanged;
* existing diagnostics are preserved or narrowed, in order;
* every old valid token remains in order, every remaining invalid token keeps
  its exact source span, and invalid-token losses exactly equal valid-token
  gains;
* a recovered skip is split into more recovery items with strictly less
  positive-width byte coverage; and
* at least one zero-width ``missing`` item is synthesized and attached to the
  same diagnostic as a positive-width invalid item.

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
    old_invalid_token_count: int | None = None
    new_invalid_token_count: int | None = None
    old_missing_item_count: int | None = None
    new_missing_item_count: int | None = None
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


def recovery_spans(
    items: Iterable[JsonObject], kind: str
) -> list[ByteSpan] | None:
    spans: list[ByteSpan] = []
    for item in items:
        if item.get("kind") != kind:
            continue
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


def recovery_items_are_well_formed(
    items: Sequence[JsonObject], diagnostic_count: int
) -> bool:
    for item in items:
        kind = item.get("kind")
        error_index = item.get("error-index")
        spans = item.get("byte-spans")
        if kind not in {"invalid", "missing"}:
            return False
        if (
            not isinstance(error_index, int)
            or isinstance(error_index, bool)
            or not 0 <= error_index < diagnostic_count
        ):
            return False
        if not isinstance(spans, list) or not spans:
            return False
        parsed_spans = [byte_span(span) for span in spans]
        if any(span is None for span in parsed_spans):
            return False
        if kind == "invalid" and any(
            span is not None and span[0] == span[1] for span in parsed_spans
        ):
            return False
        if kind == "missing" and any(
            span is not None and span[0] != span[1] for span in parsed_spans
        ):
            return False
    return True


def spans_are_strictly_ordered(spans: Sequence[ByteSpan]) -> bool:
    return all(left[1] <= right[0] for left, right in zip(spans, spans[1:]))


def newly_synthesized_missing_items(
    old_items: Sequence[JsonObject], new_items: Sequence[JsonObject]
) -> list[JsonObject]:
    old_missing = [item for item in old_items if item.get("kind") == "missing"]
    remaining = old_missing.copy()
    synthesized: list[JsonObject] = []
    for item in new_items:
        if item.get("kind") != "missing":
            continue
        try:
            matching_index = remaining.index(item)
        except ValueError:
            synthesized.append(item)
        else:
            remaining.pop(matching_index)
    return synthesized


def missing_items_share_invalid_diagnostics(
    missing_items: Sequence[JsonObject], all_items: Sequence[JsonObject]
) -> bool:
    invalid_error_indexes = {
        item.get("error-index")
        for item in all_items
        if item.get("kind") == "invalid"
    }
    return all(item.get("error-index") in invalid_error_indexes for item in missing_items)


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
    old_invalid_spans: list[ByteSpan] = []
    new_invalid_spans: list[ByteSpan] = []
    old_missing_items: list[JsonObject] = []
    new_missing_items: list[JsonObject] = []
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
            old_items_well_formed = recovery_items_are_well_formed(
                old_items, len(old_diagnostics)
            )
            new_items_well_formed = recovery_items_are_well_formed(
                new_items, len(new_diagnostics)
            )
            if not old_items_well_formed or not new_items_well_formed:
                reasons.append(
                    "recovery items have invalid kinds, error indexes, or byte spans"
                )
            if len(new_items) <= len(old_items):
                reasons.append("recovered skip was not split into more recovery items")
            old_invalid_value = recovery_spans(old_items, "invalid")
            new_invalid_value = recovery_spans(new_items, "invalid")
            if old_invalid_value is None or new_invalid_value is None:
                reasons.append("invalid recovery byte-spans are malformed")
            else:
                old_invalid_spans = old_invalid_value
                new_invalid_spans = new_invalid_value
                if not old_invalid_spans:
                    reasons.append("baseline has no invalid recovered tokens")
                if not spans_are_strictly_ordered(old_invalid_spans):
                    reasons.append("baseline invalid token spans overlap or are out of order")
                if not spans_are_strictly_ordered(new_invalid_spans):
                    reasons.append("candidate invalid token spans overlap or are out of order")
                if not is_ordered_subsequence(new_invalid_spans, old_invalid_spans):
                    reasons.append(
                        "candidate invalid tokens are not an exact ordered subset of baseline"
                    )
                if (
                    len(old_valid) + len(old_invalid_spans)
                    != len(new_valid) + len(new_invalid_spans)
                ):
                    reasons.append(
                        "valid and invalid token projections do not conserve token count"
                    )
                if len(new_invalid_spans) >= len(old_invalid_spans):
                    reasons.append("candidate does not recover fewer invalid tokens")
                if covered_bytes(new_invalid_spans) >= covered_bytes(old_invalid_spans):
                    reasons.append("candidate does not strictly reduce recovered byte coverage")

            old_missing_items = [
                item for item in old_items if item.get("kind") == "missing"
            ]
            new_missing_items = [
                item for item in new_items if item.get("kind") == "missing"
            ]
            synthesized_missing = newly_synthesized_missing_items(old_items, new_items)
            if len(new_missing_items) <= len(old_missing_items) or not synthesized_missing:
                reasons.append("candidate does not synthesize additional missing items")
            elif not missing_items_share_invalid_diagnostics(
                synthesized_missing, new_items
            ):
                reasons.append(
                    "synthesized missing items do not share a diagnostic with an invalid item"
                )

    category = "recovery-unwind" if not reasons else "residue"
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
        old_invalid_token_count=len(old_invalid_spans),
        new_invalid_token_count=len(new_invalid_spans),
        old_missing_item_count=len(old_missing_items),
        new_missing_item_count=len(new_missing_items),
        old_recovered_bytes=covered_bytes(old_invalid_spans),
        new_recovered_bytes=covered_bytes(new_invalid_spans),
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
        for category in ("recovery-unwind", "residue", "format-only")
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
            if item.category != "recovery-unwind":
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
