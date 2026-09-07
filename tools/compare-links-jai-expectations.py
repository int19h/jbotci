#!/usr/bin/env python3
"""Epoch-10 expectation comparison, separate from the parser/fixture oracle.

Pair the semantic-base and candidate fixtures in both directions. Source identity and
metadata stay exact; expectation changes need a named mechanism and a disposition of
every changed surface. No token-only projection can make a tree delta disappear.
The Empty ledger is derived from the same collector output by links_jai_ledger.

Run from the repository root, for example:

    python3 tools/compare-links-jai-expectations.py --mode c-a \
        --c-a-observations /build/jbotci/scratch/CASE/c-a.jsonl \
        --empty-dispositions /build/jbotci/scratch/CASE/empty-dispositions.json

The default baseline is a fresh Git archive. --baseline-root can reuse that archive;
it denotes its repository root, not the tests/fixtures subdirectory. --manual is an
ID map of class/surfaces/reason objects for changed existing expectations. All pinned
facets are compared whole; unpinned historical diagnostics are not interpreted as
empty arrays. The parser-stage captures and fixture oracle remain responsible for
actual outputs and warning evidence, including when an old fixture did not pin them.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any, Mapping

if __package__:
    from . import links_jai_ledger as ledger
    from . import links_jai_transcription as jai
    from .rust_debug import DebugParseError, DebugParser, exact_equal
else:
    import links_jai_ledger as ledger
    import links_jai_transcription as jai
    from rust_debug import DebugParseError, DebugParser, exact_equal


SEMANTIC_BASE = "83490da32fc174528cdc3283b9aaedca4d90bdb0"
MODES = ("c-a", "c-b", "c-c", "c-d", "c-e", "c-f", "final")
EPOCH_TAG = "links-jai-epoch"
# Each category becomes available only with its mechanism. The label never substitutes
# for an exact list of changed surfaces, or for the independent actual-parser stage gates.
MANUAL_STAGE = {
    "d1-width": "c-a",
    "d1-recovery-withdrawal": "c-a",
    "d2-empty-removal": "c-b",
    "d3-preposed": "c-c",
    "d3-jai-exception": "c-c",
    "d4-mehoi": "c-d",
    "d5-feature-reowner": "c-e",
    "d6-kehe-links": "c-f",
    "emergent-composite": "c-e",
    "regression-baseline": "c-a",
}


@dataclass(frozen=True)
class FixtureDelta:
    id: str
    path: str
    changed: tuple[str, ...]
    epoch_new: bool


def surfaces(value: Mapping[str, Any], prefix: str = "") -> dict[str, Any]:
    """Raw/hash, diagnostics and recovered projections are indivisible surfaces."""
    result = {}
    for key, child in value.items():
        name = f"{prefix}.{key}" if prefix else key
        if isinstance(child, dict) and key not in {"raw", "tree", "json", "diagnostics"}:
            if child:
                result.update(surfaces(child, name))
            else:
                result[name] = child
        else:
            result[name] = child
    return result


def expectation_delta(before: Mapping[str, Any], after: Mapping[str, Any]) -> tuple[str, ...]:
    old, new = surfaces(before), surfaces(after)
    return tuple(sorted(key for key in old.keys() | new.keys()
                        if key not in old or key not in new or not exact_equal(old[key], new[key])))


def compare_pair(old: Mapping[str, Any], new: Mapping[str, Any], path: str) -> FixtureDelta:
    old_metadata = {key: value for key, value in old.items() if key != "expectations"}
    new_metadata = {key: value for key, value in new.items() if key != "expectations"}
    ledger.require(exact_equal(old_metadata, new_metadata), f"{path}: source, identity or metadata changed")
    case_id = ledger.nonempty_text(old.get("id"), f"{path}: fixture ID")
    before, after = old.get("expectations", {}), new.get("expectations", {})
    ledger.require(isinstance(before, Mapping) and isinstance(after, Mapping), f"{path}: malformed expectations")
    return FixtureDelta(case_id, path, expectation_delta(before, after), False)


def require_new_pins(fixture: Mapping[str, Any], path: str) -> FixtureDelta:
    case_id = ledger.nonempty_text(fixture.get("id"), f"{path}: fixture ID")
    tags = fixture.get("tags")
    ledger.require(isinstance(tags, list) and all(isinstance(tag, str) for tag in tags) and EPOCH_TAG in tags,
                   f"{path}: untagged epoch-new fixture")
    syntax = fixture.get("expectations", {}).get("syntax")
    ledger.require(isinstance(syntax, Mapping), f"{path}: missing syntax expectations")
    for label, expected in [("syntax", syntax), ("recovered", syntax.get("recovered"))]:
        if expected is None:
            continue
        ledger.require(isinstance(expected, Mapping), f"{path}: malformed {label} expectations")
        status = expected.get("status")
        ledger.require(isinstance(status, str) and status in {"success", "failure"}, f"{path}: incomplete {label} status")
        diagnostics = expected.get("diagnostics")
        ledger.validate_diagnostics(diagnostics, fixture["lojban"], f"{path}: {label}")
        errors = sum(item["severity"] == "error" for item in diagnostics)
        ledger.require((errors > 0) == (status == "failure"), f"{path}: status disagrees with {label} diagnostics")
        ledger.require(label != "syntax" or status != "failure" or "raw" not in expected,
                       f"{path}: strict failure cannot have a raw tree")
        if status == "success" or label == "recovered":
            raw = expected.get("raw")
            ledger.require(isinstance(raw, str) and bool(raw) or
                           isinstance(raw, dict) and set(raw) == {"sha256"} and
                           isinstance(raw["sha256"], str) and len(raw["sha256"]) == 64 and
                           all(character in "0123456789abcdef" for character in raw["sha256"]),
                           f"{path}: missing complete {label} raw tree")
        if label == "recovered":
            ledger.exact_keys(expected.get("tree"), {"valid-tokens", "recovery-items"}, f"{path}: recovery projection")
            ledger.require(all(isinstance(value, list) for value in expected["tree"].values()),
                           f"{path}: incomplete recovery projection")
    return FixtureDelta(case_id, path, (), True)


def review_delta(delta: FixtureDelta, disposition: Mapping[str, Any], mode: str) -> None:
    ledger.exact_keys(disposition, {"class", "surfaces", "reason"}, f"{delta.id}: disposition")
    category = disposition["class"]
    ledger.require(isinstance(category, str) and category in MANUAL_STAGE, f"{delta.id}: undeclared manual class")
    ledger.require(MODES.index(MANUAL_STAGE[category]) <= MODES.index(mode), f"{delta.id}: premature mechanism")
    reviewed = disposition["surfaces"]
    ledger.require(isinstance(reviewed, list) and all(isinstance(item, str) for item in reviewed) and
                   len(reviewed) == len(set(reviewed)) and set(reviewed) == set(delta.changed),
                   f"{delta.id}: reviewed surfaces differ from actual delta")
    ledger.nonempty_text(disposition["reason"], f"{delta.id}: manual reason")
    if category == "d1-recovery-withdrawal":
        ledger.require(all(surface.startswith("syntax.recovered.") for surface in delta.changed),
                       f"{delta.id}: recovery disposition hides a non-recovered delta")


def mechanical_jai(old: Mapping[str, Any], new: Mapping[str, Any], delta: FixtureDelta, mode: str) -> int:
    """No other changed facet, acceptance flip, warning or regression is implicit."""
    if (MODES.index(mode) < MODES.index("c-c") or delta.changed != ("syntax.raw",) or
            "regression-baseline" in old.get("tags", [])):
        return 0
    before, after = old["expectations"]["syntax"], new["expectations"]["syntax"]
    if (before.get("status") != "success" or after.get("status") != "success" or
            not isinstance(before.get("raw"), str) or not isinstance(after.get("raw"), str)):
        return 0
    try:
        expected, count = jai.rewrite(DebugParser(before["raw"], preserve_number_literals=True).parse())
        actual = DebugParser(after["raw"], preserve_number_literals=True).parse()
        return count if count and exact_equal(expected, actual) else 0
    except (DebugParseError, jai.ManualShape):
        return 0


def compare_roots(baseline: Path, candidate: Path, mode: str,
                  dispositions: Mapping[str, Mapping[str, Any]]) -> dict[str, Any]:
    ledger.require(mode in MODES, f"unknown mode {mode}")
    ledger.require(isinstance(dispositions, Mapping), "manual dispositions must be an ID map")
    old_paths = fixture_paths(baseline)
    new_paths = fixture_paths(candidate)
    ledger.require(bool(old_paths), "empty semantic-base population")
    ledger.require(not old_paths - new_paths, f"unpaired baseline paths: {sorted(old_paths - new_paths)}")
    ids = set()
    changed = []
    witnesses = []
    tagged = 0
    unchanged = 0
    identical_strict = 0
    mechanical = []
    for path in sorted(new_paths):
        new = ledger.read_fixture(candidate, path)
        case_id = ledger.nonempty_text(new.get("id"), f"{path}: fixture ID")
        ledger.require(case_id not in ids, f"duplicate candidate fixture ID {case_id}")
        ids.add(case_id)
        tagged += EPOCH_TAG in new.get("tags", [])
        if path not in old_paths:
            witnesses.append(require_new_pins(new, path).id)
            continue
        old = ledger.read_fixture(baseline, path)
        delta = compare_pair(old, new, path)
        before_syntax = old.get("expectations", {}).get("syntax", {})
        if before_syntax.get("status") == "success" and "raw" in before_syntax and not any(
                surface.startswith("syntax.") and not surface.startswith("syntax.recovered.")
                for surface in delta.changed):
            identical_strict += 1
        if delta.changed:
            occurrences = mechanical_jai(old, new, delta, mode)
            if occurrences:
                mechanical.append({"id": case_id, "path": path, "class": "jai-shared-inner",
                                   "occurrences": occurrences, "surfaces": ["syntax.raw"]})
                continue
            ledger.require(case_id in dispositions, f"{case_id}: undispositioned surfaces {delta.changed}")
            review_delta(delta, dispositions[case_id], mode)
            if "regression-baseline" in old.get("tags", []):
                ledger.require(dispositions[case_id]["class"] == "regression-baseline",
                               f"{case_id}: regression baseline requires its separate manual review")
            changed.append({"id": case_id, "path": path, "surfaces": list(delta.changed),
                            "disposition": dispositions[case_id]})
        else:
            unchanged += 1
    ledger.require(tagged > 0, "epoch tagged population is zero")
    ledger.require(set(dispositions) == {row["id"] for row in changed}, "unused or unpaired manual dispositions")
    return {"mode": mode, "semantic_base": SEMANTIC_BASE, "baseline_fixtures": len(old_paths),
            "candidate_fixtures": len(new_paths), "unchanged_existing_fixtures": unchanged,
            "tagged_fixtures": tagged, "epoch_new": witnesses, "manual_deltas": changed,
            "mechanical_deltas": mechanical,
            "mechanical_classes": {
                "legacy-strict-identity": {"identical_strict_expectations": identical_strict,
                    "note": "Expectation identity only; actual legacy winners require the independent parser gate"},
                "jai-shared-inner": {"matched_occurrences": sum(row["occurrences"] for row in mechanical),
                    "state": "unprobed" if MODES.index(mode) < MODES.index("c-c") else "compared"},
                "transparent-eligibility-aliases": {"permitted_serialized_deltas": 0,
                    "note": "Alias reachability is checked separately by typed parser tests"},
                "preposed-retype": {"eligible_occurrences": 0,
                    "note": "All six frozen old preposed rows require manual disposition"},
            },
            "later_mechanisms": list(MODES[MODES.index(mode) + 1:])}


def fixture_paths(root: Path) -> set[str]:
    # Match collect_fixture_paths in xtask-common: profile definitions are configuration,
    # never fixture cases. Do not guess case membership from a source/ID naming pattern.
    return {path.relative_to(root).as_posix() for path in (root / "tests/fixtures").rglob("*.toml")
            if "profiles" not in path.relative_to(root).parts and path.is_file()}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=MODES, required=True)
    parser.add_argument("--candidate-root", type=Path, default=Path("."))
    parser.add_argument("--baseline-root", type=Path,
                        help="An existing git archive at the semantic base; otherwise create a transient one")
    parser.add_argument("--manual", type=Path)
    parser.add_argument("--c-a-observations", type=Path, required=True)
    parser.add_argument("--c-b-observations", type=Path)
    parser.add_argument("--empty-dispositions", type=Path, required=True)
    args = parser.parse_args()
    try:
        manual = {} if args.manual is None else ledger.read_json(args.manual)
        # git archive, not another parser executable or expectation-file digest, provides
        # the ordinary baseline. Never extract into the candidate worktree.
        with tempfile.TemporaryDirectory(prefix="links-jai-base-", dir="/build/jbotci/scratch") as directory:
            baseline = args.baseline_root
            if baseline is None:
                baseline = Path(directory)
                archive = subprocess.Popen(["git", "archive", SEMANTIC_BASE, "tests/fixtures"], stdout=subprocess.PIPE)
                try:
                    subprocess.run(["tar", "-x", "-C", str(baseline)], stdin=archive.stdout, check=True)
                finally:
                    archive.stdout.close()
                ledger.require(archive.wait() == 0, "git archive failed")
            report = compare_roots(baseline, args.candidate_root, args.mode, manual)
            identities, base = ledger.load_base(ledger.read_json(Path(__file__).with_name("data") / "links-jai-empty-base.json"), baseline)
            observations = {"c-a": ledger.read_observations(args.c_a_observations)}
            if args.c_b_observations is not None:
                observations["c-b"] = ledger.read_observations(args.c_b_observations)
            stage = "c-a" if args.mode == "c-a" else "c-b"
            rows = ledger.stage_rows(identities, base, observations, ledger.read_json(args.empty_dispositions), stage)
            report["empty_targets"] = len(rows)
            report["empty_ledger_stage"] = stage
            print(json.dumps(report, ensure_ascii=False, indent=2))
    except (ledger.ValidationError, OSError, subprocess.CalledProcessError) as error:
        print(f"links-jai comparer: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
