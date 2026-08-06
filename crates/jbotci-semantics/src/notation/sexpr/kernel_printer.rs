//! The version-0 S-expression printer over the typed kernel.
//!
//! The kernel is semantically explicit: an expected-type `Close` and a
//! singleton lift are nodes, not conventions. This printer is where the
//! canonical elisions of specification sections 2, 3, and 5 are applied — it
//! projects a valid kernel document onto surface syntax and decides nothing
//! about meaning. A future brace notation is a second printer over the same
//! value and inherits none of this module's S-expression accidents.
//!
//! `Datum` appears here as private serialization machinery only. It is not in
//! any kernel signature, and it is not in this module's public one either:
//! [`print_kernel_document`] takes a typed document and returns text.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use num_bigint::BigInt;

use super::super::kernel::apply::PredicateSignature;
use super::super::kernel::binder::{Bind, Category, Lambda, Let, LetRec, free_binders_of};
use super::super::kernel::content::{
    AnswerSelection, AnswerSelectionData, Content, ContentData, Query,
};
use super::super::kernel::document::KernelDocument;
use super::super::kernel::intrinsic::Intrinsic;
use super::super::kernel::performable::{
    Act, ActData, Discourse, DiscourseData, Performable, TranscriptEntry, TranscriptEntryData,
};
use super::super::kernel::predicate::{PlaceFill, PlaceFillData, PredTerm, PredTermData};
use super::super::kernel::types::{TypeAtom, TypeExpr};
use super::super::kernel::value::{
    FnValue, FnValueData, Literal, LiteralData, Operand, RefComp, RefCompData, Value, ValueData,
};
use super::datum::{Datum, Integer, print_document};
use super::type_syntax::{relation_ref_to_datum, type_to_datum, variable_to_datum};

/// Print one typed kernel document as canonical version-0 text.
///
/// `words` is the optional reference-data section of section 2.4; it is
/// supplied separately because word cards are not semantic content and the
/// kernel therefore does not carry them.
#[requires(true)]
#[ensures(ret.ends_with('\n') && !ret.ends_with("\n\n"))]
pub fn print_kernel_document(document: &KernelDocument, words: &[Datum]) -> String {
    print_document(&kernel_document_datum(document, words))
}

/// Serialize one typed kernel document as `(Smusni 0 performable [words])`.
#[requires(true)]
#[ensures(ret.form_head() == Some("Smusni"))]
pub fn kernel_document_datum(document: &KernelDocument, words: &[Datum]) -> Datum {
    let mut values = vec![Datum::unsigned(0), performable_datum(document.body())];
    if !words.is_empty() {
        values.push(Datum::form("Words", words.iter().cloned()));
    }
    Datum::form("Smusni", values)
}

/// What the surrounding position requires of the value being printed.
///
/// Both canonical elisions the printer applies are expected-type rules:
/// section 5.2 omits `Close` only at a registered `Content` operand, and
/// section 3.3 omits `Singleton` only at a statically known `Referents<T>`
/// operand. Carrying the expectation is therefore the whole mechanism.
#[invariant(::Unknown => true)]
#[invariant(::Type(_) => true, "any type is a legal expectation at a printing position")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expected<'a> {
    /// No registered operand type is available here, so every crossing prints.
    Unknown,
    /// The position requires this exact type.
    Type(&'a TypeExpr),
}

impl Expected<'_> {
    /// Report whether this position requires `Content`.
    #[requires(true)]
    #[ensures(true)]
    fn requires_content(self) -> bool {
        matches!(self, Self::Type(TypeExpr::Atom(TypeAtom::Content)))
    }

    /// Report whether this position requires a number-neutral reference.
    #[requires(true)]
    #[ensures(true)]
    fn requires_referents(self) -> bool {
        matches!(self, Self::Type(TypeExpr::Referents(_)))
    }

    /// Report whether this position determines a collection's element type.
    #[requires(true)]
    #[ensures(true)]
    fn determines_element(self, element: &TypeExpr) -> bool {
        matches!(
            self,
            Self::Type(TypeExpr::Set(declared) | TypeExpr::List(declared))
                if declared.as_ref() == element
        )
    }
}

/// Serialize a performable body.
#[requires(true)]
#[ensures(true)]
fn performable_datum(value: &Performable) -> Datum {
    match value {
        Performable::Act(act) => act_datum(act),
        Performable::Discourse(discourse) => discourse_datum(discourse),
        // Section 7.2: only on the implicit performance spine may a simple
        // entry contract to the act it realizes.
        Performable::Entry(entry) => {
            contracted_entry(entry).map_or_else(|| entry_datum(entry), |act| act_datum(act))
        }
        Performable::Let(form) => let_datum(form, performable_datum),
        Performable::Bind(form) => bind_datum(form, performable_datum),
        Performable::LetRec(form) => let_rec_datum(form, performable_datum),
    }
}

/// Return the single realized act of a contractible transcript entry.
///
/// Section 2.4 permits the contraction only when the entry's sole
/// identity-dependent fact is one `Realizes` fact and every other fact is
/// omitted, so that the document convention supplies the defaults.
#[requires(true)]
#[ensures(true)]
fn contracted_entry(entry: &TranscriptEntry) -> Option<&Act> {
    let data!(TranscriptEntry::Utterance { token, facts }) = entry.as_data() else {
        return None;
    };
    let [fact] = facts.as_slice() else {
        return None;
    };
    let data!(Content::Intrinsic {
        intrinsic,
        arguments,
    }) = fact.as_data()
    else {
        return None;
    };
    if *intrinsic != Intrinsic::Realizes {
        return None;
    }
    let [Operand::Value(subject), Operand::Act(act)] = arguments.as_slice() else {
        return None;
    };
    let data!(Value::Bound { variable, .. }) = subject.as_data() else {
        return None;
    };
    // The token must not survive anywhere else, or the contraction would drop
    // an identity the document still refers to.
    (variable == token && !free_binders_of(act).contains(token)).then_some(act)
}

/// Serialize an act.
#[requires(true)]
#[ensures(true)]
fn act_datum(value: &Act) -> Datum {
    match value.as_data() {
        data!(Act::Assert(content)) => {
            Datum::form("Assert", [content_datum(content, content_expected())])
        }
        data!(Act::Ask(query)) => Datum::form("Ask", [query_datum(query)]),
        data!(Act::Command { addressee, content }) => Datum::form(
            "Command",
            [
                value_datum(addressee, Expected::Unknown),
                content_datum(content, content_expected()),
            ],
        ),
        data!(Act::Express(content)) => {
            Datum::form("Express", [content_datum(content, content_expected())])
        }
        data!(Act::Mention(operand)) => {
            Datum::form("Mention", [operand_datum(operand, Expected::Unknown)])
        }
        data!(Act::Vocative(addressee)) => {
            Datum::form("Vocative", [value_datum(addressee, Expected::Unknown)])
        }
        data!(Act::Interpret { sign, .. }) => {
            Datum::form("InterpretAct", [value_datum(sign, Expected::Unknown)])
        }
        data!(Act::Bound { variable, .. }) => variable_to_datum(variable),
    }
}

/// Serialize a discourse computation.
#[requires(true)]
#[ensures(true)]
fn discourse_datum(value: &Discourse) -> Datum {
    match value.as_data() {
        data!(Discourse::Perform(act)) => Datum::form("Perform", [act_datum(act)]),
        data!(Discourse::PerformUtterance(entry)) => {
            Datum::form("PerformUtterance", [entry_datum(entry)])
        }
        // Section 7.1: an `Act` or `TranscriptEntry` operand of `Do` is on the
        // implicit performance spine and must not be wrapped.
        data!(Discourse::Do(items)) => Datum::form("Do", items.iter().map(performable_datum)),
        data!(Discourse::Joi(operands)) => Datum::form("Joi", operands.iter().map(discourse_datum)),
        data!(Discourse::NewTopic(inner)) => Datum::form("NewTopic", [discourse_datum(inner)]),
        data!(Discourse::Resume(inner)) => Datum::form("Resume", [discourse_datum(inner)]),
        data!(Discourse::Prior) => Datum::atom("PriorDiscourse"),
        data!(Discourse::Following) => Datum::atom("FollowingDiscourse"),
        data!(Discourse::Bound(variable)) => variable_to_datum(variable),
    }
}

/// Serialize a transcript entry.
#[requires(true)]
#[ensures(true)]
fn entry_datum(value: &TranscriptEntry) -> Datum {
    match value.as_data() {
        data!(TranscriptEntry::Utterance { token, facts }) => {
            let mut values = vec![Datum::list([Datum::list([
                variable_to_datum(token),
                Datum::atom("UtteranceToken"),
            ])])];
            values.extend(
                facts
                    .iter()
                    .map(|fact| content_datum(fact, content_expected())),
            );
            Datum::form("Utterance", values)
        }
        data!(TranscriptEntry::Bound(variable)) => variable_to_datum(variable),
    }
}

/// Serialize content, applying the section 5.2 `Close` elision.
#[requires(true)]
#[ensures(true)]
fn content_datum(value: &Content, expected: Expected<'_>) -> Datum {
    if let data!(Content::Close(predicate)) = value.as_data()
        && expected.requires_content()
        && close_is_elidable(predicate)
    {
        return predicate_datum(predicate);
    }
    match value.as_data() {
        data!(Content::Close(predicate)) => Datum::form("Close", [predicate_datum(predicate)]),
        data!(Content::Not(inner)) => Datum::form("¬", [content_datum(inner, content_expected())]),
        data!(Content::Junction { operator, operands }) => Datum::form(
            operator.as_str(),
            operands
                .iter()
                .map(|operand| content_datum(operand, content_expected())),
        ),
        data!(Content::Binary {
            operator,
            left,
            right,
        }) => Datum::form(
            operator.as_str(),
            [
                content_datum(left, content_expected()),
                content_datum(right, content_expected()),
            ],
        ),
        data!(Content::Quantified { operator, lambda }) => Datum::form(
            operator.as_str(),
            [lambda_datum(lambda, |body, expected| {
                content_datum(body, expected)
            })],
        ),
        data!(Content::Presuppose { trigger, body }) => Datum::form(
            "Presuppose",
            [
                content_datum(trigger, content_expected()),
                content_datum(body, content_expected()),
            ],
        ),
        data!(Content::Supplement { body, side }) => Datum::form(
            "Supplement",
            [
                content_datum(body, content_expected()),
                content_datum(side, content_expected()),
            ],
        ),
        data!(Content::Answer { query, selection }) => Datum::form(
            "Answer",
            [query_datum(query), answer_selection_datum(selection)],
        ),
        data!(Content::Intrinsic {
            intrinsic,
            arguments,
        }) => intrinsic_datum(*intrinsic, arguments),
        data!(Content::Apply { head, arguments }) => application_datum(head, arguments),
        data!(Content::Let(form)) => let_datum(form, |body| content_datum(body, expected)),
        data!(Content::Bind(form)) => bind_datum(form, |body| content_datum(body, expected)),
        data!(Content::LetRec(form)) => let_rec_datum(form, |body| content_datum(body, expected)),
        data!(Content::Bound(variable)) => variable_to_datum(variable),
    }
}

/// Report whether a `Close` node may be omitted at a `Content` operand.
///
/// Section 5.2 additionally requires that the term is inline and not referenced
/// elsewhere; a bound predicate term is exactly the case that is referenced, so
/// it keeps its `Close`.
#[requires(true)]
#[ensures(true)]
fn close_is_elidable(predicate: &PredTerm) -> bool {
    !matches!(predicate.as_data(), data!(PredTerm::Bound { .. }))
}

/// Serialize a query value.
#[requires(true)]
#[ensures(true)]
fn query_datum(value: &Query) -> Datum {
    match value {
        Query::Polar(content) => Datum::form("Polar", [content_datum(content, content_expected())]),
        Query::Open(lambda) => Datum::form(
            "OpenQ",
            [lambda_datum(lambda, |body, expected| {
                content_datum(body, expected)
            })],
        ),
        Query::Bound { variable, .. } => variable_to_datum(variable),
    }
}

/// Serialize an answer selection.
#[requires(true)]
#[ensures(true)]
fn answer_selection_datum(value: &AnswerSelection) -> Datum {
    match value.as_data() {
        data!(AnswerSelection::Polar(polarity)) => {
            Datum::form("PolarAnswer", [Datum::atom(polarity.as_str())])
        }
        data!(AnswerSelection::Tuple {
            values,
            exhaustivity,
        }) => {
            let mut items = vec![Datum::form(
                "Tuple",
                values
                    .iter()
                    .map(|value| value_datum(value, Expected::Unknown)),
            )];
            if let Some(exhaustivity) = exhaustivity {
                items.push(Datum::atom(exhaustivity.as_str()));
            }
            Datum::form("TupleAnswer", items)
        }
        // Section 14.1: a zero-operand selection prints as the bare atom.
        data!(AnswerSelection::Contextual(exhaustivity)) => exhaustivity.map_or_else(
            || Datum::atom("ContextualAnswer"),
            |exhaustivity| Datum::form("ContextualAnswer", [Datum::atom(exhaustivity.as_str())]),
        ),
        data!(AnswerSelection::Unresolved) => Datum::atom("UnresolvedAnswer"),
    }
}

/// Serialize a predicate term.
#[requires(true)]
#[ensures(true)]
fn predicate_datum(value: &PredTerm) -> Datum {
    match value.as_data() {
        data!(PredTerm::Relation(signature)) => relation_ref_to_datum(signature.relation()),
        data!(PredTerm::Applied { head, fills, .. }) => {
            let signature = head.signature();
            let mut values = vec![predicate_datum(head)];
            for fill in fills {
                values.extend(fill_datums(fill, &signature));
            }
            Datum::list(values)
        }
        data!(PredTerm::Bound { variable, .. }) => variable_to_datum(variable),
        data!(PredTerm::Let(form)) => let_datum(form, predicate_datum),
        data!(PredTerm::Bind(form)) => bind_datum(form, predicate_datum),
        data!(PredTerm::LetRec(form)) => let_rec_datum(form, predicate_datum),
    }
}

/// Serialize one place fill, which may occupy two datum positions.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn fill_datums(fill: &PlaceFill, signature: &PredicateSignature) -> Vec<Datum> {
    match fill.as_data() {
        data!(PlaceFill::Plain(value)) => vec![operand_datum(
            value,
            slot_expectation(signature, fill).unwrap_or(Expected::Unknown),
        )],
        data!(PlaceFill::Numbered { place, value }) => vec![
            Datum::atom(format!(":{place}")),
            operand_datum(
                value,
                slot_expectation(signature, fill).unwrap_or(Expected::Unknown),
            ),
        ],
        data!(PlaceFill::Eventuality(value)) => vec![
            Datum::atom(":Eventuality"),
            operand_datum(
                value,
                slot_expectation(signature, fill).unwrap_or(Expected::Unknown),
            ),
        ],
        data!(PlaceFill::Computed { place, value, .. }) => vec![Datum::form(
            "At",
            [
                value_datum(place, Expected::Unknown),
                operand_datum(value, Expected::Unknown),
            ],
        )],
    }
}

/// Return the accepted type of the row slot a literal fill targets.
///
/// Only literal labelled and event fills name their slot without replaying the
/// cursor, so a plain fill keeps its crossings explicit rather than guessing.
#[requires(true)]
#[ensures(true)]
fn slot_expectation<'a>(
    signature: &'a PredicateSignature,
    fill: &PlaceFill,
) -> Option<Expected<'a>> {
    use super::super::kernel::types::PlaceLabel;

    let label = match fill.as_data() {
        data!(PlaceFill::Numbered { place, .. }) => PlaceLabel::Numbered(place.clone()),
        data!(PlaceFill::Eventuality(_)) => PlaceLabel::Eventuality,
        _ => return None,
    };
    signature
        .row()
        .slots()
        .iter()
        .find(|slot| slot.label_ref() == &label)
        .map(|slot| Expected::Type(slot.accepted_type()))
}

/// Serialize any operand, applying the singleton-lift elision.
#[requires(true)]
#[ensures(true)]
fn operand_datum(value: &Operand, expected: Expected<'_>) -> Datum {
    match value {
        Operand::Value(inner) => value_datum(inner, expected),
        Operand::Content(inner) => content_datum(inner, expected),
        Operand::Predicate(inner) => predicate_datum(inner),
        Operand::Function(inner) => function_datum(inner),
        Operand::Query(inner) => query_datum(inner),
        Operand::Act(inner) => act_datum(inner),
        Operand::Discourse(inner) => discourse_datum(inner),
        Operand::Entry(inner) => entry_datum(inner),
    }
}

/// Serialize a first-order value.
#[requires(true)]
#[ensures(true)]
fn value_datum(value: &Value, expected: Expected<'_>) -> Datum {
    // Section 3.3: a singleton lift is elidable exactly at a statically known
    // `Referents<T>` operand.
    if let data!(Value::Intrinsic {
        intrinsic,
        arguments,
        ..
    }) = value.as_data()
        && *intrinsic == Intrinsic::Singleton
        && expected.requires_referents()
        && let [lifted] = arguments.as_slice()
    {
        return operand_datum(lifted, Expected::Unknown);
    }
    match value.as_data() {
        data!(Value::Literal(literal)) => literal_datum(literal),
        data!(Value::Collection {
            kind,
            element_type,
            items,
        }) => {
            let mut values = Vec::new();
            // Section 2.3: an empty collection always prints its element type,
            // and a nonempty one omits it only when the context determines it.
            if items.is_empty() || !expected.determines_element(element_type) {
                values.push(type_to_datum(element_type));
            }
            values.extend(
                items
                    .iter()
                    .map(|item| value_datum(item, Expected::Type(element_type))),
            );
            Datum::form(kind.as_str(), values)
        }
        data!(Value::Tuple(elements)) => Datum::form(
            "Tuple",
            elements
                .iter()
                .map(|element| value_datum(element, Expected::Unknown)),
        ),
        data!(Value::Sign { token, kind, facts }) => {
            let mut values = vec![Datum::list([Datum::list([
                variable_to_datum(token),
                type_to_datum(&TypeExpr::SignToken(*kind)),
            ])])];
            values.extend(
                facts
                    .iter()
                    .map(|fact| content_datum(fact, content_expected())),
            );
            Datum::form("Sign", values)
        }
        data!(Value::Intrinsic {
            intrinsic,
            arguments,
            ..
        }) => intrinsic_datum(*intrinsic, arguments),
        data!(Value::Apply {
            head,
            arguments,
            ..
        }) => application_datum(head, arguments),
        data!(Value::Let(form)) => let_datum(form, |body| value_datum(body, expected)),
        data!(Value::Bind(form)) => bind_datum(form, |body| value_datum(body, expected)),
        data!(Value::LetRec(form)) => let_rec_datum(form, |body| value_datum(body, expected)),
        data!(Value::Bound { variable, .. }) => variable_to_datum(variable),
    }
}

/// Serialize a callable value.
#[requires(true)]
#[ensures(true)]
fn function_datum(value: &FnValue) -> Datum {
    match value.as_data() {
        data!(FnValue::Lambda(lambda)) => {
            lambda_datum(lambda, |body, expected| operand_datum(body, expected))
        }
        data!(FnValue::Intrinsic {
            intrinsic,
            arguments,
            ..
        }) => intrinsic_datum(*intrinsic, arguments),
        data!(FnValue::Registered { name, .. }) => Datum::atom(name.as_str()),
        data!(FnValue::Bound { variable, .. }) => variable_to_datum(variable),
        data!(FnValue::Let(form)) => let_datum(form, function_datum),
        data!(FnValue::Bind(form)) => bind_datum(form, function_datum),
        data!(FnValue::LetRec(form)) => let_rec_datum(form, function_datum),
    }
}

/// Serialize a reference computation.
#[requires(true)]
#[ensures(true)]
fn reference_computation_datum(value: &RefComp) -> Datum {
    match value.as_data() {
        data!(RefComp::Refer { property }) => Datum::form(
            "Refer",
            [lambda_datum(property, |body, expected| {
                content_datum(body, expected)
            })],
        ),
        data!(RefComp::Typical { property }) => Datum::form(
            "Typical",
            [lambda_datum(property, |body, expected| {
                content_datum(body, expected)
            })],
        ),
        data!(RefComp::Stereotypical {
            describer,
            property,
        }) => Datum::form(
            "Stereotypical",
            [
                value_datum(describer, Expected::Unknown),
                lambda_datum(property, |body, expected| content_datum(body, expected)),
            ],
        ),
        // Section 14.1: with zero dependencies the canonical form is the bare
        // atom, not `(Context)`.
        data!(RefComp::Context { dependencies, .. }) => {
            if dependencies.is_empty() {
                Datum::atom("Context")
            } else {
                Datum::form("Context", dependencies.iter().map(variable_to_datum))
            }
        }
        data!(RefComp::Witnesses { run, .. }) => Datum::form("Witnesses", [variable_to_datum(run)]),
    }
}

/// Serialize a registered callable applied to its operands.
///
/// A nullary registered constant prints as the bare atom; the grammar has no
/// empty application.
#[requires(true)]
#[ensures(true)]
fn intrinsic_datum(intrinsic: Intrinsic, arguments: &[Operand]) -> Datum {
    if arguments.is_empty() {
        return Datum::atom(intrinsic.as_str());
    }
    Datum::form(intrinsic.as_str(), arguments.iter().map(call_operand_datum))
}

/// Serialize one operand of a registered call.
///
/// The registry table in `Intrinsic::instantiate` is written as a checker
/// rather than a projection, so declared operand types are not recoverable from
/// it. What *is* recoverable is the one fact the singleton-lift elision needs:
/// an operand of reference type type-checked against a declared operand, and
/// the kernel's operand rule admits only upcasts, so that declared operand is
/// itself a reference type and section 3.3 licenses omitting the lift. No other
/// elision is claimed here.
#[requires(true)]
#[ensures(true)]
fn call_operand_datum(argument: &Operand) -> Datum {
    let value_type = argument.value_type();
    if matches!(value_type, TypeExpr::Referents(_)) {
        return operand_datum(argument, Expected::Type(&value_type));
    }
    operand_datum(argument, Expected::Unknown)
}

/// Serialize an ordinary application without imposing left association.
#[requires(true)]
#[ensures(true)]
fn application_datum(head: &FnValue, arguments: &[Operand]) -> Datum {
    let declared = head
        .signature()
        .map(|signature| signature.parameters().to_vec())
        .unwrap_or_default();
    let mut values = vec![function_datum(head)];
    values.extend(arguments.iter().enumerate().map(|(index, argument)| {
        operand_datum(
            argument,
            declared
                .get(index)
                .map_or(Expected::Unknown, Expected::Type),
        )
    }));
    Datum::list(values)
}

/// Serialize a lambda with its complete ordered typed parameter list.
#[requires(true)]
#[ensures(ret.form_head() == Some("λ"))]
fn lambda_datum<C: Category, F>(lambda: &Lambda<C>, body: F) -> Datum
where
    F: Fn(&C, Expected<'_>) -> Datum,
{
    Datum::form(
        "λ",
        [
            Datum::list(lambda.parameters().iter().map(|parameter| {
                Datum::list([
                    variable_to_datum(parameter.variable()),
                    type_to_datum(parameter.declared_type()),
                ])
            })),
            body(lambda.body(), Expected::Type(lambda.result_type())),
        ],
    )
}

/// Serialize a declaration block as nested one-declaration `Let` forms.
#[requires(true)]
#[ensures(ret.form_head() == Some("Let"))]
fn let_datum<C: Category, F>(form: &Let<C>, body: F) -> Datum
where
    F: FnOnce(&C) -> Datum,
{
    let mut datum = body(form.body());
    for declaration in form.declarations().iter().rev() {
        datum = Datum::form(
            "Let",
            [
                Datum::list([Datum::list([
                    variable_to_datum(declaration.variable()),
                    type_to_datum(declaration.declared_type()),
                    operand_datum(
                        declaration.initializer(),
                        Expected::Type(declaration.declared_type()),
                    ),
                ])]),
                datum,
            ],
        );
    }
    datum
}

/// Serialize a dynamic binder.
#[requires(true)]
#[ensures(ret.form_head() == Some("Bind"))]
fn bind_datum<C: Category, F>(form: &Bind<C>, body: F) -> Datum
where
    F: FnOnce(&C) -> Datum,
{
    Datum::form(
        "Bind",
        [
            Datum::list([Datum::list([
                variable_to_datum(form.variable()),
                type_to_datum(form.declared_type()),
                reference_computation_datum(form.computation()),
            ])]),
            body(form.body()),
        ],
    )
}

/// Serialize a recursive binding group.
#[requires(true)]
#[ensures(ret.form_head() == Some("LetRec"))]
fn let_rec_datum<C: Category, F>(form: &LetRec<C>, body: F) -> Datum
where
    F: FnOnce(&C) -> Datum,
{
    Datum::form(
        "LetRec",
        [
            Datum::list(form.declarations().iter().map(|declaration| {
                Datum::list([
                    variable_to_datum(declaration.variable()),
                    type_to_datum(declaration.declared_type()),
                    function_datum(declaration.initializer()),
                ])
            })),
            body(form.body()),
        ],
    )
}

/// Serialize a closed literal.
#[requires(true)]
#[ensures(true)]
fn literal_datum(value: &Literal) -> Datum {
    match value.as_data() {
        data!(Literal::Integer(integer)) => integer_datum(integer),
        data!(Literal::Rational {
            numerator,
            denominator,
        }) => Datum::form(
            "/",
            [
                integer_datum(numerator),
                integer_datum(&BigInt::from(denominator.clone())),
            ],
        ),
        data!(Literal::Text(text)) => Datum::string(text.as_str()),
        data!(Literal::Force(force)) => Datum::atom(force.as_str()),
        data!(Literal::SignKind(kind)) => Datum::atom(kind.as_str()),
        data!(Literal::AnswerPolarity(polarity)) => Datum::atom(polarity.as_str()),
        data!(Literal::AnswerExhaustivity(exhaustivity)) => Datum::atom(exhaustivity.as_str()),
        data!(Literal::ScalarKind(kind)) => Datum::atom(kind.as_str()),
        data!(Literal::LabelLevel(level)) => Datum::atom(level.as_str()),
        data!(Literal::EndpointInclusion(inclusion)) => Datum::atom(inclusion.as_str()),
        data!(Literal::Proximity(proximity)) => Datum::atom(proximity.as_str()),
        data!(Literal::LexicalScopePolicy(policy)) => Datum::atom(policy.as_str()),
        data!(Literal::Scale(scale)) => Datum::atom(scale.as_str()),
    }
}

/// Serialize an exact decimal integer.
#[requires(true)]
#[ensures(ret.as_integer().is_some())]
fn integer_datum(value: &BigInt) -> Datum {
    Datum::Integer(
        Integer::try_new(&value.to_string()).expect("BigInt has a canonical decimal spelling"),
    )
}

/// The registered `Content` operand expectation.
#[requires(true)]
#[ensures(ret.requires_content())]
fn content_expected() -> Expected<'static> {
    Expected::Type(&CONTENT_TYPE)
}

/// The `Content` atom, used as a borrowed expectation.
static CONTENT_TYPE: TypeExpr = TypeExpr::Atom(TypeAtom::Content);
