//! Discord Markdown helpers shared by the presenters.
//!
//! User and dictionary text is always escaped as data so it can neither pose
//! as a control nor mention anyone (mentions are also disabled at the message
//! level). Budgets are measured in UTF-16 units, the platform's character
//! measure, and truncation never splits a code point or an escape.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use jbotci_cll::{
    discord_code_block_in_scope, discord_inline_code_in_scope, escape_discord_markdown_in_scope,
};

use crate::discord::request::utf16_len;

/// Escape arbitrary text so Discord renders it verbatim.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (true))]
pub(crate) fn escape_in_scope(text: &str, text_scope: &jbotci_output::budget::TextScope) -> String {
    escape_discord_markdown_in_scope(text, text_scope)
}

/// Strikethrough, escaped (used for elided terminators and skipped input).
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.starts_with("~~") && ret.ends_with("~~")))]
pub(crate) fn strike_in_scope(text: &str, text_scope: &jbotci_output::budget::TextScope) -> String {
    text_scope.format(format_args!("~~{}~~", escape_in_scope(text, text_scope)))
}

/// Discord subtext (`-# ...`), one line, escaped.
#[requires(!line.contains('\n'))]
#[ensures(text_scope.is_exceeded() || (ret.starts_with("-# ")))]
pub(crate) fn subtext_in_scope(
    line: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    text_scope.format(format_args!("-# {}", escape_in_scope(line, text_scope)))
}

/// Discord subtext around already-formatted Markdown (inline code, escaped
/// pieces); the caller has escaped every data run itself.
#[requires(!markdown.contains('\n'))]
#[ensures(text_scope.is_exceeded() || (ret.starts_with("-# ")))]
pub(crate) fn subtext_markdown_in_scope(
    markdown: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    text_scope.format(format_args!("-# {markdown}"))
}

/// Inline code, or escaped plain text when the value cannot be a code span
/// (Discord delimits inline code with one backtick, so a value carrying one
/// would close it). Either way the result is inert.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.is_empty() == text.is_empty()))]
pub(crate) fn inline_code_in_scope(
    text: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    discord_inline_code_in_scope(text, text_scope)
}

/// A fenced code block. Backtick runs inside are broken with a zero-width
/// space so no line can close the fence early.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.starts_with("```\n") && ret.ends_with("\n```")))]
pub(crate) fn code_block_in_scope(
    text: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    discord_code_block_in_scope(text, text_scope)
}

/// Split rendered Markdown into paragraph chunks at blank lines, never inside
/// a fenced code block, so a chunk boundary is always a safe place to cut.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.iter().all(|chunk| !chunk.is_empty())))]
pub(crate) fn split_paragraphs_in_scope(
    markdown: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = text_scope.buffer();
    let mut in_fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if line.trim().is_empty() && !in_fence {
            if !current.is_empty() {
                chunks.push(current.take_piece());
            }
            continue;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        chunks.push(current.into_piece());
    }
    chunks
}

/// Cut `text` to at most `max_units` UTF-16 units on a character boundary,
/// appending an ellipsis when anything was removed. Returns whether it was
/// cut.
#[requires(max_units >= 1)]
#[ensures(text_scope.is_exceeded() || (utf16_len(&ret.0) <= max_units))]
#[ensures(text_scope.is_exceeded() || (!ret.1 -> ret.0 == text))]
pub(crate) fn truncate_units_in_scope(
    text: &str,
    max_units: usize,
    text_scope: &jbotci_output::budget::TextScope,
) -> (String, bool) {
    if utf16_len(text) <= max_units {
        return (text_scope.copy(text), false);
    }
    let budget = max_units.saturating_sub(1);
    let mut kept = text_scope.buffer();
    let mut units = 0;
    for character in text.chars() {
        let width = character.len_utf16();
        if units + width > budget {
            break;
        }
        units += width;
        kept.push(character);
    }
    // Do not end in a dangling backslash escape.
    if kept.ends_with('\\') {
        kept.pop();
    }
    kept.push('…');
    (kept.into_piece(), true)
}

/// Join non-empty lines with newlines.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (true))]
pub(crate) fn join_lines_in_scope<I>(
    lines: I,
    text_scope: &jbotci_output::budget::TextScope,
) -> String
where
    I: IntoIterator<Item = String>,
{
    text_scope.join(lines.into_iter().filter(|line| !line.is_empty()), "\n")
}

/// A similarity as a whole percentage.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.ends_with('%')))]
pub(crate) fn percent_in_scope(
    similarity: f32,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    text_scope.format(format_args!(
        "{:.0}%",
        (similarity * 100.0).clamp(0.0, 100.0)
    ))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn escape(text: &str) -> String {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = escape_in_scope(text, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn subtext(line: &str) -> String {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = subtext_in_scope(line, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn escaping_neutralizes_controls_mentions_and_headers() {
        assert_eq!(
            escape("**bold** @everyone <@1> # h\n-# sub"),
            "\\*\\*bold\\*\\* \\@everyone \\<\\@1\\> # h\n\\-# sub"
        );
        assert_eq!(subtext("gentufa · brackets"), "-# gentufa · brackets");
        assert_eq!(inline_code("kla"), "`kla`");
        assert_eq!(inline_code("a``b"), escape("a``b"));
        assert_eq!(code_block("x\n```\ny"), "```\nx\n``\u{200b}`\ny\n```");
        assert_eq!(
            split_paragraphs("a\n\n```\nx\n\ny\n```\n\n\nb\nc\n"),
            vec![
                "a".to_owned(),
                "```\nx\n\ny\n```".to_owned(),
                "b\nc".to_owned()
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn truncation_respects_units_and_escapes() {
        assert_eq!(truncate_units("abc", 3), ("abc".to_owned(), false));
        assert_eq!(truncate_units("abcd", 3), ("ab…".to_owned(), true));
        // An astral character is two units; it is dropped rather than split.
        assert_eq!(truncate_units("a😀b", 3), ("a…".to_owned(), true));
        assert_eq!(truncate_units("ab\\*c", 4), ("ab…".to_owned(), true));
        assert_eq!(percent(0.4567), "46%");
    }
}
