//! The web gentufa entry the server calls returns a typed interruption.
//!
//! A flag that is already set must not come back as a syntax or morphology
//! failure inside [`GentufaWebResult`]. The same sentence with a clear flag
//! still parses.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::{CancellationSource, assert_exit_check};

use crate::{GentufaWebOptions, GentufaWebRequest, parse_gentufa_for_web_with_limits};

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
    let clear = parse_gentufa_for_web_with_limits(
        &request,
        None,
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("a clear flag parses");
    assert!(!matches!(clear, crate::GentufaWebResult::Error(_)));

    let source = CancellationSource::new();
    source.cancel();
    let error = parse_gentufa_for_web_with_limits(&request, None, &source.token())
        .expect_err("a set flag must not return a page");
    assert!(error.is_interrupted());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_inside_web_svg_export() {
    use crate::{
        GentufaExportFormat, GentufaScript, GentufaSuccess, GentufaWebResult,
        render_gentufa_blocks_web_export_with_limits,
    };
    let request = GentufaWebRequest {
        text: "mi klama".to_owned(),
        options: GentufaWebOptions::default(),
    };
    let GentufaWebResult::Success(GentufaSuccess { blocks_layout, .. }) =
        parse_gentufa_for_web_with_limits(
            &request,
            None,
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("parse")
    else {
        panic!("expected a layout");
    };
    let limits = bityzba::new!(jbotci_gentufa::GentufaRenderLimits {
        max_blocks: 64,
        max_columns: 64,
        max_pixels: 4_000_000,
        max_bytes: 4_000_000,
    });
    let svg = assert_exit_check(
        |token| {
            render_gentufa_blocks_web_export_with_limits(
                &blocks_layout,
                false,
                GentufaScript::Latin,
                GentufaExportFormat::Svg,
                Some(limits),
                token,
            )
        },
        |error| error.is_interrupted(),
    );
    assert!(svg.content_type.contains("svg"));
    let text = std::str::from_utf8(&svg.bytes).expect("svg export is utf-8");
    assert!(text.contains("<svg"));
}
