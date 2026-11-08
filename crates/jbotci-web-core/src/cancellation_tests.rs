//! The web gentufa entry the server calls returns a typed interruption.
//!
//! A flag that is already set must not come back as a syntax or morphology
//! failure inside [`GentufaWebResult`]. The same sentence with a clear flag
//! still parses.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;

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

/// Clear loads of the web SVG export of "mi klama" before the load that
/// aborts. Release measurement: 16 clear loads, trip 15. The export is given
/// diagram limits, so the token reaches the diagram loops and not only the
/// web entry check.
const WEB_SVG_EXPORT_LATE_TRIP: usize = 15;

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
    let run = |trip: usize| {
        let token = jbotci_interrupt::CancellationToken::trip_after_checks(trip);
        render_gentufa_blocks_web_export_with_limits(
            &blocks_layout,
            false,
            GentufaScript::Latin,
            GentufaExportFormat::Svg,
            Some(limits),
            &token,
        )
    };
    let error = run(WEB_SVG_EXPORT_LATE_TRIP).expect_err("the late trip stops the export");
    assert!(error.is_interrupted());
    run(WEB_SVG_EXPORT_LATE_TRIP + 1).expect("the next index finishes the export");
}
