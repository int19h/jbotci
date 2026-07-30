//! Shared F2LLM runtime, artifact contracts, and vector-pack schemas.

mod artifact;
mod core;
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
mod native;
#[cfg(test)]
mod oracles;
mod pack;
mod progress;
#[cfg(any(target_arch = "wasm32", feature = "native"))]
mod webgpu;
#[cfg(any(target_arch = "wasm32", feature = "native", test))]
mod webgpu_manifest;

pub use artifact::{
    ArtifactError, ArtifactManifestDigest, ArtifactPath, ArtifactPathError, ArtifactSource,
    RuntimeFuture, Sha256Digest, Sha256DigestError, VectorStore, VectorStoreError, VectorStoreKey,
    VectorStoreKeyError,
};
pub use core::{
    DEFAULT_MAX_SEQUENCE_LENGTH, EmbeddingVectorError, PackedTokenBatch, QwenByteBpeTokenizer,
    TokenWindow, mean_pool_normalized, pack_token_windows,
};
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
pub use native::{
    DEFAULT_NATIVE_ARTIFACT_ROOT, DirectoryArtifactSource, DirectoryRoot, DirectoryRootError,
};
pub use pack::{
    BuildExecution, CompatibleQueryRuntime, CorpusVectorManifest, DistanceMetric, LegacyOnnxBuild,
    Pooling, SourceProvenance, VectorElementType, WebVectorPackManifestV1, WebVectorPackManifestV2,
    artifact_identity, derive_v2_pack_id, short_hash,
};
pub use progress::{
    ProgressCounter, ProgressError, ProgressEvent, ProgressKind, ProgressPhase, ProgressSink,
};
#[cfg(any(target_arch = "wasm32", feature = "native"))]
pub use webgpu::{
    CorpusShard, CorpusVectorSpec, RuntimeAdapterInfo, RuntimeCapabilities, RuntimeError,
    RuntimeLoadOptions, WebGpuRuntime,
};

/// Schema version emitted by the future native web-vector-pack builder.
pub const WEB_VECTOR_PACK_SCHEMA_VERSION: u32 = 2;

/// Stable runtime identifier stored in F2LLM WebGPU artifacts and browser model catalogs.
pub const F2LLM_WEBGPU_RUNTIME: &str = "jbotci-webgpu-f2llm";

/// Artifact/runtime version implemented by the extracted browser runtime.
pub const F2LLM_RUNTIME_VERSION: &str = "0.2.0";

/// Explicit WebGPU alias retained for model-catalog call sites.
pub const F2LLM_WEBGPU_RUNTIME_VERSION: &str = F2LLM_RUNTIME_VERSION;
