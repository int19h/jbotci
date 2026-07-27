"""Typed place assignment and discourse-reference analysis.

``ReferenceAnalysis`` and its index/place/reference query objects are the
primary API. The fixture projection records and JSON helpers are intentionally
secondary, stable-shaped projections for corpus fixtures and debugging.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Protocol, TypeAlias, cast, final

from jbotci import _native as _rust
from jbotci._errors import _StructuredError
from jbotci.syntax import SyntaxParse
from jbotci.syntax.strict import TextSyntax

if TYPE_CHECKING:
    from jbotci import ParsedText


class SyntaxNode(Protocol):
    """Common typed protocol implemented by every generated strict syntax node."""

    def same_identity(self, other: object, /) -> bool:
        """Return whether two handles retain the same owner and tree path."""

RawSyntaxNodeId = _rust._references_RawSyntaxNodeId
TextNodeId = _rust._references_TextNodeId
ParagraphNodeId = _rust._references_ParagraphNodeId
StatementNodeId = _rust._references_StatementNodeId
BridiNodeId = _rust._references_BridiNodeId
BridiTailNodeId = _rust._references_BridiTailNodeId
SelbriNodeId = _rust._references_SelbriNodeId
TanruUnitNodeId = _rust._references_TanruUnitNodeId
TermNodeId = _rust._references_TermNodeId
SumtiNodeId = _rust._references_SumtiNodeId
FreeModifierNodeId = _rust._references_FreeModifierNodeId
AbstractionNodeId = _rust._references_AbstractionNodeId
MeksoNodeId = _rust._references_MeksoNodeId
MeksoOperatorNodeId = _rust._references_MeksoOperatorNodeId
SyntaxNodeMetadata = _rust._references_SyntaxNodeMetadata

SelbriPlaceFrameId = _rust._references_SelbriPlaceFrameId
SumtiPlaceAssignmentId = _rust._references_SumtiPlaceAssignmentId
ReferenceEdgeId = _rust._references_ReferenceEdgeId

NumberedPlaceSlot = _rust._references_NumberedPlaceSlot
ModalPlaceSlot = _rust._references_ModalPlaceSlot
PlaceQuestionPlaceSlot = _rust._references_PlaceQuestionPlaceSlot
FaiPlaceSlot = _rust._references_FaiPlaceSlot
PlaceSlot: TypeAlias = (
    NumberedPlaceSlot
    | ModalPlaceSlot
    | PlaceQuestionPlaceSlot
    | FaiPlaceSlot
)

PlaceFrameKind = _rust._references_PlaceFrameKind
NoPlaceFramePropagation = _rust._references_NoPlaceFramePropagation
ForwardPlaceFramePropagation = _rust._references_ForwardPlaceFramePropagation
ConversionPlaceFramePropagation = (
    _rust._references_ConversionPlaceFramePropagation
)
JaiPlaceFramePropagation = _rust._references_JaiPlaceFramePropagation
ConnectiveBranchesPlaceFramePropagation = (
    _rust._references_ConnectiveBranchesPlaceFramePropagation
)
CompoundPlaceFramePropagation = (
    _rust._references_CompoundPlaceFramePropagation
)
CoPlaceFramePropagation = _rust._references_CoPlaceFramePropagation
PlaceFramePropagation: TypeAlias = (
    NoPlaceFramePropagation
    | ForwardPlaceFramePropagation
    | ConversionPlaceFramePropagation
    | JaiPlaceFramePropagation
    | ConnectiveBranchesPlaceFramePropagation
    | CompoundPlaceFramePropagation
    | CoPlaceFramePropagation
)
SelbriPlaceFrame = _rust._references_SelbriPlaceFrame

AssignmentSource = _rust._references_AssignmentSource
SumtiPlaceAssignment = _rust._references_SumtiPlaceAssignment
ReferenceKind = _rust._references_ReferenceKind
VagueReferenceKind = _rust._references_VagueReferenceKind
ResolvedNodeReferenceTarget = _rust._references_ResolvedNodeReferenceTarget
ResolvedFrameReferenceTarget = _rust._references_ResolvedFrameReferenceTarget
AmbiguousNodesReferenceTarget = _rust._references_AmbiguousNodesReferenceTarget
UnresolvedReferenceTarget = _rust._references_UnresolvedReferenceTarget
VagueReferenceTarget = _rust._references_VagueReferenceTarget
ReferenceTarget: TypeAlias = (
    ResolvedNodeReferenceTarget
    | ResolvedFrameReferenceTarget
    | AmbiguousNodesReferenceTarget
    | UnresolvedReferenceTarget
    | VagueReferenceTarget
)
ReferenceRule = _rust._references_ReferenceRule
ReferenceEdge = _rust._references_ReferenceEdge

MissingRootNode = _rust._references_MissingRootNode
ReferenceAnalysisErrorValue: TypeAlias = MissingRootNode
GeneratedSyntaxIndex = _rust._references_GeneratedSyntaxIndex
PlaceAnalysis = _rust._references_PlaceAnalysis
DiscourseReferences = _rust._references_DiscourseReferences
ReferenceAnalysis = _rust._references_ReferenceAnalysis


def numbered(place: int) -> NumberedPlaceSlot:
    """Construct a numbered place slot through the Rust ``PlaceSlot`` invariant."""

    return NumberedPlaceSlot(place)


_EXCEPTION_SUBCLASS_TOKEN = object()


class ReferenceAnalysisError(
    _StructuredError[ReferenceAnalysisErrorValue]
):
    """Base class for the closed structured reference-analysis errors."""

    __slots__ = ()

    def __init__(self, value: ReferenceAnalysisErrorValue) -> None:
        if type(self) is ReferenceAnalysisError:
            raise TypeError("ReferenceAnalysisError is an abstract variant base")
        super().__init__(value)

    def __init_subclass__(
        cls, *, _token: object | None = None
    ) -> None:
        if _token is not _EXCEPTION_SUBCLASS_TOKEN:
            raise TypeError(
                "ReferenceAnalysisError has a closed exception hierarchy"
            )
        super().__init_subclass__()


@final
class MissingRootNodeError(
    ReferenceAnalysisError, _token=_EXCEPTION_SUBCLASS_TOKEN
):
    """The generated syntax index did not contain its root text node."""

    __slots__ = ()

    def __init__(self, value: MissingRootNode) -> None:
        if not isinstance(value, MissingRootNode):
            raise TypeError("value must be MissingRootNode")
        super().__init__(value)

    @property
    def value(self) -> MissingRootNode:
        """Return the exact structured Rust error value."""

        return super().value

    def __init_subclass__(cls) -> None:
        raise TypeError("MissingRootNodeError is final")


@final
@dataclass(frozen=True, slots=True)
class FixtureSpanKey:
    """Visible byte range used by reference fixtures."""

    offset: int = field(metadata={"doc": "Inclusive UTF-8 byte offset."})
    length: int = field(metadata={"doc": "UTF-8 byte length."})


@final
@dataclass(frozen=True, slots=True)
class FixtureNoPlaceFramePropagation:
    """Fixture frame with no propagation payload."""


@final
@dataclass(frozen=True, slots=True)
class FixtureForwardPlaceFramePropagation:
    """Fixture forwarding frame."""

    inner: int


@final
@dataclass(frozen=True, slots=True)
class FixtureConversionPlaceFramePropagation:
    """Fixture SE conversion frame."""

    inner: int
    converted_place: int


@final
@dataclass(frozen=True, slots=True)
class FixtureJaiPlaceFramePropagation:
    """Fixture JAI conversion frame."""

    inner: int


@final
@dataclass(frozen=True, slots=True)
class FixtureConnectiveBranchesPlaceFramePropagation:
    """Fixture connective frame."""

    branches: tuple[int, ...]


@final
@dataclass(frozen=True, slots=True)
class FixtureCompoundPlaceFramePropagation:
    """Fixture compound selbri frame."""

    head: int
    modifiers: tuple[int, ...]


@final
@dataclass(frozen=True, slots=True)
class FixtureCoPlaceFramePropagation:
    """Fixture CO-inverted frame."""

    leading: int
    trailing: int


FixturePlaceFramePropagation: TypeAlias = (
    FixtureNoPlaceFramePropagation
    | FixtureForwardPlaceFramePropagation
    | FixtureConversionPlaceFramePropagation
    | FixtureJaiPlaceFramePropagation
    | FixtureConnectiveBranchesPlaceFramePropagation
    | FixtureCompoundPlaceFramePropagation
    | FixtureCoPlaceFramePropagation
)


@final
@dataclass(frozen=True, slots=True)
class FixtureNumberedPlaceSlot:
    """Fixture numbered place slot."""

    place: int


@final
@dataclass(frozen=True, slots=True)
class FixtureModalPlaceSlot:
    """Fixture modal place slot."""

    tag: FixtureSpanKey | None


@final
@dataclass(frozen=True, slots=True)
class FixturePlaceQuestionPlaceSlot:
    """Fixture place-question slot."""


@final
@dataclass(frozen=True, slots=True)
class FixtureFaiPlaceSlot:
    """Fixture FAI slot."""


FixturePlaceSlot: TypeAlias = (
    FixtureNumberedPlaceSlot
    | FixtureModalPlaceSlot
    | FixturePlaceQuestionPlaceSlot
    | FixtureFaiPlaceSlot
)


@final
@dataclass(frozen=True, slots=True)
class FixturePlaceFrame:
    """One fixture/debug projection of a selbri place frame."""

    index: int = field(metadata={"doc": "Stable fixture frame index."})
    node: FixtureSpanKey = field(metadata={"doc": "Owning syntax-node span."})
    kind: PlaceFrameKind = field(metadata={"doc": "Place-frame kind."})
    selbri: FixtureSpanKey | None = field(
        metadata={"doc": "Associated selbri span when present."}
    )
    tanru_unit: FixtureSpanKey | None = field(
        metadata={"doc": "Associated tanru-unit span when present."}
    )
    propagation: FixturePlaceFramePropagation = field(
        metadata={"doc": "Typed place-frame propagation."}
    )


@final
@dataclass(frozen=True, slots=True)
class FixtureSumtiAssignment:
    """One fixture/debug projection of a sumti-place assignment."""

    frame: int = field(metadata={"doc": "Target frame index."})
    frame_node: FixtureSpanKey = field(metadata={"doc": "Target frame-node span."})
    selbri: FixtureSpanKey | None = field(
        metadata={"doc": "Associated selbri span when present."}
    )
    tanru_unit: FixtureSpanKey | None = field(
        metadata={"doc": "Associated tanru-unit span when present."}
    )
    slot: FixturePlaceSlot = field(metadata={"doc": "Assigned typed place slot."})
    sumti: FixtureSpanKey = field(metadata={"doc": "Assigned sumti span."})
    term: FixtureSpanKey | None = field(
        metadata={"doc": "Originating term span when present."}
    )
    source: AssignmentSource = field(
        metadata={"doc": "Rule source that established the assignment."}
    )


@final
@dataclass(frozen=True, slots=True)
class FixtureSelbriPlace:
    """One fixture/debug relation-place projection."""

    frame: int = field(metadata={"doc": "Target frame index."})
    selbri: FixtureSpanKey = field(metadata={"doc": "Selbri span."})
    place: int = field(metadata={"doc": "One-based numbered place."})
    sumti: FixtureSpanKey = field(metadata={"doc": "Assigned sumti span."})


@final
@dataclass(frozen=True, slots=True)
class FixtureResolvedNodeReferenceTarget:
    """Fixture target resolved to a visible node span."""

    node: FixtureSpanKey


@final
@dataclass(frozen=True, slots=True)
class FixtureResolvedFrameReferenceTarget:
    """Fixture target resolved to a place frame."""

    frame: int
    frame_node: FixtureSpanKey


@final
@dataclass(frozen=True, slots=True)
class FixtureAmbiguousNodesReferenceTarget:
    """Fixture target retaining multiple visible candidates."""

    nodes: tuple[FixtureSpanKey, ...]


@final
@dataclass(frozen=True, slots=True)
class FixtureUnresolvedReferenceTarget:
    """Fixture target retaining the core unresolved reason."""

    reason: str


@final
@dataclass(frozen=True, slots=True)
class FixtureVagueReferenceTarget:
    """Fixture target retaining an intentional vague-reference kind."""

    vague_kind: VagueReferenceKind


FixtureReferenceTarget: TypeAlias = (
    FixtureResolvedNodeReferenceTarget
    | FixtureResolvedFrameReferenceTarget
    | FixtureAmbiguousNodesReferenceTarget
    | FixtureUnresolvedReferenceTarget
    | FixtureVagueReferenceTarget
)


@final
@dataclass(frozen=True, slots=True)
class FixtureReferenceEdge:
    """One fixture/debug projection of a discourse-reference edge."""

    kind: ReferenceKind = field(metadata={"doc": "Reference kind."})
    source: FixtureSpanKey = field(metadata={"doc": "Reference source span."})
    target: FixtureReferenceTarget = field(
        metadata={"doc": "Typed reference target."}
    )


@final
@dataclass(frozen=True, slots=True)
class ReferenceFixtureProjection:
    """Immutable fixture/debug projection; not the primary analysis model."""

    frames: tuple[FixturePlaceFrame, ...] = field(
        metadata={"doc": "Place frames in stable fixture order."}
    )
    assignments: tuple[FixtureSumtiAssignment, ...] = field(
        metadata={"doc": "Sumti assignments in stable fixture order."}
    )
    selbri_places: tuple[FixtureSelbriPlace, ...] = field(
        metadata={"doc": "Derived selbri-place relations."}
    )
    references: tuple[FixtureReferenceEdge, ...] = field(
        metadata={"doc": "Discourse-reference edges."}
    )


def _mapping(value: object) -> dict[str, object]:
    if not isinstance(value, dict):
        raise ValueError("fixture projection member must be an object")
    return cast(dict[str, object], value)


def _sequence(value: object) -> list[object]:
    if not isinstance(value, list):
        raise ValueError("fixture projection member must be an array")
    return cast(list[object], value)


def _string(value: object) -> str:
    if not isinstance(value, str):
        raise ValueError("fixture projection member must be a string")
    return value


def _integer(value: object) -> int:
    if not isinstance(value, int):
        raise ValueError("fixture projection member must be an integer")
    return value


def _optional_span(value: object) -> FixtureSpanKey | None:
    return None if value is None else _span(value)


def _span(value: object) -> FixtureSpanKey:
    item = _mapping(value)
    return FixtureSpanKey(
        offset=_integer(item["offset"]),
        length=_integer(item["length"]),
    )


def _fixture_propagation(
    value: object,
) -> FixturePlaceFramePropagation:
    item = _mapping(value)
    kind = _string(item["kind"])
    if kind == "none":
        return FixtureNoPlaceFramePropagation()
    if kind == "forward":
        return FixtureForwardPlaceFramePropagation(
            inner=_integer(item["inner"])
        )
    if kind == "conversion":
        return FixtureConversionPlaceFramePropagation(
            inner=_integer(item["inner"]),
            converted_place=_integer(item["converted_place"]),
        )
    if kind == "jai":
        return FixtureJaiPlaceFramePropagation(
            inner=_integer(item["inner"])
        )
    if kind == "connective-branches":
        return FixtureConnectiveBranchesPlaceFramePropagation(
            branches=tuple(
                _integer(branch) for branch in _sequence(item["branches"])
            )
        )
    if kind == "compound":
        return FixtureCompoundPlaceFramePropagation(
            head=_integer(item["head"]),
            modifiers=tuple(
                _integer(modifier)
                for modifier in _sequence(item["modifiers"])
            ),
        )
    if kind == "co":
        return FixtureCoPlaceFramePropagation(
            leading=_integer(item["leading"]),
            trailing=_integer(item["trailing"]),
        )
    raise ValueError(f"unknown fixture propagation kind: {kind}")


def _fixture_slot(value: object) -> FixturePlaceSlot:
    item = _mapping(value)
    kind = _string(item["kind"])
    if kind == "numbered":
        return FixtureNumberedPlaceSlot(place=_integer(item["place"]))
    if kind == "modal":
        return FixtureModalPlaceSlot(tag=_optional_span(item["tag"]))
    if kind == "place-question":
        return FixturePlaceQuestionPlaceSlot()
    if kind == "fai":
        return FixtureFaiPlaceSlot()
    raise ValueError(f"unknown fixture place-slot kind: {kind}")


def _fixture_target(value: object) -> FixtureReferenceTarget:
    item = _mapping(value)
    kind = _string(item["kind"])
    if kind == "resolved-node":
        return FixtureResolvedNodeReferenceTarget(node=_span(item["node"]))
    if kind == "resolved-frame":
        return FixtureResolvedFrameReferenceTarget(
            frame=_integer(item["frame"]),
            frame_node=_span(item["frame_node"]),
        )
    if kind == "ambiguous-nodes":
        return FixtureAmbiguousNodesReferenceTarget(
            nodes=tuple(
                _span(node) for node in _sequence(item["nodes"])
            )
        )
    if kind == "unresolved":
        return FixtureUnresolvedReferenceTarget(
            reason=_string(item["reason"])
        )
    if kind == "vague":
        return FixtureVagueReferenceTarget(
            vague_kind=VagueReferenceKind(_string(item["vague_kind"]))
        )
    raise ValueError(f"unknown fixture reference-target kind: {kind}")


def _fixture_projection_from_json(data: str) -> ReferenceFixtureProjection:
    """Build typed immutable records from the core's canonical fixture JSON."""

    root = _mapping(cast(object, json.loads(data)))
    frames = tuple(
        FixturePlaceFrame(
            index=_integer(item["index"]),
            node=_span(item["node"]),
            kind=PlaceFrameKind(_string(item["kind"])),
            selbri=_optional_span(item["selbri"]),
            tanru_unit=_optional_span(item["tanru-unit"]),
            propagation=_fixture_propagation(item["propagation"]),
        )
        for item in (
            _mapping(value) for value in _sequence(root["frames"])
        )
    )
    assignments = tuple(
        FixtureSumtiAssignment(
            frame=_integer(item["frame"]),
            frame_node=_span(item["frame-node"]),
            selbri=_optional_span(item["selbri"]),
            tanru_unit=_optional_span(item["tanru-unit"]),
            slot=_fixture_slot(item["slot"]),
            sumti=_span(item["sumti"]),
            term=_optional_span(item["term"]),
            source=AssignmentSource(_string(item["source"])),
        )
        for item in (
            _mapping(value) for value in _sequence(root["assignments"])
        )
    )
    selbri_places = tuple(
        FixtureSelbriPlace(
            frame=_integer(item["frame"]),
            selbri=_span(item["selbri"]),
            place=_integer(item["place"]),
            sumti=_span(item["sumti"]),
        )
        for item in (
            _mapping(value) for value in _sequence(root["selbri-places"])
        )
    )
    references = tuple(
        FixtureReferenceEdge(
            kind=ReferenceKind(_string(item["kind"])),
            source=_span(item["source"]),
            target=_fixture_target(item["target"]),
        )
        for item in (
            _mapping(value) for value in _sequence(root["references"])
        )
    )
    return ReferenceFixtureProjection(
        frames=frames,
        assignments=assignments,
        selbri_places=selbri_places,
        references=references,
    )


def analyze_references(
    tree_or_parse: TextSyntax | SyntaxParse | ParsedText,
) -> ReferenceAnalysis:
    """Analyze an existing typed strict tree or successful parse without reparsing."""

    from jbotci import ParsedText

    if isinstance(tree_or_parse, ParsedText):
        return _rust._references_analyze_references(tree_or_parse.syntax)
    return _rust._references_analyze_references(tree_or_parse)


def fixture_projection(
    analysis: ReferenceAnalysis,
) -> ReferenceFixtureProjection:
    """Return the secondary typed fixture/debug projection."""

    return analysis.fixture_projection()


def fixture_projection_json(analysis: ReferenceAnalysis) -> str:
    """Return canonical fixture/debug JSON from the Rust projection."""

    return analysis.fixture_projection_json()


__all__: tuple[str, ...] = (
    "RawSyntaxNodeId",
    "TextNodeId",
    "ParagraphNodeId",
    "StatementNodeId",
    "BridiNodeId",
    "BridiTailNodeId",
    "SelbriNodeId",
    "TanruUnitNodeId",
    "TermNodeId",
    "SumtiNodeId",
    "FreeModifierNodeId",
    "AbstractionNodeId",
    "MeksoNodeId",
    "MeksoOperatorNodeId",
    "SyntaxNode",
    "SyntaxNodeMetadata",
    "SelbriPlaceFrameId",
    "SumtiPlaceAssignmentId",
    "ReferenceEdgeId",
    "NumberedPlaceSlot",
    "ModalPlaceSlot",
    "PlaceQuestionPlaceSlot",
    "FaiPlaceSlot",
    "PlaceSlot",
    "numbered",
    "PlaceFrameKind",
    "NoPlaceFramePropagation",
    "ForwardPlaceFramePropagation",
    "ConversionPlaceFramePropagation",
    "JaiPlaceFramePropagation",
    "ConnectiveBranchesPlaceFramePropagation",
    "CompoundPlaceFramePropagation",
    "CoPlaceFramePropagation",
    "PlaceFramePropagation",
    "SelbriPlaceFrame",
    "AssignmentSource",
    "SumtiPlaceAssignment",
    "ReferenceKind",
    "VagueReferenceKind",
    "ResolvedNodeReferenceTarget",
    "ResolvedFrameReferenceTarget",
    "AmbiguousNodesReferenceTarget",
    "UnresolvedReferenceTarget",
    "VagueReferenceTarget",
    "ReferenceTarget",
    "ReferenceRule",
    "ReferenceEdge",
    "MissingRootNode",
    "ReferenceAnalysisErrorValue",
    "ReferenceAnalysisError",
    "MissingRootNodeError",
    "GeneratedSyntaxIndex",
    "PlaceAnalysis",
    "DiscourseReferences",
    "ReferenceAnalysis",
    "FixtureSpanKey",
    "FixturePlaceFramePropagation",
    "FixturePlaceSlot",
    "FixturePlaceFrame",
    "FixtureSumtiAssignment",
    "FixtureSelbriPlace",
    "FixtureReferenceTarget",
    "FixtureReferenceEdge",
    "ReferenceFixtureProjection",
    "analyze_references",
    "fixture_projection",
    "fixture_projection_json",
)
