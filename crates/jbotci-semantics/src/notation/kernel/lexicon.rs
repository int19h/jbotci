//! Token spellings shared by the kernel and every notation over it.
//!
//! The kernel owns identity spellings — variable names, lexical roots, and
//! decimal place labels — because a kernel value must be printable by any
//! notation, not only by the version-0 S-expression serialization. The
//! S-expression lexer therefore reads its bare-symbol and decimal productions
//! from here rather than defining a second, silently divergent copy.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

/// The largest decimal literal any kernel or notation layer accepts, in digits.
///
/// Integers are arbitrary-precision, so an untrusted document could otherwise
/// spend unbounded time and memory on one token, and a place label with four
/// thousand digits is already far beyond any represented row. The bound belongs
/// to the kernel because [`super::types::PositiveInteger`] enforces it on
/// values that no parser ever produced.
pub const MAX_INTEGER_DIGITS: usize = 4_096;

/// Validate a bare symbol name.
///
/// This is the single definition of the version-0 bare-symbol production. The
/// kernel's typed lexical wrappers and the S-expression atom grammar share it,
/// so a name that validates as a kernel [`super::types::Variable`] is always
/// printable as a lexical atom.
#[requires(true)]
#[ensures(ret == (!text.is_empty() && text.chars().next().is_some_and(char::is_alphabetic)
    && text.chars().skip(1).all(|character| character.is_alphanumeric()
        || matches!(character, '\'' | '-' | '_' | '.'))))]
pub fn is_symbol_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters.next().is_some_and(char::is_alphabetic)
        && characters.all(|character| {
            character.is_alphanumeric() || matches!(character, '\'' | '-' | '_' | '.')
        })
}

/// Validate a canonical unbounded positive-integer token.
#[requires(true)]
#[ensures(ret -> !text.starts_with('0'))]
pub fn is_positive_integer_text(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_INTEGER_DIGITS
        && !text.starts_with('0')
        && text.bytes().all(|byte| byte.is_ascii_digit())
}
