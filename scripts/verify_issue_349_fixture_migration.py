#!/usr/bin/env python3
"""Rewrite and verify the narrowly predicted issue #349 fixture migration."""

from __future__ import annotations

import argparse
import copy
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


FIXTURE_ROOT = Path("tests/fixtures/adhoc/output")
NEW_FIXTURE = FIXTURE_ROOT / "tersmu-derived-sequence-binding.toml"
COMBINED_FIXTURES = {
    FIXTURE_ROOT / "tersmu-derived-opacity.toml": "mi nitcu lo tanxe",
    FIXTURE_ROOT / "tersmu-derived-negated-domain-import.toml": (
        "naku ro da poi mlatu cu klama"
    ),
    FIXTURE_ROOT / "tersmu-derived-attitudinal.toml": ".ui mi klama",
    FIXTURE_ROOT / "tersmu-derived-incidental.toml": "le mlatu noi blanu cu klama",
    FIXTURE_ROOT / "tersmu-derived-event-past.toml": "mi pu klama",
    FIXTURE_ROOT / "tersmu-derived-connective.toml": "mi klama gi'e cadzu",
    NEW_FIXTURE: "do nelci mi .ibabo mi nelci do",
}
FORMULA_ROLES = {
    "root",
    "content",
    "sequence-content",
    "sequence-item",
    "connection-claim",
    "aside",
    "child",
    "restriction",
    "body",
    "descriptor-body",
    "relation-body",
    "abstraction-body",
    "restrictive-relative-clause",
    "incidental-relative-clause",
    "non-claim-restrictive-relative-clause",
    "modal-body",
    "incidental",
}
QUANTIFIERS = {
    "exists",
    "forall",
    "none",
    "cardinality",
    "plural-exists",
    "plural-forall",
}
SCOPE_OPERATORS = {
    "affirmed",
    "not",
    "scoped",
    "and",
    "or",
    "implies",
    "iff",
    "exclusive-or",
    "whether-or-not",
    "connective-question",
}
SEMANTIC_ID = re.compile(
    r"\[((?:eventuality(?:/locution)?|entity|relation|parameter|formula|"
    r"predication|utterance|sequence|display):[^\]]+)\]"
)
EVENT_ID = re.compile(r"\[((?:eventuality(?:/locution)?):[^\]]+)\]")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout


def render(binary: Path, format_name: str, lojban: str) -> str:
    return subprocess.run(
        [str(binary), "tersmu", "--format", format_name, lojban],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout.removesuffix("\n")


def nested(document: dict[str, Any], *keys: str) -> Any | None:
    value: Any = document
    for key in keys:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def scope_from_context(context: str) -> str:
    operators: list[str] = []
    for segment in context.split(" > "):
        role, separator, formula = segment.partition(" ")
        if not separator or role not in FORMULA_ROLES:
            continue
        operator = formula.split(" ", 1)[0]
        if operator == "atom":
            continue
        if operator in SCOPE_OPERATORS:
            operators.append(operator)
            continue
        if operator in QUANTIFIERS:
            prefix = f"{operator} variable="
            if not formula.startswith(prefix):
                raise ValueError(f"malformed quantified context segment {segment!r}")
            binder_and_suffix = formula.removeprefix(prefix)
            boundaries = [
                boundary
                for marker in (" domain-import=", " binds=exists ", " [formula:")
                if (boundary := binder_and_suffix.find(marker)) >= 0
            ]
            if not boundaries:
                raise ValueError(f"quantified context lacks a formula suffix: {segment!r}")
            binder = binder_and_suffix[: min(boundaries)]
            if not binder:
                raise ValueError(f"quantified context lacks a binder: {segment!r}")
            operators.append(f"{operator} {binder}")
            continue
        raise ValueError(f"unrecognized scope-bearing context segment {segment!r}")
    return " > ".join(operators) if operators else "top-level"


def upgrade_claims(old_claims: str) -> str:
    lines: list[str] = []
    in_at_issue = True
    for line in old_claims.splitlines():
        if line == "asserted:":
            lines.append("at-issue commitments:")
            continue
        if line == "presupposed/projected:":
            in_at_issue = False
            lines.append(line)
            continue
        if not in_at_issue or not line.startswith("- ") or line == "- (none)":
            lines.append(line)
            continue
        prefix, marker, suffix = line.rpartition(" [mode=")
        if not marker or not suffix.endswith("]") or "; context=" not in suffix:
            raise ValueError(f"unrecognized at-issue claims line {line!r}")
        mode, context = suffix[:-1].split("; context=", 1)
        scope = scope_from_context(context)
        lines.append(f"{prefix} [mode={mode}; scope={scope}; context={context}]")
    return "\n".join(lines)


def downgrade_claims(new_claims: str) -> str:
    lines: list[str] = []
    in_at_issue = True
    for line in new_claims.splitlines():
        if line == "at-issue commitments:":
            lines.append("asserted:")
            continue
        if line == "presupposed/projected:":
            in_at_issue = False
            lines.append(line)
            continue
        if not in_at_issue or not line.startswith("- ") or line == "- (none)":
            if not in_at_issue and "scope=" in line:
                raise ValueError("scope= leaked outside the at-issue claims tier")
            lines.append(line)
            continue
        prefix, marker, suffix = line.rpartition(" [mode=")
        if not marker or not suffix.endswith("]"):
            raise ValueError(f"unrecognized at-issue claims line {line!r}")
        mode, remainder = suffix[:-1].split("; scope=", 1)
        scope, context = remainder.split("; context=", 1)
        expected_scope = scope_from_context(context)
        if scope != expected_scope:
            raise ValueError(
                f"scope projection mismatch: rendered={scope!r}, expected={expected_scope!r}"
            )
        lines.append(f"{prefix} [mode={mode}; context={context}]")
    return "\n".join(lines)


def replace_text_field(source: str, field: str, value: str) -> str:
    pattern = re.compile(rf'(?ms)^{re.escape(field)} = """\n.*?"""$')
    replacement = f'{field} = """\n{value}"""'
    replaced, count = pattern.subn(lambda _: replacement, source)
    if count != 1:
        raise ValueError(f"expected one {field} multiline field, found {count}")
    return replaced


def insert_or_replace_combined(source: str, value: str) -> str:
    if re.search(r'(?m)^combined = """$', source):
        return replace_text_field(source, "combined", value)
    tree = re.compile(r'(?ms)^tree = """\n.*?"""$')
    match = tree.search(source)
    if match is None:
        raise ValueError("fixture lacks a tree field after which combined can be inserted")
    addition = f'\ncombined = """\n{value}"""'
    return source[: match.end()] + addition + source[match.end() :]


def rewrite_fixtures(binary: Path) -> None:
    for path in sorted(FIXTURE_ROOT.glob("tersmu-derived-*.toml")):
        if path == NEW_FIXTURE:
            continue
        source = path.read_text(encoding="utf-8")
        document = tomllib.loads(source)
        lojban = document["lojban"]
        source = replace_text_field(source, "claims", render(binary, "claims", lojban))
        if path in COMBINED_FIXTURES:
            source = insert_or_replace_combined(
                source, render(binary, "combined", COMBINED_FIXTURES[path])
            )
        path.write_text(source, encoding="utf-8")

    lojban = COMBINED_FIXTURES[NEW_FIXTURE]
    claims = render(binary, "claims", lojban)
    tree = render(binary, "tree", lojban)
    combined = render(binary, "combined", lojban)
    NEW_FIXTURE.write_text(
        f'''id = "adhoc.output.tersmu-derived-sequence-binding"
lojban = "{lojban}"
tags = ["output", "tersmu", "claims", "tree", "combined", "issue-349", "eventuality", "sequence"]

[[provenance]]
kind = "adhoc"
description = "Issue #349 sequence probe: shared event binding stays structural while both locutions share one projected frame."

[expectations.output.tersmu]
claims = """
{claims}"""
tree = """
{tree}"""
combined = """
{combined}"""
''',
        encoding="utf-8",
    )


def denotation_ids(claims: str) -> list[str]:
    ids: list[str] = []
    for line in claims.splitlines():
        if not line.startswith("- denotes "):
            continue
        match = SEMANTIC_ID.search(line)
        if match is None:
            raise ValueError(f"denotation line lacks an object id: {line!r}")
        ids.append(match.group(1))
    return ids


def verify_combined(combined: str, claims: str, tree: str) -> None:
    tree_spine, separator, projected = combined.partition("\n\nprojected:\n")
    if not separator or not tree_spine or not projected:
        raise ValueError("combined output lacks its tree/projected partition")
    if "context=" in combined or "scope=" in combined:
        raise ValueError("combined output retained a claims breadcrumb")
    if "at-issue commitments:" in combined or "presupposed/projected:" in combined:
        raise ValueError("combined output retained a claims tier")
    if combined in {f"{tree}\n\n{claims}", f"{claims}\n\n{tree}"}:
        raise ValueError("combined output is a concatenation of existing formats")
    if tree_spine == tree:
        raise ValueError("combined tree did not deduplicate event-use condition suffixes")
    if projected.count("- frame: ") != 1:
        raise ValueError("combined output lacks exactly one grouped frame line")
    if "relation body:" in projected or "abstraction body:" in projected:
        raise ValueError("non-claim intensional body leaked into projected commitments")

    introduction_lines = "\n".join(
        line
        for line in projected.splitlines()
        if line.startswith("- frame: ") or line.startswith("- denotes ")
    )
    for object_id in denotation_ids(claims):
        count = introduction_lines.count(f"[{object_id}]")
        if count != 1:
            raise ValueError(
                f"denotation {object_id} appears {count} times in projected introductions"
            )

    event_ids = set(EVENT_ID.findall(combined))
    if not event_ids:
        raise ValueError("combined probe lacks eventualities")
    for event_id in event_ids:
        condition_sites = combined.count(f"[{event_id}]; time=") + combined.count(
            f"[{event_id}] {{time="
        )
        if condition_sites != 1:
            raise ValueError(
                f"event {event_id} has {condition_sites} combined condition sites"
            )

    displayed = {
        match.group(1)
        for line in tree_spine.splitlines()
        if (match := re.search(r"display=(display:[^;\]]+)", line)) is not None
    }
    for display_id in displayed:
        if display_id in projected:
            raise ValueError(f"displayed content {display_id} leaked into projected output")


def verify_existing(path: Path, base: str, binary: Path, totals: dict[str, int]) -> None:
    old_document = tomllib.loads(git("show", f"{base}:{path.as_posix()}"))
    new_document = tomllib.loads(path.read_text(encoding="utf-8"))
    old_claims = nested(old_document, "expectations", "output", "tersmu", "claims")
    new_claims = nested(new_document, "expectations", "output", "tersmu", "claims")
    if not isinstance(old_claims, str) or not isinstance(new_claims, str):
        raise ValueError("claims expectation was added, removed, or malformed")
    if upgrade_claims(old_claims) != new_claims:
        raise ValueError("base-to-PR claims reconstruction did not match")
    if downgrade_claims(new_claims) != old_claims:
        raise ValueError("PR-to-base claims reconstruction did not match")
    lojban = new_document.get("lojban")
    if not isinstance(lojban, str) or render(binary, "claims", lojban) != new_claims:
        raise ValueError("claims expectation does not match the current binary")

    tersmu = new_document["expectations"]["output"]["tersmu"]
    combined = tersmu.get("combined")
    if path in COMBINED_FIXTURES:
        if not isinstance(combined, str):
            raise ValueError("combined probe lacks an exact expectation")
        if lojban != COMBINED_FIXTURES[path]:
            raise ValueError("combined probe input changed")
        actual = render(binary, "combined", lojban)
        if actual != combined:
            raise ValueError("combined expectation does not match the current binary")
        tree = tersmu.get("tree")
        if not isinstance(tree, str):
            raise ValueError("combined probe lacks its established tree expectation")
        verify_combined(combined, new_claims, tree)
        totals["combined"] += 1
    elif combined is not None:
        raise ValueError("combined expectation was added outside the probe set")

    normalized = copy.deepcopy(new_document)
    normalized_tersmu = normalized["expectations"]["output"]["tersmu"]
    normalized_tersmu["claims"] = old_claims
    normalized_tersmu.pop("combined", None)
    if normalized != old_document:
        raise ValueError("tree, canonical JSON, metadata, or another field drifted")
    totals["existing"] += 1


def verify_new(binary: Path, totals: dict[str, int]) -> None:
    document = tomllib.loads(NEW_FIXTURE.read_text(encoding="utf-8"))
    if document.get("id") != "adhoc.output.tersmu-derived-sequence-binding":
        raise ValueError("new sequence fixture has the wrong id")
    if document.get("lojban") != COMBINED_FIXTURES[NEW_FIXTURE]:
        raise ValueError("new sequence fixture has the wrong input")
    if "issue-349" not in document.get("tags", []):
        raise ValueError("new sequence fixture lacks the issue-349 tag")
    tersmu = document["expectations"]["output"]["tersmu"]
    for format_name in ("claims", "tree", "combined"):
        expected = tersmu.get(format_name)
        if not isinstance(expected, str):
            raise ValueError(f"new sequence fixture lacks exact {format_name}")
        if render(binary, format_name, document["lojban"]) != expected:
            raise ValueError(f"new sequence {format_name} does not match the binary")
    verify_combined(tersmu["combined"], tersmu["claims"], tersmu["tree"])
    totals["combined"] += 1
    totals["new"] += 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="main")
    parser.add_argument("--binary", type=Path, default=Path("target/debug/jbotci"))
    parser.add_argument("--rewrite", action="store_true")
    args = parser.parse_args()

    if args.rewrite:
        rewrite_fixtures(args.binary)

    existing = {
        path
        for path in FIXTURE_ROOT.glob("tersmu-derived-*.toml")
        if path != NEW_FIXTURE
    }
    changed = {
        Path(path)
        for path in git("diff", "--name-only", args.base, "--", str(FIXTURE_ROOT)).splitlines()
        if path.endswith(".toml")
    }
    untracked = {
        Path(path)
        for path in git(
            "ls-files", "--others", "--exclude-standard", "--", str(FIXTURE_ROOT)
        ).splitlines()
        if path.endswith(".toml")
    }
    expected_changed = existing | {NEW_FIXTURE}
    errors: list[str] = []
    if changed | untracked != expected_changed:
        errors.append(
            "fixture set mismatch: "
            f"expected={sorted(map(str, expected_changed))}, "
            f"actual={sorted(map(str, changed | untracked))}"
        )

    totals = {"existing": 0, "new": 0, "combined": 0}
    for path in sorted(existing):
        try:
            verify_existing(path, args.base, args.binary, totals)
        except (KeyError, ValueError, subprocess.CalledProcessError, tomllib.TOMLDecodeError) as error:
            errors.append(f"{path}: {error}")
    try:
        verify_new(args.binary, totals)
    except (
        FileNotFoundError,
        KeyError,
        ValueError,
        subprocess.CalledProcessError,
        tomllib.TOMLDecodeError,
    ) as error:
        errors.append(f"{NEW_FIXTURE}: {error}")

    if errors:
        print("issue-349 fixture migration: FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(
            f"unexpected changes requiring manual review: {len(errors)}", file=sys.stderr
        )
        return 1

    print("issue-349 fixture migration: verified")
    print(f"existing claims fixtures reconstructed both ways: {totals['existing']}")
    print(f"new exact fixtures: {totals['new']}")
    print(f"combined exact probes: {totals['combined']}")
    print("canonical JSON fixtures changed: 0")
    print("tree fixtures changed: 0")
    print("unexpected changes requiring manual review: 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
