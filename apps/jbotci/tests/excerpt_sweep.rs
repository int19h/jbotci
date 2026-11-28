//! Generated excerpt sweep.
//!
//! An uncut excerpt must match the baseline byte for byte.
//! A cut excerpt must stay within 80 columns, and each interior margin mark
//! must sit in the same column on the underline.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_cli::test_harness::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use unicode_width::UnicodeWidthStr;

#[invariant(!name.is_empty())]
struct SweepCase {
    name: &'static str,
    text: String,
    must_uncut: bool,
}

#[test]
#[requires(true)]
#[ensures(true)]
fn excerpt_sweep_matches_an_uncut_baseline_and_keeps_cut_lines_narrow() {
    let cases = sweep_cases();
    let baseline = baseline_binary();
    let mut inputs = 0usize;
    let mut parity_failures = 0usize;
    let mut width_failures = 0usize;
    let mut failed = Vec::new();
    let mut notes = Vec::new();
    for case in &cases {
        for ascii in [false, true] {
            for color in [false, true] {
                inputs += 1;
                let mode = mode_name(ascii, color);
                let tag = format!("{} {mode}", case.name);
                let head = render_head(&case.text, ascii, color);
                let head_plain = strip_ansi(&head);
                let head_cut = excerpt_is_cut(&head_plain);
                if head_cut {
                    if let Some(problem) = cut_problem(&head_plain, ascii) {
                        width_failures += 1;
                        remember(&mut failed, &tag);
                        if notes.len() < 8 {
                            notes.push(format!("{tag}: {problem}"));
                        }
                    }
                }
                if let Some(binary) = &baseline {
                    let main_text = render_baseline(binary, &case.text, ascii, color);
                    let main_cut = excerpt_is_cut(&strip_ansi(&main_text));
                    if !(head_cut && main_cut) && head != main_text {
                        parity_failures += 1;
                        remember(&mut failed, &tag);
                        if notes.len() < 8 {
                            notes.push(format!(
                                "{tag}: uncut mismatch\nHEAD:\n{}\nMAIN:\n{}",
                                clip(&head),
                                clip(&main_text)
                            ));
                        }
                    }
                } else if case.must_uncut && head_cut {
                    parity_failures += 1;
                    remember(&mut failed, &tag);
                    if notes.len() < 8 {
                        notes.push(format!("{tag}: short excerpt was cut\n{}", clip(&head)));
                    }
                }
            }
        }
    }
    let summary = format!(
        "SWEEP inputs={inputs} parity_failures={parity_failures} width_failures={width_failures} baseline={}",
        if baseline.is_some() { "yes" } else { "no" }
    );
    eprintln!("{summary}");
    if let Some(path) = std::env::var_os("EXCERPT_SWEEP_LOG") {
        let mut body = summary.clone();
        body.push('\n');
        body.push_str("failures: ");
        body.push_str(&failed.join(", "));
        body.push('\n');
        for note in &notes {
            body.push_str(note);
            body.push('\n');
        }
        std::fs::write(path, body).expect("sweep log");
    }
    assert!(
        parity_failures == 0 && width_failures == 0,
        "{summary}\nfailures: {}\n{}",
        failed.join(", "),
        notes.join("\n")
    );
}

#[requires(true)]
#[ensures(true)]
fn remember(failed: &mut Vec<String>, tag: &str) {
    if !failed.iter().any(|item| item == tag) {
        failed.push(tag.to_owned());
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn sweep_cases() -> Vec<SweepCase> {
    let mut cases = Vec::new();
    for lines in 1..=5 {
        let mut text = String::from("mi nelci lo");
        if lines == 1 {
            text.push_str(" ku ku");
        } else {
            for index in 2..=lines {
                text.push('\n');
                if index == lines {
                    text.push_str("ku ku");
                } else {
                    text.push_str("barda");
                }
            }
        }
        cases.push(sweep_case(
            match lines {
                1 => "lines_1",
                2 => "lines_2",
                3 => "lines_3",
                4 => "lines_4",
                _ => "lines_5",
            },
            text,
            true,
        ));
    }
    for (name, middle) in [
        ("blank", ""),
        ("spaces", "   "),
        ("comment", "//x"),
        ("paren", "("),
        ("brace", "{"),
        ("bracket", "["),
        ("close_paren", ")"),
        ("doc_comment", "/// note"),
    ] {
        cases.push(sweep_case(
            name,
            format!("mi cusku zoi gy. abc\n{middle}\nghi"),
            true,
        ));
    }
    for gap in 1..=3 {
        let mut text = String::from("mi nelci lo\n");
        for _ in 0..gap {
            text.push('\n');
        }
        text.push_str("ku ku");
        cases.push(sweep_case(
            match gap {
                1 => "gap_1",
                2 => "gap_2",
                _ => "gap_3",
            },
            text,
            true,
        ));
    }
    cases.extend([
        sweep_case("eof_point", "mi nelci lo".to_owned(), true),
        sweep_case("eof_after_newline", "mi nelci lo\n".to_owned(), true),
        sweep_case(
            "left_cut_point",
            "mi klama lo zarci .i ".repeat(5) + "lo",
            false,
        ),
        sweep_case("keys_200", "k".repeat(200), false),
        sweep_case(
            "barda_150",
            format!("mi nelci lo {}ku ku", "barda ".repeat(150)),
            false,
        ),
        sweep_case(
            "wide_han",
            format!("mi nelci lo zoi gy. {} .gy ku ku", "漢".repeat(80)),
            false,
        ),
        sweep_case(
            "combining",
            "mi cusku zoi gy. a\u{0301}\u{0308} \u{1f469}\u{200d}\u{1f52c} .gy ku".to_owned(),
            true,
        ),
        sweep_case("tabs", "mi\tnelci\tlo ku ku".to_owned(), true),
        sweep_case(
            "seven_tabs",
            format!("{}mi nelci lo\nku ku", "\t".repeat(7)),
            true,
        ),
        sweep_case(
            "indent_27",
            format!("{}mi nelci lo\nku ku", " ".repeat(27)),
            true,
        ),
        sweep_case("control_short", "mi nelci lo \u{7}ku".to_owned(), true),
        sweep_case(
            "c1_short",
            "mi cusku zoi gy. \u{85}\u{9f} .gy ku".to_owned(),
            true,
        ),
        sweep_case("crlf_bridge", "mi nelci lo\r\n\r\nku ku".to_owned(), true),
        sweep_case("lone_cr", "lo\rku ku".to_owned(), true),
        sweep_case("warning", "mi klama fi'oi broda".to_owned(), true),
        sweep_case(
            "long_then_blank_bridge",
            format!("mi cusku zoi gy. {}\n\nghi", "a".repeat(90)),
            false,
        ),
        sweep_case(
            "long_then_far_end",
            format!(
                "mi cusku zoi gy. {}\n{}def",
                "a".repeat(90),
                "\n".repeat(99)
            ),
            false,
        ),
        sweep_case(
            "trailing_newline_end",
            format!("mi cusku zoi gy. {}\ndef\n", "a".repeat(90)),
            false,
        ),
        sweep_case(
            "zoi_four_lines",
            "mi cusku zoi gy. a\nb\nc\nd".to_owned(),
            true,
        ),
    ]);
    cases
}

#[requires(!name.is_empty())]
#[ensures(ret.name == name && ret.must_uncut == must_uncut)]
fn sweep_case(name: &'static str, text: String, must_uncut: bool) -> SweepCase {
    new!(SweepCase {
        name,
        text,
        must_uncut,
    })
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
    args.push(text);
    let cli = Cli::try_parse_from(args).expect("sweep args parse");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    run_cli_with_color_policy_and_width(cli, &mut stdout, &mut stderr, CliColorPolicy::never(), 80)
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
    command.arg(text);
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::piped());
    command.env_remove("FORCE_COLOR");
    command.env_remove("NO_COLOR");
    let output = command.output().expect("baseline render");
    String::from_utf8(output.stderr).expect("baseline stderr is utf-8")
}

#[requires(true)]
#[ensures(true)]
fn excerpt_is_cut(rendered: &str) -> bool {
    rendered.lines().any(|line| {
        let Some((gutter, body)) = line.split_once('│').or_else(|| line.split_once('|')) else {
            return false;
        };
        gutter.trim().parse::<usize>().is_ok()
            && (body.contains("...") || body.contains('…') || body.contains(":::"))
    })
}

#[requires(true)]
#[ensures(true)]
fn cut_problem(rendered: &str, ascii: bool) -> Option<String> {
    let mark = if ascii { "..." } else { "…" };
    let lines: Vec<_> = rendered.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        if !is_numbered(line) {
            continue;
        }
        let width = display_columns(line);
        if width > 80 {
            return Some(format!("line is {width} columns: {line}"));
        }
        let Some(column) = mark_column_after_gutter(line, mark) else {
            continue;
        };
        let Some(next) = lines.get(index + 1) else {
            return Some(format!("no underline under {line}"));
        };
        let next_width = display_columns(next);
        if next_width > 80 {
            return Some(format!("underline is {next_width} columns: {next}"));
        }
        let next_column = mark_column_after_gutter(next, mark);
        if next_column != Some(column) {
            return Some(format!("mark column {column} drifted\n{line}\n{next}"));
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn is_numbered(line: &str) -> bool {
    line.split_once('│')
        .or_else(|| line.split_once('|'))
        .is_some_and(|(gutter, _)| gutter.trim().parse::<usize>().is_ok())
}

#[requires(!mark.is_empty())]
#[ensures(true)]
fn mark_column_after_gutter(line: &str, mark: &str) -> Option<usize> {
    let (gutter, body) = line.split_once('│').or_else(|| line.split_once('|'))?;
    let sep_len = if line[gutter.len()..].starts_with('│') {
        '│'.len_utf8()
    } else {
        1
    };
    let prefix = gutter.len() + sep_len;
    let local = interior_mark_column(body, mark)?;
    Some(display_columns(&line[..prefix]) + local)
}

#[requires(!mark.is_empty())]
#[ensures(true)]
fn interior_mark_column(body: &str, mark: &str) -> Option<usize> {
    let mut seen = 0usize;
    let mut index = 0usize;
    let mut saw_glyph = false;
    for ch in body.chars() {
        let mark_here = index + mark.len() <= body.len() && body[index..].starts_with(mark);
        if saw_glyph && mark_here {
            let after = &body[index + mark.len()..];
            if after
                .chars()
                .any(|next| !next.is_whitespace() && !mark.starts_with(next))
            {
                return Some(seen);
            }
        }
        if !ch.is_whitespace() && !mark.starts_with(ch) {
            saw_glyph = true;
        }
        seen += display_columns(ch.encode_utf8(&mut [0; 4]));
        index += ch.len_utf8();
    }
    None
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
