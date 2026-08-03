//! Typed S-expression data and the canonical smusni printer/parser.
//!
//! Rendering code constructs [`Datum`] values; only this module turns them into
//! text.  Keeping string escaping and layout here prevents individual semantic
//! renderers from acquiring subtly different quoting rules.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

/// An atom that is safe to print without quoting.
#[invariant(!text.is_empty())]
#[invariant(text.chars().all(|character| !character.is_whitespace() && !matches!(character, '(' | ')' | '"' | ';')))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Atom {
    text: String,
}

impl Atom {
    /// Construct an atom, failing when the text would need quoting.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|atom| !atom.as_str().is_empty()) || ret.is_err())]
    pub fn try_new(text: impl Into<String>) -> Result<Self, InvalidAtom> {
        let text = text.into();
        if text.is_empty()
            || text.chars().any(|character| {
                character.is_whitespace() || matches!(character, '(' | ')' | '"' | ';')
            })
        {
            return Err(new!(InvalidAtom { text: text }));
        }
        Ok(new!(Atom { text: text }))
    }

    /// Borrow the printable atom text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Error returned when syntax-significant text is used as an atom.
#[invariant(text.is_empty() || text.chars().any(|character| character.is_whitespace() || matches!(character, '(' | ')' | '"' | ';')))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidAtom {
    text: String,
}

impl fmt::Display for InvalidAtom {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "not a valid S-expression atom: {:?}", self.text)
    }
}

impl std::error::Error for InvalidAtom {}

/// A syntax-independent datum accepted by the smusni document grammar.
// All combinations are meaningful: semantic family constraints live in the
// typed elaborator, while `Atom` enforces the only lexical restriction.
#[invariant(::Atom(_) => true)]
#[invariant(::String(_) => true)]
#[invariant(::Bool(_) => true)]
#[invariant(::Signed(_) => true)]
#[invariant(::Unsigned(_) => true)]
#[invariant(::Float(_) => true)]
#[invariant(::List(_) => true)]
#[derive(Debug, Clone, PartialEq)]
pub enum Datum {
    Atom(Atom),
    String(String),
    Bool(bool),
    Signed(i128),
    Unsigned(u128),
    Float(f64),
    List(Vec<Datum>),
}

impl Datum {
    /// Construct a trusted program atom.
    #[requires(true)]
    #[ensures(matches!(ret, Self::Atom(_)))]
    pub fn atom(text: impl Into<String>) -> Self {
        let text = text.into();
        Self::Atom(Atom::try_new(text).expect("renderer atom constants must be lexically valid"))
    }

    /// Construct a list with `head` as its first datum.
    #[requires(true)]
    #[ensures(matches!(ret, Self::List(ref values) if !values.is_empty()))]
    pub fn form(head: impl Into<String>, arguments: impl IntoIterator<Item = Datum>) -> Self {
        let mut values = vec![Self::atom(head)];
        values.extend(arguments);
        Self::List(values)
    }

    /// Construct a list from already typed data.
    #[requires(true)]
    #[ensures(matches!(ret, Self::List(_)))]
    pub fn list(values: impl IntoIterator<Item = Datum>) -> Self {
        Self::List(values.into_iter().collect())
    }

    /// Return an atom's spelling, if this datum is an atom.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Atom(_)))]
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Self::Atom(atom) => Some(atom.as_str()),
            _ => None,
        }
    }

    /// Return list children, if this datum is a list.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::List(_)))]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }

    /// Return this list's atom head when it has one.
    #[requires(true)]
    #[ensures(ret.is_some() -> matches!(self, Self::List(_)))]
    pub fn form_head(&self) -> Option<&str> {
        self.as_list()?.first()?.as_atom()
    }

    /// Count typed forms with one exact head in this subtree.
    #[requires(!head.is_empty())]
    #[ensures(true)]
    pub fn count_forms(&self, head: &str) -> usize {
        usize::from(self.form_head() == Some(head))
            + self
                .as_list()
                .into_iter()
                .flat_map(|items| items.iter())
                .map(|item| item.count_forms(head))
                .sum::<usize>()
    }
}

/// Print exactly one datum and exactly one trailing newline.
#[requires(true)]
#[ensures(ret.ends_with('\n'))]
#[ensures(!ret.ends_with("\n\n"))]
pub fn print_document(datum: &Datum) -> String {
    let mut output = String::new();
    print_datum(datum, 0, &mut output);
    output.push('\n');
    output
}

/// Print a datum using the fixed structural layout rule.
// Scalar-only lists stay on one line. A list containing a nested list places
// each item after its head on its own indented line. This depends only on tree
// shape, never term size, and is therefore stable across environments.
#[requires(true)]
#[ensures(true)]
fn print_datum(datum: &Datum, indent: usize, output: &mut String) {
    match datum {
        Datum::Atom(atom) => output.push_str(atom.as_str()),
        Datum::String(value) => output.push_str(
            &serde_json::to_string(value).expect("serializing a Rust string cannot fail"),
        ),
        Datum::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Datum::Signed(value) => output.push_str(&value.to_string()),
        Datum::Unsigned(value) => output.push_str(&value.to_string()),
        Datum::Float(value) => {
            if value.is_finite() {
                output.push_str(&value.to_string());
            } else {
                // Serde never supplies non-finite JSON numbers, but Datum is a
                // reusable syntax type. Keep the document parseable if a future
                // direct semantic form encounters one.
                output.push_str(
                    &serde_json::to_string(&value.to_string())
                        .expect("serializing a Rust string cannot fail"),
                );
            }
        }
        Datum::List(values) => print_list(values, indent, output),
    }
}

/// Print a list according to the fixed scalar-only/nested layout split.
#[requires(true)]
#[ensures(true)]
fn print_list(values: &[Datum], indent: usize, output: &mut String) {
    output.push('(');
    if values.is_empty() {
        output.push(')');
        return;
    }

    let multiline = values
        .iter()
        .skip(1)
        .any(|value| matches!(value, Datum::List(_)));
    print_datum(&values[0], indent + 2, output);
    for value in &values[1..] {
        if multiline {
            output.push('\n');
            output.extend(std::iter::repeat_n(' ', indent + 2));
        } else {
            output.push(' ');
        }
        print_datum(value, indent + 2, output);
    }
    output.push(')');
}

/// Parse error for structural validation and tooling.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub byte_offset: usize,
    message: String,
}

impl fmt::Display for ParseError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.byte_offset)
    }
}

impl std::error::Error for ParseError {}

/// Parse exactly one S-expression datum, rejecting trailing forms.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub fn parse_document(input: &str) -> Result<Datum, ParseError> {
    let mut parser = Parser { input, offset: 0 };
    parser.skip_whitespace();
    let datum = parser.parse_datum()?;
    parser.skip_whitespace();
    if parser.offset != input.len() {
        return Err(parser.error("trailing data after the document"));
    }
    Ok(datum)
}

/// Cursor used by the non-recognizing S-expression parser.
#[invariant(true)]
#[derive(Debug)]
struct Parser<'a> {
    input: &'a str,
    offset: usize,
}

impl Parser<'_> {
    /// Parse one datum at the current non-whitespace cursor.
    #[requires(self.offset <= self.input.len() && self.input.is_char_boundary(self.offset))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn parse_datum(&mut self) -> Result<Datum, ParseError> {
        let Some(character) = self.remaining().chars().next() else {
            return Err(self.error("expected a datum"));
        };
        match character {
            '(' => self.parse_list(),
            ')' => Err(self.error("unexpected closing parenthesis")),
            '"' => self.parse_string(),
            _ => self.parse_atom(),
        }
    }

    /// Parse a parenthesized list.
    #[requires(self.offset <= self.input.len() && self.input.is_char_boundary(self.offset) && self.remaining().starts_with('('))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn parse_list(&mut self) -> Result<Datum, ParseError> {
        self.offset += 1;
        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            let Some(character) = self.remaining().chars().next() else {
                return Err(self.error("unterminated list"));
            };
            if character == ')' {
                self.offset += 1;
                return Ok(Datum::List(values));
            }
            values.push(self.parse_datum()?);
        }
    }

    /// Parse one JSON-compatible string token.
    #[requires(self.offset <= self.input.len() && self.input.is_char_boundary(self.offset) && self.remaining().starts_with('"'))]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn parse_string(&mut self) -> Result<Datum, ParseError> {
        let start = self.offset;
        let bytes = self.input.as_bytes();
        let mut cursor = start + 1;
        let mut escaped = false;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                let end = cursor + 1;
                let token = &self.input[start..end];
                let value = serde_json::from_str::<String>(token).map_err(|error| {
                    new!(ParseError {
                        byte_offset: start,
                        message: format!("invalid string escape: {error}"),
                    })
                })?;
                self.offset = end;
                return Ok(Datum::String(value));
            }
            cursor += 1;
        }
        Err(self.error("unterminated string"))
    }

    /// Parse an atom without assigning semantic meaning to its spelling.
    #[requires(self.offset <= self.input.len() && self.input.is_char_boundary(self.offset) && !self.remaining().is_empty())]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn parse_atom(&mut self) -> Result<Datum, ParseError> {
        let start = self.offset;
        while let Some(character) = self.remaining().chars().next() {
            if character.is_whitespace() || matches!(character, '(' | ')') {
                break;
            }
            self.offset += character.len_utf8();
        }
        let atom = Atom::try_new(&self.input[start..self.offset]).map_err(|error| {
            new!(ParseError {
                byte_offset: start,
                message: error.to_string(),
            })
        })?;
        Ok(Datum::Atom(atom))
    }

    /// Skip Unicode whitespace.
    #[requires(self.offset <= self.input.len() && self.input.is_char_boundary(self.offset))]
    #[ensures(self.offset >= old(self.offset) && self.input.is_char_boundary(self.offset))]
    fn skip_whitespace(&mut self) {
        while let Some(character) = self.remaining().chars().next() {
            if !character.is_whitespace() {
                break;
            }
            self.offset += character.len_utf8();
        }
    }

    /// Borrow the unconsumed input.
    #[requires(self.offset <= self.input.len() && self.input.is_char_boundary(self.offset))]
    #[ensures(ret.len() == self.input.len() - self.offset)]
    fn remaining(&self) -> &str {
        &self.input[self.offset..]
    }

    /// Create an error at the current cursor.
    #[requires(true)]
    #[ensures(ret.byte_offset == self.offset)]
    fn error(&self, message: impl Into<String>) -> ParseError {
        new!(ParseError {
            byte_offset: self.offset,
            message: message.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn printer_and_parser_round_trip_hostile_strings() {
        let datum = Datum::form(
            "Hostile",
            [Datum::String(
                "quote: \" slash: \\ newline:\n nul:\0 lojban: coi".into(),
            )],
        );
        let text = print_document(&datum);
        assert_eq!(parse_document(&text), Ok(datum));
        assert!(text.ends_with('\n'));
        assert!(!text.ends_with("\n\n"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parser_rejects_multiple_documents() {
        assert!(parse_document("(Smusni 0) (Smusni 0)").is_err());
    }
}
