#!/usr/bin/env python3
"""Verify the issue #377 semantics and fixture migrations.

Issue #358 requires expectation changes to be reproducible from both sides.
This verifier freezes the exact allowlist shrink and formerly principled error
set, reproduces all four changed fixture documents with main and issue binaries,
and computes the complete behavior-class transition matrix over every
syntax-success fixture.  Every improving fixture is printed with its semantic
justification so the audit does not collapse distinct fixtures into totals.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import time
import tomllib
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any


ALLOWLIST = Path("tests/semantics-coverage-allowlist.txt")
FULL_ALICE_ID = "corpus.alis.full-alice"
FULL_ALICE_PATH = Path("tests/fixtures/corpus/alis/full-alice.toml")
NORTH_WIND_PATH = Path("tests/fixtures/cll/chrestomathy/north-wind.toml")
QUANTIFIED_FRAGMENT_PATH = Path(
    "tests/fixtures/cll/chapter-17/section-17.11/c17e11d6.toml"
)
RESET_QUESTION_PATH = Path("tests/fixtures/corpus/camxes/6695.toml")
MIGRATED_FIXTURE_PATHS = {
    FULL_ALICE_PATH,
    NORTH_WIND_PATH,
    QUANTIFIED_FRAGMENT_PATH,
    RESET_QUESTION_PATH,
}

UNSUPPORTED_PREFIX = "generated semantic builder does not yet support "
MULTIPLE_DOMAIN_ERROR = (
    "semantic question model cannot represent multiple answer domains in one direct question"
)
QUANTIFIED_FRAGMENT_ERROR_PREFIX = (
    "semantic analysis of the truth-bearing scope of quantified sumti fragment `"
)
QUANTIFIED_FRAGMENT_ERROR_SUFFIX = "` requires discourse context"
FULL_ALICE_BASE_ERROR = UNSUPPORTED_PREFIX + "mixed direct generated question kinds"

EXPECTED_REMOVED_COUNT = 18
EXPECTED_REMOVED_SHA256 = (
    "6adde23bec86c7a955c1b4dfc28dd99fd1d00ee296f292d6ba15af3f7ebbcbb9"
)
EXPECTED_OTHER_TO_SUCCESS_COUNT = 158
EXPECTED_OTHER_TO_SUCCESS_SHA256 = (
    "1dd322d1ebecfff831b9a10f2cede4f4834824c1eb6dad28faf25259b8ef1c14"
)
EXPECTED_BASE_ERROR_CLASSES = Counter(
    {
        "quantified sumti fragment": 155,
        "multiple direct answer domains": 3,
    }
)
EXPECTED_BASE_UNSUPPORTED_CLASSES = Counter(
    {
        "mixed direct generated question kinds": 15,
        "nonlogical generated statement connective": 1,
        "unsupported nonlogical statement connective ji": 2,
    }
)
EXPECTED_BASE_TOTALS = Counter(
    {"success": 22075, "other-error": 240, "unsupported": 22, "panic": 0}
)
EXPECTED_CURRENT_TOTALS = Counter(
    {"success": 22251, "other-error": 82, "unsupported": 4, "panic": 0}
)
BEHAVIOR_STATUSES = ("success", "other-error", "unsupported", "panic")
EXPECTED_MATRIX = Counter(
    {
        ("success", "success"): 22075,
        ("other-error", "success"): 158,
        ("other-error", "other-error"): 82,
        ("unsupported", "success"): 18,
        ("unsupported", "unsupported"): 4,
    }
)
EXPECTED_CURRENT_ALLOWLIST = {
    "adhoc.v0.warnings.standard-no-warning.standard-pre-i-joi-statement-connective",
    "cll.chrestomathy.alice01",
    "corpus.camxes.17625",
    "corpus.camxes.18927",
}

JUSTIFICATIONS = {
    "quantified sumti fragment": (
        "the generated fragment now carries a typed sumti-operand formula and exact "
        "quantifier scopes, so any interrogative remains structurally reachable"
    ),
    "multiple direct answer domains": (
        "CLL 7.9 and 19.5 make repeated or unlike question words simultaneous "
        "answer blanks; one direct question now preserves every ordered typed slot"
    ),
    "mixed direct generated question kinds": (
        "CLL 7.9 and 19.5 make all surface interrogatives simultaneous answer "
        "blanks; the graph pins every slot's kind, domain, parameter, and source order"
    ),
    "nonlogical generated statement connective": (
        "the interrogative statement connective is a typed connective-answer slot "
        "over both statement formulas"
    ),
    "unsupported nonlogical statement connective ji": (
        "ji supplies a typed connective-answer slot over both statement formulas "
        "instead of routing through an untyped connective label"
    ),
}


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


def ids_digest(ids: set[str]) -> str:
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


def command_for(
    binary: Path,
    path: Path,
    fixture: dict[str, Any],
) -> tuple[list[str], str | None]:
    command = [str(binary), "tersmu", "--color=never", "--format", "json"]
    if dialect := fixture.get("dialect"):
        command.extend(("--dialect", str(dialect)))
    tersmu = fixture.get("expectations", {}).get("output", {}).get("tersmu", {})
    if isinstance(tersmu, dict) and tersmu.get("story-time") is True:
        command.append("--story-time")
    value, is_file = fixture_input(path, fixture)
    if is_file:
        command.extend(("--file", value))
        return command, None
    command.extend(("--file", "/dev/stdin"))
    return command, value


def render(binary: Path, path: Path, fixture: dict[str, Any]) -> dict[str, Any]:
    command, standard_input = command_for(binary, path, fixture)
    started = time.perf_counter()
    result = subprocess.run(
        command,
        check=False,
        text=True,
        input=standard_input,
        capture_output=True,
    )
    elapsed = time.perf_counter() - started
    if result.returncode == 0:
        graph = json.loads(result.stdout)
        return {
            "status": "success",
            "json": result.stdout,
            "graph": graph,
            "elapsed": elapsed,
        }
    return {
        "status": "failure",
        "error": semantic_error(result.stderr),
        "elapsed": elapsed,
    }


def classify(
    binary: Path,
    path: Path,
    fixture: dict[str, Any],
) -> tuple[str, str | None]:
    command, standard_input = command_for(binary, path, fixture)
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


def expected_tersmu(fixture: dict[str, Any]) -> dict[str, Any] | None:
    tersmu = fixture.get("expectations", {}).get("output", {}).get("tersmu")
    return tersmu if isinstance(tersmu, dict) else None


def verify_expectation(
    path: Path,
    expectation: dict[str, Any] | None,
    result: dict[str, Any],
) -> None:
    if expectation is None:
        return
    if "json" in expectation:
        if result["status"] != "success":
            raise ValueError(f"{path}: expected success, got {result!r}")
        expected_json = expectation["json"]
        if isinstance(expected_json, str):
            if result["json"].removesuffix("\n") != expected_json:
                raise ValueError(f"{path}: inline tersmu JSON is stale")
        elif isinstance(expected_json, dict) and set(expected_json) == {"sha256"}:
            actual_hash = hashlib.sha256(
                result["json"].removesuffix("\n").encode()
            ).hexdigest()
            if actual_hash != expected_json["sha256"]:
                raise ValueError(
                    f"{path}: tersmu JSON hash is stale: {actual_hash}"
                )
        else:
            raise ValueError(f"{path}: unrecognized tersmu JSON expectation")
        return
    expected_error = expectation.get("error")
    prefix = "tersmu JSON build error: "
    if not isinstance(expected_error, str) or not expected_error.startswith(prefix):
        raise ValueError(f"{path}: malformed tersmu failure expectation")
    actual = {"status": result["status"], "error": result.get("error")}
    expected = {"status": "failure", "error": expected_error.removeprefix(prefix)}
    if actual != expected:
        raise ValueError(f"{path}: stale tersmu failure: {actual!r}")


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


def question_summaries(graph: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        value
        for value in graph["objects"].values()
        if value.get("type") == "question"
    ]


def verify_fixture_migrations(
    base_ref: str,
    base_binary: Path,
    current_binary: Path,
) -> None:
    results: dict[Path, tuple[dict[str, Any], dict[str, Any]]] = {}
    for path in sorted(MIGRATED_FIXTURE_PATHS):
        base_fixture = document(git_source(base_ref, path))
        current_fixture = document(path.read_text())
        if without_tersmu(base_fixture) != without_tersmu(current_fixture):
            raise ValueError(f"{path}: changed beyond its tersmu expectation")
        base_result = render(base_binary, path, base_fixture)
        current_result = render(current_binary, path, current_fixture)
        verify_expectation(path, expected_tersmu(base_fixture), base_result)
        verify_expectation(path, expected_tersmu(current_fixture), current_result)
        results[path] = (base_result, current_result)

    full_base, full_current = results[FULL_ALICE_PATH]
    if full_base.get("error") != FULL_ALICE_BASE_ERROR:
        raise ValueError(f"Full Alice base blocker changed: {full_base!r}")
    if full_current["status"] != "success":
        raise ValueError(f"Full Alice did not analyze end-to-end: {full_current!r}")
    full_graph = full_current["graph"]
    if len(full_graph["objects"]) != 45905 or full_graph["root"] != "sequence:46614":
        raise ValueError("Full Alice graph shape changed")
    full_hash = hashlib.sha256(
        full_current["json"].removesuffix("\n").encode()
    ).hexdigest()
    print(
        "corpus.alis.full-alice: unsupported -> success: all mixed direct "
        "questions retain ordered typed slots; "
        f"objects=45905 root=sequence:46614 elapsed={full_current['elapsed']:.2f}s "
        f"sha256={full_hash}"
    )

    fragment_base, fragment_current = results[QUANTIFIED_FRAGMENT_PATH]
    if not fragment_base.get("error", "").startswith(QUANTIFIED_FRAGMENT_ERROR_PREFIX):
        raise ValueError("quantified fragment base blocker changed")
    fragment_formulas = [
        value
        for value in fragment_current["graph"]["objects"].values()
        if value.get("type") == "formula"
    ]
    if not any(value.get("operator") == "cardinality" for value in fragment_formulas):
        raise ValueError("quantified fragment lost its cardinality scope")
    print(
        "cll.17.39.c17e11d6: other-error -> success: the typed fragment "
        "preserves its exact cardinality scope"
    )

    north_base, north_current = results[NORTH_WIND_PATH]
    base_questions = question_summaries(north_base["graph"])
    current_questions = question_summaries(north_current["graph"])
    if len(base_questions) != 1 or len(current_questions) != 3:
        raise ValueError("north-wind indirect-question count changed unexpectedly")
    added = current_questions[1:]
    if not all(
        question.get("kind") == "quantity"
        and question.get("mode") == "indirect"
        and question.get("domain") == "number"
        and len(question.get("slots", [])) == 1
        for question in added
    ):
        raise ValueError("north-wind xo kau questions are not typed quantity slots")
    print(
        "cll.chrestomathy.north-wind: success -> success: two xo kau "
        "occurrences now retain distinct indirect Number answer slots"
    )

    reset_base, reset_current = results[RESET_QUESTION_PATH]
    if not any(
        question.get("source", {}).get("text") == "ca ma ki klama"
        for question in question_summaries(reset_base["graph"])
    ):
        raise ValueError("reset fixture base graph lacks the old vacuous question")
    if question_summaries(reset_current["graph"]):
        raise ValueError("bare KI failed to cancel the preceding tense-anchor question")
    print(
        "corpus.camxes.6695: success -> success: CLL 10.13 bare KI clears "
        "the preceding ca ma anchor and its answer slot together"
    )

    changed = changed_fixture_paths(base_ref)
    if changed != MIGRATED_FIXTURE_PATHS:
        raise ValueError(f"unexpected fixture drift: {sorted(changed ^ MIGRATED_FIXTURE_PATHS)}")
    print("base-to-PR document reconstruction: 4/4")
    print("PR-to-base non-tersmu identity: 4/4")
    print("unexpected fixture drift: 0")


def error_class(error: str | None) -> str:
    if error == MULTIPLE_DOMAIN_ERROR:
        return "multiple direct answer domains"
    if (
        isinstance(error, str)
        and error.startswith(QUANTIFIED_FRAGMENT_ERROR_PREFIX)
        and error.endswith(QUANTIFIED_FRAGMENT_ERROR_SUFFIX)
    ):
        return "quantified sumti fragment"
    raise ValueError(f"unjustified former error: {error!r}")


def verify_global_transition_matrix(
    base_binary: Path,
    current_binary: Path,
    paths: dict[str, Path],
    removed: set[str],
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
    class_changes = []
    for fixture_id, base, current in outcomes:
        base_totals[base[0]] += 1
        current_totals[current[0]] += 1
        matrix[(base[0], current[0])] += 1
        if base[0] != current[0]:
            class_changes.append((fixture_id, base, current))
    for status in BEHAVIOR_STATUSES:
        base_totals.setdefault(status, 0)
        current_totals.setdefault(status, 0)
        for current_status in BEHAVIOR_STATUSES:
            matrix.setdefault((status, current_status), 0)

    if base_totals != EXPECTED_BASE_TOTALS:
        raise ValueError(f"global base totals changed: {dict(base_totals)!r}")
    if current_totals != EXPECTED_CURRENT_TOTALS:
        raise ValueError(f"global current totals changed: {dict(current_totals)!r}")
    if matrix != EXPECTED_MATRIX:
        raise ValueError(f"global transition matrix changed: {dict(matrix)!r}")

    unsupported_to_success = {
        fixture_id
        for fixture_id, base, current in class_changes
        if base[0] == "unsupported" and current[0] == "success"
    }
    if unsupported_to_success != removed:
        raise ValueError("allowlist shrink and unsupported-to-success set diverged")
    other_to_success = {
        fixture_id
        for fixture_id, base, current in class_changes
        if base[0] == "other-error" and current[0] == "success"
    }
    if len(other_to_success) != EXPECTED_OTHER_TO_SUCCESS_COUNT:
        raise ValueError("former principled error count changed")
    if ids_digest(other_to_success) != EXPECTED_OTHER_TO_SUCCESS_SHA256:
        raise ValueError("exact former principled error set changed")

    error_classes: Counter[str] = Counter()
    unsupported_classes: Counter[str] = Counter()
    for fixture_id, base, current in sorted(class_changes):
        if current[0] != "success":
            raise ValueError(f"{fixture_id}: unexplained non-success transition")
        if base[0] == "other-error":
            fixture_class = error_class(base[1])
            error_classes[fixture_class] += 1
        elif base[0] == "unsupported":
            fixture_class = str(base[1])
            unsupported_classes[fixture_class] += 1
        else:
            raise ValueError(f"{fixture_id}: unexpected transition {base} -> {current}")
        justification = JUSTIFICATIONS.get(fixture_class)
        if justification is None:
            raise ValueError(f"{fixture_id}: no justification for {fixture_class!r}")
        print(f"{fixture_id}: {base[0]} -> success: {justification}")

    if error_classes != EXPECTED_BASE_ERROR_CLASSES:
        raise ValueError(f"former error classes changed: {dict(error_classes)!r}")
    if unsupported_classes != EXPECTED_BASE_UNSUPPORTED_CLASSES:
        raise ValueError(
            f"former unsupported classes changed: {dict(unsupported_classes)!r}"
        )

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
    print("success -> error flips: 0")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=16)
    args = parser.parse_args()

    base_allowlist = allowlist_ids(git_source(args.base_ref, ALLOWLIST))
    current_allowlist = allowlist_ids(ALLOWLIST.read_text())
    if current_allowlist != EXPECTED_CURRENT_ALLOWLIST:
        raise ValueError(f"current allowlist changed: {sorted(current_allowlist)}")
    additions = current_allowlist - base_allowlist
    if additions:
        raise ValueError(f"the shrink-only allowlist gained entries: {sorted(additions)}")
    removed = base_allowlist - current_allowlist
    if len(removed) != EXPECTED_REMOVED_COUNT:
        raise ValueError(f"expected 18 removed ids, found {len(removed)}")
    if ids_digest(removed) != EXPECTED_REMOVED_SHA256:
        raise ValueError("the exact issue #377 allowlist removal set changed")

    paths = fixture_paths_by_id()
    missing = sorted((removed | {FULL_ALICE_ID}) - paths.keys())
    if missing:
        raise ValueError(f"migrated fixture ids have no fixture: {missing}")

    verify_fixture_migrations(
        args.base_ref,
        args.base_binary.resolve(),
        args.current_binary.resolve(),
    )
    verify_global_transition_matrix(
        args.base_binary.resolve(),
        args.current_binary.resolve(),
        paths,
        removed,
        args.jobs,
    )
    print("issue-377 fixture migration and transition matrix: verified")


if __name__ == "__main__":
    main()
