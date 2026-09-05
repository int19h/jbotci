extern crate bityzba;

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use bityzba::{invariant, new, requires};
use jbotci_dictionary::import::{
    ImportedDictionary, ImportedDictionaryEntry, ImportedDictionaryUser, ImportedKeyword,
    parse_lensisku_json,
};
use jbotci_dictionary::{
    CmavoSequenceIndexEntry, Dictionary, DictionaryEntry, DictionaryLujvoEntry,
    DictionaryLujvoSegment, DictionaryLujvoSegmentKind, DictionaryPatternEntry,
    DictionarySoundEntry, DictionaryUser, EntryIndex, Keyword, OwnedCmavoSequenceIndexEntry,
    OwnedDictionaryIndexes, OwnedPatternIndexEntry, OwnedRafsiIndexEntry, OwnedSelmahoIndexEntry,
    OwnedWordIndexEntry, Rafsi, RafsiIndexEntry, RafsiIndexTarget, RafsiSource, RawSelmaho,
    SelmahoIndexEntry, WordIndexEntry, WordType, build_owned_indexes, normalize_lookup_query,
    universal_gismu_rafsi_forms,
};
use jbotci_jvozba::decompose_lujvo_like;
use jbotci_morphology::{LujvoPart, possible_short_rafsi_forms};
use jbotci_phonetic::{
    IpaSegmentId, IpaTokenSequenceView, PronunciationTargetId, PronunciationTargetSequenceView,
    lojban_text_to_pronunciation_targets, lojban_text_to_tokenized_ipa,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use rayon::prelude::*;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

const VENDORED_DICTIONARY: &str = "data/dictionary-en.json";
const VENDORED_METADATA: &str = "data/dictionary-en.metadata.toml";
const VENDORED_EXTRACTED_RAFSI: &str = "data/extracted-rafsi-en.json";

/// Provenance of the vendored snapshot, as written by `cargo xtask
/// vendor-dictionary` (see `data/README.md`).
///
/// `definition_count` counts the rows of the vendored JSON and `entry_count`
/// the entries that survive best-definition selection; the two differ whenever
/// the export carries several definitions of one word.
#[invariant(
    entry_count <= definition_count,
    "selection drops definitions, never invents them"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DictionaryMetadata {
    language_tag: String,
    language_realname: String,
    source_language_tag: String,
    format: String,
    positive_scores_only: bool,
    filename: String,
    metadata_url: String,
    download_url: String,
    lensisku_created_at: String,
    sha256: String,
    definition_count: usize,
    entry_count: usize,
}

/// Vendored table of rafsi recovered from prose that the snapshot never
/// recorded structurally (see `data/README.md` and jbotci issue #768).
#[invariant(
    !rafsi.is_empty(),
    "an empty table means the vendored file lost its payload"
)]
#[invariant(
    rafsi.iter().all(|(word, forms)| {
        !word.is_empty()
            && !forms.is_empty()
            && forms.windows(2).all(|pair| pair[0] < pair[1])
    }),
    "every listed word carries at least one rafsi, sorted and duplicate free"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedRafsiTable {
    provenance: ExtractedRafsiProvenance,
    // Deserialized through a duplicate-rejecting visitor: collecting straight
    // into a map would let a repeated JSON key silently discard one of the two
    // assignments before any invariant or collision check could see it.
    #[serde(deserialize_with = "deserialize_unique_rafsi_table")]
    rafsi: BTreeMap<String, Vec<String>>,
}

/// Provenance of the extraction run that produced [`ExtractedRafsiTable`].
///
/// The build never consumes these fields beyond checking that they are filled
/// in; they exist so the vendored data documents its own origin.
#[invariant(
    !run_date.is_empty() && !method.is_empty() && !tooling.is_empty(),
    "provenance must name the run date, method, and tooling"
)]
#[invariant(
    models.len() > 1
        && models
            .iter()
            .enumerate()
            .all(|(index, model)| !model.is_empty() && !models[..index].contains(model)),
    "a vote needs at least two voters, each named exactly once"
)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedRafsiProvenance {
    run_date: String,
    models: Vec<String>,
    method: String,
    tooling: String,
}

/// Deserialize the word-to-rafsi table, rejecting repeated words.
///
/// `serde` collapses duplicate map keys silently, which would drop one of two
/// conflicting assignments before the merge could complain about it. The
/// vendored file is fail-closed data, so a repeated word is an error.
#[requires(true)]
#[ensures(true)]
fn deserialize_unique_rafsi_table<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(UniqueRafsiTableVisitor)
}

struct UniqueRafsiTableVisitor;

impl<'de> Visitor<'de> for UniqueRafsiTableVisitor {
    type Value = BTreeMap<String, Vec<String>>;

    #[requires(true)]
    #[ensures(true)]
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a map of dictionary word to its extracted rafsi")
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut table = BTreeMap::new();
        while let Some((word, forms)) = access.next_entry::<String, Vec<String>>()? {
            if let Some(previous) = table.insert(word.clone(), forms) {
                return Err(de::Error::custom(format!(
                    "extracted rafsi word `{word}` is listed more than once \
                     (first listing: {previous:?})"
                )));
            }
        }
        Ok(table)
    }
}

/// Who already holds a rafsi form while the extracted table is merged.
#[invariant(!word.is_empty(), "a claim always names its claimant")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct RafsiClaim {
    word: String,
    origin: RafsiClaimOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum RafsiClaimOrigin {
    /// The snapshot records the form as a structured rafsi of the entry.
    SnapshotListed,
    /// The form is a universal rafsi of a gismu-like snapshot entry.
    SnapshotUniversal,
    /// An earlier word of the extracted table claimed the form.
    Extracted,
}

impl RafsiClaimOrigin {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    const fn describe(self) -> &'static str {
        match self {
            Self::SnapshotListed => "a listed rafsi of",
            Self::SnapshotUniversal => "a universal gismu rafsi of",
            Self::Extracted => "an extracted rafsi of",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct GeneratedSoundEntry {
    entry_index: EntryIndex,
    ipa: String,
    segments: Vec<IpaSegmentId>,
    self_similarity: f64,
    pronunciation_targets: Vec<PronunciationTargetId>,
    pronunciation_self_similarity: f64,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct GeneratedLujvoEntry {
    entry_index: EntryIndex,
    segments: Vec<GeneratedLujvoSegment>,
    source_words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GeneratedLujvoSegment {
    kind: DictionaryLujvoSegmentKind,
    surface: String,
    source_word: Option<String>,
}

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
    if let Err(error) = run() {
        panic!("failed to generate embedded dictionary: {error}");
    }
}

#[requires(true)]
#[ensures(true)]
fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let dictionary_path = manifest_dir.join(VENDORED_DICTIONARY);
    let metadata_path = manifest_dir.join(VENDORED_METADATA);
    let extracted_rafsi_path = manifest_dir.join(VENDORED_EXTRACTED_RAFSI);
    println!("cargo:rerun-if-changed={}", dictionary_path.display());
    println!("cargo:rerun-if-changed={}", metadata_path.display());
    println!("cargo:rerun-if-changed={}", extracted_rafsi_path.display());
    println!("cargo:rerun-if-env-changed=JBOTCI_DICTIONARY_BUILD_TIMINGS");
    emit_build_timing(format_args!(
        "rayon threads: {}",
        rayon::current_num_threads()
    ));

    let input = timed_stage("read dictionary json", || {
        fs::read_to_string(&dictionary_path)
    })?;
    let metadata = timed_stage("load dictionary metadata", || {
        load_dictionary_metadata(&metadata_path)
    })?;
    let mut imported = timed_stage("parse lensisku json", || parse_lensisku_json(&input))?;
    let definition_count = imported.entries.len();
    // Captured before the reduction below: upstream records rafsi per
    // definition row, so a structured claim can sit on a row that
    // best-definition selection is about to drop, and the fail-closed
    // extracted-rafsi audit must still see it.
    let raw_structured_rafsi = timed_stage("collect raw structured rafsi", || {
        collect_raw_structured_rafsi(&imported)
    });
    // Lensisku's unfiltered export carries every definition of every word,
    // including rows that never got any text and repeat submissions of the
    // same text; jbotci embeds the one definition per word that Lensisku's own
    // ranking would pick among those that actually define something.
    let undefined = timed_stage("discard undefined entries", || {
        imported.retain_defined_entries()
    });
    let duplicates = timed_stage("select best definitions", || {
        imported.retain_best_definition_per_word()
    });
    emit_build_timing(format_args!(
        "kept {} of {definition_count} definition(s), dropping {undefined} undefined \
         and {duplicates} duplicate",
        imported.entries.len()
    ));
    timed_stage("validate dictionary metadata", || {
        validate_dictionary_metadata(&metadata, definition_count, &imported, input.as_bytes())
    })?;
    let extracted_rafsi = timed_stage("load extracted rafsi", || {
        load_extracted_rafsi(&extracted_rafsi_path)
    })?;
    timed_stage("merge extracted rafsi", || {
        merge_extracted_rafsi(&mut imported, &raw_structured_rafsi, &extracted_rafsi)
    })?;
    let leaked_entries = timed_stage("leak entries", || leak_entries(&imported));
    let indexes = timed_stage("build lookup indexes", || {
        build_owned_indexes(leaked_entries)
    });
    let sound_entries = timed_stage("build sound index", || build_sound_index(&imported));
    let word_index = timed_stage("leak word index", || leak_word_index(&indexes.word_index));
    let rafsi_index = timed_stage("leak rafsi index", || {
        leak_rafsi_index(&indexes.rafsi_index)
    });
    let selmaho_index = timed_stage("leak selmaho index", || {
        leak_selmaho_index(&indexes.selmaho_index)
    });
    let pattern_index = timed_stage("leak pattern index", || {
        leak_pattern_index(&indexes.pattern_index)
    });
    let cmavo_sequence_index = leak_cmavo_sequence_index(&indexes.cmavo_sequence_index);
    let sound_index = timed_stage("leak sound index", || leak_sound_index(&sound_entries));
    let generation_dictionary = Dictionary::from_static_slices(
        leaked_entries,
        word_index,
        rafsi_index,
        selmaho_index,
        pattern_index,
        sound_index,
        &[],
        cmavo_sequence_index,
        indexes.max_cmavo_sequence_len,
    );
    let lujvo_entries = timed_stage("build lujvo index", || {
        build_lujvo_index(&generation_dictionary)
    });
    let lujvo_index = timed_stage("leak lujvo index", || leak_lujvo_index(&lujvo_entries));
    let dictionary = Dictionary::from_static_slices(
        leaked_entries,
        word_index,
        rafsi_index,
        selmaho_index,
        pattern_index,
        sound_index,
        lujvo_index,
        cmavo_sequence_index,
        indexes.max_cmavo_sequence_len,
    );
    timed_stage("validate generated dictionary", || dictionary.validate())?;

    let generated = timed_stage("render generated dictionary", || {
        render_dictionary(
            &imported,
            &indexes,
            &sound_entries,
            &lujvo_entries,
            &metadata,
        )
    })?;
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    timed_stage("write generated dictionary", || {
        fs::write(out_dir.join("dictionary_en.rs"), generated)
    })?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn timed_stage<T>(name: &str, stage: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = stage();
    emit_build_timing(format_args!("{name}: {:?}", start.elapsed()));
    result
}

#[requires(true)]
#[ensures(true)]
fn emit_build_timing(args: std::fmt::Arguments<'_>) {
    if env::var_os("JBOTCI_DICTIONARY_BUILD_TIMINGS").is_some() {
        println!("cargo:warning=jbotci-dictionary-data generator {args}");
    }
}

#[requires(true)]
#[ensures(!ret.is_empty() || dictionary.entries.is_empty())]
fn leak_entries(dictionary: &ImportedDictionary) -> &'static [DictionaryEntry<'static>] {
    dictionary
        .entries
        .iter()
        .map(leak_entry)
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(!ret.word.is_empty() || entry.word.is_empty())]
fn leak_entry(entry: &ImportedDictionaryEntry) -> DictionaryEntry<'static> {
    DictionaryEntry {
        word: leak_str(&entry.word),
        word_type: entry.word_type,
        definition: leak_str(&entry.definition),
        definition_id: entry.definition_id,
        notes: leak_str(&entry.notes),
        score: entry.score,
        gloss_keywords: leak_keywords(&entry.gloss_keywords),
        place_keywords: leak_keywords(&entry.place_keywords),
        rafsi: leak_rafsi(&entry.rafsi),
        selmaho: entry
            .selmaho
            .as_deref()
            .map(|value| RawSelmaho(leak_str(value))),
        etymology: entry.etymology.as_deref().map(leak_str),
        jargon: entry.jargon.as_deref().map(leak_str),
        user: leak_user(&entry.user),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty() || keywords.is_empty())]
fn leak_keywords(keywords: &[ImportedKeyword]) -> &'static [Keyword<'static>] {
    keywords
        .iter()
        .map(|keyword| Keyword {
            word: leak_str(&keyword.word),
            meaning: keyword.meaning.as_deref().map(leak_str),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(!ret.is_empty() || rafsi.is_empty())]
fn leak_rafsi(rafsi: &[String]) -> &'static [Rafsi<'static>] {
    rafsi
        .iter()
        .map(|value| Rafsi(leak_str(value)))
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(!ret.username.is_empty() || user.username.is_empty())]
fn leak_user(user: &ImportedDictionaryUser) -> DictionaryUser<'static> {
    DictionaryUser {
        username: leak_str(&user.username),
        realname: user.realname.as_deref().map(leak_str),
    }
}

#[requires(true)]
#[ensures(true)]
fn leak_word_index(index: &[OwnedWordIndexEntry]) -> &'static [WordIndexEntry<'static>] {
    index
        .iter()
        .map(|entry| WordIndexEntry {
            key: leak_str(&entry.key),
            targets: entry.targets.clone().leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_rafsi_index(index: &[OwnedRafsiIndexEntry]) -> &'static [RafsiIndexEntry<'static>] {
    index
        .iter()
        .map(|entry| RafsiIndexEntry {
            key: leak_str(&entry.key),
            targets: entry.targets.clone().leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_selmaho_index(index: &[OwnedSelmahoIndexEntry]) -> &'static [SelmahoIndexEntry<'static>] {
    index
        .iter()
        .map(|entry| SelmahoIndexEntry {
            key: leak_str(&entry.key),
            targets: entry.targets.clone().leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_pattern_index(
    index: &[OwnedPatternIndexEntry],
) -> &'static [DictionaryPatternEntry<'static>] {
    index
        .iter()
        .map(|entry| DictionaryPatternEntry {
            entry_index: entry.entry_index,
            word_key: leak_str(&entry.word_key),
            rafsi_keys: entry
                .rafsi_keys
                .iter()
                .map(|key| leak_str(key))
                .collect::<Vec<_>>()
                .leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn build_sound_index(dictionary: &ImportedDictionary) -> Vec<GeneratedSoundEntry> {
    dictionary
        .entries
        .par_iter()
        .enumerate()
        .map(|(index, entry)| {
            let tokenized = lojban_text_to_tokenized_ipa(&entry.word).ok()?;
            let pronunciation = lojban_text_to_pronunciation_targets(&entry.word).ok()?;
            Some(GeneratedSoundEntry {
                entry_index: EntryIndex(index),
                ipa: tokenized.ipa,
                segments: tokenized.token_sequence.segments().to_vec(),
                self_similarity: tokenized.token_sequence.self_similarity(),
                pronunciation_targets: pronunciation.targets().to_vec(),
                pronunciation_self_similarity: pronunciation.self_similarity(),
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn leak_sound_index(entries: &[GeneratedSoundEntry]) -> &'static [DictionarySoundEntry<'static>] {
    entries
        .iter()
        .map(|entry| DictionarySoundEntry {
            entry_index: entry.entry_index,
            ipa: leak_str(&entry.ipa),
            token_sequence: IpaTokenSequenceView::new(
                entry.segments.clone().leak(),
                entry.self_similarity,
            ),
            pronunciation_targets: PronunciationTargetSequenceView::new(
                entry.pronunciation_targets.clone().leak(),
                entry.pronunciation_self_similarity,
            ),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn build_lujvo_index(dictionary: &Dictionary<'static>) -> Vec<GeneratedLujvoEntry> {
    dictionary
        .entries()
        .par_iter()
        .enumerate()
        .map(|(index, entry)| {
            if !entry.word_type.is_lujvo_like() {
                return None;
            }
            let decomposition = decompose_lujvo_like(dictionary, entry.word)?.into_data();
            Some(GeneratedLujvoEntry {
                entry_index: EntryIndex(index),
                segments: decomposition
                    .segments
                    .into_iter()
                    .map(|segment| generated_lujvo_segment(&segment.segment, segment.source))
                    .collect(),
                source_words: decomposition
                    .source_words
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect()
}

#[requires(true)]
#[ensures(!ret.surface.is_empty())]
fn generated_lujvo_segment(
    segment: &LujvoPart,
    source_word: Option<&str>,
) -> GeneratedLujvoSegment {
    match segment {
        LujvoPart::Rafsi(phonemes) => GeneratedLujvoSegment {
            kind: DictionaryLujvoSegmentKind::Rafsi,
            surface: phonemes.as_str().to_owned(),
            source_word: source_word.map(str::to_owned),
        },
        LujvoPart::Hyphen(phonemes) => GeneratedLujvoSegment {
            kind: DictionaryLujvoSegmentKind::Hyphen,
            surface: phonemes.as_str().to_owned(),
            source_word: None,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn leak_lujvo_index(entries: &[GeneratedLujvoEntry]) -> &'static [DictionaryLujvoEntry<'static>] {
    entries
        .iter()
        .map(|entry| DictionaryLujvoEntry {
            entry_index: entry.entry_index,
            segments: leak_lujvo_segments(&entry.segments),
            source_words: entry
                .source_words
                .iter()
                .map(|source_word| leak_str(source_word))
                .collect::<Vec<_>>()
                .leak(),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_lujvo_segments(
    segments: &[GeneratedLujvoSegment],
) -> &'static [DictionaryLujvoSegment<'static>] {
    segments
        .iter()
        .map(|segment| DictionaryLujvoSegment {
            kind: segment.kind,
            surface: leak_str(&segment.surface),
            source_word: segment.source_word.as_deref().map(leak_str),
        })
        .collect::<Vec<_>>()
        .leak()
}

#[requires(true)]
#[ensures(true)]
fn leak_str(value: &str) -> &'static str {
    Box::leak(value.to_owned().into_boxed_str())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|metadata| !metadata.lensisku_created_at.is_empty()) || ret.is_err())]
fn load_dictionary_metadata(path: &Path) -> Result<DictionaryMetadata, Box<dyn Error>> {
    let input = fs::read_to_string(path)?;
    Ok(toml::from_str(&input)?)
}

#[requires(true)]
#[ensures(true)]
fn validate_dictionary_metadata(
    metadata: &DictionaryMetadata,
    definition_count: usize,
    dictionary: &ImportedDictionary,
    dictionary_bytes: &[u8],
) -> Result<(), Box<dyn Error>> {
    if metadata.definition_count != definition_count {
        return Err(format!(
            "metadata definition_count {} does not match {definition_count} snapshot definitions",
            metadata.definition_count
        )
        .into());
    }

    if metadata.entry_count != dictionary.entries.len() {
        return Err(format!(
            "metadata entry_count {} does not match {} dictionary entries",
            metadata.entry_count,
            dictionary.entries.len()
        )
        .into());
    }

    let sha256 = sha256_hex(dictionary_bytes);
    if metadata.sha256 != sha256 {
        return Err(format!(
            "metadata sha256 {} does not match dictionary sha256 {sha256}",
            metadata.sha256
        )
        .into());
    }

    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|table| !table.rafsi.is_empty()) || ret.is_err())]
fn load_extracted_rafsi(path: &Path) -> Result<ExtractedRafsiTable, Box<dyn Error>> {
    let input = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&input)?)
}

/// Index every word's structured rafsi across *all* of its definition rows.
///
/// Run against the unreduced snapshot: rafsi belong to the word upstream, but
/// the export attaches them per definition row, so a claim can appear only on
/// a row that best-definition selection drops.
#[requires(true)]
#[ensures(ret.values().all(|forms| !forms.is_empty()))]
fn collect_raw_structured_rafsi(dictionary: &ImportedDictionary) -> BTreeMap<String, Vec<String>> {
    let mut raw: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in &dictionary.entries {
        if entry.rafsi.is_empty() {
            continue;
        }
        let forms = raw.entry(entry.word.clone()).or_default();
        for form in &entry.rafsi {
            if !forms.contains(form) {
                forms.push(form.clone());
            }
        }
    }
    raw
}

/// Merge the vendored extracted rafsi into the imported snapshot.
///
/// The merge is deliberately fail-closed: the extracted table was audited
/// against one specific snapshot, so any drift between the two (a word that
/// disappeared, changed word type, or grew structured rafsi of its own, or a
/// form that some other entry has since claimed) must break the build and
/// force a re-audit rather than silently produce a corrupt rafsi index. See
/// `data/README.md` for the refresh protocol.
#[requires(true)]
#[ensures(
    ret.is_err()
        || table.rafsi.iter().all(|(word, forms)| {
            dictionary.entries.iter().any(|entry| entry.word == *word)
                && dictionary
                    .entries
                    .iter()
                    .filter(|entry| entry.word == *word)
                    .all(|entry| forms.iter().all(|form| entry.rafsi.contains(form)))
        }),
    "a successful merge lands every accepted assignment on every entry of its word"
)]
fn merge_extracted_rafsi(
    dictionary: &mut ImportedDictionary,
    raw_structured_rafsi: &BTreeMap<String, Vec<String>>,
    table: &ExtractedRafsiTable,
) -> Result<(), Box<dyn Error>> {
    let mut claims = snapshot_rafsi_claims(dictionary, raw_structured_rafsi);
    for (word, forms) in &table.rafsi {
        // Best-definition selection has already reduced the snapshot to one
        // entry per word, so an extracted word resolves to exactly one target.
        let Some(index) = dictionary
            .entries
            .iter()
            .position(|entry| entry.word == *word)
        else {
            return Err(format!(
                "extracted rafsi word `{word}` is missing from the dictionary snapshot; \
                 re-audit the extraction against the refreshed snapshot"
            )
            .into());
        };
        let entry = &dictionary.entries[index];
        if !entry.word_type.is_gismu_like() {
            return Err(format!(
                "extracted rafsi word `{word}` is a {} in the dictionary snapshot, \
                 but short rafsi belong to gismu",
                entry.word_type.as_str()
            )
            .into());
        }
        // Checked against every raw definition row, not just the selected one:
        // upstream may record its structured claim on a row the selection
        // dropped, and grafting the extracted forms over that would be exactly
        // the corruption this fail-closed audit exists to prevent.
        if let Some(structured) = raw_structured_rafsi.get(word) {
            return Err(format!(
                "extracted rafsi word `{word}` now carries structured rafsi {structured:?} in the \
                 dictionary snapshot; re-audit the extracted entry against them and drop it \
                 from {VENDORED_EXTRACTED_RAFSI}",
            )
            .into());
        }

        let derivable = possible_short_rafsi_forms(word);
        for form in forms {
            if !derivable.iter().any(|candidate| candidate.form == *form) {
                return Err(format!(
                    "extracted rafsi `{form}` is not a CLL-derivable short rafsi of `{word}`"
                )
                .into());
            }
            let key = normalize_lookup_query(form);
            if let Some(claim) = claims.get(&key) {
                return Err(format!(
                    "extracted rafsi `{form}` for `{word}` is already {} `{}`",
                    claim.origin.describe(),
                    claim.word
                )
                .into());
            }
            claims.insert(
                key,
                new!(RafsiClaim {
                    word: word.clone(),
                    origin: RafsiClaimOrigin::Extracted,
                }),
            );
        }

        dictionary.entries[index].rafsi = forms.clone();
    }
    Ok(())
}

/// Index every rafsi form the snapshot already claims, listed or universal.
///
/// Listed claims come from `raw_structured_rafsi`, i.e. from *every*
/// definition row of the unreduced export: upstream attaches rafsi per row, so
/// a claim can sit on a row best-definition selection dropped, and the
/// fail-closed audit must refuse an extracted form against it rather than
/// graft over it. Universal gismu forms come from the embedded entries — a
/// word that selection dropped entirely is not in the dictionary, so its
/// universal forms claim nothing in the rafsi index the merge protects.
///
/// The listed half is therefore a superset of the keys
/// [`build_owned_indexes`] would produce; every extra key is a deliberate
/// re-audit trigger, not an index clash.
#[requires(true)]
#[ensures(true)]
fn snapshot_rafsi_claims(
    dictionary: &ImportedDictionary,
    raw_structured_rafsi: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, RafsiClaim> {
    let mut claims = BTreeMap::new();
    for (word, forms) in raw_structured_rafsi {
        for rafsi in forms {
            claims
                .entry(normalize_lookup_query(rafsi))
                .or_insert_with(|| {
                    new!(RafsiClaim {
                        word: word.clone(),
                        origin: RafsiClaimOrigin::SnapshotListed,
                    })
                });
        }
    }
    for entry in &dictionary.entries {
        if entry.word_type.is_gismu_like() {
            for (form, _) in universal_gismu_rafsi_forms(&entry.word) {
                claims.entry(form).or_insert_with(|| {
                    new!(RafsiClaim {
                        word: entry.word.clone(),
                        origin: RafsiClaimOrigin::SnapshotUniversal,
                    })
                });
            }
        }
    }
    claims
}

#[requires(true)]
#[ensures(true)]
fn render_dictionary(
    dictionary: &ImportedDictionary,
    indexes: &OwnedDictionaryIndexes,
    sound_index: &[GeneratedSoundEntry],
    lujvo_index: &[GeneratedLujvoEntry],
    metadata: &DictionaryMetadata,
) -> Result<String, Box<dyn Error>> {
    let entries = dictionary.entries.iter().map(render_entry);
    let word_index = indexes.word_index.iter().map(render_word_index_entry);
    let rafsi_index = indexes.rafsi_index.iter().map(render_rafsi_index_entry);
    let selmaho_index = indexes.selmaho_index.iter().map(render_selmaho_index_entry);
    let pattern_index = indexes.pattern_index.iter().map(render_pattern_index_entry);
    let sound_index = sound_index.iter().map(render_sound_index_entry);
    let lujvo_index = lujvo_index.iter().map(render_lujvo_index_entry);
    let cmavo_sequence_index = indexes
        .cmavo_sequence_index
        .iter()
        .map(render_cmavo_sequence_index_entry);
    let max_cmavo_sequence_len = indexes.max_cmavo_sequence_len;
    let rendered_metadata = render_metadata(metadata);

    let tokens = quote! {
        pub static ENTRIES: &[jbotci_dictionary::DictionaryEntry<'static>] = &[
            #(#entries,)*
        ];

        static WORD_INDEX: &[jbotci_dictionary::WordIndexEntry<'static>] = &[
            #(#word_index,)*
        ];

        static RAFSI_INDEX: &[jbotci_dictionary::RafsiIndexEntry<'static>] = &[
            #(#rafsi_index,)*
        ];

        static SELMAHO_INDEX: &[jbotci_dictionary::SelmahoIndexEntry<'static>] = &[
            #(#selmaho_index,)*
        ];

        static PATTERN_INDEX: &[jbotci_dictionary::DictionaryPatternEntry<'static>] = &[
            #(#pattern_index,)*
        ];

        static SOUND_INDEX: &[jbotci_dictionary::DictionarySoundEntry<'static>] = &[
            #(#sound_index,)*
        ];

        static LUJVO_INDEX: &[jbotci_dictionary::DictionaryLujvoEntry<'static>] = &[
            #(#lujvo_index,)*
        ];

        static CMAVO_SEQUENCE_INDEX: &[jbotci_dictionary::CmavoSequenceIndexEntry<'static>] = &[#(#cmavo_sequence_index,)*];

        pub static ENGLISH: jbotci_dictionary::Dictionary<'static> =
            jbotci_dictionary::Dictionary::from_static_slices(
                ENTRIES,
                WORD_INDEX,
                RAFSI_INDEX,
                SELMAHO_INDEX,
                PATTERN_INDEX,
                SOUND_INDEX,
                LUJVO_INDEX,
                CMAVO_SEQUENCE_INDEX,
                #max_cmavo_sequence_len,
            );

        pub static ENGLISH_METADATA: crate::DictionarySnapshotMetadata = #rendered_metadata;
    };

    let syntax = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&syntax))
}

#[requires(true)]
#[ensures(true)]
fn render_metadata(metadata: &DictionaryMetadata) -> TokenStream {
    let language_tag = string_literal(&metadata.language_tag);
    let language_realname = string_literal(&metadata.language_realname);
    let source_language_tag = string_literal(&metadata.source_language_tag);
    let format = string_literal(&metadata.format);
    let positive_scores_only = metadata.positive_scores_only;
    let filename = string_literal(&metadata.filename);
    let metadata_url = string_literal(&metadata.metadata_url);
    let download_url = string_literal(&metadata.download_url);
    let lensisku_created_at = string_literal(&metadata.lensisku_created_at);
    let sha256 = string_literal(&metadata.sha256);
    let definition_count = usize_literal(metadata.definition_count);
    let entry_count = usize_literal(metadata.entry_count);

    // Arguments in declaration order; `new` is `const`, so the invariant is
    // checked while the generated `static` compiles.
    quote! {
        crate::DictionarySnapshotMetadata::new(
            #language_tag,
            #language_realname,
            #source_language_tag,
            #format,
            #positive_scores_only,
            #filename,
            #metadata_url,
            #download_url,
            #lensisku_created_at,
            #sha256,
            #definition_count,
            #entry_count,
        )
    }
}

#[requires(true)]
#[ensures(true)]
fn render_entry(entry: &ImportedDictionaryEntry) -> TokenStream {
    let word = string_literal(&entry.word);
    let word_type = render_word_type(entry.word_type);
    let definition = string_literal(&entry.definition);
    let definition_id = u64_literal(entry.definition_id.0);
    let notes = string_literal(&entry.notes);
    let score = f64_literal(entry.score.0);
    let gloss_keywords = entry.gloss_keywords.iter().map(render_keyword);
    let place_keywords = entry.place_keywords.iter().map(render_keyword);
    let rafsi = entry.rafsi.iter().map(|value| {
        let value = string_literal(value);
        quote! { jbotci_dictionary::Rafsi(#value) }
    });
    let selmaho = render_optional_string_newtype(entry.selmaho.as_deref(), "RawSelmaho");
    let etymology = render_optional_str(entry.etymology.as_deref());
    let jargon = render_optional_str(entry.jargon.as_deref());
    let user = render_user(&entry.user);

    quote! {
        jbotci_dictionary::DictionaryEntry {
            word: #word,
            word_type: #word_type,
            definition: #definition,
            definition_id: jbotci_dictionary::DefinitionId(#definition_id),
            notes: #notes,
            score: jbotci_dictionary::Score(#score),
            gloss_keywords: &[#(#gloss_keywords,)*],
            place_keywords: &[#(#place_keywords,)*],
            rafsi: &[#(#rafsi,)*],
            selmaho: #selmaho,
            etymology: #etymology,
            jargon: #jargon,
            user: #user,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_keyword(keyword: &ImportedKeyword) -> TokenStream {
    let word = string_literal(&keyword.word);
    let meaning = render_optional_str(keyword.meaning.as_deref());
    quote! {
        jbotci_dictionary::Keyword {
            word: #word,
            meaning: #meaning,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_user(user: &ImportedDictionaryUser) -> TokenStream {
    let username = string_literal(&user.username);
    let realname = render_optional_str(user.realname.as_deref());
    quote! {
        jbotci_dictionary::DictionaryUser {
            username: #username,
            realname: #realname,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_index_entry(entry: &OwnedWordIndexEntry) -> TokenStream {
    let key = string_literal(&entry.key);
    let targets = entry.targets.iter().map(render_entry_index);
    quote! {
        jbotci_dictionary::WordIndexEntry {
            key: #key,
            targets: &[#(#targets,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_index_entry(entry: &OwnedRafsiIndexEntry) -> TokenStream {
    let key = string_literal(&entry.key);
    let targets = entry.targets.iter().map(render_rafsi_index_target);
    quote! {
        jbotci_dictionary::RafsiIndexEntry {
            key: #key,
            targets: &[#(#targets,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_selmaho_index_entry(entry: &OwnedSelmahoIndexEntry) -> TokenStream {
    let key = string_literal(&entry.key);
    let targets = entry.targets.iter().map(render_entry_index);
    quote! {
        jbotci_dictionary::SelmahoIndexEntry {
            key: #key,
            targets: &[#(#targets,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_pattern_index_entry(entry: &OwnedPatternIndexEntry) -> TokenStream {
    let entry_index = render_entry_index(&entry.entry_index);
    let word_key = string_literal(&entry.word_key);
    let rafsi_keys = entry.rafsi_keys.iter().map(|key| string_literal(key));
    quote! {
        jbotci_dictionary::DictionaryPatternEntry {
            entry_index: #entry_index,
            word_key: #word_key,
            rafsi_keys: &[#(#rafsi_keys,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_sound_index_entry(entry: &GeneratedSoundEntry) -> TokenStream {
    let entry_index = render_entry_index(&entry.entry_index);
    let ipa = string_literal(&entry.ipa);
    let segments = entry.segments.iter().map(render_ipa_segment_id);
    let self_similarity = f64_literal(entry.self_similarity);
    let pronunciation_targets = entry
        .pronunciation_targets
        .iter()
        .map(render_pronunciation_target_id);
    let pronunciation_self_similarity = f64_literal(entry.pronunciation_self_similarity);
    quote! {
        jbotci_dictionary::DictionarySoundEntry {
            entry_index: #entry_index,
            ipa: #ipa,
            token_sequence: jbotci_phonetic::IpaTokenSequenceView::from_static_parts(
                &[#(#segments,)*],
                #self_similarity,
            ),
            pronunciation_targets:
                jbotci_phonetic::PronunciationTargetSequenceView::from_static_parts(
                    &[#(#pronunciation_targets,)*],
                    #pronunciation_self_similarity,
                ),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_lujvo_index_entry(entry: &GeneratedLujvoEntry) -> TokenStream {
    let entry_index = render_entry_index(&entry.entry_index);
    let segments = entry.segments.iter().map(render_lujvo_segment);
    let source_words = entry
        .source_words
        .iter()
        .map(|source_word| string_literal(source_word));
    quote! {
        jbotci_dictionary::DictionaryLujvoEntry {
            entry_index: #entry_index,
            segments: &[#(#segments,)*],
            source_words: &[#(#source_words,)*],
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_lujvo_segment(segment: &GeneratedLujvoSegment) -> TokenStream {
    let kind = render_lujvo_segment_kind(segment.kind);
    let surface = string_literal(&segment.surface);
    let source_word = render_optional_str(segment.source_word.as_deref());
    quote! {
        jbotci_dictionary::DictionaryLujvoSegment {
            kind: #kind,
            surface: #surface,
            source_word: #source_word,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_lujvo_segment_kind(kind: DictionaryLujvoSegmentKind) -> TokenStream {
    match kind {
        DictionaryLujvoSegmentKind::Rafsi => {
            quote! { jbotci_dictionary::DictionaryLujvoSegmentKind::Rafsi }
        }
        DictionaryLujvoSegmentKind::Hyphen => {
            quote! { jbotci_dictionary::DictionaryLujvoSegmentKind::Hyphen }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ipa_segment_id(segment: &IpaSegmentId) -> TokenStream {
    let value = u16_literal(segment.get());
    quote! { jbotci_phonetic::IpaSegmentId::from_static_index(#value) }
}

#[requires(true)]
#[ensures(true)]
fn render_pronunciation_target_id(target: &PronunciationTargetId) -> TokenStream {
    let value = u16_literal(target.get());
    quote! { jbotci_phonetic::PronunciationTargetId::from_static_index(#value) }
}

#[requires(true)]
#[ensures(true)]
fn render_entry_index(index: &EntryIndex) -> TokenStream {
    let value = usize_literal(index.0);
    quote! { jbotci_dictionary::EntryIndex(#value) }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_index_target(target: &RafsiIndexTarget) -> TokenStream {
    let entry_index = render_entry_index(&target.entry_index);
    let source = render_rafsi_source(target.source);
    quote! {
        jbotci_dictionary::RafsiIndexTarget {
            entry_index: #entry_index,
            source: #source,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_type(word_type: WordType) -> TokenStream {
    match word_type {
        WordType::Gismu => quote! { jbotci_dictionary::WordType::Gismu },
        WordType::ExperimentalGismu => quote! { jbotci_dictionary::WordType::ExperimentalGismu },
        WordType::Lujvo => quote! { jbotci_dictionary::WordType::Lujvo },
        WordType::ZeiLujvo => quote! { jbotci_dictionary::WordType::ZeiLujvo },
        WordType::ObsoleteZeiLujvo => quote! { jbotci_dictionary::WordType::ObsoleteZeiLujvo },
        WordType::Cmavo => quote! { jbotci_dictionary::WordType::Cmavo },
        WordType::ExperimentalCmavo => quote! { jbotci_dictionary::WordType::ExperimentalCmavo },
        WordType::ObsoleteCmavo => quote! { jbotci_dictionary::WordType::ObsoleteCmavo },
        WordType::CmavoCompound => quote! { jbotci_dictionary::WordType::CmavoCompound },
        WordType::Fuivla => quote! { jbotci_dictionary::WordType::Fuivla },
        WordType::ObsoleteFuivla => quote! { jbotci_dictionary::WordType::ObsoleteFuivla },
        WordType::Cmevla => quote! { jbotci_dictionary::WordType::Cmevla },
        WordType::ObsoleteCmevla => quote! { jbotci_dictionary::WordType::ObsoleteCmevla },
        WordType::BuLetteral => quote! { jbotci_dictionary::WordType::BuLetteral },
        WordType::Phrase => quote! { jbotci_dictionary::WordType::Phrase },
        WordType::Nalvla => quote! { jbotci_dictionary::WordType::Nalvla },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_source(source: RafsiSource) -> TokenStream {
    match source {
        RafsiSource::Listed => quote! { jbotci_dictionary::RafsiSource::Listed },
        RafsiSource::UniversalShort => quote! { jbotci_dictionary::RafsiSource::UniversalShort },
        RafsiSource::UniversalLong => quote! { jbotci_dictionary::RafsiSource::UniversalLong },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_optional_string_newtype(value: Option<&str>, type_name: &str) -> TokenStream {
    match value {
        Some(value) => {
            let type_name = syn::Ident::new(type_name, proc_macro2::Span::call_site());
            let value = string_literal(value);
            quote! { Some(jbotci_dictionary::#type_name(#value)) }
        }
        None => quote! { None },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_optional_str(value: Option<&str>) -> TokenStream {
    match value {
        Some(value) => {
            let value = string_literal(value);
            quote! { Some(#value) }
        }
        None => quote! { None },
    }
}

#[requires(true)]
#[ensures(true)]
fn string_literal(value: &str) -> Literal {
    Literal::string(value)
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[requires(true)]
#[ensures(true)]
fn usize_literal(value: usize) -> Literal {
    Literal::usize_unsuffixed(value)
}

#[requires(true)]
#[ensures(true)]
fn u16_literal(value: u16) -> Literal {
    Literal::u16_unsuffixed(value)
}

#[requires(true)]
#[ensures(true)]
fn u64_literal(value: u64) -> Literal {
    Literal::u64_unsuffixed(value)
}

#[requires(true)]
#[ensures(true)]
fn f64_literal(value: f64) -> Literal {
    Literal::f64_unsuffixed(value)
}

#[requires(true)]
#[ensures(ret.len() == index.len())]
fn leak_cmavo_sequence_index(
    index: &[OwnedCmavoSequenceIndexEntry],
) -> &'static [CmavoSequenceIndexEntry<'static>] {
    Box::leak(
        index
            .iter()
            .map(|entry| {
                let components = entry
                    .components
                    .iter()
                    .map(|component| &*Box::leak(component.clone().into_boxed_str()))
                    .collect::<Vec<_>>();
                CmavoSequenceIndexEntry::from_static_parts(
                    Box::leak(components.into_boxed_slice()),
                    Box::leak(entry.targets.clone().into_boxed_slice()),
                )
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

#[requires(true)]
#[ensures(true)]
fn render_cmavo_sequence_index_entry(entry: &OwnedCmavoSequenceIndexEntry) -> TokenStream {
    let components = entry.components.iter().map(String::as_str);
    let targets = entry.targets.iter().map(render_entry_index);
    quote! { jbotci_dictionary::CmavoSequenceIndexEntry::from_static_parts(&[#(#components,)*], &[#(#targets,)*]) }
}
