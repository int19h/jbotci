//! Version-0 smusni type, row, and application kernel.
//!
//! These types are independent of the semantic renderer.  They validate the
//! concrete type grammar used by documents and by registry templates, and
//! model the two deliberately distinct application operations: positional
//! function application and labelled predicate-place filling.

use std::collections::BTreeSet;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use num_bigint::BigUint;
use unicode_normalization::UnicodeNormalization;

use super::datum::Datum;

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

    const ALL: [Self; 37] = [
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
    pub(super) fn as_biguint(&self) -> &BigUint {
        &self.value
    }

    /// Convert the exact value to an integer datum.
    #[requires(true)]
    #[ensures(ret.as_integer().is_some_and(|text| is_positive_integer_text(text)))]
    fn to_datum(&self) -> Datum {
        Datum::Integer(
            super::datum::Integer::try_new(&self.value.to_string())
                .expect("positive integers have canonical decimal spellings"),
        )
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

    /// Convert the label to its canonical datum token.
    #[requires(true)]
    #[ensures(ret.as_atom() == Some("Eventuality") || ret.as_integer().is_some())]
    fn to_datum(&self) -> Datum {
        match self {
            Self::Numbered(value) => value.to_datum(),
            Self::Eventuality => Datum::atom("Eventuality"),
        }
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

    /// Report whether an unknown numbered tail survives.
    #[requires(true)]
    #[ensures(ret == self.open_numbered_tail)]
    pub fn has_open_numbered_tail(&self) -> bool {
        self.open_numbered_tail
    }

    /// Return the canonical type-position datum.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Row"))]
    pub fn to_datum(&self) -> Datum {
        let mut children = self
            .slots
            .iter()
            .map(|slot| Datum::list([slot.label.to_datum(), slot.accepted_type.to_datum()]))
            .collect::<Vec<_>>();
        if self.open_numbered_tail {
            children.push(Datum::atom("Open"));
        }
        Datum::form("Row", children)
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

impl RelationRef {
    /// Convert a relation identity back to its canonical syntax.
    #[requires(true)]
    #[ensures(true)]
    pub fn to_datum(&self) -> Datum {
        match self {
            Self::Lexical(root) => Datum::atom(root.as_str()),
            Self::Variable(variable) => Datum::atom(variable.as_str()),
            Self::DropPlace { relation, place } => {
                Datum::form("DropPlace", [relation.to_datum(), place.to_datum()])
            }
            Self::Tanru { modifier, head } => {
                Datum::form("Tanru", [modifier.to_datum(), head.to_datum()])
            }
            Self::Scalar { kind, relation } => {
                Datum::form("Scalar", [Datum::atom(kind.as_str()), relation.to_datum()])
            }
        }
    }
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
    /// Parse the complete version-0 type grammar.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn parse(datum: &Datum) -> Result<Self, TypeParseError> {
        parse_type(datum)
    }

    /// Serialize this type with the exact type-position grammar.
    #[requires(true)]
    #[ensures(true)]
    pub fn to_datum(&self) -> Datum {
        match self {
            Self::Atom(atom) => Datum::atom(atom.as_str()),
            Self::Referents(inner) => Datum::form("Referents", [inner.to_datum()]),
            Self::Set(inner) => Datum::form("Set", [inner.to_datum()]),
            Self::Group(inner) => Datum::form("Group", [inner.to_datum()]),
            Self::List(inner) => Datum::form("List", [inner.to_datum()]),
            Self::Interval(inner) => Datum::form("Interval", [inner.to_datum()]),
            Self::Tuple(elements) => {
                Datum::form("Tuple", [Datum::list(elements.iter().map(Self::to_datum))])
            }
            Self::Function { parameters, result } => Datum::form(
                "Fn",
                [
                    Datum::list(parameters.iter().map(Self::to_datum)),
                    result.to_datum(),
                ],
            ),
            Self::Predicate(row) => Datum::form("PredTerm", [row.to_datum()]),
            Self::ReferenceComputation(inner) => Datum::form("RefComp", [inner.to_datum()]),
            Self::Act(force) => Datum::form("Act", [Datum::atom(force.as_str())]),
            Self::Query(elements) => {
                Datum::form("Query", [Datum::list(elements.iter().map(Self::to_datum))])
            }
            Self::AnswerSelection(elements) => Datum::form(
                "AnswerSelection",
                [Datum::list(elements.iter().map(Self::to_datum))],
            ),
            Self::GeneralizedQuantifier(inner) => Datum::form("GQ", [inner.to_datum()]),
            Self::Sign(kind) => Datum::form("Sign", [Datum::atom(kind.as_str())]),
            Self::SignToken(kind) => Datum::form("SignToken", [Datum::atom(kind.as_str())]),
            Self::PlaceOf {
                relation,
                accepted_type,
                candidates,
            } => {
                let mut values = vec![relation.to_datum(), accepted_type.to_datum()];
                if let Some(candidates) = candidates {
                    values.push(Datum::list(
                        candidates.as_slice().iter().map(PlaceLabel::to_datum),
                    ));
                }
                Datum::form("PlaceOf", values)
            }
        }
    }

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

/// One typed argument to predicate-place application.
#[invariant(::Plain { .. } => true)]
#[invariant(::Numbered { .. } => true)]
#[invariant(::Eventuality { .. } => true)]
#[invariant(::Computed { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateArgument {
    Plain {
        value_type: TypeExpr,
    },
    Numbered {
        place: PositiveInteger,
        value_type: TypeExpr,
    },
    Eventuality {
        value_type: TypeExpr,
    },
    Computed {
        place_type: TypeExpr,
        candidates: ComputedPlaceDomain,
        value_type: TypeExpr,
    },
}

impl PredicateArgument {
    /// Construct a computed `At` argument with a nonempty injective domain.
    #[requires(!candidates.is_empty() && labels_are_unique(&candidates))]
    #[ensures(matches!(ret, Self::Computed { .. }))]
    pub fn computed(
        place_type: TypeExpr,
        candidates: Vec<PlaceLabel>,
        value_type: TypeExpr,
    ) -> Self {
        Self::Computed {
            place_type,
            candidates: ComputedPlaceDomain::new(candidates),
            value_type,
        }
    }
}

/// A typed predicate term at one application site.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateSignature {
    relation: RelationRef,
    row: Row,
}

impl PredicateSignature {
    /// Construct a typed predicate signature.
    #[requires(true)]
    #[ensures(ret.relation == old(relation.clone()) && ret.row == old(row.clone()))]
    pub fn new(relation: RelationRef, row: Row) -> Self {
        Self { relation, row }
    }

    /// Borrow the relation identity.
    #[requires(true)]
    #[ensures(true)]
    pub fn relation(&self) -> &RelationRef {
        &self.relation
    }

    /// Borrow the effective row.
    #[requires(true)]
    #[ensures(true)]
    pub fn row(&self) -> &Row {
        &self.row
    }

    /// Apply ordinary, labelled, event, and computed fills without conflating
    /// any of their cursor behavior.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn apply(
        &self,
        arguments: &[PredicateArgument],
    ) -> Result<PredicateApplicationResult, ApplicationTypeError> {
        apply_predicate(self, arguments)
    }

    /// Delete one numbered place while preserving every surviving label.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|signature| signature.row.slots.len() + 1 == self.row.slots.len()) || ret.is_err())]
    pub fn drop_place(&self, place: PositiveInteger) -> Result<Self, ApplicationTypeError> {
        let mut slots = self.row.slots.clone();
        let Some(index) = slots
            .iter()
            .position(|slot| matches!(&slot.label, PlaceLabel::Numbered(label) if label == &place))
        else {
            return Err(ApplicationTypeError::new(
                "DropPlace target is not in the current row",
            ));
        };
        slots.remove(index);
        Ok(Self::new(
            RelationRef::DropPlace {
                relation: Box::new(self.relation.clone()),
                place,
            },
            Row::new(slots, self.row.open_numbered_tail),
        ))
    }
}

/// The statically known remainder after predicate application.
#[invariant(row_slots_are_canonical(&remaining_slots))]
#[invariant(computed_domains.iter().all(|domain| !domain.is_empty() && labels_are_unique(domain)))]
#[invariant(domains_are_pairwise_disjoint(&computed_domains))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateApplicationResult {
    remaining_slots: Vec<RowSlot>,
    open_numbered_tail: bool,
    computed_domains: Vec<Vec<PlaceLabel>>,
}

impl PredicateApplicationResult {
    /// Borrow statically unfilled slots. Computed domains remain separately
    /// reserved because answer substitution chooses which one is consumed.
    #[requires(true)]
    #[ensures(row_slots_are_canonical(ret))]
    pub fn remaining_slots(&self) -> &[RowSlot] {
        &self.remaining_slots
    }

    /// Borrow the ordered computed-place domains.
    #[requires(true)]
    #[ensures(domains_are_pairwise_disjoint(ret))]
    pub fn computed_domains(&self) -> &[Vec<PlaceLabel>] {
        &self.computed_domains
    }
}

/// An ordered function signature.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    parameters: Vec<TypeExpr>,
    result: TypeExpr,
}

impl FunctionSignature {
    /// Construct an ordered function signature, including nullary functions.
    #[requires(true)]
    #[ensures(ret.parameters.len() == old(parameters.len()) && ret.result == old(result.clone()))]
    pub fn new(parameters: Vec<TypeExpr>, result: TypeExpr) -> Self {
        Self { parameters, result }
    }

    /// Unify every argument against the complete signature.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|result| *result == &self.result) || ret.is_err())]
    pub fn apply(&self, arguments: &[TypeExpr]) -> Result<&TypeExpr, ApplicationTypeError> {
        if arguments.len() != self.parameters.len() {
            return Err(ApplicationTypeError::new(
                "function application arity mismatch",
            ));
        }
        if arguments
            .iter()
            .zip(&self.parameters)
            .any(|(actual, expected)| actual.implicit_conversion_to(expected).is_none())
        {
            return Err(ApplicationTypeError::new("function argument type mismatch"));
        }
        Ok(&self.result)
    }
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
    fn new(message: impl Into<String>) -> Self {
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

/// Predicate/function application failure.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationTypeError {
    message: String,
}

impl ApplicationTypeError {
    /// Construct a nonempty application failure.
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        assert!(!message.is_empty(), "application errors require a message");
        new!(ApplicationTypeError { message })
    }
}

impl fmt::Display for ApplicationTypeError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApplicationTypeError {}

/// Parse the type grammar recursively.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_type(datum: &Datum) -> Result<TypeExpr, TypeParseError> {
    if let Some(atom) = datum.as_atom() {
        return TypeAtom::parse(atom)
            .map(TypeExpr::Atom)
            .ok_or_else(|| TypeParseError::new(format!("unknown type atom {atom:?}")));
    }
    let items = datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("type must be an atom or type-constructor form"))?;
    let head = items
        .first()
        .and_then(Datum::as_atom)
        .ok_or_else(|| TypeParseError::new("type constructor must be a symbol"))?;
    match head {
        "Referents" | "Set" | "Group" | "List" | "Interval" | "RefComp" | "GQ" => {
            require_len(items, 2, "unary type constructor")?;
            let inner = Box::new(parse_type(&items[1])?);
            Ok(match head {
                "Referents" => TypeExpr::Referents(inner),
                "Set" => TypeExpr::Set(inner),
                "Group" => TypeExpr::Group(inner),
                "List" => TypeExpr::List(inner),
                "Interval" => TypeExpr::Interval(inner),
                "RefComp" => TypeExpr::ReferenceComputation(inner),
                "GQ" => TypeExpr::GeneralizedQuantifier(inner),
                _ => unreachable!("closed match"),
            })
        }
        "Tuple" | "Query" | "AnswerSelection" => {
            require_len(items, 2, "tuple-parameter type constructor")?;
            let elements = parse_type_list(&items[1])?;
            Ok(match head {
                "Tuple" => TypeExpr::Tuple(elements),
                "Query" => TypeExpr::Query(elements),
                "AnswerSelection" => TypeExpr::AnswerSelection(elements),
                _ => unreachable!("closed match"),
            })
        }
        "Fn" => {
            require_len(items, 3, "Fn")?;
            Ok(TypeExpr::Function {
                parameters: parse_type_list(&items[1])?,
                result: Box::new(parse_type(&items[2])?),
            })
        }
        "PredTerm" => {
            require_len(items, 2, "PredTerm")?;
            Ok(TypeExpr::Predicate(parse_row(&items[1])?))
        }
        "Act" => {
            require_len(items, 2, "Act")?;
            let force = items[1]
                .as_atom()
                .and_then(Force::parse)
                .ok_or_else(|| TypeParseError::new("Act requires a closed force literal"))?;
            Ok(TypeExpr::Act(force))
        }
        "Sign" | "SignToken" => {
            require_len(items, 2, head)?;
            let kind = items[1]
                .as_atom()
                .and_then(SignKind::parse)
                .ok_or_else(|| TypeParseError::new("sign type requires a closed sign kind"))?;
            Ok(if head == "Sign" {
                TypeExpr::Sign(kind)
            } else {
                TypeExpr::SignToken(kind)
            })
        }
        "PlaceOf" => parse_place_of(items),
        _ => Err(TypeParseError::new(format!(
            "unknown type constructor {head:?}"
        ))),
    }
}

/// Parse a parenthesized list of types.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_type_list(datum: &Datum) -> Result<Vec<TypeExpr>, TypeParseError> {
    datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("type parameter tuple must be a list"))?
        .iter()
        .map(parse_type)
        .collect()
}

/// Parse a row with unique, ordered labels and optional final `Open` marker.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_row(datum: &Datum) -> Result<Row, TypeParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("row must be a Row form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Row") {
        return Err(TypeParseError::new("row must begin with Row"));
    }
    let open_numbered_tail = items.last().and_then(Datum::as_atom) == Some("Open");
    let slot_end = items.len() - usize::from(open_numbered_tail);
    let mut slots = Vec::new();
    for slot in &items[1..slot_end] {
        let fields = slot
            .as_list()
            .ok_or_else(|| TypeParseError::new("row slot must be a two-item list"))?;
        require_len(fields, 2, "row slot")?;
        let label = parse_place_label(&fields[0])?;
        slots.push(RowSlot::new(label, parse_type(&fields[1])?));
    }
    if !row_slots_are_canonical(&slots) {
        return Err(TypeParseError::new(
            "row slots must be unique and ordered, with Eventuality last",
        ));
    }
    Ok(Row::new(slots, open_numbered_tail))
}

/// Parse a `PlaceOf` type, including its optional explicit candidate set.
#[requires(items.first().and_then(Datum::as_atom) == Some("PlaceOf"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_place_of(items: &[Datum]) -> Result<TypeExpr, TypeParseError> {
    if !matches!(items.len(), 3 | 4) {
        return Err(TypeParseError::new(
            "PlaceOf requires relation, type, and optional candidates",
        ));
    }
    let candidate_labels = items.get(3).map(parse_label_list).transpose()?;
    if candidate_labels.as_ref().is_some_and(Vec::is_empty) {
        return Err(TypeParseError::new(
            "explicit PlaceOf candidates cannot be empty",
        ));
    }
    if candidate_labels
        .as_ref()
        .is_some_and(|labels| !labels_are_unique(labels))
    {
        return Err(TypeParseError::new("PlaceOf candidates must be unique"));
    }
    let candidates = candidate_labels.map(PlaceCandidates::new);
    Ok(TypeExpr::PlaceOf {
        relation: parse_relation_ref(&items[1])?,
        accepted_type: Box::new(parse_type(&items[2])?),
        candidates,
    })
}

/// Parse one relation reference recursively.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_relation_ref(datum: &Datum) -> Result<RelationRef, TypeParseError> {
    if let Some(atom) = datum.as_atom() {
        if atom.starts_with('$') {
            return Variable::try_new(atom).map(RelationRef::Variable);
        }
        return LexicalRoot::try_new(atom).map(RelationRef::Lexical);
    }
    let items = datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("relation reference has invalid shape"))?;
    let head = items.first().and_then(Datum::as_atom);
    match head {
        Some("DropPlace") => {
            require_len(items, 3, "DropPlace relation reference")?;
            let place = parse_positive_integer(&items[2])?;
            Ok(RelationRef::DropPlace {
                relation: Box::new(parse_relation_ref(&items[1])?),
                place,
            })
        }
        Some("Tanru") => {
            require_len(items, 3, "Tanru relation reference")?;
            Ok(RelationRef::Tanru {
                modifier: Box::new(parse_relation_ref(&items[1])?),
                head: Box::new(parse_relation_ref(&items[2])?),
            })
        }
        Some("Scalar") => {
            require_len(items, 3, "Scalar relation reference")?;
            let kind = items[1]
                .as_atom()
                .and_then(ScalarKind::parse)
                .ok_or_else(|| TypeParseError::new("Scalar requires a closed scalar kind"))?;
            Ok(RelationRef::Scalar {
                kind,
                relation: Box::new(parse_relation_ref(&items[2])?),
            })
        }
        _ => Err(TypeParseError::new("unknown relation-reference form")),
    }
}

/// Parse a place-label list.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_label_list(datum: &Datum) -> Result<Vec<PlaceLabel>, TypeParseError> {
    datum
        .as_list()
        .ok_or_else(|| TypeParseError::new("candidate labels must be a list"))?
        .iter()
        .map(parse_place_label)
        .collect()
}

/// Parse one positive numbered or Eventuality label.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_place_label(datum: &Datum) -> Result<PlaceLabel, TypeParseError> {
    if datum.as_atom() == Some("Eventuality") {
        return Ok(PlaceLabel::Eventuality);
    }
    parse_positive_integer(datum).map(PlaceLabel::Numbered)
}

/// Parse an arbitrarily large positive label value.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_positive_integer(datum: &Datum) -> Result<PositiveInteger, TypeParseError> {
    datum
        .as_integer()
        .ok_or_else(|| TypeParseError::new("place label must be a positive integer"))
        .and_then(PositiveInteger::try_new)
}

/// Apply one predicate signature with the exact fill-cursor rules.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn apply_predicate(
    signature: &PredicateSignature,
    arguments: &[PredicateArgument],
) -> Result<PredicateApplicationResult, ApplicationTypeError> {
    let mut remaining = signature.row.slots.clone();
    let mut cursor_after = None::<PositiveInteger>;
    let mut computed_seen = false;
    let mut computed_domains = Vec::<Vec<PlaceLabel>>::new();
    for argument in arguments {
        match argument {
            PredicateArgument::Plain { value_type } => {
                if computed_seen {
                    return Err(ApplicationTypeError::new(
                        "plain fills cannot follow a computed At fill",
                    ));
                }
                let Some(index) = remaining.iter().position(|slot| {
                    matches!(&slot.label, PlaceLabel::Numbered(place) if cursor_after.as_ref().is_none_or(|cursor| place > cursor))
                }) else {
                    return Err(ApplicationTypeError::new("no current numbered cursor place"));
                };
                ensure_accepted(value_type, remaining[index].accepted_type())?;
                let label = remaining[index].label.clone();
                remaining.remove(index);
                let PlaceLabel::Numbered(place) = label else {
                    unreachable!("cursor selects only numbered places")
                };
                cursor_after = Some(place);
            }
            PredicateArgument::Numbered { place, value_type } => {
                if computed_seen {
                    return Err(ApplicationTypeError::new(
                        "literal labelled fills cannot follow computed At",
                    ));
                }
                let Some(index) = remaining
                    .iter()
                    .position(|slot| slot.label == PlaceLabel::Numbered(place.clone()))
                else {
                    return Err(ApplicationTypeError::new(
                        "literal place is absent, deleted, or already filled",
                    ));
                };
                ensure_accepted(value_type, remaining[index].accepted_type())?;
                remaining.remove(index);
                cursor_after = Some(place.clone());
            }
            PredicateArgument::Eventuality { value_type } => {
                if computed_seen {
                    return Err(ApplicationTypeError::new(
                        "event fills cannot follow a computed At fill",
                    ));
                }
                let Some(index) = remaining
                    .iter()
                    .position(|slot| slot.label == PlaceLabel::Eventuality)
                else {
                    return Err(ApplicationTypeError::new(
                        "event place is absent or already filled",
                    ));
                };
                ensure_accepted(value_type, remaining[index].accepted_type())?;
                remaining.remove(index);
            }
            PredicateArgument::Computed {
                place_type,
                candidates,
                value_type,
            } => {
                computed_seen = true;
                let TypeExpr::PlaceOf {
                    relation,
                    accepted_type,
                    candidates: declared,
                } = place_type
                else {
                    return Err(ApplicationTypeError::new(
                        "At place expression must have PlaceOf type",
                    ));
                };
                if relation != &signature.relation {
                    return Err(ApplicationTypeError::new(
                        "At relation identity does not match its predicate host",
                    ));
                }
                ensure_accepted(value_type, accepted_type)?;
                let candidates = candidates.as_slice();
                if declared
                    .as_ref()
                    .is_some_and(|labels| labels.as_slice() != candidates)
                {
                    return Err(ApplicationTypeError::new(
                        "At candidates disagree with the explicit PlaceOf domain",
                    ));
                }
                if declared.is_none()
                    && candidates
                        .iter()
                        .any(|label| matches!(label, PlaceLabel::Eventuality))
                {
                    return Err(ApplicationTypeError::new(
                        "an omitted PlaceOf candidate set derives numbered places only",
                    ));
                }
                if declared.is_none() {
                    let derived = remaining
                        .iter()
                        .filter(|slot| matches!(slot.label, PlaceLabel::Numbered(_)))
                        .filter(|slot| {
                            accepted_type
                                .implicit_conversion_to(slot.accepted_type())
                                .is_some()
                        })
                        .map(RowSlot::label)
                        .collect::<Vec<_>>();
                    if candidates != derived {
                        return Err(ApplicationTypeError::new(
                            "omitted PlaceOf candidates are not the exact derivable domain",
                        ));
                    }
                }
                for label in candidates {
                    let Some(slot) = remaining.iter().find(|slot| slot.label == *label) else {
                        return Err(ApplicationTypeError::new(
                            "At candidate is absent, reserved, or deleted",
                        ));
                    };
                    if accepted_type
                        .implicit_conversion_to(slot.accepted_type())
                        .is_none()
                    {
                        return Err(ApplicationTypeError::new(
                            "At candidate does not accept its declared value type",
                        ));
                    }
                }
                if computed_domains
                    .iter()
                    .any(|prior| prior.iter().any(|label| candidates.contains(label)))
                {
                    return Err(ApplicationTypeError::new(
                        "computed At candidate domains overlap",
                    ));
                }
                computed_domains.push(candidates.to_vec());
            }
        }
    }
    Ok(new!(PredicateApplicationResult {
        remaining_slots: remaining,
        open_numbered_tail: signature.row.open_numbered_tail,
        computed_domains,
    }))
}

/// Require an implicit conversion into one operand type.
#[requires(true)]
#[ensures(ret.is_ok() == actual.implicit_conversion_to(expected).is_some())]
fn ensure_accepted(actual: &TypeExpr, expected: &TypeExpr) -> Result<(), ApplicationTypeError> {
    if actual.implicit_conversion_to(expected).is_none() {
        return Err(ApplicationTypeError::new("predicate fill type mismatch"));
    }
    Ok(())
}

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
fn row_slots_are_canonical(slots: &[RowSlot]) -> bool {
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
fn labels_are_unique(labels: &[PlaceLabel]) -> bool {
    let mut seen = BTreeSet::new();
    labels.iter().all(|label| seen.insert(label.clone()))
}

/// Test pairwise disjoint computed domains.
#[requires(true)]
#[ensures(true)]
fn domains_are_pairwise_disjoint(domains: &[Vec<PlaceLabel>]) -> bool {
    let mut seen = BTreeSet::new();
    domains
        .iter()
        .flat_map(|domain| domain.iter())
        .all(|label| seen.insert(label.clone()))
}

/// Validate a raw symbol-name component.
#[requires(true)]
#[ensures(true)]
fn is_symbol_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters.next().is_some_and(char::is_alphabetic)
        && characters.all(|character| {
            character.is_alphanumeric() || matches!(character, '\'' | '-' | '_' | '.')
        })
}

/// Validate the exact unbounded positive-integer token grammar.
#[requires(true)]
#[ensures(true)]
fn is_positive_integer_text(text: &str) -> bool {
    !text.is_empty() && !text.starts_with('0') && text.bytes().all(|byte| byte.is_ascii_digit())
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
    is_symbol_name(text) && text.chars().next().is_some_and(char::is_lowercase)
}

/// Require an exact form length.
#[requires(!context.is_empty())]
#[ensures(ret.is_ok() == (items.len() == expected))]
fn require_len(items: &[Datum], expected: usize, context: &str) -> Result<(), TypeParseError> {
    if items.len() != expected {
        return Err(TypeParseError::new(format!(
            "{context} requires {} items, found {}",
            expected,
            items.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::super::datum::parse_document;
    use super::*;

    /// Parse one raw type specimen.
    #[requires(true)]
    #[ensures(true)]
    fn ty(text: &str) -> TypeExpr {
        TypeExpr::parse(&parse_document(text).expect("raw datum parses")).expect("type parses")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_type_constructor_round_trips_structurally() {
        for atom in TypeAtom::ALL {
            assert_eq!(ty(atom.as_str()), TypeExpr::Atom(atom));
        }
        let specimens = [
            "Entity",
            "(Referents Eventuality)",
            "(Set Entity)",
            "(Group Entity)",
            "(List Number)",
            "(Interval Number)",
            "(Tuple (Entity Number))",
            "(Fn (Entity Number) Content)",
            "(PredTerm (Row (1 (Referents Entity)) (Eventuality (Referents Eventuality)) Open))",
            "(RefComp (Referents Entity))",
            "(Act Assertion)",
            "(Query ())",
            "(AnswerSelection (Entity Number))",
            "(GQ Entity)",
            "(Sign Sentence)",
            "(SignToken Structured)",
            "(PlaceOf (DropPlace klama 3) (Referents Entity) (1 2 Eventuality))",
            "(PredTerm (Row (429496729600000000000000000000 Entity)))",
            "(PlaceOf (DropPlace klama 429496729600000000000000000000) Entity)",
        ];
        for specimen in specimens {
            let parsed = ty(specimen);
            assert_eq!(TypeExpr::parse(&parsed.to_datum()), Ok(parsed));
        }
    }

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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn predicate_fill_cursor_and_drop_place_preserve_labels() {
        let referents = TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity)));
        let relation = RelationRef::Lexical(LexicalRoot::try_new("klama").unwrap());
        let signature = PredicateSignature::new(
            relation,
            Row::new(
                vec![
                    RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(2), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(3), referents.clone()),
                ],
                false,
            ),
        );
        let result = signature
            .apply(&[
                PredicateArgument::Numbered {
                    place: PositiveInteger::from_u32(2),
                    value_type: referents.clone(),
                },
                PredicateArgument::Plain {
                    value_type: referents.clone(),
                },
            ])
            .unwrap();
        assert_eq!(
            result
                .remaining_slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1)]
        );
        let dropped = signature.drop_place(PositiveInteger::from_u32(2)).unwrap();
        assert_eq!(
            dropped
                .row()
                .slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1), PlaceLabel::numbered(3)]
        );

        let huge = PositiveInteger::try_new("429496729600000000000000000000").unwrap();
        let huge_signature = PredicateSignature::new(
            signature.relation().clone(),
            Row::new(
                vec![RowSlot::new(
                    PlaceLabel::Numbered(huge.clone()),
                    referents.clone(),
                )],
                false,
            ),
        );
        assert!(huge_signature.drop_place(huge).is_ok());

        let event_signature = PredicateSignature::new(
            signature.relation().clone(),
            Row::new(
                vec![
                    RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(2), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(3), referents.clone()),
                    RowSlot::new(
                        PlaceLabel::Eventuality,
                        TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                    ),
                ],
                false,
            ),
        );
        let after_event = event_signature
            .apply(&[
                PredicateArgument::Numbered {
                    place: PositiveInteger::from_u32(2),
                    value_type: referents.clone(),
                },
                PredicateArgument::Eventuality {
                    value_type: TypeExpr::Referents(Box::new(TypeExpr::Atom(
                        TypeAtom::Eventuality,
                    ))),
                },
                PredicateArgument::Plain {
                    value_type: referents,
                },
            ])
            .unwrap();
        assert_eq!(
            after_event
                .remaining_slots()
                .iter()
                .map(RowSlot::label)
                .collect::<Vec<_>>(),
            vec![PlaceLabel::numbered(1)]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn computed_at_domains_must_be_disjoint_and_relation_exact() {
        let referents = TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity)));
        let relation = RelationRef::Lexical(LexicalRoot::try_new("klama").unwrap());
        let signature = PredicateSignature::new(
            relation.clone(),
            Row::new(
                vec![
                    RowSlot::new(PlaceLabel::numbered(1), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(2), referents.clone()),
                    RowSlot::new(PlaceLabel::numbered(3), referents.clone()),
                ],
                false,
            ),
        );
        let place_type = TypeExpr::PlaceOf {
            relation,
            accepted_type: Box::new(referents.clone()),
            candidates: None,
        };
        let first = PredicateArgument::computed(
            place_type.clone(),
            vec![
                PlaceLabel::numbered(1),
                PlaceLabel::numbered(2),
                PlaceLabel::numbered(3),
            ],
            referents.clone(),
        );
        let second = PredicateArgument::computed(
            TypeExpr::PlaceOf {
                relation: signature.relation().clone(),
                accepted_type: Box::new(referents.clone()),
                candidates: Some(PlaceCandidates::new(vec![PlaceLabel::numbered(2)])),
            },
            vec![PlaceLabel::numbered(2)],
            referents,
        );
        assert!(signature.apply(&[first, second]).is_err());

        let omitted_subset = PredicateArgument::computed(
            TypeExpr::PlaceOf {
                relation: signature.relation().clone(),
                accepted_type: Box::new(TypeExpr::Referents(Box::new(TypeExpr::Atom(
                    TypeAtom::Entity,
                )))),
                candidates: None,
            },
            vec![PlaceLabel::numbered(1), PlaceLabel::numbered(2)],
            TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity))),
        );
        assert!(signature.apply(&[omitted_subset]).is_err());

        let explicit_event_type = TypeExpr::PlaceOf {
            relation: signature.relation().clone(),
            accepted_type: Box::new(TypeExpr::Referents(Box::new(TypeExpr::Atom(
                TypeAtom::Eventuality,
            )))),
            candidates: Some(PlaceCandidates::new(vec![PlaceLabel::Eventuality])),
        };
        let event_signature = PredicateSignature::new(
            signature.relation().clone(),
            Row::new(
                vec![RowSlot::new(
                    PlaceLabel::Eventuality,
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                )],
                false,
            ),
        );
        assert!(
            event_signature
                .apply(&[PredicateArgument::computed(
                    explicit_event_type,
                    vec![PlaceLabel::Eventuality],
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                )])
                .is_ok()
        );
        let derived_event_type = TypeExpr::PlaceOf {
            relation: event_signature.relation().clone(),
            accepted_type: Box::new(TypeExpr::Referents(Box::new(TypeExpr::Atom(
                TypeAtom::Eventuality,
            )))),
            candidates: None,
        };
        assert!(
            event_signature
                .apply(&[PredicateArgument::computed(
                    derived_event_type,
                    vec![PlaceLabel::Eventuality],
                    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Eventuality))),
                )])
                .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn malformed_type_row_and_relation_forms_are_rejected() {
        for specimen in [
            "BogusType",
            "(BogusType Entity)",
            "(Referents)",
            "(Act Opaque)",
            "(Sign BogusKind)",
            "(PredTerm NotARow)",
            "(PredTerm (Row (2 Entity) (1 Entity)))",
            "(PredTerm (Row (1 Entity) (1 Entity)))",
            "(PredTerm (Row (Eventuality Eventuality) (1 Entity)))",
            "(PredTerm (Row Open (1 Entity)))",
            "(PlaceOf klama Entity ())",
            "(PlaceOf klama Entity (1 1))",
            "(PlaceOf Klama Entity)",
            "(PlaceOf (DropPlace klama 0) Entity)",
            "(PlaceOf (Scalar Bogus klama) Entity)",
            "(PlaceOf (Tanru klama) Entity)",
        ] {
            let datum = parse_document(specimen).expect("malformed type specimen is lexical");
            assert!(
                TypeExpr::parse(&datum).is_err(),
                "accepted malformed type {specimen}",
            );
        }
    }
}
