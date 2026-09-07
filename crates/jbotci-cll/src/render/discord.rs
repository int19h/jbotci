//! Discord-flavoured Markdown primitives.
//!
//! Discord renders a Markdown dialect without tables, math or HTML, with `-#`
//! subtext lines, and with `:shortcode:` emoji and `@mention` syntax that
//! ordinary text must never trigger. This module is the workspace's single
//! definition of how arbitrary text is made inert in that dialect: the book
//! renderer's Discord output and the Discord app's own message text both go
//! through it, so a character escaped in one place is escaped everywhere.

use super::*;

/// Characters escaped wherever they occur. Discord renders a backslash before
/// any ASCII punctuation as the bare character, so escaping is invisible to
/// the reader. `:` is included so `word:word` cannot become an emoji
/// shortcode, `@` and the angle brackets so no text can form a mention, and
/// the bracket pairs so no text can form a masked link. Heading, subtext and
/// list markers are positional and handled by [`escape_discord_markdown_line`].
pub const DISCORD_MARKDOWN_ESCAPED: &[char] = &[
    '\\', '*', '_', '~', '`', '|', '>', '[', ']', '(', ')', '<', ':', '@',
];

/// Escape one line so Discord renders it verbatim. Heading (`#`), subtext
/// (`-#`), bullet (`-`, `+`) and ordered-list (`1.`) syntax only triggers at
/// the start of a line, so those characters are escaped only there; every
/// other escape is unconditional.
#[requires(!line.contains('\n'))]
#[ensures(!ret.trim_start().starts_with(['-', '#', '+']))]
#[ensures(line.is_empty() == ret.is_empty())]
pub fn escape_discord_markdown_line(line: &str) -> String {
    let indent = line.len() - line.trim_start().len();
    let mut escaped = String::with_capacity(line.len() + 8);
    // True while only digits have followed the indent: a `.` there would
    // complete an ordered-list marker.
    let mut in_ordered_prefix = true;
    for (index, character) in line.char_indices() {
        let after_indent = index >= indent;
        let escape = DISCORD_MARKDOWN_ESCAPED.contains(&character)
            || (index == indent && matches!(character, '-' | '#' | '+'))
            || (after_indent && index > indent && in_ordered_prefix && character == '.');
        if after_indent && !character.is_ascii_digit() {
            in_ordered_prefix = false;
        }
        if escape {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// Escape arbitrary text, line by line, so Discord renders it verbatim.
#[requires(true)]
#[ensures(text.is_empty() == ret.is_empty())]
#[ensures(ret.split('\n').count() == text.split('\n').count())]
pub fn escape_discord_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 8);
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            escaped.push('\n');
        }
        escaped.push_str(&escape_discord_markdown_line(line));
    }
    escaped
}

/// The longest run of consecutive backticks in `text`.
#[requires(true)]
#[ensures(text.contains('`') == (ret > 0))]
fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for character in text.chars() {
        if character == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// The zero-width space that keeps a backtick run from closing a fence. It is
/// the one alteration a code block makes to its content, and it is visual
/// only: the state transport carries the source itself, unaltered.
const ZERO_WIDTH_SPACE: char = '\u{200b}';

/// Inline code carrying `text`. Discord's inline code is delimited by single
/// backticks, so a value containing one cannot be a code span at all: such a
/// value, and any value spanning lines, comes back as escaped plain text,
/// which is inert and still readable. Longer delimiters are CommonMark rather
/// than documented Discord syntax, so they are not used.
#[requires(true)]
#[ensures(ret.starts_with('`') == (!text.is_empty() && !text.contains('`') && !text.contains('\n') && !text.contains('\r')))]
pub fn discord_inline_code(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    if text.contains('`') || text.contains('\n') || text.contains('\r') {
        return escape_discord_markdown(text);
    }
    format!("`{text}`")
}

/// A fenced code block carrying `text`. The fence is Discord's documented
/// triple backtick, so no run of three or more backticks may survive inside
/// it: every such run is broken with a zero-width space, which is the only
/// difference between the content and what is shown.
#[requires(true)]
#[ensures(ret.starts_with("```\n") && ret.ends_with("\n```"))]
#[ensures(!ret["```\n".len()..ret.len() - "\n```".len()].contains("```"))]
pub fn discord_code_block(text: &str) -> String {
    format!("```\n{}\n```", break_backtick_runs(text))
}

/// `text` with a zero-width space inside every backtick run of three or more,
/// so that no run of three survives. Runs are emitted in pairs, which keeps
/// the inserted spaces to the minimum a triple fence needs.
#[requires(true)]
#[ensures(!ret.contains("```"))]
#[ensures(ret.replace(ZERO_WIDTH_SPACE, "") == text)]
fn break_backtick_runs(text: &str) -> String {
    if longest_backtick_run(text) < 3 {
        return text.to_owned();
    }
    let mut broken = String::with_capacity(text.len() + 8);
    let mut run = 0;
    for character in text.chars() {
        if character == '`' {
            if run == 2 {
                broken.push(ZERO_WIDTH_SPACE);
                run = 0;
            }
            run += 1;
        } else {
            run = 0;
        }
        broken.push(character);
    }
    broken
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn escaping_neutralizes_controls_mentions_and_positional_markers() {
        assert_eq!(
            escape_discord_markdown("**bold** @everyone <@1> # h\n-# sub\n  - x\n# h"),
            "\\*\\*bold\\*\\* \\@everyone \\<\\@1\\> # h\n\\-# sub\n  \\- x\n\\# h"
        );
        assert_eq!(escape_discord_markdown_line("x-ray a#b"), "x-ray a#b");
        assert_eq!(escape_discord_markdown_line("  12. two"), "  12\\. two");
        assert_eq!(escape_discord_markdown_line("1. one"), "1\\. one");
        assert_eq!(
            escape_discord_markdown_line("1.5 not a list"),
            "1\\.5 not a list"
        );
        assert_eq!(
            escape_discord_markdown_line("v1.5 not a list"),
            "v1.5 not a list"
        );
        assert_eq!(escape_discord_markdown_line("+ plus"), "\\+ plus");
        assert_eq!(escape_discord_markdown_line("a + b"), "a + b");
        assert_eq!(
            escape_discord_markdown_line("mi klama .i do"),
            "mi klama .i do"
        );
        assert_eq!(
            escape_discord_markdown_line(":x: [a](b)"),
            "\\:x\\: \\[a\\]\\(b\\)"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn inline_code_never_carries_a_backtick_that_would_close_it() {
        assert_eq!(discord_inline_code("kla"), "`kla`");
        assert_eq!(discord_inline_code(""), "");
        for interior in [1usize, 2, 3, 4, 6, 7] {
            let ticks = "`".repeat(interior);
            for text in [
                format!("a{ticks}b"),
                format!("{ticks}leading"),
                format!("trailing{ticks}"),
                ticks.clone(),
                format!("a{ticks}**bold** [link](x) @everyone"),
            ] {
                let rendered = discord_inline_code(&text);
                assert_eq!(
                    rendered,
                    escape_discord_markdown(&text),
                    "a value with backticks is inert text, not a span"
                );
                assert!(
                    !rendered.contains("`") || rendered.contains("\\`"),
                    "every backtick is escaped: {rendered:?}"
                );
                assert!(!rendered.contains("**bold**"), "{rendered:?}");
            }
        }
        // A value spanning lines is inert text as well.
        let multiline = "mi klama\n```\n**x**";
        assert_eq!(
            discord_inline_code(multiline),
            escape_discord_markdown(multiline)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_code_block_keeps_its_triple_fence_and_breaks_interior_runs() {
        for interior in [1usize, 2, 3, 4, 6, 7] {
            let ticks = "`".repeat(interior);
            let text = format!("first\n{ticks}\n**bold** {ticks}tail\n{ticks}{ticks}");
            let rendered = discord_code_block(&text);
            assert!(rendered.starts_with("```\n") && rendered.ends_with("\n```"));
            let inside = &rendered["```\n".len()..rendered.len() - "\n```".len()];
            assert!(
                inside.lines().all(|line| longest_backtick_run(line) < 3),
                "a line could close the block: {rendered:?}"
            );
            assert_eq!(
                inside.replace('\u{200b}', ""),
                text,
                "the content is unchanged apart from the zero-width spaces"
            );
            if longest_backtick_run(&text) < 3 {
                assert_eq!(inside, text, "content without a long run is untouched");
            }
        }
    }
}
