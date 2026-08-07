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

use super::super::kernel::binder::{Bind, Category, Lambda, Let, LetRec, free_binders_of};
use super::super::kernel::content::{
    AnswerSelection, AnswerSelectionData, Content, ContentData, Query,
};
use super::super::kernel::document::{BinderUses, KernelDocument};
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
    let uses = document.binder_uses();
    let mut values = vec![Datum::unsigned(0), performable_datum(document.body(), uses)];
    if !words.is_empty() {
        values.push(Datum::form("Words", words.iter().cloned()));
    }
    Datum::form("Smusni", values)
}

/// How many times the whole document uses each binder name.
///
/// Section 2.4's utterance contraction is not a property of the entry being
/// printed: it holds only when the token is unreferenced across the document.
/// The census is taken once, by the scope audit that already walks every use,
/// and carried down from the root to every position an entry can occupy.
type DocumentUses = BinderUses;

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
fn performable_datum(value: &Performable, uses: &DocumentUses) -> Datum {
    match value {
        Performable::Act(act) => act_datum(act, uses),
        Performable::Discourse(discourse) => discourse_datum(discourse, uses),
        // Section 7.2: only on the implicit performance spine may a simple
        // entry contract to the act it realizes.
        Performable::Entry(entry) => contracted_entry(entry, uses)
            .map_or_else(|| entry_datum(entry, uses), |act| act_datum(act, uses)),
        Performable::Let(form) => let_datum(form, uses, |body| performable_datum(body, uses)),
        Performable::Bind(form) => bind_datum(form, uses, |body| performable_datum(body, uses)),
        Performable::LetRec(form) => {
            let_rec_datum(form, uses, |body| performable_datum(body, uses))
        }
    }
}

/// Return the single realized act of a contractible transcript entry.
///
/// Section 2.4 permits the contraction only when the entry's sole
/// identity-dependent fact is one `Realizes` fact, every other fact is omitted
/// so that the document convention supplies the defaults, and the token is
/// unreferenced. Unreferenced is a document-wide condition, so the census must
/// show the `Realizes` subject the contraction consumes as the token's only use
/// anywhere. The act is additionally checked for free occurrences, which covers
/// the one position the census cannot see: a `Context` dependency list stores no
/// type and therefore records no use.
#[requires(true)]
#[ensures(true)]
fn contracted_entry<'a>(entry: &'a TranscriptEntry, uses: &DocumentUses) -> Option<&'a Act> {
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
    (variable == token && uses.get(token) == Some(&1) && !free_binders_of(act).contains(token))
        .then_some(act)
}

/// Serialize an act.
#[requires(true)]
#[ensures(true)]
fn act_datum(value: &Act, uses: &DocumentUses) -> Datum {
    match value.as_data() {
        data!(Act::Assert(content)) => {
            Datum::form("Assert", [content_datum(content, content_expected(), uses)])
        }
        data!(Act::Ask(query)) => Datum::form("Ask", [query_datum(query, uses)]),
        data!(Act::Command { addressee, content }) => Datum::form(
            "Command",
            [
                value_datum(addressee, Expected::Unknown, uses),
                content_datum(content, content_expected(), uses),
            ],
        ),
        data!(Act::Express(content)) => Datum::form(
            "Express",
            [content_datum(content, content_expected(), uses)],
        ),
        data!(Act::Mention(operand)) => {
            Datum::form("Mention", [operand_datum(operand, Expected::Unknown, uses)])
        }
        data!(Act::Vocative(addressee)) => Datum::form(
            "Vocative",
            [value_datum(addressee, Expected::Unknown, uses)],
        ),
        data!(Act::Interpret { sign, .. }) => {
            Datum::form("InterpretAct", [value_datum(sign, Expected::Unknown, uses)])
        }
        data!(Act::Bound { variable, .. }) => variable_to_datum(variable),
    }
}

/// Serialize a discourse computation.
#[requires(true)]
#[ensures(true)]
fn discourse_datum(value: &Discourse, uses: &DocumentUses) -> Datum {
    match value.as_data() {
        data!(Discourse::Perform(act)) => Datum::form("Perform", [act_datum(act, uses)]),
        data!(Discourse::PerformUtterance(entry)) => {
            Datum::form("PerformUtterance", [entry_datum(entry, uses)])
        }
        // Section 7.1: an `Act` or `TranscriptEntry` operand of `Do` is on the
        // implicit performance spine and must not be wrapped.
        data!(Discourse::Do(items)) => {
            Datum::form("Do", items.iter().map(|item| performable_datum(item, uses)))
        }
        data!(Discourse::Joi(operands)) => Datum::form(
            "Joi",
            operands
                .iter()
                .map(|operand| discourse_datum(operand, uses)),
        ),
        data!(Discourse::NewTopic(inner)) => {
            Datum::form("NewTopic", [discourse_datum(inner, uses)])
        }
        data!(Discourse::Resume(inner)) => Datum::form("Resume", [discourse_datum(inner, uses)]),
        data!(Discourse::Prior) => Datum::atom("PriorDiscourse"),
        data!(Discourse::Following) => Datum::atom("FollowingDiscourse"),
        data!(Discourse::Bound(variable)) => variable_to_datum(variable),
    }
}

/// Serialize a transcript entry.
#[requires(true)]
#[ensures(true)]
fn entry_datum(value: &TranscriptEntry, uses: &DocumentUses) -> Datum {
    match value.as_data() {
        data!(TranscriptEntry::Utterance { token, facts }) => {
            let mut values = vec![Datum::list([Datum::list([
                variable_to_datum(token),
                Datum::atom("UtteranceToken"),
            ])])];
            values.extend(
                facts
                    .iter()
                    .map(|fact| content_datum(fact, content_expected(), uses)),
            );
            Datum::form("Utterance", values)
        }
        data!(TranscriptEntry::Bound(variable)) => variable_to_datum(variable),
    }
}

/// Serialize content, applying the section 5.2 `Close` elision.
#[requires(true)]
#[ensures(true)]
fn content_datum(value: &Content, expected: Expected<'_>, uses: &DocumentUses) -> Datum {
    if let data!(Content::Close(predicate)) = value.as_data()
        && expected.requires_content()
        && close_is_elidable(predicate)
    {
        return predicate_datum(predicate, uses);
    }
    match value.as_data() {
        data!(Content::Close(predicate)) => {
            Datum::form("Close", [predicate_datum(predicate, uses)])
        }
        data!(Content::Not(inner)) => {
            Datum::form("¬", [content_datum(inner, content_expected(), uses)])
        }
        data!(Content::Junction { operator, operands }) => Datum::form(
            operator.as_str(),
            operands
                .iter()
                .map(|operand| content_datum(operand, content_expected(), uses)),
        ),
        data!(Content::Binary {
            operator,
            left,
            right,
        }) => Datum::form(
            operator.as_str(),
            [
                content_datum(left, content_expected(), uses),
                content_datum(right, content_expected(), uses),
            ],
        ),
        // The lambda of a quantifier is a `Property<T>`, so the position itself
        // declares a `Content` body.
        data!(Content::Quantified { operator, lambda }) => Datum::form(
            operator.as_str(),
            [lambda_datum(
                lambda,
                content_expected(),
                |body, expected| content_datum(body, expected, uses),
            )],
        ),
        data!(Content::Presuppose { trigger, body }) => Datum::form(
            "Presuppose",
            [
                content_datum(trigger, content_expected(), uses),
                content_datum(body, content_expected(), uses),
            ],
        ),
        data!(Content::Supplement { body, side }) => Datum::form(
            "Supplement",
            [
                content_datum(body, content_expected(), uses),
                content_datum(side, content_expected(), uses),
            ],
        ),
        data!(Content::Answer { query, selection }) => Datum::form(
            "Answer",
            [
                query_datum(query, uses),
                answer_selection_datum(selection, uses),
            ],
        ),
        data!(Content::Intrinsic {
            intrinsic,
            arguments,
        }) => intrinsic_datum(*intrinsic, arguments, uses),
        data!(Content::Apply { head, arguments }) => application_datum(head, arguments, uses),
        data!(Content::Let(form)) => {
            let_datum(form, uses, |body| content_datum(body, expected, uses))
        }
        data!(Content::Bind(form)) => {
            bind_datum(form, uses, |body| content_datum(body, expected, uses))
        }
        data!(Content::LetRec(form)) => {
            let_rec_datum(form, uses, |body| content_datum(body, expected, uses))
        }
        data!(Content::Bound(variable)) => variable_to_datum(variable),
    }
}

/// Report whether a `Close` node may be omitted at a `Content` operand.
///
/// Section 5.2 requires both that the term is inline and not referenced
/// elsewhere — a bound predicate term is exactly the case that is referenced,
/// so it keeps its `Close` — and that its effective row is statically known. An
/// unknown numbered tail is the second condition failing: a `mo`-like relation
/// question's `Close` is deferred to answer substitution (section 4.3), so it
/// must stay on the surface however the term was assembled.
#[requires(true)]
#[ensures(true)]
fn close_is_elidable(predicate: &PredTerm) -> bool {
    !matches!(predicate.as_data(), data!(PredTerm::Bound { .. }))
        && !predicate.row().has_open_numbered_tail()
}

/// Serialize a query value.
#[requires(true)]
#[ensures(true)]
fn query_datum(value: &Query, uses: &DocumentUses) -> Datum {
    match value {
        Query::Polar(content) => {
            Datum::form("Polar", [content_datum(content, content_expected(), uses)])
        }
        // An open question binds a `Property<T>`, which declares a `Content`
        // body just as a quantifier's lambda does.
        Query::Open(lambda) => Datum::form(
            "OpenQ",
            [lambda_datum(
                lambda,
                content_expected(),
                |body, expected| content_datum(body, expected, uses),
            )],
        ),
        Query::Bound { variable, .. } => variable_to_datum(variable),
    }
}

/// Serialize an answer selection.
#[requires(true)]
#[ensures(true)]
fn answer_selection_datum(value: &AnswerSelection, uses: &DocumentUses) -> Datum {
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
                    .map(|value| value_datum(value, Expected::Unknown, uses)),
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
fn predicate_datum(value: &PredTerm, uses: &DocumentUses) -> Datum {
    match value.as_data() {
        data!(PredTerm::Relation(signature)) => relation_ref_to_datum(signature.relation()),
        data!(PredTerm::Applied {
            head,
            fills,
            result,
        }) => {
            let declared = result.filled_types();
            let mut values = vec![predicate_datum(head, uses)];
            for (index, fill) in fills.iter().enumerate() {
                values.extend(fill_datums(
                    fill,
                    declared.get(index).and_then(Option::as_ref),
                    uses,
                ));
            }
            Datum::list(values)
        }
        data!(PredTerm::Bound { variable, .. }) => variable_to_datum(variable),
        data!(PredTerm::Let(form)) => let_datum(form, uses, |body| predicate_datum(body, uses)),
        data!(PredTerm::Bind(form)) => bind_datum(form, uses, |body| predicate_datum(body, uses)),
        data!(PredTerm::LetRec(form)) => {
            let_rec_datum(form, uses, |body| predicate_datum(body, uses))
        }
    }
}

/// Serialize one place fill, which may occupy two datum positions.
///
/// `declared` is the type the slot this fill consumed accepts, which the
/// application kernel recorded when its cursor selected that slot. A plain fill
/// is therefore just as declared as a labelled one, so both license the same
/// expected-type elisions; only a computed fill, which reserves a domain rather
/// than consuming one slot, declares nothing.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn fill_datums(fill: &PlaceFill, declared: Option<&TypeExpr>, uses: &DocumentUses) -> Vec<Datum> {
    let expected = declared.map_or(Expected::Unknown, Expected::Type);
    match fill.as_data() {
        data!(PlaceFill::Plain(value)) => vec![operand_datum(value, expected, uses)],
        data!(PlaceFill::Numbered { place, value }) => vec![
            Datum::atom(format!(":{place}")),
            operand_datum(value, expected, uses),
        ],
        data!(PlaceFill::Eventuality(value)) => vec![
            Datum::atom(":Eventuality"),
            operand_datum(value, expected, uses),
        ],
        data!(PlaceFill::Computed { place, value, .. }) => vec![Datum::form(
            "At",
            [
                value_datum(place, Expected::Unknown, uses),
                operand_datum(value, Expected::Unknown, uses),
            ],
        )],
    }
}

/// Serialize any operand, applying the singleton-lift elision.
#[requires(true)]
#[ensures(true)]
fn operand_datum(value: &Operand, expected: Expected<'_>, uses: &DocumentUses) -> Datum {
    match value {
        Operand::Value(inner) => value_datum(inner, expected, uses),
        Operand::Content(inner) => content_datum(inner, expected, uses),
        Operand::Predicate(inner) => predicate_datum(inner, uses),
        Operand::Function(inner) => function_datum(inner, expected, uses),
        Operand::Query(inner) => query_datum(inner, uses),
        Operand::Act(inner) => act_datum(inner, uses),
        Operand::Discourse(inner) => discourse_datum(inner, uses),
        Operand::Entry(inner) => entry_datum(inner, uses),
    }
}

/// Serialize a first-order value.
#[requires(true)]
#[ensures(true)]
fn value_datum(value: &Value, expected: Expected<'_>, uses: &DocumentUses) -> Datum {
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
        return operand_datum(lifted, Expected::Unknown, uses);
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
                    .map(|item| value_datum(item, Expected::Type(element_type), uses)),
            );
            Datum::form(kind.as_str(), values)
        }
        data!(Value::Tuple(elements)) => Datum::form(
            "Tuple",
            elements
                .iter()
                .map(|element| value_datum(element, Expected::Unknown, uses)),
        ),
        data!(Value::Sign { token, kind, facts }) => {
            let mut values = vec![Datum::list([Datum::list([
                variable_to_datum(token),
                type_to_datum(&TypeExpr::SignToken(*kind)),
            ])])];
            values.extend(
                facts
                    .iter()
                    .map(|fact| content_datum(fact, content_expected(), uses)),
            );
            Datum::form("Sign", values)
        }
        data!(Value::Intrinsic {
            intrinsic,
            arguments,
            ..
        }) => intrinsic_datum(*intrinsic, arguments, uses),
        data!(Value::Apply {
            head,
            arguments,
            ..
        }) => application_datum(head, arguments, uses),
        data!(Value::Let(form)) => let_datum(form, uses, |body| value_datum(body, expected, uses)),
        data!(Value::Bind(form)) => {
            bind_datum(form, uses, |body| value_datum(body, expected, uses))
        }
        data!(Value::LetRec(form)) => {
            let_rec_datum(form, uses, |body| value_datum(body, expected, uses))
        }
        data!(Value::Bound { variable, .. }) => variable_to_datum(variable),
    }
}

/// Serialize a callable value at a position expecting `expected`.
#[requires(true)]
#[ensures(true)]
fn function_datum(value: &FnValue, expected: Expected<'_>, uses: &DocumentUses) -> Datum {
    match value.as_data() {
        data!(FnValue::Lambda(lambda)) => {
            lambda_datum(lambda, lambda_body_expected(expected), |body, expected| {
                operand_datum(body, expected, uses)
            })
        }
        data!(FnValue::Intrinsic {
            intrinsic,
            arguments,
            ..
        }) => intrinsic_datum(*intrinsic, arguments, uses),
        data!(FnValue::Registered { name, .. }) => Datum::atom(name.as_str()),
        data!(FnValue::Bound { variable, .. }) => variable_to_datum(variable),
        data!(FnValue::Let(form)) => {
            let_datum(form, uses, |body| function_datum(body, expected, uses))
        }
        data!(FnValue::Bind(form)) => {
            bind_datum(form, uses, |body| function_datum(body, expected, uses))
        }
        data!(FnValue::LetRec(form)) => {
            let_rec_datum(form, uses, |body| function_datum(body, expected, uses))
        }
    }
}

/// Return what the enclosing position requires of a lambda's body.
///
/// Only a declared function type licenses an elision inside a lambda body: a
/// `Let` or `LetRec` initializer prints its declared `Fn` type, a row slot and
/// an application parameter carry theirs, and a registered operand carries the
/// one section 14.1 declares, projected by `Intrinsic::declared_operand_types`.
/// A `Lambda<Operand>` at a polymorphic position — a `Mention` operand, a
/// `Denotes` denoted value, a `Label` target, or an application head — declares
/// nothing, and section 2.2 then reads that surface as a lambda returning a
/// `PredTerm`, so a `Content` body must print its `Close`. Deriving
/// the expectation from the lambda's own inferred result would make every
/// position look declared, and the round-trip oracle cannot see the difference:
/// the over-elided text is a fixed point of parse-then-print.
#[requires(true)]
#[ensures(true)]
fn lambda_body_expected(expected: Expected<'_>) -> Expected<'_> {
    match expected {
        Expected::Type(TypeExpr::Function { result, .. }) => Expected::Type(result),
        // Section 3.2: `GQ<T>` means exactly `Fn<(Property<T>), Content>`.
        Expected::Type(TypeExpr::GeneralizedQuantifier(_)) => content_expected(),
        _ => Expected::Unknown,
    }
}

/// Serialize a reference computation.
#[requires(true)]
#[ensures(true)]
fn reference_computation_datum(value: &RefComp, uses: &DocumentUses) -> Datum {
    match value.as_data() {
        // Every description property is a declared `Property<Referents<T>>`, so
        // these positions license the `Close` elision in the body.
        data!(RefComp::Refer { property }) => Datum::form(
            "Refer",
            [lambda_datum(
                property,
                content_expected(),
                |body, expected| content_datum(body, expected, uses),
            )],
        ),
        data!(RefComp::Typical { property }) => Datum::form(
            "Typical",
            [lambda_datum(
                property,
                content_expected(),
                |body, expected| content_datum(body, expected, uses),
            )],
        ),
        data!(RefComp::Stereotypical {
            describer,
            property,
        }) => Datum::form(
            "Stereotypical",
            [
                value_datum(describer, Expected::Unknown, uses),
                lambda_datum(property, content_expected(), |body, expected| {
                    content_datum(body, expected, uses)
                }),
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
fn intrinsic_datum(intrinsic: Intrinsic, arguments: &[Operand], uses: &DocumentUses) -> Datum {
    if arguments.is_empty() {
        return Datum::atom(intrinsic.as_str());
    }
    let argument_types = arguments
        .iter()
        .map(Category::value_type)
        .collect::<Vec<_>>();
    let declared = intrinsic.declared_operand_types(&argument_types);
    Datum::form(
        intrinsic.as_str(),
        arguments
            .iter()
            .zip(&declared)
            .map(|(argument, declared)| call_operand_datum(argument, declared.as_ref(), uses)),
    )
}

/// Serialize one operand of a registered call at its declared type.
///
/// `Intrinsic::declared_operand_types` projects section 14.1's registry, so a
/// registered operand licenses exactly the elisions its declaration licenses:
/// a `Content` operand omits `Close`, a `Referents<T>` operand omits the
/// singleton lift, and a declared `Property<T>` — every quantifier restriction,
/// `SetOf` comprehension, and prelude property operand — carries `Content` into
/// its lambda body. An operand the registry leaves free declares nothing, and
/// every crossing there stays explicit.
#[requires(true)]
#[ensures(true)]
fn call_operand_datum(
    argument: &Operand,
    declared: Option<&TypeExpr>,
    uses: &DocumentUses,
) -> Datum {
    operand_datum(
        argument,
        declared.map_or(Expected::Unknown, Expected::Type),
        uses,
    )
}

/// Serialize an ordinary application without imposing left association.
///
/// The head position declares nothing about the head itself — the operand types
/// below come from the head's own signature, which a lambda head prints in full,
/// but its result type is never on the surface.
#[requires(true)]
#[ensures(true)]
fn application_datum(head: &FnValue, arguments: &[Operand], uses: &DocumentUses) -> Datum {
    let declared = head
        .signature()
        .map(|signature| signature.parameters().to_vec())
        .unwrap_or_default();
    let mut values = vec![function_datum(head, Expected::Unknown, uses)];
    values.extend(arguments.iter().enumerate().map(|(index, argument)| {
        operand_datum(
            argument,
            declared
                .get(index)
                .map_or(Expected::Unknown, Expected::Type),
            uses,
        )
    }));
    Datum::list(values)
}

/// Serialize a lambda with its complete ordered typed parameter list.
///
/// `body_expected` is what the *enclosing* position requires of the body, never
/// what the lambda's own result type happens to be; see [`lambda_body_expected`].
#[requires(true)]
#[ensures(ret.form_head() == Some("λ"))]
fn lambda_datum<C: Category, F>(lambda: &Lambda<C>, body_expected: Expected<'_>, body: F) -> Datum
where
    F: FnOnce(&C, Expected<'_>) -> Datum,
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
            body(lambda.body(), body_expected),
        ],
    )
}

/// Serialize a declaration block as nested one-declaration `Let` forms.
#[requires(true)]
#[ensures(ret.form_head() == Some("Let"))]
fn let_datum<C: Category, F>(form: &Let<C>, uses: &DocumentUses, body: F) -> Datum
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
                        uses,
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
fn bind_datum<C: Category, F>(form: &Bind<C>, uses: &DocumentUses, body: F) -> Datum
where
    F: FnOnce(&C) -> Datum,
{
    Datum::form(
        "Bind",
        [
            Datum::list([Datum::list([
                variable_to_datum(form.variable()),
                type_to_datum(form.declared_type()),
                reference_computation_datum(form.computation(), uses),
            ])]),
            body(form.body()),
        ],
    )
}

/// Serialize a recursive binding group.
#[requires(true)]
#[ensures(ret.form_head() == Some("LetRec"))]
fn let_rec_datum<C: Category, F>(form: &LetRec<C>, uses: &DocumentUses, body: F) -> Datum
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
                    // The declared type is printed right here, so it licenses
                    // whatever elision the initializer's body allows.
                    function_datum(
                        declaration.initializer(),
                        Expected::Type(declaration.declared_type()),
                        uses,
                    ),
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
