//! Structured XML word definition cards (#709) and the `COMPOSITE-APPROX`
//! composition model for dictionary-absent lujvo and zei compounds.
//!
//! This module holds the dictionary-card MODEL and BUILDER only; XML emission
//! is a separate layer. The design follows the converged lujvo-approx spec
//! and the owner review rounds 11–13 (`tersmu-dsl-research`
//! `smusni-v2-design-record.md`):
//!
//! - Cards exist for content words only. Mirroring the owner doctrine in
//!   `jbotci-search`'s `vlacku.rs` (`push_content_dictionary_lookup_targets`):
//!   cmavo never get cards (cmavo semantics live in the semantic graph);
//!   brivla always do; cmevla only when the dictionary has an exact entry;
//!   zei compounds do; `zo`/`ma'oi` quotes define the referenced word;
//!   lerfu words and `zoi`/`lo'u` quotes never do.
//! - Dictionary-defined words — including defined lujvo and defined
//!   zei-lujvo — get a plain card: glosses, definition, notes. The dictionary
//!   is the sole authority for defined words (#450 doctrine); a defined word
//!   NEVER gets a composition approximation.
//! - Dictionary-absent gismu/fu'ivla (and entry-less-definition cmevla) get a
//!   bare `known: false` card. Dictionary-absent lujvo and zei compounds get
//!   a mechanical composition approximation when, and only when, every part
//!   is structurally renderable through the closed operator table below; any
//!   unrenderable part, unresolvable rafsi, or malformed grouping makes the
//!   whole composition fail closed (warning-only card, never a guessed or
//!   partial tree).
//!
//! Vocabulary alignment notes:
//!
//! - Abstraction kinds reuse [`crate::model::AbstractionKind`]; actuality
//!   facets reuse [`crate::model::ActualityKind`] exactly as the semantic
//!   graph builder maps CAhA (`ca'a` actual, `ka'e` capable, `nu'o`
//!   potential, `pu'i` demonstrated; `generated_builder::tense_modal`).
//! - Aspect contours reuse the semantic model's `Aspect::contour` strings
//!   (`generated_builder::tense_modal::aspect_contour_for_zaho_token`);
//!   [`ApproxAspectContour::model_contour`] pins the tokens. Note `mo'u` is
//!   treated as ZAhO `completive` per the repo dialect (it is dual-classed
//!   KOhA/ZAhO in the morphology table), NOT as an experimental KOhA.
//! - Quantity form spellings ([`ApproxQuantityForm`]) mirror the body
//!   notation's `QUANTITY FORM=` token shape (`Exact`, `All`, `AllBut`,
//!   `AtLeast`, `AtMost`, `TooFew`, `AlmostAll`, `Most`, `Many`, `Few`).
//! - Non-integer exact numbers (`pi`, `ce'i`) are represented as
//!   `Quantity { form, value: None, text: Some(..) }` where `text` is the
//!   decimal/percent rendering of the digit run (`pa pi re` → `"1.2"`,
//!   `pa re ce'i` → `"12%"`), mirroring the semantic model's
//!   `QuantityValue` integer/text duality. Valued quantifier forms without a
//!   digit tail default to 1 (CLL 18: `su'o`/`su'e`/`da'a` default to one);
//!   `mo'a` is given the same default. Unvalued forms (`ro`, `so'a`, `so'e`,
//!   `so'i`, `so'u`) reject any numeric tail (fail closed).
//!
//! Grouping/scope honesty (owner round 13, ruling 2): tree-level
//! `grouping`/`scope` by default; per-node escalation only when one tree
//! genuinely mixes explicit and assumed edges. A kind-composition edge is
//! `Explicit` when fixed by `bo`, when it joins an operand that is itself a
//! `ke...ke'e` group or `bo` chain, or when it is the sole edge of a
//! two-unit `ke...ke'e` group (that shape has no alternative); otherwise
//! `AssumedLeft`. A scope-bearing operator application (`se`/`na'e`/NU
//! families) is `Explicit` when its operand was delimited by `ke...ke'e` or
//! a `kei` closure, else `AssumedShort`. `grouping` exists exactly when the
//! tree has 3+ leaf components and at least one kind-composition edge whose
//! bases are uniform (a pure connective chain has no grouping to describe);
//! `scope` exists exactly when uniformly-based scope-bearing operators occur.

use std::collections::HashSet;

use bityzba::{data, invariant, new, requires};
use jbotci_dictionary::{Dictionary, DictionaryEntry, normalize_lookup_query};
use jbotci_jvozba::decompose_lujvo_like;
use jbotci_morphology::{
    Cmavo, LujvoPart, Selmaho, Word, WordKind, WordLike, WordLikeData, canonicalize_text,
    is_consonant, is_vowel, segment_words_with_modifiers,
};

use crate::model::{AbstractionKind, ActualityKind};

/// Warning on a successfully approximated dictionary-absent lujvo card.
pub const UNDEFINED_LUJVO_NONCE_WARNING: &str = "This compound is not in the dictionary. The composition shown is a mechanical approximation; its actual meaning and place structure are coiner-chosen.";
/// Warning on a dictionary-absent lujvo card whose parts fail closed.
pub const UNDEFINED_LUJVO_UNRECOVERABLE_WARNING: &str = "The parts of this undefined compound do not form a recoverable composition. No composition approximation is shown.";
/// Warning on a successfully approximated undefined zei compound card.
pub const ZEI_COMPOUND_NONCE_WARNING: &str = "This undefined multiword compound is nonce-coined. The composition shown is mechanical; its actual meaning and place structure are coiner-chosen.";
/// Warning on an undefined zei compound card whose parts fail closed.
pub const ZEI_COMPOUND_UNRECOVERABLE_WARNING: &str = "This undefined multiword compound has no structurally recoverable composition. No composition approximation is shown; its actual meaning and place structure are coiner-chosen.";

/// The morphological class a word card describes.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordCardKind {
    Gismu,
    Lujvo,
    Fuhivla,
    Cmevla,
    ZeiCompound,
}

impl WordCardKind {
    /// Whether this class is a compound eligible for a composition approximation.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_compound(self) -> bool {
        matches!(self, Self::Lujvo | Self::ZeiCompound)
    }
}

/// One structured word definition card (`<WORD>`) of the XML WORDS section.
#[invariant(!id.is_empty())]
#[invariant(!word.is_empty())]
#[invariant(glosses.iter().all(|gloss| !gloss.is_empty()))]
#[invariant(definition.as_ref().is_none_or(|definition| !definition.is_empty()))]
#[invariant(notes.as_ref().is_none_or(|notes| !notes.is_empty()))]
#[invariant(warnings.iter().all(|warning| !warning.is_empty()))]
#[invariant(
    composition.is_some() -> !known,
    "composition approximations exist only for dictionary-absent words"
)]
#[invariant(
    composition.is_some() -> !warnings.is_empty(),
    "a composition approximation always carries the nonce warning"
)]
#[invariant(
    (!known && kind.is_compound() && composition.is_none()) -> !warnings.is_empty(),
    "an unrecoverable compound composition must be disclosed by a warning"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCard {
    /// Card ID: the canonical spelling for one-token words; the hyphen-joined
    /// surface for zei compounds (`mi-zei-do`, `alis-zei-ninmu`; cmevla pause
    /// periods stripped). Hyphens cannot occur in Lojban words, so the two
    /// namespaces cannot collide.
    pub id: String,
    /// Canonical dictionary spelling; for zei compounds the space-separated
    /// surface (`mi zei do`), which is also the dictionary lookup text.
    pub word: String,
    pub kind: WordCardKind,
    /// False marks a dictionary-absent word (serialized as `KNOWN="false"`;
    /// the default true is elided by the emitter).
    pub known: bool,
    pub glosses: Vec<String>,
    /// Raw jbovlaste definition text with `$x_{1}$`-style place markers.
    pub definition: Option<String>,
    pub notes: Option<String>,
    pub composition: Option<CompositeApprox>,
    pub warnings: Vec<String>,
}

/// Basis of one kind-composition edge or of a whole tree's grouping.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupingBasis {
    /// Fixed by `bo`/`ke...ke'e` in the word itself.
    Explicit,
    /// Built with the CLL 5.3 default left-grouping rule because the word
    /// carries no grouping markers.
    AssumedLeft,
}

/// Basis of one scope-bearing operator application or of a whole tree's scope.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeBasis {
    /// An encoded boundary exists in the word (`kei` closure or an explicit
    /// `ke...ke'e` group around the operand).
    Explicit,
    /// Read by the CLL 12.12 default narrow-scope convention because the word
    /// carries no explicit scope marker.
    AssumedShort,
}

/// The `COMPOSITE-APPROX` content: a composition approximation tree plus the
/// tree-level honesty attributes. `PLACES="UNKNOWN"` is not modeled: it is a
/// constant required on every emission.
#[invariant(true, "audited no-op; the tree-level basis discipline is expensive-checked")]
#[expensive_invariant(
    grouping.is_some() == (approx_expr_leaf_count(root) >= 3
        && approx_expr_kind_composition_count(root) >= 1
        && approx_expr_kind_composition_groupings_all_none(root)),
    "GROUPING exists exactly for 3+ leaf components with uniformly-based kind-composition edges"
)]
#[expensive_invariant(
    scope.is_some() == (approx_expr_has_scope_bearing_nodes(root)
        && approx_expr_scopes_all_none(root)),
    "SCOPE exists exactly for uniformly-based scope-bearing operators"
)]
#[expensive_invariant(
    grouping_escalation_is_consistent(*grouping, root),
    "unmixed trees use the tree-level attribute; genuinely mixed trees escalate to per-node bases"
)]
#[expensive_invariant(
    scope_escalation_is_consistent(*scope, root),
    "unmixed trees use the tree-level attribute; genuinely mixed trees escalate to per-node bases"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeApprox {
    pub grouping: Option<GroupingBasis>,
    pub scope: Option<ScopeBasis>,
    pub root: ApproxExpr,
}

/// Polarity of a scalar (`NAhE`) negation.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarNegationPolarity {
    /// `na'e`: other than.
    Other,
    /// `no'e`: neutral midpoint.
    Neutral,
    /// `to'e`: opposite of.
    Opposite,
}

/// Logical and non-logical connective operators (`JA` and `JOI`/`BIhI`).
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproxConnective {
    /// `ja`.
    Or,
    /// `je`.
    And,
    /// `jo`.
    Iff,
    /// `ju`.
    WhetherOr,
    /// `joi`.
    Mass,
    /// `ce`.
    Set,
    /// `ce'o`.
    Sequence,
    /// `jo'e`.
    Union,
    /// `jo'u`.
    Joint,
    /// `ku'a`.
    Intersection,
    /// `pi'u`.
    CartesianProduct,
    /// `bi'i`.
    Interval,
}

/// Quantity forms, spelled to mirror the body notation's `QUANTITY FORM=` tokens.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproxQuantityForm {
    /// An exact digit run; carries `value` (integer) or `text` (non-integer).
    Exact,
    /// `ro`.
    All,
    /// `da'a`; value defaults to 1 when no digit tail follows.
    AllBut,
    /// `su'o`; value defaults to 1.
    AtLeast,
    /// `su'e`; value defaults to 1.
    AtMost,
    /// `mo'a`; value defaults to 1.
    TooFew,
    /// `so'a`.
    AlmostAll,
    /// `so'e`.
    Most,
    /// `so'i`.
    Many,
    /// `so'u`.
    Few,
}

/// Whether this form carries a value (`value` integer or `text` non-integer form).
#[requires(true)]
#[ensures(true)]
fn quantity_form_is_valued(form: ApproxQuantityForm) -> bool {
    matches!(
        form,
        ApproxQuantityForm::Exact
            | ApproxQuantityForm::AllBut
            | ApproxQuantityForm::AtLeast
            | ApproxQuantityForm::AtMost
            | ApproxQuantityForm::TooFew
    )
}

/// Aspect contours, aligned with the semantic model's `Aspect::contour` strings.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApproxAspectContour {
    /// `pu'o`.
    Prospective,
    /// `ca'o`.
    Continuative,
    /// `ba'o`.
    Retrospective,
    /// `co'a`.
    Initiative,
    /// `co'u`.
    Cessative,
    /// `mo'u`.
    Completive,
    /// `za'o`.
    Superfective,
    /// `co'i`.
    Achievative,
    /// `de'a`.
    Pausative,
    /// `di'a`.
    Resumptive,
}

impl ApproxAspectContour {
    /// The exact contour token the semantic model uses for this aspect
    /// (`generated_builder::tense_modal::aspect_contour_for_zaho_token`).
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn model_contour(self) -> &'static str {
        match self {
            Self::Prospective => "prospective",
            Self::Continuative => "continuative",
            Self::Retrospective => "retrospective",
            Self::Initiative => "initiative",
            Self::Cessative => "cessative",
            Self::Completive => "completive",
            Self::Superfective => "superfective",
            Self::Achievative => "achievative",
            Self::Pausative => "pausative",
            Self::Resumptive => "resumptive",
        }
    }
}

/// The abstract context role a pro-word denotes in discourse-free card scope.
/// These are roles, not referents; they define no ids.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableContextRole {
    /// `mi`.
    Speaker,
    /// `do`.
    Audience,
    /// `ti`/`ta`/`tu`; carries `proximity`.
    Demonstrated,
    /// `ko'a`...`fo'u`; carries `slot` 1–10.
    Assigned,
    /// `co'e`; the only role usable directly as a predicate leaf.
    EllipticalPredicate,
}

/// Deictic proximity of a demonstrated context role.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proximity {
    /// `ti`.
    Proximal,
    /// `ta`.
    Medial,
    /// `tu`.
    Distal,
}

/// Membership of one party in a personal mass.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inclusion {
    Included,
    Excluded,
}

/// Sort of a logical variable.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalVariableSort {
    /// `da`/`de`/`di`.
    Entity,
    /// `bu'a`/`bu'e`/`bu'i`.
    Predicate,
}

/// Role of a parameter referent.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterRole {
    /// `ce'u`.
    PropertySlot,
    /// `ma`.
    Argument,
}

/// A sumti-like component converted to a predicate by
/// [`ApproxExpr::ReferentOf`].
#[invariant(::Context { role, proximity, slot } =>
    matches!(*role, VariableContextRole::Demonstrated) == proximity.is_some()
        && matches!(*role, VariableContextRole::Assigned) == slot.is_some()
        && slot.is_none_or(|slot| (1..=10).contains(&slot)),
    "context roles carry exactly their role-specific data")]
#[invariant(::Named { text, .. } => !text.is_empty() && !text.contains('.'),
    "cmevla names are stripped of pause periods")]
#[invariant(::Unspecified => true)]
#[invariant(::PersonalMass { .. } => true)]
#[invariant(::LogicalVariable { series, .. } => (1..=3).contains(series))]
#[invariant(::Parameter { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApproxReferent {
    /// `mi`/`do`/`ti`/`ta`/`tu`/`ko'a`...`fo'u`.
    Context {
        role: VariableContextRole,
        proximity: Option<Proximity>,
        slot: Option<u8>,
    },
    /// A cmevla: the name stripped of pause periods, with the namer
    /// placeholder (`Context { Speaker }`) that discourse-free card scope
    /// requires.
    Named {
        text: String,
        by: Option<Box<ApproxReferent>>,
    },
    /// `zo'e`.
    Unspecified,
    /// `mi'o`/`mi'a`/`ma'a`/`do'o`.
    PersonalMass {
        speaker: Inclusion,
        audience: Inclusion,
        others: bool,
    },
    /// `da`/`de`/`di` and `bu'a`/`bu'e`/`bu'i`, series 1–3. Always
    /// implicitly existentially bound (serialized `BINDING` is constant).
    LogicalVariable { sort: LogicalVariableSort, series: u8 },
    /// `ce'u`, `ma`.
    Parameter { role: ParameterRole },
}

/// One node of a composition approximation tree. All structure is English
/// vocabulary; no cmavo surface ever appears in a tree.
#[invariant(::Component { word } => !word.is_empty())]
#[invariant(::KindComposition { .. } => true, "per-node escalation discipline is checked on CompositeApprox")]
#[invariant(::SwappedPlaces { first, second, .. } => *first == 1 && (2..=5).contains(second),
    "SE conversions swap place 1 with place 2-5")]
#[invariant(::ScalarNegation { .. } => true)]
#[invariant(::PredicationNegation { .. } => true)]
#[invariant(::Abstraction { .. } => true)]
#[invariant(::Connective { .. } => true)]
#[invariant(::TaggedPlace { .. } => true)]
#[invariant(::PlaceDeletion { index, .. } => *index > 0, "place indices are one-based")]
#[invariant(::Figurative { .. } => true)]
#[invariant(::Identity => true)]
#[invariant(::Quantity { form, value, text } =>
    (quantity_form_is_valued(*form) -> (value.is_some() != text.is_some()))
        && (!quantity_form_is_valued(*form) -> (value.is_none() && text.is_none()))
        && text.as_ref().is_none_or(|text| !text.is_empty()
            && text.chars().all(|character| character.is_ascii_digit() || character == '.' || character == '%')),
    "valued forms carry an integer value or a non-integer text form; unvalued forms carry neither")]
#[invariant(::Ordinal { .. } => true)]
#[invariant(::Cardinal { .. } => true)]
#[invariant(::Recurrence { .. } => true)]
#[invariant(::LetterOf { .. } => true)]
#[invariant(::Letter { text } => !text.is_empty())]
#[invariant(::TenseModal { actuality, aspect, space_whole, time_whole, .. } =>
    actuality.is_some() || aspect.is_some() || *space_whole || *time_whole,
    "a tense-modal wrapper carries at least one facet")]
#[invariant(::ReferentOf { .. } => true)]
#[invariant(::VariableContext { role, proximity, slot } =>
    matches!(*role, VariableContextRole::Demonstrated) == proximity.is_some()
        && matches!(*role, VariableContextRole::Assigned) == slot.is_some()
        && slot.is_none_or(|slot| (1..=10).contains(&slot)),
    "context roles carry exactly their role-specific data")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApproxExpr {
    /// IDREF to the component word's own card.
    Component { word: String },
    /// `kind` is the place-structure-bearing head (tertau, rightmost);
    /// `modifier` the kind-giving seltau. `grouping` is `None` unless the
    /// tree genuinely mixes explicit and assumed edges (per-node escalation).
    KindComposition {
        kind: Box<ApproxExpr>,
        modifier: Box<ApproxExpr>,
        grouping: Option<GroupingBasis>,
    },
    /// `se`/`te`/`ve`/`xe` place-map routing.
    SwappedPlaces {
        first: u8,
        second: u8,
        inner: Box<ApproxExpr>,
        scope: Option<ScopeBasis>,
    },
    /// `na'e`/`no'e`/`to'e`.
    ScalarNegation {
        polarity: ScalarNegationPolarity,
        inner: Box<ApproxExpr>,
        scope: Option<ScopeBasis>,
    },
    /// `na`; applies to the whole recovered predicate, never short-scope.
    PredicationNegation { inner: Box<ApproxExpr> },
    /// NU abstraction.
    Abstraction {
        kind: AbstractionKind,
        inner: Box<ApproxExpr>,
        scope: Option<ScopeBasis>,
    },
    /// JA/JOI/BIhI, infix and looser than juxtaposition.
    Connective {
        operator: ApproxConnective,
        left: Box<ApproxExpr>,
        right: Box<ApproxExpr>,
    },
    /// `jai`.
    TaggedPlace { inner: Box<ApproxExpr> },
    /// `zi'o`: deletion of the following predicate's first place.
    PlaceDeletion { index: u8, inner: Box<ApproxExpr> },
    /// `pe'a`.
    Figurative { inner: Box<ApproxExpr> },
    /// `du`, a leaf.
    Identity,
    /// A number: exact integers carry `value`; non-integer exact numbers
    /// (`pi`/`ce'i`) carry `text` (see module docs); quantifier forms per
    /// [`ApproxQuantityForm`].
    Quantity {
        form: ApproxQuantityForm,
        value: Option<i64>,
        text: Option<String>,
    },
    /// `moi` postfix on the preceding unit.
    Ordinal { inner: Box<ApproxExpr> },
    /// `mei` postfix on the preceding unit.
    Cardinal { inner: Box<ApproxExpr> },
    /// `roi` postfix on the preceding unit.
    Recurrence { inner: Box<ApproxExpr> },
    /// `bu` postfix on the preceding unit.
    LetterOf { inner: Box<ApproxExpr> },
    /// A lerfu/letteral leaf; `text` is the canonical lerfu word spelling.
    Letter { text: String },
    /// CAhA/ZAhO/ve'e/ze'e facets; consecutive facet prefixes merge into one
    /// node when their slots are disjoint.
    TenseModal {
        actuality: Option<ActualityKind>,
        aspect: Option<ApproxAspectContour>,
        space_whole: bool,
        time_whole: bool,
        inner: Box<ApproxExpr>,
    },
    /// A sumti-like component converted to a predicate.
    ReferentOf { referent: ApproxReferent },
    /// Direct predicate leaf, only for `co'e` (`EllipticalPredicate`).
    VariableContext {
        role: VariableContextRole,
        proximity: Option<Proximity>,
        slot: Option<u8>,
    },
}

// ---------------------------------------------------------------------------
// Tree inspection helpers (shared by the builder and the contract predicates)
// ---------------------------------------------------------------------------

/// Pre-order traversal of one approximation tree.
#[requires(true)]
#[ensures(true)]
fn visit_approx_expr(expr: &ApproxExpr, visitor: &mut impl FnMut(&ApproxExpr)) {
    visitor(expr);
    match expr.as_data() {
        data!(ApproxExpr::KindComposition { kind, modifier, .. }) => {
            visit_approx_expr(kind, visitor);
            visit_approx_expr(modifier, visitor);
        }
        data!(ApproxExpr::Connective { left, right, .. }) => {
            visit_approx_expr(left, visitor);
            visit_approx_expr(right, visitor);
        }
        data!(ApproxExpr::SwappedPlaces { inner, .. })
        | data!(ApproxExpr::ScalarNegation { inner, .. })
        | data!(ApproxExpr::PredicationNegation { inner })
        | data!(ApproxExpr::Abstraction { inner, .. })
        | data!(ApproxExpr::TaggedPlace { inner })
        | data!(ApproxExpr::PlaceDeletion { inner, .. })
        | data!(ApproxExpr::Figurative { inner })
        | data!(ApproxExpr::Ordinal { inner })
        | data!(ApproxExpr::Cardinal { inner })
        | data!(ApproxExpr::Recurrence { inner })
        | data!(ApproxExpr::LetterOf { inner })
        | data!(ApproxExpr::TenseModal { inner, .. }) => {
            visit_approx_expr(inner, visitor);
        }
        _ => {}
    }
}

/// Whether this expression variant is a leaf (no child expressions).
#[requires(true)]
#[ensures(true)]
fn approx_expr_is_leaf(expr: &ApproxExpr) -> bool {
    matches!(
        expr.as_data(),
        data!(ApproxExpr::Component { .. })
            | data!(ApproxExpr::Identity)
            | data!(ApproxExpr::Quantity { .. })
            | data!(ApproxExpr::Letter { .. })
            | data!(ApproxExpr::ReferentOf { .. })
            | data!(ApproxExpr::VariableContext { .. })
    )
}

#[requires(true)]
#[ensures(ret >= 1)]
fn approx_expr_leaf_count(root: &ApproxExpr) -> usize {
    let mut count = 0usize;
    visit_approx_expr(root, &mut |expr| {
        if approx_expr_is_leaf(expr) {
            count += 1;
        }
    });
    count
}

#[requires(true)]
#[ensures(true)]
fn approx_expr_kind_composition_count(root: &ApproxExpr) -> usize {
    let mut count = 0usize;
    visit_approx_expr(root, &mut |expr| {
        if matches!(expr.as_data(), data!(ApproxExpr::KindComposition { .. })) {
            count += 1;
        }
    });
    count
}

/// Whether every kind-composition node has its per-node grouping basis
/// cleared (the unmixed-tree state after tree-level attribution).
#[requires(true)]
#[ensures(true)]
fn approx_expr_kind_composition_groupings_all_none(root: &ApproxExpr) -> bool {
    let mut all_none = true;
    visit_approx_expr(root, &mut |expr| {
        if let data!(ApproxExpr::KindComposition {
            grouping: Some(_), ..
        }) = expr.as_data()
        {
            all_none = false;
        }
    });
    all_none
}

/// Whether every scope-bearing node has its per-node scope basis cleared.
#[requires(true)]
#[ensures(true)]
fn approx_expr_scopes_all_none(root: &ApproxExpr) -> bool {
    let mut all_none = true;
    visit_approx_expr(root, &mut |expr| {
        if matches!(approx_expr_scope_basis(expr), Some(Some(_))) {
            all_none = false;
        }
    });
    all_none
}

/// Whether the tree contains scope-bearing operator nodes
/// (`SwappedPlaces`/`ScalarNegation`/`Abstraction`).
#[requires(true)]
#[ensures(true)]
fn approx_expr_has_scope_bearing_nodes(root: &ApproxExpr) -> bool {
    let mut found = false;
    visit_approx_expr(root, &mut |expr| {
        if approx_expr_scope_basis(expr).is_some() {
            found = true;
        }
    });
    found
}

/// The per-node scope basis of a scope-bearing node, `None` for other nodes.
#[requires(true)]
#[ensures(true)]
fn approx_expr_scope_basis(expr: &ApproxExpr) -> Option<Option<ScopeBasis>> {
    match expr.as_data() {
        data!(ApproxExpr::SwappedPlaces { scope, .. })
        | data!(ApproxExpr::ScalarNegation { scope, .. })
        | data!(ApproxExpr::Abstraction { scope, .. }) => Some(*scope),
        _ => None,
    }
}

/// Contract predicate: per-node grouping escalation is consistent with the
/// tree-level attribute. Tree-level set ⇒ every per-node basis is cleared;
/// tree-level cleared ⇒ either everything is cleared (small or unmixed
/// trees) or every node carries a basis and both bases genuinely occur.
#[requires(true)]
#[ensures(true)]
fn grouping_escalation_is_consistent(grouping: Option<GroupingBasis>, root: &ApproxExpr) -> bool {
    let mut bases = Vec::new();
    visit_approx_expr(root, &mut |expr| {
        if let data!(ApproxExpr::KindComposition { grouping, .. }) = expr.as_data() {
            bases.push(*grouping);
        }
    });
    escalation_is_consistent(grouping.is_some(), &bases, GroupingBasis::Explicit)
}

/// Contract predicate: the scope counterpart of
/// [`grouping_escalation_is_consistent`].
#[requires(true)]
#[ensures(true)]
fn scope_escalation_is_consistent(scope: Option<ScopeBasis>, root: &ApproxExpr) -> bool {
    let mut bases = Vec::new();
    visit_approx_expr(root, &mut |expr| {
        if let Some(scope) = approx_expr_scope_basis(expr) {
            bases.push(scope);
        }
    });
    escalation_is_consistent(scope.is_some(), &bases, ScopeBasis::Explicit)
}

#[requires(true)]
#[ensures(true)]
fn escalation_is_consistent<B: Copy + PartialEq>(
    tree_level_set: bool,
    bases: &[Option<B>],
    explicit: B,
) -> bool {
    if tree_level_set {
        return bases.iter().all(Option::is_none);
    }
    if bases.iter().all(Option::is_none) {
        return true;
    }
    bases.iter().all(Option::is_some)
        && bases.iter().any(|basis| *basis == Some(explicit))
        && bases.iter().any(|basis| *basis != Some(explicit))
}

// ---------------------------------------------------------------------------
// Token machinery: classification, number reduction, grouping resolution
// ---------------------------------------------------------------------------

/// Prefix operators classified from the closed operator table. `Swap` place
/// validity is enforced by the validated [`ApproxExpr::SwappedPlaces`] node
/// built from it.
#[invariant(::Swap { .. } => true)]
#[invariant(::ScalarNegation(_) => true)]
#[invariant(::Abstraction(_) => true)]
#[invariant(::TaggedPlace => true)]
#[invariant(::PlaceDeletion => true)]
#[invariant(::Figurative => true)]
#[invariant(::Actuality(_) => true)]
#[invariant(::Aspect(_) => true)]
#[invariant(::SpaceWhole => true)]
#[invariant(::TimeWhole => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrefixOp {
    Swap { first: u8, second: u8 },
    ScalarNegation(ScalarNegationPolarity),
    Abstraction(AbstractionKind),
    TaggedPlace,
    PlaceDeletion,
    Figurative,
    Actuality(ActualityKind),
    Aspect(ApproxAspectContour),
    SpaceWhole,
    TimeWhole,
}

/// Postfix operators consuming the preceding unit.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PostfixOp {
    Ordinal,
    Cardinal,
    Recurrence,
    LetterOf,
}

/// One numeric token inside a number run.
#[invariant(::Digit(_) => true)]
#[invariant(::Point => true)]
#[invariant(::Percent => true)]
#[invariant(::Quantifier(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumTok {
    Digit(u8),
    /// `pi`: decimal point.
    Point,
    /// `ce'i`: percent.
    Percent,
    Quantifier(ApproxQuantityForm),
}

/// One classified stream element: a structural token or a numeric token
/// awaiting number reduction.
#[invariant(::Tok(_) => true)]
#[invariant(::Num(_) => true)]
#[derive(Debug)]
enum Piece {
    Tok(Tok),
    Num(NumTok),
}

/// Structural tokens after classification; `Group`/`ScopeGroup` appear only
/// after the grouping resolution passes (their non-emptiness is enforced by
/// the resolution passes that construct them).
#[invariant(::ScopeGroup { .. } => true)]
#[invariant(::Group(_) => true)]
#[invariant(::Expr(_) => true)]
#[invariant(::Prefix(_) => true)]
#[invariant(::Na => true)]
#[invariant(::Ke => true)]
#[invariant(::Kee => true)]
#[invariant(::Kei => true)]
#[invariant(::Bo => true)]
#[invariant(::Co => true)]
#[invariant(::Connective(_) => true)]
#[invariant(::Postfix(_) => true)]
#[derive(Debug)]
enum Tok {
    Expr(ApproxExpr),
    Prefix(PrefixOp),
    Na,
    Ke,
    Kee,
    Kei,
    Bo,
    Co,
    Connective(ApproxConnective),
    Postfix(PostfixOp),
    Group(Vec<Tok>),
    ScopeGroup {
        kind: AbstractionKind,
        inner: Vec<Tok>,
    },
}

/// Classify one cmavo against the closed operator table. Returns `None` for
/// every cmavo outside the table (fail closed): the spatial/motion shapes
/// (`mo'i`, `vi`/`va`/`vu`, `zo'a`/`zo'i`/`ze'o`), generics (`le'e`/`lo'e`),
/// anaphora (`ke'a`, `ri`/`ra`/`ru`, `vo'a`..`vo'u`, `go'a`..`go'u`,
/// `nei`/`no'a`, `da'e`/`da'u`/`de'e`/`de'u`/`dei`/`di'e`/`di'u`/`do'i`),
/// question/imperative pro-words (`mo`, `ko`), the typical-value `zu'i`, and
/// the experimental KOhA series are structurally unrenderable until the spec
/// pins their shapes. `mo'u` is deliberately NOT fail-closed: the repo
/// dialect treats it as ZAhO completive.
#[requires(true)]
#[ensures(true)]
fn classify_cmavo_piece(cmavo: Cmavo) -> Option<Piece> {
    let swap = |first: u8, second: u8| Piece::Tok(Tok::Prefix(PrefixOp::Swap { first, second }));
    let scalar = |polarity: ScalarNegationPolarity| {
        Piece::Tok(Tok::Prefix(PrefixOp::ScalarNegation(polarity)))
    };
    let abstraction = |kind: AbstractionKind| Piece::Tok(Tok::Prefix(PrefixOp::Abstraction(kind)));
    let prefix = |op: PrefixOp| Piece::Tok(Tok::Prefix(op));
    let connective = |operator: ApproxConnective| Piece::Tok(Tok::Connective(operator));
    let postfix = |op: PostfixOp| Piece::Tok(Tok::Postfix(op));
    let quantifier = |form: ApproxQuantityForm| Piece::Num(NumTok::Quantifier(form));
    let expr = |expr: ApproxExpr| Piece::Tok(Tok::Expr(expr));
    let context = |role: VariableContextRole, proximity: Option<Proximity>, slot: Option<u8>| {
        expr(new!(ApproxExpr::ReferentOf {
            referent: new!(ApproxReferent::Context {
                role: role,
                proximity: proximity,
                slot: slot,
            }),
        }))
    };
    Some(match cmavo {
        Cmavo::Se => swap(1, 2),
        Cmavo::Te => swap(1, 3),
        Cmavo::Ve => swap(1, 4),
        Cmavo::Xe => swap(1, 5),
        Cmavo::Nahe => scalar(ScalarNegationPolarity::Other),
        Cmavo::Nohe => scalar(ScalarNegationPolarity::Neutral),
        Cmavo::Tohe => scalar(ScalarNegationPolarity::Opposite),
        Cmavo::Nu => abstraction(AbstractionKind::Event),
        Cmavo::Muhe => abstraction(AbstractionKind::Achievement),
        Cmavo::Puhu => abstraction(AbstractionKind::Process),
        Cmavo::Zahi => abstraction(AbstractionKind::State),
        Cmavo::Zuho => abstraction(AbstractionKind::Activity),
        Cmavo::Ka => abstraction(AbstractionKind::Property),
        Cmavo::Ni => abstraction(AbstractionKind::Amount),
        Cmavo::Jei => abstraction(AbstractionKind::TruthValue),
        Cmavo::Siho => abstraction(AbstractionKind::Concept),
        Cmavo::Duhu => abstraction(AbstractionKind::Proposition),
        Cmavo::Lihi => abstraction(AbstractionKind::Experience),
        Cmavo::Suhu => abstraction(AbstractionKind::Unspecified),
        Cmavo::Na => Piece::Tok(Tok::Na),
        Cmavo::Ke => Piece::Tok(Tok::Ke),
        Cmavo::Kehe => Piece::Tok(Tok::Kee),
        Cmavo::Kei => Piece::Tok(Tok::Kei),
        Cmavo::Bo => Piece::Tok(Tok::Bo),
        Cmavo::Co => Piece::Tok(Tok::Co),
        Cmavo::Ja => connective(ApproxConnective::Or),
        Cmavo::Je => connective(ApproxConnective::And),
        Cmavo::Jo => connective(ApproxConnective::Iff),
        Cmavo::Ju => connective(ApproxConnective::WhetherOr),
        Cmavo::Joi => connective(ApproxConnective::Mass),
        Cmavo::Ce => connective(ApproxConnective::Set),
        Cmavo::Ceho => connective(ApproxConnective::Sequence),
        Cmavo::Johe => connective(ApproxConnective::Union),
        Cmavo::Johu => connective(ApproxConnective::Joint),
        Cmavo::Kuha => connective(ApproxConnective::Intersection),
        Cmavo::Pihu => connective(ApproxConnective::CartesianProduct),
        Cmavo::Bihi => connective(ApproxConnective::Interval),
        Cmavo::Jai => prefix(PrefixOp::TaggedPlace),
        Cmavo::Ziho => prefix(PrefixOp::PlaceDeletion),
        Cmavo::Peha => prefix(PrefixOp::Figurative),
        Cmavo::Du => expr(new!(ApproxExpr::Identity)),
        Cmavo::Pa => Piece::Num(NumTok::Digit(1)),
        Cmavo::Re => Piece::Num(NumTok::Digit(2)),
        Cmavo::Ci => Piece::Num(NumTok::Digit(3)),
        Cmavo::Vo => Piece::Num(NumTok::Digit(4)),
        Cmavo::Mu => Piece::Num(NumTok::Digit(5)),
        Cmavo::Xa => Piece::Num(NumTok::Digit(6)),
        Cmavo::Ze => Piece::Num(NumTok::Digit(7)),
        Cmavo::Bi => Piece::Num(NumTok::Digit(8)),
        Cmavo::So => Piece::Num(NumTok::Digit(9)),
        Cmavo::No => Piece::Num(NumTok::Digit(0)),
        Cmavo::Pi => Piece::Num(NumTok::Point),
        Cmavo::Cehi => Piece::Num(NumTok::Percent),
        Cmavo::Ro => quantifier(ApproxQuantityForm::All),
        Cmavo::Suho => quantifier(ApproxQuantityForm::AtLeast),
        Cmavo::Suhe => quantifier(ApproxQuantityForm::AtMost),
        Cmavo::Daha => quantifier(ApproxQuantityForm::AllBut),
        Cmavo::Moha => quantifier(ApproxQuantityForm::TooFew),
        Cmavo::Soha => quantifier(ApproxQuantityForm::AlmostAll),
        Cmavo::Sohe => quantifier(ApproxQuantityForm::Most),
        Cmavo::Sohi => quantifier(ApproxQuantityForm::Many),
        Cmavo::Sohu => quantifier(ApproxQuantityForm::Few),
        Cmavo::Moi => postfix(PostfixOp::Ordinal),
        Cmavo::Mei => postfix(PostfixOp::Cardinal),
        Cmavo::Roi => postfix(PostfixOp::Recurrence),
        Cmavo::Bu => postfix(PostfixOp::LetterOf),
        Cmavo::Caha => prefix(PrefixOp::Actuality(ActualityKind::Actual)),
        Cmavo::Kahe => prefix(PrefixOp::Actuality(ActualityKind::Capable)),
        Cmavo::Nuho => prefix(PrefixOp::Actuality(ActualityKind::Potential)),
        Cmavo::Puhi => prefix(PrefixOp::Actuality(ActualityKind::Demonstrated)),
        Cmavo::Puho => prefix(PrefixOp::Aspect(ApproxAspectContour::Prospective)),
        Cmavo::Caho => prefix(PrefixOp::Aspect(ApproxAspectContour::Continuative)),
        Cmavo::Baho => prefix(PrefixOp::Aspect(ApproxAspectContour::Retrospective)),
        Cmavo::Coha => prefix(PrefixOp::Aspect(ApproxAspectContour::Initiative)),
        Cmavo::Cohu => prefix(PrefixOp::Aspect(ApproxAspectContour::Cessative)),
        Cmavo::Mohu => prefix(PrefixOp::Aspect(ApproxAspectContour::Completive)),
        Cmavo::Zaho => prefix(PrefixOp::Aspect(ApproxAspectContour::Superfective)),
        Cmavo::Cohi => prefix(PrefixOp::Aspect(ApproxAspectContour::Achievative)),
        Cmavo::Deha => prefix(PrefixOp::Aspect(ApproxAspectContour::Pausative)),
        Cmavo::Diha => prefix(PrefixOp::Aspect(ApproxAspectContour::Resumptive)),
        Cmavo::Vehe => prefix(PrefixOp::SpaceWhole),
        Cmavo::Zehe => prefix(PrefixOp::TimeWhole),
        Cmavo::Mi => context(VariableContextRole::Speaker, None, None),
        Cmavo::Do => context(VariableContextRole::Audience, None, None),
        Cmavo::Ti => context(
            VariableContextRole::Demonstrated,
            Some(Proximity::Proximal),
            None,
        ),
        Cmavo::Ta => context(
            VariableContextRole::Demonstrated,
            Some(Proximity::Medial),
            None,
        ),
        Cmavo::Tu => context(
            VariableContextRole::Demonstrated,
            Some(Proximity::Distal),
            None,
        ),
        Cmavo::Koha => context(VariableContextRole::Assigned, None, Some(1)),
        Cmavo::Kohe => context(VariableContextRole::Assigned, None, Some(2)),
        Cmavo::Kohi => context(VariableContextRole::Assigned, None, Some(3)),
        Cmavo::Koho => context(VariableContextRole::Assigned, None, Some(4)),
        Cmavo::Kohu => context(VariableContextRole::Assigned, None, Some(5)),
        Cmavo::Foha => context(VariableContextRole::Assigned, None, Some(6)),
        Cmavo::Fohe => context(VariableContextRole::Assigned, None, Some(7)),
        Cmavo::Fohi => context(VariableContextRole::Assigned, None, Some(8)),
        Cmavo::Foho => context(VariableContextRole::Assigned, None, Some(9)),
        Cmavo::Fohu => context(VariableContextRole::Assigned, None, Some(10)),
        Cmavo::Zohe => expr(new!(ApproxExpr::ReferentOf {
            referent: new!(ApproxReferent::Unspecified),
        })),
        Cmavo::Miho => personal_mass(Inclusion::Included, Inclusion::Included, false),
        Cmavo::Miha => personal_mass(Inclusion::Included, Inclusion::Excluded, true),
        Cmavo::Maha => personal_mass(Inclusion::Included, Inclusion::Included, true),
        Cmavo::Doho => personal_mass(Inclusion::Excluded, Inclusion::Included, true),
        Cmavo::Da => logical_variable(LogicalVariableSort::Entity, 1),
        Cmavo::De => logical_variable(LogicalVariableSort::Entity, 2),
        Cmavo::Di => logical_variable(LogicalVariableSort::Entity, 3),
        Cmavo::Buha => logical_variable(LogicalVariableSort::Predicate, 1),
        Cmavo::Buhe => logical_variable(LogicalVariableSort::Predicate, 2),
        Cmavo::Buhi => logical_variable(LogicalVariableSort::Predicate, 3),
        Cmavo::Cehu => expr(new!(ApproxExpr::ReferentOf {
            referent: new!(ApproxReferent::Parameter {
                role: ParameterRole::PropertySlot,
            }),
        })),
        Cmavo::Ma => expr(new!(ApproxExpr::ReferentOf {
            referent: new!(ApproxReferent::Parameter {
                role: ParameterRole::Argument,
            }),
        })),
        Cmavo::Cohe => expr(new!(ApproxExpr::VariableContext {
            role: VariableContextRole::EllipticalPredicate,
            proximity: None,
            slot: None,
        })),
        Cmavo::Mohi
        | Cmavo::Vi
        | Cmavo::Va
        | Cmavo::Vu
        | Cmavo::Zoha
        | Cmavo::Zohi
        | Cmavo::Zeho
        | Cmavo::Lehe
        | Cmavo::Lohe
        | Cmavo::Keha
        | Cmavo::Ri
        | Cmavo::Ra
        | Cmavo::Ru
        | Cmavo::Voha
        | Cmavo::Vohe
        | Cmavo::Vohi
        | Cmavo::Voho
        | Cmavo::Vohu
        | Cmavo::Goha
        | Cmavo::Gohe
        | Cmavo::Gohi
        | Cmavo::Goho
        | Cmavo::Gohu
        | Cmavo::Nei
        | Cmavo::Noha
        | Cmavo::Mo
        | Cmavo::Ko
        | Cmavo::Zuhi
        | Cmavo::Dahe
        | Cmavo::Dahu
        | Cmavo::Dehe
        | Cmavo::Dehu
        | Cmavo::Dei
        | Cmavo::Dihe
        | Cmavo::Dihu
        | Cmavo::Dohi
        | Cmavo::Dahei
        | Cmavo::Deiha
        | Cmavo::Dihei
        | Cmavo::Fohai
        | Cmavo::Kihaha
        | Cmavo::Kiheha
        | Cmavo::Kihiha
        | Cmavo::Kihoha
        | Cmavo::Kihuha
        | Cmavo::Mahau
        | Cmavo::Mahei
        | Cmavo::Mahoi
        | Cmavo::Mihai
        | Cmavo::Mihau
        | Cmavo::Moho
        | Cmavo::Nauho
        | Cmavo::Nauhu
        | Cmavo::Rahai
        | Cmavo::Rauhi
        | Cmavo::Rohei
        | Cmavo::Sehe
        | Cmavo::Sohai
        | Cmavo::Tihau
        | Cmavo::Tohohe
        | Cmavo::Tuhau
        | Cmavo::Xai
        | Cmavo::Zohei
        | Cmavo::Zuhai => return None,
        _ if cmavo.is_selmaho(Selmaho::By) => expr(new!(ApproxExpr::Letter {
            text: cmavo.canonical_text().to_owned(),
        })),
        _ => return None,
    })
}

#[requires(true)]
#[ensures(true)]
fn personal_mass(speaker: Inclusion, audience: Inclusion, others: bool) -> Piece {
    Piece::Tok(Tok::Expr(new!(ApproxExpr::ReferentOf {
        referent: new!(ApproxReferent::PersonalMass {
            speaker: speaker,
            audience: audience,
            others: others,
        }),
    })))
}

#[requires((1..=3).contains(&series))]
#[ensures(true)]
fn logical_variable(sort: LogicalVariableSort, series: u8) -> Piece {
    Piece::Tok(Tok::Expr(new!(ApproxExpr::ReferentOf {
        referent: new!(ApproxReferent::LogicalVariable {
            sort: sort,
            series: series,
        }),
    })))
}

/// Number reduction: fold maximal runs of numeric tokens into single
/// `Quantity` units before any structural parsing.
#[requires(true)]
#[ensures(true)]
fn reduce_number_runs(pieces: Vec<Piece>) -> Option<Vec<Tok>> {
    let mut tokens = Vec::new();
    let mut run: Vec<NumTok> = Vec::new();
    let flush_run = |run: &mut Vec<NumTok>, tokens: &mut Vec<Tok>| -> Option<()> {
        if run.is_empty() {
            return Some(());
        }
        let quantity = reduce_number_run(run)?;
        run.clear();
        tokens.push(Tok::Expr(quantity));
        Some(())
    };
    for piece in pieces {
        match piece {
            Piece::Num(number) => run.push(number),
            Piece::Tok(token) => {
                flush_run(&mut run, &mut tokens)?;
                tokens.push(token);
            }
        }
    }
    flush_run(&mut run, &mut tokens)?;
    Some(tokens)
}

/// Reduce one maximal numeric run to one `Quantity` expression.
///
/// Shape: an optional leading quantifier cmavo, then digits with at most one
/// `pi` (decimal point) and at most one trailing `ce'i` (percent). A second
/// quantifier, a misplaced `pi`/`ce'i`, a digit tail on an unvalued
/// quantifier, or an overflowing digit run all fail closed.
#[requires(!run.is_empty())]
#[ensures(ret.as_ref().is_some_and(|expr| matches!(expr.as_data(), data!(ApproxExpr::Quantity { .. }))) || ret.is_none())]
fn reduce_number_run(run: &[NumTok]) -> Option<ApproxExpr> {
    let (form, tail) = match run.first() {
        Some(NumTok::Quantifier(form)) => (Some(*form), &run[1..]),
        _ => (None, run),
    };
    if tail.iter().any(|token| matches!(token, NumTok::Quantifier(_))) {
        return None;
    }
    let mut integer: i64 = 0;
    let mut digit_count = 0usize;
    let mut before_point = String::new();
    let mut after_point = String::new();
    let mut point_seen = false;
    let mut percent_seen = false;
    for (index, token) in tail.iter().enumerate() {
        match token {
            NumTok::Digit(digit) => {
                digit_count += 1;
                integer = integer.checked_mul(10)?.checked_add(i64::from(*digit))?;
                let digits = if point_seen {
                    &mut after_point
                } else {
                    &mut before_point
                };
                digits.push(char::from(b'0' + *digit));
            }
            NumTok::Point => {
                if point_seen || percent_seen {
                    return None;
                }
                point_seen = true;
            }
            NumTok::Percent => {
                if percent_seen || index + 1 != tail.len() {
                    return None;
                }
                percent_seen = true;
            }
            NumTok::Quantifier(_) => unreachable!("a second quantifier was rejected above"),
        }
    }
    if point_seen || percent_seen {
        // Non-integer exact number: the decimal/percent text form.
        if digit_count == 0 {
            return None;
        }
        let form = form.unwrap_or(ApproxQuantityForm::Exact);
        if !quantity_form_is_valued(form) {
            return None;
        }
        let mut text = before_point;
        if point_seen {
            text.push('.');
            text.push_str(&after_point);
        }
        if percent_seen {
            text.push('%');
        }
        return Some(new!(ApproxExpr::Quantity {
            form: form,
            value: None,
            text: Some(text),
        }));
    }
    match form {
        None => {
            if digit_count == 0 {
                return None;
            }
            Some(new!(ApproxExpr::Quantity {
                form: ApproxQuantityForm::Exact,
                value: Some(integer),
                text: None,
            }))
        }
        Some(form) if quantity_form_is_valued(form) => {
            // CLL 18: su'o/su'e/da'a without a digit tail default to one;
            // mo'a is given the same default.
            Some(new!(ApproxExpr::Quantity {
                form: form,
                value: Some(if digit_count == 0 { 1 } else { integer }),
                text: None,
            }))
        }
        Some(form) => {
            if digit_count > 0 {
                return None;
            }
            Some(new!(ApproxExpr::Quantity {
                form: form,
                value: None,
                text: None,
            }))
        }
    }
}

/// Match `ke...ke'e` pairs (nestable) into `Group` tokens. Any unmatched
/// delimiter fails closed.
#[requires(true)]
#[ensures(true)]
fn group_ke_tokens(input: Vec<Tok>) -> Option<Vec<Tok>> {
    let mut stack: Vec<Vec<Tok>> = vec![Vec::new()];
    for token in input {
        match token {
            Tok::Ke => stack.push(Vec::new()),
            Tok::Kee => {
                if stack.len() < 2 {
                    return None;
                }
                let inner = stack.pop().expect("the length check guarantees a frame");
                if inner.is_empty() {
                    return None;
                }
                stack
                    .last_mut()
                    .expect("the length check guarantees a parent frame")
                    .push(Tok::Group(inner));
            }
            other => stack
                .last_mut()
                .expect("the base frame always exists")
                .push(other),
        }
    }
    if stack.len() != 1 {
        return None;
    }
    stack.pop()
}

/// Resolve `kei` closures: each `kei` closes the innermost open NU prefix,
/// extending its scope over the maximal preceding run of tokens since that
/// prefix. An unmatched `kei` or an empty scope fails closed.
#[requires(true)]
#[ensures(ret.is_some() -> true)]
fn resolve_kei_scopes(tokens: Vec<Tok>) -> Option<Vec<Tok>> {
    let mut resolved = Vec::with_capacity(tokens.len());
    for token in tokens {
        resolved.push(match token {
            Tok::Group(inner) => Tok::Group(resolve_kei_scopes(inner)?),
            other => other,
        });
    }
    let mut output: Vec<Tok> = Vec::new();
    let mut open_abstractions: Vec<usize> = Vec::new();
    for token in resolved {
        match token {
            Tok::Prefix(PrefixOp::Abstraction(..)) => {
                open_abstractions.push(output.len());
                output.push(token);
            }
            Tok::Kei => {
                let position = open_abstractions.pop()?;
                let mut drained = output.drain(position..);
                let Some(Tok::Prefix(PrefixOp::Abstraction(kind))) = drained.next() else {
                    unreachable!("the recorded position holds an abstraction prefix");
                };
                let inner: Vec<Tok> = drained.collect();
                if inner.is_empty() {
                    return None;
                }
                output.push(Tok::ScopeGroup { kind, inner });
            }
            other => output.push(other),
        }
    }
    Some(output)
}

// ---------------------------------------------------------------------------
// Composition parser: connectives > co > juxtaposition > bo > prefix/postfix
// ---------------------------------------------------------------------------

/// Whether a parsed unit's outer boundary is fixed by an explicit grouping
/// construct, which makes a juxtaposition edge touching it explicit.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnitBoundary {
    None,
    KeGroup,
    BoChain,
}

/// Build one composition tree from a classified token stream, or fail closed.
#[requires(true)]
#[ensures(true)]
fn build_composition(pieces: Vec<Piece>) -> Option<CompositeApprox> {
    let tokens = reduce_number_runs(pieces)?;
    let tokens = group_ke_tokens(tokens)?;
    let tokens = resolve_kei_scopes(tokens)?;
    let references: Vec<&Tok> = tokens.iter().collect();
    let root = parse_sequence(&references, false)?;
    Some(finalize_composition(root))
}

/// Decide the tree-level honesty attributes and strip or keep per-node bases.
#[requires(true)]
#[ensures(true)]
fn finalize_composition(root: ApproxExpr) -> CompositeApprox {
    let leaves = approx_expr_leaf_count(&root);
    let mut grouping_bases = Vec::new();
    visit_approx_expr(&root, &mut |expr| {
        if let data!(ApproxExpr::KindComposition { grouping, .. }) = expr.as_data() {
            grouping_bases.push(*grouping);
        }
    });
    let mut scope_bases = Vec::new();
    visit_approx_expr(&root, &mut |expr| {
        if let Some(scope) = approx_expr_scope_basis(expr) {
            scope_bases.push(scope);
        }
    });
    let grouping = if leaves >= 3 && !grouping_bases.is_empty() {
        tree_level_basis(&grouping_bases)
    } else {
        None
    };
    let scope = if scope_bases.is_empty() {
        None
    } else {
        tree_level_basis(&scope_bases)
    };
    let strip_grouping = grouping.is_some() || grouping_bases.is_empty() || leaves < 3;
    let strip_scope = scope.is_some() || scope_bases.is_empty();
    let root = normalize_approx_bases(root, strip_grouping, strip_scope);
    new!(CompositeApprox {
        grouping: grouping,
        scope: scope,
        root: root,
    })
}

/// The tree-level basis of a uniformly-based node set, `None` when mixed.
#[requires(!bases.is_empty())]
#[ensures(true)]
fn tree_level_basis<B: Copy + PartialEq>(bases: &[Option<B>]) -> Option<B> {
    let first = (*bases.first().expect("bases are non-empty by precondition"))?;
    if bases.iter().all(|basis| *basis == Some(first)) {
        Some(first)
    } else {
        None
    }
}

/// Rewrite the tree so per-node bases are present exactly under mixed-tree
/// escalation (they are built as `Some` everywhere during parsing).
#[requires(true)]
#[ensures(true)]
fn normalize_approx_bases(expr: ApproxExpr, strip_grouping: bool, strip_scope: bool) -> ApproxExpr {
    match expr.into_data() {
        data!(ApproxExpr::KindComposition {
            kind,
            modifier,
            grouping,
        }) => new!(ApproxExpr::KindComposition {
            kind: Box::new(normalize_approx_bases(*kind, strip_grouping, strip_scope)),
            modifier: Box::new(normalize_approx_bases(*modifier, strip_grouping, strip_scope)),
            grouping: if strip_grouping { None } else { grouping },
        }),
        data!(ApproxExpr::SwappedPlaces {
            first,
            second,
            inner,
            scope,
        }) => new!(ApproxExpr::SwappedPlaces {
            first: first,
            second: second,
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
            scope: if strip_scope { None } else { scope },
        }),
        data!(ApproxExpr::ScalarNegation {
            polarity,
            inner,
            scope,
        }) => new!(ApproxExpr::ScalarNegation {
            polarity: polarity,
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
            scope: if strip_scope { None } else { scope },
        }),
        data!(ApproxExpr::Abstraction { kind, inner, scope }) => {
            new!(ApproxExpr::Abstraction {
                kind: kind,
                inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
                scope: if strip_scope { None } else { scope },
            })
        }
        data!(ApproxExpr::Connective {
            operator,
            left,
            right,
        }) => new!(ApproxExpr::Connective {
            operator: operator,
            left: Box::new(normalize_approx_bases(*left, strip_grouping, strip_scope)),
            right: Box::new(normalize_approx_bases(*right, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::PredicationNegation { inner }) => {
            new!(ApproxExpr::PredicationNegation {
                inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
            })
        }
        data!(ApproxExpr::TaggedPlace { inner }) => new!(ApproxExpr::TaggedPlace {
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::PlaceDeletion { index, inner }) => {
            new!(ApproxExpr::PlaceDeletion {
                index: index,
                inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
            })
        }
        data!(ApproxExpr::Figurative { inner }) => new!(ApproxExpr::Figurative {
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::Ordinal { inner }) => new!(ApproxExpr::Ordinal {
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::Cardinal { inner }) => new!(ApproxExpr::Cardinal {
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::Recurrence { inner }) => new!(ApproxExpr::Recurrence {
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::LetterOf { inner }) => new!(ApproxExpr::LetterOf {
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        data!(ApproxExpr::TenseModal {
            actuality,
            aspect,
            space_whole,
            time_whole,
            inner,
        }) => new!(ApproxExpr::TenseModal {
            actuality: actuality,
            aspect: aspect,
            space_whole: space_whole,
            time_whole: time_whole,
            inner: Box::new(normalize_approx_bases(*inner, strip_grouping, strip_scope)),
        }),
        leaf => ApproxExpr::from_data(leaf),
    }
}

/// Parse one token sequence: `na` wraps the whole resulting predicate and
/// never scopes narrowly. `ke_group_content` marks the sequence as the
/// content of a `ke...ke'e` group free of connectives and `co`.
#[requires(true)]
#[ensures(true)]
fn parse_sequence(tokens: &[&Tok], ke_group_content: bool) -> Option<ApproxExpr> {
    let mut na_count = 0usize;
    let mut rest = Vec::with_capacity(tokens.len());
    for token in tokens {
        if matches!(token, Tok::Na) {
            na_count += 1;
        } else {
            rest.push(*token);
        }
    }
    let mut expr = parse_connectives(&rest, ke_group_content)?;
    for _ in 0..na_count {
        expr = new!(ApproxExpr::PredicationNegation {
            inner: Box::new(expr),
        });
    }
    Some(expr)
}

/// Split on top-level connectives (looser than everything else) and join the
/// segments left-grouped.
#[requires(true)]
#[ensures(true)]
fn parse_connectives(tokens: &[&Tok], ke_group_content: bool) -> Option<ApproxExpr> {
    let mut operators = Vec::new();
    let mut segments: Vec<&[&Tok]> = Vec::new();
    let mut start = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if let Tok::Connective(operator) = token {
            operators.push(*operator);
            segments.push(&tokens[start..index]);
            start = index + 1;
        }
    }
    segments.push(&tokens[start..]);
    // A connective chain is never a simple two-unit ke group.
    let inner_ke_group = ke_group_content && operators.is_empty();
    let mut expr = parse_co_expression(segments.first()?, inner_ke_group)?;
    for (operator, segment) in operators.into_iter().zip(segments.iter().skip(1)) {
        let right = parse_co_expression(segment, false)?;
        expr = new!(ApproxExpr::Connective {
            operator: operator,
            left: Box::new(expr),
            right: Box::new(right),
        });
    }
    Some(expr)
}

/// Split on top-level `co`. `co` is right-recursive (EBNF `selbri-2`):
/// `X co REST` is `KindComposition { kind: REST, modifier: X }`, and the
/// `co` token itself leaves no node (CLL 5.8 mechanical normalization).
#[requires(true)]
#[ensures(true)]
fn parse_co_expression(tokens: &[&Tok], ke_group_content: bool) -> Option<ApproxExpr> {
    let mut segments: Vec<&[&Tok]> = Vec::new();
    let mut start = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token, Tok::Co) {
            segments.push(&tokens[start..index]);
            start = index + 1;
        }
    }
    segments.push(&tokens[start..]);
    let inner_ke_group = ke_group_content && segments.len() == 1;
    let mut kind = parse_juxtaposition(segments.last()?, inner_ke_group)?.0;
    for segment in segments[..segments.len() - 1].iter().rev() {
        let (modifier, _) = parse_juxtaposition(segment, false)?;
        kind = new!(ApproxExpr::KindComposition {
            kind: Box::new(kind),
            modifier: Box::new(modifier),
            grouping: Some(GroupingBasis::AssumedLeft),
        });
    }
    Some(kind)
}

/// Parse a run of juxtaposed units into a left-grouped kind-composition
/// tree; the kind child is the rightmost unit (the head).
#[requires(true)]
#[ensures(true)]
fn parse_juxtaposition(
    tokens: &[&Tok],
    ke_group_content: bool,
) -> Option<(ApproxExpr, UnitBoundary)> {
    let mut units = Vec::new();
    let mut position = 0usize;
    while position < tokens.len() {
        let (unit, boundary, next) = parse_bo_chain(tokens, position)?;
        units.push((unit, boundary));
        position = next;
    }
    let total = units.len();
    let mut iter = units.into_iter();
    let (mut acc, mut acc_boundary) = iter.next()?;
    for (unit, unit_boundary) in iter {
        let basis = if ke_group_content
            && total == 2
            && acc_boundary == UnitBoundary::None
            && unit_boundary == UnitBoundary::None
        {
            // A two-unit ke...ke'e group has only one possible shape, so the
            // delimiters fix the whole tree.
            GroupingBasis::Explicit
        } else {
            juxtaposition_edge_basis(acc_boundary, unit_boundary)
        };
        acc = new!(ApproxExpr::KindComposition {
            kind: Box::new(unit),
            modifier: Box::new(acc),
            grouping: Some(basis),
        });
        acc_boundary = UnitBoundary::None;
    }
    Some((acc, acc_boundary))
}

#[requires(true)]
#[ensures(true)]
fn juxtaposition_edge_basis(left: UnitBoundary, right: UnitBoundary) -> GroupingBasis {
    if left == UnitBoundary::None && right == UnitBoundary::None {
        GroupingBasis::AssumedLeft
    } else {
        GroupingBasis::Explicit
    }
}

/// Parse one `bo` chain (right-recursive, EBNF `selbri-6`); every `bo` edge
/// is explicitly grouped.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, _, next)| *next > start))]
fn parse_bo_chain(
    tokens: &[&Tok],
    start: usize,
) -> Option<(ApproxExpr, UnitBoundary, usize)> {
    let (first, first_boundary, mut position) = parse_unit(tokens, start)?;
    if !matches!(tokens.get(position), Some(Tok::Bo)) {
        return Some((first, first_boundary, position));
    }
    let mut units = vec![first];
    while matches!(tokens.get(position), Some(Tok::Bo)) {
        position += 1;
        let (unit, _, next) = parse_unit(tokens, position)?;
        units.push(unit);
        position = next;
    }
    let mut kind = units.pop().expect("a bo chain holds at least two units");
    for modifier in units.into_iter().rev() {
        kind = new!(ApproxExpr::KindComposition {
            kind: Box::new(kind),
            modifier: Box::new(modifier),
            grouping: Some(GroupingBasis::Explicit),
        });
    }
    Some((kind, UnitBoundary::BoChain, position))
}

/// Parse one tight unit: prefix operators (short scope, CLL 12.12) around
/// the following unit or explicit group, postfix operators binding tighter
/// than prefixes (EBNF `tanru-unit-2`).
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, _, next)| *next > position))]
fn parse_unit(tokens: &[&Tok], position: usize) -> Option<(ApproxExpr, UnitBoundary, usize)> {
    let token = tokens.get(position)?;
    let (mut expr, boundary, mut position) = match token {
        Tok::Expr(leaf) => (leaf.clone(), UnitBoundary::None, position + 1),
        Tok::Group(inner) => {
            let references: Vec<&Tok> = inner.iter().collect();
            let simple = !inner
                .iter()
                .any(|token| matches!(token, Tok::Connective(..) | Tok::Co));
            (
                parse_sequence(&references, simple)?,
                UnitBoundary::KeGroup,
                position + 1,
            )
        }
        Tok::ScopeGroup { kind, inner } => {
            let references: Vec<&Tok> = inner.iter().collect();
            (
                new!(ApproxExpr::Abstraction {
                    kind: *kind,
                    inner: Box::new(parse_sequence(&references, false)?),
                    scope: Some(ScopeBasis::Explicit),
                }),
                UnitBoundary::None,
                position + 1,
            )
        }
        Tok::Prefix(op) => {
            let (operand, operand_boundary, next) = parse_unit(tokens, position + 1)?;
            (
                apply_prefix_operator(*op, operand, operand_boundary),
                UnitBoundary::None,
                next,
            )
        }
        Tok::Na
        | Tok::Ke
        | Tok::Kee
        | Tok::Kei
        | Tok::Bo
        | Tok::Co
        | Tok::Connective(..)
        | Tok::Postfix(..) => return None,
    };
    while let Some(Tok::Postfix(op)) = tokens.get(position) {
        expr = apply_postfix_operator(*op, expr);
        position += 1;
    }
    Some((expr, boundary, position))
}

#[requires(true)]
#[ensures(true)]
fn apply_prefix_operator(
    op: PrefixOp,
    operand: ApproxExpr,
    operand_boundary: UnitBoundary,
) -> ApproxExpr {
    let scope = Some(if operand_boundary == UnitBoundary::KeGroup {
        ScopeBasis::Explicit
    } else {
        ScopeBasis::AssumedShort
    });
    match op {
        PrefixOp::Swap { first, second } => new!(ApproxExpr::SwappedPlaces {
            first: first,
            second: second,
            inner: Box::new(operand),
            scope: scope,
        }),
        PrefixOp::ScalarNegation(polarity) => new!(ApproxExpr::ScalarNegation {
            polarity: polarity,
            inner: Box::new(operand),
            scope: scope,
        }),
        PrefixOp::Abstraction(kind) => new!(ApproxExpr::Abstraction {
            kind: kind,
            inner: Box::new(operand),
            scope: scope,
        }),
        PrefixOp::TaggedPlace => new!(ApproxExpr::TaggedPlace {
            inner: Box::new(operand),
        }),
        PrefixOp::PlaceDeletion => new!(ApproxExpr::PlaceDeletion {
            index: 1,
            inner: Box::new(operand),
        }),
        PrefixOp::Figurative => new!(ApproxExpr::Figurative {
            inner: Box::new(operand),
        }),
        PrefixOp::Actuality(actuality) => {
            apply_tense_modal_facet(operand, Some(actuality), None, false, false)
        }
        PrefixOp::Aspect(aspect) => {
            apply_tense_modal_facet(operand, None, Some(aspect), false, false)
        }
        PrefixOp::SpaceWhole => apply_tense_modal_facet(operand, None, None, true, false),
        PrefixOp::TimeWhole => apply_tense_modal_facet(operand, None, None, false, true),
    }
}

/// Apply one tense-modal facet, merging into an existing `TenseModal` operand
/// when the touched slots are free (consecutive facet prefixes form one node).
#[requires(
    actuality.is_some() as usize + aspect.is_some() as usize + space_whole as usize + time_whole as usize == 1
)]
#[ensures(matches!(ret.as_data(), data!(ApproxExpr::TenseModal { .. })))]
fn apply_tense_modal_facet(
    operand: ApproxExpr,
    actuality: Option<ActualityKind>,
    aspect: Option<ApproxAspectContour>,
    space_whole: bool,
    time_whole: bool,
) -> ApproxExpr {
    let can_merge = matches!(
        operand.as_data(),
        data!(ApproxExpr::TenseModal {
            actuality: existing_actuality,
            aspect: existing_aspect,
            space_whole: existing_space,
            time_whole: existing_time,
            ..
        }) if (actuality.is_none() || existing_actuality.is_none())
            && (aspect.is_none() || existing_aspect.is_none())
            && (!space_whole || !existing_space)
            && (!time_whole || !existing_time)
    );
    if can_merge {
        let data!(ApproxExpr::TenseModal {
            actuality: existing_actuality,
            aspect: existing_aspect,
            space_whole: existing_space,
            time_whole: existing_time,
            inner,
        }) = operand.into_data()
        else {
            unreachable!("mergeability was just observed on the same value");
        };
        return new!(ApproxExpr::TenseModal {
            actuality: actuality.or(existing_actuality),
            aspect: aspect.or(existing_aspect),
            space_whole: space_whole || existing_space,
            time_whole: time_whole || existing_time,
            inner: inner,
        });
    }
    new!(ApproxExpr::TenseModal {
        actuality: actuality,
        aspect: aspect,
        space_whole: space_whole,
        time_whole: time_whole,
        inner: Box::new(operand),
    })
}

#[requires(true)]
#[ensures(true)]
fn apply_postfix_operator(op: PostfixOp, inner: ApproxExpr) -> ApproxExpr {
    match op {
        PostfixOp::Ordinal => new!(ApproxExpr::Ordinal {
            inner: Box::new(inner),
        }),
        PostfixOp::Cardinal => new!(ApproxExpr::Cardinal {
            inner: Box::new(inner),
        }),
        PostfixOp::Recurrence => new!(ApproxExpr::Recurrence {
            inner: Box::new(inner),
        }),
        PostfixOp::LetterOf => new!(ApproxExpr::LetterOf {
            inner: Box::new(inner),
        }),
    }
}

// ---------------------------------------------------------------------------
// Card builder
// ---------------------------------------------------------------------------

/// Build the structured word cards for the content words of one text.
///
/// Selection mirrors the owner doctrine in `jbotci-search`'s `vlacku.rs`
/// (`push_content_dictionary_lookup_targets`, re-implemented here on the
/// morphology types to avoid a semantics→search dependency): cmavo never get
/// cards; brivla do; cmevla only with an exact dictionary entry; zei
/// compounds do; `zo`/`ma'oi`/`lo'u` quotes define the referenced words;
/// lerfu words and `zoi` quotes never do. Cards are deduplicated by ID,
/// first occurrence winning, and every component referenced by a composition
/// gets its own card appended after the compound's card.
#[requires(true)]
#[expensive_ensures(ret.iter().enumerate().all(|(index, card)| {
    ret[..index].iter().all(|earlier| earlier.id != card.id)
}), "card IDs are unique")]
pub fn build_xml_word_cards(dictionary: &Dictionary<'_>, words: &[WordLike]) -> Vec<WordCard> {
    let mut builder = WordCardBuilder {
        dictionary,
        cards: Vec::new(),
        built_ids: HashSet::new(),
    };
    for word_like in words {
        builder.push_word_like_cards(word_like);
    }
    builder.cards
}

#[invariant(true)]
struct WordCardBuilder<'dict> {
    dictionary: &'dict Dictionary<'dict>,
    cards: Vec<WordCard>,
    built_ids: HashSet<String>,
}

impl<'dict> WordCardBuilder<'dict> {
    #[requires(true)]
    #[ensures(true)]
    fn push_word_like_cards(&mut self, word_like: &WordLike) {
        match word_like.as_data() {
            data!(WordLike::PlainWord(word)) => self.push_content_word_cards(word),
            // The quoted word is the referenced content; the `zo`/`ma'oi`
            // quote markers are not.
            data!(WordLike::QuotedWord { word, .. }) => self.push_content_word_cards(word),
            data!(WordLike::SelmahoQuotedWord { word, .. }) => self.push_content_word_cards(word),
            data!(WordLike::QuotedWords { quoted_words, .. }) => {
                for word in quoted_words {
                    self.push_content_word_cards(word);
                }
            }
            // `zoi` quotes non-Lojban text (no dictionary entry by
            // construction); the remaining delimited quotes reference no
            // defined word; a lerfu word is a BY letteral, hence a cmavo by
            // morphology.
            data!(WordLike::DelimitedNonLojbanQuote { .. })
            | data!(WordLike::DelimitedWordQuote { .. })
            | data!(WordLike::LerfuWord { .. }) => {}
            data!(WordLike::ZeiCompound { .. }) => self.push_zei_compound_card(word_like),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_content_word_cards(&mut self, word: &Word) {
        match word.kind() {
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => {
                let kind = word_card_kind_for_brivla(word.kind());
                self.push_brivla_card(canonicalize_text(word.phonemes().as_str()), kind);
            }
            WordKind::Cmevla => {
                // A morphology-valid cmevla is content only when the
                // dictionary has an exact entry.
                let word_text = canonicalize_text(word.phonemes().as_str());
                if self.dictionary.lookup_words(&word_text).next().is_some() {
                    self.push_brivla_card(word_text, WordCardKind::Cmevla);
                }
            }
            WordKind::Cmavo => {}
        }
    }

    /// Build (or skip, when already built) the card for one brivla. The card
    /// is registered before component recursion, which is the cycle guard:
    /// every recursive reference to an already-built or in-progress word
    /// stops at the registry. (A true reference cycle is not constructible
    /// through `decompose_lujvo_like` — component sources are dictionary
    /// words referenced by the compound's own rafsi — the guard is
    /// defense-in-depth, not a load-bearing assumption.)
    #[requires(!word_text.is_empty())]
    #[ensures(true)]
    fn push_brivla_card(&mut self, word_text: String, kind: WordCardKind) {
        if self.built_ids.contains(&word_text) {
            return;
        }
        let entries = self.dictionary.lookup_words(&word_text).collect::<Vec<_>>();
        if let Some(entry) = entries
            .iter()
            .copied()
            .find(|entry| !entry.definition.is_empty())
        {
            self.built_ids.insert(word_text.clone());
            self.cards.push(known_word_card(word_text.clone(), word_text, kind, entry));
            return;
        }
        match kind {
            WordCardKind::Lujvo => self.push_unknown_lujvo_card(word_text),
            WordCardKind::Gismu | WordCardKind::Fuhivla | WordCardKind::Cmevla => {
                self.built_ids.insert(word_text.clone());
                self.cards.push(new!(WordCard {
                    id: word_text.clone(),
                    word: word_text,
                    kind: kind,
                    known: false,
                    glosses: Vec::new(),
                    definition: None,
                    notes: None,
                    composition: None,
                    warnings: Vec::new(),
                }));
            }
            WordCardKind::ZeiCompound => {
                unreachable!("zei compounds are built by the zei pipeline")
            }
        }
    }

    #[requires(!word_text.is_empty())]
    #[ensures(true)]
    fn push_unknown_lujvo_card(&mut self, word_text: String) {
        let composed = self.compose_lujvo(&word_text);
        match composed {
            Some((composition, components)) => {
                self.built_ids.insert(word_text.clone());
                self.cards.push(new!(WordCard {
                    id: word_text.clone(),
                    word: word_text,
                    kind: WordCardKind::Lujvo,
                    known: false,
                    glosses: Vec::new(),
                    definition: None,
                    notes: None,
                    composition: Some(composition),
                    warnings: vec![UNDEFINED_LUJVO_NONCE_WARNING.to_owned()],
                }));
                for (component, kind) in components {
                    self.push_brivla_card(component, kind);
                }
            }
            None => {
                self.built_ids.insert(word_text.clone());
                self.cards.push(new!(WordCard {
                    id: word_text.clone(),
                    word: word_text,
                    kind: WordCardKind::Lujvo,
                    known: false,
                    glosses: Vec::new(),
                    definition: None,
                    notes: None,
                    composition: None,
                    warnings: vec![UNDEFINED_LUJVO_UNRECOVERABLE_WARNING.to_owned()],
                }));
            }
        }
    }

    /// Decompose one dictionary-absent lujvo and build its composition tree,
    /// collecting the referenced brivla components in stream order.
    #[requires(!word_text.is_empty())]
    #[ensures(true)]
    fn compose_lujvo(&self, word_text: &str) -> Option<(CompositeApprox, Vec<(String, WordCardKind)>)> {
        let decomposition = decompose_lujvo_like(self.dictionary, word_text)?;
        let mut pieces = Vec::new();
        let mut components = Vec::new();
        for segment in &decomposition.segments {
            match &segment.segment {
                LujvoPart::Hyphen(_) => {}
                LujvoPart::Rafsi(_) => {
                    // An unresolvable rafsi fails the whole composition closed.
                    let source = segment.source?;
                    pieces.push(classify_lujvo_source(source, &mut components)?);
                }
            }
        }
        let composition = build_composition(pieces)?;
        Some((composition, components))
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_zei_compound_card(&mut self, word_like: &WordLike) {
        let mut parts = Vec::new();
        flatten_zei_parts(word_like, &mut parts);
        let piece_texts = parts
            .iter()
            .map(zei_part_piece_text)
            .collect::<Vec<_>>();
        let word_text = piece_texts.join(" zei ");
        let id = piece_texts
            .iter()
            .map(|piece| piece.replace(' ', "-"))
            .collect::<Vec<_>>()
            .join("-zei-");
        if self.built_ids.contains(&id) {
            return;
        }
        let entries = self.dictionary.lookup_words(&word_text).collect::<Vec<_>>();
        if let Some(entry) = entries
            .iter()
            .copied()
            .find(|entry| !entry.definition.is_empty())
        {
            // Defined zei-lujvo: the dictionary is the sole authority, never
            // an approximation.
            self.built_ids.insert(id.clone());
            self.cards.push(known_word_card(id, word_text, WordCardKind::ZeiCompound, entry));
            return;
        }
        let mut pieces = Vec::new();
        let mut components = Vec::new();
        let mut recoverable = true;
        for part in &parts {
            match classify_zei_part(part, &mut components) {
                Some(piece) => pieces.push(piece),
                None => {
                    recoverable = false;
                    break;
                }
            }
        }
        let composition = if recoverable {
            build_composition(pieces)
        } else {
            None
        };
        match composition {
            Some(composition) => {
                self.built_ids.insert(id.clone());
                self.cards.push(new!(WordCard {
                    id: id,
                    word: word_text,
                    kind: WordCardKind::ZeiCompound,
                    known: false,
                    glosses: Vec::new(),
                    definition: None,
                    notes: None,
                    composition: Some(composition),
                    warnings: vec![ZEI_COMPOUND_NONCE_WARNING.to_owned()],
                }));
                for (component, kind) in components {
                    self.push_brivla_card(component, kind);
                }
            }
            None => {
                self.built_ids.insert(id.clone());
                self.cards.push(new!(WordCard {
                    id: id,
                    word: word_text,
                    kind: WordCardKind::ZeiCompound,
                    known: false,
                    glosses: Vec::new(),
                    definition: None,
                    notes: None,
                    composition: None,
                    warnings: vec![ZEI_COMPOUND_UNRECOVERABLE_WARNING.to_owned()],
                }));
            }
        }
    }
}

/// Build the plain card for a dictionary-defined word: gloss keywords,
/// definition, and notes from the first entry with a non-empty definition
/// (dictionary iteration order is curated and deterministic).
#[requires(!entry.definition.is_empty())]
#[ensures(ret.known)]
fn known_word_card(
    id: String,
    word_text: String,
    kind: WordCardKind,
    entry: &DictionaryEntry<'_>,
) -> WordCard {
    new!(WordCard {
        id: id,
        word: word_text,
        kind: kind,
        known: true,
        glosses: entry
            .gloss_keywords
            .iter()
            .map(|keyword| keyword.word)
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect(),
        definition: Some(entry.definition.to_owned()),
        notes: (!entry.notes.is_empty()).then(|| entry.notes.to_owned()),
        composition: None,
        warnings: Vec::new(),
    })
}

#[requires(true)]
#[ensures(matches!(ret, WordCardKind::Gismu | WordCardKind::Lujvo | WordCardKind::Fuhivla))]
fn word_card_kind_for_brivla(kind: WordKind) -> WordCardKind {
    match kind {
        WordKind::Gismu => WordCardKind::Gismu,
        WordKind::Lujvo => WordCardKind::Lujvo,
        WordKind::Fuhivla => WordCardKind::Fuhivla,
        WordKind::Cmevla => WordCardKind::Cmevla,
        WordKind::Cmavo => unreachable!("cmavo never become brivla cards"),
    }
}

/// Classify one resolved lujvo rafsi source word. Operator cmavo classify
/// against the closed table; anything else must morphologically parse as a
/// brivla component (its card is queued), else the composition fails closed.
#[requires(!source.is_empty())]
#[ensures(true)]
fn classify_lujvo_source(
    source: &str,
    components: &mut Vec<(String, WordCardKind)>,
) -> Option<Piece> {
    if let Some(cmavo) = Cmavo::from_text(source) {
        return classify_cmavo_piece(cmavo);
    }
    let kind = brivla_kind_for_source(source)?;
    push_unique_component(components, source.to_owned(), kind);
    Some(Piece::Tok(Tok::Expr(new!(ApproxExpr::Component {
        word: source.to_owned(),
    }))))
}

/// Morphologically classify a non-cmavo source word as a brivla card kind.
#[requires(!source.is_empty())]
#[ensures(ret.is_some() -> matches!(ret, Some(WordCardKind::Gismu | WordCardKind::Lujvo | WordCardKind::Fuhivla)))]
fn brivla_kind_for_source(source: &str) -> Option<WordCardKind> {
    let words = segment_words_with_modifiers(source).ok()?;
    let [word_like] = words.as_slice() else {
        return None;
    };
    let word = word_like.bare_word()?;
    match word.kind() {
        WordKind::Gismu => Some(WordCardKind::Gismu),
        WordKind::Lujvo => Some(WordCardKind::Lujvo),
        WordKind::Fuhivla => Some(WordCardKind::Fuhivla),
        WordKind::Cmevla | WordKind::Cmavo => None,
    }
}

#[requires(!word.is_empty())]
#[ensures(components.len() >= old(components.len()))]
fn push_unique_component(
    components: &mut Vec<(String, WordCardKind)>,
    word: String,
    kind: WordCardKind,
) {
    if !components.iter().any(|(existing, _)| existing == &word) {
        components.push((word, kind));
    }
}

/// One flattened zei compound part: the rightmost word of each nesting level,
/// or a non-compound left-spine word-like.
#[invariant(::Word(_) => true)]
#[invariant(::WordLike(_) => true)]
#[derive(Debug, Clone, Copy)]
enum ZeiPartRef<'a> {
    Word(&'a Word),
    WordLike(&'a WordLike),
}

/// Flatten a zei compound's nested left spine into stream order
/// (`[[tavla zei kumfa] zei barda]` → `[tavla, kumfa, barda]`).
#[requires(true)]
#[ensures(parts.len() > old(parts.len()))]
fn flatten_zei_parts<'a>(word_like: &'a WordLike, parts: &mut Vec<ZeiPartRef<'a>>) {
    match word_like.as_data() {
        data!(WordLike::ZeiCompound { left, right, .. }) => {
            flatten_zei_parts(left, parts);
            parts.push(ZeiPartRef::Word(right));
        }
        data!(WordLike::PlainWord(word)) => parts.push(ZeiPartRef::Word(word)),
        _ => parts.push(ZeiPartRef::WordLike(word_like)),
    }
}

/// The canonical surface text of one zei part, used for the card's `word`
/// and ID. Cmevla pause periods are stripped; lerfu parts use the canonical
/// lerfu word spelling (`abu`, not `a bu`); every other part is its
/// canonical word text (space-joined for multi-word parts).
#[requires(true)]
#[ensures(!ret.is_empty())]
fn zei_part_piece_text(part: &ZeiPartRef<'_>) -> String {
    match part {
        ZeiPartRef::Word(word) => word_piece_text(word),
        ZeiPartRef::WordLike(word_like) => {
            if let Some(text) = lerfu_part_piece_text(word_like) {
                return text;
            }
            let mut words = Vec::new();
            collect_word_like_words(word_like, &mut words);
            words
                .iter()
                .map(|word| word_piece_text(word))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

/// The canonical lerfu word spelling of a `LerfuWord` part, mirroring the
/// letteral lookup convention in `vlacku.rs` (`letteral_lookup_text`): a
/// single consonant base takes `y`, a single vowel base takes `bu`, and any
/// other base is `<base> bu`.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn lerfu_part_piece_text(word_like: &WordLike) -> Option<String> {
    let data!(WordLike::LerfuWord { base, .. }) = word_like.as_data() else {
        return None;
    };
    let base_text = word_like_lookup_text(base)?;
    let normalized = normalize_lookup_query(&base_text);
    let mut chars = normalized.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        if is_consonant(first) {
            return Some(format!("{first}y"));
        }
        if is_vowel(first) {
            return Some(format!("{first}bu"));
        }
    }
    Some(format!("{normalized} bu"))
}

/// The canonical dictionary lookup spelling of one word-like, mirroring
/// `word_like_lookup_text` in `vlacku.rs`.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn word_like_lookup_text(word_like: &WordLike) -> Option<String> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => Some(word_piece_text(word)),
        data!(WordLike::LerfuWord { .. }) => lerfu_part_piece_text(word_like),
        data!(WordLike::ZeiCompound { left, right, .. }) => Some(format!(
            "{} zei {}",
            word_like_lookup_text(left)?,
            word_piece_text(right)
        )),
        data!(WordLike::QuotedWord { .. })
        | data!(WordLike::SelmahoQuotedWord { .. })
        | data!(WordLike::DelimitedNonLojbanQuote { .. })
        | data!(WordLike::QuotedWords { .. })
        | data!(WordLike::DelimitedWordQuote { .. }) => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_piece_text(word: &Word) -> String {
    let text = canonicalize_text(word.phonemes().as_str());
    if word.is_cmevla() {
        return text.trim_matches('.').to_owned();
    }
    text
}

/// Collect every leaf word of a word-like, for surface text of parts that
/// are not single words (compound lerfu).
#[requires(true)]
#[ensures(true)]
fn collect_word_like_words<'a>(word_like: &'a WordLike, words: &mut Vec<&'a Word>) {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => words.push(word),
        data!(WordLike::QuotedWord { zo, word }) => {
            words.push(zo);
            words.push(word);
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            words.push(mahoi);
            words.push(word);
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            closing_delimiter,
            ..
        }) => {
            words.push(zoi);
            words.push(opening_delimiter);
            words.push(closing_delimiter);
        }
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            words.push(lohu);
            words.extend(quoted_words);
            words.push(lehu);
        }
        data!(WordLike::DelimitedWordQuote { marker, .. }) => words.push(marker),
        data!(WordLike::LerfuWord { base, bu }) => {
            collect_word_like_words(base, words);
            words.push(bu);
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            collect_word_like_words(left, words);
            words.push(zei);
            words.push(right);
        }
    }
}

/// Classify one flattened zei part. Brivla parts become `Component` leaves
/// (their cards are queued); cmevla parts become `Named` placeholders through
/// the same `ReferentOf` conversion as in-discourse name-as-selbri (owner
/// round 13, ruling 3); lerfu parts become `Letter` leaves; cmavo parts
/// classify against the closed operator table. Anything else fails the whole
/// composition closed.
#[requires(true)]
#[ensures(true)]
fn classify_zei_part(
    part: &ZeiPartRef<'_>,
    components: &mut Vec<(String, WordCardKind)>,
) -> Option<Piece> {
    match part {
        ZeiPartRef::Word(word) => match word.kind() {
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => {
                let text = word_piece_text(word);
                push_unique_component(components, text.clone(), word_card_kind_for_brivla(word.kind()));
                Some(Piece::Tok(Tok::Expr(new!(ApproxExpr::Component {
                    word: text,
                }))))
            }
            WordKind::Cmevla => Some(Piece::Tok(Tok::Expr(new!(ApproxExpr::ReferentOf {
                referent: new!(ApproxReferent::Named {
                    text: word_piece_text(word),
                    by: Some(Box::new(new!(ApproxReferent::Context {
                        role: VariableContextRole::Speaker,
                        proximity: None,
                        slot: None,
                    }))),
                }),
            })))),
            WordKind::Cmavo => classify_cmavo_piece(
                word.cmavo().expect("a word of kind Cmavo has a cmavo variant"),
            ),
        },
        ZeiPartRef::WordLike(word_like) => match word_like.as_data() {
            data!(WordLike::LerfuWord { .. }) => {
                Some(Piece::Tok(Tok::Expr(new!(ApproxExpr::Letter {
                    text: zei_part_piece_text(part),
                }))))
            }
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use bityzba::{ensures, requires};

    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn dictionary() -> &'static Dictionary<'static> {
        jbotci_dictionary_data::english()
    }

    /// Parse `text` with the real morphology segmenter (every test input is
    /// validated morphologically, never assumed) and build its cards.
    #[requires(true)]
    #[ensures(true)]
    fn cards_for(text: &str) -> Vec<WordCard> {
        let words = segment_words_with_modifiers(text)
            .unwrap_or_else(|error| panic!("test input `{text}` must segment: {error:?}"));
        build_xml_word_cards(dictionary(), &words)
    }

    /// Parse `text` and assert it is exactly one zei compound.
    #[requires(true)]
    #[ensures(true)]
    fn assert_zei_compound(text: &str) {
        let words = segment_words_with_modifiers(text)
            .unwrap_or_else(|error| panic!("test input `{text}` must segment: {error:?}"));
        let [word_like] = words.as_slice() else {
            panic!("test input `{text}` must be a single word-like");
        };
        assert!(
            matches!(word_like.as_data(), data!(WordLike::ZeiCompound { .. })),
            "test input `{text}` must be a zei compound"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn only_card(text: &str) -> WordCard {
        let cards = cards_for(text);
        assert_eq!(cards.len(), 1, "expected exactly one card for `{text}`");
        cards.into_iter().next().expect("one card was asserted")
    }

    #[requires(true)]
    #[ensures(true)]
    fn composition_of(card: &WordCard) -> &CompositeApprox {
        card.composition.as_ref().expect("card has a composition")
    }

    #[requires(true)]
    #[ensures(true)]
    fn component_word(expr: &ApproxExpr) -> &str {
        match expr.as_data() {
            data!(ApproxExpr::Component { word }) => word,
            other => panic!("expected a Component leaf, got {other:?}"),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn as_kind_composition(
        expr: &ApproxExpr,
    ) -> (&ApproxExpr, &ApproxExpr, Option<GroupingBasis>) {
        match expr.as_data() {
            data!(ApproxExpr::KindComposition {
                kind,
                modifier,
                grouping,
            }) => (kind, modifier, *grouping),
            other => panic!("expected a KindComposition, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn known_gismu_card_carries_dictionary_content() {
        let card = only_card("barda");
        assert_eq!(card.id, "barda");
        assert_eq!(card.word, "barda");
        assert_eq!(card.kind, WordCardKind::Gismu);
        assert!(card.known);
        assert_eq!(card.glosses, ["big", "large"]);
        // The officialdata entry is the first entry with a definition.
        assert_eq!(
            card.definition.as_deref(),
            Some(
                "$x_{1}$ is big/large in property/dimension(s) $x_{2}$ (ka) as compared with standard/norm $x_{3}$."
            )
        );
        assert!(card.notes.as_ref().is_some_and(|notes| !notes.is_empty()));
        assert!(card.composition.is_none());
        assert!(card.warnings.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_gismu_ribga_is_a_known_card() {
        let card = only_card("ribga");
        assert_eq!(card.kind, WordCardKind::Gismu);
        assert!(card.known);
        assert!(card.definition.as_ref().is_some_and(|d| !d.is_empty()));
        assert!(card.composition.is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unknown_gismu_gets_bare_unknown_card() {
        // `sfoto` is morphologically a valid gismu and dictionary-absent
        // (both facts verified here through the real machinery).
        let words = segment_words_with_modifiers("sfoto").expect("sfoto segments");
        let [word_like] = words.as_slice() else {
            panic!("sfoto is a single word");
        };
        assert_eq!(
            word_like.bare_word().map(Word::kind),
            Some(WordKind::Gismu)
        );
        assert_eq!(dictionary().lookup_words("sfoto").count(), 0);

        let card = only_card("sfoto");
        assert_eq!(card.kind, WordCardKind::Gismu);
        assert!(!card.known);
        assert!(card.definition.is_none());
        assert!(card.composition.is_none());
        assert!(card.warnings.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn skamymlatu_builds_kind_composition_and_component_cards() {
        assert_eq!(dictionary().lookup_words("skamymlatu").count(), 0);
        let cards = cards_for("skamymlatu");
        assert_eq!(cards.len(), 3);
        let card = &cards[0];
        assert_eq!(card.id, "skamymlatu");
        assert_eq!(card.kind, WordCardKind::Lujvo);
        assert!(!card.known);
        assert_eq!(card.warnings, [UNDEFINED_LUJVO_NONCE_WARNING.to_owned()]);
        let composition = composition_of(card);
        assert_eq!(composition.grouping, None);
        assert_eq!(composition.scope, None);
        let (kind, modifier, grouping) = as_kind_composition(&composition.root);
        assert_eq!(grouping, None);
        assert_eq!(component_word(kind), "mlatu");
        assert_eq!(component_word(modifier), "skami");
        // Component cards follow, in stream order.
        assert_eq!(cards[1].id, "skami");
        assert!(cards[1].known);
        assert_eq!(cards[1].glosses, ["computer"]);
        assert_eq!(cards[2].id, "mlatu");
        assert!(cards[2].known);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn relseljirna_places_quantity_kind_and_swapped_places_base() {
        assert_eq!(dictionary().lookup_words("relseljirna").count(), 0);
        let cards = cards_for("relseljirna");
        assert_eq!(cards.len(), 2);
        let composition = composition_of(&cards[0]);
        assert_eq!(composition.grouping, None);
        assert_eq!(composition.scope, Some(ScopeBasis::AssumedShort));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        match modifier.as_data() {
            data!(ApproxExpr::Quantity { form, value, text }) => {
                assert_eq!(*form, ApproxQuantityForm::Exact);
                assert_eq!(*value, Some(2));
                assert_eq!(*text, None);
            }
            other => panic!("expected a Quantity kind, got {other:?}"),
        }
        match kind.as_data() {
            data!(ApproxExpr::SwappedPlaces {
                first,
                second,
                inner,
                scope,
            }) => {
                assert_eq!((*first, *second), (1, 2));
                assert_eq!(*scope, None);
                assert_eq!(component_word(inner), "jirna");
            }
            other => panic!("expected SwappedPlaces, got {other:?}"),
        }
        // Only the brivla component gets a card; `re` and `se` are cmavo.
        assert_eq!(cards[1].id, "jirna");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn kei_closure_makes_abstraction_scope_explicit() {
        assert_eq!(dictionary().lookup_words("nunbracmakezyxli").count(), 0);
        let cards = cards_for("nunbracmakezyxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        assert_eq!(composition.scope, Some(ScopeBasis::Explicit));
        assert_eq!(composition.grouping, Some(GroupingBasis::AssumedLeft));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "nixli");
        match modifier.as_data() {
            data!(ApproxExpr::Abstraction {
                kind: abstraction_kind,
                inner,
                scope,
            }) => {
                assert_eq!(*abstraction_kind, AbstractionKind::Event);
                assert_eq!(*scope, None, "tree-level SCOPE strips per-node bases");
                let (inner_kind, inner_modifier, _) = as_kind_composition(inner);
                assert_eq!(component_word(inner_kind), "cmalu");
                assert_eq!(component_word(inner_modifier), "barda");
            }
            other => panic!("expected an Abstraction spanning barda-cmalu, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn without_kei_abstraction_scope_is_assumed_short() {
        assert_eq!(dictionary().lookup_words("nunbracmaxli").count(), 0);
        let cards = cards_for("nunbracmaxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        assert_eq!(composition.scope, Some(ScopeBasis::AssumedShort));
        assert_eq!(composition.grouping, Some(GroupingBasis::AssumedLeft));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "nixli");
        let (inner_kind, inner_modifier, _) = as_kind_composition(modifier);
        assert_eq!(component_word(inner_kind), "cmalu");
        match inner_modifier.as_data() {
            data!(ApproxExpr::Abstraction {
                kind: abstraction_kind,
                inner,
                ..
            }) => {
                assert_eq!(*abstraction_kind, AbstractionKind::Event);
                assert_eq!(
                    component_word(inner),
                    "barda",
                    "without kei the abstraction spans only the following unit"
                );
            }
            other => panic!("expected a short-scope Abstraction of barda, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bo_nests_right_and_makes_grouping_explicit() {
        assert_eq!(dictionary().lookup_words("bracmaborxli").count(), 0);
        let cards = cards_for("bracmaborxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        assert_eq!(composition.grouping, Some(GroupingBasis::Explicit));
        assert_eq!(composition.scope, None);
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(modifier), "barda");
        let (inner_kind, inner_modifier, _) = as_kind_composition(kind);
        assert_eq!(component_word(inner_kind), "nixli");
        assert_eq!(component_word(inner_modifier), "cmalu");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_three_part_lujvo_groups_assumed_left() {
        assert_eq!(dictionary().lookup_words("bracmaxli").count(), 0);
        let cards = cards_for("bracmaxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        assert_eq!(composition.grouping, Some(GroupingBasis::AssumedLeft));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "nixli");
        let (inner_kind, inner_modifier, _) = as_kind_composition(modifier);
        assert_eq!(component_word(inner_kind), "cmalu");
        assert_eq!(component_word(inner_modifier), "barda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bo_first_pair_marks_the_whole_tree_grouping_explicit() {
        assert_eq!(dictionary().lookup_words("braborcmaxli").count(), 0);
        let cards = cards_for("braborcmaxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        assert_eq!(composition.grouping, Some(GroupingBasis::Explicit));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "nixli");
        let (inner_kind, inner_modifier, _) = as_kind_composition(modifier);
        assert_eq!(component_word(inner_kind), "cmalu");
        assert_eq!(component_word(inner_modifier), "barda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ke_group_marks_the_whole_tree_grouping_explicit() {
        assert_eq!(dictionary().lookup_words("kembracmakepxli").count(), 0);
        let cards = cards_for("kembracmakepxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        assert_eq!(composition.grouping, Some(GroupingBasis::Explicit));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "nixli");
        let (inner_kind, inner_modifier, _) = as_kind_composition(modifier);
        assert_eq!(component_word(inner_kind), "cmalu");
        assert_eq!(component_word(inner_modifier), "barda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stacked_se_nests_swapped_places() {
        assert_eq!(dictionary().lookup_words("terselju'o").count(), 0);
        let cards = cards_for("terselju'o");
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "terselju'o");
        let composition = composition_of(&cards[0]);
        assert_eq!(composition.grouping, None, "a single unit makes no grouping assumption");
        assert_eq!(composition.scope, Some(ScopeBasis::AssumedShort));
        match composition.root.as_data() {
            data!(ApproxExpr::SwappedPlaces {
                first,
                second,
                inner,
                ..
            }) => {
                assert_eq!((*first, *second), (1, 3));
                match inner.as_data() {
                    data!(ApproxExpr::SwappedPlaces {
                        first,
                        second,
                        inner,
                        ..
                    }) => {
                        assert_eq!((*first, *second), (1, 2));
                        assert_eq!(component_word(inner), "djuno");
                    }
                    other => panic!("expected nested SwappedPlaces, got {other:?}"),
                }
            }
            other => panic!("expected SwappedPlaces, got {other:?}"),
        }
        assert_eq!(cards[1].id, "djuno");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn defined_lujvo_gets_a_plain_card() {
        let card = only_card("bavlamdei");
        assert_eq!(card.kind, WordCardKind::Lujvo);
        assert!(card.known);
        assert_eq!(card.glosses, ["next day", "tomorrow"]);
        assert!(card.definition.as_ref().is_some_and(|d| !d.is_empty()));
        assert!(card.composition.is_none());
        assert!(card.warnings.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn defined_zei_lujvo_gets_a_plain_card() {
        assert_zei_compound("abu zei sance");
        let card = only_card("abu zei sance");
        assert_eq!(card.kind, WordCardKind::ZeiCompound);
        assert_eq!(card.id, "abu-zei-sance");
        assert_eq!(card.word, "abu zei sance");
        assert!(card.known);
        assert!(card.definition.as_ref().is_some_and(|d| !d.is_empty()));
        assert!(card.composition.is_none());
        assert!(card.warnings.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn defined_cmevla_zei_lujvo_gets_a_plain_card() {
        // The cmevla pause period is stripped for the card ID and does not
        // break the dictionary lookup (lookup normalization filters periods).
        assert_zei_compound("atlantik. zei braxamsi");
        let cards = cards_for("atlantik. zei braxamsi");
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.id, "atlantik-zei-braxamsi");
        assert!(card.known);
        assert!(card.composition.is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mi_zei_do_builds_context_role_composition() {
        assert_zei_compound("mi zei do");
        let cards = cards_for("mi zei do");
        assert_eq!(cards.len(), 1, "pro-word parts are cmavo: no component cards");
        let card = &cards[0];
        assert_eq!(card.id, "mi-zei-do");
        assert_eq!(card.word, "mi zei do");
        assert!(!card.known);
        assert_eq!(card.warnings, [ZEI_COMPOUND_NONCE_WARNING.to_owned()]);
        let composition = composition_of(card);
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_context_role(kind, VariableContextRole::Audience, None, None);
        assert_context_role(modifier, VariableContextRole::Speaker, None, None);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_context_role(
        expr: &ApproxExpr,
        role: VariableContextRole,
        proximity: Option<Proximity>,
        slot: Option<u8>,
    ) {
        match expr.as_data() {
            data!(ApproxExpr::ReferentOf { referent }) => match referent.as_data() {
                data!(ApproxReferent::Context {
                    role: actual_role,
                    proximity: actual_proximity,
                    slot: actual_slot,
                }) => {
                    assert_eq!((*actual_role, *actual_proximity, *actual_slot), (role, proximity, slot));
                }
                other => panic!("expected a Context referent, got {other:?}"),
            },
            other => panic!("expected a ReferentOf, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmevla_zei_part_becomes_named_referent() {
        assert_zei_compound(".alis. zei ninmu");
        let cards = cards_for(".alis. zei ninmu");
        assert_eq!(cards.len(), 2);
        let card = &cards[0];
        assert_eq!(card.id, "alis-zei-ninmu");
        assert_eq!(card.word, "alis zei ninmu");
        assert!(!card.known);
        let composition = composition_of(card);
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "ninmu");
        match modifier.as_data() {
            data!(ApproxExpr::ReferentOf { referent }) => match referent.as_data() {
                data!(ApproxReferent::Named { text, by }) => {
                    assert_eq!(text, "alis");
                    let Some(by) = by else {
                        panic!("a cmevla referent carries the namer placeholder");
                    };
                    assert!(matches!(
                        by.as_data(),
                        data!(ApproxReferent::Context {
                            role: VariableContextRole::Speaker,
                            ..
                        })
                    ));
                }
                other => panic!("expected a Named referent, got {other:?}"),
            },
            other => panic!("expected a ReferentOf, got {other:?}"),
        }
        assert_eq!(cards[1].id, "ninmu");
        assert!(cards[1].known);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vocative_zei_part_fails_closed() {
        assert_zei_compound("coi zei ninmu");
        let cards = cards_for("coi zei ninmu");
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.id, "coi-zei-ninmu");
        assert!(!card.known);
        assert!(card.composition.is_none());
        assert_eq!(card.warnings, [ZEI_COMPOUND_UNRECOVERABLE_WARNING.to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn experimental_koha_zei_part_fails_closed() {
        assert_zei_compound("da'ei zei ninmu");
        let card = only_card("da'ei zei ninmu");
        assert!(card.composition.is_none());
        assert_eq!(card.warnings, [ZEI_COMPOUND_UNRECOVERABLE_WARNING.to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bo_first_lujvo_fails_closed() {
        // borbracmaxli is a morphologically valid lujvo whose veljvo begins
        // with bo: no tanru reading, fail closed.
        let words = segment_words_with_modifiers("borbracmaxli").expect("borbracmaxli segments");
        let [word_like] = words.as_slice() else {
            panic!("borbracmaxli is a single word");
        };
        assert_eq!(
            word_like.bare_word().map(Word::kind),
            Some(WordKind::Lujvo)
        );
        assert_eq!(dictionary().lookup_words("borbracmaxli").count(), 0);

        let cards = cards_for("borbracmaxli");
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert!(!card.known);
        assert!(card.composition.is_none());
        assert_eq!(card.warnings, [UNDEFINED_LUJVO_UNRECOVERABLE_WARNING.to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmavo_never_produce_cards() {
        let cards = cards_for("mi klama");
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "klama");
        assert!(cards[0].known);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn repeated_component_produces_one_card() {
        assert_zei_compound("barda zei barda");
        let cards = cards_for("barda zei barda");
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "barda-zei-barda");
        assert_eq!(cards[1].id, "barda");
        let composition = composition_of(&cards[0]);
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "barda");
        assert_eq!(component_word(modifier), "barda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_zei_left_spine_groups_left() {
        assert_zei_compound("tavla zei kumfa zei barda");
        let cards = cards_for("tavla zei kumfa zei barda");
        assert_eq!(cards.len(), 4);
        let card = &cards[0];
        assert_eq!(card.id, "tavla-zei-kumfa-zei-barda");
        assert_eq!(card.word, "tavla zei kumfa zei barda");
        let composition = composition_of(card);
        assert_eq!(composition.grouping, Some(GroupingBasis::AssumedLeft));
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "barda");
        let (inner_kind, inner_modifier, _) = as_kind_composition(modifier);
        assert_eq!(component_word(inner_kind), "kumfa");
        assert_eq!(component_word(inner_modifier), "tavla");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn connective_joins_juxtaposition_segments() {
        assert_zei_compound("barda zei je zei nixli");
        let cards = cards_for("barda zei je zei nixli");
        let composition = composition_of(&cards[0]);
        assert_eq!(composition.grouping, None, "no kind-composition edge, no grouping");
        match composition.root.as_data() {
            data!(ApproxExpr::Connective {
                operator,
                left,
                right,
            }) => {
                assert_eq!(*operator, ApproxConnective::And);
                assert_eq!(component_word(left), "barda");
                assert_eq!(component_word(right), "nixli");
            }
            other => panic!("expected a Connective, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn na_negates_the_whole_recovered_predicate() {
        assert_eq!(dictionary().lookup_words("narbracmaxli").count(), 0);
        let cards = cards_for("narbracmaxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        match composition.root.as_data() {
            data!(ApproxExpr::PredicationNegation { inner }) => {
                let (kind, modifier, _) = as_kind_composition(inner);
                assert_eq!(component_word(kind), "nixli");
                let (inner_kind, inner_modifier, _) = as_kind_composition(modifier);
                assert_eq!(component_word(inner_kind), "cmalu");
                assert_eq!(component_word(inner_modifier), "barda");
            }
            other => panic!("expected PredicationNegation, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zio_deletes_the_first_place_of_the_following_predicate() {
        assert_eq!(dictionary().lookup_words("zilbracmaxli").count(), 0);
        let cards = cards_for("zilbracmaxli");
        let card = &cards[0];
        let composition = composition_of(&card);
        let (kind, modifier, _) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "nixli");
        let (_, inner_modifier, _) = as_kind_composition(modifier);
        match inner_modifier.as_data() {
            data!(ApproxExpr::PlaceDeletion { index, inner }) => {
                assert_eq!(*index, 1);
                assert_eq!(component_word(inner), "barda");
            }
            other => panic!("expected PlaceDeletion, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tense_modal_facets_wrap_and_merge() {
        assert_zei_compound("ca'a zei co'a zei broda");
        let cards = cards_for("ca'a zei co'a zei broda");
        let composition = composition_of(&cards[0]);
        match composition.root.as_data() {
            data!(ApproxExpr::TenseModal {
                actuality,
                aspect,
                space_whole,
                time_whole,
                inner,
            }) => {
                assert_eq!(*actuality, Some(ActualityKind::Actual));
                assert_eq!(*aspect, Some(ApproxAspectContour::Initiative));
                assert!(!space_whole);
                assert!(!time_whole);
                assert_eq!(component_word(inner), "broda");
            }
            other => panic!("expected one merged TenseModal, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn space_extent_sets_the_space_whole_facet() {
        assert_zei_compound("ve'e zei broda");
        let cards = cards_for("ve'e zei broda");
        let composition = composition_of(&cards[0]);
        match composition.root.as_data() {
            data!(ApproxExpr::TenseModal {
                actuality,
                aspect,
                space_whole,
                time_whole,
                inner,
            }) => {
                assert_eq!(*actuality, None);
                assert_eq!(*aspect, None);
                assert!(*space_whole);
                assert!(!time_whole);
                assert_eq!(component_word(inner), "broda");
            }
            other => panic!("expected TenseModal, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn assigned_slots_follow_the_koha_foha_series() {
        assert_zei_compound("ko'a zei broda");
        let cards = cards_for("ko'a zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_context_role(modifier, VariableContextRole::Assigned, None, Some(1));

        assert_zei_compound("fo'a zei broda");
        let cards = cards_for("fo'a zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_context_role(modifier, VariableContextRole::Assigned, None, Some(6));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn personal_mass_referent_records_membership() {
        assert_zei_compound("ma'a zei broda");
        let cards = cards_for("ma'a zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        match modifier.as_data() {
            data!(ApproxExpr::ReferentOf { referent }) => match referent.as_data() {
                data!(ApproxReferent::PersonalMass {
                    speaker,
                    audience,
                    others,
                }) => {
                    assert_eq!((*speaker, *audience, *others), (
                        Inclusion::Included,
                        Inclusion::Included,
                        true,
                    ));
                }
                other => panic!("expected PersonalMass, got {other:?}"),
            },
            other => panic!("expected ReferentOf, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn da_series_becomes_implicitly_bound_entity_variables() {
        assert_zei_compound("da zei broda");
        let cards = cards_for("da zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        match modifier.as_data() {
            data!(ApproxExpr::ReferentOf { referent }) => match referent.as_data() {
                data!(ApproxReferent::LogicalVariable { sort, series }) => {
                    assert_eq!((*sort, *series), (LogicalVariableSort::Entity, 1));
                }
                other => panic!("expected LogicalVariable, got {other:?}"),
            },
            other => panic!("expected ReferentOf, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn postfix_moi_makes_an_ordinal_of_the_preceding_quantity() {
        assert_zei_compound("re zei moi");
        let cards = cards_for("re zei moi");
        let composition = composition_of(&cards[0]);
        match composition.root.as_data() {
            data!(ApproxExpr::Ordinal { inner }) => match inner.as_data() {
                data!(ApproxExpr::Quantity { form, value, .. }) => {
                    assert_eq!((*form, *value), (ApproxQuantityForm::Exact, Some(2)));
                }
                other => panic!("expected a Quantity inside Ordinal, got {other:?}"),
            },
            other => panic!("expected Ordinal, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn postfix_mei_roi_bu_wrap_the_preceding_unit() {
        for (text, expected) in [
            ("re zei mei", "Cardinal"),
            ("re zei roi", "Recurrence"),
            ("re zei bu", "LetterOf"),
        ] {
            assert_zei_compound(text);
            let cards = cards_for(text);
            let root = &composition_of(&cards[0]).root;
            let matches = match (expected, root.as_data()) {
                ("Cardinal", data!(ApproxExpr::Cardinal { .. }))
                | ("Recurrence", data!(ApproxExpr::Recurrence { .. }))
                | ("LetterOf", data!(ApproxExpr::LetterOf { .. })) => true,
                _ => false,
            };
            assert!(matches, "expected {expected} for `{text}`, got {root:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quantifier_forms_and_defaults() {
        assert_zei_compound("ro zei broda");
        let cards = cards_for("ro zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_quantity(modifier, ApproxQuantityForm::All, None, None);

        assert_zei_compound("su'o zei re zei broda");
        let cards = cards_for("su'o zei re zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_quantity(modifier, ApproxQuantityForm::AtLeast, Some(2), None);

        assert_zei_compound("su'o zei broda");
        let cards = cards_for("su'o zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_quantity(modifier, ApproxQuantityForm::AtLeast, Some(1), None);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_quantity(
        expr: &ApproxExpr,
        form: ApproxQuantityForm,
        value: Option<i64>,
        text: Option<&str>,
    ) {
        match expr.as_data() {
            data!(ApproxExpr::Quantity {
                form: actual_form,
                value: actual_value,
                text: actual_text,
            }) => {
                assert_eq!(
                    (*actual_form, *actual_value, actual_text.as_deref()),
                    (form, value, text)
                );
            }
            other => panic!("expected a Quantity, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pi_number_run_becomes_a_text_form_exact_quantity() {
        assert_zei_compound("pa zei pi zei re zei broda");
        let cards = cards_for("pa zei pi zei re zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_quantity(modifier, ApproxQuantityForm::Exact, None, Some("1.2"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn number_reduction_unit_cases() {
        let quantity = |run: &[NumTok]| reduce_number_run(run);
        assert_quantity(
            &quantity(&[NumTok::Digit(1), NumTok::Digit(2)]).expect("12"),
            ApproxQuantityForm::Exact,
            Some(12),
            None,
        );
        assert_quantity(
            &quantity(&[NumTok::Digit(1), NumTok::Point, NumTok::Digit(2)]).expect("1.2"),
            ApproxQuantityForm::Exact,
            None,
            Some("1.2"),
        );
        assert_quantity(
            &quantity(&[NumTok::Digit(1), NumTok::Digit(2), NumTok::Percent]).expect("12%"),
            ApproxQuantityForm::Exact,
            None,
            Some("12%"),
        );
        assert_quantity(
            &quantity(&[NumTok::Quantifier(ApproxQuantityForm::AllBut)]).expect("da'a"),
            ApproxQuantityForm::AllBut,
            Some(1),
            None,
        );
        assert_quantity(
            &quantity(&[NumTok::Quantifier(ApproxQuantityForm::Few)]).expect("so'u"),
            ApproxQuantityForm::Few,
            None,
            None,
        );
        // Malformed runs fail closed.
        for run in [
            &[NumTok::Quantifier(ApproxQuantityForm::All), NumTok::Digit(1)][..],
            &[NumTok::Digit(1), NumTok::Point, NumTok::Point][..],
            &[NumTok::Digit(1), NumTok::Percent, NumTok::Digit(2)][..],
            &[NumTok::Quantifier(ApproxQuantityForm::All), NumTok::Quantifier(ApproxQuantityForm::Few)][..],
            &[NumTok::Point][..],
        ] {
            assert!(quantity(run).is_none(), "run {run:?} must fail closed");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lerfu_zei_part_becomes_a_letter_leaf() {
        assert_zei_compound("by zei kantu");
        let cards = cards_for("by zei kantu");
        let (kind, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_eq!(component_word(kind), "kantu");
        match modifier.as_data() {
            data!(ApproxExpr::Letter { text }) => assert_eq!(text, "by"),
            other => panic!("expected a Letter leaf, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn du_and_coe_are_predicate_leaves() {
        assert_zei_compound("du zei broda");
        let cards = cards_for("du zei broda");
        let (kind, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        assert_eq!(component_word(kind), "broda");
        assert!(matches!(modifier.as_data(), data!(ApproxExpr::Identity)));

        assert_zei_compound("co'e zei broda");
        let cards = cards_for("co'e zei broda");
        let (_, modifier, _) = as_kind_composition(&composition_of(&cards[0]).root);
        match modifier.as_data() {
            data!(ApproxExpr::VariableContext { role, .. }) => {
                assert_eq!(*role, VariableContextRole::EllipticalPredicate);
            }
            other => panic!("expected a VariableContext leaf, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ke_group_operand_makes_scope_explicit() {
        assert_zei_compound("se zei ke zei barda zei cmalu zei ke'e zei broda");
        let cards = cards_for("se zei ke zei barda zei cmalu zei ke'e zei broda");
        let composition = composition_of(&cards[0]);
        assert_eq!(composition.scope, Some(ScopeBasis::Explicit));
        // The ke-delimited inner edge is explicit; the edge joining the
        // se-wrapped unit to broda is assumed-left, so the tree escalates to
        // per-node bases.
        assert_eq!(composition.grouping, None);
        let (kind, modifier, top_grouping) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "broda");
        assert_eq!(top_grouping, Some(GroupingBasis::AssumedLeft));
        match modifier.as_data() {
            data!(ApproxExpr::SwappedPlaces {
                first,
                second,
                inner,
                scope,
            }) => {
                assert_eq!((*first, *second), (1, 2));
                assert_eq!(*scope, None, "tree-level SCOPE strips per-node bases");
                let (inner_kind, inner_modifier, inner_grouping) = as_kind_composition(inner);
                assert_eq!(component_word(inner_kind), "cmalu");
                assert_eq!(component_word(inner_modifier), "barda");
                assert_eq!(inner_grouping, Some(GroupingBasis::Explicit));
            }
            other => panic!("expected SwappedPlaces over the group, got {other:?}"),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn genuinely_mixed_bases_escalate_to_per_node() {
        // ke barda cmalu nixli ke'e broda: the group content's edges are
        // assumed-left, the edge joining the group is explicit.
        assert_zei_compound("ke zei barda zei cmalu zei nixli zei ke'e zei broda");
        let cards = cards_for("ke zei barda zei cmalu zei nixli zei ke'e zei broda");
        let composition = composition_of(&cards[0]);
        assert_eq!(composition.grouping, None, "mixed trees omit the tree-level attribute");
        let (kind, modifier, top_grouping) = as_kind_composition(&composition.root);
        assert_eq!(component_word(kind), "broda");
        assert_eq!(top_grouping, Some(GroupingBasis::Explicit));
        let (inner_kind, inner_modifier, inner_grouping) = as_kind_composition(modifier);
        assert_eq!(component_word(inner_kind), "nixli");
        assert_eq!(inner_grouping, Some(GroupingBasis::AssumedLeft));
        let (deepest_kind, deepest_modifier, deepest_grouping) = as_kind_composition(inner_modifier);
        assert_eq!(component_word(deepest_kind), "cmalu");
        assert_eq!(component_word(deepest_modifier), "barda");
        assert_eq!(deepest_grouping, Some(GroupingBasis::AssumedLeft));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_backed_cmevla_gets_a_card() {
        // Find the first cmevla entry with a definition and verify the whole
        // pipeline on it (cmevla without entries never produce cards).
        let entry = dictionary()
            .entries()
            .iter()
            .find(|entry| {
                entry.word_type == jbotci_dictionary::WordType::Cmevla && !entry.definition.is_empty()
            })
            .expect("the dictionary contains cmevla entries");
        let word = entry.word;
        let words = segment_words_with_modifiers(word)
            .unwrap_or_else(|error| panic!("dictionary cmevla `{word}` must segment: {error:?}"));
        let [word_like] = words.as_slice() else {
            panic!("`{word}` is a single word");
        };
        assert_eq!(
            word_like.bare_word().map(Word::kind),
            Some(WordKind::Cmevla)
        );
        let cards = build_xml_word_cards(dictionary(), &words);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].kind, WordCardKind::Cmevla);
        assert!(cards[0].known);
        assert!(cards[0].composition.is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmevla_without_entry_gets_no_card() {
        assert_eq!(dictionary().lookup_words("alis").count(), 0);
        let cards = cards_for(".alis.");
        assert!(cards.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zo_quote_defines_the_referenced_word_not_the_marker() {
        let cards = cards_for("zo barda");
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, "barda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cards_are_deduplicated_across_occurrences() {
        let cards = cards_for("barda cmalu barda");
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "barda");
        assert_eq!(cards[1].id, "cmalu");
    }
}

