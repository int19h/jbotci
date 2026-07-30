//! Native filesystem adapters for immutable F2LLM artifacts.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{ArtifactError, ArtifactPath, ArtifactSource, RuntimeFuture};
#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};

/// Shared dev-box fallback used when a native golden invocation does not supply an artifact root.
pub const DEFAULT_NATIVE_ARTIFACT_ROOT: &str = "/build/jbotci/scratch/f2llm-native-artifacts";

/// A canonical absolute directory root, kept distinct from artifact-relative paths and HTTP roots.
#[invariant(path.is_absolute())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryRoot {
    path: PathBuf,
}

impl DirectoryRoot {
    /// Resolves and validates an existing artifact directory.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|root| root.as_path().is_absolute()) || ret.is_err())]
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DirectoryRootError> {
        let requested = path.as_ref();
        let canonical = requested.canonicalize().map_err(|error| {
            DirectoryRootError::unavailable(requested.to_path_buf(), error.to_string())
        })?;
        if !canonical.is_dir() {
            return Err(DirectoryRootError::not_directory(canonical));
        }
        Ok(new!(DirectoryRoot { path: canonical }))
    }

    #[requires(true)]
    #[ensures(ret.is_absolute())]
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    #[requires(true)]
    #[ensures(ret.is_absolute())]
    fn resolve(&self, path: &ArtifactPath) -> PathBuf {
        self.path.join(path.as_str())
    }
}

/// Failures while validating a native artifact directory root.
#[invariant(::Unavailable { message, .. } => !message.is_empty())]
#[invariant(::NotDirectory { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryRootError {
    Unavailable { path: PathBuf, message: String },
    NotDirectory { path: PathBuf },
}

impl DirectoryRootError {
    #[requires(!message.is_empty())]
    #[ensures(matches!(
        ret.as_data(),
        bityzba::data!(DirectoryRootError::Unavailable { .. })
    ))]
    fn unavailable(path: PathBuf, message: String) -> Self {
        new!(DirectoryRootError::Unavailable {
            path: path,
            message: message,
        })
    }

    #[requires(true)]
    #[ensures(matches!(
        ret.as_data(),
        bityzba::data!(DirectoryRootError::NotDirectory { .. })
    ))]
    fn not_directory(path: PathBuf) -> Self {
        new!(DirectoryRootError::NotDirectory { path: path })
    }
}

impl std::fmt::Display for DirectoryRootError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            bityzba::data!(DirectoryRootError::Unavailable { path, message }) => write!(
                formatter,
                "artifact directory `{}` is unavailable: {message}",
                path.display()
            ),
            bityzba::data!(DirectoryRootError::NotDirectory { path }) => {
                write!(
                    formatter,
                    "artifact root `{}` is not a directory",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DirectoryRootError {}

/// An immutable filesystem-backed artifact source rooted at a validated directory.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct DirectoryArtifactSource {
    root: DirectoryRoot,
}

impl DirectoryArtifactSource {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(root: DirectoryRoot) -> Self {
        Self { root }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn root(&self) -> &DirectoryRoot {
        &self.root
    }
}

#[contract_trait]
impl ArtifactSource for DirectoryArtifactSource {
    fn fetch<'a>(
        &'a self,
        path: &'a ArtifactPath,
    ) -> RuntimeFuture<'a, Result<Vec<u8>, ArtifactError>> {
        Box::pin(async move {
            let resolved = self.root.resolve(path);
            fs::read(&resolved).map_err(|error| {
                ArtifactError::unavailable(
                    path.clone(),
                    format!("{} ({})", error, resolved.display()),
                )
            })
        })
    }
}
