//! Lojban dialect formula model and parser.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bityzba::expensive_ensures;
use bityzba::{ensures, fields, invariant, requires};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DIALECT_SWAP_OPERATOR: &str = "\u{1f8d0}";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
pub struct DialectError {
    message: String,
}

impl DialectError {
    #[requires(!message.is_empty(), "dialect errors must have a diagnostic message")]
    #[ensures(!ret.message.is_empty())]
    fn new(message: String) -> Self {
        Self { message }
    }

    #[ensures(!ret.is_empty())]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DialectFeature {
    Cbm,
    Gadganzu,
    AllowCgv,
    CaseInsensitive,
    SoiAdverbials,
    TermHierarchy,
    ZantufaAdverbials,
    ZantufaCmavo,
    ZantufaConnectives,
    ZantufaMex,
    ZantufaMorphology,
    ZantufaQuotes,
    ZantufaTags,
    ZantufaTerms,
}

impl DialectFeature {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Cbm,
            Self::Gadganzu,
            Self::AllowCgv,
            Self::CaseInsensitive,
            Self::SoiAdverbials,
            Self::TermHierarchy,
            Self::ZantufaAdverbials,
            Self::ZantufaCmavo,
            Self::ZantufaConnectives,
            Self::ZantufaMex,
            Self::ZantufaMorphology,
            Self::ZantufaQuotes,
            Self::ZantufaTags,
            Self::ZantufaTerms,
        ]
    }

    #[ensures(!ret.is_empty())]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cbm => "cbm",
            Self::Gadganzu => "gadganzu",
            Self::AllowCgv => "allow-cgv",
            Self::CaseInsensitive => "case-insensitive",
            Self::SoiAdverbials => "soi-adverbials",
            Self::TermHierarchy => "term-hierarchy",
            Self::ZantufaAdverbials => "zantufa-adverbials",
            Self::ZantufaCmavo => "zantufa-cmavo",
            Self::ZantufaConnectives => "zantufa-connectives",
            Self::ZantufaMex => "zantufa-mex",
            Self::ZantufaMorphology => "zantufa-morphology",
            Self::ZantufaQuotes => "zantufa-quotes",
            Self::ZantufaTags => "zantufa-tags",
            Self::ZantufaTerms => "zantufa-terms",
        }
    }

    #[ensures(!ret.is_empty())]
    fn atom_name(self) -> String {
        self.name().to_ascii_uppercase()
    }
}

impl fmt::Display for DialectFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CmavoDialectEntry {
    Swap {
        left: String,
        right: String,
    },
    Expansion {
        source: String,
        replacement: Vec<String>,
    },
}

impl CmavoDialectEntry {
    #[expensive_ensures(ret -> self.normalized_words().iter().all(|word| is_normalized_cmavo(word)))]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Swap { left, right } => is_normalized_cmavo(left) && is_normalized_cmavo(right),
            Self::Expansion {
                source,
                replacement,
            } => {
                is_normalized_cmavo(source)
                    && !replacement.is_empty()
                    && replacement.iter().all(|word| is_normalized_cmavo(word))
            }
        }
    }

    #[cfg_attr(not(feature = "expensive_contracts"), allow(dead_code))]
    fn normalized_words(&self) -> Vec<&str> {
        match self {
            Self::Swap { left, right } => vec![left, right],
            Self::Expansion {
                source,
                replacement,
            } => std::iter::once(source.as_str())
                .chain(replacement.iter().map(String::as_str))
                .collect(),
        }
    }
}

#[invariant(!self.source_text.is_empty(), "transform source text must not be empty")]
#[invariant(!self.target_text.is_empty(), "transform target text must not be empty")]
#[invariant(!self.group_key.is_empty(), "transform group key must not be empty")]
#[invariant(self.output_count > 0, "transform output count must be positive")]
#[invariant(self.output_index < self.output_count, "transform output index must be in range")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CmavoDialectTransform {
    pub source_text: String,
    pub target_text: String,
    pub group_key: String,
    pub output_index: usize,
    pub output_count: usize,
}

#[invariant(self.cmavo_entries.iter().all(CmavoDialectEntry::is_valid), "cmavo dialect entries must be normalized and internally valid")]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DialectDefinition {
    pub cmavo_entries: Vec<CmavoDialectEntry>,
    pub features: BTreeSet<DialectFeature>,
}

impl DialectDefinition {
    #[ensures(ret.is_baseline())]
    pub fn baseline() -> Self {
        Self::default()
    }

    #[ensures(ret == self.cmavo_entries.is_empty() && self.features.is_empty())]
    pub fn is_baseline(&self) -> bool {
        self.cmavo_entries.is_empty() && self.features.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinDialect {
    pub name: &'static str,
    pub definition: &'static str,
    pub dialect: DialectDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DialectDefinitionEntry {
    Cmavo(CmavoDialectEntry),
    Feature(DialectFeatureToggle, DialectFeature),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialectFeatureToggle {
    Enable,
    Disable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DialectToken {
    OpenParen,
    CloseParen,
    Atom(String),
}

pub fn parse_dialect_definition(source: &str) -> Result<DialectDefinition, DialectError> {
    parse_dialect_definition_with_reference_resolver(source, &lookup_builtin_dialect_reference)
}

pub fn builtin_dialects() -> Vec<BuiltinDialect> {
    builtin_dialect_sources()
        .into_iter()
        .map(|(name, definition)| {
            let dialect = parse_builtin_dialect(name, definition);
            BuiltinDialect {
                name,
                definition,
                dialect,
            }
        })
        .collect()
}

#[ensures(!ret.is_empty())]
pub fn builtin_dialect_names() -> Vec<&'static str> {
    builtin_dialect_sources()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

pub fn find_builtin_dialect(requested_name: &str) -> Option<BuiltinDialect> {
    let canonical_name = builtin_reference_canonical_name(requested_name);
    builtin_dialects()
        .into_iter()
        .find(|dialect| dialect.name == canonical_name)
}

#[requires(!source.is_empty(), "builtin dialect definitions must not be empty")]
fn parse_builtin_dialect(name: &str, source: &str) -> DialectDefinition {
    parse_dialect_definition_with_reference_resolver(source, &|reference| {
        lookup_builtin_dialect_reference_in_stack(reference, &[name])
    })
    .unwrap_or_else(|error| {
        panic!(
            "invalid builtin dialect `{name}` definition `{source}`: {}",
            error.message()
        )
    })
}

fn parse_dialect_definition_with_reference_resolver(
    source: &str,
    reference_resolver: &dyn Fn(&str) -> Result<DialectDefinition, DialectError>,
) -> Result<DialectDefinition, DialectError> {
    let tokens = tokenize(source);
    let (entries, rest) = parse_dialect_token_entries(reference_resolver, &tokens, 0)?;
    if let Some(token) = rest.first() {
        return Err(DialectError::new(format!(
            "Unexpected token after dialect definition: {}",
            token.text()
        )));
    }
    Ok(definition_from_entries(entries))
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|(entries, rest)| !entries.is_empty() || rest.is_empty()))]
fn parse_dialect_token_entries<'a>(
    reference_resolver: &dyn Fn(&str) -> Result<DialectDefinition, DialectError>,
    tokens: &'a [DialectToken],
    start: usize,
) -> Result<(Vec<DialectDefinitionEntry>, &'a [DialectToken]), DialectError> {
    match tokens.get(start) {
        Some(DialectToken::OpenParen) => {
            parse_entries(reference_resolver, Vec::new(), &tokens[start + 1..])
        }
        None => Err(DialectError::new("Expected a dialect list.".to_owned())),
        Some(token) => Err(DialectError::new(format!(
            "Expected `(` to start dialect definition, found: {}",
            token.text()
        ))),
    }
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|(_, rest)| rest.len() <= tokens.len()))]
fn parse_entries<'a>(
    reference_resolver: &dyn Fn(&str) -> Result<DialectDefinition, DialectError>,
    mut acc: Vec<DialectDefinitionEntry>,
    mut tokens: &'a [DialectToken],
) -> Result<(Vec<DialectDefinitionEntry>, &'a [DialectToken]), DialectError> {
    loop {
        match tokens.first() {
            Some(DialectToken::CloseParen) => return Ok((acc, &tokens[1..])),
            None => return Err(DialectError::new("Unclosed dialect list.".to_owned())),
            Some(_) => {
                let (entry, rest) = parse_entry(reference_resolver, tokens)?;
                acc.extend(entry);
                tokens = rest;
            }
        }
    }
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|(_, rest)| rest.len() < tokens.len()))]
fn parse_entry<'a>(
    reference_resolver: &dyn Fn(&str) -> Result<DialectDefinition, DialectError>,
    tokens: &'a [DialectToken],
) -> Result<(Vec<DialectDefinitionEntry>, &'a [DialectToken]), DialectError> {
    match tokens {
        [DialectToken::Atom(atom_text), rest @ ..] => {
            if let Some((toggle, feature)) = parse_feature_toggle_atom(atom_text)? {
                return Ok((vec![DialectDefinitionEntry::Feature(toggle, feature)], rest));
            }
            if atom_text.is_empty() {
                return Err(DialectError::new(
                    "Dialect reference names cannot be empty.".to_owned(),
                ));
            }
            let referenced_definition = reference_resolver(atom_text)?;
            Ok((dialect_definition_entries(&referenced_definition), rest))
        }
        [
            DialectToken::OpenParen,
            DialectToken::Atom(lhs),
            DialectToken::Atom(op),
            rest @ ..,
        ] if is_swap_operator(op) => match rest {
            [
                DialectToken::Atom(rhs),
                DialectToken::CloseParen,
                after_entry @ ..,
            ] => Ok((
                vec![DialectDefinitionEntry::Cmavo(CmavoDialectEntry::Swap {
                    left: normalize_dialect_word(lhs)?,
                    right: normalize_dialect_word(rhs)?,
                })],
                after_entry,
            )),
            _ => Err(DialectError::new(
                "Swap entries must have exactly one word on each side.".to_owned(),
            )),
        },
        [
            DialectToken::OpenParen,
            DialectToken::Atom(lhs),
            DialectToken::Atom(op),
            rest @ ..,
        ] if is_expansion_operator(op) => {
            let (rhs_words, after_words) = collect_entry_words(rest);
            match after_words {
                [DialectToken::CloseParen, after_entry @ ..] => {
                    if rhs_words.is_empty() {
                        return Err(DialectError::new(
                            "Expansion entries require at least one replacement word.".to_owned(),
                        ));
                    }
                    Ok((
                        vec![DialectDefinitionEntry::Cmavo(
                            CmavoDialectEntry::Expansion {
                                source: normalize_dialect_word(lhs)?,
                                replacement: rhs_words
                                    .iter()
                                    .map(|word| normalize_dialect_word(word))
                                    .collect::<Result<_, _>>()?,
                            },
                        )],
                        after_entry,
                    ))
                }
                [] => Err(DialectError::new("Unclosed expansion entry.".to_owned())),
                [token, ..] => Err(DialectError::new(format!(
                    "Unexpected token in expansion entry: {}",
                    token.text()
                ))),
            }
        }
        [
            DialectToken::OpenParen,
            DialectToken::Atom(_),
            DialectToken::Atom(op),
            ..,
        ] => Err(DialectError::new(format!("Unknown dialect operator: {op}"))),
        [
            DialectToken::OpenParen,
            DialectToken::Atom(lhs),
            DialectToken::CloseParen,
            ..,
        ] => Err(DialectError::new(format!(
            "Dialect entry for `{lhs}` is missing an operator."
        ))),
        [DialectToken::OpenParen, DialectToken::CloseParen, ..] => Err(DialectError::new(
            "Dialect entries cannot be empty.".to_owned(),
        )),
        [DialectToken::OpenParen] => Err(DialectError::new("Unclosed dialect entry.".to_owned())),
        [DialectToken::OpenParen, token, ..] => Err(DialectError::new(format!(
            "Dialect entry must start with a word, found: {}",
            token.text()
        ))),
        [token, ..] => Err(DialectError::new(format!(
            "Expected dialect entry, found: {}",
            token.text()
        ))),
        [] => Err(DialectError::new("Expected dialect entry.".to_owned())),
    }
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|value| value.is_none_or(|(_, feature)| DialectFeature::all().contains(&feature))))]
fn parse_feature_toggle_atom(
    atom_text: &str,
) -> Result<Option<(DialectFeatureToggle, DialectFeature)>, DialectError> {
    match atom_text.chars().next() {
        Some('+') => Ok(Some((
            DialectFeatureToggle::Enable,
            parse_dialect_feature(&atom_text[1..])?,
        ))),
        Some('-') => Ok(Some((
            DialectFeatureToggle::Disable,
            parse_dialect_feature(&atom_text[1..])?,
        ))),
        _ => Ok(None),
    }
}

#[requires(!raw_feature.is_empty(), "feature toggles must name a feature")]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|feature| DialectFeature::all().contains(feature)))]
fn parse_dialect_feature(raw_feature: &str) -> Result<DialectFeature, DialectError> {
    let requested_name = strip_diacritics(raw_feature).to_ascii_uppercase();
    DialectFeature::all()
        .iter()
        .copied()
        .find(|feature| feature.atom_name() == requested_name)
        .ok_or_else(|| DialectError::new(format!("Unknown dialect feature: {requested_name}")))
}

#[ensures(ret.1.len() <= tokens.len())]
fn collect_entry_words(tokens: &[DialectToken]) -> (Vec<String>, &[DialectToken]) {
    let mut words = Vec::new();
    let mut index = 0;
    while let Some(DialectToken::Atom(word)) = tokens.get(index) {
        words.push(word.clone());
        index += 1;
    }
    (words, &tokens[index..])
}

#[expensive_ensures(ret.iter().all(|entry| match entry { DialectDefinitionEntry::Cmavo(entry) => entry.is_valid(), DialectDefinitionEntry::Feature(_, feature) => DialectFeature::all().contains(feature) }))]
fn dialect_definition_entries(definition: &DialectDefinition) -> Vec<DialectDefinitionEntry> {
    definition
        .features
        .iter()
        .copied()
        .map(|feature| DialectDefinitionEntry::Feature(DialectFeatureToggle::Enable, feature))
        .chain(
            definition
                .cmavo_entries
                .iter()
                .cloned()
                .map(DialectDefinitionEntry::Cmavo),
        )
        .collect()
}

fn definition_from_entries(entries: Vec<DialectDefinitionEntry>) -> DialectDefinition {
    let mut cmavo_entries = Vec::new();
    let mut features = BTreeSet::new();
    for entry in entries {
        match entry {
            DialectDefinitionEntry::Cmavo(cmavo_entry) => {
                cmavo_entries.push(cmavo_entry);
            }
            DialectDefinitionEntry::Feature(DialectFeatureToggle::Enable, feature) => {
                features.insert(feature);
            }
            DialectDefinitionEntry::Feature(DialectFeatureToggle::Disable, feature) => {
                features.remove(&feature);
            }
        }
    }
    DialectDefinition::new(fields! {
        cmavo_entries: cmavo_entries,
        features: features,
    })
}

fn lookup_builtin_dialect_reference(
    reference_name: &str,
) -> Result<DialectDefinition, DialectError> {
    lookup_builtin_dialect_reference_in_stack(reference_name, &[])
}

fn lookup_builtin_dialect_reference_in_stack(
    reference_name: &str,
    stack: &[&str],
) -> Result<DialectDefinition, DialectError> {
    let canonical_name = builtin_reference_canonical_name(reference_name);
    if stack.contains(&canonical_name) {
        let mut cycle: Vec<&str> = stack.iter().rev().copied().collect();
        cycle.push(canonical_name);
        return Err(DialectError::new(format!(
            "Builtin dialect reference cycle: {}",
            cycle.join(" -> ")
        )));
    }
    let sources = builtin_dialect_source_map();
    let Some(source) = sources.get(canonical_name) else {
        return Err(DialectError::new(format!(
            "Unknown dialect reference: {reference_name}"
        )));
    };
    let mut next_stack = stack.to_vec();
    next_stack.push(canonical_name);
    parse_dialect_definition_with_reference_resolver(source, &|reference| {
        lookup_builtin_dialect_reference_in_stack(reference, &next_stack)
    })
}

#[ensures(!ret.is_empty())]
fn builtin_reference_canonical_name(reference_name: &str) -> &str {
    match reference_name {
        "zantufa-cmavo" => "zantufa/cmavo",
        "zantufa-connectives" => "zantufa/connectives",
        "zantufa-terms" => "zantufa/terms",
        "zantufa-tags" => "zantufa/tags",
        "zantufa-adverbials" => "zantufa/adverbials",
        "zantufa-quotes" => "zantufa/quotes",
        "zantufa-morphology" => "zantufa/morphology",
        "zantufa-mex" => "zantufa/mex",
        other => other,
    }
}

#[ensures(!ret.is_empty())]
fn builtin_dialect_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("cbm", "(+CBM)"),
        ("gadganzu", "(+GADGANZU)"),
        ("allow-cgv", "(+ALLOW-CGV)"),
        ("case-insensitive", "(+CASE-INSENSITIVE)"),
        ("soi-adverbials", "(+SOI-ADVERBIALS)"),
        ("term-hierarchy", "(+TERM-HIERARCHY)"),
        ("zantufa/cmavo", "(+ZANTUFA-CMAVO)"),
        ("zantufa/connectives", "(+ZANTUFA-CONNECTIVES)"),
        ("zantufa/terms", "(+ZANTUFA-TERMS)"),
        ("zantufa/tags", "(+ZANTUFA-TAGS)"),
        ("zantufa/adverbials", "(+ZANTUFA-ADVERBIALS)"),
        ("zantufa/quotes", "(+ZANTUFA-QUOTES)"),
        ("zantufa/morphology", "(+ZANTUFA-MORPHOLOGY)"),
        ("zantufa/mex", "(+ZANTUFA-MEX)"),
        (
            "zantufa",
            "(cbm soi-adverbials term-hierarchy zantufa/cmavo zantufa/connectives zantufa/terms zantufa/tags zantufa/adverbials zantufa/quotes zantufa/morphology)",
        ),
        ("jboponei", "((po ↦ lo su'u) (nei ↦ kei))"),
        (
            "ce-ki-tau",
            "((ce'u 🣐 ce) (ke'a 🣐 ki) (tu'a 🣐 tau) (su'o 🣐 su))",
        ),
        ("ce-ki-tau-jau", "(ce-ki-tau (jo'u 🣐 jau))"),
        ("ce-ki-tau-joi", "(ce-ki-tau (jo'u 🣐 joi))"),
        ("ce-ki-tau-jei", "(ce-ki-tau (jo'u 🣐 jei))"),
    ]
}

#[ensures(!ret.is_empty())]
fn builtin_dialect_source_map() -> BTreeMap<&'static str, &'static str> {
    builtin_dialect_sources().into_iter().collect()
}

#[ensures(!ret.is_empty() || source.trim().is_empty())]
fn tokenize(source: &str) -> Vec<DialectToken> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        while chars.get(index).is_some_and(|value| value.is_whitespace()) {
            index += 1;
        }
        let Some(value) = chars.get(index).copied() else {
            break;
        };
        match value {
            '(' => {
                tokens.push(DialectToken::OpenParen);
                index += 1;
            }
            ')' => {
                tokens.push(DialectToken::CloseParen);
                index += 1;
            }
            _ => {
                let start = index;
                while chars
                    .get(index)
                    .is_some_and(|value| !is_atom_boundary(*value))
                {
                    index += 1;
                }
                tokens.push(DialectToken::Atom(chars[start..index].iter().collect()));
            }
        }
    }
    tokens
}

fn is_atom_boundary(value: char) -> bool {
    value.is_whitespace() || matches!(value, '(' | ')')
}

fn is_swap_operator(op: &str) -> bool {
    matches!(op, "<->" | "↔") || op == DIALECT_SWAP_OPERATOR
}

fn is_expansion_operator(op: &str) -> bool {
    matches!(op, "->" | "↦")
}

#[requires(!raw_word.is_empty(), "dialect words must not be empty")]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|word| is_normalized_cmavo(word)))]
fn normalize_dialect_word(raw_word: &str) -> Result<String, DialectError> {
    let normalized: String = raw_word
        .chars()
        .filter_map(normalize_dialect_char)
        .collect();
    parse_cmavo_form(&normalized).ok_or_else(|| {
        DialectError::new(format!(
            "Dialect token is not exactly one morphologically valid cmavo word: {raw_word}"
        ))
    })
}

#[ensures(ret -> !word.is_empty())]
fn is_normalized_cmavo(word: &str) -> bool {
    parse_cmavo_form(word).as_deref() == Some(word)
}

fn normalize_dialect_char(value: char) -> Option<char> {
    let normalized = match value {
        '\'' | 'h' | 'H' | '\u{2019}' | '\u{a78b}' | '\u{a78c}' | '\u{02bb}' | '\u{02bf}'
        | '\u{02b0}' | '\u{02d2}' => '\'',
        'Á' | 'À' | 'à' => 'á',
        'É' | 'È' | 'è' => 'é',
        'Í' | 'Ì' | 'ì' => 'í',
        'Ó' | 'Ò' | 'ò' => 'ó',
        'Ú' | 'Ù' | 'ù' => 'ú',
        'Ý' | 'Ỳ' | 'ỳ' => 'ý',
        'Ĭ' => 'ĭ',
        'Ŭ' => 'ŭ',
        _ => value.to_ascii_lowercase(),
    };
    if is_valid_normalized_char(normalized) {
        Some(normalized)
    } else {
        None
    }
}

fn strip_diacritics(text: &str) -> String {
    text.chars()
        .filter_map(|value| {
            Some(match value {
                'á' | 'Á' | 'à' | 'À' => 'a',
                'é' | 'É' | 'è' | 'È' => 'e',
                'í' | 'Í' | 'ì' | 'Ì' | 'ĭ' | 'Ĭ' => 'i',
                'ó' | 'Ó' | 'ò' | 'Ò' => 'o',
                'ú' | 'Ú' | 'ù' | 'Ù' | 'ŭ' | 'Ŭ' => 'u',
                'ý' | 'Ý' | 'ỳ' | 'Ỳ' => 'y',
                '\u{0301}' | '\u{0300}' | '\u{0306}' => return None,
                _ => value,
            })
        })
        .collect()
}

fn is_valid_normalized_char(value: char) -> bool {
    is_vowel(value) || is_consonant(value) || matches!(value, 'y' | 'ý' | '\'' | 'ĭ' | 'ŭ')
}

fn parse_cmavo_form(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || chars.first().is_some_and(|value| *value == '\'') {
        return None;
    }
    if chars.iter().all(|value| matches!(value, 'y' | 'ý')) {
        return Some(strip_diacritics(text).to_ascii_lowercase());
    }
    parse_cmavo_form_main(&chars)
}

#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn parse_cmavo_form_main(chars: &[char]) -> Option<String> {
    if starts_with_cluster(chars, 0) {
        return None;
    }
    for (onset, after_onset) in parse_onsets(chars, 0) {
        if let Some(rest) = parse_cmavo_form_tail(chars, after_onset) {
            return Some(onset + &rest);
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn parse_cmavo_form_tail(chars: &[char], start: usize) -> Option<String> {
    for (nucleus, after_nucleus) in parse_nuclei(chars, start) {
        if after_nucleus == chars.len() {
            return Some(nucleus);
        }
        if chars.get(after_nucleus) == Some(&'\'')
            && let Some(rest) = parse_cmavo_form_tail(chars, after_nucleus + 1)
        {
            return Some(format!("{nucleus}'{rest}"));
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(ret.iter().all(|(_, end)| *end >= start && *end <= chars.len()))]
fn parse_onsets(chars: &[char], start: usize) -> Vec<(String, usize)> {
    let mut onsets = Vec::new();
    if let Some((glide, end)) = parse_glide(chars, start) {
        onsets.push((glide, end));
    }
    for end in (start..=chars.len()).rev() {
        if let Some(initial) = parse_initial(chars, start, end) {
            onsets.push((initial, end));
        }
    }
    onsets
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|value| value.chars().count() == end - start))]
fn parse_initial(chars: &[char], start: usize, end: usize) -> Option<String> {
    let initial: String = chars.get(start..end)?.iter().collect();
    let valid_shape = match end - start {
        0 => true,
        1 => initial.chars().all(is_consonant),
        2 => starts_with_initial_pair(chars, start),
        3 => valid_three_consonant_initial(chars, start),
        _ => false,
    };
    valid_shape.then_some(initial)
}

#[requires(start <= chars.len())]
#[ensures(ret.iter().all(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_nuclei(chars: &[char], start: usize) -> Vec<(String, usize)> {
    let mut nuclei = Vec::new();
    if let Some((diphthong, end)) = parse_diphthong(chars, start) {
        nuclei.push((diphthong, end));
    }
    if let Some((single, end)) = parse_single_vowel(chars, start) {
        nuclei.push((single, end));
    }
    nuclei
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_diphthong(chars: &[char], start: usize) -> Option<(String, usize)> {
    let first = normalize_vowel(*chars.get(start)?);
    let second = normalize_vowel(*chars.get(start + 1)?);
    if is_diphthong_pair(first, second) {
        let output = if matches!(second, 'i') {
            format!("{first}ĭ")
        } else {
            format!("{first}ŭ")
        };
        Some((output, start + 2))
    } else {
        None
    }
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end == start + 1))]
fn parse_single_vowel(chars: &[char], start: usize) -> Option<(String, usize)> {
    let value = *chars.get(start)?;
    if is_vowel(value) || matches!(value, 'y' | 'ý') {
        Some((
            strip_diacritics(&value.to_string()).to_ascii_lowercase(),
            start + 1,
        ))
    } else {
        None
    }
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_glide(chars: &[char], start: usize) -> Option<(String, usize)> {
    let first = base_semivowel(*chars.get(start)?)?;
    if !matches!(first, 'i' | 'u') {
        return None;
    }
    if let Some((vowel, end)) = parse_single_vowel(chars, start + 1)
        && !matches!(vowel.as_str(), "i" | "u")
    {
        let glide = if first == 'i' { "ĭ" } else { "ŭ" };
        return Some((format!("{glide}{vowel}"), end));
    }
    None
}

#[requires(index <= chars.len())]
fn starts_with_cluster(chars: &[char], index: usize) -> bool {
    chars
        .get(index..index + 2)
        .is_some_and(|pair| pair.iter().copied().all(is_consonant))
}

#[requires(index <= chars.len())]
fn starts_with_initial_pair(chars: &[char], index: usize) -> bool {
    chars
        .get(index..index + 2)
        .is_some_and(|pair| is_initial_pair(pair[0], pair[1]))
}

#[requires(index <= chars.len())]
fn valid_three_consonant_initial(chars: &[char], index: usize) -> bool {
    chars.get(index..index + 3).is_some_and(|triple| {
        is_consonant(triple[0])
            && is_consonant(triple[1])
            && is_consonant(triple[2])
            && !is_sibilant(triple[0])
            && is_other_consonant(triple[1])
            && is_liquid(triple[2])
    })
}

fn is_vowel(value: char) -> bool {
    matches!(
        value,
        'a' | 'e'
            | 'i'
            | 'o'
            | 'u'
            | 'á'
            | 'é'
            | 'í'
            | 'ó'
            | 'ú'
            | 'à'
            | 'è'
            | 'ì'
            | 'ò'
            | 'ù'
            | 'ĭ'
            | 'ŭ'
    )
}

fn normalize_vowel(value: char) -> char {
    match value {
        'á' | 'à' => 'a',
        'é' | 'è' => 'e',
        'í' | 'ì' => 'i',
        'ó' | 'ò' => 'o',
        'ú' | 'ù' => 'u',
        'ý' | 'ỳ' => 'y',
        'ĭ' => 'i',
        'ŭ' => 'u',
        _ => value,
    }
}

fn is_consonant(value: char) -> bool {
    matches!(
        value,
        'b' | 'c'
            | 'd'
            | 'f'
            | 'g'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'v'
            | 'x'
            | 'z'
    )
}

fn is_sibilant(value: char) -> bool {
    matches!(value, 'c' | 'j' | 's' | 'z')
}

fn is_other_consonant(value: char) -> bool {
    matches!(
        value,
        'p' | 'b' | 'f' | 'v' | 't' | 'd' | 'x' | 'k' | 'g' | 'm' | 'n'
    )
}

fn is_liquid(value: char) -> bool {
    matches!(value, 'l' | 'r')
}

fn is_initial_pair(first: char, second: char) -> bool {
    matches!(
        (first, second),
        ('b', 'l')
            | ('b', 'r')
            | ('c', 'f')
            | ('c', 'k')
            | ('c', 'l')
            | ('c', 'm')
            | ('c', 'n')
            | ('c', 'p')
            | ('c', 'r')
            | ('c', 't')
            | ('d', 'j')
            | ('d', 'r')
            | ('d', 'z')
            | ('f', 'l')
            | ('f', 'r')
            | ('g', 'l')
            | ('g', 'r')
            | ('j', 'b')
            | ('j', 'd')
            | ('j', 'g')
            | ('j', 'm')
            | ('j', 'v')
            | ('k', 'l')
            | ('k', 'r')
            | ('m', 'r')
            | ('p', 'l')
            | ('p', 'r')
            | ('s', 'f')
            | ('s', 'k')
            | ('s', 'l')
            | ('s', 'm')
            | ('s', 'n')
            | ('s', 'p')
            | ('s', 'r')
            | ('s', 't')
            | ('t', 'c')
            | ('t', 'r')
            | ('t', 's')
            | ('v', 'l')
            | ('v', 'r')
            | ('x', 'l')
            | ('x', 'r')
            | ('z', 'b')
            | ('z', 'd')
            | ('z', 'g')
            | ('z', 'm')
            | ('z', 'v')
    )
}

fn is_diphthong_pair(first: char, second: char) -> bool {
    matches!(
        (first, second),
        ('a', 'i') | ('a', 'u') | ('e', 'i') | ('o', 'i')
    )
}

fn base_semivowel(value: char) -> Option<char> {
    match value {
        'i' | 'í' | 'ì' | 'ĭ' => Some('i'),
        'u' | 'ú' | 'ù' | 'ŭ' => Some('u'),
        _ => None,
    }
}

impl DialectToken {
    #[ensures(!ret.is_empty())]
    fn text(&self) -> String {
        match self {
            Self::OpenParen => "(".to_owned(),
            Self::CloseParen => ")".to_owned(),
            Self::Atom(value) => value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::{contract_trait, requires};

    #[test]
    fn parses_feature_only_definitions() {
        assert_eq!(
            parse_dialect_definition("(cbm)").expect("dialect").features,
            BTreeSet::from([DialectFeature::Cbm])
        );
        assert_eq!(
            parse_dialect_definition("(+CBM +GADGANZU -CBM)")
                .expect("dialect")
                .features,
            BTreeSet::from([DialectFeature::Gadganzu])
        );
    }

    #[test]
    fn parses_case_insensitive_builtin() {
        assert_eq!(
            parse_dialect_definition("(case-insensitive)")
                .expect("dialect")
                .features,
            BTreeSet::from([DialectFeature::CaseInsensitive])
        );
    }

    #[test]
    fn rejects_legacy_no_cgv_alias() {
        assert!(parse_dialect_definition("(no-cgv)").is_err());
    }

    #[test]
    fn parses_swaps_and_expansions() {
        let dialect =
            parse_dialect_definition("((ce'u <-> ce) (la'u -> la'e di'u))").expect("dialect");
        assert_eq!(
            dialect.cmavo_entries,
            vec![
                CmavoDialectEntry::Swap {
                    left: "ce'u".into(),
                    right: "ce".into(),
                },
                CmavoDialectEntry::Expansion {
                    source: "la'u".into(),
                    replacement: vec!["la'e".into(), "di'u".into()],
                },
            ]
        );
    }

    #[test]
    fn expands_builtin_references_before_explicit_entries() {
        let dialect = parse_dialect_definition("(ce-ki-tau (jo'u ↔ jau))").expect("dialect");
        assert_eq!(
            dialect.cmavo_entries,
            vec![
                CmavoDialectEntry::Swap {
                    left: "ce'u".into(),
                    right: "ce".into(),
                },
                CmavoDialectEntry::Swap {
                    left: "ke'a".into(),
                    right: "ki".into(),
                },
                CmavoDialectEntry::Swap {
                    left: "tu'a".into(),
                    right: "taŭ".into(),
                },
                CmavoDialectEntry::Swap {
                    left: "su'o".into(),
                    right: "su".into(),
                },
                CmavoDialectEntry::Swap {
                    left: "jo'u".into(),
                    right: "jaŭ".into(),
                },
            ]
        );
    }

    #[test]
    fn parses_zantufa_and_legacy_slash_aliases() {
        let zantufa = parse_dialect_definition("(zantufa)").expect("dialect");
        assert!(zantufa.features.contains(&DialectFeature::Cbm));
        assert!(
            zantufa
                .features
                .contains(&DialectFeature::ZantufaMorphology)
        );
        assert_eq!(
            parse_dialect_definition("(zantufa-cmavo)")
                .expect("dialect")
                .features,
            BTreeSet::from([DialectFeature::ZantufaCmavo])
        );
    }

    #[test]
    #[should_panic(expected = "dialect errors must have a diagnostic message")]
    fn direct_contract_violation_is_reported() {
        let _ = DialectError::new(String::new());
    }

    #[test]
    fn cmavo_transform_validity_checks_output_bounds() {
        assert!(
            CmavoDialectTransform::try_from_raw(fields!(CmavoDialectTransform {
                source_text: String::from("mi"),
                target_text: String::from("do"),
                group_key: String::from("mi->do"),
                output_index: 1,
                output_count: 1,
            }))
            .is_err()
        );
    }

    #[contract_trait]
    trait PositiveMapper {
        #[requires(value > 0, "trait precondition requires positive input")]
        #[ensures(ret > 0, "trait postcondition requires positive output")]
        fn map_positive(&self, value: i32) -> i32;
    }

    struct BadMapper;

    #[contract_trait]
    impl PositiveMapper for BadMapper {
        fn map_positive(&self, _value: i32) -> i32 {
            -1
        }
    }

    #[test]
    #[should_panic(expected = "trait precondition requires positive input")]
    fn trait_contract_precondition_is_reported_on_concrete_call() {
        let mapper = BadMapper;
        let _ = mapper.map_positive(0);
    }

    #[test]
    #[should_panic(expected = "trait postcondition requires positive output")]
    fn trait_contract_postcondition_is_reported_on_dyn_call() {
        let mapper: &dyn PositiveMapper = &BadMapper;
        let _ = mapper.map_positive(1);
    }
}
