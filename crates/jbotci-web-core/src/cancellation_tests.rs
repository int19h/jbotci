//! The web gentufa entry the server calls returns a typed interruption.
//!
//! A flag that is already set must not come back as a syntax or morphology
//! failure inside [`GentufaWebResult`]. The same sentence with a clear flag
//! still parses.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;

use crate::{
    GentufaWebOptions, GentufaWebRequest, parse_gentufa_for_web_with_limits_and_cancellation,
};

#[test]
#[requires(true)]
#[ensures(true)]
fn cancellation_is_a_web_interruption_and_not_a_parse_error() {
    // `GentufaWebRequest` carries only the audited no-op invariant, so it has
    // no generated constructor.
    let request = GentufaWebRequest {
        text: "mi klama le zarci".to_owned(),
        options: GentufaWebOptions::default(),
    };
    let clear = parse_gentufa_for_web_with_limits_and_cancellation(
        &request,
        None,
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("a clear flag parses");
    assert!(!matches!(clear, crate::GentufaWebResult::Error(_)));

    let source = CancellationSource::new();
    source.cancel();
    let error = parse_gentufa_for_web_with_limits_and_cancellation(&request, None, &source.token())
        .expect_err("a set flag must not return a page");
    assert!(error.is_interrupted());
}
