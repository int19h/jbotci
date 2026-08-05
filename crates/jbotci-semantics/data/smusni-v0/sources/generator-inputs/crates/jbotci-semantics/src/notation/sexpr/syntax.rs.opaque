//! Exact version-0 smusni serialization syntax.
//!
//! [`Datum`](super::datum::Datum) is deliberately only the final lexical
//! serialization boundary.  This module gives every structured production in
//! specification section 2.2 its own typed representation and rejects forms
//! that merely happen to be S-expression lists.

use std::collections::BTreeSet;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer as _;
use unicode_normalization::UnicodeNormalization;

use super::datum::{Datum, Integer, parse_datums, parse_document, print_document};
use super::type_system::{LexicalRoot, PositiveInteger, SignKind, TypeExpr, Variable};

/// NFC-normalized text carried by a string datum.
#[invariant(text.nfc().eq(text.chars()))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NfcText {
    text: String,
}

impl NfcText {
    /// Normalize caller text for renderer construction.
    #[requires(true)]
    #[ensures(ret.as_str().nfc().eq(ret.as_str().chars()))]
    pub fn new(text: impl AsRef<str>) -> Self {
        new!(NfcText {
            text: text.as_ref().nfc().collect(),
        })
    }

    /// Borrow normalized text.
    #[requires(true)]
    #[ensures(ret.nfc().eq(ret.chars()))]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Closed lexical classes for a value-position atom.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AtomClass {
    LexicalRoot,
    RegisteredPascal,
    CallableGlyph,
}

/// A namespace-checked value atom.
#[invariant(classify_value_atom(&token) == Some(*class))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueAtom {
    token: String,
    class: AtomClass,
}

impl ValueAtom {
    /// Parse a closed version-0 value atom.
    #[requires(true)]
    #[ensures(ret.is_ok() == classify_value_atom(token).is_some())]
    pub fn try_new(token: &str) -> Result<Self, V0ParseError> {
        let class = classify_value_atom(token)
            .ok_or_else(|| V0ParseError::new(format!("unregistered value atom {token:?}")))?;
        Ok(new!(ValueAtom {
            token: token.to_owned(),
            class,
        }))
    }

    /// Borrow the exact token spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.token
    }

    /// Return the disjoint lexical namespace.
    #[requires(true)]
    #[ensures(ret == self.class)]
    pub fn class(&self) -> AtomClass {
        self.class
    }
}

/// An exact reduced rational literal.
#[invariant(is_reduced_rational(numerator.as_str(), denominator.as_str()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    numerator: Integer,
    denominator: Integer,
}

impl Rational {
    /// Validate positive denominator and lowest terms.
    #[requires(true)]
    #[ensures(ret.is_ok() == is_reduced_rational(old(numerator.clone()).as_str(), old(denominator.clone()).as_str()))]
    pub fn try_new(numerator: Integer, denominator: Integer) -> Result<Self, V0ParseError> {
        if !is_reduced_rational(numerator.as_str(), denominator.as_str()) {
            return Err(V0ParseError::new(
                "rational denominator must be positive and the fraction must be reduced",
            ));
        }
        Ok(new!(Rational {
            numerator,
            denominator,
        }))
    }

    /// Serialize as the reserved `(/ numerator denominator)` production.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("/"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "/",
            [
                Datum::Integer(self.numerator.clone()),
                Datum::Integer(self.denominator.clone()),
            ],
        )
    }
}

/// One typed lambda declaration.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    variable: Variable,
    declared_type: TypeExpr,
}

impl Declaration {
    /// Construct a typed declaration.
    #[requires(true)]
    #[ensures(ret.variable == old(variable.clone()) && ret.declared_type == old(declared_type.clone()))]
    pub fn new(variable: Variable, declared_type: TypeExpr) -> Self {
        Self {
            variable,
            declared_type,
        }
    }

    /// Borrow the binder name.
    #[requires(true)]
    #[ensures(ret.as_str().starts_with('$'))]
    pub fn variable(&self) -> &Variable {
        &self.variable
    }

    /// Borrow the declared type.
    #[requires(true)]
    #[ensures(true)]
    pub fn declared_type(&self) -> &TypeExpr {
        &self.declared_type
    }

    /// Serialize a declaration pair.
    #[requires(true)]
    #[ensures(ret.as_list().is_some_and(|items| items.len() == 2))]
    fn to_datum(&self) -> Datum {
        Datum::list([
            Datum::atom(self.variable.as_str()),
            self.declared_type.to_datum(),
        ])
    }
}

/// A closed transparent-prelude binding name, permitted only by `Let`.
#[invariant(PRELUDE_NAMES.contains(&token.as_str()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreludeName {
    token: String,
}

impl PreludeName {
    /// Parse one exact prelude registry name.
    #[requires(true)]
    #[ensures(ret.is_ok() == PRELUDE_NAMES.contains(&token))]
    pub fn try_new(token: &str) -> Result<Self, V0ParseError> {
        if !PRELUDE_NAMES.contains(&token) {
            return Err(V0ParseError::new(format!(
                "unregistered prelude binding name {token:?}"
            )));
        }
        Ok(new!(PreludeName {
            token: token.to_owned(),
        }))
    }

    /// Borrow the canonical spelling.
    #[requires(true)]
    #[ensures(PRELUDE_NAMES.contains(&ret))]
    pub fn as_str(&self) -> &str {
        &self.token
    }
}

/// One typed value binding.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueBinding {
    declaration: Declaration,
    value: Box<V0Expr>,
}

/// A transparent-prelude value binding.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreludeBinding {
    name: PreludeName,
    declared_type: TypeExpr,
    value: Box<V0Expr>,
}

impl PreludeBinding {
    /// Construct a typed transparent-prelude binding.
    #[requires(true)]
    #[ensures(ret.name == old(name.clone()) && ret.declared_type == old(declared_type.clone()) && ret.value.as_ref() == &old(value.clone()))]
    pub fn new(name: PreludeName, declared_type: TypeExpr, value: V0Expr) -> Self {
        Self {
            name,
            declared_type,
            value: Box::new(value),
        }
    }

    /// Serialize a prelude binding triple.
    #[requires(true)]
    #[ensures(ret.as_list().is_some_and(|items| items.len() == 3))]
    fn to_datum(&self) -> Datum {
        Datum::list([
            Datum::atom(self.name.as_str()),
            self.declared_type.to_datum(),
            self.value.to_datum(),
        ])
    }
}

/// The two closed binding-name productions accepted by `Let`.
#[invariant(::Variable(_) => true)]
#[invariant(::Prelude(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LetBinding {
    Variable(ValueBinding),
    Prelude(PreludeBinding),
}

impl LetBinding {
    /// Serialize either exact `let-name` production.
    #[requires(true)]
    #[ensures(ret.as_list().is_some_and(|items| items.len() == 3))]
    fn to_datum(&self) -> Datum {
        match self {
            Self::Variable(binding) => binding.to_datum(),
            Self::Prelude(binding) => binding.to_datum(),
        }
    }
}

impl ValueBinding {
    /// Construct a typed value binding.
    #[requires(true)]
    #[ensures(ret.declaration == old(declaration.clone()) && ret.value.as_ref() == &old(value.clone()))]
    pub fn new(declaration: Declaration, value: V0Expr) -> Self {
        Self {
            declaration,
            value: Box::new(value),
        }
    }

    /// Borrow the binding declaration.
    #[requires(true)]
    #[ensures(true)]
    pub fn declaration(&self) -> &Declaration {
        &self.declaration
    }

    /// Borrow the initializer.
    #[requires(true)]
    #[ensures(true)]
    pub fn value(&self) -> &V0Expr {
        &self.value
    }

    /// Serialize a binding triple.
    #[requires(true)]
    #[ensures(ret.as_list().is_some_and(|items| items.len() == 3))]
    fn to_datum(&self) -> Datum {
        Datum::list([
            Datum::atom(self.declaration.variable.as_str()),
            self.declaration.declared_type.to_datum(),
            self.value.to_datum(),
        ])
    }
}

/// A lambda with a nonempty, duplicate-free ordered parameter list.
#[invariant(!parameters.is_empty() && declarations_are_unique(&parameters))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lambda {
    parameters: Vec<Declaration>,
    body: Box<V0Expr>,
}

impl Lambda {
    /// Construct a checked lambda.
    #[requires(!parameters.is_empty() && declarations_are_unique(&parameters))]
    #[ensures(ret.parameters.len() == old(parameters.len()) && ret.body.as_ref() == &old(body.clone()))]
    pub fn new(parameters: Vec<Declaration>, body: V0Expr) -> Self {
        new!(Lambda {
            parameters,
            body: Box::new(body),
        })
    }

    /// Borrow the ordered parameters.
    #[requires(true)]
    #[ensures(!ret.is_empty() && declarations_are_unique(ret))]
    pub fn parameters(&self) -> &[Declaration] {
        &self.parameters
    }

    /// Borrow the body.
    #[requires(true)]
    #[ensures(true)]
    pub fn body(&self) -> &V0Expr {
        &self.body
    }

    /// Serialize the exact lambda special form.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("λ"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "λ",
            [
                Datum::list(self.parameters.iter().map(Declaration::to_datum)),
                self.body.to_datum(),
            ],
        )
    }
}

/// A canonical one-binding `Let` form.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetForm {
    binding: LetBinding,
    body: Box<V0Expr>,
}

impl LetForm {
    /// Construct one canonical `Let` form.
    #[requires(true)]
    #[ensures(ret.binding == old(binding.clone()) && ret.body.as_ref() == &old(body.clone()))]
    pub fn new(binding: LetBinding, body: V0Expr) -> Self {
        Self {
            binding,
            body: Box::new(body),
        }
    }

    /// Serialize the exact single-binding special form.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Let"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "Let",
            [Datum::list([self.binding.to_datum()]), self.body.to_datum()],
        )
    }
}

/// A canonical one-variable `Bind` form.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindForm {
    binding: ValueBinding,
    body: Box<V0Expr>,
}

impl BindForm {
    /// Construct one canonical `Bind` form.
    #[requires(true)]
    #[ensures(ret.binding == old(binding.clone()) && ret.body.as_ref() == &old(body.clone()))]
    pub fn new(binding: ValueBinding, body: V0Expr) -> Self {
        Self {
            binding,
            body: Box::new(body),
        }
    }

    /// Serialize the exact single-binding special form.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Bind"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "Bind",
            [Datum::list([self.binding.to_datum()]), self.body.to_datum()],
        )
    }
}

/// A guarded recursive binding group. Syntax enforces lambda initializers;
/// semantic cycle analysis is a later typed-IR responsibility.
#[invariant(!bindings.is_empty() && bindings_are_unique(&bindings) && bindings.iter().all(|binding| matches!(binding.value.as_ref(), V0Expr::Lambda(_))))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetRec {
    bindings: Vec<ValueBinding>,
    body: Box<V0Expr>,
}

impl LetRec {
    /// Construct a syntactically guarded recursive group.
    #[requires(!bindings.is_empty() && bindings_are_unique(&bindings) && bindings.iter().all(|binding| matches!(binding.value.as_ref(), V0Expr::Lambda(_))))]
    #[ensures(ret.bindings.len() == old(bindings.len()) && ret.body.as_ref() == &old(body.clone()))]
    pub fn new(bindings: Vec<ValueBinding>, body: V0Expr) -> Self {
        new!(LetRec {
            bindings,
            body: Box::new(body),
        })
    }

    /// Serialize the exact multi-binding special form.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("LetRec"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "LetRec",
            [
                Datum::list(self.bindings.iter().map(ValueBinding::to_datum)),
                self.body.to_datum(),
            ],
        )
    }
}

/// One application argument, with place fills distinct from values.
#[invariant(::Value(_) => true)]
#[invariant(::NumberedPlace { .. } => true)]
#[invariant(::EventualityPlace { .. } => true)]
#[invariant(::ComputedPlace { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationArgument {
    Value(Box<V0Expr>),
    NumberedPlace {
        place: PositiveInteger,
        value: Box<V0Expr>,
    },
    EventualityPlace {
        value: Box<V0Expr>,
    },
    ComputedPlace {
        place: Box<V0Expr>,
        value: Box<V0Expr>,
    },
}

impl ApplicationArgument {
    /// Serialize one argument production.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn to_datums(&self) -> Vec<Datum> {
        match self {
            Self::Value(value) => vec![value.to_datum()],
            Self::NumberedPlace { place, value } => {
                vec![Datum::atom(format!(":{place}")), value.to_datum()]
            }
            Self::EventualityPlace { value } => {
                vec![Datum::atom(":Eventuality"), value.to_datum()]
            }
            Self::ComputedPlace { place, value } => {
                vec![Datum::form("At", [place.to_datum(), value.to_datum()])]
            }
        }
    }
}

/// A nonempty ordinary application.
#[invariant(!arguments.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    head: Box<V0Expr>,
    arguments: Vec<ApplicationArgument>,
}

impl Application {
    /// Construct one application with at least one argument.
    #[requires(!arguments.is_empty())]
    #[ensures(ret.head.as_ref() == &old(head.clone()) && ret.arguments.len() == old(arguments.len()))]
    pub fn new(head: V0Expr, arguments: Vec<ApplicationArgument>) -> Self {
        new!(Application {
            head: Box::new(head),
            arguments,
        })
    }

    /// Borrow the head expression.
    #[requires(true)]
    #[ensures(true)]
    pub fn head(&self) -> &V0Expr {
        &self.head
    }

    /// Borrow ordered arguments.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn arguments(&self) -> &[ApplicationArgument] {
        &self.arguments
    }

    /// Serialize without imposing left association.
    #[requires(true)]
    #[ensures(ret.as_list().is_some_and(|items| items.len() >= 2))]
    fn to_datum(&self) -> Datum {
        let mut values = vec![self.head.to_datum()];
        values.extend(
            self.arguments
                .iter()
                .flat_map(ApplicationArgument::to_datums),
        );
        Datum::list(values)
    }
}

/// An utterance token binder with one or more analyzer facts.
#[invariant(!items.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utterance {
    token: Variable,
    items: Vec<V0Expr>,
}

impl Utterance {
    /// Construct an utterance token boundary.
    #[requires(!items.is_empty())]
    #[ensures(ret.token == old(token.clone()) && ret.items.len() == old(items.len()))]
    pub fn new(token: Variable, items: Vec<V0Expr>) -> Self {
        new!(Utterance { token, items })
    }

    /// Serialize the exact token-binder grammar.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Utterance"))]
    fn to_datum(&self) -> Datum {
        let mut values = vec![Datum::list([Datum::list([
            Datum::atom(self.token.as_str()),
            Datum::atom("UtteranceToken"),
        ])])];
        values.extend(self.items.iter().map(V0Expr::to_datum));
        Datum::form("Utterance", values)
    }
}

/// A sign token binder with one or more analyzer facts.
#[invariant(!items.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sign {
    token: Variable,
    kind: SignKind,
    items: Vec<V0Expr>,
}

impl Sign {
    /// Construct a sign token boundary.
    #[requires(!items.is_empty())]
    #[ensures(ret.token == old(token.clone()) && ret.kind == kind && ret.items.len() == old(items.len()))]
    pub fn new(token: Variable, kind: SignKind, items: Vec<V0Expr>) -> Self {
        new!(Sign { token, kind, items })
    }

    /// Serialize the exact sign-binder grammar.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Sign"))]
    fn to_datum(&self) -> Datum {
        let mut values = vec![Datum::list([Datum::list([
            Datum::atom(self.token.as_str()),
            TypeExpr::SignToken(self.kind).to_datum(),
        ])])];
        values.extend(self.items.iter().map(V0Expr::to_datum));
        Datum::form("Sign", values)
    }
}

/// A positive fallback-object identity.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(PositiveInteger);

impl ObjectId {
    /// Parse `%n`.
    #[requires(true)]
    #[ensures(ret.is_ok() == text.strip_prefix('%').is_some_and(|digits| PositiveInteger::try_new(digits).is_ok()))]
    fn parse(text: &str) -> Result<Self, V0ParseError> {
        let value = text
            .strip_prefix('%')
            .and_then(|digits| PositiveInteger::try_new(digits).ok())
            .ok_or_else(|| V0ParseError::new("fallback object id must be %positive-integer"))?;
        Ok(Self(value))
    }

    /// Borrow the numeric identity for raw-tree ordering validation.
    #[requires(true)]
    #[ensures(*ret > BigUint::from(0u8))]
    fn as_biguint(&self) -> &BigUint {
        self.0.as_biguint()
    }

    /// Serialize the identity token.
    #[requires(true)]
    #[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('%')))]
    fn to_datum(&self) -> Datum {
        Datum::atom(format!("%{}", self.0))
    }
}

/// One raw fallback field.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawField {
    name: NfcText,
    value: RawValue,
}

/// One raw object definition.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawObject {
    id: ObjectId,
    type_name: NfcText,
    fields: Vec<RawField>,
}

/// One inline identity-free product/newtype.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRecord {
    type_name: NfcText,
    fields: Vec<RawField>,
}

/// One inline algebraic-sum constructor.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawVariant {
    enum_type: NfcText,
    constructor: NfcText,
    fields: Vec<RawField>,
}

/// One typed raw-map entry.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMapEntry {
    key: RawValue,
    value: RawValue,
}

/// The closed raw fallback grammar.
#[invariant(::Object(_) => true)]
#[invariant(::Ref(_) => true)]
#[invariant(::Record(_) => true)]
#[invariant(::Variant(_) => true)]
#[invariant(::List(_) => true)]
#[invariant(::Map(_) => true)]
#[invariant(::Atom(_) => true)]
#[invariant(::TypedAtom { .. } => true)]
#[invariant(::Scalar { .. } => true)]
#[invariant(::String(_) => true)]
#[invariant(::Null => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawValue {
    Object(RawObject),
    Ref(ObjectId),
    Record(RawRecord),
    Variant(RawVariant),
    List(Vec<RawValue>),
    Map(Vec<RawMapEntry>),
    Atom(NfcText),
    TypedAtom {
        model_enum_type: NfcText,
        case: NfcText,
    },
    Scalar {
        model_scalar_type: NfcText,
        lexical_value: NfcText,
    },
    String(NfcText),
    Null,
}

impl RawValue {
    /// Serialize one raw production.
    #[requires(true)]
    #[ensures(true)]
    fn to_datum(&self) -> Datum {
        match self {
            Self::Object(object) => {
                let mut values = vec![
                    object.id.to_datum(),
                    Datum::string(object.type_name.as_str()),
                ];
                values.extend(object.fields.iter().map(|field| {
                    Datum::form(
                        "Field",
                        [Datum::string(field.name.as_str()), field.value.to_datum()],
                    )
                }));
                Datum::form("Object", values)
            }
            Self::Ref(id) => Datum::form("Ref", [id.to_datum()]),
            Self::Record(record) => Datum::form(
                "RawRecord",
                std::iter::once(Datum::string(record.type_name.as_str())).chain(
                    record.fields.iter().map(|field| {
                        Datum::form(
                            "Field",
                            [Datum::string(field.name.as_str()), field.value.to_datum()],
                        )
                    }),
                ),
            ),
            Self::Variant(variant) => Datum::form(
                "RawVariant",
                [
                    Datum::string(variant.enum_type.as_str()),
                    Datum::string(variant.constructor.as_str()),
                ]
                .into_iter()
                .chain(variant.fields.iter().map(|field| {
                    Datum::form(
                        "Field",
                        [Datum::string(field.name.as_str()), field.value.to_datum()],
                    )
                })),
            ),
            Self::List(values) => Datum::form("RawList", values.iter().map(Self::to_datum)),
            Self::Map(entries) => Datum::form(
                "RawMap",
                entries.iter().map(|entry| {
                    Datum::form("Entry", [entry.key.to_datum(), entry.value.to_datum()])
                }),
            ),
            Self::Atom(value) => Datum::form("RawAtom", [Datum::string(value.as_str())]),
            Self::TypedAtom {
                model_enum_type,
                case,
            } => Datum::form(
                "RawTypedAtom",
                [
                    Datum::string(model_enum_type.as_str()),
                    Datum::string(case.as_str()),
                ],
            ),
            Self::Scalar {
                model_scalar_type,
                lexical_value,
            } => Datum::form(
                "RawScalar",
                [
                    Datum::string(model_scalar_type.as_str()),
                    Datum::string(lexical_value.as_str()),
                ],
            ),
            Self::String(value) => Datum::form("RawString", [Datum::string(value.as_str())]),
            Self::Null => Datum::form("RawNull", []),
        }
    }
}

/// A raw root with depth-first first-definition/reference ordering proved.
#[invariant(raw_identity_order_is_valid(&root))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTree {
    root: RawValue,
}

impl RawTree {
    /// Validate a raw identity root.
    #[requires(raw_identity_order_is_valid(&root))]
    #[ensures(ret.root == old(root.clone()))]
    pub fn new(root: RawValue) -> Self {
        new!(RawTree { root })
    }

    /// Borrow the raw root.
    #[requires(true)]
    #[ensures(raw_identity_order_is_valid(ret))]
    pub fn root(&self) -> &RawValue {
        &self.root
    }
}

/// A registered ASCII fallback reason spelling.
#[invariant(is_fallback_reason(&text))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FallbackReason {
    text: String,
}

impl FallbackReason {
    /// Parse the stable reason-id lexical domain.
    #[requires(true)]
    #[ensures(ret.is_ok() == is_fallback_reason(text))]
    pub fn try_new(text: &str) -> Result<Self, V0ParseError> {
        if !is_fallback_reason(text) {
            return Err(V0ParseError::new("invalid fallback reason id"));
        }
        Ok(new!(FallbackReason {
            text: text.to_owned(),
        }))
    }

    /// Borrow the stable id.
    #[requires(true)]
    #[ensures(is_fallback_reason(ret))]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// A local fallback that inhabits one known static type.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFallback {
    expected_type: TypeExpr,
    reason: FallbackReason,
    raw: RawTree,
}

impl LocalFallback {
    /// Serialize the exact local fallback grammar.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Fallback"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "Fallback",
            [
                self.expected_type.to_datum(),
                Datum::string(self.reason.as_str()),
                self.raw.root.to_datum(),
            ],
        )
    }
}

/// A whole-document fallback, valid only under `Smusni`.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedGraph {
    model_root_type: NfcText,
    reason: FallbackReason,
    raw: RawTree,
}

impl TypedGraph {
    /// Serialize the whole-document fallback grammar.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("TypedGraph"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "TypedGraph",
            [
                Datum::string(self.model_root_type.as_str()),
                Datum::string(self.reason.as_str()),
                self.raw.root.to_datum(),
            ],
        )
    }
}

/// Every value-position section-2.2 datum production.
#[invariant(::Atom(_) => true)]
#[invariant(::String(_) => true)]
#[invariant(::Integer(_) => true)]
#[invariant(::Rational(_) => true)]
#[invariant(::Variable(_) => true)]
#[invariant(::Application(_) => true)]
#[invariant(::Lambda(_) => true)]
#[invariant(::Let(_) => true)]
#[invariant(::Bind(_) => true)]
#[invariant(::LetRec(_) => true)]
#[invariant(::Utterance(_) => true)]
#[invariant(::Sign(_) => true)]
#[invariant(::Fallback(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V0Expr {
    Atom(ValueAtom),
    String(NfcText),
    Integer(Integer),
    Rational(Rational),
    Variable(Variable),
    Application(Application),
    Lambda(Lambda),
    Let(LetForm),
    Bind(BindForm),
    LetRec(LetRec),
    Utterance(Utterance),
    Sign(Sign),
    Fallback(LocalFallback),
}

impl V0Expr {
    /// Parse one exact expression datum.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn parse(datum: &Datum) -> Result<Self, V0ParseError> {
        parse_expression(datum)
    }

    /// Serialize through the one shared escaping path.
    #[requires(true)]
    #[ensures(true)]
    pub fn to_datum(&self) -> Datum {
        match self {
            Self::Atom(atom) => Datum::atom(atom.as_str()),
            Self::String(text) => Datum::string(text.as_str()),
            Self::Integer(integer) => Datum::Integer(integer.clone()),
            Self::Rational(rational) => rational.to_datum(),
            Self::Variable(variable) => Datum::atom(variable.as_str()),
            Self::Application(application) => application.to_datum(),
            Self::Lambda(lambda) => lambda.to_datum(),
            Self::Let(form) => form.to_datum(),
            Self::Bind(form) => form.to_datum(),
            Self::LetRec(form) => form.to_datum(),
            Self::Utterance(form) => form.to_datum(),
            Self::Sign(form) => form.to_datum(),
            Self::Fallback(fallback) => fallback.to_datum(),
        }
    }
}

/// One dictionary word card.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordCard {
    root: LexicalRoot,
    definition: NfcText,
}

impl WordCard {
    /// Serialize the closed `Word` schema.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Word"))]
    fn to_datum(&self) -> Datum {
        Datum::form(
            "Word",
            [
                Datum::atom(self.root.as_str()),
                Datum::string(self.definition.as_str()),
            ],
        )
    }
}

/// The two grammar-level document-body alternatives.
#[invariant(::Performable(_) => true)]
#[invariant(::TypedGraph(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentBody {
    Performable(V0Expr),
    TypedGraph(TypedGraph),
}

/// One complete version-0 document.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V0Document {
    body: DocumentBody,
    words: Option<Vec<WordCard>>,
}

impl V0Document {
    /// Borrow the syntax-level body.
    #[requires(true)]
    #[ensures(true)]
    pub fn body(&self) -> &DocumentBody {
        &self.body
    }

    /// Borrow optional word cards.
    #[requires(true)]
    #[ensures(ret.is_some() == self.words.is_some())]
    pub fn words(&self) -> Option<&[WordCard]> {
        self.words.as_deref()
    }

    /// Serialize exactly `(Smusni 0 body [words])`.
    #[requires(true)]
    #[ensures(ret.form_head() == Some("Smusni"))]
    pub fn to_datum(&self) -> Datum {
        let body = match &self.body {
            DocumentBody::Performable(expression) => expression.to_datum(),
            DocumentBody::TypedGraph(graph) => graph.to_datum(),
        };
        let mut values = vec![Datum::unsigned(0), body];
        if let Some(words) = &self.words {
            values.push(Datum::form("Words", words.iter().map(WordCard::to_datum)));
        }
        Datum::form("Smusni", values)
    }
}

/// Parse one complete version-0 document from text.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_v0_document(input: &str) -> Result<V0Document, V0ParseError> {
    let datum = parse_document(input).map_err(|error| V0ParseError::new(error.to_string()))?;
    parse_document_datum(&datum)
}

/// Parse one declared fragment expression.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_v0_expression(input: &str) -> Result<V0Expr, V0ParseError> {
    let datum = parse_document(input).map_err(|error| V0ParseError::new(error.to_string()))?;
    parse_expression(&datum)
}

/// Parse one or more fragment expressions from one fenced block.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_v0_expressions(input: &str) -> Result<Vec<V0Expr>, V0ParseError> {
    parse_datums(input)
        .map_err(|error| V0ParseError::new(error.to_string()))?
        .iter()
        .map(parse_expression)
        .collect()
}

/// Print a validated document with exactly one trailing newline.
#[requires(true)]
#[ensures(ret.ends_with('\n') && !ret.ends_with("\n\n"))]
pub fn print_v0_document(document: &V0Document) -> String {
    print_document(&document.to_datum())
}

/// Exact-grammar failure.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V0ParseError {
    message: String,
}

impl V0ParseError {
    /// Construct a nonempty failure.
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        assert!(!message.is_empty(), "syntax failures require a message");
        new!(V0ParseError { message })
    }
}

impl fmt::Display for V0ParseError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for V0ParseError {}

/// Validate and parse the document packaging grammar.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_document_datum(datum: &Datum) -> Result<V0Document, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("document must be a Smusni form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Smusni")
        || !matches!(items.len(), 3 | 4)
        || items.get(1).and_then(Datum::as_integer) != Some("0")
    {
        return Err(V0ParseError::new(
            "document must be exactly (Smusni 0 performable [words])",
        ));
    }
    let body = if items[2].form_head() == Some("TypedGraph") {
        DocumentBody::TypedGraph(parse_typed_graph(&items[2])?)
    } else {
        DocumentBody::Performable(parse_expression(&items[2])?)
    };
    let words = items.get(3).map(parse_words).transpose()?;
    Ok(V0Document { body, words })
}

/// Parse one expression, dispatching every special form before application.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_expression(datum: &Datum) -> Result<V0Expr, V0ParseError> {
    if let Some(integer) = datum.as_integer() {
        return Ok(V0Expr::Integer(
            Integer::try_new(integer).expect("raw datum integers are canonical"),
        ));
    }
    if let Datum::String(text) = datum {
        return Ok(V0Expr::String(NfcText::new(text)));
    }
    if let Some(atom) = datum.as_atom() {
        if atom.starts_with('$') {
            return Variable::try_new(atom)
                .map(V0Expr::Variable)
                .map_err(|error| V0ParseError::new(error.to_string()));
        }
        return ValueAtom::try_new(atom).map(V0Expr::Atom);
    }
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("expression has no valid datum production"))?;
    let head = items.first().and_then(Datum::as_atom);
    match head {
        Some("/") => parse_rational(items).map(V0Expr::Rational),
        Some("λ") => parse_lambda(items).map(V0Expr::Lambda),
        Some("Let") => parse_let(items).map(V0Expr::Let),
        Some("Bind") => parse_bind(items).map(V0Expr::Bind),
        Some("LetRec") => parse_let_rec(items).map(V0Expr::LetRec),
        Some("Utterance") => parse_utterance(items).map(V0Expr::Utterance),
        Some("Sign") => parse_sign(items).map(V0Expr::Sign),
        Some("Fallback") => parse_local_fallback(items).map(V0Expr::Fallback),
        Some(head) if RESERVED_NON_APPLICATION_HEADS.contains(&head) => Err(V0ParseError::new(
            format!("{head} is reserved for a different grammar production"),
        )),
        _ => parse_application(items).map(V0Expr::Application),
    }
}

/// Parse exact rational syntax.
#[requires(items.first().and_then(Datum::as_atom) == Some("/"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_rational(items: &[Datum]) -> Result<Rational, V0ParseError> {
    require_len(items, 3, "rational")?;
    let numerator = integer_from(&items[1], "rational numerator")?;
    let denominator = integer_from(&items[2], "rational denominator")?;
    Rational::try_new(numerator, denominator)
}

/// Parse exact lambda syntax.
#[requires(items.first().and_then(Datum::as_atom) == Some("λ"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_lambda(items: &[Datum]) -> Result<Lambda, V0ParseError> {
    require_len(items, 3, "lambda")?;
    let parameters = parse_declarations(&items[1])?;
    if parameters.is_empty() || !declarations_are_unique(&parameters) {
        return Err(V0ParseError::new(
            "lambda parameters must be nonempty and unique",
        ));
    }
    Ok(Lambda::new(parameters, parse_expression(&items[2])?))
}

/// Parse canonical single-binding `Let` syntax.
#[requires(items.first().and_then(Datum::as_atom) == Some("Let"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_let(items: &[Datum]) -> Result<LetForm, V0ParseError> {
    require_len(items, 3, "Let")?;
    let bindings = items[1]
        .as_list()
        .ok_or_else(|| V0ParseError::new("Let binding container must be a list"))?;
    if bindings.len() != 1 {
        return Err(V0ParseError::new(
            "canonical Let contains exactly one binding",
        ));
    }
    Ok(LetForm::new(
        parse_let_binding(&bindings[0])?,
        parse_expression(&items[2])?,
    ))
}

/// Parse canonical single-variable `Bind` syntax.
#[requires(items.first().and_then(Datum::as_atom) == Some("Bind"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_bind(items: &[Datum]) -> Result<BindForm, V0ParseError> {
    require_len(items, 3, "Bind")?;
    let bindings = items[1]
        .as_list()
        .ok_or_else(|| V0ParseError::new("Bind binding container must be a list"))?;
    if bindings.len() != 1 {
        return Err(V0ParseError::new(
            "canonical Bind contains exactly one binding",
        ));
    }
    Ok(BindForm::new(
        parse_value_binding(&bindings[0])?,
        parse_expression(&items[2])?,
    ))
}

/// Parse a guarded LetRec.
#[requires(items.first().and_then(Datum::as_atom) == Some("LetRec"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_let_rec(items: &[Datum]) -> Result<LetRec, V0ParseError> {
    require_len(items, 3, "LetRec")?;
    let bindings = items[1]
        .as_list()
        .ok_or_else(|| V0ParseError::new("LetRec bindings must be a list"))?
        .iter()
        .map(parse_value_binding)
        .collect::<Result<Vec<_>, _>>()?;
    if bindings.is_empty()
        || !bindings_are_unique(&bindings)
        || bindings
            .iter()
            .any(|binding| !matches!(binding.value.as_ref(), V0Expr::Lambda(_)))
    {
        return Err(V0ParseError::new(
            "LetRec requires unique, nonempty lambda bindings",
        ));
    }
    Ok(LetRec::new(bindings, parse_expression(&items[2])?))
}

/// Parse one binding triple.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_value_binding(datum: &Datum) -> Result<ValueBinding, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("value binding must be a triple"))?;
    require_len(items, 3, "value binding")?;
    let variable = parse_variable(&items[0])?;
    let declared_type =
        TypeExpr::parse(&items[1]).map_err(|error| V0ParseError::new(error.to_string()))?;
    Ok(ValueBinding::new(
        Declaration::new(variable, declared_type),
        parse_expression(&items[2])?,
    ))
}

/// Parse either exact `let-name` production.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_let_binding(datum: &Datum) -> Result<LetBinding, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("Let binding must be a triple"))?;
    require_len(items, 3, "Let binding")?;
    if items[0].as_atom().is_some_and(|name| name.starts_with('$')) {
        return parse_value_binding(datum).map(LetBinding::Variable);
    }
    let name = items[0]
        .as_atom()
        .ok_or_else(|| V0ParseError::new("Let name must be a variable or prelude name"))?;
    let declared_type =
        TypeExpr::parse(&items[1]).map_err(|error| V0ParseError::new(error.to_string()))?;
    Ok(LetBinding::Prelude(PreludeBinding::new(
        PreludeName::try_new(name)?,
        declared_type,
        parse_expression(&items[2])?,
    )))
}

/// Parse lambda declarations.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_declarations(datum: &Datum) -> Result<Vec<Declaration>, V0ParseError> {
    datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("declarations must be a list"))?
        .iter()
        .map(|datum| {
            let items = datum
                .as_list()
                .ok_or_else(|| V0ParseError::new("declaration must be a pair"))?;
            require_len(items, 2, "declaration")?;
            Ok(Declaration::new(
                parse_variable(&items[0])?,
                TypeExpr::parse(&items[1]).map_err(|error| V0ParseError::new(error.to_string()))?,
            ))
        })
        .collect()
}

/// Parse an utterance token binder.
#[requires(items.first().and_then(Datum::as_atom) == Some("Utterance"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_utterance(items: &[Datum]) -> Result<Utterance, V0ParseError> {
    if items.len() < 3 {
        return Err(V0ParseError::new("Utterance requires a token and facts"));
    }
    let (token, declared_type) = parse_token_declaration(&items[1])?;
    if declared_type != TypeExpr::Atom(super::type_system::TypeAtom::UtteranceToken) {
        return Err(V0ParseError::new(
            "Utterance binder must declare UtteranceToken",
        ));
    }
    let facts = items[2..]
        .iter()
        .map(parse_expression)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Utterance::new(token, facts))
}

/// Parse a sign token binder.
#[requires(items.first().and_then(Datum::as_atom) == Some("Sign"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_sign(items: &[Datum]) -> Result<Sign, V0ParseError> {
    if items.len() < 3 {
        return Err(V0ParseError::new("Sign requires a token and facts"));
    }
    let (token, declared_type) = parse_token_declaration(&items[1])?;
    let TypeExpr::SignToken(kind) = declared_type else {
        return Err(V0ParseError::new(
            "Sign binder must declare (SignToken sign-kind)",
        ));
    };
    let facts = items[2..]
        .iter()
        .map(parse_expression)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Sign::new(token, kind, facts))
}

/// Parse the one-entry token binder list shared by Utterance and Sign.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_token_declaration(datum: &Datum) -> Result<(Variable, TypeExpr), V0ParseError> {
    let outer = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("token binder must be a list"))?;
    if outer.len() != 1 {
        return Err(V0ParseError::new(
            "token binder must contain one declaration",
        ));
    }
    let inner = outer[0]
        .as_list()
        .ok_or_else(|| V0ParseError::new("token declaration must be a pair"))?;
    require_len(inner, 2, "token declaration")?;
    Ok((
        parse_variable(&inner[0])?,
        TypeExpr::parse(&inner[1]).map_err(|error| V0ParseError::new(error.to_string()))?,
    ))
}

/// Parse an ordinary application and its separate argument productions.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_application(items: &[Datum]) -> Result<Application, V0ParseError> {
    if items.len() < 2 {
        return Err(V0ParseError::new(
            "application requires a head and at least one argument",
        ));
    }
    let head = parse_expression(&items[0])?;
    let mut arguments = Vec::new();
    let mut index = 1usize;
    while index < items.len() {
        if let Some(marker) = items[index].as_atom().and_then(parse_place_marker) {
            let value = items
                .get(index + 1)
                .ok_or_else(|| V0ParseError::new("place marker requires a following value"))?;
            arguments.push(match marker {
                PlaceMarker::Numbered(place) => ApplicationArgument::NumberedPlace {
                    place,
                    value: Box::new(parse_expression(value)?),
                },
                PlaceMarker::Eventuality => ApplicationArgument::EventualityPlace {
                    value: Box::new(parse_expression(value)?),
                },
            });
            index += 2;
            continue;
        }
        if items[index].form_head() == Some("At") {
            let fields = items[index].as_list().expect("form head implies list");
            require_len(fields, 3, "At")?;
            arguments.push(ApplicationArgument::ComputedPlace {
                place: Box::new(parse_expression(&fields[1])?),
                value: Box::new(parse_expression(&fields[2])?),
            });
        } else {
            arguments.push(ApplicationArgument::Value(Box::new(parse_expression(
                &items[index],
            )?)));
        }
        index += 1;
    }
    Ok(Application::new(head, arguments))
}

/// Parse one local fallback.
#[requires(items.first().and_then(Datum::as_atom) == Some("Fallback"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_local_fallback(items: &[Datum]) -> Result<LocalFallback, V0ParseError> {
    require_len(items, 4, "Fallback")?;
    let expected_type =
        TypeExpr::parse(&items[1]).map_err(|error| V0ParseError::new(error.to_string()))?;
    let reason = items[2]
        .as_string()
        .ok_or_else(|| V0ParseError::new("Fallback reason must be a string"))?;
    let raw = parse_raw_tree(&items[3])?;
    Ok(LocalFallback {
        expected_type,
        reason: FallbackReason::try_new(reason)?,
        raw,
    })
}

/// Parse whole-document fallback.
#[requires(datum.form_head() == Some("TypedGraph"))]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_typed_graph(datum: &Datum) -> Result<TypedGraph, V0ParseError> {
    let items = datum.as_list().expect("form head implies list");
    require_len(items, 4, "TypedGraph")?;
    let root_type = items[1]
        .as_string()
        .ok_or_else(|| V0ParseError::new("TypedGraph root type must be a string"))?;
    if root_type.is_empty() {
        return Err(V0ParseError::new("TypedGraph root type must not be empty"));
    }
    let reason = items[2]
        .as_string()
        .ok_or_else(|| V0ParseError::new("TypedGraph reason must be a string"))?;
    let raw = parse_raw_tree(&items[3])?;
    let RawValue::Object(root) = raw.root() else {
        return Err(V0ParseError::new("TypedGraph raw root must be an Object"));
    };
    if root.id.as_biguint() != &BigUint::from(1u8) || root.type_name.as_str() != root_type {
        return Err(V0ParseError::new(
            "TypedGraph raw root must define %1 with the declared root type",
        ));
    }
    Ok(TypedGraph {
        model_root_type: NfcText::new(root_type),
        reason: FallbackReason::try_new(reason)?,
        raw,
    })
}

/// Parse and validate a raw identity root.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_tree(datum: &Datum) -> Result<RawTree, V0ParseError> {
    let root = parse_raw_value(datum)?;
    if !raw_identity_order_is_valid(&root) {
        return Err(V0ParseError::new(
            "raw object identities must be first-defined depth-first as %1, %2, ...",
        ));
    }
    Ok(RawTree::new(root))
}

/// Parse the closed raw grammar recursively.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_value(datum: &Datum) -> Result<RawValue, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("raw value must use a registered raw form"))?;
    match items.first().and_then(Datum::as_atom) {
        Some("Object") => {
            if items.len() < 3 {
                return Err(V0ParseError::new("Object requires id and type name"));
            }
            let id = items[1]
                .as_atom()
                .ok_or_else(|| V0ParseError::new("Object id must be an atom"))
                .and_then(ObjectId::parse)?;
            let type_name = text_from(&items[2], "Object type name")?;
            let fields = items[3..]
                .iter()
                .map(parse_raw_field)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(RawValue::Object(RawObject {
                id,
                type_name,
                fields,
            }))
        }
        Some("Ref") => {
            require_len(items, 2, "Ref")?;
            let id = items[1]
                .as_atom()
                .ok_or_else(|| V0ParseError::new("Ref id must be an atom"))
                .and_then(ObjectId::parse)?;
            Ok(RawValue::Ref(id))
        }
        Some("RawRecord") => {
            if items.len() < 2 {
                return Err(V0ParseError::new("RawRecord requires a type name"));
            }
            Ok(RawValue::Record(RawRecord {
                type_name: text_from(&items[1], "RawRecord type name")?,
                fields: items[2..]
                    .iter()
                    .map(parse_raw_field)
                    .collect::<Result<Vec<_>, _>>()?,
            }))
        }
        Some("RawVariant") => {
            if items.len() < 3 {
                return Err(V0ParseError::new(
                    "RawVariant requires an enum type and constructor",
                ));
            }
            Ok(RawValue::Variant(RawVariant {
                enum_type: text_from(&items[1], "RawVariant enum type")?,
                constructor: text_from(&items[2], "RawVariant constructor")?,
                fields: items[3..]
                    .iter()
                    .map(parse_raw_field)
                    .collect::<Result<Vec<_>, _>>()?,
            }))
        }
        Some("RawList") => Ok(RawValue::List(
            items[1..]
                .iter()
                .map(parse_raw_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Some("RawMap") => Ok(RawValue::Map(
            items[1..]
                .iter()
                .map(parse_raw_entry)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Some("RawAtom") => {
            require_len(items, 2, "RawAtom")?;
            Ok(RawValue::Atom(text_from(&items[1], "RawAtom")?))
        }
        Some("RawTypedAtom") => {
            require_len(items, 3, "RawTypedAtom")?;
            Ok(RawValue::TypedAtom {
                model_enum_type: text_from(&items[1], "RawTypedAtom type")?,
                case: text_from(&items[2], "RawTypedAtom case")?,
            })
        }
        Some("RawScalar") => {
            require_len(items, 3, "RawScalar")?;
            Ok(RawValue::Scalar {
                model_scalar_type: text_from(&items[1], "RawScalar type")?,
                lexical_value: text_from(&items[2], "RawScalar value")?,
            })
        }
        Some("RawString") => {
            require_len(items, 2, "RawString")?;
            Ok(RawValue::String(text_from(&items[1], "RawString")?))
        }
        Some("RawNull") => {
            require_len(items, 1, "RawNull")?;
            Ok(RawValue::Null)
        }
        _ => Err(V0ParseError::new("unknown raw fallback form")),
    }
}

/// Parse one exact `(Entry raw raw)`.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_entry(datum: &Datum) -> Result<RawMapEntry, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("raw map entry must be an Entry form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Entry") {
        return Err(V0ParseError::new("RawMap may contain only Entry forms"));
    }
    require_len(items, 3, "Entry")?;
    Ok(RawMapEntry {
        key: parse_raw_value(&items[1])?,
        value: parse_raw_value(&items[2])?,
    })
}

/// Parse one exact `(Field string raw)`.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_raw_field(datum: &Datum) -> Result<RawField, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("raw field must be a Field form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Field") {
        return Err(V0ParseError::new("raw object child must be Field"));
    }
    require_len(items, 3, "Field")?;
    Ok(RawField {
        name: text_from(&items[1], "Field name")?,
        value: parse_raw_value(&items[2])?,
    })
}

/// Parse optional document word cards.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_words(datum: &Datum) -> Result<Vec<WordCard>, V0ParseError> {
    let items = datum
        .as_list()
        .ok_or_else(|| V0ParseError::new("Words section must be a form"))?;
    if items.first().and_then(Datum::as_atom) != Some("Words") {
        return Err(V0ParseError::new("third document child must be Words"));
    }
    // The grammar is `words ::= (Words word-card+)`, and section 2.4 requires
    // omitting the section entirely when there is nothing to declare.
    if items.len() < 2 {
        return Err(V0ParseError::new(
            "Words requires at least one Word card; omit the section when empty",
        ));
    }
    items[1..]
        .iter()
        .map(|word| {
            let fields = word
                .as_list()
                .ok_or_else(|| V0ParseError::new("word card must be a Word form"))?;
            if fields.first().and_then(Datum::as_atom) != Some("Word") {
                return Err(V0ParseError::new("Words may contain only Word cards"));
            }
            require_len(fields, 3, "Word")?;
            let root = fields[1]
                .as_atom()
                .ok_or_else(|| V0ParseError::new("Word root must be a lexical atom"))?;
            Ok(WordCard {
                root: LexicalRoot::try_new(root)
                    .map_err(|error| V0ParseError::new(error.to_string()))?,
                definition: text_from(&fields[2], "Word definition")?,
            })
        })
        .collect()
}

/// Parse `$name` from an atom datum.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn parse_variable(datum: &Datum) -> Result<Variable, V0ParseError> {
    datum
        .as_atom()
        .ok_or_else(|| V0ParseError::new("variable must be an atom"))
        .and_then(|atom| {
            Variable::try_new(atom).map_err(|error| V0ParseError::new(error.to_string()))
        })
}

/// Parse an integer payload from a raw datum.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn integer_from(datum: &Datum, context: &str) -> Result<Integer, V0ParseError> {
    let text = datum
        .as_integer()
        .ok_or_else(|| V0ParseError::new(format!("{context} must be an integer")))?;
    Ok(Integer::try_new(text).expect("raw datum integers are canonical"))
}

/// Parse one string into NFC text.
#[requires(!context.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn text_from(datum: &Datum, context: &str) -> Result<NfcText, V0ParseError> {
    datum
        .as_string()
        .map(NfcText::new)
        .ok_or_else(|| V0ParseError::new(format!("{context} must be a string")))
}

/// Place marker recognized only in application-argument position.
#[invariant(::Numbered(_) => true)]
#[invariant(::Eventuality => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum PlaceMarker {
    Numbered(PositiveInteger),
    Eventuality,
}

/// Parse `:n` and `:Eventuality` without accepting them as values.
#[requires(true)]
#[ensures(true)]
fn parse_place_marker(atom: &str) -> Option<PlaceMarker> {
    let label = atom.strip_prefix(':')?;
    if label == "Eventuality" {
        return Some(PlaceMarker::Eventuality);
    }
    PositiveInteger::try_new(label)
        .ok()
        .map(PlaceMarker::Numbered)
}

/// Validate rational sign, denominator, and reduction.
#[requires(true)]
#[ensures(true)]
fn is_reduced_rational(numerator: &str, denominator: &str) -> bool {
    if denominator.starts_with('-') || denominator == "0" {
        return false;
    }
    let Ok(numerator) = numerator.parse::<BigInt>() else {
        return false;
    };
    let Ok(denominator) = denominator.parse::<BigInt>() else {
        return false;
    };
    numerator.gcd(&denominator) == BigInt::from(1u8)
}

/// Check unique declaration names.
#[requires(true)]
#[ensures(true)]
fn declarations_are_unique(declarations: &[Declaration]) -> bool {
    let mut seen = BTreeSet::new();
    declarations
        .iter()
        .all(|declaration| seen.insert(declaration.variable.as_str()))
}

/// Check unique recursive binding names.
#[requires(true)]
#[ensures(true)]
fn bindings_are_unique(bindings: &[ValueBinding]) -> bool {
    let mut seen = BTreeSet::new();
    bindings
        .iter()
        .all(|binding| seen.insert(binding.declaration.variable.as_str()))
}

/// Validate depth-first `%id` definition/reference order.
#[requires(true)]
#[ensures(true)]
fn raw_identity_order_is_valid(root: &RawValue) -> bool {
    #[requires(true)]
    #[ensures(true)]
    fn visit(value: &RawValue, defined: &mut BTreeSet<ObjectId>, next: &mut BigUint) -> bool {
        match value {
            RawValue::Object(object) => {
                if object.id.as_biguint() != next || !defined.insert(object.id.clone()) {
                    return false;
                }
                *next += 1u8;
                object
                    .fields
                    .iter()
                    .all(|field| visit(&field.value, defined, next))
            }
            RawValue::Ref(id) => defined.contains(id),
            RawValue::Record(record) => record
                .fields
                .iter()
                .all(|field| visit(&field.value, defined, next)),
            RawValue::Variant(variant) => variant
                .fields
                .iter()
                .all(|field| visit(&field.value, defined, next)),
            RawValue::List(values) => values.iter().all(|value| visit(value, defined, next)),
            RawValue::Map(entries) => entries.iter().all(|entry| {
                visit(&entry.key, defined, next) && visit(&entry.value, defined, next)
            }),
            RawValue::Atom(_)
            | RawValue::TypedAtom { .. }
            | RawValue::Scalar { .. }
            | RawValue::String(_)
            | RawValue::Null => true,
        }
    }
    visit(root, &mut BTreeSet::new(), &mut BigUint::from(1u8))
}

/// Stable fallback ids are ASCII names in the closed `smusni.fallback`
/// namespace.
#[requires(true)]
#[ensures(true)]
fn is_fallback_reason(text: &str) -> bool {
    text.starts_with("smusni.fallback.")
        && text.len() > "smusni.fallback.".len()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

/// Classify the three disjoint atom namespaces.
#[requires(true)]
#[ensures(true)]
fn classify_value_atom(token: &str) -> Option<AtomClass> {
    if CALLABLE_GLYPHS.contains(&token) {
        return Some(AtomClass::CallableGlyph);
    }
    if REGISTERED_PASCAL_ATOMS.contains(&token) {
        return Some(AtomClass::RegisteredPascal);
    }
    LexicalRoot::try_new(token)
        .ok()
        .map(|_| AtomClass::LexicalRoot)
}

/// Require an exact list size.
#[requires(!context.is_empty())]
#[ensures(ret.is_ok() == (items.len() == expected))]
fn require_len(items: &[Datum], expected: usize, context: &str) -> Result<(), V0ParseError> {
    if items.len() != expected {
        return Err(V0ParseError::new(format!(
            "{context} requires {} items, found {}",
            expected,
            items.len()
        )));
    }
    Ok(())
}

const CALLABLE_GLYPHS: &[&str] = &[
    "¬", "∧", "∨", "→", "↔", "⊕", "∀", "∃", "=", "≠", "<", "≤", ">", "≥", "+", "−", "×", "÷", "∈",
    "∪", "∩",
];

const PRELUDE_NAMES: &[&str] = &[
    "DescribedAs",
    "Named",
    "InnatelyCapable",
    "MotionVector",
    "Some",
    "No",
    "Every",
    "Exactly",
    "AtLeast",
    "AtMost",
    "MoreThan",
    "FewerThan",
    "≠",
    ">",
    "≥",
    "∪",
    "∩",
    "This",
    "That",
    "Yonder",
];

// This is the closed section-14.1 callable/prelude/literal namespace plus the
// two generated names used by the frozen samples. Checkpoint 2 replaces those
// generated admissions with the minted registry table; unknown PascalCase
// spellings remain rejected.
const REGISTERED_PASCAL_ATOMS: &[&str] = &[
    "Abstract",
    "ActivityOf",
    "Address",
    "Among",
    "Answer",
    "Assertion",
    "Ask",
    "Assert",
    "AtLeast",
    "AtMost",
    "Audience",
    "AudienceOf",
    "Card",
    "Closed",
    "Close",
    "Combine",
    "Command",
    "Concept",
    "Context",
    "ContextualAnswer",
    "Contrast",
    "CurrentGround",
    "Deictic",
    "DeicticPlaceOf",
    "DeicticTimeOf",
    "Denotes",
    "DescribedAs",
    "Directive",
    "DistanceScale",
    "Distal",
    "Division",
    "Do",
    "DropPlace",
    "Every",
    "Exactly",
    "Exhaustive",
    "ExperienceOf",
    "Express",
    "Expressive",
    "Extensional",
    "FewerThan",
    "FollowingDiscourse",
    "Here",
    "InnatelyCapable",
    "Intensional",
    "InterpretAct",
    "InterpretContent",
    "Interval",
    "Item",
    "Joi",
    "Label",
    "Letteral",
    "List",
    "LocutionOf",
    "MathExpression",
    "Measure",
    "Medial",
    "Mention",
    "MentionSome",
    "Mentioning",
    "MoreThan",
    "MotionVector",
    "Name",
    "Named",
    "NameSign",
    "Neutral",
    "NewTopic",
    "No",
    "Now",
    "Opaque",
    "OpaqueQuote",
    "Open",
    "OpenQ",
    "Opposite",
    "OtherThan",
    "Perform",
    "PerformUtterance",
    "Polar",
    "PolarAnswer",
    "Presuppose",
    "PriorDiscourse",
    "ProcessOf",
    "Proximal",
    "Question",
    "QuestionOf",
    "Quotation",
    "Quotes",
    "Realizes",
    "Refer",
    "Reify",
    "Resume",
    "Scalar",
    "Sentence",
    "SentenceSign",
    "Set",
    "SetOf",
    "Singleton",
    "Some",
    "Speaker",
    "SpeakerOf",
    "Stereotypical",
    "Structured",
    "StructuredQuote",
    "Supplement",
    "Tanru",
    "Text",
    "TextOf",
    "That",
    "This",
    "TruthValue",
    "Tuple",
    "TupleAnswer",
    "Typical",
    "Unknown",
    "UnresolvedAnswer",
    "Utters",
    "Vocative",
    "Witnesses",
    "Word",
    "Yonder",
    "Yes",
    "ZipWith",
];

const RESERVED_NON_APPLICATION_HEADS: &[&str] = &[
    "At",
    "Entry",
    "Field",
    "Object",
    "RawAtom",
    "RawList",
    "RawMap",
    "RawNull",
    "RawRecord",
    "RawScalar",
    "RawString",
    "RawTypedAtom",
    "RawVariant",
    "Ref",
    "Smusni",
    "TypedGraph",
    "Word",
    "Words",
];

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_document_round_trip_preserves_all_special_forms() {
        let source = r#"(Smusni 0
  (Let (($act (Act Assertion)
          (Assert (/ -1 2))))
    (Do
      $act
      (Bind (($x (Referents Entity) (Refer (λ (($r (Referents Entity))) (klama $r)))))
        (Utterance (($u UtteranceToken))
          (Sign (($s (SignToken Sentence)))
            (TextOf $s "mi klama"))
          (Realizes $u (Assert (klama :2 $x (At $p This))))))))
  (Words (Word klama "x1 goes")))"#;
        let document = parse_v0_document(source).expect("complete grammar specimen parses");
        let rendered = print_v0_document(&document);
        assert_eq!(parse_v0_document(&rendered), Ok(document));
        assert!(rendered.ends_with('\n'));
        assert!(!rendered.ends_with("\n\n"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fallback_grammar_enforces_identity_order_and_reason_domain() {
        let unbounded = "%429496729600000000000000000000";
        let object_id = ObjectId::parse(unbounded).expect("unbounded object id parses");
        assert_eq!(object_id.to_datum(), Datum::atom(unbounded));

        assert!(parse_v0_document(
            r#"(Smusni 0 (TypedGraph "SemanticGraph" "smusni.fallback.test" (Object %1 "SemanticGraph" (Field "self" (Ref %1)))))"#
        )
        .is_ok());
        assert!(
            parse_v0_document(r#"(Smusni 0 (TypedGraph "SemanticGraph" "smusni.fallback.test" (Object %2 "SemanticGraph")))"#)
                .is_err()
        );
        assert!(
            parse_v0_expression(r#"(Fallback Content "not registered" (Object %1 "Root"))"#)
                .is_err()
        );
        for malformed in [
            r#"(Fallback Content "smusni." (RawNull))"#,
            r#"(Fallback Content "smusni.Bad" (RawNull))"#,
            r#"(Fallback Content "smusni.fallback.test" (Ref %1))"#,
            r#"(Fallback Content "smusni.fallback.test" (Object %1 "Root" (Field "x" (Object %1 "Again"))))"#,
            r#"(Fallback Content "smusni.fallback.test" (UnknownRaw))"#,
            r#"(Fallback Content "smusni.fallback.test" (RawNull extra))"#,
            r#"(Fallback Content "smusni.fallback.test" "raw")"#,
        ] {
            assert!(
                parse_v0_expression(malformed).is_err(),
                "accepted malformed fallback {malformed}",
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn malformed_lexical_and_structured_forms_are_rejected() {
        assert!(parse_v0_expression("(Let ((This (Referents Entity) That)) This)").is_ok());
        assert!(parse_v0_expression("(klama :429496729600000000000000000000 This)").is_ok());
        for malformed in [
            "BogusPascal",
            "01",
            "-0",
            "(/ 2 4)",
            "(/ 1 -2)",
            "(λ () This)",
            "(Let (($x Entity This) ($y Entity That)) $x)",
            "(Let ((BogusPrelude Entity This)) This)",
            "(Bind ((This Entity That)) This)",
            "(LetRec (($x Entity This)) $x)",
            "(LetRec ((This Entity (λ (($x Entity)) $x))) This)",
            "(At $p This)",
            "(klama :0 This)",
        ] {
            assert!(
                parse_v0_expression(malformed).is_err(),
                "accepted malformed {malformed}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn binder_types_use_only_closed_atoms_or_complete_constructors() {
        for valid in [
            "(Let (($state State Context)) $state)",
            "(Let (($events (Referents Process) Context)) $events)",
            "(Let (($set (Set Entity) Context)) $set)",
            "(Let (($predicate (PredTerm (Row (1 (Referents Entity)) Open)) klama)) $predicate)",
        ] {
            assert!(
                parse_v0_expression(valid).is_ok(),
                "rejected closed type specimen {valid}",
            );
        }
        for invalid in [
            "(Let (($state Eventuality/State Context)) $state)",
            "(Let (($set Set Context)) $set)",
            "(Let (($predicate PredTerm klama)) $predicate)",
            "(Let (($relation Relation klama)) $relation)",
            "(Let (($mass Mass Context)) $mass)",
        ] {
            assert!(
                parse_v0_expression(invalid).is_err(),
                "accepted invented or incomplete type specimen {invalid}",
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vertical_bar_symbols_and_hostile_strings_round_trip() {
        let expression =
            parse_v0_expression(r#"(|lo jban| "quote: \" slash: \\ newline:\n nul:\u0000")"#)
                .expect("escaped lexical head parses");
        let datum = expression.to_datum();
        let text = print_document(&datum);
        assert_eq!(parse_v0_expression(&text), Ok(expression));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_and_word_card_grammar_is_closed() {
        for malformed in [
            r#"(Smusni 1 (Assert This))"#,
            r#"(Smusni 0 (Assert This) (Words) extra)"#,
            // `words ::= (Words word-card+)`: an empty section is not the same
            // as an omitted one.
            r#"(Smusni 0 (Assert This) (Words))"#,
            r#"(Smusni 0 (Assert This) (Word klama "goes"))"#,
            r#"(Smusni 0 (Assert This) (Words (Word Klama "goes")))"#,
            r#"(Smusni 0 (Assert This) (Words (Word klama "goes" extra)))"#,
            // `λ` is the lambda marker, so it is not a word-card root either.
            r#"(Smusni 0 (Assert This) (Words (Word λ "lambda")))"#,
        ] {
            assert!(
                parse_v0_document(malformed).is_err(),
                "accepted malformed document {malformed}",
            );
        }
        // A document with no word cards omits the section entirely.
        assert!(parse_v0_document("(Smusni 0 (Assert This))\n").is_ok());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reserved_lambda_marker_is_not_a_value_position_atom() {
        // The marker still heads its own special form.
        assert!(parse_v0_expression("(λ (($x Entity)) (klama $x))").is_ok());
        // ... but it is not a callable atom or a bare operand.
        for malformed in ["λ", "(λ)", "(Assert λ)", "(klama λ)", "(λ This)"] {
            assert!(
                parse_v0_expression(malformed).is_err(),
                "accepted reserved lambda marker in value position: {malformed}",
            );
        }
        // The escaped spelling stays available and cannot be confused with it.
        assert!(parse_v0_expression("(|λ| This)").is_ok());
    }
}
