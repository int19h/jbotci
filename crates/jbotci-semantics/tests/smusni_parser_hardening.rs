//! Bounds and totality for the parsers that can receive untrusted text.
//!
//! The smusni acceptance parser and the internal debug codec both parse text a
//! host may have received from anywhere. Neither may be driven into unbounded
//! recursion, unbounded allocation, or a panic by a hostile document.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_semantics::notation::sexpr::datum::{
    Datum, MAX_DOCUMENT_BYTES, MAX_INTEGER_DIGITS, MAX_PARSE_DEPTH, parse_datums, parse_document,
};
use jbotci_semantics::notation::sexpr::internal_raw::parse_capture;
use jbotci_semantics::notation::sexpr::{parse_v0_document, parse_v0_expression};

/// One nested list of the requested depth, wrapping the given body.
#[requires(depth > 0 && !body.is_empty())]
#[ensures(!ret.is_empty())]
fn nested(depth: usize, body: &str) -> String {
    format!("{}{body}{}", "(".repeat(depth), ")".repeat(depth))
}

#[test]
#[requires(true)]
#[ensures(true)]
fn list_nesting_is_bounded_instead_of_overflowing_the_stack() {
    // Just inside the bound parses; just outside it is a returned error, not a
    // crash. The depth limit is what makes the recursive descent safe on text
    // the host did not author.
    assert!(parse_document(&nested(MAX_PARSE_DEPTH, "A")).is_ok());
    let error = parse_document(&nested(MAX_PARSE_DEPTH + 1, "A"))
        .expect_err("nesting past the limit must be rejected");
    assert!(
        error.to_string().contains("parse depth limit"),
        "unexpected error: {error}"
    );
    // The same bound applies to the multi-datum entry point and to every parser
    // layered on the datum parser.
    assert!(parse_datums(&nested(MAX_PARSE_DEPTH + 1, "A")).is_err());
    assert!(parse_v0_expression(&nested(MAX_PARSE_DEPTH + 1, "A")).is_err());
    assert!(parse_v0_document(&nested(MAX_PARSE_DEPTH + 1, "A")).is_err());
    assert!(parse_capture(&nested(MAX_PARSE_DEPTH + 1, "A")).is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn document_size_is_bounded_before_any_allocation() {
    let oversized = format!("({})", "A ".repeat(MAX_DOCUMENT_BYTES / 2 + 1));
    assert!(oversized.len() > MAX_DOCUMENT_BYTES);
    let error = parse_document(&oversized).expect_err("an oversized document must be rejected");
    assert!(
        error.to_string().contains("parse limit"),
        "unexpected error: {error}"
    );
    assert!(parse_datums(&oversized).is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn integer_literals_have_a_bounded_digit_length() {
    // Integers are arbitrary precision, so an unbounded digit run is unbounded
    // work. The bound is generous enough that every real literal parses.
    let accepted = format!("1{}", "0".repeat(MAX_INTEGER_DIGITS - 1));
    assert_eq!(accepted.len(), MAX_INTEGER_DIGITS);
    assert!(parse_document(&accepted).is_ok());
    let rejected = format!("1{}", "0".repeat(MAX_INTEGER_DIGITS));
    assert!(
        parse_document(&rejected).is_err(),
        "a digit run past the limit must be rejected"
    );
    // The internal codec's `%id` tokens use the same bounded domain.
    let identity = format!("%1{}", "0".repeat(MAX_INTEGER_DIGITS));
    assert!(
        parse_capture(&format!(
            r#"(TypedGraph "SemanticGraph" "smusni.projection.graph.unbound-variable" (Object {identity} "SemanticGraph"))"#
        ))
        .is_err()
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_data_derived_atom_is_fallible_rather_than_panicking() {
    // `Datum::atom` is for compile-time-constant spellings. Anything derived
    // from model data, a dictionary, or untrusted input uses the fallible
    // constructor, which reports invalid spellings instead of aborting.
    for invalid in ["", " ", "a b", "(", ")", "\"quoted\""] {
        assert!(
            Datum::try_atom(invalid.to_owned()).is_err(),
            "accepted invalid atom spelling {invalid:?}"
        );
    }
    assert!(Datum::try_atom("klama".to_owned()).is_ok());
    assert!(Datum::try_atom("|abu zei sance|".to_owned()).is_ok());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn the_internal_codec_keeps_its_identity_order_and_reason_domain() {
    // These are the codec's own laws, checked through the codec's own parser:
    // the public acceptance parser rejects every one of these inputs.
    assert!(
        parse_capture(
            r#"(TypedGraph "SemanticGraph" "smusni.projection.graph.unbound-variable" (Object %1 "SemanticGraph" (Field "self" (Ref %1))))"#
        )
        .is_ok()
    );
    for malformed in [
        // `%2` before `%1` breaks depth-first first-definition order.
        r#"(TypedGraph "SemanticGraph" "smusni.projection.graph.unbound-variable" (Object %2 "SemanticGraph"))"#,
        // The declared root type must match the raw root's type name.
        r#"(TypedGraph "Other" "smusni.projection.graph.unbound-variable" (Object %1 "SemanticGraph"))"#,
        // The reason must be in the closed registered namespace.
        r#"(Fallback Content "not registered" (Object %1 "Root"))"#,
        r#"(Fallback Content "smusni." (RawNull))"#,
        r#"(Fallback Content "smusni.Bad" (RawNull))"#,
        // A reference with no definition, a redefinition, an unknown raw form,
        // an over-long form, and a non-raw payload are all rejected.
        r#"(Fallback Content "smusni.projection.math.power" (Ref %1))"#,
        r#"(Fallback Content "smusni.projection.math.power" (Object %1 "Root" (Field "x" (Object %1 "Again"))))"#,
        r#"(Fallback Content "smusni.projection.math.power" (UnknownRaw))"#,
        r#"(Fallback Content "smusni.projection.math.power" (RawNull extra))"#,
        r#"(Fallback Content "smusni.projection.math.power" "raw")"#,
        // A compact document is not a capture root.
        "(Smusni 0 (Assert (klama This)))",
    ] {
        assert!(
            parse_capture(malformed).is_err(),
            "accepted malformed capture {malformed}"
        );
    }
}
