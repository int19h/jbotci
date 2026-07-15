#!/usr/bin/env python3
"""Verify the issue #376 allowlist and tersmu fixture migrations.

Issue #358 requires expectation changes to be reproducible from both sides.
This verifier freezes the exact allowlist shrink, reproduces every old blocker
with a main binary, reproduces every new result with the issue binary, and
proves that all 69 changed fixture documents differ only in their tersmu
expectation.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import tomllib
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any


ALLOWLIST = Path("tests/semantics-coverage-allowlist.txt")
FULL_ALICE_ID = "corpus.alis.full-alice"
FULL_ALICE_PATH = Path("tests/fixtures/corpus/alis/full-alice.toml")
EXPECTED_REMOVED_COUNT = 68
EXPECTED_REMOVED_SHA256 = (
    "153f5e5f9f2b94970b73bc0b11f074ff4075789b40d9aaa86e4f4a8648a80599"
)
UNSUPPORTED_PREFIX = "generated semantic builder does not yet support "
EXPECTED_BASE_CLASSES = Counter(
    {
        "prenex subbridi relation label": 62,
        "relative head pro-sumti outside relative clause": 6,
    }
)
CLASS_JUSTIFICATIONS = {
    "prenex subbridi relation label": (
        "CLL 16 prenex terms remain in the typed relation label while their "
        "quantifier scopes wrap the subordinate formula in written order"
    ),
    "relative head pro-sumti outside relative clause": (
        "CLL 7.10 and 8.1 ke'a becomes a typed relative-head parameter when "
        "abstract, while a concrete relative head remains bound across nested abstractions"
    ),
}
FULL_ALICE_BASE_ERROR = (
    "generated semantic builder does not yet support relative statement without formula"
)
FULL_ALICE_CURRENT_ERROR = (
    "generated semantic builder does not yet support mixed direct generated question kinds"
)
EXPECTED_BASE_TOTALS = Counter(
    {"success": 22007, "other-error": 240, "unsupported": 90, "panic": 0}
)
EXPECTED_CURRENT_TOTALS = Counter(
    {"success": 22075, "other-error": 240, "unsupported": 22, "panic": 0}
)
BEHAVIOR_STATUSES = ("success", "other-error", "unsupported", "panic")


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=False, text=True, capture_output=True)


def git_source(base_ref: str, path: Path) -> str:
    result = run("git", "show", f"{base_ref}:{path.as_posix()}")
    if result.returncode != 0:
        raise ValueError(result.stderr.strip())
    return result.stdout


def document(source: str) -> dict[str, Any]:
    return tomllib.loads(source)


def allowlist_ids(source: str) -> set[str]:
    return {
        line
        for raw_line in source.splitlines()
        if (line := raw_line.strip()) and not line.startswith("#")
    }


def removed_digest(ids: set[str]) -> str:
    payload = "".join(f"{fixture_id}\n" for fixture_id in sorted(ids))
    return hashlib.sha256(payload.encode()).hexdigest()


def fixture_paths_by_id() -> dict[str, Path]:
    paths: dict[str, Path] = {}
    for path in Path("tests/fixtures").rglob("*.toml"):
        try:
            fixture = document(path.read_text())
        except (OSError, tomllib.TOMLDecodeError):
            continue
        fixture_id = fixture.get("id")
        if isinstance(fixture_id, str):
            if fixture_id in paths:
                raise ValueError(f"duplicate fixture id {fixture_id!r}")
            paths[fixture_id] = path
    return paths


def fixture_input(path: Path, fixture: dict[str, Any]) -> tuple[str, bool]:
    if "lojban" in fixture:
        return str(fixture["lojban"]), False
    if "lojban-filename" in fixture:
        return str(path.parent / str(fixture["lojban-filename"])), True
    raise ValueError(f"{path}: fixture has no Lojban input")


def semantic_error(stderr: str) -> str:
    for line in reversed(stderr.splitlines()):
        if line.startswith("semantic error: "):
            return line.removeprefix("semantic error: ")
    raise ValueError(f"could not find semantic error in stderr: {stderr[-500:]!r}")


def render(binary: Path, path: Path, fixture: dict[str, Any]) -> dict[str, str]:
    command = [str(binary), "tersmu", "--color=never", "--format", "json"]
    if dialect := fixture.get("dialect"):
        command.extend(("--dialect", str(dialect)))
    tersmu = fixture.get("expectations", {}).get("output", {}).get("tersmu", {})
    if isinstance(tersmu, dict) and tersmu.get("story-time") is True:
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


def classify(
    binary: Path,
    path: Path,
    fixture: dict[str, Any],
) -> tuple[str, str | None]:
    command = [str(binary), "tersmu", "--color=never", "--format", "json"]
    if dialect := fixture.get("dialect"):
        command.extend(("--dialect", str(dialect)))
    tersmu = fixture.get("expectations", {}).get("output", {}).get("tersmu", {})
    if isinstance(tersmu, dict) and tersmu.get("story-time") is True:
        command.append("--story-time")
    value, is_file = fixture_input(path, fixture)
    standard_input = None
    if is_file:
        command.extend(("--file", value))
    else:
        command.extend(("--file", "/dev/stdin"))
        standard_input = value
    result = subprocess.run(
        command,
        check=False,
        text=True,
        input=standard_input,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    if result.returncode == 0:
        return "success", None
    try:
        error = semantic_error(result.stderr)
    except ValueError:
        message = result.stderr.splitlines()[-1] if result.stderr else "no stderr"
        if result.returncode == 101 or "panicked at" in result.stderr:
            return "panic", message
        return "other-error", message
    if error.startswith(UNSUPPORTED_PREFIX):
        return "unsupported", error.removeprefix(UNSUPPORTED_PREFIX)
    return "other-error", error


def normalized_expectation(fixture: dict[str, Any]) -> dict[str, str]:
    tersmu = fixture["expectations"]["output"]["tersmu"]
    if "json" in tersmu:
        return {"status": "success", "json": tersmu["json"]}
    error = tersmu["error"]
    prefix = "tersmu JSON build error: "
    if not error.startswith(prefix):
        raise ValueError(f"unexpected tersmu fixture error: {error!r}")
    return {"status": "failure", "error": error.removeprefix(prefix)}


def without_tersmu(fixture: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(fixture)
    output = result.get("expectations", {}).get("output")
    if isinstance(output, dict):
        output.pop("tersmu", None)
        if not output:
            result["expectations"].pop("output")
    return result


def changed_fixture_paths(base_ref: str) -> set[Path]:
    result = run("git", "diff", "--name-only", base_ref, "--", "tests/fixtures")
    if result.returncode != 0:
        raise ValueError(result.stderr.strip())
    return {Path(line) for line in result.stdout.splitlines() if line}


def verify_document_migration(
    base_ref: str,
    path: Path,
    current_result: dict[str, str],
) -> None:
    base_fixture = document(git_source(base_ref, path))
    current_fixture = document(path.read_text())
    if without_tersmu(base_fixture) != without_tersmu(current_fixture):
        raise ValueError(f"{path}: changed beyond its tersmu expectation")
    if normalized_expectation(current_fixture) != current_result:
        raise ValueError(f"{path}: current tersmu expectation is stale")


def verify_global_transition_matrix(
    base_binary: Path,
    current_binary: Path,
    paths: dict[str, Path],
    expected_changed: set[str],
    jobs: int,
) -> None:
    fixtures = []
    for fixture_id, path in paths.items():
        fixture = document(path.read_text())
        syntax = fixture.get("expectations", {}).get("syntax")
        if isinstance(syntax, dict) and syntax.get("status") == "success":
            fixtures.append((fixture_id, path, fixture))

    def classify_pair(
        item: tuple[str, Path, dict[str, Any]],
    ) -> tuple[str, tuple[str, str | None], tuple[str, str | None]]:
        fixture_id, path, fixture = item
        return (
            fixture_id,
            classify(base_binary, path, fixture),
            classify(current_binary, path, fixture),
        )

    with ThreadPoolExecutor(max_workers=jobs) as executor:
        outcomes = list(executor.map(classify_pair, fixtures))

    base_totals: Counter[str] = Counter()
    current_totals: Counter[str] = Counter()
    matrix: Counter[tuple[str, str]] = Counter()
    changed = []
    for fixture_id, base, current in outcomes:
        base_totals[base[0]] += 1
        current_totals[current[0]] += 1
        matrix[(base[0], current[0])] += 1
        if base != current:
            changed.append((fixture_id, base, current))

    for status in BEHAVIOR_STATUSES:
        base_totals.setdefault(status, 0)
        current_totals.setdefault(status, 0)
    if base_totals != EXPECTED_BASE_TOTALS:
        raise ValueError(f"global base totals changed: {dict(base_totals)!r}")
    if current_totals != EXPECTED_CURRENT_TOTALS:
        raise ValueError(f"global current totals changed: {dict(current_totals)!r}")
    changed_ids = {fixture_id for fixture_id, _base, _current in changed}
    if changed_ids != expected_changed:
        raise ValueError(
            f"unexpected global behavior changes: {sorted(changed_ids ^ expected_changed)}"
        )
    success_regressions = [
        outcome for outcome in changed if outcome[1][0] == "success" and outcome[2][0] != "success"
    ]
    if success_regressions:
        raise ValueError(f"success regressions: {success_regressions!r}")

    print("global behavior totals:")
    print(f"  base: {dict(base_totals)}")
    print(f"  current: {dict(current_totals)}")
    print("global transition matrix:")
    for base_status in BEHAVIOR_STATUSES:
        for current_status in BEHAVIOR_STATUSES:
            print(
                f"  {base_status} -> {current_status}: "
                f"{matrix[(base_status, current_status)]}"
            )
    changed_classes = Counter((base, current) for _fixture_id, base, current in changed)
    print("changed behavior classes:")
    for (base, current), count in sorted(changed_classes.items()):
        print(f"  {base} -> {current}: {count}")
    print("success -> error flips: 0")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    parser.add_argument("--global-matrix", action="store_true")
    parser.add_argument("--jobs", type=int, default=16)
    args = parser.parse_args()

    base_allowlist = allowlist_ids(git_source(args.base_ref, ALLOWLIST))
    current_allowlist = allowlist_ids(ALLOWLIST.read_text())
    additions = current_allowlist - base_allowlist
    if additions:
        raise ValueError(f"the shrink-only allowlist gained entries: {sorted(additions)}")
    removed = base_allowlist - current_allowlist
    if len(removed) != EXPECTED_REMOVED_COUNT:
        raise ValueError(f"expected 68 removed ids, found {len(removed)}")
    if removed_digest(removed) != EXPECTED_REMOVED_SHA256:
        raise ValueError("the exact issue #376 allowlist removal set changed")

    paths = fixture_paths_by_id()
    missing = sorted((removed | {FULL_ALICE_ID}) - paths.keys())
    if missing:
        raise ValueError(f"migrated fixture ids have no fixture: {missing}")

    base_classes: Counter[str] = Counter()
    current_results: dict[str, dict[str, str]] = {}
    for fixture_id in sorted(removed):
        path = paths[fixture_id]
        current_fixture = document(path.read_text())
        base_fixture = document(git_source(args.base_ref, path))
        base_result = render(args.base_binary, path, base_fixture)
        base_error = base_result.get("error", "")
        if base_result.get("status") != "failure" or not base_error.startswith(
            UNSUPPORTED_PREFIX
        ):
            raise ValueError(f"{fixture_id}: base result is not an unsupported blocker")
        fixture_class = base_error.removeprefix(UNSUPPORTED_PREFIX)
        justification = CLASS_JUSTIFICATIONS.get(fixture_class)
        if justification is None:
            raise ValueError(f"{fixture_id}: no justification for {fixture_class!r}")
        base_classes[fixture_class] += 1

        current_result = render(args.current_binary, path, current_fixture)
        current_results[fixture_id] = current_result
        if current_result.get("status") != "success":
            raise ValueError(f"{fixture_id}: expected success, got {current_result!r}")
        verify_document_migration(args.base_ref, path, current_result)
        print(f"{fixture_id}: {fixture_class} -> success: {justification}")

    if base_classes != EXPECTED_BASE_CLASSES:
        raise ValueError(f"base class counts changed: {dict(sorted(base_classes.items()))!r}")

    full_alice_base = document(git_source(args.base_ref, FULL_ALICE_PATH))
    full_alice_current = document(FULL_ALICE_PATH.read_text())
    base_result = render(args.base_binary, FULL_ALICE_PATH, full_alice_base)
    current_result = render(args.current_binary, FULL_ALICE_PATH, full_alice_current)
    if base_result != {"status": "failure", "error": FULL_ALICE_BASE_ERROR}:
        raise ValueError(f"full Alice base blocker changed: {base_result!r}")
    if current_result != {"status": "failure", "error": FULL_ALICE_CURRENT_ERROR}:
        raise ValueError(f"full Alice next blocker changed: {current_result!r}")
    verify_document_migration(args.base_ref, FULL_ALICE_PATH, current_result)
    print(
        "corpus.alis.full-alice: relative statement without formula -> mixed direct "
        "generated question kinds: the relative's two branches and modal claim now "
        "lower, and the intervening forethought statement label also lowers; analysis "
        "reaches the pre-existing mixed-question child"
    )

    expected_changed = {paths[fixture_id] for fixture_id in removed} | {FULL_ALICE_PATH}
    changed = changed_fixture_paths(args.base_ref)
    if changed != expected_changed:
        raise ValueError(f"unexpected fixture drift: {sorted(changed ^ expected_changed)}")

    print("issue-376 fixture migration: verified")
    print("transition unsupported -> success: 68")
    print("transition unsupported -> unsupported (different class): 1")
    print("transition success -> error: 0")
    for fixture_class, count in sorted(base_classes.items()):
        print(f"base class {fixture_class}: {count}")
    print("base-to-PR document reconstruction: 69/69")
    print("PR-to-base non-tersmu identity: 69/69")
    print("unexpected fixture drift: 0")
    if args.global_matrix:
        verify_global_transition_matrix(
            args.base_binary,
            args.current_binary,
            paths,
            removed | {FULL_ALICE_ID},
            args.jobs,
        )


if __name__ == "__main__":
    main()
