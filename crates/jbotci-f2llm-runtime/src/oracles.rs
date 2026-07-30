#[allow(unused_imports)]
use bityzba::{ensures, requires};
use serde_json::Value;

use crate::Sha256Digest;

const PROVENANCE: &str = include_str!("../testdata/goldens/provenance.json");
const CURRENT_80M: &[u8] =
    include_bytes!("../testdata/goldens/current-v0.2.0/f2llm-v2-80m-q4-320/goldens.json");
const LEGACY_GENERATOR: &[u8] =
    include_bytes!("../../../tools/f2llm-oracles/legacy-v0.1.0/generate-f2llm-goldens.py");
const LEGACY_ORCHESTRATOR: &[u8] =
    include_bytes!("../../../tools/f2llm-oracles/legacy-v0.1.0/prepare-f2llm-size.py");
const LEGACY_REQUIREMENTS: &[u8] =
    include_bytes!("../../../tools/f2llm-oracles/legacy-v0.1.0/requirements.txt");
const LEGACY_README: &[u8] = include_bytes!("../../../tools/f2llm-oracles/legacy-v0.1.0/README.md");
const LEGACY_REFERENCE_HARNESS: &[u8] =
    include_bytes!("../../../tools/f2llm-oracles/legacy-v0.1.0/test-runtime-reference.mjs");
const CURRENT_GENERATOR: &[u8] =
    include_bytes!("../../../tools/f2llm-oracles/generate-f2llm-goldens.py");
const CURRENT_REQUIREMENTS: &[u8] = include_bytes!("../../../tools/f2llm-oracles/requirements.txt");

#[requires(!path.is_empty())]
#[ensures(ret.is_some() == matches!(
    path,
    "crates/jbotci-f2llm-runtime/testdata/goldens/legacy-v0.1.0/f2llm-v2-160m-q4-640/goldens.json"
        | "crates/jbotci-f2llm-runtime/testdata/goldens/legacy-v0.1.0/f2llm-v2-330m-q4-896/goldens.json"
        | "crates/jbotci-f2llm-runtime/testdata/goldens/legacy-v0.1.0/f2llm-v2-0.6b-q4-1024/goldens.json"
))]
fn legacy_golden_bytes(path: &str) -> Option<&'static [u8]> {
    match path {
        "crates/jbotci-f2llm-runtime/testdata/goldens/legacy-v0.1.0/f2llm-v2-160m-q4-640/goldens.json" => {
            Some(include_bytes!(
                "../testdata/goldens/legacy-v0.1.0/f2llm-v2-160m-q4-640/goldens.json"
            ))
        }
        "crates/jbotci-f2llm-runtime/testdata/goldens/legacy-v0.1.0/f2llm-v2-330m-q4-896/goldens.json" => {
            Some(include_bytes!(
                "../testdata/goldens/legacy-v0.1.0/f2llm-v2-330m-q4-896/goldens.json"
            ))
        }
        "crates/jbotci-f2llm-runtime/testdata/goldens/legacy-v0.1.0/f2llm-v2-0.6b-q4-1024/goldens.json" => {
            Some(include_bytes!(
                "../testdata/goldens/legacy-v0.1.0/f2llm-v2-0.6b-q4-1024/goldens.json"
            ))
        }
        _ => None,
    }
}

#[requires(!path.is_empty())]
#[ensures(ret.is_some() == matches!(
    path,
    "crates/jbotci-f2llm-runtime/testdata/artifacts/legacy-v0.1.0/f2llm-v2-160m-webgpu/v1/manifest.json"
        | "crates/jbotci-f2llm-runtime/testdata/artifacts/legacy-v0.1.0/f2llm-v2-330m-webgpu/v1/manifest.json"
        | "crates/jbotci-f2llm-runtime/testdata/artifacts/legacy-v0.1.0/f2llm-v2-0.6b-webgpu/v1/manifest.json"
))]
fn legacy_manifest_bytes(path: &str) -> Option<&'static [u8]> {
    match path {
        "crates/jbotci-f2llm-runtime/testdata/artifacts/legacy-v0.1.0/f2llm-v2-160m-webgpu/v1/manifest.json" => {
            Some(include_bytes!(
                "../testdata/artifacts/legacy-v0.1.0/f2llm-v2-160m-webgpu/v1/manifest.json"
            ))
        }
        "crates/jbotci-f2llm-runtime/testdata/artifacts/legacy-v0.1.0/f2llm-v2-330m-webgpu/v1/manifest.json" => {
            Some(include_bytes!(
                "../testdata/artifacts/legacy-v0.1.0/f2llm-v2-330m-webgpu/v1/manifest.json"
            ))
        }
        "crates/jbotci-f2llm-runtime/testdata/artifacts/legacy-v0.1.0/f2llm-v2-0.6b-webgpu/v1/manifest.json" => {
            Some(include_bytes!(
                "../testdata/artifacts/legacy-v0.1.0/f2llm-v2-0.6b-webgpu/v1/manifest.json"
            ))
        }
        _ => None,
    }
}

#[requires(!path.is_empty())]
#[ensures(ret.is_some() == matches!(
    path,
    "tools/f2llm-oracles/legacy-v0.1.0/generate-f2llm-goldens.py"
        | "tools/f2llm-oracles/legacy-v0.1.0/prepare-f2llm-size.py"
        | "tools/f2llm-oracles/legacy-v0.1.0/requirements.txt"
        | "tools/f2llm-oracles/legacy-v0.1.0/README.md"
        | "tools/f2llm-oracles/legacy-v0.1.0/test-runtime-reference.mjs"
))]
fn legacy_harness_bytes(path: &str) -> Option<&'static [u8]> {
    match path {
        "tools/f2llm-oracles/legacy-v0.1.0/generate-f2llm-goldens.py" => Some(LEGACY_GENERATOR),
        "tools/f2llm-oracles/legacy-v0.1.0/prepare-f2llm-size.py" => Some(LEGACY_ORCHESTRATOR),
        "tools/f2llm-oracles/legacy-v0.1.0/requirements.txt" => Some(LEGACY_REQUIREMENTS),
        "tools/f2llm-oracles/legacy-v0.1.0/README.md" => Some(LEGACY_README),
        "tools/f2llm-oracles/legacy-v0.1.0/test-runtime-reference.mjs" => {
            Some(LEGACY_REFERENCE_HARNESS)
        }
        _ => None,
    }
}

#[requires(bytes.len() % 4 == 0)]
#[ensures(ret.len() == bytes.len() / 4)]
fn decode_f32le(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four-byte chunk")))
        .collect()
}

#[requires(values.iter().all(|value| value.as_f64().is_some()))]
#[ensures(ret.len() == values.len() * 4)]
fn encode_json_f32le(values: &[Value]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| {
            (value.as_f64().expect("numeric embedding component") as f32).to_le_bytes()
        })
        .collect()
}

#[requires(!values.is_empty())]
#[ensures(ret.is_finite())]
fn squared_norm(values: &[f32]) -> f32 {
    values.iter().map(|value| value * value).sum()
}

#[requires(true)]
#[ensures(true)]
#[test]
fn vendored_legacy_bytes_are_bound_to_generator_and_artifact_digests() {
    let provenance: Value = serde_json::from_str(PROVENANCE).expect("provenance JSON");
    let generator = &provenance["legacy_generator"];
    for (path_field, digest_field) in [
        ("path", "sha256"),
        ("orchestrator_path", "orchestrator_sha256"),
        ("requirements_path", "requirements_sha256"),
        ("readme_path", "readme_sha256"),
        ("reference_harness_path", "reference_harness_sha256"),
    ] {
        let path = generator[path_field].as_str().expect("harness path");
        let bytes = legacy_harness_bytes(path).expect("vendored harness bytes");
        assert_eq!(
            Sha256Digest::of_bytes(bytes).as_str(),
            generator[digest_field].as_str().expect("harness digest")
        );
    }

    let fixtures = provenance["legacy_fixtures"]
        .as_array()
        .expect("legacy fixture array");
    assert_eq!(fixtures.len(), 3);
    for fixture in fixtures {
        assert_eq!(fixture["runtime_version"], "0.1.0");
        assert_eq!(fixture["reference_q4_onnx_available"], false);

        let golden_path = fixture["golden_path"].as_str().expect("golden path");
        let golden_bytes = legacy_golden_bytes(golden_path).expect("vendored golden");
        assert_eq!(
            Sha256Digest::of_bytes(golden_bytes).as_str(),
            fixture["golden_sha256"].as_str().expect("golden digest")
        );
        let golden: Value = serde_json::from_slice(golden_bytes).expect("legacy golden JSON");
        assert_eq!(golden["schema_version"], 1);
        assert_eq!(golden["runtime_version"], "0.1.0");
        assert_eq!(golden["model_key"], fixture["model_key"]);
        assert_eq!(golden["cases"].as_array().expect("legacy cases").len(), 5);

        let manifest_path = fixture["source_artifact_manifest_path"]
            .as_str()
            .expect("artifact manifest path");
        let manifest_bytes =
            legacy_manifest_bytes(manifest_path).expect("vendored artifact manifest");
        assert_eq!(
            Sha256Digest::of_bytes(manifest_bytes).as_str(),
            fixture["source_artifact_manifest_sha256"]
                .as_str()
                .expect("artifact manifest digest")
        );
        let manifest: Value =
            serde_json::from_slice(manifest_bytes).expect("artifact manifest JSON");
        assert_eq!(manifest["artifact_version"], "0.1.0");
        assert_eq!(manifest["model_key"], fixture["model_key"]);
        assert_ne!(
            fixture["source_artifact_manifest_sha256"],
            fixture["current_artifact_manifest_sha256"]
        );
        assert_eq!(
            fixture["reference_q4_onnx_sha256"]
                .as_str()
                .expect("reference ONNX digest")
                .len(),
            64
        );
    }
}

#[requires(true)]
#[ensures(true)]
#[test]
fn current_80m_oracle_covers_window_and_batch_boundaries_with_verified_bytes() {
    let provenance: Value = serde_json::from_str(PROVENANCE).expect("provenance JSON");
    let current_fixture = &provenance["current_fixture"];
    assert_eq!(current_fixture["runtime_version"], "0.2.0");
    assert_eq!(
        current_fixture["golden_path"],
        "crates/jbotci-f2llm-runtime/testdata/goldens/current-v0.2.0/f2llm-v2-80m-q4-320/goldens.json"
    );
    assert_eq!(
        current_fixture["generator_path"],
        "tools/f2llm-oracles/generate-f2llm-goldens.py"
    );
    assert_eq!(
        current_fixture["requirements_path"],
        "tools/f2llm-oracles/requirements.txt"
    );
    assert_eq!(
        Sha256Digest::of_bytes(CURRENT_80M).as_str(),
        current_fixture["golden_sha256"]
            .as_str()
            .expect("current golden digest")
    );
    assert_eq!(
        Sha256Digest::of_bytes(CURRENT_GENERATOR).as_str(),
        current_fixture["generator_sha256"]
            .as_str()
            .expect("current generator digest")
    );
    assert_eq!(
        Sha256Digest::of_bytes(CURRENT_REQUIREMENTS).as_str(),
        current_fixture["requirements_sha256"]
            .as_str()
            .expect("current requirements digest")
    );

    let golden: Value = serde_json::from_slice(CURRENT_80M).expect("current golden JSON");
    assert_eq!(golden["schema_version"], 2);
    assert_eq!(golden["runtime"], "jbotci-webgpu-f2llm");
    assert_eq!(golden["runtime_version"], "0.2.0");
    assert_eq!(golden["dimensions"], 320);
    assert_eq!(golden["max_sequence_length"], 512);
    assert_eq!(golden["pooling"], "mean_normalized_windows");
    assert_eq!(
        Sha256Digest::of_bytes(CURRENT_GENERATOR).as_str(),
        golden["reference"]["generator"]["sha256"]
            .as_str()
            .expect("generator digest")
    );
    assert_eq!(
        golden["reference"]["published_onnx"]["model_sha256"],
        current_fixture["published_q4_onnx_sha256"]
    );
    assert_eq!(
        golden["reference"]["published_onnx"]["manifest_sha256"],
        current_fixture["published_q4_onnx_manifest_sha256"]
    );
    assert_eq!(
        golden["reference"]["published_onnx"]["model_byte_length"],
        55_252_118
    );
    assert_eq!(
        golden["reference"]["target_artifact"]["manifest_sha256"],
        current_fixture["target_artifact_manifest_sha256"]
    );
    assert_eq!(
        golden["reference"]["target_artifact"]["artifact_version"],
        "0.2.0"
    );

    let cases = golden["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 13);
    let mut actual_windows = Vec::new();
    for (case_index, case) in cases.iter().enumerate() {
        let input = case["input"].as_str().expect("case input");
        assert_eq!(
            Sha256Digest::of_bytes(input.as_bytes()).as_str(),
            case["input_sha256"].as_str().expect("input digest")
        );
        let token_ids = case["token_ids"].as_array().expect("token IDs");
        let windows = case["windows"].as_array().expect("token windows");
        let flattened = windows
            .iter()
            .flat_map(|window| window.as_array().expect("window").iter())
            .collect::<Vec<_>>();
        assert_eq!(flattened, token_ids.iter().collect::<Vec<_>>());
        assert_eq!(
            case["token_count"].as_u64().expect("token count") as usize,
            token_ids.len()
        );
        for (window_index, window) in windows.iter().enumerate() {
            let window = window.as_array().expect("window");
            assert!(!window.is_empty());
            assert!(window.len() <= 512);
            actual_windows.push((case_index, window_index));
        }

        let embedding = case["embedding"].as_array().expect("embedding");
        assert_eq!(embedding.len(), 320);
        let embedding_bytes = encode_json_f32le(embedding);
        assert_eq!(
            Sha256Digest::of_bytes(&embedding_bytes).as_str(),
            case["embedding_f32le_sha256"]
                .as_str()
                .expect("embedding digest")
        );
        let values = decode_f32le(&embedding_bytes);
        assert!((squared_norm(&values) - 1.0).abs() < 0.000_002);

        let window_embeddings = case["window_embeddings"]
            .as_array()
            .expect("window embeddings");
        assert_eq!(window_embeddings.len(), windows.len());
        for window_embedding in window_embeddings {
            let embedding = window_embedding["embedding"]
                .as_array()
                .expect("window embedding");
            assert_eq!(embedding.len(), 320);
            let bytes = encode_json_f32le(embedding);
            assert_eq!(
                Sha256Digest::of_bytes(&bytes).as_str(),
                window_embedding["embedding_f32le_sha256"]
                    .as_str()
                    .expect("window embedding digest")
            );
        }
    }

    let by_name = |name: &str| {
        cases
            .iter()
            .find(|case| case["name"] == name)
            .expect("named case")
    };
    assert_eq!(by_name("empty")["input"], "");
    assert!(
        by_name("non-ascii")["input"]
            .as_str()
            .expect("non-ASCII input")
            .chars()
            .any(|character| !character.is_ascii())
    );
    for token_count in [511_u64, 512, 513] {
        assert_eq!(
            by_name(&format!("token-length-{token_count}"))["token_count"],
            token_count
        );
    }
    assert_eq!(
        by_name("multi-window-1025")["window_token_counts"],
        serde_json::json!([512, 512, 1])
    );

    let execution = &golden["execution"];
    assert_eq!(execution["window_batch_size"], 8);
    let batches = execution["batches"].as_array().expect("execution batches");
    let recorded_windows = batches
        .iter()
        .flat_map(|batch| batch["windows"].as_array().expect("batch windows"))
        .map(|window| {
            (
                window["case_index"].as_u64().expect("case index") as usize,
                window["window_index"].as_u64().expect("window index") as usize,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(recorded_windows, actual_windows);
    assert_eq!(
        batches[0]["windows"].as_array().expect("first batch")[7],
        serde_json::json!({"case_index": 7, "window_index": 0})
    );
    assert_eq!(
        batches[1]["windows"].as_array().expect("second batch")[0],
        serde_json::json!({"case_index": 8, "window_index": 0})
    );
}
