//! Generated excerpt sweep.
//!
//! An uncut excerpt must equal the baseline stderr after the deliberate
//! control escape. The renderer leaves a C1 control unchanged. This copy
//! writes `\u{hex}` so the byte cannot start a terminal sequence. Tab, CR,
//! and LF stay as they are. A source that contains any other C0, DEL, or C1
//! control is not a parity failure.
//!
//! A cut is the renderer's prepared-boundary flag. The sweep does not search
//! the rendered text for `...` or `…` to decide that. Width and mark checks
//! read the drawn rows, and a mark check runs only when the renderer cut the
//! excerpt. A fold mark is not a horizontal cut.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_cli::test_harness::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use unicode_width::UnicodeWidthStr;

#[invariant(!name.is_empty())]
struct SweepCase {
    name: String,
    text: String,
}

#[invariant(true)]
struct SweepTotals {
    inputs: usize,
    parity_failures: usize,
    width_failures: usize,
    mark_failures: usize,
    control_leaks: usize,
    failed: Vec<String>,
    notes: Vec<String>,
}

#[test]
#[requires(true)]
#[ensures(true)]
fn excerpt_sweep_matches_an_uncut_baseline_and_keeps_cut_lines_narrow() {
    let cases = sweep_cases();
    assert!(
        cases.len() >= 307,
        "sweep has {} base cases, the wide set has 307",
        cases.len()
    );
    let baseline = baseline_binary();
    let workers = 8usize;
    let chunk = cases.len().div_ceil(workers);
    let baseline_ref = baseline.as_deref();
    let parts = thread::scope(|scope| {
        let mut joins = Vec::new();
        for piece in cases.chunks(chunk) {
            joins.push(scope.spawn(move || run_chunk(piece, baseline_ref)));
        }
        joins
            .into_iter()
            .map(|join| join.join().expect("sweep worker"))
            .collect::<Vec<_>>()
    });
    let mut totals = SweepTotals {
        inputs: 0,
        parity_failures: 0,
        width_failures: 0,
        mark_failures: 0,
        control_leaks: 0,
        failed: Vec::new(),
        notes: Vec::new(),
    };
    for part in parts {
        totals.inputs += part.inputs;
        totals.parity_failures += part.parity_failures;
        totals.width_failures += part.width_failures;
        totals.mark_failures += part.mark_failures;
        totals.control_leaks += part.control_leaks;
        for tag in part.failed {
            remember(&mut totals.failed, &tag);
        }
        for note in part.notes {
            if totals.notes.len() < 12 {
                totals.notes.push(note);
            }
        }
    }
    let summary = format!(
        "SWEEP inputs={} parity_failures={} width_failures={} mark_failures={} control_leaks={} baseline={} cases={}",
        totals.inputs,
        totals.parity_failures,
        totals.width_failures,
        totals.mark_failures,
        totals.control_leaks,
        if baseline.is_some() { "yes" } else { "no" },
        cases.len(),
    );
    eprintln!("{summary}");
    if let Some(path) = std::env::var_os("EXCERPT_SWEEP_LOG") {
        let mut body = summary.clone();
        body.push('\n');
        body.push_str("failures: ");
        body.push_str(&totals.failed.join(", "));
        body.push('\n');
        for note in &totals.notes {
            body.push_str(note);
            body.push('\n');
        }
        std::fs::write(path, body).expect("sweep log");
    }
    assert!(
        totals.parity_failures == 0
            && totals.width_failures == 0
            && totals.mark_failures == 0
            && totals.control_leaks == 0,
        "{summary}\nfailures: {}\n{}",
        totals.failed.join(", "),
        totals.notes.join("\n")
    );
}

#[requires(true)]
#[ensures(true)]
fn run_chunk(cases: &[SweepCase], baseline: Option<&Path>) -> SweepTotals {
    let mut totals = SweepTotals {
        inputs: 0,
        parity_failures: 0,
        width_failures: 0,
        mark_failures: 0,
        control_leaks: 0,
        failed: Vec::new(),
        notes: Vec::new(),
    };
    for case in cases {
        let controlled = source_has_terminal_control(&case.text);
        let mut cut_for = [false, false];
        for (index, ascii) in [false, true].into_iter().enumerate() {
            cut_for[index] = gentufa_excerpt_is_cut(&case.text, ascii);
        }
        for (index, ascii) in [false, true].into_iter().enumerate() {
            let cut = cut_for[index];
            for color in [false, true] {
                totals.inputs += 1;
                let mode = mode_name(ascii, color);
                let tag = format!("{} {mode}", case.name);
                let head = render_head(&case.text, ascii, color);
                if control_leak(&head, color) {
                    totals.control_leaks += 1;
                    remember(&mut totals.failed, &tag);
                    note(
                        &mut totals.notes,
                        format!("{tag}: raw terminal control in head stderr"),
                    );
                }
                let head_plain = strip_ansi(&head);
                let head_wide = row_over_width(&head_plain);
                let head_mark = cut && mark_anomaly(&head_plain, ascii);
                let Some(binary) = baseline else {
                    continue;
                };
                let main_text = render_baseline(binary, &case.text, ascii, color);
                let main_plain = strip_ansi(&main_text);
                // Uncut text with no deliberate control escape must match.
                if !controlled && !cut && head != main_text {
                    totals.parity_failures += 1;
                    remember(&mut totals.failed, &tag);
                    note(
                        &mut totals.notes,
                        format!(
                            "{tag}: uncut mismatch\nHEAD:\n{}\nMAIN:\n{}",
                            clip(&head),
                            clip(&main_text)
                        ),
                    );
                }
                if head_wide && !row_over_width(&main_plain) {
                    totals.width_failures += 1;
                    remember(&mut totals.failed, &tag);
                    note(&mut totals.notes, format!("{tag}: head-only width"));
                }
                let main_mark = cut && mark_anomaly(&main_plain, ascii);
                if head_mark && !main_mark {
                    totals.mark_failures += 1;
                    remember(&mut totals.failed, &tag);
                    note(&mut totals.notes, format!("{tag}: head-only mark"));
                }
            }
        }
    }
    totals
}

#[requires(true)]
#[ensures(true)]
fn note(notes: &mut Vec<String>, text: String) {
    if notes.len() < 8 {
        notes.push(text);
    }
}

#[requires(true)]
#[ensures(true)]
fn remember(failed: &mut Vec<String>, tag: &str) {
    if !failed.iter().any(|item| item == tag) {
        failed.push(tag.to_owned());
    }
}

#[requires(!name.is_empty())]
#[ensures(!ret.name.is_empty())]
fn sweep_case(name: String, text: String) -> SweepCase {
    new!(SweepCase { name, text })
}

#[requires(true)]
#[ensures(ret.len() >= 307)]
fn sweep_cases() -> Vec<SweepCase> {
    let mut cases = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |family: &str, name: &str, text: String| {
        if seen.insert(text.clone()) {
            cases.push(sweep_case(format!("{family}/{name}"), text));
        }
    };

    for lines in 1..=5 {
        for indent in [0usize, 15, 27] {
            let pad = " ".repeat(indent);
            let text = if lines == 1 {
                format!("{pad}mi nelci lo ku ku")
            } else {
                let mut parts = vec![format!("{pad}mi nelci lo")];
                for _ in 0..lines - 2 {
                    parts.push(format!("{pad}barda"));
                }
                parts.push(format!("{pad}ku ku"));
                parts.join("\n")
            };
            add("lines", &format!("lines_{lines}_ind{indent}"), text);
        }
    }

    let middles = [
        ("blank", ""),
        ("spaces", "   "),
        ("comment", "//x"),
        ("doc", "/// note"),
        ("paren", "("),
        ("close_paren", ")"),
        ("brace", "{"),
        ("close_brace", "}"),
        ("bracket", "["),
        ("close_bracket", "]"),
    ];
    for (name, middle) in middles {
        add(
            "middle",
            &format!("zoi_mid_{name}"),
            format!("mi cusku zoi gy. abc\n{middle}\nghi"),
        );
        add(
            "middle",
            &format!("zoi_mid_{name}_long"),
            format!("mi cusku zoi gy. {}\n{middle}\nghi", "a".repeat(90)),
        );
    }

    for gap in 1..=3 {
        add(
            "gaps",
            &format!("gap_{gap}"),
            format!("mi nelci lo\n{}\nku ku", "\n".repeat(gap - 1)),
        );
        add(
            "gaps",
            &format!("gap_{gap}_ind15"),
            format!(
                "mi nelci lo\n{}\n{}ku ku",
                "\n".repeat(gap - 1),
                " ".repeat(15)
            ),
        );
    }

    add("point", "eof_point", "mi nelci lo".to_owned());
    add("point", "eof_after_newline", "mi nelci lo\n".to_owned());
    add(
        "point",
        "line_end_2line",
        "mi nelci lo barda\nku ku".to_owned(),
    );
    add(
        "point",
        "line_end_3line",
        "mi nelci lo barda\nzarci\nku ku".to_owned(),
    );
    add(
        "point",
        "zoi_line_end",
        "mi cusku zoi gy. abc\ndef".to_owned(),
    );
    add("point", "mid_line", "mi tavla kkk do".to_owned());
    add("point", "simple_ku_ku", "mi nelci lo ku ku".to_owned());
    add("point", "warning_fihoi", "mi klama fi'oi broda".to_owned());
    add("point", "lu_2line", "mi cusku lu\ndo citka".to_owned());
    add(
        "point",
        "lu_2line_trailing_nl",
        "mi cusku lu\ndo citka\n".to_owned(),
    );

    for count in [20usize, 25, 30, 40, 60, 100, 150, 200] {
        add(
            "longline",
            &format!("barda_{count}"),
            format!("mi nelci lo {}ku ku", "barda ".repeat(count)),
        );
    }
    for count in [120usize, 200, 400] {
        add("longline", &format!("keys_{count}"), "k".repeat(count));
    }
    for (index, suffix) in [
        "lo",
        "mi cusku lu",
        "mi cusku zoi gy. def",
        "mi cusku zoi gy. 漢字",
        "mi sei broda",
        "mi to broda",
    ]
    .into_iter()
    .enumerate()
    {
        add(
            "longline",
            &format!("left_cut_{index}"),
            format!("{}{suffix}", "mi klama lo zarci .i ".repeat(5)),
        );
    }
    add(
        "longline",
        "no_left_cut_barda30",
        format!("lo {}ku ku", "barda ".repeat(30)),
    );
    add(
        "longline",
        "two_labels_long",
        format!("lo {}ku ku", "barda ".repeat(15)),
    );

    for count in [10usize, 30, 40, 60, 80, 120, 200] {
        add(
            "han",
            &format!("han_zoi_{count}"),
            format!("mi nelci lo zoi gy. {} .gy ku ku", "漢".repeat(count)),
        );
    }
    for count in [30usize, 60, 120] {
        add(
            "han",
            &format!("han_nelci_{count}"),
            format!("mi nelci lo {} ku ku", "漢".repeat(count)),
        );
    }
    add(
        "han",
        "han_ascii_mix",
        format!("mi nelci lo zoi gy. {} .gy ku ku", "漢a".repeat(60)),
    );

    add(
        "combining",
        "comb_short",
        "mi cusku zoi gy. a\u{0301}\u{0308} \u{1f469}\u{200d}\u{1f52c} .gy ku".to_owned(),
    );
    add(
        "combining",
        "comb_mid",
        format!("mi cusku zoi gy. {} .gy ku", "a\u{0301}".repeat(40)),
    );
    add(
        "combining",
        "comb_long",
        format!("mi cusku zoi gy. {} .gy ku", "a\u{0301}".repeat(120)),
    );
    add(
        "combining",
        "zwj_long",
        format!(
            "mi cusku zoi gy. {} .gy ku",
            "\u{1f469}\u{200d}\u{1f52c}".repeat(40)
        ),
    );
    add(
        "combining",
        "comb_nelci",
        "mi nelci lo a\u{0301}\u{0308} ku ku".to_owned(),
    );

    add("tabs", "tabs_short", "mi\tnelci\tlo ku ku".to_owned());
    add(
        "tabs",
        "seven_tabs",
        format!("{}mi nelci lo\nku ku", "\t".repeat(7)),
    );
    add(
        "tabs",
        "tab_long",
        format!("mi nelci lo\t{}ku ku", "barda ".repeat(30)),
    );
    add("tabs", "tab_before_point", "mi nelci\tlo".to_owned());
    add(
        "tabs",
        "tab_middle_line",
        "mi nelci lo\n\t\nku ku".to_owned(),
    );

    for code in (0x00..0x09).chain(0x0B..0x20) {
        add(
            "c0",
            &format!("c0_{code:02x}"),
            format!("mi nelci lo {} ku", char::from(code as u8)),
        );
    }
    add("c0", "del_7f", "mi nelci lo \u{7f} ku".to_owned());
    add(
        "c0",
        "c0_cluster",
        "mi nelci lo \u{7}\u{1b}\u{7f} ku".to_owned(),
    );
    add(
        "c0",
        "c0_in_cut_run",
        format!(
            "mi cusku zoi gy. {}\nab\u{1}\u{7}\u{1b}\u{7f}cd",
            "a".repeat(90)
        ),
    );

    for code in [0x80u32, 0x85, 0x9B, 0x9F] {
        let ch = char::from_u32(code).expect("c1");
        add(
            "c1",
            &format!("c1_zoi_{code:02x}"),
            format!("mi cusku zoi gy. {ch} .gy ku"),
        );
    }
    for code in [0x80u32, 0x85, 0x9F] {
        let ch = char::from_u32(code).expect("c1");
        add(
            "c1",
            &format!("c1_nelci_{code:02x}"),
            format!("mi nelci lo {ch} ku"),
        );
    }
    add(
        "c1",
        "c1_cluster_zoi",
        "mi cusku zoi gy. \u{85}\u{9f} .gy ku".to_owned(),
    );
    add(
        "c1",
        "c1_in_cut_run",
        format!("mi cusku zoi gy. {}\ndef\u{85}\u{9f}", "a".repeat(90)),
    );
    add(
        "c1",
        "c1_long_line",
        format!("mi nelci lo \u{85}{}ku ku", "barda ".repeat(30)),
    );

    for code in [
        0x202Au32, 0x202B, 0x202C, 0x202D, 0x202E, 0x2066, 0x2067, 0x2068, 0x2069,
    ] {
        let ch = char::from_u32(code).expect("bidi");
        add(
            "bidi",
            &format!("bidi_{code:04x}"),
            format!("mi cusku zoi gy. a{ch} .gy ku"),
        );
    }
    add(
        "bidi",
        "bidi_long",
        format!(
            "mi cusku zoi gy. a{}\u{202e}\u{202e}\u{202e}a .gy ku",
            "a".repeat(90)
        ),
    );

    add("crlf", "crlf_2line", "mi nelci lo\r\nku ku\r\n".to_owned());
    add("crlf", "crlf_bridge", "mi nelci lo\r\n\r\nku ku".to_owned());
    add("crlf", "lone_cr", "lo\rku ku".to_owned());
    add("crlf", "cr_at_eof", "mi nelci lo\r".to_owned());
    add(
        "crlf",
        "crlf_long",
        format!("mi cusku zoi gy. {}\r\ndef\r\n", "a".repeat(90)),
    );
    add(
        "crlf",
        "mixed_endings",
        "mi nelci lo\r\nbarda\nku ku\r\n".to_owned(),
    );
    add("crlf", "cr_then_crlf", "mi nelci lo\r\r\nku ku".to_owned());
    add(
        "crlf",
        "cr_only_separators",
        "mi nelci lo\rku ku\rdo".to_owned(),
    );

    for indent in [1usize, 8, 15, 27, 28, 30, 40] {
        let pad = " ".repeat(indent);
        add(
            "indent",
            &format!("ind{indent}_nelci"),
            format!("{pad}mi nelci lo\n{pad}ku ku"),
        );
        add(
            "indent",
            &format!("ind{indent}_zoi_long"),
            format!("{pad}mi cusku zoi gy. {}\n{pad}def", "a".repeat(90)),
        );
    }
    add(
        "indent",
        "ind27_first_line_only",
        format!("{}mi nelci lo\nku ku", " ".repeat(27)),
    );
    add(
        "c0",
        "control_short_committed",
        "mi nelci lo \u{7}ku".to_owned(),
    );

    for count in [100usize, 101, 105, 120] {
        add(
            "gutter3",
            &format!("g3_{count}"),
            format!("mi nelci lo\n{}ku ku", "barda\n".repeat(count - 2)),
        );
    }
    add(
        "gutter3",
        "g3_far_end",
        format!(
            "mi cusku zoi gy. {}\n{}\ndef",
            "a".repeat(90),
            "\n".repeat(99)
        ),
    );
    add(
        "gutter3",
        "g3_long_last",
        format!(
            "mi cusku zoi gy. abc\n{}{}def",
            "mi klama\n".repeat(99),
            "barda ".repeat(25)
        ),
    );
    add(
        "gutter3",
        "g3_long_first",
        format!(
            "mi cusku zoi gy. {}\n{}def",
            "a".repeat(90),
            "mi klama\n".repeat(99)
        ),
    );
    add(
        "gutter3",
        "g3_indent20",
        format!(
            "{}mi nelci lo\n{}{}ku ku",
            " ".repeat(20),
            format!("{}barda\n", " ".repeat(20)).repeat(103),
            " ".repeat(20)
        ),
    );

    add(
        "mixed",
        "seam_raw_then_prepared",
        format!(
            "mi cusku zoi gy. abc\n{}{}def",
            "mi klama\n".repeat(10),
            "barda ".repeat(25)
        ),
    );
    add(
        "mixed",
        "seam_prepared_then_raw",
        format!(
            "mi cusku zoi gy. {}\n{}def",
            "a".repeat(90),
            "mi klama\n".repeat(10)
        ),
    );
    add(
        "mixed",
        "seam_3digit_raw_prepared",
        format!(
            "mi nelci lo\n{}{}ku ku",
            "barda\n".repeat(102),
            "barda ".repeat(25)
        ),
    );
    add(
        "mixed",
        "seam_3digit_prepared_raw",
        format!(
            "{}mi nelci lo\n{}ku ku",
            " ".repeat(30),
            "barda\n".repeat(102)
        ),
    );
    add(
        "mixed",
        "seam_ind15",
        format!(
            "{}mi nelci lo\n{}{}{}ku ku",
            " ".repeat(15),
            format!("{}barda\n", " ".repeat(15)).repeat(10),
            " ".repeat(15),
            "barda ".repeat(25)
        ),
    );
    add(
        "mixed",
        "seam_blank_98",
        format!(
            "mi cusku zoi gy. {}\n{}\ndef",
            "a".repeat(90),
            "\n".repeat(98)
        ),
    );

    add("dots", "dots_end_57", format!("kkk {}...", "a".repeat(50)));
    add(
        "dots",
        "ellipsis_end",
        format!("kkk zoi gy. {}\u{2026}", "a".repeat(40)),
    );
    add("dots", "dots_end_short", "mi tavla kkk do ...".to_owned());
    add("dots", "dots_mid", "mi tavla ... do kkk".to_owned());
    add(
        "dots",
        "dots_inside_zoi",
        "mi cusku zoi gy. a...b .gy ku".to_owned(),
    );
    add(
        "dots",
        "ellipsis_inside_zoi",
        "mi cusku zoi gy. \u{2026} \u{2026} .gy ku".to_owned(),
    );
    add("dots", "dots_lo", "lo ... ku ku".to_owned());
    add("dots", "dots_nelci", "mi nelci lo ... ku ku".to_owned());
    add("dots", "dots_edge_71", format!("{}...", "k".repeat(68)));
    add("dots", "ellipsis_x30", "\u{2026}".repeat(30));
    add("dots", "dots_x20", "...".repeat(20));
    add(
        "dots",
        "dots_mixed",
        "mi tavla a\u{2026}b...c\u{2025}d kkk".to_owned(),
    );

    for count in 8..16 {
        add(
            "border",
            &format!("border_barda_{count}"),
            format!("mi nelci lo {}ku ku", "barda ".repeat(count)),
        );
    }
    for width in [68usize, 70, 72, 74, 76, 78, 80, 84, 88, 96] {
        add("border", &format!("border_k_{width}"), "k".repeat(width));
    }
    for count in 9..13 {
        add(
            "border",
            &format!("border_2line_{count}"),
            format!("mi nelci lo\n{}ku ku", "barda ".repeat(count)),
        );
    }

    add(
        "multiline",
        "zoi_4line",
        "mi cusku zoi gy. a\nb\nc\nd".to_owned(),
    );
    add(
        "multiline",
        "zoi_7line",
        format!("mi cusku zoi gy. a\n{}c", "b\n".repeat(5)),
    );
    add(
        "multiline",
        "zoi_8line_fold",
        format!("mi cusku zoi gy. a\n{}c", "b\n".repeat(6)),
    );
    add(
        "multiline",
        "zoi_10line_fold",
        format!("mi cusku zoi gy. a\n{}c", "b\n".repeat(8)),
    );
    add(
        "multiline",
        "lu_9line",
        format!("mi cusku lu\n{}", "mi klama\n".repeat(8)),
    );
    add(
        "multiline",
        "long_first_short_rest",
        format!("mi cusku zoi gy. {}\nmi klama\ndef", "a".repeat(90)),
    );
    add(
        "multiline",
        "short_first_long_second",
        format!("mi cusku zoi gy. abc\n{}\ndef", "barda ".repeat(30)),
    );
    add(
        "multiline",
        "all_three_long",
        format!(
            "mi cusku zoi gy. {}\n{}\n{}",
            "a".repeat(90),
            "b".repeat(90),
            "c".repeat(90)
        ),
    );
    add(
        "multiline",
        "trailing_newline_end",
        format!("mi cusku zoi gy. {}\ndef\n", "a".repeat(90)),
    );
    add(
        "multiline",
        "nelci_5line_long_mid",
        format!("mi nelci lo\n{}\nbarda\nbarda\nku ku", "barda ".repeat(30)),
    );
    add(
        "multiline",
        "zoi_long_line2_caret",
        format!(
            "mi nelci lo zoi gy. \u{7}\u{1b} .gy\n{}ku ku",
            "barda ".repeat(25)
        ),
    );
    add(
        "multiline",
        "controls_uncut_line2",
        format!(
            "mi cusku zoi gy. {}\nab\u{1}\u{7}\u{1b}\u{7f}\u{85}\u{9b}\u{9f}cd",
            "a".repeat(90)
        ),
    );

    for count in 0..7 {
        add(
            "eof",
            &format!("zoi_gy_nl{count}"),
            format!("zoi gy{}", "\n".repeat(count)),
        );
        add(
            "eof",
            &format!("zoi_gy_nl{count}_x"),
            format!("zoi gy{}x", "\n".repeat(count)),
        );
        add(
            "eof",
            &format!("zoi_gy_x_nl{count}"),
            format!("zoi gy\nx{}", "\n".repeat(count)),
        );
        add(
            "eof",
            &format!("cusku_abc_nl{count}"),
            format!("mi cusku zoi gy. abc{}", "\n".repeat(count)),
        );
        add(
            "eof",
            &format!("cusku_abc_nl{count}_def"),
            format!("mi cusku zoi gy. abc{}def", "\n".repeat(count)),
        );
        add(
            "eof",
            &format!("nelci_lo_nl{count}"),
            format!("mi nelci lo{}", "\n".repeat(count)),
        );
        add(
            "eof",
            &format!("nelci_lo_nl{count}_kuku"),
            format!("mi nelci lo{}ku ku", "\n".repeat(count)),
        );
    }
    for (base_name, base) in [("nelci", "mi nelci lo"), ("cusku", "mi cusku zoi gy. abc")] {
        for (name, middle) in [
            ("blank", ""),
            ("comment", "//x"),
            ("paren", "("),
            ("brace", "{"),
            ("spaces", "   "),
        ] {
            for count in 0..3 {
                add(
                    "eof",
                    &format!("{base_name}_mid_{name}_nl{count}"),
                    format!("{base}\n{middle}{}", "\n".repeat(count)),
                );
            }
        }
    }
    for gap in 1..=3 {
        add(
            "eof",
            &format!("nelci_gap_eof_{gap}"),
            format!("mi nelci lo\n{}", "\n".repeat(gap)),
        );
        add(
            "eof",
            &format!("cusku_gap_eof_{gap}"),
            format!("mi cusku zoi gy. abc\n{}", "\n".repeat(gap)),
        );
    }

    for count in 0..5 {
        add(
            "eof_other",
            &format!("lu_nl{count}"),
            format!("mi cusku lu{}", "\n".repeat(count)),
        );
        add(
            "eof_other",
            &format!("sei_nl{count}"),
            format!("mi sei broda{}", "\n".repeat(count)),
        );
        add(
            "eof_other",
            &format!("to_nl{count}"),
            format!("mi to broda{}", "\n".repeat(count)),
        );
    }
    for count in 0..3 {
        add(
            "eof_other",
            &format!("warning_nl{count}"),
            format!("mi klama fi'oi broda{}", "\n".repeat(count)),
        );
    }

    cases
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn mode_name(ascii: bool, color: bool) -> &'static str {
    match (ascii, color) {
        (false, false) => "unicode-plain",
        (false, true) => "unicode-color",
        (true, false) => "ascii-plain",
        (true, true) => "ascii-color",
    }
}

#[requires(true)]
#[ensures(true)]
fn baseline_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("JBOTCI_BASELINE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let fallback = Path::new("/build/jbotci/target/lead-probe/release/jbotci");
    fallback.is_file().then(|| fallback.to_path_buf())
}

#[requires(true)]
#[ensures(true)]
fn render_head(text: &str, ascii: bool, color: bool) -> String {
    let color_arg = if color {
        "--color=always"
    } else {
        "--color=never"
    };
    let mut args = vec!["jbotci", color_arg, "gentufa"];
    if ascii {
        args.push("--ascii");
    }
    let cli = Cli::try_parse_from(args).expect("sweep args parse");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    run_cli_with_stdin_and_width(
        cli,
        &mut stdout,
        &mut stderr,
        CliColorPolicy::never(),
        80,
        text,
    )
    .expect("head render");
    String::from_utf8(stderr).expect("head stderr is utf-8")
}

#[requires(binary.is_file())]
#[ensures(true)]
fn render_baseline(binary: &Path, text: &str, ascii: bool, color: bool) -> String {
    let mut command = Command::new(binary);
    command.arg(if color {
        "--color=always"
    } else {
        "--color=never"
    });
    command.arg("gentufa");
    if ascii {
        command.arg("--ascii");
    }
    command.stdin(Stdio::piped());
    command.stdout(Stdio::null());
    command.stderr(Stdio::piped());
    command.env_remove("FORCE_COLOR");
    command.env_remove("NO_COLOR");
    command.env_remove("CLICOLOR");
    command.env_remove("CLICOLOR_FORCE");
    let mut child = command.spawn().expect("baseline render");
    {
        let mut stdin = child.stdin.take().expect("baseline stdin");
        use std::io::Write;
        stdin.write_all(text.as_bytes()).expect("baseline write");
    }
    let output = child.wait_with_output().expect("baseline wait");
    String::from_utf8(output.stderr).expect("baseline stderr is utf-8")
}

/// C0, DEL, and C1 other than CR, LF, and tab.
#[requires(true)]
#[ensures(true)]
fn source_has_terminal_control(text: &str) -> bool {
    text.chars().any(is_terminal_control)
}

#[requires(true)]
#[ensures(true)]
fn is_terminal_control(character: char) -> bool {
    let code = character as u32;
    character != '\n'
        && character != '\r'
        && character != '\t'
        && (code <= 0x1F || code == 0x7F || (0x80..=0x9F).contains(&code))
}

#[requires(true)]
#[ensures(true)]
fn control_leak(raw: &str, allow_ansi: bool) -> bool {
    let mut chars = raw.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && allow_ansi && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next == 'm' || (!next.is_ascii_digit() && next != ';') {
                    break;
                }
            }
            continue;
        }
        if is_terminal_control(character) {
            return true;
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn row_over_width(plain: &str) -> bool {
    plain.lines().any(|line| {
        (numbered_body(line).is_some() || is_mark_row(line)) && display_columns(line) > 80
    })
}

#[requires(true)]
#[ensures(true)]
fn mark_anomaly(plain: &str, ascii: bool) -> bool {
    let mark = if ascii { "..." } else { "…" };
    let lines: Vec<&str> = plain.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let Some(body_at) = numbered_body_index(line) else {
            continue;
        };
        for column in interior_mark_columns(&line[body_at..], mark) {
            let absolute = display_columns(&line[..body_at]) + column;
            let Some(next) = lines.get(index + 1) else {
                return true;
            };
            if numbered_body(next).is_some() || !is_gutter_row(next) {
                return true;
            }
            if !mark_columns(next, mark).contains(&absolute) {
                return true;
            }
        }
    }
    for (index, line) in lines.iter().enumerate() {
        if !is_mark_row(line) {
            continue;
        }
        let Some(previous) = index.checked_sub(1).and_then(|at| lines.get(at)) else {
            continue;
        };
        if numbered_body(previous).is_none() {
            continue;
        }
        let previous_marks = mark_columns(previous, mark);
        for column in interior_mark_columns(line, mark) {
            if !previous_marks.contains(&column) {
                return true;
            }
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn numbered_body(line: &str) -> Option<&str> {
    numbered_body_index(line).map(|index| &line[index..])
}

#[requires(true)]
#[ensures(true)]
fn numbered_body_index(line: &str) -> Option<usize> {
    let trimmed_at = line.len() - line.trim_start().len();
    let trimmed = &line[trimmed_at..];
    let digits = trimmed
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits == 0 {
        return None;
    }
    let mut index = trimmed_at + digits;
    while line.as_bytes().get(index) == Some(&b' ') {
        index += 1;
    }
    let rest = line.get(index..)?;
    if rest.starts_with('│') {
        return Some(index + '│'.len_utf8());
    }
    if rest.starts_with('|') {
        return Some(index + 1);
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn is_gutter_row(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('│') || trimmed.starts_with('|')
}

#[requires(true)]
#[ensures(true)]
fn is_mark_row(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(body) = trimmed
        .strip_prefix('│')
        .or_else(|| trimmed.strip_prefix('|'))
    else {
        return false;
    };
    !body
        .chars()
        .any(|character| character.is_ascii_alphabetic())
}

#[requires(!mark.is_empty())]
#[ensures(true)]
fn interior_mark_columns(text: &str, mark: &str) -> Vec<usize> {
    let mut columns = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with(mark) {
            let before = text[..index].trim();
            let after = text[index + mark.len()..].trim();
            if !before.is_empty() && !after.is_empty() {
                columns.push(display_columns(&text[..index]));
            }
            index += mark.len();
            continue;
        }
        index += text[index..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1);
    }
    columns
}

#[requires(!mark.is_empty())]
#[ensures(true)]
fn mark_columns(text: &str, mark: &str) -> Vec<usize> {
    let mut columns = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if text[index..].starts_with(mark) {
            columns.push(display_columns(&text[..index]));
            index += mark.len();
            continue;
        }
        index += text[index..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(1);
    }
    columns
}

#[requires(true)]
#[ensures(true)]
fn display_columns(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

#[requires(true)]
#[ensures(true)]
fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('\u{1b}') {
        out.push_str(&rest[..start]);
        rest = &rest[start + '\u{1b}'.len_utf8()..];
        if let Some(stripped) = rest.strip_prefix('[') {
            if let Some(end) = stripped.find('m') {
                rest = &stripped[end + 1..];
                continue;
            }
        }
    }
    out.push_str(rest);
    out
}

#[requires(true)]
#[ensures(ret.len() <= 604)]
fn clip(text: &str) -> String {
    let mut end = text.len().min(600);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut short = text[..end].to_owned();
    if end < text.len() {
        short.push_str(" ...");
    }
    short
}
