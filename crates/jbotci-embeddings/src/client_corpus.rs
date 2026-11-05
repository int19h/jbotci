//! Search receives an opaque capability only after model identity, corpus identity, and byte proofs pass.

use super::*;

#[invariant(::Absent => true)]
// A missing compatible pack is Absent, rather than a stale installed corpus.
#[invariant(::Stale { error } => !matches!(error, EmbeddingError::MissingCompatiblePack { .. }))]
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
        match self.as_data() {
            data!(CorpusState::Absent) => CorpusAvailability::Absent,
            data!(CorpusState::Stale { .. }) => CorpusAvailability::Stale,
            data!(CorpusState::VerifiedCurrent { .. }) => CorpusAvailability::VerifiedCurrent,
        }
    }

    #[requires(!model_key.is_empty())]
    #[ensures(true)]
    pub(super) fn require_current(
        self,
        model_key: &str,
    ) -> Result<VerifiedCorpus<T>, EmbeddingError> {
        match self.into_data() {
            data!(CorpusState::VerifiedCurrent { corpus }) => Ok(corpus),
            data!(CorpusState::Absent) => Err(EmbeddingError::MissingCompatiblePack {
                model_key: model_key.to_owned(),
            }),
            data!(CorpusState::Stale { error }) => Err(error),
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
        Ok(corpus) => new!(CorpusState::VerifiedCurrent { corpus }),
        Err(EmbeddingError::MissingCompatiblePack { .. }) => new!(CorpusState::Absent),
        Err(error) => new!(CorpusState::Stale { error }),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn missing_pack_cannot_be_constructed_as_a_stale_corpus() {
        let missing = bityzba::try_new!(CorpusState::<DictionaryEmbeddingItem>::Stale {
            error: EmbeddingError::MissingCompatiblePack {
                model_key: DEFAULT_MODEL_KEY.to_owned()
            },
        });
        assert!(missing.is_err());
        let stale = bityzba::try_new!(CorpusState::<DictionaryEmbeddingItem>::Stale {
            error: EmbeddingError::InvalidIndex {
                message: "corpus fingerprint mismatch".to_owned()
            },
        })
        .unwrap();
        assert_eq!(stale.availability(), CorpusAvailability::Stale);
        assert!(matches!(
            stale.require_current(DEFAULT_MODEL_KEY),
            Err(EmbeddingError::InvalidIndex { .. })
        ));
        let absent = new!(CorpusState::<DictionaryEmbeddingItem>::Absent);
        assert_eq!(absent.availability(), CorpusAvailability::Absent);
        assert!(matches!(
            absent.require_current(DEFAULT_MODEL_KEY),
            Err(EmbeddingError::MissingCompatiblePack { .. })
        ));
    }
}
