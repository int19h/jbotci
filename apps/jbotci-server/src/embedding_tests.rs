//! Test the server cache and query wrappers with real corpus proofs and synthetic inference.

use super::*;
use jbotci_embeddings::{
    CUKTA_CORPUS_ID, EmbeddingBackend, EmbeddingCatalog, EmbeddingCatalogModel, EmbeddingModelSpec,
    EmbeddingPackManifest, INDEX_BASE_VERSION, INDEX_SCHEMA_VERSION, QueryEmbedding,
    VLACKU_CORPUS_ID, build_embedding_pack, deterministic_pack_id, read_latest_pack_manifest,
    sha256_hex_bytes, validate_pack_dir,
};
use std::fs;

#[invariant(true)]
#[derive(Debug)]
struct UnitBackend {
    dimensions: NonZeroUsize,
    calls: usize,
}

#[contract_trait]
impl EmbeddingBackend for UnitBackend {
    fn dimensions(&self) -> std::result::Result<usize, EmbeddingError> {
        Ok(self.dimensions.get())
    }

    fn embed(&mut self, _input: &str) -> std::result::Result<QueryEmbedding, EmbeddingError> {
        self.calls += 1;
        let mut values = vec![0.0; self.dimensions.get()];
        values[0] = 1.0;
        Ok(QueryEmbedding { values })
    }
}

#[invariant(true)]
#[derive(Debug)]
struct FixtureService {
    model_key: String,
    index_root: PathBuf,
    backend: UnitBackend,
}

#[contract_trait]
impl ServerEmbeddingSearch for FixtureService {
    fn load(model_key: &str, paths: &EmbeddingSearchPaths) -> Result<Self> {
        let spec = model_spec(model_key).expect("fixture model");
        Ok(Self {
            model_key: model_key.to_owned(),
            index_root: paths.index_dir.clone().expect("fixture index"),
            backend: UnitBackend {
                dimensions: NonZeroUsize::new(spec.dimensions).unwrap(),
                calls: 0,
            },
        })
    }

    fn model_key(&self) -> &str {
        &self.model_key
    }

    fn semantic_vlacku_hits_window_filtered<F: FnMut(usize) -> bool>(
        &mut self,
        query: &str,
        skip: usize,
        count: usize,
        entry_allowed: F,
    ) -> std::result::Result<Vec<DictionarySemanticHit>, EmbeddingError> {
        jbotci_embeddings::semantic_vlacku_hits_window_filtered(
            &mut self.backend,
            query,
            skip,
            count,
            &self.index_root,
            &self.model_key,
            entry_allowed,
        )
    }

    fn semantic_cukta_output(
        &mut self,
        chunks: &[jbotci_cll::CllSearchChunk],
        query: &str,
        window: CuktaSearchWindow,
        targets: CuktaTargetFilter,
    ) -> std::result::Result<CuktaSearchOutput, EmbeddingError> {
        jbotci_embeddings::semantic_cukta_output(
            &mut self.backend,
            chunks,
            query,
            window,
            targets,
            &self.index_root,
            &self.model_key,
        )
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_dir() -> tempfile::TempDir {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    tempfile::tempdir_in(root).unwrap()
}

#[requires(source.is_dir())]
#[ensures(true)]
fn copy_raw_pack(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_raw_pack(&entry.path(), &destination.join(entry.file_name()));
        } else if entry
            .path()
            .extension()
            .is_none_or(|extension| extension != "br")
        {
            fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
}

#[requires([VLACKU_CORPUS_ID, CUKTA_CORPUS_ID].contains(&stale))]
#[ensures(true)]
fn stale_fixture(
    index: &Path,
    original_root: &Path,
    original: &EmbeddingPackManifest,
    spec: &EmbeddingModelSpec,
    stale: &str,
) -> PathBuf {
    let mut manifest = original.clone();
    manifest
        .corpora
        .iter_mut()
        .find(|corpus| corpus.corpus_id == stale)
        .unwrap()
        .fingerprint = sha256_hex_bytes(b"previous corpus inputs");
    let fingerprint = |id| {
        &manifest
            .corpora
            .iter()
            .find(|corpus| corpus.corpus_id == id)
            .unwrap()
            .fingerprint
    };
    manifest.pack_id = deterministic_pack_id(
        &spec.input_format_version,
        &spec.model_revision,
        &spec.native_sha256,
        fingerprint(VLACKU_CORPUS_ID),
        fingerprint(CUKTA_CORPUS_ID),
    );
    let relative = format!(
        "models/{}/packs/{}/manifest.json",
        spec.model_key, manifest.pack_id
    );
    let manifest_path = index.join(INDEX_BASE_VERSION).join(&relative);
    let pack_root = manifest_path.parent().unwrap();
    copy_raw_pack(original_root, pack_root);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let catalog = EmbeddingCatalog {
        schema_version: INDEX_SCHEMA_VERSION,
        models: vec![EmbeddingCatalogModel {
            model_key: spec.model_key.clone(),
            latest_pack_id: manifest.pack_id,
            manifest_url: relative,
        }],
    };
    fs::write(
        index.join(INDEX_BASE_VERSION).join("catalog.json"),
        serde_json::to_vec(&catalog).unwrap(),
    )
    .unwrap();
    validate_pack_dir(pack_root).unwrap();
    pack_root.to_owned()
}

#[requires([VLACKU_CORPUS_ID, CUKTA_CORPUS_ID].contains(&id))]
#[ensures(true)]
fn query(
    cache: &mut EmbeddingSearchCache<FixtureService>,
    id: &str,
) -> std::result::Result<usize, SemanticSearchError> {
    let query = SearchQuery::new("query").unwrap();
    if id == VLACKU_CORPUS_ID {
        run_semantic_vlacku_hits(
            cache,
            &query,
            NonZeroUsize::new(1).unwrap(),
            &VlackuSearchOptions::default(),
        )
        .map(|hits| hits.len())
    } else {
        run_semantic_cukta_search(
            cache,
            &query,
            CuktaSearchWindow::first(1),
            CuktaTargetFilter::default(),
        )
        .map(|output| output.matches.len())
    }
}

#[requires(true)]
#[ensures(true)]
fn query_calls(cache: &EmbeddingSearchCache<FixtureService>) -> usize {
    match cache {
        EmbeddingSearchCache::Loaded { service } => service.backend.calls,
        _ => panic!("a stale corpus cannot make the model service unavailable"),
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn server_search_wrappers_keep_each_current_corpus_available() {
    let spec = model_spec(SERVER_DEFAULT_EMBEDDING_MODEL_KEY).unwrap();
    let source = fixture_dir();
    let mut backend = UnitBackend {
        dimensions: NonZeroUsize::new(spec.dimensions).unwrap(),
        calls: 0,
    };
    build_embedding_pack(
        &mut backend,
        jbotci_dictionary_data::english(),
        cll_search_all_chunks(embedded_cll_site().unwrap()),
        source.path(),
        &spec,
        false,
    )
    .unwrap();
    let (original_root, original) =
        read_latest_pack_manifest(source.path(), &spec.model_key).unwrap();
    for stale in [VLACKU_CORPUS_ID, CUKTA_CORPUS_ID] {
        for damage in [None, Some("hash"), Some("missing")] {
            let index = fixture_dir();
            let root = stale_fixture(index.path(), &original_root, &original, &spec, stale);
            let current = if stale == VLACKU_CORPUS_ID {
                CUKTA_CORPUS_ID
            } else {
                VLACKU_CORPUS_ID
            };
            if let Some(damage) = damage {
                let corpus = original
                    .corpora
                    .iter()
                    .find(|corpus| corpus.corpus_id == current)
                    .unwrap();
                let path = root.join(&corpus.shards[0].url);
                if damage == "hash" {
                    let mut bytes = fs::read(&path).unwrap();
                    bytes[0] ^= 1;
                    fs::write(path, bytes).unwrap();
                } else {
                    fs::remove_file(path).unwrap();
                }
            }
            let mut cache = EmbeddingSearchCache::<FixtureService>::Unloaded {
                model_key: new!(ServerEmbeddingModelKey {
                    value: spec.model_key.clone()
                }),
                paths: EmbeddingSearchPaths {
                    model_dir: None,
                    index_dir: Some(index.path().to_owned()),
                },
            };
            let error = query(&mut cache, stale).unwrap_err();
            assert!(
                matches!(error.as_data(), bityzba::data!(SemanticSearchError::Failed { message })
            if message.contains(stale) && message.contains("fingerprint mismatch")),
                "{error}"
            );
            assert_eq!(query_calls(&cache), 0);
            if let Some(damage) = damage {
                let error = query(&mut cache, current).unwrap_err();
                let expected = if damage == "hash" {
                    "SHA-256 mismatch"
                } else {
                    "failed to read"
                };
                assert!(
                    matches!(error.as_data(), bityzba::data!(SemanticSearchError::Failed { message })
                if message.contains(expected)),
                    "{error}"
                );
                assert_eq!(query_calls(&cache), 0);
                continue;
            }
            assert_eq!(query(&mut cache, current).unwrap(), 1);
            assert_eq!(query_calls(&cache), 1);
            assert!(query(&mut cache, stale).is_err());
            assert_eq!(query_calls(&cache), 1);
            assert_eq!(query(&mut cache, current).unwrap(), 1);
            assert_eq!(query_calls(&cache), 2);
        }
    }
}
