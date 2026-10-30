#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_f2llm_runtime::QwenByteBpeTokenizer;
use serde::Deserialize;
use serde_json::Value;

#[invariant(true)]
#[derive(Deserialize)]
struct GoldenCase {
    text: String,
    token_ids: Vec<u32>,
}

#[invariant(true)]
#[derive(Deserialize)]
struct TokenizerGoldens {
    source_revision: String,
    tokenizer: Value,
    cases: Vec<GoldenCase>,
}

#[test]
#[requires(true)]
#[ensures(true)]
fn full_token_ids_match_the_pinned_reference() {
    let golden: TokenizerGoldens =
        serde_json::from_slice(include_bytes!("../testdata/tokenizer/pinned.json")).unwrap();
    assert_eq!(
        golden.source_revision,
        "f4a16a11c9f5c8c7e22694653de6ce75430f4538"
    );
    let tokenizer =
        QwenByteBpeTokenizer::from_compact_json(&serde_json::to_vec(&golden.tokenizer).unwrap())
            .unwrap();
    assert_eq!(golden.cases.len(), 44);
    assert_eq!(
        golden.tokenizer["added_tokens"].as_array().unwrap().len(),
        26
    );
    for case in golden.cases {
        assert_eq!(
            tokenizer.encode_untruncated(&case.text).unwrap(),
            case.token_ids,
            "complete token IDs for {:?}",
            case.text
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tokenizer_metadata_must_describe_supported_matching_rules() {
    let golden: TokenizerGoldens =
        serde_json::from_slice(include_bytes!("../testdata/tokenizer/pinned.json")).unwrap();
    for field in ["single_word", "lstrip", "rstrip", "normalized"] {
        let mut artifact = golden.tokenizer.clone();
        artifact["added_tokens"][0][field] = Value::Bool(true);
        let error =
            QwenByteBpeTokenizer::from_compact_json(&serde_json::to_vec(&artifact).unwrap())
                .unwrap_err();
        assert!(
            error.contains("unsupported F2LLM added token matching rules"),
            "{error}"
        );
    }
    for field in ["pre_tokenizer_pattern", "added_tokens"] {
        let mut artifact = golden.tokenizer.clone();
        artifact.as_object_mut().unwrap().remove(field);
        assert!(
            QwenByteBpeTokenizer::from_compact_json(&serde_json::to_vec(&artifact).unwrap())
                .is_err()
        );
    }
    let mut artifact = golden.tokenizer.clone();
    artifact["pre_tokenizer_pattern"] = Value::String("case-sensitive".to_owned());
    assert!(
        QwenByteBpeTokenizer::from_compact_json(&serde_json::to_vec(&artifact).unwrap())
            .unwrap_err()
            .contains("pinned pattern")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn added_tokens_are_nonempty_and_unambiguous() {
    let golden: TokenizerGoldens =
        serde_json::from_slice(include_bytes!("../testdata/tokenizer/pinned.json")).unwrap();
    let mut artifact = golden.tokenizer.clone();
    artifact["added_tokens"][0]["content"] = Value::String(String::new());
    assert!(
        QwenByteBpeTokenizer::from_compact_json(&serde_json::to_vec(&artifact).unwrap())
            .unwrap_err()
            .contains("must not be empty")
    );
    for field in ["content", "id"] {
        let mut artifact = golden.tokenizer.clone();
        artifact["added_tokens"][1][field] = artifact["added_tokens"][0][field].clone();
        assert!(
            QwenByteBpeTokenizer::from_compact_json(&serde_json::to_vec(&artifact).unwrap())
                .unwrap_err()
                .contains("must be unique")
        );
    }
}
