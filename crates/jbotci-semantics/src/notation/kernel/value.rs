//! First-order values, callables, reference computations, and the polymorphic
//! operand union.
//!
//! Specification section 3.1 stratifies the language by semantic category, and
//! the kernel follows it: a `Refer` outside a `Bind` initializer or an act in
//! ordinary value position is unrepresentable rather than merely rejected.
//! [`Operand`] is the one place the stratification is joined, and it exists
//! only because the specification's own signatures are polymorphic there —
//! `Mention : A -> Act<Mentioning>`, `Denotes : SignToken<K> × A -> Content`.

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer as _;

use super::apply::FunctionSignature;
use super::binder::{Bind, BinderSet, Category, Lambda, Let, LetRec};
use super::content::{Content, Query};
use super::document::ScopeFacts;
use super::error::KernelTypeError;
use super::intrinsic::{Intrinsic, atom, kernel_accepts, property_of, referents};
use super::performable::{Act, Discourse, TranscriptEntry};
use super::predicate::PredTerm;
use super::types::{
    AnswerExhaustivity, AnswerPolarity, EndpointInclusion, Force, LabelLevel, LexicalScopePolicy,
    Proximity, RegisteredName, ScalarKind, ScaleName, SignKind, TextLiteral, TypeAtom, TypeExpr,
    Variable,
};

/// A closed literal value.
///
/// The closed literal families of section 14.1 are values, not atoms the
/// renderer may mint: a literal is valid only in a signature position naming
/// its family, and its family is recoverable from this enum alone.
#[invariant(::Integer(_) => true)]
#[invariant(::Rational { numerator, denominator } => *denominator > BigUint::from(0u8)
    && numerator.magnitude().gcd(denominator) == BigUint::from(1u8),
    "exact rationals are in lowest terms with a positive denominator")]
#[invariant(::Text(_) => true)]
#[invariant(::Force(_) => true)]
#[invariant(::SignKind(_) => true)]
#[invariant(::AnswerPolarity(_) => true)]
#[invariant(::AnswerExhaustivity(_) => true)]
#[invariant(::ScalarKind(_) => true)]
#[invariant(::LabelLevel(_) => true)]
#[invariant(::EndpointInclusion(_) => true)]
#[invariant(::Proximity(_) => true)]
#[invariant(::LexicalScopePolicy(_) => true)]
#[invariant(::Scale(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Integer(BigInt),
    Rational {
        numerator: BigInt,
        denominator: BigUint,
    },
    Text(TextLiteral),
    Force(Force),
    SignKind(SignKind),
    AnswerPolarity(AnswerPolarity),
    AnswerExhaustivity(AnswerExhaustivity),
    ScalarKind(ScalarKind),
    LabelLevel(LabelLevel),
    EndpointInclusion(EndpointInclusion),
    Proximity(Proximity),
    LexicalScopePolicy(LexicalScopePolicy),
    Scale(ScaleName),
}

impl Literal {
    /// Construct an exact integer literal.
    #[requires(true)]
    #[ensures(true)]
    pub fn integer(value: impl Into<BigInt>) -> Self {
        new!(Literal::Integer(value.into()))
    }

    /// Construct an exact rational in lowest terms.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn rational(numerator: BigInt, denominator: BigUint) -> Result<Self, KernelTypeError> {
        if denominator == BigUint::from(0u8) {
            return Err(KernelTypeError::new("a rational denominator is positive"));
        }
        if numerator.magnitude().gcd(&denominator) != BigUint::from(1u8) {
            return Err(KernelTypeError::new(
                "a rational literal is in lowest terms",
            ));
        }
        Ok(new!(Literal::Rational {
            numerator,
            denominator,
        }))
    }

    /// Construct an NFC text literal.
    #[requires(true)]
    #[ensures(true)]
    pub fn text(text: impl AsRef<str>) -> Self {
        new!(Literal::Text(TextLiteral::new(text)))
    }

    /// Return the literal's principal type.
    ///
    /// Section 3.1 gives nonnegative integers principal type `Natural`;
    /// negative integers and rationals are `Number`.
    #[requires(true)]
    #[ensures(true)]
    pub fn value_type(&self) -> TypeExpr {
        match self.as_data() {
            data!(Literal::Integer(value)) => atom(if *value >= BigInt::from(0) {
                TypeAtom::Natural
            } else {
                TypeAtom::Number
            }),
            data!(Literal::Rational { .. }) => atom(TypeAtom::Number),
            data!(Literal::Text(_)) => atom(TypeAtom::Text),
            data!(Literal::Force(_)) => atom(TypeAtom::Force),
            data!(Literal::SignKind(_)) => atom(TypeAtom::SignKind),
            data!(Literal::AnswerPolarity(_)) => atom(TypeAtom::AnswerPolarity),
            data!(Literal::AnswerExhaustivity(_)) => atom(TypeAtom::AnswerExhaustivity),
            data!(Literal::ScalarKind(_)) => atom(TypeAtom::ScalarKind),
            data!(Literal::LabelLevel(_)) => atom(TypeAtom::LabelLevel),
            data!(Literal::EndpointInclusion(_)) => atom(TypeAtom::EndpointInclusion),
            data!(Literal::Proximity(_)) => atom(TypeAtom::Proximity),
            data!(Literal::LexicalScopePolicy(_)) => atom(TypeAtom::LexicalScopePolicy),
            data!(Literal::Scale(_)) => atom(TypeAtom::Scale),
        }
    }
}

/// The two version-0 homogeneous collection constructors.
#[invariant(
    true,
    "both members name a legal container; element typing is a Value invariant"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CollectionKind {
    Set,
    List,
}

impl CollectionKind {
    /// Return the canonical constructor spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Set => "Set",
            Self::List => "List",
        }
    }

    /// Build the collection type over an element type.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::Set(_) | TypeExpr::List(_)))]
    pub fn collection_type(self, element: TypeExpr) -> TypeExpr {
        match self {
            Self::Set => TypeExpr::Set(Box::new(element)),
            Self::List => TypeExpr::List(Box::new(element)),
        }
    }
}

/// A first-order kernel value.
#[invariant(::Literal(_) => true)]
#[invariant(::Collection { element_type, items, .. } => items.iter()
    .all(|item| kernel_accepts(&item.value_type(), element_type)),
    "every collection item inhabits the declared element type")]
#[invariant(::Tuple(elements) => !elements.is_empty(), "a tuple is a nonempty ordered product")]
#[invariant(::Sign { facts, .. } => !facts.is_empty(),
    "a sign-token binder carries at least one analyzer fact")]
#[invariant(::Intrinsic { intrinsic, arguments, value_type } =>
    intrinsic.instantiate(&operand_types(arguments)).as_ref() == Ok(value_type),
    "the stored type is exactly the registered signature's instantiation")]
#[invariant(::Apply { head, arguments, value_type } => head.signature()
    .is_some_and(|signature| kernel_checked_apply(&signature, &operand_types(arguments))
        .as_ref() == Ok(value_type)),
    "an application's stored type is its callee's result type under the strict operand rule")]
#[invariant(::Let(_) => true)]
#[invariant(::Bind(_) => true)]
#[invariant(::LetRec(_) => true)]
#[invariant(::Bound { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Literal(Literal),
    Collection {
        kind: CollectionKind,
        element_type: TypeExpr,
        items: Vec<Value>,
    },
    Tuple(Vec<Value>),
    Sign {
        token: Variable,
        kind: SignKind,
        facts: Vec<Content>,
    },
    Intrinsic {
        intrinsic: Intrinsic,
        arguments: Vec<Operand>,
        value_type: TypeExpr,
    },
    Apply {
        head: FnValue,
        arguments: Vec<Operand>,
        value_type: TypeExpr,
    },
    Let(Let<Value>),
    Bind(Bind<Value>),
    LetRec(LetRec<Value>),
    Bound {
        variable: Variable,
        value_type: TypeExpr,
    },
}

impl Value {
    /// Construct a literal value.
    #[requires(true)]
    #[ensures(true)]
    pub fn literal(literal: Literal) -> Self {
        new!(Value::Literal(literal))
    }

    /// Construct a homogeneous, possibly empty typed collection.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn collection(
        kind: CollectionKind,
        element_type: TypeExpr,
        items: Vec<Value>,
    ) -> Result<Self, KernelTypeError> {
        if !items
            .iter()
            .all(|item| kernel_accepts(&item.value_type(), &element_type))
        {
            return Err(KernelTypeError::new(
                "a collection item does not inhabit the declared element type",
            ));
        }
        Ok(new!(Value::Collection {
            kind,
            element_type,
            items,
        }))
    }

    /// Construct a nonempty heterogeneous ordered product.
    #[requires(true)]
    #[ensures(ret.is_ok() == !old(elements.is_empty()))]
    pub fn tuple(elements: Vec<Value>) -> Result<Self, KernelTypeError> {
        if elements.is_empty() {
            return Err(KernelTypeError::new("a tuple is nonempty"));
        }
        Ok(new!(Value::Tuple(elements)))
    }

    /// Construct a sign-token binder with its analyzer facts.
    #[requires(true)]
    #[ensures(ret.is_ok() == !old(facts.is_empty()))]
    pub fn sign(
        token: Variable,
        kind: SignKind,
        facts: Vec<Content>,
    ) -> Result<Self, KernelTypeError> {
        if facts.is_empty() {
            return Err(KernelTypeError::new(
                "a Sign binder carries at least one analyzer fact",
            ));
        }
        Ok(new!(Value::Sign { token, kind, facts }))
    }

    /// Apply a registered callable whose result is a first-order value.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn intrinsic(
        intrinsic: Intrinsic,
        arguments: Vec<Operand>,
    ) -> Result<Self, KernelTypeError> {
        let value_type = intrinsic.instantiate(&operand_types(&arguments))?;
        Ok(new!(Value::Intrinsic {
            intrinsic,
            arguments,
            value_type,
        }))
    }

    /// Apply a callable value whose result is a first-order value.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn apply(head: FnValue, arguments: Vec<Operand>) -> Result<Self, KernelTypeError> {
        let signature = head
            .signature()
            .ok_or_else(|| KernelTypeError::new("only a callable value can be applied"))?;
        let value_type = kernel_checked_apply(&signature, &operand_types(&arguments))?;
        Ok(new!(Value::Apply {
            head,
            arguments,
            value_type,
        }))
    }

    /// Bind a block of inert declarations over a value body.
    #[requires(true)]
    #[ensures(true)]
    pub fn let_form(form: Let<Value>) -> Self {
        new!(Value::Let(form))
    }

    /// Run a reference computation over a value body.
    #[requires(true)]
    #[ensures(true)]
    pub fn bind_form(form: Bind<Value>) -> Self {
        new!(Value::Bind(form))
    }

    /// Tie a recursive function group over a value body.
    #[requires(true)]
    #[ensures(true)]
    pub fn let_rec_form(form: LetRec<Value>) -> Self {
        new!(Value::LetRec(form))
    }

    /// Reference a bound first-order value.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable, value_type: TypeExpr) -> Self {
        new!(Value::Bound {
            variable,
            value_type,
        })
    }
}

impl Value {
    /// Record this value's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(Value::Literal(_)) => {}
            data!(Value::Collection { items, .. }) => {
                for item in items {
                    item.collect_scope_facts(facts);
                }
            }
            data!(Value::Tuple(elements)) => {
                for element in elements {
                    element.collect_scope_facts(facts);
                }
            }
            data!(Value::Sign {
                token,
                kind,
                facts: items
            }) => {
                facts.record_introduction(token, TypeExpr::SignToken(*kind));
                for item in items {
                    item.collect_scope_facts(facts);
                }
            }
            data!(Value::Intrinsic { arguments, .. }) => {
                for argument in arguments {
                    argument.collect_scope_facts(facts);
                }
            }
            data!(Value::Apply {
                head,
                arguments,
                ..
            }) => {
                head.collect_scope_facts(facts);
                for argument in arguments {
                    argument.collect_scope_facts(facts);
                }
            }
            data!(Value::Let(form)) => facts.record_let(form),
            data!(Value::Bind(form)) => facts.record_bind(form),
            data!(Value::LetRec(form)) => facts.record_let_rec(form),
            data!(Value::Bound {
                variable,
                value_type
            }) => {
                facts.record_use(variable, value_type.clone());
            }
        }
    }
}

#[contract_trait]
impl Category for Value {
    fn value_type(&self) -> TypeExpr {
        match self.as_data() {
            data!(Value::Literal(literal)) => literal.value_type(),
            data!(Value::Collection {
                kind,
                element_type,
                ..
            }) => kind.collection_type(element_type.clone()),
            data!(Value::Tuple(elements)) => {
                TypeExpr::Tuple(elements.iter().map(Category::value_type).collect())
            }
            data!(Value::Sign { kind, .. }) => TypeExpr::SignToken(*kind),
            data!(Value::Intrinsic { value_type, .. })
            | data!(Value::Apply { value_type, .. })
            | data!(Value::Bound { value_type, .. }) => value_type.clone(),
            data!(Value::Let(form)) => form.body().value_type(),
            data!(Value::Bind(form)) => form.body().value_type(),
            data!(Value::LetRec(form)) => form.body().value_type(),
        }
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(Value::Literal(_)) => {}
            data!(Value::Collection { items, .. }) => {
                for item in items {
                    item.append_free_binders_to(binders);
                }
            }
            data!(Value::Tuple(elements)) => {
                for element in elements {
                    element.append_free_binders_to(binders);
                }
            }
            data!(Value::Sign { token, facts, .. }) => {
                let mut inner = BinderSet::new();
                for fact in facts {
                    fact.append_free_binders_to(&mut inner);
                }
                inner.remove(token);
                binders.extend(inner);
            }
            data!(Value::Intrinsic { arguments, .. }) => {
                for argument in arguments {
                    argument.append_free_binders_to(binders);
                }
            }
            data!(Value::Apply {
                head,
                arguments,
                ..
            }) => {
                head.append_free_binders_to(binders);
                for argument in arguments {
                    argument.append_free_binders_to(binders);
                }
            }
            data!(Value::Let(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(Value::Bind(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(Value::LetRec(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(Value::Bound { variable, .. }) => {
                binders.insert(variable.clone());
            }
        }
    }
}

/// A callable kernel value.
#[invariant(::Lambda(_) => true)]
#[invariant(::Intrinsic { intrinsic, arguments, value_type } =>
    intrinsic.instantiate(&operand_types(arguments)).as_ref() == Ok(value_type)
        && function_signature_of(value_type).is_some(),
    "a partially applied registered callable still denotes a function")]
#[invariant(::Registered { .. } => true)]
#[invariant(::Bound { .. } => true)]
#[invariant(::Let(_) => true)]
#[invariant(::Bind(_) => true)]
#[invariant(::LetRec(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnValue {
    Lambda(Lambda<Operand>),
    Intrinsic {
        intrinsic: Intrinsic,
        arguments: Vec<Operand>,
        value_type: TypeExpr,
    },
    /// A relation from a versioned generated table, carrying the signature the
    /// registry declared for it. The kernel cannot close those tables — they
    /// live above it — but it can refuse to apply one without a declared row.
    Registered {
        name: RegisteredName,
        signature: FunctionSignature,
    },
    Bound {
        variable: Variable,
        signature: FunctionSignature,
    },
    Let(Let<FnValue>),
    Bind(Bind<FnValue>),
    LetRec(LetRec<FnValue>),
}

impl FnValue {
    /// Introduce a function by lambda abstraction.
    #[requires(true)]
    #[ensures(ret.is_lambda())]
    pub fn lambda(lambda: Lambda<Operand>) -> Self {
        new!(FnValue::Lambda(lambda))
    }

    /// Partially apply a registered callable to obtain a function value.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn intrinsic(
        intrinsic: Intrinsic,
        arguments: Vec<Operand>,
    ) -> Result<Self, KernelTypeError> {
        let value_type = intrinsic.instantiate(&operand_types(&arguments))?;
        if function_signature_of(&value_type).is_none() {
            return Err(KernelTypeError::new(
                "this registered callable does not return a function value",
            ));
        }
        Ok(new!(FnValue::Intrinsic {
            intrinsic,
            arguments,
            value_type,
        }))
    }

    /// Name a relation from a versioned generated table.
    #[requires(true)]
    #[ensures(true)]
    pub fn registered(name: RegisteredName, signature: FunctionSignature) -> Self {
        new!(FnValue::Registered { name, signature })
    }

    /// Reference a bound callable.
    #[requires(true)]
    #[ensures(true)]
    pub fn bound(variable: Variable, signature: FunctionSignature) -> Self {
        new!(FnValue::Bound {
            variable,
            signature,
        })
    }

    /// Report whether this callable is an inert lambda.
    ///
    /// `LetRec` initializers must be inert lambdas (section 2.2), and this is
    /// the predicate its declaration invariant states.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_lambda(&self) -> bool {
        matches!(self.as_data(), data!(FnValue::Lambda(_)))
    }

    /// Return the ordered signature this callable applies with.
    #[requires(true)]
    #[ensures(true)]
    pub fn signature(&self) -> Option<FunctionSignature> {
        match self.as_data() {
            data!(FnValue::Registered { signature, .. })
            | data!(FnValue::Bound { signature, .. }) => Some(signature.clone()),
            _ => function_signature_of(&self.value_type()),
        }
    }
}

impl FnValue {
    /// Record this callable's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(FnValue::Lambda(lambda)) => facts.record_lambda(lambda),
            data!(FnValue::Intrinsic { arguments, .. }) => {
                for argument in arguments {
                    argument.collect_scope_facts(facts);
                }
            }
            data!(FnValue::Registered { .. }) => {}
            data!(FnValue::Bound {
                variable,
                signature,
            }) => facts.record_use(variable, signature.function_type()),
            data!(FnValue::Let(form)) => facts.record_let(form),
            data!(FnValue::Bind(form)) => facts.record_bind(form),
            data!(FnValue::LetRec(form)) => facts.record_let_rec(form),
        }
    }
}

#[contract_trait]
impl Category for FnValue {
    fn value_type(&self) -> TypeExpr {
        match self.as_data() {
            data!(FnValue::Lambda(lambda)) => lambda.function_type(),
            data!(FnValue::Intrinsic { value_type, .. }) => value_type.clone(),
            data!(FnValue::Registered { signature, .. })
            | data!(FnValue::Bound { signature, .. }) => signature.function_type(),
            data!(FnValue::Let(form)) => form.body().value_type(),
            data!(FnValue::Bind(form)) => form.body().value_type(),
            data!(FnValue::LetRec(form)) => form.body().value_type(),
        }
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(FnValue::Lambda(lambda)) => {
                binders.extend(lambda.free_binders().iter().cloned());
            }
            data!(FnValue::Intrinsic { arguments, .. }) => {
                for argument in arguments {
                    argument.append_free_binders_to(binders);
                }
            }
            data!(FnValue::Registered { .. }) => {}
            data!(FnValue::Bound { variable, .. }) => {
                binders.insert(variable.clone());
            }
            data!(FnValue::Let(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(FnValue::Bind(form)) => binders.extend(form.free_binders().iter().cloned()),
            data!(FnValue::LetRec(form)) => binders.extend(form.free_binders().iter().cloned()),
        }
    }
}

/// A dynamic reference computation.
///
/// `RefComp` is its own category, not a [`Value`]. That is what makes a bare
/// `Refer` in ordinary operand position unrepresentable: the only field in the
/// whole kernel typed `RefComp` is a `Bind` initializer.
#[invariant(::Refer { property } => reference_property_result(property).is_some(),
    "Refer takes a property of one number-neutral reference")]
#[invariant(::Typical { property } => reference_property_result(property).is_some(),
    "Typical takes a property of one number-neutral reference")]
#[invariant(::Stereotypical { describer, property } =>
    kernel_accepts(&describer.value_type(), &referents(atom(TypeAtom::Entity)))
        && reference_property_result(property).is_some(),
    "Stereotypical names its describer explicitly rather than assuming the speaker")]
#[invariant(::Context { result_type, .. } => matches!(result_type, TypeExpr::Referents(_)),
    "a contextual computation introduces a number-neutral reference")]
#[invariant(::Witnesses { result_type, .. } => matches!(result_type, TypeExpr::Referents(_)),
    "a witness selection introduces a number-neutral reference")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefComp {
    Refer {
        property: Lambda<Content>,
    },
    Typical {
        property: Lambda<Content>,
    },
    Stereotypical {
        describer: Value,
        property: Lambda<Content>,
    },
    Context {
        dependencies: Vec<Variable>,
        result_type: TypeExpr,
    },
    Witnesses {
        run: Variable,
        result_type: TypeExpr,
    },
}

impl RefComp {
    /// Obtain a number-neutral reference from a description property.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn refer(property: Lambda<Content>) -> Result<Self, KernelTypeError> {
        require_reference_property(&property)?;
        Ok(new!(RefComp::Refer { property }))
    }

    /// Obtain the typical reference of a description property.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn typical(property: Lambda<Content>) -> Result<Self, KernelTypeError> {
        require_reference_property(&property)?;
        Ok(new!(RefComp::Typical { property }))
    }

    /// Obtain a describer's stereotypical reference.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn stereotypical(
        describer: Value,
        property: Lambda<Content>,
    ) -> Result<Self, KernelTypeError> {
        require_reference_property(&property)?;
        if !kernel_accepts(&describer.value_type(), &referents(atom(TypeAtom::Entity))) {
            return Err(KernelTypeError::new(
                "a stereotypical describer is a reference to entities",
            ));
        }
        Ok(new!(RefComp::Stereotypical {
            describer,
            property,
        }))
    }

    /// Record a fixed or dependency-limited contextual computation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn context(
        dependencies: Vec<Variable>,
        result_type: TypeExpr,
    ) -> Result<Self, KernelTypeError> {
        if !matches!(result_type, TypeExpr::Referents(_)) {
            return Err(KernelTypeError::new(
                "a Context computation selects a number-neutral reference",
            ));
        }
        Ok(new!(RefComp::Context {
            dependencies,
            result_type,
        }))
    }

    /// Retrieve the exported selection of one successful quantifier run.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    pub fn witnesses(run: Variable, result_type: TypeExpr) -> Result<Self, KernelTypeError> {
        if !matches!(result_type, TypeExpr::Referents(_)) {
            return Err(KernelTypeError::new(
                "a witness selection is a number-neutral reference",
            ));
        }
        Ok(new!(RefComp::Witnesses { run, result_type }))
    }

    /// Return the reference type this computation introduces.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::Referents(_)))]
    pub fn result_type(&self) -> TypeExpr {
        match self.as_data() {
            data!(RefComp::Refer { property })
            | data!(RefComp::Typical { property })
            | data!(RefComp::Stereotypical { property, .. }) => {
                reference_property_result(property).expect("validated at construction")
            }
            data!(RefComp::Context { result_type, .. })
            | data!(RefComp::Witnesses { result_type, .. }) => result_type.clone(),
        }
    }

    /// Return the `RefComp<T>` type of this computation.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::ReferenceComputation(_)))]
    pub fn computation_type(&self) -> TypeExpr {
        TypeExpr::ReferenceComputation(Box::new(self.result_type()))
    }

    /// Add every freely used binder name to `binders`.
    #[requires(true)]
    #[ensures(true)]
    pub fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self.as_data() {
            data!(RefComp::Refer { property }) | data!(RefComp::Typical { property }) => {
                binders.extend(property.free_binders().iter().cloned());
            }
            data!(RefComp::Stereotypical {
                describer,
                property,
            }) => {
                describer.append_free_binders_to(binders);
                binders.extend(property.free_binders().iter().cloned());
            }
            data!(RefComp::Context { dependencies, .. }) => {
                binders.extend(dependencies.iter().cloned());
            }
            data!(RefComp::Witnesses { run, .. }) => {
                binders.insert(run.clone());
            }
        }
    }

    /// Record this computation's binder introductions and uses.
    ///
    /// A `Witnesses` run handle is used at the one type section 9.4 gives it —
    /// the `Content` identity of the quantifier application — so the audit can
    /// check it against its binder. A `Context` dependency list stores no type
    /// at all, so it records no use; `append_free_binders_to` still reports
    /// those names, which is what makes the document's free-binder rule reject
    /// an unbound dependency.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self.as_data() {
            data!(RefComp::Refer { property }) | data!(RefComp::Typical { property }) => {
                facts.record_lambda(property);
            }
            data!(RefComp::Stereotypical {
                describer,
                property,
            }) => {
                describer.collect_scope_facts(facts);
                facts.record_lambda(property);
            }
            data!(RefComp::Witnesses { run, .. }) => {
                facts.record_use(run, atom(TypeAtom::Content));
            }
            data!(RefComp::Context { .. }) => {}
        }
    }
}

/// Any first-class kernel value, used only where a registered signature is
/// genuinely polymorphic across categories.
#[invariant(::Value(_) => true)]
#[invariant(::Content(_) => true)]
#[invariant(::Predicate(_) => true)]
#[invariant(::Function(_) => true)]
#[invariant(::Query(_) => true)]
#[invariant(::Act(_) => true)]
#[invariant(::Discourse(_) => true)]
#[invariant(::Entry(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Value(Value),
    Content(Content),
    Predicate(PredTerm),
    Function(FnValue),
    Query(Query),
    Act(Act),
    Discourse(Discourse),
    Entry(TranscriptEntry),
}

impl Operand {
    /// Record this operand's binder introductions and uses.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn collect_scope_facts(&self, facts: &mut ScopeFacts) {
        match self {
            Self::Value(value) => value.collect_scope_facts(facts),
            Self::Content(content) => content.collect_scope_facts(facts),
            Self::Predicate(predicate) => predicate.collect_scope_facts(facts),
            Self::Function(function) => function.collect_scope_facts(facts),
            Self::Query(query) => query.collect_scope_facts(facts),
            Self::Act(act) => act.collect_scope_facts(facts),
            Self::Discourse(discourse) => discourse.collect_scope_facts(facts),
            Self::Entry(entry) => entry.collect_scope_facts(facts),
        }
    }
}

#[contract_trait]
impl Category for Operand {
    fn value_type(&self) -> TypeExpr {
        match self {
            Self::Value(value) => value.value_type(),
            Self::Content(content) => content.value_type(),
            Self::Predicate(predicate) => predicate.value_type(),
            Self::Function(function) => function.value_type(),
            Self::Query(query) => query.value_type(),
            Self::Act(act) => act.value_type(),
            Self::Discourse(discourse) => discourse.value_type(),
            Self::Entry(entry) => entry.value_type(),
        }
    }

    fn append_free_binders_to(&self, binders: &mut BinderSet) {
        match self {
            Self::Value(value) => value.append_free_binders_to(binders),
            Self::Content(content) => content.append_free_binders_to(binders),
            Self::Predicate(predicate) => predicate.append_free_binders_to(binders),
            Self::Function(function) => function.append_free_binders_to(binders),
            Self::Query(query) => query.append_free_binders_to(binders),
            Self::Act(act) => act.append_free_binders_to(binders),
            Self::Discourse(discourse) => discourse.append_free_binders_to(binders),
            Self::Entry(entry) => entry.append_free_binders_to(binders),
        }
    }
}

/// Project the ordered operand types of an application.
#[requires(true)]
#[ensures(ret.len() == arguments.len())]
pub(super) fn operand_types(arguments: &[Operand]) -> Vec<TypeExpr> {
    arguments.iter().map(Category::value_type).collect()
}

/// Apply an ordered function signature under the strict kernel operand rule.
///
/// [`FunctionSignature::apply`] is the notation-era application kernel and still
/// admits the implicit conversions of section 3.1, including the singleton lift.
/// The kernel keeps that crossing explicit, so every kernel application node
/// checks its operands here first and delegates only the result computation to
/// the unchanged signature kernel. Predicate fills are the one deliberate
/// exception: they run through `apply_predicate`, which keeps the lift until
/// route migration can re-measure the fill path against a strict rule.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(super) fn kernel_checked_apply(
    signature: &FunctionSignature,
    arguments: &[TypeExpr],
) -> Result<TypeExpr, KernelTypeError> {
    if arguments.len() != signature.parameters().len() {
        return Err(KernelTypeError::new("function application arity mismatch"));
    }
    for (actual, declared) in arguments.iter().zip(signature.parameters()) {
        if !kernel_accepts(actual, declared) {
            return Err(KernelTypeError::new(
                "a function argument does not inhabit its declared parameter type",
            ));
        }
    }
    Ok(signature.apply(arguments)?.clone())
}

/// Read a callable type as an ordered function signature.
///
/// `GQ<T>` is a distinct type constructor whose meaning is exactly
/// `Fn<(Property<T>), Content>` (section 3.2), so applying a generalized
/// quantifier uses the ordinary function rule instead of a second application
/// form.
#[requires(true)]
#[ensures(true)]
pub(super) fn function_signature_of(value: &TypeExpr) -> Option<FunctionSignature> {
    match value {
        TypeExpr::Function { parameters, result } => Some(FunctionSignature::new(
            parameters.clone(),
            result.as_ref().clone(),
        )),
        TypeExpr::GeneralizedQuantifier(element) => Some(FunctionSignature::new(
            vec![property_of(element.as_ref().clone())],
            atom(TypeAtom::Content),
        )),
        _ => None,
    }
}

/// Return the reference type a description property selects.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| matches!(value, TypeExpr::Referents(_))))]
pub(super) fn reference_property_result(property: &Lambda<Content>) -> Option<TypeExpr> {
    let [parameter] = property.parameters() else {
        return None;
    };
    matches!(parameter.declared_type(), TypeExpr::Referents(_))
        .then(|| parameter.declared_type().clone())
}

/// Require a one-parameter property over a number-neutral reference.
#[requires(true)]
#[ensures(ret.is_ok() == reference_property_result(property).is_some())]
fn require_reference_property(property: &Lambda<Content>) -> Result<(), KernelTypeError> {
    if reference_property_result(property).is_none() {
        return Err(KernelTypeError::new(
            "a reference computation takes one property of a number-neutral reference",
        ));
    }
    Ok(())
}
