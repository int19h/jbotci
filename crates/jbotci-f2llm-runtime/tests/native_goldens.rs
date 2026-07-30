#![cfg(all(feature = "native", not(target_arch = "wasm32")))]

use std::cell::Cell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use jbotci_f2llm_runtime::{
    CorpusShard, CorpusVectorSpec, DEFAULT_NATIVE_ARTIFACT_ROOT, DirectoryArtifactSource,
    DirectoryRoot, F2LLM_WEBGPU_RUNTIME, ProgressError, ProgressEvent, ProgressSink,
    RuntimeCapabilities, RuntimeFuture, RuntimeLoadOptions, Sha256Digest, VectorStore,
    VectorStoreError, VectorStoreKey, WebGpuRuntime,
};
use serde_json::{Value, json};

const MIN_REFERENCE_COSINE: f32 = 0.999;
const GOLDEN_MODE_ENV: &str = "JBOTCI_F2LLM_GOLDEN_MODE";
const ARTIFACT_ROOT_ENV: &str = "JBOTCI_F2LLM_ARTIFACT_ROOT";
const EVIDENCE_PATH_ENV: &str = "JBOTCI_F2LLM_NATIVE_EVIDENCE";
const FORCE_FALLBACK_ENV: &str = "JBOTCI_F2LLM_FORCE_FALLBACK_ADAPTER";

#[invariant(!id.is_empty())]
#[invariant(!model_key.is_empty())]
#[invariant(!golden_relative.is_empty())]
#[derive(Debug, Clone, Copy)]
struct GoldenSpec {
    id: &'static str,
    model_key: &'static str,
    golden_relative: &'static str,
}

#[invariant(reports.get() < usize::MAX)]
#[derive(Debug)]
struct CountingProgress {
    reports: Cell<usize>,
}

#[contract_trait]
impl ProgressSink for CountingProgress {
    fn report<'a>(
        &'a mut self,
        _event: &'a ProgressEvent,
    ) -> RuntimeFuture<'a, Result<(), ProgressError>> {
        self.reports.set(self.reports.get() + 1);
        Box::pin(async { Ok(()) })
    }
}

#[invariant(!message.is_empty())]
#[derive(Debug)]
struct RejectingVectorStore {
    message: &'static str,
}

#[contract_trait]
impl VectorStore for RejectingVectorStore {
    fn read<'a>(
        &'a self,
        key: &'a VectorStoreKey,
    ) -> RuntimeFuture<'a, Result<Vec<u8>, VectorStoreError>> {
        Box::pin(async move { Err(VectorStoreError::new(key.clone(), self.message.to_owned())) })
    }
}

#[requires(true)]
#[ensures(true)]
#[test]
fn all_vendored_native_goldens() {
    let artifact_root = env::var_os(ARTIFACT_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NATIVE_ARTIFACT_ROOT));
    let force_fallback = match env::var(FORCE_FALLBACK_ENV).ok().as_deref() {
        None | Some("0") => false,
        Some("1") => true,
        Some(value) => panic!("{FORCE_FALLBACK_ENV} must be `0` or `1`, got `{value}`"),
    };
    let specs = golden_specs();
    let selected = match env::var(GOLDEN_MODE_ENV).ok().as_deref() {
        None | Some("all") => specs.as_slice(),
        Some("80m") => &specs[..1],
        Some(value) => panic!("{GOLDEN_MODE_ENV} must be `all` or `80m`, got `{value}`"),
    };
    let mut evidence_models = Vec::with_capacity(selected.len());
    let mut common_adapter = None;

    for spec in selected {
        let golden_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(spec.golden_relative);
        let golden: Value = serde_json::from_slice(
            &fs::read(&golden_path)
                .unwrap_or_else(|error| panic!("reading `{}`: {error}", golden_path.display())),
        )
        .unwrap_or_else(|error| panic!("parsing `{}`: {error}", golden_path.display()));
        assert_eq!(golden["model_key"], spec.model_key);
        let dimensions = required_usize(&golden, "dimensions");
        let max_sequence_length = golden["max_sequence_length"]
            .as_u64()
            .map(|value| value as usize)
            .unwrap_or(512);
        let model_artifact_root = artifact_root.join(spec.model_key);
        let source = DirectoryArtifactSource::new(
            DirectoryRoot::open(&model_artifact_root).unwrap_or_else(|error| {
                panic!(
                    "opening native artifact root `{}`: {error}; run the documented artifact download command or set {ARTIFACT_ROOT_ENV}",
                    model_artifact_root.display()
                )
            }),
        );
        let mut options = RuntimeLoadOptions::new(
            spec.model_key.to_owned(),
            F2LLM_WEBGPU_RUNTIME.to_owned(),
            required_string(&golden, "runtime_version").to_owned(),
            max_sequence_length,
            dimensions,
            RuntimeCapabilities::EmbeddingOnly,
        );
        if force_fallback {
            options = options.with_force_fallback_adapter_for_testing();
        }
        let mut progress = new!(CountingProgress {
            reports: Cell::new(0),
        });
        let mut runtime = pollster::block_on(WebGpuRuntime::load(options, &source, &mut progress))
            .unwrap_or_else(|error| panic!("loading {} native runtime: {error}", spec.model_key));
        assert_eq!(runtime.capabilities(), RuntimeCapabilities::EmbeddingOnly);
        assert!(progress.reports.get() > 1);

        let adapter = runtime.adapter_info();
        let adapter_evidence = json!({
            "name": adapter.name(),
            "backend": adapter.backend(),
            "device_type": adapter.device_type(),
            "driver": adapter.driver(),
            "driver_info": adapter.driver_info(),
            "force_fallback_adapter": force_fallback,
        });
        if let Some(common) = &common_adapter {
            assert_eq!(
                common, &adapter_evidence,
                "all native golden models must use the same adapter"
            );
        } else {
            common_adapter = Some(adapter_evidence.clone());
        }
        println!(
            "F2LLM native adapter for {}: {} ({}, {}, driver={} {})",
            spec.model_key,
            adapter.name(),
            adapter.backend(),
            adapter.device_type(),
            adapter.driver(),
            adapter.driver_info(),
        );

        assert_embedding_only_scoring_is_typed(&mut runtime, dimensions);
        let cases = golden["cases"]
            .as_array()
            .unwrap_or_else(|| panic!("{} golden cases must be an array", spec.model_key));
        let inputs = cases
            .iter()
            .map(|case| json_string(case, "input").to_owned())
            .collect::<Vec<_>>();
        let mut token_windows = Vec::with_capacity(cases.len());
        for case in cases {
            let name = required_string(case, "name");
            let actual = runtime
                .token_windows(json_string(case, "input"))
                .unwrap_or_else(|error| panic!("tokenizing {name}: {error}"));
            let expected = expected_windows(case);
            assert_eq!(actual, expected, "{spec:?} case `{name}` token windows");
            let flattened = actual.iter().flatten().copied().collect::<Vec<_>>();
            assert_eq!(
                flattened,
                expected_token_ids(case),
                "{spec:?} case `{name}` token IDs"
            );
            token_windows.push(actual);
        }

        let embeddings = pollster::block_on(runtime.embed_texts(&inputs))
            .unwrap_or_else(|error| panic!("embedding {} golden cases: {error}", spec.model_key));
        assert_eq!(embeddings.len(), cases.len());
        let mut evidence_cases = Vec::with_capacity(cases.len());
        let mut model_min_cosine = 1.0_f32;
        for ((case, windows), embedding) in cases.iter().zip(token_windows).zip(embeddings) {
            let name = required_string(case, "name");
            assert_eq!(embedding.len(), dimensions, "{name} dimensions");
            let expected = expected_embedding(case);
            let cosine = cosine_similarity(&embedding, &expected).unwrap_or_else(|error| {
                panic!(
                    "{} case `{name}` cannot compare native and reference embeddings: {error}",
                    spec.model_key
                )
            });
            model_min_cosine = model_min_cosine.min(cosine);
            assert!(
                cosine >= MIN_REFERENCE_COSINE,
                "{} case `{name}` cosine {cosine:.9} is below {MIN_REFERENCE_COSINE}",
                spec.model_key
            );
            let bytes = f32le_bytes(&embedding);
            evidence_cases.push(json!({
                "name": name,
                "token_ids": windows.iter().flatten().copied().collect::<Vec<_>>(),
                "windows": windows,
                "embedding_f32le_hex": lower_hex(&bytes),
                "embedding_f32le_sha256": Sha256Digest::of_bytes(&bytes).as_str(),
                "reference_cosine": cosine,
            }));
        }
        println!(
            "F2LLM native golden {} passed {} cases: min reference cosine {:.9}, exact token IDs/windows",
            spec.model_key,
            cases.len(),
            model_min_cosine
        );
        evidence_models.push(json!({
            "id": spec.id,
            "model_key": spec.model_key,
            "runtime_version": required_string(&golden, "runtime_version"),
            "dimensions": dimensions,
            "max_sequence_length": max_sequence_length,
            "golden_path": spec.golden_relative,
            "case_count": cases.len(),
            "min_reference_cosine": model_min_cosine,
            "cases": evidence_cases,
        }));
    }

    let evidence = json!({
        "schema": "jbotci-f2llm-native-goldens-v1",
        "target": "native",
        "runtime": F2LLM_WEBGPU_RUNTIME,
        "capabilities": "embedding-only",
        "minimum_reference_cosine": MIN_REFERENCE_COSINE,
        "exact_token_ids": true,
        "exact_windows": true,
        "adapter": common_adapter.expect("at least one selected model"),
        "models": evidence_models,
    });
    let evidence_path = native_evidence_path();
    write_json(&evidence_path, &evidence);
    println!(
        "wrote native golden evidence to {}",
        evidence_path.display()
    );
}

#[requires(true)]
#[ensures(ret.len() == 4)]
fn golden_specs() -> Vec<GoldenSpec> {
    vec![
        new!(GoldenSpec {
            id: "80m",
            model_key: "f2llm-v2-80m-q4-320",
            golden_relative: "testdata/goldens/current-v0.2.0/f2llm-v2-80m-q4-320/goldens.json",
        }),
        new!(GoldenSpec {
            id: "160m",
            model_key: "f2llm-v2-160m-q4-640",
            golden_relative: "testdata/goldens/legacy-v0.1.0/f2llm-v2-160m-q4-640/goldens.json",
        }),
        new!(GoldenSpec {
            id: "330m",
            model_key: "f2llm-v2-330m-q4-896",
            golden_relative: "testdata/goldens/legacy-v0.1.0/f2llm-v2-330m-q4-896/goldens.json",
        }),
        new!(GoldenSpec {
            id: "0.6b",
            model_key: "f2llm-v2-0.6b-q4-1024",
            golden_relative: "testdata/goldens/legacy-v0.1.0/f2llm-v2-0.6b-q4-1024/goldens.json",
        }),
    ]
}

#[requires(dimensions > 0)]
#[ensures(true)]
fn assert_embedding_only_scoring_is_typed(runtime: &mut WebGpuRuntime, dimensions: usize) {
    let corpus = CorpusVectorSpec::new(
        "capability-probe".to_owned(),
        "capability-probe".to_owned(),
        1,
        dimensions,
        vec![CorpusShard::new(
            VectorStoreKey::parse("capability-probe").expect("literal vector key"),
            dimensions * 2,
        )],
    );
    let error = pollster::block_on(runtime.score_f16_vectors(
        &corpus,
        &vec![0.0; dimensions],
        &new!(RejectingVectorStore {
            message: "embedding-only scoring must fail before reading vectors",
        }),
    ))
    .expect_err("embedding-only scoring must fail");
    assert_eq!(
        error.unavailable_capability(),
        Some(RuntimeCapabilities::EmbeddingAndF16Scoring)
    );
}

#[requires(value.is_object())]
#[requires(!key.is_empty())]
#[ensures(!ret.is_empty())]
fn required_string<'a>(value: &'a Value, key: &str) -> &'a str {
    let value = json_string(value, key);
    assert!(!value.is_empty(), "golden field `{key}` must not be empty");
    value
}

#[requires(value.is_object())]
#[requires(!key.is_empty())]
#[ensures(true)]
fn json_string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("golden field `{key}` must be a string"))
}

#[requires(value.is_object())]
#[requires(!key.is_empty())]
#[ensures(ret > 0)]
fn required_usize(value: &Value, key: &str) -> usize {
    value[key]
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .filter(|number| *number > 0)
        .unwrap_or_else(|| panic!("golden field `{key}` must be a positive integer"))
}

#[requires(case.is_object())]
#[ensures(!ret.is_empty() && ret.iter().all(|window| !window.is_empty()))]
fn expected_windows(case: &Value) -> Vec<Vec<u32>> {
    if let Some(windows) = case["windows"].as_array() {
        windows
            .iter()
            .map(|window| json_u32_array(window, "window"))
            .collect()
    } else {
        vec![expected_token_ids(case)]
    }
}

#[requires(case.is_object())]
#[ensures(!ret.is_empty())]
fn expected_token_ids(case: &Value) -> Vec<u32> {
    json_u32_array(&case["token_ids"], "token_ids")
}

#[requires(case.is_object())]
#[ensures(!ret.is_empty())]
fn expected_embedding(case: &Value) -> Vec<f32> {
    case["embedding"]
        .as_array()
        .expect("golden embedding must be an array")
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|number| number as f32)
                .filter(|number| number.is_finite())
                .expect("golden embedding component must be finite")
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn json_u32_array(value: &Value, label: &str) -> Vec<u32> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("golden {label} must be an array"))
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|number| u32::try_from(number).ok())
                .unwrap_or_else(|| panic!("golden {label} token must be a u32"))
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|cosine| cosine.is_finite()) || ret.is_err())]
fn cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32, String> {
    if left.is_empty() || right.is_empty() {
        return Err("cannot compare an empty embedding".to_owned());
    }
    if left.len() != right.len() {
        return Err(format!(
            "embedding dimensions differ: {} and {}",
            left.len(),
            right.len()
        ));
    }
    if left.iter().chain(right).any(|value| !value.is_finite()) {
        return Err("cannot compare an embedding with non-finite components".to_owned());
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm == 0.0 || right_norm == 0.0 {
        return Err("cannot compare a zero embedding".to_owned());
    }
    if !left_norm.is_finite() || !right_norm.is_finite() {
        return Err("cannot compare an embedding with a non-finite norm".to_owned());
    }
    let cosine = dot / (left_norm * right_norm);
    if !cosine.is_finite() {
        return Err("embedding cosine is non-finite".to_owned());
    }
    Ok(cosine)
}

#[test]
#[requires(true)]
#[ensures(true)]
fn native_cosine_rejects_zero_and_non_finite_embeddings() {
    let reference = [1.0_f32, 0.0];

    assert!(
        cosine_similarity(&[0.0, -0.0], &reference)
            .unwrap_err()
            .contains("zero embedding")
    );
    assert!(
        cosine_similarity(&[1.0, f32::NAN], &reference)
            .unwrap_err()
            .contains("non-finite components")
    );
}

#[requires(true)]
#[ensures(ret.len() == values.len() * 4)]
fn f32le_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == bytes.len() * 2)]
fn lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn native_evidence_path() -> PathBuf {
    env::var_os(EVIDENCE_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let target = env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("target"));
            target.join("f2llm-native-goldens.json")
        })
}

#[requires(value.is_object())]
#[ensures(path.is_file())]
fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("creating `{}`: {error}", parent.display()));
    }
    let bytes = serde_json::to_vec_pretty(value).expect("serialize native golden evidence");
    fs::write(path, bytes).unwrap_or_else(|error| panic!("writing `{}`: {error}", path.display()));
}
