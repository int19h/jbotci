#!/usr/bin/env python3
"""Verify that issue #353 fixture edits are only predicted event-binding changes."""

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
EVENT_ID_TEXT = r"eventuality(?:/[A-Za-z]+)?:\d+"
EVENT_ID = re.compile(rf"\[({EVENT_ID_TEXT})\]")
PREDICATION_ID = re.compile(r"\[(predication:\d+)\]")
EVENT_ANNOTATION = re.compile(
    rf" \{{(event|tanru-head-event)=[^{{}}\n]*\[({EVENT_ID_TEXT})\]\}}"
)
EVENT_LABEL_TEXT = rf"[^,\n>{{}}]*?\[{EVENT_ID_TEXT}\]"
BINDING_ANNOTATION = re.compile(
    rf" binds=exists {EVENT_LABEL_TEXT}(?:, {EVENT_LABEL_TEXT})*"
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


def is_eventuality_object(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and value.get("type") == "referent"
        and isinstance(value.get("sort"), str)
        and (
            value["sort"] == "eventuality"
            or value["sort"].startswith("eventuality/")
        )
    )


def normalize_json(
    old_json: dict[str, Any],
    new_json: dict[str, Any],
    counts: dict[str, int],
) -> tuple[set[str], list[tuple[str, ...]]]:
    old_normalized = copy.deepcopy(old_json)
    new_normalized = copy.deepcopy(new_json)
    old_objects = old_normalized.get("objects")
    new_objects = new_normalized.get("objects")
    if not isinstance(old_objects, dict) or not isinstance(new_objects, dict):
        raise ValueError("canonical JSON lacks an objects map")
    if set(old_objects) != set(new_objects):
        raise ValueError("canonical JSON object IDs changed")

    generated: set[str] = set()
    referential: set[str] = set()
    for object_id, new_object in new_objects.items():
        old_object = old_objects[object_id]
        if not is_eventuality_object(new_object):
            if isinstance(new_object, dict) and "denotation" in new_object:
                raise ValueError(f"non-eventuality {object_id} gained denotation")
            continue
        if not is_eventuality_object(old_object):
            raise ValueError(f"eventuality identity changed for {object_id}")
        denotation = new_object.pop("denotation", None)
        if denotation == "generated-bound":
            if new_object.get("category") is not None:
                raise ValueError(f"generated event {object_id} retained category")
            if new_object.get("scopeDependence") is not None:
                raise ValueError(f"generated event {object_id} retained scopeDependence")
            if old_object.pop("category", None) != "constant":
                raise ValueError(f"generated event {object_id} was not formerly constant")
            if not isinstance(old_object.pop("scopeDependence", None), dict):
                raise ValueError(
                    f"generated event {object_id} lacked former scopeDependence"
                )
            generated.add(object_id)
            counts["generated_identities"] += 1
        elif denotation == "referential":
            referential.add(object_id)
            counts["referential_identities"] += 1
        else:
            raise ValueError(f"eventuality {object_id} has invalid denotation {denotation!r}")

    bound: dict[str, str] = {}
    owner_bindings: list[tuple[str, ...]] = []
    for owner_id, new_object in new_objects.items():
        if not isinstance(new_object, dict):
            continue
        eventualities = new_object.pop("boundEventualities", None)
        if eventualities is None:
            continue
        if new_object.get("type") not in {"formula", "sequence"}:
            raise ValueError(f"non-scope owner {owner_id} gained boundEventualities")
        if not isinstance(eventualities, list) or not eventualities:
            raise ValueError(f"owner {owner_id} has an empty or malformed binding")
        if not all(isinstance(eventuality, str) for eventuality in eventualities):
            raise ValueError(f"owner {owner_id} has a non-ID binding")
        if len(set(eventualities)) != len(eventualities):
            raise ValueError(f"owner {owner_id} repeats a binding")
        owner_bindings.append(tuple(eventualities))
        counts[f"{new_object['type']}_owners"] += 1
        for eventuality in eventualities:
            if eventuality not in generated:
                raise ValueError(
                    f"owner {owner_id} binds non-generated event {eventuality}"
                )
            if eventuality in bound:
                raise ValueError(
                    f"event {eventuality} is bound by both {bound[eventuality]} and {owner_id}"
                )
            bound[eventuality] = owner_id
            counts["binding_edges"] += 1
    if set(bound) != generated:
        missing = sorted(generated - set(bound))
        extra = sorted(set(bound) - generated)
        raise ValueError(f"generated-event binding mismatch: missing={missing}, extra={extra}")
    if generated & referential:
        raise ValueError("an eventuality has both generated and referential identity")
    if old_normalized != new_normalized:
        raise ValueError("canonical JSON changed beyond event identity and binding edges")
    return generated, owner_bindings


def validate_predication_annotations(
    text: str, objects: dict[str, Any] | None
) -> int:
    if objects is None:
        return len(EVENT_ANNOTATION.findall(text))
    count = 0
    for line in text.splitlines():
        predication_match = PREDICATION_ID.search(line)
        if predication_match is None:
            continue
        predication_id = predication_match.group(1)
        predication = objects.get(predication_id)
        if not isinstance(predication, dict) or predication.get("type") != "predication":
            raise ValueError(f"rendered unknown predication {predication_id}")
        annotations = {
            kind: eventuality
            for kind, eventuality in EVENT_ANNOTATION.findall(line)
        }
        expected_event = predication.get("eventuality")
        if annotations.get("event") != expected_event:
            raise ValueError(
                f"{predication_id} event marker {annotations.get('event')!r} "
                f"does not match {expected_event!r}"
            )
        tanru = predication.get("tanruLink")
        expected_head_event: Any | None = None
        if isinstance(tanru, dict):
            head = objects.get(tanru.get("head"))
            if isinstance(head, dict):
                expected_head_event = head.get("eventuality")
        if annotations.get("tanru-head-event") != expected_head_event:
            raise ValueError(
                f"{predication_id} tanru-head marker "
                f"{annotations.get('tanru-head-event')!r} does not match "
                f"{expected_head_event!r}"
            )
        count += len(annotations)
    return count


def normalize_rendered_additions(
    text: str,
    objects: dict[str, Any] | None,
    generated: set[str],
    owner_bindings: list[tuple[str, ...]],
) -> tuple[str, int, int]:
    event_marker_count = validate_predication_annotations(text, objects)
    normalized, removed_event_markers = EVENT_ANNOTATION.subn("", text)
    if removed_event_markers != event_marker_count:
        raise ValueError("event marker accounting mismatch")

    owner_binding_set = set(owner_bindings)
    binding_marker_count = 0

    def strip_binding(match: re.Match[str]) -> str:
        nonlocal binding_marker_count
        eventualities = tuple(EVENT_ID.findall(match.group(0)))
        if not eventualities or any(event not in generated for event in eventualities):
            raise ValueError(f"invalid rendered binding marker {match.group(0)!r}")
        if eventualities not in owner_binding_set:
            raise ValueError(
                f"rendered binding {eventualities!r} does not match an owner edge"
            )
        binding_marker_count += 1
        return ""

    normalized = BINDING_ANNOTATION.sub(strip_binding, normalized)
    if "binds=exists" in normalized:
        raise ValueError("unrecognized rendered binding marker")
    return normalized, event_marker_count, binding_marker_count


def remove_old_generated_denotations(
    claims: str, generated: set[str]
) -> tuple[str, int]:
    normalized: list[str] = []
    removed = 0
    for line in claims.splitlines(keepends=True):
        if line.startswith("- denotes ") and any(
            f"[{eventuality}]" in line for eventuality in generated
        ):
            removed += 1
            continue
        normalized.append(line)
    return "".join(normalized), removed


def verify_fixture(path: Path, base: str, totals: dict[str, int]) -> None:
    old_text = git("show", f"{base}:{path.as_posix()}")
    new_text = path.read_text(encoding="utf-8")
    old_document = tomllib.loads(old_text)
    new_document = tomllib.loads(new_text)
    normalized_document = copy.deepcopy(new_document)

    old_json_text = nested(old_document, "expectations", "output", "tersmu", "json")
    new_json_text = nested(new_document, "expectations", "output", "tersmu", "json")
    old_claims = nested(old_document, "expectations", "output", "tersmu", "claims")
    new_claims = nested(new_document, "expectations", "output", "tersmu", "claims")
    old_tree = nested(old_document, "expectations", "output", "tersmu", "tree")
    new_tree = nested(new_document, "expectations", "output", "tersmu", "tree")
    objects: dict[str, Any] | None
    if isinstance(old_json_text, str) and isinstance(new_json_text, str):
        old_json = json.loads(old_json_text)
        new_json = json.loads(new_json_text)
        generated, owner_bindings = normalize_json(old_json, new_json, totals)
        objects = new_json["objects"]
        replace_nested(
            normalized_document,
            old_json_text,
            "expectations",
            "output",
            "tersmu",
            "json",
        )
        totals["json_fixtures"] += 1
    elif old_json_text is None and new_json_text is None:
        rendered = "\n".join(
            text
            for text in (new_claims, new_tree)
            if isinstance(text, str)
        )
        owner_bindings = []
        for match in BINDING_ANNOTATION.finditer(rendered):
            binding = tuple(EVENT_ID.findall(match.group(0)))
            if binding and binding not in owner_bindings:
                owner_bindings.append(binding)
        generated = {
            eventuality
            for binding in owner_bindings
            for eventuality in binding
        }
        if not generated:
            raise ValueError("claims/tree-only migration has no generated-event binding")
        objects = None
        totals["generated_identities"] += len(generated)
        totals["binding_edges"] += sum(len(binding) for binding in owner_bindings)
        totals["claims_tree_only_fixtures"] += 1
    else:
        raise ValueError("tersmu JSON expectation was added or removed")

    if old_claims != new_claims:
        if not isinstance(old_claims, str) or not isinstance(new_claims, str):
            raise ValueError("claims expectation was added or removed")
        normalized_old_claims, removed_denotations = remove_old_generated_denotations(
            old_claims, generated
        )
        normalized_new_claims, event_markers, binding_markers = (
            normalize_rendered_additions(
                new_claims, objects, generated, owner_bindings
            )
        )
        if normalized_new_claims != normalized_old_claims:
            raise ValueError(
                "claims changed beyond generated-event denotation removal and explicit markers"
            )
        totals["claim_fixtures"] += 1
        totals["claim_denotations_removed"] += removed_denotations
        totals["claim_event_markers"] += event_markers
        totals["claim_binding_markers"] += binding_markers
        replace_nested(
            normalized_document,
            old_claims,
            "expectations",
            "output",
            "tersmu",
            "claims",
        )

    if old_tree != new_tree:
        if not isinstance(old_tree, str) or not isinstance(new_tree, str):
            raise ValueError("tree expectation was added or removed")
        normalized_tree, event_markers, binding_markers = normalize_rendered_additions(
            new_tree, objects, generated, owner_bindings
        )
        if normalized_tree != old_tree:
            raise ValueError("tree changed beyond explicit event and binding markers")
        totals["tree_fixtures"] += 1
        totals["tree_event_markers"] += event_markers
        totals["tree_binding_markers"] += binding_markers
        replace_nested(
            normalized_document,
            old_tree,
            "expectations",
            "output",
            "tersmu",
            "tree",
        )

    if normalized_document != old_document:
        raise ValueError("fixture metadata or another expectation changed")
    totals["fixtures"] += 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="main", help="revision with pre-migration fixtures")
    args = parser.parse_args()

    changed = [
        Path(path)
        for path in git(
            "diff", "--name-only", args.base, "--", str(FIXTURE_ROOT)
        ).splitlines()
        if path.endswith(".toml")
    ]
    totals = {
        "fixtures": 0,
        "json_fixtures": 0,
        "claims_tree_only_fixtures": 0,
        "generated_identities": 0,
        "referential_identities": 0,
        "binding_edges": 0,
        "formula_owners": 0,
        "sequence_owners": 0,
        "claim_fixtures": 0,
        "claim_denotations_removed": 0,
        "claim_event_markers": 0,
        "claim_binding_markers": 0,
        "tree_fixtures": 0,
        "tree_event_markers": 0,
        "tree_binding_markers": 0,
    }
    errors: list[str] = []
    for path in changed:
        try:
            verify_fixture(path, args.base, totals)
        except (ValueError, KeyError, json.JSONDecodeError, tomllib.TOMLDecodeError) as error:
            errors.append(f"{path}: {error}")

    if errors:
        print("event-binding fixture migration: FAILED", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        print(
            f"unexpected changes requiring manual review: {len(errors)}",
            file=sys.stderr,
        )
        return 1
    print("event-binding fixture migration: verified")
    print(f"fixtures touched: {totals['fixtures']}")
    print(f"canonical JSON fixtures: {totals['json_fixtures']}")
    print(
        "claims/tree-only fixtures: "
        f"{totals['claims_tree_only_fixtures']}"
    )
    print(f"generated-bound identities: {totals['generated_identities']}")
    print(f"referential identities: {totals['referential_identities']}")
    print(f"binding edges: {totals['binding_edges']}")
    print(f"  formula owners: {totals['formula_owners']}")
    print(f"  sequence owners: {totals['sequence_owners']}")
    print(f"claims fixtures mechanically normalized: {totals['claim_fixtures']}")
    print(f"  generated denotation lines removed: {totals['claim_denotations_removed']}")
    print(f"  explicit event markers: {totals['claim_event_markers']}")
    print(f"  explicit binding markers: {totals['claim_binding_markers']}")
    print(f"tree fixtures mechanically normalized: {totals['tree_fixtures']}")
    print(f"  explicit event markers: {totals['tree_event_markers']}")
    print(f"  explicit binding markers: {totals['tree_binding_markers']}")
    print("unexpected changes requiring manual review: 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
