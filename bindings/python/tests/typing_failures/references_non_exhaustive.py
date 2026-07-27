"""An omitted reference-target payload must fail exhaustive narrowing."""

from typing import assert_never

from jbotci.semantics import references


def incomplete_target(value: references.ReferenceTarget) -> str:
    if isinstance(value, references.ResolvedNodeReferenceTarget):
        return str(value.node.value)
    if isinstance(value, references.ResolvedFrameReferenceTarget):
        return str(value.frame.value)
    if isinstance(value, references.AmbiguousNodesReferenceTarget):
        return str(len(value.nodes))
    if isinstance(value, references.UnresolvedReferenceTarget):
        return value.reason
    assert_never(value)
