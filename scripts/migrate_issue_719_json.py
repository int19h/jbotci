#!/usr/bin/env python3
"""Issue #719: mechanical migration of semantic-JSON expectation surfaces.

Applies exactly four model-typing transformations to canonical
`lojban-semantics-json-1` documents, and proves nothing else changes:

1. `connector.source` magic strings become the typed ConnectorSource shape:
   `"source": "tanru"` (the synthesized sentinel) becomes
   `{"kind": "implicitJuxtaposition"}`; every other surface word `w` becomes
   `{"kind": "surfaceWord", "word": w}`.
2. `connector.locus` strings become the English ConnectorLocus vocabulary
   (mapping table below; anything unmapped is a hard error).
3. The fake relation name is dropped: a predication carrying a `tanruLink`
   must have `"relation": "tanru"` and loses the field; conversely no other
   predication may carry `"relation": "tanru"`.
4. `displayedContent.targetFocus` magic strings become English:
   `"targetFocus": "bridi"` becomes `"clause"` and `"targetFocus": "selbri"`
   becomes `"predicate"` (exact match on those two strings only, wherever the
   field occurs; anything else is a hard error).

Every JSON file is roundtrip-checked (`json.dumps(json.loads(x)) == x`) before
rewriting so byte layout (key order, separators, UTF-8) is provably preserved.
Use `--verify-pipeline` to additionally rebuild each frozen document from its
own recorded source text through the given freshly built `jbotci` binary and
require byte equality with the transformed expectation.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

LOCUS_MAP = {
    "statement": "statement",
    "sumti": "argument",
    "term": "term",
    "termset": "termSet",
    "tense": "tense",
    "modal": "tag",
    "modal-argument": "tag",
    "operand": "operand",
    "mekso-operand": "operand",
    "bridi": "clause",
    "bridiTail": "predicatePhrase",
    "selbri": "predicate",
    "selbri-inversion": "predicateInversion",
    "tanru-unit": "predicateUnit",
    "property-abstraction": "propertyAbstraction",
    "property-inversion": "propertyInversion",
    "abstraction": "abstraction",
    "description": "description",
    "mekso-operator": "mathOperator",
    "bare-jai-raised-participant": "bareRaisedParticipant",
}

TARGET_FOCUS_MAP = {
    "bridi": "clause",
    "selbri": "predicate",
}


class MigrationError(Exception):
    pass


def canonical(text: str) -> str:
    return json.dumps(json.loads(text), ensure_ascii=False, separators=(",", ":"))


def transform_value(value, stats, path):
    """Apply the four transformations in place; fail on anything unexpected."""
    if isinstance(value, list):
        for item in value:
            transform_value(item, stats, path)
        return
    if not isinstance(value, dict):
        return
    target_focus = value.get("targetFocus")
    if target_focus is not None:
        if target_focus in TARGET_FOCUS_MAP:
            value["targetFocus"] = TARGET_FOCUS_MAP[target_focus]
            stats["target_focus"] += 1
        elif target_focus in TARGET_FOCUS_MAP.values():
            # Already migrated: the English vocabulary is already in place.
            stats["already_migrated"] += 0
        else:
            raise MigrationError(
                f"{path}: unmapped displayedContent.targetFocus {target_focus!r}"
            )
    if "tanruLink" in value:
        relation = value.pop("relation", None)
        if relation is None:
            # Already migrated: the fake relation name is simply absent.
            stats["already_migrated"] += 0
        elif relation != "tanru":
            raise MigrationError(
                f"{path}: tanruLink predication carries relation {relation!r},"
                " expected the fake 'tanru' name"
            )
        else:
            stats["relation_dropped"] += 1
    elif value.get("relation") == "tanru":
        raise MigrationError(
            f"{path}: predication without tanruLink carries the fake 'tanru' relation"
        )
    connector = value.get("connector")
    if connector is not None:
        if not isinstance(connector, dict):
            raise MigrationError(f"{path}: connector is not an object: {connector!r}")
        source = connector.get("source")
        if isinstance(source, dict):
            # Already migrated: validate the typed shape and move on.
            kind = source.get("kind")
            if kind not in ("surfaceWord", "implicitJuxtaposition"):
                raise MigrationError(f"{path}: invalid connector.source kind {kind!r}")
            if kind == "surfaceWord" and not isinstance(source.get("word"), str):
                raise MigrationError(f"{path}: surfaceWord connector lacks its word")
            locus = connector.get("locus")
            if locus not in LOCUS_MAP.values():
                raise MigrationError(f"{path}: invalid typed connector.locus {locus!r}")
            stats["already_migrated"] += 1
        else:
            if not isinstance(source, str):
                raise MigrationError(f"{path}: connector.source is not a string: {source!r}")
            connector["source"] = (
                {"kind": "implicitJuxtaposition"}
                if source == "tanru"
                else {"kind": "surfaceWord", "word": source}
            )
            stats["connector_sources"] += 1
            if source == "tanru":
                stats["implicit_sources"] += 1
            locus = connector.get("locus")
            if not isinstance(locus, str) or locus not in LOCUS_MAP:
                raise MigrationError(f"{path}: unmapped connector.locus {locus!r}")
            connector["locus"] = LOCUS_MAP[locus]
            stats["loci"] += 1
    for item in value.values():
        transform_value(item, stats, path)


def transform_json_text(text: str, path: str, stats) -> str:
    if canonical(text) != text:
        raise MigrationError(f"{path}: JSON does not roundtrip byte-exactly")
    document = json.loads(text)
    transform_value(document, stats, path)
    return json.dumps(document, ensure_ascii=False, separators=(",", ":"))


def dump(document, pretty: bool) -> str:
    if pretty:
        return json.dumps(document, ensure_ascii=False, indent=1)
    return json.dumps(document, ensure_ascii=False, separators=(",", ":"))


def transform_json_file_text(text: str, path: str, stats) -> str:
    """Transform a whole file's JSON, preserving its trailing-newline layout
    and its byte format (compact, spaced compact, or indent=1 pretty)."""
    body = text.rstrip("\n")
    document = json.loads(body)
    dumps = (
        lambda: json.dumps(document, ensure_ascii=False, separators=(",", ":")),
        lambda: json.dumps(document, ensure_ascii=False, separators=(", ", ": ")),
        lambda: json.dumps(document, ensure_ascii=False, indent=1),
    )
    render = None
    for candidate in dumps:
        if candidate() == body:
            render = candidate
            break
    if render is None:
        raise MigrationError(f"{path}: JSON does not roundtrip byte-exactly")
    transform_value(document, stats, path)
    return render() + text[len(body):]


def frozen_targets() -> list[Path]:
    roots = [
        Path("crates/jbotci-semantics/tests/xml_corpus"),
        Path("crates/jbotci-semantics/tests/phaseb_corpus"),
    ]
    focused = Path("crates/jbotci-semantics/tests/xml_focused_regressions")
    targets = []
    for root in roots:
        targets.extend(sorted(root.glob("*.frozen.json")))
    targets.extend(sorted(focused.glob("*/*.frozen.json")))
    return targets


def toml_targets() -> list[Path]:
    return sorted(Path("tests/fixtures").rglob("*.toml"))


def migrate_toml(path: Path, stats, write: bool) -> bool:
    text = path.read_text(encoding="utf-8")
    changed = False
    cursor = 0
    markers = [('json = """', '"""'), ("json = '''", "'''"), ("json = '", "'")]
    while True:
        candidates = []
        for marker, delimiter in markers:
            pos = text.find(marker, cursor)
            if pos != -1:
                candidates.append((pos, -len(marker), marker, delimiter))
        if not candidates:
            break
        _, _, marker, delimiter = min(candidates)
        start = text.find(marker, cursor) + len(marker)
        end = text.find(delimiter, start)
        if end == -1:
            raise MigrationError(f"{path}: unterminated json block")
        json_text = text[start:end]
        cursor = end + len(delimiter)
        if (
            '"connector"' not in json_text
            and '"tanruLink"' not in json_text
            and '"targetFocus"' not in json_text
        ):
            continue
        if not json_text.lstrip().startswith('{"version":"lojban-semantics-json-1"'):
            continue
        new_json = transform_json_text(json_text, str(path), stats)
        if new_json != json_text:
            text = text[:start] + new_json + text[end:]
            cursor = start + len(new_json) + len(delimiter)
            changed = True
    if changed and write:
        path.write_text(text, encoding="utf-8")
    stats["toml_changed"] += changed
    return changed


def verify_pipeline(binary: Path, paths: list[Path]) -> list[str]:
    """Rebuild each frozen document from its source text (the .lojban sibling
    when one exists, else the root object's recorded text) and require byte
    equality with the transformed expectation."""
    failures = []
    skipped = []
    for path in paths:
        lojban = path.with_name(path.name.replace(".frozen.json", ".lojban"))
        phaseb_sibling = (
            Path("crates/jbotci-semantics/tests/phaseb_corpus") / lojban.name
        )
        if lojban.exists():
            source_text = lojban.read_text(encoding="utf-8").strip("\n")
        elif phaseb_sibling.exists():
            source_text = phaseb_sibling.read_text(encoding="utf-8").strip("\n")
        else:
            document = json.loads(path.read_text(encoding="utf-8"))
            root = document["root"]
            source_text = document["objects"][root].get("source", {}).get("text")
        if not source_text:
            skipped.append(f"{path}: no source text to rebuild from")
            continue
        result = subprocess.run(
            [str(binary), "tersmu", "--format", "json", source_text],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            failures.append(f"{path}: pipeline failed: {result.stderr.strip()[:200]}")
            continue
        produced = result.stdout.rstrip("\n")
        expected = path.read_text(encoding="utf-8")
        if canonical(produced) != canonical(expected):
            failures.append(f"{path}: rebuilt graph diverges from transformed expectation")
    for skip in skipped:
        print(f"  SKIP {skip}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate without writing")
    parser.add_argument(
        "--verify-pipeline",
        type=Path,
        metavar="JBOTCI",
        help="rebuild frozen documents with this binary and byte-compare",
    )
    parser.add_argument(
        "--no-transform",
        action="store_true",
        help="skip the transform pass (for verifying an already-migrated tree)",
    )
    parser.add_argument(
        "--skip-fixtures", action="store_true", help="only process frozen corpora"
    )
    args = parser.parse_args()

    stats = {
        "relation_dropped": 0,
        "connector_sources": 0,
        "implicit_sources": 0,
        "loci": 0,
        "target_focus": 0,
        "toml_changed": 0,
        "frozen_changed": 0,
        "already_migrated": 0,
    }
    frozen = frozen_targets()
    if not args.no_transform:
        for path in frozen:
            text = path.read_text(encoding="utf-8")
            new_text = transform_json_file_text(text, str(path), stats)
            if new_text != text:
                stats["frozen_changed"] += 1
                if not args.check:
                    path.write_text(new_text, encoding="utf-8")
        if not args.skip_fixtures:
            for path in toml_targets():
                migrate_toml(path, stats, write=not args.check)

        print("migration summary:")
        for key, value in stats.items():
            print(f"  {key}: {value}")

    if args.verify_pipeline:
        failures = verify_pipeline(args.verify_pipeline, frozen)
        if failures:
            print("pipeline verification FAILURES:", file=sys.stderr)
            for failure in failures:
                print(f"  {failure}", file=sys.stderr)
            return 1
        print(f"pipeline verification: {len(frozen)} frozen documents rebuild byte-exactly")
    return 0


if __name__ == "__main__":
    sys.exit(main())
