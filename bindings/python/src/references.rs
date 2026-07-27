//! Typed Python bindings for place assignment and discourse-reference analysis.

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU8;
use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use jbotci_semantics::references::{
    AssignmentSource, GeneratedReferenceAnalysis as RustGeneratedReferenceAnalysis, PlaceFrameKind,
    PlaceFramePropagation, PlaceSlot, ReferenceAnalysisError as RustReferenceError, ReferenceKind,
    ReferenceRule, ReferenceTarget, SelbriPlaceFrame, SumtiPlaceAssignment, VagueReferenceKind,
};
use jbotci_syntax::generated_model::TreeNode as GeneratedSyntaxTreeNode;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyTuple};
use self_cell::self_cell;

use crate::InvalidInputError;
use crate::parser::strict_parse_root_from_python;
use crate::source::PySourceSpan;
use crate::support::{
    PythonStringEnum, public_exception_with_value, register_private_object, register_string_enum,
    register_type, sequence_to_tuple, string_enum_member,
};
use crate::syntax::{
    StrictSyntaxModel, StrictTextRootHandle, extract_syntax_value, strict_text_to_python,
    wrap_syntax_value,
};

const PUBLIC_MODULE: &str = "jbotci.semantics.references";

/// Ordered inventory of native names owned by the reference-analysis domain.
pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_references_RawSyntaxNodeId",
    "_references_TextNodeId",
    "_references_ParagraphNodeId",
    "_references_StatementNodeId",
    "_references_BridiNodeId",
    "_references_BridiTailNodeId",
    "_references_SelbriNodeId",
    "_references_TanruUnitNodeId",
    "_references_TermNodeId",
    "_references_SumtiNodeId",
    "_references_FreeModifierNodeId",
    "_references_AbstractionNodeId",
    "_references_MeksoNodeId",
    "_references_MeksoOperatorNodeId",
    "_references_SyntaxNodeMetadata",
    "_references_SelbriPlaceFrameId",
    "_references_SumtiPlaceAssignmentId",
    "_references_ReferenceEdgeId",
    "_references_NumberedPlaceSlot",
    "_references_ModalPlaceSlot",
    "_references_PlaceQuestionPlaceSlot",
    "_references_FaiPlaceSlot",
    "_references_PlaceFrameKind",
    "_references_NoPlaceFramePropagation",
    "_references_ForwardPlaceFramePropagation",
    "_references_ConversionPlaceFramePropagation",
    "_references_JaiPlaceFramePropagation",
    "_references_ConnectiveBranchesPlaceFramePropagation",
    "_references_CompoundPlaceFramePropagation",
    "_references_CoPlaceFramePropagation",
    "_references_SelbriPlaceFrame",
    "_references_AssignmentSource",
    "_references_SumtiPlaceAssignment",
    "_references_ReferenceKind",
    "_references_VagueReferenceKind",
    "_references_ResolvedNodeReferenceTarget",
    "_references_ResolvedFrameReferenceTarget",
    "_references_AmbiguousNodesReferenceTarget",
    "_references_UnresolvedReferenceTarget",
    "_references_VagueReferenceTarget",
    "_references_ReferenceRule",
    "_references_ReferenceEdge",
    "_references_MissingRootNode",
    "_references_GeneratedSyntaxIndex",
    "_references_PlaceAnalysis",
    "_references_DiscourseReferences",
    "_references_ReferenceAnalysis",
    "_references_analyze_references",
    "_references_RUNTIME_INVENTORY",
];

macro_rules! define_reference_string_enum_binding {
    (
        $type:ty,
        $native_name:literal,
        $python_name:literal,
        $doc:literal,
        { $($variant:path => ($member:literal, $value:literal)),+ $(,)? }
    ) => {
        #[contract_trait]
        impl PythonStringEnum for $type {
            fn native_export_name() -> &'static str {
                $native_name
            }

            fn python_type_name() -> &'static str {
                $python_name
            }

            fn python_module_name() -> &'static str {
                PUBLIC_MODULE
            }

            fn python_doc() -> &'static str {
                $doc
            }

            fn variants() -> &'static [Self] {
                const VALUES: &[$type] = &[$($variant),+];
                VALUES
            }

            fn python_member_name(self) -> Cow<'static, str> {
                match self {
                    $($variant => Cow::Borrowed($member)),+
                }
            }

            fn python_value(self) -> &'static str {
                match self {
                    $($variant => $value),+
                }
            }
        }
    };
}

define_reference_string_enum_binding!(
    PlaceFrameKind,
    "_references_PlaceFrameKind",
    "PlaceFrameKind",
    "Structural role of a selbri place frame.",
    {
        PlaceFrameKind::Bridi => ("BRIDI", "bridi"),
        PlaceFrameKind::BridiTail => ("BRIDI_TAIL", "bridi-tail"),
        PlaceFrameKind::BaseSelbri => ("BASE_SELBRI", "base-selbri"),
        PlaceFrameKind::TanruUnit => ("TANRU_UNIT", "tanru-unit"),
        PlaceFrameKind::Converted => ("CONVERTED", "converted"),
        PlaceFrameKind::JaiConverted => ("JAI_CONVERTED", "jai-converted"),
        PlaceFrameKind::LinkedUnit => ("LINKED_UNIT", "linked-unit"),
        PlaceFrameKind::ConnectiveBranching => ("CONNECTIVE_BRANCHING", "connective-branching"),
        PlaceFrameKind::Compound => ("COMPOUND", "compound"),
        PlaceFrameKind::CoInverted => ("CO_INVERTED", "co-inverted"),
        PlaceFrameKind::Forwarding => ("FORWARDING", "forwarding"),
        PlaceFrameKind::Abstraction => ("ABSTRACTION", "abstraction"),
        PlaceFrameKind::ProBridi => ("PRO_BRIDI", "pro-bridi"),
        PlaceFrameKind::Unknown => ("UNKNOWN", "unknown"),
    }
);

define_reference_string_enum_binding!(
    AssignmentSource,
    "_references_AssignmentSource",
    "AssignmentSource",
    "Grammar path that produced one sumti-place assignment.",
    {
        AssignmentSource::SequentialTerm => ("SEQUENTIAL_TERM", "sequential-term"),
        AssignmentSource::FaTerm => ("FA_TERM", "fa-term"),
        AssignmentSource::ModalTerm => ("MODAL_TERM", "modal-term"),
        AssignmentSource::LinkedSumti => ("LINKED_SUMTI", "linked-sumti"),
        AssignmentSource::CoSeltauTerm => ("CO_SELTAU_TERM", "co-seltau-term"),
        AssignmentSource::TermsetBranch => ("TERMSET_BRANCH", "termset-branch"),
        AssignmentSource::SharedHeadTerm => ("SHARED_HEAD_TERM", "shared-head-term"),
        AssignmentSource::SharedTailTerm => ("SHARED_TAIL_TERM", "shared-tail-term"),
        AssignmentSource::Propagated => ("PROPAGATED", "propagated"),
    }
);

define_reference_string_enum_binding!(
    ReferenceKind,
    "_references_ReferenceKind",
    "ReferenceKind",
    "Semantic family of one discourse-reference edge.",
    {
        ReferenceKind::SumtiAssociation => ("SUMTI_ASSOCIATION", "sumti-association"),
        ReferenceKind::RelativePhraseHead => ("RELATIVE_PHRASE_HEAD", "relative-phrase-head"),
        ReferenceKind::RelativePhraseArgument => ("RELATIVE_PHRASE_ARGUMENT", "relative-phrase-argument"),
        ReferenceKind::ProBridiAssignment => ("PRO_BRIDI_ASSIGNMENT", "pro-bridi-assignment"),
        ReferenceKind::Koha => ("KOHA", "koha"),
        ReferenceKind::Ri => ("RI", "ri"),
        ReferenceKind::Cehu => ("CEHU", "cehu"),
        ReferenceKind::Letter => ("LETTER", "letter"),
        ReferenceKind::Ra => ("RA", "ra"),
        ReferenceKind::Ru => ("RU", "ru"),
        ReferenceKind::Keha => ("KEHA", "keha"),
        ReferenceKind::VohaSeries => ("VOHA_SERIES", "voha-series"),
        ReferenceKind::DaSeries => ("DA_SERIES", "da-series"),
        ReferenceKind::BrodaSeries => ("BRODA_SERIES", "broda-series"),
        ReferenceKind::GohaSeries => ("GOHA_SERIES", "goha-series"),
        ReferenceKind::Utterance => ("UTTERANCE", "utterance"),
    }
);

define_reference_string_enum_binding!(
    VagueReferenceKind,
    "_references_VagueReferenceKind",
    "VagueReferenceKind",
    "Intentionally unresolved vague-reference family.",
    {
        VagueReferenceKind::DistantSumti => ("DISTANT_SUMTI", "distant-sumti"),
        VagueReferenceKind::RecentSumti => ("RECENT_SUMTI", "recent-sumti"),
        VagueReferenceKind::Bridi => ("BRIDI", "bridi"),
    }
);

define_reference_string_enum_binding!(
    ReferenceRule,
    "_references_ReferenceRule",
    "ReferenceRule",
    "Exact resolver rule that produced one discourse-reference edge.",
    {
        ReferenceRule::DiheFollowingWhenPresent => ("DIHE_FOLLOWING_WHEN_PRESENT", "di'e refers to the following utterance when one is present"),
        ReferenceRule::DiheFollowing => ("DIHE_FOLLOWING", "di'e refers to the following utterance"),
        ReferenceRule::PrenexCeiAssignment => ("PRENEX_CEI_ASSIGNMENT", "prenex CEI assignment binds the following bridi"),
        ReferenceRule::LetteralProSumtiLatestInitial => ("LETTERAL_PRO_SUMTI_LATEST_INITIAL", "letteral pro-sumti resolves to the latest sumti with the same initial string"),
        ReferenceRule::GoiEquatesHead => ("GOI_EQUATES_HEAD", "GOI relative clause equates its sumti with the relative-clause head"),
        ReferenceRule::GoiAssignsHeadProSumti => ("GOI_ASSIGNS_HEAD_PRO_SUMTI", "GOI assigns the relative-clause head pro-sumti to its sumti"),
        ReferenceRule::GoiX1RelativeHead => ("GOI_X1_RELATIVE_HEAD", "GOI relative phrase marker relates x1 to the relative-clause head"),
        ReferenceRule::GoiX2AttachedSumti => ("GOI_X2_ATTACHED_SUMTI", "GOI relative phrase marker relates x2 to the attached sumti"),
        ReferenceRule::CeiAssignsEnclosingBridi => ("CEI_ASSIGNS_ENCLOSING_BRIDI", "CEI assigns a pro-bridi word to the enclosing bridi"),
        ReferenceRule::WrappedRiReferenceSource => ("WRAPPED_RI_REFERENCE_SOURCE", "wrapped ri exposes the complete sumti as a reference source"),
        ReferenceRule::WrappedKehaReferenceSource => ("WRAPPED_KEHA_REFERENCE_SOURCE", "wrapped ke'a exposes the complete sumti as a reference source"),
        ReferenceRule::RiPreviousSumti => ("RI_PREVIOUS_SUMTI", "ri repeats the previous complete sumti"),
        ReferenceRule::CehuCurrentAbstraction => ("CEHU_CURRENT_ABSTRACTION", "ce'u refers to the current abstraction"),
        ReferenceRule::RaVague => ("RA_VAGUE", "ra is intentionally vague and is not resolved heuristically"),
        ReferenceRule::RuVague => ("RU_VAGUE", "ru is intentionally vague and is not resolved heuristically"),
        ReferenceRule::KehaCurrentRelativeHead => ("KEHA_CURRENT_RELATIVE_HEAD", "ke'a refers to the current relative-clause head"),
        ReferenceRule::NeighborUtteranceByForm => ("NEIGHBOR_UTTERANCE_BY_FORM", "utterance pro-sumti resolves to a neighboring utterance when determined by form"),
        ReferenceRule::VohaCurrentBridiPlace => ("VOHA_CURRENT_BRIDI_PLACE", "vo'a-series refers to a place of the current bridi"),
        ReferenceRule::DaActiveVariableBinding => ("DA_ACTIVE_VARIABLE_BINDING", "later da/de/di mentions refer to the active variable binding"),
        ReferenceRule::KohaGoiBinding => ("KOHA_GOI_BINDING", "KOhA resolves through an explicit GOI binding"),
        ReferenceRule::GohiPreviousBridi => ("GOHI_PREVIOUS_BRIDI", "go'i repeats the previous bridi"),
        ReferenceRule::GoheSecondPriorBridi => ("GOHE_SECOND_PRIOR_BRIDI", "go'e repeats the second-prior bridi"),
        ReferenceRule::GohaUnresolvedContextSensitive => ("GOHA_UNRESOLVED_CONTEXT_SENSITIVE", "this GOhA form is context-sensitive and is not resolved heuristically"),
        ReferenceRule::NeiCurrentBridi => ("NEI_CURRENT_BRIDI", "nei refers to the current bridi"),
        ReferenceRule::NohaOuterBridi => ("NOHA_OUTER_BRIDI", "no'a refers to an outer bridi"),
        ReferenceRule::PrenexBindingProSelbri => ("PRENEX_BINDING_PRO_SELBRI", "prenex binding resolves this pro-selbri word"),
        ReferenceRule::CeiBindingProBridi => ("CEI_BINDING_PRO_BRIDI", "CEI binding resolves this pro-bridi word"),
        ReferenceRule::CeiBindingBroda => ("CEI_BINDING_BRODA", "CEI binding resolves this broda-series bridi"),
    }
);

#[requires(true)]
#[ensures(true)]
fn native_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn enum_to_python<E: PythonStringEnum>(py: Python<'_>, value: E) -> PyResult<Py<PyAny>> {
    string_enum_member(&native_module(py)?, value).map(Bound::unbind)
}

/// Opaque identity shared by every ID produced by one analysis.
#[invariant(true, "Arc identity is unique while any scoped ID remains alive")]
#[derive(Debug, Clone)]
struct AnalysisToken(Arc<()>);

impl AnalysisToken {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        Self(Arc::new(()))
    }
}

impl PartialEq for AnalysisToken {
    #[requires(true)]
    #[ensures(ret == Arc::ptr_eq(&self.0, &other.0))]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for AnalysisToken {}

impl Hash for AnalysisToken {
    #[requires(true)]
    #[ensures(true)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

macro_rules! define_scoped_id {
    ($rust_name:ident, $python_name:literal) => {
        #[invariant(true, "the opaque token scopes the integer to one analysis")]
        #[pyclass(name = $python_name, frozen, eq, hash, module = "jbotci.semantics.references", skip_from_py_object)]
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct $rust_name {
            value: usize,
            token: AnalysisToken,
        }

        impl $rust_name {
            #[requires(true)]
            #[ensures(ret.value == value)]
            fn scoped(value: usize, token: &AnalysisToken) -> Self {
                Self {
                    value,
                    token: token.clone(),
                }
            }
        }

        #[pymethods]
        impl $rust_name {
            #[classattr]
            #[allow(non_upper_case_globals)]
            const __match_args__: (&'static str,) = ("value",);

            #[requires(true)]
            #[ensures(ret == self.value)]
            #[getter]
            fn value(&self) -> usize {
                self.value
            }

            #[requires(true)]
            #[ensures(true)]
            fn __repr__(&self) -> String {
                format!("{PUBLIC_MODULE}.{}({})", $python_name, self.value)
            }
        }
    };
}

define_scoped_id!(PyRawSyntaxNodeId, "RawSyntaxNodeId");
define_scoped_id!(PySelbriPlaceFrameId, "SelbriPlaceFrameId");
define_scoped_id!(PySumtiPlaceAssignmentId, "SumtiPlaceAssignmentId");
define_scoped_id!(PyReferenceEdgeId, "ReferenceEdgeId");

macro_rules! define_typed_syntax_id {
    ($rust_name:ident, $python_name:literal) => {
        #[invariant(true, "the opaque token scopes the typed node index to one analysis")]
        #[pyclass(name = $python_name, frozen, eq, hash, module = "jbotci.semantics.references", skip_from_py_object)]
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct $rust_name {
            value: usize,
            token: AnalysisToken,
        }

        impl $rust_name {
            #[requires(true)]
            #[ensures(ret.value == value)]
            fn scoped(value: usize, token: &AnalysisToken) -> Self {
                Self {
                    value,
                    token: token.clone(),
                }
            }
        }

        #[pymethods]
        impl $rust_name {
            #[classattr]
            #[allow(non_upper_case_globals)]
            const __match_args__: (&'static str,) = ("value",);

            #[requires(true)]
            #[ensures(ret == self.value)]
            #[getter]
            fn value(&self) -> usize {
                self.value
            }

            /// Return the same index with its grammar-family type erased.
            #[requires(true)]
            #[ensures(ret.value == self.value)]
            #[getter]
            fn raw_id(&self) -> PyRawSyntaxNodeId {
                PyRawSyntaxNodeId::scoped(self.value, &self.token)
            }

            #[requires(true)]
            #[ensures(true)]
            fn __repr__(&self) -> String {
                format!("{PUBLIC_MODULE}.{}({})", $python_name, self.value)
            }
        }
    };
}

define_typed_syntax_id!(PyTextNodeId, "TextNodeId");
define_typed_syntax_id!(PyParagraphNodeId, "ParagraphNodeId");
define_typed_syntax_id!(PyStatementNodeId, "StatementNodeId");
define_typed_syntax_id!(PyBridiNodeId, "BridiNodeId");
define_typed_syntax_id!(PyBridiTailNodeId, "BridiTailNodeId");
define_typed_syntax_id!(PySelbriNodeId, "SelbriNodeId");
define_typed_syntax_id!(PyTanruUnitNodeId, "TanruUnitNodeId");
define_typed_syntax_id!(PyTermNodeId, "TermNodeId");
define_typed_syntax_id!(PySumtiNodeId, "SumtiNodeId");
define_typed_syntax_id!(PyFreeModifierNodeId, "FreeModifierNodeId");
define_typed_syntax_id!(PyAbstractionNodeId, "AbstractionNodeId");
define_typed_syntax_id!(PyMeksoNodeId, "MeksoNodeId");
define_typed_syntax_id!(PyMeksoOperatorNodeId, "MeksoOperatorNodeId");

/// Borrowed core analysis tied to the exact generated text owner.
#[invariant(true, "GeneratedReferenceAnalysis borrows the self-cell owner")]
#[derive(Debug)]
struct BorrowedReferenceAnalysis<'tree> {
    analysis: RustGeneratedReferenceAnalysis<'tree>,
}

self_cell!(
    struct ReferenceCell {
        owner: StrictTextRootHandle,

        #[covariant]
        dependent: BorrowedReferenceAnalysis,
    }
);

/// Shared owning state retained by every analysis-derived Python wrapper.
#[invariant(
    true,
    "the self-cell keeps the exact strict tree alive for its analysis"
)]
struct ReferenceState {
    cell: ReferenceCell,
    token: AnalysisToken,
}

impl ReferenceState {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn new(root: StrictTextRootHandle) -> Result<Self, RustReferenceError> {
        let cell = ReferenceCell::try_new(root, |root| {
            RustGeneratedReferenceAnalysis::analyze(root.root())
                .map(|analysis| BorrowedReferenceAnalysis { analysis })
        })?;
        Ok(Self {
            cell,
            token: AnalysisToken::new(),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_analysis<R>(
        &self,
        function: impl FnOnce(&StrictTextRootHandle, &RustGeneratedReferenceAnalysis<'_>) -> R,
    ) -> R {
        self.cell
            .with_dependent(|root, dependent| function(root, &dependent.analysis))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn reference_root_from_python(value: &Bound<'_, PyAny>) -> PyResult<StrictTextRootHandle> {
    if let Ok(root) = strict_parse_root_from_python(value) {
        return Ok(root);
    }
    let handle = extract_syntax_value(value).map_err(|_| {
        PyTypeError::new_err("expected jbotci.syntax.SyntaxParse or strict TextSyntax")
    })?;
    StrictTextRootHandle::from_handle(handle).ok_or_else(|| {
        PyTypeError::new_err("expected jbotci.syntax.SyntaxParse or strict TextSyntax")
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_token(expected: &AnalysisToken, actual: &AnalysisToken, kind: &str) -> PyResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(InvalidInputError::new_err(format!(
            "{kind} belongs to a different ReferenceAnalysis"
        )))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn raw_id_value(state: &ReferenceState, id: &PyRawSyntaxNodeId) -> PyResult<usize> {
    validate_token(&state.token, &id.token, "node ID")?;
    Ok(id.value)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn frame_id_value(state: &ReferenceState, id: &PySelbriPlaceFrameId) -> PyResult<usize> {
    validate_token(&state.token, &id.token, "place-frame ID")?;
    Ok(id.value)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn assignment_id_value(state: &ReferenceState, id: &PySumtiPlaceAssignmentId) -> PyResult<usize> {
    validate_token(&state.token, &id.token, "place-assignment ID")?;
    Ok(id.value)
}

/// Exact index metadata for one generated syntax node.
#[invariant(true, "the retained Rust metadata enforces leaf and span ordering")]
#[pyclass(
    name = "SyntaxNodeMetadata",
    frozen,
    eq,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PySyntaxNodeMetadata {
    value: jbotci_semantics::references::SyntaxNodeMetadata,
    token: AnalysisToken,
}

impl PySyntaxNodeMetadata {
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    fn from_rust(
        value: jbotci_semantics::references::SyntaxNodeMetadata,
        token: &AnalysisToken,
    ) -> Self {
        Self {
            value,
            token: token.clone(),
        }
    }
}

#[pymethods]
impl PySyntaxNodeMetadata {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = (
        "id",
        "parent",
        "preorder",
        "depth",
        "leaf_start",
        "leaf_end",
        "first_source_span",
        "last_source_span",
    );

    #[requires(true)]
    #[ensures(ret.value == self.value.id.0)]
    #[getter]
    fn id(&self) -> PyRawSyntaxNodeId {
        PyRawSyntaxNodeId::scoped(self.value.id.0, &self.token)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn parent(&self) -> Option<PyRawSyntaxNodeId> {
        self.value
            .parent
            .map(|id| PyRawSyntaxNodeId::scoped(id.0, &self.token))
    }

    #[requires(true)]
    #[ensures(ret == self.value.preorder)]
    #[getter]
    fn preorder(&self) -> usize {
        self.value.preorder
    }

    #[requires(true)]
    #[ensures(ret == self.value.depth)]
    #[getter]
    fn depth(&self) -> usize {
        self.value.depth
    }

    #[requires(true)]
    #[ensures(ret == self.value.leaf_start)]
    #[getter]
    fn leaf_start(&self) -> usize {
        self.value.leaf_start
    }

    #[requires(true)]
    #[ensures(ret == self.value.leaf_end)]
    #[getter]
    fn leaf_end(&self) -> usize {
        self.value.leaf_end
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn first_source_span(&self) -> Option<PySourceSpan> {
        self.value
            .first_source_span
            .clone()
            .map(PySourceSpan::from_rust)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn last_source_span(&self) -> Option<PySourceSpan> {
        self.value
            .last_source_span
            .clone()
            .map(PySourceSpan::from_rust)
    }
}

/// Numbered place slot (`x1` through `x255`).
#[invariant(true, "construction and the core NonZeroU8 enforce a nonzero place")]
#[pyclass(
    name = "NumberedPlaceSlot",
    frozen,
    eq,
    hash,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PyNumberedPlaceSlot {
    place: u8,
}

#[pymethods]
impl PyNumberedPlaceSlot {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("place",);

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|slot| slot.place > 0) || ret.is_err())]
    #[new]
    fn new(place: i64) -> PyResult<Self> {
        let place = u8::try_from(place)
            .ok()
            .filter(|place| *place > 0)
            .ok_or_else(|| PyValueError::new_err("place must be between 1 and 255"))?;
        Ok(Self { place })
    }

    #[requires(true)]
    #[ensures(ret == self.place)]
    #[getter]
    fn place(&self) -> u8 {
        self.place
    }

    #[requires(true)]
    #[ensures(ret == Some(self.place))]
    fn numbered_index(&self) -> Option<u8> {
        Some(self.place)
    }
}

/// Modal place slot, optionally retaining the syntax node for its tag.
#[invariant(true, "None represents a modal whose tag has no indexed node")]
#[pyclass(
    name = "ModalPlaceSlot",
    frozen,
    eq,
    hash,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PyModalPlaceSlot {
    tag: Option<PyRawSyntaxNodeId>,
}

#[pymethods]
impl PyModalPlaceSlot {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("tag",);

    #[requires(true)]
    #[ensures(true)]
    #[new]
    #[pyo3(signature = (tag=None))]
    fn new(tag: Option<PyRef<'_, PyRawSyntaxNodeId>>) -> Self {
        Self {
            tag: tag.map(|tag| (*tag).clone()),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.tag)]
    #[getter]
    fn tag(&self) -> Option<PyRawSyntaxNodeId> {
        self.tag.clone()
    }

    #[requires(true)]
    #[ensures(ret.is_none())]
    fn numbered_index(&self) -> Option<u8> {
        None
    }
}

macro_rules! define_unit_place_slot {
    ($rust_name:ident, $python_name:literal) => {
        #[invariant(true, "fieldless payload variant carries no invalid state")]
        #[pyclass(name = $python_name, frozen, eq, hash, module = "jbotci.semantics.references", skip_from_py_object)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        struct $rust_name;

        #[pymethods]
        impl $rust_name {
            #[classattr]
            #[allow(non_upper_case_globals)]
            const __match_args__: () = ();

            #[requires(true)]
            #[ensures(true)]
            #[new]
            fn new() -> Self {
                Self
            }

            #[requires(true)]
            #[ensures(ret.is_none())]
            fn numbered_index(&self) -> Option<u8> {
                None
            }
        }
    };
}

define_unit_place_slot!(PyPlaceQuestionPlaceSlot, "PlaceQuestionPlaceSlot");
define_unit_place_slot!(PyFaiPlaceSlot, "FaiPlaceSlot");

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn place_slot_from_python(state: &ReferenceState, value: &Bound<'_, PyAny>) -> PyResult<PlaceSlot> {
    if let Ok(value) = value.extract::<PyRef<'_, PyNumberedPlaceSlot>>() {
        return Ok(PlaceSlot::Numbered(
            NonZeroU8::new(value.place).expect("validated numbered slots are nonzero"),
        ));
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyModalPlaceSlot>>() {
        let tag = value
            .tag
            .as_ref()
            .map(|tag| raw_id_value(state, tag))
            .transpose()?
            .map(jbotci_semantics::references::RawSyntaxNodeId);
        return Ok(PlaceSlot::Modal(tag));
    }
    if value.is_instance_of::<PyPlaceQuestionPlaceSlot>() {
        return Ok(PlaceSlot::PlaceQuestion);
    }
    if value.is_instance_of::<PyFaiPlaceSlot>() {
        return Ok(PlaceSlot::Fai);
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.semantics.references PlaceSlot variant",
    ))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn place_slot_to_python(
    py: Python<'_>,
    state: &ReferenceState,
    value: PlaceSlot,
) -> PyResult<Py<PyAny>> {
    match value {
        PlaceSlot::Numbered(place) => {
            Ok(Py::new(py, PyNumberedPlaceSlot { place: place.get() })?.into_any())
        }
        PlaceSlot::Modal(tag) => Ok(Py::new(
            py,
            PyModalPlaceSlot {
                tag: tag.map(|tag| PyRawSyntaxNodeId::scoped(tag.0, &state.token)),
            },
        )?
        .into_any()),
        PlaceSlot::PlaceQuestion => Ok(Py::new(py, PyPlaceQuestionPlaceSlot)?.into_any()),
        PlaceSlot::Fai => Ok(Py::new(py, PyFaiPlaceSlot)?.into_any()),
    }
}

macro_rules! define_single_frame_propagation {
    ($rust_name:ident, $python_name:literal, $field:ident) => {
        #[invariant(true, "the typed ID carries the analysis scope")]
        #[pyclass(name = $python_name, frozen, eq, module = "jbotci.semantics.references", skip_from_py_object)]
        #[derive(Debug, Clone, PartialEq, Eq)]
        struct $rust_name {
            $field: PySelbriPlaceFrameId,
        }

        #[pymethods]
        impl $rust_name {
            #[classattr]
            #[allow(non_upper_case_globals)]
            const __match_args__: (&'static str,) = (stringify!($field),);

            #[requires(true)]
            #[ensures(ret == self.$field)]
            #[getter]
            fn $field(&self) -> PySelbriPlaceFrameId {
                self.$field.clone()
            }
        }
    };
}

/// A frame whose place behavior is intrinsic.
#[invariant(true, "fieldless payload variant carries no invalid state")]
#[pyclass(
    name = "NoPlaceFramePropagation",
    frozen,
    eq,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PyNoPlaceFramePropagation;

#[pymethods]
impl PyNoPlaceFramePropagation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: () = ();
}

define_single_frame_propagation!(
    PyForwardPlaceFramePropagation,
    "ForwardPlaceFramePropagation",
    inner
);
define_single_frame_propagation!(
    PyJaiPlaceFramePropagation,
    "JaiPlaceFramePropagation",
    inner
);

/// SE conversion of one inner place frame.
#[invariant(true, "the core NonZeroU8 enforces a nonzero converted place")]
#[pyclass(
    name = "ConversionPlaceFramePropagation",
    frozen,
    eq,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyConversionPlaceFramePropagation {
    inner: PySelbriPlaceFrameId,
    converted_place: u8,
}

#[pymethods]
impl PyConversionPlaceFramePropagation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("inner", "converted_place");

    #[requires(true)]
    #[ensures(ret == self.inner)]
    #[getter]
    fn inner(&self) -> PySelbriPlaceFrameId {
        self.inner.clone()
    }

    #[requires(true)]
    #[ensures(ret == self.converted_place)]
    #[getter]
    fn converted_place(&self) -> u8 {
        self.converted_place
    }
}

/// A connective frame with one or more branch frame IDs.
#[invariant(true, "the core resolver supplies the exact ordered branch list")]
#[pyclass(
    name = "ConnectiveBranchesPlaceFramePropagation",
    frozen,
    eq,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyConnectiveBranchesPlaceFramePropagation {
    branches: Arc<[PySelbriPlaceFrameId]>,
}

#[pymethods]
impl PyConnectiveBranchesPlaceFramePropagation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("branches",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn branches(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.branches.iter().cloned()).map(Bound::unbind)
    }
}

/// Compound selbri propagation through a head and ordered modifiers.
#[invariant(true, "the core resolver supplies the exact modifier sequence")]
#[pyclass(
    name = "CompoundPlaceFramePropagation",
    frozen,
    eq,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyCompoundPlaceFramePropagation {
    head: PySelbriPlaceFrameId,
    modifiers: Arc<[PySelbriPlaceFrameId]>,
}

#[pymethods]
impl PyCompoundPlaceFramePropagation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("head", "modifiers");

    #[requires(true)]
    #[ensures(ret == self.head)]
    #[getter]
    fn head(&self) -> PySelbriPlaceFrameId {
        self.head.clone()
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn modifiers(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.modifiers.iter().cloned()).map(Bound::unbind)
    }
}

/// CO inversion between leading and trailing frames.
#[invariant(true, "both typed IDs retain the exact analysis scope")]
#[pyclass(
    name = "CoPlaceFramePropagation",
    frozen,
    eq,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyCoPlaceFramePropagation {
    leading: PySelbriPlaceFrameId,
    trailing: PySelbriPlaceFrameId,
}

#[pymethods]
impl PyCoPlaceFramePropagation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("leading", "trailing");

    #[requires(true)]
    #[ensures(ret == self.leading)]
    #[getter]
    fn leading(&self) -> PySelbriPlaceFrameId {
        self.leading.clone()
    }

    #[requires(true)]
    #[ensures(ret == self.trailing)]
    #[getter]
    fn trailing(&self) -> PySelbriPlaceFrameId {
        self.trailing.clone()
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn propagation_to_python(
    py: Python<'_>,
    state: &ReferenceState,
    value: &PlaceFramePropagation,
) -> PyResult<Py<PyAny>> {
    let frame_id = |id: jbotci_semantics::references::SelbriPlaceFrameId| {
        PySelbriPlaceFrameId::scoped(id.0, &state.token)
    };
    match value {
        PlaceFramePropagation::None => Ok(Py::new(py, PyNoPlaceFramePropagation)?.into_any()),
        PlaceFramePropagation::Forward { inner } => Ok(Py::new(
            py,
            PyForwardPlaceFramePropagation {
                inner: frame_id(*inner),
            },
        )?
        .into_any()),
        PlaceFramePropagation::Conversion {
            inner,
            converted_place,
        } => Ok(Py::new(
            py,
            PyConversionPlaceFramePropagation {
                inner: frame_id(*inner),
                converted_place: converted_place.get(),
            },
        )?
        .into_any()),
        PlaceFramePropagation::Jai { inner } => Ok(Py::new(
            py,
            PyJaiPlaceFramePropagation {
                inner: frame_id(*inner),
            },
        )?
        .into_any()),
        PlaceFramePropagation::ConnectiveBranches { branches } => Ok(Py::new(
            py,
            PyConnectiveBranchesPlaceFramePropagation {
                branches: Arc::from(branches.iter().copied().map(frame_id).collect::<Vec<_>>()),
            },
        )?
        .into_any()),
        PlaceFramePropagation::Compound { head, modifiers } => Ok(Py::new(
            py,
            PyCompoundPlaceFramePropagation {
                head: frame_id(*head),
                modifiers: Arc::from(modifiers.iter().copied().map(frame_id).collect::<Vec<_>>()),
            },
        )?
        .into_any()),
        PlaceFramePropagation::Co { leading, trailing } => Ok(Py::new(
            py,
            PyCoPlaceFramePropagation {
                leading: frame_id(*leading),
                trailing: frame_id(*trailing),
            },
        )?
        .into_any()),
    }
}

/// Shared identity for one analysis-owned result record.
#[invariant(true, "the Arc keeps the complete analysis owner alive")]
#[derive(Clone)]
struct AnalysisRecordHandle {
    state: Arc<ReferenceState>,
    value: usize,
}

impl AnalysisRecordHandle {
    #[requires(true)]
    #[ensures(ret.value == value)]
    fn new(state: &Arc<ReferenceState>, value: usize) -> Self {
        Self {
            state: Arc::clone(state),
            value,
        }
    }
}

impl PartialEq for AnalysisRecordHandle {
    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.state, &other.state) && self.value == other.value
    ))]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && self.value == other.value
    }
}

impl Eq for AnalysisRecordHandle {}

impl Hash for AnalysisRecordHandle {
    #[requires(true)]
    #[ensures(true)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.state).hash(state);
        self.value.hash(state);
    }
}

/// One selbri place frame produced by the core resolver.
#[invariant(true, "the record handle retains and scopes the core frame")]
#[pyclass(
    name = "SelbriPlaceFrame",
    frozen,
    eq,
    hash,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
struct PySelbriPlaceFrame {
    handle: AnalysisRecordHandle,
}

impl PySelbriPlaceFrame {
    #[requires(true)]
    #[ensures(ret.handle.value == value)]
    fn new(state: &Arc<ReferenceState>, value: usize) -> Self {
        Self {
            handle: AnalysisRecordHandle::new(state, value),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_frame<R>(&self, function: impl FnOnce(&SelbriPlaceFrame) -> R) -> R {
        self.handle.state.with_analysis(|_, analysis| {
            let frame = analysis
                .place_analysis
                .frame(jbotci_semantics::references::SelbriPlaceFrameId(
                    self.handle.value,
                ))
                .expect("analysis-derived frame IDs remain valid");
            function(frame)
        })
    }
}

#[pymethods]
impl PySelbriPlaceFrame {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = ("id", "node", "kind", "selbri", "tanru_unit", "propagation");

    #[requires(true)]
    #[ensures(ret.value == self.handle.value)]
    #[getter]
    fn id(&self) -> PySelbriPlaceFrameId {
        PySelbriPlaceFrameId::scoped(self.handle.value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn node(&self) -> PyRawSyntaxNodeId {
        let value = self.with_frame(|frame| frame.node.0);
        PyRawSyntaxNodeId::scoped(value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.with_frame(|frame| frame.kind))
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selbri(&self) -> Option<PySelbriNodeId> {
        self.with_frame(|frame| frame.selbri)
            .map(|id| PySelbriNodeId::scoped(id.0.0, &self.handle.state.token))
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn tanru_unit(&self) -> Option<PyTanruUnitNodeId> {
        self.with_frame(|frame| frame.tanru_unit)
            .map(|id| PyTanruUnitNodeId::scoped(id.0.0, &self.handle.state.token))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn propagation(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.with_frame(|frame| propagation_to_python(py, &self.handle.state, &frame.propagation))
    }
}

/// One sumti-to-place assignment produced by the core resolver.
#[invariant(true, "the record handle retains and scopes the core assignment")]
#[pyclass(
    name = "SumtiPlaceAssignment",
    frozen,
    eq,
    hash,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
struct PySumtiPlaceAssignment {
    handle: AnalysisRecordHandle,
}

impl PySumtiPlaceAssignment {
    #[requires(true)]
    #[ensures(ret.handle.value == value)]
    fn new(state: &Arc<ReferenceState>, value: usize) -> Self {
        Self {
            handle: AnalysisRecordHandle::new(state, value),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_assignment<R>(&self, function: impl FnOnce(&SumtiPlaceAssignment) -> R) -> R {
        self.handle.state.with_analysis(|_, analysis| {
            let assignment = analysis
                .place_analysis
                .assignment(jbotci_semantics::references::SumtiPlaceAssignmentId(
                    self.handle.value,
                ))
                .expect("analysis-derived assignment IDs remain valid");
            function(assignment)
        })
    }
}

#[pymethods]
impl PySumtiPlaceAssignment {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = ("id", "frame", "slot", "sumti", "term", "source");

    #[requires(true)]
    #[ensures(ret.value == self.handle.value)]
    #[getter]
    fn id(&self) -> PySumtiPlaceAssignmentId {
        PySumtiPlaceAssignmentId::scoped(self.handle.value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn frame(&self) -> PySelbriPlaceFrameId {
        let value = self.with_assignment(|assignment| assignment.frame.0);
        PySelbriPlaceFrameId::scoped(value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn slot(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        place_slot_to_python(
            py,
            &self.handle.state,
            self.with_assignment(|assignment| assignment.slot),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn sumti(&self) -> PySumtiNodeId {
        let value = self.with_assignment(|assignment| assignment.sumti.0.0);
        PySumtiNodeId::scoped(value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn term(&self) -> Option<PyTermNodeId> {
        self.with_assignment(|assignment| assignment.term)
            .map(|id| PyTermNodeId::scoped(id.0.0, &self.handle.state.token))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn source(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.with_assignment(|assignment| assignment.source))
    }
}

/// Reference target resolved to one generated syntax node.
#[invariant(true, "the retained state keeps the target node owner alive")]
#[pyclass(
    name = "ResolvedNodeReferenceTarget",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyResolvedNodeReferenceTarget {
    state: Arc<ReferenceState>,
    node: PyRawSyntaxNodeId,
}

#[pymethods]
impl PyResolvedNodeReferenceTarget {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("node",);

    #[requires(true)]
    #[ensures(ret == self.node)]
    #[getter]
    fn node(&self) -> PyRawSyntaxNodeId {
        self.node.clone()
    }

    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.state, &other.state) && self.node == other.node
    ))]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && self.node == other.node
    }
}

/// Reference target resolved to one selbri place frame.
#[invariant(true, "the retained state keeps the target frame owner alive")]
#[pyclass(
    name = "ResolvedFrameReferenceTarget",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyResolvedFrameReferenceTarget {
    state: Arc<ReferenceState>,
    frame: PySelbriPlaceFrameId,
}

#[pymethods]
impl PyResolvedFrameReferenceTarget {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("frame",);

    #[requires(true)]
    #[ensures(ret == self.frame)]
    #[getter]
    fn frame(&self) -> PySelbriPlaceFrameId {
        self.frame.clone()
    }

    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.state, &other.state) && self.frame == other.frame
    ))]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && self.frame == other.frame
    }
}

/// Reference target with multiple exact node candidates.
#[invariant(true, "the retained state keeps every candidate node owner alive")]
#[pyclass(
    name = "AmbiguousNodesReferenceTarget",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyAmbiguousNodesReferenceTarget {
    state: Arc<ReferenceState>,
    nodes: Arc<[PyRawSyntaxNodeId]>,
}

#[pymethods]
impl PyAmbiguousNodesReferenceTarget {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("nodes",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn nodes(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.nodes.iter().cloned()).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.state, &other.state) && self.nodes == other.nodes
    ))]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && self.nodes == other.nodes
    }
}

/// Reference target that the core resolver could not determine.
#[invariant(true, "the retained state keeps the unresolved target owner alive")]
#[pyclass(
    name = "UnresolvedReferenceTarget",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyUnresolvedReferenceTarget {
    state: Arc<ReferenceState>,
    reason: String,
}

#[pymethods]
impl PyUnresolvedReferenceTarget {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("reason",);

    #[requires(true)]
    #[ensures(ret == self.reason.as_str())]
    #[getter]
    fn reason(&self) -> &str {
        &self.reason
    }

    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.state, &other.state) && self.reason == other.reason
    ))]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && self.reason == other.reason
    }
}

/// Intentionally vague reference target.
#[invariant(true, "the retained state keeps the vague target owner alive")]
#[pyclass(
    name = "VagueReferenceTarget",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyVagueReferenceTarget {
    state: Arc<ReferenceState>,
    kind: VagueReferenceKind,
}

#[pymethods]
impl PyVagueReferenceTarget {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("kind",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.kind)
    }

    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.state, &other.state) && self.kind == other.kind
    ))]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state) && self.kind == other.kind
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn reference_target_to_python(
    py: Python<'_>,
    state: &Arc<ReferenceState>,
    value: &ReferenceTarget,
) -> PyResult<Py<PyAny>> {
    let node_id = |value: usize| PyRawSyntaxNodeId::scoped(value, &state.token);
    let frame_id = |value: usize| PySelbriPlaceFrameId::scoped(value, &state.token);
    match value {
        ReferenceTarget::ResolvedNode(node) => Ok(Py::new(
            py,
            PyResolvedNodeReferenceTarget {
                state: Arc::clone(state),
                node: node_id(node.0),
            },
        )?
        .into_any()),
        ReferenceTarget::ResolvedFrame(frame) => Ok(Py::new(
            py,
            PyResolvedFrameReferenceTarget {
                state: Arc::clone(state),
                frame: frame_id(frame.0),
            },
        )?
        .into_any()),
        ReferenceTarget::AmbiguousNodes(nodes) => Ok(Py::new(
            py,
            PyAmbiguousNodesReferenceTarget {
                state: Arc::clone(state),
                nodes: Arc::from(nodes.iter().map(|node| node_id(node.0)).collect::<Vec<_>>()),
            },
        )?
        .into_any()),
        ReferenceTarget::Unresolved(reason) => Ok(Py::new(
            py,
            PyUnresolvedReferenceTarget {
                state: Arc::clone(state),
                reason: reason.clone(),
            },
        )?
        .into_any()),
        ReferenceTarget::Vague(kind) => Ok(Py::new(
            py,
            PyVagueReferenceTarget {
                state: Arc::clone(state),
                kind: *kind,
            },
        )?
        .into_any()),
    }
}

/// One typed discourse-reference edge produced by the core resolver.
#[invariant(true, "the record handle retains and scopes the core edge")]
#[pyclass(
    name = "ReferenceEdge",
    frozen,
    eq,
    hash,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone, PartialEq, Eq, Hash)]
struct PyReferenceEdge {
    handle: AnalysisRecordHandle,
}

impl PyReferenceEdge {
    #[requires(true)]
    #[ensures(ret.handle.value == value)]
    fn new(state: &Arc<ReferenceState>, value: usize) -> Self {
        Self {
            handle: AnalysisRecordHandle::new(state, value),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_edge<R>(
        &self,
        function: impl FnOnce(&jbotci_semantics::references::ReferenceEdge) -> R,
    ) -> R {
        self.handle.state.with_analysis(|_, analysis| {
            let edge = analysis
                .discourse_references
                .edges()
                .get(self.handle.value)
                .expect("analysis-derived reference edge IDs remain valid");
            function(edge)
        })
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn node_id_from_python(
    state: &ReferenceState,
    value: &Bound<'_, PyAny>,
) -> PyResult<Option<(usize, StrictSyntaxModel)>> {
    let handle = extract_syntax_value(value)
        .map_err(|_| PyTypeError::new_err("expected a generated strict syntax node"))?;
    let model = handle
        .strict_model()
        .ok_or_else(|| PyTypeError::new_err("expected a generated strict syntax node"))?;
    let path = handle.path().clone();
    state.with_analysis(|root, analysis| {
        if !root.owns(&handle) {
            return Err(InvalidInputError::new_err(
                "syntax node belongs to a different ReferenceAnalysis",
            ));
        }
        Ok(GeneratedSyntaxTreeNode::node_at_path(root.root(), &path)
            .and_then(|node| analysis.syntax_index.id_of(node))
            .map(|id| (id.0, model)))
    })
}

/// Borrowed generated-syntax index owned by a ReferenceAnalysis.
#[invariant(true, "the Arc retains the exact syntax root and borrowed index")]
#[pyclass(
    name = "GeneratedSyntaxIndex",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyGeneratedSyntaxIndex {
    state: Arc<ReferenceState>,
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn typed_node_id<R>(
    state: &ReferenceState,
    node: &Bound<'_, PyAny>,
    expected: StrictSyntaxModel,
    construct: fn(usize, &AnalysisToken) -> R,
) -> PyResult<Option<R>> {
    let Some((value, model)) = node_id_from_python(state, node)? else {
        return Ok(None);
    };
    if model != expected {
        return Err(PyTypeError::new_err(format!(
            "syntax node has model {model:?}, expected {expected:?}"
        )));
    }
    Ok(Some(construct(value, &state.token)))
}

#[pymethods]
impl PyGeneratedSyntaxIndex {
    /// Return the typed root text ID.
    #[requires(true)]
    #[ensures(true)]
    fn root(&self) -> PyTextNodeId {
        let value = self
            .state
            .with_analysis(|_, analysis| analysis.syntax_index.root().0.0);
        PyTextNodeId::scoped(value, &self.state.token)
    }

    /// Return the number of indexed generated nodes.
    #[requires(true)]
    #[ensures(ret > 0)]
    fn node_count(&self) -> usize {
        self.state
            .with_analysis(|_, analysis| analysis.syntax_index.node_count())
    }

    /// Resolve a raw node ID to the original typed Python syntax handle.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn node(
        &self,
        py: Python<'_>,
        id: PyRef<'_, PyRawSyntaxNodeId>,
    ) -> PyResult<Option<Py<PyAny>>> {
        let value = raw_id_value(&self.state, &id)?;
        let handle = self.state.with_analysis(|root, analysis| {
            analysis
                .syntax_index
                .node(jbotci_semantics::references::RawSyntaxNodeId(value))
                .and_then(|node| root.handle_for_node(node))
        });
        handle
            .map(|handle| wrap_syntax_value(py, handle))
            .transpose()
    }

    /// Return exact parent/order/depth/span metadata for a raw node ID.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn metadata(&self, id: PyRef<'_, PyRawSyntaxNodeId>) -> PyResult<Option<PySyntaxNodeMetadata>> {
        let value = raw_id_value(&self.state, &id)?;
        Ok(self
            .state
            .with_analysis(|_, analysis| {
                analysis
                    .syntax_index
                    .metadata(jbotci_semantics::references::RawSyntaxNodeId(value))
                    .cloned()
            })
            .map(|value| PySyntaxNodeMetadata::from_rust(value, &self.state.token)))
    }

    /// Resolve an original generated syntax handle to its raw node ID.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn id_of(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyRawSyntaxNodeId>> {
        Ok(node_id_from_python(&self.state, node)?
            .map(|(value, _)| PyRawSyntaxNodeId::scoped(value, &self.state.token)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn text_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyTextNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::TextSyntax,
            PyTextNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn paragraph_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyParagraphNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::ParagraphSyntax,
            PyParagraphNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn statement_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyStatementNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::StatementSyntax,
            PyStatementNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn bridi_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyBridiNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::BridiSyntax,
            PyBridiNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn bridi_tail_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyBridiTailNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::BridiTailSyntax,
            PyBridiTailNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn selbri_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PySelbriNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::SelbriSyntax,
            PySelbriNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn tanru_unit_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyTanruUnitNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::TanruUnitSyntax,
            PyTanruUnitNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn term_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyTermNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::TermSyntax,
            PyTermNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn sumti_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PySumtiNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::SumtiSyntax,
            PySumtiNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn free_modifier_node_id(
        &self,
        node: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyFreeModifierNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::FreeModifierSyntax,
            PyFreeModifierNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn abstraction_node_id(
        &self,
        node: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyAbstractionNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::AbstractionTanruUnitSyntax,
            PyAbstractionNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn mekso_node_id(&self, node: &Bound<'_, PyAny>) -> PyResult<Option<PyMeksoNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::MeksoSyntax,
            PyMeksoNodeId::scoped,
        )
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn mekso_operator_node_id(
        &self,
        node: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyMeksoOperatorNodeId>> {
        typed_node_id(
            &self.state,
            node,
            StrictSyntaxModel::MeksoOperatorSyntax,
            PyMeksoOperatorNodeId::scoped,
        )
    }
}

/// Place-frame and sumti-assignment query facade owned by ReferenceAnalysis.
#[invariant(true, "the Arc retains the exact core PlaceAnalysis and syntax owner")]
#[pyclass(
    name = "PlaceAnalysis",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyPlaceAnalysis {
    state: Arc<ReferenceState>,
}

#[pymethods]
impl PyPlaceAnalysis {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn frames(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let count = self
            .state
            .with_analysis(|_, analysis| analysis.place_analysis.frames().len());
        sequence_to_tuple(
            py,
            (0..count).map(|value| PySelbriPlaceFrame::new(&self.state, value)),
        )
        .map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn frame(&self, id: PyRef<'_, PySelbriPlaceFrameId>) -> PyResult<Option<PySelbriPlaceFrame>> {
        let value = frame_id_value(&self.state, &id)?;
        let exists = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .frame(jbotci_semantics::references::SelbriPlaceFrameId(value))
                .is_some()
        });
        Ok(exists.then(|| PySelbriPlaceFrame::new(&self.state, value)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn frames_for_node(
        &self,
        py: Python<'_>,
        node: PyRef<'_, PyRawSyntaxNodeId>,
    ) -> PyResult<Py<PyTuple>> {
        let value = raw_id_value(&self.state, &node)?;
        let values = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .frames_for_node(jbotci_semantics::references::RawSyntaxNodeId(value))
                .iter()
                .map(|id| PySelbriPlaceFrameId::scoped(id.0, &self.state.token))
                .collect::<Vec<_>>()
        });
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn assignments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let count = self
            .state
            .with_analysis(|_, analysis| analysis.place_analysis.assignments().len());
        sequence_to_tuple(
            py,
            (0..count).map(|value| PySumtiPlaceAssignment::new(&self.state, value)),
        )
        .map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn assignment(
        &self,
        id: PyRef<'_, PySumtiPlaceAssignmentId>,
    ) -> PyResult<Option<PySumtiPlaceAssignment>> {
        let value = assignment_id_value(&self.state, &id)?;
        let exists = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .assignment(jbotci_semantics::references::SumtiPlaceAssignmentId(value))
                .is_some()
        });
        Ok(exists.then(|| PySumtiPlaceAssignment::new(&self.state, value)))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn assignments_for_sumti(
        &self,
        py: Python<'_>,
        sumti: PyRef<'_, PySumtiNodeId>,
    ) -> PyResult<Py<PyTuple>> {
        validate_token(&self.state.token, &sumti.token, "sumti node ID")?;
        let values = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .assignments_for_sumti(jbotci_semantics::references::SumtiNodeId(
                    jbotci_semantics::references::RawSyntaxNodeId(sumti.value),
                ))
                .iter()
                .map(|id| PySumtiPlaceAssignmentId::scoped(id.0, &self.state.token))
                .collect::<Vec<_>>()
        });
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn assignments_for_term(
        &self,
        py: Python<'_>,
        term: PyRef<'_, PyTermNodeId>,
    ) -> PyResult<Py<PyTuple>> {
        validate_token(&self.state.token, &term.token, "term node ID")?;
        let values = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .assignments_for_term(jbotci_semantics::references::TermNodeId(
                    jbotci_semantics::references::RawSyntaxNodeId(term.value),
                ))
                .iter()
                .map(|id| PySumtiPlaceAssignmentId::scoped(id.0, &self.state.token))
                .collect::<Vec<_>>()
        });
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn assignments_for_frame(
        &self,
        py: Python<'_>,
        frame: PyRef<'_, PySelbriPlaceFrameId>,
    ) -> PyResult<Py<PyTuple>> {
        let value = frame_id_value(&self.state, &frame)?;
        let values = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .assignments_for_frame(jbotci_semantics::references::SelbriPlaceFrameId(value))
                .iter()
                .map(|id| PySumtiPlaceAssignmentId::scoped(id.0, &self.state.token))
                .collect::<Vec<_>>()
        });
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn assignments_for_frame_slot(
        &self,
        py: Python<'_>,
        frame: PyRef<'_, PySelbriPlaceFrameId>,
        slot: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyTuple>> {
        let frame = frame_id_value(&self.state, &frame)?;
        let slot = place_slot_from_python(&self.state, slot)?;
        let values = self.state.with_analysis(|_, analysis| {
            analysis
                .place_analysis
                .assignments_for_frame_slot(
                    jbotci_semantics::references::SelbriPlaceFrameId(frame),
                    slot,
                )
                .iter()
                .map(|id| PySumtiPlaceAssignmentId::scoped(id.0, &self.state.token))
                .collect::<Vec<_>>()
        });
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn first_argument_for_place(
        &self,
        frame: PyRef<'_, PySelbriPlaceFrameId>,
        slot: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PySumtiNodeId>> {
        let frame = frame_id_value(&self.state, &frame)?;
        let slot = place_slot_from_python(&self.state, slot)?;
        Ok(self
            .state
            .with_analysis(|_, analysis| {
                analysis.place_analysis.first_argument_for_place(
                    jbotci_semantics::references::SelbriPlaceFrameId(frame),
                    slot,
                )
            })
            .map(|id| PySumtiNodeId::scoped(id.0.0, &self.state.token)))
    }
}

/// Discourse-reference edge query facade owned by ReferenceAnalysis.
#[invariant(true, "the Arc retains the exact core discourse-reference result")]
#[pyclass(
    name = "DiscourseReferences",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyDiscourseReferences {
    state: Arc<ReferenceState>,
}

#[pymethods]
impl PyDiscourseReferences {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn edges(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let count = self
            .state
            .with_analysis(|_, analysis| analysis.discourse_references.edges().len());
        sequence_to_tuple(
            py,
            (0..count).map(|value| PyReferenceEdge::new(&self.state, value)),
        )
        .map(Bound::unbind)
    }
}

/// Fieldless structured value for a missing generated root index entry.
#[invariant(true, "fieldless error variant carries no invalid state")]
#[pyclass(
    name = "MissingRootNode",
    frozen,
    eq,
    hash,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PyMissingRootNode;

#[pymethods]
impl PyMissingRootNode {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: () = ();

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> &'static str {
        "syntax index did not contain the root text node"
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.semantics.references.MissingRootNode()"
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_error_to_python(py: Python<'_>, error: RustReferenceError) -> PyErr {
    let (exception_name, value) = match error {
        RustReferenceError::MissingRootNode => {
            let value = match Py::new(py, PyMissingRootNode) {
                Ok(value) => value.into_any(),
                Err(error) => return error,
            };
            ("MissingRootNodeError", value)
        }
    };
    public_exception_with_value(py, PUBLIC_MODULE, exception_name, value)
}

/// Owning immutable reference analysis over one exact strict syntax root.
#[invariant(true, "the Arc owns the self-referential tree plus core analysis")]
#[pyclass(
    name = "ReferenceAnalysis",
    frozen,
    module = "jbotci.semantics.references",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyReferenceAnalysis {
    state: Arc<ReferenceState>,
}

#[pymethods]
impl PyReferenceAnalysis {
    /// Return the original #557 strict TextSyntax root with stable owner identity.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn syntax(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.state
            .cell
            .with_dependent(|root, _| strict_text_to_python(py, root))
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn syntax_index(&self) -> PyGeneratedSyntaxIndex {
        PyGeneratedSyntaxIndex {
            state: Arc::clone(&self.state),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn place_analysis(&self) -> PyPlaceAnalysis {
        PyPlaceAnalysis {
            state: Arc::clone(&self.state),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn discourse_references(&self) -> PyDiscourseReferences {
        PyDiscourseReferences {
            state: Arc::clone(&self.state),
        }
    }

    /// Return the typed fixture/debug projection, not the primary object model.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn fixture_projection(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json = self.fixture_projection_json()?;
        py.import(PUBLIC_MODULE)?
            .getattr("_fixture_projection_from_json")?
            .call1((json,))
            .map(Bound::unbind)
    }

    /// Return canonical JSON for fixture comparison and debugging only.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
    fn fixture_projection_json(&self) -> PyResult<String> {
        self.state.with_analysis(|_, analysis| {
            analysis
                .fixture_projection_json()
                .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
        })
    }
}

/// Analyze a #557 strict TextSyntax root or #559 successful SyntaxParse directly.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_references_analyze_references")]
fn analyze_references(
    py: Python<'_>,
    tree_or_parse: &Bound<'_, PyAny>,
) -> PyResult<PyReferenceAnalysis> {
    let root = reference_root_from_python(tree_or_parse)?;
    let state = py
        .detach(|| ReferenceState::new(root))
        .map_err(|error| reference_error_to_python(py, error))?;
    Ok(PyReferenceAnalysis {
        state: Arc::new(state),
    })
}

/// Register the complete private native reference-analysis surface.
#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_type::<PyRawSyntaxNodeId>(module, "_references_RawSyntaxNodeId")?;
    register_type::<PyTextNodeId>(module, "_references_TextNodeId")?;
    register_type::<PyParagraphNodeId>(module, "_references_ParagraphNodeId")?;
    register_type::<PyStatementNodeId>(module, "_references_StatementNodeId")?;
    register_type::<PyBridiNodeId>(module, "_references_BridiNodeId")?;
    register_type::<PyBridiTailNodeId>(module, "_references_BridiTailNodeId")?;
    register_type::<PySelbriNodeId>(module, "_references_SelbriNodeId")?;
    register_type::<PyTanruUnitNodeId>(module, "_references_TanruUnitNodeId")?;
    register_type::<PyTermNodeId>(module, "_references_TermNodeId")?;
    register_type::<PySumtiNodeId>(module, "_references_SumtiNodeId")?;
    register_type::<PyFreeModifierNodeId>(module, "_references_FreeModifierNodeId")?;
    register_type::<PyAbstractionNodeId>(module, "_references_AbstractionNodeId")?;
    register_type::<PyMeksoNodeId>(module, "_references_MeksoNodeId")?;
    register_type::<PyMeksoOperatorNodeId>(module, "_references_MeksoOperatorNodeId")?;
    register_type::<PySyntaxNodeMetadata>(module, "_references_SyntaxNodeMetadata")?;
    register_type::<PySelbriPlaceFrameId>(module, "_references_SelbriPlaceFrameId")?;
    register_type::<PySumtiPlaceAssignmentId>(module, "_references_SumtiPlaceAssignmentId")?;
    register_type::<PyReferenceEdgeId>(module, "_references_ReferenceEdgeId")?;

    register_type::<PyNumberedPlaceSlot>(module, "_references_NumberedPlaceSlot")?;
    register_type::<PyModalPlaceSlot>(module, "_references_ModalPlaceSlot")?;
    register_type::<PyPlaceQuestionPlaceSlot>(module, "_references_PlaceQuestionPlaceSlot")?;
    register_type::<PyFaiPlaceSlot>(module, "_references_FaiPlaceSlot")?;
    register_string_enum::<PlaceFrameKind>(module)?;
    register_type::<PyNoPlaceFramePropagation>(module, "_references_NoPlaceFramePropagation")?;
    register_type::<PyForwardPlaceFramePropagation>(
        module,
        "_references_ForwardPlaceFramePropagation",
    )?;
    register_type::<PyConversionPlaceFramePropagation>(
        module,
        "_references_ConversionPlaceFramePropagation",
    )?;
    register_type::<PyJaiPlaceFramePropagation>(module, "_references_JaiPlaceFramePropagation")?;
    register_type::<PyConnectiveBranchesPlaceFramePropagation>(
        module,
        "_references_ConnectiveBranchesPlaceFramePropagation",
    )?;
    register_type::<PyCompoundPlaceFramePropagation>(
        module,
        "_references_CompoundPlaceFramePropagation",
    )?;
    register_type::<PyCoPlaceFramePropagation>(module, "_references_CoPlaceFramePropagation")?;
    register_type::<PySelbriPlaceFrame>(module, "_references_SelbriPlaceFrame")?;

    register_string_enum::<AssignmentSource>(module)?;
    register_type::<PySumtiPlaceAssignment>(module, "_references_SumtiPlaceAssignment")?;
    register_string_enum::<ReferenceKind>(module)?;
    register_string_enum::<VagueReferenceKind>(module)?;
    register_type::<PyResolvedNodeReferenceTarget>(
        module,
        "_references_ResolvedNodeReferenceTarget",
    )?;
    register_type::<PyResolvedFrameReferenceTarget>(
        module,
        "_references_ResolvedFrameReferenceTarget",
    )?;
    register_type::<PyAmbiguousNodesReferenceTarget>(
        module,
        "_references_AmbiguousNodesReferenceTarget",
    )?;
    register_type::<PyUnresolvedReferenceTarget>(module, "_references_UnresolvedReferenceTarget")?;
    register_type::<PyVagueReferenceTarget>(module, "_references_VagueReferenceTarget")?;
    register_string_enum::<ReferenceRule>(module)?;
    register_type::<PyReferenceEdge>(module, "_references_ReferenceEdge")?;
    register_type::<PyMissingRootNode>(module, "_references_MissingRootNode")?;

    register_type::<PyGeneratedSyntaxIndex>(module, "_references_GeneratedSyntaxIndex")?;
    register_type::<PyPlaceAnalysis>(module, "_references_PlaceAnalysis")?;
    register_type::<PyDiscourseReferences>(module, "_references_DiscourseReferences")?;
    register_type::<PyReferenceAnalysis>(module, "_references_ReferenceAnalysis")?;
    module.add_function(wrap_pyfunction!(analyze_references, module)?)?;
    register_private_object(
        module,
        "_references_RUNTIME_INVENTORY",
        sequence_to_tuple(module.py(), NATIVE_EXPORTS.iter().copied())?,
    )?;
    Ok(())
}

#[pymethods]
impl PyReferenceEdge {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = ("id", "kind", "source", "target", "rule");

    #[requires(true)]
    #[ensures(ret.value == self.handle.value)]
    #[getter]
    fn id(&self) -> PyReferenceEdgeId {
        PyReferenceEdgeId::scoped(self.handle.value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.with_edge(|edge| edge.kind))
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source(&self) -> PyRawSyntaxNodeId {
        let value = self.with_edge(|edge| edge.source.0);
        PyRawSyntaxNodeId::scoped(value, &self.handle.state.token)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn target(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.with_edge(|edge| reference_target_to_python(py, &self.handle.state, &edge.target))
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn rule(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.with_edge(|edge| edge.rule))
    }
}
