//! Lensisku JSON import support.

use bityzba::{invariant, requires};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

use crate::{DefinitionId, Score, WordType};

/// Imported Lensisku dictionary snapshot.
#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct ImportedDictionary {
    pub entries: Vec<ImportedDictionaryEntry>,
}

/// Owned Lensisku dictionary entry.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct ImportedDictionaryEntry {
    pub word: String,
    #[serde(rename = "word_type")]
    pub word_type: WordType,
    pub definition: String,
    pub definition_id: DefinitionId,
    #[serde(default, deserialize_with = "deserialize_empty_string_for_null")]
    pub notes: String,
    pub score: Score,
    #[serde(default, deserialize_with = "deserialize_keyword_vec")]
    pub gloss_keywords: Vec<ImportedKeyword>,
    #[serde(default, deserialize_with = "deserialize_keyword_vec")]
    pub place_keywords: Vec<ImportedKeyword>,
    #[serde(default, deserialize_with = "deserialize_rafsi_vec")]
    pub rafsi: Vec<String>,
    #[serde(default)]
    pub selmaho: Option<String>,
    #[serde(default)]
    pub etymology: Option<String>,
    #[serde(default)]
    pub jargon: Option<String>,
    pub user: ImportedDictionaryUser,
}

/// Owned imported keyword.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct ImportedKeyword {
    pub word: String,
    pub meaning: Option<String>,
}

/// Owned imported contributor metadata.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
pub struct ImportedDictionaryUser {
    pub username: String,
    #[serde(default)]
    pub realname: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[invariant(true)]
#[invariant(::Text(..) => true)]
#[invariant(::List(..) => true)]
enum RafsiField {
    Text(String),
    List(Vec<String>),
}

/// Lensisku import error.
#[derive(Debug, Error)]
#[invariant(true)]
#[invariant(::Json(..) => true)]
pub enum LensiskuImportError {
    #[error("failed to parse Lensisku dictionary JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Parse a Lensisku JSON dictionary snapshot.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|dictionary| !dictionary.entries.is_empty()))]
pub fn parse_lensisku_json(input: &str) -> Result<ImportedDictionary, LensiskuImportError> {
    let entries = serde_json::from_str::<Vec<ImportedDictionaryEntry>>(input)?;
    Ok(ImportedDictionary { entries })
}

#[requires(true)]
#[ensures(true)]
fn deserialize_empty_string_for_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[requires(true)]
#[ensures(true)]
fn deserialize_keyword_vec<'de, D>(deserializer: D) -> Result<Vec<ImportedKeyword>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<ImportedKeyword>>::deserialize(deserializer)?.unwrap_or_default())
}

#[requires(true)]
#[ensures(true)]
fn deserialize_rafsi_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(field) = Option::<RafsiField>::deserialize(deserializer)? else {
        return Ok(Vec::new());
    };
    let values = match field {
        RafsiField::Text(value) => split_rafsi_text(&value),
        RafsiField::List(values) => values
            .iter()
            .flat_map(|value| split_rafsi_text(value))
            .collect(),
    };
    Ok(values)
}

#[requires(true)]
#[ensures(true)]
fn split_rafsi_text(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_current_lensisku_shape() {
        let json = r#"[
            {
                "word": "a",
                "word_type": "cmavo",
                "selmaho": "A",
                "definition": "logical connective: sumti afterthought or.",
                "definition_id": 1339,
                "notes": null,
                "score": 100003.0,
                "gloss_keywords": [{"word": "or", "meaning": "inclusive or"}],
                "user": {"username": "officialdata", "realname": "Official Data"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid Lensisku JSON");
        let entry = &dictionary.entries[0];
        assert_eq!(entry.word, "a");
        assert_eq!(entry.word_type, WordType::Cmavo);
        assert_eq!(entry.notes, "");
        assert_eq!(
            entry.gloss_keywords[0].meaning.as_deref(),
            Some("inclusive or")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_unknown_word_type() {
        let json = r#"[
            {
                "word": "x",
                "word_type": "mystery",
                "definition": "bad",
                "definition_id": 1,
                "score": 1.0,
                "user": {"username": "test"}
            }
        ]"#;

        assert!(parse_lensisku_json(json).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_unknown_entry_field() {
        let json = r#"[
            {
                "word": "x",
                "word_type": "cmavo",
                "definition": "bad",
                "definition_id": 1,
                "score": 1.0,
                "user": {"username": "test"},
                "unexpected": true
            }
        ]"#;

        assert!(parse_lensisku_json(json).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_whitespace_padded_rafsi() {
        let json = r#"[
            {
                "word": "banli",
                "word_type": "gismu",
                "definition": "great",
                "definition_id": 1,
                "score": 1.0,
                "rafsi": "ban     bau",
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid rafsi field");
        assert_eq!(dictionary.entries[0].rafsi, vec!["ban", "bau"]);
    }
}
