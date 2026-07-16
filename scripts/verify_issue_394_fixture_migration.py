#!/usr/bin/env python3
"""Verify the renderer-only issue #394 fixture migration.

Issue #358 requires a two-sided gate for every changed existing fixture. This
script reconstructs every migrated document in both directions, reproduces all
new and migrated derived expectations from the appropriate binaries, checks
every full-Alice eventuality content edge, pins the unchanged ka guard, and
proves canonical JSON byte identity on six inputs.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import tomllib
from pathlib import Path
from typing import Any


MIGRATED_FIXTURES = (
    Path("tests/fixtures/adhoc/output/tersmu-derived-sequence-binding.toml"),
    Path("tests/fixtures/corpus/alis/full-alice.toml"),
)
FULL_ALICE = Path("tests/fixtures/corpus/alis/full-alice.toml")
NEW_FIXTURES = (
    Path("tests/fixtures/adhoc/output/tersmu-derived-eventuality-content.toml"),
    Path(
        "tests/fixtures/adhoc/output/tersmu-derived-eventuality-content-headline.toml"
    ),
    Path(
        "tests/fixtures/adhoc/output/tersmu-derived-nested-eventuality-content.toml"
    ),
    Path("tests/fixtures/adhoc/output/tersmu-derived-proposition-abstraction.toml"),
)
KA_GUARD = Path(
    "tests/fixtures/adhoc/output/tersmu-derived-property-abstraction.toml"
)
HEADLINE = (
    "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u "
    "xendo je cnikansa ro lo jmive kei ta'i lo racli"
)
CANONICAL_INPUTS = (
    ("headline", HEADLINE),
    ("nitcu-lo-nu", "mi nitcu lo nu mi klama"),
    ("kucli-duhu", "mi kucli lo du'u do klama"),
    ("nested-nu", "mi djica lo nu do djica lo nu mi klama"),
    ("ka-guard", "mi kakne lo ka ce'u bajra"),
    ("tensed", "mi pu klama"),
)


def run(*args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(args, check=False, capture_output=True)


def git_source(base_ref: str, path: Path) -> str:
    result = run("git", "show", f"{base_ref}:{path}")
    if result.returncode != 0:
        raise ValueError(result.stderr.decode().strip())
    return result.stdout.decode()


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


def render_format(
    binary: Path,
    path: Path,
    fixture: dict[str, Any],
    format_name: str,
) -> str:
    command = [str(binary), "tersmu", "--color=never", "--format", format_name]
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
    if result.returncode != 0:
        raise ValueError(
            f"{path}: {format_name} render failed: {result.stderr.decode().strip()}"
        )
    return result.stdout.decode().removesuffix("\n")


def render_derived(
    binary: Path, path: Path, fixture: dict[str, Any]
) -> dict[str, Any]:
    expectation = tersmu(fixture)
    rendered = {
        format_name: render_format(binary, path, fixture, format_name)
        for format_name in ("tree", "tree+proj")
        if format_name in expectation
    }
    if "status" in expectation:
        rendered["status"] = "success"
    return rendered


def normalized_expectation(fixture: dict[str, Any]) -> dict[str, Any]:
    expectation = copy.deepcopy(tersmu(fixture))
    expectation.pop("story-time", None)
    return expectation


def verify_two_sided_document(
    path: Path, old: dict[str, Any], current: dict[str, Any]
) -> None:
    forward = copy.deepcopy(old)
    forward["expectations"]["output"]["tersmu"] = copy.deepcopy(tersmu(current))
    if forward != current:
        raise ValueError(
            f"{path}: base-to-PR reconstruction changed non-tersmu data"
        )

    reverse = copy.deepcopy(current)
    reverse["expectations"]["output"]["tersmu"] = copy.deepcopy(tersmu(old))
    if reverse != old:
        raise ValueError(
            f"{path}: PR-to-base reconstruction changed non-tersmu data"
        )


def verify_derived_expectations(
    binary: Path, path: Path, fixture: dict[str, Any]
) -> None:
    expectation = tersmu(fixture)
    for format_name in ("tree", "tree+proj"):
        expected = expectation.get(format_name)
        if expected is None:
            continue
        actual = render_format(binary, path, fixture, format_name)
        if isinstance(expected, str):
            matches = actual == expected
        elif isinstance(expected, dict) and set(expected) == {"sha256"}:
            matches = hashlib.sha256(actual.encode()).hexdigest() == expected["sha256"]
        else:
            raise ValueError(f"{path}: unsupported {format_name} expectation {expected!r}")
        if not matches:
            raise ValueError(f"{path}: {format_name} expectation is stale")


def verify_full_alice_content_edges(current_binary: Path) -> None:
    fixture = document(FULL_ALICE.read_text())
    graph = json.loads(render_format(current_binary, FULL_ALICE, fixture, "json"))
    objects = graph["objects"]
    edges = [
        (eventuality, object_["content"])
        for eventuality, object_ in objects.items()
        if object_.get("denotation") == "generated-bound" and "content" in object_
    ]
    if len(edges) != 58:
        raise ValueError(f"full Alice: expected 58 eventuality content edges, got {len(edges)}")
    target_types = [objects[target]["type"] for _, target in edges]
    if target_types.count("formula") != 57 or target_types.count("sequence") != 1:
        raise ValueError(f"full Alice: unexpected content target types {target_types!r}")
    for format_name in ("tree", "tree+proj"):
        rendered = render_format(current_binary, FULL_ALICE, fixture, format_name)
        missing = [
            (eventuality, target)
            for eventuality, target in edges
            if f"[{target}]" not in rendered
        ]
        if missing:
            raise ValueError(f"full Alice {format_name}: missing content edges {missing!r}")
        print(f"full Alice {format_name}: content targets 58/58")


def verify_expected_surfaces(fixtures: dict[Path, dict[str, Any]]) -> None:
    nitcu = tersmu(
        fixtures[
            Path("tests/fixtures/adhoc/output/tersmu-derived-eventuality-content.toml")
        ]
    )
    for format_name in ("tree", "tree+proj"):
        output = nitcu[format_name]
        if "abstraction content: atom" not in output or "lo klama[eventuality:" not in output:
            raise ValueError(f"nitcu {format_name}: content branch or enriched label missing")
        if "content=formula:" in output:
            raise ValueError(f"nitcu {format_name}: content leaked back into details")
    projected = nitcu["tree+proj"].split("\n\nprojected:\n", maxsplit=1)[1]
    if "[predication:12]" in projected:
        raise ValueError("nitcu: intensional klama leaked into projected commitments")

    headline = tersmu(
        fixtures[
            Path(
                "tests/fixtures/adhoc/output/tersmu-derived-eventuality-content-headline.toml"
            )
        ]
    )
    for format_name in ("tree", "tree+proj"):
        output = headline[format_name]
        for relation in ("troci(", "tarti(", "xendo(", "cnikansa("):
            if relation not in output:
                raise ValueError(f"headline {format_name}: missing {relation}")

    nested = tersmu(
        fixtures[
            Path(
                "tests/fixtures/adhoc/output/tersmu-derived-nested-eventuality-content.toml"
            )
        ]
    )
    for format_name in ("tree", "tree+proj"):
        if nested[format_name].count("abstraction content:") != 2:
            raise ValueError(f"nested nu {format_name}: expected exactly two content branches")

    proposition = tersmu(
        fixtures[
            Path(
                "tests/fixtures/adhoc/output/tersmu-derived-proposition-abstraction.toml"
            )
        ]
    )
    for format_name in ("tree", "tree+proj"):
        if "abstraction body: atom" not in proposition[format_name]:
            raise ValueError(f"du'u {format_name}: abstraction body missing")


def verify_canonical_identity(base_binary: Path, current_binary: Path) -> None:
    for name, source in CANONICAL_INPUTS:
        base = run(str(base_binary), "tersmu", "--color=never", "--format", "json", source)
        current = run(
            str(current_binary), "tersmu", "--color=never", "--format", "json", source
        )
        if base.returncode != 0 or current.returncode != 0:
            raise ValueError(f"canonical JSON probe {name} failed")
        if base.stdout != current.stdout or base.stderr != current.stderr:
            raise ValueError(f"canonical JSON probe {name} is not byte-identical")
        digest = hashlib.sha256(current.stdout).hexdigest()
        print(f"canonical JSON {name}: identical bytes={len(current.stdout)} sha256={digest}")


def changed_fixture_paths(base_ref: str) -> set[Path]:
    result = run("git", "diff", "--name-only", base_ref, "--", "tests/fixtures")
    if result.returncode != 0:
        raise ValueError(result.stderr.decode().strip())
    return {Path(line) for line in result.stdout.decode().splitlines() if line}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path, required=True)
    args = parser.parse_args()

    for path in MIGRATED_FIXTURES:
        old = document(git_source(args.base_ref, path))
        current = document(path.read_text())
        verify_two_sided_document(path, old, current)
        verify_derived_expectations(args.base_binary, path, old)
        verify_derived_expectations(args.current_binary, path, current)

    fixtures = {path: document(path.read_text()) for path in NEW_FIXTURES}
    for path, fixture in fixtures.items():
        if normalized_expectation(fixture) != render_derived(
            args.current_binary, path, fixture
        ):
            raise ValueError(f"{path}: current expectation is stale")
    verify_expected_surfaces(fixtures)
    verify_full_alice_content_edges(args.current_binary)

    if git_source(args.base_ref, KA_GUARD) != KA_GUARD.read_text():
        raise ValueError(f"{KA_GUARD}: negative guard fixture changed")
    ka_fixture = document(KA_GUARD.read_text())
    base_ka = render_derived(args.base_binary, KA_GUARD, ka_fixture)
    current_ka = render_derived(args.current_binary, KA_GUARD, ka_fixture)
    if base_ka != current_ka or current_ka != normalized_expectation(ka_fixture):
        raise ValueError("ka relation-body guard is not byte-identical")

    verify_canonical_identity(args.base_binary, args.current_binary)

    expected_paths = {*MIGRATED_FIXTURES, *NEW_FIXTURES}
    changed_paths = changed_fixture_paths(args.base_ref)
    if changed_paths != expected_paths:
        extra = sorted(changed_paths - expected_paths)
        missing = sorted(expected_paths - changed_paths)
        raise ValueError(f"unexpected fixture drift: extra={extra}, missing={missing}")

    print("issue-394 fixture migration: verified")
    print("existing fixture documents reproduced: base 2/2; current 2/2")
    print("base-to-PR document reconstruction: 2/2")
    print("PR-to-base document reconstruction: 2/2")
    print("new derived fixtures reproduced: 4/4")
    print("ka relation-body guard: byte-identical")
    print("canonical JSON identity: 6/6")
    print("unexpected fixture drift: 0")


if __name__ == "__main__":
    main()
