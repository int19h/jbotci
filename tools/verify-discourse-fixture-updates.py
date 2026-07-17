#!/usr/bin/env python3
"""Verify the fixture churn for issues #447 and #448.

The script renders every affected fixture with an exact-main jbotci binary and
with the candidate binary.  It loads the old fixture from the requested Git
revision, proves that both binaries match their respective checked-in
expectations (including hash-only expectations), and then proves that the old
semantic graph embeds in the new one.  Every old reference is checked after
renumbering; the only accepted reference changes are topic-formula wrappers,
topic connection claims, and NIhO paragraph framing.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import tomllib
from typing import Any, Iterable


SEMANTIC_ID = re.compile(r"^[a-z][a-zA-Z0-9-]*(?:/[a-zA-Z0-9-]+)?:[1-9][0-9]*$")
DROP_REFERENCE = object()


class VerificationError(Exception):
    """A fixture changed in a way not covered by the two documented fixes."""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-bin", required=True, type=pathlib.Path)
    parser.add_argument("--current-bin", required=True, type=pathlib.Path)
    parser.add_argument(
        "--baseline-revision",
        default="origin/main",
        help="Git revision containing the old fixture expectations (default: origin/main)",
    )
    parser.add_argument(
        "--check-current-expectations",
        action="store_true",
        help="also require each working-tree expectation to equal current output",
    )
    parser.add_argument(
        "--manually-reviewed",
        action="append",
        default=[],
        type=pathlib.Path,
        help=(
            "fixture whose non-mechanical changes were manually reviewed; only references "
            "retargeted to newly retained topic or NIhO transition-support objects are accepted"
        ),
    )
    parser.add_argument("fixtures", nargs="+", type=pathlib.Path)
    return parser.parse_args()


def is_semantic_id(value: Any) -> bool:
    return isinstance(value, str) and SEMANTIC_ID.fullmatch(value) is not None


def semantic_references(value: Any) -> Iterable[str]:
    if is_semantic_id(value):
        yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from semantic_references(child)
    elif isinstance(value, list):
        for child in value:
            yield from semantic_references(child)


def semantic_reference_edges(
    value: Any, path: tuple[str | int, ...] = ()
) -> Iterable[tuple[tuple[str | int, ...], str]]:
    if is_semantic_id(value):
        yield path, value
    elif isinstance(value, dict):
        for key, child in value.items():
            yield from semantic_reference_edges(child, (*path, key))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from semantic_reference_edges(child, (*path, index))


def without_references(value: Any) -> Any:
    """Return the complete non-reference shape of a JSON value."""

    if is_semantic_id(value):
        return DROP_REFERENCE
    if isinstance(value, dict):
        result = {}
        for key, child in value.items():
            projected = without_references(child)
            if projected is not DROP_REFERENCE:
                result[key] = projected
        return result
    if isinstance(value, list):
        result = []
        dropped_reference = False
        for child in value:
            projected = without_references(child)
            if projected is DROP_REFERENCE:
                dropped_reference = True
            else:
                result.append(projected)
        if dropped_reference and not result:
            return DROP_REFERENCE
        return result
    return value


def shallow_signature(value: dict[str, Any]) -> str:
    return json.dumps(
        without_references(value),
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    )


def fixture_input(path: pathlib.Path) -> tuple[dict[str, Any], str]:
    with path.open("rb") as fixture_file:
        fixture = tomllib.load(fixture_file)
    if "lojban" in fixture:
        return fixture, fixture["lojban"]
    source_path = path.parent / fixture["lojban-filename"]
    return fixture, source_path.read_text(encoding="utf-8")


def repository_root() -> pathlib.Path:
    completed = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise VerificationError(f"cannot locate repository root: {completed.stderr.strip()}")
    return pathlib.Path(completed.stdout.strip()).resolve()


def git_file(revision: str, path: pathlib.PurePosixPath) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{path.as_posix()}"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise VerificationError(
            f"cannot read {path} from {revision}: {completed.stderr.decode(errors='replace').strip()}"
        )
    return completed.stdout


def baseline_fixture_input(
    path: pathlib.Path,
    revision: str,
    root: pathlib.Path,
) -> tuple[dict[str, Any], str]:
    try:
        relative = path.resolve().relative_to(root)
    except ValueError as error:
        raise VerificationError(f"fixture {path} is outside repository {root}") from error
    git_path = pathlib.PurePosixPath(relative.as_posix())
    fixture = tomllib.loads(git_file(revision, git_path).decode("utf-8"))
    if "lojban" in fixture:
        return fixture, fixture["lojban"]
    source_path = git_path.parent / fixture["lojban-filename"]
    return fixture, git_file(revision, source_path).decode("utf-8")


TERSMU_FORMATS = ("json", "tree", "tree+proj")


def fixture_configuration_without_tersmu_renderings(
    fixture: dict[str, Any],
) -> dict[str, Any]:
    projected = json.loads(json.dumps(fixture))
    tersmu = projected["expectations"]["output"]["tersmu"]
    for output_format in TERSMU_FORMATS:
        if output_format in tersmu:
            tersmu[output_format] = "<rendering expectation>"
    return projected


def render_output(
    binary: pathlib.Path,
    fixture: dict[str, Any],
    lojban: str,
    output_format: str,
) -> str:
    command = [str(binary), "tersmu", "--format", output_format]
    dialect = fixture.get("dialect")
    if dialect is not None:
        command.extend(["--dialect", dialect])
    tersmu = fixture["expectations"]["output"]["tersmu"]
    if tersmu.get("story-time", False):
        command.append("--story-time")
    completed = subprocess.run(
        command,
        input=lojban,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise VerificationError(
            f"{binary} failed with status {completed.returncode}: {completed.stderr.strip()}"
        )
    return completed.stdout.strip()


def render_graph(
    binary: pathlib.Path, fixture: dict[str, Any], lojban: str
) -> tuple[str, dict[str, Any]]:
    output = render_output(binary, fixture, lojban, "json")
    try:
        graph = json.loads(output)
    except json.JSONDecodeError as error:
        raise VerificationError(f"{binary} emitted invalid JSON: {error}") from error
    return output, graph


def expected_output(
    fixture: dict[str, Any], output_format: str
) -> str | dict[str, str]:
    return fixture["expectations"]["output"]["tersmu"][output_format]


def expectation_matches(expectation: str | dict[str, str], output: str) -> bool:
    if isinstance(expectation, str):
        return output == expectation
    expected_text = expectation.get("text", "")
    expected_hash = expectation.get("sha256")
    text_matches = not expected_text or output == expected_text
    hash_matches = (
        expected_hash is None
        or hashlib.sha256(output.encode()).hexdigest() == expected_hash
    )
    return text_matches and hash_matches


def require_valid_graph_shape(graph: dict[str, Any], label: str) -> dict[str, dict[str, Any]]:
    if graph.get("version") != "lojban-semantics-json-1":
        raise VerificationError(f"{label} has an unexpected semantic graph version")
    objects = graph.get("objects")
    root = graph.get("root")
    if not isinstance(objects, dict) or root not in objects:
        raise VerificationError(f"{label} has a missing object map or dangling root")
    dangling = {
        reference
        for value in objects.values()
        for reference in semantic_references(value)
        if reference not in objects
    }
    if dangling:
        raise VerificationError(f"{label} has dangling references: {sorted(dangling)[:5]}")
    return objects


def reachable_objects(graph: dict[str, Any]) -> set[str]:
    objects = graph["objects"]
    pending = [graph["root"]]
    reached: set[str] = set()
    while pending:
        object_id = pending.pop()
        if object_id in reached:
            continue
        reached.add(object_id)
        pending.extend(
            reference
            for reference in semantic_references(objects[object_id])
            if reference not in reached
        )
    return reached


def topic_addition_closure(objects: dict[str, dict[str, Any]]) -> tuple[set[str], list[str]]:
    topic_predications = [
        object_id
        for object_id, value in objects.items()
        if value.get("type") == "predication" and value.get("relation") == "topicOf"
    ]
    allowed: set[str] = set()
    topic_formulae: set[str] = set()

    def add_outgoing_closure(seed: str) -> None:
        pending = [seed]
        while pending:
            object_id = pending.pop()
            if object_id in allowed:
                continue
            allowed.add(object_id)
            pending.extend(semantic_references(objects[object_id]))

    for predication_id in topic_predications:
        predication = objects[predication_id]
        arguments = predication.get("arguments", {})
        topic = arguments.get("x1", {}).get("value")
        comment = arguments.get("x2", {}).get("value")
        if (
            predication.get("mode") not in {"asserted", "restrictive", "incidental"}
            or predication.get("introducedBy") != "zo'u"
            or predication.get("source", {}).get("construct") != "topic-comment"
            or not is_semantic_id(topic)
            or objects[topic].get("type") not in {"referent", "parameter"}
            or (
                objects[topic].get("type") == "parameter"
                and objects[topic].get("sort") != "entity"
            )
            or not is_semantic_id(comment)
            or objects[comment].get("type") != "referent"
            or objects[comment].get("sort") != "eventuality"
        ):
            raise VerificationError(f"malformed topic-comment predication {predication_id}")
        allowed.add(predication_id)
        eventuality = predication.get("eventuality")
        if is_semantic_id(eventuality):
            allowed.add(eventuality)
        add_outgoing_closure(topic)
        # The comment itself is retained, except when a compound comment needs
        # a newly reified eventuality.  Permit that one node but do not absorb
        # the existing comment graph into the addition closure.
        allowed.add(comment)
        for formula_id, value in objects.items():
            if value.get("type") == "formula" and value.get("predication") == predication_id:
                topic_formulae.add(formula_id)

    allowed.update(topic_formulae)
    changed = True
    while changed:
        changed = False
        for object_id, value in objects.items():
            if value.get("type") not in {"formula", "question"} or object_id in allowed:
                continue
            if any(reference in allowed for reference in semantic_references(value)):
                allowed.add(object_id)
                changed = True
    return allowed, topic_predications


def transition_support_closure(objects: dict[str, dict[str, Any]]) -> set[str]:
    """Return quote wrappers required when a retained NIhO makes quote content a sequence."""

    paragraph_boundaries = {
        object_id
        for object_id, value in objects.items()
        if value.get("type") == "sequence"
        and value.get("source", {}).get("construct") == "paragraph-boundary"
    }

    def sequence_contains_boundary(sequence_id: str) -> bool:
        pending = [sequence_id]
        visited: set[str] = set()
        while pending:
            object_id = pending.pop()
            if object_id in paragraph_boundaries:
                return True
            if object_id in visited:
                continue
            visited.add(object_id)
            value = objects[object_id]
            pending.extend(
                reference
                for reference in semantic_references(value.get("items", []))
                if objects[reference].get("type") == "sequence"
            )
        return False

    allowed: set[str] = set()
    for object_id, value in objects.items():
        content = value.get("content")
        if (
            value.get("type") != "utterance"
            or value.get("force") != "quote"
            or value.get("source", {}).get("construct") != "quotation"
            or not is_semantic_id(content)
            or objects[content].get("type") != "sequence"
            or not sequence_contains_boundary(content)
        ):
            continue
        allowed.add(object_id)
        eventuality = value.get("eventuality")
        if is_semantic_id(eventuality):
            allowed.add(eventuality)
    return allowed


def old_stable_sequence(value: dict[str, Any]) -> bool:
    source = value.get("source")
    plain_same_topic_frame = (
        value.get("type") == "sequence"
        and value.get("relation") == "same-topic-continuation"
        and not value.get("content")
        and not value.get("connectionClaims")
        and not value.get("nonlogicalConnection")
        and not value.get("force")
    )
    return not (
        plain_same_topic_frame
        and (source is None or source.get("construct") == "text")
    )


def semantic_id_order(object_id: str) -> tuple[int, str]:
    return int(object_id.rsplit(":", 1)[1]), object_id


def topic_scaffolding(
    objects: dict[str, dict[str, Any]],
) -> tuple[set[str], set[str], set[str]]:
    topic_predications = {
        object_id
        for object_id, value in objects.items()
        if value.get("type") == "predication"
        and value.get("relation") == "topicOf"
        and value.get("introducedBy") == "zo'u"
        and value.get("source", {}).get("construct") == "topic-comment"
    }
    topic_atoms = {
        object_id
        for object_id, value in objects.items()
        if value.get("type") == "formula"
        and value.get("operator") == "atom"
        and value.get("predication") in topic_predications
    }
    topic_wrappers = {
        object_id
        for object_id, value in objects.items()
        if value.get("type") == "formula"
        and value.get("operator") == "and"
        and value.get("source", {}).get("construct") == "topic-comment"
        and len(value.get("children", [])) >= 2
        and all(child in topic_atoms for child in value["children"][1:])
    }
    return topic_predications, topic_atoms, topic_wrappers


def paragraph_frame_ids(objects: dict[str, dict[str, Any]]) -> set[str]:
    return {
        object_id
        for object_id, value in objects.items()
        if value.get("type") == "sequence"
        and value.get("source", {}).get("construct")
        in {"paragraph", "paragraph-boundary"}
    }


def validate_paragraph_frames(
    objects: dict[str, dict[str, Any]], frame_ids: set[str]
) -> None:
    for object_id in frame_ids:
        value = objects[object_id]
        construct = value["source"]["construct"]
        if construct == "paragraph":
            if (
                value.get("relation") != "same-topic-continuation"
                or len(value.get("items", [])) < 2
                or value.get("content") is not None
                or value.get("connectionClaims")
                or value.get("nonlogicalConnection") is not None
            ):
                raise VerificationError(f"malformed paragraph frame {object_id}")
            continue
        relation = value.get("relation")
        boundary = relation.get("paragraph-boundary") if isinstance(relation, dict) else None
        if (
            not isinstance(boundary, dict)
            or boundary.get("transition") not in {"new-topic", "resume-prior-topic"}
            or not isinstance(boundary.get("additional"), list)
            or any(
                transition not in {"new-topic", "resume-prior-topic"}
                for transition in boundary["additional"]
            )
            or len(value.get("items", [])) > 2
            or value.get("content") is not None
            or value.get("connectionClaims")
            or value.get("nonlogicalConnection") is not None
        ):
            raise VerificationError(f"malformed paragraph-boundary frame {object_id}")


def flattened_paragraph_items(
    objects: dict[str, dict[str, Any]],
    object_id: str,
    frame_ids: set[str],
) -> list[str]:
    flattened: list[str] = []
    pending: list[tuple[str, bool]] = [(object_id, False)]
    visiting: set[str] = set()
    while pending:
        current, leaving = pending.pop()
        if current not in frame_ids:
            flattened.append(current)
            continue
        if leaving:
            visiting.remove(current)
            continue
        if current in visiting:
            raise VerificationError(f"paragraph frame cycle at {current}")
        visiting.add(current)
        pending.append((current, True))
        for child in reversed(objects[current].get("items", [])):
            pending.append((child, False))
    return flattened


def match_retained_objects(
    old_objects: dict[str, dict[str, Any]],
    new_objects: dict[str, dict[str, Any]],
    allow_topic_retargets: bool,
) -> tuple[dict[str, str], set[str]]:
    old_signatures = {
        object_id: shallow_signature(value) for object_id, value in old_objects.items()
    }
    new_signatures = {
        object_id: shallow_signature(value) for object_id, value in new_objects.items()
    }
    old_by_signature: dict[str, list[str]] = collections.defaultdict(list)
    new_counts: collections.Counter[str] = collections.Counter()
    for object_id, signature in old_signatures.items():
        old_by_signature[signature].append(object_id)
    new_counts.update(new_signatures.values())

    new_frames = paragraph_frame_ids(new_objects)
    new_boundary_leaf_signatures = {
        tuple(
            new_signatures[item]
            for item in flattened_paragraph_items(new_objects, frame, new_frames)
        )
        for frame in new_frames
        if new_objects[frame]["source"]["construct"] == "paragraph-boundary"
    }
    omitted_frames = {
        object_id
        for object_id, value in old_objects.items()
        if not old_stable_sequence(value)
        and tuple(old_signatures[item] for item in value.get("items", []))
        in new_boundary_leaf_signatures
    }
    for signature, old_ids in old_by_signature.items():
        retained_old_ids = [
            object_id for object_id in old_ids if object_id not in omitted_frames
        ]
        deficit = len(retained_old_ids) - new_counts[signature]
        if deficit <= 0:
            continue
        raise VerificationError(
            f"candidate removed or changed {deficit} non-framing objects with signature "
            f"{signature[:160]}"
        )

    def incoming_contexts(
        objects: dict[str, dict[str, Any]],
        signatures: dict[str, str],
    ) -> dict[str, collections.Counter[tuple[tuple[str | int, ...], str]]]:
        contexts: dict[
            str, collections.Counter[tuple[tuple[str | int, ...], str]]
        ] = collections.defaultdict(collections.Counter)
        for source_id, source in objects.items():
            source_signature = signatures[source_id]
            for path, target_id in semantic_reference_edges(source):
                contexts[target_id][(path, source_signature)] += 1
        return contexts

    old_contexts = incoming_contexts(old_objects, old_signatures)
    new_contexts = incoming_contexts(new_objects, new_signatures)
    _topic_predications, topic_atoms, topic_wrappers = topic_scaffolding(new_objects)
    topic_closure, _topic_predication_list = topic_addition_closure(new_objects)
    topic_wrapped_children = {
        new_objects[wrapper]["children"][0] for wrapper in topic_wrappers
    }
    topic_comment_events = {
        new_objects[predication]
        .get("arguments", {})
        .get("x2", {})
        .get("value")
        for predication in _topic_predications
    }
    topic_comment_events.discard(None)

    def structural_colors(
        objects: dict[str, dict[str, Any]],
        signatures: dict[str, str],
        wrappers: set[str],
        ignored_claims: set[str],
    ) -> dict[str, str]:
        edges: dict[str, list[tuple[tuple[str | int, ...], str]]] = {}
        for object_id, value in objects.items():
            object_edges = []
            for path, target_id in semantic_reference_edges(value):
                if path and path[0] == "boundEventualities":
                    continue
                if path and path[0] == "connectionClaims" and target_id in ignored_claims:
                    continue
                while target_id in wrappers:
                    target_id = objects[target_id]["children"][0]
                object_edges.append((path, target_id))
            edges[object_id] = object_edges

        colors = {
            object_id: hashlib.sha256(signature.encode()).hexdigest()
            for object_id, signature in signatures.items()
        }
        for _ in range(16):
            colors = {
                object_id: hashlib.sha256(
                    json.dumps(
                        [
                            signatures[object_id],
                            [
                                [path, colors[target_id]]
                                for path, target_id in edges[object_id]
                            ],
                        ],
                        ensure_ascii=False,
                        separators=(",", ":"),
                    ).encode()
                ).hexdigest()
                for object_id in objects
            }
        return colors

    old_colors = structural_colors(old_objects, old_signatures, set(), set())
    new_colors = structural_colors(
        new_objects, new_signatures, topic_wrappers, topic_atoms
    )

    def outgoing_context(
        object_id: str,
        objects: dict[str, dict[str, Any]],
        signatures: dict[str, str],
        wrappers: set[str],
    ) -> collections.Counter[tuple[tuple[str | int, ...], str]]:
        context: collections.Counter[tuple[tuple[str | int, ...], str]] = (
            collections.Counter()
        )
        for path, target_id in semantic_reference_edges(objects[object_id]):
            if path and path[0] == "boundEventualities":
                continue
            while target_id in wrappers:
                target_id = objects[target_id]["children"][0]
            context[(path, signatures[target_id])] += 1
        return context

    old_outgoing = {
        object_id: outgoing_context(object_id, old_objects, old_signatures, set())
        for object_id in old_objects
    }
    new_outgoing = {
        object_id: outgoing_context(
            object_id, new_objects, new_signatures, topic_wrappers
        )
        for object_id in new_objects
    }
    new_by_signature: dict[str, list[str]] = collections.defaultdict(list)
    for object_id, signature in new_signatures.items():
        new_by_signature[signature].append(object_id)

    mapping: dict[str, str] = {}
    for signature, all_old_ids in old_by_signature.items():
        old_ids = [object_id for object_id in all_old_ids if object_id not in omitted_frames]
        if not old_ids:
            continue
        new_ids = new_by_signature[signature]
        if len(old_ids) == 1 and len(new_ids) == 1:
            mapping[old_ids[0]] = new_ids[0]
            continue

        compatible: dict[str, list[str]] = {}
        for old_id in old_ids:
            base_compatible = [
                new_id
                for new_id in new_ids
                if (
                    allow_topic_retargets
                    or not (old_outgoing[old_id] - new_outgoing[new_id])
                )
                and (
                    allow_topic_retargets
                    or
                    not (old_contexts[old_id] - new_contexts[new_id])
                    or new_id in topic_wrapped_children
                    or new_id in topic_comment_events
                )
            ]
            color_compatible = [
                new_id
                for new_id in base_compatible
                if old_colors[old_id] == new_colors[new_id]
            ]
            compatible[old_id] = color_compatible or base_compatible
            if not compatible[old_id]:
                raise VerificationError(
                    f"retained object {old_id} has no reference-context-compatible candidate"
                )

        candidate_to_old: dict[str, str] = {}

        def assign(old_id: str, visited: set[str]) -> bool:
            ordered = sorted(compatible[old_id], key=semantic_id_order)
            for new_id in ordered:
                if new_id not in candidate_to_old:
                    candidate_to_old[new_id] = old_id
                    return True
            for new_id in ordered:
                if new_id in visited:
                    continue
                visited.add(new_id)
                previous = candidate_to_old[new_id]
                if assign(previous, visited):
                    candidate_to_old[new_id] = old_id
                    return True
            return False

        for old_id in sorted(old_ids, key=lambda item: (len(compatible[item]), semantic_id_order(item))):
            if not assign(old_id, set()):
                raise VerificationError(
                    f"objects with signature {signature[:160]} have no one-to-one retained mapping"
                )
        mapping.update({old_id: new_id for new_id, old_id in candidate_to_old.items()})

    def comparable_edges(
        objects: dict[str, dict[str, Any]],
        wrappers: set[str],
        ignored_claims: set[str],
    ) -> tuple[
        dict[str, list[tuple[tuple[str | int, ...], str]]],
        dict[str, list[tuple[tuple[str | int, ...], str]]],
    ]:
        outgoing: dict[str, list[tuple[tuple[str | int, ...], str]]] = {}
        incoming: dict[str, list[tuple[tuple[str | int, ...], str]]] = (
            collections.defaultdict(list)
        )
        for source_id, value in objects.items():
            source_edges = []
            for path, target_id in semantic_reference_edges(value):
                if path and path[0] == "boundEventualities":
                    continue
                if path and path[0] == "connectionClaims" and target_id in ignored_claims:
                    continue
                while target_id in wrappers:
                    target_id = objects[target_id]["children"][0]
                source_edges.append((path, target_id))
                incoming[target_id].append((path, source_id))
            outgoing[source_id] = source_edges
        return outgoing, incoming

    old_edges, old_incoming = comparable_edges(old_objects, set(), set())
    new_edges, new_incoming = comparable_edges(
        new_objects, topic_wrappers, topic_atoms
    )

    for _ in range(4):
        refined = dict(mapping)
        retained_new_ids = set(mapping.values())
        for signature, all_old_ids in old_by_signature.items():
            old_ids = [
                object_id
                for object_id in all_old_ids
                if object_id not in omitted_frames
            ]
            new_ids = new_by_signature[signature]
            if len(old_ids) <= 1:
                continue

            compatible: dict[str, list[str]] = {}
            group_incompatible = False
            for old_id in old_ids:
                candidates = []
                for new_id in new_ids:
                    new_outgoing_by_path = dict(new_edges[new_id])
                    outgoing_matches = all(
                        old_target not in mapping
                        or new_outgoing_by_path.get(path) == mapping[old_target]
                        or (
                            allow_topic_retargets
                            and new_outgoing_by_path.get(path) not in retained_new_ids
                        )
                        for path, old_target in old_edges[old_id]
                    )
                    if not outgoing_matches:
                        continue
                    if (
                        new_id not in topic_wrapped_children
                        and new_id not in topic_comment_events
                    ):
                        new_incoming_edges = set(new_incoming[new_id])
                        incoming_matches = all(
                            old_source in omitted_frames
                            or old_source not in mapping
                            or (path, mapping[old_source]) in new_incoming_edges
                            or (
                                allow_topic_retargets
                                and dict(new_edges[mapping[old_source]]).get(path)
                                not in retained_new_ids
                            )
                            for path, old_source in old_incoming[old_id]
                        )
                        if not incoming_matches:
                            continue
                    candidates.append(new_id)
                compatible[old_id] = candidates
                if not candidates:
                    if allow_topic_retargets:
                        group_incompatible = True
                        break
                    raise VerificationError(
                        f"retained object {old_id} has no adjacency-compatible candidate"
                    )
            if group_incompatible:
                continue

            candidate_to_old: dict[str, str] = {}

            def assign_refined(old_id: str, visited: set[str]) -> bool:
                preferred = sorted(
                    compatible[old_id],
                    key=lambda new_id: (
                        old_colors[old_id] != new_colors[new_id],
                        semantic_id_order(new_id),
                    ),
                )
                for new_id in preferred:
                    if new_id not in candidate_to_old:
                        candidate_to_old[new_id] = old_id
                        return True
                for new_id in preferred:
                    if new_id in visited:
                        continue
                    visited.add(new_id)
                    previous = candidate_to_old[new_id]
                    if assign_refined(previous, visited):
                        candidate_to_old[new_id] = old_id
                        return True
                return False

            assignment_failed = False
            for old_id in sorted(
                old_ids,
                key=lambda item: (len(compatible[item]), semantic_id_order(item)),
            ):
                if not assign_refined(old_id, set()):
                    if allow_topic_retargets:
                        assignment_failed = True
                        break
                    raise VerificationError(
                        f"objects with signature {signature[:160]} have no adjacency-preserving mapping"
                    )
            if assignment_failed:
                continue
            for new_id, old_id in candidate_to_old.items():
                refined[old_id] = new_id
        if refined == mapping:
            break
        mapping = refined

    # Repeated source-less placeholders are intentionally indistinguishable by
    # their own fields.  Once their owning objects are paired, the reference
    # path provides the identity: propagate those exact owner edges through the
    # mapping, swapping otherwise-symmetric candidates as needed.
    for _ in range(8):
        changed = False
        reverse = {new_id: old_id for old_id, new_id in mapping.items()}
        for old_source, new_source in sorted(mapping.items(), key=lambda item: semantic_id_order(item[0])):
            new_targets = dict(new_edges[new_source])
            for path, old_target in old_edges[old_source]:
                if old_target not in mapping:
                    continue
                new_target = new_targets.get(path)
                if (
                    new_target is None
                    or old_signatures[old_target] != new_signatures[new_target]
                    or mapping[old_target] == new_target
                ):
                    continue
                displaced_old = reverse.get(new_target)
                displaced_new = mapping[old_target]
                mapping[old_target] = new_target
                if displaced_old is not None:
                    mapping[displaced_old] = displaced_new
                changed = True
                reverse = {candidate: original for original, candidate in mapping.items()}
        if not changed:
            break
    return mapping, omitted_frames


def paragraph_frame_replacements(
    old_objects: dict[str, dict[str, Any]],
    new_objects: dict[str, dict[str, Any]],
    retained_mapping: dict[str, str],
    omitted_old_frames: set[str],
    new_frame_ids: set[str],
) -> dict[str, str]:
    replacements: dict[str, str] = {}
    used_new_frames: set[str] = set()
    for old_id in sorted(omitted_old_frames, key=semantic_id_order):
        expected_items = [retained_mapping[item] for item in old_objects[old_id]["items"]]
        candidates = [
            new_id
            for new_id in new_frame_ids - used_new_frames
            if new_objects[new_id]["source"]["construct"] == "paragraph-boundary"
            and flattened_paragraph_items(new_objects, new_id, new_frame_ids) == expected_items
        ]
        if len(candidates) != 1:
            raise VerificationError(
                f"old flat sequence {old_id} has {len(candidates)} candidate NIhO replacements"
            )
        replacement = candidates[0]
        replacements[replacement] = old_id
        used_new_frames.add(replacement)

    reverse = {new_id: old_id for old_id, new_id in retained_mapping.items()}
    for new_id in new_frame_ids:
        if new_id in replacements:
            continue
        leaves = flattened_paragraph_items(new_objects, new_id, new_frame_ids)
        if (
            new_objects[new_id]["source"]["construct"] == "paragraph-boundary"
            and len(leaves) == 1
            and leaves[0] in reverse
        ):
            replacements[new_id] = reverse[leaves[0]]

    transition_support = transition_support_closure(new_objects)
    for new_id in transition_support:
        new_value = new_objects[new_id]
        if new_value.get("type") != "utterance" or new_value.get("force") != "quote":
            continue
        old_content = replacements.get(new_value.get("content"))
        if old_content is None:
            continue
        candidates = [
            old_id
            for old_id, old_value in old_objects.items()
            if old_value.get("type") == "utterance"
            and old_value.get("force") == "quote"
            and old_value.get("content") == old_content
            and shallow_signature(old_value) == shallow_signature(new_value)
        ]
        if len(candidates) != 1:
            continue
        old_id = candidates[0]
        replacements[new_id] = old_id
        old_eventuality = old_objects[old_id].get("eventuality")
        new_eventuality = new_value.get("eventuality")
        if is_semantic_id(old_eventuality) and is_semantic_id(new_eventuality):
            replacements[new_eventuality] = old_eventuality
    return replacements


def normalize_new_reference(
    object_id: str,
    objects: dict[str, dict[str, Any]],
    retained_reverse: dict[str, str],
    topic_wrappers: set[str],
    frame_replacements: dict[str, str],
) -> str:
    seen: set[str] = set()
    while object_id in topic_wrappers:
        if object_id in seen:
            raise VerificationError(f"topic wrapper cycle at {object_id}")
        seen.add(object_id)
        object_id = objects[object_id]["children"][0]
    if object_id in frame_replacements:
        return frame_replacements[object_id]
    return retained_reverse.get(object_id, f"<new:{object_id}>")


def normalize_retained_new_object(
    value: dict[str, Any],
    objects: dict[str, dict[str, Any]],
    retained_reverse: dict[str, str],
    topic_atoms: set[str],
    topic_wrappers: set[str],
    frame_replacements: dict[str, str],
) -> dict[str, Any]:
    projected = json.loads(json.dumps(value))
    if projected.get("type") == "sequence" and "connectionClaims" in projected:
        projected["connectionClaims"] = [
            claim for claim in projected["connectionClaims"] if claim not in topic_atoms
        ]
        if not projected["connectionClaims"]:
            del projected["connectionClaims"]

    def normalize(child: Any) -> Any:
        if is_semantic_id(child):
            return normalize_new_reference(
                child,
                objects,
                retained_reverse,
                topic_wrappers,
                frame_replacements,
            )
        if isinstance(child, dict):
            return {key: normalize(nested) for key, nested in child.items()}
        if isinstance(child, list):
            return [normalize(nested) for nested in child]
        return child

    return normalize(projected)


def restore_mechanically_promoted_topic_bindings(
    old_value: dict[str, Any],
    new_id: str,
    normalized: dict[str, Any],
    new_objects: dict[str, dict[str, Any]],
    retained_mapping: dict[str, str],
    topic_wrappers: set[str],
) -> None:
    old_bindings = old_value.get("boundEventualities", [])
    normalized_bindings = normalized.get("boundEventualities", [])
    if not old_bindings or normalized_bindings == old_bindings:
        return
    if any(binding not in old_bindings for binding in normalized_bindings):
        return
    missing = [binding for binding in old_bindings if binding not in normalized_bindings]
    promoted = {retained_mapping[binding] for binding in missing}
    wrappers = [
        new_objects[wrapper]
        for wrapper in topic_wrappers
        if new_objects[wrapper].get("children", [None])[0] == new_id
        and promoted <= set(new_objects[wrapper].get("boundEventualities", []))
    ]
    if len(wrappers) != 1:
        return
    normalized["boundEventualities"] = old_bindings


def reviewed_reference_retargets(
    old_value: Any,
    new_value: Any,
    allowed_new_targets: set[str],
    path: tuple[str | int, ...] = (),
) -> list[tuple[tuple[str | int, ...], str, str]] | None:
    if old_value == new_value:
        return []
    if is_semantic_id(old_value) and isinstance(new_value, str):
        match = re.fullmatch(r"<new:(.+)>", new_value)
        if match is not None and match.group(1) in allowed_new_targets:
            return [(path, old_value, match.group(1))]
        return None
    if isinstance(old_value, dict) and isinstance(new_value, dict):
        if old_value.keys() != new_value.keys():
            return None
        changes: list[tuple[tuple[str | int, ...], str, str]] = []
        for key in old_value:
            nested = reviewed_reference_retargets(
                old_value[key], new_value[key], allowed_new_targets, (*path, key)
            )
            if nested is None:
                return None
            changes.extend(nested)
        return changes
    if isinstance(old_value, list) and isinstance(new_value, list):
        if len(old_value) != len(new_value):
            return None
        changes = []
        for index, (old_child, new_child) in enumerate(zip(old_value, new_value)):
            nested = reviewed_reference_retargets(
                old_child,
                new_child,
                allowed_new_targets,
                (*path, index),
            )
            if nested is None:
                return None
            changes.extend(nested)
        return changes
    return None


def verify_mechanical_change(
    old_graph: dict[str, Any],
    new_graph: dict[str, Any],
    allow_topic_retargets: bool,
) -> tuple[int, int, int, list[str]]:
    old_objects = require_valid_graph_shape(old_graph, "baseline graph")
    new_objects = require_valid_graph_shape(new_graph, "candidate graph")
    reachable = reachable_objects(new_graph)
    if reachable != set(new_objects):
        raise VerificationError(
            f"candidate graph has {len(set(new_objects) - reachable)} unreachable objects"
        )

    topic_closure, topic_predication_list = topic_addition_closure(new_objects)
    topic_predications, topic_atoms, topic_wrappers = topic_scaffolding(new_objects)
    if topic_predications != set(topic_predication_list):
        raise VerificationError("candidate contains a malformed topicOf predication")

    new_frames = paragraph_frame_ids(new_objects)
    validate_paragraph_frames(new_objects, new_frames)
    retained, omitted_old_frames = match_retained_objects(
        old_objects, new_objects, allow_topic_retargets
    )
    retained_reverse = {new_id: old_id for old_id, new_id in retained.items()}
    frame_replacements = paragraph_frame_replacements(
        old_objects,
        new_objects,
        retained,
        omitted_old_frames,
        new_frames,
    )

    normalized_root = normalize_new_reference(
        new_graph["root"],
        new_objects,
        retained_reverse,
        topic_wrappers,
        frame_replacements,
    )
    if normalized_root != old_graph["root"]:
        raise VerificationError(
            f"candidate root normalizes to {normalized_root}, expected {old_graph['root']}"
        )

    manual_retargets: list[str] = []
    transition_support = transition_support_closure(new_objects)
    allowed_new_targets = (topic_closure | transition_support) - set(retained_reverse)
    for old_id, new_id in retained.items():
        normalized = normalize_retained_new_object(
            new_objects[new_id],
            new_objects,
            retained_reverse,
            topic_atoms,
            topic_wrappers,
            frame_replacements,
        )
        restore_mechanically_promoted_topic_bindings(
            old_objects[old_id],
            new_id,
            normalized,
            new_objects,
            retained,
            topic_wrappers,
        )
        if normalized != old_objects[old_id]:
            if allow_topic_retargets:
                retargets = reviewed_reference_retargets(
                    old_objects[old_id], normalized, allowed_new_targets
                )
                if retargets:
                    for path, old_target, new_target in retargets:
                        rendered_path = ".".join(str(component) for component in path)
                        manual_retargets.append(
                            f"{old_id}->{new_id}:{rendered_path} {old_target}->{new_target}"
                        )
                    continue
            raise VerificationError(
                f"retained object {old_id} -> {new_id} changed fields or references: "
                f"old={json.dumps(old_objects[old_id], sort_keys=True)} "
                f"new-normalized={json.dumps(normalized, sort_keys=True)}"
            )

    allowed_additions = topic_closure | new_frames | transition_support
    unmapped = set(new_objects) - set(retained_reverse)
    unaccounted = unmapped - allowed_additions
    if unaccounted:
        sample = sorted(unaccounted, key=semantic_id_order)[:8]
        raise VerificationError(
            f"candidate added {len(unaccounted)} objects outside documented additions: {sample}"
        )

    new_topic_predications = topic_predications & unmapped
    new_paragraph_boundaries = {
        object_id
        for object_id in new_frames & unmapped
        if new_objects[object_id]["source"]["construct"] == "paragraph-boundary"
    }
    if not new_topic_predications and not new_paragraph_boundaries:
        raise VerificationError("candidate graph contains neither documented discourse addition")
    if not (new_topic_predications | new_paragraph_boundaries) <= reachable:
        raise VerificationError("a documented discourse addition is not reachable from the graph root")

    return (
        len(new_topic_predications),
        len(new_paragraph_boundaries),
        len(new_objects) - len(old_objects),
        manual_retargets,
    )


def verify_fixture(
    path: pathlib.Path,
    baseline_binary: pathlib.Path,
    current_binary: pathlib.Path,
    baseline_revision: str,
    root: pathlib.Path,
    check_current_expectations: bool,
    allow_topic_retargets: bool,
) -> tuple[int, int, int, list[str]]:
    baseline_fixture, baseline_lojban = baseline_fixture_input(
        path, baseline_revision, root
    )
    current_fixture, current_lojban = fixture_input(path)
    if baseline_lojban != current_lojban:
        raise VerificationError("fixture Lojban input changed alongside its expectation")
    if fixture_configuration_without_tersmu_renderings(
        baseline_fixture
    ) != fixture_configuration_without_tersmu_renderings(current_fixture):
        raise VerificationError(
            "fixture configuration changed beyond its tersmu rendering expectations"
        )

    baseline_output, baseline_graph = render_graph(
        baseline_binary, baseline_fixture, baseline_lojban
    )
    baseline_expectation = expected_output(baseline_fixture, "json")
    if not expectation_matches(baseline_expectation, baseline_output):
        raise VerificationError("exact-main output does not match the checked-in baseline expectation")
    current_output, current_graph = render_graph(
        current_binary, current_fixture, current_lojban
    )
    current_expectation = expected_output(current_fixture, "json")
    if check_current_expectations and not expectation_matches(
        current_expectation, current_output
    ):
        raise VerificationError("candidate output does not match the updated working-tree expectation")

    baseline_formats = {
        output_format
        for output_format in TERSMU_FORMATS
        if output_format in baseline_fixture["expectations"]["output"]["tersmu"]
    }
    current_formats = {
        output_format
        for output_format in TERSMU_FORMATS
        if output_format in current_fixture["expectations"]["output"]["tersmu"]
    }
    if baseline_formats != current_formats:
        raise VerificationError("the set of tersmu rendering expectations changed")
    for output_format in sorted(baseline_formats - {"json"}):
        baseline_rendering = render_output(
            baseline_binary, baseline_fixture, baseline_lojban, output_format
        )
        if not expectation_matches(
            expected_output(baseline_fixture, output_format), baseline_rendering
        ):
            raise VerificationError(
                f"exact-main {output_format} output does not match the baseline expectation"
            )
        if check_current_expectations:
            current_rendering = render_output(
                current_binary, current_fixture, current_lojban, output_format
            )
            if not expectation_matches(
                expected_output(current_fixture, output_format), current_rendering
            ):
                raise VerificationError(
                    f"candidate {output_format} output does not match the working-tree expectation"
                )
    return verify_mechanical_change(
        baseline_graph, current_graph, allow_topic_retargets
    )


def main() -> int:
    args = parse_args()
    try:
        root = repository_root()
    except VerificationError as error:
        print(f"FLAGGED repository: {error}")
        print("SUMMARY confirmed-mechanical=0 flagged=1 manually-reviewed=0")
        return 1
    confirmed = 0
    flagged = 0
    manually_reviewed = 0
    manual_paths = {path.resolve() for path in args.manually_reviewed}
    for path in args.fixtures:
        manual = path.resolve() in manual_paths
        try:
            topics, transitions, object_delta, retargets = verify_fixture(
                path,
                args.baseline_bin,
                args.current_bin,
                args.baseline_revision,
                root,
                args.check_current_expectations,
                manual,
            )
        except (OSError, KeyError, TypeError, VerificationError) as error:
            flagged += 1
            print(f"FLAGGED {path}: {error}")
            continue
        if retargets:
            flagged += 1
            manually_reviewed += 1
            print(
                f"MANUALLY-REVIEWED {path}: topic-relations={topics} "
                f"paragraph-transitions={transitions} object-delta={object_delta} "
                f"reviewed-reference-retargets={len(retargets)}"
            )
            for retarget in retargets:
                print(f"  RETARGET {retarget}")
        else:
            confirmed += 1
            print(
                f"CONFIRMED {path}: topic-relations={topics} "
                f"paragraph-transitions={transitions} object-delta={object_delta}"
            )
    print(
        f"SUMMARY confirmed-mechanical={confirmed} flagged={flagged} "
        f"manually-reviewed={manually_reviewed}"
    )
    return 0 if flagged == manually_reviewed else 1


if __name__ == "__main__":
    sys.exit(main())
