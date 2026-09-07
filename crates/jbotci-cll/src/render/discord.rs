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

/// Inline code. Backtick runs inside are broken with a zero-width space so the
/// span cannot end early; that alteration is presentation-only.
#[requires(true)]
#[ensures(ret.starts_with('`') && ret.ends_with('`'))]
pub fn discord_inline_code(text: &str) -> String {
    format!("`{}`", text.replace('`', "`\u{200b}"))
}

/// A fenced code block. A fence inside the text is broken with a zero-width
/// space between its backticks so the block cannot end early.
#[requires(true)]
#[ensures(ret.starts_with("```\n") && ret.ends_with("\n```"))]
pub fn discord_code_block(text: &str) -> String {
    format!("```\n{}\n```", text.replace("```", "`\u{200b}``"))
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
    fn code_spans_cannot_be_closed_early() {
        assert_eq!(discord_inline_code("a``b"), "`a`\u{200b}`\u{200b}b`");
        assert_eq!(
            discord_code_block("x\n```\ny"),
            "```\nx\n`\u{200b}``\ny\n```"
        );
    }
}
