#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ArtifactManifestDigest, ArtifactPath, Sha256Digest, WEB_VECTOR_PACK_SCHEMA_VERSION};

#[invariant(::F16Le => true)]
#[invariant(::F32Le => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorElementType {
    #[serde(rename = "f16le")]
    F16Le,
    #[serde(rename = "f32le")]
    F32Le,
}

impl VectorElementType {
    #[requires(true)]
    #[ensures(ret == 2 || ret == 4)]
    pub fn byte_width(self) -> u64 {
        match self {
            Self::F16Le => 2,
            Self::F32Le => 4,
        }
    }
}

#[invariant(::Dot => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    #[serde(rename = "dot")]
    Dot,
}

#[invariant(::MeanNormalizedWindows => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pooling {
    #[serde(rename = "mean_normalized_windows")]
    MeanNormalizedWindows,
}

#[invariant(!runtime.is_empty())]
#[invariant(!version.is_empty())]
#[invariant(!dtype.is_empty())]
#[invariant(!device.is_empty())]
#[invariant(*max_window_tokens > 1)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibleQueryRuntime {
    pub runtime: String,
    pub version: String,
    pub dtype: String,
    pub device: String,
    pub pooling: Pooling,
    pub max_window_tokens: usize,
}

#[invariant(!runtime.is_empty())]
#[invariant(!backend.is_empty())]
#[invariant(!adapter.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildExecution {
    pub runtime: String,
    pub backend: String,
    pub adapter: String,
}

/// Truthful identity and execution provenance for vectors derived from a WebGPU artifact.
///
/// The manifest digest hashes the exact published bytes. The path records where those bytes came
/// from but deliberately does not participate in identity.
#[invariant(
    ::WebGpuArtifact {
        runtime,
        artifact_version,
        ..
    } => !runtime.is_empty() && !artifact_version.is_empty()
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SourceProvenance {
    WebGpuArtifact {
        manifest_path: ArtifactPath,
        manifest_sha256: ArtifactManifestDigest,
        runtime: String,
        artifact_version: String,
        execution: BuildExecution,
    },
}

impl SourceProvenance {
    #[requires(true)]
    #[ensures(ret.as_str().len() == 64)]
    pub fn manifest_sha256(&self) -> &ArtifactManifestDigest {
        match self.as_data() {
            data!(SourceProvenance::WebGpuArtifact {
                manifest_sha256,
                ..
            }) => manifest_sha256,
        }
    }
}

#[invariant(!corpus_id.is_empty())]
#[invariant(!input_format_version.is_empty())]
#[invariant(*row_count > 0)]
#[invariant(*dimensions > 0)]
#[invariant(*vector_byte_len > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusVectorManifest {
    pub corpus_id: String,
    pub input_format_version: String,
    pub input_hash: Sha256Digest,
    pub row_count: usize,
    pub dimensions: usize,
    pub items_url: ArtifactPath,
    pub items_sha256: Sha256Digest,
    pub vector_url: ArtifactPath,
    pub vector_byte_len: u64,
    pub vector_sha256: Sha256Digest,
}

/// The published ONNX-derived F2LLM pack schema. This type intentionally retains the original
/// mandatory `q4_onnx_sha256` field and does not pretend that v1 represented native WebGPU builds.
#[invariant(*schema_version == 1)]
#[invariant(!artifact_version.is_empty())]
#[invariant(!model_key.is_empty())]
#[invariant(!web_model.is_empty())]
#[invariant(!vector_space_key.is_empty())]
#[invariant(!pack_id.is_empty())]
#[invariant(!input_format_version.is_empty())]
#[invariant(*max_sequence_length > 1)]
#[invariant(*max_window_tokens == *max_sequence_length)]
#[invariant(*dimensions > 0)]
#[invariant(*normalized)]
#[invariant(!compatible_query_runtimes.is_empty())]
#[invariant(!corpora.is_empty())]
#[invariant(
    corpora.iter().all(|corpus| {
        corpus.input_format_version == *input_format_version
            && corpus.dimensions == *dimensions
            && u64::try_from(corpus.row_count)
                .ok()
                .and_then(|rows| rows.checked_mul(u64::try_from(corpus.dimensions).ok()?))
                .and_then(|elements| elements.checked_mul(element_type.byte_width()))
                == Some(corpus.vector_byte_len)
    })
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebVectorPackManifestV1 {
    pub schema_version: u32,
    pub artifact_version: String,
    pub model_key: String,
    pub model_revision: String,
    pub web_model: String,
    pub q4_onnx_sha256: Sha256Digest,
    pub vector_space_key: String,
    pub pack_id: String,
    pub input_format_version: String,
    pub input_hash: Sha256Digest,
    pub max_sequence_length: usize,
    pub pooling: Pooling,
    pub max_window_tokens: usize,
    pub built_by: LegacyOnnxBuild,
    pub dimensions: usize,
    pub element_type: VectorElementType,
    pub normalized: bool,
    pub distance: DistanceMetric,
    pub compatible_query_runtimes: Vec<CompatibleQueryRuntime>,
    pub corpora: Vec<CorpusVectorManifest>,
}

#[invariant(!runtime.is_empty())]
#[invariant(!provider.is_empty())]
#[invariant(!dtype.is_empty())]
#[invariant(!source.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyOnnxBuild {
    pub runtime: String,
    pub provider: String,
    pub dtype: String,
    pub source: String,
}

/// Pack manifest emitted by the future native builder.
#[invariant(*schema_version == WEB_VECTOR_PACK_SCHEMA_VERSION)]
#[invariant(!artifact_version.is_empty())]
#[invariant(!model_key.is_empty())]
#[invariant(!web_model.is_empty())]
#[invariant(!vector_space_key.is_empty())]
#[invariant(!input_format_version.is_empty())]
#[invariant(*max_sequence_length > 1)]
#[invariant(*max_window_tokens == *max_sequence_length)]
#[invariant(*dimensions > 0)]
#[invariant(*normalized)]
#[invariant(!compatible_query_runtimes.is_empty())]
#[invariant(!corpora.is_empty())]
#[invariant(
    derive_v2_pack_id(
        model_key,
        source_provenance.manifest_sha256(),
        input_hash,
        vector_space_key,
    ) == *pack_id
)]
#[invariant(
    corpora.iter().enumerate().all(|(index, corpus)| {
        corpus.input_format_version == *input_format_version
            && corpus.dimensions == *dimensions
            && u64::try_from(corpus.row_count)
                .ok()
                .and_then(|rows| rows.checked_mul(u64::try_from(corpus.dimensions).ok()?))
                .and_then(|elements| elements.checked_mul(element_type.byte_width()))
                == Some(corpus.vector_byte_len)
            && corpora[..index]
                .iter()
                .all(|prior| prior.corpus_id != corpus.corpus_id)
    })
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebVectorPackManifestV2 {
    pub schema_version: u32,
    pub artifact_version: String,
    pub model_key: String,
    pub model_revision: String,
    pub web_model: String,
    pub source_provenance: SourceProvenance,
    pub vector_space_key: String,
    pub pack_id: String,
    pub input_format_version: String,
    pub input_hash: Sha256Digest,
    pub max_sequence_length: usize,
    pub pooling: Pooling,
    pub max_window_tokens: usize,
    pub dimensions: usize,
    pub element_type: VectorElementType,
    pub normalized: bool,
    pub distance: DistanceMetric,
    pub compatible_query_runtimes: Vec<CompatibleQueryRuntime>,
    pub corpora: Vec<CorpusVectorManifest>,
}

/// Reproduces the Python `short_hash`: hash the UTF-8 hexadecimal digest text again and take
/// twelve lowercase hexadecimal characters.
#[requires(digest.as_str().len() == 64)]
#[ensures(
    ret.len() == 12
        && ret
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
)]
pub fn short_hash(digest: &Sha256Digest) -> String {
    format!("{:x}", Sha256::digest(digest.as_str().as_bytes()))[..12].to_owned()
}

/// Derives the D2 artifact-identity segment from exact published manifest bytes.
#[requires(manifest_digest.as_str().len() == 64)]
#[ensures(
    ret.len() == 12
        && ret
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
)]
pub fn artifact_identity(manifest_digest: &ArtifactManifestDigest) -> String {
    short_hash(manifest_digest.as_sha256())
}

/// Derives the exact D2 v2 pack identity.
#[requires(!model_key.is_empty())]
#[requires(!vector_space_key.is_empty())]
#[ensures(
    ret == format!(
        "{model_key}-v2-a{}-c{}-{vector_space_key}",
        artifact_identity(artifact_manifest_sha256),
        short_hash(input_hash)
    )
)]
pub fn derive_v2_pack_id(
    model_key: &str,
    artifact_manifest_sha256: &ArtifactManifestDigest,
    input_hash: &Sha256Digest,
    vector_space_key: &str,
) -> String {
    format!(
        "{model_key}-v2-a{}-c{}-{vector_space_key}",
        artifact_identity(artifact_manifest_sha256),
        short_hash(input_hash)
    )
}

#[cfg(test)]
mod tests {
    use bityzba::new;

    use super::*;

    const PUBLISHED_V1_MANIFEST: &[u8] =
        include_bytes!("../testdata/manifests/v1/published-80m.json");
    const V1_MANIFEST: &str = r#"{
      "schema_version": 1,
      "artifact_version": "f2llm-vector-pack-windowed-ca-v1",
      "model_key": "f2llm-v2-80m-q4-320",
      "model_revision": "",
      "web_model": "codefuse-ai/F2LLM-v2-80M",
      "q4_onnx_sha256": "00ec8cc51400b74b0d215b794536a81a24f9002926c340ebde139092b3a36cc6",
      "vector_space_key": "jbotci-browser-f2llm-q4-f16-windowed-512-v1",
      "pack_id": "legacy-pack-id",
      "input_format_version": "f2llm-v2-330m-q4-k-m-v0",
      "input_hash": "f6bcb6c55027144dec1f9c53fc337745b431ed08d062e64782be2bb7abfe1839",
      "max_sequence_length": 512,
      "pooling": "mean_normalized_windows",
      "max_window_tokens": 512,
      "built_by": {
        "runtime": "onnxruntime",
        "provider": "CPUExecutionProvider",
        "dtype": "q4",
        "source": "com.microsoft MatMulNBits/GatherBlockQuantized"
      },
      "dimensions": 2,
      "element_type": "f16le",
      "normalized": true,
      "distance": "dot",
      "compatible_query_runtimes": [{
        "runtime": "jbotci-webgpu-f2llm",
        "version": "0.2.0",
        "dtype": "q4",
        "device": "webgpu",
        "pooling": "mean_normalized_windows",
        "max_window_tokens": 512
      }],
      "corpora": [{
        "corpus_id": "test",
        "input_format_version": "f2llm-v2-330m-q4-k-m-v0",
        "input_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "row_count": 2,
        "dimensions": 2,
        "items_url": "corpora/test/items.json",
        "items_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "vector_url": "corpora/test/vectors.f16",
        "vector_byte_len": 8,
        "vector_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
      }]
    }"#;

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn python_short_hash_regression_vectors_are_exact() {
        let manifest_digest = ArtifactManifestDigest::parse(
            "f25482d5612b2f74f5b76739eb33bdb52862866918cc7a5e4fb7dfb3aa06c6c2",
        )
        .expect("manifest digest");
        let input_hash =
            Sha256Digest::parse("f6bcb6c55027144dec1f9c53fc337745b431ed08d062e64782be2bb7abfe1839")
                .expect("input hash");
        assert_eq!(artifact_identity(&manifest_digest), "14e4bb2003b9");
        assert_eq!(short_hash(&input_hash), "496a983b304e");
        assert_eq!(
            derive_v2_pack_id(
                "f2llm-v2-80m-q4-320",
                &manifest_digest,
                &input_hash,
                "jbotci-browser-f2llm-q4-f16-windowed-512-v1",
            ),
            "f2llm-v2-80m-q4-320-v2-a14e4bb2003b9-c496a983b304e-jbotci-browser-f2llm-q4-f16-windowed-512-v1"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn v1_schema_keeps_truthful_onnx_provenance() {
        let manifest: WebVectorPackManifestV1 =
            serde_json::from_slice(PUBLISHED_V1_MANIFEST).expect("published v1 manifest");
        assert_eq!(
            Sha256Digest::of_bytes(PUBLISHED_V1_MANIFEST).as_str(),
            "1bd8048537ea8d3f813103550ba9217b91508f95674cf78802b0b1c31efe734e"
        );
        assert_eq!(
            manifest.q4_onnx_sha256.as_str(),
            "00ec8cc51400b74b0d215b794536a81a24f9002926c340ebde139092b3a36cc6"
        );
        assert_eq!(manifest.compatible_query_runtimes.len(), 2);
        assert_eq!(manifest.corpora.len(), 2);
        let value = serde_json::to_value(&manifest).expect("serialize v1");
        assert!(value.get("q4_onnx_sha256").is_some());
        assert!(value.get("source_provenance").is_none());
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn v2_schema_is_tagged_and_never_fakes_an_onnx_digest() {
        let manifest_digest = ArtifactManifestDigest::parse(
            "f25482d5612b2f74f5b76739eb33bdb52862866918cc7a5e4fb7dfb3aa06c6c2",
        )
        .expect("manifest digest");
        let input_hash =
            Sha256Digest::parse("f6bcb6c55027144dec1f9c53fc337745b431ed08d062e64782be2bb7abfe1839")
                .expect("input hash");
        let vector_space_key = "jbotci-browser-f2llm-q4-f16-windowed-512-v1";
        let pack_id = derive_v2_pack_id(
            "f2llm-v2-80m-q4-320",
            &manifest_digest,
            &input_hash,
            vector_space_key,
        );
        let manifest = new!(WebVectorPackManifestV2 {
            schema_version: WEB_VECTOR_PACK_SCHEMA_VERSION,
            artifact_version: "f2llm-vector-pack-windowed-ca-v2".to_owned(),
            model_key: "f2llm-v2-80m-q4-320".to_owned(),
            model_revision: "f4a16a11c9f5c8c7e22694653de6ce75430f4538".to_owned(),
            web_model: "codefuse-ai/F2LLM-v2-80M".to_owned(),
            source_provenance: new!(SourceProvenance::WebGpuArtifact {
                manifest_path: ArtifactPath::parse("models/f2llm-v2-80m-webgpu/v1/manifest.json")
                    .expect("artifact path"),
                manifest_sha256: manifest_digest,
                runtime: "jbotci-webgpu-f2llm".to_owned(),
                artifact_version: "0.2.0".to_owned(),
                execution: new!(BuildExecution {
                    runtime: "wgpu".to_owned(),
                    backend: "vulkan".to_owned(),
                    adapter: "llvmpipe".to_owned(),
                }),
            }),
            vector_space_key: vector_space_key.to_owned(),
            pack_id: pack_id,
            input_format_version: "f2llm-v2-330m-q4-k-m-v0".to_owned(),
            input_hash: input_hash,
            max_sequence_length: 512,
            pooling: Pooling::MeanNormalizedWindows,
            max_window_tokens: 512,
            dimensions: 2,
            element_type: VectorElementType::F16Le,
            normalized: true,
            distance: DistanceMetric::Dot,
            compatible_query_runtimes: vec![new!(CompatibleQueryRuntime {
                runtime: "jbotci-webgpu-f2llm".to_owned(),
                version: "0.2.0".to_owned(),
                dtype: "q4".to_owned(),
                device: "webgpu".to_owned(),
                pooling: Pooling::MeanNormalizedWindows,
                max_window_tokens: 512,
            })],
            corpora: vec![new!(CorpusVectorManifest {
                corpus_id: "test".to_owned(),
                input_format_version: "f2llm-v2-330m-q4-k-m-v0".to_owned(),
                input_hash: Sha256Digest::parse(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                )
                .expect("corpus hash"),
                row_count: 2,
                dimensions: 2,
                items_url: ArtifactPath::parse("corpora/test/items.json").expect("items path"),
                items_sha256: Sha256Digest::parse(
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                )
                .expect("items hash"),
                vector_url: ArtifactPath::parse("corpora/test/vectors.f16").expect("vector path"),
                vector_byte_len: 8,
                vector_sha256: Sha256Digest::parse(
                    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                )
                .expect("vector hash"),
            })],
        });

        let value = serde_json::to_value(&manifest).expect("serialize v2");
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["source_provenance"]["kind"], "web-gpu-artifact");
        assert!(value.get("q4_onnx_sha256").is_none());
        let round_trip: WebVectorPackManifestV2 =
            serde_json::from_value(value.clone()).expect("deserialize v2");
        assert_eq!(round_trip, manifest);

        let mut wrong_pack_id = value.clone();
        wrong_pack_id["pack_id"] = serde_json::json!("wrong-pack-id");
        assert!(serde_json::from_value::<WebVectorPackManifestV2>(wrong_pack_id).is_err());

        let mut mismatched_source_identity = value.clone();
        mismatched_source_identity["source_provenance"]["manifest_sha256"] =
            serde_json::json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        assert!(
            serde_json::from_value::<WebVectorPackManifestV2>(mismatched_source_identity).is_err()
        );

        let mut fake_onnx = value;
        fake_onnx["q4_onnx_sha256"] =
            serde_json::json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        assert!(serde_json::from_value::<WebVectorPackManifestV2>(fake_onnx).is_err());
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn manifest_serde_rejects_vector_shape_and_version_lies() {
        let mut value: serde_json::Value = serde_json::from_str(V1_MANIFEST).expect("fixture JSON");
        value["schema_version"] = serde_json::json!(2);
        assert!(serde_json::from_value::<WebVectorPackManifestV1>(value.clone()).is_err());
        value["schema_version"] = serde_json::json!(1);
        value["corpora"][0]["vector_byte_len"] = serde_json::json!(7);
        assert!(serde_json::from_value::<WebVectorPackManifestV1>(value).is_err());
    }
}
