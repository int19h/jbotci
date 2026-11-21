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

/// Escape one line so Discord renders it verbatim.
///
/// A heading (`#`), subtext (`-#`), bullet (`-`), or ordered list (`1.`) is
/// syntax at the start of a line. The character after the marker is the one
/// Discord's own rule accepts. If `line` begins a line, pass `at_line_start`
/// as true. If `line` contains a CR, that CR is a line ending. The CR stays
/// in the output. Every other escape is unconditional.
#[requires(!line.contains('\n'))]
#[ensures(scope.is_exceeded() || (line.is_empty() == ret.is_empty()))]
#[expensive_ensures(scope.is_exceeded() || super::commonmark_lines(&ret).iter().enumerate().all(|(index, (content, _))| {
    (index == 0 && !at_line_start) || discord_marker_byte(content).is_none()
}))]
pub(crate) fn escape_discord_markdown_line_at(
    line: &str,
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    if line.as_bytes().contains(&b'\r') {
        return escape_discord_markdown_at(line, scope, at_line_start);
    }
    let marker_at = at_line_start.then(|| discord_marker_byte(line)).flatten();
    let mut escaped = scope.buffer();
    for (index, character) in line.char_indices() {
        if DISCORD_MARKDOWN_ESCAPED.contains(&character) || marker_at == Some(index) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped.into_piece()
}

/// Byte index of the one character that starts a Discord line marker.
///
/// A heading is one to three `#` characters. Discord then accepts JavaScript
/// whitespace, and it also accepts the end of the line. Subtext is `-#` and
/// then an ASCII space. A bullet is `-`. An ordered list is digits and then
/// `.`. Those two accept an ASCII space. A tab does not count. `+` is not a
/// marker.
#[requires(true)]
#[ensures(ret.is_none_or(|index| {
    line.is_char_boundary(index)
        && index < line.len()
        && matches!(line.as_bytes()[index], b'#' | b'-' | b'.')
}))]
fn discord_marker_byte(line: &str) -> Option<usize> {
    // The indent is a space or a tab. A no-break space is not an indent.
    // Discord's own rules indent with ASCII spaces. A tab stays here because
    // this check is only the gap before the marker.
    let indent = line
        .bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count();
    let tail = &line[indent..];
    let bytes = tail.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let hashes = bytes.iter().take_while(|byte| **byte == b'#').count();
    if (1..=3).contains(&hashes) && discord_heading_separator(&tail[hashes..]) {
        return Some(indent);
    }
    if bytes[0] == b'-' {
        let rest = &tail[1..];
        if rest.starts_with('#') && discord_subtext_separator(&rest[1..]) {
            return Some(indent);
        }
        if discord_list_separator(rest) {
            return Some(indent);
        }
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits >= 1
        && bytes.get(digits) == Some(&b'.')
        && discord_list_separator(&tail[digits + 1..])
    {
        return Some(indent + digits);
    }
    None
}

/// True for a character in JavaScript `\s`.
///
/// Discord's heading rule uses that set after one to three `#` characters.
#[requires(true)]
#[ensures(ret == (
    matches!(
        character,
        '\t' | '\n' | '\u{000b}' | '\u{000c}' | '\r' | ' '
            | '\u{00a0}' | '\u{1680}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
            | '\u{205f}' | '\u{3000}' | '\u{feff}'
    ) || ('\u{2000}'..='\u{200a}').contains(&character)
))]
fn discord_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n'
            | '\u{000b}'
            | '\u{000c}'
            | '\r'
            | ' '
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    ) || ('\u{2000}'..='\u{200a}').contains(&character)
}

/// True when `rest` can end a Discord heading.
///
/// The end of the line counts. A line feed is JavaScript whitespace, so a
/// heading can take its text from the next line. A following character also
/// counts when it is in that set. U+3000 is in the set.
#[requires(true)]
#[ensures(ret == (rest.is_empty() || rest.chars().next().is_some_and(discord_js_whitespace)))]
fn discord_heading_separator(rest: &str) -> bool {
    rest.is_empty() || rest.chars().next().is_some_and(discord_js_whitespace)
}

/// True when `rest` can end a `-#` marker.
///
/// Discord accepts one or more ASCII spaces and then text on the same line.
/// A tab does not count. U+3000 does not count. A lone space also counts
/// here. The Discord input block uses `-# ` as a header, so that prefix
/// stays escaped.
#[requires(true)]
#[ensures(ret == rest.starts_with(' '))]
fn discord_subtext_separator(rest: &str) -> bool {
    rest.starts_with(' ')
}

/// True when `rest` can end a `-` bullet or an `N.` list.
///
/// Discord accepts one ASCII space and then more text. The line ending
/// counts as that text. A tab does not count. U+3000 does not count. `+`
/// is not a bullet.
#[requires(true)]
#[ensures(ret == rest.starts_with(' '))]
fn discord_list_separator(rest: &str) -> bool {
    rest.starts_with(' ')
}

/// Escape one line that begins at column zero.
#[requires(!line.contains('\n'))]
#[ensures(scope.is_exceeded() || (line.is_empty() == ret.is_empty()))]
#[expensive_ensures(scope.is_exceeded() || super::commonmark_lines(&ret).iter().all(|(content, _)| {
    discord_marker_byte(content).is_none()
}))]
pub fn escape_discord_markdown_line_in_scope(
    line: &str,
    scope: &jbotci_budget::TextScope,
) -> String {
    escape_discord_markdown_line_at(line, scope, true)
}

/// Escape arbitrary text, line by line, so Discord renders it verbatim.
///
/// The first line uses `at_line_start`. Each later line begins a line of its own.
/// A line ends at LF, CRLF, or a lone CR. The ending stays in the output.
#[requires(true)]
#[ensures(scope.is_exceeded() || (text.is_empty() == ret.is_empty()))]
#[ensures(scope.is_exceeded() || (ret.split('\n').count() == text.split('\n').count()))]
#[expensive_ensures(scope.is_exceeded() || super::commonmark_lines(&ret).iter().enumerate().all(|(index, (content, _))| {
    (index == 0 && !at_line_start) || discord_marker_byte(content).is_none()
}))]
pub(crate) fn escape_discord_markdown_at(
    text: &str,
    scope: &jbotci_budget::TextScope,
    at_line_start: bool,
) -> String {
    let mut escaped = scope.buffer();
    for (index, (line, ending)) in super::commonmark_lines(text).into_iter().enumerate() {
        escaped.push_str(&escape_discord_markdown_line_at(
            line,
            scope,
            at_line_start || index > 0,
        ));
        escaped.push_str(ending);
    }
    escaped.into_piece()
}

/// Escape arbitrary text that begins at column zero.
#[requires(true)]
#[ensures(scope.is_exceeded() || (text.is_empty() == ret.is_empty()))]
#[ensures(scope.is_exceeded() || (ret.split('\n').count() == text.split('\n').count()))]
pub fn escape_discord_markdown_in_scope(text: &str, scope: &jbotci_budget::TextScope) -> String {
    escape_discord_markdown_at(text, scope, true)
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
#[ensures(scope.is_exceeded() || (ret.starts_with('`') == (!text.is_empty() && !text.contains('`') && !text.contains('\n') && !text.contains('\r'))))]
pub fn discord_inline_code_in_scope(text: &str, scope: &jbotci_budget::TextScope) -> String {
    if text.is_empty() {
        return String::new();
    }
    if text.contains('`') || text.contains('\n') || text.contains('\r') {
        return escape_discord_markdown_in_scope(text, scope);
    }
    scope.format(format_args!("`{text}`"))
}

/// A fenced code block carrying `text`. The fence is Discord's documented
/// triple backtick, so no run of three or more backticks may survive inside
/// it: every such run is broken with a zero-width space, which is the only
/// difference between the content and what is shown.
#[requires(true)]
#[ensures(scope.is_exceeded() || (ret.starts_with("```\n") && ret.ends_with("\n```")))]
#[ensures(scope.is_exceeded() || (!ret["```\n".len()..ret.len() - "\n```".len()].contains("```")))]
pub fn discord_code_block_in_scope(text: &str, scope: &jbotci_budget::TextScope) -> String {
    scope.format(format_args!(
        "```\n{}\n```",
        break_backtick_runs_in_scope(text, scope)
    ))
}

/// `text` with a zero-width space inside every backtick run of three or more,
/// so that no run of three survives. Runs are emitted in pairs, which keeps
/// the inserted spaces to the minimum a triple fence needs.
#[requires(true)]
#[ensures(scope.is_exceeded() || (!ret.contains("```")))]
// Content of its own may already carry zero-width spaces, so what is
// preserved is the text apart from them, not the text with mine removed.
#[ensures(scope.is_exceeded() || (ret.chars().filter(|character| *character != ZERO_WIDTH_SPACE)
    .eq(text.chars().filter(|character| *character != ZERO_WIDTH_SPACE))))]
fn break_backtick_runs_in_scope(text: &str, scope: &jbotci_budget::TextScope) -> String {
    if longest_backtick_run(text) < 3 {
        return scope.copy(text);
    }
    let mut broken = scope.buffer();
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
    broken.into_piece()
}

#[requires(true)]
#[ensures(true)]
pub fn escape_discord_markdown_line(line: &str) -> String {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    scope
        .finish(escape_discord_markdown_line_in_scope(line, &scope))
        .expect("unrestricted Discord Markdown")
}

#[requires(true)]
#[ensures(true)]
pub fn escape_discord_markdown(text: &str) -> String {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    scope
        .finish(escape_discord_markdown_in_scope(text, &scope))
        .expect("unrestricted Discord Markdown")
}

#[requires(true)]
#[ensures(true)]
pub fn discord_inline_code(text: &str) -> String {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    scope
        .finish(discord_inline_code_in_scope(text, &scope))
        .expect("unrestricted Discord Markdown")
}

#[requires(true)]
#[ensures(true)]
pub fn discord_code_block(text: &str) -> String {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    scope
        .finish(discord_code_block_in_scope(text, &scope))
        .expect("unrestricted Discord Markdown")
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
            "1.5 not a list"
        );
        assert_eq!(
            escape_discord_markdown_line("v1.5 not a list"),
            "v1.5 not a list"
        );
        assert_eq!(escape_discord_markdown_line("+ plus"), "+ plus");
        assert_eq!(escape_discord_markdown_line("a + b"), "a + b");
        assert_eq!(
            escape_discord_markdown_line("mi klama .i do"),
            "mi klama .i do"
        );
        assert_eq!(escape_discord_markdown(".ui"), ".ui");
        assert_eq!(escape_discord_markdown(".uinai"), ".uinai");
        let scope = jbotci_budget::TextScope::new(
            jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
        );
        assert_eq!(
            escape_discord_markdown_at("roi-nai", &scope, true),
            "roi-nai"
        );
        assert_eq!(escape_discord_markdown_at("-nai", &scope, false), "-nai");
        assert_eq!(escape_discord_markdown_at("-nai", &scope, true), "-nai");
        assert_eq!(escape_discord_markdown_line("#a"), "#a");
        assert_eq!(escape_discord_markdown_line("# a"), "\\# a");
        assert_eq!(escape_discord_markdown_line("## a"), "\\## a");
        assert_eq!(escape_discord_markdown_line("-nai"), "-nai");
        assert_eq!(escape_discord_markdown_line("- a"), "\\- a");
        assert_eq!(escape_discord_markdown_line("1.a"), "1.a");
        assert_eq!(escape_discord_markdown_line("1."), "1.");
        assert_eq!(escape_discord_markdown_line("-# sub"), "\\-# sub");
        assert_eq!(escape_discord_markdown_line("-#sub"), "-#sub");
        assert_eq!(
            escape_discord_markdown_at("1. aroi", &scope, true),
            "1\\. aroi"
        );
        assert_eq!(escape_discord_markdown_at("1. a", &scope, false), "1. a");
        assert_eq!(
            escape_discord_markdown_line(":x: [a](b)"),
            "\\:x\\: \\[a\\]\\(b\\)"
        );
        assert_eq!(escape_discord_markdown("#\r\n"), "\\#\r\n");
        assert_eq!(escape_discord_markdown("1.\r\n"), "1.\r\n");
        assert_eq!(
            escape_discord_markdown_line("#\u{3000}title"),
            "\\#\u{3000}title"
        );
        assert_eq!(
            escape_discord_markdown_line("#\u{00a0}title"),
            "\\#\u{00a0}title"
        );
        assert_eq!(
            escape_discord_markdown_line("#\u{feff}title"),
            "\\#\u{feff}title"
        );
        assert_eq!(
            escape_discord_markdown_line("#\u{0085}title"),
            "#\u{0085}title"
        );
        assert_eq!(escape_discord_markdown_line("#### a"), "#### a");
        assert_eq!(escape_discord_markdown_line("-#"), "-#");
        assert_eq!(escape_discord_markdown_line("-# "), "\\-# ");
        assert_eq!(
            escape_discord_markdown_line("-#\u{3000}title"),
            "-#\u{3000}title"
        );
        assert_eq!(escape_discord_markdown_line("- "), "\\- ");
        assert_eq!(escape_discord_markdown_line("-  x"), "\\-  x");
        assert_eq!(escape_discord_markdown_line("-\titem"), "-\titem");
        assert_eq!(escape_discord_markdown_line("#\ttitle"), "\\#\ttitle");
        assert_eq!(escape_discord_markdown_line("-#\ttitle"), "-#\ttitle");
        assert_eq!(
            escape_discord_markdown_line("-\u{3000}item"),
            "-\u{3000}item"
        );
        assert_eq!(escape_discord_markdown_line("1. "), "1\\. ");
        assert_eq!(escape_discord_markdown_line("1.  "), "1\\.  ");
        assert_eq!(
            escape_discord_markdown_line("1.\u{3000}item"),
            "1.\u{3000}item"
        );
        assert_eq!(escape_discord_markdown("#\r"), "\\#\r");
        assert_eq!(escape_discord_markdown("a\r# b"), "a\r\\# b");
        assert_eq!(escape_discord_markdown("\u{00a0}# a"), "\u{00a0}# a");
        assert_eq!(escape_discord_markdown("  # a"), "  \\# a");
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
    fn content_that_already_carries_zero_width_spaces_is_still_carried() {
        // A reader can paste a zero-width space; the block keeps it and still
        // breaks the runs that would close the fence.
        let text = "a\u{200b}b\n``\u{200b}`\n```";
        let rendered = discord_code_block(text);
        let inside = &rendered["```\n".len()..rendered.len() - "\n```".len()];
        assert!(!inside.contains("```"), "{rendered:?}");
        assert_eq!(
            inside.matches('\u{200b}').count(),
            3,
            "one space is added to the run that needed it: {inside:?}"
        );
        assert_eq!(inside.replace('\u{200b}', ""), text.replace('\u{200b}', ""));
        // Inline code refuses the value for its backticks, not its spaces.
        assert_eq!(discord_inline_code("a\u{200b}b"), "`a\u{200b}b`");
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
                text.replace('\u{200b}', ""),
                "the content is unchanged apart from the zero-width spaces"
            );
            if longest_backtick_run(&text) < 3 {
                assert_eq!(inside, text, "content without a long run is untouched");
            }
        }
    }
}
