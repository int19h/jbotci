//! Version-0 smusni types, rows, and relation identities.
//!
//! These types are the kernel's own vocabulary: they are independent of every
//! notation over the kernel, including the version-0 S-expression
//! serialization. The concrete type grammar that reads and writes them lives in
//! the `notation::sexpr::type_syntax` module; nothing here mentions `Datum`.

use std::collections::BTreeSet;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use num_bigint::BigUint;
use unicode_normalization::UnicodeNormalization;

use super::lexicon::{is_positive_integer_text, is_symbol_name};

/// A version-0 primitive type atom.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypeAtom {
    Entity,
    Eventuality,
    Achievement,
    Process,
    Activity,
    State,
    Experience,
    Locution,
    Location,
    Amount,
    Scale,
    TruthValue,
    Epistemology,
    Concept,
    AbstractNature,
    Proposition,
    Question,
    Text,
    Number,
    Natural,
    Cardinal,
    DeicticGround,
    Content,
    Discourse,
    TranscriptEntry,
    Performable,
    UtteranceToken,
    Force,
    SignKind,
    AnswerPolarity,
    AnswerExhaustivity,
    ScalarKind,
    LabelLevel,
    EndpointInclusion,
    RowTailMarker,
    Proximity,
    LexicalScopePolicy,
}

impl TypeAtom {
    /// Parse one member of the closed primitive-type namespace.
    #[requires(true)]
    #[ensures(ret.is_some() == Self::ALL.iter().any(|atom| atom.as_str() == text))]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|atom| atom.as_str() == text)
    }

    /// Return the canonical type spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entity => "Entity",
            Self::Eventuality => "Eventuality",
            Self::Achievement => "Achievement",
            Self::Process => "Process",
            Self::Activity => "Activity",
            Self::State => "State",
            Self::Experience => "Experience",
            Self::Locution => "Locution",
            Self::Location => "Location",
            Self::Amount => "Amount",
            Self::Scale => "Scale",
            Self::TruthValue => "TruthValue",
            Self::Epistemology => "Epistemology",
            Self::Concept => "Concept",
            Self::AbstractNature => "AbstractNature",
            Self::Proposition => "Proposition",
            Self::Question => "Question",
            Self::Text => "Text",
            Self::Number => "Number",
            Self::Natural => "Natural",
            Self::Cardinal => "Cardinal",
            Self::DeicticGround => "DeicticGround",
            Self::Content => "Content",
            Self::Discourse => "Discourse",
            Self::TranscriptEntry => "TranscriptEntry",
            Self::Performable => "Performable",
            Self::UtteranceToken => "UtteranceToken",
            Self::Force => "Force",
            Self::SignKind => "SignKind",
            Self::AnswerPolarity => "AnswerPolarity",
            Self::AnswerExhaustivity => "AnswerExhaustivity",
            Self::ScalarKind => "ScalarKind",
            Self::LabelLevel => "LabelLevel",
            Self::EndpointInclusion => "EndpointInclusion",
            Self::RowTailMarker => "RowTailMarker",
            Self::Proximity => "Proximity",
            Self::LexicalScopePolicy => "LexicalScopePolicy",
        }
    }

    /// Every closed primitive sort atom, in specification order.
    pub const ALL: [Self; 37] = [
        Self::Entity,
        Self::Eventuality,
        Self::Achievement,
        Self::Process,
        Self::Activity,
        Self::State,
        Self::Experience,
        Self::Locution,
        Self::Location,
        Self::Amount,
        Self::Scale,
        Self::TruthValue,
        Self::Epistemology,
        Self::Concept,
        Self::AbstractNature,
        Self::Proposition,
        Self::Question,
        Self::Text,
        Self::Number,
        Self::Natural,
        Self::Cardinal,
        Self::DeicticGround,
        Self::Content,
        Self::Discourse,
        Self::TranscriptEntry,
        Self::Performable,
        Self::UtteranceToken,
        Self::Force,
        Self::SignKind,
        Self::AnswerPolarity,
        Self::AnswerExhaustivity,
        Self::ScalarKind,
        Self::LabelLevel,
        Self::EndpointInclusion,
        Self::RowTailMarker,
        Self::Proximity,
        Self::LexicalScopePolicy,
    ];
}

/// Closed act-force index used by `(Act force)`.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Force {
    Assertion,
    Question,
    Directive,
    Expressive,
    Mentioning,
    Address,
}

impl Force {
    /// Parse a closed force literal.
    #[requires(true)]
    #[ensures(ret.is_some() == Self::ALL.iter().any(|value| value.as_str() == text))]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.as_str() == text)
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Assertion => "Assertion",
            Self::Question => "Question",
            Self::Directive => "Directive",
            Self::Expressive => "Expressive",
            Self::Mentioning => "Mentioning",
            Self::Address => "Address",
        }
    }

    const ALL: [Self; 6] = [
        Self::Assertion,
        Self::Question,
        Self::Directive,
        Self::Expressive,
        Self::Mentioning,
        Self::Address,
    ];
}

/// Closed sign-kind index.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignKind {
    Name,
    Sentence,
    Quotation,
    Word,
    Letteral,
    MathExpression,
    Connective,
    Text,
    Structured,
    Opaque,
}

impl SignKind {
    /// Parse a closed sign-kind literal.
    #[requires(true)]
    #[ensures(ret.is_some() == Self::ALL.iter().any(|value| value.as_str() == text))]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.as_str() == text)
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Sentence => "Sentence",
            Self::Quotation => "Quotation",
            Self::Word => "Word",
            Self::Letteral => "Letteral",
            Self::MathExpression => "MathExpression",
            Self::Connective => "Connective",
            Self::Text => "Text",
            Self::Structured => "Structured",
            Self::Opaque => "Opaque",
        }
    }

    const ALL: [Self; 10] = [
        Self::Name,
        Self::Sentence,
        Self::Quotation,
        Self::Word,
        Self::Letteral,
        Self::MathExpression,
        Self::Connective,
        Self::Text,
        Self::Structured,
        Self::Opaque,
    ];
}

/// Closed scalar relation-former index.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScalarKind {
    OtherThan,
    Opposite,
    Neutral,
}

impl ScalarKind {
    /// Parse a scalar-kind literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "OtherThan" | "Opposite" | "Neutral"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "OtherThan" => Some(Self::OtherThan),
            "Opposite" => Some(Self::Opposite),
            "Neutral" => Some(Self::Neutral),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OtherThan => "OtherThan",
            Self::Opposite => "Opposite",
            Self::Neutral => "Neutral",
        }
    }
}

/// A validated `$name` binder/reference token.
#[invariant(text.starts_with('$') && text.nfc().eq(text.chars()) && is_symbol_name(&text[1..]))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable {
    text: String,
}

impl Variable {
    /// Parse a variable token.
    #[requires(true)]
    #[ensures(ret.is_ok() == (text.nfc().eq(text.chars()) && text.strip_prefix('$').is_some_and(is_symbol_name)))]
    pub fn try_new(text: &str) -> Result<Self, TypeParseError> {
        if !text.nfc().eq(text.chars()) || !text.strip_prefix('$').is_some_and(is_symbol_name) {
            return Err(TypeParseError::new("invalid variable token"));
        }
        Ok(new!(Variable {
            text: text.to_owned(),
        }))
    }

    /// Compose `$<token>_<index>` from an already valid bare-symbol token.
    ///
    /// The precondition is what makes the result valid by construction: a bare
    /// symbol followed by `_` and decimal digits is itself a bare symbol, so no
    /// caller has to sanitize text or handle a parse failure. Rejecting `_` in
    /// the token additionally keeps `(token, index)` recoverable from the
    /// spelling, which is what makes derived namespaces collision-free.
    #[requires(is_symbol_name(token) && !token.contains('_'))]
    #[ensures(ret.as_str().starts_with('$'))]
    pub(crate) fn from_token_and_index(token: &str, index: usize) -> Self {
        new!(Variable {
            text: format!("${token}_{index}"),
        })
    }

    /// Borrow the complete `$name` spelling.
    #[requires(true)]
    #[ensures(ret.starts_with('$'))]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// A validated lowercase or escaped lowercase lexical root.
#[invariant(is_lexical_root_token(&text))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LexicalRoot {
    text: String,
}

impl LexicalRoot {
    /// Parse a lexical-root token without consulting dictionary semantics.
    #[requires(true)]
    #[ensures(ret.is_ok() == is_lexical_root_token(text))]
    pub fn try_new(text: &str) -> Result<Self, TypeParseError> {
        if !is_lexical_root_token(text) {
            return Err(TypeParseError::new("invalid lexical-root token"));
        }
        Ok(new!(LexicalRoot {
            text: text.to_owned(),
        }))
    }

    /// Borrow the canonical token spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// An arbitrarily large positive decimal integer used by place syntax.
#[invariant(*value > BigUint::from(0u8))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PositiveInteger {
    value: BigUint,
}

impl PositiveInteger {
    /// Parse the exact positive-integer token grammar.
    #[requires(true)]
    #[ensures(ret.is_ok() == is_positive_integer_text(text))]
    pub fn try_new(text: &str) -> Result<Self, TypeParseError> {
        if !is_positive_integer_text(text) {
            return Err(TypeParseError::new("expected a canonical positive integer"));
        }
        let value = text
            .parse::<BigUint>()
            .expect("ASCII decimal digits parse as BigUint");
        Ok(new!(PositiveInteger { value }))
    }

    /// Construct a positive place from a machine value.
    #[requires(value > 0)]
    #[ensures(ret.value == BigUint::from(value))]
    pub fn from_u32(value: u32) -> Self {
        new!(PositiveInteger {
            value: BigUint::from(value),
        })
    }

    /// Borrow the arbitrary-precision value for sibling syntax validators.
    #[requires(true)]
    #[ensures(ret == &self.value)]
    pub(crate) fn as_biguint(&self) -> &BigUint {
        &self.value
    }
}

impl fmt::Display for PositiveInteger {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(formatter)
    }
}

/// A numbered or distinguished row label.
#[invariant(::Numbered(_) => true)]
#[invariant(::Eventuality => true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaceLabel {
    Numbered(PositiveInteger),
    Eventuality,
}

impl PlaceLabel {
    /// Construct a positive numbered label.
    #[requires(value > 0)]
    #[ensures(matches!(ret, Self::Numbered(ref number) if number == &PositiveInteger::from_u32(value)))]
    pub fn numbered(value: u32) -> Self {
        Self::Numbered(PositiveInteger::from_u32(value))
    }
}

/// A nonempty unique explicit `PlaceOf` candidate set.
#[invariant(!labels.is_empty() && labels_are_unique(&labels))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceCandidates {
    labels: Vec<PlaceLabel>,
}

impl PlaceCandidates {
    /// Validate an explicit candidate set.
    #[requires(!labels.is_empty() && labels_are_unique(&labels))]
    #[ensures(ret.labels.len() == old(labels.len()))]
    pub fn new(labels: Vec<PlaceLabel>) -> Self {
        new!(PlaceCandidates { labels })
    }

    /// Borrow the ordered candidates.
    #[requires(true)]
    #[ensures(!ret.is_empty() && labels_are_unique(ret))]
    pub fn as_slice(&self) -> &[PlaceLabel] {
        &self.labels
    }
}

/// A computed-fill domain: nonempty and unique. An explicit domain may include
/// the distinguished event place; a derivable omitted domain may not.
#[invariant(!labels.is_empty() && labels_are_unique(&labels))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputedPlaceDomain {
    labels: Vec<PlaceLabel>,
}

impl ComputedPlaceDomain {
    /// Validate a computed fill domain.
    #[requires(!labels.is_empty() && labels_are_unique(&labels))]
    #[ensures(ret.labels.len() == old(labels.len()))]
    pub fn new(labels: Vec<PlaceLabel>) -> Self {
        new!(ComputedPlaceDomain { labels })
    }

    /// Borrow the ordered candidates.
    #[requires(true)]
    #[ensures(!ret.is_empty() && labels_are_unique(ret))]
    pub fn as_slice(&self) -> &[PlaceLabel] {
        &self.labels
    }
}

/// One effective-row slot.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowSlot {
    label: PlaceLabel,
    accepted_type: Box<TypeExpr>,
}

impl RowSlot {
    /// Construct a typed slot.
    #[requires(true)]
    #[ensures(ret.label == old(label.clone()) && ret.accepted_type.as_ref() == &old(accepted_type.clone()))]
    pub fn new(label: PlaceLabel, accepted_type: TypeExpr) -> Self {
        RowSlot {
            label,
            accepted_type: Box::new(accepted_type),
        }
    }

    /// Return the slot label.
    #[requires(true)]
    #[ensures(ret == self.label)]
    pub fn label(&self) -> PlaceLabel {
        self.label.clone()
    }

    /// Borrow the slot label.
    ///
    /// The application kernel compares labels far more often than it needs to
    /// own one, so the borrowed form exists beside [`Self::label`].
    #[requires(true)]
    #[ensures(ret == &self.label)]
    pub fn label_ref(&self) -> &PlaceLabel {
        &self.label
    }

    /// Borrow the accepted value type.
    #[requires(true)]
    #[ensures(true)]
    pub fn accepted_type(&self) -> &TypeExpr {
        &self.accepted_type
    }
}

/// A validated effective predicate row.
#[invariant(row_slots_are_canonical(&slots))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    slots: Vec<RowSlot>,
    open_numbered_tail: bool,
}

impl Row {
    /// Construct a row whose labels are unique and canonically ordered.
    #[requires(row_slots_are_canonical(&slots))]
    #[ensures(ret.slots.len() == old(slots.len()) && ret.open_numbered_tail == open_numbered_tail)]
    pub fn new(slots: Vec<RowSlot>, open_numbered_tail: bool) -> Self {
        new!(Row {
            slots,
            open_numbered_tail,
        })
    }

    /// Borrow the ordered slots.
    #[requires(true)]
    #[ensures(row_slots_are_canonical(ret))]
    pub fn slots(&self) -> &[RowSlot] {
        &self.slots
    }

    /// Move the ordered slots out of a row the caller already owns.
    #[requires(true)]
    #[ensures(row_slots_are_canonical(&ret))]
    pub fn into_slots(self) -> Vec<RowSlot> {
        self.into_data().slots
    }

    /// Report whether an unknown numbered tail survives.
    #[requires(true)]
    #[ensures(ret == self.open_numbered_tail)]
    pub fn has_open_numbered_tail(&self) -> bool {
        self.open_numbered_tail
    }
}

/// A relation identity usable by `PredTerm` provenance and `PlaceOf`.
#[invariant(::Lexical(_) => true)]
#[invariant(::Variable(_) => true)]
#[invariant(::DropPlace { .. } => true)]
#[invariant(::Tanru { .. } => true)]
#[invariant(::Scalar { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationRef {
    Lexical(LexicalRoot),
    Variable(Variable),
    DropPlace {
        relation: Box<RelationRef>,
        place: PositiveInteger,
    },
    Tanru {
        modifier: Box<RelationRef>,
        head: Box<RelationRef>,
    },
    Scalar {
        kind: ScalarKind,
        relation: Box<RelationRef>,
    },
}

/// A fully parsed section-2.2 type expression.
#[invariant(::Atom(_) => true)]
#[invariant(::Referents(_) => true)]
#[invariant(::Set(_) => true)]
#[invariant(::Group(_) => true)]
#[invariant(::List(_) => true)]
#[invariant(::Interval(_) => true)]
#[invariant(::Tuple(_) => true)]
#[invariant(::Function { .. } => true)]
#[invariant(::Predicate(_) => true)]
#[invariant(::ReferenceComputation(_) => true)]
#[invariant(::Act(_) => true)]
#[invariant(::Query(_) => true)]
#[invariant(::AnswerSelection(_) => true)]
#[invariant(::GeneralizedQuantifier(_) => true)]
#[invariant(::Sign(_) => true)]
#[invariant(::SignToken(_) => true)]
#[invariant(::PlaceOf { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Atom(TypeAtom),
    Referents(Box<TypeExpr>),
    Set(Box<TypeExpr>),
    Group(Box<TypeExpr>),
    List(Box<TypeExpr>),
    Interval(Box<TypeExpr>),
    Tuple(Vec<TypeExpr>),
    Function {
        parameters: Vec<TypeExpr>,
        result: Box<TypeExpr>,
    },
    Predicate(Row),
    ReferenceComputation(Box<TypeExpr>),
    Act(Force),
    Query(Vec<TypeExpr>),
    AnswerSelection(Vec<TypeExpr>),
    GeneralizedQuantifier(Box<TypeExpr>),
    Sign(SignKind),
    SignToken(SignKind),
    PlaceOf {
        relation: RelationRef,
        accepted_type: Box<TypeExpr>,
        candidates: Option<PlaceCandidates>,
    },
}

impl TypeExpr {
    /// Test the exact declared subtype relation.
    #[requires(true)]
    #[ensures(ret || self != supertype)]
    pub fn is_subtype_of(&self, supertype: &Self) -> bool {
        if self == supertype {
            return true;
        }
        match (self, supertype) {
            (Self::Atom(sub), Self::Atom(sup)) => primitive_subtype(*sub, *sup),
            (Self::Referents(sub), Self::Referents(sup)) => sub.is_subtype_of(sup),
            (sub, Self::Atom(TypeAtom::Entity)) => is_constructed_first_order(sub),
            _ => false,
        }
    }

    /// Determine the one permitted implicit conversion class.
    #[requires(true)]
    #[ensures(ret.is_some() -> self == expected || !matches!(ret, Some(ImplicitConversion::Identity)))]
    pub fn implicit_conversion_to(&self, expected: &Self) -> Option<ImplicitConversion> {
        if self == expected {
            return Some(ImplicitConversion::Identity);
        }
        if self.is_subtype_of(expected) {
            return Some(match (self, expected) {
                (Self::Referents(_), Self::Referents(_)) => {
                    ImplicitConversion::CovariantReferentsUpcast
                }
                _ => ImplicitConversion::Upcast,
            });
        }
        if self == &Self::Atom(TypeAtom::Natural) && expected == &Self::Atom(TypeAtom::Cardinal) {
            return Some(ImplicitConversion::NaturalToCardinal);
        }
        if self == &Self::Atom(TypeAtom::Natural)
            && expected == &Self::Referents(Box::new(Self::Atom(TypeAtom::Cardinal)))
        {
            return Some(ImplicitConversion::NaturalToCardinalSingletonLift);
        }
        if self == &Self::Referents(Box::new(Self::Atom(TypeAtom::Natural)))
            && expected == &Self::Referents(Box::new(Self::Atom(TypeAtom::Cardinal)))
        {
            return Some(ImplicitConversion::PointwiseNaturalToCardinal);
        }
        let Self::Referents(expected_inner) = expected else {
            return None;
        };
        if self == expected_inner.as_ref() || self.is_subtype_of(expected_inner) {
            return Some(ImplicitConversion::SingletonLift);
        }
        None
    }
}

/// The complete implicit-conversion vocabulary.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplicitConversion {
    Identity,
    Upcast,
    CovariantReferentsUpcast,
    NaturalToCardinal,
    NaturalToCardinalSingletonLift,
    PointwiseNaturalToCardinal,
    SingletonLift,
}

/// Type/parser failure with stable human-readable context.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParseError {
    message: String,
}

impl TypeParseError {
    /// Construct a nonempty parser failure.
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    pub(crate) fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        assert!(!message.is_empty(), "type parser errors require a message");
        new!(TypeParseError { message })
    }
}

impl fmt::Display for TypeParseError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TypeParseError {}

/// Check the primitive subtype DAG exactly as declared in section 3.1.
#[requires(sub != sup)]
#[ensures(true)]
fn primitive_subtype(sub: TypeAtom, sup: TypeAtom) -> bool {
    if sub == TypeAtom::Natural && sup == TypeAtom::Number {
        return true;
    }
    if sup == TypeAtom::Entity
        && matches!(
            sub,
            TypeAtom::Eventuality
                | TypeAtom::Location
                | TypeAtom::Amount
                | TypeAtom::Scale
                | TypeAtom::TruthValue
                | TypeAtom::Epistemology
                | TypeAtom::Concept
                | TypeAtom::AbstractNature
                | TypeAtom::Proposition
                | TypeAtom::Question
                | TypeAtom::Text
                | TypeAtom::Number
                | TypeAtom::Natural
                | TypeAtom::Cardinal
        )
    {
        return true;
    }
    if sup == TypeAtom::Eventuality
        && matches!(
            sub,
            TypeAtom::Achievement
                | TypeAtom::Process
                | TypeAtom::Activity
                | TypeAtom::State
                | TypeAtom::Experience
                | TypeAtom::Locution
        )
    {
        return true;
    }
    sup == TypeAtom::Entity
        && matches!(
            sub,
            TypeAtom::Achievement
                | TypeAtom::Process
                | TypeAtom::Activity
                | TypeAtom::State
                | TypeAtom::Experience
                | TypeAtom::Locution
        )
}

/// Identify constructed first-order object sorts.
#[requires(true)]
#[ensures(true)]
fn is_constructed_first_order(value: &TypeExpr) -> bool {
    matches!(
        value,
        TypeExpr::Set(_)
            | TypeExpr::Group(_)
            | TypeExpr::List(_)
            | TypeExpr::Tuple(_)
            | TypeExpr::Interval(_)
            | TypeExpr::Sign(_)
            | TypeExpr::SignToken(_)
            | TypeExpr::Atom(TypeAtom::UtteranceToken)
    )
}

/// Validate canonical row order: numbered ascending, then at most one event.
#[requires(true)]
#[ensures(true)]
pub(crate) fn row_slots_are_canonical(slots: &[RowSlot]) -> bool {
    let mut last = None::<&PositiveInteger>;
    let mut saw_event = false;
    for slot in slots {
        match &slot.label {
            PlaceLabel::Numbered(place) => {
                if saw_event || last.is_some_and(|last| place <= last) {
                    return false;
                }
                last = Some(place);
            }
            PlaceLabel::Eventuality => {
                if saw_event {
                    return false;
                }
                saw_event = true;
            }
        }
    }
    true
}

/// Test label uniqueness.
#[requires(true)]
#[ensures(true)]
pub(crate) fn labels_are_unique(labels: &[PlaceLabel]) -> bool {
    let mut seen = BTreeSet::new();
    labels.iter().all(|label| seen.insert(label.clone()))
}

/// Validate lowercase or escaped lowercase relation spelling.
#[requires(true)]
#[ensures(true)]
fn is_lexical_root_token(text: &str) -> bool {
    if !text.nfc().eq(text.chars()) {
        return false;
    }
    if let Some(inner) = text
        .strip_prefix('|')
        .and_then(|text| text.strip_suffix('|'))
    {
        let mut escaped = false;
        let mut decoded = String::new();
        for character in inner.chars() {
            if escaped {
                if !matches!(character, '|' | '\\') {
                    return false;
                }
                decoded.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '|' {
                return false;
            } else {
                decoded.push(character);
            }
        }
        return !escaped && decoded.chars().next().is_some_and(char::is_lowercase);
    }
    // Specification section 2.1 reserves `λ` as the lambda special-form marker
    // rather than a callable atom, so the bare glyph is not a lexical root even
    // though it is a lowercase alphabetic character. The escaped spelling above
    // stays available: `|λ|` cannot be confused with the marker.
    if text == "λ" {
        return false;
    }
    is_symbol_name(text) && text.chars().next().is_some_and(char::is_lowercase)
}

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn subtype_and_implicit_conversions_are_one_way() {
        let achievement = TypeExpr::Atom(TypeAtom::Achievement);
        let eventuality = TypeExpr::Atom(TypeAtom::Eventuality);
        let entity = TypeExpr::Atom(TypeAtom::Entity);
        let natural = TypeExpr::Atom(TypeAtom::Natural);
        let cardinal = TypeExpr::Atom(TypeAtom::Cardinal);
        let natural_referents = TypeExpr::Referents(Box::new(natural.clone()));
        let cardinal_referents = TypeExpr::Referents(Box::new(cardinal.clone()));
        assert!(achievement.is_subtype_of(&eventuality));
        assert!(achievement.is_subtype_of(&entity));
        assert!(!entity.is_subtype_of(&eventuality));
        assert!(natural.is_subtype_of(&entity));
        assert_eq!(
            natural.implicit_conversion_to(&cardinal),
            Some(ImplicitConversion::NaturalToCardinal)
        );
        assert_eq!(
            natural.implicit_conversion_to(&cardinal_referents),
            Some(ImplicitConversion::NaturalToCardinalSingletonLift)
        );
        assert_eq!(
            natural_referents.implicit_conversion_to(&cardinal_referents),
            Some(ImplicitConversion::PointwiseNaturalToCardinal)
        );
        assert_eq!(cardinal.implicit_conversion_to(&natural), None);
        assert_eq!(cardinal_referents.implicit_conversion_to(&natural), None);
        assert_eq!(
            cardinal_referents.implicit_conversion_to(&natural_referents),
            None
        );
        assert_eq!(
            entity.implicit_conversion_to(&TypeExpr::Referents(Box::new(entity.clone()))),
            Some(ImplicitConversion::SingletonLift)
        );
        assert!(
            TypeExpr::Referents(Box::new(eventuality))
                .implicit_conversion_to(&TypeExpr::Referents(Box::new(entity)))
                .is_some()
        );
    }
}

/// Closed answer-polarity literals.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnswerPolarity {
    Yes,
    No,
    Unknown,
}

impl AnswerPolarity {
    /// Parse a closed polarity literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "Yes" | "No" | "Unknown"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Yes" => Some(Self::Yes),
            "No" => Some(Self::No),
            "Unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "Yes",
            Self::No => "No",
            Self::Unknown => "Unknown",
        }
    }
}

/// Closed answer-exhaustivity literals.
///
/// Omitting the operand is the canonical spelling of genuinely undetermined
/// exhaustivity (section 12.2), so this enum has no third "unknown" member.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnswerExhaustivity {
    Exhaustive,
    MentionSome,
}

impl AnswerExhaustivity {
    /// Parse a closed exhaustivity literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "Exhaustive" | "MentionSome"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Exhaustive" => Some(Self::Exhaustive),
            "MentionSome" => Some(Self::MentionSome),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exhaustive => "Exhaustive",
            Self::MentionSome => "MentionSome",
        }
    }
}

/// Closed label-level literals.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LabelLevel {
    Item,
    Division,
}

impl LabelLevel {
    /// Parse a closed label-level literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "Item" | "Division"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Item" => Some(Self::Item),
            "Division" => Some(Self::Division),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Item => "Item",
            Self::Division => "Division",
        }
    }
}

/// Closed endpoint-inclusion literals.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EndpointInclusion {
    Open,
    Closed,
}

impl EndpointInclusion {
    /// Parse a closed endpoint-inclusion literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "Open" | "Closed"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Open" => Some(Self::Open),
            "Closed" => Some(Self::Closed),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Closed => "Closed",
        }
    }
}

/// Closed deictic-proximity literals.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Proximity {
    Proximal,
    Medial,
    Distal,
}

impl Proximity {
    /// Parse a closed proximity literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "Proximal" | "Medial" | "Distal"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Proximal" => Some(Self::Proximal),
            "Medial" => Some(Self::Medial),
            "Distal" => Some(Self::Distal),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proximal => "Proximal",
            Self::Medial => "Medial",
            Self::Distal => "Distal",
        }
    }
}

/// Closed lexical scope-policy literals.
///
/// Section 6.3 makes these a property of the semantic place; missing or
/// contradictory policy metadata fails closed rather than being guessed.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LexicalScopePolicy {
    Extensional,
    Intensional,
    Opaque,
}

impl LexicalScopePolicy {
    /// Parse a closed scope-policy literal.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(text, "Extensional" | "Intensional" | "Opaque"))]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Extensional" => Some(Self::Extensional),
            "Intensional" => Some(Self::Intensional),
            "Opaque" => Some(Self::Opaque),
            _ => None,
        }
    }

    /// Return the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Extensional => "Extensional",
            Self::Intensional => "Intensional",
            Self::Opaque => "Opaque",
        }
    }
}

/// NFC text carried by a `Text` literal.
#[invariant(text.nfc().eq(text.chars()))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextLiteral {
    text: String,
}

impl TextLiteral {
    /// Normalize caller text for kernel construction.
    #[requires(true)]
    #[ensures(ret.as_str().nfc().eq(ret.as_str().chars()))]
    pub fn new(text: impl AsRef<str>) -> Self {
        new!(TextLiteral {
            text: text.as_ref().nfc().collect(),
        })
    }

    /// Borrow the normalized text.
    #[requires(true)]
    #[ensures(ret.nfc().eq(ret.chars()))]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// A member of the versioned generated scale table.
///
/// The table itself lives above the kernel with the rest of the generated
/// registries, so the kernel enforces the spelling namespace — a scale literal
/// is a PascalCase atom, never a minted lowercase root — and the registry layer
/// enforces membership.
#[invariant(is_pascal_case_symbol(&text))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScaleName {
    text: String,
}

impl ScaleName {
    /// Parse a scale-literal spelling.
    #[requires(true)]
    #[ensures(ret.is_ok() == is_pascal_case_symbol(text))]
    pub fn try_new(text: &str) -> Result<Self, TypeParseError> {
        if !is_pascal_case_symbol(text) {
            return Err(TypeParseError::new("a scale literal is a PascalCase atom"));
        }
        Ok(new!(ScaleName {
            text: text.to_owned(),
        }))
    }

    /// Borrow the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// A relation from a versioned generated table, such as an indicator relation.
///
/// Generated relation tables are closed by the registry layer, not by the
/// kernel; what the kernel enforces is that such a relation is applied only
/// through a declared signature carried at the application site.
#[invariant(is_pascal_case_symbol(&text))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegisteredName {
    text: String,
}

impl RegisteredName {
    /// Parse a generated relation spelling.
    #[requires(true)]
    #[ensures(ret.is_ok() == is_pascal_case_symbol(text))]
    pub fn try_new(text: &str) -> Result<Self, TypeParseError> {
        if !is_pascal_case_symbol(text) {
            return Err(TypeParseError::new(
                "a generated relation name is a PascalCase atom",
            ));
        }
        Ok(new!(RegisteredName {
            text: text.to_owned(),
        }))
    }

    /// Borrow the canonical spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Validate a PascalCase registered spelling.
#[requires(true)]
#[ensures(ret -> is_symbol_name(text))]
pub(crate) fn is_pascal_case_symbol(text: &str) -> bool {
    text.nfc().eq(text.chars())
        && is_symbol_name(text)
        && text.chars().next().is_some_and(char::is_uppercase)
}
