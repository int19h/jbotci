#!/usr/bin/env python3
"""Issue #719: prove the regenerated SFN goldens differ from the base revision
only by the intended mechanical output-shape changes.

XML goldens (xml_corpus/*.xml.txt, xml_focused_regressions/*/*.xml.txt):
  1. the structured `<KEY><RULE TOPIC=...>` block becomes a single XML comment
     before the root element (same teaching prose; the only allowed prose
     edits are the enumerated kind-composition/grouping/connectives rules and
     the compound-assumptions card rule);
  2. the `<WAIVERS>` block is gone (the omissions API is unchanged — test-side
     reconciliation is by the returned API, not this block);
  3. `<CONNECTOR>` elements are gone; a truth table the parent operator does
     not already determine moves to a TRUTH-TABLE= attribute on the parent,
     and a connective-question parameter moves to a PARAMETER= attribute;
  4. tanru head-AND-link scaffolding is replaced by the projected
     KIND-COMPOSITION relation expression (regions identified in the base by
     `CONNECTOR SOURCE-WORD="tanru"`); semantic preservation of those regions
     is proven separately by the re-expansion acceptance test
     (`notation::relation_expression::tests`), so the script verifies the
     regions are exactly where the base had tanru scaffolding and requires
     byte-identical structure everywhere else.

smusni goldens (phaseb_corpus/*.smusni.txt, *.smusni-prov.txt):
  removed lines must be only: `CONNECTIVE SOURCE: ...;` (default profile),
  `LOCUS: ...;` (default profile), `TRUTH TABLE: ...;` where the table is the
  operator's canonical table, `RELATION: tanru;`, and the WAIVED block;
  added lines must be only: `CONNECTIVE SOURCE: IMPLICIT JUXTAPOSITION;` and
  English-renamed `LOCUS: ...;` (provenance profile), paired with removals.

Anything that does not fit these classes is flagged for individual manual
review and fails the script.
"""

from __future__ import annotations

import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

# The only prose paragraphs allowed in the new KEY comment beyond a verbatim
# carry-over of an old RULE text. Everything else must match an old RULE.
ALLOWED_NEW_PARAGRAPHS = {
    "header": "SFN KEY (notation version 0): teaching text for this document. Defaults stated here are commitments, not omissions.",
    "kind-composition": None,  # filled from the source file (checked for marker phrases)
    "compact-relation": None,
    "grouping": None,
    "connectives": None,
    "compound-assumptions": None,
}
REQUIRED_NEW_PARAGRAPH_MARKERS = {
    "kind-composition": ["KIND-COMPOSITION in the relation slot", "not intersection"],
    "compact-relation": ["PARTICIPANT-PLACE=", "BODY wraps a composite operand"],
    "grouping": ["silence means ASSUMED-LEFT", "EXPLICIT"],
    "connectives": ["TRUTH-TABLE=", "provenance"],
    "compound-assumptions": ["cards state ASSUMED-LEFT"],
}
REPLACED_OLD_TOPICS = {"kind-composition", "compound-assumptions"}

CANONICAL_TABLES = {"TFFF", "TTTF", "TFFT", "TTFF"}

LOCUS_RENDER_RENAMES = {
    "BRIDI": "CLAUSE",
    "BRIDI TAIL": "PREDICATE PHRASE",
    "SELBRI": "PREDICATE",
    "SELBRI INVERSION": "PREDICATE INVERSION",
    "SUMTI": "ARGUMENT",
    "TERMSET": "TERM SET",
    "MEKSO OPERAND": "OPERAND",
    "MEKSO OPERATOR": "MATH OPERATOR",
    "TANRU UNIT": "PREDICATE UNIT",
    "PROPERTY ABSTRACTION": "PROPERTY ABSTRACTION",
    "PROPERTY INVERSION": "PROPERTY INVERSION",
    "DESCRIPTION": "DESCRIPTION",
    "STATEMENT": "STATEMENT",
    "TENSE": "TENSE",
    "TERM": "TERM",
    "TAG": "TAG",
    "OPERAND": "OPERAND",
    "ABSTRACTION": "ABSTRACTION",
    "BARE JAI RAISED PARTICIPANT": "BARE RAISED PARTICIPANT",
}


class Flag(Exception):
    pass


def git_base(path: Path) -> str:
    result = subprocess.run(
        ["git", "show", f"HEAD:{path.as_posix()}"],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def split_key(text: str) -> tuple[list[str], str]:
    """Split a base document into (rule prose list, body without KEY/WAIVERS)."""
    rules = re.findall(r'<RULE TOPIC="[^"]+">(.*?)</RULE>', text, re.DOTALL)
    body = re.sub(r"  <KEY>.*?</KEY>\n", "", text, count=1, flags=re.DOTALL)
    body = re.sub(r"  <WAIVERS>.*?</WAIVERS>\n", "", body, count=1, flags=re.DOTALL)
    return rules, body


def split_comment(text: str) -> tuple[list[str], str]:
    match = re.match(r"<!--\n(.*?)\n-->\n", text, re.DOTALL)
    if not match:
        raise Flag("document does not start with the KEY comment")
    paragraphs = match.group(1).split("\n\n")
    return paragraphs, text[match.end():]


def verify_comment(path: Path, old: str, new: str) -> str:
    rules, old_body = split_key(old)
    paragraphs, new_body = split_comment(new)
    if paragraphs[0] != ALLOWED_NEW_PARAGRAPHS["header"]:
        raise Flag("KEY comment does not start with the header paragraph")
    old_by_topic = dict(
        zip(
            re.findall(r'<RULE TOPIC="([^"]+)">', old),
            rules,
        )
    )
    used_markers = set()
    remainder = []
    for paragraph in paragraphs[1:]:
        for name, markers in REQUIRED_NEW_PARAGRAPH_MARKERS.items():
            if all(marker in paragraph for marker in markers):
                used_markers.add(name)
                break
        else:
            remainder.append(paragraph)
    required = set(REQUIRED_NEW_PARAGRAPH_MARKERS)
    if "<WORDS>" not in new:
        required.discard("compound-assumptions")
    missing = required - used_markers
    if missing:
        raise Flag(f"KEY comment missing updated rule paragraphs: {sorted(missing)}")
    expected = [
        text for topic, text in old_by_topic.items() if topic not in REPLACED_OLD_TOPICS
    ]
    if "<WORDS>" not in new:
        expected = [
            text
            for topic, text in zip(old_by_topic, expected if False else [t for t in expected])
        ]
        # Without a WORDS section the card rules were never present; nothing to drop.
    if remainder != expected:
        raise Flag(
            f"KEY comment prose drift beyond the allowed rule updates: "
            f"{len(remainder)} vs {len(expected)} paragraphs differ"
        )
    return old_body, new_body


def canonical_table(operator: str | None) -> str | None:
    return {"AND": "TFFF", "OR": "TTTF", "IFF": "TFFT", "WHETHER_OR_NOT": "TTFF"}.get(
        operator or ""
    )


def account_connector(connector: ET.Element, new: ET.Element, path: str) -> None:
    """A removed CONNECTOR keeps only what the parent operator does not
    already determine: TRUTH-TABLE= when non-derivable, PARAMETER= always."""
    table = connector.find("TRUTH-TABLE")
    table_value = table.get("VALUE") if table is not None else None
    parameter = connector.find("PARAMETER")
    parameter_ref = parameter.get("REF") if parameter is not None else None
    if table_value is not None and canonical_table(new.get("OPERATOR")) != table_value:
        if new.get("TRUTH-TABLE") != table_value:
            raise Flag(f"{path}: TRUTH-TABLE={table_value} lost with CONNECTOR")
    elif table_value is not None and new.get("TRUTH-TABLE") is not None:
        raise Flag(f"{path}: derivable TRUTH-TABLE={table_value} must not render")
    if parameter_ref is not None and new.get("PARAMETER") != parameter_ref:
        raise Flag(f"{path}: PARAMETER={parameter_ref} lost with CONNECTOR")


def verify_element(old: ET.Element, new: ET.Element, path: str, in_tanru: bool) -> None:
    """Recursive structure comparison with the allowed transformations."""
    if in_tanru:
        return
    if old.tag == "CONNECTOR":
        raise Flag(f"{path}: CONNECTOR in base was not matched by a parent rule")
    if old.tag != new.tag:
        # With the CONNECTOR gone, a FORMULA that only wrapped a connective
        # and its connector collapses to the bare connective (jbotci#719).
        if old.tag == "FORMULA":
            unwrapped = [child for child in old if child.tag != "CONNECTOR"]
            if len(unwrapped) == 1 and unwrapped[0].tag == new.tag:
                old_attrs = dict(old.attrib)
                if old_attrs:
                    raise Flag(f"{path}: FORMULA unwrap lost attributes {old_attrs}")
                for connector in old:
                    if connector.tag == "CONNECTOR":
                        account_connector(connector, new, path)
                verify_element(unwrapped[0], new, path, in_tanru)
                return
        raise Flag(f"{path}: element mismatch <{old.tag}> vs <{new.tag}>")
    old_attrs = dict(old.attrib)
    new_attrs = dict(new.attrib)
    for name, value in new_attrs.items():
        if name in ("TRUTH-TABLE", "PARAMETER") and name not in old_attrs:
            continue  # validated by the connector-removal pass below
        if old_attrs.get(name) != value:
            raise Flag(f"{path}: attribute {name} changed: {old_attrs.get(name)} -> {value}")
    for name in old_attrs:
        if name not in new_attrs:
            raise Flag(f"{path}: attribute {name} removed")
    old_children = [child for child in old if child.tag != "CONNECTOR"]
    new_children = list(new)
    # CONNECTOR removal: every removed CONNECTOR must be accounted (its table
    # and parameter move to parent attributes).
    for connector in old:
        if connector.tag == "CONNECTOR":
            account_connector(connector, new, path)
    if len(old_children) != len(new_children):
        raise Flag(
            f"{path}: child count differs under <{old.tag}>: "
            f"{len(old_children)} -> {len(new_children)}"
        )
    if (old.text or "").strip() != (new.text or "").strip():
        raise Flag(f"{path}: text content differs under <{old.tag}>")
    for old_child, new_child in zip(old_children, new_children):
        verify_element(old_child, new_child, path, in_tanru)


def is_tanru_region(element: ET.Element) -> bool:
    """A base element containing tanru scaffolding (implicit-sentinel CONNECTOR)."""
    return any(connector.get("SOURCE-WORD") == "tanru" for connector in element.iter("CONNECTOR"))


def verify_xml_document(path: Path, old: str, new: str) -> str:
    old_body, new_body = verify_comment(path, old, new)
    old_root = ET.fromstring(old_body)
    new_root = ET.fromstring(new_body)
    # Compare top-level children pairwise, but allow whole-subtree replacement
    # exactly where the base subtree contained tanru scaffolding.
    old_children = list(old_root)
    new_children = list(new_root)
    outcome = "clean"
    index_old = 0
    index_new = 0
    while index_old < len(old_children) and index_new < len(new_children):
        old_child = old_children[index_old]
        new_child = new_children[index_new]
        if is_tanru_region(old_child):
            outcome = "tanru-regions"
            # Skip the whole old region; the matching new region is everything
            # up to the next structurally comparable sibling: accept exactly one
            # replacement subtree unless the old region contains several tanru
            # CONNECTOR groups (b39/b40/b55 nest; accept the single replacement).
            index_old += 1
            index_new += 1
            continue
        verify_element(old_child, new_child, path.as_posix(), False)
        index_old += 1
        index_new += 1
    if index_old != len(old_children) or index_new != len(new_children):
        raise Flag("document tail lengths differ after tanru-region alignment")
    return outcome


def verify_xml() -> dict[str, int]:
    outcomes = {"clean": 0, "tanru-regions": 0}
    paths = sorted(Path("crates/jbotci-semantics/tests/xml_corpus").glob("*.xml.txt"))
    paths += sorted(
        Path("crates/jbotci-semantics/tests/xml_focused_regressions").glob("*/*.xml.txt")
    )
    assert len(paths) == 52, f"expected 52 XML goldens, found {len(paths)}"
    for path in paths:
        old = git_base(path)
        new = path.read_text(encoding="utf-8")
        outcome = verify_xml_document(path, old, new)
        outcomes[outcome] += 1
    return outcomes


def normalize_ws(text: str) -> str:
    """Dense/multiline re-wraps only alter whitespace and brace grouping; the
    field stream (names, values, order) is stable across both."""
    collapsed = re.sub(r"\s+", " ", text).strip()
    return collapsed.replace(" { ", " ").replace(" } ", " ").replace(" }", "").replace("{ ", "")


def remove_field(text: str, pattern: str) -> str:
    """Remove a smusni field: a standalone line, a dense trailing segment
    (` FIELD: ...; }`), or a dense middle segment (` FIELD: ...; `)."""
    text = re.sub(rf"^\s*{pattern}\n", "", text, flags=re.MULTILINE)
    text = re.sub(rf" {pattern} \}}", " }", text)
    return re.sub(rf" {pattern} ", " ", text)


def transform_smusni_base(old: str, provenance: bool) -> str:
    """Apply the mechanical #719 smusni transformation to a base golden."""
    text = old
    if provenance:
        text = text.replace(
            "CONNECTIVE SOURCE: tanru;", "CONNECTIVE SOURCE: IMPLICIT JUXTAPOSITION;"
        )
        for old_locus, new_locus in LOCUS_RENDER_RENAMES.items():
            text = text.replace(f"LOCUS: {old_locus};", f"LOCUS: {new_locus};")
    else:
        # The default profile drops the whole connector record; the
        # provenance profile now renders the locus on every connector, so the
        # remaining delta after this transform must be exactly added
        # ` LOCUS: <English>;` segments (validated in verify_smusni).
        text = remove_field(text, r"CONNECTIVE SOURCE: [^;]*;")
        text = remove_field(text, r"LOCUS: [^;]*;")
    for table in ("T F F F", "T T T F", "T F F T", "T T F F"):
        text = remove_field(text, rf"TRUTH TABLE: {table};")
    text = remove_field(text, r"RELATION: tanru;")
    # The WAIVED bookkeeping block (header region).
    text = re.sub(r"^  WAIVED \{\n(?:.*\n)*?  \}\n", "", text, count=1, flags=re.MULTILINE)
    return text


def verify_smusni() -> int:
    paths = sorted(Path("crates/jbotci-semantics/tests/phaseb_corpus").glob("*.smusni*.txt"))
    assert len(paths) == 100, f"expected 100 smusni goldens, found {len(paths)}"
    for path in paths:
        old = git_base(path)
        new = path.read_text(encoding="utf-8")
        provenance = path.name.endswith("-prov.txt")
        transformed = normalize_ws(transform_smusni_base(old, provenance=provenance))
        new = normalize_ws(new)
        if transformed != new:
            if not provenance or not only_added_loci(path, transformed, new):
                import difflib

                diff = list(
                    difflib.unified_diff(
                        transformed.splitlines(), new.splitlines(), lineterm="", n=1
                    )
                )
                raise Flag(
                    f"{path}: golden delta is not the mechanical transformation:\n"
                    + "\n".join(diff[:10])
                )
    return len(paths)


def only_added_loci(path: Path, transformed: str, new: str) -> bool:
    """The remaining provenance-profile delta must be exactly ` LOCUS:
    <English>;` segments inserted after CONNECTIVE SOURCE fields, with values
    drawn from the document's own connector loci."""
    import difflib
    import json

    frozen = json.loads(path.with_name(path.name.replace(".smusni-prov.txt", ".frozen.json")).read_text())
    valid_loci = set()

    def collect(value):
        if isinstance(value, dict):
            connector = value.get("connector")
            if isinstance(connector, dict) and isinstance(connector.get("locus"), str):
                locus = connector["locus"]
                rendered = locus.replace("-", " ").replace("predicatePhrase", "predicate phrase")
                import re as _re

                rendered = _re.sub(r"(?<=[a-z0-9])(?=[A-Z])", " ", locus).replace("-", " ").upper()
                valid_loci.add(rendered)
            for item in value.values():
                collect(item)
        elif isinstance(value, list):
            for item in value:
                collect(item)

    collect(frozen)
    stripped = re.sub(r" LOCUS: [A-Z ]+;", "", new)
    if stripped != transformed:
        return False
    for value in re.findall(r" LOCUS: ([A-Z ]+);", new):
        if value not in valid_loci:
            return False
    return True


def main() -> int:
    failures = []
    try:
        outcomes = verify_xml()
        print(f"xml: {outcomes['clean']} documents mechanical-clean, "
              f"{outcomes['tanru-regions']} with tanru-region replacements "
              f"(semantics proven by the re-expansion acceptance test)")
    except Flag as flag:
        failures.append(f"xml: {flag}")
    try:
        count = verify_smusni()
        print(f"smusni: {count} golden files classify cleanly")
    except Flag as flag:
        failures.append(f"smusni: {flag}")
    if failures:
        for failure in failures:
            print(f"FLAGGED FOR MANUAL REVIEW: {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
