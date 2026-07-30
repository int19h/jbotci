//! Cross-platform contracts and schemas for F2LLM artifact loading and vector packs.
//!
//! Runtime execution remains in `jbotci-ui` during N0. This crate deliberately contains
//! only the target-neutral types and interfaces that later extraction work will consume.

mod artifact;
#[cfg(test)]
mod oracles;
mod pack;
mod progress;

pub use artifact::{
    ArtifactError, ArtifactManifestDigest, ArtifactPath, ArtifactPathError, ArtifactSource,
    RuntimeFuture, Sha256Digest, Sha256DigestError, VectorStore, VectorStoreError, VectorStoreKey,
    VectorStoreKeyError,
};
pub use pack::{
    BuildExecution, CompatibleQueryRuntime, CorpusVectorManifest, DistanceMetric, LegacyOnnxBuild,
    Pooling, SourceProvenance, VectorElementType, WebVectorPackManifestV1, WebVectorPackManifestV2,
    artifact_identity, derive_v2_pack_id, short_hash,
};
pub use progress::{
    ProgressCounter, ProgressError, ProgressEvent, ProgressKind, ProgressPhase, ProgressSink,
};

/// Schema version emitted by the future native web-vector-pack builder.
pub const WEB_VECTOR_PACK_SCHEMA_VERSION: u32 = 2;
