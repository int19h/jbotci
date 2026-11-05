//! Exercise installed-index updates through the same setup entry point as the CLI and desktop.

use super::*;
use std::io::{BufRead, Seek, SeekFrom};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::thread::JoinHandle;

#[requires(true)]
#[ensures(true)]
fn setup_test_dir() -> tempfile::TempDir {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    tempfile::tempdir_in(root).unwrap()
}

#[invariant(true)]
struct PackServer {
    address: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl PackServer {
    #[requires(root.is_dir())]
    #[ensures(ret.worker.is_some())]
    fn start(root: PathBuf) -> Self {
        let listener = (18800..18900)
            .find_map(|port| TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).ok())
            .expect("a test server port in 18800..18900");
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_requests = Arc::clone(&requests);
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if worker_stop.load(AtomicOrdering::Acquire) {
                    break;
                }
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let mut parts = line.split_whitespace();
                assert_eq!(parts.next(), Some("GET"));
                let relative = parts.next().unwrap().trim_start_matches('/').to_owned();
                worker_requests.lock().unwrap().push(relative.clone());
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                        break;
                    }
                }
                let path = root.join(safe_pack_relative_path(&relative).unwrap());
                if let Ok(mut file) = File::open(path) {
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        file.metadata().unwrap().len()
                    )
                    .unwrap();
                    // Broken clients can stop reading after an error. The server must still stop on drop.
                    let _ = std::io::copy(&mut file, &mut stream);
                } else {
                    stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                }
            }
        });
        Self {
            address,
            requests,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for PackServer {
    #[requires(true)]
    #[ensures(self.worker.is_none())]
    fn drop(&mut self) {
        self.stop.store(true, AtomicOrdering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

#[requires(rows > 0)]
#[requires([VLACKU_CORPUS_ID, CUKTA_CORPUS_ID].contains(&id))]
#[ensures(true)]
fn write_fixture_items(path: &Path, id: &str, rows: usize, old: bool) {
    let input_hash = sha256_hex_bytes(if old { b"old input" } else { b"current input" });
    if id == VLACKU_CORPUS_ID {
        let items = (0..rows)
            .map(|row| DictionaryEmbeddingItem {
                entry_index: row,
                word: format!("fixture-{row}"),
                definition_id: row as u64,
                input_hash: input_hash.clone(),
                kind: "gismu".to_owned(),
            })
            .collect::<Vec<_>>();
        write_json_file(path, &items).unwrap();
    } else {
        let items = (0..rows)
            .map(|row| CllEmbeddingItem {
                chunk_index: row,
                input_hash: input_hash.clone(),
            })
            .collect::<Vec<_>>();
        write_json_file(path, &items).unwrap();
    }
}

#[requires(root.is_dir())]
#[ensures(ret.model_key == spec.model_key)]
fn current_fixture(root: &Path, spec: &EmbeddingModelSpec) -> EmbeddingPackManifest {
    let identity = current_native_corpus_identity().unwrap();
    let pack_id = deterministic_pack_id(
        &spec.input_format_version,
        &spec.model_revision,
        &spec.native_sha256,
        &identity.dictionary_fingerprint,
        &identity.cll_fingerprint,
    );
    let pack_dir = pack_root(root, &spec.model_key, &pack_id);
    // Real setup selects the compiled model and corpora. Synthetic unit vectors avoid model inference.
    let mut corpora = Vec::new();
    for (id, rows, fingerprint) in [
        (
            VLACKU_CORPUS_ID,
            identity.dictionary_rows,
            &identity.dictionary_fingerprint,
        ),
        (
            CUKTA_CORPUS_ID,
            identity.cll_rows,
            &identity.cll_fingerprint,
        ),
    ] {
        let prefix = format!("corpora/{id}");
        let items_url = format!("{prefix}/items.json");
        write_fixture_items(&pack_dir.join(&items_url), id, rows, false);
        let mut values = vec![0.0; rows * spec.dimensions];
        for row in values.chunks_exact_mut(spec.dimensions) {
            row[0] = 1.0;
        }
        let shard = write_vector_chunk_file(
            &pack_dir.join(&prefix),
            &prefix,
            "vectors.f32",
            &values,
            spec.dimensions,
        )
        .unwrap();
        corpora.push(CorpusManifest {
            corpus_id: id.to_owned(),
            input_format_version: spec.input_format_version.clone(),
            fingerprint: fingerprint.clone(),
            row_count: rows,
            dimensions: spec.dimensions,
            items_sha256: sha256_hex_file(&pack_dir.join(&items_url)).unwrap(),
            items_url,
            shards: vec![shard],
        });
    }
    let manifest = EmbeddingPackManifest {
        vector_space: native_vector_space_key(),
        model_sha256: Some(spec.native_sha256.clone()),
        build_provenance: None,
        legacy_adoption: None,
        vector_parameters: Some(native_vector_parameters()),
        schema_version: INDEX_SCHEMA_VERSION,
        model_key: spec.model_key.clone(),
        model_revision: spec.model_revision.clone(),
        pack_id,
        input_format_version: spec.input_format_version.clone(),
        built_by: EmbeddingRuntime {
            runtime: "fixture".to_owned(),
            version: "test".to_owned(),
        },
        dimensions: spec.dimensions,
        element_type: "f32le".to_owned(),
        normalized: true,
        distance: "dot".to_owned(),
        compatible_query_runtimes: vec![],
        corpora,
    };
    write_json_file(&pack_dir.join("manifest.json"), &manifest).unwrap();
    write_catalog(root, spec, &manifest.pack_id).unwrap();
    manifest
}

#[test]
#[requires(true)]
#[ensures(true)]
fn public_setup_updates_one_corpus_without_force_and_repairs_damaged_candidates() {
    let remote = setup_test_dir();
    let spec = model_spec(DEFAULT_MODEL_KEY).unwrap();
    let desired = current_fixture(remote.path(), &spec);
    let remote_pack = pack_root(remote.path(), &spec.model_key, &desired.pack_id);
    let server = PackServer::start(remote.path().join(INDEX_BASE_VERSION));
    let models = setup_test_dir();
    let model = model_file_path(models.path(), &spec);
    ensure_parent_dir(&model).unwrap();
    // Size-only setup validation accepts a sparse fixture. The precomputed path never opens llama.cpp.
    File::create(&model)
        .unwrap()
        .set_len(spec.native_size_bytes)
        .unwrap();
    for changed in [VLACKU_CORPUS_ID, CUKTA_CORPUS_ID] {
        for damaged in [false, true] {
            let index = setup_test_dir();
            let mut old = desired.clone();
            old.corpora
                .iter_mut()
                .find(|corpus| corpus.corpus_id == changed)
                .unwrap()
                .fingerprint = sha256_hex_bytes(b"older corpus");
            old.pack_id = deterministic_pack_id(
                &spec.input_format_version,
                &spec.model_revision,
                &spec.native_sha256,
                &old.corpora[0].fingerprint,
                &old.corpora[1].fingerprint,
            );
            assert_ne!(old.pack_id, desired.pack_id);
            let old_pack = pack_root(index.path(), &spec.model_key, &old.pack_id);
            for relative in native_pack_remote_paths(&desired).unwrap() {
                ensure_parent_dir(&old_pack.join(&relative)).unwrap();
                fs::copy(remote_pack.join(&relative), old_pack.join(relative)).unwrap();
            }
            let changed_corpus = old
                .corpora
                .iter_mut()
                .find(|corpus| corpus.corpus_id == changed)
                .unwrap();
            let items_path = old_pack.join(&changed_corpus.items_url);
            write_fixture_items(&items_path, changed, changed_corpus.row_count, true);
            changed_corpus.items_sha256 = sha256_hex_file(&items_path).unwrap();
            let vector_path = old_pack.join(&changed_corpus.shards[0].url);
            let mut file = File::options().write(true).open(&vector_path).unwrap();
            file.write_all(&0.0_f32.to_le_bytes()).unwrap();
            file.write_all(&1.0_f32.to_le_bytes()).unwrap();
            drop(file);
            changed_corpus.shards[0].sha256 = sha256_hex_file(&vector_path).unwrap();
            write_json_file(&old_pack.join("manifest.json"), &old).unwrap();
            write_catalog(index.path(), &spec, &old.pack_id).unwrap();
            validate_pack_dir(&old_pack).unwrap();
            let unchanged = old
                .corpora
                .iter()
                .find(|corpus| corpus.corpus_id != changed)
                .unwrap();
            if damaged {
                let mut file = File::options()
                    .write(true)
                    .open(old_pack.join(&unchanged.shards[0].url))
                    .unwrap();
                file.seek(SeekFrom::Start(0)).unwrap();
                // A one-bit, same-size change stays near unit length but fails the SHA-256 proof.
                file.write_all(&[1]).unwrap();
            }
            server.requests.lock().unwrap().clear();
            assert!(!pack_root(index.path(), &spec.model_key, &desired.pack_id).exists());
            let options = SetupOptions {
                force: false,
                use_precomputed: UsePrecomputed::Always,
                skip_validation: true,
                index_dir: Some(index.path().to_owned()),
                model_dir: Some(models.path().to_owned()),
                precomputed_base_url: format!("http://{}", server.address),
                ..SetupOptions::default()
            };
            if changed == VLACKU_CORPUS_ID && !damaged {
                let path = catalog_path(index.path()).unwrap();
                let original: EmbeddingCatalog = read_json_file(&path).unwrap();
                for corruption in ["schema", "model", "path", "missing", "identity"] {
                    let mut catalog = original.clone();
                    let mut manifest = old.clone();
                    match corruption {
                        "schema" => catalog.schema_version += 1,
                        "model" => catalog.models.clear(),
                        "path" => catalog.models[0].manifest_url = "../manifest.json".to_owned(),
                        "identity" => manifest.pack_id.push_str("-wrong"),
                        "missing" => {}
                        _ => unreachable!(),
                    }
                    write_json_file(&path, &catalog).unwrap();
                    let manifest_path = old_pack.join("manifest.json");
                    write_json_file(&manifest_path, &manifest).unwrap();
                    if corruption == "missing" {
                        fs::remove_file(&manifest_path).unwrap();
                    }
                    assert!(native::setup_embeddings(&options).is_err(), "{corruption}");
                    assert!(server.requests.lock().unwrap().is_empty(), "{corruption}");
                }
                write_json_file(&path, &original).unwrap();
                write_json_file(&old_pack.join("manifest.json"), &old).unwrap();
            }
            let report = native::setup_embeddings(&options).unwrap();
            assert_eq!(report.pack_id, desired.pack_id);
            assert_eq!(report.index_source, SetupIndexSource::DownloadedPrecomputed);
            let manifest_url = format!(
                "models/{}/packs/{}/manifest.json",
                spec.model_key, desired.pack_id
            );
            let prefix = manifest_url.strip_suffix("manifest.json").unwrap();
            let changed_corpus = desired
                .corpora
                .iter()
                .find(|corpus| corpus.corpus_id == changed)
                .unwrap();
            let mut expected = vec![
                "catalog.json".to_owned(),
                manifest_url.clone(),
                format!("{prefix}{}", changed_corpus.items_url),
                format!("{prefix}{}", changed_corpus.shards[0].url),
            ];
            if damaged {
                expected.push(format!("{prefix}{}", unchanged.shards[0].url));
            }
            let mut fetched = server.requests.lock().unwrap().clone();
            expected.sort();
            fetched.sort();
            assert_eq!(fetched, expected, "{changed}, damaged={damaged}");
            let installed = pack_root(index.path(), &spec.model_key, &desired.pack_id);
            for relative in native_pack_remote_paths(&desired).unwrap() {
                assert_eq!(
                    sha256_hex_file(&installed.join(&relative)).unwrap(),
                    sha256_hex_file(&remote_pack.join(&relative)).unwrap()
                );
            }
            validate_pack_dir(&installed).unwrap();
            assert_eq!(
                read_latest_pack_manifest(index.path(), &spec.model_key)
                    .unwrap()
                    .1
                    .pack_id,
                desired.pack_id
            );
        }
    }
}
