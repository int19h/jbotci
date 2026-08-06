//! Typed, lazy Python projection of the embedded English dictionary.

use std::mem::size_of;
use std::sync::Arc;
#[cfg(test)]
use std::{
    cell::RefCell,
    marker::PhantomData,
    panic::{AssertUnwindSafe, catch_unwind, panic_any, resume_unwind},
    rc::Rc,
};

#[cfg(test)]
use bityzba::try_new;
use bityzba::{contract_trait, data, invariant, new, requires};
use jbotci_dictionary::{
    DefinitionId, Dictionary, DictionaryEntry, DictionaryLujvoSegmentKind,
    DictionaryValidationError, EntryIndex, Keyword, RafsiAvailability, RafsiAvailabilityData,
    RafsiCandidate, RafsiClaimKind, RafsiSource, Score, WordType, normalize_lookup_query,
    normalize_pattern_lookup_key, universal_gismu_rafsi_forms,
};
use jbotci_dictionary_data::{DictionarySnapshotMetadata, english, english_metadata};
use jbotci_morphology::ShortRafsiShape;
use jbotci_phonetic::{IpaSegmentId, PronunciationTargetId, ipa_segment_symbol};
use pyo3::exceptions::{PyIndexError, PyTypeError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
#[cfg(test)]
use pyo3::types::PyDict;
use pyo3::types::{PyAny, PyModule, PySlice, PySliceMethods, PyString, PyTuple};

use crate::support::{
    PythonStringEnum, extract_string_enum, register_private_object, register_string_enum,
    register_type, sequence_to_tuple, string_enum_member, string_repr,
};

const PUBLIC_MODULE: &str = "jbotci.dictionary";

#[cfg(test)]
thread_local! {
    // Embedded-Python unit tests construct private, unregistered modules. A
    // thread-local resolver keeps concurrent test threads independent even on
    // free-threaded Python, without replacing process-global `sys.modules`.
    static TEST_NATIVE_MODULE: RefCell<Option<Py<PyModule>>> = const { RefCell::new(None) };
    static TEST_PUBLIC_MODULE: RefCell<Option<Py<PyModule>>> = const { RefCell::new(None) };
}

#[requires(true)]
#[ensures(true)]
fn native_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
    #[cfg(test)]
    if let Some(module) =
        TEST_NATIVE_MODULE.with(|slot| slot.borrow().as_ref().map(|module| module.clone_ref(py)))
    {
        return Ok(module.into_bound(py));
    }
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(true)]
fn public_module<'py>(py: Python<'py>) -> PyResult<Bound<'py, PyModule>> {
    #[cfg(test)]
    if let Some(module) =
        TEST_PUBLIC_MODULE.with(|slot| slot.borrow().as_ref().map(|module| module.clone_ref(py)))
    {
        return Ok(module.into_bound(py));
    }
    py.import(PUBLIC_MODULE)
}

/// Ordered inventory of native names owned by this domain.
pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_dictionary_Dictionary",
    "_dictionary_DictionaryEntries",
    "_dictionary_DictionaryEntry",
    "_dictionary_WordType",
    "_dictionary_DefinitionId",
    "_dictionary_Score",
    "_dictionary_Keyword",
    "_dictionary_Rafsi",
    "_dictionary_RawSelmaho",
    "_dictionary_DictionaryUser",
    "_dictionary_EntryIndex",
    "_dictionary_RafsiSource",
    "_dictionary_RafsiMatch",
    "_dictionary_RafsiClaimKind",
    "_dictionary_FreeRafsiAvailability",
    "_dictionary_TakenRafsiAvailability",
    "_dictionary_RafsiCandidate",
    "_dictionary_DictionarySoundEntry",
    "_dictionary_IpaTokenSequenceView",
    "_dictionary_IpaSegmentId",
    "_dictionary_PronunciationTargetSequenceView",
    "_dictionary_PronunciationTargetId",
    "_dictionary_DictionaryLujvoEntry",
    "_dictionary_DictionaryLujvoSegment",
    "_dictionary_DictionaryLujvoSegmentKind",
    "_dictionary_DictionaryPatternEntry",
    "_dictionary_DictionarySnapshotMetadata",
    "_dictionary_InvalidEntryValidationDetail",
    "_dictionary_WordIndexMismatchValidationDetail",
    "_dictionary_RafsiIndexMismatchValidationDetail",
    "_dictionary_SelmahoIndexMismatchValidationDetail",
    "_dictionary_PatternIndexMismatchValidationDetail",
    "_dictionary_InvalidSoundIndexEntryValidationDetail",
    "_dictionary_InvalidLujvoIndexEntryValidationDetail",
    "_dictionary_normalize_lookup_query",
    "_dictionary_normalize_pattern_lookup_key",
    "_dictionary_universal_gismu_rafsi_forms",
    "_dictionary_word_type_is_gismu_like",
    "_dictionary_word_type_is_lujvo_like",
    "_dictionary_english",
    "_dictionary_english_metadata",
];

#[invariant(
    std::ptr::eq(*dictionary, english()),
    "the owner retains exactly the validated embedded English dictionary"
)]
#[derive(Debug)]
struct DictionaryOwner {
    dictionary: &'static Dictionary<'static>,
}

impl DictionaryOwner {
    #[requires(true)]
    #[ensures(true)]
    fn english() -> Self {
        new!(DictionaryOwner {
            dictionary: english(),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary(&self) -> &'static Dictionary<'static> {
        self.dictionary
    }

    /// Recover a typed source position from a reference returned by this
    /// dictionary without a second linear scan or unsafe pointer arithmetic.
    #[requires(
        (entry as *const DictionaryEntry<'static>)
            .addr()
            .checked_sub(self.dictionary.entries().as_ptr().addr())
            .is_some_and(|offset| {
                offset % size_of::<DictionaryEntry<'static>>() == 0
                    && offset / size_of::<DictionaryEntry<'static>>()
                        < self.dictionary.entries().len()
            })
    )]
    #[ensures(ret.0 < self.dictionary.entries().len())]
    fn entry_position(&self, entry: &DictionaryEntry<'static>) -> EntryPosition {
        let entries = self.dictionary.entries();
        let stride = size_of::<DictionaryEntry<'static>>();
        assert!(stride > 0, "dictionary entries must not be zero-sized");
        let base = entries.as_ptr().addr();
        let address = (entry as *const DictionaryEntry<'static>).addr();
        let byte_offset = address
            .checked_sub(base)
            .expect("query result must belong to the dictionary entry slice");
        assert_eq!(
            byte_offset % stride,
            0,
            "query result must be entry-aligned"
        );
        let position = byte_offset / stride;
        assert!(
            entries
                .get(position)
                .is_some_and(|candidate| std::ptr::eq(candidate, entry)),
            "query result must point into the dictionary entry slice"
        );
        EntryPosition(position)
    }
}

#[invariant(
    true,
    "position validity is contextual to its retained dictionary owner"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct EntryPosition(usize);

#[invariant(
    true,
    "position validity is contextual to its retained dictionary owner"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SoundPosition(usize);

#[invariant(
    true,
    "position validity is contextual to its retained dictionary owner"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LujvoPosition(usize);

#[invariant(
    true,
    "position validity is contextual to its retained dictionary owner"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PatternPosition(usize);

#[invariant(true, "both unit variants carry no invalid state")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum KeywordKind {
    Gloss,
    Place,
}

#[invariant(
    true,
    "position validity is contextual to its retained dictionary owner"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct KeywordPosition {
    entry: EntryPosition,
    kind: KeywordKind,
    item: usize,
}

#[invariant(
    true,
    "position validity is contextual to its retained dictionary owner"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct RafsiPosition {
    entry: EntryPosition,
    item: usize,
}

#[invariant(
    position.0 < owner.dictionary().entries().len(),
    "entry position exists in the retained dictionary"
)]
#[derive(Debug, Clone)]
struct EntryReference {
    owner: Arc<DictionaryOwner>,
    position: EntryPosition,
}

impl EntryReference {
    #[requires(position.0 < owner.dictionary().entries().len())]
    #[ensures(ret.position == position)]
    fn new(owner: Arc<DictionaryOwner>, position: EntryPosition) -> Self {
        new!(EntryReference { owner, position })
    }

    #[requires(self.position.0 < self.owner.dictionary().entries().len())]
    #[ensures(std::ptr::eq(ret, &self.owner.dictionary().entries()[self.position.0]))]
    fn entry(&self) -> &'static DictionaryEntry<'static> {
        &self.owner.dictionary().entries()[self.position.0]
    }
}

#[invariant(
    position.0 < owner.dictionary().sound_index().len(),
    "sound position exists in the retained dictionary"
)]
#[derive(Debug, Clone)]
struct SoundReference {
    owner: Arc<DictionaryOwner>,
    position: SoundPosition,
}

impl SoundReference {
    #[requires(position.0 < owner.dictionary().sound_index().len())]
    #[ensures(ret.position == position)]
    fn new(owner: Arc<DictionaryOwner>, position: SoundPosition) -> Self {
        new!(SoundReference { owner, position })
    }

    #[requires(self.position.0 < self.owner.dictionary().sound_index().len())]
    #[ensures(std::ptr::eq(ret, &self.owner.dictionary().sound_index()[self.position.0]))]
    fn sound(&self) -> &'static jbotci_dictionary::DictionarySoundEntry<'static> {
        &self.owner.dictionary().sound_index()[self.position.0]
    }
}

#[invariant(
    position.0 < owner.dictionary().lujvo_index().len(),
    "lujvo position exists in the retained dictionary"
)]
#[derive(Debug, Clone)]
struct LujvoReference {
    owner: Arc<DictionaryOwner>,
    position: LujvoPosition,
}

impl LujvoReference {
    #[requires(position.0 < owner.dictionary().lujvo_index().len())]
    #[ensures(ret.position == position)]
    fn new(owner: Arc<DictionaryOwner>, position: LujvoPosition) -> Self {
        new!(LujvoReference { owner, position })
    }

    #[requires(self.position.0 < self.owner.dictionary().lujvo_index().len())]
    #[ensures(std::ptr::eq(ret, &self.owner.dictionary().lujvo_index()[self.position.0]))]
    fn lujvo(&self) -> &'static jbotci_dictionary::DictionaryLujvoEntry<'static> {
        &self.owner.dictionary().lujvo_index()[self.position.0]
    }
}

#[invariant(
    *item < lujvo.lujvo().segments.len(),
    "segment position exists in the retained lujvo decomposition"
)]
#[derive(Debug, Clone)]
struct LujvoSegmentReference {
    lujvo: LujvoReference,
    item: usize,
}

impl LujvoSegmentReference {
    #[requires(item < lujvo.lujvo().segments.len())]
    #[ensures(ret.item == item)]
    fn new(lujvo: LujvoReference, item: usize) -> Self {
        new!(LujvoSegmentReference { lujvo, item })
    }

    #[requires(self.item < self.lujvo.lujvo().segments.len())]
    #[ensures(std::ptr::eq(ret, &self.lujvo.lujvo().segments[self.item]))]
    fn segment(&self) -> &'static jbotci_dictionary::DictionaryLujvoSegment<'static> {
        &self.lujvo.lujvo().segments[self.item]
    }
}

#[invariant(
    position.0 < owner.dictionary().pattern_index().len(),
    "pattern position exists in the retained dictionary"
)]
#[derive(Debug, Clone)]
struct PatternReference {
    owner: Arc<DictionaryOwner>,
    position: PatternPosition,
}

impl PatternReference {
    #[requires(position.0 < owner.dictionary().pattern_index().len())]
    #[ensures(ret.position == position)]
    fn new(owner: Arc<DictionaryOwner>, position: PatternPosition) -> Self {
        new!(PatternReference { owner, position })
    }

    #[requires(self.position.0 < self.owner.dictionary().pattern_index().len())]
    #[ensures(std::ptr::eq(ret, &self.owner.dictionary().pattern_index()[self.position.0]))]
    fn pattern(&self) -> &'static jbotci_dictionary::DictionaryPatternEntry<'static> {
        &self.owner.dictionary().pattern_index()[self.position.0]
    }
}

#[invariant(
    ::Borrowed => owner.dictionary().entries().get(position.entry.0).is_some_and(|entry| {
        position.item < match position.kind {
            KeywordKind::Gloss => entry.gloss_keywords.len(),
            KeywordKind::Place => entry.place_keywords.len(),
        }
    }),
    "borrowed keyword positions address an existing keyword in the retained dictionary"
)]
#[invariant(
    ::Owned => true,
    "the Rust keyword model permits every owned word and optional-meaning combination"
)]
#[derive(Debug, Clone)]
enum KeywordStorage {
    Borrowed {
        owner: Arc<DictionaryOwner>,
        position: KeywordPosition,
    },
    Owned {
        word: String,
        meaning: Option<String>,
    },
}

impl KeywordStorage {
    #[requires(true)]
    #[ensures(true)]
    fn value(&self) -> Keyword<'_> {
        match self.as_data() {
            data!(KeywordStorage::Borrowed { owner, position }) => {
                let entry = &owner.dictionary().entries()[position.entry.0];
                match position.kind {
                    KeywordKind::Gloss => entry.gloss_keywords[position.item],
                    KeywordKind::Place => entry.place_keywords[position.item],
                }
            }
            data!(KeywordStorage::Owned { word, meaning }) => Keyword {
                word,
                meaning: meaning.as_deref(),
            },
        }
    }
}

#[invariant(
    ::Borrowed => owner.dictionary().entries().get(position.entry.0).is_some_and(|entry| {
        position.item < entry.rafsi.len()
    }),
    "borrowed rafsi positions address an existing listed rafsi in the retained dictionary"
)]
#[invariant(
    ::Owned(_) => true,
    "the Rust rafsi model permits every owned string value"
)]
#[derive(Debug, Clone)]
enum RafsiStorage {
    Borrowed {
        owner: Arc<DictionaryOwner>,
        position: RafsiPosition,
    },
    Owned(String),
}

impl RafsiStorage {
    #[requires(true)]
    #[ensures(true)]
    fn value(&self) -> &str {
        match self.as_data() {
            data!(RafsiStorage::Borrowed { owner, position }) => {
                owner.dictionary().entries()[position.entry.0].rafsi[position.item].0
            }
            data!(RafsiStorage::Owned(value)) => value,
        }
    }
}

#[invariant(
    ::Borrowed => owner.dictionary().entries().get(entry.0).is_some_and(|entry| {
        entry.selmaho.is_some()
    }),
    "borrowed selma'o positions address a present selma'o in the retained dictionary"
)]
#[invariant(
    ::Owned(_) => true,
    "the Rust raw selma'o model permits every owned string value"
)]
#[derive(Debug, Clone)]
enum RawSelmahoStorage {
    Borrowed {
        owner: Arc<DictionaryOwner>,
        entry: EntryPosition,
    },
    Owned(String),
}

impl RawSelmahoStorage {
    #[requires(true)]
    #[ensures(true)]
    fn value(&self) -> &str {
        match self.as_data() {
            data!(RawSelmahoStorage::Borrowed { owner, entry }) => {
                owner.dictionary().entries()[entry.0]
                    .selmaho
                    .expect("selma'o wrapper is created only for a present value")
                    .0
            }
            data!(RawSelmahoStorage::Owned(value)) => value,
        }
    }
}

#[invariant(
    ::Borrowed => owner.dictionary().entries().get(entry.0).is_some(),
    "borrowed user positions address an existing entry in the retained dictionary"
)]
#[invariant(
    ::Owned => true,
    "the Rust user model permits every owned username and optional-realname combination"
)]
#[derive(Debug, Clone)]
enum UserStorage {
    Borrowed {
        owner: Arc<DictionaryOwner>,
        entry: EntryPosition,
    },
    Owned {
        username: String,
        realname: Option<String>,
    },
}

impl UserStorage {
    #[requires(true)]
    #[ensures(true)]
    fn username(&self) -> &str {
        match self.as_data() {
            data!(UserStorage::Borrowed { owner, entry }) => {
                owner.dictionary().entries()[entry.0].user.username
            }
            data!(UserStorage::Owned { username, .. }) => username,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn realname(&self) -> Option<&str> {
        match self.as_data() {
            data!(UserStorage::Borrowed { owner, entry }) => {
                owner.dictionary().entries()[entry.0].user.realname
            }
            data!(UserStorage::Owned { realname, .. }) => realname.as_deref(),
        }
    }
}

#[contract_trait]
impl PythonStringEnum for WordType {
    fn native_export_name() -> &'static str {
        "_dictionary_WordType"
    }

    fn python_type_name() -> &'static str {
        "WordType"
    }

    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }

    fn python_doc() -> &'static str {
        "Lensisku dictionary word type."
    }

    fn variants() -> &'static [Self] {
        const VARIANTS: &[WordType] = &[
            WordType::Gismu,
            WordType::ExperimentalGismu,
            WordType::Lujvo,
            WordType::ZeiLujvo,
            WordType::ObsoleteZeiLujvo,
            WordType::Cmavo,
            WordType::ExperimentalCmavo,
            WordType::ObsoleteCmavo,
            WordType::CmavoCompound,
            WordType::Fuivla,
            WordType::ObsoleteFuivla,
            WordType::Cmevla,
            WordType::ObsoleteCmevla,
            WordType::BuLetteral,
            WordType::Phrase,
        ];
        VARIANTS
    }

    fn python_member_name(self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(match self {
            Self::Gismu => "GISMU",
            Self::ExperimentalGismu => "EXPERIMENTAL_GISMU",
            Self::Lujvo => "LUJVO",
            Self::ZeiLujvo => "ZEI_LUJVO",
            Self::ObsoleteZeiLujvo => "OBSOLETE_ZEI_LUJVO",
            Self::Cmavo => "CMAVO",
            Self::ExperimentalCmavo => "EXPERIMENTAL_CMAVO",
            Self::ObsoleteCmavo => "OBSOLETE_CMAVO",
            Self::CmavoCompound => "CMAVO_COMPOUND",
            Self::Fuivla => "FUIVLA",
            Self::ObsoleteFuivla => "OBSOLETE_FUIVLA",
            Self::Cmevla => "CMEVLA",
            Self::ObsoleteCmevla => "OBSOLETE_CMEVLA",
            Self::BuLetteral => "BU_LETTERAL",
            Self::Phrase => "PHRASE",
        })
    }

    fn python_value(self) -> &'static str {
        self.as_str()
    }
}

#[contract_trait]
impl PythonStringEnum for RafsiSource {
    fn native_export_name() -> &'static str {
        "_dictionary_RafsiSource"
    }

    fn python_type_name() -> &'static str {
        "RafsiSource"
    }

    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }

    fn python_doc() -> &'static str {
        "Source of a rafsi lookup match."
    }

    fn variants() -> &'static [Self] {
        const VARIANTS: &[RafsiSource] = &[
            RafsiSource::Listed,
            RafsiSource::UniversalShort,
            RafsiSource::UniversalLong,
        ];
        VARIANTS
    }

    fn python_member_name(self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(match self {
            Self::Listed => "LISTED",
            Self::UniversalShort => "UNIVERSAL_SHORT",
            Self::UniversalLong => "UNIVERSAL_LONG",
        })
    }

    fn python_value(self) -> &'static str {
        match self {
            Self::Listed => "listed",
            Self::UniversalShort => "universal-short",
            Self::UniversalLong => "universal-long",
        }
    }
}

#[contract_trait]
impl PythonStringEnum for DictionaryLujvoSegmentKind {
    fn native_export_name() -> &'static str {
        "_dictionary_DictionaryLujvoSegmentKind"
    }

    fn python_type_name() -> &'static str {
        "DictionaryLujvoSegmentKind"
    }

    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }

    fn python_doc() -> &'static str {
        "Kind of a precomputed lujvo decomposition segment."
    }

    fn variants() -> &'static [Self] {
        const VARIANTS: &[DictionaryLujvoSegmentKind] = &[
            DictionaryLujvoSegmentKind::Rafsi,
            DictionaryLujvoSegmentKind::Hyphen,
        ];
        VARIANTS
    }

    fn python_member_name(self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(match self {
            Self::Rafsi => "RAFSI",
            Self::Hyphen => "HYPHEN",
        })
    }

    fn python_value(self) -> &'static str {
        match self {
            Self::Rafsi => "rafsi",
            Self::Hyphen => "hyphen",
        }
    }
}

/// Lensisku definition identifier.
#[invariant(true, "every u64 is a valid Rust DefinitionId, including zero")]
#[pyclass(
    name = "DefinitionId",
    frozen,
    eq,
    hash,
    ord,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PyDefinitionId {
    value: DefinitionId,
}

#[pymethods]
impl PyDefinitionId {
    /// Construct an identifier from its unconstrained Rust `u64` value.
    #[requires(true)]
    #[ensures(ret.value.get() == value)]
    #[new]
    fn new(value: u64) -> Self {
        Self {
            value: DefinitionId(value),
        }
    }

    /// Return the underlying Rust identifier value.
    #[requires(true)]
    #[ensures(ret == self.value.get())]
    #[getter]
    fn value(&self) -> u64 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __int__(&self) -> u64 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __index__(&self) -> u64 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("{PUBLIC_MODULE}.DefinitionId({})", self.value.get())
    }
}

/// Lensisku score value.
#[invariant(true, "every f64 is a valid Rust Score, including non-finite values")]
#[pyclass(
    name = "Score",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct PyScore {
    value: Score,
}

#[pymethods]
impl PyScore {
    /// Construct a score from any Rust `f64`, including non-finite values.
    #[requires(true)]
    #[ensures(ret.value.get().to_bits() == value.to_bits())]
    #[new]
    fn new(value: f64) -> Self {
        Self {
            value: Score(value),
        }
    }

    /// Return the underlying Rust score value.
    #[requires(true)]
    #[ensures(ret.to_bits() == self.value.get().to_bits())]
    #[getter]
    fn value(&self) -> f64 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret.to_bits() == self.value.get().to_bits())]
    fn __float__(&self) -> f64 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __richcmp__(&self, other: &Self, operation: CompareOp) -> bool {
        match operation {
            CompareOp::Lt => self.value < other.value,
            CompareOp::Le => self.value <= other.value,
            CompareOp::Eq => self.value == other.value,
            CompareOp::Ne => self.value != other.value,
            CompareOp::Gt => self.value > other.value,
            CompareOp::Ge => self.value >= other.value,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("{PUBLIC_MODULE}.Score({:?})", self.value.get())
    }
}

/// Index into a dictionary entry sequence.
#[invariant(
    true,
    "every usize is a valid Rust EntryIndex, even when contextual lookup misses"
)]
#[pyclass(
    name = "EntryIndex",
    frozen,
    eq,
    hash,
    ord,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PyEntryIndex {
    value: EntryIndex,
}

impl PyEntryIndex {
    #[requires(true)]
    #[ensures(ret.value == value)]
    fn from_rust(value: EntryIndex) -> Self {
        Self { value }
    }

    #[requires(true)]
    #[ensures(ret == self.value)]
    fn into_rust(self) -> EntryIndex {
        self.value
    }
}

#[pymethods]
impl PyEntryIndex {
    /// Construct a typed index without imposing a dictionary-specific bound.
    #[requires(true)]
    #[ensures(ret.value.get() == value)]
    #[new]
    fn new(value: usize) -> Self {
        Self {
            value: EntryIndex(value),
        }
    }

    /// Return the underlying Rust index value.
    #[requires(true)]
    #[ensures(ret == self.value.get())]
    #[getter]
    fn value(&self) -> usize {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __int__(&self) -> usize {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __index__(&self) -> usize {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("{PUBLIC_MODULE}.EntryIndex({})", self.value.get())
    }
}

/// Gloss or place keyword value.
#[invariant(
    true,
    "the validated KeywordStorage field makes every keyword wrapper state valid"
)]
#[pyclass(
    name = "Keyword",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyKeyword {
    storage: KeywordStorage,
}

impl PyKeyword {
    #[requires(owner.dictionary().entries().get(position.entry.0).is_some_and(|entry| {
        position.item < match position.kind {
            KeywordKind::Gloss => entry.gloss_keywords.len(),
            KeywordKind::Place => entry.place_keywords.len(),
        }
    }))]
    #[ensures(true)]
    fn borrowed(owner: Arc<DictionaryOwner>, position: KeywordPosition) -> Self {
        Self {
            storage: new!(KeywordStorage::Borrowed { owner, position }),
        }
    }
}

#[pymethods]
impl PyKeyword {
    /// Construct a standalone keyword value.
    #[requires(true)]
    #[ensures(ret.storage.value().word == word)]
    #[new]
    #[pyo3(signature = (word, meaning = None))]
    fn new(word: &str, meaning: Option<&str>) -> Self {
        Self {
            storage: new!(KeywordStorage::Owned {
                word: word.to_owned(),
                meaning: meaning.map(str::to_owned),
            }),
        }
    }

    /// Return the keyword text.
    #[requires(true)]
    #[ensures(ret == self.storage.value().word)]
    #[getter]
    fn word(&self) -> &str {
        self.storage.value().word
    }

    /// Return the optional disambiguating meaning.
    #[requires(true)]
    #[ensures(ret == self.storage.value().meaning)]
    #[getter]
    fn meaning(&self) -> Option<&str> {
        self.storage.value().meaning
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let meaning = match self.meaning() {
            Some(value) => string_repr(py, value)?,
            None => "None".to_owned(),
        };
        Ok(format!(
            "{PUBLIC_MODULE}.Keyword(word={}, meaning={meaning})",
            string_repr(py, self.word())?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        self.word() == other.word() && self.meaning() == other.meaning()
    }
}

/// Rafsi value.
#[invariant(true)]
#[pyclass(
    name = "Rafsi",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyRafsi {
    storage: RafsiStorage,
}

impl PyRafsi {
    #[requires(owner.dictionary().entries().get(position.entry.0).is_some_and(|entry| {
        position.item < entry.rafsi.len()
    }))]
    #[ensures(true)]
    fn borrowed(owner: Arc<DictionaryOwner>, position: RafsiPosition) -> Self {
        Self {
            storage: new!(RafsiStorage::Borrowed { owner, position }),
        }
    }

    #[requires(true)]
    #[ensures(ret.storage.value() == old(value.clone()).as_str())]
    fn owned(value: String) -> Self {
        Self {
            storage: new!(RafsiStorage::Owned(value)),
        }
    }
}

#[pymethods]
impl PyRafsi {
    /// Construct a standalone rafsi value without contextual validation.
    #[requires(true)]
    #[ensures(ret.storage.value() == value)]
    #[new]
    fn new(value: &str) -> Self {
        Self::owned(value.to_owned())
    }

    /// Return the underlying rafsi text.
    #[requires(true)]
    #[ensures(ret == self.storage.value())]
    #[getter]
    fn value(&self) -> &str {
        self.storage.value()
    }

    #[requires(true)]
    #[ensures(ret == self.storage.value())]
    fn __str__(&self) -> &str {
        self.storage.value()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.Rafsi({})",
            string_repr(py, self.value())?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __richcmp__(&self, other: &Self, operation: CompareOp) -> bool {
        operation.matches(self.value().cmp(other.value()))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        PyString::new(py, self.value()).hash()
    }
}

/// Raw Lensisku selma'o value.
#[invariant(true)]
#[pyclass(
    name = "RawSelmaho",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyRawSelmaho {
    storage: RawSelmahoStorage,
}

impl PyRawSelmaho {
    #[requires(owner.dictionary().entries().get(entry.0).is_some_and(|entry| {
        entry.selmaho.is_some()
    }))]
    #[ensures(true)]
    fn borrowed(owner: Arc<DictionaryOwner>, entry: EntryPosition) -> Self {
        Self {
            storage: new!(RawSelmahoStorage::Borrowed { owner, entry }),
        }
    }
}

#[pymethods]
impl PyRawSelmaho {
    /// Construct a standalone raw selma'o value.
    #[requires(true)]
    #[ensures(ret.storage.value() == value)]
    #[new]
    fn new(value: &str) -> Self {
        Self {
            storage: new!(RawSelmahoStorage::Owned(value.to_owned())),
        }
    }

    /// Return the underlying raw selma'o text.
    #[requires(true)]
    #[ensures(ret == self.storage.value())]
    #[getter]
    fn value(&self) -> &str {
        self.storage.value()
    }

    #[requires(true)]
    #[ensures(ret == self.storage.value())]
    fn __str__(&self) -> &str {
        self.storage.value()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.RawSelmaho({})",
            string_repr(py, self.value())?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __richcmp__(&self, other: &Self, operation: CompareOp) -> bool {
        operation.matches(self.value().cmp(other.value()))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __hash__(&self, py: Python<'_>) -> PyResult<isize> {
        PyString::new(py, self.value()).hash()
    }
}

/// Lensisku contributor metadata.
#[invariant(true)]
#[pyclass(
    name = "DictionaryUser",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionaryUser {
    storage: UserStorage,
}

impl PyDictionaryUser {
    #[requires(owner.dictionary().entries().get(entry.0).is_some())]
    #[ensures(true)]
    fn borrowed(owner: Arc<DictionaryOwner>, entry: EntryPosition) -> Self {
        Self {
            storage: new!(UserStorage::Borrowed { owner, entry }),
        }
    }
}

#[pymethods]
impl PyDictionaryUser {
    /// Construct standalone contributor metadata.
    #[requires(true)]
    #[ensures(ret.storage.username() == username)]
    #[new]
    #[pyo3(signature = (username, realname = None))]
    fn new(username: &str, realname: Option<&str>) -> Self {
        Self {
            storage: new!(UserStorage::Owned {
                username: username.to_owned(),
                realname: realname.map(str::to_owned),
            }),
        }
    }

    /// Return the contributor username.
    #[requires(true)]
    #[ensures(ret == self.storage.username())]
    #[getter]
    fn username(&self) -> &str {
        self.storage.username()
    }

    /// Return the contributor's optional real name.
    #[requires(true)]
    #[ensures(ret == self.storage.realname())]
    #[getter]
    fn realname(&self) -> Option<&str> {
        self.storage.realname()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let realname = match self.realname() {
            Some(value) => string_repr(py, value)?,
            None => "None".to_owned(),
        };
        Ok(format!(
            "{PUBLIC_MODULE}.DictionaryUser(username={}, realname={realname})",
            string_repr(py, self.username())?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        self.username() == other.username() && self.realname() == other.realname()
    }
}

/// Immutable embedded dictionary.
#[invariant(true)]
#[pyclass(
    name = "Dictionary",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub(crate) struct PyDictionary {
    owner: Arc<DictionaryOwner>,
}

impl PyDictionary {
    #[requires(true)]
    #[ensures(ret.owner.dictionary().entries().len() == english().entries().len())]
    fn english() -> Self {
        Self {
            owner: Arc::new(DictionaryOwner::english()),
        }
    }

    /// Borrow the validated embedded dictionary without exposing binding ownership.
    #[requires(true)]
    #[ensures(std::ptr::eq(ret, self.owner.dictionary()))]
    pub(crate) fn dictionary(&self) -> &'static Dictionary<'static> {
        self.owner.dictionary()
    }
}

#[pymethods]
impl PyDictionary {
    /// Return the number of entries in this dictionary.
    #[requires(true)]
    #[ensures(ret == self.owner.dictionary().entries().len())]
    fn __len__(&self) -> usize {
        self.owner.dictionary().entries().len()
    }

    /// Return the lazy immutable source-order entry sequence.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn entries(&self, py: Python<'_>) -> PyResult<Py<PyDictionaryEntries>> {
        Py::new(
            py,
            PyDictionaryEntries {
                owner: Arc::clone(&self.owner),
            },
        )
    }

    /// Index the source-order entries with normal Python integer/slice rules.
    #[requires(true)]
    #[ensures(true)]
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        entry_sequence_item(py, &self.owner, key)
    }

    /// Iterate lazily over source-order entries.
    #[requires(true)]
    #[ensures(true)]
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyDictionaryEntryIterator>> {
        Py::new(
            py,
            PyDictionaryEntryIterator {
                owner: Arc::clone(&self.owner),
                next: 0,
            },
        )
    }

    /// Return an entry by a typed index, or `None` when it is out of range.
    #[requires(true)]
    #[ensures(true)]
    fn entry_for_index(
        &self,
        py: Python<'_>,
        index: &PyEntryIndex,
    ) -> PyResult<Option<Py<PyDictionaryEntry>>> {
        self.owner
            .dictionary()
            .entry_for_index(PyEntryIndex::into_rust(*index))
            .map(|entry| entry_object(py, Arc::clone(&self.owner), entry))
            .transpose()
    }

    /// Return an entry's typed index when it belongs to this dictionary.
    #[requires(true)]
    #[ensures(true)]
    fn entry_index_for_entry(&self, entry: &PyDictionaryEntry) -> Option<PyEntryIndex> {
        if !Arc::ptr_eq(&self.owner, &entry.reference.owner) {
            return None;
        }
        self.owner
            .dictionary()
            .entry_index_for_entry(entry.entry())
            .map(PyEntryIndex::from_rust)
    }

    /// Return the first entry matching a normalized word query.
    #[requires(true)]
    #[ensures(true)]
    fn lookup_word(&self, py: Python<'_>, query: &str) -> PyResult<Option<Py<PyDictionaryEntry>>> {
        self.owner
            .dictionary()
            .lookup_word(query)
            .map(|entry| entry_object(py, Arc::clone(&self.owner), entry))
            .transpose()
    }

    /// Return every entry matching a normalized word query in collision order.
    #[requires(true)]
    #[ensures(true)]
    fn lookup_words(&self, py: Python<'_>, query: &str) -> PyResult<Py<PyTuple>> {
        entries_to_tuple(
            py,
            Arc::clone(&self.owner),
            self.owner.dictionary().lookup_words(query),
        )
    }

    /// Return entries whose normalized words begin with `prefix`.
    #[requires(true)]
    #[ensures(true)]
    fn entries_by_word_prefix(&self, py: Python<'_>, prefix: &str) -> PyResult<Py<PyTuple>> {
        entries_to_tuple(
            py,
            Arc::clone(&self.owner),
            self.owner.dictionary().entries_by_word_prefix(prefix),
        )
    }

    /// Return every rafsi match with its exact provenance.
    #[requires(true)]
    #[ensures(true)]
    fn lookup_rafsi(&self, py: Python<'_>, query: &str) -> PyResult<Py<PyTuple>> {
        let matches = self
            .owner
            .dictionary()
            .lookup_rafsi(query)
            .map(|matched| {
                Py::new(
                    py,
                    PyRafsiMatch {
                        entry: EntryReference::new(
                            Arc::clone(&self.owner),
                            self.owner.entry_position(matched.entry),
                        ),
                        source: matched.source,
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, matches)?.unbind())
    }

    /// Return the word and type of every entry claiming a rafsi.
    #[requires(true)]
    #[ensures(true)]
    fn rafsi_claimants(&self, py: Python<'_>, rafsi: &str) -> PyResult<Py<PyTuple>> {
        let module = native_module(py)?;
        let claimants = self
            .owner
            .dictionary()
            .rafsi_claimants(rafsi)
            .map(|(word, word_type)| {
                let word = PyString::new(py, word).into_any().unbind();
                let word_type = string_enum_member(&module, word_type)?.unbind();
                Ok(sequence_to_tuple(py, [word, word_type])?.unbind())
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, claimants)?.unbind())
    }

    /// Return every short rafsi a gismu could claim, with its availability.
    #[requires(true)]
    #[ensures(true)]
    fn short_rafsi_candidates(&self, py: Python<'_>, gismu: &str) -> PyResult<Py<PyTuple>> {
        let candidates = self
            .owner
            .dictionary()
            .short_rafsi_candidates(gismu)
            .into_iter()
            .map(|candidate| Py::new(py, PyRafsiCandidate::from_rust(candidate)))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, candidates)?.unbind())
    }

    /// Return every entry with exactly the requested raw selma'o.
    #[requires(true)]
    #[ensures(true)]
    fn entries_by_selmaho(&self, py: Python<'_>, selmaho: &str) -> PyResult<Py<PyTuple>> {
        entries_to_tuple(
            py,
            Arc::clone(&self.owner),
            self.owner.dictionary().entries_by_selmaho(selmaho),
        )
    }

    /// Return the first non-empty gloss for each requested lookup word.
    ///
    /// A bare string is rejected because treating it as a batch of characters
    /// is almost certainly a caller error.
    #[requires(true)]
    #[ensures(true)]
    fn first_gloss_keywords_for_words(
        &self,
        py: Python<'_>,
        words: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyTuple>> {
        if words.is_instance_of::<PyString>() {
            return Err(PyTypeError::new_err(
                "words must be a sequence of strings, not a bare string",
            ));
        }
        let owned_words = words.extract::<Vec<String>>()?;
        let borrowed_words = owned_words.iter().map(String::as_str).collect::<Vec<_>>();
        let results = self
            .owner
            .dictionary()
            .first_gloss_keywords_for_words(&borrowed_words);
        Ok(sequence_to_tuple(py, results)?.unbind())
    }

    /// Return precomputed sound records in dictionary-entry order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn sound_index(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .owner
            .dictionary()
            .sound_index()
            .iter()
            .enumerate()
            .map(|(position, _)| {
                Py::new(
                    py,
                    PyDictionarySoundEntry {
                        reference: SoundReference::new(
                            Arc::clone(&self.owner),
                            SoundPosition(position),
                        ),
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return precomputed lujvo decomposition records in entry order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn lujvo_index(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .owner
            .dictionary()
            .lujvo_index()
            .iter()
            .enumerate()
            .map(|(position, _)| {
                Py::new(
                    py,
                    PyDictionaryLujvoEntry {
                        reference: LujvoReference::new(
                            Arc::clone(&self.owner),
                            LujvoPosition(position),
                        ),
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return generated pattern keys in dictionary-entry order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn pattern_index(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .owner
            .dictionary()
            .pattern_index()
            .iter()
            .enumerate()
            .map(|(position, _)| {
                Py::new(
                    py,
                    PyDictionaryPatternEntry {
                        reference: PatternReference::new(
                            Arc::clone(&self.owner),
                            PatternPosition(position),
                        ),
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return generated lujvo decomposition data for a typed entry index.
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_decomposition_for_entry_index(
        &self,
        py: Python<'_>,
        index: &PyEntryIndex,
    ) -> PyResult<Option<Py<PyDictionaryLujvoEntry>>> {
        let rust_index = PyEntryIndex::into_rust(*index);
        let Some(_) = self
            .owner
            .dictionary()
            .lujvo_decomposition_for_entry_index(rust_index)
        else {
            return Ok(None);
        };
        let position = self
            .owner
            .dictionary()
            .lujvo_index()
            .binary_search_by(|entry| entry.entry_index.cmp(&rust_index))
            .expect("successful public lookup must identify an index record");
        Py::new(
            py,
            PyDictionaryLujvoEntry {
                reference: LujvoReference::new(Arc::clone(&self.owner), LujvoPosition(position)),
            },
        )
        .map(Some)
    }

    /// Validate the dictionary and raise a typed structural error on failure.
    #[requires(true)]
    #[ensures(true)]
    fn validate(&self, py: Python<'_>) -> PyResult<()> {
        match self.owner.dictionary().validate() {
            Ok(()) => Ok(()),
            Err(error) => Err(dictionary_validation_py_err(py, error)?),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("{PUBLIC_MODULE}.english")
    }
}

/// Lazy immutable source-order dictionary entry sequence.
#[invariant(true)]
#[pyclass(
    name = "DictionaryEntries",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionaryEntries {
    owner: Arc<DictionaryOwner>,
}

#[pymethods]
impl PyDictionaryEntries {
    #[requires(true)]
    #[ensures(ret == self.owner.dictionary().entries().len())]
    fn __len__(&self) -> usize {
        self.owner.dictionary().entries().len()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        entry_sequence_item(py, &self.owner, key)
    }

    #[requires(true)]
    #[ensures(true)]
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyDictionaryEntryIterator>> {
        Py::new(
            py,
            PyDictionaryEntryIterator {
                owner: Arc::clone(&self.owner),
                next: 0,
            },
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "{PUBLIC_MODULE}.DictionaryEntries(length={})",
            self.owner.dictionary().entries().len()
        )
    }
}

#[invariant(
    true,
    "every next value is a safe exhausted state because iteration uses slice get"
)]
#[pyclass(
    name = "_DictionaryEntryIterator",
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug)]
struct PyDictionaryEntryIterator {
    owner: Arc<DictionaryOwner>,
    next: usize,
}

#[pymethods]
impl PyDictionaryEntryIterator {
    #[requires(true)]
    #[ensures(true)]
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[requires(true)]
    #[ensures(true)]
    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyDictionaryEntry>>> {
        let Some(entry) = self.owner.dictionary().entries().get(self.next) else {
            return Ok(None);
        };
        self.next += 1;
        entry_object(py, Arc::clone(&self.owner), entry).map(Some)
    }
}

/// One source dictionary entry.
#[invariant(
    true,
    "the validated EntryReference field makes every wrapper state in range"
)]
#[pyclass(
    name = "DictionaryEntry",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionaryEntry {
    reference: EntryReference,
}

impl PyDictionaryEntry {
    #[requires(self.reference.position.0 < self.reference.owner.dictionary().entries().len())]
    #[ensures(std::ptr::eq(ret, self.reference.entry()))]
    fn entry(&self) -> &'static DictionaryEntry<'static> {
        self.reference.entry()
    }
}

#[pymethods]
impl PyDictionaryEntry {
    /// Return the source word exactly as stored in the snapshot.
    #[requires(true)]
    #[ensures(ret == self.entry().word)]
    #[getter]
    fn word(&self) -> &str {
        self.entry().word
    }

    /// Return the exact Lensisku word type.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word_type(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let module = native_module(py)?;
        string_enum_member(&module, self.entry().word_type).map(Bound::unbind)
    }

    /// Return the dictionary definition text.
    #[requires(true)]
    #[ensures(ret == self.entry().definition)]
    #[getter]
    fn definition(&self) -> &str {
        self.entry().definition
    }

    /// Return the typed Lensisku definition identifier.
    #[requires(true)]
    #[ensures(ret.value == self.entry().definition_id)]
    #[getter]
    fn definition_id(&self) -> PyDefinitionId {
        PyDefinitionId {
            value: self.entry().definition_id,
        }
    }

    /// Return the entry notes text.
    #[requires(true)]
    #[ensures(ret == self.entry().notes)]
    #[getter]
    fn notes(&self) -> &str {
        self.entry().notes
    }

    /// Return the Lensisku score without adding finiteness constraints.
    #[requires(true)]
    #[ensures(ret.value.get().to_bits() == self.entry().score.get().to_bits())]
    #[getter]
    fn score(&self) -> PyScore {
        PyScore {
            value: self.entry().score,
        }
    }

    /// Return gloss keywords in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn gloss_keywords(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        keyword_tuple(
            py,
            Arc::clone(&self.reference.owner),
            self.reference.position,
            KeywordKind::Gloss,
        )
    }

    /// Return place keywords in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn place_keywords(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        keyword_tuple(
            py,
            Arc::clone(&self.reference.owner),
            self.reference.position,
            KeywordKind::Place,
        )
    }

    /// Return explicitly listed rafsi in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn rafsi(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .entry()
            .rafsi
            .iter()
            .enumerate()
            .map(|(item, _)| {
                Py::new(
                    py,
                    PyRafsi::borrowed(
                        Arc::clone(&self.reference.owner),
                        RafsiPosition {
                            entry: self.reference.position,
                            item,
                        },
                    ),
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return the optional raw selma'o value.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self, py: Python<'_>) -> PyResult<Option<Py<PyRawSelmaho>>> {
        self.entry()
            .selmaho
            .map(|_| {
                Py::new(
                    py,
                    PyRawSelmaho::borrowed(
                        Arc::clone(&self.reference.owner),
                        self.reference.position,
                    ),
                )
            })
            .transpose()
    }

    /// Return optional etymology text.
    #[requires(true)]
    #[ensures(ret == self.entry().etymology)]
    #[getter]
    fn etymology(&self) -> Option<&str> {
        self.entry().etymology
    }

    /// Return the optional jargon category.
    #[requires(true)]
    #[ensures(ret == self.entry().jargon)]
    #[getter]
    fn jargon(&self) -> Option<&str> {
        self.entry().jargon
    }

    /// Return contributor metadata for this entry.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn user(&self, py: Python<'_>) -> PyResult<Py<PyDictionaryUser>> {
        Py::new(
            py,
            PyDictionaryUser::borrowed(Arc::clone(&self.reference.owner), self.reference.position),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.DictionaryEntry(word={})",
            string_repr(py, self.entry().word)?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
            && self.reference.position == other.reference.position
    }
}

#[requires(owner.dictionary().entry_index_for_entry(entry).is_some())]
#[ensures(true)]
fn entry_object(
    py: Python<'_>,
    owner: Arc<DictionaryOwner>,
    entry: &DictionaryEntry<'static>,
) -> PyResult<Py<PyDictionaryEntry>> {
    let position = owner.entry_position(entry);
    Py::new(
        py,
        PyDictionaryEntry {
            reference: EntryReference::new(owner, position),
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn entries_to_tuple<'entry>(
    py: Python<'_>,
    owner: Arc<DictionaryOwner>,
    entries: impl IntoIterator<Item = &'entry DictionaryEntry<'static>>,
) -> PyResult<Py<PyTuple>> {
    let values = entries
        .into_iter()
        .map(|entry| entry_object(py, Arc::clone(&owner), entry))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(sequence_to_tuple(py, values)?.unbind())
}

#[requires(owner.dictionary().entries().get(entry.0).is_some())]
#[ensures(true)]
fn keyword_tuple(
    py: Python<'_>,
    owner: Arc<DictionaryOwner>,
    entry: EntryPosition,
    kind: KeywordKind,
) -> PyResult<Py<PyTuple>> {
    let count = match kind {
        KeywordKind::Gloss => owner.dictionary().entries()[entry.0].gloss_keywords.len(),
        KeywordKind::Place => owner.dictionary().entries()[entry.0].place_keywords.len(),
    };
    let values = (0..count)
        .map(|item| {
            Py::new(
                py,
                PyKeyword::borrowed(Arc::clone(&owner), KeywordPosition { entry, kind, item }),
            )
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(sequence_to_tuple(py, values)?.unbind())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|position| *position < length) || ret.is_err())]
fn normalize_sequence_index(index: isize, length: usize) -> PyResult<usize> {
    let length_isize = isize::try_from(length).expect("Python sequence length must fit isize");
    let normalized = if index < 0 {
        length_isize.checked_add(index)
    } else {
        Some(index)
    };
    match normalized.and_then(|value| usize::try_from(value).ok()) {
        Some(position) if position < length => Ok(position),
        _ => Err(PyIndexError::new_err("dictionary entry index out of range")),
    }
}

#[requires(true)]
#[ensures(true)]
fn entry_sequence_item(
    py: Python<'_>,
    owner: &Arc<DictionaryOwner>,
    key: &Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let length = owner.dictionary().entries().len();
    if let Ok(slice) = key.cast::<PySlice>() {
        let indices = slice
            .indices(isize::try_from(length).expect("Python sequence length must fit isize"))?;
        let mut values = Vec::with_capacity(indices.slicelength);
        let mut current = indices.start;
        for item in 0..indices.slicelength {
            let position = usize::try_from(current).expect("normalized slice position is valid");
            values.push(Py::new(
                py,
                PyDictionaryEntry {
                    reference: EntryReference::new(Arc::clone(owner), EntryPosition(position)),
                },
            )?);
            if item + 1 < indices.slicelength {
                current = current
                    .checked_add(indices.step)
                    .expect("another normalized slice position must fit isize");
            }
        }
        return Ok(sequence_to_tuple(py, values)?.unbind().into_any());
    }
    let index_value = py.import("operator")?.getattr("index")?.call1((key,))?;
    let index = index_value
        .extract::<isize>()
        .map_err(|_| PyIndexError::new_err("dictionary entry index cannot fit a platform index"))?;
    let position = normalize_sequence_index(index, length)?;
    Py::new(
        py,
        PyDictionaryEntry {
            reference: EntryReference::new(Arc::clone(owner), EntryPosition(position)),
        },
    )
    .map(Py::into_any)
}

/// Result of a rafsi lookup with provenance.
#[invariant(
    true,
    "the validated EntryReference field makes every rafsi match entry in range"
)]
#[pyclass(
    name = "RafsiMatch",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyRafsiMatch {
    entry: EntryReference,
    source: RafsiSource,
}

#[pymethods]
impl PyRafsiMatch {
    /// Return the matched dictionary entry.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn entry(&self, py: Python<'_>) -> PyResult<Py<PyDictionaryEntry>> {
        Py::new(
            py,
            PyDictionaryEntry {
                reference: self.entry.clone(),
            },
        )
    }

    /// Return the exact listed or universal rafsi provenance.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let module = native_module(py)?;
        string_enum_member(&module, self.source).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        let source = self.source.python_member_name();
        format!(
            "{PUBLIC_MODULE}.RafsiMatch(entry={PUBLIC_MODULE}.english[{}], source={PUBLIC_MODULE}.RafsiSource.{source})",
            self.entry.position.0
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.entry.owner, &other.entry.owner)
            && self.entry.position == other.entry.position
            && self.source == other.source
    }
}

#[contract_trait]
impl PythonStringEnum for RafsiClaimKind {
    fn native_export_name() -> &'static str {
        "_dictionary_RafsiClaimKind"
    }

    fn python_type_name() -> &'static str {
        "RafsiClaimKind"
    }

    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }

    fn python_doc() -> &'static str {
        "Standing of the gismu that already claim a short rafsi."
    }

    fn variants() -> &'static [Self] {
        const VARIANTS: &[RafsiClaimKind] =
            &[RafsiClaimKind::Official, RafsiClaimKind::Experimental];
        VARIANTS
    }

    fn python_member_name(self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(match self {
            Self::Official => "OFFICIAL",
            Self::Experimental => "EXPERIMENTAL",
        })
    }

    fn python_value(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Experimental => "experimental",
        }
    }
}

/// Availability alternative for a short rafsi that no gismu has claimed.
#[invariant(true, "the free alternative carries no state")]
#[pyclass(
    name = "FreeRafsiAvailability",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PyFreeRafsiAvailability;

#[pymethods]
impl PyFreeRafsiAvailability {
    /// Construct the free availability alternative.
    #[requires(true)]
    #[ensures(true)]
    #[new]
    fn new() -> Self {
        PyFreeRafsiAvailability
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn __repr__(&self) -> String {
        format!("{PUBLIC_MODULE}.FreeRafsiAvailability()")
    }
}

/// Availability alternative for a short rafsi that gismu already claim.
#[invariant(
    true,
    "PyO3 requires the declared class shape; the checked constructor and validated Rust storage keep the claimant list non-empty"
)]
#[pyclass(
    name = "TakenRafsiAvailability",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyTakenRafsiAvailability {
    kind: RafsiClaimKind,
    words: Vec<String>,
}

#[pymethods]
impl PyTakenRafsiAvailability {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("kind", "words");
    /// Construct a taken availability alternative with its claimants.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, kind: &Bound<'_, PyAny>, words: Vec<String>) -> PyResult<Self> {
        let module = native_module(py)?;
        let kind = extract_string_enum::<RafsiClaimKind>(&module, kind)?;
        if words.is_empty() || words.iter().any(|word| word.is_empty()) {
            return Err(PyTypeError::new_err(
                "a taken rafsi must name at least one non-empty claimant word",
            ));
        }
        Ok(PyTakenRafsiAvailability { kind, words })
    }

    /// Return whether the claim is official or experimental.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let module = native_module(py)?;
        string_enum_member(&module, self.kind).map(Bound::unbind)
    }

    /// Return the gismu that hold this rafsi.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn words(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.words.iter().cloned()).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let words = self
            .words
            .iter()
            .map(|word| string_repr(py, word))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(format!(
            "{PUBLIC_MODULE}.TakenRafsiAvailability(kind={PUBLIC_MODULE}.RafsiClaimKind.{}, words=({},))",
            self.kind.python_member_name(),
            words.join(", "),
        ))
    }
}

/// One short rafsi a gismu could claim, with its dictionary standing.
#[invariant(
    true,
    "PyO3 requires the declared class shape; values are projected from validated Rust RafsiCandidate storage"
)]
#[pyclass(
    name = "RafsiCandidate",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyRafsiCandidate {
    form: String,
    shape: ShortRafsiShape,
    availability: RafsiAvailability,
}

impl PyRafsiCandidate {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(candidate: RafsiCandidate) -> Self {
        let candidate = candidate.into_data();
        PyRafsiCandidate {
            form: candidate.form,
            shape: candidate.shape,
            availability: candidate.availability,
        }
    }
}

#[pymethods]
impl PyRafsiCandidate {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("form", "shape", "availability");

    /// Return the rafsi spelling.
    #[requires(true)]
    #[ensures(ret == self.form.as_str())]
    #[getter]
    fn form(&self) -> &str {
        &self.form
    }

    /// Return the short rafsi shape this spelling realizes.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn shape(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let module = native_module(py)?;
        string_enum_member(&module, self.shape).map(Bound::unbind)
    }

    /// Return whether the rafsi is free, or who already claims it.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn availability(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.availability.as_data() {
            data!(RafsiAvailability::Free) => Ok(Py::new(py, PyFreeRafsiAvailability)?.into_any()),
            data!(RafsiAvailability::Taken { kind, words }) => Ok(Py::new(
                py,
                PyTakenRafsiAvailability {
                    kind: *kind,
                    words: words.iter().cloned().collect::<Vec<_>>(),
                },
            )?
            .into_any()),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.RafsiCandidate(form={}, shape={PUBLIC_MODULE}.ShortRafsiShape.{}, availability={})",
            string_repr(py, &self.form)?,
            self.shape.python_member_name(),
            self.availability(py)?.bind(py).repr()?,
        ))
    }
}

/// Precomputed sound data for one pronounceable dictionary entry.
#[invariant(
    true,
    "the validated SoundReference field makes every sound wrapper state in range"
)]
#[pyclass(
    name = "DictionarySoundEntry",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionarySoundEntry {
    reference: SoundReference,
}

impl PyDictionarySoundEntry {
    #[requires(self.reference.position.0 < self.reference.owner.dictionary().sound_index().len())]
    #[ensures(std::ptr::eq(ret, self.reference.sound()))]
    fn sound(&self) -> &'static jbotci_dictionary::DictionarySoundEntry<'static> {
        self.reference.sound()
    }
}

#[pymethods]
impl PyDictionarySoundEntry {
    /// Return the typed source entry index.
    #[requires(true)]
    #[ensures(ret.value == self.sound().entry_index)]
    #[getter]
    fn entry_index(&self) -> PyEntryIndex {
        PyEntryIndex::from_rust(self.sound().entry_index)
    }

    /// Return the precomputed IPA rendering.
    #[requires(true)]
    #[ensures(ret == self.sound().ipa)]
    #[getter]
    fn ipa(&self) -> &str {
        self.sound().ipa
    }

    /// Return the retained precomputed IPA token sequence.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn token_sequence(&self, py: Python<'_>) -> PyResult<Py<PyIpaTokenSequenceView>> {
        Py::new(
            py,
            PyIpaTokenSequenceView {
                reference: self.reference.clone(),
            },
        )
    }

    /// Return the retained Lojban pronunciation-target sequence used by scoring.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|view| {
        let view = view.bind(py).borrow();
        Arc::ptr_eq(&view.reference.owner, &self.reference.owner)
            && view.reference.position == self.reference.position
    }) || ret.is_err())]
    #[getter]
    fn pronunciation_targets(
        &self,
        py: Python<'_>,
    ) -> PyResult<Py<PyPronunciationTargetSequenceView>> {
        Py::new(
            py,
            PyPronunciationTargetSequenceView {
                reference: self.reference.clone(),
            },
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.DictionarySoundEntry(entry_index={}, ipa={})",
            self.sound().entry_index.get(),
            string_repr(py, self.sound().ipa)?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
            && self.reference.position == other.reference.position
    }
}

/// Borrowed IPA token sequence retained through its dictionary owner.
#[invariant(
    true,
    "the validated SoundReference field makes every IPA sequence wrapper state in range"
)]
#[pyclass(
    name = "IpaTokenSequenceView",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyIpaTokenSequenceView {
    reference: SoundReference,
}

impl PyIpaTokenSequenceView {
    #[requires(self.reference.position.0 < self.reference.owner.dictionary().sound_index().len())]
    #[ensures(ret == self.reference.sound().token_sequence)]
    fn sequence(&self) -> jbotci_phonetic::IpaTokenSequenceView<'static> {
        self.reference.sound().token_sequence
    }
}

#[pymethods]
impl PyIpaTokenSequenceView {
    /// Return phonetic segment identifiers in sequence order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .sequence()
            .segments
            .iter()
            .copied()
            .map(|value| PyIpaSegmentId { value });
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return the precomputed ALINE self-similarity.
    #[requires(true)]
    #[ensures(ret.to_bits() == self.sequence().self_similarity.to_bits())]
    #[getter]
    fn self_similarity(&self) -> f64 {
        self.sequence().self_similarity
    }

    /// Return the number of phonetic segments.
    #[requires(true)]
    #[ensures(ret == self.sequence().segment_count())]
    fn __len__(&self) -> usize {
        self.sequence().segment_count()
    }

    /// Return the number of phonetic segments.
    #[requires(true)]
    #[ensures(ret == self.sequence().segment_count())]
    fn segment_count(&self) -> usize {
        self.sequence().segment_count()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "{PUBLIC_MODULE}.IpaTokenSequenceView(segment_count={}, self_similarity={:?})",
            self.sequence().segment_count(),
            self.sequence().self_similarity
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
            && self.reference.position == other.reference.position
    }
}

/// Identifier for one segment in the phonetic inventory.
#[invariant(true, "the wrapped Rust IpaSegmentId enforces its static-table bound")]
#[pyclass(
    name = "IpaSegmentId",
    frozen,
    eq,
    hash,
    ord,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PyIpaSegmentId {
    value: IpaSegmentId,
}

#[pymethods]
impl PyIpaSegmentId {
    /// Return the bounded static segment-table index.
    #[requires(true)]
    #[ensures(ret == self.value.get())]
    #[getter]
    fn value(&self) -> u16 {
        self.value.get()
    }

    /// Return the canonical IPA symbol for this segment.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn symbol(&self) -> &'static str {
        ipa_segment_symbol(self.value).expect("every valid IPA segment id has a symbol")
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __int__(&self) -> u16 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __index__(&self) -> u16 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.IpaSegmentId(value={}, symbol={})",
            self.value.get(),
            string_repr(py, self.symbol())?
        ))
    }
}

/// Borrowed Lojban pronunciation targets retained through their dictionary owner.
#[invariant(
    true,
    "the validated SoundReference field makes every pronunciation-target sequence wrapper state in range"
)]
#[pyclass(
    name = "PronunciationTargetSequenceView",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyPronunciationTargetSequenceView {
    reference: SoundReference,
}

impl PyPronunciationTargetSequenceView {
    #[requires(self.reference.position.0 < self.reference.owner.dictionary().sound_index().len())]
    #[ensures(ret == self.reference.sound().pronunciation_targets)]
    fn sequence(&self) -> jbotci_phonetic::PronunciationTargetSequenceView<'static> {
        self.reference.sound().pronunciation_targets
    }
}

#[pymethods]
impl PyPronunciationTargetSequenceView {
    /// Return pronunciation target identifiers in sequence order.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|targets| {
        targets.bind(py).len() == self.sequence().target_count()
    }) || ret.is_err())]
    #[getter]
    fn targets(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .sequence()
            .targets
            .iter()
            .copied()
            .map(|value| PyPronunciationTargetId { value });
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return the precomputed pronunciation-target ALINE self-similarity.
    #[requires(true)]
    #[ensures(ret.to_bits() == self.sequence().self_similarity.to_bits())]
    #[getter]
    fn self_similarity(&self) -> f64 {
        self.sequence().self_similarity
    }

    /// Return the number of pronunciation targets.
    #[requires(true)]
    #[ensures(ret == self.sequence().target_count())]
    fn __len__(&self) -> usize {
        self.sequence().target_count()
    }

    /// Return the number of pronunciation targets.
    #[requires(true)]
    #[ensures(ret == self.sequence().target_count())]
    fn target_count(&self) -> usize {
        self.sequence().target_count()
    }

    #[requires(true)]
    #[ensures(ret.starts_with(PUBLIC_MODULE))]
    fn __repr__(&self) -> String {
        format!(
            "{PUBLIC_MODULE}.PronunciationTargetSequenceView(target_count={}, self_similarity={:?})",
            self.sequence().target_count(),
            self.sequence().self_similarity
        )
    }

    #[requires(true)]
    #[ensures(ret == (Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
        && self.reference.position == other.reference.position))]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
            && self.reference.position == other.reference.position
    }
}

/// Identifier for one Lojban pronunciation target and its accepted realizations.
#[invariant(
    true,
    "the wrapped Rust PronunciationTargetId enforces its static target and realization-table validity"
)]
#[pyclass(
    name = "PronunciationTargetId",
    frozen,
    eq,
    hash,
    ord,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PyPronunciationTargetId {
    value: PronunciationTargetId,
}

#[pymethods]
impl PyPronunciationTargetId {
    /// Return the bounded static pronunciation-target inventory index.
    #[requires(true)]
    #[ensures(ret == self.value.get())]
    #[getter]
    fn value(&self) -> u16 {
        self.value.get()
    }

    /// Return the number of accepted concrete IPA realizations.
    #[requires(true)]
    #[ensures(ret == self.value.realization_count())]
    #[getter]
    fn realization_count(&self) -> usize {
        self.value.realization_count()
    }

    /// Return one accepted IPA realization, or `None` when out of bounds.
    ///
    /// Negative indices return `None`; unlike the `realizations` tuple, this
    /// Rust-parity method does not apply Python negative-index normalization.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| result.as_ref().is_none_or(|segment| {
        (0..self.value.realization_count())
            .any(|index| self.value.realization(index) == Some(segment.value))
    })) || ret.is_err())]
    fn realization(
        &self,
        py: Python<'_>,
        index: &Bound<'_, PyAny>,
    ) -> PyResult<Option<PyIpaSegmentId>> {
        let index = py.import("operator")?.getattr("index")?.call1((index,))?;
        if index.lt(0)? || index.ge(self.value.realization_count())? {
            return Ok(None);
        }
        let index = index
            .extract::<usize>()
            .expect("an integer bounded by realization_count must fit usize");
        Ok(self
            .value
            .realization(index)
            .map(|value| PyIpaSegmentId { value }))
    }

    /// Return every accepted concrete IPA realization in Rust inventory order.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|realizations| {
        realizations.bind(py).len() == self.value.realization_count()
    }) || ret.is_err())]
    #[getter]
    fn realizations(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = (0..self.value.realization_count()).map(|index| PyIpaSegmentId {
            value: self
                .value
                .realization(index)
                .expect("indices below realization_count must have a realization"),
        });
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __int__(&self) -> u16 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn __index__(&self) -> u16 {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret.starts_with(PUBLIC_MODULE))]
    fn __repr__(&self) -> String {
        format!(
            "{PUBLIC_MODULE}.PronunciationTargetId(value={}, realization_count={})",
            self.value.get(),
            self.value.realization_count()
        )
    }
}

/// Precomputed lujvo decomposition for one dictionary entry.
#[invariant(
    true,
    "the validated LujvoReference field makes every decomposition wrapper state in range"
)]
#[pyclass(
    name = "DictionaryLujvoEntry",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionaryLujvoEntry {
    reference: LujvoReference,
}

impl PyDictionaryLujvoEntry {
    #[requires(self.reference.position.0 < self.reference.owner.dictionary().lujvo_index().len())]
    #[ensures(std::ptr::eq(ret, self.reference.lujvo()))]
    fn lujvo(&self) -> &'static jbotci_dictionary::DictionaryLujvoEntry<'static> {
        self.reference.lujvo()
    }
}

#[pymethods]
impl PyDictionaryLujvoEntry {
    /// Return the typed source entry index.
    #[requires(true)]
    #[ensures(ret.value == self.lujvo().entry_index)]
    #[getter]
    fn entry_index(&self) -> PyEntryIndex {
        PyEntryIndex::from_rust(self.lujvo().entry_index)
    }

    /// Return decomposition segments in surface order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .lujvo()
            .segments
            .iter()
            .enumerate()
            .map(|(item, _)| {
                Py::new(
                    py,
                    PyDictionaryLujvoSegment {
                        reference: LujvoSegmentReference::new(self.reference.clone(), item),
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(sequence_to_tuple(py, values)?.unbind())
    }

    /// Return source words in decomposition order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_words(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(sequence_to_tuple(py, self.lujvo().source_words.iter().copied())?.unbind())
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "{PUBLIC_MODULE}.DictionaryLujvoEntry(entry_index={})",
            self.lujvo().entry_index.get()
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
            && self.reference.position == other.reference.position
    }
}

/// One segment of a precomputed lujvo decomposition.
#[invariant(
    true,
    "the validated LujvoSegmentReference field makes every segment wrapper state in range"
)]
#[pyclass(
    name = "DictionaryLujvoSegment",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionaryLujvoSegment {
    reference: LujvoSegmentReference,
}

impl PyDictionaryLujvoSegment {
    #[requires(self.reference.item < self.reference.lujvo.lujvo().segments.len())]
    #[ensures(std::ptr::eq(ret, self.reference.segment()))]
    fn segment(&self) -> &'static jbotci_dictionary::DictionaryLujvoSegment<'static> {
        self.reference.segment()
    }
}

#[pymethods]
impl PyDictionaryLujvoSegment {
    /// Return whether this segment is a rafsi or a hyphen.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let module = native_module(py)?;
        string_enum_member(&module, self.segment().kind).map(Bound::unbind)
    }

    /// Return the exact decomposed surface text.
    #[requires(true)]
    #[ensures(ret == self.segment().surface)]
    #[getter]
    fn surface(&self) -> &str {
        self.segment().surface
    }

    /// Return the source word for rafsi segments, or `None` for hyphens.
    #[requires(true)]
    #[ensures(ret == self.segment().source_word)]
    #[getter]
    fn source_word(&self) -> Option<&str> {
        self.segment().source_word
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let kind = self.segment().kind.python_member_name();
        Ok(format!(
            "{PUBLIC_MODULE}.DictionaryLujvoSegment(kind={PUBLIC_MODULE}.DictionaryLujvoSegmentKind.{kind}, surface={})",
            string_repr(py, self.segment().surface)?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.lujvo.owner, &other.reference.lujvo.owner)
            && self.reference.lujvo.position == other.reference.lujvo.position
            && self.reference.item == other.reference.item
    }
}

/// Generated pattern-search keys for one dictionary entry.
#[invariant(
    true,
    "the validated PatternReference field makes every pattern wrapper state in range"
)]
#[pyclass(
    name = "DictionaryPatternEntry",
    frozen,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PyDictionaryPatternEntry {
    reference: PatternReference,
}

impl PyDictionaryPatternEntry {
    #[requires(self.reference.position.0 < self.reference.owner.dictionary().pattern_index().len())]
    #[ensures(std::ptr::eq(ret, self.reference.pattern()))]
    fn pattern(&self) -> &'static jbotci_dictionary::DictionaryPatternEntry<'static> {
        self.reference.pattern()
    }
}

#[pymethods]
impl PyDictionaryPatternEntry {
    /// Return the typed source entry index.
    #[requires(true)]
    #[ensures(ret.value == self.pattern().entry_index)]
    #[getter]
    fn entry_index(&self) -> PyEntryIndex {
        PyEntryIndex::from_rust(self.pattern().entry_index)
    }

    /// Return the normalized word pattern key.
    #[requires(true)]
    #[ensures(ret == self.pattern().word_key)]
    #[getter]
    fn word_key(&self) -> &str {
        self.pattern().word_key
    }

    /// Return normalized rafsi pattern keys in generated order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn rafsi_keys(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(sequence_to_tuple(py, self.pattern().rafsi_keys.iter().copied())?.unbind())
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.DictionaryPatternEntry(entry_index={}, word_key={})",
            self.pattern().entry_index.get(),
            string_repr(py, self.pattern().word_key)?
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn __eq__(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.reference.owner, &other.reference.owner)
            && self.reference.position == other.reference.position
    }
}

/// Metadata for the embedded English Lensisku snapshot.
#[invariant(true)]
#[pyclass(
    name = "DictionarySnapshotMetadata",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PyDictionarySnapshotMetadata {
    metadata: &'static DictionarySnapshotMetadata,
}

#[pymethods]
impl PyDictionarySnapshotMetadata {
    /// Return the BCP 47 language tag.
    #[requires(true)]
    #[ensures(ret == self.metadata.language_tag)]
    #[getter]
    fn language_tag(&self) -> &'static str {
        self.metadata.language_tag
    }

    /// Return the human-readable language name.
    #[requires(true)]
    #[ensures(ret == self.metadata.language_realname)]
    #[getter]
    fn language_realname(&self) -> &'static str {
        self.metadata.language_realname
    }

    /// Return the vendored snapshot format.
    #[requires(true)]
    #[ensures(ret == self.metadata.format)]
    #[getter]
    fn format(&self) -> &'static str {
        self.metadata.format
    }

    /// Return the vendored snapshot filename.
    #[requires(true)]
    #[ensures(ret == self.metadata.filename)]
    #[getter]
    fn filename(&self) -> &'static str {
        self.metadata.filename
    }

    /// Return the upstream metadata URL.
    #[requires(true)]
    #[ensures(ret == self.metadata.metadata_url)]
    #[getter]
    fn metadata_url(&self) -> &'static str {
        self.metadata.metadata_url
    }

    /// Return the upstream snapshot download URL.
    #[requires(true)]
    #[ensures(ret == self.metadata.download_url)]
    #[getter]
    fn download_url(&self) -> &'static str {
        self.metadata.download_url
    }

    /// Return the upstream Lensisku creation timestamp.
    #[requires(true)]
    #[ensures(ret == self.metadata.lensisku_created_at)]
    #[getter]
    fn lensisku_created_at(&self) -> &'static str {
        self.metadata.lensisku_created_at
    }

    /// Return the vendored snapshot SHA-256 digest.
    #[requires(true)]
    #[ensures(ret == self.metadata.sha256)]
    #[getter]
    fn sha256(&self) -> &'static str {
        self.metadata.sha256
    }

    /// Return the exact number of embedded entries.
    #[requires(true)]
    #[ensures(ret == self.metadata.entry_count)]
    #[getter]
    fn entry_count(&self) -> usize {
        self.metadata.entry_count
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.DictionarySnapshotMetadata(language_tag={}, entry_count={})",
            string_repr(py, self.metadata.language_tag)?,
            self.metadata.entry_count
        ))
    }
}

/// An invalid source entry found during dictionary validation.
#[invariant(true)]
#[pyclass(
    name = "InvalidEntryValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyInvalidEntryValidationDetail {
    index: usize,
    reason: &'static str,
}

#[pymethods]
impl PyInvalidEntryValidationDetail {
    /// Return the invalid source entry index.
    #[requires(true)]
    #[ensures(ret == self.index)]
    #[getter]
    fn index(&self) -> usize {
        self.index
    }

    /// Return the structural validation reason.
    #[requires(true)]
    #[ensures(ret == self.reason)]
    #[getter]
    fn reason(&self) -> &'static str {
        self.reason
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "invalid dictionary entry at index {}: {}",
            self.index, self.reason
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.InvalidEntryValidationDetail(index={}, reason={})",
            self.index,
            string_repr(py, self.reason)?
        ))
    }
}

/// Validation detail for a mismatched generated word index.
#[invariant(true)]
#[pyclass(
    name = "WordIndexMismatchValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PyWordIndexMismatchValidationDetail;

#[pymethods]
impl PyWordIndexMismatchValidationDetail {
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> &'static str {
        "word index does not match dictionary entries"
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.dictionary.WordIndexMismatchValidationDetail()"
    }
}

/// Validation detail for a mismatched generated rafsi index.
#[invariant(true)]
#[pyclass(
    name = "RafsiIndexMismatchValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PyRafsiIndexMismatchValidationDetail;

#[pymethods]
impl PyRafsiIndexMismatchValidationDetail {
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> &'static str {
        "rafsi index does not match dictionary entries"
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.dictionary.RafsiIndexMismatchValidationDetail()"
    }
}

/// Validation detail for a mismatched generated selma'o index.
#[invariant(true)]
#[pyclass(
    name = "SelmahoIndexMismatchValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PySelmahoIndexMismatchValidationDetail;

#[pymethods]
impl PySelmahoIndexMismatchValidationDetail {
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> &'static str {
        "selma'o index does not match dictionary entries"
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.dictionary.SelmahoIndexMismatchValidationDetail()"
    }
}

/// Validation detail for a mismatched generated pattern index.
#[invariant(true)]
#[pyclass(
    name = "PatternIndexMismatchValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PyPatternIndexMismatchValidationDetail;

#[pymethods]
impl PyPatternIndexMismatchValidationDetail {
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> &'static str {
        "pattern index does not match dictionary entries"
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.dictionary.PatternIndexMismatchValidationDetail()"
    }
}

/// An invalid generated sound record found during validation.
#[invariant(true)]
#[pyclass(
    name = "InvalidSoundIndexEntryValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyInvalidSoundIndexEntryValidationDetail {
    index: usize,
    reason: &'static str,
}

#[pymethods]
impl PyInvalidSoundIndexEntryValidationDetail {
    /// Return the invalid sound-record index.
    #[requires(true)]
    #[ensures(ret == self.index)]
    #[getter]
    fn index(&self) -> usize {
        self.index
    }

    /// Return the structural validation reason.
    #[requires(true)]
    #[ensures(ret == self.reason)]
    #[getter]
    fn reason(&self) -> &'static str {
        self.reason
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "invalid dictionary sound index entry at index {}: {}",
            self.index, self.reason
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.InvalidSoundIndexEntryValidationDetail(index={}, reason={})",
            self.index,
            string_repr(py, self.reason)?
        ))
    }
}

/// An invalid generated lujvo record found during validation.
#[invariant(true)]
#[pyclass(
    name = "InvalidLujvoIndexEntryValidationDetail",
    frozen,
    eq,
    module = "jbotci.dictionary",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyInvalidLujvoIndexEntryValidationDetail {
    index: usize,
    reason: &'static str,
}

#[pymethods]
impl PyInvalidLujvoIndexEntryValidationDetail {
    /// Return the invalid lujvo-record index.
    #[requires(true)]
    #[ensures(ret == self.index)]
    #[getter]
    fn index(&self) -> usize {
        self.index
    }

    /// Return the structural validation reason.
    #[requires(true)]
    #[ensures(ret == self.reason)]
    #[getter]
    fn reason(&self) -> &'static str {
        self.reason
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "invalid dictionary lujvo index entry at index {}: {}",
            self.index, self.reason
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.InvalidLujvoIndexEntryValidationDetail(index={}, reason={})",
            self.index,
            string_repr(py, self.reason)?
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn dictionary_validation_py_err(
    py: Python<'_>,
    error: DictionaryValidationError,
) -> PyResult<PyErr> {
    let detail = match error {
        DictionaryValidationError::InvalidEntry { index, reason } => {
            Py::new(py, PyInvalidEntryValidationDetail { index, reason })?.into_any()
        }
        DictionaryValidationError::WordIndexMismatch => {
            Py::new(py, PyWordIndexMismatchValidationDetail)?.into_any()
        }
        DictionaryValidationError::RafsiIndexMismatch => {
            Py::new(py, PyRafsiIndexMismatchValidationDetail)?.into_any()
        }
        DictionaryValidationError::SelmahoIndexMismatch => {
            Py::new(py, PySelmahoIndexMismatchValidationDetail)?.into_any()
        }
        DictionaryValidationError::PatternIndexMismatch => {
            Py::new(py, PyPatternIndexMismatchValidationDetail)?.into_any()
        }
        DictionaryValidationError::InvalidSoundIndexEntry { index, reason } => Py::new(
            py,
            PyInvalidSoundIndexEntryValidationDetail { index, reason },
        )?
        .into_any(),
        DictionaryValidationError::InvalidLujvoIndexEntry { index, reason } => Py::new(
            py,
            PyInvalidLujvoIndexEntryValidationDetail { index, reason },
        )?
        .into_any(),
    };
    let exception = public_module(py)?
        .getattr("DictionaryValidationError")?
        .call1((detail,))?;
    Ok(PyErr::from_value(exception))
}

/// Normalize a dictionary lookup query exactly as the Rust dictionary does.
#[requires(true)]
#[ensures(ret == normalize_lookup_query(raw))]
#[pyfunction(name = "normalize_lookup_query")]
fn py_normalize_lookup_query(raw: &str) -> String {
    normalize_lookup_query(raw)
}

/// Normalize a pattern lookup key exactly as the Rust dictionary does.
#[requires(true)]
#[ensures(ret == normalize_pattern_lookup_key(raw))]
#[pyfunction(name = "normalize_pattern_lookup_key")]
fn py_normalize_pattern_lookup_key(raw: &str) -> String {
    normalize_pattern_lookup_key(raw)
}

/// Return universal gismu rafsi forms and their provenance.
#[requires(true)]
#[ensures(true)]
#[pyfunction(name = "universal_gismu_rafsi_forms")]
fn py_universal_gismu_rafsi_forms(py: Python<'_>, word: &str) -> PyResult<Py<PyTuple>> {
    let module = native_module(py)?;
    let values = universal_gismu_rafsi_forms(word)
        .into_iter()
        .map(|(rafsi, source)| {
            let rafsi = Py::new(py, PyRafsi::owned(rafsi))?.into_any();
            let source = string_enum_member(&module, source)?.unbind();
            Ok(sequence_to_tuple(py, [rafsi, source])?.unbind())
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(sequence_to_tuple(py, values)?.unbind())
}

/// Call the Rust gismu-like predicate after exact registered-enum extraction.
#[requires(true)]
#[ensures(true)]
#[pyfunction(name = "_word_type_is_gismu_like")]
fn py_word_type_is_gismu_like(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<bool> {
    let module = native_module(py)?;
    Ok(extract_string_enum::<WordType>(&module, value)?.is_gismu_like())
}

/// Call the Rust lujvo-like predicate after exact registered-enum extraction.
#[requires(true)]
#[ensures(true)]
#[pyfunction(name = "_word_type_is_lujvo_like")]
fn py_word_type_is_lujvo_like(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<bool> {
    let module = native_module(py)?;
    Ok(extract_string_enum::<WordType>(&module, value)?.is_lujvo_like())
}

#[requires(true)]
#[ensures(true)]
fn register_types(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_type::<PyDictionary>(module, "_dictionary_Dictionary")?;
    register_type::<PyDictionaryEntries>(module, "_dictionary_DictionaryEntries")?;
    register_type::<PyDictionaryEntry>(module, "_dictionary_DictionaryEntry")?;
    register_type::<PyDefinitionId>(module, "_dictionary_DefinitionId")?;
    register_type::<PyScore>(module, "_dictionary_Score")?;
    register_type::<PyKeyword>(module, "_dictionary_Keyword")?;
    register_type::<PyRafsi>(module, "_dictionary_Rafsi")?;
    register_type::<PyRawSelmaho>(module, "_dictionary_RawSelmaho")?;
    register_type::<PyDictionaryUser>(module, "_dictionary_DictionaryUser")?;
    register_type::<PyEntryIndex>(module, "_dictionary_EntryIndex")?;
    register_type::<PyRafsiMatch>(module, "_dictionary_RafsiMatch")?;
    register_type::<PyFreeRafsiAvailability>(module, "_dictionary_FreeRafsiAvailability")?;
    register_type::<PyTakenRafsiAvailability>(module, "_dictionary_TakenRafsiAvailability")?;
    register_type::<PyRafsiCandidate>(module, "_dictionary_RafsiCandidate")?;
    register_type::<PyDictionarySoundEntry>(module, "_dictionary_DictionarySoundEntry")?;
    register_type::<PyIpaTokenSequenceView>(module, "_dictionary_IpaTokenSequenceView")?;
    register_type::<PyIpaSegmentId>(module, "_dictionary_IpaSegmentId")?;
    register_type::<PyPronunciationTargetSequenceView>(
        module,
        "_dictionary_PronunciationTargetSequenceView",
    )?;
    register_type::<PyPronunciationTargetId>(module, "_dictionary_PronunciationTargetId")?;
    register_type::<PyDictionaryLujvoEntry>(module, "_dictionary_DictionaryLujvoEntry")?;
    register_type::<PyDictionaryLujvoSegment>(module, "_dictionary_DictionaryLujvoSegment")?;
    register_type::<PyDictionaryPatternEntry>(module, "_dictionary_DictionaryPatternEntry")?;
    register_type::<PyDictionarySnapshotMetadata>(
        module,
        "_dictionary_DictionarySnapshotMetadata",
    )?;
    register_type::<PyInvalidEntryValidationDetail>(
        module,
        "_dictionary_InvalidEntryValidationDetail",
    )?;
    register_type::<PyWordIndexMismatchValidationDetail>(
        module,
        "_dictionary_WordIndexMismatchValidationDetail",
    )?;
    register_type::<PyRafsiIndexMismatchValidationDetail>(
        module,
        "_dictionary_RafsiIndexMismatchValidationDetail",
    )?;
    register_type::<PySelmahoIndexMismatchValidationDetail>(
        module,
        "_dictionary_SelmahoIndexMismatchValidationDetail",
    )?;
    register_type::<PyPatternIndexMismatchValidationDetail>(
        module,
        "_dictionary_PatternIndexMismatchValidationDetail",
    )?;
    register_type::<PyInvalidSoundIndexEntryValidationDetail>(
        module,
        "_dictionary_InvalidSoundIndexEntryValidationDetail",
    )?;
    register_type::<PyInvalidLujvoIndexEntryValidationDetail>(
        module,
        "_dictionary_InvalidLujvoIndexEntryValidationDetail",
    )?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn register_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let normalize = wrap_pyfunction!(py_normalize_lookup_query, module)?;
    register_private_object(module, "_dictionary_normalize_lookup_query", normalize)?;
    let normalize_pattern = wrap_pyfunction!(py_normalize_pattern_lookup_key, module)?;
    register_private_object(
        module,
        "_dictionary_normalize_pattern_lookup_key",
        normalize_pattern,
    )?;
    let universal = wrap_pyfunction!(py_universal_gismu_rafsi_forms, module)?;
    register_private_object(module, "_dictionary_universal_gismu_rafsi_forms", universal)?;
    let gismu_like = wrap_pyfunction!(py_word_type_is_gismu_like, module)?;
    register_private_object(module, "_dictionary_word_type_is_gismu_like", gismu_like)?;
    let lujvo_like = wrap_pyfunction!(py_word_type_is_lujvo_like, module)?;
    register_private_object(module, "_dictionary_word_type_is_lujvo_like", lujvo_like)?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn register_values(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let dictionary = Py::new(module.py(), PyDictionary::english())?;
    register_private_object(module, "_dictionary_english", dictionary)?;
    let metadata = Py::new(
        module.py(),
        PyDictionarySnapshotMetadata {
            metadata: english_metadata(),
        },
    )?;
    register_private_object(module, "_dictionary_english_metadata", metadata)
}

/// Register all dictionary-owned native exports for one interpreter module.
#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_types(module)?;
    register_string_enum::<WordType>(module)?;
    register_string_enum::<RafsiSource>(module)?;
    register_string_enum::<DictionaryLujvoSegmentKind>(module)?;
    register_string_enum::<RafsiClaimKind>(module)?;
    register_functions(module)?;
    register_values(module)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[invariant(true)]
    #[derive(Debug)]
    struct DeliberateModuleOverridePanic;

    #[invariant(true)]
    struct TestModuleOverrideScope {
        previous_native: Option<Py<PyModule>>,
        previous_public: Option<Py<PyModule>>,
        not_send: PhantomData<Rc<()>>,
    }

    impl TestModuleOverrideScope {
        #[requires(true)]
        #[ensures(true)]
        fn enter(
            native: Option<&Bound<'_, PyModule>>,
            public: Option<&Bound<'_, PyModule>>,
        ) -> Self {
            let previous_native = TEST_NATIVE_MODULE
                .with(|slot| slot.replace(native.map(|module| module.clone().unbind())));
            let previous_public = TEST_PUBLIC_MODULE
                .with(|slot| slot.replace(public.map(|module| module.clone().unbind())));
            Self {
                previous_native,
                previous_public,
                not_send: PhantomData,
            }
        }
    }

    impl Drop for TestModuleOverrideScope {
        #[requires(true)]
        #[ensures(true)]
        fn drop(&mut self) {
            TEST_PUBLIC_MODULE.with(|slot| {
                slot.replace(self.previous_public.take());
            });
            TEST_NATIVE_MODULE.with(|slot| {
                slot.replace(self.previous_native.take());
            });
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_dictionary_validation_error_projection(
        py: Python<'_>,
        native_module: &Bound<'_, PyModule>,
        public_exception_type: &Bound<'_, PyAny>,
        error: DictionaryValidationError,
        detail_export: &str,
        detail_class_name: &str,
        expected_index: Option<usize>,
        expected_reason: Option<&str>,
    ) {
        let expected_message = error.to_string();
        let py_error = dictionary_validation_py_err(py, error).unwrap();
        assert_eq!(
            py_error.get_type(py).as_ptr(),
            public_exception_type.as_ptr()
        );

        let exception = py_error.value(py);
        assert_eq!(exception.to_string(), expected_message);
        let args_any = exception.getattr("args").unwrap();
        let args = args_any.cast::<PyTuple>().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(
            args.get_item(0).unwrap().extract::<String>().unwrap(),
            expected_message
        );
        let detail = exception.getattr("detail").unwrap();
        let expected_detail_type = native_module.getattr(detail_export).unwrap();
        assert_eq!(detail.get_type().as_ptr(), expected_detail_type.as_ptr());
        assert_eq!(
            detail
                .get_type()
                .getattr("__module__")
                .unwrap()
                .extract::<String>()
                .unwrap(),
            PUBLIC_MODULE
        );
        assert_eq!(
            detail
                .get_type()
                .getattr("__name__")
                .unwrap()
                .extract::<String>()
                .unwrap(),
            detail_class_name
        );
        assert_eq!(detail.to_string(), expected_message);

        if let Some(expected_index) = expected_index {
            assert_eq!(
                detail.getattr("index").unwrap().extract::<usize>().unwrap(),
                expected_index
            );
        }
        if let Some(expected_reason) = expected_reason {
            assert_eq!(
                detail
                    .getattr("reason")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                expected_reason
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn owner_recovers_source_positions_without_scanning() {
        let owner = DictionaryOwner::english();
        let entry = owner.dictionary().lookup_word("klama").unwrap();
        let position = owner.entry_position(entry);
        assert!(std::ptr::eq(
            entry,
            &owner.dictionary().entries()[position.0]
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_view_retains_owner_without_materializing_entries() {
        let owner = Arc::new(DictionaryOwner::english());
        let entry = owner.dictionary().lookup_word("bangu").unwrap();
        let entry_position = owner.entry_position(entry);
        assert_eq!(Arc::strong_count(&owner), 1);
        let keyword = PyKeyword::borrowed(
            Arc::clone(&owner),
            KeywordPosition {
                entry: entry_position,
                kind: KeywordKind::Gloss,
                item: 0,
            },
        );
        assert_eq!(Arc::strong_count(&owner), 2);
        drop(owner);
        assert_eq!(keyword.word(), "language");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pronunciation_target_view_retains_its_sound_owner() {
        let owner = Arc::new(DictionaryOwner::english());
        let entry = owner.dictionary().lookup_word("prami").unwrap();
        let entry_index = owner.dictionary().entry_index_for_entry(entry).unwrap();
        let position = owner
            .dictionary()
            .sound_index()
            .iter()
            .position(|sound| sound.entry_index == entry_index)
            .expect("prami has a generated sound record");
        assert_eq!(Arc::strong_count(&owner), 1);
        let sequence = PyPronunciationTargetSequenceView {
            reference: SoundReference::new(Arc::clone(&owner), SoundPosition(position)),
        };
        assert_eq!(Arc::strong_count(&owner), 2);
        drop(owner);

        let rhotic = PyPronunciationTargetId {
            value: sequence
                .sequence()
                .targets
                .iter()
                .copied()
                .find(|target| target.realization_count() > 1)
                .expect("prami contains the multi-realization rhotic target"),
        };
        assert!(rhotic.realization_count() > 1);
        assert!(rhotic.value.realization(0).is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_references_reject_out_of_range_contextual_positions() {
        let owner = Arc::new(DictionaryOwner::english());
        assert!(
            try_new!(EntryReference {
                owner: Arc::clone(&owner),
                position: EntryPosition(owner.dictionary().entries().len()),
            })
            .is_err()
        );
        assert!(
            try_new!(SoundReference {
                owner: Arc::clone(&owner),
                position: SoundPosition(owner.dictionary().sound_index().len()),
            })
            .is_err()
        );
        assert!(
            try_new!(LujvoReference {
                owner: Arc::clone(&owner),
                position: LujvoPosition(owner.dictionary().lujvo_index().len()),
            })
            .is_err()
        );
        assert!(
            try_new!(PatternReference {
                owner: Arc::clone(&owner),
                position: PatternPosition(owner.dictionary().pattern_index().len()),
            })
            .is_err()
        );
        let lujvo = LujvoReference::new(Arc::clone(&owner), LujvoPosition(0));
        let item = lujvo.lujvo().segments.len();
        assert!(try_new!(LujvoSegmentReference { lujvo, item }).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn test_module_overrides_are_thread_local_and_panic_safe() {
        Python::initialize();
        Python::attach(|py| {
            assert!(TEST_NATIVE_MODULE.with(|slot| slot.borrow().is_none()));
            assert!(TEST_PUBLIC_MODULE.with(|slot| slot.borrow().is_none()));
            let modules_any = py.import("sys").unwrap().getattr("modules").unwrap();
            let modules = modules_any.cast::<PyDict>().unwrap();
            let native_before = modules
                .get_item("jbotci._native")
                .unwrap()
                .map(|module| module.as_ptr());
            let public_before = modules
                .get_item(PUBLIC_MODULE)
                .unwrap()
                .map(|module| module.as_ptr());
            let native_override = PyModule::new(py, "test_native_override").unwrap();
            let public_override = PyModule::new(py, "test_public_override").unwrap();

            let result = catch_unwind(AssertUnwindSafe(|| {
                let _scope =
                    TestModuleOverrideScope::enter(Some(&native_override), Some(&public_override));
                assert_eq!(
                    native_module(py).unwrap().as_ptr(),
                    native_override.as_ptr()
                );
                assert_eq!(
                    public_module(py).unwrap().as_ptr(),
                    public_override.as_ptr()
                );
                panic_any(DeliberateModuleOverridePanic);
            }));
            match result {
                Err(payload) if payload.is::<DeliberateModuleOverridePanic>() => {}
                Err(payload) => resume_unwind(payload),
                Ok(()) => panic!("module override scope did not propagate the sentinel panic"),
            }
            assert!(TEST_NATIVE_MODULE.with(|slot| slot.borrow().is_none()));
            assert!(TEST_PUBLIC_MODULE.with(|slot| slot.borrow().is_none()));
            assert_eq!(
                modules
                    .get_item("jbotci._native")
                    .unwrap()
                    .map(|module| module.as_ptr()),
                native_before
            );
            assert_eq!(
                modules
                    .get_item(PUBLIC_MODULE)
                    .unwrap()
                    .map(|module| module.as_ptr()),
                public_before
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn validation_failures_have_public_typed_python_details() {
        Python::initialize();
        Python::attach(|py| {
            let native_module = PyModule::new(py, "jbotci._native").unwrap();
            register(&native_module).unwrap();
            let public_module = PyModule::new(py, PUBLIC_MODULE).unwrap();
            py.run(
                c"\
class JbotciError(Exception):
    pass

class DictionaryValidationError(JbotciError):
    def __init__(self, detail):
        self.detail = detail
        super().__init__(str(detail))
",
                Some(&public_module.dict()),
                None,
            )
            .unwrap();
            let _scope = TestModuleOverrideScope::enter(Some(&native_module), Some(&public_module));
            let exception_type = public_module.getattr("DictionaryValidationError").unwrap();
            assert_eq!(
                exception_type
                    .getattr("__module__")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                PUBLIC_MODULE
            );

            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::InvalidEntry {
                    index: 11,
                    reason: "entry reason",
                },
                "_dictionary_InvalidEntryValidationDetail",
                "InvalidEntryValidationDetail",
                Some(11),
                Some("entry reason"),
            );
            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::WordIndexMismatch,
                "_dictionary_WordIndexMismatchValidationDetail",
                "WordIndexMismatchValidationDetail",
                None,
                None,
            );
            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::RafsiIndexMismatch,
                "_dictionary_RafsiIndexMismatchValidationDetail",
                "RafsiIndexMismatchValidationDetail",
                None,
                None,
            );
            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::SelmahoIndexMismatch,
                "_dictionary_SelmahoIndexMismatchValidationDetail",
                "SelmahoIndexMismatchValidationDetail",
                None,
                None,
            );
            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::PatternIndexMismatch,
                "_dictionary_PatternIndexMismatchValidationDetail",
                "PatternIndexMismatchValidationDetail",
                None,
                None,
            );
            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::InvalidSoundIndexEntry {
                    index: 12,
                    reason: "sound reason",
                },
                "_dictionary_InvalidSoundIndexEntryValidationDetail",
                "InvalidSoundIndexEntryValidationDetail",
                Some(12),
                Some("sound reason"),
            );
            assert_dictionary_validation_error_projection(
                py,
                &native_module,
                &exception_type,
                DictionaryValidationError::InvalidLujvoIndexEntry {
                    index: 13,
                    reason: "lujvo reason",
                },
                "_dictionary_InvalidLujvoIndexEntryValidationDetail",
                "InvalidLujvoIndexEntryValidationDetail",
                Some(13),
                Some("lujvo reason"),
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn python_projection_matches_the_complete_rust_snapshot() {
        Python::initialize();
        Python::attach(|py| {
            let module = PyModule::new(py, "jbotci._native").unwrap();
            register(&module).unwrap();
            let _scope = TestModuleOverrideScope::enter(Some(&module), None);
            let rust_dictionary = english();
            let python_dictionary = module.getattr("_dictionary_english").unwrap();
            assert_eq!(
                python_dictionary.len().unwrap(),
                rust_dictionary.entries().len()
            );

            for (index, rust_entry) in rust_dictionary.entries().iter().enumerate() {
                let python_entry = python_dictionary
                    .call_method1("__getitem__", (index,))
                    .unwrap();
                assert_eq!(
                    python_entry
                        .getattr("word")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_entry.word
                );
                assert_eq!(
                    python_entry
                        .getattr("word_type")
                        .unwrap()
                        .getattr("value")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_entry.word_type.as_str()
                );
                assert_eq!(
                    python_entry
                        .getattr("definition")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_entry.definition
                );
                assert_eq!(
                    python_entry
                        .getattr("definition_id")
                        .unwrap()
                        .getattr("value")
                        .unwrap()
                        .extract::<u64>()
                        .unwrap(),
                    rust_entry.definition_id.get()
                );
                assert_eq!(
                    python_entry
                        .getattr("notes")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_entry.notes
                );
                assert_eq!(
                    python_entry
                        .getattr("score")
                        .unwrap()
                        .getattr("value")
                        .unwrap()
                        .extract::<f64>()
                        .unwrap()
                        .to_bits(),
                    rust_entry.score.get().to_bits()
                );

                for (attribute, rust_keywords) in [
                    ("gloss_keywords", rust_entry.gloss_keywords),
                    ("place_keywords", rust_entry.place_keywords),
                ] {
                    let python_keywords_any = python_entry.getattr(attribute).unwrap();
                    let python_keywords = python_keywords_any.cast::<PyTuple>().unwrap();
                    assert_eq!(python_keywords.len(), rust_keywords.len());
                    for (python_keyword, rust_keyword) in
                        python_keywords.iter().zip(rust_keywords.iter())
                    {
                        assert_eq!(
                            python_keyword
                                .getattr("word")
                                .unwrap()
                                .extract::<String>()
                                .unwrap(),
                            rust_keyword.word
                        );
                        assert_eq!(
                            python_keyword
                                .getattr("meaning")
                                .unwrap()
                                .extract::<Option<String>>()
                                .unwrap()
                                .as_deref(),
                            rust_keyword.meaning
                        );
                    }
                }

                let python_rafsi_any = python_entry.getattr("rafsi").unwrap();
                let python_rafsi = python_rafsi_any.cast::<PyTuple>().unwrap();
                assert_eq!(python_rafsi.len(), rust_entry.rafsi.len());
                for (python_rafsi, rust_rafsi) in python_rafsi.iter().zip(rust_entry.rafsi.iter()) {
                    assert_eq!(
                        python_rafsi
                            .getattr("value")
                            .unwrap()
                            .extract::<String>()
                            .unwrap(),
                        rust_rafsi.0
                    );
                }

                let python_selmaho = python_entry.getattr("selmaho").unwrap();
                match rust_entry.selmaho {
                    Some(rust_selmaho) => assert_eq!(
                        python_selmaho
                            .getattr("value")
                            .unwrap()
                            .extract::<String>()
                            .unwrap(),
                        rust_selmaho.0
                    ),
                    None => assert!(python_selmaho.is_none()),
                }
                assert_eq!(
                    python_entry
                        .getattr("etymology")
                        .unwrap()
                        .extract::<Option<String>>()
                        .unwrap()
                        .as_deref(),
                    rust_entry.etymology
                );
                assert_eq!(
                    python_entry
                        .getattr("jargon")
                        .unwrap()
                        .extract::<Option<String>>()
                        .unwrap()
                        .as_deref(),
                    rust_entry.jargon
                );
                let python_user = python_entry.getattr("user").unwrap();
                assert_eq!(
                    python_user
                        .getattr("username")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_entry.user.username
                );
                assert_eq!(
                    python_user
                        .getattr("realname")
                        .unwrap()
                        .extract::<Option<String>>()
                        .unwrap()
                        .as_deref(),
                    rust_entry.user.realname
                );
            }

            let collision_query = rust_dictionary
                .entries()
                .iter()
                .find(|entry| rust_dictionary.lookup_words(entry.word).count() > 1)
                .expect("snapshot contains a normalized collision")
                .word;
            let rust_collision = rust_dictionary
                .lookup_words(collision_query)
                .map(|entry| entry.word)
                .collect::<Vec<_>>();
            let python_collision_any = python_dictionary
                .call_method1("lookup_words", (collision_query,))
                .unwrap();
            let python_collision = python_collision_any.cast::<PyTuple>().unwrap();
            assert_eq!(python_collision.len(), rust_collision.len());
            for (python_entry, rust_word) in python_collision.iter().zip(rust_collision) {
                assert_eq!(
                    python_entry
                        .getattr("word")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_word
                );
            }

            let python_sounds_any = python_dictionary.getattr("sound_index").unwrap();
            let python_sounds = python_sounds_any.cast::<PyTuple>().unwrap();
            assert_eq!(python_sounds.len(), rust_dictionary.sound_index().len());
            for (python_sound, rust_sound) in
                python_sounds.iter().zip(rust_dictionary.sound_index())
            {
                assert_eq!(
                    python_sound
                        .getattr("entry_index")
                        .unwrap()
                        .getattr("value")
                        .unwrap()
                        .extract::<usize>()
                        .unwrap(),
                    rust_sound.entry_index.get()
                );
                assert_eq!(
                    python_sound
                        .getattr("ipa")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_sound.ipa
                );
                let python_sequence = python_sound.getattr("token_sequence").unwrap();
                assert_eq!(
                    python_sequence
                        .getattr("self_similarity")
                        .unwrap()
                        .extract::<f64>()
                        .unwrap()
                        .to_bits(),
                    rust_sound.token_sequence.self_similarity.to_bits()
                );
                let python_segments_any = python_sequence.getattr("segments").unwrap();
                let python_segments = python_segments_any.cast::<PyTuple>().unwrap();
                assert_eq!(
                    python_segments
                        .iter()
                        .map(|segment| {
                            segment.getattr("value").unwrap().extract::<u16>().unwrap()
                        })
                        .collect::<Vec<_>>(),
                    rust_sound
                        .token_sequence
                        .segments
                        .iter()
                        .map(|segment| segment.get())
                        .collect::<Vec<_>>()
                );

                let python_target_sequence = python_sound.getattr("pronunciation_targets").unwrap();
                assert_eq!(
                    python_target_sequence
                        .getattr("self_similarity")
                        .unwrap()
                        .extract::<f64>()
                        .unwrap()
                        .to_bits(),
                    rust_sound.pronunciation_targets.self_similarity.to_bits()
                );
                assert_eq!(
                    python_target_sequence.len().unwrap(),
                    rust_sound.pronunciation_targets.target_count()
                );
                assert_eq!(
                    python_target_sequence
                        .call_method0("target_count")
                        .unwrap()
                        .extract::<usize>()
                        .unwrap(),
                    rust_sound.pronunciation_targets.target_count()
                );
                let python_targets_any = python_target_sequence.getattr("targets").unwrap();
                let python_targets = python_targets_any.cast::<PyTuple>().unwrap();
                assert_eq!(
                    python_targets.len(),
                    rust_sound.pronunciation_targets.targets.len()
                );
                for (python_target, rust_target) in python_targets
                    .iter()
                    .zip(rust_sound.pronunciation_targets.targets)
                {
                    assert_eq!(
                        python_target
                            .getattr("value")
                            .unwrap()
                            .extract::<u16>()
                            .unwrap(),
                        rust_target.get()
                    );
                    assert_eq!(
                        python_target
                            .getattr("realization_count")
                            .unwrap()
                            .extract::<usize>()
                            .unwrap(),
                        rust_target.realization_count()
                    );
                    let python_realizations_any = python_target.getattr("realizations").unwrap();
                    let python_realizations = python_realizations_any.cast::<PyTuple>().unwrap();
                    assert_eq!(python_realizations.len(), rust_target.realization_count());
                    for (index, python_realization) in python_realizations.iter().enumerate() {
                        assert_eq!(
                            python_realization
                                .getattr("value")
                                .unwrap()
                                .extract::<u16>()
                                .unwrap(),
                            rust_target
                                .realization(index)
                                .expect("indices below realization_count must be present")
                                .get()
                        );
                    }
                }
            }

            let python_lujvo_any = python_dictionary.getattr("lujvo_index").unwrap();
            let python_lujvo = python_lujvo_any.cast::<PyTuple>().unwrap();
            assert_eq!(python_lujvo.len(), rust_dictionary.lujvo_index().len());
            for (python_lujvo, rust_lujvo) in python_lujvo.iter().zip(rust_dictionary.lujvo_index())
            {
                assert_eq!(
                    python_lujvo
                        .getattr("entry_index")
                        .unwrap()
                        .getattr("value")
                        .unwrap()
                        .extract::<usize>()
                        .unwrap(),
                    rust_lujvo.entry_index.get()
                );
                let python_source_words = python_lujvo
                    .getattr("source_words")
                    .unwrap()
                    .extract::<Vec<String>>()
                    .unwrap();
                assert!(
                    python_source_words
                        .iter()
                        .map(String::as_str)
                        .eq(rust_lujvo.source_words.iter().copied())
                );
                let python_segments_any = python_lujvo.getattr("segments").unwrap();
                let python_segments = python_segments_any.cast::<PyTuple>().unwrap();
                assert_eq!(python_segments.len(), rust_lujvo.segments.len());
                for (python_segment, rust_segment) in
                    python_segments.iter().zip(rust_lujvo.segments)
                {
                    assert_eq!(
                        python_segment
                            .getattr("kind")
                            .unwrap()
                            .getattr("value")
                            .unwrap()
                            .extract::<String>()
                            .unwrap(),
                        rust_segment.kind.python_value()
                    );
                    assert_eq!(
                        python_segment
                            .getattr("surface")
                            .unwrap()
                            .extract::<String>()
                            .unwrap(),
                        rust_segment.surface
                    );
                    assert_eq!(
                        python_segment
                            .getattr("source_word")
                            .unwrap()
                            .extract::<Option<String>>()
                            .unwrap()
                            .as_deref(),
                        rust_segment.source_word
                    );
                }
            }

            let python_patterns_any = python_dictionary.getattr("pattern_index").unwrap();
            let python_patterns = python_patterns_any.cast::<PyTuple>().unwrap();
            assert_eq!(python_patterns.len(), rust_dictionary.pattern_index().len());
            for (python_pattern, rust_pattern) in
                python_patterns.iter().zip(rust_dictionary.pattern_index())
            {
                assert_eq!(
                    python_pattern
                        .getattr("entry_index")
                        .unwrap()
                        .getattr("value")
                        .unwrap()
                        .extract::<usize>()
                        .unwrap(),
                    rust_pattern.entry_index.get()
                );
                assert_eq!(
                    python_pattern
                        .getattr("word_key")
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_pattern.word_key
                );
                let python_rafsi_keys = python_pattern
                    .getattr("rafsi_keys")
                    .unwrap()
                    .extract::<Vec<String>>()
                    .unwrap();
                assert!(
                    python_rafsi_keys
                        .iter()
                        .map(String::as_str)
                        .eq(rust_pattern.rafsi_keys.iter().copied())
                );
            }

            let rust_metadata = english_metadata();
            let python_metadata = module.getattr("_dictionary_english_metadata").unwrap();
            for (attribute, rust_value) in [
                ("language_tag", rust_metadata.language_tag),
                ("language_realname", rust_metadata.language_realname),
                ("format", rust_metadata.format),
                ("filename", rust_metadata.filename),
                ("metadata_url", rust_metadata.metadata_url),
                ("download_url", rust_metadata.download_url),
                ("lensisku_created_at", rust_metadata.lensisku_created_at),
                ("sha256", rust_metadata.sha256),
            ] {
                assert_eq!(
                    python_metadata
                        .getattr(attribute)
                        .unwrap()
                        .extract::<String>()
                        .unwrap(),
                    rust_value
                );
            }
            assert_eq!(
                python_metadata
                    .getattr("entry_count")
                    .unwrap()
                    .extract::<usize>()
                    .unwrap(),
                rust_metadata.entry_count
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn python_prami_target_preserves_every_rust_rhotic_realization() {
        Python::initialize();
        Python::attach(|py| {
            let module = PyModule::new(py, "jbotci._native").unwrap();
            register(&module).unwrap();
            let _scope = TestModuleOverrideScope::enter(Some(&module), None);
            let rust_dictionary = english();
            let rust_entry = rust_dictionary.lookup_word("prami").unwrap();
            let rust_entry_index = rust_dictionary.entry_index_for_entry(rust_entry).unwrap();
            let sound_position = rust_dictionary
                .sound_index()
                .iter()
                .position(|sound| sound.entry_index == rust_entry_index)
                .expect("prami has a generated sound record");
            let (target_position, rust_target) = rust_dictionary.sound_index()[sound_position]
                .pronunciation_targets
                .targets
                .iter()
                .copied()
                .enumerate()
                .find(|(_, target)| target.realization_count() > 1)
                .expect("prami contains the multi-realization rhotic target");
            assert!(rust_target.realization_count() > 1);

            let python_dictionary = module.getattr("_dictionary_english").unwrap();
            let python_sounds_any = python_dictionary.getattr("sound_index").unwrap();
            let python_sound = python_sounds_any
                .cast::<PyTuple>()
                .unwrap()
                .get_item(sound_position)
                .unwrap();
            let python_targets_any = python_sound
                .getattr("pronunciation_targets")
                .unwrap()
                .getattr("targets")
                .unwrap();
            let python_target = python_targets_any
                .cast::<PyTuple>()
                .unwrap()
                .get_item(target_position)
                .unwrap();

            for index in 0..rust_target.realization_count() {
                let python_realization =
                    python_target.call_method1("realization", (index,)).unwrap();
                assert_eq!(
                    python_realization
                        .getattr("value")
                        .unwrap()
                        .extract::<u16>()
                        .unwrap(),
                    rust_target
                        .realization(index)
                        .expect("indices below realization_count must be present")
                        .get()
                );
            }
            for index in [
                rust_target.realization_count(),
                rust_target.realization_count() + 100,
            ] {
                assert!(
                    python_target
                        .call_method1("realization", (index,))
                        .unwrap()
                        .is_none()
                );
            }
            assert!(
                python_target
                    .call_method1("realization", (-1,))
                    .unwrap()
                    .is_none()
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn numeric_wrappers_preserve_noncontextual_rust_invariants() {
        assert_eq!(PyDefinitionId::new(0).value(), 0);
        assert_eq!(PyEntryIndex::new(usize::MAX).value(), usize::MAX);
        assert!(PyScore::new(f64::NAN).value().is_nan());
        assert_eq!(PyScore::new(f64::INFINITY).value(), f64::INFINITY);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn negative_indices_follow_python_rules() {
        assert_eq!(normalize_sequence_index(-1, 4).unwrap(), 3);
        assert_eq!(normalize_sequence_index(-4, 4).unwrap(), 0);
        assert!(normalize_sequence_index(-5, 4).is_err());
        assert!(normalize_sequence_index(4, 4).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn enum_metadata_matches_rust_values() {
        for variant in WordType::variants() {
            assert_eq!(variant.python_value(), variant.as_str());
        }
    }
}
