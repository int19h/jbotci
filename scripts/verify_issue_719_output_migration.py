#!/usr/bin/env python3
"""Issue #719: prove the regenerated SFN goldens differ from the PR BASE
(origin/main) only by the intended mechanical output-shape changes.

XML goldens (xml_corpus/*.xml.txt, xml_focused_regressions/*/*.xml.txt):
  1. the structured `<KEY><RULE TOPIC=...>` block becomes a single XML comment
     before the root element (same teaching prose; the only allowed prose
     edits are the enumerated kind-composition/grouping/connectives rules and
     the compound-assumptions card rule);
  2. the `<WAIVERS>` block is gone;
  3. `<CONNECTOR>` elements are gone; a truth table the parent operator does
     not already determine moves to a TRUTH-TABLE= attribute on the parent,
     and a connective-question parameter to a PARAMETER= attribute — and only
     a parent that actually had a CONNECTOR child may gain those attributes;
  4. tanru head-AND-link scaffolding is replaced by the projected
     KIND-COMPOSITION relation expression. A replaceable region is exactly an
     element whose own CONNECTOR child spells the old implicit sentinel
     (SOURCE-WORD="tanru") — nothing else in the document may change, and the
     replacement subtree must carry a KIND-COMPOSITION (projected) or the loud
     sidecar form. Semantic preservation of those regions is proven
     separately by the rendered-surface re-expansion acceptance test.
  5. `TARGET-FOCUS="SELBRI"` / `"BRIDI"` become `"PREDICATE"` / `"CLAUSE"`.

smusni goldens (phaseb_corpus/*.smusni.txt, *.smusni-prov.txt): both profiles
are parsed into declaration trees and compared structurally (no whitespace or
brace erasure). Allowed transformations: drop CONNECTIVE SOURCE / LOCUS /
RELATION: tanru fields and the WAIVED block; drop TRUTH TABLE fields whose
table is the operator's canonical table; rename TARGET FOCUS values; and,
provenance profile only, CONNECTIVE SOURCE: tanru becomes CONNECTIVE SOURCE:
IMPLICIT JUXTAPOSITION, LOCUS values are renamed to English, and every
remaining CONNECTIVE SOURCE field gains an immediately following LOCUS field
whose value must come from the document's own connector loci (count-checked).
Anything else fails and is flagged for individual manual review.
"""

from __future__ import annotations

import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

BASE_REF = subprocess.run(
    ["git", "merge-base", "origin/main", "HEAD"],
    capture_output=True,
    text=True,
    check=True,
).stdout.strip()

REQUIRED_NEW_PARAGRAPH_MARKERS = {
    "kind-composition": ["KIND-COMPOSITION in the relation slot", "not intersection"],
    "compact-relation": ["PARTICIPANT-PLACE=", "BODY wraps a composite operand"],
    "grouping": ["silence means ASSUMED-LEFT", "EXPLICIT"],
    "connectives": ["TRUTH-TABLE=", "provenance"],
    "compound-assumptions": ["cards state ASSUMED-LEFT"],
}
REPLACED_OLD_TOPICS = {"kind-composition", "compound-assumptions"}

CANONICAL_TABLES = {"TFFF", "TTTF", "TFFT", "TTFF"}
CANONICAL_TABLES_SPACED = {"T F F F", "T T T F", "T F F T", "T T F F"}

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
    "BARE JAI RAISED PARTICIPANT": "BARE RAISED PARTICIPANT",
}
TARGET_FOCUS_RENAMES = {"SELBRI": "PREDICATE", "BRIDI": "CLAUSE"}


class Flag(Exception):
    pass


def git_base(path: Path) -> str:
    result = subprocess.run(
        ["git", "show", f"{BASE_REF}:{path.as_posix()}"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise Flag(f"{path}: not present at the PR base {BASE_REF[:10]}")
    return result.stdout


# --------------------------------------------------------------------------
# XML verification
# --------------------------------------------------------------------------


def split_key(text: str) -> tuple[list[str], list[str], str]:
    topics = re.findall(r'<RULE TOPIC="([^"]+)">', text)
    rules = re.findall(r'<RULE TOPIC="[^"]+">(.*?)</RULE>', text, re.DOTALL)
    body = re.sub(r"  <KEY>.*?</KEY>\n", "", text, count=1, flags=re.DOTALL)
    body = re.sub(r"  <WAIVERS>.*?</WAIVERS>\n", "", body, count=1, flags=re.DOTALL)
    return topics, rules, body


def split_comment(text: str) -> tuple[list[str], str]:
    match = re.match(r"<!--\n(.*?)\n-->\n", text, re.DOTALL)
    if not match:
        raise Flag("document does not start with the KEY comment")
    return match.group(1).split("\n\n"), text[match.end():]


def verify_comment(path: Path, old: str, new: str) -> str:
    topics, rules, old_body = split_key(old)
    paragraphs, new_body = split_comment(new)
    if not paragraphs or paragraphs[0] != (
        "SFN KEY (notation version 0): teaching text for this document. "
        "Defaults stated here are commitments, not omissions."
    ):
        raise Flag(f"{path}: KEY comment does not start with the header paragraph")
    old_by_topic = dict(zip(topics, rules))
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
        raise Flag(f"{path}: KEY comment missing updated rule paragraphs: {sorted(missing)}")
    expected = [text for topic, text in old_by_topic.items() if topic not in REPLACED_OLD_TOPICS]
    if remainder != expected:
        raise Flag(
            f"{path}: KEY comment prose drift beyond the allowed rule updates: "
            f"{len(remainder)} vs {len(expected)} paragraphs differ"
        )
    return old_body, new_body


def canonical_table(operator: str | None) -> str | None:
    return {"AND": "TFFF", "OR": "TTTF", "IFF": "TFFT", "WHETHER_OR_NOT": "TTFF"}.get(
        operator or ""
    )


def account_connector(connector: ET.Element, new: ET.Element, path: str) -> None:
    """A removed CONNECTOR keeps only what the parent operator does not
    already determine, and exactly that: TRUTH-TABLE= appears iff this base
    connector carried a non-derivable table, PARAMETER= iff it carried a
    parameter — anything else added to the parent is rejected."""
    table = connector.find("TRUTH-TABLE")
    table_value = table.get("VALUE") if table is not None else None
    parameter = connector.find("PARAMETER")
    parameter_ref = parameter.get("REF") if parameter is not None else None
    expected_table = (
        table_value
        if table_value is not None and canonical_table(new.get("OPERATOR")) != table_value
        else None
    )
    if new.get("TRUTH-TABLE") != expected_table:
        raise Flag(
            f"{path}: TRUTH-TABLE mismatch on {new.tag}: expected "
            f"{expected_table!r} from the base CONNECTOR, found {new.get('TRUTH-TABLE')!r}"
        )
    if new.get("PARAMETER") != parameter_ref:
        raise Flag(
            f"{path}: PARAMETER mismatch on {new.tag}: expected "
            f"{parameter_ref!r} from the base CONNECTOR, found {new.get('PARAMETER')!r}"
        )


def is_tanru_region(element: ET.Element) -> bool:
    """Exactly an element that carries the old implicit-sentinel CONNECTOR
    itself (direct child), i.e. the tanru head-AND formula (or its FORMULA
    wrapper). Nothing larger may be treated as replaceable."""
    return any(
        child.tag == "CONNECTOR" and child.get("SOURCE-WORD") == "tanru"
        for child in element
    )


def verify_element(old: ET.Element, new: ET.Element, path: str, connector_provenance: bool = False) -> None:
    """Recursive structure comparison with the allowed transformations.
    `connector_provenance` is true when an enclosing FORMULA unwrap already
    accounted this element's connector-derived attributes."""
    if is_tanru_region(old):
        raise Flag(f"{path}: internal error: tanru region reached in strict comparison")
    if old.tag != new.tag:
        # With the CONNECTOR gone, a FORMULA that only wrapped a connective
        # and its connector collapses to the bare connective (jbotci#719).
        if old.tag == "FORMULA":
            unwrapped = [child for child in old if child.tag != "CONNECTOR"]
            if len(unwrapped) == 1 and unwrapped[0].tag == new.tag:
                if old.attrib:
                    raise Flag(f"{path}: FORMULA unwrap lost attributes {dict(old.attrib)}")
                for connector in old:
                    if connector.tag == "CONNECTOR":
                        account_connector(connector, new, path)
                verify_element(unwrapped[0], new, path, bool(
                    [child for child in old if child.tag == "CONNECTOR"]
                ))
                return
        raise Flag(f"{path}: element mismatch <{old.tag}> vs <{new.tag}>")
    connectors = [child for child in old if child.tag == "CONNECTOR"]
    for name, value in new.attrib.items():
        if name in ("TRUTH-TABLE", "PARAMETER") and name not in old.attrib:
            # Provenance-locked: only a parent that had a CONNECTOR child may
            # gain connector-derived attributes (values checked below).
            if not connectors and not connector_provenance:
                raise Flag(
                    f"{path}: {name}={value} added without a CONNECTOR on the base element"
                )
            continue
        if old.attrib.get(name) == value:
            continue
        # TARGET-FOCUS English rename (jbotci#719 addendum).
        if name == "TARGET-FOCUS" and TARGET_FOCUS_RENAMES.get(old.attrib.get(name)) == value:
            continue
        raise Flag(f"{path}: attribute {name} changed: {old.attrib.get(name)} -> {value}")
    for name in old.attrib:
        if name not in new.attrib:
            raise Flag(f"{path}: attribute {name} removed")
    for connector in connectors:
        account_connector(connector, new, path)
    old_children = [child for child in old if child.tag != "CONNECTOR"]
    new_children = list(new)
    index_old = 0
    index_new = 0
    while index_old < len(old_children) and index_new < len(new_children):
        old_child = old_children[index_old]
        new_child = new_children[index_new]
        if is_tanru_region(old_child):
            # The whole tanru AND element (or its FORMULA wrapper) is the only
            # replaceable region; the replacement must carry the composition.
            if new_child.find(".//KIND-COMPOSITION") is None:
                raise Flag(
                    f"{path}: tanru region replaced by a subtree without KIND-COMPOSITION"
                )
            index_old += 1
            index_new += 1
            continue
        verify_element(old_child, new_child, path)
        index_old += 1
        index_new += 1
    if index_old != len(old_children) or index_new != len(new_children):
        raise Flag(
            f"{path}: child count differs under <{old.tag}>: "
            f"{len(old_children)} -> {len(new_children)}"
        )
    if (old.text or "").strip() != (new.text or "").strip():
        raise Flag(f"{path}: text content differs under <{old.tag}>")


def verify_xml_document(path: Path, old: str, new: str) -> str:
    old_body, new_body = verify_comment(path, old, new)
    old_root = ET.fromstring(old_body)
    new_root = ET.fromstring(new_body)
    if old_root.tag != new_root.tag or old_root.attrib != new_root.attrib:
        raise Flag(f"{path}: root element mismatch")
    verify_element(old_root, new_root, path.as_posix())
    return "tanru-regions" if old_root.find('.//CONNECTOR[@SOURCE-WORD="tanru"]') is not None else "clean"


def verify_xml() -> dict[str, int]:
    outcomes = {"clean": 0, "tanru-regions": 0}
    paths = sorted(Path("crates/jbotci-semantics/tests/xml_corpus").glob("*.xml.txt"))
    paths += sorted(
        Path("crates/jbotci-semantics/tests/xml_focused_regressions").glob("*/*.xml.txt")
    )
    assert len(paths) == 52, f"expected 52 XML goldens, found {len(paths)}"
    for path in paths:
        outcome = verify_xml_document(path, git_base(path), path.read_text(encoding="utf-8"))
        outcomes[outcome] += 1
    return outcomes


# --------------------------------------------------------------------------
# smusni verification: a small declaration-tree parser
# --------------------------------------------------------------------------


class SField:
    __slots__ = ("name", "value")

    def __init__(self, name: str, value: str):
        self.name = name
        self.value = value

    def __eq__(self, other):
        return isinstance(other, SField) and (self.name, self.value) == (other.name, other.value)

    def __repr__(self):
        return f"{self.name}: {self.value};"


class SGroup:
    __slots__ = ("head", "items", "closer")

    def __init__(self, head: str, closer: str, items: list):
        self.head = head
        self.closer = closer
        self.items = items

    def __eq__(self, other):
        return (
            isinstance(other, SGroup)
            and (self.head, self.closer, self.items) == (other.head, other.closer, other.items)
        )

    def __repr__(self):
        return f"{self.head} {self.closer[0]}...{self.closer[1]}"


def smusni_tokens(text: str) -> list[str]:
    """Tokenize into braces, parens, semicolons, quoted strings, and atoms."""
    tokens = []
    index = 0
    while index < len(text):
        char = text[index]
        if char.isspace():
            index += 1
        elif char in "{}();:":
            tokens.append(char)
            index += 1
        elif char == '"':
            end = index + 1
            while end < len(text):
                if text[end] == "\\":
                    end += 2
                elif text[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
            tokens.append(text[index:end])
            index = end
        else:
            end = index
            while end < len(text) and not text[end].isspace() and text[end] not in '{}();:"':
                end += 1
            tokens.append(text[index:end])
            index = end
    return tokens


def parse_smusni_items(tokens: list[str], position: int, closers: tuple[str, ...]) -> tuple[list, int]:
    """Parse a sequence of fields/groups/collections/bare entries."""
    items = []
    head_words: list[str] = []
    while position < len(tokens):
        token = tokens[position]
        if token in closers:
            if head_words:
                items.append(" ".join(head_words))
            return items, position
        if token in ("}", ")"):
            raise Flag(f"unbalanced closer {token} at token {position}: {tokens[max(0, position - 14):position + 4]}")
        if token == ";":
            if head_words:
                items.append(" ".join(head_words))
                head_words = []
            position += 1
            continue
        if token == "{":
            body, position = parse_smusni_items(tokens, position + 1, ("}",))
            items.append(SGroup(" ".join(head_words), "{}", body))
            head_words = []
            position += 1
            if position < len(tokens) and tokens[position] == ";":
                # The `NAME = { ... };` facet-field form: the group is the
                # value; consume its trailing semicolon.
                position += 1
            continue
        if token == "(":
            body, position = parse_smusni_items(tokens, position + 1, (")",))
            items.append(SGroup(" ".join(head_words), "()", body))
            head_words = []
            position += 1
            if position < len(tokens) and tokens[position] == ";":
                position += 1
            continue
        if token == ":" and head_words:
            # A field (`NAME: value;`) ends with `;` before any group opener;
            # an indexed group entry (`[1]: RESTRICTIVE { ... }`) hits `{` or
            # `(` first, so the colon is head text, not a field separator.
            lookahead = position + 1
            is_field = True
            while lookahead < len(tokens) and tokens[lookahead] != ";":
                if tokens[lookahead] in ("{", "("):
                    is_field = False
                    break
                lookahead += 1
            if is_field:
                name = " ".join(head_words)
                head_words = []
                value_tokens = []
                position += 1
                while position < len(tokens) and tokens[position] != ";":
                    value_tokens.append(tokens[position])
                    position += 1
                if position >= len(tokens):
                    raise Flag(f"unterminated field {name}")
                items.append(SField(name, " ".join(value_tokens)))
                position += 1
                continue
            head_words.append(token)
            position += 1
            continue
        head_words.append(token)
        position += 1
    return items, position


def parse_smusni(text: str) -> list:
    items, position = parse_smusni_items(smusni_tokens(text), 0, ())
    if position != len(smusni_tokens(text)):
        raise Flag("smusni parse did not consume the document")
    return items


def transform_smusni_tree(items: list, provenance: bool) -> list:
    """Apply the mechanical #719 smusni transformation to a parsed tree."""
    result = []
    for item in items:
        if isinstance(item, SField):
            if item.name == "CONNECTIVE SOURCE":
                if provenance:
                    value = "IMPLICIT JUXTAPOSITION" if item.value == "tanru" else item.value
                    result.append(SField(item.name, value))
                continue
            if item.name == "LOCUS":
                if provenance:
                    renamed = LOCUS_RENDER_RENAMES.get(item.value, item.value)
                    result.append(SField(item.name, renamed))
                continue
            if item.name == "TRUTH TABLE" and item.value in CANONICAL_TABLES_SPACED:
                continue
            if item.name == "RELATION" and item.value == "tanru":
                continue
            if item.name == "TARGET FOCUS" and item.value in TARGET_FOCUS_RENAMES:
                result.append(SField(item.name, TARGET_FOCUS_RENAMES[item.value]))
                continue
            result.append(item)
        elif isinstance(item, SGroup):
            if item.head == "WAIVED":
                continue
            result.append(SGroup(item.head, item.closer, transform_smusni_tree(item.items, provenance)))
        else:
            result.append(item)
    return result


INLINEABLE_HEADS = {"TANRU LINK", "PROVENANCE"}


def expand_inlineable(items: list) -> list:
    """The writer collapses TANRU LINK and PROVENANCE subheadings to a bare
    field sequence or wraps them in braces purely as a function of the host
    declaration's direct-field count, so removing the RELATION: tanru field
    legitimately flips that layout. Accept either form at the same position
    with an identical item sequence — and nothing else."""
    expanded = []
    for item in items:
        if (
            isinstance(item, SGroup)
            and item.head in INLINEABLE_HEADS
            and item.items
            and isinstance(item.items[0], SField)
        ):
            # The flat layout folds the heading into the first field name:
            # TANRU LINK { HEAD: p; M: q; } ≡ TANRU LINK HEAD: p; M: q;
            first = item.items[0]
            expanded.append(SField(f"{item.head} {first.name}", first.value))
            expanded.extend(item.items[1:])
        else:
            expanded.append(item)
    return expanded


def compare_smusni_trees(old: list, new: list, path: str, valid_loci: set[str]) -> None:
    """Exact structural comparison, allowing precisely the provenance-profile
    LOCUS insertions (each must immediately follow a CONNECTIVE SOURCE field
    and take a value from the document's own connector loci)."""
    old = expand_inlineable(old)
    new = expand_inlineable(new)
    old_index = 0
    new_index = 0
    previous_new_field = None
    while old_index < len(old) and new_index < len(new):
        old_item = old[old_index]
        new_item = new[new_index]
        if old_item == new_item:
            old_index += 1
            new_index += 1
            previous_new_field = new_item
            continue
        if (
            isinstance(old_item, SGroup)
            and isinstance(new_item, SGroup)
            and old_item.head == new_item.head
            and old_item.closer == new_item.closer
        ):
            compare_smusni_trees(old_item.items, new_item.items, path, valid_loci)
            old_index += 1
            new_index += 1
            previous_new_field = new_item
            continue
        if (
            isinstance(old_item, SGroup)
            and isinstance(new_item, SGroup)
            and old_item.head in INLINEABLE_HEADS
            and new_item.head in INLINEABLE_HEADS
            and old_item.items == new_item.items
        ):
            old_index += 1
            new_index += 1
            previous_new_field = new_item
            continue
        if (
            isinstance(new_item, SField)
            and new_item.name == "LOCUS"
            and isinstance(previous_new_field, SField)
            and previous_new_field.name == "CONNECTIVE SOURCE"
        ):
            if new_item.value not in valid_loci:
                raise Flag(f"{path}: added LOCUS value {new_item.value!r} not in document loci")
            new_index += 1
            previous_new_field = new_item
            continue
        raise Flag(f"{path}: structural mismatch: {old_item!r} vs {new_item!r}")
    for trailing in new[new_index:]:
        if not (
            isinstance(trailing, SField)
            and trailing.name == "LOCUS"
            and isinstance(previous_new_field, SField)
            and previous_new_field.name == "CONNECTIVE SOURCE"
            and trailing.value in valid_loci
        ):
            raise Flag(f"{path}: trailing item {trailing!r}")
        previous_new_field = trailing
    if old_index != len(old):
        raise Flag(f"{path}: items missing at the end: {old[old_index]!r}")


def collect_loci(items: list, into: set[str]) -> None:
    for item in items:
        if isinstance(item, SField) and item.name == "LOCUS":
            renamed = LOCUS_RENDER_RENAMES.get(item.value, item.value)
            into.add(renamed)
        elif isinstance(item, SGroup):
            collect_loci(item.items, into)


JSON_LOCUS_TO_RENDER = {
    "statement": "STATEMENT",
    "sumti": "ARGUMENT",
    "term": "TERM",
    "termset": "TERM SET",
    "tense": "TENSE",
    "modal": "TAG",
    "modal-argument": "TAG",
    "operand": "OPERAND",
    "mekso-operand": "OPERAND",
    "bridi": "CLAUSE",
    "bridiTail": "PREDICATE PHRASE",
    "selbri": "PREDICATE",
    "selbri-inversion": "PREDICATE INVERSION",
    "tanru-unit": "PREDICATE UNIT",
    "property-abstraction": "PROPERTY ABSTRACTION",
    "property-inversion": "PROPERTY INVERSION",
    "abstraction": "ABSTRACTION",
    "description": "DESCRIPTION",
    "mekso-operator": "MATH OPERATOR",
    "bare-jai-raised-participant": "BARE RAISED PARTICIPANT",
}


def expected_locus_multiset(path: Path) -> "collections.Counter":
    """The LOCUS values the document's own connectors must produce, derived
    from the BASE frozen graph's connector loci (never from the new output —
    a self-authorizing set proves nothing)."""
    import collections
    import json

    frozen_name = path.name.replace(".smusni-prov.txt", ".frozen.json").replace(
        ".smusni.txt", ".frozen.json"
    )
    frozen = json.loads(git_base(path.with_name(frozen_name)))
    expected = collections.Counter()

    def walk(value):
        if isinstance(value, dict):
            connector = value.get("connector")
            if isinstance(connector, dict) and isinstance(connector.get("locus"), str):
                locus = connector["locus"]
                if locus not in JSON_LOCUS_TO_RENDER:
                    raise Flag(f"{path}: unmapped base connector.locus {locus!r}")
                expected[JSON_LOCUS_TO_RENDER[locus]] += 1
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)

    walk(frozen)
    return expected


def actual_locus_multiset(items: list, into: "collections.Counter") -> None:
    import collections

    for item in items:
        if isinstance(item, SField) and item.name == "LOCUS":
            into[item.value] += 1
        elif isinstance(item, SGroup):
            actual_locus_multiset(item.items, into)


def verify_smusni() -> int:
    import collections

    paths = sorted(Path("crates/jbotci-semantics/tests/phaseb_corpus").glob("*.smusni*.txt"))
    assert len(paths) == 100, f"expected 100 smusni goldens, found {len(paths)}"
    for path in paths:
        provenance = path.name.endswith("-prov.txt")
        old = parse_smusni(git_base(path))
        new = parse_smusni(path.read_text(encoding="utf-8"))
        transformed = transform_smusni_tree(old, provenance)
        if transformed == new:
            continue
        if not provenance:
            raise Flag(f"{path}: default-profile delta is not the mechanical transformation")
        # Base-derived expected values: every added LOCUS must come from the
        # base graph's own connector loci, and the multiset of rendered LOCUS
        # fields must equal the base connector multiset exactly (value AND
        # connector association, not a self-authorizing set).
        expected = expected_locus_multiset(path)
        actual: "collections.Counter" = collections.Counter()
        actual_locus_multiset(new, actual)
        if actual != expected:
            raise Flag(
                f"{path}: rendered LOCUS multiset differs from the base "
                f"connector loci: expected {dict(expected)}, found {dict(actual)}"
            )
        compare_smusni_trees(transformed, new, path, set(expected))
    return len(paths)


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
