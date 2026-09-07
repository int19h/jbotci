"""The exact old-JAI to shared-atom transcription; never an acceptance oracle.

Read the baseline Debug tree, preserve every token and source-span field, and add
only the shared grammar's prescribed layers. The complete result must equal the
candidate tree. Recovery, ProBridi's guard split and precedence movement are not
mechanical. Regression fixtures are excluded by the caller even for a matching tree.
"""

from __future__ import annotations

from typing import Any, Iterator

if __package__:
    from .rust_debug import Form
else:
    from rust_debug import Form


class ManualShape(ValueError):
    """The old subtree has no approved mechanical transcription."""


SAME_PRODUCTS = frozenset({
    "SumtiSelbriTanruUnit", "QuotedBridiSelbriTanruUnit",
    "QuotedTextSelbriTanruUnit", "TextSelbriTanruUnit", "OrdinalTanruUnit",
    "OperatorSelbriTanruUnit", "WordTanruUnit",
})
OLD_ARMS = SAME_PRODUCTS | {
    "ConvertedJaiInnerTanruUnit", "ScalarNegatedJaiInnerTanruUnit",
    "GroupedJaiInnerTanruUnit", "ProBridiTanruUnit",
}
JAI_FIELDS = ("jai", "tense_modal", "inner_unit")
ATOM_FIELDS = ("conversions", "base")
CONVERSION_FIELDS = ("se", "inner_unit")
SCALAR_FIELDS = ("nahe", "inner_unit")
GROUP_FIELDS = ("ke", "selbri", "kehe")
OLD_CONNECTED_FIELDS = ("leading_selbri", "continuations")
OLD_TANRU_FIELDS = ("first_unit", "additional_units")
NONE = Form("None")


def product(name: str, **fields: Any) -> Form:
    return Form(name, fields=tuple(fields.items()))


def arm(name: str, value: Any) -> Form:
    return Form(name, args=(value,))


def fields(value: Any, name: str, names: tuple[str, ...]) -> tuple[Any, ...]:
    if not (isinstance(value, Form) and value.name == name and value.args is None and
            value.fields is not None and tuple(key for key, _ in value.fields) == names):
        raise ManualShape(f"expected {name} fields {names}")
    return tuple(child for _, child in value.fields)


def sequence(value: Any) -> list[Any]:
    if not isinstance(value, list):
        raise ManualShape("expected a grammar sequence")
    return value


def shared_atom(base: Form, conversions: list[Any] | None = None) -> Form:
    return product("TanruUnitAtomSyntax", conversions=[] if conversions is None else conversions, base=base)


def shared_bound(old_unit: Form) -> Form:
    """A mini-ladder leaf has no old links, assignments or BO continuation."""
    linked = product("LinkedTanruUnitSyntax", base=map_inner(old_unit), linkargs=NONE)
    unit = product("TanruUnitSyntax", base=linked, assignments=[])
    plain = arm("PlainBoTanruUnit", product("PlainBoTanruUnitSyntax", leading_unit=unit, bo_tail=NONE))
    return product("BoundSelbriSyntax", leading_selbri=plain, bo_tail=NONE)


def map_group(old: Form) -> Form:
    ke, connected, kehe = fields(old, "GroupedJaiInnerTanruUnitSyntax", GROUP_FIELDS)
    leading, tails = fields(connected, "ConnectedJaiInnerSelbriSyntax", OLD_CONNECTED_FIELDS)
    first, adjacent = fields(leading, "TanruJaiInnerSelbriSyntax", OLD_TANRU_FIELDS)
    adjacent, tails = sequence(adjacent), sequence(tails)
    if not tails:
        # Pure adjacency has the same precedence in both ladders.
        groups = [product("ConnectedSelbriSyntax", leading_selbri=shared_bound(unit), continuations=[])
                  for unit in [first, *adjacent]]
        body = product("TanruSelbriSyntax", first_selbri=groups[0], additional_selbri=groups[1:])
    else:
        if adjacent:
            raise ManualShape("mini-ladder mixes adjacency with connective precedence")
        continuations = []
        for tail in tails:
            connective, trailing = fields(tail, "ConnectedJaiInnerSelbriContinuationSyntax",
                                          ("connective", "trailing_selbri"))
            unit, more = fields(trailing, "TanruJaiInnerSelbriSyntax", OLD_TANRU_FIELDS)
            if sequence(more):
                raise ManualShape("mini-ladder mixes connective operands with adjacency")
            continuations.append(arm("SimpleConnectedSelbriContinuation", product(
                "SimpleConnectedSelbriContinuationSyntax", connective=connective,
                trailing_selbri=shared_bound(unit))))
        body = product("TanruSelbriSyntax", first_selbri=product(
            "ConnectedSelbriSyntax", leading_selbri=shared_bound(first), continuations=continuations),
            additional_selbri=[])
    return shared_atom(arm("GroupedTanruUnit", product("GroupedTanruUnitSyntax", ke=ke, selbri=body, kehe=kehe)))


def map_inner(old: Any) -> Form:
    if not (isinstance(old, Form) and old.fields is None and old.args is not None and len(old.args) == 1):
        raise ManualShape("expected one old JaiInnerTanruUnit arm")
    payload = old.args[0]
    if old.name in SAME_PRODUCTS:
        if not isinstance(payload, Form) or payload.name != f"{old.name}Syntax":
            raise ManualShape(f"unexpected {old.name} product")
        return shared_atom(old)
    if old.name == "ConvertedJaiInnerTanruUnit":
        se, inner = fields(payload, "ConvertedJaiInnerTanruUnitSyntax", CONVERSION_FIELDS)
        conversions, base = fields(map_inner(inner), "TanruUnitAtomSyntax", ATOM_FIELDS)
        return shared_atom(base, [se, *sequence(conversions)])
    if old.name == "ScalarNegatedJaiInnerTanruUnit":
        nahe, inner = fields(payload, "ScalarNegatedJaiInnerTanruUnitSyntax", SCALAR_FIELDS)
        return shared_atom(arm("ScalarNegatedTanruUnit", product(
            "ScalarNegatedTanruUnitSyntax", nahe=nahe, inner_unit=arm("TanruUnitAtom", map_inner(inner)))))
    if old.name == "GroupedJaiInnerTanruUnit":
        return map_group(payload)
    if old.name == "ProBridiTanruUnit":
        raise ManualShape("shared GohaWord/ProBridi guard split requires manual review")
    raise ManualShape(f"unmapped old JAI arm {old.name}")


def nodes(value: Any) -> Iterator[Form]:
    """Structural Debug-data traversal, not a parallel Lojban parser."""
    if isinstance(value, Form):
        yield value
        for child in value.args or ():
            yield from nodes(child)
        for _, child in value.fields or ():
            yield from nodes(child)
    elif isinstance(value, (list, tuple)):
        for child in value:
            yield from nodes(child)


def rewrite(value: Any) -> tuple[Any, int]:
    """Rewrite only JaiModal.inner_unit fields; count actual outer occurrences."""
    count = 0

    def descend(node: Any) -> Any:
        nonlocal count
        if isinstance(node, Form):
            args = None if node.args is None else tuple(descend(child) for child in node.args)
            named = None if node.fields is None else tuple((key, descend(child)) for key, child in node.fields)
            mapped = Form(node.name, fields=named, args=args)
            if node.name == "JaiModalTanruUnitSyntax":
                jai, tag, inner = fields(mapped, node.name, JAI_FIELDS)
                count += 1
                return product(node.name, jai=jai, tense_modal=tag, inner_unit=map_inner(inner))
            return mapped
        if isinstance(node, list):
            return [descend(child) for child in node]
        if isinstance(node, tuple):
            return tuple(descend(child) for child in node)
        return node

    return descend(value), count
