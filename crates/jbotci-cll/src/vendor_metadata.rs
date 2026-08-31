//! Parsing for the flat vendored records that fix the reference book's
//! identity: `vendor/cll/.env` (the book's own declaration of its title,
//! version, and publisher line) and `vendor/cll.VENDORED_FROM` (the pin we
//! vendored it at).
//!
//! The accepted format is deliberately narrow rather than a general `.env`
//! dialect. A vendored file that uses a construct this module does not
//! interpret — single quotes, escapes, an inline comment — is rejected with an
//! explanation instead of being silently misread into the reported edition,
//! because a wrong edition is worse than a failed build.
//!
//! Accepted:
//!
//! - blank lines and whole-line `#` comments, which are skipped;
//! - otherwise exactly `KEY<separator>VALUE`, where `KEY` is non-empty and made
//!   of ASCII alphanumerics, `_`, or `-`, and no key repeats;
//! - `VALUE` either bare, or wrapped in one pair of double quotes; either way
//!   it must be non-empty and carry no leading or trailing whitespace, and it
//!   must contain no `"`, no `\`, and no `#`.
//!
//! `build.rs` includes this file by path to produce the generated edition
//! constants, and the crate's own tests include it so they check the same parse
//! the build performed rather than a substring approximation of it.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

/// Parses one flat `KEY<separator>VALUE` record file. Values come back in file
/// order, canonical: trimmed, unquoted, and non-empty.
#[requires(!source_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|fields| fields
    .iter()
    .all(|(key, value)| !key.is_empty() && !value.is_empty() && value.trim() == value))
    || ret.is_err())]
pub(crate) fn parse_key_value_file(
    text: &str,
    separator: char,
    source_name: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(separator)
            .ok_or_else(|| format!("{source_name} has a line without {separator:?}: {line:?}"))?;
        let key = key.trim();
        if key.is_empty() {
            return Err(format!(
                "{source_name} has a line with an empty key: {line:?}"
            ));
        }
        if !key.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            return Err(format!(
                "{source_name} has a key with unsupported characters: {key:?}"
            ));
        }
        if fields.iter().any(|(existing, _)| existing == key) {
            return Err(format!("{source_name} repeats the key {key:?}"));
        }
        fields.push((key.to_owned(), parse_value(value, key, source_name)?));
    }
    Ok(fields)
}

/// Unwraps and validates one record's value. Rejects every construct the
/// module does not interpret rather than guessing at its meaning.
#[requires(!key.is_empty())]
#[requires(!source_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty() && value.trim() == value) || ret.is_err())]
fn parse_value(value: &str, key: &str, source_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.starts_with('\'') {
        return Err(format!(
            "{source_name} single-quotes the value of {key:?}, which is not an interpreted form here"
        ));
    }
    let value = if let Some(rest) = value.strip_prefix('"') {
        rest.strip_suffix('"')
            .ok_or_else(|| format!("{source_name} has an unterminated quoted value for {key:?}"))?
    } else {
        value
    };
    for (character, explanation) in [
        ('"', "an embedded double quote"),
        ('\\', "a backslash escape"),
        ('#', "an inline comment marker"),
    ] {
        if value.contains(character) {
            return Err(format!(
                "{source_name} has {explanation} in the value of {key:?}, which is not an interpreted form here"
            ));
        }
    }
    if value.trim() != value {
        return Err(format!(
            "{source_name} pads the value of {key:?} with whitespace"
        ));
    }
    if value.is_empty() {
        return Err(format!("{source_name} has an empty value for {key:?}"));
    }
    Ok(value.to_owned())
}

/// Looks up a record the edition cannot be reported without.
#[requires(!key.is_empty())]
#[requires(!source_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
pub(crate) fn required_field<'a>(
    fields: &'a [(String, String)],
    key: &str,
    source_name: &str,
) -> Result<&'a str, String> {
    fields
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.as_str())
        .ok_or_else(|| format!("{source_name} is missing a non-empty {key}"))
}

/// Checks the two vendored records against each other.
///
/// The book names itself `<edition>-<version>` while the repository tags the
/// same release `v<version>`; anything else means the submodule and the pin
/// record disagree about which edition is actually vendored, so the edition we
/// would report could be wrong.
#[requires(!version.is_empty())]
#[requires(!release_tag.is_empty())]
#[ensures(ret.is_ok() == (release_tag
    .strip_prefix('v')
    .is_some_and(|tag| !tag.is_empty()
        && version
            .strip_suffix(tag)
            .and_then(|edition| edition.strip_suffix('-'))
            .is_some_and(|edition| !edition.is_empty()))))]
pub(crate) fn check_version_matches_release_tag(
    version: &str,
    release_tag: &str,
) -> Result<(), String> {
    let tag_version = release_tag.strip_prefix('v').filter(|tag| !tag.is_empty());
    let Some(tag_version) = tag_version else {
        return Err(format!(
            "vendored CLL release tag {release_tag:?} is not of the form v<version>"
        ));
    };
    let edition = version
        .strip_suffix(tag_version)
        .and_then(|edition| edition.strip_suffix('-'))
        .filter(|edition| !edition.is_empty());
    if edition.is_none() {
        return Err(format!(
            "vendored CLL edition {version:?} is not of the form <edition>-{tag_version} required by the pinned release tag {release_tag:?}"
        ));
    }
    Ok(())
}
