#!/usr/bin/env python3
"""Verify that issue #351 fixture edits are only typed event annotations."""

from __future__ import annotations

import copy
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


FIXTURE_ROOT = Path("tests/fixtures/adhoc/output")
NEW_FIXTURES = {
    "tersmu-derived-event-tenseless.toml": (
        "time=unspecified; actuality=unspecified",
    ),
    "tersmu-derived-event-past.toml": (
        "time=before(anchor=now[eventuality:3]",
        "actuality=unspecified",
    ),
    "tersmu-derived-event-actuality.toml": ("actuality=actual",),
    "tersmu-derived-event-interval.toml": (
        "time=before(anchor=now[eventuality:3]",
        "time-interval=medium(anchor=unspecified)",
    ),
    "tersmu-derived-event-aspect.toml": (
        "aspect=continuative(anchor=unspecified",
        "interval-modifiers=[aspect(continuative(anchor=unspecified",
    ),
    "tersmu-derived-event-space.toml": (
        "space=distanceFrom(anchor=here[entity:4]; sticky=false; "
        "details={distance=short; otherwise=unspecified})",
    ),
    "tersmu-derived-connective-event.toml": (
        "scoped {event=eventuality[eventuality:22]; "
        "time=before(anchor=now[eventuality:3]",
        "binds=exists eventuality[eventuality:22] "
        "{time=before(anchor=now[eventuality:3]",
    ),
}
EVENT_ID = r"eventuality(?:/[A-Za-z]+)?:\d+"
EVENT_LABEL = rf"[^{{}}\n]*?\[{EVENT_ID}\]"
UNTENSED_CONDITIONS = (
    r"time=unspecified; actuality=(?:unspecified|actual); aspect=unspecified; "
    r"recurrence=unspecified; space=unspecified; spatial-aspect=unspecified; "
    r"spatial-recurrence=unspecified; details=unspecified"
)
EVENT_SITE = re.compile(
    rf" \{{(event|tanru-head-event)=({EVENT_LABEL}); ({UNTENSED_CONDITIONS})\}}"
)
BINDING_EVENT = re.compile(
    rf"({EVENT_LABEL}) \{{({UNTENSED_CONDITIONS})\}}"
)
DENOTATION_CONDITIONS = re.compile(
    rf"(denotes {EVENT_LABEL} \[)({UNTENSED_CONDITIONS}); "
)
UTTERANCE_EVENT = re.compile(
    rf"^(\s*(?:\w+\s+)*utterance \w+) \{{event={EVENT_LABEL}\}}( \[utterance:\d+\])$",
    re.MULTILINE,
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


def normalize_existing_rendering(text: str, tree: bool) -> tuple[str, dict[str, int]]:
    counts = {"sites": 0, "bindings": 0, "denotations": 0, "utterances": 0}

    def strip_site(match: re.Match[str]) -> str:
        counts["sites"] += 1
        return f" {{{match.group(1)}={match.group(2)}}}"

    normalized = EVENT_SITE.sub(strip_site, text)

    def strip_binding(match: re.Match[str]) -> str:
        counts["bindings"] += 1
        return match.group(1)

    normalized = BINDING_EVENT.sub(strip_binding, normalized)

    def strip_denotation(match: re.Match[str]) -> str:
        counts["denotations"] += 1
        return match.group(1)

    normalized = DENOTATION_CONDITIONS.sub(strip_denotation, normalized)
    if tree:
        normalized, counts["utterances"] = UTTERANCE_EVENT.subn(r"\1\2", normalized)

    if "time=" in normalized or "actuality=" in normalized:
        raise ValueError("unrecognized eventuality annotation remained after normalization")
    return normalized, counts


def verify_existing(path: Path, totals: dict[str, int]) -> None:
    old_document = tomllib.loads(git("show", f"main:{path.as_posix()}"))
    new_document = tomllib.loads(path.read_text(encoding="utf-8"))
    normalized_document = copy.deepcopy(new_document)

    for field in ("claims", "tree"):
        old_rendering = nested(old_document, "expectations", "output", "tersmu", field)
        new_rendering = nested(new_document, "expectations", "output", "tersmu", field)
        if not isinstance(old_rendering, str) or not isinstance(new_rendering, str):
            raise ValueError(f"{field} expectation was added, removed, or malformed")
        normalized, counts = normalize_existing_rendering(
            new_rendering, tree=field == "tree"
        )
        if normalized != old_rendering:
            raise ValueError(f"{field} changed beyond predicted issue #351 annotations")
        if counts["sites"] == 0 or counts["bindings"] == 0:
            raise ValueError(f"{field} lacks an annotated event use or binding site")
        if field == "claims" and counts["denotations"] < 2:
            raise ValueError("claims lacks annotated locution/now denotation lines")
        if field == "tree" and counts["utterances"] != 1:
            raise ValueError("tree lacks exactly one annotated utterance event")
        for key, count in counts.items():
            totals[f"{field}_{key}"] += count
        replace_nested(
            normalized_document,
            old_rendering,
            "expectations",
            "output",
            "tersmu",
            field,
        )

    if normalized_document != old_document:
        raise ValueError("fixture metadata, JSON, or another expectation changed")
    totals["existing"] += 1


def verify_new(path: Path, totals: dict[str, int]) -> None:
    document = tomllib.loads(path.read_text(encoding="utf-8"))
    if "issue-351" not in document.get("tags", []):
        raise ValueError("new regression fixture lacks the issue-351 tag")
    required = NEW_FIXTURES[path.name]
    for field in ("claims", "tree"):
        rendering = nested(document, "expectations", "output", "tersmu", field)
        if not isinstance(rendering, str) or not rendering:
            raise ValueError(f"new fixture lacks a nonempty exact {field} expectation")
        for marker in required:
            if marker not in rendering:
                raise ValueError(f"{field} lacks required marker {marker!r}")
        if "time=unspecified" not in rendering or "details=unspecified" not in rendering:
            raise ValueError(f"{field} does not exercise explicit absence markers")
    totals["new"] += 1


def main() -> int:
    changed = {
        Path(path)
        for path in git(
            "diff", "--name-only", "main", "--", str(FIXTURE_ROOT)
        ).splitlines()
        if path.endswith(".toml")
    }
    added = {
        Path(path)
        for path in git(
            "diff",
            "--diff-filter=A",
            "--name-only",
            "main",
            "--",
            str(FIXTURE_ROOT),
        ).splitlines()
        if path.endswith(".toml")
    }
    added |= {
        Path(path)
        for path in git(
            "ls-files", "--others", "--exclude-standard", "--", str(FIXTURE_ROOT)
        ).splitlines()
        if path.endswith(".toml")
    }
    paths = changed | added
    expected_new = {FIXTURE_ROOT / name for name in NEW_FIXTURES}
    if added != expected_new:
        print("eventuality-annotation fixture migration: FAILED", file=sys.stderr)
        print(
            f"unexpected new fixture set: expected={sorted(map(str, expected_new))}, "
            f"actual={sorted(map(str, added))}",
            file=sys.stderr,
        )
        return 1

    totals = {
        "existing": 0,
        "new": 0,
        "claims_sites": 0,
        "claims_bindings": 0,
        "claims_denotations": 0,
        "claims_utterances": 0,
        "tree_sites": 0,
        "tree_bindings": 0,
        "tree_denotations": 0,
        "tree_utterances": 0,
    }
    errors: list[str] = []
    for path in sorted(paths):
        try:
            if path in expected_new:
                verify_new(path, totals)
            elif path.name.startswith("tersmu-derived-"):
                verify_existing(path, totals)
            else:
                raise ValueError("unexpected regenerated fixture")
        except (ValueError, KeyError, tomllib.TOMLDecodeError) as error:
            errors.append(f"{path}: {error}")

    if errors:
        print("eventuality-annotation fixture migration: FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(f"unexpected changes requiring manual review: {len(errors)}", file=sys.stderr)
        return 1

    print("eventuality-annotation fixture migration: verified")
    print(f"existing fixtures mechanically normalized: {totals['existing']}")
    print(f"new exact regression fixtures: {totals['new']}")
    print(f"claims event sites: {totals['claims_sites']}")
    print(f"claims binding sites: {totals['claims_bindings']}")
    print(f"claims referential-event denotations: {totals['claims_denotations']}")
    print(f"tree event sites: {totals['tree_sites']}")
    print(f"tree binding sites: {totals['tree_bindings']}")
    print(f"tree utterance-event sites: {totals['tree_utterances']}")
    print("unexpected changes requiring manual review: 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
