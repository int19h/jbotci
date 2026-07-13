#!/usr/bin/env python3
"""Verify that issue #352 fixture edits are only the predicted wire changes."""

from __future__ import annotations

import argparse
import copy
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


FIXTURE_ROOT = Path("tests/fixtures")
FIXED_LINE = re.compile(
    r"^- denotes (.+) \[binder-dependence=fixed; (?:constant|indexical);(.*)\]$"
)
UNDERSPECIFIED_LINE = re.compile(
    r"^- denotes (.+) \[binder-dependence=underspecified; may-depend-on=.+; constant;(.*)\]$"
)


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout


def nested(document: dict[str, Any], *keys: str) -> Any | None:
    value: Any = document
    for key in keys:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def replace_nested(document: dict[str, Any], value: Any, *keys: str) -> None:
    target: Any = document
    for key in keys[:-1]:
        target = target[key]
    target[keys[-1]] = value


def validate_and_strip_scope_dependence(value: Any, counts: dict[str, int]) -> Any:
    if isinstance(value, list):
        return [validate_and_strip_scope_dependence(item, counts) for item in value]
    if not isinstance(value, dict):
        return value

    normalized = {
        key: validate_and_strip_scope_dependence(item, counts)
        for key, item in value.items()
        if key != "scopeDependence"
    }
    dependence = value.get("scopeDependence")
    is_constant = value.get("type") == "referent" and value.get("category") == "constant"
    if is_constant:
        if not isinstance(dependence, dict):
            raise ValueError("constant referent lacks scopeDependence")
        kind = dependence.get("kind")
        if kind == "fixed":
            if dependence != {"kind": "fixed"}:
                raise ValueError(f"malformed fixed scopeDependence: {dependence!r}")
            counts["fixed"] += 1
        elif kind == "underspecified":
            binders = dependence.get("mayDependOn")
            if set(dependence) != {"kind", "mayDependOn"}:
                raise ValueError(f"unexpected underspecified fields: {dependence!r}")
            if not isinstance(binders, list) or not binders:
                raise ValueError("underspecified mayDependOn must be a nonempty array")
            if not all(isinstance(binder, str) for binder in binders):
                raise ValueError("mayDependOn values must be object ids")
            if len(set(binders)) != len(binders):
                raise ValueError(f"mayDependOn contains duplicates: {binders!r}")
            counts["underspecified"] += 1
        else:
            raise ValueError(f"unknown scopeDependence kind: {kind!r}")
        counts["annotated"] += 1
    elif dependence is not None:
        raise ValueError("scopeDependence added to a non-constant object")
    return normalized


def normalize_claims(claims: str) -> tuple[str, int]:
    normalized: list[str] = []
    changed = 0
    for line in claims.splitlines(keepends=True):
        ending = "\n" if line.endswith("\n") else ""
        content = line.removesuffix("\n")
        match = FIXED_LINE.fullmatch(content)
        if match is not None:
            normalized.append(f"- exists {match.group(1)} [constant;{match.group(2)}]{ending}")
            changed += 1
            continue
        match = UNDERSPECIFIED_LINE.fullmatch(content)
        if match is not None:
            normalized.append(f"- exists {match.group(1)} [constant;{match.group(2)}]{ending}")
            changed += 1
            continue
        normalized.append(line)
    return "".join(normalized), changed


def verify_fixture(path: Path, base: str, totals: dict[str, int]) -> None:
    old_text = git("show", f"{base}:{path.as_posix()}")
    new_text = path.read_text(encoding="utf-8")
    old_document = tomllib.loads(old_text)
    new_document = tomllib.loads(new_text)
    normalized_document = copy.deepcopy(new_document)

    old_json_text = nested(old_document, "expectations", "output", "tersmu", "json")
    new_json_text = nested(new_document, "expectations", "output", "tersmu", "json")
    json_changed = old_json_text != new_json_text
    if json_changed:
        if not isinstance(old_json_text, str) or not isinstance(new_json_text, str):
            raise ValueError("tersmu JSON expectation was added or removed")
        old_json = json.loads(old_json_text)
        new_json = json.loads(new_json_text)
        normalized_json = validate_and_strip_scope_dependence(new_json, totals)
        if normalized_json != old_json:
            raise ValueError("canonical JSON changed beyond scopeDependence additions")
        replace_nested(
            normalized_document,
            old_json_text,
            "expectations",
            "output",
            "tersmu",
            "json",
        )

    old_claims = nested(old_document, "expectations", "output", "tersmu", "claims")
    new_claims = nested(new_document, "expectations", "output", "tersmu", "claims")
    claims_changed = old_claims != new_claims
    if claims_changed:
        if not isinstance(old_claims, str) or not isinstance(new_claims, str):
            raise ValueError("claims expectation was added or removed")
        normalized_claims, line_count = normalize_claims(new_claims)
        if normalized_claims != old_claims:
            raise ValueError("claims changed beyond formulaic denotation annotations")
        totals["claim_lines"] += line_count
        totals["claim_fixtures"] += 1
        replace_nested(
            normalized_document,
            old_claims,
            "expectations",
            "output",
            "tersmu",
            "claims",
        )

    if not json_changed and not claims_changed:
        raise ValueError("fixture changed without a tersmu JSON or claims migration")
    if normalized_document != old_document:
        raise ValueError("fixture metadata or another expectation changed")
    totals["fixtures"] += 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="main", help="git revision with pre-migration fixtures")
    args = parser.parse_args()

    changed = [
        Path(path)
        for path in git("diff", "--name-only", args.base, "--", str(FIXTURE_ROOT)).splitlines()
        if path.endswith(".toml")
    ]
    totals = {
        "fixtures": 0,
        "annotated": 0,
        "fixed": 0,
        "underspecified": 0,
        "claim_fixtures": 0,
        "claim_lines": 0,
    }
    errors: list[str] = []
    for path in changed:
        try:
            verify_fixture(path, args.base, totals)
        except (ValueError, KeyError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
            errors.append(f"{path}: {error}")

    if errors:
        print("scope-dependence fixture migration: FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("scope-dependence fixture migration: verified")
    print(f"fixtures touched: {totals['fixtures']}")
    print(f"nodes annotated: {totals['annotated']}")
    print(f"  fixed: {totals['fixed']}")
    print(f"  underspecified: {totals['underspecified']}")
    print(f"claims fixtures mechanically normalized: {totals['claim_fixtures']}")
    print(f"claims denotation lines annotated: {totals['claim_lines']}")
    print("unexpected changes requiring manual review: 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
