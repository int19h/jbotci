//! Discord Markdown helpers shared by the presenters.
//!
//! User and dictionary text is always escaped as data so it can neither pose
//! as a control nor mention anyone (mentions are also disabled at the message
//! level). Budgets are measured in UTF-16 units, the platform's character
//! measure, and truncation never splits a code point or an escape.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use jbotci_cll::{discord_code_block, discord_inline_code, escape_discord_markdown};

use crate::discord::request::utf16_len;

/// Escape arbitrary text so Discord renders it verbatim.
#[requires(true)]
#[ensures(true)]
pub(crate) fn escape(text: &str) -> String {
    escape_discord_markdown(text)
}

/// Bold, escaped.
#[requires(true)]
#[ensures(ret.starts_with("**") && ret.ends_with("**"))]
pub(crate) fn bold(text: &str) -> String {
    format!("**{}**", escape(text))
}

/// Italic, escaped.
#[requires(true)]
#[ensures(ret.starts_with('*') && ret.ends_with('*'))]
pub(crate) fn italic(text: &str) -> String {
    format!("*{}*", escape(text))
}

/// Strikethrough, escaped (used for elided terminators and skipped input).
#[requires(true)]
#[ensures(ret.starts_with("~~") && ret.ends_with("~~"))]
pub(crate) fn strike(text: &str) -> String {
    format!("~~{}~~", escape(text))
}

/// Discord subtext (`-# ...`), one line, escaped.
#[requires(!line.contains('\n'))]
#[ensures(ret.starts_with("-# "))]
pub(crate) fn subtext(line: &str) -> String {
    format!("-# {}", escape(line))
}

/// Discord subtext around already-formatted Markdown (inline code, escaped
/// pieces); the caller has escaped every data run itself.
#[requires(!markdown.contains('\n'))]
#[ensures(ret.starts_with("-# "))]
pub(crate) fn subtext_markdown(markdown: &str) -> String {
    format!("-# {markdown}")
}

/// Inline code. Backtick runs inside are broken with a zero-width space so
/// the span cannot end early; that alteration is presentation-only.
#[requires(true)]
#[ensures(ret.starts_with('`') && ret.ends_with('`'))]
pub(crate) fn inline_code(text: &str) -> String {
    discord_inline_code(text)
}

/// A fenced code block. A fence inside the text is broken with a zero-width
/// space between its backticks so the block cannot end early.
#[requires(true)]
#[ensures(ret.starts_with("```\n") && ret.ends_with("\n```"))]
pub(crate) fn code_block(text: &str) -> String {
    discord_code_block(text)
}

/// Split rendered Markdown into paragraph chunks at blank lines, never inside
/// a fenced code block, so a chunk boundary is always a safe place to cut.
#[requires(true)]
#[ensures(ret.iter().all(|chunk| !chunk.is_empty()))]
pub(crate) fn split_paragraphs(markdown: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if line.trim().is_empty() && !in_fence {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            continue;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Cut `text` to at most `max_units` UTF-16 units on a character boundary,
/// appending an ellipsis when anything was removed. Returns whether it was
/// cut.
#[requires(max_units >= 1)]
#[ensures(utf16_len(&ret.0) <= max_units)]
#[ensures(!ret.1 -> ret.0 == text)]
pub(crate) fn truncate_units(text: &str, max_units: usize) -> (String, bool) {
    if utf16_len(text) <= max_units {
        return (text.to_owned(), false);
    }
    let budget = max_units.saturating_sub(1);
    let mut kept = String::new();
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
    (kept, true)
}

/// Join non-empty lines with newlines.
#[requires(true)]
#[ensures(true)]
pub(crate) fn join_lines<I>(lines: I) -> String
where
    I: IntoIterator<Item = String>,
{
    lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A similarity as a whole percentage.
#[requires(true)]
#[ensures(ret.ends_with('%'))]
pub(crate) fn percent(similarity: f32) -> String {
    format!("{:.0}%", (similarity * 100.0).clamp(0.0, 100.0))
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
        assert_eq!(bold("a*b"), "**a\\*b**");
        assert_eq!(subtext("gentufa · brackets"), "-# gentufa · brackets");
        assert_eq!(inline_code("a``b"), "`a`\u{200b}`\u{200b}b`");
        assert_eq!(code_block("x\n```\ny"), "```\nx\n`\u{200b}``\ny\n```");
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
