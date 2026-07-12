#!/usr/bin/env python3
"""Migrate and verify the mechanical domainImport fixture change for issue #279."""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
import subprocess
import sys
import tomllib
from typing import Any


PROJECTIVE = "projective"
UNIVERSAL_OPERATORS = {"forall", "pluralForall"}


class VerificationError(Exception):
    """A fixture differs from its base for a non-mechanical reason."""


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return result.stdout


def base_fixture_texts(base_ref: str, paths: list[str]) -> dict[str, str]:
    process = subprocess.Popen(
        ["git", "cat-file", "--batch"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )
    if process.stdin is None or process.stdout is None:
        raise VerificationError("could not open git cat-file pipes")
    texts: dict[str, str] = {}
    for path in paths:
        process.stdin.write(f"{base_ref}:{path}\n".encode())
        process.stdin.flush()
        header = process.stdout.readline().decode().rstrip("\n")
        parts = header.split()
        if len(parts) != 3 or parts[1] != "blob":
            raise VerificationError(f"{path}: unexpected git cat-file header {header!r}")
        size = int(parts[2])
        content = process.stdout.read(size)
        if process.stdout.read(1) != b"\n":
            raise VerificationError(f"{path}: malformed git cat-file boundary")
        texts[path] = content.decode("utf-8")
    process.stdin.close()
    if process.wait() != 0:
        raise VerificationError("git cat-file failed")
    return texts


def tersmu_json(document: dict[str, Any]) -> str | None:
    value = (
        document.get("expectations", {})
        .get("output", {})
        .get("tersmu", {})
        .get("json")
    )
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, dict) and isinstance(value.get("text"), str):
        return value["text"]
    raise VerificationError("tersmu JSON expectation has an unsupported TOML shape")


def replace_tersmu_json(document: dict[str, Any], replacement: str) -> None:
    tersmu = document["expectations"]["output"]["tersmu"]
    value = tersmu["json"]
    if isinstance(value, str):
        tersmu["json"] = replacement
        return
    if isinstance(value, dict) and isinstance(value.get("text"), str):
        value["text"] = replacement
        return
    raise VerificationError("tersmu JSON expectation has an unsupported TOML shape")


def qualifies_for_domain_import(node: dict[str, Any]) -> bool:
    return (
        node.get("type") == "formula"
        and node.get("operator") in UNIVERSAL_OPERATORS
        and "restriction" in node
    )


def expected_graph_from_old(old_graph: dict[str, Any]) -> tuple[dict[str, Any], int]:
    expected = copy.deepcopy(old_graph)
    objects = expected.get("objects")
    if not isinstance(objects, dict):
        raise VerificationError("tersmu graph has no object map")

    marked = 0
    for object_id, node in list(objects.items()):
        if not isinstance(node, dict):
            raise VerificationError(f"object {object_id} is not a map")
        if "domainImport" in node:
            raise VerificationError(f"old object {object_id} already has domainImport")
        if not qualifies_for_domain_import(node):
            continue

        rebuilt: dict[str, Any] = {}
        inserted = False
        for key, value in node.items():
            rebuilt[key] = value
            if key == "restriction":
                rebuilt["domainImport"] = PROJECTIVE
                inserted = True
        if not inserted:
            raise VerificationError(f"qualifying object {object_id} has no restriction field")
        objects[object_id] = rebuilt
        marked += 1
    return expected, marked


def assert_new_graph_iff(path: str, graph: dict[str, Any]) -> None:
    objects = graph.get("objects")
    if not isinstance(objects, dict):
        raise VerificationError(f"{path}: tersmu graph has no object map")
    for object_id, node in objects.items():
        if not isinstance(node, dict):
            raise VerificationError(f"{path}: object {object_id} is not a map")
        expected = PROJECTIVE if qualifies_for_domain_import(node) else None
        actual = node.get("domainImport")
        if actual != expected:
            raise VerificationError(
                f"{path}: object {object_id} has domainImport={actual!r}, expected {expected!r}"
            )


def compact_json(value: dict[str, Any]) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def verify_fixture(path: str, old_text: str, write: bool) -> tuple[bool, int]:
    current_path = Path(path)
    if not current_path.is_file():
        raise VerificationError(f"{path}: fixture is missing from the working tree")
    current_text = current_path.read_text(encoding="utf-8")
    old_document = tomllib.loads(old_text)
    current_document = tomllib.loads(current_text)
    old_json_text = tersmu_json(old_document)

    if old_json_text is None:
        if current_document != old_document:
            raise VerificationError(f"{path}: changed without a tersmu JSON expectation")
        return False, 0

    old_graph = json.loads(old_json_text)
    expected_graph, marked = expected_graph_from_old(old_graph)
    expected_json_text = compact_json(expected_graph)
    touched = marked > 0

    if write and touched:
        current_json_text = tersmu_json(current_document)
        if current_json_text == old_json_text:
            if current_text.count(old_json_text) != 1:
                raise VerificationError(
                    f"{path}: old tersmu JSON does not occur exactly once in the fixture"
                )
            current_text = current_text.replace(old_json_text, expected_json_text, 1)
            current_path.write_text(current_text, encoding="utf-8")
            current_document = tomllib.loads(current_text)
        elif current_json_text != expected_json_text:
            raise VerificationError(f"{path}: cannot migrate an independently changed JSON value")

    current_json_text = tersmu_json(current_document)
    if current_json_text is None:
        raise VerificationError(f"{path}: current tersmu JSON expectation is missing")
    current_graph = json.loads(current_json_text)
    if current_graph != expected_graph:
        raise VerificationError(f"{path}: JSON differs beyond the derived domainImport markers")
    if current_json_text != expected_json_text:
        raise VerificationError(f"{path}: JSON field order or formatting is not mechanical")
    assert_new_graph_iff(path, current_graph)

    normalized_current = copy.deepcopy(current_document)
    replace_tersmu_json(normalized_current, old_json_text)
    if normalized_current != old_document:
        raise VerificationError(f"{path}: TOML differs outside the tersmu JSON expectation")
    return touched, marked


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--base-ref",
        default=git("merge-base", "HEAD", "origin/main").strip(),
        help="base tree to compare (default: this branch's merge-base with origin/main)",
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="apply the verified mechanical marker insertion before checking",
    )
    args = parser.parse_args()

    paths = [
        path
        for path in git(
            "ls-tree", "-r", "--name-only", args.base_ref, "--", "tests/fixtures"
        ).splitlines()
        if path.endswith(".toml")
    ]
    try:
        old_texts = base_fixture_texts(args.base_ref, paths)
    except VerificationError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    touched = 0
    marked = 0
    failures: list[str] = []
    for path in paths:
        try:
            fixture_touched, fixture_marked = verify_fixture(
                path, old_texts[path], args.write
            )
            touched += int(fixture_touched)
            marked += fixture_marked
        except (VerificationError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
            failures.append(str(error))

    for failure in failures:
        print(f"ERROR: {failure}", file=sys.stderr)
    print(
        f"fixtures_checked={len(paths)} fixtures_touched={touched} "
        f"nodes_marked={marked} differences_beyond_marker={len(failures)}"
    )
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
