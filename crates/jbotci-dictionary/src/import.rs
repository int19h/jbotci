//! Lensisku JSON import support.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::expensive_ensures;
use bityzba::{invariant, requires};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

use crate::{DefinitionId, Score, WordType};

/// Imported Lensisku dictionary snapshot.
///
/// A Lensisku export may carry several definitions of the same word: since
/// upstream migration `V157` the `positive_scores_only=false` export returns
/// every definition row rather than the best one per word. Use
/// [`ImportedDictionary::retain_best_definition_per_word`] to reduce a snapshot
/// to the one-definition-per-word shape jbotci embeds.
#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct ImportedDictionary {
    pub entries: Vec<ImportedDictionaryEntry>,
}

impl ImportedDictionary {
    /// Discard entries whose definition text is empty, returning how many were
    /// dropped.
    ///
    /// Lensisku's unfiltered export contains definition rows that never got any
    /// text — as of the 2026-09-01 English export, nine of them, every one
    /// negatively scored. A row that defines nothing is not a dictionary entry
    /// and [`crate::Dictionary::validate`] rejects it outright.
    ///
    /// Run this *before* [`Self::retain_best_definition_per_word`]: a word that
    /// also has a real definition then keeps it even when the empty row would
    /// have outranked it, and a word whose every definition is empty drops out
    /// of the dictionary entirely rather than being embedded as a blank.
    #[requires(true)]
    #[ensures(
        self.entries.len() + ret == old(self.entries.len()),
        "every entry is either kept or counted as dropped"
    )]
    #[expensive_ensures(
        self.entries.iter().all(|entry| !entry.definition.is_empty()),
        "no entry survives without definition text"
    )]
    pub fn retain_defined_entries(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|entry| !entry.definition.is_empty());
        before - self.entries.len()
    }

    /// Reduce the snapshot to a single definition per word, returning how many
    /// entries were dropped.
    ///
    /// The surviving definition of each word is the one Lensisku itself would
    /// have picked: the highest vote score wins, and the lowest definition id
    /// breaks a tie. That is verbatim the `ORDER BY f.score DESC,
    /// f.definitionid ASC` of upstream's `export_best_definitions()`, which is
    /// still what `positive_scores_only=true` uses to choose among a word's
    /// positive-scored definitions; applying it to an unfiltered export
    /// extends the same rule to words that have no positive-scored definition
    /// at all.
    ///
    /// Words are compared by their exact text rather than by
    /// [`crate::normalize_lookup_query`]: the duplicates this removes are
    /// several `valsi` rows spelling one word, whereas two words that merely
    /// normalize alike are distinct dictionary entries that must both survive.
    ///
    /// Entry order is otherwise preserved, so the embedded dictionary keeps
    /// the export's own ordering.
    #[requires(true)]
    #[ensures(
        self.entries.len() + ret == old(self.entries.len()),
        "every entry is either kept or counted as dropped"
    )]
    #[expensive_ensures(
        self.entries
            .iter()
            .map(|entry| entry.word.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == self.entries.len(),
        "no word keeps more than one definition"
    )]
    pub fn retain_best_definition_per_word(&mut self) -> usize {
        let mut best: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, entry) in self.entries.iter().enumerate() {
            match best.entry(entry.word.as_str()) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(index);
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    if entry.outranks(&self.entries[*slot.get()]) {
                        slot.insert(index);
                    }
                }
            }
        }

        // `best` borrows `self.entries`, so collect the verdict before the
        // retain pass takes it mutably.
        let mut kept = vec![false; self.entries.len()];
        for index in best.into_values() {
            kept[index] = true;
        }
        let dropped = kept.iter().filter(|keep| !**keep).count();
        let mut index = 0;
        self.entries.retain(|_| {
            let keep = kept[index];
            index += 1;
            keep
        });
        dropped
    }
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
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    pub selmaho: Option<String>,
    #[serde(default)]
    pub etymology: Option<String>,
    #[serde(default)]
    pub jargon: Option<String>,
    pub user: ImportedDictionaryUser,
}

impl ImportedDictionaryEntry {
    /// Report whether this entry beats `other` as the definition of their word.
    ///
    /// Scores are compared with [`f64::total_cmp`] so the ranking stays a total
    /// order for every value serde can hand back, including a non-finite score
    /// that [`crate::Dictionary::validate`] would later reject.
    #[requires(true)]
    #[ensures(
        !ret || !(self.score.0 < other.score.0),
        "a winning entry never scores below the entry it displaces"
    )]
    fn outranks(&self, other: &Self) -> bool {
        match self.score.0.total_cmp(&other.score.0) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => self.definition_id < other.definition_id,
        }
    }
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
#[expensive_ensures(ret.as_ref().is_ok_and(|dictionary| {
    dictionary.entries.iter().all(|entry| {
        entry.selmaho.as_ref().is_none_or(|text| !text.trim().is_empty())
            && entry.rafsi.iter().all(|rafsi| !rafsi.is_empty())
    })
}) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|value| value.as_ref().is_none_or(|text| !text.trim().is_empty())) || ret.is_err())]
fn deserialize_optional_non_empty_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.filter(|value| !value.trim().is_empty()))
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
#[ensures(ret.as_ref().is_ok_and(|values| values.iter().all(|value| !value.is_empty())) || ret.is_err())]
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
#[ensures(ret.iter().all(|value| !value.is_empty()))]
fn split_rafsi_text(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(!word.is_empty())]
    #[ensures(ret.word == word && ret.definition_id == DefinitionId(definition_id))]
    fn entry(word: &str, definition_id: u64, score: f64) -> ImportedDictionaryEntry {
        ImportedDictionaryEntry {
            word: word.to_owned(),
            word_type: WordType::Gismu,
            definition: format!("definition {definition_id}"),
            definition_id: DefinitionId(definition_id),
            notes: String::new(),
            score: Score(score),
            gloss_keywords: Vec::new(),
            place_keywords: Vec::new(),
            rafsi: Vec::new(),
            selmaho: None,
            etymology: None,
            jargon: None,
            user: ImportedDictionaryUser {
                username: "tester".to_owned(),
                realname: None,
            },
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn undefined_entries_are_discarded_before_selection() {
        let mut blank = entry("mlatu", 10, 5.0);
        blank.definition = String::new();
        let mut dictionary = ImportedDictionary {
            entries: vec![blank, entry("mlatu", 11, 1.0)],
        };
        // The blank outscores the real definition, so discarding it first is
        // what keeps `mlatu` defined at all.
        assert_eq!(dictionary.retain_defined_entries(), 1);
        assert_eq!(dictionary.retain_best_definition_per_word(), 0);
        assert_eq!(dictionary.entries[0].definition_id, DefinitionId(11));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_word_with_only_undefined_entries_drops_out() {
        let mut blank = entry("mlatu", 10, 0.0);
        blank.definition = String::new();
        let mut dictionary = ImportedDictionary {
            entries: vec![blank, entry("broda", 11, 0.0)],
        };
        assert_eq!(dictionary.retain_defined_entries(), 1);
        assert_eq!(
            dictionary
                .entries
                .iter()
                .map(|entry| entry.word.as_str())
                .collect::<Vec<_>>(),
            vec!["broda"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn best_definition_selection_prefers_the_highest_score() {
        let mut dictionary = ImportedDictionary {
            entries: vec![
                entry("mlatu", 10, 0.0),
                entry("mlatu", 11, 3.0),
                entry("mlatu", 12, -1.0),
            ],
        };
        assert_eq!(dictionary.retain_best_definition_per_word(), 2);
        assert_eq!(
            dictionary
                .entries
                .iter()
                .map(|entry| entry.definition_id)
                .collect::<Vec<_>>(),
            vec![DefinitionId(11)]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn best_definition_selection_breaks_score_ties_by_lowest_definition_id() {
        let mut dictionary = ImportedDictionary {
            entries: vec![entry("mlatu", 12, 2.0), entry("mlatu", 11, 2.0)],
        };
        assert_eq!(dictionary.retain_best_definition_per_word(), 1);
        assert_eq!(dictionary.entries[0].definition_id, DefinitionId(11));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn best_definition_selection_keeps_export_order_and_distinct_words() {
        let mut dictionary = ImportedDictionary {
            entries: vec![
                entry("broda", 1, 0.0),
                entry("mlatu", 2, 0.0),
                entry("mlatu", 3, 1.0),
                entry("zbasu", 4, 0.0),
            ],
        };
        assert_eq!(dictionary.retain_best_definition_per_word(), 1);
        assert_eq!(
            dictionary
                .entries
                .iter()
                .map(|entry| entry.word.as_str())
                .collect::<Vec<_>>(),
            vec!["broda", "mlatu", "zbasu"]
        );
        assert_eq!(dictionary.entries[1].definition_id, DefinitionId(3));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn best_definition_selection_keeps_words_that_only_normalize_alike() {
        let mut dictionary = ImportedDictionary {
            entries: vec![entry("ba'e", 1, 0.0), entry("bahe", 2, 0.0)],
        };
        assert_eq!(dictionary.retain_best_definition_per_word(), 0);
        assert_eq!(dictionary.entries.len(), 2);
    }

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
    fn parses_empty_lensisku_snapshot() {
        let dictionary = parse_lensisku_json("[]").expect("empty Lensisku JSON");

        assert!(dictionary.entries.is_empty());
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_rafsi_list_without_empty_segments() {
        let json = r#"[
            {
                "word": "banli",
                "word_type": "gismu",
                "definition": "great",
                "definition_id": 1,
                "score": 1.0,
                "rafsi": ["ban     bau", "", "   "],
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid rafsi field");
        assert_eq!(dictionary.entries[0].rafsi, vec!["ban", "bau"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_blank_selmaho_as_absent() {
        let json = r#"[
            {
                "word": "brode",
                "word_type": "experimental gismu",
                "selmaho": "",
                "definition": "predicate variable 2",
                "definition_id": 1,
                "score": 1.0,
                "user": {"username": "test"}
            },
            {
                "word": "brodi",
                "word_type": "experimental gismu",
                "selmaho": "   ",
                "definition": "predicate variable 3",
                "definition_id": 2,
                "score": 1.0,
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid entries");
        assert_eq!(dictionary.entries[0].selmaho, None);
        assert_eq!(dictionary.entries[1].selmaho, None);
    }
}
