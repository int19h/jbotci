//! Categories and the four binding forms.
//!
//! Every kernel value belongs to exactly one semantic category, and each
//! category answers the same two questions: what its type is, and which binders
//! it uses freely. [`Category`] states that contract once, which is what lets
//! `Lambda`, `Let`, `Bind`, and `LetRec` be generic over the category of their
//! body instead of being repeated per category.
//!
//! Free-binder sets are computed by the constructors and carried on the binding
//! forms. Host planning needs them per binding site, and recomputing them at
//! every use would make a placement failure hard to attribute to the binder
//! that actually caused it.

use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, expensive_invariant, invariant, new, requires};

use super::error::KernelTypeError;
use super::types::{TypeExpr, Variable};
use super::value::{FnValue, Operand, RefComp};

/// The free binders of a kernel value.
///
/// Binder names are the kernel's identities, so a plain ordered set of names is
/// the whole content of the judgment; the document invariant separately proves
/// that no binder shadows another, which is what makes names identities.
pub type BinderSet = BTreeSet<Variable>;

/// One semantic category of kernel values.
#[contract_trait]
pub trait Category {
    /// Return the kernel type of this value.
    ///
    /// Implementations read a stored annotation or a category constant; they
    /// never re-infer a subtree, because every child validated its own type at
    /// construction.
    #[requires(true)]
    #[ensures(true)]
    fn value_type(&self) -> TypeExpr;

    /// Add every freely used binder name to `binders`.
    #[requires(true)]
    #[ensures(true)]
    fn append_free_binders_to(&self, binders: &mut BinderSet);
}

/// Collect the free binders of one kernel value.
#[requires(true)]
#[ensures(true)]
pub fn free_binders_of<C: Category + ?Sized>(value: &C) -> BinderSet {
    let mut binders = BinderSet::new();
    value.append_free_binders_to(&mut binders);
    binders
}

/// One typed lambda parameter.
#[invariant(
    true,
    "every variable/type pair is a legal parameter; uniqueness is a lambda invariant"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParameter {
    variable: Variable,
    declared_type: TypeExpr,
}

impl TypedParameter {
    /// Declare one typed parameter.
    #[requires(true)]
    #[ensures(ret.variable == old(variable.clone()) && ret.declared_type == old(declared_type.clone()))]
    pub fn new(variable: Variable, declared_type: TypeExpr) -> Self {
        Self {
            variable,
            declared_type,
        }
    }

    /// Borrow the parameter name.
    #[requires(true)]
    #[ensures(ret.as_str().starts_with('$'))]
    pub fn variable(&self) -> &Variable {
        &self.variable
    }

    /// Borrow the declared parameter type.
    #[requires(true)]
    #[ensures(true)]
    pub fn declared_type(&self) -> &TypeExpr {
        &self.declared_type
    }
}

/// A lambda with a nonempty, duplicate-free ordered parameter list.
///
/// The result type is stored rather than inferred on demand: specification
/// section 2.2 infers it from the body once, and every application site then
/// checks against a value instead of walking the body again.
#[invariant(!parameters.is_empty() && parameters_are_unique(parameters), "λ prints a complete ordered typed parameter list")]
#[invariant(*result == body.value_type(), "the lambda result type is the body's type")]
#[expensive_invariant(*free_binders == lambda_free_binders(parameters, body.as_ref()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lambda<C: Category> {
    parameters: Vec<TypedParameter>,
    result: TypeExpr,
    body: Box<C>,
    free_binders: BinderSet,
}

impl<C: Category> Lambda<C> {
    /// Construct a lambda over an already valid body.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|lambda| lambda.parameters.len() == old(parameters.len())) || ret.is_err())]
    pub fn new(parameters: Vec<TypedParameter>, body: C) -> Result<Self, KernelTypeError> {
        if parameters.is_empty() {
            return Err(KernelTypeError::new(
                "a lambda binds at least one parameter",
            ));
        }
        if !parameters_are_unique(&parameters) {
            return Err(KernelTypeError::new(
                "lambda parameter names must be distinct",
            ));
        }
        let result = body.value_type();
        let free_binders = lambda_free_binders(&parameters, &body);
        Ok(new!(Lambda {
            parameters,
            result,
            body: Box::new(body),
            free_binders,
        }))
    }

    /// Borrow the ordered parameters.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn parameters(&self) -> &[TypedParameter] {
        &self.parameters
    }

    /// Borrow the inferred result type.
    #[requires(true)]
    #[ensures(true)]
    pub fn result_type(&self) -> &TypeExpr {
        &self.result
    }

    /// Borrow the body.
    #[requires(true)]
    #[ensures(true)]
    pub fn body(&self) -> &C {
        &self.body
    }

    /// Borrow the carried free-binder set.
    #[requires(true)]
    #[ensures(true)]
    pub fn free_binders(&self) -> &BinderSet {
        &self.free_binders
    }

    /// Return the function type this lambda inhabits.
    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::Function { .. }))]
    pub fn function_type(&self) -> TypeExpr {
        TypeExpr::Function {
            parameters: self
                .parameters
                .iter()
                .map(|parameter| parameter.declared_type.clone())
                .collect(),
            result: Box::new(self.result.clone()),
        }
    }
}

/// One inert `Let` declaration.
///
/// Only `$variable` declarations exist in the kernel. The closed prelude names
/// of specification section 14.1 are the format's implicit prelude and never
/// print as a document binding, so they are modelled as registered callables
/// rather than as declarations a graph could mint.
#[invariant(
    initializer.value_type().implicit_conversion_to(declared_type).is_some(),
    "a declaration's initializer must inhabit its declared type"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    variable: Variable,
    declared_type: TypeExpr,
    initializer: Operand,
}

impl Declaration {
    /// Declare one inert shared value.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|declaration| declaration.variable == old(variable.clone())) || ret.is_err())]
    pub fn new(
        variable: Variable,
        declared_type: TypeExpr,
        initializer: Operand,
    ) -> Result<Self, KernelTypeError> {
        if initializer
            .value_type()
            .implicit_conversion_to(&declared_type)
            .is_none()
        {
            return Err(KernelTypeError::new(
                "Let initializer does not inhabit its declared type",
            ));
        }
        Ok(new!(Declaration {
            variable,
            declared_type,
            initializer,
        }))
    }

    /// Borrow the declared name.
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

    /// Borrow the initializer.
    #[requires(true)]
    #[ensures(true)]
    pub fn initializer(&self) -> &Operand {
        &self.initializer
    }
}

/// A host's whole nonrecursive declaration block.
///
/// The kernel keeps one host's declarations together because that is the
/// placement decision; specification section 2.2 nests one `Let` per
/// declaration, and that nesting is the printer's business. Declarations are
/// sequential, so a later initializer may use an earlier name.
#[invariant(!declarations.is_empty(), "a Let block declares at least one name")]
#[invariant(declarations_are_unique(declarations), "declared names are distinct")]
#[expensive_invariant(*free_binders == let_free_binders(declarations, body.as_ref()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Let<C: Category> {
    declarations: Vec<Declaration>,
    body: Box<C>,
    free_binders: BinderSet,
}

impl<C: Category> Let<C> {
    /// Bind a nonempty block of inert declarations over a body.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|form| form.declarations.len() == old(declarations.len())) || ret.is_err())]
    pub fn new(declarations: Vec<Declaration>, body: C) -> Result<Self, KernelTypeError> {
        if declarations.is_empty() {
            return Err(KernelTypeError::new("a Let declares at least one name"));
        }
        if !declarations_are_unique(&declarations) {
            return Err(KernelTypeError::new("Let declares a name twice"));
        }
        let free_binders = let_free_binders(&declarations, &body);
        Ok(new!(Let {
            declarations,
            body: Box::new(body),
            free_binders,
        }))
    }

    /// Borrow the ordered declarations.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Borrow the body.
    #[requires(true)]
    #[ensures(true)]
    pub fn body(&self) -> &C {
        &self.body
    }

    /// Borrow the carried free-binder set.
    #[requires(true)]
    #[ensures(true)]
    pub fn free_binders(&self) -> &BinderSet {
        &self.free_binders
    }
}

/// The one printed dynamic-value binder.
///
/// `Bind` runs a reference computation and evaluates its body in the updated
/// dynamic context, so its initializer is a [`RefComp`] rather than an inert
/// operand. Making the field's type `RefComp` is what makes a `Refer` in
/// ordinary value position unrepresentable.
#[invariant(
    computation.result_type().implicit_conversion_to(declared_type).is_some(),
    "the computation's result must inhabit the declared binder type"
)]
#[expensive_invariant(*free_binders == bind_free_binders(variable, computation, body.as_ref()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind<C: Category> {
    variable: Variable,
    declared_type: TypeExpr,
    computation: Box<RefComp>,
    body: Box<C>,
    free_binders: BinderSet,
}

impl<C: Category> Bind<C> {
    /// Bind the result of one reference computation over a body.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|form| form.variable == old(variable.clone())) || ret.is_err())]
    pub fn new(
        variable: Variable,
        declared_type: TypeExpr,
        computation: RefComp,
        body: C,
    ) -> Result<Self, KernelTypeError> {
        if computation
            .result_type()
            .implicit_conversion_to(&declared_type)
            .is_none()
        {
            return Err(KernelTypeError::new(
                "Bind computation does not produce its declared binder type",
            ));
        }
        let free_binders = bind_free_binders(&variable, &computation, &body);
        Ok(new!(Bind {
            variable,
            declared_type,
            computation: Box::new(computation),
            body: Box::new(body),
            free_binders,
        }))
    }

    /// Borrow the bound name.
    #[requires(true)]
    #[ensures(ret.as_str().starts_with('$'))]
    pub fn variable(&self) -> &Variable {
        &self.variable
    }

    /// Borrow the declared binder type.
    #[requires(true)]
    #[ensures(true)]
    pub fn declared_type(&self) -> &TypeExpr {
        &self.declared_type
    }

    /// Borrow the reference computation.
    #[requires(true)]
    #[ensures(true)]
    pub fn computation(&self) -> &RefComp {
        &self.computation
    }

    /// Borrow the body.
    #[requires(true)]
    #[ensures(true)]
    pub fn body(&self) -> &C {
        &self.body
    }

    /// Borrow the carried free-binder set.
    #[requires(true)]
    #[ensures(true)]
    pub fn free_binders(&self) -> &BinderSet {
        &self.free_binders
    }
}

/// One member of a recursive binding group.
#[invariant(initializer.is_lambda(), "every LetRec initializer is an inert lambda")]
#[invariant(
    initializer.value_type().implicit_conversion_to(declared_type).is_some(),
    "a recursive initializer must inhabit its declared type"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecursiveDeclaration {
    variable: Variable,
    declared_type: TypeExpr,
    initializer: FnValue,
}

impl RecursiveDeclaration {
    /// Declare one mutually recursive inert function.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|declaration| declaration.variable == old(variable.clone())) || ret.is_err())]
    pub fn new(
        variable: Variable,
        declared_type: TypeExpr,
        initializer: FnValue,
    ) -> Result<Self, KernelTypeError> {
        if !initializer.is_lambda() {
            return Err(KernelTypeError::new(
                "a LetRec initializer must be an inert lambda",
            ));
        }
        if initializer
            .value_type()
            .implicit_conversion_to(&declared_type)
            .is_none()
        {
            return Err(KernelTypeError::new(
                "LetRec initializer does not inhabit its declared type",
            ));
        }
        Ok(new!(RecursiveDeclaration {
            variable,
            declared_type,
            initializer,
        }))
    }

    /// Borrow the declared name.
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

    /// Borrow the recursive initializer.
    #[requires(true)]
    #[ensures(ret.is_lambda())]
    pub fn initializer(&self) -> &FnValue {
        &self.initializer
    }
}

/// The only multi-binding form: one recursive environment of inert lambdas.
#[invariant(!declarations.is_empty(), "a LetRec group is nonempty")]
#[invariant(
    recursive_declarations_are_unique(declarations),
    "declared names are distinct"
)]
#[expensive_invariant(*free_binders == let_rec_free_binders(declarations, body.as_ref()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetRec<C: Category> {
    declarations: Vec<RecursiveDeclaration>,
    body: Box<C>,
    free_binders: BinderSet,
}

impl<C: Category> LetRec<C> {
    /// Tie a nonempty group of mutually visible lambdas over a body.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|form| form.declarations.len() == old(declarations.len())) || ret.is_err())]
    pub fn new(declarations: Vec<RecursiveDeclaration>, body: C) -> Result<Self, KernelTypeError> {
        if declarations.is_empty() {
            return Err(KernelTypeError::new("a LetRec group is nonempty"));
        }
        if !recursive_declarations_are_unique(&declarations) {
            return Err(KernelTypeError::new("LetRec declares a name twice"));
        }
        let free_binders = let_rec_free_binders(&declarations, &body);
        Ok(new!(LetRec {
            declarations,
            body: Box::new(body),
            free_binders,
        }))
    }

    /// Borrow the ordered group members.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn declarations(&self) -> &[RecursiveDeclaration] {
        &self.declarations
    }

    /// Borrow the body.
    #[requires(true)]
    #[ensures(true)]
    pub fn body(&self) -> &C {
        &self.body
    }

    /// Borrow the carried free-binder set.
    #[requires(true)]
    #[ensures(true)]
    pub fn free_binders(&self) -> &BinderSet {
        &self.free_binders
    }
}

/// Compute a lambda's free binders: the body's, less its parameters.
#[requires(true)]
#[ensures(parameters.iter().all(|parameter| !ret.contains(parameter.variable())))]
pub(super) fn lambda_free_binders<C: Category>(
    parameters: &[TypedParameter],
    body: &C,
) -> BinderSet {
    let mut binders = free_binders_of(body);
    for parameter in parameters {
        binders.remove(parameter.variable());
    }
    binders
}

/// Compute a `Let` block's free binders.
///
/// Declarations are sequential, so declaration `i` may use the names declared
/// before it and no later name.
/// A name that survives here is a use of some enclosing binder of the same
/// spelling; the document's no-shadowing invariant is what rejects that, so
/// this function does not silently absorb it.
#[requires(true)]
#[ensures(true)]
pub(super) fn let_free_binders<C: Category>(declarations: &[Declaration], body: &C) -> BinderSet {
    let mut binders = free_binders_of(body);
    for declaration in declarations.iter().rev() {
        binders.remove(declaration.variable());
        declaration
            .initializer()
            .append_free_binders_to(&mut binders);
    }
    binders
}

/// Compute a `Bind`'s free binders. The computation runs outside the binder.
#[requires(true)]
#[ensures(true)]
pub(super) fn bind_free_binders<C: Category>(
    variable: &Variable,
    computation: &RefComp,
    body: &C,
) -> BinderSet {
    let mut binders = free_binders_of(body);
    binders.remove(variable);
    computation.append_free_binders_to(&mut binders);
    binders
}

/// Compute a `LetRec` group's free binders. Every initializer sees every name.
#[requires(true)]
#[ensures(declarations.iter().all(|declaration| !ret.contains(declaration.variable())))]
pub(super) fn let_rec_free_binders<C: Category>(
    declarations: &[RecursiveDeclaration],
    body: &C,
) -> BinderSet {
    let mut binders = free_binders_of(body);
    for declaration in declarations {
        declaration
            .initializer()
            .append_free_binders_to(&mut binders);
    }
    for declaration in declarations {
        binders.remove(declaration.variable());
    }
    binders
}

/// Test distinct lambda parameter names.
#[requires(true)]
#[ensures(true)]
pub(super) fn parameters_are_unique(parameters: &[TypedParameter]) -> bool {
    let mut seen = BTreeSet::new();
    parameters
        .iter()
        .all(|parameter| seen.insert(parameter.variable()))
}

/// Test distinct declared names.
#[requires(true)]
#[ensures(true)]
pub(super) fn declarations_are_unique(declarations: &[Declaration]) -> bool {
    let mut seen = BTreeSet::new();
    declarations
        .iter()
        .all(|declaration| seen.insert(declaration.variable()))
}

/// Test distinct recursively declared names.
#[requires(true)]
#[ensures(true)]
pub(super) fn recursive_declarations_are_unique(declarations: &[RecursiveDeclaration]) -> bool {
    let mut seen = BTreeSet::new();
    declarations
        .iter()
        .all(|declaration| seen.insert(declaration.variable()))
}
