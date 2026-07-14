#!/usr/bin/env python3
"""Rewrite and verify the issue #374 semantics fixture migration.

Issue #358 requires a two-sided gate for expectation changes. This script
freezes the exact 400-fixture allowlist removal, reproduces every old blocker
with a main binary, reproduces every current result with the issue binary, and
proves that the sole fixture document migration changes only its tersmu
expectation.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import subprocess
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any


ALLOWLIST = "tests/semantics-coverage-allowlist.txt"
MIGRATED_FIXTURE = "tests/fixtures/cll/chrestomathy/in-xanadu.toml"
EXPECTED_REMOVED_COUNT = 400
EXPECTED_REMOVED_SHA256 = (
    "7fc41e56c68b617711a3bfb8b2b32bf3d49076c6721577f72f94398aa2d7d51d"
)
UNSUPPORTED_PREFIX = "generated semantic builder does not yet support "
CURRENT_BLOCKERS = {
    "corpus.camxes.22418": (
        "semantic graph invariant failed: multiple generated tanru arguments assign "
        "visible place x1",
        "CO lowering succeeds; the fixture reaches a pre-existing duplicate-x1 graph "
        "invariant in the surrounding gasnu/cliva argument structure",
    ),
    "corpus.camxes.2393": (
        "semantic analysis of the truth-bearing scope of quantified sumti fragment `ro da "
        "poi cmavo:zo-<<cmavo:my>> pa moi lei cmene be ke'a lerfu` requires discourse "
        "context",
        "connected-selbri lowering succeeds; the fixture reaches the pre-existing "
        "truth-bearing quantified-fragment context requirement",
    ),
}
CLASS_JUSTIFICATIONS = {
    "BO grouped bridi tail": (
        "BO-grouped tail branches now lower structurally and retain their argument flow"
    ),
    "CO selbri": (
        "CO inversion now builds the trailing tertau as the head and preserves the leading "
        "unit as its explicit tanru modifier"
    ),
    "MOI sumti selbri": (
        "ME...MOI now produces a typed ordinal relation with its three-place structure"
    ),
    "bridi tail without possible terms": (
        "term-free tail wrappers now delegate to the same structural selbri lowering"
    ),
    "connected event tense on non-relation selbri": (
        "connected event tense now applies to each structural non-atomic selbri branch"
    ),
    "connected bridi tail": (
        "connected and BO-grouped bridi tails now lower both branches with their shared "
        "argument stream"
    ),
    "forethought simple bridi tail": (
        "forethought simple tails now lower both branches even when a wrapper has no "
        "possible trailing terms"
    ),
    "forethought bridi branch with post-CU terms": (
        "forethought branches now share and distribute post-CU term assignments"
    ),
    "forethought statement as bridi": (
        "forethought statement branches now lower through the typed bridi formula path"
    ),
    "linkargs sumti selbri": (
        "ME linkargs now start at x3, after x2 remains the ME source referent"
    ),
    "modal terms on grouped/connected tanru heads": (
        "modal terms now attach to every grouped or connected head predication"
    ),
    "modal shared forethought bridi term": (
        "a modal term shared by a forethought bridi now attaches to every branch"
    ),
    "modal terms on connected tanru unit head": (
        "modal terms now attach to every predication in a connected tanru-unit head"
    ),
    "modal terms on grouped tanru head": (
        "modal terms now propagate through a grouped tanru head to its predications"
    ),
    "non-atomic tanru unit property arguments": (
        "property arguments now propagate through grouped and connected tanru structure"
    ),
    "non-word tanru unit": (
        "typed relation labels and recursive argument lowering now cover non-word units"
    ),
    "preallocated connected tanru unit head eventuality": (
        "the preallocated head event now belongs to the leading connected branch while all "
        "branches share x1"
    ),
    "preallocated connected BO-bound tanru unit eventuality": (
        "the preallocated head event now flows through BO grouping to the leading connected "
        "branch"
    ),
    "scoped CO selbri": (
        "scoped CO uses the same inversion and tanru-link graph as matrix CO"
    ),
    "scoped connected tanru unit": (
        "connected property branches now share their scoped arguments without collapsing "
        "either relation"
    ),
    "scoped forethought bridi connection": (
        "scoped forethought branches now retain branch-local formulas while sharing the "
        "scoped argument"
    ),
    "scoped grouped tanru unit head": (
        "grouped heads now receive scoped visible arguments through structural lowering"
    ),
    "scoped scalar grouped tanru unit head": (
        "scalar-negated grouped heads now preserve grouping, arguments, and scalar scope"
    ),
    "scoped scalar tanru unit head": (
        "scalar-negated heads now preserve scoped arguments and scalar scope"
    ),
    "scoped sumti selbri": (
        "scoped ME now retains its x1 participant and x2 source referent"
    ),
    "tagged or connected selbri": (
        "tagged and connected selbri now lower as structural formulas with branch-local "
        "events and shared arguments"
    ),
    "tagged scalar-negated tanru unit": (
        "tagged scalar negation now stays attached to the structural tanru head"
    ),
    "tanru": (
        "multi-unit tanru now retain the tertau predication and an explicit TanruLink to a "
        "property abstraction for every modifier"
    ),
    "tanru without visible x1": (
        "tanru lowering now creates a typed elided x1 when no visible participant is present"
    ),
}
EXPECTED_BASE_CLASS_COUNTS = Counter(
    {
        "CO selbri": 17,
        "MOI sumti selbri": 17,
        "connected bridi tail": 3,
        "connected event tense on non-relation selbri": 2,
        "forethought simple bridi tail": 3,
        "linkargs sumti selbri": 2,
        "modal shared forethought bridi term": 1,
        "modal terms on connected tanru unit head": 1,
        "modal terms on grouped tanru head": 5,
        "non-atomic tanru unit property arguments": 6,
        "non-word tanru unit": 8,
        "preallocated connected BO-bound tanru unit eventuality": 1,
        "preallocated connected tanru unit head eventuality": 4,
        "scoped CO selbri": 21,
        "scoped connected tanru unit": 30,
        "scoped forethought bridi connection": 8,
        "scoped sumti selbri": 18,
        "tagged or connected selbri": 124,
        "tanru": 129,
    }
)


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=False, text=True, capture_output=True)


def git_source(base_ref: str, path: str) -> str:
    result = run("git", "show", f"{base_ref}:{path}")
    if result.returncode != 0:
        raise ValueError(result.stderr.strip())
    return result.stdout


def document(source: str) -> dict[str, Any]:
    return tomllib.loads(source)


def tersmu(fixture: dict[str, Any]) -> dict[str, Any]:
    value = fixture["expectations"]["output"]["tersmu"]
    if not isinstance(value, dict):
        raise ValueError("tersmu expectation is not a table")
    return value


def tersmu_options(fixture: dict[str, Any]) -> dict[str, Any]:
    value = fixture.get("expectations", {}).get("output", {}).get("tersmu", {})
    if not isinstance(value, dict):
        raise ValueError("tersmu expectation is not a table")
    return value


def semantic_error(stderr: str) -> str:
    for line in reversed(stderr.splitlines()):
        for prefix in ("semantic error: ", "syntax error: ", "morphology error: "):
            if line.startswith(prefix):
                return line.removeprefix(prefix)
    raise ValueError(f"could not find the tersmu error in stderr: {stderr!r}")


def fixture_input(path: Path, fixture: dict[str, Any]) -> tuple[str, bool]:
    if "lojban" in fixture:
        return fixture["lojban"], False
    if "lojban-filename" in fixture:
        return str(path.parent / fixture["lojban-filename"]), True
    raise ValueError(f"{path}: fixture has neither lojban nor lojban-filename")


def render(binary: Path, path: Path, fixture: dict[str, Any]) -> dict[str, Any]:
    command = [str(binary), "tersmu", "--color=never", "--format", "json"]
    dialect = fixture.get("dialect")
    if dialect is not None:
        command.extend(("--dialect", dialect))
    if tersmu_options(fixture).get("story-time", False):
        command.append("--story-time")
    value, is_file = fixture_input(path, fixture)
    if is_file:
        command.extend(("--file", value))
    else:
        command.append(value)
    result = run(*command)
    if result.returncode == 0:
        json.loads(result.stdout)
        return {"status": "success", "json": result.stdout.removesuffix("\n")}
    return {"status": "failure", "error": semantic_error(result.stderr)}


def normalized_expectation(fixture: dict[str, Any]) -> dict[str, Any]:
    expectation = copy.deepcopy(tersmu(fixture))
    expectation.setdefault("status", "success")
    expectation.pop("story-time", None)
    if expectation.get("status") == "failure" and isinstance(
        expectation.get("error"), str
    ):
        expectation["error"] = expectation["error"].removeprefix(
            "tersmu JSON build error: "
        )
    return expectation


def allowlist_ids(source: str) -> set[str]:
    return {
        line
        for line in source.splitlines()
        if line and not line.startswith("#")
    }


def removed_digest(ids: set[str]) -> str:
    payload = "".join(f"{fixture_id}\n" for fixture_id in sorted(ids))
    return hashlib.sha256(payload.encode()).hexdigest()


def fixture_paths_by_id() -> dict[str, Path]:
    paths: dict[str, Path] = {}
    for path in Path("tests/fixtures").rglob("*.toml"):
        fixture = document(path.read_text())
        fixture_id = fixture.get("id")
        if not isinstance(fixture_id, str):
            continue
        if fixture_id in paths:
            raise ValueError(f"duplicate fixture id: {fixture_id}")
        paths[fixture_id] = path
    return paths


def migrated_fixture_source(base_source: str, current_result: dict[str, Any]) -> str:
    if current_result.get("status") != "success":
        raise ValueError("in-Xanadu must have a successful current tersmu result")
    rendered = current_result["json"]
    if "'''" in rendered:
        raise ValueError("in-Xanadu JSON cannot be represented by the fixture string form")
    replacement = f"[expectations.output.tersmu]\njson = '''{rendered}'''\n"
    updated, count = re.subn(
        r'(?ms)^\[expectations\.output\.tersmu\]\n.*\Z',
        lambda _: replacement,
        base_source,
        count=1,
    )
    if count != 1:
        raise ValueError("in-Xanadu does not have exactly one final tersmu section")
    return updated


def changed_fixture_paths(base_ref: str) -> set[str]:
    result = run("git", "diff", "--name-only", base_ref, "--", "tests/fixtures")
    if result.returncode != 0:
        raise ValueError(result.stderr.strip())
    return {line for line in result.stdout.splitlines() if line}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    base_allowlist_source = git_source(args.base_ref, ALLOWLIST)
    base_allowlist = allowlist_ids(base_allowlist_source)
    paths = fixture_paths_by_id()
    missing = sorted(base_allowlist - paths.keys())
    if missing:
        raise ValueError(f"allowlisted fixture ids have no fixture: {missing}")

    current_cache: dict[str, dict[str, Any]] = {}
    if args.write:
        current_allowlist = allowlist_ids(Path(ALLOWLIST).read_text())
        existing_removed = base_allowlist - current_allowlist
        if (
            current_allowlist <= base_allowlist
            and len(existing_removed) == EXPECTED_REMOVED_COUNT
            and removed_digest(existing_removed) == EXPECTED_REMOVED_SHA256
        ):
            removed = existing_removed
        else:
            for fixture_id in sorted(base_allowlist):
                path = paths[fixture_id]
                current_cache[fixture_id] = render(
                    args.current_binary, path, document(path.read_text())
                )
            removed = {
                fixture_id
                for fixture_id, result in current_cache.items()
                if not (
                    result.get("status") == "failure"
                    and str(result.get("error", "")).startswith(UNSUPPORTED_PREFIX)
                )
            }
    else:
        current_allowlist = allowlist_ids(Path(ALLOWLIST).read_text())
        additions = current_allowlist - base_allowlist
        if additions:
            raise ValueError(f"the shrink-only allowlist gained entries: {sorted(additions)}")
        removed = base_allowlist - current_allowlist

    if len(removed) != EXPECTED_REMOVED_COUNT:
        raise ValueError(f"expected 400 removed ids, found {len(removed)}")
    if removed_digest(removed) != EXPECTED_REMOVED_SHA256:
        raise ValueError("the exact issue #374 fixture-id set changed")

    if args.write:
        remaining = base_allowlist - removed
        lines = [
            line
            for line in base_allowlist_source.splitlines()
            if line.startswith("#") or line in remaining
        ]
        Path(ALLOWLIST).write_text("\n".join(lines) + "\n")

    base_classes: Counter[str] = Counter()
    for fixture_id in sorted(removed):
        path = paths[fixture_id]
        fixture = document(path.read_text())
        base_result = render(args.base_binary, path, fixture)
        base_error = str(base_result.get("error", ""))
        if base_result.get("status") != "failure" or not base_error.startswith(
            UNSUPPORTED_PREFIX
        ):
            raise ValueError(f"{fixture_id}: base result is not the expected blocker")
        fixture_class = base_error.removeprefix(UNSUPPORTED_PREFIX)
        justification = CLASS_JUSTIFICATIONS.get(fixture_class)
        if justification is None:
            raise ValueError(f"{fixture_id}: missing justification for {fixture_class!r}")
        base_classes[fixture_class] += 1

        current_result = current_cache.get(fixture_id)
        if current_result is None:
            current_result = render(args.current_binary, path, fixture)
            current_cache[fixture_id] = current_result
        blocker = CURRENT_BLOCKERS.get(fixture_id)
        if blocker is None:
            if current_result.get("status") != "success":
                raise ValueError(
                    f"{fixture_id}: current result did not succeed: {current_result!r}"
                )
            transition = justification
        else:
            expected_error, transition = blocker
            if current_result != {"status": "failure", "error": expected_error}:
                raise ValueError(
                    f"{fixture_id}: unexpected next blocker: {current_result!r}"
                )
        print(f"{fixture_id}: {fixture_class}: {transition}")

    if base_classes != EXPECTED_BASE_CLASS_COUNTS:
        raise ValueError(
            f"issue #374 base class counts changed: {dict(sorted(base_classes.items()))!r}"
        )

    base_fixture_source = git_source(args.base_ref, MIGRATED_FIXTURE)
    base_fixture = document(base_fixture_source)
    migrated_id = base_fixture["id"]
    base_result = render(args.base_binary, Path(MIGRATED_FIXTURE), base_fixture)
    if normalized_expectation(base_fixture) != base_result:
        raise ValueError("in-Xanadu base expectation is stale")
    expected_fixture_source = migrated_fixture_source(
        base_fixture_source, current_cache[migrated_id]
    )
    if args.write:
        Path(MIGRATED_FIXTURE).write_text(expected_fixture_source)
    current_fixture_source = Path(MIGRATED_FIXTURE).read_text()
    if current_fixture_source != expected_fixture_source:
        raise ValueError("in-Xanadu changed beyond its tersmu expectation")
    current_fixture = document(current_fixture_source)
    if normalized_expectation(current_fixture) != current_cache[migrated_id]:
        raise ValueError("in-Xanadu current expectation is stale")
    reverse = copy.deepcopy(current_fixture)
    reverse["expectations"]["output"]["tersmu"] = copy.deepcopy(tersmu(base_fixture))
    if reverse != base_fixture:
        raise ValueError("in-Xanadu PR-to-base reconstruction changed non-tersmu data")

    changed = changed_fixture_paths(args.base_ref)
    if changed != {MIGRATED_FIXTURE}:
        raise ValueError(f"unexpected fixture drift: {sorted(changed)}")

    print("issue-374 fixture migration: verified")
    print(f"allowlist removals: {len(removed)}")
    print(f"current semantic successes: {len(removed) - len(CURRENT_BLOCKERS)}")
    print(f"individually justified next blockers: {len(CURRENT_BLOCKERS)}")
    for fixture_class, count in sorted(base_classes.items()):
        print(f"base class {fixture_class}: {count}")
    print("base-to-PR document reconstruction: 1/1")
    print("PR-to-base document reconstruction: 1/1")
    print("unexpected fixture drift: 0")


if __name__ == "__main__":
    main()
