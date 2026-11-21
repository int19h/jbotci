//! Exercise installed-index updates through the same setup entry point as the CLI and desktop.

use super::*;
use std::io::{Seek, SeekFrom};
use std::net::TcpListener;
use std::sync::Condvar;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::thread::JoinHandle;

/// Retry a socket call that a signal interrupts.
/// When the error is not an interrupt, return that error.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| error.kind() != std::io::ErrorKind::Interrupted))]
fn retry_interrupted<T>(mut operation: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    loop {
        match operation() {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn setup_test_dir() -> tempfile::TempDir {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    tempfile::tempdir_in(root).unwrap()
}

const PACK_SOCKET_POLL: std::time::Duration = std::time::Duration::from_millis(100);
const PACK_HANG_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);

/// One read from the pack server socket.
/// `Line` keeps the bytes through the newline, or through end of file.
/// That text is not empty.
/// `End` means the client closed the socket before the next byte.
/// `Stopped` means the stop flag is true.
/// `Broken` means the socket failed for a reason other than a stall.
#[invariant(true)]
#[invariant(::Line(text) => !text.is_empty())]
enum PackRead {
    Line(String),
    End,
    Stopped,
    Broken,
}

/// One write to the pack server socket.
/// `Done` means every byte was written.
/// `Stopped` means the stop flag is true.
/// `Broken` means the socket failed for a reason other than a stall.
#[invariant(true)]
enum PackWrite {
    Done,
    Stopped,
    Broken,
}

#[requires(true)]
#[ensures(true)]
fn pack_io_is_a_stall(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock || error.kind() == std::io::ErrorKind::TimedOut
}

/// Read one request line.
/// The socket timeout is 100 milliseconds.
/// A stall is tried again while the stop flag is false.
/// A stall of 60 seconds with a false stop flag fails the hang limit.
/// The failure text says that this limit only stops a hang.
/// A true stop flag ends the read.
#[requires(true)]
#[ensures(true)]
fn read_pack_line(stream: &mut std::net::TcpStream, stop: &AtomicBool) -> PackRead {
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    let mut progressed = std::time::Instant::now();
    loop {
        if stop.load(AtomicOrdering::Acquire) {
            return new!(PackRead::Stopped);
        }
        if progressed.elapsed() >= PACK_HANG_LIMIT {
            panic!("this limit only stops a hang");
        }
        match retry_interrupted(|| std::io::Read::read(stream, &mut byte)) {
            Ok(0) => {
                if line.is_empty() {
                    return new!(PackRead::End);
                }
                return new!(PackRead::Line(String::from_utf8_lossy(&line).into_owned(),));
            }
            Ok(_) => {
                progressed = std::time::Instant::now();
                line.push(byte[0]);
                if byte[0] == b'\n' {
                    return new!(PackRead::Line(String::from_utf8_lossy(&line).into_owned(),));
                }
            }
            Err(error) if pack_io_is_a_stall(&error) => continue,
            Err(_) => return new!(PackRead::Broken),
        }
    }
}

/// Write every byte.
/// The socket timeout is 100 milliseconds.
/// A stall is tried again while the stop flag is false.
/// A stall of 60 seconds with a false stop flag fails the hang limit.
/// The failure text says that this limit only stops a hang.
/// A true stop flag ends the write.
#[requires(true)]
#[ensures(true)]
fn write_pack_bytes(
    stream: &mut std::net::TcpStream,
    bytes: &[u8],
    stop: &AtomicBool,
) -> PackWrite {
    let mut offset = 0;
    let mut progressed = std::time::Instant::now();
    while offset < bytes.len() {
        if stop.load(AtomicOrdering::Acquire) {
            return PackWrite::Stopped;
        }
        if progressed.elapsed() >= PACK_HANG_LIMIT {
            panic!("this limit only stops a hang");
        }
        match retry_interrupted(|| std::io::Write::write(stream, &bytes[offset..])) {
            Ok(0) => return PackWrite::Broken,
            Ok(count) => {
                offset += count;
                progressed = std::time::Instant::now();
            }
            Err(error) if pack_io_is_a_stall(&error) => continue,
            Err(_) => return PackWrite::Broken,
        }
    }
    PackWrite::Done
}

#[invariant(true)]
struct PackServer {
    address: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    request_wake: Arc<Condvar>,
    accept_ready: Arc<Mutex<bool>>,
    accept_wake: Arc<Condvar>,
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
        retry_interrupted(|| listener.set_nonblocking(true))
            .expect("the pack server socket accepts nonblocking mode");
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let request_wake = Arc::new(Condvar::new());
        let accept_ready = Arc::new(Mutex::new(false));
        let accept_wake = Arc::new(Condvar::new());
        let stop = Arc::new(AtomicBool::new(false));
        let worker_requests = Arc::clone(&requests);
        let worker_wake = Arc::clone(&request_wake);
        let worker_ready = Arc::clone(&accept_ready);
        let worker_ready_wake = Arc::clone(&accept_wake);
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            // The listener does not block in accept.
            // The first WouldBlock poll records that the accept loop is running.
            // Each pass reads the stop flag.
            // Shutdown can join without a wakeup connection.
            // An interrupted accept is tried again.
            'accept: while !worker_stop.load(AtomicOrdering::Acquire) {
                let (mut stream, _) = match retry_interrupted(|| listener.accept()) {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        {
                            let mut ready = worker_ready
                                .lock()
                                .expect("the pack server accept flag is open");
                            if !*ready {
                                *ready = true;
                                worker_ready_wake.notify_all();
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("the pack server accept failed: {error}"),
                };
                // On macOS and BSD, an accepted socket keeps the nonblocking mode of the listener.
                // A nonblocking read returns WouldBlock.
                // Set this socket to blocking mode before the read.
                retry_interrupted(|| stream.set_nonblocking(false))
                    .expect("the pack server accepted socket accepts blocking mode");
                // The socket timeout is 100 milliseconds.
                // A stall is tried again until the stop flag is true or the hang limit fires.
                retry_interrupted(|| stream.set_read_timeout(Some(PACK_SOCKET_POLL)))
                    .expect("the pack server accepted socket accepts a read timeout");
                retry_interrupted(|| stream.set_write_timeout(Some(PACK_SOCKET_POLL)))
                    .expect("the pack server accepted socket accepts a write timeout");
                let line = match read_pack_line(&mut stream, &worker_stop).into_data() {
                    data!(PackRead::Line(line)) => line,
                    data!(PackRead::Stopped) => break,
                    data!(PackRead::End) => continue,
                    data!(PackRead::Broken) => continue,
                };
                let mut parts = line.split_whitespace();
                assert_eq!(parts.next(), Some("GET"));
                let relative = parts.next().unwrap().trim_start_matches('/').to_owned();
                // Record the request, then wake a test that waits for it.
                // The wake uses the same mutex as the request list.
                {
                    let mut recorded = worker_requests.lock().unwrap();
                    recorded.push(relative.clone());
                    worker_wake.notify_one();
                }
                loop {
                    match read_pack_line(&mut stream, &worker_stop).into_data() {
                        data!(PackRead::Stopped) => break 'accept,
                        data!(PackRead::Broken) => continue 'accept,
                        data!(PackRead::End) => break,
                        data!(PackRead::Line(header)) if header == "\r\n" => break,
                        data!(PackRead::Line(_)) => {}
                    }
                }
                let path = root.join(safe_pack_relative_path(&relative).unwrap());
                if let Ok(mut file) = File::open(path) {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        file.metadata().unwrap().len()
                    );
                    // io::copy uses sendfile for a file and a socket.
                    // sendfile ignores the write timeout.
                    // Each chunk uses the 100 millisecond timeout.
                    match write_pack_bytes(&mut stream, header.as_bytes(), &worker_stop) {
                        PackWrite::Done => {}
                        PackWrite::Stopped => break,
                        PackWrite::Broken => continue,
                    }
                    let mut buf = [0_u8; 64 * 1024];
                    loop {
                        let count = match retry_interrupted(|| file.read(&mut buf)) {
                            Ok(0) => break,
                            Ok(count) => count,
                            Err(_) => break,
                        };
                        match write_pack_bytes(&mut stream, &buf[..count], &worker_stop) {
                            PackWrite::Done => {}
                            PackWrite::Stopped => break 'accept,
                            PackWrite::Broken => break,
                        }
                    }
                } else {
                    match write_pack_bytes(
                        &mut stream,
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        &worker_stop,
                    ) {
                        PackWrite::Stopped => break,
                        PackWrite::Done | PackWrite::Broken => {}
                    }
                }
            }
        });
        Self {
            address,
            requests,
            request_wake,
            accept_ready,
            accept_wake,
            stop,
            worker: Some(worker),
        }
    }

    /// Stop the worker and return its result.
    /// The accept loop reads the stop flag.
    /// A side thread joins the worker.
    /// If that join does not finish in 60 seconds, return an error.
    /// The error says that this limit only stops a hang.
    /// On that failure the side thread keeps waiting until the process exits.
    /// This path does not open a wakeup connection.
    #[requires(true)]
    #[ensures(self.worker.is_none())]
    fn shutdown(&mut self) -> Result<(), String> {
        self.stop.store(true, AtomicOrdering::Release);
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = match worker.join() {
                Ok(()) => Ok(()),
                Err(payload) => Err(pack_server_panic_message(payload)),
            };
            let _ = result_tx.send(result);
        });
        match result_rx.recv_timeout(PACK_HANG_LIMIT) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                Err("this limit only stops a hang".to_owned())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err("the hang limit thread stopped".to_owned())
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.starts_with("the pack server thread panicked"))]
fn pack_server_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return format!("the pack server thread panicked: {message}");
    }
    if let Some(message) = payload.downcast_ref::<&str>() {
        return format!("the pack server thread panicked: {message}");
    }
    "the pack server thread panicked".to_owned()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_shutdown_joins_without_a_client() {
    // No client connects.
    // The test waits until the accept loop records its first poll.
    // The accept loop reads the stop flag.
    // Shutdown returns without a wakeup connection.
    let root = setup_test_dir();
    let mut server = PackServer::start(root.path().to_owned());
    wait_until_pack_accept_is_ready(&server);
    server
        .shutdown()
        .expect("the pack server thread finishes cleanly");
}

/// Wait until the accept loop records its first poll.
/// A wall clock of 60 seconds waits with the ready flag.
/// When the flag is true, a correct run returns.
/// When the flag stays false, the clock fires.
/// The failure text says that this limit only stops a hang.
#[requires(true)]
#[ensures(true)]
fn wait_until_pack_accept_is_ready(server: &PackServer) {
    let guard = server
        .accept_ready
        .lock()
        .expect("the pack server accept flag is open");
    let (guard, _status) = server
        .accept_wake
        .wait_timeout_while(guard, PACK_HANG_LIMIT, |ready| !*ready)
        .expect("the pack server accept flag is open");
    if !*guard {
        panic!("this limit only stops a hang");
    }
}

/// Wait until the worker records one request.
/// A wall clock of 60 seconds waits with the request list.
/// When the record is present, a correct run returns.
/// When the record does not come, the clock fires.
/// The failure text says that this limit only stops a hang.
#[requires(true)]
#[ensures(true)]
fn wait_for_pack_request(server: &PackServer) {
    let guard = server
        .requests
        .lock()
        .expect("the pack server request list is open");
    let (guard, _status) = server
        .request_wake
        .wait_timeout_while(guard, std::time::Duration::from_secs(60), |requests| {
            requests.is_empty()
        })
        .expect("the pack server request list is open");
    if guard.is_empty() {
        panic!("this limit only stops a hang");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pack_server_shutdown_returns_during_a_blocked_write() {
    // The file length is 256 MiB.
    // The client sends the request and does not read the response.
    // The test waits until the worker records that request.
    // The write fills the socket buffer and then blocks.
    // The short timeout returns.
    // The worker sees the stop flag.
    // Shutdown returns the worker result.
    let root = setup_test_dir();
    let big = File::create(root.path().join("big")).unwrap();
    big.set_len(256 * 1024 * 1024).unwrap();
    drop(big);
    let mut server = PackServer::start(root.path().to_owned());
    let mut client = std::net::TcpStream::connect(server.address).unwrap();
    client.write_all(b"GET /big HTTP/1.1\r\n\r\n").unwrap();
    wait_for_pack_request(&server);
    let started = std::time::Instant::now();
    let result = server.shutdown();
    let elapsed = started.elapsed();
    drop(client);
    eprintln!(
        "PACK_SHUTDOWN_OK={} PACK_SHUTDOWN_MS={}",
        result.is_ok(),
        elapsed.as_millis()
    );
    result.expect("the pack server thread finishes cleanly");
}

impl Drop for PackServer {
    #[requires(true)]
    #[ensures(self.worker.is_none())]
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            let _ = std::io::Write::write_fmt(
                &mut std::io::stderr(),
                format_args!("the pack server thread failed during drop: {error}\n"),
            );
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
    let mut server = PackServer::start(remote.path().join(INDEX_BASE_VERSION));
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
    server
        .shutdown()
        .expect("the pack server thread finishes cleanly");
}
