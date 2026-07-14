#!/usr/bin/env python3
"""Verify the deliberately narrow issue #373 tersmu fixture migration.

Issue #358 requires a two-sided gate for fixture expectation changes. This
script names every migrated fixture, proves the old expectation with a main
binary, proves the new expectation with the current binary, and reconstructs
each parsed TOML document in both directions by replacing only its tersmu
expectation.
"""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import tomllib
from pathlib import Path
from typing import Any


FIXTURES = {
    "tests/fixtures/cll/chapter-06/section-6.11/c6e11d7.toml": (
        "connected vocative",
        "joint .e now distributes performative vocativeTarget predications instead of "
        "inventing one composite addressee",
    ),
    "tests/fixtures/cll/chapter-08/section-8.9/c8e9d3.toml": (
        "connected vocative",
        "joint .e now distributes performative vocativeTarget predications instead of "
        "inventing one composite addressee",
    ),
    "tests/fixtures/cll/chapter-17/section-17.11/c17e11d6.toml": (
        "quantified fragment",
        "a standalone quantified description needs a truth-bearing discourse scope and "
        "therefore reports the typed context requirement",
    ),
    "tests/fixtures/cll/chrestomathy/alice01.toml": (
        "next blocker",
        "quantified sumti lowering succeeds and exposes the pre-existing tagged or "
        "connected selbri blocker",
    ),
    "tests/fixtures/cll/chrestomathy/in-xanadu.toml": (
        "next blocker",
        "multi-continuation sumti lowering succeeds and exposes the pre-existing CO "
        "selbri blocker",
    ),
}


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


def fixture_input(path: Path, fixture: dict[str, Any]) -> tuple[str, bool]:
    if "lojban" in fixture:
        return fixture["lojban"], False
    if "lojban-filename" in fixture:
        return str(path.parent / fixture["lojban-filename"]), True
    raise ValueError(f"{path}: fixture has neither lojban nor lojban-filename")


def semantic_error(stderr: str) -> str:
    prefixes = (
        "semantic error: ",
        "syntax error: ",
        "morphology error: ",
    )
    for line in reversed(stderr.splitlines()):
        for prefix in prefixes:
            if line.startswith(prefix):
                return line.removeprefix(prefix)
    raise ValueError(f"could not find the tersmu error in stderr: {stderr!r}")


def render(binary: Path, path: Path, fixture: dict[str, Any]) -> dict[str, Any]:
    command = [str(binary), "tersmu", "--color=never", "--format", "json"]
    dialect = fixture.get("dialect")
    if dialect is not None:
        command.extend(("--dialect", dialect))
    if tersmu(fixture).get("story-time", False):
        command.append("--story-time")
    input_value, is_file = fixture_input(path, fixture)
    if is_file:
        command.extend(("--file", input_value))
    else:
        command.append(input_value)
    result = run(*command)
    if result.returncode == 0:
        return {
            "status": "success",
            "json": result.stdout.removesuffix("\n"),
        }
    return {
        "status": "failure",
        "error": f"tersmu JSON build error: {semantic_error(result.stderr)}",
    }


def normalized_expectation(fixture: dict[str, Any]) -> dict[str, Any]:
    expectation = copy.deepcopy(tersmu(fixture))
    expectation.setdefault("status", "success")
    expectation.pop("story-time", None)
    return expectation


def verify_connected_vocative(path: str, current: dict[str, Any]) -> None:
    graph = json.loads(tersmu(current)["json"])
    objects = graph["objects"]
    utterance = objects[graph["root"]]
    if utterance.get("force") != "vocative" or "content" not in utterance:
        raise ValueError(f"{path}: connected vocative lacks formula content")
    formula = objects[utterance["content"]]
    if formula.get("operator") != "and":
        raise ValueError(f"{path}: connected vocative is not an and formula")
    if formula.get("connector", {}).get("source") != "e":
        raise ValueError(f"{path}: connected vocative lost its .e connector")
    children = [objects[child] for child in formula.get("children", [])]
    predications = [objects[child["predication"]] for child in children]
    if len(predications) != 2 or any(
        predication.get("relation") != "vocativeTarget"
        for predication in predications
    ):
        raise ValueError(f"{path}: vocativeTarget was not distributed over both branches")


def verify_expected_transition(path: str, current: dict[str, Any]) -> None:
    expectation = normalized_expectation(current)
    if path.endswith(("c6e11d7.toml", "c8e9d3.toml")):
        if expectation.get("status") != "success":
            raise ValueError(f"{path}: connected vocative must succeed")
        verify_connected_vocative(path, current)
        return
    expected_errors = {
        "tests/fixtures/cll/chapter-17/section-17.11/c17e11d6.toml": (
            "tersmu JSON build error: semantic analysis of the truth-bearing scope of "
            "quantified sumti fragment `vei ny lo prenu` requires discourse context"
        ),
        "tests/fixtures/cll/chrestomathy/alice01.toml": (
            "tersmu JSON build error: generated semantic builder does not yet support "
            "tagged or connected selbri"
        ),
        "tests/fixtures/cll/chrestomathy/in-xanadu.toml": (
            "tersmu JSON build error: generated semantic builder does not yet support CO selbri"
        ),
    }
    if expectation != {"status": "failure", "error": expected_errors[path]}:
        raise ValueError(f"{path}: unexpected failure transition: {expectation!r}")


def verify_two_sided_documents(
    path: str,
    old: dict[str, Any],
    current: dict[str, Any],
) -> None:
    forward = copy.deepcopy(old)
    forward["expectations"]["output"]["tersmu"] = copy.deepcopy(tersmu(current))
    if forward != current:
        raise ValueError(f"{path}: base-to-PR reconstruction changed non-tersmu data")

    reverse = copy.deepcopy(current)
    reverse["expectations"]["output"]["tersmu"] = copy.deepcopy(tersmu(old))
    if reverse != old:
        raise ValueError(f"{path}: PR-to-base reconstruction changed non-tersmu data")


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
    args = parser.parse_args()

    for path_text, (fixture_class, justification) in FIXTURES.items():
        path = Path(path_text)
        old = document(git_source(args.base_ref, path_text))
        current = document(path.read_text())
        verify_two_sided_documents(path_text, old, current)
        if normalized_expectation(old) != render(args.base_binary, path, old):
            raise ValueError(f"{path_text}: base expectation is stale")
        if normalized_expectation(current) != render(args.current_binary, path, current):
            raise ValueError(f"{path_text}: current expectation is stale")
        verify_expected_transition(path_text, current)
        print(f"{path_text}: {fixture_class}: {justification}")

    changed = changed_fixture_paths(args.base_ref)
    expected = set(FIXTURES)
    if changed != expected:
        extra = sorted(changed - expected)
        missing = sorted(expected - changed)
        raise ValueError(f"unexpected fixture drift: extra={extra}, missing={missing}")

    print("issue-373 fixture migration: verified")
    print("base expectations reproduced: 5/5")
    print("current expectations reproduced: 5/5")
    print("base-to-PR document reconstruction: 5/5")
    print("PR-to-base document reconstruction: 5/5")
    print("unexpected fixture drift: 0")


if __name__ == "__main__":
    main()
