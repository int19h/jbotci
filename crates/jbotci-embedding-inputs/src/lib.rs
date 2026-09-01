//! Canonical text inputs for embedding-based jbotci search.

use std::fmt;
use std::fmt::Write as _;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_cll::{CllSearchChunk, CllSearchChunkKind, cll_search_all_chunks};
use jbotci_dictionary::{Dictionary, DictionaryEntry};
use jbotci_search::vlacku::{grouped_word_type_filter_key, normalize_word_type_filter};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const DEFAULT_MODEL_KEY: &str = "f2llm-v2-330m-q4-k-m-896";
pub const DEFAULT_MODEL_REVISION: &str = "e76f54804b54782f5bed93c09f63201e38a1a99b";
pub const DEFAULT_MODEL_DIMENSIONS: usize = 896;
pub const DEFAULT_INPUT_FORMAT_VERSION: &str = "f2llm-v2-330m-q4-k-m-v0";
pub const VLACKU_CORPUS_ID: &str = "vlacku-en";
pub const CUKTA_CORPUS_ID: &str = "cukta-cll";
pub const RETRIEVAL_QUERY_PREFIX: &str =
    "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: ";
pub const RETRIEVAL_DOCUMENT_PREFIX: &str = "title: {title} | text: {text}";

const PLACE_PLACEHOLDER: char = '\u{2423}';

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[invariant(true)]
pub struct EmbeddingInputCorpus {
    pub model_key: String,
    pub model_revision: String,
    pub input_format_version: String,
    pub input_hash: String,
    pub dictionary_hash: String,
    pub cll_hash: String,
    pub dictionary: Vec<EmbeddingInputDocument>,
    pub cll: Vec<EmbeddingInputDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[invariant(true)]
pub struct EmbeddingInputDocument {
    pub id: usize,
    pub input: String,
    pub input_hash: String,
    pub kind: Option<String>,
}

/// Raw JSON transport shape. Fingerprints in this DTO are untrusted claims until conversion.
#[invariant(true)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EmbeddingInputCorpusDto {
    model_key: String,
    model_revision: String,
    input_format_version: String,
    input_hash: String,
    dictionary_hash: String,
    cll_hash: String,
    dictionary: Vec<EmbeddingInputDocumentDto>,
    cll: Vec<EmbeddingInputDocumentDto>,
}

/// Raw document transport shape. All field combinations can occur at the deserialization boundary.
#[invariant(true)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EmbeddingInputDocumentDto {
    id: usize,
    input: String,
    input_hash: String,
    kind: Option<String>,
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingInputError {
    message: String,
}

impl From<jbotci_cll::CllError> for EmbeddingInputError {
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    fn from(error: jbotci_cll::CllError) -> Self {
        new!(EmbeddingInputError {
            message: format!("failed to load embedded CLL for embedding inputs: {error}"),
        })
    }
}

impl From<serde_json::Error> for EmbeddingInputError {
    #[requires(true)]
    #[ensures(!ret.message.is_empty())]
    fn from(error: serde_json::Error) -> Self {
        new!(EmbeddingInputError {
            message: format!("failed to serialize embedding input corpus JSON: {error}"),
        })
    }
}

impl fmt::Display for EmbeddingInputError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for EmbeddingInputError {}

/// Validation failures for the corpus JSON consumed by the future native pack builder.
#[invariant(::Json { message } => !message.is_empty())]
#[invariant(::Metadata { field } => !field.is_empty())]
#[invariant(::UnsupportedInputFormat { actual } => actual != DEFAULT_INPUT_FORMAT_VERSION)]
#[invariant(::EmptyCorpus { corpus } => !corpus.is_empty())]
#[invariant(::DocumentId { corpus, expected, actual } => !corpus.is_empty() && expected != actual)]
#[invariant(
    ::Fingerprint {
        field,
        expected,
        actual,
    } => !field.is_empty() && expected != actual
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingInputCorpusError {
    Json {
        message: String,
    },
    Metadata {
        field: String,
    },
    UnsupportedInputFormat {
        actual: String,
    },
    EmptyCorpus {
        corpus: String,
    },
    DocumentId {
        corpus: String,
        expected: usize,
        actual: usize,
    },
    Fingerprint {
        field: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for EmbeddingInputCorpusError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(EmbeddingInputCorpusError::Json { message }) => {
                write!(formatter, "invalid embedding corpus JSON: {message}")
            }
            data!(EmbeddingInputCorpusError::Metadata { field }) => {
                write!(
                    formatter,
                    "embedding corpus field `{field}` must not be empty"
                )
            }
            data!(EmbeddingInputCorpusError::UnsupportedInputFormat { actual }) => write!(
                formatter,
                "unsupported embedding corpus input format `{actual}`; expected `{DEFAULT_INPUT_FORMAT_VERSION}`"
            ),
            data!(EmbeddingInputCorpusError::EmptyCorpus { corpus }) => {
                write!(formatter, "embedding corpus `{corpus}` must not be empty")
            }
            data!(EmbeddingInputCorpusError::DocumentId {
                corpus,
                expected,
                actual,
            }) => write!(
                formatter,
                "embedding corpus `{corpus}` document id mismatch: expected {expected}, got {actual}"
            ),
            data!(EmbeddingInputCorpusError::Fingerprint {
                field,
                expected,
                actual,
            }) => write!(
                formatter,
                "embedding corpus fingerprint `{field}` mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for EmbeddingInputCorpusError {}

impl EmbeddingInputCorpus {
    /// Deserializes the canonical corpus transport and verifies every serialized fingerprint.
    ///
    /// `model_key` remains corpus metadata rather than a target-model constraint: all four F2LLM
    /// WebGPU artifacts consume the same canonical corpus even though the exporter currently
    /// records the native 330M model key.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|corpus| {
        corpus.input_format_version == DEFAULT_INPUT_FORMAT_VERSION
            && !corpus.dictionary.is_empty()
            && !corpus.cll.is_empty()
    }) || ret.is_err())]
    pub fn from_json(json: &str) -> Result<Self, EmbeddingInputCorpusError> {
        let dto: EmbeddingInputCorpusDto = serde_json::from_str(json).map_err(|error| {
            new!(EmbeddingInputCorpusError::Json {
                message: error.to_string(),
            })
        })?;
        if dto.model_key.is_empty() {
            return Err(new!(EmbeddingInputCorpusError::Metadata {
                field: "modelKey".to_owned(),
            }));
        }
        if dto.model_revision.is_empty() {
            return Err(new!(EmbeddingInputCorpusError::Metadata {
                field: "modelRevision".to_owned(),
            }));
        }
        if dto.input_format_version != DEFAULT_INPUT_FORMAT_VERSION {
            return Err(new!(EmbeddingInputCorpusError::UnsupportedInputFormat {
                actual: dto.input_format_version,
            }));
        }
        let dictionary = validate_corpus_documents("dictionary", dto.dictionary, VLACKU_CORPUS_ID)?;
        let cll = validate_corpus_documents("cll", dto.cll, CUKTA_CORPUS_ID)?;
        let dictionary_hash = dictionary_documents_hash(&dictionary);
        require_fingerprint("dictionaryHash", &dto.dictionary_hash, &dictionary_hash)?;
        let cll_hash = input_documents_hash(CUKTA_CORPUS_ID, &cll);
        require_fingerprint("cllHash", &dto.cll_hash, &cll_hash)?;
        let input_hash = combined_input_hash(&dictionary_hash, &cll_hash);
        require_fingerprint("inputHash", &dto.input_hash, &input_hash)?;
        Ok(Self {
            model_key: dto.model_key,
            model_revision: dto.model_revision,
            input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
            input_hash,
            dictionary_hash,
            cll_hash,
            dictionary,
            cll,
        })
    }
}

#[requires(!corpus_name.is_empty())]
#[requires(!corpus_id.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|documents| {
    !documents.is_empty()
        && documents
            .iter()
            .enumerate()
            .all(|(index, document)| document.id == index)
}) || ret.is_err())]
fn validate_corpus_documents(
    corpus_name: &str,
    documents: Vec<EmbeddingInputDocumentDto>,
    corpus_id: &str,
) -> Result<Vec<EmbeddingInputDocument>, EmbeddingInputCorpusError> {
    if documents.is_empty() {
        return Err(new!(EmbeddingInputCorpusError::EmptyCorpus {
            corpus: corpus_name.to_owned(),
        }));
    }
    let mut validated = Vec::with_capacity(documents.len());
    for (expected_id, document) in documents.into_iter().enumerate() {
        if document.id != expected_id {
            return Err(new!(EmbeddingInputCorpusError::DocumentId {
                corpus: corpus_name.to_owned(),
                expected: expected_id,
                actual: document.id,
            }));
        }
        let input_hash = sha256_hex_bytes(document.input.as_bytes());
        require_fingerprint(
            &format!("{corpus_id}[{expected_id}].inputHash"),
            &document.input_hash,
            &input_hash,
        )?;
        validated.push(EmbeddingInputDocument {
            id: expected_id,
            input: document.input,
            input_hash,
            kind: document.kind,
        });
    }
    Ok(validated)
}

#[requires(!field.is_empty())]
#[requires(expected.len() == 64)]
#[ensures(ret.is_ok() == (claimed == expected))]
fn require_fingerprint(
    field: &str,
    claimed: &str,
    expected: &str,
) -> Result<(), EmbeddingInputCorpusError> {
    if claimed == expected {
        Ok(())
    } else {
        Err(new!(EmbeddingInputCorpusError::Fingerprint {
            field: field.to_owned(),
            expected: expected.to_owned(),
            actual: claimed.to_owned(),
        }))
    }
}

#[requires(true)]
#[ensures(ret.starts_with(RETRIEVAL_QUERY_PREFIX))]
pub fn build_retrieval_query_input(content: &str) -> String {
    format!("{RETRIEVAL_QUERY_PREFIX}{content}")
}

#[requires(true)]
#[ensures(ret.contains(" | text: "))]
pub fn build_retrieval_document_input(content: &str, title: &str) -> String {
    let safe_title = if title.trim().is_empty() {
        "none"
    } else {
        title
    };
    format!("title: {safe_title} | text: {content}")
}

#[requires(true)]
#[ensures(ret.contains(&entry.word))]
pub fn dictionary_embedding_input(entry: &DictionaryEntry<'_>) -> String {
    let mut body_parts = Vec::new();
    let definition = replace_dollar_markup_with_placeholder(entry.definition);
    if !definition.trim().is_empty() {
        body_parts.push(definition);
    }
    let glosses = entry
        .gloss_keywords
        .iter()
        .map(|keyword| match keyword.meaning {
            Some(meaning) => format!("{} ({meaning})", keyword.word),
            None => keyword.word.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("; ");
    if !glosses.trim().is_empty() {
        body_parts.push(glosses);
    }
    build_retrieval_document_input(&body_parts.join("\n"), entry.word)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn dictionary_embedding_kind(entry: &DictionaryEntry<'_>) -> String {
    grouped_word_type_filter_key(&normalize_word_type_filter(entry.word_type.as_str()))
}

#[requires(true)]
#[ensures(true)]
pub fn replace_dollar_markup_with_placeholder(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    loop {
        let Some(start) = rest.find('$') else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('$') else {
            output.push('$');
            output.push_str(after_start);
            break;
        };
        output.push(PLACE_PLACEHOLDER);
        rest = &after_start[end + 1..];
    }
    output
}

#[requires(true)]
#[ensures(ret.contains(" | text: "))]
pub fn cll_embedding_input(chunk: &CllSearchChunk) -> String {
    build_retrieval_document_input(&chunk.text, &cll_embedding_title(chunk))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn cll_embedding_title(chunk: &CllSearchChunk) -> String {
    match chunk.kind {
        CllSearchChunkKind::Section => {
            filter_not_blank([chunk.label.as_str(), chunk.section_title.as_str()]).join(" — ")
        }
        CllSearchChunkKind::Paragraph => cll_section_number_fallback(chunk),
        CllSearchChunkKind::Example => {
            let section_number = cll_section_number_fallback(chunk);
            if let Some(example_number) = extract_example_number(&chunk.label) {
                filter_not_blank([section_number.as_str(), example_number.as_str()]).join(" ")
            } else {
                section_number
            }
        }
    }
}

/// The short designation an embedded document carries for its section. Sections
/// of numbered chapters use the book's section number; appendix sections have
/// none, so they fall back to their stable section id.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn cll_section_number_fallback(chunk: &CllSearchChunk) -> String {
    match chunk
        .section_number
        .as_deref()
        .map(str::trim)
        .filter(|number| !number.is_empty())
    {
        Some(number) => number.to_owned(),
        None => chunk.section_id.clone(),
    }
}

#[requires(true)]
#[ensures(true)]
fn extract_example_number(label: &str) -> Option<String> {
    let stripped = label
        .strip_prefix("Example ")
        .map(str::trim)
        .unwrap_or(label.trim());
    parse_numeric_token(stripped).or_else(|| label.split_whitespace().find_map(parse_numeric_token))
}

#[requires(true)]
#[ensures(true)]
fn parse_numeric_token(token: &str) -> Option<String> {
    (!token.is_empty()
        && token.chars().any(|ch| ch.is_ascii_digit())
        && token.chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
    .then(|| token.to_owned())
}

#[requires(true)]
#[ensures(true)]
fn filter_not_blank<const N: usize>(values: [&str; N]) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == 64)]
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    lowercase_hex(Sha256::digest(bytes))
}

#[requires(true)]
#[ensures(ret.len() == bytes.as_ref().len() * 2)]
pub fn lowercase_hex(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing hex to a String cannot fail");
    }
    output
}

#[requires(true)]
#[ensures(ret.len() == 64)]
pub fn dictionary_fingerprint(dictionary: &Dictionary<'_>) -> String {
    let mut hasher = Sha256::new();
    for entry in dictionary.entries() {
        let input = dictionary_embedding_input(entry);
        hasher.update(entry.word.as_bytes());
        hasher.update([0]);
        hasher.update(entry.definition_id.0.to_le_bytes());
        hasher.update([0]);
        hasher.update(input.as_bytes());
        hasher.update([0]);
    }
    lowercase_hex(hasher.finalize())
}

#[requires(true)]
#[ensures(ret.len() == 64)]
pub fn cll_fingerprint(chunks: &[CllSearchChunk]) -> String {
    let mut hasher = Sha256::new();
    for chunk in chunks {
        let input = cll_embedding_input(chunk);
        hasher.update(chunk.label.as_bytes());
        hasher.update([0]);
        hasher.update(input.as_bytes());
        hasher.update([0]);
    }
    lowercase_hex(hasher.finalize())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| !corpus.dictionary.is_empty()) || ret.is_err())]
pub fn embedding_input_corpus() -> Result<EmbeddingInputCorpus, EmbeddingInputError> {
    let dictionary = jbotci_dictionary_data::english();
    let cll = cll_search_all_chunks(jbotci_cll::embedded_cll_site()?).to_vec();
    Ok(embedding_input_corpus_from_parts(dictionary, &cll))
}

#[requires(true)]
#[ensures(!ret.input_hash.is_empty())]
pub fn embedding_input_corpus_from_parts(
    dictionary: &Dictionary<'_>,
    cll_chunks: &[CllSearchChunk],
) -> EmbeddingInputCorpus {
    let dictionary_docs = dictionary
        .entries()
        .iter()
        .enumerate()
        .map(|(id, entry)| {
            let input = dictionary_embedding_input(entry);
            EmbeddingInputDocument {
                id,
                input_hash: sha256_hex_bytes(input.as_bytes()),
                input,
                kind: Some(dictionary_embedding_kind(entry)),
            }
        })
        .collect::<Vec<_>>();
    let cll_docs = cll_chunks
        .iter()
        .enumerate()
        .map(|(id, chunk)| {
            let input = cll_embedding_input(chunk);
            EmbeddingInputDocument {
                id,
                input_hash: sha256_hex_bytes(input.as_bytes()),
                input,
                kind: Some(cll_embedding_kind(chunk).to_owned()),
            }
        })
        .collect::<Vec<_>>();
    let dictionary_hash = dictionary_documents_hash(&dictionary_docs);
    let cll_hash = input_documents_hash(CUKTA_CORPUS_ID, &cll_docs);
    let input_hash = combined_input_hash(&dictionary_hash, &cll_hash);
    EmbeddingInputCorpus {
        model_key: DEFAULT_MODEL_KEY.to_owned(),
        model_revision: DEFAULT_MODEL_REVISION.to_owned(),
        input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
        input_hash,
        dictionary_hash,
        cll_hash,
        dictionary: dictionary_docs,
        cll: cll_docs,
    }
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn dictionary_documents_hash(documents: &[EmbeddingInputDocument]) -> String {
    input_documents_hash(VLACKU_CORPUS_ID, documents)
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn input_documents_hash(corpus_id: &str, documents: &[EmbeddingInputDocument]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(corpus_id.as_bytes());
    hasher.update([0]);
    for document in documents {
        hasher.update(document_id_hash_bytes(document.id));
        hasher.update([0]);
        hasher.update(document.input_hash.as_bytes());
        hasher.update([0]);
        if let Some(kind) = &document.kind {
            hasher.update(kind.as_bytes());
        }
        hasher.update([0]);
    }
    lowercase_hex(hasher.finalize())
}

#[requires(true)]
#[ensures(ret.len() == 8)]
fn document_id_hash_bytes(id: usize) -> [u8; 8] {
    u64::try_from(id)
        .expect("embedding document ids must fit in u64")
        .to_le_bytes()
}

#[requires(dictionary_hash.len() == 64)]
#[requires(cll_hash.len() == 64)]
#[ensures(ret.len() == 64)]
fn combined_input_hash(dictionary_hash: &str, cll_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(DEFAULT_INPUT_FORMAT_VERSION.as_bytes());
    hasher.update([0]);
    hasher.update(dictionary_hash.as_bytes());
    hasher.update([0]);
    hasher.update(cll_hash.as_bytes());
    lowercase_hex(hasher.finalize())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|json| !json.is_empty()) || ret.is_err())]
pub fn embedding_input_corpus_json() -> Result<String, EmbeddingInputError> {
    serde_json::to_string(&embedding_input_corpus()?).map_err(EmbeddingInputError::from)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn cll_embedding_kind(chunk: &CllSearchChunk) -> &'static str {
    match chunk.kind {
        CllSearchChunkKind::Section => "section",
        CllSearchChunkKind::Paragraph => "paragraph",
        CllSearchChunkKind::Example => "example",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{data, ensures, new, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn retrieval_prefixes_match_f2llm() {
        assert_eq!(
            build_retrieval_query_input("klama"),
            "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: klama"
        );
        assert_eq!(
            build_retrieval_document_input("goer", "klama"),
            "title: klama | text: goer"
        );
        assert_eq!(
            build_retrieval_document_input("goer", " "),
            "title: none | text: goer"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_embedding_input_matches_v0_place_placeholder() {
        assert_eq!(
            replace_dollar_markup_with_placeholder("$x_1$ goes to $x_2$"),
            "\u{2423} goes to \u{2423}"
        );
        assert_eq!(
            replace_dollar_markup_with_placeholder("broken $x_1"),
            "broken $x_1"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_embedding_input_uses_title_definition_and_glosses() {
        let dictionary = jbotci_dictionary_data::english();
        let entry = dictionary
            .entries()
            .iter()
            .find(|entry| entry.word == "klama")
            .expect("klama entry");
        let input = dictionary_embedding_input(entry);

        assert!(input.starts_with("title: klama | text: "));
        assert!(input.contains("come"));
        assert!(input.contains("go"));
        assert!(input.contains('\u{2423}'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cll_embedding_title_matches_v0_rules() {
        let chunk = new!(CllSearchChunk {
            kind: CllSearchChunkKind::Example,
            role: None,
            section_id: "section-klama".to_owned(),
            anchor_id: "example".to_owned(),
            section_number: Some("2.1".to_owned()),
            section_title: "A test section".to_owned(),
            label: "Example 2.3".to_owned(),
            text: "mi klama".to_owned(),
            tagged_words: Default::default(),
        });
        assert_eq!(cll_embedding_title(&chunk), "2.1 2.3");

        let paragraph = chunk.with_data(data! {
            kind: CllSearchChunkKind::Paragraph,
            label: "Paragraph in 2.1. A test section".to_owned(),
        });
        assert_eq!(cll_embedding_title(&paragraph), "2.1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_id_hash_bytes_are_target_independent() {
        assert_eq!(document_id_hash_bytes(0), [0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(document_id_hash_bytes(1), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            document_id_hash_bytes(0x0102_0304),
            [4, 3, 2, 1, 0, 0, 0, 0]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn corpus_document_hash_uses_fixed_width_document_ids() {
        let documents = [
            EmbeddingInputDocument {
                id: 1,
                input: "first".to_owned(),
                input_hash: sha256_hex_bytes(b"first"),
                kind: Some("example".to_owned()),
            },
            EmbeddingInputDocument {
                id: 0x0102_0304,
                input: "second".to_owned(),
                input_hash: sha256_hex_bytes(b"second"),
                kind: None,
            },
        ];

        assert_eq!(
            input_documents_hash("test-corpus", &documents),
            "6c80634525e74a7bff41c626509a90110dbd2906bb8e03b9b9f4b1968202549c"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exported_corpus_has_whole_and_per_entry_hashes() {
        let corpus = embedding_input_corpus().unwrap();
        assert_eq!(corpus.model_key, DEFAULT_MODEL_KEY);
        assert_eq!(
            corpus.input_hash,
            "baaf35f8f6fe22617a74efb770736886b275271510e0123c55b623582e17f011"
        );
        assert_eq!(
            corpus.dictionary_hash,
            "93842b1db26acb5367c43be89f832a8331e8521def8b019eac9b14c938262e77"
        );
        assert_eq!(
            corpus.cll_hash,
            "2e87a303741701a65b6dc97ea2bb6fd35af52dbb10d5f4121411038a08c3409d"
        );
        assert_eq!(corpus.input_hash.len(), 64);
        assert!(
            corpus
                .dictionary
                .iter()
                .all(|doc| doc.input_hash.len() == 64)
        );
        assert!(corpus.cll.iter().all(|doc| doc.input_hash.len() == 64));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exported_corpus_json_propagates_successfully() {
        let json = embedding_input_corpus_json().unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&json).unwrap();
        assert_eq!(
            value.get("modelKey").and_then(serde_json::Value::as_str),
            Some(DEFAULT_MODEL_KEY)
        );
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn canonical_corpus_json_round_trips_through_validated_deserialization() {
        let expected = embedding_input_corpus().expect("embedded corpus");
        let json = serde_json::to_string(&expected).expect("serialize corpus");
        let actual = EmbeddingInputCorpus::from_json(&json).expect("validate corpus");
        assert_eq!(actual, expected);
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn corpus_deserialization_recomputes_document_and_aggregate_hashes() {
        let json = embedding_input_corpus_json().expect("corpus JSON");
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        value["dictionary"][0]["input"] = serde_json::json!("tampered");
        let tampered = serde_json::to_string(&value).expect("serialize tampered corpus");
        let error = EmbeddingInputCorpus::from_json(&tampered).expect_err("hash mismatch");
        let data!(EmbeddingInputCorpusError::Fingerprint { field, .. }) = error.as_data() else {
            panic!("expected fingerprint error, got {error}");
        };
        assert_eq!(field, "vlacku-en[0].inputHash");

        let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        value["dictionaryHash"] =
            serde_json::json!("0000000000000000000000000000000000000000000000000000000000000000");
        let tampered = serde_json::to_string(&value).expect("serialize tampered corpus");
        let error = EmbeddingInputCorpus::from_json(&tampered).expect_err("aggregate mismatch");
        let data!(EmbeddingInputCorpusError::Fingerprint { field, .. }) = error.as_data() else {
            panic!("expected fingerprint error, got {error}");
        };
        assert_eq!(field, "dictionaryHash");

        let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        value["cll"][0]["input"] = serde_json::json!("tampered");
        let tampered = serde_json::to_string(&value).expect("serialize tampered corpus");
        let error = EmbeddingInputCorpus::from_json(&tampered).expect_err("CLL hash mismatch");
        let data!(EmbeddingInputCorpusError::Fingerprint { field, .. }) = error.as_data() else {
            panic!("expected fingerprint error, got {error}");
        };
        assert_eq!(field, "cukta-cll[0].inputHash");

        for field in ["cllHash", "inputHash"] {
            let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
            value[field] = serde_json::json!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            );
            let tampered = serde_json::to_string(&value).expect("serialize tampered corpus");
            let error = EmbeddingInputCorpus::from_json(&tampered).expect_err("aggregate mismatch");
            let data!(EmbeddingInputCorpusError::Fingerprint {
                field: actual_field,
                ..
            }) = error.as_data()
            else {
                panic!("expected fingerprint error, got {error}");
            };
            assert_eq!(actual_field, field);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn corpus_deserialization_rejects_noncanonical_document_ids() {
        let json = embedding_input_corpus_json().expect("corpus JSON");
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        value["cll"][0]["id"] = serde_json::json!(9);
        let tampered = serde_json::to_string(&value).expect("serialize tampered corpus");
        let error = EmbeddingInputCorpus::from_json(&tampered).expect_err("document id mismatch");
        assert!(matches!(
            error.as_data(),
            data!(EmbeddingInputCorpusError::DocumentId {
                corpus,
                expected: 0,
                actual: 9,
            }) if corpus == "cll"
        ));
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn corpus_model_key_is_metadata_not_a_web_artifact_constraint() {
        let json = embedding_input_corpus_json().expect("corpus JSON");
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        value["modelKey"] = serde_json::json!("different-target-model");
        let changed = serde_json::to_string(&value).expect("serialize changed corpus");
        let corpus = EmbeddingInputCorpus::from_json(&changed).expect("metadata is allowed");
        assert_eq!(corpus.model_key, "different-target-model");
    }
}
