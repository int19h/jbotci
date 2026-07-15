#!/usr/bin/env python3
"""Verify issue #379's fixture migrations and complete behavior matrix.

Issue #358 requires changed expectations to be reconstructed from both sides.
This verifier proves the final allowlist removal, reconstructs Alice and full
Alice with the main and issue binaries, checks the mechanism-specific graphs,
classifies every syntax-success fixture through the full four-by-four matrix,
and verifies byte identity for unaffected probes.
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
ALICE_PATH = Path("tests/fixtures/cll/chrestomathy/alice01.toml")
FULL_ALICE_PATH = Path("tests/fixtures/corpus/alis/full-alice.toml")
MIGRATED_FIXTURE_PATHS = {ALICE_PATH, FULL_ALICE_PATH}
ISSUE_IDS = {"cll.chrestomathy.alice01", "corpus.alis.full-alice"}
UNSUPPORTED_PREFIX = "generated semantic builder does not yet support "
BASE_ALICE_ERROR = UNSUPPORTED_PREFIX + "quantified sumti"
BASE_ALLOWLIST = {"cll.chrestomathy.alice01"}
REMOVED_ALLOWLIST_SHA256 = (
    "17a503d48f6b5d52d5399b7028a1c9449916b883ee62e5c85662bf267071e156"
)
ALICE_CURRENT_SHA256 = (
    "975e2da770b1b0d02ad8dca3baeabb588efb89b26f1283f0a6fdf4383e697320"
)
FULL_ALICE_BASE_SHA256 = (
    "05eb3650a96bfd9674db87a02af33b5c6e0594bfc99384303787f118b9db16d6"
)
FULL_ALICE_CURRENT_SHA256 = (
    "cea057ee660a723749cff0e624eab6cdeeab7575825ca4883f9acb92ba063fdb"
)
BEHAVIOR_STATUSES = ("success", "other-error", "unsupported", "panic")
EXPECTED_BASE_TOTALS = Counter(
    {"success": 22253, "other-error": 83, "unsupported": 1, "panic": 0}
)
EXPECTED_CURRENT_TOTALS = Counter(
    {"success": 22254, "other-error": 83, "unsupported": 0, "panic": 0}
)
EXPECTED_MATRIX = Counter(
    {
        ("success", "success"): 22253,
        ("other-error", "other-error"): 83,
        ("unsupported", "success"): 1,
    }
)
UNAFFECTED_INPUTS = (
    "mi klama",
    "mi klama i je do cadzu",
    "lo mlatu joi lo gerku cu klama",
    "ganse je zukte nirna",
    "le cecmu ji le velsku cu vajni",
    "ga mi broda gi do brode",
    "pu mi klama",
    "mi na klama",
)
STATIC_PROBES = {
    "CO stream partition": (
        "fe lu .ua virnu li'u fa le se lanzu ba cusku co jinvi be fi mi",
        "58aef6de91c0313da255cadbe26b8c151ee808322e8933616dd994134c2d8c08",
    ),
    "connected branch-local FAI": (
        "mi jai gau kalri fai le vorme gi'e zgana",
        "1fb668ed916bdd6ea57ef03647b6199615b5886e2154c67648d8f73abb32429a",
    ),
    "quantified FAI scope": (
        "mi jai gau morsi fai su'o da",
        "cf0fe629b09d037a6de11ddfc9c487cda5014e586bf2d9c196c0995b729e719d",
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
            output = result.stdout.removesuffix("\n")
            outcome["json"] = output
            outcome["sha256"] = hashlib.sha256(output.encode()).hexdigest()
            outcome["graph"] = json.loads(output)
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
        if isinstance(expected, str) and result["json"] != expected:
            raise ValueError(f"{path}: inline tersmu JSON is stale")
        if isinstance(expected, dict) and expected.get("sha256") != result["sha256"]:
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


def named_predications(graph: dict[str, Any], relation: str) -> list[dict[str, Any]]:
    return [
        value
        for value in graph["objects"].values()
        if value.get("type") == "predication" and value.get("relation") == relation
    ]


def argument_source(graph: dict[str, Any], predication: dict[str, Any], place: str) -> str | None:
    value = predication.get("arguments", {}).get(place, {}).get("value")
    return graph["objects"].get(value, {}).get("source", {}).get("text")


def verify_static_probe_graphs(current_binary: Path) -> None:
    graphs: dict[str, dict[str, Any]] = {}
    for name, (source, expected_hash) in STATIC_PROBES.items():
        result = execute(
            current_binary,
            Path(f"static/{name}"),
            {"lojban": source},
            capture_json=True,
        )
        if result["status"] != "success" or result["sha256"] != expected_hash:
            raise ValueError(f"{name}: exact output changed: {result!r}")
        graphs[name] = result["graph"]
        print(
            f"static probe {name}: success objects={len(result['graph']['objects'])} "
            f"sha256={result['sha256']}"
        )

    co = graphs["CO stream partition"]
    cusku = named_predications(co, "cusku")
    jinvi = named_predications(co, "jinvi")
    if len(cusku) != 1 or len(jinvi) != 1:
        raise ValueError("CO probe lost its cusku/jinvi predications")
    if argument_source(co, cusku[0], "x2") != "lu .ua virnu li'u":
        raise ValueError("pre-CO FE term did not remain cusku x2")
    if jinvi[0].get("arguments", {}).get("x3", {}).get("value") != "entity:1":
        raise ValueError("post-CO BE/FI term did not remain jinvi x3")

    connected = graphs["connected branch-local FAI"]
    kalri = named_predications(connected, "kalri")
    zgana = named_predications(connected, "zgana")
    if len(kalri) != 1 or len(zgana) != 1:
        raise ValueError("connected FAI probe lost a bridi branch")
    if argument_source(connected, kalri[0], "x1") != "le vorme":
        raise ValueError("branch-local FAI did not restore kalri x1")
    if zgana[0].get("arguments", {}).get("x1", {}).get("value") != "entity:1":
        raise ValueError("shared leading mi did not remain zgana x1")

    quantified = graphs["quantified FAI scope"]
    morsi = named_predications(quantified, "morsi")
    if len(morsi) != 1:
        raise ValueError("quantified FAI probe lost morsi")
    restored = morsi[0].get("arguments", {}).get("x1", {}).get("value")
    quantifiers = [
        value
        for value in quantified["objects"].values()
        if value.get("type") == "formula"
        and value.get("operator") == "cardinality"
        and value.get("variable") == restored
    ]
    if len(quantifiers) != 1 or quantifiers[0].get("source", {}).get("text") != "su'o da":
        raise ValueError("quantified FAI x1 escaped its cardinality scope")
    print("static mechanism graphs: CO partition, branch-local FAI, quantified FAI scope")


def verify_fixture_migrations(
    base_ref: str, base_binary: Path, current_binary: Path
) -> None:
    results: dict[Path, tuple[dict[str, Any], dict[str, Any]]] = {}
    for path in sorted(MIGRATED_FIXTURE_PATHS):
        base_fixture = document(git_source(base_ref, path))
        current_fixture = document(path.read_text())
        if without_tersmu(base_fixture) != without_tersmu(current_fixture):
            raise ValueError(f"{path}: changed beyond its tersmu expectation")
        base_result = execute(base_binary, path, base_fixture, capture_json=True)
        current_result = execute(current_binary, path, current_fixture, capture_json=True)
        verify_expectation(path, expected_tersmu(base_fixture), base_result)
        verify_expectation(path, expected_tersmu(current_fixture), current_result)
        results[path] = (base_result, current_result)

    base_alice, current_alice = results[ALICE_PATH]
    if base_alice.get("error") != BASE_ALICE_ERROR:
        raise ValueError(f"Alice base blocker changed: {base_alice!r}")
    if current_alice.get("sha256") != ALICE_CURRENT_SHA256:
        raise ValueError(f"Alice final graph changed: {current_alice!r}")
    alice_graph = current_alice["graph"]
    if not alice_graph["root"].startswith("sequence:") or len(alice_graph["objects"]) != 4121:
        raise ValueError("Alice final graph shape changed")
    print(
        "cll.chrestomathy.alice01: unsupported -> success: "
        f"objects=4121 sha256={ALICE_CURRENT_SHA256}"
    )

    base_full, current_full = results[FULL_ALICE_PATH]
    if base_full.get("sha256") != FULL_ALICE_BASE_SHA256:
        raise ValueError(f"full Alice base graph changed: {base_full!r}")
    if current_full.get("sha256") != FULL_ALICE_CURRENT_SHA256:
        raise ValueError(f"full Alice final graph changed: {current_full!r}")
    full_graph = current_full["graph"]
    if not full_graph["root"].startswith("sequence:") or len(full_graph["objects"]) != 45905:
        raise ValueError("full Alice final graph shape changed")
    print(
        "corpus.alis.full-alice: success -> success graph correction: "
        f"objects=45905 sha256={FULL_ALICE_CURRENT_SHA256}"
    )

    changed = changed_fixture_paths(base_ref)
    if changed != MIGRATED_FIXTURE_PATHS:
        raise ValueError(f"unexpected fixture drift: {sorted(changed ^ MIGRATED_FIXTURE_PATHS)}")
    print("base-to-PR document reconstruction: 2/2")
    print("PR-to-base non-tersmu identity: 2/2")
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
    if changed_ids != {"cll.chrestomathy.alice01"}:
        raise ValueError(f"unexpected behavior-class changes: {sorted(changed_ids)}")

    print("global behavior totals:")
    print(f"  base: {dict(base_totals)}")
    print(f"  current: {dict(current_totals)}")
    print("global transition matrix:")
    for base in BEHAVIOR_STATUSES:
        for current in BEHAVIOR_STATUSES:
            print(f"  {base} -> {current}: {matrix[(base, current)]}")
    print("success -> error flips: 0")


def verify_unaffected_identity(base_binary: Path, current_binary: Path) -> None:
    for source in UNAFFECTED_INPUTS:
        command = ("tersmu", "--color=never", "--file", "/dev/stdin")
        base = subprocess.run(
            (str(base_binary), *command),
            check=False,
            input=source,
            text=True,
            capture_output=True,
        )
        current = subprocess.run(
            (str(current_binary), *command),
            check=False,
            input=source,
            text=True,
            capture_output=True,
        )
        base_bytes = base.stdout.encode() + base.stderr.encode()
        current_bytes = current.stdout.encode() + current.stderr.encode()
        if base.returncode != current.returncode or base_bytes != current_bytes:
            raise ValueError(f"unaffected probe changed: {source!r}")
        digest = hashlib.sha256(current_bytes).hexdigest()
        print(f"byte-identical: {source!r} bytes={len(current_bytes)} sha256={digest}")
    print(f"main-vs-PR unaffected byte identity: {len(UNAFFECTED_INPUTS)}/{len(UNAFFECTED_INPUTS)}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    parser.add_argument("--jobs", type=int, default=16)
    args = parser.parse_args()

    base_allowlist = allowlist_ids(git_source(args.base_ref, ALLOWLIST))
    current_allowlist = allowlist_ids(ALLOWLIST.read_text())
    if base_allowlist != BASE_ALLOWLIST:
        raise ValueError(f"base allowlist changed: {sorted(base_allowlist)}")
    if current_allowlist:
        raise ValueError(f"final allowlist is not empty: {sorted(current_allowlist)}")
    removed = base_allowlist - current_allowlist
    if ids_digest(removed) != REMOVED_ALLOWLIST_SHA256:
        raise ValueError(f"the exact issue #379 removal set changed: {sorted(removed)}")

    paths = fixture_paths_by_id()
    if missing := sorted(ISSUE_IDS - paths.keys()):
        raise ValueError(f"issue fixture ids have no fixture: {missing}")
    base_binary = args.base_binary.resolve()
    current_binary = args.current_binary.resolve()
    verify_fixture_migrations(args.base_ref, base_binary, current_binary)
    verify_static_probe_graphs(current_binary)
    verify_global_transition_matrix(base_binary, current_binary, paths, args.jobs)
    verify_unaffected_identity(base_binary, current_binary)
    print("allowlist: 0 entries; unsupported/panic coverage is unconditional")
    print("issue-379 fixture migration and transition matrix: verified")


if __name__ == "__main__":
    main()
