//! Construction and printing tests for the typed smusni kernel.
//!
//! The positive tests build a representative value of every semantic category
//! and print it; the acceptance parser is used as an independent oracle for the
//! printer, never as the type gate. The negative tests are the ones that carry
//! the weight of this increment: each of them constructs a shape that the
//! notation's rules forbid and asserts that the kernel refuses it, so none of
//! them would pass if a constructor validated nothing.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use jbotci_semantics::notation::kernel::apply::{FunctionSignature, PredicateSignature};
use jbotci_semantics::notation::kernel::binder::{
    Bind, Declaration, Lambda, Let, LetRec, RecursiveDeclaration, TypedParameter,
};
use jbotci_semantics::notation::kernel::content::{AnswerSelection, Content, JunctionOp, Query};
use jbotci_semantics::notation::kernel::document::KernelDocument;
use jbotci_semantics::notation::kernel::intrinsic::Intrinsic;
use jbotci_semantics::notation::kernel::performable::{
    Act, Discourse, Performable, TranscriptEntry,
};
use jbotci_semantics::notation::kernel::predicate::{PlaceFill, PredTerm};
use jbotci_semantics::notation::kernel::types::{
    AnswerPolarity, LexicalRoot, PlaceLabel, PositiveInteger, RelationRef, Row, RowSlot, SignKind,
    TypeAtom, TypeExpr, Variable,
};
use jbotci_semantics::notation::kernel::value::{
    CollectionKind, FnValue, Literal, Operand, RefComp, Value,
};
use jbotci_semantics::notation::sexpr::kernel_printer::print_kernel_document;
use jbotci_semantics::notation::sexpr::parse_v0_document;

/// Build the `Entity` sort.
#[requires(true)]
#[ensures(true)]
fn entity() -> TypeExpr {
    TypeExpr::Atom(TypeAtom::Entity)
}

/// Build `Referents<T>`.
#[requires(true)]
#[ensures(matches!(ret, TypeExpr::Referents(_)))]
fn referents(inner: TypeExpr) -> TypeExpr {
    TypeExpr::Referents(Box::new(inner))
}

/// Build the `Content` sort.
#[requires(true)]
#[ensures(true)]
fn content_type() -> TypeExpr {
    TypeExpr::Atom(TypeAtom::Content)
}

/// Parse a variable spelling.
#[requires(text.starts_with('$'))]
#[ensures(ret.as_str() == text)]
fn variable(text: &str) -> Variable {
    Variable::try_new(text).expect("test variable spellings are valid")
}

/// Build the effective row of a five-place event-licensed root.
#[requires(places > 0)]
#[ensures(ret.slots().len() == places as usize + 1)]
fn ordinary_row(places: u32) -> Row {
    let mut slots = (1..=places)
        .map(|place| RowSlot::new(PlaceLabel::numbered(place), referents(entity())))
        .collect::<Vec<_>>();
    slots.push(RowSlot::new(
        PlaceLabel::Eventuality,
        referents(TypeExpr::Atom(TypeAtom::Eventuality)),
    ));
    Row::new(slots, false)
}

/// Name one lexical relation with its effective row.
#[requires(!root.is_empty() && places > 0)]
#[ensures(true)]
fn lexical(root: &str, places: u32) -> PredTerm {
    PredTerm::relation(PredicateSignature::new(
        RelationRef::Lexical(LexicalRoot::try_new(root).expect("test roots are valid")),
        ordinary_row(places),
    ))
}

/// Build the deictic `Speaker` constant.
#[requires(true)]
#[ensures(true)]
fn speaker() -> Value {
    Value::intrinsic(Intrinsic::Speaker, Vec::new()).expect("Speaker is a registered constant")
}

/// Fill `klama` x2 with the speaker and close the result.
#[requires(true)]
#[ensures(true)]
fn simple_assertion() -> Act {
    let predicate = PredTerm::applied(
        lexical("klama", 5),
        vec![PlaceFill::numbered(
            PositiveInteger::from_u32(2),
            Operand::Value(speaker()),
        )],
    )
    .expect("filling one numbered place of klama is well typed");
    Act::assert(Content::close(predicate).expect("the remaining row is closeable"))
}

/// Print a document and reparse it through the acceptance grammar.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn round_trip(document: &KernelDocument) -> String {
    let text = print_kernel_document(document, &[]);
    let parsed = parse_v0_document(&text)
        .unwrap_or_else(|error| panic!("printer emitted a non-document: {error}\n{text}"));
    let reprinted = jbotci_semantics::notation::sexpr::print_v0_document(&parsed);
    assert_eq!(reprinted, text, "printing is not stable under reparsing");
    text
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_category_constructs_and_prints_a_document() {
    // A single document that reaches every category: a transcript entry with a
    // sign value and two facts, a directive, an expressive act, a question with
    // an answer, a quantified formula, a dynamic reference, a recursive group,
    // and a typed collection.
    let token = variable("$u");
    let sign_token = variable("$s");

    let sign = Value::sign(
        sign_token.clone(),
        SignKind::Sentence,
        vec![
            Content::intrinsic(
                Intrinsic::TextOf,
                vec![
                    Operand::Value(Value::bound(
                        sign_token.clone(),
                        TypeExpr::SignToken(SignKind::Sentence),
                    )),
                    Operand::Value(Value::literal(Literal::text("mi klama"))),
                ],
            )
            .expect("TextOf relates a sign token to its wording"),
        ],
    )
    .expect("a sign binder carries facts");

    let quantified = Content::quantified(
        jbotci_semantics::notation::kernel::content::QuantifierOp::ForAll,
        Lambda::new(
            vec![TypedParameter::new(
                variable("$x"),
                referents(TypeExpr::Atom(TypeAtom::Eventuality)),
            )],
            Content::close(
                PredTerm::applied(
                    lexical("purci", 2),
                    vec![PlaceFill::eventuality(Operand::Value(Value::bound(
                        variable("$x"),
                        referents(TypeExpr::Atom(TypeAtom::Eventuality)),
                    )))],
                )
                .expect("filling the event place of purci is well typed"),
            )
            .expect("the remaining row is closeable"),
        )
        .expect("a one-parameter lambda is well formed"),
    );

    let reference = RefComp::refer(
        Lambda::new(
            vec![TypedParameter::new(variable("$r"), referents(entity()))],
            Content::close(
                PredTerm::applied(
                    lexical("gerku", 1),
                    vec![PlaceFill::plain(Operand::Value(Value::bound(
                        variable("$r"),
                        referents(entity()),
                    )))],
                )
                .expect("filling gerku x1 is well typed"),
            )
            .expect("the remaining row is closeable"),
        )
        .expect("a description property is a one-parameter lambda"),
    )
    .expect("Refer takes a reference property");

    let dynamic = Content::bind_form(
        Bind::new(
            variable("$dog"),
            referents(entity()),
            reference,
            Content::close(
                PredTerm::applied(
                    lexical("bajra", 2),
                    vec![PlaceFill::plain(Operand::Value(Value::bound(
                        variable("$dog"),
                        referents(entity()),
                    )))],
                )
                .expect("filling bajra x1 is well typed"),
            )
            .expect("the remaining row is closeable"),
        )
        .expect("the computation produces the declared reference type"),
    );

    let recursive_signature = FunctionSignature::new(vec![referents(entity())], content_type());
    let recursive = Content::let_rec_form(
        LetRec::new(
            vec![
                RecursiveDeclaration::new(
                    variable("$loop"),
                    recursive_signature.function_type(),
                    FnValue::lambda(
                        Lambda::new(
                            vec![TypedParameter::new(variable("$y"), referents(entity()))],
                            Operand::Content(
                                Content::apply(
                                    FnValue::bound(variable("$loop"), recursive_signature.clone()),
                                    vec![Operand::Value(Value::bound(
                                        variable("$y"),
                                        referents(entity()),
                                    ))],
                                )
                                .expect("the recursive call matches its signature"),
                            ),
                        )
                        .expect("a one-parameter lambda is well formed"),
                    ),
                )
                .expect("a recursive initializer is an inert lambda"),
            ],
            Content::apply(
                FnValue::bound(variable("$loop"), recursive_signature),
                vec![Operand::Value(speaker())],
            )
            .expect("the call matches its signature"),
        )
        .expect("a recursive group is nonempty and uniquely named"),
    );

    let question = Query::open(
        Lambda::new(
            vec![TypedParameter::new(variable("$who"), referents(entity()))],
            Content::close(
                PredTerm::applied(
                    lexical("klama", 5),
                    vec![PlaceFill::plain(Operand::Value(Value::bound(
                        variable("$who"),
                        referents(entity()),
                    )))],
                )
                .expect("filling klama x1 is well typed"),
            )
            .expect("the remaining row is closeable"),
        )
        .expect("an open question binds a nonempty answer domain"),
    );
    let answered = Content::answer(
        Query::open(
            Lambda::new(
                vec![TypedParameter::new(variable("$what"), referents(entity()))],
                Content::close(
                    PredTerm::applied(
                        lexical("zdani", 2),
                        vec![PlaceFill::plain(Operand::Value(Value::bound(
                            variable("$what"),
                            referents(entity()),
                        )))],
                    )
                    .expect("filling zdani x1 is well typed"),
                )
                .expect("the remaining row is closeable"),
            )
            .expect("an open question binds a nonempty answer domain"),
        ),
        AnswerSelection::tuple(vec![speaker()], None).expect("an answer tuple is nonempty"),
    )
    .expect("the selection matches the query's answer tuple");

    let entry = TranscriptEntry::utterance(
        token.clone(),
        vec![
            Content::intrinsic(
                Intrinsic::SpeakerOf,
                vec![
                    Operand::Value(Value::bound(
                        token.clone(),
                        TypeExpr::Atom(TypeAtom::UtteranceToken),
                    )),
                    Operand::Value(speaker()),
                ],
            )
            .expect("SpeakerOf relates a token to its speaker"),
            Content::intrinsic(
                Intrinsic::Realizes,
                vec![
                    Operand::Value(Value::bound(
                        token.clone(),
                        TypeExpr::Atom(TypeAtom::UtteranceToken),
                    )),
                    Operand::Act(Act::assert(
                        Content::junction(JunctionOp::And, vec![quantified, dynamic, recursive])
                            .expect("a junction has at least two operands"),
                    )),
                ],
            )
            .expect("Realizes relates a token to an act"),
        ],
    )
    .expect("an utterance boundary carries facts");

    let body = Performable::Discourse(
        Discourse::sequence(vec![
            Performable::Entry(entry),
            Performable::Act(Act::ask(question)),
            Performable::Act(Act::assert(answered)),
            Performable::Act(
                Act::command(speaker(), Content::close(lexical("cliva", 3)).unwrap())
                    .expect("a directive names its addressee"),
            ),
            Performable::Act(Act::express(
                Content::close(lexical("gleki", 3)).expect("the row is closeable"),
            )),
            Performable::Act(Act::vocative(speaker()).expect("an address names its addressee")),
            Performable::Act(Act::mention(Operand::Value(sign))),
        ])
        .expect("a discourse sequence has at least two items"),
    );

    let document = KernelDocument::new(body).expect("the document is closed and well scoped");
    let text = round_trip(&document);
    assert!(
        text.starts_with("(Smusni\n  0\n"),
        "unexpected packaging:\n{text}"
    );
    for expected in [
        "(Utterance",
        "(Realizes",
        "(∀",
        "(Bind",
        "(LetRec",
        "(Ask",
        "(Answer",
        "(Command",
        "(Express",
        "(Mention",
        "(Vocative",
        "(Sign",
    ] {
        assert!(text.contains(expected), "missing {expected} in\n{text}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn close_prints_exactly_when_the_notation_cannot_elide_it() {
    // Inline at a registered Content operand: elided.
    let inline = KernelDocument::new(Performable::Act(simple_assertion()))
        .expect("the document is closed and well scoped");
    let inline_text = round_trip(&inline);
    assert!(
        inline_text.contains("(klama :2 Speaker)"),
        "expected the inline Close to be elided:\n{inline_text}"
    );
    assert!(!inline_text.contains("Close"), "{inline_text}");

    // Bound and therefore referenced elsewhere: printed.
    let shared = PredTerm::applied(
        lexical("klama", 5),
        vec![PlaceFill::numbered(
            PositiveInteger::from_u32(2),
            Operand::Value(speaker()),
        )],
    )
    .expect("filling one numbered place of klama is well typed");
    let shared_row = shared.row();
    let bound = KernelDocument::new(Performable::Act(Act::assert(Content::let_form(
        Let::new(
            vec![
                Declaration::new(
                    variable("$p"),
                    TypeExpr::Predicate(shared_row.clone()),
                    Operand::Predicate(shared),
                )
                .expect("the initializer inhabits its declared type"),
            ],
            Content::close(PredTerm::bound(variable("$p"), shared_row))
                .expect("the bound row is closeable"),
        )
        .expect("a Let block is nonempty"),
    ))))
    .expect("the document is closed and well scoped");
    let bound_text = round_trip(&bound);
    assert!(
        bound_text.contains("(Close $p)"),
        "a bound predicate term keeps its Close:\n{bound_text}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_declaration_block_prints_one_let_per_declaration() {
    let first = Declaration::new(
        variable("$a"),
        referents(entity()),
        Operand::Value(speaker()),
    )
    .expect("the initializer inhabits its declared type");
    let second = Declaration::new(
        variable("$b"),
        referents(entity()),
        Operand::Value(Value::bound(variable("$a"), referents(entity()))),
    )
    .expect("a later declaration may use an earlier name");
    let document = KernelDocument::new(Performable::Act(Act::assert(Content::let_form(
        Let::new(
            vec![first, second],
            Content::close(
                PredTerm::applied(
                    lexical("klama", 5),
                    vec![PlaceFill::plain(Operand::Value(Value::bound(
                        variable("$b"),
                        referents(entity()),
                    )))],
                )
                .expect("filling klama x1 is well typed"),
            )
            .expect("the remaining row is closeable"),
        )
        .expect("a Let block is nonempty"),
    ))))
    .expect("the document is closed and well scoped");
    let text = round_trip(&document);
    assert_eq!(
        text.matches("(Let").count(),
        2,
        "one surface Let per declaration:\n{text}"
    );
    assert!(text.contains("$a"), "{text}");
    assert!(text.contains("$b"), "{text}");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_singleton_lift_is_elided_only_where_the_operand_type_is_known() {
    let lifted = Value::intrinsic(
        Intrinsic::Singleton,
        vec![Operand::Value(Value::literal(Literal::integer(3)))],
    )
    .expect("Singleton lifts any value");
    // `Among` declares reference operands, so the lift is elidable there.
    let elided = Content::intrinsic(
        Intrinsic::Among,
        vec![Operand::Value(lifted.clone()), Operand::Value(lifted)],
    )
    .expect("Among relates two references");
    let document = KernelDocument::new(Performable::Act(Act::assert(elided)))
        .expect("the document is closed and well scoped");
    let text = round_trip(&document);
    assert!(
        text.contains("(Among 3 3)"),
        "expected the lift to be elided at a reference operand:\n{text}"
    );

    // At `Mention`, which is polymorphic, nothing is known and the lift prints.
    let mentioned = Value::intrinsic(
        Intrinsic::Singleton,
        vec![Operand::Value(Value::literal(Literal::integer(3)))],
    )
    .expect("Singleton lifts any value");
    let document = KernelDocument::new(Performable::Act(Act::mention(Operand::Value(mentioned))))
        .expect("the document is closed and well scoped");
    let text = round_trip(&document);
    assert!(
        text.contains("(Singleton 3)"),
        "expected the lift to print where no operand type is known:\n{text}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_simple_transcript_entry_contracts_on_the_performance_spine() {
    let token = variable("$u");
    let realizes = Content::intrinsic(
        Intrinsic::Realizes,
        vec![
            Operand::Value(Value::bound(
                token.clone(),
                TypeExpr::Atom(TypeAtom::UtteranceToken),
            )),
            Operand::Act(simple_assertion()),
        ],
    )
    .expect("Realizes relates a token to an act");
    let contracted = KernelDocument::new(Performable::Entry(
        TranscriptEntry::utterance(token.clone(), vec![realizes.clone()])
            .expect("an utterance boundary carries facts"),
    ))
    .expect("the document is closed and well scoped");
    let text = round_trip(&contracted);
    assert!(
        !text.contains("Utterance"),
        "a defaults-only entry contracts on the spine:\n{text}"
    );

    let speaker_of = Content::intrinsic(
        Intrinsic::SpeakerOf,
        vec![
            Operand::Value(Value::bound(
                token.clone(),
                TypeExpr::Atom(TypeAtom::UtteranceToken),
            )),
            Operand::Value(speaker()),
        ],
    )
    .expect("SpeakerOf relates a token to its speaker");
    let retained = KernelDocument::new(Performable::Entry(
        TranscriptEntry::utterance(token, vec![speaker_of, realizes])
            .expect("an utterance boundary carries facts"),
    ))
    .expect("the document is closed and well scoped");
    let text = round_trip(&retained);
    assert!(
        text.contains("(Utterance"),
        "an entry with extra metadata keeps its boundary:\n{text}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_referenced_utterance_token_keeps_its_boundary() {
    // Section 2.4 contracts an entry only when its token is unreferenced across
    // the whole document, which is not a property of the entry alone. Here the
    // entry's sole fact is the `Realizes` fact, so only the document-wide
    // condition can refuse the contraction: the act names the token a second
    // time.
    let token = variable("$u");
    let mentioned = Act::mention(Operand::Value(Value::bound(
        token.clone(),
        TypeExpr::Atom(TypeAtom::UtteranceToken),
    )));
    let document = KernelDocument::new(Performable::Entry(
        TranscriptEntry::utterance(
            token.clone(),
            vec![
                Content::intrinsic(
                    Intrinsic::Realizes,
                    vec![
                        Operand::Value(Value::bound(
                            token.clone(),
                            TypeExpr::Atom(TypeAtom::UtteranceToken),
                        )),
                        Operand::Act(mentioned),
                    ],
                )
                .expect("Realizes relates a token to an act"),
            ],
        )
        .expect("an utterance boundary carries facts"),
    ))
    .expect("the document is closed and well scoped");
    let text = round_trip(&document);
    assert!(
        text.contains("(Utterance"),
        "a still-referenced token keeps its boundary:\n{text}"
    );

    // A reference from a *second* entry is the case the rule is written for,
    // and it is stronger than "does not contract": the token's binder scope is
    // its own entry's facts, so such a reference escapes as a free binder and
    // no document containing it exists at all.
    let other = variable("$v");
    let realizes = |tok: &Variable| {
        Content::intrinsic(
            Intrinsic::Realizes,
            vec![
                Operand::Value(Value::bound(
                    tok.clone(),
                    TypeExpr::Atom(TypeAtom::UtteranceToken),
                )),
                Operand::Act(simple_assertion()),
            ],
        )
        .expect("Realizes relates a token to an act")
    };
    let second = TranscriptEntry::utterance(
        other.clone(),
        vec![
            realizes(&other),
            Content::intrinsic(
                Intrinsic::SpeakerOf,
                vec![
                    Operand::Value(Value::bound(
                        token.clone(),
                        TypeExpr::Atom(TypeAtom::UtteranceToken),
                    )),
                    Operand::Value(speaker()),
                ],
            )
            .expect("SpeakerOf relates a token to its speaker"),
        ],
    )
    .expect("an utterance boundary carries facts");
    assert!(
        KernelDocument::new(Performable::Discourse(
            Discourse::sequence(vec![
                Performable::Entry(
                    TranscriptEntry::utterance(token.clone(), vec![realizes(&token)])
                        .expect("an utterance boundary carries facts"),
                ),
                Performable::Entry(second),
            ])
            .expect("a discourse sequence has at least two items"),
        ))
        .is_err(),
        "a token named by a second entry escapes its binder"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn operand_positions_require_an_explicit_singleton_lift() {
    // The kernel's operand rule admits identity, a declared upcast, and the
    // finite `Natural`-to-`Cardinal` embedding, but never the section 3.1
    // singleton lift: a crossing is a node, and eliding it is the printer's
    // decision. Predicate fills are the one disclosed exception, so this test
    // exercises the two operand paths a fill does not cover.
    let natural = TypeExpr::Atom(TypeAtom::Natural);
    let three = Value::literal(Literal::integer(3));
    let lifted = Value::intrinsic(Intrinsic::Singleton, vec![Operand::Value(three.clone())])
        .expect("Singleton lifts any value");

    // A `Let` declaration.
    assert!(
        Declaration::new(
            variable("$n"),
            referents(natural.clone()),
            Operand::Value(three.clone()),
        )
        .is_err(),
        "a declaration accepted an initializer that needs a singleton lift"
    );
    assert!(
        Declaration::new(
            variable("$n"),
            referents(natural.clone()),
            Operand::Value(lifted.clone()),
        )
        .is_ok(),
        "the same value with an explicit Singleton must declare"
    );

    // A function application argument.
    let signature = FunctionSignature::new(vec![referents(natural)], content_type());
    assert!(
        Content::apply(
            FnValue::bound(variable("$f"), signature.clone()),
            vec![Operand::Value(three)],
        )
        .is_err(),
        "an application accepted an argument that needs a singleton lift"
    );
    assert!(
        Content::apply(
            FnValue::bound(variable("$f"), signature),
            vec![Operand::Value(lifted)],
        )
        .is_ok(),
        "the same argument with an explicit Singleton must apply"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_lambda_at_a_polymorphic_position_prints_its_close() {
    // Section 2.2 elides `Close` inside a lambda body only where the enclosing
    // position declares the function type. `Mention` is polymorphic, so nothing
    // declares a `Content` result here and the surface would otherwise read as a
    // lambda returning a `PredTerm`. This is asserted against the printed text
    // because the round-trip oracle cannot see the drift: the over-elided text
    // is a fixed point of parse-then-print, so reprint stability still holds
    // while the meaning changes.
    let body = Content::close(
        PredTerm::applied(
            lexical("gerku", 1),
            vec![PlaceFill::plain(Operand::Value(Value::bound(
                variable("$x"),
                referents(entity()),
            )))],
        )
        .expect("filling gerku x1 is well typed"),
    )
    .expect("the remaining row is closeable");
    let lambda = FnValue::lambda(
        Lambda::new(
            vec![TypedParameter::new(variable("$x"), referents(entity()))],
            Operand::Content(body.clone()),
        )
        .expect("a one-parameter lambda is well formed"),
    );
    let mentioned = KernelDocument::new(Performable::Act(Act::mention(Operand::Function(lambda))))
        .expect("the document is closed and well scoped");
    let text = print_kernel_document(&mentioned, &[]);
    assert!(
        text.contains("(Close") && text.contains("(gerku $x)"),
        "a Content body at a polymorphic lambda position keeps its Close:\n{text}"
    );

    // The same lambda under a `Let` whose declared `Fn` type is printed does
    // license the elision.
    let signature = FunctionSignature::new(vec![referents(entity())], content_type());
    let declared = KernelDocument::new(Performable::Act(Act::mention(Operand::Content(
        Content::let_form(
            Let::new(
                vec![
                    Declaration::new(
                        variable("$p"),
                        signature.function_type(),
                        Operand::Function(FnValue::lambda(
                            Lambda::new(
                                vec![TypedParameter::new(variable("$x"), referents(entity()))],
                                Operand::Content(body),
                            )
                            .expect("a one-parameter lambda is well formed"),
                        )),
                    )
                    .expect("the initializer inhabits its declared type"),
                ],
                Content::apply(
                    FnValue::bound(variable("$p"), signature),
                    vec![Operand::Value(speaker())],
                )
                .expect("the call matches its signature"),
            )
            .expect("a Let block is nonempty"),
        ),
    ))))
    .expect("the document is closed and well scoped");
    let text = print_kernel_document(&declared, &[]);
    assert!(
        !text.contains("Close"),
        "a declared Fn type licenses the elision in the initializer:\n{text}"
    );
}

/// Build the one-place property `(λ ((name (Referents Entity))) (root name))`.
///
/// Each caller needs its own parameter spelling: a document introduces every
/// name exactly once, so two properties in one document cannot share one.
#[requires(!root.is_empty() && name.starts_with('$'))]
#[ensures(true)]
fn one_place_property(root: &str, name: &str) -> FnValue {
    let body = Content::close(
        PredTerm::applied(
            lexical(root, 1),
            vec![PlaceFill::plain(Operand::Value(Value::bound(
                variable(name),
                referents(entity()),
            )))],
        )
        .expect("filling x1 is well typed"),
    )
    .expect("the remaining row is closeable");
    FnValue::lambda(
        Lambda::new(
            vec![TypedParameter::new(variable(name), referents(entity()))],
            Operand::Content(body),
        )
        .expect("a one-parameter lambda is well formed"),
    )
}

/// Collapse the renderer's line breaks and indentation to single spaces.
///
/// The canonical forms quoted below are the specification's one-line spellings;
/// what these tests are about is which crossings print, not where the renderer
/// wraps.
#[requires(true)]
#[ensures(!ret.contains('\n'))]
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Assert the section 9.4 application `((Exactly 3 restriction) scope)`.
///
/// The restriction is an operand of the registered `Exactly` node; the nuclear
/// scope is an argument of the `Apply` node over the generalized quantifier it
/// returns. Those are the two construction shapes a property occupies in one
/// and the same quantifier application.
#[requires(true)]
#[ensures(true)]
fn quantified_assertion(restriction: FnValue, scope: FnValue) -> KernelDocument {
    let quantifier = FnValue::intrinsic(
        Intrinsic::Exactly,
        vec![
            Operand::Value(Value::literal(Literal::integer(3))),
            Operand::Function(restriction),
        ],
    )
    .expect("Exactly takes a count and a restriction property");
    let application = Content::apply(quantifier, vec![Operand::Function(scope)])
        .expect("a generalized quantifier applies to a nuclear scope");
    KernelDocument::new(Performable::Act(Act::assert(application)))
        .expect("the document is closed and well scoped")
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_quantifier_restriction_prints_the_canonical_property_of_section_9_4() {
    // Section 14.1 declares `Exactly`'s restriction operand a `PureProperty<T>`,
    // so section 5.2's omit-exactly-when rule licenses dropping the `Close` in
    // its body, and section 9.4's canonical document prints the property as
    // `(Exactly 3 (λ (($x ...)) (gerku $x)))`. This is asserted against the
    // printed text rather than through the round-trip oracle, which cannot see
    // this class at all: the acceptance grammar is untyped, so the spelling with
    // the explicit `Close` is just as much a fixed point of parse-then-print as
    // the canonical one, and reprint stability holds for both.
    let document = quantified_assertion(
        one_place_property("gerku", "$x"),
        one_place_property("bajra", "$y"),
    );
    let text = round_trip(&document);
    assert!(
        collapsed(&text).contains("(Exactly 3 (λ (($x (Referents Entity))) (gerku $x)))"),
        "a declared property operand prints section 9.4's canonical form:\n{text}"
    );
    assert!(
        !text.contains("Close"),
        "every position in this application declares Content:\n{text}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn both_construction_shapes_of_one_application_print_alike() {
    // The kernel has no partial application of a registered callable, so the two
    // shapes a property takes in one section 9.4 application are the `Intrinsic`
    // node's restriction operand and the `Apply` node's nuclear-scope argument.
    // Only the second used to carry a declared type, so the same property
    // printed differently depending on which shape a route built it into.
    // Swapping the two properties must not change how either one prints.
    let dog = "(λ (($x (Referents Entity))) (gerku $x))";
    let runner = "(λ (($y (Referents Entity))) (bajra $y))";
    let restricted = collapsed(&print_kernel_document(
        &quantified_assertion(
            one_place_property("gerku", "$x"),
            one_place_property("bajra", "$y"),
        ),
        &[],
    ));
    let swapped = collapsed(&print_kernel_document(
        &quantified_assertion(
            one_place_property("bajra", "$y"),
            one_place_property("gerku", "$x"),
        ),
        &[],
    ));
    for text in [&restricted, &swapped] {
        assert!(
            text.contains(dog) && text.contains(runner),
            "both shapes print the same property text:\n{text}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn predicate_application_rejects_ill_typed_and_out_of_row_fills() {
    // A place the row does not have.
    assert!(
        PredTerm::applied(
            lexical("gerku", 1),
            vec![PlaceFill::numbered(
                PositiveInteger::from_u32(4),
                Operand::Value(speaker()),
            )],
        )
        .is_err(),
        "filled a place outside the row"
    );

    // More plain fills than surviving numbered places.
    assert!(
        PredTerm::applied(
            lexical("gerku", 1),
            vec![
                PlaceFill::plain(Operand::Value(speaker())),
                PlaceFill::plain(Operand::Value(speaker())),
            ],
        )
        .is_err(),
        "over-applied a one-place relation"
    );

    // An operand of the wrong type. Section 3.1 lets a raw `T` singleton-lift
    // into a `Referents<T>` place, so the rejected case has to be a value with
    // no conversion at all into a reference place.
    assert!(
        PredTerm::applied(
            lexical("gerku", 1),
            vec![PlaceFill::plain(Operand::Content(
                Content::close(lexical("melbi", 2)).expect("the row is closeable")
            ))],
        )
        .is_err(),
        "filled a reference place with content"
    );

    // A second fill of the distinguished event place.
    let event = Operand::Value(
        Value::intrinsic(Intrinsic::Now, Vec::new()).expect("Now is a registered constant"),
    );
    assert!(
        PredTerm::applied(
            lexical("gerku", 1),
            vec![
                PlaceFill::eventuality(event.clone()),
                PlaceFill::eventuality(event),
            ],
        )
        .is_err(),
        "filled the event place twice"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn let_rec_rejects_an_initializer_that_is_not_an_inert_lambda() {
    let signature = FunctionSignature::new(vec![referents(entity())], content_type());
    let not_a_lambda = FnValue::bound(variable("$other"), signature.clone());
    assert!(
        RecursiveDeclaration::new(variable("$loop"), signature.function_type(), not_a_lambda,)
            .is_err(),
        "accepted a non-lambda LetRec initializer"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_reference_computation_is_reachable_only_through_bind() {
    // `RefComp` is not an `Operand`, so a `Refer` in ordinary value position is
    // unrepresentable rather than merely rejected. What a runtime test can show
    // is the consequence: no declaration can ever be given a `RefComp` type,
    // because no operand inhabits one.
    let computation_type = TypeExpr::ReferenceComputation(Box::new(referents(entity())));
    assert!(
        Declaration::new(variable("$r"), computation_type, Operand::Value(speaker()),).is_err(),
        "a Let declaration accepted a reference-computation type"
    );

    // A description property must select a number-neutral reference.
    assert!(
        RefComp::refer(
            Lambda::new(
                vec![TypedParameter::new(variable("$n"), entity())],
                Content::close(lexical("gerku", 1)).expect("the row is closeable"),
            )
            .expect("a one-parameter lambda is well formed"),
        )
        .is_err(),
        "Refer accepted a property of a raw entity"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn close_is_undefined_for_a_row_it_cannot_close() {
    // An unknown numbered tail is not closeable: its surviving places are not
    // yet known.
    let open_row = Row::new(
        vec![RowSlot::new(PlaceLabel::numbered(1), referents(entity()))],
        true,
    );
    assert!(
        Content::close(PredTerm::bound(variable("$mo"), open_row)).is_err(),
        "closed a row with an unknown tail"
    );

    // A surviving higher-order place receives no invented default.
    let function_row = Row::new(
        vec![RowSlot::new(
            PlaceLabel::numbered(1),
            TypeExpr::Function {
                parameters: vec![referents(entity())],
                result: Box::new(content_type()),
            },
        )],
        false,
    );
    assert!(
        Content::close(PredTerm::bound(variable("$p"), function_row)).is_err(),
        "closed a row with a surviving function place"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn closed_arity_and_selection_rules_are_enforced() {
    let assertion = Content::close(lexical("gerku", 1)).expect("the row is closeable");

    // A one-operand junction contracts to its operand and never prints.
    assert!(
        Content::junction(JunctionOp::And, vec![assertion.clone()]).is_err(),
        "accepted a one-operand junction"
    );

    // A polar answer cannot answer an open question.
    let open = Query::open(
        Lambda::new(
            vec![TypedParameter::new(variable("$x"), referents(entity()))],
            assertion.clone(),
        )
        .expect("a one-parameter lambda is well formed"),
    );
    assert!(
        Content::answer(open, AnswerSelection::polar(AnswerPolarity::Yes)).is_err(),
        "answered an open question with a polarity"
    );

    // A registered callable rejects wrong arity and wrong operand types.
    assert!(
        Content::intrinsic(Intrinsic::Among, vec![Operand::Value(speaker())]).is_err(),
        "accepted a one-operand Among"
    );
    assert!(
        Value::intrinsic(Intrinsic::Card, vec![Operand::Value(speaker())],).is_err(),
        "counted a reference rather than a set"
    );

    // A collection item must inhabit its declared element type.
    assert!(
        Value::collection(
            CollectionKind::Set,
            referents(entity()),
            vec![Value::literal(Literal::text("not a referent"))],
        )
        .is_err(),
        "accepted a mistyped collection item"
    );

    // A reference-only discourse constant is never performed.
    assert!(
        Discourse::sequence(vec![
            Performable::Act(simple_assertion()),
            Performable::Discourse(Discourse::prior()),
        ])
        .is_err(),
        "performed PriorDiscourse"
    );
    assert!(
        KernelDocument::new(Performable::Discourse(Discourse::following())).is_err(),
        "made FollowingDiscourse a document body"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn the_document_audit_rejects_unbound_and_shadowed_names() {
    // A free variable has no printable binder.
    let unbound = Content::close(
        PredTerm::applied(
            lexical("gerku", 1),
            vec![PlaceFill::plain(Operand::Value(Value::bound(
                variable("$loose"),
                referents(entity()),
            )))],
        )
        .expect("filling gerku x1 is well typed"),
    )
    .expect("the remaining row is closeable");
    assert!(
        KernelDocument::new(Performable::Act(Act::assert(unbound))).is_err(),
        "accepted a document with a free variable"
    );

    // The same name introduced twice would stop being an identity.
    let inner = Let::new(
        vec![
            Declaration::new(
                variable("$x"),
                referents(entity()),
                Operand::Value(speaker()),
            )
            .expect("the initializer inhabits its declared type"),
        ],
        Content::close(
            PredTerm::applied(
                lexical("gerku", 1),
                vec![PlaceFill::plain(Operand::Value(Value::bound(
                    variable("$x"),
                    referents(entity()),
                )))],
            )
            .expect("filling gerku x1 is well typed"),
        )
        .expect("the remaining row is closeable"),
    )
    .expect("a Let block is nonempty");
    let shadowing = Let::new(
        vec![
            Declaration::new(
                variable("$x"),
                referents(entity()),
                Operand::Value(speaker()),
            )
            .expect("the initializer inhabits its declared type"),
        ],
        Content::let_form(inner),
    )
    .expect("a Let block is nonempty");
    assert!(
        KernelDocument::new(Performable::Act(Act::assert(Content::let_form(shadowing)))).is_err(),
        "accepted a document that shadows a binder name"
    );

    // A use whose recorded type disagrees with its binder is not well scoped.
    let mistyped = Let::new(
        vec![
            Declaration::new(
                variable("$y"),
                referents(entity()),
                Operand::Value(speaker()),
            )
            .expect("the initializer inhabits its declared type"),
        ],
        Content::close(
            PredTerm::applied(
                lexical("gerku", 1),
                vec![PlaceFill::plain(Operand::Value(Value::bound(
                    variable("$y"),
                    referents(TypeExpr::Atom(TypeAtom::Eventuality)),
                )))],
            )
            .expect("filling gerku x1 is well typed"),
        )
        .expect("the remaining row is closeable"),
    )
    .expect("a Let block is nonempty");
    assert!(
        KernelDocument::new(Performable::Act(Act::assert(Content::let_form(mistyped)))).is_err(),
        "accepted a use at a type its binder did not declare"
    );
}

/// Assert a `Witnesses` run over a `$run` binder declared at `declared_type`.
#[requires(true)]
#[ensures(true)]
fn witnesses_document(declared_type: TypeExpr, initializer: Operand) -> Performable {
    let selection = RefComp::witnesses(variable("$run"), referents(entity()))
        .expect("a witness selection is a number-neutral reference");
    let retrieval = Content::bind_form(
        Bind::new(
            variable("$dogs"),
            referents(entity()),
            selection,
            Content::close(
                PredTerm::applied(
                    lexical("tatpi", 2),
                    vec![PlaceFill::plain(Operand::Value(Value::bound(
                        variable("$dogs"),
                        referents(entity()),
                    )))],
                )
                .expect("filling tatpi x1 is well typed"),
            )
            .expect("the remaining row is closeable"),
        )
        .expect("the computation produces the declared reference type"),
    );
    Performable::Act(Act::assert(Content::let_form(
        Let::new(
            vec![
                Declaration::new(variable("$run"), declared_type, initializer)
                    .expect("the initializer inhabits its declared type"),
            ],
            retrieval,
        )
        .expect("a Let block is nonempty"),
    )))
}

#[test]
#[requires(true)]
#[ensures(true)]
fn the_audit_sees_witness_runs_and_bound_relation_identities() {
    // Both positions used to record no use at all, so the audit could not check
    // them against their binders. Section 9.4 types a `Witnesses` run handle as
    // the `Content` identity of the quantifier application.
    assert!(
        KernelDocument::new(witnesses_document(
            content_type(),
            Operand::Content(Content::close(lexical("gerku", 1)).expect("the row is closeable")),
        ))
        .is_ok(),
        "a run handle declared at Content is well scoped"
    );
    assert!(
        KernelDocument::new(witnesses_document(
            referents(entity()),
            Operand::Value(speaker()),
        ))
        .is_err(),
        "accepted a Witnesses run at a type its binder did not declare"
    );

    // A bound relation identity is used at the row the signature carries.
    let declared_row = ordinary_row(1);
    let bound_relation = |row: Row| {
        Performable::Act(Act::assert(Content::let_form(
            Let::new(
                vec![
                    Declaration::new(
                        variable("$r"),
                        TypeExpr::Predicate(declared_row.clone()),
                        Operand::Predicate(lexical("gerku", 1)),
                    )
                    .expect("the initializer inhabits its declared type"),
                ],
                Content::close(PredTerm::relation(PredicateSignature::new(
                    RelationRef::Variable(variable("$r")),
                    row,
                )))
                .expect("the row is closeable"),
            )
            .expect("a Let block is nonempty"),
        )))
    };
    assert!(
        KernelDocument::new(bound_relation(ordinary_row(1))).is_ok(),
        "a bound relation used at its declared row is well scoped"
    );
    assert!(
        KernelDocument::new(bound_relation(ordinary_row(2))).is_err(),
        "accepted a bound relation used at a row its binder did not declare"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_collection_prints_its_element_type_when_the_context_does_not_determine_it() {
    // The acceptance parser has no type-position production for a collection's
    // element type, so this case is checked against the printed text directly
    // rather than through the round-trip oracle.
    let document = KernelDocument::new(Performable::Act(Act::mention(Operand::Value(
        Value::collection(CollectionKind::List, referents(entity()), vec![speaker()])
            .expect("every item inhabits the element type"),
    ))))
    .expect("the document is closed and well scoped");
    let text = print_kernel_document(&document, &[]);
    assert!(
        text.contains("(List") && text.contains("(Referents Entity)"),
        "expected an explicit element type:\n{text}"
    );

    let empty = KernelDocument::new(Performable::Act(Act::mention(Operand::Value(
        Value::collection(CollectionKind::Set, entity(), Vec::new())
            .expect("an empty collection is legal"),
    ))))
    .expect("the document is closed and well scoped");
    let text = print_kernel_document(&empty, &[]);
    assert!(
        text.contains("(Set Entity)"),
        "an empty collection always prints its element type:\n{text}"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn binding_forms_carry_the_free_binders_of_their_bodies() {
    let outer = variable("$outer");
    let body = Content::close(
        PredTerm::applied(
            lexical("klama", 5),
            vec![
                PlaceFill::plain(Operand::Value(Value::bound(
                    variable("$inner"),
                    referents(entity()),
                ))),
                PlaceFill::plain(Operand::Value(Value::bound(
                    outer.clone(),
                    referents(entity()),
                ))),
            ],
        )
        .expect("filling klama x1 and x2 is well typed"),
    )
    .expect("the remaining row is closeable");
    let lambda = Lambda::new(
        vec![TypedParameter::new(variable("$inner"), referents(entity()))],
        body,
    )
    .expect("a one-parameter lambda is well formed");
    assert!(
        lambda.free_binders().contains(&outer),
        "the enclosing binder is free in the lambda"
    );
    assert!(
        !lambda.free_binders().contains(&variable("$inner")),
        "the parameter is bound by the lambda"
    );
}
