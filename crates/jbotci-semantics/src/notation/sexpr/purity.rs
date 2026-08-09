//! Projection-side purity refinement for section 3.2 properties.
//!
//! `PureProperty` deliberately does not exist in the printable kernel type
//! language. This module instead computes the context/effect/stability summary
//! from an already placed kernel value. Keeping the API over kernel values
//! makes a later cache carried beside those values an optimization of this
//! judgment, rather than a change to what the judgment means.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};

use super::super::kernel::binder::{Bind, Lambda, Let};
use super::super::kernel::content::{
    AnswerSelection, AnswerSelectionData, Content, ContentData, Query,
};
use super::super::kernel::intrinsic::Intrinsic;
use super::super::kernel::predicate::{PlaceFill, PlaceFillData, PredTerm, PredTermData};
use super::super::kernel::types::Variable;
use super::super::kernel::value::{
    FnValue, FnValueData, Operand, RefComp, RefCompData, Value, ValueData,
};

/// Whether evaluation preserves the incoming dynamic context.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextSummary {
    Identity,
    ChangedOrUnknown,
}

/// Whether evaluation emits any ordered effect.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectSummary {
    Empty,
    PresentOrUnknown,
}

/// Whether one lexical site and argument tuple are stable within a performance.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StabilitySummary {
    Stable,
    UnstableOrUnknown,
}

/// Section 3.2's three independent refinement coordinates.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PuritySummary {
    context: ContextSummary,
    effects: EffectSummary,
    stability: StabilitySummary,
}

impl PuritySummary {
    /// The identity summary of constants, variables, and inert assembly.
    #[requires(true)]
    #[ensures(ret.is_pure())]
    fn pure() -> Self {
        PuritySummary {
            context: ContextSummary::Identity,
            effects: EffectSummary::Empty,
            stability: StabilitySummary::Stable,
        }
    }

    /// A conservative summary for an effect or opaque computation.
    #[requires(true)]
    #[ensures(!ret.is_pure())]
    fn unproven() -> Self {
        PuritySummary {
            context: ContextSummary::ChangedOrUnknown,
            effects: EffectSummary::PresentOrUnknown,
            stability: StabilitySummary::UnstableOrUnknown,
        }
    }

    /// A known ordered effect whose surrounding operands still contribute.
    #[requires(true)]
    #[ensures(!ret.is_pure())]
    fn with_effect(self) -> Self {
        PuritySummary {
            context: ContextSummary::ChangedOrUnknown,
            effects: EffectSummary::PresentOrUnknown,
            stability: StabilitySummary::UnstableOrUnknown,
        }
    }

    /// Compose two evaluations in their actual left-to-right order.
    #[requires(true)]
    #[ensures(ret.is_pure() == (self.is_pure() && next.is_pure()))]
    fn then(self, next: Self) -> Self {
        PuritySummary {
            context: if self.context == ContextSummary::Identity
                && next.context == ContextSummary::Identity
            {
                ContextSummary::Identity
            } else {
                ContextSummary::ChangedOrUnknown
            },
            effects: if self.effects == EffectSummary::Empty && next.effects == EffectSummary::Empty
            {
                EffectSummary::Empty
            } else {
                EffectSummary::PresentOrUnknown
            },
            stability: if self.stability == StabilitySummary::Stable
                && next.stability == StabilitySummary::Stable
            {
                StabilitySummary::Stable
            } else {
                StabilitySummary::UnstableOrUnknown
            },
        }
    }

    /// Test the exact `Γ = Δ`, empty-effect, site-stable refinement.
    #[requires(true)]
    #[ensures(ret == (self.context == ContextSummary::Identity
        && self.effects == EffectSummary::Empty
        && self.stability == StabilitySummary::Stable))]
    pub(super) fn is_pure(self) -> bool {
        self.context == ContextSummary::Identity
            && self.effects == EffectSummary::Empty
            && self.stability == StabilitySummary::Stable
    }
}

/// Identity-preserving bindings available while one candidate is summarized.
///
/// The environment stores kernel operands, not their spellings. That lets a
/// `Let`-bound callable or lambda parameter retain its application summary
/// without duplicating or beta-expanding its syntax.
#[invariant(true)]
#[derive(Debug, Clone, Default)]
struct SummaryEnvironment {
    values: BTreeMap<Variable, Operand>,
}

impl SummaryEnvironment {
    /// Add or replace one lexically scoped identity.
    #[requires(true)]
    #[ensures(ret.values.contains_key(&old(variable.clone())))]
    fn with(&self, variable: Variable, value: Operand) -> Self {
        let mut ret = self.clone();
        ret.values.insert(variable, value);
        ret
    }

    /// Resolve one identity-preserving binding.
    #[requires(true)]
    #[ensures(true)]
    fn get(&self, variable: &Variable) -> Option<&Operand> {
        self.values.get(variable)
    }
}

/// Summarize one candidate property after planner placement.
#[requires(true)]
#[ensures(true)]
pub(super) fn property_summary(property: &FnValue) -> PuritySummary {
    let Some(signature) = property.signature() else {
        return PuritySummary::unproven();
    };
    let arguments = signature
        .parameters()
        .iter()
        .enumerate()
        .map(|(index, declared)| {
            Operand::Value(Value::bound(
                Variable::from_token_and_index("purityArgument", index),
                declared.clone(),
            ))
        })
        .collect::<Vec<_>>();
    summarize_application(property, &arguments, &SummaryEnvironment::default())
}

/// Test the projection-side refinement required by `PureProperty` positions.
#[requires(true)]
#[ensures(ret == property_summary(property).is_pure())]
pub(super) fn is_pure_property(property: &FnValue) -> bool {
    property_summary(property).is_pure()
}

/// Identify the registered operand positions whose ordinary surface function
/// types carry the registry-only purity precondition.
#[requires(true)]
#[ensures(true)]
pub(super) fn intrinsic_requires_pure_property(
    intrinsic: Intrinsic,
    argument_index: usize,
) -> bool {
    match intrinsic {
        Intrinsic::SetOf | Intrinsic::Every => argument_index == 0,
        Intrinsic::Exactly
        | Intrinsic::AtLeast
        | Intrinsic::AtMost
        | Intrinsic::MoreThan
        | Intrinsic::FewerThan => matches!(argument_index, 1 | 2),
        _ => false,
    }
}

/// Summarize an ordinary operand evaluation without invoking a callable value.
#[requires(true)]
#[ensures(true)]
fn summarize_operand(value: &Operand, environment: &SummaryEnvironment) -> PuritySummary {
    match value {
        Operand::Value(value) => summarize_value(value, environment),
        Operand::Content(value) => summarize_content(value, environment),
        Operand::Predicate(value) => summarize_predicate(value, environment),
        Operand::Function(value) => summarize_function_value(value, environment),
        // Query, act, discourse, and transcript-entry operands are inert
        // first-class values here. Performing an act is a Performable operation,
        // not evaluation of an `Act` operand passed to a fact such as Realizes.
        Operand::Query(_) | Operand::Act(_) | Operand::Discourse(_) | Operand::Entry(_) => {
            PuritySummary::pure()
        }
    }
}

/// Summarize a sequence of operands in registered argument order.
#[requires(true)]
#[ensures(true)]
fn summarize_operands(values: &[Operand], environment: &SummaryEnvironment) -> PuritySummary {
    values.iter().fold(PuritySummary::pure(), |summary, value| {
        summary.then(summarize_operand(value, environment))
    })
}

/// Summarize a first-order value.
#[requires(true)]
#[ensures(true)]
fn summarize_value(value: &Value, environment: &SummaryEnvironment) -> PuritySummary {
    match value.as_data() {
        data!(Value::Literal(_)) | data!(Value::Bound { .. }) => PuritySummary::pure(),
        data!(Value::Collection { items, .. }) | data!(Value::Tuple(items)) => {
            items.iter().fold(PuritySummary::pure(), |summary, item| {
                summary.then(summarize_value(item, environment))
            })
        }
        data!(Value::Sign { facts, .. }) => {
            facts.iter().fold(PuritySummary::pure(), |summary, fact| {
                summary.then(summarize_content(fact, environment))
            })
        }
        data!(Value::Intrinsic {
            intrinsic,
            arguments,
            ..
        }) => summarize_operands(arguments, environment).then(summarize_intrinsic_invocation(
            *intrinsic,
            arguments,
            environment,
        )),
        data!(Value::Apply {
            head,
            arguments,
            ..
        }) => summarize_application(head, arguments, environment),
        data!(Value::Let(form)) => summarize_value_let(form, environment),
        data!(Value::Bind(form)) => summarize_value_bind(form, environment),
        data!(Value::LetRec(_)) => PuritySummary::unproven(),
    }
}

/// Summarize content evaluation.
#[requires(true)]
#[ensures(true)]
fn summarize_content(value: &Content, environment: &SummaryEnvironment) -> PuritySummary {
    match value.as_data() {
        data!(Content::Close(predicate)) => summarize_predicate(predicate, environment),
        data!(Content::Not(inner)) => summarize_content(inner, environment),
        data!(Content::Junction { operands, .. }) => operands
            .iter()
            .fold(PuritySummary::pure(), |summary, operand| {
                summary.then(summarize_content(operand, environment))
            }),
        data!(Content::Binary { left, right, .. }) => {
            summarize_content(left, environment).then(summarize_content(right, environment))
        }
        data!(Content::Quantified { lambda, .. }) => summarize_content_lambda(lambda, environment),
        data!(Content::Presuppose { trigger, body }) => summarize_content(trigger, environment)
            .then(summarize_content(body, environment))
            .with_effect(),
        data!(Content::Supplement { body, side }) => summarize_content(body, environment)
            .then(summarize_content(side, environment))
            .with_effect(),
        data!(Content::Answer { query, selection }) => summarize_query(query, environment)
            .then(summarize_answer_selection(selection, environment)),
        data!(Content::Intrinsic {
            intrinsic,
            arguments,
        }) => summarize_operands(arguments, environment).then(summarize_intrinsic_invocation(
            *intrinsic,
            arguments,
            environment,
        )),
        data!(Content::Apply { head, arguments }) => {
            summarize_application(head, arguments, environment)
        }
        data!(Content::Let(form)) => summarize_content_let(form, environment),
        data!(Content::Bind(form)) => summarize_content_bind(form, environment),
        data!(Content::LetRec(_)) => PuritySummary::unproven(),
        data!(Content::Bound(variable)) => match environment.get(variable) {
            Some(Operand::Content(content)) => summarize_content(content, environment),
            _ => PuritySummary::unproven(),
        },
    }
}

/// Summarize a question when an `Answer` actually evaluates it.
#[requires(true)]
#[ensures(true)]
fn summarize_query(value: &Query, environment: &SummaryEnvironment) -> PuritySummary {
    match value {
        Query::Polar(content) => summarize_content(content, environment),
        Query::Open(lambda) => summarize_content_lambda(lambda, environment),
        Query::Bound { .. } => PuritySummary::unproven(),
    }
}

/// Summarize the values carried by one answer selection.
#[requires(true)]
#[ensures(true)]
fn summarize_answer_selection(
    value: &AnswerSelection,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    match value.as_data() {
        data!(AnswerSelection::Tuple { values, .. }) => {
            values.iter().fold(PuritySummary::pure(), |summary, value| {
                summary.then(summarize_value(value, environment))
            })
        }
        data!(AnswerSelection::Polar(_))
        | data!(AnswerSelection::Contextual(_))
        | data!(AnswerSelection::Unresolved) => PuritySummary::pure(),
    }
}

/// Summarize inert predicate construction and its filled operands.
#[requires(true)]
#[ensures(true)]
fn summarize_predicate(value: &PredTerm, environment: &SummaryEnvironment) -> PuritySummary {
    match value.as_data() {
        data!(PredTerm::Relation(_)) => PuritySummary::pure(),
        data!(PredTerm::Bound { variable, .. }) => match environment.get(variable) {
            Some(Operand::Predicate(predicate)) => summarize_predicate(predicate, environment),
            _ => PuritySummary::unproven(),
        },
        data!(PredTerm::Applied { head, fills, .. }) => fills
            .iter()
            .fold(summarize_predicate(head, environment), |summary, fill| {
                summary.then(summarize_fill(fill, environment))
            }),
        data!(PredTerm::Let(form)) => summarize_predicate_let(form, environment),
        data!(PredTerm::Bind(form)) => summarize_predicate_bind(form, environment),
        data!(PredTerm::LetRec(_)) => PuritySummary::unproven(),
    }
}

/// Summarize one lexical place fill, including a computed place expression.
#[requires(true)]
#[ensures(true)]
fn summarize_fill(value: &PlaceFill, environment: &SummaryEnvironment) -> PuritySummary {
    match value.as_data() {
        data!(PlaceFill::Plain(value))
        | data!(PlaceFill::Numbered { value, .. })
        | data!(PlaceFill::Eventuality(value)) => summarize_operand(value, environment),
        data!(PlaceFill::Computed { place, value, .. }) => {
            summarize_value(place, environment).then(summarize_operand(value, environment))
        }
    }
}

/// Summarize construction of a callable without invoking its body.
#[requires(true)]
#[ensures(true)]
fn summarize_function_value(value: &FnValue, environment: &SummaryEnvironment) -> PuritySummary {
    match value.as_data() {
        data!(FnValue::Lambda(_))
        | data!(FnValue::Registered { .. })
        | data!(FnValue::Bound { .. }) => PuritySummary::pure(),
        data!(FnValue::Intrinsic { arguments, .. }) => summarize_operands(arguments, environment),
        data!(FnValue::Let(form)) => summarize_function_let(form, environment),
        data!(FnValue::Bind(form)) => summarize_function_bind(form, environment),
        data!(FnValue::LetRec(_)) => PuritySummary::unproven(),
    }
}

/// Summarize a call, including callee construction, arguments, and invocation.
#[requires(true)]
#[ensures(true)]
fn summarize_application(
    head: &FnValue,
    arguments: &[Operand],
    environment: &SummaryEnvironment,
) -> PuritySummary {
    summarize_function_value(head, environment)
        .then(summarize_operands(arguments, environment))
        .then(summarize_invocation(head, arguments, environment))
}

/// Instantiate a callable summary without manufacturing any lexical site.
#[requires(true)]
#[ensures(true)]
fn summarize_invocation(
    head: &FnValue,
    arguments: &[Operand],
    environment: &SummaryEnvironment,
) -> PuritySummary {
    match head.as_data() {
        data!(FnValue::Lambda(lambda)) => summarize_operand_lambda(lambda, arguments, environment),
        data!(FnValue::Intrinsic {
            intrinsic,
            arguments: captured,
            ..
        }) => {
            let all_arguments = captured
                .iter()
                .chain(arguments)
                .cloned()
                .collect::<Vec<_>>();
            summarize_intrinsic_invocation(*intrinsic, &all_arguments, environment)
        }
        // Version-0 irreducible generated relations carry the normative inert,
        // identity, site-stable summary required by section 14.2.
        data!(FnValue::Registered { .. }) => PuritySummary::pure(),
        data!(FnValue::Bound { variable, .. }) => match environment.get(variable) {
            Some(Operand::Function(function)) => {
                summarize_invocation(function, arguments, environment)
            }
            _ => PuritySummary::unproven(),
        },
        data!(FnValue::Let(form)) => {
            let nested = environment_for_let(form, environment);
            summarize_invocation(form.body(), arguments, &nested)
        }
        data!(FnValue::Bind(form)) => summarize_invocation(form.body(), arguments, environment),
        data!(FnValue::LetRec(_)) => PuritySummary::unproven(),
    }
}

/// Apply the normative dynamic behavior of one fully instantiated intrinsic.
#[requires(true)]
#[ensures(true)]
fn summarize_intrinsic_invocation(
    intrinsic: Intrinsic,
    arguments: &[Operand],
    environment: &SummaryEnvironment,
) -> PuritySummary {
    let property = |index| {
        arguments
            .get(index)
            .map_or_else(PuritySummary::unproven, |operand| {
                summarize_callable_operand(operand, environment)
            })
    };
    match intrinsic {
        Intrinsic::SetOf => property(0),
        Intrinsic::ZipWith => property(0),
        Intrinsic::Some | Intrinsic::No => property(0).then(property(1)),
        Intrinsic::Every => property(0).then(property(1)).with_effect(),
        Intrinsic::Exactly
        | Intrinsic::AtLeast
        | Intrinsic::AtMost
        | Intrinsic::MoreThan
        | Intrinsic::FewerThan => property(1).then(property(2)),
        Intrinsic::MotionVector => property(2),
        // `InnatelyCapable` passes its host property beneath `Refer`; the
        // property is not evaluated by this transparent call. Every remaining
        // intrinsic has the ordinary summary obtained from its evaluated
        // operands alone.
        _ => PuritySummary::pure(),
    }
}

/// Summarize a callable operand at symbolic arguments of its declared types.
#[requires(true)]
#[ensures(true)]
fn summarize_callable_operand(
    operand: &Operand,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    let Operand::Function(function) = operand else {
        return PuritySummary::unproven();
    };
    let Some(signature) = function.signature() else {
        return PuritySummary::unproven();
    };
    let arguments = signature
        .parameters()
        .iter()
        .enumerate()
        .map(|(index, declared)| {
            Operand::Value(Value::bound(
                Variable::from_token_and_index("purityNestedArgument", index),
                declared.clone(),
            ))
        })
        .collect::<Vec<_>>();
    summarize_application(function, &arguments, environment)
}

/// Apply one operand-valued lambda to already evaluated arguments.
#[requires(true)]
#[ensures(true)]
fn summarize_operand_lambda(
    lambda: &Lambda<Operand>,
    arguments: &[Operand],
    environment: &SummaryEnvironment,
) -> PuritySummary {
    if lambda.parameters().len() != arguments.len() {
        return PuritySummary::unproven();
    }
    let nested = lambda
        .parameters()
        .iter()
        .zip(arguments)
        .fold(environment.clone(), |nested, (parameter, argument)| {
            nested.with(parameter.variable().clone(), argument.clone())
        });
    summarize_operand(lambda.body(), &nested)
}

/// Evaluate one content lambda at stable symbolic arguments.
#[requires(true)]
#[ensures(true)]
fn summarize_content_lambda(
    lambda: &Lambda<Content>,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    let nested = lambda.parameters().iter().enumerate().fold(
        environment.clone(),
        |nested, (index, parameter)| {
            nested.with(
                parameter.variable().clone(),
                Operand::Value(Value::bound(
                    Variable::from_token_and_index("purityQuantifiedArgument", index),
                    parameter.declared_type().clone(),
                )),
            )
        },
    );
    summarize_content(lambda.body(), &nested)
}

/// Extend a summary environment with one sequential `Let` block.
#[requires(true)]
#[ensures(form.declarations().iter().all(|declaration| ret.values.contains_key(declaration.variable())))]
fn environment_for_let<C: super::super::kernel::binder::Category>(
    form: &Let<C>,
    environment: &SummaryEnvironment,
) -> SummaryEnvironment {
    form.declarations()
        .iter()
        .fold(environment.clone(), |nested, declaration| {
            nested.with(
                declaration.variable().clone(),
                declaration.initializer().clone(),
            )
        })
}

/// Summarize a sequential declaration block once, in declaration order.
#[requires(true)]
#[ensures(true)]
fn summarize_declarations<C: super::super::kernel::binder::Category>(
    form: &Let<C>,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    let mut nested = environment.clone();
    let mut summary = PuritySummary::pure();
    for declaration in form.declarations() {
        summary = summary.then(summarize_operand(declaration.initializer(), &nested));
        nested = nested.with(
            declaration.variable().clone(),
            declaration.initializer().clone(),
        );
    }
    summary
}

/// Summarize a `Let` whose body is a first-order value.
#[requires(true)]
#[ensures(true)]
fn summarize_value_let(form: &Let<Value>, environment: &SummaryEnvironment) -> PuritySummary {
    let nested = environment_for_let(form, environment);
    summarize_declarations(form, environment).then(summarize_value(form.body(), &nested))
}

/// Summarize a `Let` whose body is content.
#[requires(true)]
#[ensures(true)]
fn summarize_content_let(form: &Let<Content>, environment: &SummaryEnvironment) -> PuritySummary {
    let nested = environment_for_let(form, environment);
    summarize_declarations(form, environment).then(summarize_content(form.body(), &nested))
}

/// Summarize a `Let` whose body is a predicate term.
#[requires(true)]
#[ensures(true)]
fn summarize_predicate_let(
    form: &Let<PredTerm>,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    let nested = environment_for_let(form, environment);
    summarize_declarations(form, environment).then(summarize_predicate(form.body(), &nested))
}

/// Summarize a `Let` whose body is a callable.
#[requires(true)]
#[ensures(true)]
fn summarize_function_let(form: &Let<FnValue>, environment: &SummaryEnvironment) -> PuritySummary {
    let nested = environment_for_let(form, environment);
    summarize_declarations(form, environment).then(summarize_function_value(form.body(), &nested))
}

/// Summarize one `Bind` computation.
#[requires(true)]
#[ensures(ret.is_pure() == matches!(computation.as_data(), data!(RefComp::Context { .. })))]
fn summarize_computation(computation: &RefComp) -> PuritySummary {
    match computation.as_data() {
        // Both fixed and dependency-limited Context are read-only and stable at
        // their already assigned lexical closure site.
        data!(RefComp::Context { .. }) => PuritySummary::pure(),
        data!(RefComp::Refer { .. })
        | data!(RefComp::Typical { .. })
        | data!(RefComp::Stereotypical { .. })
        | data!(RefComp::Witnesses { .. }) => PuritySummary::unproven(),
    }
}

/// Summarize a first-order value `Bind`.
#[requires(true)]
#[ensures(true)]
fn summarize_value_bind(form: &Bind<Value>, environment: &SummaryEnvironment) -> PuritySummary {
    summarize_computation(form.computation()).then(summarize_value(form.body(), environment))
}

/// Summarize a content `Bind`.
#[requires(true)]
#[ensures(true)]
fn summarize_content_bind(form: &Bind<Content>, environment: &SummaryEnvironment) -> PuritySummary {
    summarize_computation(form.computation()).then(summarize_content(form.body(), environment))
}

/// Summarize a predicate-term `Bind`.
#[requires(true)]
#[ensures(true)]
fn summarize_predicate_bind(
    form: &Bind<PredTerm>,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    summarize_computation(form.computation()).then(summarize_predicate(form.body(), environment))
}

/// Summarize a callable `Bind`.
#[requires(true)]
#[ensures(true)]
fn summarize_function_bind(
    form: &Bind<FnValue>,
    environment: &SummaryEnvironment,
) -> PuritySummary {
    summarize_computation(form.computation())
        .then(summarize_function_value(form.body(), environment))
}

#[cfg(test)]
mod tests {
    use super::super::super::kernel::apply::PredicateSignature;
    use super::super::super::kernel::binder::{Category, Declaration, TypedParameter};
    use super::super::super::kernel::predicate::PlaceFill;
    use super::super::super::kernel::types::{
        LexicalRoot, PlaceLabel, RelationRef, Row, RowSlot, TypeAtom, TypeExpr,
    };
    use super::*;

    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::Atom(value) if value == atom))]
    fn atom_type(atom: TypeAtom) -> TypeExpr {
        TypeExpr::Atom(atom)
    }

    #[requires(true)]
    #[ensures(matches!(ret, TypeExpr::Referents(_)))]
    fn referents_type(element: TypeExpr) -> TypeExpr {
        TypeExpr::Referents(Box::new(element))
    }

    #[requires(true)]
    #[ensures(ret.signature().is_some())]
    fn one_place_property(body: Content) -> FnValue {
        let parameter = TypedParameter::new(
            Variable::try_new("$candidate").expect("valid variable"),
            referents_type(atom_type(TypeAtom::Entity)),
        );
        FnValue::lambda(
            Lambda::new(vec![parameter], Operand::Content(body)).expect("typed property"),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn lexical_content() -> Content {
        let entity = referents_type(atom_type(TypeAtom::Entity));
        let term = PredTerm::relation(PredicateSignature::new(
            RelationRef::Lexical(LexicalRoot::try_new("prenu").expect("valid root")),
            Row::new(
                vec![RowSlot::new(PlaceLabel::numbered(1), entity.clone())],
                false,
            ),
        ));
        let term = PredTerm::applied(
            term,
            vec![PlaceFill::plain(Operand::Value(Value::bound(
                Variable::try_new("$candidate").expect("valid variable"),
                entity,
            )))],
        )
        .expect("typed fill");
        Content::close(term).expect("closed row")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lexical_properties_and_dependency_limited_context_compose_purely() {
        let entity = referents_type(atom_type(TypeAtom::Entity));
        let context = RefComp::context(
            vec![Variable::try_new("$candidate").expect("valid variable")],
            entity.clone(),
        )
        .expect("typed Context");
        let body = Content::bind_form(
            Bind::new(
                Variable::try_new("$contextual").expect("valid variable"),
                entity,
                context,
                lexical_content(),
            )
            .expect("typed Bind"),
        );
        assert!(is_pure_property(&one_place_property(body)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn let_bound_property_application_copies_the_callable_summary() {
        let entity = referents_type(atom_type(TypeAtom::Entity));
        let shared = one_place_property(lexical_content());
        let shared_variable = Variable::try_new("$shared").expect("valid variable");
        let declaration = Declaration::new(
            shared_variable.clone(),
            shared.value_type(),
            Operand::Function(shared),
        )
        .expect("typed declaration");
        let called = Content::apply(
            FnValue::bound(
                shared_variable,
                super::super::super::kernel::apply::FunctionSignature::new(
                    vec![entity.clone()],
                    atom_type(TypeAtom::Content),
                ),
            ),
            vec![Operand::Value(Value::bound(
                Variable::try_new("$candidate").expect("valid variable"),
                entity,
            ))],
        )
        .expect("typed application");
        let body = Content::let_form(Let::new(vec![declaration], called).expect("typed Let"));
        assert!(is_pure_property(&one_place_property(body)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_presupposition_supplement_and_every_break_purity() {
        let entity = referents_type(atom_type(TypeAtom::Entity));
        let reference_property = Lambda::new(
            vec![TypedParameter::new(
                Variable::try_new("$introduced").expect("valid variable"),
                entity.clone(),
            )],
            lexical_content(),
        )
        .expect("typed reference property");
        let referred = Content::bind_form(
            Bind::new(
                Variable::try_new("$introduced").expect("valid variable"),
                entity.clone(),
                RefComp::refer(reference_property).expect("typed Refer"),
                lexical_content(),
            )
            .expect("typed Bind"),
        );
        assert!(!is_pure_property(&one_place_property(referred)));
        assert!(!is_pure_property(&one_place_property(Content::presuppose(
            lexical_content(),
            lexical_content(),
        ))));
        assert!(!is_pure_property(&one_place_property(Content::supplement(
            lexical_content(),
            lexical_content(),
        ))));

        let restriction = one_place_property(lexical_content());
        let scope = one_place_property(lexical_content());
        let every = FnValue::intrinsic(Intrinsic::Every, vec![Operand::Function(restriction)])
            .expect("Every returns a GQ");
        let body =
            Content::apply(every, vec![Operand::Function(scope)]).expect("Every accepts a scope");
        assert!(!is_pure_property(&one_place_property(body)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn purity_requirement_table_covers_set_and_quantifier_positions() {
        assert!(intrinsic_requires_pure_property(Intrinsic::SetOf, 0));
        assert!(intrinsic_requires_pure_property(Intrinsic::Every, 0));
        assert!(intrinsic_requires_pure_property(Intrinsic::Exactly, 1));
        assert!(intrinsic_requires_pure_property(Intrinsic::Exactly, 2));
        assert!(!intrinsic_requires_pure_property(Intrinsic::Some, 0));
        assert!(!intrinsic_requires_pure_property(Intrinsic::Exactly, 0));
    }
}
