#!/usr/bin/env python3
"""Verify issue #378 fixture migrations and the complete behavior matrix.

Issue #358 requires expectation changes to be reproducible from both sides.
This verifier freezes the exact allowlist shrink, reconstructs all four changed
fixture documents with main and issue binaries, checks the mechanism-specific
graphs, and classifies every syntax-success fixture in both revisions.  It also
prints every issue fixture's transition and justification instead of reducing
the audit to totals.
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
JOI_PATH = Path(
    "tests/fixtures/adhoc/v0/warnings/standard-no-warning/"
    "standard-pre-i-joi-statement-connective.toml"
)
ALICE_PATH = Path("tests/fixtures/cll/chrestomathy/alice01.toml")
SUI_PATH = Path("tests/fixtures/corpus/camxes/17625.toml")
JI_PATH = Path("tests/fixtures/corpus/camxes/18927.toml")
MIGRATED_FIXTURE_PATHS = {JOI_PATH, ALICE_PATH, SUI_PATH, JI_PATH}

ISSUE_IDS = {
    "adhoc.v0.warnings.standard-no-warning.standard-pre-i-joi-statement-connective",
    "cll.chrestomathy.alice01",
    "corpus.camxes.17625",
    "corpus.camxes.18927",
}
EXPECTED_CURRENT_ALLOWLIST = {"cll.chrestomathy.alice01"}
EXPECTED_REMOVED_SHA256 = (
    "4833e3167729bbb97d7abd25f6d65863db91e942e39fb143fbbb3cb6957e628b"
)
UNSUPPORTED_PREFIX = "generated semantic builder does not yet support "
BASE_ERRORS = {
    JOI_PATH: UNSUPPORTED_PREFIX + "nonlogical generated statement connective",
    ALICE_PATH: UNSUPPORTED_PREFIX + "connected bridi tail",
    SUI_PATH: UNSUPPORTED_PREFIX
    + "unsupported nonlogical argument connective su'i",
    JI_PATH: UNSUPPORTED_PREFIX + "unsupported nonlogical relation connective ji",
}
CURRENT_ERRORS = {
    ALICE_PATH: UNSUPPORTED_PREFIX + "quantified sumti",
    SUI_PATH: (
        "semantic interpretation is undefined for the experimental VUhU argument "
        "connective `su'i` outside a mekso expression"
    ),
}
BEHAVIOR_STATUSES = ("success", "other-error", "unsupported", "panic")
EXPECTED_BASE_TOTALS = Counter(
    {"success": 22251, "other-error": 82, "unsupported": 4, "panic": 0}
)
EXPECTED_CURRENT_TOTALS = Counter(
    {"success": 22253, "other-error": 83, "unsupported": 1, "panic": 0}
)
EXPECTED_MATRIX = Counter(
    {
        ("success", "success"): 22251,
        ("other-error", "other-error"): 82,
        ("unsupported", "success"): 2,
        ("unsupported", "other-error"): 1,
        ("unsupported", "unsupported"): 1,
    }
)
JUSTIFICATIONS = {
    JOI_PATH: (
        "CLL 14.14-14.15 defines pre-I JOI as a joint-event connection; the "
        "sequence now carries typed operator=mass metadata and no truth formula"
    ),
    ALICE_PATH: (
        "CLL 7.6 defines NEI as the current bridi; both Alice paragraphs now "
        "replay the complete GIhE graph with x1 substituted, then the full fixture "
        "reaches the unrelated #379 static quantified-sumti blocker"
    ),
    SUI_PATH: (
        "CLL 18.5 defines VUhU between mekso operands, not general sumti; the "
        "experimental extension now names that semantic gap precisely"
    ),
    JI_PATH: (
        "CLL 14.13 and 19.5 define connective-question answer slots; JI now "
        "builds a typed connective parameter over the two property branches"
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
    binary: Path, path: Path, fixture: dict[str, Any]
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


def execute(
    binary: Path,
    path: Path,
    fixture: dict[str, Any],
    *,
    capture_json: bool,
) -> dict[str, Any]:
    command, standard_input = command_for(binary, path, fixture)
    result = subprocess.run(
        command,
        check=False,
        text=True,
        input=standard_input,
        stdout=subprocess.PIPE if capture_json else subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    if result.returncode == 0:
        outcome: dict[str, Any] = {"status": "success"}
        if capture_json:
            outcome["json"] = result.stdout
            outcome["graph"] = json.loads(result.stdout)
        return outcome
    try:
        error = semantic_error(result.stderr)
    except ValueError:
        message = result.stderr.splitlines()[-1] if result.stderr else "no stderr"
        status = (
            "panic"
            if result.returncode == 101 or "panicked at" in result.stderr
            else "other-error"
        )
        return {"status": status, "error": message}
    status = "unsupported" if error.startswith(UNSUPPORTED_PREFIX) else "other-error"
    return {"status": status, "error": error}


def expected_tersmu(fixture: dict[str, Any]) -> dict[str, Any] | None:
    value = fixture.get("expectations", {}).get("output", {}).get("tersmu")
    return value if isinstance(value, dict) else None


def verify_expectation(
    path: Path, expectation: dict[str, Any] | None, result: dict[str, Any]
) -> None:
    if expectation is None:
        return
    if "json" in expectation:
        if result["status"] != "success":
            raise ValueError(f"{path}: expected success, got {result!r}")
        expected = expectation["json"]
        actual = result["json"].removesuffix("\n")
        if isinstance(expected, str) and actual != expected:
            raise ValueError(f"{path}: inline tersmu JSON is stale")
        if isinstance(expected, dict) and expected.get("sha256") != hashlib.sha256(
            actual.encode()
        ).hexdigest():
            raise ValueError(f"{path}: tersmu JSON hash is stale")
        return
    prefix = "tersmu JSON build error: "
    expected_error = expectation.get("error")
    if not isinstance(expected_error, str) or not expected_error.startswith(prefix):
        raise ValueError(f"{path}: malformed tersmu failure expectation")
    if result.get("error") != expected_error.removeprefix(prefix):
        raise ValueError(f"{path}: stale tersmu failure: {result!r}")


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


def formulae(graph: dict[str, Any], operator: str) -> list[dict[str, Any]]:
    return [
        value
        for value in graph["objects"].values()
        if value.get("type") == "formula" and value.get("operator") == operator
    ]


def verify_issue_graphs(
    results: dict[Path, dict[str, Any]], current_binary: Path
) -> None:
    joi = results[JOI_PATH]["graph"]
    sequence = joi["objects"][joi["root"]]
    if sequence.get("type") != "sequence" or sequence.get("content") is not None:
        raise ValueError("pre-I JOI became a truth-functional formula")
    if sequence.get("nonlogicalConnection") != {
        "operator": "mass",
        "connector": {"source": "joi", "locus": "statement"},
    }:
        raise ValueError("pre-I JOI lost its typed mass connection")
    if formulae(joi, "and"):
        raise ValueError("pre-I JOI regressed to logical AND")

    ji = results[JI_PATH]["graph"]
    utterance = ji["objects"][ji["root"]]
    question = ji["objects"].get(utterance.get("content"), {})
    connections = formulae(ji, "connectiveQuestion")
    if (
        utterance.get("force") != "ask"
        or question.get("kind") != "connective"
        or question.get("mode") != "direct"
        or len(connections) != 1
    ):
        raise ValueError("JI relation connection is not a direct connective question")
    connector = connections[0].get("connector", {})
    parameter = ji["objects"].get(connector.get("parameter"), {})
    if (
        connector.get("source") != "ji"
        or connector.get("locus") != "property-abstraction"
        or connector.get("truthTable") is not None
        or parameter.get("sort") != "connective"
        or parameter.get("role") != "connectiveQuestion"
    ):
        raise ValueError("JI relation graph lost typed connector metadata")

    alice_text = (ALICE_PATH.parent / "texts/alice01.lojban").read_text().splitlines()
    for line_number in (5, 13):
        fixture = {"lojban": alice_text[line_number - 1]}
        outcome = execute(current_binary, ALICE_PATH, fixture, capture_json=True)
        if outcome["status"] != "success":
            raise ValueError(f"Alice paragraph {line_number} still fails: {outcome!r}")
        replay = [
            value
            for value in formulae(outcome["graph"], "and")
            if value.get("connector", {}).get("source") == "gi'e"
            and any(
                diagnostic.get("message")
                == "recursive inherited pro-bridi argument was elided to keep the semantic graph finite"
                for diagnostic in value.get("diagnostics", [])
            )
        ]
        if not replay or any(len(value.get("children", [])) < 2 for value in replay):
            raise ValueError(f"Alice paragraph {line_number} lost connected NEI replay")


def verify_fixture_migrations(
    base_ref: str, base_binary: Path, current_binary: Path
) -> None:
    current_results: dict[Path, dict[str, Any]] = {}
    for path in sorted(MIGRATED_FIXTURE_PATHS):
        base_fixture = document(git_source(base_ref, path))
        current_fixture = document(path.read_text())
        if without_tersmu(base_fixture) != without_tersmu(current_fixture):
            raise ValueError(f"{path}: changed beyond its tersmu expectation")
        base_result = execute(base_binary, path, base_fixture, capture_json=True)
        current_result = execute(current_binary, path, current_fixture, capture_json=True)
        verify_expectation(path, expected_tersmu(base_fixture), base_result)
        verify_expectation(path, expected_tersmu(current_fixture), current_result)
        if base_result.get("error") != BASE_ERRORS[path]:
            raise ValueError(f"{path}: base blocker changed: {base_result!r}")
        expected_current_error = CURRENT_ERRORS.get(path)
        if expected_current_error is None and current_result["status"] != "success":
            raise ValueError(f"{path}: did not become successful: {current_result!r}")
        if expected_current_error is not None and current_result.get("error") != expected_current_error:
            raise ValueError(f"{path}: current blocker changed: {current_result!r}")
        current_results[path] = current_result
        print(
            f"{current_fixture['id']}: {base_result['status']} -> "
            f"{current_result['status']}: {JUSTIFICATIONS[path]}"
        )

    verify_issue_graphs(current_results, current_binary)
    changed = changed_fixture_paths(base_ref)
    if changed != MIGRATED_FIXTURE_PATHS:
        raise ValueError(f"unexpected fixture drift: {sorted(changed ^ MIGRATED_FIXTURE_PATHS)}")
    print("exact mechanism graphs: JOI mass, JI connective question, connected NEI replay")
    print("base-to-PR document reconstruction: 4/4")
    print("PR-to-base non-tersmu identity: 4/4")
    print("unexpected fixture drift: 0")


def verify_global_transition_matrix(
    base_binary: Path, current_binary: Path, paths: dict[str, Path], jobs: int
) -> None:
    fixtures = []
    for fixture_id, path in paths.items():
        fixture = document(path.read_text())
        syntax = fixture.get("expectations", {}).get("syntax")
        if isinstance(syntax, dict) and syntax.get("status") == "success":
            fixtures.append((fixture_id, path, fixture))

    def classify_pair(
        item: tuple[str, Path, dict[str, Any]],
    ) -> tuple[str, str, str]:
        fixture_id, path, fixture = item
        base = execute(base_binary, path, fixture, capture_json=False)["status"]
        current = execute(current_binary, path, fixture, capture_json=False)["status"]
        return fixture_id, base, current

    with ThreadPoolExecutor(max_workers=jobs) as executor:
        outcomes = list(executor.map(classify_pair, fixtures))

    base_totals: Counter[str] = Counter()
    current_totals: Counter[str] = Counter()
    matrix: Counter[tuple[str, str]] = Counter()
    changed_ids: set[str] = set()
    for fixture_id, base, current in outcomes:
        base_totals[base] += 1
        current_totals[current] += 1
        matrix[(base, current)] += 1
        if base != current:
            changed_ids.add(fixture_id)
    for base in BEHAVIOR_STATUSES:
        base_totals.setdefault(base, 0)
        current_totals.setdefault(base, 0)
        for current in BEHAVIOR_STATUSES:
            matrix.setdefault((base, current), 0)
    if base_totals != EXPECTED_BASE_TOTALS:
        raise ValueError(f"global base totals changed: {dict(base_totals)!r}")
    if current_totals != EXPECTED_CURRENT_TOTALS:
        raise ValueError(f"global current totals changed: {dict(current_totals)!r}")
    if matrix != EXPECTED_MATRIX:
        raise ValueError(f"global transition matrix changed: {dict(matrix)!r}")
    if changed_ids != ISSUE_IDS - EXPECTED_CURRENT_ALLOWLIST:
        raise ValueError(f"unexpected behavior-class changes: {sorted(changed_ids)}")

    print("global behavior totals:")
    print(f"  base: {dict(base_totals)}")
    print(f"  current: {dict(current_totals)}")
    print("global transition matrix:")
    for base in BEHAVIOR_STATUSES:
        for current in BEHAVIOR_STATUSES:
            print(f"  {base} -> {current}: {matrix[(base, current)]}")
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
    if current_allowlist - base_allowlist:
        raise ValueError("the shrink-only allowlist gained entries")
    removed = base_allowlist - current_allowlist
    if len(removed) != 3 or ids_digest(removed) != EXPECTED_REMOVED_SHA256:
        raise ValueError(f"the exact issue #378 removal set changed: {sorted(removed)}")

    paths = fixture_paths_by_id()
    if missing := sorted(ISSUE_IDS - paths.keys()):
        raise ValueError(f"issue fixture ids have no fixture: {missing}")
    verify_fixture_migrations(
        args.base_ref, args.base_binary.resolve(), args.current_binary.resolve()
    )
    verify_global_transition_matrix(
        args.base_binary.resolve(), args.current_binary.resolve(), paths, args.jobs
    )
    print(
        "allowlist: 1 entry; cll.chrestomathy.alice01 is blocked only by the "
        "separate #379 static quantified-sumti call site"
    )
    print("issue-378 fixture migration and transition matrix: verified")


if __name__ == "__main__":
    main()
