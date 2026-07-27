"""Intentional variant, immutability, and analysis-scope typing failures."""

from jbotci import source, syntax
from jbotci.semantics import references


def wrong_variant_fields(
    end: syntax.SyntaxExpectedTokenEndOfInput,
    cmavo: syntax.SyntaxExpectedTokenCmavo,
) -> None:
    end.cmavo  # E: attr-defined has no attribute "cmavo"
    cmavo.name  # E: attr-defined has no attribute "name"


def mutate_immutable_values(
    span: source.SourceSpan,
    slot: references.NumberedPlaceSlot,
) -> None:
    span.byte_start = 1  # E: misc Property "byte_start"
    slot.place = 2  # E: misc Property "place"


def mix_analysis_id_families(
    places: references.PlaceAnalysis,
    frame: references.SelbriPlaceFrameId,
    assignment: references.SumtiPlaceAssignmentId,
    raw_node: references.RawSyntaxNodeId,
) -> None:
    places.frame(assignment)  # E: arg-type Argument 1 to "frame"
    places.assignment(frame)  # E: arg-type Argument 1 to "assignment"
    places.assignments_for_sumti(raw_node)  # E: arg-type Argument 1 to "assignments_for_sumti"
