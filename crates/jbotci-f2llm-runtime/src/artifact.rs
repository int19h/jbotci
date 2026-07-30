use std::fmt;
use std::future::Future;
use std::pin::Pin;

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A boxed, non-`Send` future suitable for both browser and owning-thread native execution.
pub type RuntimeFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A normalized UTF-8 path relative to a separately typed artifact root.
///
/// Reserved URL characters and percent escapes are rejected so the same value has one
/// interpretation under directory and HTTP roots. Root types and their resolution adapters are
/// intentionally deferred to native bring-up; accepting a raw root string here would erase the
/// security distinction the adapters must enforce.
#[invariant(
    !self.is_empty()
        && !self.starts_with('/')
        && !self.ends_with('/')
        && !self.contains('\\')
        && !self.contains(':')
        && !self.contains('?')
        && !self.contains('#')
        && !self.contains('%')
        && self.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.chars().any(char::is_control)
        })
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactPath(String);

impl ArtifactPath {
    /// Validates a relative artifact path without performing filesystem or URL resolution.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|path| !path.as_str().is_empty()) || ret.is_err())]
    pub fn parse(value: impl Into<String>) -> Result<Self, ArtifactPathError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ArtifactPathError::Empty);
        }
        if value.starts_with('/') {
            return Err(ArtifactPathError::Absolute);
        }
        if value.ends_with('/') {
            return Err(ArtifactPathError::TrailingSeparator);
        }
        if value.contains('\\') {
            return Err(ArtifactPathError::Backslash);
        }
        if value.contains(':') {
            return Err(ArtifactPathError::SchemeOrDrive);
        }
        if value.contains('?') || value.contains('#') || value.contains('%') {
            return Err(ArtifactPathError::UrlMetacharacter);
        }
        for component in value.split('/') {
            if component.is_empty() {
                return Err(ArtifactPathError::EmptyComponent);
            }
            if component == "." {
                return Err(ArtifactPathError::CurrentDirectory);
            }
            if component == ".." {
                return Err(ArtifactPathError::ParentDirectory);
            }
            if component.chars().any(char::is_control) {
                return Err(ArtifactPathError::ControlCharacter);
            }
        }
        Ok(new!(ArtifactPath(value)))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactPath {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for ArtifactPath {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<String> for ArtifactPath {
    type Error = ArtifactPathError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|path| !path.as_str().is_empty()) || ret.is_err())]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ArtifactPath {
    type Error = ArtifactPathError;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|path| !path.as_str().is_empty()) || ret.is_err())]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[invariant(::Empty => true)]
#[invariant(::Absolute => true)]
#[invariant(::TrailingSeparator => true)]
#[invariant(::Backslash => true)]
#[invariant(::SchemeOrDrive => true)]
#[invariant(::UrlMetacharacter => true)]
#[invariant(::EmptyComponent => true)]
#[invariant(::CurrentDirectory => true)]
#[invariant(::ParentDirectory => true)]
#[invariant(::ControlCharacter => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ArtifactPathError {
    #[error("artifact path must not be empty")]
    Empty,
    #[error("artifact path must be relative")]
    Absolute,
    #[error("artifact path must not end with a separator")]
    TrailingSeparator,
    #[error("artifact path must use forward slashes")]
    Backslash,
    #[error("artifact path must not contain a URL scheme or drive prefix")]
    SchemeOrDrive,
    #[error("artifact path must not contain URL query, fragment, or percent metacharacters")]
    UrlMetacharacter,
    #[error("artifact path must not contain empty components")]
    EmptyComponent,
    #[error("artifact path must not contain `.` components")]
    CurrentDirectory,
    #[error("artifact path must not contain `..` components")]
    ParentDirectory,
    #[error("artifact path must not contain control characters")]
    ControlCharacter,
}

/// A canonical lowercase SHA-256 hexadecimal digest.
#[invariant(
    self.len() == 64
        && self
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|digest| digest.as_str().len() == 64) || ret.is_err())]
    pub fn parse(value: impl Into<String>) -> Result<Self, Sha256DigestError> {
        let value = value.into();
        if value.len() != 64 {
            return Err(new!(Sha256DigestError::Length {
                actual: value.len(),
            }));
        }
        if !value
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
        {
            return Err(new!(Sha256DigestError::Character));
        }
        Ok(new!(Sha256Digest(value)))
    }

    #[requires(true)]
    #[ensures(ret.len() == 64)]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        new!(Sha256Digest(format!("{:x}", Sha256::digest(bytes))))
    }

    #[requires(true)]
    #[ensures(ret.len() == 64)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The SHA-256 digest of exact, immutable, published artifact-manifest bytes.
///
/// This domain type prevents an ONNX, tensor, or corpus digest from being passed to the v2
/// artifact-identity derivation accidentally.
#[invariant(self.as_str().len() == 64)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactManifestDigest(Sha256Digest);

impl ArtifactManifestDigest {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|digest| digest.as_str().len() == 64) || ret.is_err())]
    pub fn parse(value: impl Into<String>) -> Result<Self, Sha256DigestError> {
        let digest = Sha256Digest::parse(value)?;
        Ok(new!(ArtifactManifestDigest(digest)))
    }

    #[requires(true)]
    #[ensures(ret.as_str().len() == 64)]
    pub fn of_published_bytes(bytes: &[u8]) -> Self {
        new!(ArtifactManifestDigest(Sha256Digest::of_bytes(bytes)))
    }

    #[requires(true)]
    #[ensures(ret.as_str().len() == 64)]
    pub fn as_sha256(&self) -> &Sha256Digest {
        &self.0
    }

    #[requires(true)]
    #[ensures(ret.len() == 64)]
    pub fn as_str(&self) -> &str {
        self.as_sha256().as_str()
    }
}

impl fmt::Display for ArtifactManifestDigest {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[invariant(::Length { actual } => *actual != 64)]
#[invariant(::Character => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sha256DigestError {
    Length { actual: usize },
    Character,
}

impl fmt::Display for Sha256DigestError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(Sha256DigestError::Length { actual }) => write!(
                formatter,
                "SHA-256 digest must contain 64 hexadecimal characters, got {actual}"
            ),
            data!(Sha256DigestError::Character) => {
                formatter.write_str("SHA-256 digest must use lowercase hexadecimal characters")
            }
        }
    }
}

impl std::error::Error for Sha256DigestError {}

/// Failures while fetching immutable model-artifact bytes.
#[invariant(::Unavailable { message, .. } => !message.is_empty())]
#[invariant(::Integrity { expected, actual, .. } => expected != actual)]
#[invariant(::InvalidContent { message, .. } => !message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactError {
    Unavailable {
        path: ArtifactPath,
        message: String,
    },
    Integrity {
        path: ArtifactPath,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    InvalidContent {
        path: ArtifactPath,
        message: String,
    },
}

impl fmt::Display for ArtifactError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(ArtifactError::Unavailable { path, message }) => {
                write!(formatter, "artifact `{path}` is unavailable: {message}")
            }
            data!(ArtifactError::Integrity {
                path,
                expected,
                actual,
            }) => write!(
                formatter,
                "artifact `{path}` SHA-256 mismatch: expected {expected}, got {actual}"
            ),
            data!(ArtifactError::InvalidContent { path, message }) => {
                write!(
                    formatter,
                    "artifact `{path}` has invalid content: {message}"
                )
            }
        }
    }
}

impl std::error::Error for ArtifactError {}

/// An opaque browser vector-store key, distinct from an artifact path or URL.
#[invariant(!self.is_empty() && !self.chars().any(char::is_control))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VectorStoreKey(String);

impl VectorStoreKey {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|key| !key.as_str().is_empty()) || ret.is_err())]
    pub fn parse(value: impl Into<String>) -> Result<Self, VectorStoreKeyError> {
        let value = value.into();
        if value.is_empty() {
            return Err(VectorStoreKeyError::Empty);
        }
        if value.chars().any(char::is_control) {
            return Err(VectorStoreKeyError::ControlCharacter);
        }
        Ok(new!(VectorStoreKey(value)))
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VectorStoreKey {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[invariant(::Empty => true)]
#[invariant(::ControlCharacter => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum VectorStoreKeyError {
    #[error("vector-store key must not be empty")]
    Empty,
    #[error("vector-store key must not contain control characters")]
    ControlCharacter,
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorStoreError {
    pub key: VectorStoreKey,
    pub message: String,
}

impl fmt::Display for VectorStoreError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "vector-store read failed for `{}`: {}",
            self.key, self.message
        )
    }
}

impl std::error::Error for VectorStoreError {}

#[contract_trait]
pub trait ArtifactSource {
    #[requires(true)]
    #[ensures(true)]
    fn fetch<'a>(
        &'a self,
        path: &'a ArtifactPath,
    ) -> RuntimeFuture<'a, Result<Vec<u8>, ArtifactError>>;
}

#[contract_trait]
pub trait VectorStore {
    #[requires(true)]
    #[ensures(true)]
    fn read<'a>(
        &'a self,
        key: &'a VectorStoreKey,
    ) -> RuntimeFuture<'a, Result<Vec<u8>, VectorStoreError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn artifact_paths_accept_only_normalized_relative_paths() {
        let path = ArtifactPath::parse("tensors/model.layers.0/qweight-0001.bin")
            .expect("normalized path");
        assert_eq!(path.as_str(), "tensors/model.layers.0/qweight-0001.bin");

        for invalid in [
            "",
            "/absolute",
            "trailing/",
            "two//components",
            "./current",
            "parent/../escape",
            r"windows\path",
            "https://example.invalid/chunk",
            "C:/drive/path",
            "query?value",
            "fragment#value",
            "encoded/%2e%2e/path",
            "control/\n/path",
        ] {
            assert!(
                ArtifactPath::parse(invalid).is_err(),
                "{invalid:?} must be rejected"
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn artifact_path_deserialization_enforces_the_invariant() {
        let valid: ArtifactPath =
            serde_json::from_str(r#""tokenizer.compact.json""#).expect("valid path");
        assert_eq!(valid.as_str(), "tokenizer.compact.json");
        assert!(serde_json::from_str::<ArtifactPath>(r#""../manifest.json""#).is_err());
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn sha256_digest_is_lowercase_and_round_trips_through_serde() {
        let digest = Sha256Digest::of_bytes(b"published manifest bytes");
        assert_eq!(digest.as_str().len(), 64);
        let json = serde_json::to_string(&digest).expect("serialize digest");
        let decoded: Sha256Digest = serde_json::from_str(&json).expect("deserialize digest");
        assert_eq!(decoded, digest);
        assert!(Sha256Digest::parse(digest.as_str().to_uppercase()).is_err());

        let manifest_digest =
            ArtifactManifestDigest::of_published_bytes(b"published manifest bytes");
        assert_eq!(manifest_digest.as_sha256(), &digest);
        let json = serde_json::to_string(&manifest_digest).expect("serialize manifest digest");
        let decoded: ArtifactManifestDigest =
            serde_json::from_str(&json).expect("deserialize manifest digest");
        assert_eq!(decoded, manifest_digest);
    }

    #[derive(Debug, Default)]
    #[invariant(true)]
    struct ObjectSafeSources;

    #[contract_trait]
    impl ArtifactSource for ObjectSafeSources {
        fn fetch<'a>(
            &'a self,
            _path: &'a ArtifactPath,
        ) -> RuntimeFuture<'a, Result<Vec<u8>, ArtifactError>> {
            Box::pin(async { Ok(Vec::new()) })
        }
    }

    #[contract_trait]
    impl VectorStore for ObjectSafeSources {
        fn read<'a>(
            &'a self,
            _key: &'a VectorStoreKey,
        ) -> RuntimeFuture<'a, Result<Vec<u8>, VectorStoreError>> {
            Box::pin(async { Ok(Vec::new()) })
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn artifact_and_vector_sources_are_separate_object_safe_interfaces() {
        let sources = ObjectSafeSources;
        let artifact_source: &dyn ArtifactSource = &sources;
        let vector_store: &dyn VectorStore = &sources;
        let path = ArtifactPath::parse("manifest.json").expect("artifact path");
        let key = VectorStoreKey::parse("remote/model/pack/vectors").expect("store key");
        drop(artifact_source.fetch(&path));
        drop(vector_store.read(&key));
    }
}
