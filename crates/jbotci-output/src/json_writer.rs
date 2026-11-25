//! Streaming JSON presentation with a byte limit.
use crate::budget::{ByteLimit, OutputBudget};
use crate::{JsonRenderOptions, OutputError, is_compact_json_scalar, is_constructor_object};
use bityzba::{invariant, requires};
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::{self, Write};

#[invariant(true)]
#[derive(Clone, Copy)]
enum Color {
    Punctuation,
    Constructor,
    Field,
    String,
    Number,
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render(
    value: &Value,
    indent: usize,
    options: JsonRenderOptions,
    compact: bool,
    limit: ByteLimit,
) -> Result<String, OutputError> {
    let mut output = OutputBudget::new(limit).writer();
    write_value(&mut output, value, indent, options, compact)
        .map_err(|error| OutputError::from_render_io(error, limit))?;
    Ok(output.into_string().expect("JSON rendering writes UTF-8"))
}

#[requires(true)]
#[ensures(true)]
fn colored<W: Write>(
    output: &mut W,
    color: Color,
    enabled: bool,
    write: impl FnOnce(&mut W) -> io::Result<()>,
) -> io::Result<()> {
    if enabled {
        output.write_all(match color {
            Color::Punctuation => b"\x1b[90m",
            Color::Constructor => b"\x1b[94m",
            Color::Field => b"\x1b[32m",
            Color::String => b"\x1b[33m",
            Color::Number => b"\x1b[35m",
        })?;
    }
    write(output)?;
    if enabled {
        output.write_all(b"\x1b[39m")?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn punctuation<W: Write>(
    output: &mut W,
    text: &[u8],
    options: JsonRenderOptions,
) -> io::Result<()> {
    colored(output, Color::Punctuation, options.color, |output| {
        output.write_all(text)
    })
}

#[requires(true)]
#[ensures(true)]
fn key<W: Write>(
    output: &mut W,
    text: &str,
    constructor: bool,
    options: JsonRenderOptions,
) -> io::Result<()> {
    colored(
        output,
        if constructor {
            Color::Constructor
        } else {
            Color::Field
        },
        options.color,
        |output| write_json_string(output, text),
    )
}

#[requires(true)]
#[ensures(true)]
fn pad(output: &mut crate::budget::BudgetedWriter, indent: usize) -> io::Result<()> {
    output.repeat(b' ', indent)
}

#[requires(true)]
#[ensures(true)]
fn write_value(
    output: &mut crate::budget::BudgetedWriter,
    value: &Value,
    indent: usize,
    options: JsonRenderOptions,
    compact: bool,
) -> io::Result<()> {
    match value {
        Value::Array(items) => write_array(output, items, indent, options, compact),
        Value::Object(fields) if compact && is_constructor_object(fields) => {
            let (name, payload) = fields.iter().next().expect("one constructor");
            punctuation(output, b"{", options)?;
            key(output, name, true, options)?;
            punctuation(output, b":", options)?;
            if options.indent.spaces() != 0 {
                output.write_all(b" ")?;
            }
            match payload {
                Value::Object(fields) => write_object(output, fields, indent, options, true),
                value => write_value(
                    output,
                    value,
                    indent
                        .checked_add(options.indent.spaces())
                        .ok_or_else(|| io::Error::other("JSON indentation overflow"))?,
                    options,
                    true,
                ),
            }?;
            punctuation(output, b"}", options)
        }
        Value::Object(fields) => write_object(output, fields, indent, options, compact),
        Value::String(text) => colored(output, Color::String, options.color, |output| {
            write_json_string(output, text)
        }),
        scalar => colored(output, Color::Number, options.color, |output| {
            scalar
                .serialize(&mut serde_json::Serializer::new(output))
                .map_err(json_io_error)
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn write_array(
    output: &mut crate::budget::BudgetedWriter,
    items: &[Value],
    indent: usize,
    options: JsonRenderOptions,
    compact: bool,
) -> io::Result<()> {
    if compact && items.is_empty() {
        return output.write_all(b"[]");
    }
    punctuation(output, b"[", options)?;
    let multiline = !items.is_empty()
        && options.indent.spaces() != 0
        && !(compact && items.iter().all(is_compact_json_scalar));
    let next_indent = if multiline {
        indent
            .checked_add(options.indent.spaces())
            .ok_or_else(|| io::Error::other("JSON indentation overflow"))?
    } else {
        indent
    };
    for (index, value) in items.iter().enumerate() {
        if index != 0 {
            punctuation(output, b",", options)?;
            if !multiline && compact && options.indent.spaces() != 0 {
                output.write_all(b" ")?;
            }
        }
        if multiline {
            output.write_all(b"\n")?;
            pad(output, next_indent)?;
        }
        write_value(output, value, next_indent, options, compact)?;
    }
    if multiline {
        output.write_all(b"\n")?;
        pad(output, indent)?;
    }
    punctuation(output, b"]", options)
}

#[requires(true)]
#[ensures(true)]
fn write_object(
    output: &mut crate::budget::BudgetedWriter,
    fields: &Map<String, Value>,
    indent: usize,
    options: JsonRenderOptions,
    compact: bool,
) -> io::Result<()> {
    punctuation(output, b"{", options)?;
    let multiline = !fields.is_empty() && options.indent.spaces() != 0;
    let next_indent = if multiline {
        indent
            .checked_add(options.indent.spaces())
            .ok_or_else(|| io::Error::other("JSON indentation overflow"))?
    } else {
        indent
    };
    for (index, (name, value)) in fields.iter().enumerate() {
        if index != 0 {
            punctuation(output, b",", options)?;
        }
        if multiline {
            output.write_all(b"\n")?;
            pad(output, next_indent)?;
        }
        key(
            output,
            name,
            !compact && is_constructor_object(fields),
            options,
        )?;
        punctuation(output, b":", options)?;
        if options.indent.spaces() != 0 {
            output.write_all(b" ")?;
        }
        write_value(output, value, next_indent, options, compact)?;
    }
    if multiline {
        output.write_all(b"\n")?;
        pad(output, indent)?;
    }
    punctuation(output, b"}", options)
}

#[requires(true)]
#[ensures(true)]
fn json_io_error(error: serde_json::Error) -> io::Error {
    io::Error::new(error.io_error_kind().unwrap_or(io::ErrorKind::Other), error)
}

/// Write one JSON string. DEL and C1 controls use `\u00XX`. A reader gets
/// the original character. Other characters follow the usual JSON escapes.
#[requires(true)]
#[ensures(true)]
pub(crate) fn write_json_string<W: Write>(output: &mut W, text: &str) -> io::Result<()> {
    output.write_all(b"\"")?;
    let mut start = 0usize;
    for (index, character) in text.char_indices() {
        if !json_string_needs_escape(character) {
            continue;
        }
        if index > start {
            output.write_all(text[start..index].as_bytes())?;
        }
        write_json_escape(output, character)?;
        start = index + character.len_utf8();
    }
    if start < text.len() {
        output.write_all(text[start..].as_bytes())?;
    }
    output.write_all(b"\"")
}

#[requires(true)]
#[ensures(true)]
fn json_string_needs_escape(character: char) -> bool {
    matches!(character, '"' | '\\' | '\u{08}' | '\u{0C}')
        || (character as u32) < 0x20
        || character == '\u{7F}'
        || (0x80..=0x9F).contains(&(character as u32))
}

#[requires(true)]
#[ensures(true)]
fn write_json_escape<W: Write>(output: &mut W, character: char) -> io::Result<()> {
    match character {
        '"' => output.write_all(b"\\\""),
        '\\' => output.write_all(b"\\\\"),
        '\u{08}' => output.write_all(b"\\b"),
        '\u{0C}' => output.write_all(b"\\f"),
        '\n' => output.write_all(b"\\n"),
        '\r' => output.write_all(b"\\r"),
        '\t' => output.write_all(b"\\t"),
        other => {
            let code = other as u32;
            let hex = b"0123456789abcdef";
            output.write_all(&[
                b'\\',
                b'u',
                b'0',
                b'0',
                hex[((code >> 4) & 0xF) as usize],
                hex[(code & 0xF) as usize],
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonRenderOptions;
    use crate::budget::ByteLimit;
    use bityzba::requires;
    use serde_json::Value;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_reader_gets_the_original_control() {
        let mut fields = serde_json::Map::new();
        fields.insert("a\u{7f}".to_owned(), Value::String("a\u{85}b".to_owned()));
        let value = Value::Object(fields);
        let limit = ByteLimit::new(4096).expect("positive limit");
        for color in [false, true] {
            let options = JsonRenderOptions {
                color,
                ..JsonRenderOptions::default()
            };
            let encoded = render(&value, 0, options, true, limit).expect("json");
            assert!(encoded.contains("\\u007f"), "{encoded}");
            assert!(encoded.contains("\\u0085"), "{encoded}");
            assert!(!encoded.contains('\u{85}'), "{encoded}");
            assert!(!encoded.contains('\u{7f}'), "{encoded}");
            let plain = strip_style(&encoded);
            let decoded: Value = serde_json::from_str(&plain).expect("decode");
            assert_eq!(decoded["a\u{7f}"], Value::String("a\u{85}b".to_owned()));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn strip_style(text: &str) -> String {
        let mut output = String::new();
        let mut characters = text.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\u{1b}' && characters.peek() == Some(&'[') {
                characters.next();
                for next in characters.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
                continue;
            }
            output.push(character);
        }
        output
    }
}
