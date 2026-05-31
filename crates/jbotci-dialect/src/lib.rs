//! Lojban dialect formula model and parser.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bityzba::{data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DIALECT_SWAP_OPERATOR: &str = "\u{1f8d0}";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
#[invariant(true)]
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
    #[requires(true)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DialectFeature {
    Cbm,
    Gadganzu,
    CaseInsensitive,
    SoiAdverbials,
    TermHierarchy,
    ZantufaAdverbials,
    ZantufaConnectives,
    ZantufaMex,
    ZantufaMorphology,
    ZantufaQuotes,
    ZantufaTags,
    ZantufaTerms,
}

impl DialectFeature {
    #[requires(true)]
    #[ensures(true)]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Cbm,
            Self::Gadganzu,
            Self::CaseInsensitive,
            Self::SoiAdverbials,
            Self::TermHierarchy,
            Self::ZantufaAdverbials,
            Self::ZantufaConnectives,
            Self::ZantufaMex,
            Self::ZantufaMorphology,
            Self::ZantufaQuotes,
            Self::ZantufaTags,
            Self::ZantufaTerms,
        ]
    }

    #[ensures(!ret.is_empty())]
    #[requires(true)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cbm => "cbm",
            Self::Gadganzu => "gadganzu",
            Self::CaseInsensitive => "case-insensitive",
            Self::SoiAdverbials => "soi-adverbials",
            Self::TermHierarchy => "term-hierarchy",
            Self::ZantufaAdverbials => "zantufa-adverbials",
            Self::ZantufaConnectives => "zantufa-connectives",
            Self::ZantufaMex => "zantufa-mex",
            Self::ZantufaMorphology => "zantufa-morphology",
            Self::ZantufaQuotes => "zantufa-quotes",
            Self::ZantufaTags => "zantufa-tags",
            Self::ZantufaTerms => "zantufa-terms",
        }
    }

    #[ensures(!ret.is_empty())]
    #[requires(true)]
    fn atom_name(self) -> String {
        self.name().to_ascii_uppercase()
    }
}

impl fmt::Display for DialectFeature {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[invariant(true)]
#[invariant(::Swap => is_normalized_cmavo(left) && is_normalized_cmavo(right))]
#[invariant(::Expansion => is_normalized_cmavo(source) && !replacement.is_empty() && replacement.iter().all(|word| is_normalized_cmavo(word)))]
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
    #[requires(true)]
    #[ensures(ret -> match self.as_data() {
        data!(CmavoDialectEntry::Swap { left, right }) => is_normalized_cmavo(left) && is_normalized_cmavo(right),
        data!(CmavoDialectEntry::Expansion { source, replacement }) => is_normalized_cmavo(source)
            && !replacement.is_empty()
            && replacement.iter().all(|word| is_normalized_cmavo(word)),
    })]
    pub fn is_valid(&self) -> bool {
        match self.as_data() {
            data!(CmavoDialectEntry::Swap { left, right }) => {
                is_normalized_cmavo(left) && is_normalized_cmavo(right)
            }
            data!(CmavoDialectEntry::Expansion {
                source,
                replacement,
            }) => {
                is_normalized_cmavo(source)
                    && !replacement.is_empty()
                    && replacement.iter().all(|word| is_normalized_cmavo(word))
            }
        }
    }
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
    #[requires(true)]
    pub fn baseline() -> Self {
        Self::default()
    }

    #[ensures(ret == self.cmavo_entries.is_empty() && self.features.is_empty())]
    #[requires(true)]
    pub fn is_baseline(&self) -> bool {
        self.cmavo_entries.is_empty() && self.features.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct BuiltinDialect {
    pub name: &'static str,
    pub definition: &'static str,
    pub dialect: DialectDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct CustomDialect {
    pub name: String,
    pub definition: String,
    pub show_in_gentufa: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct DialectSettings {
    pub custom_dialects: Vec<CustomDialect>,
    pub hidden_builtin_gentufa_dialects: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Atom(_) => true)]
#[invariant(::Group(_) => true)]
enum DialectFormulaComponent {
    Atom(String),
    Group(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct JohauShorthandSwap {
    code: char,
    left: &'static str,
    right: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Cmavo(_) => true)]
#[invariant(::Feature(_, _) => true)]
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
#[invariant(true)]
#[invariant(::Atom(_) => true)]
enum DialectToken {
    OpenParen,
    CloseParen,
    Atom(String),
}

#[requires(true)]
#[ensures(true)]
pub fn parse_dialect_definition(source: &str) -> Result<DialectDefinition, DialectError> {
    parse_dialect_definition_with_reference_resolver(source, &lookup_builtin_dialect_reference)
}

#[requires(true)]
#[ensures(true)]
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
#[requires(true)]
pub fn builtin_dialect_names() -> Vec<&'static str> {
    builtin_dialect_sources()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub fn find_builtin_dialect(requested_name: &str) -> Option<BuiltinDialect> {
    let canonical_name = builtin_reference_canonical_name(requested_name);
    builtin_dialects()
        .into_iter()
        .find(|dialect| dialect.name == canonical_name)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_dialect_definition_with_custom_dialects(
    custom_dialects: &[CustomDialect],
    source: &str,
) -> Result<DialectDefinition, DialectError> {
    parse_dialect_definition_with_reference_resolver(source, &|reference| {
        lookup_custom_or_builtin_dialect_reference(custom_dialects, reference, &[])
    })
}

#[requires(true)]
#[ensures(true)]
pub fn parse_dialect_selection_formula(
    settings: &DialectSettings,
    source: &str,
) -> Result<DialectDefinition, DialectError> {
    let trimmed = source.trim();
    if trimmed.starts_with('(') {
        parse_dialect_definition_with_custom_dialects(&settings.custom_dialects, trimmed)
    } else {
        parse_dialect_definition_with_custom_dialects(
            &settings.custom_dialects,
            &format!("({trimmed})"),
        )
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
pub fn custom_dialect_is_valid(
    existing: &[CustomDialect],
    custom: &CustomDialect,
) -> Result<(), DialectError> {
    let stripped_name = custom.name.trim();
    if stripped_name.is_empty() {
        return Err(DialectError::new("Dialect name is required.".to_owned()));
    }
    if is_builtin_dialect_reference(stripped_name) {
        return Err(DialectError::new(
            "Builtin dialect names are read-only.".to_owned(),
        ));
    }
    let duplicate_count = existing
        .iter()
        .filter(|other| other.name.trim() == stripped_name)
        .count();
    if duplicate_count > 1 {
        return Err(DialectError::new(
            "Dialect names must be unique.".to_owned(),
        ));
    }
    parse_dialect_definition_with_custom_dialects(existing, &custom.definition).map(|_| ())
}

#[requires(true)]
#[ensures(true)]
pub fn dialect_name_shows_in_gentufa_picker(dialect_name: &str) -> bool {
    !dialect_name.trim().contains('/')
}

#[requires(true)]
#[ensures(true)]
pub fn dialect_formula_top_level_references(formula_text: &str) -> Vec<String> {
    dialect_formula_components(formula_text)
        .into_iter()
        .filter_map(|component| match component {
            DialectFormulaComponent::Atom(atom)
                if !atom.is_empty() && !atom.starts_with('+') && !atom.starts_with('-') =>
            {
                Some(atom)
            }
            _ => None,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub fn add_dialect_formula_reference(dialect_name: &str, formula_text: &str) -> String {
    let clean_name = dialect_name.trim();
    if clean_name.is_empty()
        || dialect_formula_top_level_references(formula_text)
            .iter()
            .any(|reference| reference == clean_name)
    {
        return normalize_formula_text(formula_text);
    }
    let mut components = dialect_formula_components(formula_text);
    components.push(DialectFormulaComponent::Atom(clean_name.to_owned()));
    render_dialect_formula_components(&components)
}

#[requires(true)]
#[ensures(true)]
pub fn remove_dialect_formula_reference(dialect_name: &str, formula_text: &str) -> String {
    let clean_name = dialect_name.trim();
    let components = dialect_formula_components(formula_text)
        .into_iter()
        .filter(|component| component != &DialectFormulaComponent::Atom(clean_name.to_owned()))
        .collect::<Vec<_>>();
    render_dialect_formula_components(&components)
}

#[requires(true)]
#[ensures(true)]
pub fn replace_dialect_formula_reference(
    previous_name: &str,
    next_name: &str,
    formula_text: &str,
) -> String {
    let clean_previous = previous_name.trim();
    let clean_next = next_name.trim();
    if clean_previous.is_empty() {
        return normalize_formula_text(formula_text);
    }
    if clean_next.is_empty() {
        return remove_dialect_formula_reference(clean_previous, formula_text);
    }
    let components = dialect_formula_components(formula_text)
        .into_iter()
        .map(|component| match component {
            DialectFormulaComponent::Atom(atom) if atom == clean_previous => {
                DialectFormulaComponent::Atom(clean_next.to_owned())
            }
            other => other,
        })
        .collect::<Vec<_>>();
    render_dialect_formula_components(&components)
}

#[requires(true)]
#[ensures(true)]
pub fn dialect_definition_to_text(definition: &DialectDefinition) -> String {
    render_dialect_definition_entries(&dialect_definition_entries(definition))
}

#[requires(true)]
#[ensures(true)]
pub fn cmavo_dialect_entries_to_definition(entries: &[CmavoDialectEntry]) -> String {
    let definition = new!(DialectDefinition {
        cmavo_entries: entries.to_vec(),
        features: BTreeSet::new(),
    });
    dialect_definition_to_text(&definition)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
pub fn custom_dialect_definition_to_johau_uri(definition: &str) -> Result<String, DialectError> {
    parse_dialect_definition(definition)
        .and_then(|definition| dialect_definition_to_johau_uri(&definition))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
pub fn custom_dialect_definition_to_johau_uri_with_custom_dialects(
    custom_dialects: &[CustomDialect],
    definition: &str,
) -> Result<String, DialectError> {
    parse_dialect_definition_with_custom_dialects(custom_dialects, definition)
        .and_then(|definition| dialect_definition_to_johau_uri(&definition))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
pub fn dialect_definition_to_johau_uri(
    definition: &DialectDefinition,
) -> Result<String, DialectError> {
    if definition.cmavo_entries.is_empty() && definition.features.is_empty() {
        return Err(DialectError::new(
            "Dialect QR payloads require at least one entry.".to_owned(),
        ));
    }
    let mut entries = definition
        .features
        .iter()
        .copied()
        .map(|feature| Ok(dialect_feature_compact_name(feature)))
        .collect::<Result<Vec<_>, DialectError>>()?;
    entries.extend(render_compact_cmavo_entries(&definition.cmavo_entries)?);
    Ok(format!("WEB+JOHAU:{}", entries.join(".")))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
pub fn parse_johau_dialect_uri(raw_uri: &str) -> Result<DialectDefinition, DialectError> {
    let payload = johau_dialect_payload(raw_uri)?;
    let definition = parse_compact_payload(payload)?;
    if definition.cmavo_entries.is_empty() && definition.features.is_empty() {
        Err(DialectError::new(
            "Dialect QR payloads require at least one entry.".to_owned(),
        ))
    } else {
        Ok(definition)
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
pub fn import_johau_dialect_settings(
    raw_uri: &str,
    settings: &DialectSettings,
) -> Result<(String, DialectSettings), DialectError> {
    let imported = parse_johau_dialect_uri(raw_uri)?;
    let imported_definition = dialect_definition_to_text(&imported);
    let canonical_imported = canonical_dialect_definition(&imported);
    let existing_match = settings.custom_dialects.iter().find(|custom| {
        !custom.name.trim().is_empty()
            && canonical_custom_definition(settings, custom).as_ref() == Some(&canonical_imported)
    });
    let selected_name = existing_match
        .map(|custom| custom.name.trim().to_owned())
        .unwrap_or_else(|| next_johau_import_name(&settings.custom_dialects));
    let mut next_settings = settings.clone();
    if existing_match.is_none() {
        next_settings.custom_dialects.push(CustomDialect {
            name: selected_name.clone(),
            definition: imported_definition,
            show_in_gentufa: dialect_name_shows_in_gentufa_picker(&selected_name),
        });
    }
    Ok((selected_name, next_settings))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn lookup_custom_or_builtin_dialect_reference(
    custom_dialects: &[CustomDialect],
    reference_name: &str,
    stack: &[String],
) -> Result<DialectDefinition, DialectError> {
    let canonical_builtin = builtin_reference_canonical_name(reference_name);
    if let Some(dialect) = find_builtin_dialect(canonical_builtin) {
        return Ok(dialect.dialect);
    }

    if stack.iter().any(|name| name == reference_name) {
        let mut cycle = stack.iter().rev().cloned().collect::<Vec<_>>();
        cycle.push(reference_name.to_owned());
        return Err(DialectError::new(format!(
            "Dialect reference cycle: {}",
            cycle.join(" -> ")
        )));
    }

    let Some(custom) = custom_dialects
        .iter()
        .find(|custom| custom.name.trim() == reference_name)
    else {
        return Err(DialectError::new(format!(
            "Unknown dialect reference: {reference_name}"
        )));
    };
    let mut next_stack = stack.to_vec();
    next_stack.push(reference_name.to_owned());
    parse_dialect_definition_with_reference_resolver(&custom.definition, &|reference| {
        lookup_custom_or_builtin_dialect_reference(custom_dialects, reference, &next_stack)
    })
}

#[requires(true)]
#[ensures(true)]
fn is_builtin_dialect_reference(reference_name: &str) -> bool {
    find_builtin_dialect(reference_name).is_some()
}

#[requires(true)]
#[ensures(true)]
fn normalize_formula_text(formula_text: &str) -> String {
    render_dialect_formula_components(&dialect_formula_components(formula_text))
}

#[requires(true)]
#[ensures(true)]
fn dialect_formula_components(formula_text: &str) -> Vec<DialectFormulaComponent> {
    parse_formula_components(strip_outer_dialect_formula_parens(formula_text.trim()))
}

#[requires(true)]
#[ensures(true)]
fn strip_outer_dialect_formula_parens(formula_text: &str) -> &str {
    formula_text
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .unwrap_or(formula_text)
}

#[requires(true)]
#[ensures(true)]
fn parse_formula_components(raw_text: &str) -> Vec<DialectFormulaComponent> {
    let chars = raw_text.chars().collect::<Vec<_>>();
    let mut components = Vec::new();
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
                let (group_text, after_group) = collect_parenthesized_formula_group(&chars, index);
                components.push(DialectFormulaComponent::Group(group_text));
                index = after_group;
            }
            ')' => {
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
                let atom = chars[start..index].iter().collect::<String>();
                components.push(DialectFormulaComponent::Atom(atom.trim().to_owned()));
            }
        }
    }
    components
}

#[requires(start < chars.len())]
#[ensures(ret.1 > start)]
fn collect_parenthesized_formula_group(chars: &[char], start: usize) -> (String, usize) {
    let mut depth = 0usize;
    let mut text = String::new();
    let mut index = start;
    while let Some(value) = chars.get(index).copied() {
        text.push(value);
        match value {
            '(' => {
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                index += 1;
                if depth == 0 {
                    return (text, index);
                }
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    (text, index)
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_formula_components(components: &[DialectFormulaComponent]) -> String {
    let rendered = components
        .iter()
        .filter_map(|component| {
            let text = match component {
                DialectFormulaComponent::Atom(atom) | DialectFormulaComponent::Group(atom) => atom,
            };
            (!text.is_empty()).then(|| text.clone())
        })
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        String::new()
    } else {
        format!("({})", rendered.join(" "))
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_definition_entries(entries: &[DialectDefinitionEntry]) -> String {
    let rendered = entries
        .iter()
        .map(render_dialect_definition_entry)
        .collect::<Vec<_>>();
    format!("({})", rendered.join(" "))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_dialect_definition_entry(entry: &DialectDefinitionEntry) -> String {
    match entry {
        DialectDefinitionEntry::Feature(DialectFeatureToggle::Enable, feature) => {
            format!("+{}", feature.atom_name())
        }
        DialectDefinitionEntry::Feature(DialectFeatureToggle::Disable, feature) => {
            format!("-{}", feature.atom_name())
        }
        DialectDefinitionEntry::Cmavo(cmavo_entry) => match cmavo_entry.as_data() {
            data!(CmavoDialectEntry::Swap { left, right }) => {
                format!(
                    "({} {DIALECT_SWAP_OPERATOR} {})",
                    definition_cmavo_word(left),
                    definition_cmavo_word(right)
                )
            }
            data!(CmavoDialectEntry::Expansion {
                source,
                replacement,
            }) => {
                format!(
                    "({} ↦ {})",
                    definition_cmavo_word(source),
                    replacement
                        .iter()
                        .map(|word| definition_cmavo_word(word))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn definition_cmavo_word(word: &str) -> String {
    strip_diacritics(word).to_ascii_lowercase()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn dialect_feature_compact_name(feature: DialectFeature) -> String {
    feature.name().replace('-', "").to_ascii_uppercase()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn render_compact_cmavo_entries(
    entries: &[CmavoDialectEntry],
) -> Result<Vec<String>, DialectError> {
    let mut rendered = Vec::new();
    let mut index = 0;
    while let Some(entry) = entries.get(index) {
        if let Some(first_code) = common_swap_code(entry) {
            let mut codes = vec![first_code];
            index += 1;
            while let Some(next_entry) = entries.get(index) {
                let Some(code) = common_swap_code(next_entry) else {
                    break;
                };
                if codes.contains(&code) {
                    break;
                }
                codes.push(code);
                index += 1;
            }
            rendered.push(format!("-{}", codes.into_iter().collect::<String>()));
        } else {
            rendered.push(render_compact_cmavo_entry(entry)?);
            index += 1;
        }
    }
    Ok(rendered)
}

#[requires(entry.is_valid())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn render_compact_cmavo_entry(entry: &CmavoDialectEntry) -> Result<String, DialectError> {
    match entry.as_data() {
        data!(CmavoDialectEntry::Swap { left, right }) => Ok(format!(
            "{}-{}",
            cmavo_to_compact(left)?,
            cmavo_to_compact(right)?
        )),
        data!(CmavoDialectEntry::Expansion {
            source,
            replacement,
        }) => {
            if replacement.is_empty() {
                return Err(DialectError::new(
                    "Expansion entries require at least one replacement word.".to_owned(),
                ));
            }
            Ok(format!(
                "{}*{}",
                cmavo_to_compact(source)?,
                replacement
                    .iter()
                    .map(|word| cmavo_to_compact(word))
                    .collect::<Result<Vec<_>, _>>()?
                    .join("+")
            ))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn johau_dialect_payload(raw_uri: &str) -> Result<&str, DialectError> {
    let stripped = raw_uri.trim();
    let Some((scheme, payload)) = stripped.split_once(':') else {
        return Err(DialectError::new(
            "Dialect QR URI must use the web+johau scheme.".to_owned(),
        ));
    };
    if !scheme.eq_ignore_ascii_case("web+johau") {
        return Err(DialectError::new(
            "Dialect QR URI must use the web+johau scheme.".to_owned(),
        ));
    }
    if payload.is_empty() {
        Err(DialectError::new("Dialect QR payload is empty.".to_owned()))
    } else {
        Ok(payload)
    }
}

#[requires(!payload.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn parse_compact_payload(payload: &str) -> Result<DialectDefinition, DialectError> {
    let compact_entries = split_compact(".", "entry", payload)?;
    let mut entries = Vec::new();
    for entry in compact_entries {
        entries.extend(parse_compact_entry(entry)?);
    }
    Ok(definition_from_entries(entries))
}

#[requires(!entry.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn parse_compact_entry(entry: &str) -> Result<Vec<DialectDefinitionEntry>, DialectError> {
    if let Some(shorthand_codes) = entry.strip_prefix('-') {
        return parse_common_swap_shorthand(shorthand_codes).map(|entries| {
            entries
                .into_iter()
                .map(DialectDefinitionEntry::Cmavo)
                .collect()
        });
    }
    let swap_count = entry.matches('-').count();
    let expansion_count = entry.matches('*').count();
    match (swap_count, expansion_count) {
        (1, 0) => {
            let Some((left_compact, right_compact)) = entry.split_once('-') else {
                return Err(DialectError::new(
                    "Swap entries must contain exactly one `-` separator.".to_owned(),
                ));
            };
            Ok(vec![DialectDefinitionEntry::Cmavo(new!(
                CmavoDialectEntry::Swap {
                    left: compact_to_cmavo(left_compact)?,
                    right: compact_to_cmavo(right_compact)?,
                }
            ))])
        }
        (0, 1) => {
            let Some((source_compact, replacement_compact)) = entry.split_once('*') else {
                return Err(DialectError::new(
                    "Expansion entries must contain exactly one `*` separator.".to_owned(),
                ));
            };
            let replacement_parts = split_compact("+", "replacement word", replacement_compact)?;
            Ok(vec![DialectDefinitionEntry::Cmavo(new!(
                CmavoDialectEntry::Expansion {
                    source: compact_to_cmavo(source_compact)?,
                    replacement: replacement_parts
                        .iter()
                        .map(|part| compact_to_cmavo(part))
                        .collect::<Result<Vec<_>, _>>()?,
                }
            ))])
        }
        (0, 0) => Ok(vec![DialectDefinitionEntry::Feature(
            DialectFeatureToggle::Enable,
            parse_compact_dialect_feature(entry)?,
        )]),
        _ => Err(DialectError::new(
            "Dialect QR entries must contain exactly one swap `-` or expansion `*` operator."
                .to_owned(),
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn parse_common_swap_shorthand(
    shorthand_codes: &str,
) -> Result<Vec<CmavoDialectEntry>, DialectError> {
    if shorthand_codes.is_empty() {
        return Err(DialectError::new(
            "Dialect QR common-swap shorthand cannot be empty.".to_owned(),
        ));
    }
    let swaps = shorthand_codes
        .chars()
        .map(common_swap_for_code)
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    if swaps.iter().any(|swap| !seen.insert(swap.code)) {
        return Err(DialectError::new(
            "Dialect QR common-swap shorthand cannot repeat a swap code.".to_owned(),
        ));
    }
    Ok(swaps
        .into_iter()
        .map(|swap| {
            Ok(new!(CmavoDialectEntry::Swap {
                left: normalize_dialect_word(swap.left)?,
                right: normalize_dialect_word(swap.right)?,
            }))
        })
        .collect::<Result<Vec<_>, DialectError>>()?)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn common_swap_for_code(raw_code: char) -> Result<JohauShorthandSwap, DialectError> {
    let code = raw_code.to_ascii_uppercase();
    common_johau_shorthand_swaps()
        .into_iter()
        .find(|swap| swap.code == code)
        .ok_or_else(|| {
            DialectError::new(format!(
                "Dialect QR common-swap shorthand can only contain C, T, K, V, D, or S: {raw_code}"
            ))
        })
}

#[requires(entry.is_valid())]
#[ensures(true)]
fn common_swap_code(entry: &CmavoDialectEntry) -> Option<char> {
    match entry.as_data() {
        data!(CmavoDialectEntry::Swap { left, right }) => {
            find_common_swap(left, right).map(|swap| swap.code)
        }
        data!(CmavoDialectEntry::Expansion { .. }) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn find_common_swap(raw_left: &str, raw_right: &str) -> Option<JohauShorthandSwap> {
    let canonical_left = canonical_cmavo(raw_left)?;
    let canonical_right = canonical_cmavo(raw_right)?;
    common_johau_shorthand_swaps().into_iter().find(|swap| {
        (canonical_left == swap.left && canonical_right == swap.right)
            || (canonical_left == swap.right && canonical_right == swap.left)
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn common_johau_shorthand_swaps() -> Vec<JohauShorthandSwap> {
    vec![
        JohauShorthandSwap {
            code: 'C',
            left: "ce",
            right: "ce'u",
        },
        JohauShorthandSwap {
            code: 'T',
            left: "tau",
            right: "tu'a",
        },
        JohauShorthandSwap {
            code: 'K',
            left: "ki",
            right: "ke'a",
        },
        JohauShorthandSwap {
            code: 'V',
            left: "voi",
            right: "poi'i",
        },
        JohauShorthandSwap {
            code: 'D',
            left: "du",
            right: "du'u",
        },
        JohauShorthandSwap {
            code: 'S',
            left: "su",
            right: "su'o",
        },
    ]
}

#[requires(!separator.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn split_compact<'a>(
    separator: &str,
    label: &str,
    value: &'a str,
) -> Result<Vec<&'a str>, DialectError> {
    if value.is_empty() {
        return Err(DialectError::new(format!(
            "Dialect QR {label} cannot be empty."
        )));
    }
    if value.ends_with(separator) {
        return Err(DialectError::new(format!(
            "Dialect QR {label} list cannot end with `{separator}`."
        )));
    }
    let parts = value.split(separator).collect::<Vec<_>>();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(DialectError::new(format!(
            "Dialect QR {label} list cannot contain empty items."
        )));
    }
    Ok(parts)
}

#[requires(!raw_word.is_empty(), "compact cmavo encoding requires a word")]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn cmavo_to_compact(raw_word: &str) -> Result<String, DialectError> {
    let normalized = normalize_dialect_word(raw_word)?;
    strip_diacritics(&normalized)
        .chars()
        .map(encode_compact_cmavo_char)
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn encode_compact_cmavo_char(value: char) -> Result<char, DialectError> {
    if value == '\'' {
        Ok('H')
    } else if value.is_ascii_lowercase() {
        Ok(value.to_ascii_uppercase())
    } else if value.is_ascii_uppercase() {
        Ok(value)
    } else {
        Err(DialectError::new(format!(
            "Dialect cmavo contains a character that cannot be encoded in a QR payload: {value}"
        )))
    }
}

#[requires(!raw_compact.is_empty(), "compact cmavo decoding requires a token")]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn compact_to_cmavo(raw_compact: &str) -> Result<String, DialectError> {
    let decoded = raw_compact
        .chars()
        .map(decode_compact_cmavo_char)
        .collect::<Result<String, _>>()?;
    let normalized = normalize_dialect_word(&decoded)?;
    if cmavo_to_compact(&normalized).as_deref() == Ok(&raw_compact.to_ascii_uppercase()) {
        Ok(strip_diacritics(&normalized).to_ascii_lowercase())
    } else {
        Err(DialectError::new(format!(
            "Dialect QR token is not exactly one morphologically valid cmavo word: {raw_compact}"
        )))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn decode_compact_cmavo_char(value: char) -> Result<char, DialectError> {
    let upper = value.to_ascii_uppercase();
    if upper == 'H' {
        Ok('\'')
    } else if upper.is_ascii_uppercase() {
        Ok(upper.to_ascii_lowercase())
    } else {
        Err(DialectError::new(format!(
            "Dialect QR cmavo words can only contain ASCII letters: {value}"
        )))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|word| !word.is_empty()))]
fn canonical_cmavo(raw_word: &str) -> Option<String> {
    normalize_dialect_word(raw_word)
        .ok()
        .map(|word| strip_diacritics(&word).to_ascii_lowercase())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.message().is_empty()))]
fn parse_compact_dialect_feature(raw_feature: &str) -> Result<DialectFeature, DialectError> {
    let requested_name = strip_diacritics(raw_feature).to_ascii_uppercase();
    DialectFeature::all()
        .iter()
        .copied()
        .find(|feature| dialect_feature_compact_name(*feature) == requested_name)
        .ok_or_else(|| DialectError::new(format!("Unknown dialect feature: {requested_name}")))
}

#[requires(true)]
#[ensures(true)]
fn canonical_dialect_definition(definition: &DialectDefinition) -> DialectDefinition {
    new!(DialectDefinition {
        cmavo_entries: canonical_cmavo_dialect_entries(&definition.cmavo_entries),
        features: definition.features.clone(),
    })
}

#[requires(true)]
#[ensures(ret.iter().all(CmavoDialectEntry::is_valid))]
fn canonical_cmavo_dialect_entries(entries: &[CmavoDialectEntry]) -> Vec<CmavoDialectEntry> {
    entries.iter().map(canonical_cmavo_dialect_entry).collect()
}

#[requires(entry.is_valid())]
#[ensures(ret.is_valid())]
fn canonical_cmavo_dialect_entry(entry: &CmavoDialectEntry) -> CmavoDialectEntry {
    match entry.as_data() {
        data!(CmavoDialectEntry::Swap { left, right }) => {
            let left_key = strip_diacritics(left).to_ascii_lowercase();
            let right_key = strip_diacritics(right).to_ascii_lowercase();
            if left_key <= right_key {
                new!(CmavoDialectEntry::Swap {
                    left: left.clone(),
                    right: right.clone(),
                })
            } else {
                new!(CmavoDialectEntry::Swap {
                    left: right.clone(),
                    right: left.clone(),
                })
            }
        }
        data!(CmavoDialectEntry::Expansion {
            source,
            replacement,
        }) => new!(CmavoDialectEntry::Expansion {
            source: source.clone(),
            replacement: replacement.clone(),
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn canonical_custom_definition(
    settings: &DialectSettings,
    custom: &CustomDialect,
) -> Option<DialectDefinition> {
    parse_dialect_definition_with_custom_dialects(&settings.custom_dialects, &custom.definition)
        .ok()
        .map(|definition| canonical_dialect_definition(&definition))
}

#[requires(true)]
#[ensures(!ret.trim().is_empty())]
fn next_johau_import_name(custom_dialects: &[CustomDialect]) -> String {
    let existing_names = custom_dialects
        .iter()
        .map(|custom| custom.name.trim())
        .collect::<BTreeSet<_>>();
    let base_name = "jo'au import";
    if !existing_names.contains(base_name) {
        return base_name.to_owned();
    }
    for index in 2.. {
        let candidate = format!("{base_name} {index}");
        if !existing_names.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded import-name sequence must contain a free candidate")
}

#[requires(!source.is_empty(), "builtin dialect definitions must not be empty")]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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
#[requires(true)]
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
#[requires(true)]
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
#[requires(true)]
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
                vec![DialectDefinitionEntry::Cmavo(new!(
                    CmavoDialectEntry::Swap {
                        left: normalize_dialect_word(lhs)?,
                        right: normalize_dialect_word(rhs)?,
                    }
                ))],
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
                        vec![DialectDefinitionEntry::Cmavo(new!(
                            CmavoDialectEntry::Expansion {
                                source: normalize_dialect_word(lhs)?,
                                replacement: rhs_words
                                    .iter()
                                    .map(|word| normalize_dialect_word(word))
                                    .collect::<Result<_, _>>()?,
                            }
                        ))],
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
#[requires(true)]
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
#[requires(true)]
fn collect_entry_words(tokens: &[DialectToken]) -> (Vec<String>, &[DialectToken]) {
    let mut words = Vec::new();
    let mut index = 0;
    while let Some(DialectToken::Atom(word)) = tokens.get(index) {
        words.push(word.clone());
        index += 1;
    }
    (words, &tokens[index..])
}

#[requires(true)]
#[ensures(ret.iter().all(|entry| match entry { DialectDefinitionEntry::Cmavo(entry) => entry.is_valid(), DialectDefinitionEntry::Feature(_, feature) => DialectFeature::all().contains(feature) }))]
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

#[requires(true)]
#[ensures(true)]
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
    new!(DialectDefinition {
        cmavo_entries: cmavo_entries,
        features: features,
    })
}

#[requires(true)]
#[ensures(true)]
fn lookup_builtin_dialect_reference(
    reference_name: &str,
) -> Result<DialectDefinition, DialectError> {
    lookup_builtin_dialect_reference_in_stack(reference_name, &[])
}

#[requires(true)]
#[ensures(true)]
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
#[requires(true)]
fn builtin_reference_canonical_name(reference_name: &str) -> &str {
    match reference_name {
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
#[requires(true)]
fn builtin_dialect_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("cbm", "(+CBM)"),
        ("gadganzu", "(+GADGANZU)"),
        ("case-insensitive", "(+CASE-INSENSITIVE)"),
        ("soi-adverbials", "(+SOI-ADVERBIALS)"),
        ("term-hierarchy", "(+TERM-HIERARCHY)"),
        ("zantufa/connectives", "(+ZANTUFA-CONNECTIVES)"),
        ("zantufa/terms", "(+ZANTUFA-TERMS)"),
        ("zantufa/tags", "(+ZANTUFA-TAGS)"),
        ("zantufa/adverbials", "(+ZANTUFA-ADVERBIALS)"),
        ("zantufa/quotes", "(+ZANTUFA-QUOTES)"),
        ("zantufa/morphology", "(+ZANTUFA-MORPHOLOGY)"),
        ("zantufa/mex", "(+ZANTUFA-MEX)"),
        (
            "zantufa",
            "(cbm soi-adverbials term-hierarchy zantufa/connectives zantufa/terms zantufa/tags zantufa/adverbials zantufa/quotes zantufa/morphology)",
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
#[requires(true)]
fn builtin_dialect_source_map() -> BTreeMap<&'static str, &'static str> {
    builtin_dialect_sources().into_iter().collect()
}

#[ensures(!ret.is_empty() || source.trim().is_empty())]
#[requires(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_atom_boundary(value: char) -> bool {
    value.is_whitespace() || matches!(value, '(' | ')')
}

#[requires(true)]
#[ensures(true)]
fn is_swap_operator(op: &str) -> bool {
    matches!(op, "<->" | "↔") || op == DIALECT_SWAP_OPERATOR
}

#[requires(true)]
#[ensures(true)]
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
#[requires(true)]
fn is_normalized_cmavo(word: &str) -> bool {
    parse_cmavo_form(word).as_deref() == Some(word)
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_valid_normalized_char(value: char) -> bool {
    is_vowel(value) || is_consonant(value) || matches!(value, 'y' | 'ý' | '\'' | 'ĭ' | 'ŭ')
}

#[requires(true)]
#[ensures(true)]
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
#[requires(true)]
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
#[ensures(true)]
fn starts_with_cluster(chars: &[char], index: usize) -> bool {
    chars
        .get(index..index + 2)
        .is_some_and(|pair| pair.iter().copied().all(is_consonant))
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_with_initial_pair(chars: &[char], index: usize) -> bool {
    chars
        .get(index..index + 2)
        .is_some_and(|pair| is_initial_pair(pair[0], pair[1]))
}

#[requires(index <= chars.len())]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_sibilant(value: char) -> bool {
    matches!(value, 'c' | 'j' | 's' | 'z')
}

#[requires(true)]
#[ensures(true)]
fn is_other_consonant(value: char) -> bool {
    matches!(
        value,
        'p' | 'b' | 'f' | 'v' | 't' | 'd' | 'x' | 'k' | 'g' | 'm' | 'n'
    )
}

#[requires(true)]
#[ensures(true)]
fn is_liquid(value: char) -> bool {
    matches!(value, 'l' | 'r')
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_diphthong_pair(first: char, second: char) -> bool {
    matches!(
        (first, second),
        ('a', 'i') | ('a', 'u') | ('e', 'i') | ('o', 'i')
    )
}

#[requires(true)]
#[ensures(true)]
fn base_semivowel(value: char) -> Option<char> {
    match value {
        'i' | 'í' | 'ì' | 'ĭ' => Some('i'),
        'u' | 'ú' | 'ù' | 'ŭ' => Some('u'),
        _ => None,
    }
}

impl DialectToken {
    #[ensures(!ret.is_empty())]
    #[requires(true)]
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
    use bityzba::{contract_trait, ensures, invariant, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
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
    #[requires(true)]
    #[ensures(true)]
    fn parses_case_insensitive_builtin() {
        assert_eq!(
            parse_dialect_definition("(case-insensitive)")
                .expect("dialect")
                .features,
            BTreeSet::from([DialectFeature::CaseInsensitive])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_legacy_no_cgv_alias() {
        assert!(parse_dialect_definition("(no-cgv)").is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_swaps_and_expansions() {
        let dialect =
            parse_dialect_definition("((ce'u <-> ce) (la'u -> la'e di'u))").expect("dialect");
        assert_eq!(
            dialect.cmavo_entries,
            vec![
                new!(CmavoDialectEntry::Swap {
                    left: "ce'u".into(),
                    right: "ce".into(),
                }),
                new!(CmavoDialectEntry::Expansion {
                    source: "la'u".into(),
                    replacement: vec!["la'e".into(), "di'u".into()],
                }),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn expands_builtin_references_before_explicit_entries() {
        let dialect = parse_dialect_definition("(ce-ki-tau (jo'u ↔ jau))").expect("dialect");
        assert_eq!(
            dialect.cmavo_entries,
            vec![
                new!(CmavoDialectEntry::Swap {
                    left: "ce'u".into(),
                    right: "ce".into(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "ke'a".into(),
                    right: "ki".into(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "tu'a".into(),
                    right: "taŭ".into(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "su'o".into(),
                    right: "su".into(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "jo'u".into(),
                    right: "jaŭ".into(),
                }),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zantufa_and_legacy_slash_aliases() {
        let zantufa = parse_dialect_definition("(zantufa)").expect("dialect");
        assert!(zantufa.features.contains(&DialectFeature::Cbm));
        assert!(
            zantufa
                .features
                .contains(&DialectFeature::ZantufaMorphology)
        );
        assert!(parse_dialect_definition("(zantufa-cmavo)").is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn edits_dialect_formula_references_without_touching_inline_entries() {
        let swap = format!("((ce'u {DIALECT_SWAP_OPERATOR} ce))");
        assert_eq!(add_dialect_formula_reference("cbm", ""), "(cbm)");
        assert_eq!(
            add_dialect_formula_reference("gadganzu", &format!("(cbm {swap})")),
            format!("(cbm {swap} gadganzu)")
        );
        assert_eq!(
            remove_dialect_formula_reference("gadganzu", &format!("(cbm {swap} gadganzu)")),
            format!("(cbm {swap})")
        );
        assert_eq!(
            replace_dialect_formula_reference("custom", "renamed", "(ce-ki-tau custom -CBM)"),
            "(ce-ki-tau renamed -CBM)"
        );
        assert_eq!(
            dialect_formula_top_level_references(&format!("(cbm {swap} +GADGANZU renamed)")),
            vec!["cbm".to_owned(), "renamed".to_owned()]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn validates_and_resolves_custom_dialect_settings() {
        let custom = CustomDialect {
            name: "custom-base".to_owned(),
            definition: format!("(cbm (ce'u {DIALECT_SWAP_OPERATOR} ce))"),
            show_in_gentufa: true,
        };
        let referencing = CustomDialect {
            name: "custom-derived".to_owned(),
            definition: "(custom-base gadganzu)".to_owned(),
            show_in_gentufa: true,
        };
        assert!(custom_dialect_is_valid(&[custom.clone(), referencing.clone()], &custom).is_ok());
        let resolved = parse_dialect_definition_with_custom_dialects(
            &[custom.clone(), referencing.clone()],
            "(custom-derived)",
        )
        .expect("custom dialect");
        assert!(resolved.features.contains(&DialectFeature::Cbm));
        assert!(resolved.features.contains(&DialectFeature::Gadganzu));
        assert_eq!(resolved.cmavo_entries.len(), 1);

        let duplicate = CustomDialect {
            name: "custom-base".to_owned(),
            definition: "()".to_owned(),
            show_in_gentufa: true,
        };
        assert!(custom_dialect_is_valid(&[custom.clone(), duplicate.clone()], &duplicate).is_err());
        let builtin_alias = CustomDialect {
            name: "zantufa-connectives".to_owned(),
            definition: "()".to_owned(),
            show_in_gentufa: true,
        };
        assert!(custom_dialect_is_valid(&[builtin_alias.clone()], &builtin_alias).is_err());

        let first_cycle = CustomDialect {
            name: "first".to_owned(),
            definition: "(second)".to_owned(),
            show_in_gentufa: true,
        };
        let second_cycle = CustomDialect {
            name: "second".to_owned(),
            definition: "(first)".to_owned(),
            show_in_gentufa: true,
        };
        assert!(
            parse_dialect_definition_with_custom_dialects(&[first_cycle, second_cycle], "(first)")
                .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn imports_and_exports_johau_dialect_payloads() {
        let definition = format!("((ce'u {DIALECT_SWAP_OPERATOR} ce) (la'u ↦ la'e di'u))");
        let canonical = format!("((ce {DIALECT_SWAP_OPERATOR} ce'u) (la'u ↦ la'e di'u))");
        assert_eq!(
            custom_dialect_definition_to_johau_uri(&definition).expect("Johau URI"),
            "WEB+JOHAU:-C.LAHU*LAHE+DIHU"
        );
        assert_eq!(
            dialect_definition_to_text(
                &parse_johau_dialect_uri("web+johau:-C.LAHU*LAHE+DIHU").expect("Johau payload")
            ),
            canonical
        );
        assert_eq!(
            custom_dialect_definition_to_johau_uri("(ce-ki-tau (jo'u ↔ jei))").expect("Johau URI"),
            "WEB+JOHAU:-CKTS.JOHU-JEI"
        );
        assert_eq!(
            custom_dialect_definition_to_johau_uri_with_custom_dialects(
                &[CustomDialect {
                    name: "custom-base".to_owned(),
                    definition: format!("(cbm (ce'u {DIALECT_SWAP_OPERATOR} ce))"),
                    show_in_gentufa: true,
                }],
                "(custom-base)",
            )
            .expect("Johau URI"),
            "WEB+JOHAU:CBM.-C"
        );
        assert!(parse_johau_dialect_uri("WEB+JOHAU:NOCGV").is_err());
        assert!(parse_johau_dialect_uri("WEB+JOHAU:-CC").is_err());

        let (imported_name, imported_settings) = import_johau_dialect_settings(
            "WEB+JOHAU:-C.LAHU*LAHE+DIHU",
            &DialectSettings::default(),
        )
        .expect("import");
        assert_eq!(imported_name, "jo'au import");
        assert_eq!(imported_settings.custom_dialects.len(), 1);
        assert_eq!(imported_settings.custom_dialects[0].definition, canonical);

        let existing = DialectSettings {
            custom_dialects: vec![CustomDialect {
                name: "already here".to_owned(),
                definition,
                show_in_gentufa: true,
            }],
            hidden_builtin_gentufa_dialects: BTreeSet::new(),
        };
        let (reused_name, reused_settings) =
            import_johau_dialect_settings("WEB+JOHAU:-C.LAHU*LAHE+DIHU", &existing)
                .expect("import");
        assert_eq!(reused_name, "already here");
        assert_eq!(reused_settings.custom_dialects.len(), 1);
    }

    #[test]
    #[should_panic(expected = "dialect errors must have a diagnostic message")]
    #[requires(true)]
    #[ensures(true)]
    fn direct_contract_violation_is_reported() {
        let _ = DialectError::new(String::new());
    }

    #[contract_trait]
    trait PositiveMapper {
        #[requires(value > 0, "trait precondition requires positive input")]
        #[ensures(ret > 0, "trait postcondition requires positive output")]
        fn map_positive(&self, value: i32) -> i32;
    }

    #[invariant(true)]
    struct BadMapper;

    #[contract_trait]
    impl PositiveMapper for BadMapper {
        #[requires(true)]
        #[ensures(true)]
        fn map_positive(&self, _value: i32) -> i32 {
            -1
        }
    }

    #[test]
    #[should_panic(expected = "trait precondition requires positive input")]
    #[ensures(true)]
    #[requires(true)]
    fn trait_contract_precondition_is_reported_on_concrete_call() {
        let mapper = BadMapper;
        let _ = mapper.map_positive(0);
    }

    #[test]
    #[should_panic(expected = "trait postcondition requires positive output")]
    #[ensures(true)]
    #[requires(true)]
    fn trait_contract_postcondition_is_reported_on_dyn_call() {
        let mapper: &dyn PositiveMapper = &BadMapper;
        let _ = mapper.map_positive(1);
    }
}
