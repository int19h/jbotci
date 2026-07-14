#!/usr/bin/env python3
"""Rewrite and verify the issue #368 tersmu fixture migration.

The gate is deliberately narrow because issue #358 makes broad fixture
regeneration untrustworthy. It names every affected fixture, renders only the
old `combined` surface for those inputs, and reconstructs the migration in both
directions before accepting the checked-in files.
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import subprocess
import tomllib
from pathlib import Path
from typing import Any


FIXTURES = (
    "tests/fixtures/adhoc/output/tersmu-derived-attitudinal.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-connective-event.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-connective.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-domain-import.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-event-actuality.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-event-aspect.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-event-interval.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-event-past.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-event-space.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-event-tenseless.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-incidental.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-negated-domain-import.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-nonveridical-relative.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-opacity.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-property-abstraction.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-restrictive-relative.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-sequence-binding.toml",
    "tests/fixtures/adhoc/output/tersmu-derived-speaker-description.toml",
)
PROFILE = "tests/fixtures/profiles/all.toml"
ALLOWED_FIXTURE_DIFFS = frozenset((*FIXTURES, PROFILE))
DESCRIPTION_REPLACEMENTS = {
    "tests/fixtures/adhoc/output/tersmu-derived-incidental.toml": (
        "Issue #278 projective tier: le characterizing skicu, its non-claim property body, and noi incidental claims are explicit.",
        "Issue #278 projection behavior: the le characterizing skicu body stays structural while noi incidental content is displaced.",
    ),
    "tests/fixtures/adhoc/output/tersmu-derived-opacity.toml": (
        "Issue #278 opacity trap: the lo descriptor commitment must be top-level in the claims ledger.",
        "Issue #278 opacity trap: the lo descriptor commitment must appear in the top-level projected section.",
    ),
}

MULTILINE_FIELD = r'(?ms)^{field} = """\n.*?"""(?:\n|$)'
TAGS_LINE = re.compile(r"(?m)^tags = .*?$")


def run(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, check=check, text=True, capture_output=True)


def git_source(base_ref: str, path: str) -> str:
    return run("git", "show", f"{base_ref}:{path}").stdout


def document(source: str) -> dict[str, Any]:
    return tomllib.loads(source)


def tersmu(document: dict[str, Any]) -> dict[str, Any]:
    value = document["expectations"]["output"]["tersmu"]
    if not isinstance(value, dict):
        raise ValueError("tersmu expectation is not a table")
    return value


def render(binary: Path, format_name: str, fixture: dict[str, Any]) -> str:
    command = [str(binary), "tersmu", "--format", format_name]
    dialect = fixture.get("dialect")
    if dialect is not None:
        command.extend(("--dialect", dialect))
    if tersmu(fixture).get("story-time", False):
        command.append("--story-time")
    command.append(fixture["lojban"])
    result = run(*command)
    if result.stderr:
        raise ValueError(f"unexpected tersmu diagnostics: {result.stderr}")
    return result.stdout.removesuffix("\n")


def migrated_tags(old_tags: list[str]) -> list[str]:
    had_removed_format = any(tag in {"claims", "combined"} for tag in old_tags)
    migrated: list[str] = []
    for tag in old_tags:
        if tag in {"claims", "combined"}:
            continue
        migrated.append(tag)
        if tag == "tree" and had_removed_format:
            migrated.append("tree+proj")
    return migrated


def replace_tags(source: str, tags: list[str]) -> str:
    replacement = f"tags = {json.dumps(tags, ensure_ascii=False)}"
    updated, count = TAGS_LINE.subn(replacement, source, count=1)
    if count != 1:
        raise ValueError("fixture does not have exactly one tags line")
    return updated


def remove_multiline_field(source: str, field: str) -> str:
    pattern = re.compile(MULTILINE_FIELD.format(field=re.escape(field)))
    updated, count = pattern.subn("", source, count=1)
    if count != 1:
        raise ValueError(f"fixture does not have exactly one {field!r} field")
    return updated


def insert_tree_proj(source: str, value: str) -> str:
    tree = re.compile(MULTILINE_FIELD.format(field="tree"))
    match = tree.search(source)
    if match is None:
        raise ValueError("fixture has no tree field for tree+proj insertion")
    field = f'"tree+proj" = """\n{value}"""\n'
    return source[: match.end()] + field + source[match.end() :]


def forward_source(path: str, old_source: str, value: str) -> str:
    old = document(old_source)
    updated = remove_multiline_field(old_source, "claims")
    old_combined = tersmu(old).get("combined")
    if old_combined is None:
        updated = insert_tree_proj(updated, value)
    else:
        updated, count = re.subn(
            r'(?m)^combined = """$', '"tree+proj" = """', updated, count=1
        )
        if count != 1:
            raise ValueError("combined field could not be renamed")
    updated = replace_tags(updated, migrated_tags(old["tags"]))
    if path in DESCRIPTION_REPLACEMENTS:
        old_description, new_description = DESCRIPTION_REPLACEMENTS[path]
        if updated.count(old_description) != 1:
            raise ValueError(f"{path}: old description is not unique")
        updated = updated.replace(old_description, new_description)
    return updated


def expected_profile_source(old_source: str) -> str:
    old = document(old_source)
    migrated = [
        facet
        for facet in old["facets"]
        if facet not in {"tersmu-claims", "tersmu-combined"}
    ]
    tree_index = migrated.index("tersmu-tree")
    migrated.insert(tree_index + 1, "tersmu-tree+proj")
    replacement = f"facets = {json.dumps(migrated)}"
    updated, count = re.subn(r"(?m)^facets = .*?$", replacement, old_source, count=1)
    if count != 1:
        raise ValueError("all profile lacks one facets line")
    return updated


def verify_documents(
    path: str,
    old: dict[str, Any],
    current: dict[str, Any],
    expected_tree_proj: str,
) -> None:
    expected = copy.deepcopy(old)
    expected["tags"] = migrated_tags(old["tags"])
    if path in DESCRIPTION_REPLACEMENTS:
        _, new_description = DESCRIPTION_REPLACEMENTS[path]
        expected["provenance"][0]["description"] = new_description
    expected_tersmu = tersmu(expected)
    expected_tersmu.pop("claims")
    expected_tersmu.pop("combined", None)
    expected_tersmu["tree+proj"] = expected_tree_proj
    if current != expected:
        raise ValueError("base-to-PR document reconstruction did not match")

    reconstructed = copy.deepcopy(current)
    reconstructed["tags"] = old["tags"]
    if path in DESCRIPTION_REPLACEMENTS:
        old_description, _ = DESCRIPTION_REPLACEMENTS[path]
        reconstructed["provenance"][0]["description"] = old_description
    reconstructed_tersmu = tersmu(reconstructed)
    reconstructed_tersmu.pop("tree+proj")
    reconstructed_tersmu["claims"] = tersmu(old)["claims"]
    if "combined" in tersmu(old):
        reconstructed_tersmu["combined"] = tersmu(old)["combined"]
    if reconstructed != old:
        raise ValueError("PR-to-base document reconstruction did not match")


def changed_fixture_paths(base_ref: str) -> set[str]:
    output = run(
        "git", "diff", "--name-only", base_ref, "--", "tests/fixtures"
    ).stdout
    return {line for line in output.splitlines() if line}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-ref", default="origin/main")
    parser.add_argument("--base-binary", type=Path, required=True)
    parser.add_argument("--current-binary", type=Path)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    existing_combined = 0
    generated_tree_proj = 0
    for path_text in FIXTURES:
        path = Path(path_text)
        old_source = git_source(args.base_ref, path_text)
        old = document(old_source)
        old_tersmu = tersmu(old)
        if not isinstance(old_tersmu.get("claims"), str):
            raise ValueError(f"{path_text}: base fixture lacks claims")
        base_output = render(args.base_binary, "combined", old)
        if "combined" in old_tersmu:
            if old_tersmu["combined"] != base_output:
                raise ValueError(f"{path_text}: base combined expectation is stale")
            existing_combined += 1
        else:
            generated_tree_proj += 1

        expected_source = forward_source(path_text, old_source, base_output)
        if args.write:
            path.write_text(expected_source)
        current_source = path.read_text()
        if current_source != expected_source:
            raise ValueError(f"{path_text}: source changed beyond the narrow migration")
        current = document(current_source)
        verify_documents(path_text, old, current, base_output)
        current_output = tersmu(current).get("tree+proj")
        if current_output != base_output:
            raise ValueError(f"{path_text}: tree+proj is not byte-identical to base combined")
        if args.current_binary is not None:
            rendered = render(args.current_binary, "tree+proj", current)
            if rendered != current_output:
                raise ValueError(f"{path_text}: current tree+proj expectation is stale")

    old_profile = git_source(args.base_ref, PROFILE)
    expected_profile = expected_profile_source(old_profile)
    profile_path = Path(PROFILE)
    if args.write:
        profile_path.write_text(expected_profile)
    if profile_path.read_text() != expected_profile:
        raise ValueError("all profile changed beyond the facet removal/rename")

    changed = changed_fixture_paths(args.base_ref)
    if changed != ALLOWED_FIXTURE_DIFFS:
        extra = sorted(changed - ALLOWED_FIXTURE_DIFFS)
        missing = sorted(ALLOWED_FIXTURE_DIFFS - changed)
        raise ValueError(f"unexpected fixture drift: extra={extra}, missing={missing}")

    print("issue-368 fixture migration: verified")
    print(f"claims fixtures migrated to tree+proj: {len(FIXTURES)}")
    print(f"existing combined expectations renamed byte-identically: {existing_combined}")
    print(f"new targeted tree+proj expectations from base combined: {generated_tree_proj}")
    print("base-to-PR document reconstruction: 18/18")
    print("PR-to-base document reconstruction: 18/18")
    print("unexpected fixture drift: 0")


if __name__ == "__main__":
    main()
