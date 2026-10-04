//! Lensisku JSON import support.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::expensive_ensures;
use bityzba::{invariant, new, requires};
use serde::{Deserialize, Deserializer};
use thiserror::Error;

use crate::{
    DefinitionId, RafsiClaimKind, RafsiSource, Score, WordType, normalize_lookup_query,
    universal_gismu_rafsi_forms,
};

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
    /// How many of the export's rows were not dictionary words at all (such as
    /// Lensisku `wiki` articles) and were therefore set aside at parse time.
    pub non_word_row_count: usize,
}

impl ImportedDictionary {
    /// Return how many rows the export held, words and non-words alike.
    ///
    /// This is the snapshot's `definition_count` as long as it is taken before
    /// any of the `retain_*` reductions run.
    #[requires(true)]
    #[ensures(ret == self.entries.len() + self.non_word_row_count)]
    pub fn row_count(&self) -> usize {
        self.entries.len() + self.non_word_row_count
    }

    /// Discard entries whose definition text is empty, returning how many were
    /// dropped.
    ///
    /// Lensisku's unfiltered export contains definition rows that never got any
    /// text — nine of them in the 2026-09-01 English export, none positively
    /// scored. The score is beside the point, though: a row that defines
    /// nothing is not a dictionary entry, whatever its votes say, and
    /// [`crate::Dictionary::validate`] rejects it outright.
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
    #[expensive_ensures(
        old(self.entries.clone()).iter().all(|candidate| {
            self.entries
                .iter()
                .find(|survivor| survivor.word == candidate.word)
                .is_some_and(|survivor| !candidate.outranks(survivor))
        }),
        "every word's survivor is the entry its whole pre-call group ranks highest"
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
///
/// Built by [`parse_lensisku_json`] from an export row that is a word, so
/// downstream code never encounters a non-word row.
#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct ImportedDictionaryEntry {
    pub word: String,
    pub word_type: WordType,
    pub definition: String,
    pub definition_id: DefinitionId,
    pub notes: String,
    pub score: Score,
    pub gloss_keywords: Vec<ImportedKeyword>,
    pub place_keywords: Vec<ImportedKeyword>,
    /// The row's structured rafsi, each with the standing of the Lensisku
    /// column it came from (`rafsi` is official, `experimental_rafsi` is
    /// experimental), less the derived 4-letter form.
    pub rafsi: Vec<ImportedRafsi>,
    pub selmaho: Option<String>,
    pub etymology: Option<String>,
    pub jargon: Option<String>,
    pub user: ImportedDictionaryUser,
}

/// One imported rafsi with the standing of its assignment.
#[invariant(!form.is_empty(), "a rafsi is a non-empty form")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedRafsi {
    pub form: String,
    pub standing: RafsiClaimKind,
}

/// One row of a Lensisku dictionary export, exactly as serialized.
///
/// Lensisku records rafsi in two columns: `rafsi` for official assignments and
/// `experimental_rafsi` for experimental ones. [`Self::into_entry`] keeps each
/// form with its column's standing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct LensiskuRow {
    word: String,
    word_type: LensiskuRowKind,
    definition: String,
    definition_id: DefinitionId,
    #[serde(default, deserialize_with = "deserialize_empty_string_for_null")]
    notes: String,
    score: Score,
    #[serde(default, deserialize_with = "deserialize_keyword_vec")]
    gloss_keywords: Vec<ImportedKeyword>,
    #[serde(default, deserialize_with = "deserialize_keyword_vec")]
    place_keywords: Vec<ImportedKeyword>,
    #[serde(default, deserialize_with = "deserialize_rafsi_vec")]
    rafsi: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_rafsi_vec")]
    experimental_rafsi: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    selmaho: Option<String>,
    #[serde(default)]
    etymology: Option<String>,
    #[serde(default)]
    jargon: Option<String>,
    user: ImportedDictionaryUser,
}

impl LensiskuRow {
    /// Convert a word row into an entry of type `word_type`.
    ///
    /// Each form keeps the standing of the column it came from. An
    /// experimental rafsi may sit on any word (Lensisku gives the official
    /// cmavo `ma` the experimental rafsi `maz`), but a word whose own standing
    /// is experimental cannot hold an official assignment, and a form listed
    /// in both columns has no single standing. The conversion refuses both
    /// rather than guessing.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|entry| {
        entry.word_type == word_type
            && (word_type.rafsi_claim_kind() == RafsiClaimKind::Official
                || entry
                    .rafsi
                    .iter()
                    .all(|rafsi| rafsi.standing == RafsiClaimKind::Experimental))
    }) || ret.is_err())]
    fn into_entry(
        self,
        word_type: WordType,
    ) -> Result<ImportedDictionaryEntry, LensiskuImportError> {
        let mut official = self.rafsi;
        let mut experimental = self.experimental_rafsi;
        // The derivable form is dropped from both columns before the checks:
        // it adds nothing whatever column upstream filed it under.
        discard_universal_short_rafsi(&self.word, word_type, &mut official);
        discard_universal_short_rafsi(&self.word, word_type, &mut experimental);
        if !official.is_empty() && word_type.rafsi_claim_kind() != RafsiClaimKind::Official {
            return Err(LensiskuImportError::RafsiStandingMismatch {
                word: self.word,
                definition_id: self.definition_id.get(),
                word_type,
            });
        }
        if let Some(form) = official.iter().find(|form| experimental.contains(form)) {
            return Err(LensiskuImportError::RafsiListedTwice {
                word: self.word,
                definition_id: self.definition_id.get(),
                form: form.clone(),
            });
        }
        let rafsi = official
            .into_iter()
            .map(|form| (form, RafsiClaimKind::Official))
            .chain(
                experimental
                    .into_iter()
                    .map(|form| (form, RafsiClaimKind::Experimental)),
            )
            .map(|(form, standing)| {
                new!(ImportedRafsi {
                    form: form,
                    standing: standing
                })
            })
            .collect();
        Ok(ImportedDictionaryEntry {
            word: self.word,
            word_type,
            definition: self.definition,
            definition_id: self.definition_id,
            notes: self.notes,
            score: self.score,
            gloss_keywords: self.gloss_keywords,
            place_keywords: self.place_keywords,
            rafsi,
            selmaho: self.selmaho,
            etymology: self.etymology,
            jargon: self.jargon,
            user: self.user,
        })
    }
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

/// The `word_type` column of a Lensisku export row.
///
/// Lensisku's dictionary export also carries rows that are not words. Such a
/// row is a free-form article stored in the `valsi` table (the first, seen in
/// 2026-10, is a Markdown write-up titled "Periodic-table gismu assignment
/// algorithm"), so it has no word form, no place structure, and no business
/// in word lookup or in the embedding corpus. It is recognised by name so that
/// any *other* unknown type still fails the import, as a new word class must
/// be classified consciously in [`WordType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Word(..) => true)]
#[invariant(::Wiki => true)]
enum LensiskuRowKind {
    Word(WordType),
    /// Lensisku's `wiki` type: an article rather than a word.
    Wiki,
}

impl<'de> Deserialize<'de> for LensiskuRowKind {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::IntoDeserializer;

        let name = String::deserialize(deserializer)?;
        if name == "wiki" {
            return Ok(Self::Wiki);
        }
        // Delegate to `WordType` so an unknown name keeps serde's error that
        // lists every accepted word type.
        WordType::deserialize(name.into_deserializer()).map(Self::Word)
    }
}

/// Drop the forms that are `word`'s own derived 4-letter rafsi.
///
/// Since 2026-09 Lensisku lists a gismu's 4-letter rafsi (the gismu minus its
/// final vowel, e.g. `celd` for `celdi`) as a structured rafsi on gismu and
/// experimental gismu that have rafsi at all. jbotci derives that form itself
/// ([`RafsiSource::UniversalShort`]), so listing it adds nothing, and keeping
/// it would look like an upstream rafsi assignment to the extracted-rafsi
/// audit and the rafsi index. Exactly the form jbotci derives is dropped,
/// compared after lookup normalization, so the two can never disagree: a form
/// jbotci does not derive, such as `brod` (the broda family is the CLL
/// exception) or a short rafsi like `ba'u`, always survives.
#[requires(true)]
#[ensures(
    universal_gismu_rafsi_forms(word)
        .iter()
        .filter(|(_, source)| *source == RafsiSource::UniversalShort)
        .all(|(derived, _)| {
            !word_type.is_gismu_like()
                || forms.iter().all(|form| normalize_lookup_query(form) != *derived)
        }),
    "no surviving form of a gismu-like word is its derived 4-letter rafsi"
)]
#[ensures(forms.len() <= old(forms.len()))]
fn discard_universal_short_rafsi(word: &str, word_type: WordType, forms: &mut Vec<String>) {
    if !word_type.is_gismu_like() {
        return;
    }
    let Some((derived, _)) = universal_gismu_rafsi_forms(word)
        .into_iter()
        .find(|(_, source)| *source == RafsiSource::UniversalShort)
    else {
        return;
    };
    forms.retain(|form| normalize_lookup_query(form) != derived);
}

/// Lensisku import error.
#[derive(Debug, Error)]
#[invariant(true)]
#[invariant(::Json(..) => true)]
#[invariant(::NonWordRowWithWordData { .. } => true)]
#[invariant(::RafsiStandingMismatch { .. } => true)]
#[invariant(::RafsiListedTwice { .. } => true)]
pub enum LensiskuImportError {
    #[error("failed to parse Lensisku dictionary JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A non-word row carries rafsi or a selma'o. Dropping it would silently
    /// discard a word-level claim, which the fail-closed rafsi audit exists to
    /// prevent, so the import refuses instead.
    #[error(
        "Lensisku row `{word}` (definition {definition_id}) is not a word but carries \
         rafsi or a selma'o"
    )]
    NonWordRowWithWordData { word: String, definition_id: u64 },
    /// A word whose own standing is experimental lists an official rafsi.
    /// An experimental or obsolete word cannot bind the official register.
    #[error(
        "Lensisku row `{word}` (definition {definition_id}) is a {} word but lists an \
         official rafsi",
        word_type.as_str()
    )]
    RafsiStandingMismatch {
        word: String,
        definition_id: u64,
        word_type: WordType,
    },
    /// A form appears in both rafsi columns of one row, so it has no single
    /// standing.
    #[error(
        "Lensisku row `{word}` (definition {definition_id}) lists rafsi `{form}` as both \
         official and experimental"
    )]
    RafsiListedTwice {
        word: String,
        definition_id: u64,
        form: String,
    },
}

/// Parse a Lensisku JSON dictionary snapshot.
///
/// Non-word rows (Lensisku `wiki` articles) are counted in
/// [`ImportedDictionary::non_word_row_count`] and otherwise discarded.
#[requires(true)]
#[expensive_ensures(ret.as_ref().is_ok_and(|dictionary| {
    dictionary.entries.iter().all(|entry| {
        entry.selmaho.as_ref().is_none_or(|text| !text.trim().is_empty())
            && entry.rafsi.iter().all(|rafsi| !rafsi.form.is_empty())
    })
}) || ret.is_err())]
pub fn parse_lensisku_json(input: &str) -> Result<ImportedDictionary, LensiskuImportError> {
    let rows = serde_json::from_str::<Vec<LensiskuRow>>(input)?;
    let mut entries = Vec::with_capacity(rows.len());
    let mut non_word_row_count = 0;
    for row in rows {
        match row.word_type {
            LensiskuRowKind::Word(word_type) => entries.push(row.into_entry(word_type)?),
            LensiskuRowKind::Wiki => {
                if !row.rafsi.is_empty()
                    || !row.experimental_rafsi.is_empty()
                    || row.selmaho.is_some()
                {
                    return Err(LensiskuImportError::NonWordRowWithWordData {
                        word: row.word,
                        definition_id: row.definition_id.get(),
                    });
                }
                non_word_row_count += 1;
            }
        }
    }
    Ok(ImportedDictionary {
        entries,
        non_word_row_count,
    })
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

    #[requires(true)]
    #[ensures(ret.len() == entry.rafsi.len())]
    fn rafsi_forms(entry: &ImportedDictionaryEntry) -> Vec<&str> {
        entry
            .rafsi
            .iter()
            .map(|rafsi| rafsi.form.as_str())
            .collect()
    }

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
            non_word_row_count: 0,
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
            non_word_row_count: 0,
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
            non_word_row_count: 0,
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
            non_word_row_count: 0,
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
            non_word_row_count: 0,
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
            non_word_row_count: 0,
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

        // The error comes through `LensiskuRowKind`'s delegation to
        // `WordType`, which must keep serde's list of accepted types.
        let Err(LensiskuImportError::Json(error)) = parse_lensisku_json(json) else {
            panic!("an unknown word type must fail JSON parsing");
        };
        let message = error.to_string();
        assert!(message.contains("unknown variant `mystery`"), "{message}");
        assert!(message.contains("`experimental gismu`"), "{message}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn accepts_experimental_rafsi_on_every_experimental_standing_type() {
        for word_type in ["nalvla", "obsolete cmavo", "obsolete fu'ivla"] {
            let json = format!(
                r#"[
                    {{
                        "word": "zbaxu",
                        "word_type": "{word_type}",
                        "definition": "x",
                        "definition_id": 1,
                        "score": 1.0,
                        "experimental_rafsi": "zbx",
                        "user": {{"username": "test"}}
                    }}
                ]"#
            );
            let dictionary = parse_lensisku_json(&json).expect("experimental standing type");
            assert_eq!(rafsi_forms(&dictionary.entries[0]), ["zbx"]);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_brod_because_jbotci_never_derives_it() {
        let json = r#"[
            {
                "word": "brodi",
                "word_type": "gismu",
                "definition": "x",
                "definition_id": 1,
                "score": 1.0,
                "rafsi": "brod",
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid entry");
        assert_eq!(rafsi_forms(&dictionary.entries[0]), ["brod"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sets_wiki_rows_aside() {
        let json = r#"[
            {
                "word": "Periodic-table gismu assignment algorithm",
                "word_type": "wiki",
                "definition": "**Periodic-table gismu assignment algorithm**",
                "definition_id": 1,
                "score": 0.0,
                "user": {"username": "test"}
            },
            {
                "word": "a",
                "word_type": "cmavo",
                "definition": "or",
                "definition_id": 2,
                "score": 1.0,
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("wiki rows are recognised");
        assert_eq!(dictionary.non_word_row_count, 1);
        assert_eq!(dictionary.row_count(), 2);
        assert_eq!(
            dictionary
                .entries
                .iter()
                .map(|entry| entry.word.as_str())
                .collect::<Vec<_>>(),
            vec!["a"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_wiki_rows_carrying_word_data() {
        for extra in [
            r#""rafsi": "pat""#,
            r#""experimental_rafsi": "pat""#,
            r#""selmaho": "UI""#,
        ] {
            let json = format!(
                r#"[
                    {{
                        "word": "article",
                        "word_type": "wiki",
                        "definition": "text",
                        "definition_id": 7,
                        "score": 0.0,
                        {extra},
                        "user": {{"username": "test"}}
                    }}
                ]"#
            );
            assert!(matches!(
                parse_lensisku_json(&json),
                Err(LensiskuImportError::NonWordRowWithWordData {
                    definition_id: 7,
                    ..
                })
            ));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn discards_only_the_four_letter_rafsi_of_gismu() {
        let json = r#"[
            {
                "word": "bacru",
                "word_type": "gismu",
                "definition": "utter",
                "definition_id": 1,
                "score": 1.0,
                "rafsi": "bac ba'u bacr",
                "user": {"username": "test"}
            },
            {
                "word": "celdi",
                "word_type": "experimental gismu",
                "definition": "x",
                "definition_id": 2,
                "score": 1.0,
                "rafsi": "celd",
                "user": {"username": "test"}
            },
            {
                "word": "gu'e",
                "word_type": "cmavo",
                "selmaho": "GUhA",
                "definition": "and",
                "definition_id": 3,
                "score": 1.0,
                "rafsi": "gu'e",
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid entries");
        assert_eq!(rafsi_forms(&dictionary.entries[0]), ["bac", "ba'u"]);
        assert!(dictionary.entries[1].rafsi.is_empty());
        assert_eq!(rafsi_forms(&dictionary.entries[2]), ["gu'e"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn merges_experimental_rafsi_without_the_four_letter_form() {
        let json = r#"[
            {
                "word": "kenjo",
                "word_type": "experimental gismu",
                "definition": "x",
                "definition_id": 1,
                "score": 1.0,
                "rafsi": null,
                "experimental_rafsi": "kej kenj",
                "user": {"username": "test"}
            },
            {
                "word": "so'y",
                "word_type": "experimental cmavo",
                "definition": "x",
                "definition_id": 2,
                "score": 1.0,
                "experimental_rafsi": "sox",
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid entries");
        assert_eq!(rafsi_forms(&dictionary.entries[0]), ["kej"]);
        assert_eq!(rafsi_forms(&dictionary.entries[1]), ["sox"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_each_rafsi_with_the_standing_of_its_column() {
        // Lensisku gives the official cmavo `ma` only the experimental rafsi
        // `maz`, and an official word may carry both kinds at once.
        let json = r#"[
            {
                "word": "ma",
                "word_type": "cmavo",
                "selmaho": "KOhA",
                "definition": "x",
                "definition_id": 1,
                "score": 1.0,
                "experimental_rafsi": "maz",
                "user": {"username": "test"}
            },
            {
                "word": "ckeji",
                "word_type": "gismu",
                "definition": "x",
                "definition_id": 2,
                "score": 1.0,
                "rafsi": "cke kej",
                "experimental_rafsi": "cki",
                "user": {"username": "test"}
            }
        ]"#;

        let dictionary = parse_lensisku_json(json).expect("valid entries");
        let standings = |index: usize| {
            dictionary.entries[index]
                .rafsi
                .iter()
                .map(|rafsi| (rafsi.form.as_str(), rafsi.standing))
                .collect::<Vec<_>>()
        };
        assert_eq!(standings(0), [("maz", RafsiClaimKind::Experimental)]);
        assert_eq!(
            standings(1),
            [
                ("cke", RafsiClaimKind::Official),
                ("kej", RafsiClaimKind::Official),
                ("cki", RafsiClaimKind::Experimental),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_an_official_rafsi_on_an_experimental_word() {
        let json = r#"[
            {
                "word": "kenjo",
                "word_type": "experimental gismu",
                "definition": "x",
                "definition_id": 9,
                "score": 1.0,
                "rafsi": "kej",
                "user": {"username": "test"}
            }
        ]"#;
        assert!(matches!(
            parse_lensisku_json(json),
            Err(LensiskuImportError::RafsiStandingMismatch {
                definition_id: 9,
                word_type: WordType::ExperimentalGismu,
                ..
            })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_a_rafsi_listed_in_both_columns() {
        let json = r#"[
            {
                "word": "ma",
                "word_type": "cmavo",
                "definition": "x",
                "definition_id": 3,
                "score": 1.0,
                "rafsi": "maz",
                "experimental_rafsi": "maz",
                "user": {"username": "test"}
            }
        ]"#;
        assert!(matches!(
            parse_lensisku_json(json),
            Err(LensiskuImportError::RafsiListedTwice { definition_id: 3, ref form, .. })
                if form == "maz"
        ));
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
        assert_eq!(rafsi_forms(&dictionary.entries[0]), ["ban", "bau"]);
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
        assert_eq!(rafsi_forms(&dictionary.entries[0]), ["ban", "bau"]);
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
