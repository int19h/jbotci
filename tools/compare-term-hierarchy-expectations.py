#!/usr/bin/env python3
"""Fail-closed classifier for the epoch-6 term-hierarchy expectation regeneration.

EPOCH 6c RE-BASELINE.  The baseline archive is `ARCHIVE_COMMIT`, the epoch-6b merge and this
epoch's own implementation base, so every class earlier epochs contributed is already applied
in the baseline and must now classify nothing at all; they stay in place because a nonzero
incidence would mean the baseline is not the tree it claims to be.  6c contributes two classes
of its own, `bo-joint-sum-wrapper` and `jai-payload-widening`, and one population that is
listed rather than classified: fixtures this epoch ADDED, which have no baseline entry to
compare against.

Usage (from the repository root, after the consolidated `fixture-rewrite` passes):

    python3 tools/compare-term-hierarchy-expectations.py tests/fixtures \\
        --baseline-root /build/jbotci/scratch/06c/baseline-fixtures

`--baseline-root` is an archive of `tests/fixtures` taken *before* the rewrite; the positional
argument is the rewritten tree.  The classifier reads the *old* Rust-Debug tree and rewrites it
with exactly the mechanical shapes the epoch plan approves.  The rewritten old tree must then
equal the new tree byte for byte; anything else is emitted as manual residue for an individual
ledger disposition.  Nothing is inferred from the new tree, from fixture text, or from a
span-only comparison, so an ownership change can never be laundered as a re-typing.

Class `bo-joint-sum-wrapper` (#827) — the BO joints that became sums
    Rolling Zantufa admits a BO joint with no connective at all, which no sourced grammar does,
    so the joint positions that used to hold a single product now hold a two-arm sum: the
    sourced arm and the Zantufa connectorless one.  Three positions move, and the rewrite at
    each is the same one-to-one wrap with the payload carried across verbatim:

    * `SumtiBoundSyntax.bound_tail`: `BoundSumtiTailSyntax` gains the `BoundSumtiTail` wrapper.
    * `StagBoundTermConnectionSyntax.continuations`: `StagBoundTermContinuationSyntax` gains
      the `StagBoundTermContinuation` wrapper.
    * `BoundNormalTermConnectionSyntax.continuations`: `BoundNormalTermContinuationSyntax`
      gains the `BoundNormalTermContinuation` wrapper.

    The class is fail-closed on both sides: the old value must be exactly the sourced product
    with its own field names, and the payload must survive the wrap field for field.  A
    Zantufa arm can never appear on the old side -- the baseline grammar has no such node --
    so an old value that is already wrapped, or wrapped in the Zantufa arm, is residue.

Class `jai-payload-widening` (#827) — the JAI term payload
    The Zantufa JAI term's payload became the shared `(sumti / KU_elidible)` node every other
    tag-led term takes, so an overt sumti payload gains the `Sumti` arm wrapper.  The elided
    and explicit-KU payloads are new surfaces with no baseline entry, so only the overt arm can
    appear on the old side; anything else at that position is residue.

Class (i) `flat-sum-wrapper` — the wrapper paths of the five former `term` sum siblings
    The old flat `term` sum was `pehe_termset_connection / bound_term_connection /
    termset_group / connected_term / simple_term`.  Two of those five siblings wrapped an
    atom without contributing structure, and the composed ladder drops both wrappers:

    * `connected_term` had a `zero_or_more` continuation list, so every connectionless term
      in the corpus was `ConnectedTerm { leading_term, continuations: [] }`.  These
      degenerate zero-continuation wrappers are the enumerated class-(i) case: each level of
      the ladder now requires at least one continuation, so a connectionless term selects
      its leaf directly.
    * `simple_term` was a *nested sum branch*, which prints an extra `SimpleTerm(..)` wrapper
      variant.  Every ladder level now re-lists the leaves (mechanism E), so that wrapper is
      gone.  At the old flat-`term` position the strip is class (i); at a PEhE operand it is
      the class (ii) re-typing below.

    The other three siblings (`pehe_termset_connection`, `termset_group`,
    `bound_term_connection`) contribute real structure and are never rewritten: they are
    compared structurally, and `bound_term_connection` is the route #796 deletes.

    Class (i) is atom-only and fail-closed.  A wrapper is stripped only when its payload is
    one of the plain leaves — never a connection (`ConnectedTerm`, `StagBoundTermConnection`,
    `BoundTermConnection`, `TermsetGroup`, `PeheTermsetConnection`), never a termset or GEK
    atom (`ForethoughtTermset`, `NuhiTermset`, `KeTermset`), and never a leaf whose own sumti
    carries a VUhO attachment, which is the epoch-4/D6 ownership surface.  Recovered trees
    are excluded from every mechanical class.

    Per-level nesting-depth validation (plan C7): every strip is performed at a position
    whose old and new ladder levels are named in `POSITIONS`, and the surviving leaf's
    variant name must be a member of that position's *new* level inventory
    (`NEW_LEVEL_INVENTORY`, transcribed from the rebuilt grammar).  A leaf that landed at
    the wrong ladder depth — for example a `NonabsTaggedSumtiTerm` outside a CEhE
    continuation, or a `TaggedSumtiTerm` inside one — is residue, not a re-leveling.

Class (ii) `pehe-cehe-retyping` — PEhE and CEhE operand re-typing
    The PEhE operand level changed from `pehe_termset_operand` to `cehe_term` and the CEhE
    continuation from `simple_term` to `nonabs_term`.  Both re-typings are accepted only
    with the parent shape *and* the connective exhaustively proven:

    * PEhE: the parent must be exactly `PeheTermsetConnectionSyntax.leading_term` or
      `PeheTermsetConnectionContinuationSyntax.trailing_term`, and every governing PEhE
      connective must be `JoikConnective` or `JekConnective` — the corrected #806 domain.  A
      pre-existing `pe'e` + EK or `pe'e` + VUhU fixture is an acceptance *flip*, not a
      re-typing, and stays in residue.
    * CEhE: the parent must be exactly `TermsetGroupContinuationSyntax.trailing_term` with
      its `cehe` token present, and only the tag-led leaf may change name
      (`TaggedSumtiTerm` -> `NonabsTaggedSumtiTerm`).  The payload must be otherwise
      identical.

Class (iii) from the plan, sumti-term pass-through, is PROHIBITED and deliberately not
implemented: it would accept ownership changes at identical spans.

Class (iv) `t3-loose-connection-warning` — the T3 construct warning
    The loose (T3) tier is a diagnosed extension, so every loose continuation now carries
    `syntax.warning.experimental-term-loose-connection` anchored on its connective token.
    That warning is *additive to a diagnostics list and nothing else*: a fixture in this
    class must keep its tree, its status and every other leaf byte-identical, and its
    `diagnostics` leaves may only gain entries whose `code` is exactly the T3 warning.
    Deleting the additions from the new list must reproduce the old list element for element
    and in order, so no pre-existing diagnostic can be reordered, retimed, re-spanned or
    dropped under cover of the new warning.  A fixture that had no `diagnostics` leaf at all
    is treated as having had an empty one, which is the only leaf-set difference this
    classifier accepts.

    Unlike the tree classes, this one is *not* excluded on recovered expectations: a
    diagnostics list is a flat pinned list rather than a tree shape, and the additive check
    is exactly as fail-closed on `expectations.syntax.recovered.diagnostics` as it is on
    `expectations.syntax.diagnostics`.  Recovered *trees* remain excluded.

    Because the T3 warning fires on surfaces the epoch's own witnesses pin, this is also the
    only class permitted to appear on a C1-C6 witness.  A witness delta that classifies as
    anything else — or that mixes this class with any other — is still a hard error.

#796 flip `stagless-bo-route-rejection`
    The deleted standard stag-less BO term route makes its surfaces reject.  A fixture whose
    old tree contains `BoundTermConnection` and whose status flips success -> failure is a
    mechanical regeneration of that deletion; the flip is accepted only with that node
    present in the old tree, and the new failure diagnostics are still pinned exactly.

Interpretation recorded for review: plan C7 requires "atom-only old-shape ... fail-closed",
and reconciliation B9 spells that out as "excluding connectives, VUhO, relatives, termsets,
GEK, BO, recovered trees".  Every one of those exclusions is implemented above except a
blanket exclusion of terms whose sumti carries a relative clause.  A relative clause is
sumti-internal structure, not a term-tier shape, and class (i) already requires the
unwrapped payload to match the regenerated tree byte for byte, so no relative-clause
ownership change can survive the comparison; excluding them would move 1,769 pre-epoch
fixtures into hand-written residue rows without adding any safety.  The VUhO attachment is
excluded because that *is* a term-versus-sumti ownership surface (epoch-4 residual, D6).

The baseline archive is exactly `git archive 2397912147 tests/fixtures` (`ARCHIVE_COMMIT`),
the fixture tree at the epoch-6b merge, so it is reproducible rather than hand-assembled.
Every candidate fixture must therefore have a baseline entry and every baseline entry must
have a candidate: an unpaired fixture on either side is a hard error, never a skip.  The one
exception is a fixture this epoch ADDED, which is identified from
`git diff --diff-filter=A EPOCH_BASE..HEAD` rather than from its mere absence, is reported in
its own pinned list, and is never classified -- there is nothing to classify it against.  A
candidate the archive lacks and that git does not record as added by this epoch is still a
hard error.

The one value the classifier does not compare exactly is the `description` prose of a
`provenance` entry, which carries no expectation.  Every other provenance field stays exact --
the entries must correspond one for one and agree on every other key -- and a fixture whose
prose moved is listed in the report with its number pinned like the mechanical classes, so an
unreviewed prose edit still fails the run.

Epoch-new witnesses are not classified, so the only thing standing behind them is what they
pin.  A witness that omits `expectations.syntax.diagnostics` pins its tree and leaves its
warning stream unspecified, which is how a construct can quietly stop warning -- or start --
without any expectation moving.  Every epoch-new witness must therefore carry the key, empty
where the expectation is silence, and one that does not is a hard error rather than a count.
The check is on the key's presence, not its contents: what the warnings ARE is the writer's
output and the reviewer's artifact, and only their absence from the fixture is mechanical.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
import json
from pathlib import Path
import re
import subprocess
import sys
import threading
import tomllib
from typing import Any, Iterator

EPOCH_BASE = "2397912147"
# The commit whose `tests/fixtures` tree the baseline archive reproduces byte for byte.
# Epoch 6c re-baselines to its own implementation base, the epoch-6b merge, so every class
# earlier epochs contributed is already applied in the baseline and must now find nothing;
# they stay wired in because a nonzero incidence would mean the archive is not the tree it
# claims to be.
ARCHIVE_COMMIT = "2397912147"


@dataclass(frozen=True)
class Form:
    name: str
    fields: tuple[tuple[str, Any], ...] | None = None
    args: tuple[Any, ...] | None = None


class DebugParseError(ValueError):
    pass


IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*")
NUMBER = re.compile(r"-?[0-9]+(?:\.[0-9]+)?")
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
SPACE = re.compile(r"\s*")


class DebugParser:
    """Parser for the Rust `Debug` rendering the fixtures pin.

    Every scan is anchored with `pattern.match(text, pos)` rather than slicing the
    remaining input: the long-text fixtures pin multi-megabyte trees, and re-slicing per
    token makes the parse quadratic.
    """

    def __init__(self, text: str) -> None:
        self.text = text
        self.pos = 0

    def parse(self) -> Any:
        value = self._value()
        self._space()
        if self.pos != len(self.text):
            raise DebugParseError(f"unexpected input at byte {self.pos}")
        return value

    def _space(self) -> None:
        self.pos = SPACE.match(self.text, self.pos).end()

    def _value(self) -> Any:
        self._space()
        if self.pos >= len(self.text):
            raise DebugParseError("unexpected end of input")
        char = self.text[self.pos]
        if char == '"':
            match = STRING.match(self.text, self.pos)
            if not match:
                raise DebugParseError(f"unterminated string at byte {self.pos}")
            self.pos = match.end()
            return json.loads(match.group(0))
        if char == "[":
            return self._sequence("[", "]", list)
        if char == "(":
            return tuple(self._sequence("(", ")", list))
        number = NUMBER.match(self.text, self.pos)
        if number:
            token = number.group(0)
            self.pos = number.end()
            return float(token) if "." in token else int(token)
        name = self._identifier()
        self._space()
        if self._take("{"):
            fields: list[tuple[str, Any]] = []
            self._space()
            while not self._take("}"):
                key = self._identifier()
                self._space()
                self._expect(":")
                fields.append((key, self._value()))
                self._space()
                if not self._take(","):
                    self._expect("}")
                    break
                self._space()
            return Form(name=name, fields=tuple(fields))
        if self._take("("):
            args = self._sequence_body(")")
            return Form(name=name, args=tuple(args))
        if name == "true":
            return True
        if name == "false":
            return False
        return Form(name=name)

    def _sequence(self, opening: str, closing: str, factory: Any) -> Any:
        self._expect(opening)
        return factory(self._sequence_body(closing))

    def _sequence_body(self, closing: str) -> list[Any]:
        values: list[Any] = []
        self._space()
        while not self._take(closing):
            values.append(self._value())
            self._space()
            if not self._take(","):
                self._expect(closing)
                break
            self._space()
        return values

    def _identifier(self) -> str:
        self._space()
        match = IDENTIFIER.match(self.text, self.pos)
        if not match:
            raise DebugParseError(f"expected identifier at byte {self.pos}")
        self.pos = match.end()
        return match.group(0)

    def _take(self, token: str) -> bool:
        if self.text.startswith(token, self.pos):
            self.pos += len(token)
            return True
        return False

    def _expect(self, token: str) -> None:
        if not self._take(token):
            raise DebugParseError(f"expected {token!r} at byte {self.pos}")


# --- the ladder, transcribed from the grammar ------------------------------------------
#
# Every term-family position, with the rule that produced its operand in the baseline
# archive (`ARCHIVE_COMMIT`, the epoch-6b merge) and the ladder level that produces it now.
# The 25 consumer sites keep their arity and their level name, so they are listed by the
# field that holds the term.  Any term-family node that turns up at a position missing from
# this table is residue.
#
# Because the archive is the 6b merge rather than an earlier base, every re-leveling 6a and 6b
# performed is already applied on BOTH sides: the PEhE operand reads `cehe_term`, the
# TermsetGroup operands `loose_term`/`nonabs_term` and the GOI payload `normal_term` in the
# baseline too.  No position's level moves in this epoch; what moves is the shape at three BO
# joints and one payload, which the sum-wrapper table below carries.

CONSUMER_POSITIONS: tuple[tuple[str, str], ...] = (
    ("ZantufaIauStatementTermsTailSyntax", "terms"),
    ("ZantufaBareStatementTermsTailSyntax", "terms"),
    ("PrenexFragmentSyntax", "terms"),
    ("PrenexStatementSyntax", "prenex_terms"),
    ("TermsFragmentSyntax", "terms"),
    ("BridiWithLeadingTermsSyntax", "leading_terms"),
    ("BridiWithPostCuTermsSyntax", "leading_terms"),
    ("CuTermsBridiTailSyntax", "terms"),
    ("ZantufaGroupedBridiTailSyntax", "tail_terms"),
    ("SelbriSimpleBridiTailSyntax", "terms"),
    ("DirectForethoughtBridiConnectionSyntax", "tail_terms"),
    ("BridiTailKeContinuationSyntax", "tail_terms"),
    ("GihekBridiTailKeContinuationSyntax", "tail_terms"),
    ("BridiTailBoContinuationSyntax", "tail_terms"),
    ("BridiTailContinuationSyntax", "tail_terms"),
    ("PrenexSubbridiSyntax", "prenex_terms"),
    ("ForethoughtTermsetSyntax", "terms"),
    ("ForethoughtTermsetBranchSyntax", "terms"),
    ("ZantufaForethoughtTermsetBranchSyntax", "terms"),
    ("NuhiTermsetSyntax", "termset"),
    ("KeTermsetSyntax", "termset"),
    ("LaheTermWrapperSyntax", "inner_term"),
    ("ScalarNegatedTermWrapperWithBoSyntax", "inner_term"),
    ("ScalarNegatedTermWrapperSyntax", "inner_term"),
    ("SeiFreeModifierSyntax", "terms"),
)

# (parent Debug struct, field) -> (old level, new level)
POSITIONS: dict[tuple[str, str], tuple[str, str]] = {
    position: ("term", "term") for position in CONSUMER_POSITIONS
}
POSITIONS.update(
    {
        ("PeheTermsetConnectionSyntax", "leading_term"): ("cehe_term", "cehe_term"),
        ("PeheTermsetConnectionContinuationSyntax", "trailing_term"): (
            "cehe_term",
            "cehe_term",
        ),
        ("TermsetGroupSyntax", "leading_term"): ("loose_term", "loose_term"),
        ("TermsetGroupContinuationSyntax", "trailing_term"): ("nonabs_term", "nonabs_term"),
        ("ConnectedTermSyntax", "leading_term"): ("bound_term", "bound_term"),
        ("ConnectedTermContinuationSyntax", "trailing_term"): ("bound_term", "bound_term"),
        ("StagBoundTermConnectionSyntax", "leading_term"): ("simple_term", "simple_term"),
        ("StagBoundTermContinuationSyntax", "trailing_term"): ("simple_term", "simple_term"),
        # #794's shared normal-flavour payload constituent, in the baseline since epoch 6b.
        ("SumtiAssociationRelativeClauseSyntax", "sumti"): ("normal_term", "normal_term"),
        # The #796 route, deleted by this epoch; it can only appear on the old side.
        ("BoundTermConnectionSyntax", "leading_term"): ("simple_term", "<deleted>"),
        ("BoundTermConnectionSyntax", "trailing_term"): ("simple_term", "<deleted>"),
    }
)

# Epoch 6c: the positions whose single product became a two-arm sum, and the wrap each one
# takes.  `(parent Debug struct, field) -> (sum arm, sourced product, class)`.  Every one of
# these is a pure wrap: the sourced product keeps its own name and every field it had, and the
# Zantufa arm that shares the sum with it has no baseline counterpart at all.
SUM_WRAPPER_POSITIONS: dict[tuple[str, str], tuple[str, str, str]] = {
    ("SumtiBoundSyntax", "bound_tail"): (
        "BoundSumtiTail",
        "BoundSumtiTailSyntax",
        "bo-joint-sum-wrapper",
    ),
    ("StagBoundTermConnectionSyntax", "continuations"): (
        "StagBoundTermContinuation",
        "StagBoundTermContinuationSyntax",
        "bo-joint-sum-wrapper",
    ),
    ("BoundNormalTermConnectionSyntax", "continuations"): (
        "BoundNormalTermContinuation",
        "BoundNormalTermContinuationSyntax",
        "bo-joint-sum-wrapper",
    ),
    ("JaiTaggedSumtiTermSyntax", "sumti"): (
        "Sumti",
        "SumtiSyntax",
        "jai-payload-widening",
    ),
}

ATOM_LEAVES = frozenset(
    {
        "PlaceTaggedSumtiTerm",
        "JaiTaggedSumtiTerm",
        "ElidedNaheFihoTagTerm",
        "TaggedSumtiBeforeTagTerm",
        "TaggedSumtiTerm",
        "NonabsTaggedSumtiTerm",
        "NoihaAdverbialTerm",
        "FihoiAdverbialTerm",
        "SoiAdverbialTerm",
        "NaKuTerm",
        "SumtiTerm",
        "BareNaTerm",
        "ZantufaJoikChainedPlaceTagTerm",
        "ForethoughtTermset",
        "NuhiTermset",
        "KeTermset",
    }
)

# Atoms that carry a termset or a GEK.  Sol's fail-closed rule excludes them from every
# mechanical rewrite even though the old grammar listed them as `simple_term` leaves.
TERMSET_ATOMS = frozenset({"ForethoughtTermset", "NuhiTermset", "KeTermset"})

TERM_CONNECTION_NAMES = frozenset(
    {
        "ConnectedTerm",
        "StagBoundTermConnection",
        "BoundTermConnection",
        "TermsetGroup",
        "PeheTermsetConnection",
    }
)

# The leaves epoch 6b added -- D3's sourced NUhI-less `gek_termset` and its
# `ZantufaConnectives`-gated companion -- are in the baseline now.  This epoch adds exactly one
# leaf to every term level: the `ZANTUFA-TAGS`-gated JOIK-chained place tag (D5-3).  Transcribed
# from the `rule "term" ... -> enum` arm lists of `crates/jbotci-syntax/src/grammar/generated.rs`
# and diffed arm-for-arm against the same lists at `ARCHIVE_COMMIT`: across all nine levels this
# one name is the ONLY difference, which is what lets the old inventory below be written as the
# new one minus it rather than transcribed twice.
EPOCH_LEAF_DELTA = frozenset({"ZantufaJoikChainedPlaceTagTerm"})

# The two termset leaves epoch 6b added, kept as a named constant because the levels below are
# written in terms of the shared atom set and these are not atoms.
BASELINE_TERMSET_LEAVES = frozenset({"GekTermset", "ZantufaGekTermset"})

NEW_LEVEL_INVENTORY: dict[str, frozenset[str]] = {
    "term": frozenset(
        {"PeheTermsetConnection", "TermsetGroup", "ConnectedTerm", "StagBoundTermConnection"}
        | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "cehe_term": frozenset(
        {"TermsetGroup", "ConnectedTerm", "StagBoundTermConnection"}
        | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "loose_term": frozenset(
        {"ConnectedTerm", "StagBoundTermConnection"}
        | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "nonabs_term": frozenset(
        {"ConnectedTerm", "StagBoundTermConnection"}
        | (ATOM_LEAVES - {"TaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "bound_term": frozenset(
        {"StagBoundTermConnection"}
        | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "simple_term": frozenset(
        (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"}) | BASELINE_TERMSET_LEAVES
    ),
    "normal_term": frozenset(
        {"ConnectedNormalTerm", "BoundNormalTermConnection"}
        | (ATOM_LEAVES - {"TaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "bound_normal_term": frozenset(
        {"BoundNormalTermConnection"}
        | (ATOM_LEAVES - {"TaggedSumtiTerm"})
        | BASELINE_TERMSET_LEAVES
    ),
    "normal_term_atom": frozenset(
        (ATOM_LEAVES - {"TaggedSumtiTerm"}) | BASELINE_TERMSET_LEAVES
    ),
    "<deleted>": frozenset(),
}

# The baseline archive is the epoch-6b MERGE, so the OLD side of every comparison is the
# composed ladder with 6b's own levels already in it -- including `normal_term`, which is why
# the GOI payload position below now reads the same level on both sides.  Each level therefore
# carries its new inventory minus this epoch's one added leaf; `<deleted>` keeps the #796 route
# reachable so a stray occurrence still fails closed.
OLD_LEVEL_INVENTORY: dict[str, frozenset[str]] = {
    level: inventory - EPOCH_LEAF_DELTA
    for level, inventory in NEW_LEVEL_INVENTORY.items()
    if level != "<deleted>"
}
OLD_LEVEL_INVENTORY["<deleted>"] = frozenset()

# The #791 tripwire node types (Kimi R3).  A diff that touches one of them is never
# mechanical: the epoch plan requires stopping and fixing the grammar rather than the
# expectation.  The single enumerated exception is the degenerate zero-continuation
# `ConnectedTerm` wrapper handled by class (i).
TRIPWIRE_NAMES = frozenset({"ConnectedTerm", "ConnectedLinkedTerm", "LinkedSumti", "Linkargs"})

PEHE_CONNECTIVE_NAMES = frozenset({"JoikConnective", "JekConnective"})

# The reviewed regeneration result.  A re-run against the same archive must reproduce it
# exactly; `--expect-changed`/`--expect-manual` override the pins for an exploratory run.
# 44 pre-epoch fixtures move: the 42 that carry one of the re-typed joints and the two xfail
# fixtures whose recovered trees carry one, spliced rather than rewritten.
EXPECTED_CHANGED = 44
# Every class earlier epochs contributed is already applied in the re-baselined archive, so
# each must now find exactly nothing; they stay wired in because a nonzero incidence would mean
# the archive is not the tree it claims to be.  This epoch's two classes carry all 44: the BO
# joints that became sums, and the one fixture holding a JAI term whose payload widened.
EXPECTED_MECHANICAL = {
    "bo-joint-sum-wrapper": 43,
    "jai-payload-widening": 1,
    "flat-sum-wrapper": 0,
    "goi-payload-retyping": 0,
    "pehe-cehe-retyping": 0,
    "stagless-bo-route-rejection": 0,
    "t3-loose-connection-warning": 0,
}
# No fixture needed an individual disposition: every pre-epoch move is one of the two wraps.
EXPECTED_MANUAL = 0
# Prose-only provenance edits.  This epoch edits no provenance prose of its own.
EXPECTED_PROSE = 0
# Fixtures this epoch ADDED: the D5 witnesses.  They have no baseline entry, so they are
# pinned by count rather than classified.
EXPECTED_NEW_WITNESSES = 38

CLASS_BO_JOINT_SUM_WRAPPER = "bo-joint-sum-wrapper"
CLASS_JAI_PAYLOAD_WIDENING = "jai-payload-widening"
CLASS_GOI_PAYLOAD_RETYPING = "goi-payload-retyping"
CLASS_FLAT_SUM_WRAPPER = "flat-sum-wrapper"
CLASS_PEHE_CEHE_RETYPING = "pehe-cehe-retyping"
CLASS_BO_ROUTE_REJECTION = "stagless-bo-route-rejection"
CLASS_T3_LOOSE_WARNING = "t3-loose-connection-warning"
MECHANICAL_CLASSES = (
    CLASS_BO_JOINT_SUM_WRAPPER,
    CLASS_JAI_PAYLOAD_WIDENING,
    CLASS_GOI_PAYLOAD_RETYPING,
    CLASS_FLAT_SUM_WRAPPER,
    CLASS_PEHE_CEHE_RETYPING,
    CLASS_BO_ROUTE_REJECTION,
    CLASS_T3_LOOSE_WARNING,
)

T3_LOOSE_WARNING_CODE = "syntax.warning.experimental-term-loose-connection"


def t3_warning_only_addition(old_value: Any, new_value: Any) -> bool:
    """True when `new_value` is `old_value` plus T3 warning entries and nothing else.

    Fail-closed: a non-list on either side, a removal, a reorder, or any added entry whose
    `code` is not the T3 warning all return False and fall through to manual residue.
    """
    old_list = [] if old_value is None else old_value
    new_list = [] if new_value is None else new_value
    if not isinstance(old_list, list) or not isinstance(new_list, list):
        return False
    surviving = [entry for entry in new_list if entry.get("code") != T3_LOOSE_WARNING_CODE]
    if surviving != old_list:
        return False
    return len(new_list) > len(old_list)


class Divergence(Exception):
    def __init__(self, path: str, reason: str) -> None:
        super().__init__(f"{path}: {reason}")
        self.path = path
        self.reason = reason


def form_name(value: Any) -> str:
    return value.name if isinstance(value, Form) else type(value).__name__


def single_arg(value: Any, name: str) -> Any | None:
    """Return the single payload of `Name(payload)`, or None."""
    if isinstance(value, Form) and value.name == name and value.args is not None:
        if len(value.args) == 1:
            return value.args[0]
    return None


def own_sumti_carries_vuho(leaf: Form) -> bool:
    """True when the leaf's own sumti has a VUhO attachment (the D6 ownership surface)."""
    payload = leaf.args[0] if leaf.args and len(leaf.args) == 1 else None
    if payload is None:
        return False
    candidates: list[Any] = []
    if isinstance(payload, Form):
        if payload.fields is not None:
            candidates.extend(child for key, child in payload.fields if key == "sumti")
        elif payload.args is not None:
            candidates.extend(payload.args)
    for candidate in candidates:
        sumti = single_arg(candidate, "Sumti")
        if sumti is None:
            sumti = candidate
        if isinstance(sumti, Form) and sumti.fields is not None:
            for key, child in sumti.fields:
                if key == "vuho_attachment" and form_name(child) != "None":
                    return True
    return False


def require_mechanical_atom(leaf: Any, path: str, wrapper: str) -> Form:
    if not isinstance(leaf, Form):
        raise Divergence(path, f"{wrapper} payload is not a syntax node")
    if leaf.name in TERM_CONNECTION_NAMES:
        raise Divergence(path, f"{wrapper} payload is the connection node {leaf.name}")
    if leaf.name in TERMSET_ATOMS:
        raise Divergence(path, f"{wrapper} payload is the termset/GEK atom {leaf.name}")
    if leaf.name not in ATOM_LEAVES:
        raise Divergence(path, f"{wrapper} payload {leaf.name} is not a term leaf")
    if own_sumti_carries_vuho(leaf):
        raise Divergence(path, f"{wrapper} payload carries a VUhO attachment")
    return leaf


def require_new_level_admits(name: str, new_level: str, path: str) -> None:
    inventory = NEW_LEVEL_INVENTORY.get(new_level)
    if inventory is None:
        raise Divergence(path, f"unknown ladder level {new_level}")
    if name not in inventory:
        raise Divergence(path, f"{name} is not a leaf of the {new_level} level")


def pehe_connectives_are_corrected(parent: Form, field: str, path: str) -> None:
    """Prove the connective domain of a PEhE parent before accepting a re-typing."""
    fields = dict(parent.fields or ())
    if field == "trailing_term":
        connectives = [fields.get("connective")]
    else:
        continuations = fields.get("continuations")
        if not isinstance(continuations, list) or not continuations:
            raise Divergence(path, "PEhE parent has no continuation to prove")
        connectives = []
        for continuation in continuations:
            if not isinstance(continuation, Form) or continuation.fields is None:
                raise Divergence(path, "PEhE continuation is not a syntax node")
            connectives.append(dict(continuation.fields).get("connective"))
    for connective in connectives:
        if form_name(connective) not in PEHE_CONNECTIVE_NAMES:
            raise Divergence(
                path, f"PEhE connective {form_name(connective)} is outside the JOIK/JEK domain"
            )


def rewrite_term_position(
    old: Any,
    new: Any,
    parent: Form,
    field: str,
    levels: tuple[str, str],
    path: str,
    classes: set[str],
) -> Any:
    """Apply the approved mechanical rewrites at one term-family position.

    Returns the rewritten *old* value; the caller compares it structurally with `new`.
    """
    old_level, new_level = levels
    if not isinstance(old, Form):
        return old
    if old.name not in OLD_LEVEL_INVENTORY.get(old_level, frozenset()):
        raise Divergence(path, f"{old.name} is not a member of the old {old_level} level")

    # Class (i): the degenerate zero-continuation `connected_term` wrapper.
    payload = single_arg(old, "ConnectedTerm")
    if payload is not None:
        if not isinstance(payload, Form) or payload.name != "ConnectedTermSyntax":
            raise Divergence(path, "ConnectedTerm payload is not ConnectedTermSyntax")
        keys = tuple(key for key, _ in payload.fields or ())
        if keys != ("leading_term", "continuations"):
            raise Divergence(path, f"unexpected ConnectedTermSyntax fields {keys}")
        fields = dict(payload.fields or ())
        if fields["continuations"] == []:
            leaf = require_mechanical_atom(
                fields["leading_term"], path, "zero-continuation ConnectedTerm"
            )
            require_new_level_admits(leaf.name, new_level, path)
            classes.add(CLASS_FLAT_SUM_WRAPPER)
            return leaf
        return old  # a real loose connection: compared structurally, tripwire on any diff

    # Class (i) / class (ii): the nested `simple_term` sum branch.
    payload = single_arg(old, "SimpleTerm")
    if payload is not None:
        leaf = require_mechanical_atom(payload, path, "SimpleTerm wrapper")
        require_new_level_admits(leaf.name, new_level, path)
        if old_level == "pehe_termset_operand":
            pehe_connectives_are_corrected(parent, field, path)
            classes.add(CLASS_PEHE_CEHE_RETYPING)
        else:
            classes.add(CLASS_FLAT_SUM_WRAPPER)
        return leaf

    # Class (ii): the CEhE continuation takes the unguarded tag-led leaf.  The two rules
    # differ only in the absorption assertion, so the payload must survive the rename
    # field for field.
    payload = single_arg(old, "TaggedSumtiTerm")
    if payload is not None and new_level == "nonabs_term":
        if (parent.name, field) != ("TermsetGroupContinuationSyntax", "trailing_term"):
            raise Divergence(path, "nonabs re-typing outside a CEhE continuation")
        if form_name(dict(parent.fields or ()).get("cehe")) == "None":
            raise Divergence(path, "CEhE continuation has no CEhE token")
        if not isinstance(payload, Form) or payload.name != "TaggedSumtiTermSyntax":
            raise Divergence(path, "TaggedSumtiTerm payload is not TaggedSumtiTermSyntax")
        keys = tuple(key for key, _ in payload.fields or ())
        if keys != ("tense_modal", "sumti"):
            raise Divergence(path, f"unexpected TaggedSumtiTermSyntax fields {keys}")
        require_new_level_admits("NonabsTaggedSumtiTerm", new_level, path)
        classes.add(CLASS_PEHE_CEHE_RETYPING)
        return Form(
            name="NonabsTaggedSumtiTerm",
            args=(Form(name="NonabsTaggedSumtiTermSyntax", fields=payload.fields),),
        )

    return old


def compare_tree(
    old: Any,
    new: Any,
    classes: set[str],
    path: str = "",
    position: tuple[Form, str, tuple[str, str]] | None = None,
    list_position: tuple[Form, str] | None = None,
) -> None:
    """Structural comparison of the rewritten old tree against the new tree."""
    if position is not None:
        parent, field, levels = position
        old = rewrite_term_position(old, new, parent, field, levels, path, classes)

    if isinstance(old, Form) or isinstance(new, Form):
        if not (isinstance(old, Form) and isinstance(new, Form)):
            raise Divergence(path, f"{form_name(old)} became {form_name(new)}")
        if old.name != new.name:
            reason = f"{old.name} became {new.name}"
            if old.name in TRIPWIRE_NAMES or new.name in TRIPWIRE_NAMES:
                reason += " (#791 tripwire node type)"
            raise Divergence(path, reason)
        if (old.fields is None) != (new.fields is None) or (old.args is None) != (
            new.args is None
        ):
            raise Divergence(path, f"{old.name} changed shape")
        if old.fields is not None:
            old_keys = tuple(key for key, _ in old.fields)
            new_keys = tuple(key for key, _ in new.fields or ())
            if old_keys != new_keys:
                raise Divergence(path, f"{old.name} fields {old_keys} became {new_keys}")
            if new.name == "ConnectedTermSyntax" and dict(new.fields or ()).get(
                "continuations"
            ) == []:
                raise Divergence(path, "regenerated tree has a zero-continuation ConnectedTerm")
            for (key, old_child), (_, new_child) in zip(old.fields, new.fields or ()):
                child_path = f"{path}.{key}" if path else key
                compare_tree(
                    wrap_sum_position(old_child, old, key, child_path, classes),
                    new_child,
                    classes,
                    child_path,
                    _position_for(old, key),
                    (old, key) if (old.name, key) in SUM_WRAPPER_POSITIONS else None,
                )
        if old.args is not None:
            if len(old.args) != len(new.args or ()):
                raise Divergence(path, f"{old.name} arity changed")
            for index, (old_child, new_child) in enumerate(zip(old.args, new.args or ())):
                compare_tree(old_child, new_child, classes, f"{path}({index})")
        return

    if isinstance(old, list) or isinstance(new, list):
        if not (isinstance(old, list) and isinstance(new, list)):
            raise Divergence(path, f"{form_name(old)} became {form_name(new)}")
        if len(old) != len(new):
            raise Divergence(path, f"sequence length {len(old)} became {len(new)}")
        for index, (old_child, new_child) in enumerate(zip(old, new)):
            element_path = f"{path}[{index}]"
            if list_position is not None:
                parent, field = list_position
                old_child = wrap_sum_position(old_child, parent, field, element_path, classes)
            compare_tree(old_child, new_child, classes, element_path, position, list_position)
        return

    if isinstance(old, tuple) or isinstance(new, tuple):
        if not (isinstance(old, tuple) and isinstance(new, tuple)) or len(old) != len(new):
            raise Divergence(path, "tuple shape changed")
        for index, (old_child, new_child) in enumerate(zip(old, new)):
            compare_tree(old_child, new_child, classes, f"{path}({index})")
        return

    if old != new:
        raise Divergence(path, f"{old!r} became {new!r}")


def _position_for(parent: Form, field: str) -> tuple[Form, str, tuple[str, str]] | None:
    levels = POSITIONS.get((parent.name, field))
    if levels is None:
        return None
    return parent, field, levels


def wrap_sum_position(old: Any, parent: Form, field: str, path: str, classes: set[str]) -> Any:
    """Apply the epoch-6c sum wrap at one joint position, or leave the value alone.

    The wrap is applied only to the exact sourced product the baseline grammar could produce
    there; every other value falls through unchanged and is compared structurally, which is
    what turns an unexpected shape into manual residue instead of a laundered rewrite.
    """
    wrap = SUM_WRAPPER_POSITIONS.get((parent.name, field))
    if wrap is None or not isinstance(old, Form):
        return old
    arm, product, class_name = wrap
    if old.name == "Some":
        if old.args is None or len(old.args) != 1:
            return old
        inner = wrap_sum_position(old.args[0], parent, field, path, classes)
        return old if inner is old.args[0] else Form(name="Some", args=(inner,))
    if old.name != product:
        return old
    classes.add(class_name)
    return Form(name=arm, args=(old,))


def assert_no_retired_shapes(tree: str, path: str) -> None:
    """Invariant on the regenerated tree: the retired nested-sum wrapper is gone.

    Every ladder level re-lists its leaves, so `simple_term` is only ever reached through an
    `arc`, which prints the selected leaf directly.  The retired zero-continuation
    `ConnectedTerm` wrapper is rejected structurally in `compare_tree`.
    """
    if "SimpleTerm(" in tree:
        raise Divergence(path, "regenerated tree still contains a SimpleTerm wrapper")


def leaves(value: Any, path: tuple[str, ...] = ()) -> Iterator[tuple[tuple[str, ...], Any]]:
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, (*path, key))
    else:
        yield path, value


TREE_TOKEN = re.compile(r'^\s*(?:\w+:\s*)?\w+ @\[(?P<span>.*?)\) "(?P<text>.*)"[,]?$')
BRACKET_TOKEN = re.compile(r"[A-Za-z][A-Za-z',\.]*")


def tree_token_projection(value: str) -> tuple[tuple[str, str], ...]:
    result: list[tuple[str, str]] = []
    for line in value.splitlines():
        match = TREE_TOKEN.match(line)
        if match:
            result.append((match.group("span"), match.group("text")))
    return tuple(result)


def json_token_projection(value: Any) -> tuple[tuple[str, Any], ...]:
    result: list[tuple[str, Any]] = []

    def visit(node: Any) -> None:
        if isinstance(node, dict):
            if set(node) >= {"phonemes", "span"}:
                result.append((node["phonemes"], node["span"]))
            for child in node.values():
                visit(child)
        elif isinstance(node, list):
            for child in node:
                visit(child)

    visit(value)
    return tuple(result)


def bracket_token_projection(value: str) -> tuple[str, ...]:
    return tuple(BRACKET_TOKEN.findall(value))


def provenance_prose_only(old_value: Any, new_value: Any) -> bool:
    """True when a `provenance` array moved in its `description` prose and nowhere else.

    `leaves` yields the whole array of tables as one value, so the entries are paired here:
    they must correspond one for one, carry the same keys, and agree on every key other than
    `description`.  A re-sourced, added or dropped provenance entry is therefore still residue.
    """
    if not isinstance(old_value, list) or not isinstance(new_value, list):
        return False
    if len(old_value) != len(new_value):
        return False
    for old_entry, new_entry in zip(old_value, new_value):
        if not isinstance(old_entry, dict) or not isinstance(new_entry, dict):
            return False
        if set(old_entry) != set(new_entry):
            return False
        if any(old_entry[key] != new_entry[key] for key in old_entry if key != "description"):
            return False
    return True


def compare_fixture(
    old: dict[str, Any], new: dict[str, Any]
) -> tuple[set[str], list[str], bool]:
    old_leaves = dict(leaves(old))
    new_leaves = dict(leaves(new))
    residue: list[str] = []
    classes: set[str] = set()
    prose = False
    if set(old_leaves) != set(new_leaves):
        added_paths = set(new_leaves) - set(old_leaves)
        removed_paths = set(old_leaves) - set(new_leaves)
        # A fixture that carried no diagnostics at all gains the leaf outright when the T3
        # warning starts firing.  That is the one leaf-set difference class (iv) accepts, and
        # only in the additive direction: a removed leaf is never mechanical.
        t3_added = {
            path
            for path in added_paths
            if path[-1:] == ("diagnostics",) and t3_warning_only_addition(None, new_leaves[path])
        }
        if not removed_paths and t3_added == added_paths:
            classes.add(CLASS_T3_LOOSE_WARNING)
            for path in t3_added:
                old_leaves[path] = []
        else:
            added = sorted(".".join(path) for path in added_paths)
            removed = sorted(".".join(path) for path in removed_paths)
            reasons = []
            if added:
                reasons.append(f"expectation leaves added: {', '.join(added)}")
            if removed:
                reasons.append(f"expectation leaves removed: {', '.join(removed)}")
            return classes, reasons, prose

    old_status = old_leaves.get(("expectations", "syntax", "status"))
    new_status = new_leaves.get(("expectations", "syntax", "status"))
    old_raw = old_leaves.get(("expectations", "syntax", "raw"))
    bo_route_flip = (
        old_status == "success"
        and new_status == "failure"
        and isinstance(old_raw, str)
        and "BoundTermConnection(" in old_raw
    )

    for path, old_value in old_leaves.items():
        new_value = new_leaves[path]
        if old_value == new_value:
            continue
        joined = ".".join(path)
        # Provenance prose, not an expectation: recorded and counted rather than compared.
        if path == ("provenance",) and provenance_prose_only(old_value, new_value):
            prose = True
            continue
        # Checked before the recovered exclusion: a diagnostics list is a flat pinned list,
        # not a tree shape, so the additive T3 check is equally fail-closed on the recovered
        # expectations.  Recovered trees stay excluded by the branch below.
        if path[-1:] == ("diagnostics",):
            if t3_warning_only_addition(old_value, new_value):
                classes.add(CLASS_T3_LOOSE_WARNING)
            else:
                residue.append(joined)
            continue
        if path[:2] == ("expectations", "syntax") and "recovered" in path:
            residue.append(f"recovered-tree leaf {joined} (excluded from mechanical classes)")
            continue
        if path == ("expectations", "syntax", "raw"):
            try:
                old_form = DebugParser(old_value).parse()
                new_form = DebugParser(new_value).parse()
            except (DebugParseError, TypeError) as error:
                residue.append(f"syntax tree parser: {error}")
                continue
            try:
                assert_no_retired_shapes(new_value, "syntax.raw")
                compare_tree(old_form, new_form, classes)
            except Divergence as divergence:
                residue.append(f"syntax tree {divergence.path or '<root>'}: {divergence.reason}")
            continue
        if path == ("expectations", "syntax", "status") and bo_route_flip:
            classes.add(CLASS_BO_ROUTE_REJECTION)
            continue
        if path[-1:] == ("json",) and "gentufa" in path:
            try:
                old_json = json.loads(old_value)
                new_json = json.loads(new_value)
            except json.JSONDecodeError as error:
                residue.append(f"gentufa.json parser: {error}")
                continue
            if json_token_projection(old_json) != json_token_projection(new_json):
                residue.append("gentufa.json token/span projection changed")
            continue
        if path[-1:] == ("tree",) and "gentufa" in path:
            if tree_token_projection(old_value) != tree_token_projection(new_value):
                residue.append("gentufa.tree token/span projection changed")
            continue
        if path[-1:] == ("brackets",) and "gentufa" in path:
            if bracket_token_projection(old_value) != bracket_token_projection(new_value):
                residue.append("gentufa.brackets token projection changed")
            continue
        # Diagnostics, statuses, digests, semantics refs and every other leaf
        # are deliberately exact.
        residue.append(joined)
    if not classes and old != new and not residue and not prose:
        residue.append("no mechanical term-hierarchy shape found")
    return classes, residue, prose


def classify_one(job: tuple[str, str, str]) -> tuple[str, list[str], list[str], bool]:
    """Worker entry point.  The texts are re-read here so the parent never holds them."""
    repository_path, baseline_file, candidate_file = job
    old = tomllib.loads(Path(baseline_file).read_text())
    new = tomllib.loads(Path(candidate_file).read_text())
    classes, residue, prose = compare_fixture(old, new)
    return repository_path, sorted(classes), residue, prose


def collect_jobs(
    candidate_root: Path, baseline_root: Path, witnesses: set[str]
) -> tuple[list[tuple[str, str, str]], list[tuple[str, str, str]], list[str], list[str]]:
    jobs: list[tuple[str, str, str]] = []
    witness_jobs: list[tuple[str, str, str]] = []
    # Fixtures this epoch ADDED have no baseline entry to classify against, so they are
    # neither classified nor silently dropped: they are listed and pinned by count, and the
    # authored expectation is the reviewed artifact.  Epoch 6a's archive sat at the C1-C6
    # tip, after its own witnesses landed, so it had no such population; 6b re-baselines to
    # its implementation base and therefore does.
    epoch_new: list[str] = []
    # An unpaired fixture on either side is unclassifiable, so it is reported rather than
    # skipped: a missing baseline entry would otherwise drop a re-pinned fixture -- including
    # a new epoch witness -- out of the audit without a trace.
    unpaired: list[str] = []
    seen: set[Path] = set()
    for candidate_path in sorted(candidate_root.rglob("*.toml")):
        relative = candidate_path.relative_to(candidate_root)
        seen.add(relative)
        repository_path = (Path("tests/fixtures") / relative).as_posix()
        baseline_file = baseline_root / relative
        if not baseline_file.exists():
            if repository_path in witnesses:
                epoch_new.append(repository_path)
            else:
                unpaired.append(f"{repository_path}: absent from the baseline archive")
            continue
        if baseline_file.read_bytes() == candidate_path.read_bytes():
            continue
        if repository_path in witnesses:
            # C1-C6 witnesses carry commit-local exact pins.  Since the T3 warning ruling they
            # are allowed exactly one delta -- the additive class-(iv) re-pin -- and are
            # classified for it rather than skipped; any other delta means the pin was wrong.
            witness_jobs.append((repository_path, str(baseline_file), str(candidate_path)))
            continue
        jobs.append((repository_path, str(baseline_file), str(candidate_path)))
    for baseline_path in sorted(baseline_root.rglob("*.toml")):
        relative = baseline_path.relative_to(baseline_root)
        if relative not in seen:
            repository_path = (Path("tests/fixtures") / relative).as_posix()
            unpaired.append(f"{repository_path}: absent from the candidate tree")
    return jobs, witness_jobs, unpaired, epoch_new


def epoch_new_missing_diagnostics(candidate_root: Path, epoch_new: list[str]) -> list[str]:
    """Epoch-new witnesses that pin no `expectations.syntax.diagnostics` list.

    An epoch-new witness has no baseline entry, so no class checks it and the authored
    expectation is the whole audit.  Omitting the key leaves the warning stream unpinned,
    and `fixture-rewrite` fills the list only where the key already exists, so the omission
    is also self-perpetuating.  A witness with no `expectations.syntax` at all cannot carry
    the key and is reported for the same reason: the exception has to be argued here rather
    than fall out of a lookup that quietly finds nothing.
    """
    offenders: list[str] = []
    for repository_path in epoch_new:
        relative = Path(repository_path).relative_to("tests/fixtures")
        document = tomllib.loads((candidate_root / relative).read_text(encoding="utf-8"))
        syntax = document.get("expectations", {}).get("syntax")
        if syntax is None:
            offenders.append(f"{repository_path}: no expectations.syntax to pin diagnostics on")
        elif "diagnostics" not in syntax:
            offenders.append(f"{repository_path}: expectations.syntax pins no diagnostics list")
    return sorted(offenders)


def run(args: argparse.Namespace) -> int:
    witnesses = set(
        subprocess.run(
            [
                "git",
                "diff",
                "--diff-filter=A",
                "--name-only",
                f"{EPOCH_BASE}..HEAD",
                "--",
                "tests/fixtures",
            ],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        ).stdout.splitlines()
    )
    jobs, witness_jobs, unpaired, epoch_new = collect_jobs(
        args.candidate, args.baseline_root, witnesses
    )

    mechanical: dict[str, list[str]] = {name: [] for name in MECHANICAL_CLASSES}
    manual: list[tuple[str, list[str]]] = []
    prose_edits: list[str] = []
    witness_rewarns: list[str] = []
    witness_deltas: list[str] = []
    with ProcessPoolExecutor(max_workers=args.jobs) as pool:
        for repository_path, classes, residue, prose in pool.map(classify_one, jobs, chunksize=16):
            if prose:
                prose_edits.append(repository_path)
            if residue:
                manual.append((repository_path, residue))
            else:
                for classification in classes:
                    mechanical[classification].append(repository_path)
        for repository_path, classes, residue, prose in pool.map(
            classify_one, witness_jobs, chunksize=4
        ):
            if prose:
                prose_edits.append(repository_path)
            if not residue and classes == [CLASS_T3_LOOSE_WARNING]:
                witness_rewarns.append(repository_path)
            else:
                witness_deltas.append(repository_path)

    lines = [f"changed: {len(jobs)}"]
    for classification, paths in mechanical.items():
        lines.append(f"mechanical {classification}: {len(paths)}")
    lines.append(f"manual: {len(manual)}")
    for path, reasons in sorted(manual):
        lines.append(f"  {path}: {'; '.join(reasons)}")
    lines.append(f"prose-only provenance edits: {len(prose_edits)}")
    lines.extend(f"  {path}" for path in sorted(prose_edits))
    lines.append(f"epoch-witness T3 re-pins: {len(witness_rewarns)}")
    lines.extend(f"  {path}" for path in sorted(witness_rewarns))
    lines.append(f"epoch-new witnesses (authored, unclassifiable): {len(epoch_new)}")
    lines.extend(f"  {path}" for path in sorted(epoch_new))
    unpinned_diagnostics = epoch_new_missing_diagnostics(args.candidate, epoch_new)
    if unpinned_diagnostics:
        lines.append(
            "epoch-new witnesses without a diagnostics pin (must be empty): "
            f"{len(unpinned_diagnostics)}"
        )
        lines.extend(f"  {entry}" for entry in unpinned_diagnostics)
    if unpaired:
        lines.append(f"unpaired fixtures (must be empty): {len(unpaired)}")
        lines.extend(f"  {entry}" for entry in sorted(unpaired))
    if witness_deltas:
        lines.append(f"epoch-witness deltas (must be empty): {len(witness_deltas)}")
        lines.extend(f"  {path}" for path in sorted(witness_deltas))
    report = "\n".join(lines)
    print(report)
    if args.report:
        args.report.write_text(report + "\n")
    if args.report_json:
        args.report_json.write_text(
            json.dumps(
                {
                    "changed": len(jobs),
                    "mechanical": mechanical,
                    "manual": [
                        {"fixture": path, "reasons": reasons} for path, reasons in sorted(manual)
                    ],
                    "prose_edits": sorted(prose_edits),
                    "unpaired": sorted(unpaired),
                    "witness_rewarns": sorted(witness_rewarns),
                    "witness_deltas": sorted(witness_deltas),
                    "epoch_new": sorted(epoch_new),
                    "epoch_new_missing_diagnostics": unpinned_diagnostics,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )

    if unpaired:
        print(
            "error: every fixture must be paired with the baseline archive "
            f"({ARCHIVE_COMMIT}); an unpaired one is never skipped"
        )
        return 1
    if witness_deltas:
        print("error: epoch witnesses may only take the additive T3 warning re-pin")
        return 1
    if unpinned_diagnostics:
        print(
            "error: every epoch-new witness must pin expectations.syntax.diagnostics, "
            "empty where the expectation is silence"
        )
        return 1
    if len(jobs) != args.expect_changed:
        print(f"error: expected {args.expect_changed} changed pre-epoch fixtures")
        return 1
    if {name: len(paths) for name, paths in mechanical.items()} != EXPECTED_MECHANICAL:
        print("error: mechanical class incidences differ from the reviewed C7 set")
        return 1
    if len(manual) != args.expect_manual:
        print(f"error: expected {args.expect_manual} manual-residue fixtures")
        return 1
    if len(prose_edits) != args.expect_prose:
        print(f"error: expected {args.expect_prose} prose-only provenance edits")
        return 1
    if len(epoch_new) != args.expect_new_witnesses:
        print(f"error: expected {args.expect_new_witnesses} epoch-new witness fixtures")
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--baseline-root", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--report-json", type=Path)
    parser.add_argument("--jobs", type=int, default=15)
    parser.add_argument("--expect-changed", type=int, default=EXPECTED_CHANGED)
    parser.add_argument("--expect-manual", type=int, default=EXPECTED_MANUAL)
    parser.add_argument("--expect-prose", type=int, default=EXPECTED_PROSE)
    parser.add_argument(
        "--expect-new-witnesses", type=int, default=EXPECTED_NEW_WITNESSES
    )
    args = parser.parse_args()

    # The pinned trees nest deeply enough to exhaust the default interpreter stack.
    sys.setrecursionlimit(200_000)
    threading.stack_size(512 * 1024 * 1024)
    result: list[int] = []
    worker = threading.Thread(target=lambda: result.append(run(args)))
    worker.start()
    worker.join()
    return result[0] if result else 1


if __name__ == "__main__":
    raise SystemExit(main())
