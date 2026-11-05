//! Search receives an opaque capability only after model identity, corpus identity, and byte proofs pass.

use super::*;

#[invariant(::Absent => true)]
#[invariant(::Stale => true)]
#[invariant(::VerifiedCurrent => true)]
pub(super) enum CorpusState<T> {
    Absent,
    Stale { error: EmbeddingError },
    VerifiedCurrent { corpus: VerifiedCorpus<T> },
}

#[invariant(true)]
pub(super) struct VerifiedCorpus<T> {
    loaded: Arc<LoadedCorpus<T>>,
}

impl<T> std::ops::Deref for VerifiedCorpus<T> {
    type Target = LoadedCorpus<T>;

    #[requires(true)]
    #[ensures(true)]
    fn deref(&self) -> &Self::Target {
        &self.loaded
    }
}

impl<T> CorpusState<T> {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn availability(&self) -> CorpusAvailability {
        match self {
            Self::Absent => CorpusAvailability::Absent,
            Self::Stale { .. } => CorpusAvailability::Stale,
            Self::VerifiedCurrent { .. } => CorpusAvailability::VerifiedCurrent,
        }
    }

    #[requires(!model_key.is_empty())]
    #[ensures(true)]
    pub(super) fn require_current(
        self,
        model_key: &str,
    ) -> Result<VerifiedCorpus<T>, EmbeddingError> {
        match self {
            Self::VerifiedCurrent { corpus } => Ok(corpus),
            Self::Absent => Err(EmbeddingError::MissingCompatiblePack {
                model_key: model_key.to_owned(),
            }),
            Self::Stale { error } => Err(error),
        }
    }
}

#[requires(spec.dimensions > 0)]
#[ensures(true)]
fn load<T: DeserializeOwned>(
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    corpus_id: &str,
    fingerprint: &str,
    rows: usize,
    cache: &Mutex<HashMap<LoadedCorpusCacheKey, Arc<LoadedCorpus<T>>>>,
) -> CorpusState<T> {
    let result = (|| {
        let (pack_dir, manifest) = load_model_pack_for_search(index_root, spec)?;
        let corpus = manifest_corpus(&manifest, corpus_id)?;
        validate_native_corpus_manifest(
            corpus,
            fingerprint,
            rows,
            spec.dimensions,
            &spec.input_format_version,
        )?;
        let loaded = load_cached_corpus(cache, &pack_dir, &manifest, corpus)?;
        Ok(VerifiedCorpus { loaded })
    })();
    match result {
        Ok(corpus) => CorpusState::VerifiedCurrent { corpus },
        Err(EmbeddingError::MissingCompatiblePack { .. }) => CorpusState::Absent,
        Err(error) => CorpusState::Stale { error },
    }
}

#[requires(spec.dimensions > 0)]
#[ensures(true)]
pub(super) fn dictionary(
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    identity: &NativeCorpusIdentity,
) -> CorpusState<DictionaryEmbeddingItem> {
    load(
        index_root,
        spec,
        VLACKU_CORPUS_ID,
        &identity.dictionary_fingerprint,
        identity.dictionary_rows,
        dictionary_corpus_cache(),
    )
}

#[requires(spec.dimensions > 0)]
#[ensures(true)]
pub(super) fn cll(
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    identity: &NativeCorpusIdentity,
) -> CorpusState<CllEmbeddingItem> {
    load(
        index_root,
        spec,
        CUKTA_CORPUS_ID,
        &identity.cll_fingerprint,
        identity.cll_rows,
        cll_corpus_cache(),
    )
}
