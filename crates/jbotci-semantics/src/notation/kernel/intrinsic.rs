//! The closed registered-callable table of specification section 14.1.
//!
//! An intrinsic is a primitive or transparent-prelude name whose application is
//! ordinary: its operands are evaluated and its registered signature fixes the
//! result. The control and effect primitives — `Close`, the logical operators
//! and quantifiers, `Presuppose`, `Supplement`, the force constructors,
//! `Refer`, `Context`, `Polar`, `OpenQ`, `Answer`, and the collection
//! constructors that also name types — are kernel forms of their own instead,
//! because their transfer behaviour is not ordinary application.
//!
//! [`Intrinsic::instantiate`] *is* the registered signature table. It is the
//! only way a kernel application node learns its result type, so an unregistered
//! or wrongly typed call cannot be constructed at all.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

use super::apply::ApplicationTypeError;
use super::types::{SignKind, TypeAtom, TypeExpr};

/// A registered version-0 callable with an ordinary application rule.
#[invariant(
    true,
    "every member is one closed registry name; its arity and operand rules are enforced by instantiate"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Intrinsic {
    // Kernel crossings with ordinary application.
    Singleton,
    Reify,
    QuestionOf,
    InterpretContent,
    // Reference algebra.
    Among,
    Combine,
    // Indexicals.
    Deictic,
    Speaker,
    Audience,
    Now,
    Here,
    CurrentGround,
    This,
    That,
    Yonder,
    // Content-to-object abstraction crossings.
    Measure,
    TruthValue,
    ExperienceOf,
    ProcessOf,
    ActivityOf,
    Concept,
    Abstract,
    // Signs.
    OpaqueQuote,
    StructuredQuote,
    NameSign,
    SentenceSign,
    // Collections and mathematics.
    SetOf,
    Card,
    Interval,
    ZipWith,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    ElementOf,
    Union,
    Intersection,
    // Utterance, sign, and metadata facts.
    Realizes,
    SpeakerOf,
    AudienceOf,
    LocutionOf,
    DeicticTimeOf,
    DeicticPlaceOf,
    TextOf,
    Denotes,
    Quotes,
    Utters,
    Label,
    // Generalized-quantifier prelude.
    Some,
    No,
    Every,
    Exactly,
    AtLeast,
    AtMost,
    MoreThan,
    FewerThan,
    // Remaining transparent prelude helpers.
    DescribedAs,
    Named,
    InnatelyCapable,
    MotionVector,
}

impl Intrinsic {
    /// Parse one registered callable spelling.
    #[requires(true)]
    #[ensures(ret.is_some() == Self::ALL.iter().any(|value| value.as_str() == text))]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.as_str() == text)
    }

    /// Return the canonical callable spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Singleton => "Singleton",
            Self::Reify => "Reify",
            Self::QuestionOf => "QuestionOf",
            Self::InterpretContent => "InterpretContent",
            Self::Among => "Among",
            Self::Combine => "Combine",
            Self::Deictic => "Deictic",
            Self::Speaker => "Speaker",
            Self::Audience => "Audience",
            Self::Now => "Now",
            Self::Here => "Here",
            Self::CurrentGround => "CurrentGround",
            Self::This => "This",
            Self::That => "That",
            Self::Yonder => "Yonder",
            Self::Measure => "Measure",
            Self::TruthValue => "TruthValue",
            Self::ExperienceOf => "ExperienceOf",
            Self::ProcessOf => "ProcessOf",
            Self::ActivityOf => "ActivityOf",
            Self::Concept => "Concept",
            Self::Abstract => "Abstract",
            Self::OpaqueQuote => "OpaqueQuote",
            Self::StructuredQuote => "StructuredQuote",
            Self::NameSign => "NameSign",
            Self::SentenceSign => "SentenceSign",
            Self::SetOf => "SetOf",
            Self::Card => "Card",
            Self::Interval => "Interval",
            Self::ZipWith => "ZipWith",
            Self::Equal => "=",
            Self::NotEqual => "≠",
            Self::Less => "<",
            Self::LessOrEqual => "≤",
            Self::Greater => ">",
            Self::GreaterOrEqual => "≥",
            Self::Add => "+",
            Self::Subtract => "−",
            Self::Multiply => "×",
            Self::Divide => "÷",
            Self::ElementOf => "∈",
            Self::Union => "∪",
            Self::Intersection => "∩",
            Self::Realizes => "Realizes",
            Self::SpeakerOf => "SpeakerOf",
            Self::AudienceOf => "AudienceOf",
            Self::LocutionOf => "LocutionOf",
            Self::DeicticTimeOf => "DeicticTimeOf",
            Self::DeicticPlaceOf => "DeicticPlaceOf",
            Self::TextOf => "TextOf",
            Self::Denotes => "Denotes",
            Self::Quotes => "Quotes",
            Self::Utters => "Utters",
            Self::Label => "Label",
            Self::Some => "Some",
            Self::No => "No",
            Self::Every => "Every",
            Self::Exactly => "Exactly",
            Self::AtLeast => "AtLeast",
            Self::AtMost => "AtMost",
            Self::MoreThan => "MoreThan",
            Self::FewerThan => "FewerThan",
            Self::DescribedAs => "DescribedAs",
            Self::Named => "Named",
            Self::InnatelyCapable => "InnatelyCapable",
            Self::MotionVector => "MotionVector",
        }
    }

    /// Report whether this name is a transparent prelude binding rather than an
    /// irreducible primitive. Both print directly; the distinction matters to
    /// expansion checking, not to the printer.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_prelude(self) -> bool {
        matches!(
            self,
            Self::NotEqual
                | Self::Greater
                | Self::GreaterOrEqual
                | Self::Union
                | Self::Intersection
                | Self::This
                | Self::That
                | Self::Yonder
                | Self::Some
                | Self::No
                | Self::Every
                | Self::Exactly
                | Self::AtLeast
                | Self::AtMost
                | Self::MoreThan
                | Self::FewerThan
                | Self::DescribedAs
                | Self::Named
                | Self::InnatelyCapable
                | Self::MotionVector
        )
    }

    /// Instantiate this callable's registered signature at one application.
    ///
    /// This is the whole registered signature table for ordinary callables. It
    /// rejects wrong arity, wrong operand types, and every polymorphic
    /// instantiation the specification does not license.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn instantiate(self, arguments: &[TypeExpr]) -> Result<TypeExpr, ApplicationTypeError> {
        match self {
            Self::Singleton => {
                let [value] = exact(arguments, self)?;
                Ok(referents(value.clone()))
            }
            Self::Reify => {
                accepts_all(arguments, &[atom(TypeAtom::Content)], self)?;
                Ok(atom(TypeAtom::Proposition))
            }
            Self::QuestionOf => {
                let [query] = exact(arguments, self)?;
                if !matches!(query, TypeExpr::Query(_)) {
                    return Err(mismatch(self, "QuestionOf accepts a Query"));
                }
                Ok(atom(TypeAtom::Question))
            }
            Self::InterpretContent => {
                let [sign] = exact(arguments, self)?;
                if !is_sign_or_token(sign) {
                    return Err(mismatch(self, "InterpretContent accepts a sign or token"));
                }
                Ok(atom(TypeAtom::Content))
            }
            Self::Among => {
                let [left, right] = exact(arguments, self)?;
                common_referents(left, right, self)?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Combine => {
                let [left, right] = exact(arguments, self)?;
                common_referents(left, right, self)
            }
            Self::Deictic => {
                accepts_all(
                    arguments,
                    &[atom(TypeAtom::Proximity), atom(TypeAtom::DeicticGround)],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::Entity)))
            }
            Self::Speaker | Self::Audience | Self::This | Self::That | Self::Yonder => {
                accepts_all(arguments, &[], self)?;
                Ok(referents(atom(TypeAtom::Entity)))
            }
            Self::Now => {
                accepts_all(arguments, &[], self)?;
                Ok(referents(atom(TypeAtom::Eventuality)))
            }
            Self::Here => {
                accepts_all(arguments, &[], self)?;
                Ok(referents(atom(TypeAtom::Location)))
            }
            Self::CurrentGround => {
                accepts_all(arguments, &[], self)?;
                Ok(atom(TypeAtom::DeicticGround))
            }
            Self::Measure => {
                accepts_all(
                    arguments,
                    &[atom(TypeAtom::Content), referents(atom(TypeAtom::Scale))],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::Amount)))
            }
            Self::TruthValue => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::Content),
                        referents(atom(TypeAtom::Epistemology)),
                    ],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::TruthValue)))
            }
            Self::ExperienceOf => {
                accepts_all(
                    arguments,
                    &[atom(TypeAtom::Content), referents(atom(TypeAtom::Entity))],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::Experience)))
            }
            Self::ProcessOf => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::Content),
                        referents(atom(TypeAtom::Eventuality)),
                    ],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::Process)))
            }
            Self::ActivityOf => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::Content),
                        referents(atom(TypeAtom::Eventuality)),
                    ],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::Activity)))
            }
            Self::Concept => {
                accepts_all(
                    arguments,
                    &[atom(TypeAtom::Content), referents(atom(TypeAtom::Entity))],
                    self,
                )?;
                Ok(referents(atom(TypeAtom::Concept)))
            }
            Self::Abstract => {
                let [content, subject] = exact(arguments, self)?;
                accepts(content, &atom(TypeAtom::Content), self)?;
                if !matches!(subject, TypeExpr::Referents(_)) {
                    return Err(mismatch(self, "Abstract takes a reference as its subject"));
                }
                Ok(referents(atom(TypeAtom::AbstractNature)))
            }
            Self::OpaqueQuote => {
                accepts_all(arguments, &[atom(TypeAtom::Text)], self)?;
                Ok(TypeExpr::Sign(SignKind::Opaque))
            }
            Self::NameSign => {
                accepts_all(arguments, &[atom(TypeAtom::Text)], self)?;
                Ok(TypeExpr::Sign(SignKind::Name))
            }
            Self::SentenceSign => {
                accepts_all(arguments, &[atom(TypeAtom::Content)], self)?;
                Ok(TypeExpr::Sign(SignKind::Sentence))
            }
            Self::StructuredQuote => {
                let [operand] = exact(arguments, self)?;
                if !matches!(
                    operand,
                    TypeExpr::Atom(TypeAtom::TranscriptEntry) | TypeExpr::Atom(TypeAtom::Discourse)
                ) {
                    return Err(mismatch(
                        self,
                        "StructuredQuote quotes a transcript entry or discourse",
                    ));
                }
                Ok(TypeExpr::Sign(SignKind::Structured))
            }
            Self::SetOf => {
                let [property] = exact(arguments, self)?;
                let element = content_property_element(property)
                    .ok_or_else(|| mismatch(self, "SetOf comprehends a one-parameter property"))?;
                Ok(TypeExpr::Set(Box::new(element)))
            }
            Self::Card => {
                let [collection] = exact(arguments, self)?;
                if !matches!(collection, TypeExpr::Set(_) | TypeExpr::List(_)) {
                    return Err(mismatch(self, "Card counts a set or list"));
                }
                Ok(atom(TypeAtom::Cardinal))
            }
            Self::Interval => {
                let [lower, upper, first, second] = exact(arguments, self)?;
                accepts(first, &atom(TypeAtom::EndpointInclusion), self)?;
                accepts(second, &atom(TypeAtom::EndpointInclusion), self)?;
                let ordered = ordered_common_type(lower, upper).ok_or_else(|| {
                    mismatch(self, "Interval requires one registered ordered type")
                })?;
                Ok(TypeExpr::Interval(Box::new(ordered)))
            }
            Self::ZipWith => {
                let Some((relation, lists)) = arguments.split_first() else {
                    return Err(mismatch(self, "ZipWith takes a relation and its lists"));
                };
                let TypeExpr::Function { parameters, result } = relation else {
                    return Err(mismatch(self, "ZipWith takes a function"));
                };
                if result.as_ref() != &atom(TypeAtom::Content) {
                    return Err(mismatch(self, "ZipWith's relation returns Content"));
                }
                if parameters.len() != lists.len() || lists.is_empty() {
                    return Err(mismatch(self, "ZipWith takes one list per parameter"));
                }
                for (parameter, list) in parameters.iter().zip(lists) {
                    let TypeExpr::List(element) = list else {
                        return Err(mismatch(self, "ZipWith's operands are lists"));
                    };
                    accepts(element, parameter, self)?;
                }
                Ok(atom(TypeAtom::Content))
            }
            Self::Equal | Self::NotEqual => {
                let [left, right] = exact(arguments, self)?;
                common_type(left, right)
                    .ok_or_else(|| mismatch(self, "equality compares one common type"))?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Less | Self::LessOrEqual | Self::Greater | Self::GreaterOrEqual => {
                let [left, right] = exact(arguments, self)?;
                ordered_common_type(left, right).ok_or_else(|| {
                    mismatch(self, "comparison requires one registered ordered type")
                })?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Add | Self::Subtract | Self::Divide => {
                accepts_all(
                    arguments,
                    &[atom(TypeAtom::Number), atom(TypeAtom::Number)],
                    self,
                )?;
                Ok(atom(TypeAtom::Number))
            }
            Self::Multiply => {
                let [left, right] = exact(arguments, self)?;
                if let (TypeExpr::Set(first), TypeExpr::Set(second)) = (left, right) {
                    return Ok(TypeExpr::Set(Box::new(TypeExpr::Tuple(vec![
                        first.as_ref().clone(),
                        second.as_ref().clone(),
                    ]))));
                }
                accepts(left, &atom(TypeAtom::Number), self)?;
                accepts(right, &atom(TypeAtom::Number), self)?;
                Ok(atom(TypeAtom::Number))
            }
            Self::ElementOf => {
                let [element, set] = exact(arguments, self)?;
                let TypeExpr::Set(declared) = set else {
                    return Err(mismatch(self, "∈ tests membership in a set"));
                };
                accepts(element, declared, self)?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Union | Self::Intersection => {
                let [left, right] = exact(arguments, self)?;
                let (TypeExpr::Set(first), TypeExpr::Set(second)) = (left, right) else {
                    return Err(mismatch(self, "set operations take two sets"));
                };
                let element = common_type(first, second)
                    .ok_or_else(|| mismatch(self, "set operands need one common element type"))?;
                Ok(TypeExpr::Set(Box::new(element)))
            }
            Self::Realizes => {
                let [token, act] = exact(arguments, self)?;
                accepts(token, &atom(TypeAtom::UtteranceToken), self)?;
                if !matches!(act, TypeExpr::Act(_)) {
                    return Err(mismatch(self, "Realizes relates a token to an act"));
                }
                Ok(atom(TypeAtom::Content))
            }
            Self::SpeakerOf | Self::AudienceOf => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::UtteranceToken),
                        referents(atom(TypeAtom::Entity)),
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::LocutionOf => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::UtteranceToken),
                        referents(atom(TypeAtom::Locution)),
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::DeicticTimeOf => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::UtteranceToken),
                        referents(atom(TypeAtom::Eventuality)),
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::DeicticPlaceOf => {
                accepts_all(
                    arguments,
                    &[
                        atom(TypeAtom::UtteranceToken),
                        referents(atom(TypeAtom::Location)),
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::TextOf => {
                let [token, text] = exact(arguments, self)?;
                if !matches!(
                    token,
                    TypeExpr::Atom(TypeAtom::UtteranceToken) | TypeExpr::SignToken(_)
                ) {
                    return Err(mismatch(
                        self,
                        "TextOf describes an utterance or sign token",
                    ));
                }
                accepts(text, &atom(TypeAtom::Text), self)?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Denotes => {
                let [token, _denoted] = exact(arguments, self)?;
                if !matches!(token, TypeExpr::SignToken(_)) {
                    return Err(mismatch(self, "Denotes describes a sign token"));
                }
                Ok(atom(TypeAtom::Content))
            }
            Self::Quotes => {
                let [token, quoted] = exact(arguments, self)?;
                if !matches!(token, TypeExpr::SignToken(_)) {
                    return Err(mismatch(self, "Quotes describes a sign token"));
                }
                if !is_quotable(quoted) {
                    return Err(mismatch(self, "Quotes accepts a quotable value"));
                }
                Ok(atom(TypeAtom::Content))
            }
            Self::Utters => {
                let [speaker, uttered] = exact(arguments, self)?;
                accepts(speaker, &referents(atom(TypeAtom::Entity)), self)?;
                if !is_utterable(uttered) {
                    return Err(mismatch(self, "Utters accepts an utterable value"));
                }
                Ok(atom(TypeAtom::Content))
            }
            Self::Label => {
                let [level, ordinal, _target] = exact(arguments, self)?;
                accepts(level, &atom(TypeAtom::LabelLevel), self)?;
                accepts(ordinal, &atom(TypeAtom::Number), self)?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Some | Self::No | Self::Every => {
                let [restriction] = exact(arguments, self)?;
                let element = content_property_element(restriction).ok_or_else(|| {
                    mismatch(self, "a quantifier restriction is a one-parameter property")
                })?;
                Ok(TypeExpr::GeneralizedQuantifier(Box::new(element)))
            }
            Self::Exactly | Self::AtLeast | Self::AtMost | Self::MoreThan | Self::FewerThan => {
                let [count, restriction] = exact(arguments, self)?;
                accepts(count, &atom(TypeAtom::Natural), self)?;
                let element = content_property_element(restriction).ok_or_else(|| {
                    mismatch(self, "a quantifier restriction is a one-parameter property")
                })?;
                Ok(TypeExpr::GeneralizedQuantifier(Box::new(element)))
            }
            Self::DescribedAs => {
                accepts_all(
                    arguments,
                    &[
                        referents(atom(TypeAtom::Entity)),
                        referents(atom(TypeAtom::Entity)),
                        property_of(referents(atom(TypeAtom::Entity))),
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::Named => {
                accepts_all(
                    arguments,
                    &[atom(TypeAtom::Text), referents(atom(TypeAtom::Entity))],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::InnatelyCapable => {
                accepts_all(
                    arguments,
                    &[
                        referents(atom(TypeAtom::Entity)),
                        TypeExpr::Function {
                            parameters: vec![
                                referents(atom(TypeAtom::Entity)),
                                referents(atom(TypeAtom::Eventuality)),
                            ],
                            result: Box::new(atom(TypeAtom::Content)),
                        },
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
            Self::MotionVector => {
                accepts_all(
                    arguments,
                    &[
                        referents(atom(TypeAtom::Eventuality)),
                        referents(atom(TypeAtom::Entity)),
                        TypeExpr::Function {
                            parameters: vec![
                                referents(atom(TypeAtom::Entity)),
                                referents(atom(TypeAtom::Entity)),
                            ],
                            result: Box::new(atom(TypeAtom::Content)),
                        },
                    ],
                    self,
                )?;
                Ok(atom(TypeAtom::Content))
            }
        }
    }

    /// Project the operand types section 14.1 declares for one instantiation.
    ///
    /// This is the projection view of the table [`Intrinsic::instantiate`]
    /// checks, and the two MUST be edited together: `instantiate` decides
    /// whether an application is legal, this reports what the specification
    /// declared each accepted operand to be. They are kept adjacent, arm for arm
    /// and in the same order, so that pairing obligation stays visible. This
    /// method decides nothing and rejects nothing; an instantiation
    /// `instantiate` would refuse simply projects `None` wherever no declaration
    /// can be read off it.
    ///
    /// An entry is `Some` exactly where section 14.1 fixes that operand's type,
    /// either outright or as a schema whose variables this instantiation
    /// resolves. It is `None` where the declaration leaves the operand free: a
    /// bare schema variable (`Singleton`'s `T`, `=`'s `T`, `Denotes`' denoted
    /// value, `Label`'s target, a comparison's ordered `T`) or one of the closed
    /// overload sets (`Quotable`, `Utterable`, sign-or-token, set-or-list, the
    /// two `×` readings).
    ///
    /// A projected type is an *expectation*: a printer may drop a crossing the
    /// reader reinstates from it. A schema variable is therefore resolved only
    /// from surface that survives such an elision — another operand, or a
    /// property lambda's typed parameter list, which always prints in full. The
    /// element type of a `∈`, `∪`, `∩`, or `×` set operand is readable only from
    /// that same collection, which is the one surface the expectation would let
    /// a printer strip, so those entries stay `None` even though the operand
    /// must be a set.
    #[requires(true)]
    #[ensures(ret.len() == argument_types.len())]
    pub fn declared_operand_types(self, argument_types: &[TypeExpr]) -> Vec<Option<TypeExpr>> {
        let arity = argument_types.len();
        match self {
            // `Singleton : T -> Referents<T>` constrains nothing.
            Self::Singleton => free(arity),
            Self::Reify => fit(vec![Some(atom(TypeAtom::Content))], arity),
            Self::QuestionOf => fit(
                vec![same_shape(argument_types.first(), |value| {
                    matches!(value, TypeExpr::Query(_))
                })],
                arity,
            ),
            // A sign-or-token overload set.
            Self::InterpretContent => free(arity),
            Self::Among | Self::Combine => {
                let common = match argument_types {
                    [left, right]
                        if matches!(left, TypeExpr::Referents(_))
                            && matches!(right, TypeExpr::Referents(_)) =>
                    {
                        common_type(left, right)
                    }
                    _ => None,
                };
                fit(vec![common.clone(), common], arity)
            }
            Self::Deictic => fit(
                vec![
                    Some(atom(TypeAtom::Proximity)),
                    Some(atom(TypeAtom::DeicticGround)),
                ],
                arity,
            ),
            Self::Speaker
            | Self::Audience
            | Self::This
            | Self::That
            | Self::Yonder
            | Self::Now
            | Self::Here
            | Self::CurrentGround => free(arity),
            Self::Measure => fit(
                vec![
                    Some(atom(TypeAtom::Content)),
                    Some(referents(atom(TypeAtom::Scale))),
                ],
                arity,
            ),
            Self::TruthValue => fit(
                vec![
                    Some(atom(TypeAtom::Content)),
                    Some(referents(atom(TypeAtom::Epistemology))),
                ],
                arity,
            ),
            Self::ExperienceOf => fit(
                vec![
                    Some(atom(TypeAtom::Content)),
                    Some(referents(atom(TypeAtom::Entity))),
                ],
                arity,
            ),
            Self::ProcessOf | Self::ActivityOf => fit(
                vec![
                    Some(atom(TypeAtom::Content)),
                    Some(referents(atom(TypeAtom::Eventuality))),
                ],
                arity,
            ),
            Self::Concept => fit(
                vec![
                    Some(atom(TypeAtom::Content)),
                    Some(referents(atom(TypeAtom::Entity))),
                ],
                arity,
            ),
            // The subject is any reference; its referent type is the operand's
            // own and no crossing is keyed on it.
            Self::Abstract => fit(
                vec![
                    Some(atom(TypeAtom::Content)),
                    same_shape(argument_types.get(1), |value| {
                        matches!(value, TypeExpr::Referents(_))
                    }),
                ],
                arity,
            ),
            Self::OpaqueQuote | Self::NameSign => fit(vec![Some(atom(TypeAtom::Text))], arity),
            Self::SentenceSign => fit(vec![Some(atom(TypeAtom::Content))], arity),
            // A transcript-entry-or-discourse overload set.
            Self::StructuredQuote => free(arity),
            Self::SetOf => fit(vec![declared_property(argument_types.first())], arity),
            // A set-or-list overload set.
            Self::Card => free(arity),
            Self::Interval => fit(
                vec![
                    None,
                    None,
                    Some(atom(TypeAtom::EndpointInclusion)),
                    Some(atom(TypeAtom::EndpointInclusion)),
                ],
                arity,
            ),
            Self::ZipWith => {
                let Some((relation, lists)) = argument_types.split_first() else {
                    return free(arity);
                };
                let TypeExpr::Function { parameters, result } = relation else {
                    return free(arity);
                };
                if result.as_ref() != &atom(TypeAtom::Content)
                    || parameters.len() != lists.len()
                    || lists.is_empty()
                {
                    return free(arity);
                }
                // The relation declares `Content`; each list's element type is
                // the matching declared parameter, which the relation prints.
                let mut projected = vec![Some(relation.clone())];
                projected.extend(
                    parameters
                        .iter()
                        .map(|parameter| Some(TypeExpr::List(Box::new(parameter.clone())))),
                );
                fit(projected, arity)
            }
            // Equality declares a bare `T`.
            Self::Equal | Self::NotEqual => free(arity),
            // Comparison declares a bare `T` constrained to the ordered family,
            // which is not one type.
            Self::Less | Self::LessOrEqual | Self::Greater | Self::GreaterOrEqual => free(arity),
            Self::Add | Self::Subtract | Self::Divide => fit(
                vec![Some(atom(TypeAtom::Number)), Some(atom(TypeAtom::Number))],
                arity,
            ),
            Self::Multiply => {
                if matches!(argument_types, [TypeExpr::Set(_), TypeExpr::Set(_)]) {
                    // The cartesian-product reading; both element types are the
                    // operands' own.
                    return free(arity);
                }
                fit(
                    vec![Some(atom(TypeAtom::Number)), Some(atom(TypeAtom::Number))],
                    arity,
                )
            }
            // `∈ : T × Set<T>` fixes `T` from the set operand itself.
            Self::ElementOf => free(arity),
            // Both operands are sets of one common element type, which only
            // those same two collections show.
            Self::Union | Self::Intersection => free(arity),
            Self::Realizes => fit(
                vec![
                    Some(atom(TypeAtom::UtteranceToken)),
                    same_shape(argument_types.get(1), |value| {
                        matches!(value, TypeExpr::Act(_))
                    }),
                ],
                arity,
            ),
            Self::SpeakerOf | Self::AudienceOf => fit(
                vec![
                    Some(atom(TypeAtom::UtteranceToken)),
                    Some(referents(atom(TypeAtom::Entity))),
                ],
                arity,
            ),
            Self::LocutionOf => fit(
                vec![
                    Some(atom(TypeAtom::UtteranceToken)),
                    Some(referents(atom(TypeAtom::Locution))),
                ],
                arity,
            ),
            Self::DeicticTimeOf => fit(
                vec![
                    Some(atom(TypeAtom::UtteranceToken)),
                    Some(referents(atom(TypeAtom::Eventuality))),
                ],
                arity,
            ),
            Self::DeicticPlaceOf => fit(
                vec![
                    Some(atom(TypeAtom::UtteranceToken)),
                    Some(referents(atom(TypeAtom::Location))),
                ],
                arity,
            ),
            // An utterance-token-or-sign-token overload set, then `Text`.
            Self::TextOf => fit(vec![None, Some(atom(TypeAtom::Text))], arity),
            // The denoted value is any type; the quoted one is a `Quotable`.
            Self::Denotes | Self::Quotes => fit(
                vec![
                    same_shape(argument_types.first(), |value| {
                        matches!(value, TypeExpr::SignToken(_))
                    }),
                    None,
                ],
                arity,
            ),
            // The uttered value is an `Utterable`.
            Self::Utters => fit(vec![Some(referents(atom(TypeAtom::Entity))), None], arity),
            // `Label`'s target is any type.
            Self::Label => fit(
                vec![
                    Some(atom(TypeAtom::LabelLevel)),
                    Some(atom(TypeAtom::Number)),
                    None,
                ],
                arity,
            ),
            Self::Some | Self::No | Self::Every => {
                fit(vec![declared_property(argument_types.first())], arity)
            }
            Self::Exactly | Self::AtLeast | Self::AtMost | Self::MoreThan | Self::FewerThan => fit(
                vec![
                    Some(atom(TypeAtom::Natural)),
                    declared_property(argument_types.get(1)),
                ],
                arity,
            ),
            Self::DescribedAs => fit(
                vec![
                    Some(referents(atom(TypeAtom::Entity))),
                    Some(referents(atom(TypeAtom::Entity))),
                    Some(property_of(referents(atom(TypeAtom::Entity)))),
                ],
                arity,
            ),
            Self::Named => fit(
                vec![
                    Some(atom(TypeAtom::Text)),
                    Some(referents(atom(TypeAtom::Entity))),
                ],
                arity,
            ),
            Self::InnatelyCapable => fit(
                vec![
                    Some(referents(atom(TypeAtom::Entity))),
                    Some(TypeExpr::Function {
                        parameters: vec![
                            referents(atom(TypeAtom::Entity)),
                            referents(atom(TypeAtom::Eventuality)),
                        ],
                        result: Box::new(atom(TypeAtom::Content)),
                    }),
                ],
                arity,
            ),
            Self::MotionVector => fit(
                vec![
                    Some(referents(atom(TypeAtom::Eventuality))),
                    Some(referents(atom(TypeAtom::Entity))),
                    Some(TypeExpr::Function {
                        parameters: vec![
                            referents(atom(TypeAtom::Entity)),
                            referents(atom(TypeAtom::Entity)),
                        ],
                        result: Box::new(atom(TypeAtom::Content)),
                    }),
                ],
                arity,
            ),
        }
    }

    /// Every registered ordinary callable, in table order.
    pub const ALL: [Self; 66] = [
        Self::Singleton,
        Self::Reify,
        Self::QuestionOf,
        Self::InterpretContent,
        Self::Among,
        Self::Combine,
        Self::Deictic,
        Self::Speaker,
        Self::Audience,
        Self::Now,
        Self::Here,
        Self::CurrentGround,
        Self::This,
        Self::That,
        Self::Yonder,
        Self::Measure,
        Self::TruthValue,
        Self::ExperienceOf,
        Self::ProcessOf,
        Self::ActivityOf,
        Self::Concept,
        Self::Abstract,
        Self::OpaqueQuote,
        Self::StructuredQuote,
        Self::NameSign,
        Self::SentenceSign,
        Self::SetOf,
        Self::Card,
        Self::Interval,
        Self::ZipWith,
        Self::Equal,
        Self::NotEqual,
        Self::Less,
        Self::LessOrEqual,
        Self::Greater,
        Self::GreaterOrEqual,
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::ElementOf,
        Self::Union,
        Self::Intersection,
        Self::Realizes,
        Self::SpeakerOf,
        Self::AudienceOf,
        Self::LocutionOf,
        Self::DeicticTimeOf,
        Self::DeicticPlaceOf,
        Self::TextOf,
        Self::Denotes,
        Self::Quotes,
        Self::Utters,
        Self::Label,
        Self::Some,
        Self::No,
        Self::Every,
        Self::Exactly,
        Self::AtLeast,
        Self::AtMost,
        Self::MoreThan,
        Self::FewerThan,
        Self::DescribedAs,
        Self::Named,
        Self::InnatelyCapable,
        Self::MotionVector,
    ];
}

/// Build a primitive sort type.
#[requires(true)]
#[ensures(ret == TypeExpr::Atom(value))]
pub(super) fn atom(value: TypeAtom) -> TypeExpr {
    TypeExpr::Atom(value)
}

/// Build a number-neutral reference type.
#[requires(true)]
#[ensures(matches!(ret, TypeExpr::Referents(_)))]
pub(super) fn referents(inner: TypeExpr) -> TypeExpr {
    TypeExpr::Referents(Box::new(inner))
}

/// Build a `Property<T>`: a one-parameter content function.
#[requires(true)]
#[ensures(matches!(ret, TypeExpr::Function { .. }))]
pub(super) fn property_of(parameter: TypeExpr) -> TypeExpr {
    TypeExpr::Function {
        parameters: vec![parameter],
        result: Box::new(atom(TypeAtom::Content)),
    }
}

/// Declare nothing about any operand of an application of this arity.
#[requires(true)]
#[ensures(ret.len() == arity && ret.iter().all(Option::is_none))]
fn free(arity: usize) -> Vec<Option<TypeExpr>> {
    vec![None; arity]
}

/// Force a projection to name exactly one entry per supplied operand.
///
/// A projection is written for the arity its registered signature declares; an
/// application of another arity is one `Intrinsic::instantiate` rejects, and the
/// surplus or missing operands simply declare nothing.
#[requires(true)]
#[ensures(ret.len() == arity)]
fn fit(mut projected: Vec<Option<TypeExpr>>, arity: usize) -> Vec<Option<TypeExpr>> {
    projected.resize(arity, None);
    projected
}

/// Project a schema operand whose type constructor section 14.1 fixes but whose
/// parameter this instantiation reads off the accepted operand itself.
///
/// This is sound only because no crossing is keyed on that parameter: the
/// printer's elisions read the outermost constructor, which the signature fixed.
#[requires(true)]
#[ensures(ret.is_some() == actual.is_some_and(shape))]
fn same_shape(actual: Option<&TypeExpr>, shape: fn(&TypeExpr) -> bool) -> Option<TypeExpr> {
    actual.filter(|value| shape(value)).cloned()
}

/// Project a declared `Property<T>` operand, resolving `T` from the operand.
///
/// A property is written as a lambda whose typed parameter list always prints,
/// so a reader recovers `T` even from a surface with every licensed crossing
/// elided; the `Content` result is the signature's own and never varies.
#[requires(true)]
#[ensures(ret.is_some() == actual.is_some_and(|value| content_property_element(value).is_some()))]
fn declared_property(actual: Option<&TypeExpr>) -> Option<TypeExpr> {
    content_property_element(actual?).map(property_of)
}

/// Require an exact argument count and borrow the arguments as an array.
#[requires(true)]
#[ensures(ret.is_ok() == (arguments.len() == N))]
fn exact<const N: usize>(
    arguments: &[TypeExpr],
    intrinsic: Intrinsic,
) -> Result<&[TypeExpr; N], ApplicationTypeError> {
    arguments.try_into().map_err(|_| {
        ApplicationTypeError::new(format!(
            "{} takes {N} operands, found {}",
            intrinsic.as_str(),
            arguments.len()
        ))
    })
}

/// Check a whole ordered operand list against declared parameter types.
#[requires(true)]
#[ensures(ret.is_ok() -> arguments.len() == expected.len())]
fn accepts_all(
    arguments: &[TypeExpr],
    expected: &[TypeExpr],
    intrinsic: Intrinsic,
) -> Result<(), ApplicationTypeError> {
    if arguments.len() != expected.len() {
        return Err(ApplicationTypeError::new(format!(
            "{} takes {} operands, found {}",
            intrinsic.as_str(),
            expected.len(),
            arguments.len()
        )));
    }
    for (actual, declared) in arguments.iter().zip(expected) {
        accepts(actual, declared, intrinsic)?;
    }
    Ok(())
}

/// Require one operand to inhabit its declared type.
///
/// Singleton lifting is deliberately not accepted here: specification section
/// 3.1 makes it a crossing, and the kernel keeps crossings explicit so the
/// printer decides whether the notation elides one.
#[requires(true)]
#[ensures(ret.is_ok() == kernel_accepts(actual, declared))]
fn accepts(
    actual: &TypeExpr,
    declared: &TypeExpr,
    intrinsic: Intrinsic,
) -> Result<(), ApplicationTypeError> {
    if !kernel_accepts(actual, declared) {
        return Err(ApplicationTypeError::new(format!(
            "{} rejects an operand of the supplied type",
            intrinsic.as_str()
        )));
    }
    Ok(())
}

/// Test the kernel's operand rule: identity, declared upcast, or the finite
/// `Natural`-to-`Cardinal` embedding, but never an implicit singleton lift.
#[requires(true)]
#[ensures(ret -> actual.implicit_conversion_to(declared).is_some())]
pub(super) fn kernel_accepts(actual: &TypeExpr, declared: &TypeExpr) -> bool {
    use super::types::ImplicitConversion;

    matches!(
        actual.implicit_conversion_to(declared),
        Some(
            ImplicitConversion::Identity
                | ImplicitConversion::Upcast
                | ImplicitConversion::CovariantReferentsUpcast
                | ImplicitConversion::NaturalToCardinal
                | ImplicitConversion::PointwiseNaturalToCardinal
        )
    )
}

/// Return the least common type of two operands under the kernel rule.
#[requires(true)]
#[ensures(true)]
pub(super) fn common_type(left: &TypeExpr, right: &TypeExpr) -> Option<TypeExpr> {
    if kernel_accepts(left, right) {
        return Some(right.clone());
    }
    if kernel_accepts(right, left) {
        return Some(left.clone());
    }
    None
}

/// Return the common reference type of two `Referents` operands.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| matches!(value, TypeExpr::Referents(_))) || ret.is_err())]
fn common_referents(
    left: &TypeExpr,
    right: &TypeExpr,
    intrinsic: Intrinsic,
) -> Result<TypeExpr, ApplicationTypeError> {
    if !matches!(left, TypeExpr::Referents(_)) || !matches!(right, TypeExpr::Referents(_)) {
        return Err(mismatch(intrinsic, "both operands must be references"));
    }
    common_type(left, right)
        .ok_or_else(|| mismatch(intrinsic, "operands have no common reference type"))
}

/// Return the common type of two operands when it is a registered ordered type.
#[requires(true)]
#[ensures(true)]
fn ordered_common_type(left: &TypeExpr, right: &TypeExpr) -> Option<TypeExpr> {
    let common = common_type(left, right)?;
    matches!(
        common,
        TypeExpr::Atom(TypeAtom::Number | TypeAtom::Natural | TypeAtom::Cardinal)
    )
    .then_some(common)
}

/// Return the parameter type of a one-parameter content property.
#[requires(true)]
#[ensures(true)]
pub(super) fn content_property_element(value: &TypeExpr) -> Option<TypeExpr> {
    let TypeExpr::Function { parameters, result } = value else {
        return None;
    };
    if parameters.len() != 1 || result.as_ref() != &atom(TypeAtom::Content) {
        return None;
    }
    parameters.first().cloned()
}

/// Test the sign/sign-token operand family.
#[requires(true)]
#[ensures(true)]
fn is_sign_or_token(value: &TypeExpr) -> bool {
    matches!(value, TypeExpr::Sign(_) | TypeExpr::SignToken(_))
}

/// Test the closed `Quotable` overload set of section 14.1.
#[requires(true)]
#[ensures(true)]
fn is_quotable(value: &TypeExpr) -> bool {
    is_sign_or_token(value)
        || matches!(
            value,
            TypeExpr::Atom(TypeAtom::Text | TypeAtom::TranscriptEntry | TypeAtom::Discourse)
        )
}

/// Test the closed `Utterable` overload set of section 14.1.
#[requires(true)]
#[ensures(true)]
fn is_utterable(value: &TypeExpr) -> bool {
    is_sign_or_token(value)
        || matches!(
            value,
            TypeExpr::Act(_) | TypeExpr::Atom(TypeAtom::TranscriptEntry)
        )
}

/// Build an operand-mismatch failure naming the callable.
#[requires(!detail.is_empty())]
#[ensures(true)]
fn mismatch(intrinsic: Intrinsic, detail: &str) -> ApplicationTypeError {
    ApplicationTypeError::new(format!("{}: {detail}", intrinsic.as_str()))
}
