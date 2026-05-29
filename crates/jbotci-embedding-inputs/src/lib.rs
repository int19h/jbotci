//! Canonical text inputs for embedding-based jbotci search.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_cll::{CllSearchChunk, CllSearchChunkKind, cll_search_all_chunks};
use jbotci_dictionary::{Dictionary, DictionaryEntry};
use jbotci_search::vlacku::{grouped_word_type_filter_key, normalize_word_type_filter};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const DEFAULT_MODEL_KEY: &str = "embedding-gemma-300m-q4-768";
pub const DEFAULT_MODEL_REVISION: &str = "8dd0ca2a66a8f14470acb0e2a71f801afbc5fb73";
pub const DEFAULT_MODEL_DIMENSIONS: usize = 768;
pub const DEFAULT_INPUT_FORMAT_VERSION: &str = "egemma-v0-parity-2";
pub const VLACKU_CORPUS_ID: &str = "vlacku-en";
pub const CUKTA_CORPUS_ID: &str = "cukta-cll";
pub const RETRIEVAL_QUERY_PREFIX: &str = "task: search result | query: ";
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
    RETRIEVAL_DOCUMENT_PREFIX
        .replace("{title}", safe_title)
        .replace("{text}", content)
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

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cll_section_number_fallback(chunk: &CllSearchChunk) -> String {
    if chunk.section_number.trim().is_empty() {
        chunk.section_id.clone()
    } else {
        chunk.section_number.clone()
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
    hex_digest(Sha256::digest(bytes))
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
    hex_digest(hasher.finalize())
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
    hex_digest(hasher.finalize())
}

#[requires(true)]
#[ensures(!ret.dictionary.is_empty())]
pub fn embedding_input_corpus() -> EmbeddingInputCorpus {
    let dictionary = jbotci_dictionary_data::english();
    let cll = jbotci_cll::embedded_cll_site()
        .map(|site| cll_search_all_chunks(site).to_vec())
        .unwrap_or_default();
    embedding_input_corpus_from_parts(dictionary, &cll)
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
        hasher.update(document.id.to_le_bytes());
        hasher.update([0]);
        hasher.update(document.input_hash.as_bytes());
        hasher.update([0]);
        if let Some(kind) = &document.kind {
            hasher.update(kind.as_bytes());
        }
        hasher.update([0]);
    }
    hex_digest(hasher.finalize())
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
    hex_digest(hasher.finalize())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn embedding_input_corpus_json() -> String {
    serde_json::to_string(&embedding_input_corpus()).unwrap_or_else(|_| "{}".to_owned())
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
    use bityzba::{ensures, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn retrieval_prefixes_match_v0() {
        assert_eq!(
            build_retrieval_query_input("klama"),
            "task: search result | query: klama"
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
        let chunk = CllSearchChunk {
            kind: CllSearchChunkKind::Example,
            section_id: "section-klama".to_owned(),
            anchor_id: "example".to_owned(),
            section_number: "2.1".to_owned(),
            section_title: "A test section".to_owned(),
            label: "Example 2.3".to_owned(),
            text: "mi klama".to_owned(),
            tagged_words: Default::default(),
        };
        assert_eq!(cll_embedding_title(&chunk), "2.1 2.3");

        let paragraph = CllSearchChunk {
            kind: CllSearchChunkKind::Paragraph,
            label: "Paragraph in 2.1. A test section".to_owned(),
            ..chunk
        };
        assert_eq!(cll_embedding_title(&paragraph), "2.1");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exported_corpus_has_whole_and_per_entry_hashes() {
        let corpus = embedding_input_corpus();
        assert_eq!(corpus.model_key, DEFAULT_MODEL_KEY);
        assert_eq!(corpus.input_hash.len(), 64);
        assert!(
            corpus
                .dictionary
                .iter()
                .all(|doc| doc.input_hash.len() == 64)
        );
        assert!(corpus.cll.iter().all(|doc| doc.input_hash.len() == 64));
    }
}
