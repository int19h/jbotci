//! Strict, recovered, and completion bindings for the public syntax parser facade.

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use bityzba::{contract_trait, data, ensures, expensive_ensures, invariant, new, requires};
use jbotci_syntax::{
    ExperimentalConstruct, ParseOptions, RecoveredSyntaxParse, RecoveredSyntaxParseAttempt,
    SyntaxConstructContext, SyntaxError as RustSyntaxError, SyntaxErrorKind, SyntaxExpectation,
    SyntaxExpectationReason, SyntaxExpectationReasonData, SyntaxExpectedToken,
    SyntaxExpectedTokenData, SyntaxParse, SyntaxParseAttempt, SyntaxRecoveryErrorPolicy,
    SyntaxRecoveryParseAttempt, SyntaxRecoveryParseData, SyntaxTextBoundaryKind,
    SyntaxTextStructureEvent, SyntaxTextStructureEventData, SyntaxTextUnit,
    SyntaxTextUnitGranularity, SyntaxWarning, SyntaxWarningDisplay,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyTuple};

use crate::InvalidInputError;
use crate::diagnostics::{PyDiagnostic, PyTraceOptions, PyTraceReport};
use crate::dialect::PyDialectDefinition;
use crate::morphology::{TokenHandle, extract_word_like};
use crate::source::PySourceId;
use crate::support::{
    PythonStringEnum, extract_sequence, extract_string_enum, register_private_object,
    register_string_enum, register_type, sequence_to_tuple, string_enum_member,
};
use crate::syntax::{
    RecoveredTextRootHandle, StrictTextRootHandle, extract_syntax_token, recovered_text_root,
    recovered_text_to_python, strict_text_root, strict_text_to_python, syntax_token_to_python,
};

const PUBLIC_MODULE: &str = "jbotci.syntax";
const DEFAULT_RECOVERY_ERRORS_PER_STATEMENT: i128 =
    SyntaxRecoveryErrorPolicy::DEFAULT_PER_STATEMENT.get() as i128;
const DEFAULT_RECOVERY_ERROR_HARD_CAP: i128 =
    SyntaxRecoveryErrorPolicy::DEFAULT_GLOBAL_HARD_CAP.get() as i128;

pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_syntax_parser_SYNTAX_TRACE_FILTERS",
    "_syntax_parser_ENUM_INVENTORY",
    "_syntax_parser_SyntaxTextUnitGranularity",
    "_syntax_parser_SyntaxTextBoundaryKind",
    "_syntax_parser_SyntaxErrorKind",
    "_syntax_parser_SyntaxWordCategory",
    "_syntax_parser_ExperimentalConstruct",
    "_syntax_parser_SyntaxRecoveryErrorPolicy",
    "_syntax_parser_ParseOptions",
    "_syntax_parser_SyntaxTextUnit",
    "_syntax_parser_SyntaxTextStructureEventBoundary",
    "_syntax_parser_SyntaxTextStructureEventContainerOpen",
    "_syntax_parser_SyntaxTextStructureEventContainerClose",
    "_syntax_parser_SyntaxConstructContext",
    "_syntax_parser_SyntaxExpectedTokenCmavo",
    "_syntax_parser_SyntaxExpectedTokenSelmaho",
    "_syntax_parser_SyntaxExpectedTokenWordCategory",
    "_syntax_parser_SyntaxExpectedTokenEndOfInput",
    "_syntax_parser_SyntaxExpectedTokenNamed",
    "_syntax_parser_SyntaxExpectationReasonContinueCurrent",
    "_syntax_parser_SyntaxExpectationReasonStartNested",
    "_syntax_parser_SyntaxExpectationReasonEndThenStart",
    "_syntax_parser_SyntaxExpectation",
    "_syntax_parser_SyntaxErrorNotImplemented",
    "_syntax_parser_SyntaxErrorParse",
    "_syntax_parser_SyntaxWarning",
    "_syntax_parser_SyntaxWarningDisplay",
    "_syntax_parser_SyntaxParse",
    "_syntax_parser_SyntaxParseAttempt",
    "_syntax_parser_RecoveredSyntaxParse",
    "_syntax_parser_RecoveredSyntaxParseAttempt",
    "_syntax_parser_SyntaxRecoveryParseValid",
    "_syntax_parser_SyntaxRecoveryParseRecovered",
    "_syntax_parser_SyntaxRecoveryParseAttempt",
    "_syntax_parser_syntax_tokens_with_options",
    "_syntax_parser_partition_syntax_text_units",
    "_syntax_parser_syntax_text_structure",
    "_syntax_parser_parse_text_attempt",
    "_syntax_parser_parse_syntax_tree_attempt",
    "_syntax_parser_parse_syntax_tree_recovered_attempt",
    "_syntax_parser_parse_syntax_tree_with_recovery_attempt",
    "_syntax_parser_expected_continuations",
    "_syntax_parser_expected_continuations_with_time_limit",
    "_syntax_parser_syntax_warning_display",
    "_syntax_parser_syntax_warning_displays",
];

const PARSER_ENUM_INVENTORY: &[&str] = &[
    "SyntaxTextUnitGranularity",
    "SyntaxTextBoundaryKind",
    "SyntaxErrorKind",
    "SyntaxWordCategory",
    "ExperimentalConstruct",
];

macro_rules! define_syntax_string_enum_binding {
    (
        $type:ty,
        $native_name:literal,
        $python_name:literal,
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
                concat!("jbotci syntax enum ", $python_name, ".")
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

define_syntax_string_enum_binding!(
    SyntaxTextUnitGranularity,
    "_syntax_parser_SyntaxTextUnitGranularity",
    "SyntaxTextUnitGranularity",
    {
        SyntaxTextUnitGranularity::Paragraph => ("PARAGRAPH", "paragraph"),
        SyntaxTextUnitGranularity::Statement => ("STATEMENT", "statement"),
    }
);

define_syntax_string_enum_binding!(
    SyntaxTextBoundaryKind,
    "_syntax_parser_SyntaxTextBoundaryKind",
    "SyntaxTextBoundaryKind",
    {
        SyntaxTextBoundaryKind::I => ("I", "i"),
        SyntaxTextBoundaryKind::Niho => ("NIHO", "niho"),
    }
);

define_syntax_string_enum_binding!(
    SyntaxErrorKind,
    "_syntax_parser_SyntaxErrorKind",
    "SyntaxErrorKind",
    {
        SyntaxErrorKind::UnexpectedEnd => ("UNEXPECTED_END", "unexpected-end"),
        SyntaxErrorKind::UnexpectedCmavo => ("UNEXPECTED_CMAVO", "unexpected-cmavo"),
        SyntaxErrorKind::UnexpectedBrivla => ("UNEXPECTED_BRIVLA", "unexpected-brivla"),
        SyntaxErrorKind::UnexpectedCmevla => ("UNEXPECTED_CMEVLA", "unexpected-cmevla"),
        SyntaxErrorKind::UnexpectedQuote => ("UNEXPECTED_QUOTE", "unexpected-quote"),
        SyntaxErrorKind::UnexpectedLerfu => ("UNEXPECTED_LERFU", "unexpected-lerfu"),
        SyntaxErrorKind::UnexpectedZeiCompound => (
            "UNEXPECTED_ZEI_COMPOUND",
            "unexpected-zei-compound"
        ),
        SyntaxErrorKind::UnexpectedWord => ("UNEXPECTED_WORD", "unexpected-word"),
        SyntaxErrorKind::IncompleteText => ("INCOMPLETE_TEXT", "incomplete-text"),
        SyntaxErrorKind::IncompleteStatement => (
            "INCOMPLETE_STATEMENT",
            "incomplete-statement"
        ),
        SyntaxErrorKind::IncompleteBridi => ("INCOMPLETE_BRIDI", "incomplete-bridi"),
        SyntaxErrorKind::IncompleteTerm => ("INCOMPLETE_TERM", "incomplete-term"),
        SyntaxErrorKind::IncompleteSumti => ("INCOMPLETE_SUMTI", "incomplete-sumti"),
        SyntaxErrorKind::IncompleteSelbri => ("INCOMPLETE_SELBRI", "incomplete-selbri"),
        SyntaxErrorKind::IncompleteFreeModifier => (
            "INCOMPLETE_FREE_MODIFIER",
            "incomplete-free-modifier"
        ),
        SyntaxErrorKind::IncompleteMekso => ("INCOMPLETE_MEKSO", "incomplete-mekso"),
        SyntaxErrorKind::IncompleteQuote => ("INCOMPLETE_QUOTE", "incomplete-quote"),
        SyntaxErrorKind::IncompleteForethoughtConnection => (
            "INCOMPLETE_FORETHOUGHT_CONNECTION",
            "incomplete-forethought-connection"
        ),
        SyntaxErrorKind::InvalidBridiTailConnection => (
            "INVALID_BRIDI_TAIL_CONNECTION",
            "invalid-bridi-tail-connection"
        ),
        SyntaxErrorKind::InvalidConstruct => ("INVALID_CONSTRUCT", "invalid-construct"),
    }
);

define_syntax_string_enum_binding!(
    jbotci_syntax::SyntaxWordCategory,
    "_syntax_parser_SyntaxWordCategory",
    "SyntaxWordCategory",
    {
        jbotci_syntax::SyntaxWordCategory::Brivla => ("BRIVLA", "brivla"),
        jbotci_syntax::SyntaxWordCategory::Cmevla => ("CMEVLA", "cmevla"),
        jbotci_syntax::SyntaxWordCategory::SelbriWord => ("SELBRI_WORD", "selbri-word"),
        jbotci_syntax::SyntaxWordCategory::ProSumti => ("PRO_SUMTI", "pro-sumti"),
        jbotci_syntax::SyntaxWordCategory::LetterWord => ("LETTER_WORD", "letter-word"),
        jbotci_syntax::SyntaxWordCategory::Quote => ("QUOTE", "quote"),
    }
);

define_syntax_string_enum_binding!(
    ExperimentalConstruct,
    "_syntax_parser_ExperimentalConstruct",
    "ExperimentalConstruct",
    {
        ExperimentalConstruct::ExperimentalCmavo => ("EXPERIMENTAL_CMAVO", "experimental-cmavo"),
        ExperimentalConstruct::ExperimentalZohOiQuote => ("EXPERIMENTAL_ZOH_OI_QUOTE", "experimental-zoh-oi-quote"),
        ExperimentalConstruct::ExperimentalMehOiQuote => ("EXPERIMENTAL_MEH_OI_QUOTE", "experimental-meh-oi-quote"),
        ExperimentalConstruct::ExperimentalMehOiSelbriUnit => ("EXPERIMENTAL_MEH_OI_SELBRI_UNIT", "experimental-meh-oi-selbri-unit"),
        ExperimentalConstruct::ExperimentalLohOiBridiDescription => ("EXPERIMENTAL_LOH_OI_BRIDI_DESCRIPTION", "experimental-loh-oi-bridi-description"),
        ExperimentalConstruct::ExperimentalLohAiReplacementFree => ("EXPERIMENTAL_LOH_AI_REPLACEMENT_FREE", "experimental-loh-ai-replacement-free"),
        ExperimentalConstruct::ExperimentalJacuPredicateTailConnective => ("EXPERIMENTAL_JACU_PREDICATE_TAIL_CONNECTIVE", "experimental-jacu-predicate-tail-connective"),
        ExperimentalConstruct::ExperimentalJeIStatementConnective => ("EXPERIMENTAL_JE_I_STATEMENT_CONNECTIVE", "experimental-je-i-statement-connective"),
        ExperimentalConstruct::ExperimentalMultipleNaFragment => ("EXPERIMENTAL_MULTIPLE_NA_FRAGMENT", "experimental-multiple-na-fragment"),
        ExperimentalConstruct::ExperimentalEmptyPrenex => ("EXPERIMENTAL_EMPTY_PRENEX", "experimental-empty-prenex"),
        ExperimentalConstruct::ExperimentalBareCuPredicate => ("EXPERIMENTAL_BARE_CU_PREDICATE", "experimental-bare-cu-predicate"),
        ExperimentalConstruct::ExperimentalNaheArgumentWithoutBo => ("EXPERIMENTAL_NAHE_ARGUMENT_WITHOUT_BO", "experimental-nahe-argument-without-bo"),
        ExperimentalConstruct::ExperimentalVuhoScopedAttachment => ("EXPERIMENTAL_VUHO_SCOPED_ATTACHMENT", "experimental-vuho-scoped-attachment"),
        ExperimentalConstruct::ExperimentalNohoiSelbriRelativeClause => ("EXPERIMENTAL_NOHOI_SELBRI_RELATIVE_CLAUSE", "experimental-nohoi-selbri-relative-clause"),
        ExperimentalConstruct::ExperimentalSimplerSumtiConnective => ("EXPERIMENTAL_SIMPLER_SUMTI_CONNECTIVE", "experimental-simpler-sumti-connective"),
        ExperimentalConstruct::ExperimentalExplicitCuPredicateTailStarter => ("EXPERIMENTAL_EXPLICIT_CU_PREDICATE_TAIL_STARTER", "experimental-explicit-cu-predicate-tail-starter"),
        ExperimentalConstruct::ExperimentalRelativeClauseConnective => ("EXPERIMENTAL_RELATIVE_CLAUSE_CONNECTIVE", "experimental-relative-clause-connective"),
        ExperimentalConstruct::ExperimentalSimplerForethoughtConnective => ("EXPERIMENTAL_SIMPLER_FORETHOUGHT_CONNECTIVE", "experimental-simpler-forethought-connective"),
        ExperimentalConstruct::ExperimentalSimplerTermConnective => ("EXPERIMENTAL_SIMPLER_TERM_CONNECTIVE", "experimental-simpler-term-connective"),
        ExperimentalConstruct::ExperimentalMexOperatorConnective => ("EXPERIMENTAL_MEX_OPERATOR_CONNECTIVE", "experimental-mex-operator-connective"),
        ExperimentalConstruct::ExperimentalSimplerDescriptorHeadConnective => ("EXPERIMENTAL_SIMPLER_DESCRIPTOR_HEAD_CONNECTIVE", "experimental-simpler-descriptor-head-connective"),
        ExperimentalConstruct::ExperimentalJiAsJaConnective => ("EXPERIMENTAL_JI_AS_JA_CONNECTIVE", "experimental-ji-as-ja-connective"),
        ExperimentalConstruct::ExperimentalGadganzuGadri => ("EXPERIMENTAL_GADGANZU_GADRI", "experimental-gadganzu-gadri"),
        ExperimentalConstruct::ExperimentalIauReset => ("EXPERIMENTAL_IAU_RESET", "experimental-iau-reset"),
        ExperimentalConstruct::ExperimentalGohoiSelbriUnit => ("EXPERIMENTAL_GOHOI_SELBRI_UNIT", "experimental-gohoi-selbri-unit"),
        ExperimentalConstruct::ExperimentalKeTermset => ("EXPERIMENTAL_KE_TERMSET", "experimental-ke-termset"),
        ExperimentalConstruct::ExperimentalCuTermsSelbri => ("EXPERIMENTAL_CU_TERMS_SELBRI", "experimental-cu-terms-selbri"),
        ExperimentalConstruct::ExperimentalLaheNaheTermWrapper => ("EXPERIMENTAL_LAHE_NAHE_TERM_WRAPPER", "experimental-lahe-nahe-term-wrapper"),
        ExperimentalConstruct::ExperimentalForethoughtRelativeClauseConnective => ("EXPERIMENTAL_FORETHOUGHT_RELATIVE_CLAUSE_CONNECTIVE", "experimental-forethought-relative-clause-connective"),
        ExperimentalConstruct::ExperimentalBroadAConnective => ("EXPERIMENTAL_BROAD_A_CONNECTIVE", "experimental-broad-a-connective"),
        ExperimentalConstruct::ExperimentalVuhuConnective => ("EXPERIMENTAL_VUHU_CONNECTIVE", "experimental-vuhu-connective"),
        ExperimentalConstruct::ExperimentalNahuPredicateConnective => ("EXPERIMENTAL_NAHU_PREDICATE_CONNECTIVE", "experimental-nahu-predicate-connective"),
        ExperimentalConstruct::ExperimentalFaAsTag => ("EXPERIMENTAL_FA_AS_TAG", "experimental-fa-as-tag"),
        ExperimentalConstruct::ExperimentalFlattenedTag => ("EXPERIMENTAL_FLATTENED_TAG", "experimental-flattened-tag"),
        ExperimentalConstruct::ExperimentalCbmCmevlaSelbriWord => ("EXPERIMENTAL_CBM_CMEVLA_SELBRI_WORD", "experimental-cbm-cmevla-selbri-word"),
        ExperimentalConstruct::ExperimentalCbmLaNameAsDescriptor => ("EXPERIMENTAL_CBM_LA_NAME_AS_DESCRIPTOR", "experimental-cbm-la-name-as-descriptor"),
        ExperimentalConstruct::ExperimentalDictionaryDoiVocative => ("EXPERIMENTAL_DICTIONARY_DOI_VOCATIVE", "experimental-dictionary-doi-vocative"),
        ExperimentalConstruct::ExperimentalDictionaryCoiVocative => ("EXPERIMENTAL_DICTIONARY_COI_VOCATIVE", "experimental-dictionary-coi-vocative"),
        ExperimentalConstruct::ExperimentalDictionarySeiFreeModifier => ("EXPERIMENTAL_DICTIONARY_SEI_FREE_MODIFIER", "experimental-dictionary-sei-free-modifier"),
        ExperimentalConstruct::ExperimentalDictionaryPaNumber => ("EXPERIMENTAL_DICTIONARY_PA_NUMBER", "experimental-dictionary-pa-number"),
        ExperimentalConstruct::ExperimentalDictionaryFahaTag => ("EXPERIMENTAL_DICTIONARY_FAHA_TAG", "experimental-dictionary-faha-tag"),
        ExperimentalConstruct::ExperimentalDictionaryUiIndicator => ("EXPERIMENTAL_DICTIONARY_UI_INDICATOR", "experimental-dictionary-ui-indicator"),
        ExperimentalConstruct::ExperimentalNoihaAdverbial => ("EXPERIMENTAL_NOIHA_ADVERBIAL", "experimental-noiha-adverbial"),
        ExperimentalConstruct::ExperimentalFihoiAdverbial => ("EXPERIMENTAL_FIHOI_ADVERBIAL", "experimental-fihoi-adverbial"),
        ExperimentalConstruct::ExperimentalSoiAdverbial => ("EXPERIMENTAL_SOI_ADVERBIAL", "experimental-soi-adverbial"),
        ExperimentalConstruct::ExperimentalPreposedLinkargs => ("EXPERIMENTAL_PREPOSED_LINKARGS", "experimental-preposed-linkargs"),
        ExperimentalConstruct::ExperimentalEmptyLinkargs => ("EXPERIMENTAL_EMPTY_LINKARGS", "experimental-empty-linkargs"),
        ExperimentalConstruct::ExperimentalBroadBoStatementConnective => ("EXPERIMENTAL_BROAD_BO_STATEMENT_CONNECTIVE", "experimental-broad-bo-statement-connective"),
        ExperimentalConstruct::ExperimentalBroadKePredicateContinuation => ("EXPERIMENTAL_BROAD_KE_PREDICATE_CONTINUATION", "experimental-broad-ke-predicate-continuation"),
        ExperimentalConstruct::ExperimentalTermHierarchyBoConnection => ("EXPERIMENTAL_TERM_HIERARCHY_BO_CONNECTION", "experimental-term-hierarchy-bo-connection"),
        ExperimentalConstruct::ExperimentalBareNaTerm => ("EXPERIMENTAL_BARE_NA_TERM", "experimental-bare-na-term"),
        ExperimentalConstruct::ExperimentalXohiTagSelbri => ("EXPERIMENTAL_XOHI_TAG_SELBRI", "experimental-xohi-tag-selbri"),
        ExperimentalConstruct::ExperimentalZantufaCmavo => ("EXPERIMENTAL_ZANTUFA_CMAVO", "experimental-zantufa-cmavo"),
        ExperimentalConstruct::ExperimentalZantufaForethoughtGihi => ("EXPERIMENTAL_ZANTUFA_FORETHOUGHT_GIHI", "experimental-zantufa-forethought-gihi"),
        ExperimentalConstruct::ExperimentalZantufaNaryForethought => ("EXPERIMENTAL_ZANTUFA_NARY_FORETHOUGHT", "experimental-zantufa-nary-forethought"),
        ExperimentalConstruct::ExperimentalZantufaGek => ("EXPERIMENTAL_ZANTUFA_GEK", "experimental-zantufa-gek"),
        ExperimentalConstruct::ExperimentalZantufaPoihaBrigahi => ("EXPERIMENTAL_ZANTUFA_POIHA_BRIGAHI", "experimental-zantufa-poiha-brigahi"),
        ExperimentalConstruct::ExperimentalZantufaJaiTagTerm => ("EXPERIMENTAL_ZANTUFA_JAI_TAG_TERM", "experimental-zantufa-jai-tag-term"),
        ExperimentalConstruct::ExperimentalZantufaTag => ("EXPERIMENTAL_ZANTUFA_TAG", "experimental-zantufa-tag"),
        ExperimentalConstruct::ExperimentalZantufaGroupedBridiTail => ("EXPERIMENTAL_ZANTUFA_GROUPED_BRIDI_TAIL", "experimental-zantufa-grouped-bridi-tail"),
        ExperimentalConstruct::ExperimentalZantufaStatementTerms => ("EXPERIMENTAL_ZANTUFA_STATEMENT_TERMS", "experimental-zantufa-statement-terms"),
        ExperimentalConstruct::ExperimentalZantufaStatementRelativeClause => ("EXPERIMENTAL_ZANTUFA_STATEMENT_RELATIVE_CLAUSE", "experimental-zantufa-statement-relative-clause"),
        ExperimentalConstruct::ExperimentalZantufaStatementFreeModifier => ("EXPERIMENTAL_ZANTUFA_STATEMENT_FREE_MODIFIER", "experimental-zantufa-statement-free-modifier"),
        ExperimentalConstruct::ExperimentalZantufaStatementAbstraction => ("EXPERIMENTAL_ZANTUFA_STATEMENT_ABSTRACTION", "experimental-zantufa-statement-abstraction"),
        ExperimentalConstruct::ExperimentalZantufaMex => ("EXPERIMENTAL_ZANTUFA_MEX", "experimental-zantufa-mex"),
        ExperimentalConstruct::ExperimentalZantufaRahoiQuote => ("EXPERIMENTAL_ZANTUFA_RAHOI_QUOTE", "experimental-zantufa-rahoi-quote"),
        ExperimentalConstruct::ExperimentalZantufaMuhoiSelbriUnit => ("EXPERIMENTAL_ZANTUFA_MUHOI_SELBRI_UNIT", "experimental-zantufa-muhoi-selbri-unit"),
        ExperimentalConstruct::ExperimentalZantufaLuheiSelbriUnit => ("EXPERIMENTAL_ZANTUFA_LUHEI_SELBRI_UNIT", "experimental-zantufa-luhei-selbri-unit"),
        ExperimentalConstruct::CllProhibitedFreeModifierPlacement => ("CLL_PROHIBITED_FREE_MODIFIER_PLACEMENT", "cll-prohibited-free-modifier-placement"),
    }
);

/// Immutable two-tier error budget for recovered syntax parsing.
#[invariant(true)]
#[pyclass(
    name = "SyntaxRecoveryErrorPolicy",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PySyntaxRecoveryErrorPolicy {
    value: SyntaxRecoveryErrorPolicy,
}

impl PySyntaxRecoveryErrorPolicy {
    #[requires(true)]
    #[ensures(ret.value.per_statement() == old(value.per_statement()))]
    #[ensures(ret.value.global_hard_cap() == old(value.global_hard_cap()))]
    fn from_rust(value: SyntaxRecoveryErrorPolicy) -> Self {
        Self { value }
    }

    #[requires(true)]
    #[ensures(ret == &self.value)]
    fn rust(&self) -> &SyntaxRecoveryErrorPolicy {
        &self.value
    }
}

#[pymethods]
impl PySyntaxRecoveryErrorPolicy {
    #[classattr]
    const DEFAULT_PER_STATEMENT: usize = SyntaxRecoveryErrorPolicy::DEFAULT_PER_STATEMENT.get();

    #[classattr]
    const DEFAULT_GLOBAL_HARD_CAP: usize = SyntaxRecoveryErrorPolicy::DEFAULT_GLOBAL_HARD_CAP.get();

    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("per_statement", "global_hard_cap");

    /// Construct a checked syntax recovery error policy.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (*, per_statement=DEFAULT_RECOVERY_ERRORS_PER_STATEMENT, global_hard_cap=DEFAULT_RECOVERY_ERROR_HARD_CAP))]
    fn new(per_statement: i128, global_hard_cap: i128) -> PyResult<Self> {
        Ok(Self::from_rust(
            SyntaxRecoveryErrorPolicy::default()
                .with_per_statement_limit(checked_recovery_limit(per_statement, "per_statement")?)
                .with_global_hard_cap(checked_recovery_limit(global_hard_cap, "global_hard_cap")?),
        ))
    }

    /// Return the non-zero recovery error limit for one statement.
    #[requires(true)]
    #[ensures(ret == self.value.per_statement().get())]
    #[getter]
    fn per_statement(&self) -> usize {
        self.value.per_statement().get()
    }

    /// Return the non-zero recovery error limit for the complete input.
    #[requires(true)]
    #[ensures(ret == self.value.global_hard_cap().get())]
    #[getter]
    fn global_hard_cap(&self) -> usize {
        self.value.global_hard_cap().get()
    }

    /// Return a copy with a checked per-statement recovery error limit.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| i128::try_from(value.rust().per_statement().get()).ok() == Some(limit)) || ret.is_err())]
    fn with_per_statement_limit(&self, limit: i128) -> PyResult<Self> {
        Ok(Self::from_rust(
            self.value
                .clone()
                .with_per_statement_limit(checked_recovery_limit(limit, "per_statement")?),
        ))
    }

    /// Return a copy with a checked global recovery error hard cap.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| i128::try_from(value.rust().global_hard_cap().get()).ok() == Some(limit)) || ret.is_err())]
    fn with_global_hard_cap(&self, limit: i128) -> PyResult<Self> {
        Ok(Self::from_rust(self.value.clone().with_global_hard_cap(
            checked_recovery_limit(limit, "global_hard_cap")?,
        )))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.syntax.SyntaxRecoveryErrorPolicy(per_statement={}, global_hard_cap={})",
            self.value.per_statement(),
            self.value.global_hard_cap()
        )
    }
}

/// Immutable strict/recovered syntax parser configuration.
#[invariant(true)]
#[pyclass(
    name = "ParseOptions",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyParseOptions {
    value: Arc<ParseOptions>,
}

impl PyParseOptions {
    #[requires(value.recovery_error_policy.per_statement().get() > 0)]
    #[requires(value.recovery_error_policy.global_hard_cap().get() > 0)]
    #[expensive_ensures(ret.value.as_ref() == &old(value.clone()))]
    fn from_rust(value: ParseOptions) -> Self {
        Self {
            value: Arc::new(value),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.value.as_ref())]
    fn rust(&self) -> &ParseOptions {
        self.value.as_ref()
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|converted| i128::try_from(*converted).ok() == Some(value)) || ret.is_err())]
fn checked_usize(value: i128, parameter: &str) -> PyResult<usize> {
    usize::try_from(value).map_err(|_| {
        InvalidInputError::new_err(format!(
            "{parameter} must be a nonnegative platform-sized integer"
        ))
    })
}

#[requires(!parameter.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|converted| *converted > 0) || ret.is_err())]
fn checked_recovery_limit(value: i128, parameter: &str) -> PyResult<usize> {
    if value <= 0 {
        return Err(InvalidInputError::new_err(format!(
            "{parameter} must be greater than zero"
        )));
    }
    checked_usize(value, parameter)
}

#[pymethods]
impl PyParseOptions {
    /// Construct syntax options by overriding values obtained from Rust defaults.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (*, dialect=None, trace=None, error_context_depth=None, recovery_error_policy=None, max_recovery_errors=None))]
    fn new(
        dialect: Option<PyRef<'_, PyDialectDefinition>>,
        trace: Option<PyRef<'_, PyTraceOptions>>,
        error_context_depth: Option<i128>,
        recovery_error_policy: Option<PyRef<'_, PySyntaxRecoveryErrorPolicy>>,
        max_recovery_errors: Option<i128>,
    ) -> PyResult<Self> {
        if recovery_error_policy.is_some() && max_recovery_errors.is_some() {
            return Err(InvalidInputError::new_err(
                "recovery_error_policy and max_recovery_errors are mutually exclusive",
            ));
        }
        let mut value = ParseOptions::default();
        if let Some(dialect) = dialect {
            value = value.with_dialect_definition(dialect.rust());
        }
        if let Some(trace) = trace {
            value = value.with_trace_options(trace.rust().clone());
        }
        if let Some(depth) = error_context_depth {
            value = value.with_error_context_depth(checked_usize(depth, "error_context_depth")?);
        }
        if let Some(policy) = recovery_error_policy {
            value.recovery_error_policy = policy.rust().clone();
        }
        if let Some(limit) = max_recovery_errors {
            value = value
                .with_max_recovery_errors(checked_recovery_limit(limit, "max_recovery_errors")?);
        }
        Ok(Self::from_rust(value))
    }

    /// Return the complete Rust default syntax options.
    #[requires(true)]
    #[ensures(true)]
    #[staticmethod]
    fn default() -> Self {
        Self::from_rust(ParseOptions::default())
    }

    /// Return a copy using the supplied declarative syntax dialect.
    #[requires(true)]
    #[ensures(ret.rust().dialect == *dialect.rust())]
    fn with_dialect(&self, dialect: PyRef<'_, PyDialectDefinition>) -> Self {
        Self::from_rust(
            self.value
                .as_ref()
                .clone()
                .with_dialect_definition(dialect.rust()),
        )
    }

    /// Return a copy using the supplied trace configuration.
    #[requires(true)]
    #[ensures(ret.rust().trace == *trace.rust())]
    fn with_trace(&self, trace: PyRef<'_, PyTraceOptions>) -> Self {
        Self::from_rust(
            self.value
                .as_ref()
                .clone()
                .with_trace_options(trace.rust().clone()),
        )
    }

    /// Return a copy using the requested syntax-context nesting depth.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| i128::try_from(value.rust().error_context_depth).ok() == Some(depth)) || ret.is_err())]
    fn with_error_context_depth(&self, depth: i128) -> PyResult<Self> {
        Ok(Self::from_rust(
            self.value
                .as_ref()
                .clone()
                .with_error_context_depth(checked_usize(depth, "error_context_depth")?),
        ))
    }

    /// Return a copy using the supplied two-tier syntax recovery error policy.
    #[requires(true)]
    #[ensures(ret.rust().recovery_error_policy == *policy.rust())]
    fn with_recovery_error_policy(&self, policy: PyRef<'_, PySyntaxRecoveryErrorPolicy>) -> Self {
        let mut value = self.value.as_ref().clone();
        value.recovery_error_policy = policy.rust().clone();
        Self::from_rust(value)
    }

    /// Return a copy using a checked non-zero syntax recovery limit.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| i128::try_from(value.rust().recovery_error_policy.global_hard_cap().get()).ok() == Some(limit)) || ret.is_err())]
    fn with_max_recovery_errors(&self, limit: i128) -> PyResult<Self> {
        Ok(Self::from_rust(
            self.value
                .as_ref()
                .clone()
                .with_max_recovery_errors(checked_recovery_limit(limit, "max_recovery_errors")?),
        ))
    }

    /// Return the immutable declarative syntax dialect.
    #[requires(true)]
    #[ensures(ret.rust() == &self.value.dialect)]
    #[getter]
    fn dialect(&self) -> PyDialectDefinition {
        PyDialectDefinition::from_rust(self.value.dialect.clone())
    }

    /// Return the immutable syntax trace configuration.
    #[requires(true)]
    #[ensures(ret.rust() == &self.value.trace)]
    #[getter]
    fn trace(&self) -> PyTraceOptions {
        PyTraceOptions::from_rust(self.value.trace.clone())
    }

    /// Return the number of nested construct contexts retained on errors.
    #[requires(true)]
    #[ensures(ret == self.value.error_context_depth)]
    #[getter]
    fn error_context_depth(&self) -> usize {
        self.value.error_context_depth
    }

    /// Return the immutable two-tier syntax recovery error policy.
    #[requires(true)]
    #[ensures(ret.rust() == &self.value.recovery_error_policy)]
    #[getter]
    fn recovery_error_policy(&self) -> PySyntaxRecoveryErrorPolicy {
        PySyntaxRecoveryErrorPolicy::from_rust(self.value.recovery_error_policy.clone())
    }

    /// Return the non-zero maximum recovered syntax-error count.
    #[requires(true)]
    #[ensures(ret == self.value.recovery_error_policy.global_hard_cap().get())]
    #[getter]
    fn max_recovery_errors(&self) -> usize {
        self.value.recovery_error_policy.global_hard_cap().get()
    }
}

/// Non-empty token range treated as one syntax text unit.
#[invariant(
    true,
    "the retained Rust text unit enforces its ordered non-empty range"
)]
#[pyclass(
    name = "SyntaxTextUnit",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PySyntaxTextUnit {
    value: SyntaxTextUnit,
}

#[pymethods]
impl PySyntaxTextUnit {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("token_start", "token_end");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    /// Construct a checked half-open token range.
    fn new(token_start: usize, token_end: usize) -> PyResult<Self> {
        if token_start >= token_end {
            return Err(InvalidInputError::new_err(
                "SyntaxTextUnit requires token_start < token_end",
            ));
        }
        Ok(Self {
            value: new!(SyntaxTextUnit {
                token_start,
                token_end,
            }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.token_start)]
    #[getter]
    /// Return the inclusive token start index.
    fn token_start(&self) -> usize {
        self.value.token_start
    }

    #[requires(true)]
    #[ensures(ret == self.value.token_end)]
    #[getter]
    /// Return the exclusive token end index.
    fn token_end(&self) -> usize {
        self.value.token_end
    }
}

#[requires(true)]
#[ensures(true)]
fn native_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn enum_from_python<E: PythonStringEnum>(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<E> {
    extract_string_enum(&native_module(py)?, value)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn enum_to_python<E: PythonStringEnum>(py: Python<'_>, value: E) -> PyResult<Py<PyAny>> {
    string_enum_member(&native_module(py)?, value).map(Bound::unbind)
}

/// Boundary event emitted while partitioning syntax text.
#[invariant(true, "private construction fixes the retained Rust event variant")]
#[pyclass(
    name = "SyntaxTextStructureEventBoundary",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PySyntaxTextStructureEventBoundary {
    value: SyntaxTextStructureEvent,
}

#[pymethods]
impl PySyntaxTextStructureEventBoundary {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("kind", "depth");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, kind: &Bound<'_, PyAny>, depth: usize) -> PyResult<Self> {
        Ok(Self {
            value: new!(SyntaxTextStructureEvent::Boundary {
                kind: enum_from_python(py, kind)?,
                depth,
            }),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the boundary kind.
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(SyntaxTextStructureEvent::Boundary { kind, .. }) = self.value.as_data() else {
            unreachable!("private class fixes the event variant")
        };
        enum_to_python(py, *kind)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the container nesting depth.
    fn depth(&self) -> usize {
        let data!(SyntaxTextStructureEvent::Boundary { depth, .. }) = self.value.as_data() else {
            unreachable!("private class fixes the event variant")
        };
        *depth
    }
}

/// Container-open event emitted while partitioning syntax text.
#[invariant(true, "private construction fixes the retained Rust event variant")]
#[pyclass(
    name = "SyntaxTextStructureEventContainerOpen",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PySyntaxTextStructureEventContainerOpen {
    value: SyntaxTextStructureEvent,
}

#[pymethods]
impl PySyntaxTextStructureEventContainerOpen {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("opener", "depth");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, opener: &Bound<'_, PyAny>, depth: usize) -> PyResult<Self> {
        let opener = enum_from_python(py, opener)?;
        if !matches!(
            opener,
            jbotci_morphology::Cmavo::Lu
                | jbotci_morphology::Cmavo::Tuhe
                | jbotci_morphology::Cmavo::To
        ) {
            return Err(InvalidInputError::new_err(
                "container opener must be lu, tu'e, or to",
            ));
        }
        Ok(Self {
            value: new!(SyntaxTextStructureEvent::ContainerOpen { opener, depth }),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the opening cmavo.
    fn opener(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(SyntaxTextStructureEvent::ContainerOpen { opener, .. }) = self.value.as_data()
        else {
            unreachable!("private class fixes the event variant")
        };
        enum_to_python(py, *opener)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the new container nesting depth.
    fn depth(&self) -> usize {
        let data!(SyntaxTextStructureEvent::ContainerOpen { depth, .. }) = self.value.as_data()
        else {
            unreachable!("private class fixes the event variant")
        };
        *depth
    }
}

/// Container-close event emitted while partitioning syntax text.
#[invariant(true, "private construction fixes the retained Rust event variant")]
#[pyclass(
    name = "SyntaxTextStructureEventContainerClose",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PySyntaxTextStructureEventContainerClose {
    value: SyntaxTextStructureEvent,
}

#[pymethods]
impl PySyntaxTextStructureEventContainerClose {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("closer", "depth", "matched");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        py: Python<'_>,
        closer: &Bound<'_, PyAny>,
        depth: usize,
        matched: bool,
    ) -> PyResult<Self> {
        let closer = enum_from_python(py, closer)?;
        if !matches!(
            closer,
            jbotci_morphology::Cmavo::Lihu
                | jbotci_morphology::Cmavo::Tuhu
                | jbotci_morphology::Cmavo::Toi
        ) {
            return Err(InvalidInputError::new_err(
                "container closer must be li'u, tu'u, or toi",
            ));
        }
        Ok(Self {
            value: new!(SyntaxTextStructureEvent::ContainerClose {
                closer,
                depth,
                matched,
            }),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the closing cmavo.
    fn closer(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(SyntaxTextStructureEvent::ContainerClose { closer, .. }) = self.value.as_data()
        else {
            unreachable!("private class fixes the event variant")
        };
        enum_to_python(py, *closer)
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the resulting container nesting depth.
    fn depth(&self) -> usize {
        let data!(SyntaxTextStructureEvent::ContainerClose { depth, .. }) = self.value.as_data()
        else {
            unreachable!("private class fixes the event variant")
        };
        *depth
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return whether the closer matched an open container.
    fn matched(&self) -> bool {
        let data!(SyntaxTextStructureEvent::ContainerClose { matched, .. }) = self.value.as_data()
        else {
            unreachable!("private class fixes the event variant")
        };
        *matched
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn structure_event_to_python(
    py: Python<'_>,
    value: SyntaxTextStructureEvent,
) -> PyResult<Py<PyAny>> {
    match value.as_data() {
        data!(SyntaxTextStructureEvent::Boundary { .. }) => {
            Ok(Py::new(py, PySyntaxTextStructureEventBoundary { value })?.into_any())
        }
        data!(SyntaxTextStructureEvent::ContainerOpen { .. }) => {
            Ok(Py::new(py, PySyntaxTextStructureEventContainerOpen { value })?.into_any())
        }
        data!(SyntaxTextStructureEvent::ContainerClose { .. }) => {
            Ok(Py::new(py, PySyntaxTextStructureEventContainerClose { value })?.into_any())
        }
    }
}

/// Named parser construct and the source range active at failure.
#[invariant(true, "the retained Rust context enforces its name and byte range")]
#[pyclass(
    name = "SyntaxConstructContext",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxConstructContext {
    value: SyntaxConstructContext,
}

#[pymethods]
impl PySyntaxConstructContext {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("construct", "byte_start", "byte_end");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(construct: String, byte_start: usize, byte_end: usize) -> PyResult<Self> {
        if construct.is_empty() {
            return Err(InvalidInputError::new_err("construct must not be empty"));
        }
        if byte_start > byte_end {
            return Err(InvalidInputError::new_err(
                "byte_start must not exceed byte_end",
            ));
        }
        Ok(Self {
            value: SyntaxConstructContext::new(construct, byte_start, byte_end),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.construct.as_str())]
    #[getter]
    /// Return the grammar construct name.
    fn construct(&self) -> &str {
        &self.value.construct
    }

    #[requires(true)]
    #[ensures(ret == self.value.byte_start)]
    #[getter]
    /// Return the inclusive UTF-8 byte start.
    fn byte_start(&self) -> usize {
        self.value.byte_start
    }

    #[requires(true)]
    #[ensures(ret == self.value.byte_end)]
    #[getter]
    /// Return the exclusive UTF-8 byte end.
    fn byte_end(&self) -> usize {
        self.value.byte_end
    }
}

/// Expected concrete cmavo alternative.
#[invariant(
    true,
    "private construction fixes the retained Rust expected-token variant"
)]
#[pyclass(
    name = "SyntaxExpectedTokenCmavo",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectedTokenCmavo {
    value: SyntaxExpectedToken,
}

#[pymethods]
impl PySyntaxExpectedTokenCmavo {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("cmavo",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            value: new!(SyntaxExpectedToken::Cmavo(enum_from_python(py, cmavo)?)),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the expected cmavo.
    fn cmavo(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(SyntaxExpectedToken::Cmavo(cmavo)) = self.value.as_data() else {
            unreachable!("private class fixes the expected-token variant")
        };
        enum_to_python(py, *cmavo)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    /// Return a compact human-readable expectation.
    fn summary_text(&self) -> String {
        self.value.summary_text()
    }
}

/// Expected selma'o alternative.
#[invariant(
    true,
    "private construction fixes the retained Rust expected-token variant"
)]
#[pyclass(
    name = "SyntaxExpectedTokenSelmaho",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectedTokenSelmaho {
    value: SyntaxExpectedToken,
}

#[pymethods]
impl PySyntaxExpectedTokenSelmaho {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("selmaho",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            value: new!(SyntaxExpectedToken::Selmaho(
                enum_from_python(py, selmaho,)?
            )),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the expected selma'o.
    fn selmaho(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(SyntaxExpectedToken::Selmaho(selmaho)) = self.value.as_data() else {
            unreachable!("private class fixes the expected-token variant")
        };
        enum_to_python(py, *selmaho)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    /// Return a compact human-readable expectation.
    fn summary_text(&self) -> String {
        self.value.summary_text()
    }
}

/// Expected morphology word-category alternative.
#[invariant(
    true,
    "private construction fixes the retained Rust expected-token variant"
)]
#[pyclass(
    name = "SyntaxExpectedTokenWordCategory",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectedTokenWordCategory {
    value: SyntaxExpectedToken,
}

#[pymethods]
impl PySyntaxExpectedTokenWordCategory {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("category",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, category: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            value: new!(SyntaxExpectedToken::WordCategory(enum_from_python(
                py, category,
            )?)),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the expected word category.
    fn category(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(SyntaxExpectedToken::WordCategory(category)) = self.value.as_data() else {
            unreachable!("private class fixes the expected-token variant")
        };
        enum_to_python(py, *category)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    /// Return a compact human-readable expectation.
    fn summary_text(&self) -> String {
        self.value.summary_text()
    }
}

/// Expected end-of-input alternative.
#[invariant(
    true,
    "private construction fixes the retained Rust expected-token variant"
)]
#[pyclass(
    name = "SyntaxExpectedTokenEndOfInput",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectedTokenEndOfInput {
    value: SyntaxExpectedToken,
}

#[pymethods]
impl PySyntaxExpectedTokenEndOfInput {
    #[classattr]
    #[allow(non_upper_case_globals)]
    #[requires(true)]
    #[ensures(ret.bind(py).is_empty())]
    fn __match_args__(py: Python<'_>) -> Py<PyTuple> {
        PyTuple::empty(py).unbind()
    }

    #[requires(true)]
    #[ensures(matches!(ret.value.as_data(), data!(SyntaxExpectedToken::EndOfInput)))]
    #[new]
    fn new() -> Self {
        Self {
            value: new!(SyntaxExpectedToken::EndOfInput),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    /// Return a compact human-readable expectation.
    fn summary_text(&self) -> String {
        self.value.summary_text()
    }
}

/// Expected named grammar token alternative.
#[invariant(
    true,
    "private construction fixes the retained Rust expected-token variant"
)]
#[pyclass(
    name = "SyntaxExpectedTokenNamed",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectedTokenNamed {
    value: SyntaxExpectedToken,
}

#[pymethods]
impl PySyntaxExpectedTokenNamed {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("name",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(name: String) -> PyResult<Self> {
        if name.is_empty() {
            return Err(InvalidInputError::new_err("name must not be empty"));
        }
        Ok(Self {
            value: new!(SyntaxExpectedToken::Named(name)),
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the expected grammar token name.
    fn name(&self) -> &str {
        let data!(SyntaxExpectedToken::Named(name)) = self.value.as_data() else {
            unreachable!("private class fixes the expected-token variant")
        };
        name
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    /// Return a compact human-readable expectation.
    fn summary_text(&self) -> String {
        self.value.summary_text()
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn expected_token_from_python(value: &Bound<'_, PyAny>) -> PyResult<SyntaxExpectedToken> {
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectedTokenCmavo>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectedTokenSelmaho>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectedTokenWordCategory>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectedTokenEndOfInput>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectedTokenNamed>>() {
        return Ok(value.value.clone());
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.syntax SyntaxExpectedToken variant",
    ))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn expected_token_to_python(py: Python<'_>, value: SyntaxExpectedToken) -> PyResult<Py<PyAny>> {
    match value.as_data() {
        data!(SyntaxExpectedToken::Cmavo(_)) => {
            Ok(Py::new(py, PySyntaxExpectedTokenCmavo { value })?.into_any())
        }
        data!(SyntaxExpectedToken::Selmaho(_)) => {
            Ok(Py::new(py, PySyntaxExpectedTokenSelmaho { value })?.into_any())
        }
        data!(SyntaxExpectedToken::WordCategory(_)) => {
            Ok(Py::new(py, PySyntaxExpectedTokenWordCategory { value })?.into_any())
        }
        data!(SyntaxExpectedToken::EndOfInput) => {
            Ok(Py::new(py, PySyntaxExpectedTokenEndOfInput { value })?.into_any())
        }
        data!(SyntaxExpectedToken::Named(_)) => {
            Ok(Py::new(py, PySyntaxExpectedTokenNamed { value })?.into_any())
        }
    }
}

/// Expectation reason that continues the current construct.
#[invariant(true, "private construction fixes the retained Rust reason variant")]
#[pyclass(
    name = "SyntaxExpectationReasonContinueCurrent",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectationReasonContinueCurrent {
    value: SyntaxExpectationReason,
}

#[pymethods]
impl PySyntaxExpectationReasonContinueCurrent {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("construct",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(construct: String) -> PyResult<Self> {
        if construct.is_empty() {
            return Err(InvalidInputError::new_err("construct must not be empty"));
        }
        Ok(Self {
            value: new!(SyntaxExpectationReason::ContinueCurrent { construct }),
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the current grammar construct.
    fn construct(&self) -> &str {
        self.value.construct()
    }
}

/// Expectation reason that starts one nested construct.
#[invariant(true, "private construction fixes the retained Rust reason variant")]
#[pyclass(
    name = "SyntaxExpectationReasonStartNested",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectationReasonStartNested {
    value: SyntaxExpectationReason,
}

#[pymethods]
impl PySyntaxExpectationReasonStartNested {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("construct",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(construct: String) -> PyResult<Self> {
        if construct.is_empty() {
            return Err(InvalidInputError::new_err("construct must not be empty"));
        }
        Ok(Self {
            value: new!(SyntaxExpectationReason::StartNested { construct }),
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the nested grammar construct.
    fn construct(&self) -> &str {
        self.value.construct()
    }
}

/// Expectation reason that ends constructs before starting another.
#[invariant(true, "private construction fixes the retained Rust reason variant")]
#[pyclass(
    name = "SyntaxExpectationReasonEndThenStart",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectationReasonEndThenStart {
    value: SyntaxExpectationReason,
}

#[pymethods]
impl PySyntaxExpectationReasonEndThenStart {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("starts", "ends");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(starts: String, ends: &Bound<'_, PyAny>) -> PyResult<Self> {
        if starts.is_empty() {
            return Err(InvalidInputError::new_err("starts must not be empty"));
        }
        let ends = extract_sequence(ends, "ends", |value| value.extract::<String>())?;
        if ends.iter().any(String::is_empty) {
            return Err(InvalidInputError::new_err(
                "ends must not contain empty construct names",
            ));
        }
        Ok(Self {
            value: new!(SyntaxExpectationReason::EndThenStart { starts, ends }),
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the construct that would start.
    fn starts(&self) -> &str {
        self.value.construct()
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return constructs that would end first.
    fn ends(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let data!(SyntaxExpectationReason::EndThenStart { ends, .. }) = self.value.as_data() else {
            unreachable!("private class fixes the expectation-reason variant")
        };
        sequence_to_tuple(py, ends.iter().map(String::as_str)).map(Bound::unbind)
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn expectation_reason_from_python(value: &Bound<'_, PyAny>) -> PyResult<SyntaxExpectationReason> {
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectationReasonContinueCurrent>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectationReasonStartNested>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySyntaxExpectationReasonEndThenStart>>() {
        return Ok(value.value.clone());
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.syntax SyntaxExpectationReason variant",
    ))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn expectation_reason_to_python(
    py: Python<'_>,
    value: SyntaxExpectationReason,
) -> PyResult<Py<PyAny>> {
    match value.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { .. }) => {
            Ok(Py::new(py, PySyntaxExpectationReasonContinueCurrent { value })?.into_any())
        }
        data!(SyntaxExpectationReason::StartNested { .. }) => {
            Ok(Py::new(py, PySyntaxExpectationReasonStartNested { value })?.into_any())
        }
        data!(SyntaxExpectationReason::EndThenStart { .. }) => {
            Ok(Py::new(py, PySyntaxExpectationReasonEndThenStart { value })?.into_any())
        }
    }
}

/// One parser continuation expectation with a non-empty token set.
#[invariant(true, "the retained Rust expectation enforces a non-empty token set")]
#[pyclass(
    name = "SyntaxExpectation",
    frozen,
    eq,
    hash,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySyntaxExpectation {
    value: SyntaxExpectation,
}

#[pymethods]
impl PySyntaxExpectation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("tokens", "reason");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(tokens: &Bound<'_, PyAny>, reason: &Bound<'_, PyAny>) -> PyResult<Self> {
        let tokens = extract_sequence(tokens, "tokens", expected_token_from_python)?;
        if tokens.is_empty() {
            return Err(InvalidInputError::new_err(
                "tokens must contain at least one expected token",
            ));
        }
        Ok(Self {
            value: SyntaxExpectation::new(tokens, expectation_reason_from_python(reason)?),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the non-empty expected-token alternatives.
    fn tokens(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .value
            .tokens
            .iter()
            .cloned()
            .map(|value| expected_token_to_python(py, value))
            .collect::<PyResult<Vec<_>>>()?;
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return why the parser expects these tokens.
    fn reason(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        expectation_reason_to_python(py, self.value.reason.clone())
    }
}

/// Typed error indicating that syntax parsing is not implemented for the input.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "SyntaxErrorNotImplemented",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PySyntaxErrorNotImplemented {
    value: Arc<RustSyntaxError>,
}

#[pymethods]
impl PySyntaxErrorNotImplemented {
    #[classattr]
    #[allow(non_upper_case_globals)]
    #[requires(true)]
    #[ensures(ret.bind(py).is_empty())]
    fn __match_args__(py: Python<'_>) -> Py<PyTuple> {
        PyTuple::empty(py).unbind()
    }

    #[requires(true)]
    #[ensures(matches!(ret.value.as_ref(), RustSyntaxError::NotImplemented))]
    #[new]
    fn new() -> Self {
        Self {
            value: Arc::new(RustSyntaxError::NotImplemented),
        }
    }

    #[requires(true)]
    #[ensures(ret == "syntax.not-implemented")]
    #[getter]
    /// Return the stable diagnostic code.
    fn code(&self) -> &'static str {
        "syntax.not-implemented"
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    /// Convert this typed error to a source-aware diagnostic.
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyDiagnostic {
        PyDiagnostic::from_rust(
            self.value
                .to_diagnostic(source_id.map(|value| value.clone_rust()), source),
        )
    }
}

/// Typed syntax parse failure with continuations and construct context.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "SyntaxErrorParse",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PySyntaxErrorParse {
    value: Arc<RustSyntaxError>,
}

#[pymethods]
impl PySyntaxErrorParse {
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
    ) = (
        "kind",
        "byte_start",
        "byte_end",
        "reason",
        "expected",
        "expectations",
        "contexts",
    );

    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        py: Python<'_>,
        kind: &Bound<'_, PyAny>,
        byte_start: usize,
        byte_end: usize,
        reason: String,
        expected: &Bound<'_, PyAny>,
        expectations: &Bound<'_, PyAny>,
        contexts: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        if byte_start > byte_end {
            return Err(InvalidInputError::new_err(
                "byte_start must not exceed byte_end",
            ));
        }
        let expected = extract_sequence(expected, "expected", |value| value.extract::<String>())?;
        let expectations = extract_sequence(expectations, "expectations", |value| {
            value
                .extract::<PyRef<'_, PySyntaxExpectation>>()
                .map(|value| value.value.clone())
                .map_err(|_| PyTypeError::new_err("expected a SyntaxExpectation"))
        })?;
        let contexts = extract_sequence(contexts, "contexts", |value| {
            value
                .extract::<PyRef<'_, PySyntaxConstructContext>>()
                .map(|value| value.value.clone())
                .map_err(|_| PyTypeError::new_err("expected a SyntaxConstructContext"))
        })?;
        Ok(Self {
            value: Arc::new(RustSyntaxError::Parse {
                kind: enum_from_python(py, kind)?,
                byte_start,
                byte_end,
                reason,
                expected,
                expectations,
                contexts,
            }),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the stable parse-error kind.
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let RustSyntaxError::Parse { kind, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        enum_to_python(py, *kind)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the stable diagnostic code.
    fn code(&self) -> &'static str {
        let RustSyntaxError::Parse { kind, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        kind.code()
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the inclusive UTF-8 failure byte start.
    fn byte_start(&self) -> usize {
        let RustSyntaxError::Parse { byte_start, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        *byte_start
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the exclusive UTF-8 failure byte end.
    fn byte_end(&self) -> usize {
        let RustSyntaxError::Parse { byte_end, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        *byte_end
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the parser's human-readable failure reason.
    fn reason(&self) -> &str {
        let RustSyntaxError::Parse { reason, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        reason
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return compact expected-token summaries.
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let RustSyntaxError::Parse { expected, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        sequence_to_tuple(py, expected.iter().map(String::as_str)).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return structured continuation expectations.
    fn expectations(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let RustSyntaxError::Parse { expectations, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        sequence_to_tuple(
            py,
            expectations.iter().cloned().map(PySyntaxExpectation::from),
        )
        .map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return nested parser construct contexts.
    fn contexts(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let RustSyntaxError::Parse { contexts, .. } = self.value.as_ref() else {
            unreachable!("private class fixes the syntax-error variant")
        };
        sequence_to_tuple(
            py,
            contexts
                .iter()
                .cloned()
                .map(|value| PySyntaxConstructContext { value }),
        )
        .map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    /// Convert this typed error to a source-aware diagnostic.
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyDiagnostic {
        PyDiagnostic::from_rust(
            self.value
                .to_diagnostic(source_id.map(|value| value.clone_rust()), source),
        )
    }
}

impl From<SyntaxExpectation> for PySyntaxExpectation {
    #[requires(true)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from(value: SyntaxExpectation) -> Self {
        Self { value }
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn syntax_error_to_python(py: Python<'_>, value: Arc<RustSyntaxError>) -> PyResult<Py<PyAny>> {
    match value.as_ref() {
        RustSyntaxError::NotImplemented => {
            Ok(Py::new(py, PySyntaxErrorNotImplemented { value })?.into_any())
        }
        RustSyntaxError::Parse { .. } => Ok(Py::new(py, PySyntaxErrorParse { value })?.into_any()),
    }
}

/// Typed non-fatal syntax warning with an exact anchor token.
#[invariant(true, "the retained Rust warning enforces source attribution")]
#[pyclass(
    name = "SyntaxWarning",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PySyntaxWarning {
    value: Arc<SyntaxWarning>,
}

#[pymethods]
impl PySyntaxWarning {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("kind", "anchor_index", "anchor");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the warning kind.
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }

    #[requires(true)]
    #[ensures(ret == self.value.anchor_index)]
    #[getter]
    /// Return the warning anchor token index.
    fn anchor_index(&self) -> usize {
        self.value.anchor_index
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the exact anchor token.
    fn anchor(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        syntax_token_to_python(py, TokenHandle::from_rust(self.value.anchor.clone()))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the stable diagnostic code.
    fn code(&self) -> &'static str {
        self.value.kind.code()
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the warning message.
    fn message(&self) -> &'static str {
        self.value.message()
    }

    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    /// Convert this warning to a source-aware diagnostic.
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyDiagnostic {
        PyDiagnostic::from_rust(
            self.value
                .to_diagnostic(source_id.map(|value| value.clone_rust()), source),
        )
    }
}

/// Source-rendered syntax warning coordinates and display text.
#[invariant(
    true,
    "the retained Rust display enforces non-empty labels and messages"
)]
#[pyclass(
    name = "SyntaxWarningDisplay",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PySyntaxWarningDisplay {
    value: SyntaxWarningDisplay,
}

#[pymethods]
impl PySyntaxWarningDisplay {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the source label used for display.
    fn source_label(&self) -> &str {
        &self.value.source_label
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the warning kind.
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the display message.
    fn message(&self) -> &str {
        &self.value.message
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    #[getter]
    /// Return the one-based display line.
    fn line(&self) -> usize {
        self.value.line
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    #[getter]
    /// Return the one-based display column.
    fn column(&self) -> usize {
        self.value.column
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the zero-based selection start within the display line.
    fn selection_start(&self) -> usize {
        self.value.selection_start
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the selected display length.
    fn selection_length(&self) -> usize {
        self.value.selection_length
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the experimental cmavo associated with the warning, if any.
    fn experimental_cmavo(&self) -> Option<&str> {
        self.value.experimental_cmavo.as_deref()
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    /// Return the rendered source context.
    fn context(&self) -> &str {
        &self.value.context
    }
}

/// Successful strict syntax parse retaining the typed tree and all warnings.
#[invariant(true, "the root handle and warnings retain all Rust-owned parser data")]
#[pyclass(
    name = "SyntaxParse",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxParse {
    root: StrictTextRootHandle,
    warnings: Arc<[SyntaxWarning]>,
}

/// Extract the exact strict root owner retained by a successful parse result.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(crate) fn strict_parse_root_from_python(
    value: &Bound<'_, PyAny>,
) -> PyResult<StrictTextRootHandle> {
    value
        .extract::<PyRef<'_, PySyntaxParse>>()
        .map(|parse| parse.root.clone())
        .map_err(|_| {
            pyo3::exceptions::PyTypeError::new_err(
                "expected jbotci.syntax.SyntaxParse or strict TextSyntax",
            )
        })
}

impl PySyntaxParse {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(value: SyntaxParse) -> Self {
        let value = value.into_data();
        Self {
            root: strict_text_root(*value.parse_tree),
            warnings: Arc::from(value.warnings),
        }
    }
}

#[pymethods]
impl PySyntaxParse {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the typed strict syntax root.
    fn parse_tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        strict_text_to_python(py, &self.root)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return syntax warnings in source order.
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.warnings.iter().cloned().map(|value| PySyntaxWarning {
                value: Arc::new(value),
            }),
        )
        .map(Bound::unbind)
    }
}

#[invariant(::Success { .. } => true)]
#[invariant(::Error { .. } => true)]
#[derive(Debug, Clone)]
enum StrictParseOutcome {
    Success { parse: PySyntaxParse },
    Error { error: Arc<RustSyntaxError> },
}

/// Non-raising strict parse attempt with an exact result, error, and optional trace.
#[invariant(true, "the outcome retains exactly one strict parse or typed error")]
#[pyclass(
    name = "SyntaxParseAttempt",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxParseAttempt {
    outcome: StrictParseOutcome,
    trace: Option<jbotci_diagnostics::TraceReport>,
}

impl PySyntaxParseAttempt {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(value: SyntaxParseAttempt) -> Self {
        let outcome = match value.result {
            Ok(parse) => StrictParseOutcome::Success {
                parse: PySyntaxParse::from_rust(parse),
            },
            Err(error) => StrictParseOutcome::Error {
                error: Arc::new(error),
            },
        };
        Self {
            outcome,
            trace: value.trace,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_result(result: Result<SyntaxParse, RustSyntaxError>) -> Self {
        let outcome = match result {
            Ok(parse) => StrictParseOutcome::Success {
                parse: PySyntaxParse::from_rust(parse),
            },
            Err(error) => StrictParseOutcome::Error {
                error: Arc::new(error),
            },
        };
        Self {
            outcome,
            trace: None,
        }
    }
}

#[pymethods]
impl PySyntaxParseAttempt {
    #[requires(true)]
    #[ensures(ret == matches!(&self.outcome, StrictParseOutcome::Success { .. }))]
    #[getter]
    /// Return whether strict parsing succeeded.
    fn succeeded(&self) -> bool {
        matches!(&self.outcome, StrictParseOutcome::Success { .. })
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the strict parse on success.
    fn result(&self) -> Option<PySyntaxParse> {
        match &self.outcome {
            StrictParseOutcome::Success { parse } => Some(parse.clone()),
            StrictParseOutcome::Error { .. } => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the typed syntax error on failure.
    fn error(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match &self.outcome {
            StrictParseOutcome::Success { .. } => Ok(None),
            StrictParseOutcome::Error { error } => {
                syntax_error_to_python(py, Arc::clone(error)).map(Some)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the optional parser trace.
    fn trace(&self) -> Option<PyTraceReport> {
        self.trace.clone().map(PyTraceReport::from_rust)
    }
}

/// Recovered syntax parse retaining typed recovery fields, errors, and warnings.
#[invariant(true, "the root and errors retain the exact recovered parser result")]
#[pyclass(
    name = "RecoveredSyntaxParse",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyRecoveredSyntaxParse {
    root: RecoveredTextRootHandle,
    errors: Arc<[RustSyntaxError]>,
    warnings: Arc<[SyntaxWarning]>,
}

impl PyRecoveredSyntaxParse {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(value: RecoveredSyntaxParse) -> Self {
        let value = value.into_data();
        Self {
            root: recovered_text_root(*value.parse_tree),
            errors: Arc::from(value.errors),
            warnings: Arc::from(value.warnings),
        }
    }
}

/// Recovered parse result paired with its optional trace report.
#[invariant(
    true,
    "the recovered result and optional trace are immutable Rust values"
)]
#[pyclass(
    name = "RecoveredSyntaxParseAttempt",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyRecoveredSyntaxParseAttempt {
    result: PyRecoveredSyntaxParse,
    trace: Option<jbotci_diagnostics::TraceReport>,
}

impl PyRecoveredSyntaxParseAttempt {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(value: RecoveredSyntaxParseAttempt) -> Self {
        Self {
            result: PyRecoveredSyntaxParse::from_rust(value.result),
            trace: value.trace,
        }
    }
}

#[pymethods]
impl PyRecoveredSyntaxParseAttempt {
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the recovered parse result.
    fn result(&self) -> PyRecoveredSyntaxParse {
        self.result.clone()
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the optional parser trace.
    fn trace(&self) -> Option<PyTraceReport> {
        self.trace.clone().map(PyTraceReport::from_rust)
    }
}

/// Strict-success alternative returned by a strict-or-recovered parse attempt.
#[invariant(true, "the payload is a strict parser success")]
#[pyclass(
    name = "SyntaxRecoveryParseValid",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxRecoveryParseValid {
    parse: PySyntaxParse,
}

#[pymethods]
impl PySyntaxRecoveryParseValid {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("parse",);

    #[requires(true)]
    #[ensures(true)]
    #[new]
    fn new(parse: PyRef<'_, PySyntaxParse>) -> Self {
        Self {
            parse: parse.clone(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the strict-success parse payload.
    fn parse(&self) -> PySyntaxParse {
        self.parse.clone()
    }
}

/// Recovered alternative returned by a strict-or-recovered parse attempt.
#[invariant(true, "the payload is the exact recovered parser result")]
#[pyclass(
    name = "SyntaxRecoveryParseRecovered",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxRecoveryParseRecovered {
    parse: PyRecoveredSyntaxParse,
}

#[pymethods]
impl PySyntaxRecoveryParseRecovered {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("parse",);

    #[requires(true)]
    #[ensures(true)]
    #[new]
    fn new(parse: PyRef<'_, PyRecoveredSyntaxParse>) -> Self {
        Self {
            parse: parse.clone(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the recovered parse payload.
    fn parse(&self) -> PyRecoveredSyntaxParse {
        self.parse.clone()
    }
}

#[invariant(::Valid { .. } => true)]
#[invariant(::Recovered { .. } => true)]
#[derive(Debug, Clone)]
enum RecoveryParseOutcome {
    Valid { parse: PySyntaxParse },
    Recovered { parse: PyRecoveredSyntaxParse },
}

/// Non-raising parse attempt returning a closed strict-or-recovered result.
#[invariant(true, "the outcome retains the exact strict-or-recovered Rust variant")]
#[pyclass(
    name = "SyntaxRecoveryParseAttempt",
    frozen,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxRecoveryParseAttempt {
    outcome: RecoveryParseOutcome,
    trace: Option<jbotci_diagnostics::TraceReport>,
}

impl PySyntaxRecoveryParseAttempt {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(value: SyntaxRecoveryParseAttempt) -> Self {
        let outcome = match value.result.into_data() {
            data!(SyntaxRecoveryParse::Valid { parse }) => RecoveryParseOutcome::Valid {
                parse: PySyntaxParse::from_rust(parse),
            },
            data!(SyntaxRecoveryParse::Recovered { parse }) => RecoveryParseOutcome::Recovered {
                parse: PyRecoveredSyntaxParse::from_rust(parse),
            },
        };
        Self {
            outcome,
            trace: value.trace,
        }
    }
}

#[pymethods]
impl PySyntaxRecoveryParseAttempt {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the closed strict-success or recovered result variant.
    fn result(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &self.outcome {
            RecoveryParseOutcome::Valid { parse } => Ok(Py::new(
                py,
                PySyntaxRecoveryParseValid {
                    parse: parse.clone(),
                },
            )?
            .into_any()),
            RecoveryParseOutcome::Recovered { parse } => Ok(Py::new(
                py,
                PySyntaxRecoveryParseRecovered {
                    parse: parse.clone(),
                },
            )?
            .into_any()),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[getter]
    /// Return the optional parser trace.
    fn trace(&self) -> Option<PyTraceReport> {
        self.trace.clone().map(PyTraceReport::from_rust)
    }
}

#[requires(true)]
#[ensures(true)]
fn rust_options(options: Option<&PyParseOptions>) -> ParseOptions {
    options.map_or_else(ParseOptions::default, |value| value.rust().clone())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn words_from_python(value: &Bound<'_, PyAny>) -> PyResult<Vec<jbotci_morphology::WordLike>> {
    extract_sequence(value, "words", |value| {
        extract_word_like(value).map(|handle| handle.into_owned())
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn tokens_from_python(value: &Bound<'_, PyAny>) -> PyResult<Vec<jbotci_syntax::Token>> {
    extract_sequence(value, "tokens", |value| {
        extract_syntax_token(value).map(|handle| handle.clone_rust())
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn warnings_from_python(value: &Bound<'_, PyAny>) -> PyResult<Vec<SyntaxWarning>> {
    extract_sequence(value, "warnings", |value| {
        value
            .extract::<PyRef<'_, PySyntaxWarning>>()
            .map(|value| value.value.as_ref().clone())
            .map_err(|_| PyTypeError::new_err("expected a jbotci.syntax.SyntaxWarning"))
    })
}

/// Normalize morphology words into the exact syntax-token stream.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_syntax_tokens_with_options")]
#[pyo3(signature = (words, *, options=None))]
fn syntax_tokens_with_options(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<Py<PyTuple>> {
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    let tokens = py.detach(|| jbotci_syntax::syntax_tokens_with_options(&words, &options));
    let values = tokens
        .into_iter()
        .map(TokenHandle::from_rust)
        .map(|handle| syntax_token_to_python(py, handle))
        .collect::<PyResult<Vec<_>>>()?;
    sequence_to_tuple(py, values).map(Bound::unbind)
}

/// Partition normalized syntax tokens at formal top-level text boundaries.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_partition_syntax_text_units")]
fn partition_syntax_text_units(
    py: Python<'_>,
    tokens: &Bound<'_, PyAny>,
    granularity: &Bound<'_, PyAny>,
) -> PyResult<Py<PyTuple>> {
    let tokens = tokens_from_python(tokens)?;
    let granularity = enum_from_python(py, granularity)?;
    let units = py.detach(|| jbotci_syntax::partition_syntax_text_units(&tokens, granularity));
    sequence_to_tuple(
        py,
        units.into_iter().map(|value| PySyntaxTextUnit { value }),
    )
    .map(Bound::unbind)
}

/// Return formal boundary/container events for normalized syntax tokens.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_syntax_text_structure")]
fn syntax_text_structure(py: Python<'_>, tokens: &Bound<'_, PyAny>) -> PyResult<Py<PyTuple>> {
    let tokens = tokens_from_python(tokens)?;
    let events = py.detach(|| jbotci_syntax::syntax_text_structure(&tokens));
    let values = events
        .into_iter()
        .map(|value| structure_event_to_python(py, value))
        .collect::<PyResult<Vec<_>>>()?;
    sequence_to_tuple(py, values).map(Bound::unbind)
}

/// Attempt the direct strict `parse_text` Rust operation without raising.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_parse_text_attempt")]
#[pyo3(signature = (words, *, options=None))]
fn parse_text_attempt(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<PySyntaxParseAttempt> {
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    let result = py.detach(|| {
        jbotci_syntax::parse_text(&words, &options).map(|parse_tree| {
            new!(SyntaxParse {
                parse_tree: Box::new(parse_tree),
                warnings: Vec::new(),
            })
        })
    });
    Ok(PySyntaxParseAttempt::from_result(result))
}

/// Attempt strict syntax parsing, retaining structured errors and trace data.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_parse_syntax_tree_attempt")]
#[pyo3(signature = (words, *, source=None, options=None))]
fn parse_syntax_tree_attempt(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    source: Option<String>,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<PySyntaxParseAttempt> {
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    Ok(match source {
        Some(source) => PySyntaxParseAttempt::from_rust(py.detach(|| {
            jbotci_syntax::parse_syntax_tree_with_source_and_options_attempt(
                &words, &source, &options,
            )
        })),
        None => PySyntaxParseAttempt::from_result(
            py.detach(|| jbotci_syntax::parse_syntax_tree_with_options(&words, &options)),
        ),
    })
}

/// Attempt recovered syntax parsing with exact Rust error-slot indices.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_parse_syntax_tree_recovered_attempt")]
#[pyo3(signature = (words, *, source, options=None))]
fn parse_syntax_tree_recovered_attempt(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    source: String,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<PyRecoveredSyntaxParseAttempt> {
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    Ok(PyRecoveredSyntaxParseAttempt::from_rust(py.detach(|| {
        jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options_attempt(
            &words, &source, &options,
        )
    })))
}

/// Attempt strict-or-recovered parsing without converting strict successes.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_parse_syntax_tree_with_recovery_attempt")]
#[pyo3(signature = (words, *, source, options=None))]
fn parse_syntax_tree_with_recovery_attempt(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    source: String,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<PySyntaxRecoveryParseAttempt> {
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    Ok(PySyntaxRecoveryParseAttempt::from_rust(py.detach(|| {
        jbotci_syntax::parse_syntax_tree_with_recovery_with_source_and_options_attempt(
            &words, &source, &options,
        )
    })))
}

/// Return grammar continuations expected after a morphology word prefix.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_expected_continuations")]
#[pyo3(signature = (words, *, options=None))]
fn expected_continuations(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<Py<PyTuple>> {
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    let expectations = py.detach(|| jbotci_syntax::expected_continuations(&words, &options));
    sequence_to_tuple(
        py,
        expectations
            .into_iter()
            .map(|value| PySyntaxExpectation { value }),
    )
    .map(Bound::unbind)
}

/// Return grammar continuations subject to a finite nonnegative wall-clock limit.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_expected_continuations_with_time_limit")]
#[pyo3(signature = (words, time_limit, *, options=None))]
fn expected_continuations_with_time_limit(
    py: Python<'_>,
    words: &Bound<'_, PyAny>,
    time_limit: f64,
    options: Option<PyRef<'_, PyParseOptions>>,
) -> PyResult<Py<PyTuple>> {
    if !time_limit.is_finite() || time_limit < 0.0 {
        return Err(InvalidInputError::new_err(
            "time_limit must be finite and nonnegative",
        ));
    }
    let time_limit = Duration::try_from_secs_f64(time_limit).map_err(|_| {
        InvalidInputError::new_err("time_limit is too large to represent as a duration")
    })?;
    let words = words_from_python(words)?;
    let options = rust_options(options.as_deref());
    let expectations = py.detach(|| {
        jbotci_syntax::expected_continuations_with_time_limit(&words, &options, time_limit)
    });
    sequence_to_tuple(
        py,
        expectations
            .into_iter()
            .map(|value| PySyntaxExpectation { value }),
    )
    .map(Bound::unbind)
}

/// Render one typed syntax warning for terminal-oriented display.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_syntax_warning_display")]
fn syntax_warning_display(
    source_label: &str,
    source: &str,
    tokens: &Bound<'_, PyAny>,
    warning: PyRef<'_, PySyntaxWarning>,
) -> PyResult<PySyntaxWarningDisplay> {
    if source_label.is_empty() {
        return Err(InvalidInputError::new_err("source_label must not be empty"));
    }
    let tokens = tokens_from_python(tokens)?;
    Ok(PySyntaxWarningDisplay {
        value: jbotci_syntax::syntax_warning_display(
            source_label,
            source,
            &tokens,
            warning.value.as_ref(),
        ),
    })
}

/// Render an immutable sequence of typed syntax warnings for display.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction(name = "_syntax_parser_syntax_warning_displays")]
fn syntax_warning_displays(
    py: Python<'_>,
    source_label: &str,
    source: &str,
    tokens: &Bound<'_, PyAny>,
    warnings: &Bound<'_, PyAny>,
) -> PyResult<Py<PyTuple>> {
    if source_label.is_empty() {
        return Err(InvalidInputError::new_err("source_label must not be empty"));
    }
    let tokens = tokens_from_python(tokens)?;
    let warnings = warnings_from_python(warnings)?;
    let displays = jbotci_syntax::syntax_warning_displays(source_label, source, &tokens, &warnings);
    sequence_to_tuple(
        py,
        displays
            .into_iter()
            .map(|value| PySyntaxWarningDisplay { value }),
    )
    .map(Bound::unbind)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_string_enum::<SyntaxTextUnitGranularity>(module)?;
    register_string_enum::<SyntaxTextBoundaryKind>(module)?;
    register_string_enum::<SyntaxErrorKind>(module)?;
    register_string_enum::<jbotci_syntax::SyntaxWordCategory>(module)?;
    register_string_enum::<ExperimentalConstruct>(module)?;
    register_type::<PySyntaxRecoveryErrorPolicy>(
        module,
        "_syntax_parser_SyntaxRecoveryErrorPolicy",
    )?;
    register_type::<PyParseOptions>(module, "_syntax_parser_ParseOptions")?;
    register_type::<PySyntaxTextUnit>(module, "_syntax_parser_SyntaxTextUnit")?;
    register_type::<PySyntaxTextStructureEventBoundary>(
        module,
        "_syntax_parser_SyntaxTextStructureEventBoundary",
    )?;
    register_type::<PySyntaxTextStructureEventContainerOpen>(
        module,
        "_syntax_parser_SyntaxTextStructureEventContainerOpen",
    )?;
    register_type::<PySyntaxTextStructureEventContainerClose>(
        module,
        "_syntax_parser_SyntaxTextStructureEventContainerClose",
    )?;
    register_type::<PySyntaxConstructContext>(module, "_syntax_parser_SyntaxConstructContext")?;
    register_type::<PySyntaxExpectedTokenCmavo>(module, "_syntax_parser_SyntaxExpectedTokenCmavo")?;
    register_type::<PySyntaxExpectedTokenSelmaho>(
        module,
        "_syntax_parser_SyntaxExpectedTokenSelmaho",
    )?;
    register_type::<PySyntaxExpectedTokenWordCategory>(
        module,
        "_syntax_parser_SyntaxExpectedTokenWordCategory",
    )?;
    register_type::<PySyntaxExpectedTokenEndOfInput>(
        module,
        "_syntax_parser_SyntaxExpectedTokenEndOfInput",
    )?;
    register_type::<PySyntaxExpectedTokenNamed>(module, "_syntax_parser_SyntaxExpectedTokenNamed")?;
    register_type::<PySyntaxExpectationReasonContinueCurrent>(
        module,
        "_syntax_parser_SyntaxExpectationReasonContinueCurrent",
    )?;
    register_type::<PySyntaxExpectationReasonStartNested>(
        module,
        "_syntax_parser_SyntaxExpectationReasonStartNested",
    )?;
    register_type::<PySyntaxExpectationReasonEndThenStart>(
        module,
        "_syntax_parser_SyntaxExpectationReasonEndThenStart",
    )?;
    register_type::<PySyntaxExpectation>(module, "_syntax_parser_SyntaxExpectation")?;
    register_type::<PySyntaxErrorNotImplemented>(
        module,
        "_syntax_parser_SyntaxErrorNotImplemented",
    )?;
    register_type::<PySyntaxErrorParse>(module, "_syntax_parser_SyntaxErrorParse")?;
    register_type::<PySyntaxWarning>(module, "_syntax_parser_SyntaxWarning")?;
    register_type::<PySyntaxWarningDisplay>(module, "_syntax_parser_SyntaxWarningDisplay")?;
    register_type::<PySyntaxParse>(module, "_syntax_parser_SyntaxParse")?;
    register_type::<PySyntaxParseAttempt>(module, "_syntax_parser_SyntaxParseAttempt")?;
    register_type::<PyRecoveredSyntaxParse>(module, "_syntax_parser_RecoveredSyntaxParse")?;
    register_type::<PyRecoveredSyntaxParseAttempt>(
        module,
        "_syntax_parser_RecoveredSyntaxParseAttempt",
    )?;
    register_type::<PySyntaxRecoveryParseValid>(module, "_syntax_parser_SyntaxRecoveryParseValid")?;
    register_type::<PySyntaxRecoveryParseRecovered>(
        module,
        "_syntax_parser_SyntaxRecoveryParseRecovered",
    )?;
    register_type::<PySyntaxRecoveryParseAttempt>(
        module,
        "_syntax_parser_SyntaxRecoveryParseAttempt",
    )?;
    register_private_object(
        module,
        "_syntax_parser_SYNTAX_TRACE_FILTERS",
        sequence_to_tuple(
            module.py(),
            jbotci_syntax::SYNTAX_TRACE_FILTERS.iter().copied(),
        )?,
    )?;
    register_private_object(
        module,
        "_syntax_parser_ENUM_INVENTORY",
        sequence_to_tuple(module.py(), PARSER_ENUM_INVENTORY.iter().copied())?,
    )?;
    module.add_function(wrap_pyfunction!(syntax_tokens_with_options, module)?)?;
    module.add_function(wrap_pyfunction!(partition_syntax_text_units, module)?)?;
    module.add_function(wrap_pyfunction!(syntax_text_structure, module)?)?;
    module.add_function(wrap_pyfunction!(parse_text_attempt, module)?)?;
    module.add_function(wrap_pyfunction!(parse_syntax_tree_attempt, module)?)?;
    module.add_function(wrap_pyfunction!(
        parse_syntax_tree_recovered_attempt,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(
        parse_syntax_tree_with_recovery_attempt,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(expected_continuations, module)?)?;
    module.add_function(wrap_pyfunction!(
        expected_continuations_with_time_limit,
        module
    )?)?;
    module.add_function(wrap_pyfunction!(syntax_warning_display, module)?)?;
    module.add_function(wrap_pyfunction!(syntax_warning_displays, module)?)?;
    Ok(())
}

#[pymethods]
impl PyRecoveredSyntaxParse {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return the typed recovered syntax root.
    fn parse_tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        recovered_text_to_python(py, &self.root)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return typed recovery errors in parser order.
    fn errors(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .errors
            .iter()
            .cloned()
            .map(Arc::new)
            .map(|value| syntax_error_to_python(py, value))
            .collect::<PyResult<Vec<_>>>()?;
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    /// Return syntax warnings in source order.
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.warnings.iter().cloned().map(|value| PySyntaxWarning {
                value: Arc::new(value),
            }),
        )
        .map(Bound::unbind)
    }
}
