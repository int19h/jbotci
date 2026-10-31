use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::R2UploadObject;
#[cfg(test)]
use crate::R2UploadObjectData;
use crate::r2_skip::{LiveMetadata, local_proof_with};

const WRANGLER_VERSION: &str = "4.149.0";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(180);
const ATTEMPTS: usize = 3;
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[invariant(account.len() == 32 && account.bytes().all(|byte| byte.is_ascii_hexdigit()))]
pub(super) struct Verifier {
    script: PathBuf,
    account: String,
    scratch: PathBuf,
}

impl Verifier {
    #[requires(true)]
    #[ensures(ret.as_ref().map_or(true, |reader| !reader.account.is_empty()))]
    pub(super) fn from_env() -> Result<Self> {
        require_unix_host(cfg!(unix))?;
        let account = std::env::var("CLOUDFLARE_ACCOUNT_ID")
            .context("Set CLOUDFLARE_ACCOUNT_ID before R2 access")?;
        if account.len() != 32 || !account.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("CLOUDFLARE_ACCOUNT_ID must contain 32 hexadecimal digits");
        }
        let install = PathBuf::from(std::env::var_os("JBOTCI_R2_WRANGLER_DIR")
            .context("Set JBOTCI_R2_WRANGLER_DIR to the pinned Wrangler installation. See docs/embedding-publish.md")?);
        let package = install.join("node_modules/wrangler");
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(package.join("package.json"))?)?;
        if metadata["name"] != "wrangler" || metadata["version"] != WRANGLER_VERSION {
            bail!("The publisher requires Wrangler {WRANGLER_VERSION}");
        }
        let script = package.join("wrangler-dist/cli.js").canonicalize()?;
        let scratch = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"))
            .join("r2-readback");
        fs::create_dir_all(&scratch)?;
        Ok(Self::from_data(data!(Verifier {
            script: script,
            account: account,
            scratch: scratch,
        })))
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn verify_existing(
        &self,
        bucket: &str,
        object: &R2UploadObject,
        report: impl Fn(&VerificationFacts),
    ) -> Result<()> {
        let expected = fingerprint(&object.local_path)?;
        self.verify_with_readback(
            object,
            &expected,
            |downloaded, metadata| self.read_existing(bucket, object, downloaded, metadata),
            report,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn verify_with_readback(
        &self,
        object: &R2UploadObject,
        expected: &Fingerprint,
        readback: impl FnOnce(&Path, &Path) -> Result<()>,
        report: impl Fn(&VerificationFacts),
    ) -> Result<()> {
        let temporary = Readback::new(&self.scratch)?;
        let downloaded = temporary.path.join("object");
        let metadata = temporary.path.join("metadata.json");
        let result = (|| {
            readback(&downloaded, &metadata)?;
            let metadata = readback_metadata(&metadata, object)?;
            verification_facts(object, expected, &downloaded, &metadata)
        })();
        match result {
            Ok(facts) => {
                report(&facts);
                facts.require_match(object)
            }
            Err(error) => {
                report(&VerificationFacts::unavailable());
                Err(error).with_context(|| {
                    format!("R2 object `{}` verification failed", object.object_key)
                })
            }
        }
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    fn read_existing(
        &self,
        bucket: &str,
        object: &R2UploadObject,
        downloaded: &Path,
        metadata: &Path,
    ) -> Result<()> {
        let mut get = self.node_command();
        get.arg("-e")
            .arg(concat!(
                include_str!("r2_readback.cjs"),
                "\nmain(process.argv.slice(1)).catch(reportReadbackError);"
            ))
            .arg(&self.script)
            .arg(&self.account)
            .arg(bucket.trim())
            .arg(&object.object_key)
            .arg(object.content_encoding.unwrap_or("identity"))
            .arg(downloaded)
            .arg(metadata);
        run_readback(&mut get, COMMAND_TIMEOUT).context("R2 read-back failed")
    }

    #[requires(!bucket.is_empty() && !prefix.is_empty())]
    #[ensures(true)]
    pub(super) fn live_metadata(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<std::collections::BTreeMap<String, crate::r2_skip::LiveMetadata>> {
        let temporary = Readback::new(&self.scratch)?;
        let output = temporary.path.join("live.json");
        let mut list = self.node_command();
        list.arg("-e")
            .arg(concat!(include_str!("r2_readback.cjs"), "\nmetadataMain(process.argv.slice(1)).catch(() => { console.error('R2 live metadata request failed'); process.exitCode = 1; });"))
            .arg(&self.script).arg(&self.account).arg(bucket).arg(prefix).arg(&output);
        run_bounded(&mut list, COMMAND_TIMEOUT).context("R2 live metadata failed")?;
        let objects: Vec<crate::r2_skip::LiveMetadata> =
            serde_json::from_slice(&fs::read(output)?)?;
        let mut metadata = std::collections::BTreeMap::new();
        for object in objects {
            if metadata.insert(object.key.clone(), object).is_some() {
                bail!("R2 returned duplicate live metadata keys");
            }
        }
        Ok(metadata)
    }

    #[requires(true)]
    #[ensures(true)]
    fn node_command(&self) -> Command {
        let mut command = Command::new("node");
        configure_child_environment(&mut command, &self.account);
        command.stdin(Stdio::null());
        command
    }
}

// The verifier exposes only reads. Only this separate type can construct an R2 put command.
#[invariant(true)]
pub(super) struct Publisher {
    reader: Verifier,
}

impl Publisher {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn from_env() -> Result<Self> {
        Ok(Self {
            reader: Verifier::from_env()?,
        })
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn put(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        self.put_with_readback(bucket, object, |downloaded, metadata| {
            self.reader
                .read_existing(bucket, object, downloaded, metadata)
        })
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn verify_existing(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        self.reader.verify_existing(bucket, object, |_| {})
    }

    #[requires(!bucket.is_empty() && !prefix.is_empty())]
    #[ensures(true)]
    pub(super) fn live_metadata(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<std::collections::BTreeMap<String, LiveMetadata>> {
        self.reader.live_metadata(bucket, prefix)
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    fn put_with_readback(
        &self,
        bucket: &str,
        object: &R2UploadObject,
        mut readback: impl FnMut(&Path, &Path) -> Result<()>,
    ) -> Result<()> {
        let target = format!("{}/{}", bucket.trim(), object.object_key);
        let expected = fingerprint(&object.local_path)?;
        retry(
            ATTEMPTS,
            Duration::from_secs(2),
            || {
                self.reader.verify_with_readback(
                    object,
                    &expected,
                    |downloaded, metadata| {
                        let mut put = self.reader.node_command();
                        put.arg(&self.reader.script)
                            .args(["r2", "object", "put", &target, "--file"])
                            .arg(&object.local_path)
                            .args([
                                "--content-type",
                                object.content_type,
                                "--cache-control",
                                object.cache_control,
                                "--remote",
                                "--force",
                            ]);
                        if let Some(encoding) = object.content_encoding {
                            put.args(["--content-encoding", encoding]);
                        }
                        run_bounded(&mut put, COMMAND_TIMEOUT).context("R2 upload failed")?;
                        readback(downloaded, metadata)
                    },
                    |_| {},
                )
            },
            std::thread::sleep,
        )
        .with_context(|| format!("R2 object `{target}` failed after {ATTEMPTS} attempts"))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() == is_unix)]
fn require_unix_host(is_unix: bool) -> Result<()> {
    if !is_unix {
        bail!("R2 access requires a Unix host. Use Linux or macOS.");
    }
    Ok(())
}

#[requires(!account.is_empty())]
#[ensures(true)]
fn configure_child_environment(command: &mut Command, account: &str) {
    command
        .env("CLOUDFLARE_ACCOUNT_ID", account)
        .env("WRANGLER_WRITE_LOGS", "false")
        .env("WRANGLER_LOG", "log")
        .env("WRANGLER_LOG_SANITIZE", "true")
        .env("WRANGLER_SEND_METRICS", "false")
        .env("CI", "true")
        .env("NODE_V8_COVERAGE", "");
    for name in [
        "WRANGLER_LOG_PATH",
        "WRANGLER_OUTPUT_FILE_PATH",
        "WRANGLER_OUTPUT_FILE_DIRECTORY",
        "DEBUG",
        "NODE_DEBUG",
        "NODE_DEBUG_NATIVE",
        "NODE_OPTIONS",
        "SSLKEYLOGFILE",
        "LOG_STREAM",
        "LOG_TOKENS",
    ] {
        command.env_remove(name);
    }
}

#[invariant(true)]
#[derive(Deserialize)]
struct ReadbackMetadata {
    stored: LiveMetadata,
    body_encoding: Option<String>,
}

#[invariant(::Matched => true)]
#[invariant(::Mismatched => true)]
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
enum Fact {
    Matched,
    Mismatched { reason: String },
}

impl Fact {
    #[requires(true)]
    #[ensures(true)]
    fn from_result(result: Result<()>) -> Self {
        match result {
            Ok(()) => Self::Matched,
            Err(error) => Self::Mismatched {
                reason: format!("{error:#}"),
            },
        }
    }
}

#[invariant(true)]
#[derive(Debug, Serialize)]
pub(super) struct VerificationFacts {
    stored_size: Fact,
    stored_md5: Fact,
    encoding: Fact,
    body_sha256: Fact,
}

impl VerificationFacts {
    #[requires(true)]
    #[ensures(true)]
    fn unavailable() -> Self {
        Self {
            stored_size: Fact::Mismatched {
                reason: "proof unavailable: read-back failed".to_owned(),
            },
            stored_md5: Fact::Mismatched {
                reason: "proof unavailable: read-back failed".to_owned(),
            },
            encoding: Fact::Mismatched {
                reason: "proof unavailable: read-back failed".to_owned(),
            },
            body_sha256: Fact::Mismatched {
                reason: "proof unavailable: read-back failed".to_owned(),
            },
        }
    }

    #[requires(true)]
    #[ensures(ret.is_ok() == (matches!(&self.stored_size, Fact::Matched)
        && matches!(&self.stored_md5, Fact::Matched)
        && matches!(&self.encoding, Fact::Matched)
        && matches!(&self.body_sha256, Fact::Matched)))]
    fn require_match(&self, object: &R2UploadObject) -> Result<()> {
        for (name, fact) in [
            ("stored size", &self.stored_size),
            ("stored MD5", &self.stored_md5),
            ("encoding", &self.encoding),
            ("body SHA-256", &self.body_sha256),
        ] {
            if let Fact::Mismatched { reason } = fact {
                bail!("R2 object `{}`: {name}: {reason}", object.object_key);
            }
        }
        Ok(())
    }
}

#[requires(true)]
#[ensures(true)]
fn verification_facts(
    object: &R2UploadObject,
    expected: &Fingerprint,
    downloaded: &Path,
    metadata: &ReadbackMetadata,
) -> Result<VerificationFacts> {
    let actual = fingerprint(downloaded)?;
    let check_key = || -> Result<()> {
        if metadata.stored.key != object.object_key {
            bail!("stored object key differs from the upload");
        }
        Ok(())
    };
    let stored_size = Fact::from_result((|| {
        check_key()?;
        if metadata.stored.stored_size()? != expected.size {
            bail!("stored size differs from the local upload file");
        }
        Ok(())
    })());
    let stored_md5 = Fact::from_result((|| {
        check_key()?;
        if metadata.stored.stored_md5()? != expected.md5 {
            bail!("stored MD5 differs from the local upload file");
        }
        Ok(())
    })());
    let encoding = Fact::from_result((|| {
        check_key()?;
        if metadata.stored.stored_encoding()? != object.content_encoding {
            bail!("stored Content-Encoding differs from the upload");
        }
        match (object.content_encoding, metadata.body_encoding.as_deref()) {
            (None, None) | (Some("br"), Some("br")) => Ok(()),
            (Some("br"), None | Some("identity")) => {
                bail!("R2 returned a decoded Brotli body. Exact stored bytes cannot be verified")
            }
            _ => bail!("R2 returned an unexpected Content-Encoding"),
        }
    })());
    Ok(VerificationFacts {
        stored_size,
        stored_md5,
        encoding,
        body_sha256: Fact::from_result(compare_fingerprint(expected, &actual)),
    })
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn verify_readback(
    object: &R2UploadObject,
    expected: &Fingerprint,
    downloaded: &Path,
    metadata: &ReadbackMetadata,
) -> Result<()> {
    verification_facts(object, expected, downloaded, metadata)?.require_match(object)
}

#[requires(true)]
#[ensures(true)]
fn readback_metadata(path: &Path, object: &R2UploadObject) -> Result<ReadbackMetadata> {
    serde_json::from_slice(&fs::read(path)?).map_err(|_| {
        anyhow::anyhow!(
            "R2 object `{}` has malformed read-back metadata",
            object.object_key
        )
    })
}

#[invariant(true)]
#[derive(Debug, PartialEq, Eq)]
struct Fingerprint {
    size: u64,
    digest: [u8; 32],
    md5: [u8; 16],
}

#[requires(true)]
#[ensures(true)]
fn fingerprint(path: &Path) -> Result<Fingerprint> {
    let mut digest = Sha256::new();
    let proof = local_proof_with(path, |bytes| digest.update(bytes))?;
    Ok(Fingerprint {
        size: proof.size,
        digest: digest.finalize().into(),
        md5: proof.md5,
    })
}

#[requires(true)]
#[ensures(ret.is_ok() == (expected == actual))]
fn compare_fingerprint(expected: &Fingerprint, actual: &Fingerprint) -> Result<()> {
    if expected != actual {
        bail!("R2 read-back size or SHA-256 digest differs from the source file");
    }
    Ok(())
}

#[requires(attempts > 0)]
#[ensures(true)]
fn retry(
    mut attempts: usize,
    mut backoff: Duration,
    mut operation: impl FnMut() -> Result<()>,
    mut sleep: impl FnMut(Duration),
) -> Result<()> {
    loop {
        match operation() {
            Ok(()) => return Ok(()),
            Err(error) => {
                attempts -= 1;
                if attempts == 0 {
                    return Err(error);
                }
                eprintln!(
                    "R2 attempt failed: {error:#}. Retry in {} seconds",
                    backoff.as_secs()
                );
                sleep(backoff);
                backoff = backoff.saturating_mul(2);
            }
        }
    }
}

#[requires(!timeout.is_zero())]
#[ensures(true)]
fn run_bounded(command: &mut Command, timeout: Duration) -> Result<()> {
    require_unix_host(cfg!(unix))?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .context("Cannot start the publisher command")?;
    wait_bounded(&mut child, timeout)
}

#[requires(!timeout.is_zero())]
#[ensures(true)]
fn wait_bounded(child: &mut std::process::Child, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    bail!("Publisher command failed with {status}");
                }
                return Ok(());
            }
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            result => {
                #[cfg(unix)]
                // Kill descendants too if the command starts another process.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                if let Err(error) = result {
                    return Err(error.into());
                }
                bail!(
                    "Publisher command timed out after {} seconds",
                    timeout.as_secs_f64()
                );
            }
        }
    }
}

#[invariant(true)]
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReadbackDiagnostic {
    Changed,
    Decoded,
    Metadata,
    Http,
    Io,
}

#[invariant(true)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticEnvelope {
    r2_readback_error: ReadbackDiagnostic,
    body_etag: Option<String>,
    stored_etag: Option<String>,
}

#[requires(bytes.len() <= 4096)]
#[ensures(ret.len() < 2048)]
fn redacted_readback_stderr(bytes: &[u8]) -> String {
    // ETag values are not credentials. Keep their bounded ASCII fields separate from arbitrary stderr.
    let Ok(value) = serde_json::from_slice::<DiagnosticEnvelope>(bytes) else {
        return "[redacted untrusted read-back stderr]".to_owned();
    };
    match (value.r2_readback_error, value.body_etag, value.stored_etag) {
        (ReadbackDiagnostic::Changed, Some(body), Some(stored))
            if body.len() <= 128 && body.is_ascii() && stored.len() <= 128 && stored.is_ascii() =>
        {
            format!(
                "R2 object changed between metadata and body reads; body ETag={}; stored ETag={}",
                serde_json::to_string(&body).expect("ASCII string serialization"),
                serde_json::to_string(&stored).expect("ASCII string serialization")
            )
        }
        (ReadbackDiagnostic::Decoded, None, None) => {
            "R2 returned a decoded Brotli body. Exact stored bytes cannot be verified".to_owned()
        }
        (ReadbackDiagnostic::Metadata, None, None) => {
            "R2 returned invalid or incomplete object metadata".to_owned()
        }
        (ReadbackDiagnostic::Http, None, None) => "R2 read-back HTTP request failed".to_owned(),
        (ReadbackDiagnostic::Io, None, None) => {
            "R2 read-back failed (untrusted details redacted)".to_owned()
        }
        _ => "[redacted untrusted read-back stderr]".to_owned(),
    }
}

#[requires(!timeout.is_zero())]
#[ensures(true)]
fn run_readback(command: &mut Command, timeout: Duration) -> Result<()> {
    require_unix_host(cfg!(unix))?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stderr = child
        .stderr
        .take()
        .context("missing read-back stderr pipe")?;
    let reader = std::thread::spawn(move || {
        // Drain after the limit too, so verbose failures cannot block the child or grow memory use.
        let mut bytes = Vec::new();
        stderr.by_ref().take(4096).read_to_end(&mut bytes)?;
        std::io::copy(&mut stderr, &mut std::io::sink())?;
        Ok::<_, std::io::Error>(bytes)
    });
    let result = wait_bounded(&mut child, timeout);
    #[cfg(unix)]
    // Close inherited pipes if a command exits while one of its descendants survives.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let bytes = reader.join().ok().and_then(Result::ok).unwrap_or_default();
    result.with_context(|| format!("Read-back stderr: {}", redacted_readback_stderr(&bytes)))
}

#[invariant(true)]
struct Readback {
    path: PathBuf,
}
impl Readback {
    #[requires(true)]
    #[ensures(true)]
    fn new(root: &Path) -> Result<Self> {
        loop {
            let path = root.join(format!(
                "{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }
}
impl Drop for Readback {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn receipt(object: &R2UploadObject) -> serde_json::Value {
        use md5::Md5;
        let bytes = fs::read(&object.local_path).unwrap();
        let mut http = serde_json::json!({"contentType": object.content_type, "cacheControl": object.cache_control});
        if let Some(encoding) = object.content_encoding {
            http["contentEncoding"] = encoding.into();
        }
        serde_json::json!({"stored": {"key": object.object_key, "size": bytes.len(),
            "etag": format!("{:x}", Md5::digest(&bytes)), "http_metadata": http},
            "body_encoding": object.content_encoding})
    }

    #[invariant(true)]
    struct R6Server {
        url: String,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
        requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl R6Server {
        #[requires(true)]
        #[ensures(true)]
        fn start(stored: serde_json::Value, body: Vec<u8>, encoding: Option<&'static str>) -> Self {
            use std::io::{Read, Write};
            let listener = (18800..=18899)
                .find_map(|port| std::net::TcpListener::bind(("127.0.0.1", port)).ok())
                .unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let worker_stop = stop.clone();
            let worker_requests = requests.clone();
            let worker = std::thread::spawn(move || {
                while !worker_stop.load(Ordering::Relaxed) {
                    let (mut socket, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2));
                            continue;
                        }
                        Err(error) => panic!("{error}"),
                    };
                    socket
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    while !request.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        if socket.read(&mut byte).unwrap() == 0 {
                            break;
                        }
                        request.push(byte[0]);
                        assert!(request.len() < 16 * 1024);
                    }
                    let request = String::from_utf8(request).unwrap();
                    let first = request.lines().next().unwrap().to_owned();
                    worker_requests.lock().unwrap().push(first.clone());
                    assert!(
                        first.starts_with("GET "),
                        "read-only requests only: {first}"
                    );
                    let listing = first
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .split('?')
                        .next()
                        .unwrap()
                        .ends_with("/objects");
                    if listing {
                        let json = serde_json::to_vec(
                            &serde_json::json!({"success":true,"result":[stored]}),
                        )
                        .unwrap();
                        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", json.len()).unwrap();
                        socket.write_all(&json).unwrap();
                    } else {
                        let etag = stored["etag"].as_str().unwrap();
                        if encoding == Some("br") {
                            write!(socket, "HTTP/1.1 200 OK\r\nETag: \"{etag}\"\r\nContent-Encoding: br\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                            socket.write_all(&body).unwrap();
                        } else {
                            write!(socket, "HTTP/1.1 200 OK\r\nETag: W/\"{etag}\"\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n", body.len()).unwrap();
                            socket.write_all(&body).unwrap();
                            socket.write_all(b"\r\n0\r\n\r\n").unwrap();
                        }
                    }
                }
            });
            Self {
                url,
                stop,
                requests,
                worker: Some(worker),
            }
        }
    }

    impl Drop for R6Server {
        #[requires(true)]
        #[ensures(true)]
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r2_read_only_verifier_uses_http_and_reports_each_fact_and_wrong_local_files() {
        let root = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap());
        let temporary = Readback::new(&root).unwrap();
        let reader = Verifier::from_data(data!(Verifier {
            script: temporary.path.join("must-not-run.cjs"),
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        for encoding in [None, Some("br")] {
            for case in ["match", "size", "md5", "encoding", "body", "wrong-local"] {
                let original = temporary.path.join("source.f16");
                let sibling = crate::brotli_sidecar_path(&original).unwrap();
                fs::write(&original, b"validated original bytes").unwrap();
                if encoding.is_some() {
                    fs::write(&sibling, b"\x83\x0b\x80validated original bytes\x03").unwrap();
                } else if sibling.exists() {
                    fs::remove_file(&sibling).unwrap();
                }
                let args = crate::VerifyR2ObjectArgs {
                    bucket: "stub-bucket".to_owned(),
                    key: "packs/test.f16".to_owned(),
                    file: original.clone(),
                    brotli_file: encoding.map(|_| sibling.clone()),
                };
                let good = args.object().unwrap();
                let mut stored = receipt(&good)["stored"].clone();
                let mut body = fs::read(&good.local_path).unwrap();
                match case {
                    "size" => stored["size"] = (body.len() + 1).into(),
                    "md5" => stored["etag"] = "00000000000000000000000000000000".into(),
                    "encoding" if encoding.is_some() => {
                        stored["http_metadata"]
                            .as_object_mut()
                            .unwrap()
                            .remove("contentEncoding");
                    }
                    "encoding" => stored["http_metadata"]["contentEncoding"] = "br".into(),
                    "body" => body[0] ^= 1,
                    "wrong-local" if encoding.is_some() => {
                        fs::write(&sibling, b"\x85\x0b\x80validated original bytes\x03").unwrap()
                    }
                    "wrong-local" => {
                        let mut wrong = body.clone();
                        wrong[0] ^= 1;
                        fs::write(&original, wrong).unwrap();
                    }
                    _ => {}
                }
                let object = args.object().unwrap();
                let server = R6Server::start(stored, body, encoding);
                let expected = fingerprint(&object.local_path).unwrap();
                let report = std::cell::RefCell::new(None);
                let result = reader.verify_with_readback(&object, &expected, |downloaded, metadata| {
                    let mut command = reader.node_command();
                    command.arg("-e").arg(concat!(include_str!("r2_readback.cjs"), "\nconst [apiBase,key,encoding,downloaded,metadata] = process.argv.slice(1); readback({account:'stub-account',bucket:'stub-bucket',key,encoding,downloaded,metadata,headers:{Authorization:'Bearer stub-token'},apiBase}).catch(reportReadbackError);"))
                        .arg(&server.url).arg(&object.object_key).arg(object.content_encoding.unwrap_or("identity")).arg(downloaded).arg(metadata);
                    run_readback(&mut command, Duration::from_secs(5))
                }, |facts| *report.borrow_mut() = Some(serde_json::to_value(facts).unwrap()));
                assert_eq!(
                    result.is_ok(),
                    case == "match",
                    "{encoding:?}/{case}: {result:?}"
                );
                let report = report.into_inner().unwrap();
                for (field, failure) in [
                    ("stored_size", case == "size"),
                    ("stored_md5", case == "md5" || case == "wrong-local"),
                    ("encoding", case == "encoding"),
                    ("body_sha256", case == "body" || case == "wrong-local"),
                ] {
                    assert_eq!(
                        report[field]["status"],
                        if failure { "mismatched" } else { "matched" },
                        "{encoding:?}/{case}/{field}: {report}"
                    );
                }
                let requests = server.requests.lock().unwrap();
                assert_eq!(requests.len(), 2);
                assert!(requests.iter().all(|request| request.starts_with("GET ")));
            }
        }
        let original = temporary.path.join("source.f16");
        let sibling = crate::brotli_sidecar_path(&original).unwrap();
        let mut args = crate::VerifyR2ObjectArgs {
            bucket: "stub-bucket".to_owned(),
            key: "packs/test.f16".to_owned(),
            file: original,
            brotli_file: None,
        };
        assert_eq!(args.object().unwrap().content_encoding, Some("br"));
        let other = temporary.path.join("other.br");
        fs::copy(&sibling, &other).unwrap();
        args.brotli_file = Some(other);
        assert!(args.object().is_err());
        args.brotli_file = Some(sibling.clone());
        fs::remove_file(&sibling).unwrap();
        assert!(args.object().is_err());
        args.brotli_file = None;
        let object = args.object().unwrap();
        assert_eq!(object.content_encoding, None);
        let expected = fingerprint(&object.local_path).unwrap();
        let report = std::cell::RefCell::new(None);
        assert!(
            reader
                .verify_with_readback(
                    &object,
                    &expected,
                    |_, _| bail!("stub network failure"),
                    |facts| *report.borrow_mut() = Some(serde_json::to_value(facts).unwrap())
                )
                .is_err()
        );
        let report = report.into_inner().unwrap();
        for field in ["stored_size", "stored_md5", "encoding", "body_sha256"] {
            assert_eq!(report[field]["status"], "mismatched");
            assert!(
                report[field]["reason"]
                    .as_str()
                    .unwrap()
                    .contains("unavailable")
            );
        }
        fs::write(&sibling, b"invalid Brotli").unwrap();
        assert!(args.object().is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stub_retry_succeeds_and_uses_bounded_backoff() {
        let mut calls = 0;
        let mut delays = Vec::new();
        retry(
            3,
            Duration::from_secs(2),
            || {
                calls += 1;
                if calls < 3 {
                    bail!("stub transient failure");
                }
                Ok(())
            },
            |delay| delays.push(delay),
        )
        .unwrap();
        assert_eq!(calls, 3);
        assert_eq!(delays, [Duration::from_secs(2), Duration::from_secs(4)]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stub_permanent_failure_stops_at_attempt_limit() {
        let mut calls = 0;
        let error = retry(
            3,
            Duration::ZERO,
            || {
                calls += 1;
                bail!("stub permanent failure");
            },
            |_| {},
        )
        .unwrap_err();
        assert_eq!(calls, 3);
        assert!(error.to_string().contains("stub permanent failure"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn readback_rejects_same_size_wrong_digest_and_wrong_size() {
        let expected = Fingerprint {
            size: 2,
            digest: [1; 32],
            md5: [0; 16],
        };
        assert!(
            compare_fingerprint(
                &expected,
                &Fingerprint {
                    size: 2,
                    digest: [2; 32],
                    md5: [0; 16]
                }
            )
            .is_err()
        );
        assert!(
            compare_fingerprint(
                &expected,
                &Fingerprint {
                    size: 3,
                    digest: [1; 32],
                    md5: [0; 16]
                }
            )
            .is_err()
        );
        assert!(compare_fingerprint(&expected, &expected).is_ok());
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stub_uploader_reads_back_actual_bytes() {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        fs::create_dir_all(&root).unwrap();
        let temporary = Readback::new(&root).unwrap();
        let script = temporary.path.join("stub.cjs");
        fs::write(
            &script,
            r#"
const fs = require('node:fs');
const path = require('node:path');
const args = process.argv.slice(2);
if (process.env.CLOUDFLARE_ACCOUNT_ID !== '0123456789abcdef0123456789abcdef') process.exit(9);
if (!args.includes('--remote')) process.exit(10);
const file = args[args.indexOf('--file') + 1];
const remote = path.join(__dirname, 'remote');
if (args[2] !== 'put') process.exit(11);
fs.copyFileSync(file, remote);
"#,
        )
        .unwrap();
        let source = temporary.path.join("source");
        fs::write(&source, b"exact upload bytes").unwrap();
        let object = R2UploadObject::from_data(data!(R2UploadObject {
            local_path: source,
            object_key: "packs/test.f16".to_owned(),
            content_type: "application/octet-stream",
            content_encoding: None,
            cache_control: "immutable",
        }));
        let reader = Verifier::from_data(data!(Verifier {
            script: script,
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        let publisher = Publisher { reader };
        publisher
            .put_with_readback("stub-bucket", &object, |downloaded, metadata| {
                fs::copy(temporary.path.join("remote"), downloaded)?;
                fs::write(metadata, serde_json::to_vec(&receipt(&object))?)?;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            fs::read(temporary.path.join("remote")).unwrap(),
            b"exact upload bytes"
        );
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stub_uploader_rejects_readback_mismatch_after_three_attempts() {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        fs::create_dir_all(&root).unwrap();
        let temporary = Readback::new(&root).unwrap();
        let script = temporary.path.join("stub.cjs");
        fs::write(
            &script,
            r#"
const fs = require('node:fs');
const path = require('node:path');
const args = process.argv.slice(2);
if (args[2] !== 'put') process.exit(11);
fs.appendFileSync(path.join(__dirname, 'calls'), 'put\n');
"#,
        )
        .unwrap();
        let source = temporary.path.join("source");
        fs::write(&source, b"right bytes").unwrap();
        let object = R2UploadObject::from_data(data!(R2UploadObject {
            local_path: source,
            object_key: "test.f16".to_owned(),
            content_type: "application/octet-stream",
            content_encoding: None,
            cache_control: "immutable",
        }));
        let reader = Verifier::from_data(data!(Verifier {
            script: script,
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        let publisher = Publisher { reader };
        let error = publisher
            .put_with_readback("stub-bucket", &object, |downloaded, metadata| {
                fs::write(downloaded, b"wrong bytes")?;
                fs::write(metadata, serde_json::to_vec(&receipt(&object))?)?;
                Ok(())
            })
            .unwrap_err();
        assert!(format!("{error:#}").contains("SHA-256 digest differs"));
        assert_eq!(
            fs::read_to_string(temporary.path.join("calls")).unwrap(),
            "put\nput\nput\n"
        );
        assert_eq!(fs::read_dir(&temporary.path).unwrap().count(), 3);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r4_readback_requires_every_stored_fact_and_the_exact_body() {
        let root = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap());
        let temporary = Readback::new(&root).unwrap();
        for encoding in [None, Some("br")] {
            let source = temporary.path.join("source");
            let original = b"validated original bytes";
            let encoded = b"\x83\x0b\x80validated original bytes\x03";
            fs::write(
                &source,
                if encoding.is_some() {
                    encoded.as_slice()
                } else {
                    original.as_slice()
                },
            )
            .unwrap();
            let object = R2UploadObject::from_data(data!(R2UploadObject {
                local_path: source,
                object_key: "packs/test.f16".to_owned(),
                content_type: "application/octet-stream",
                content_encoding: encoding,
                cache_control: "immutable",
            }));
            let expected = fingerprint(&object.local_path).unwrap();
            let downloaded = temporary.path.join("download");
            fs::copy(&object.local_path, &downloaded).unwrap();
            let good = receipt(&object);
            let metadata: ReadbackMetadata = serde_json::from_value(good.clone()).unwrap();
            verify_readback(&object, &expected, &downloaded, &metadata).unwrap();
            let mut cases = Vec::new();
            let mut wrong = good.clone();
            wrong["stored"]["size"] = (expected.size + 1).into();
            cases.push(("stored size", wrong));
            let mut wrong = good.clone();
            wrong["stored"]["etag"] = "00000000000000000000000000000000".into();
            cases.push(("stored MD5", wrong));
            let mut wrong = good.clone();
            wrong["stored"]["etag"] =
                format!("{}-2", good["stored"]["etag"].as_str().unwrap()).into();
            cases.push(("not a plain", wrong));
            for (field, reason) in [
                ("etag", "missing stored ETag"),
                ("size", "missing stored size"),
                ("http_metadata", "missing stored HTTP metadata"),
            ] {
                let mut wrong = good.clone();
                wrong["stored"].as_object_mut().unwrap().remove(field);
                cases.push((reason, wrong));
            }
            let mut wrong = good.clone();
            if encoding.is_some() {
                wrong["stored"]["http_metadata"]
                    .as_object_mut()
                    .unwrap()
                    .remove("contentEncoding");
            } else {
                wrong["stored"]["http_metadata"]["contentEncoding"] = "br".into();
            }
            cases.push(("Content-Encoding", wrong));
            for (reason, value) in cases {
                let metadata = serde_json::from_value(value).unwrap();
                let error =
                    verify_readback(&object, &expected, &downloaded, &metadata).unwrap_err();
                let text = format!("{error:#}");
                assert!(
                    text.contains(reason) && text.contains(&object.object_key),
                    "{encoding:?}: {text}"
                );
            }
            let mut wrong_body = fs::read(&object.local_path).unwrap();
            wrong_body[0] ^= 1;
            fs::write(&downloaded, wrong_body).unwrap();
            assert!(
                verify_readback(&object, &expected, &downloaded, &metadata)
                    .unwrap_err()
                    .to_string()
                    .contains("SHA-256")
            );
            if encoding.is_some() {
                // Equal decoded content is insufficient: read-back must retain the encoded stream.
                fs::write(&downloaded, b"\x85\x0b\x80validated original bytes\x03").unwrap();
                assert!(verify_readback(&object, &expected, &downloaded, &metadata).is_err());
                fs::write(&downloaded, original).unwrap();
                let mut decoded = good;
                decoded["body_encoding"] = serde_json::Value::Null;
                assert!(
                    verify_readback(
                        &object,
                        &expected,
                        &downloaded,
                        &serde_json::from_value(decoded).unwrap()
                    )
                    .unwrap_err()
                    .to_string()
                    .contains("decoded Brotli")
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn skipped_readback_uses_the_existing_comparison_and_never_uploads() {
        let root = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap());
        fs::create_dir_all(&root).unwrap();
        let temporary = Readback::new(&root).unwrap();
        let source = temporary.path.join("source");
        fs::write(&source, b"right bytes").unwrap();
        let object = R2UploadObject::from_data(data!(R2UploadObject {
            local_path: source,
            object_key: "packs/test.f16".to_owned(),
            content_type: "application/octet-stream",
            content_encoding: None,
            cache_control: "immutable",
        }));
        let reader = Verifier::from_data(data!(Verifier {
            script: temporary.path.join("must-not-run.cjs"),
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        for bytes in [b"right bytes", b"wrong bytes"] {
            let mut calls = 0;
            let expected = fingerprint(&object.local_path).unwrap();
            let result = reader.verify_with_readback(
                &object,
                &expected,
                |downloaded, metadata| {
                    calls += 1;
                    fs::write(downloaded, bytes)?;
                    fs::write(metadata, serde_json::to_vec(&receipt(&object))?)?;
                    Ok(())
                },
                |_| {},
            );
            assert_eq!(calls, 1);
            assert_eq!(result.is_ok(), bytes == b"right bytes");
        }
        assert_eq!(fs::read_dir(&temporary.path).unwrap().count(), 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_unix_publication_is_refused() {
        assert!(require_unix_host(true).is_ok());
        assert!(
            require_unix_host(false)
                .unwrap_err()
                .to_string()
                .contains("requires a Unix host")
        );
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_http_readback_uses_local_server_stubs() {
        run_bounded(
            Command::new("node")
                .arg("--test")
                .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/r2-readback.test.cjs")),
            Duration::from_secs(30),
        )
        .unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r5_changed_diagnostics_accept_only_bounded_ascii_etags() {
        let safe = serde_json::json!({"r2_readback_error":"changed", "body_etag":"\"wrong\"", "stored_etag":"stored"});
        let error = redacted_readback_stderr(&serde_json::to_vec(&safe).unwrap());
        assert!(error.contains("wrong") && error.contains("stored"));
        let controls = serde_json::json!({"r2_readback_error":"changed", "body_etag":"\u{0001}".repeat(128), "stored_etag":"\u{0001}".repeat(128)});
        let error = redacted_readback_stderr(&serde_json::to_vec(&controls).unwrap());
        assert!(error.len() < 2048 && !error.contains('\u{0001}'));
        for field in ["body_etag", "stored_etag"] {
            for unsafe_value in ["x".repeat(129), "non-ASCII é".to_owned()] {
                let mut wrong = safe.clone();
                wrong[field] = unsafe_value.into();
                assert!(
                    redacted_readback_stderr(&serde_json::to_vec(&wrong).unwrap())
                        .contains("redacted")
                );
            }
        }
        let mut wrong = safe;
        wrong["Authorization"] = "FAKE_TOKEN".into();
        let error = redacted_readback_stderr(&serde_json::to_vec(&wrong).unwrap());
        assert!(error.contains("redacted") && !error.contains("FAKE_TOKEN"));
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r4_stderr_surfaces_known_errors_and_redacts_untrusted_text() {
        let root = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap());
        let temporary = Readback::new(&root).unwrap();
        let script = temporary.path.join("failure.cjs");
        fs::write(
            &script,
            r#"console.error(JSON.stringify({r2_readback_error:'changed',body_etag:'W/"00000000000000000000000000000000"',stored_etag:'3503455e520328d564c05beb0020a517'})); process.exitCode=1;"#,
        )
        .unwrap();
        let error =
            run_readback(Command::new("node").arg(&script), Duration::from_secs(5)).unwrap_err();
        assert!(format!("{error:#}").contains("R2 object changed between metadata and body reads"));
        assert!(format!("{error:#}").contains("W/\\\"00000000000000000000000000000000\\\""));
        assert!(format!("{error:#}").contains("3503455e520328d564c05beb0020a517"));
        for text in [
            "Authorization: Bearer FAKE_TOKEN; X-Auth-Key: FAKE_KEY".to_owned(),
            format!("{{\"r2_readback_error\":\"changed\",\"Authorization\":\"FAKE_TOKEN\"}}"),
            "FAKE_KEY".repeat(10_000),
        ] {
            fs::write(
                &script,
                format!(
                    "process.stderr.write({}); process.exitCode=1;",
                    serde_json::to_string(&text).unwrap()
                ),
            )
            .unwrap();
            let error = run_readback(Command::new("node").arg(&script), Duration::from_secs(5))
                .unwrap_err();
            let error = format!("{error:#}");
            assert!(error.contains("redacted") && error.len() < 512);
            for forbidden in ["FAKE_TOKEN", "FAKE_KEY", "Authorization", "X-Auth-Key"] {
                assert!(!error.contains(forbidden));
            }
        }
        fs::write(
            &script,
            "process.stderr.write('FAKE_TOKEN'); setTimeout(() => {}, 30000);",
        )
        .unwrap();
        let error = run_readback(
            Command::new("node").arg(&script),
            Duration::from_millis(100),
        )
        .unwrap_err();
        let text = format!("{error:#}");
        assert!(
            text.contains("timed out") && text.contains("redacted") && !text.contains("FAKE_TOKEN")
        );
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_child_environment_blocks_fake_token_logs() {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        fs::create_dir_all(&root).unwrap();
        let temporary = Readback::new(&root).unwrap();
        let mut command = Command::new("node");
        command
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/wrangler-4.149.0-logging.cjs"),
            )
            .arg("--stub-upload")
            .env("TEST_OUTPUT_DIR", &temporary.path)
            .env("WRANGLER_WRITE_LOGS", "true")
            .env("WRANGLER_LOG", "debug")
            .env("WRANGLER_LOG_SANITIZE", "false")
            .env("WRANGLER_SEND_METRICS", "true")
            .env("CI", "false")
            .env("NODE_V8_COVERAGE", "hostile logging setting");
        let removed = [
            "WRANGLER_LOG_PATH",
            "WRANGLER_OUTPUT_FILE_PATH",
            "WRANGLER_OUTPUT_FILE_DIRECTORY",
            "DEBUG",
            "NODE_DEBUG",
            "NODE_DEBUG_NATIVE",
            "NODE_OPTIONS",
            "SSLKEYLOGFILE",
            "LOG_STREAM",
            "LOG_TOKENS",
        ];
        for name in removed {
            command.env(name, "hostile logging setting");
        }
        configure_child_environment(&mut command, "0123456789abcdef0123456789abcdef");
        let output = command.output().unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(output.stdout, b"upload stub completed\n");
        assert!(output.stderr.is_empty());
        let token = b"FAKE_PUBLICATION_TOKEN_1068_REGRESSION_ONLY";
        assert!(
            !output
                .stdout
                .windows(token.len())
                .any(|bytes| bytes == token)
        );
        assert!(
            !output
                .stderr
                .windows(token.len())
                .any(|bytes| bytes == token)
        );
        for entry in walkdir::WalkDir::new(&temporary.path) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let bytes = fs::read(entry.path()).unwrap();
                assert!(
                    !bytes.windows(token.len()).any(|bytes| bytes == token),
                    "{}",
                    entry.path().display()
                );
            }
        }
        let observed: serde_json::Value =
            serde_json::from_slice(&fs::read(temporary.path.join("environment.json")).unwrap())
                .unwrap();
        for (name, value) in [
            ("WRANGLER_WRITE_LOGS", "false"),
            ("WRANGLER_LOG", "log"),
            ("WRANGLER_LOG_SANITIZE", "true"),
            ("WRANGLER_SEND_METRICS", "false"),
            ("CI", "true"),
            ("NODE_V8_COVERAGE", ""),
        ] {
            assert_eq!(observed[name], value);
        }
        for name in removed {
            assert!(observed[name].is_null(), "{name}");
        }
    }

    #[cfg(unix)]
    #[requires(pid > 0)]
    #[ensures(true)]
    fn process_has_exited(pid: u32) -> bool {
        let output = Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        if output.stdout.is_empty() {
            assert!(output.stderr.is_empty(), "{:?}", output);
            return true;
        }
        assert!(output.status.success(), "{:?}", output);
        // A zombie already exited, but its new parent has not reaped it yet.
        String::from_utf8(output.stdout)
            .unwrap()
            .trim_start()
            .starts_with('Z')
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn timeout_stops_the_child_and_its_grandchild() {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        fs::create_dir_all(&root).unwrap();
        let temporary = Readback::new(&root).unwrap();
        let pids_file = temporary.path.join("pids");
        let error = run_bounded(
            Command::new("sh")
                .args([
                    "-c",
                    "sleep 5 & grandchild=$!; printf '%s %s' \"$$\" \"$grandchild\" > \"$1\"; wait",
                    "r2-tree-stub",
                ])
                .arg(&pids_file),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(error.to_string().contains("timed out"));
        let text = fs::read_to_string(&pids_file).unwrap();
        let pids: Vec<u32> = text
            .split_whitespace()
            .map(|pid| pid.parse().unwrap())
            .collect();
        let [child, grandchild] = pids.as_slice() else {
            panic!("The stub did not record both process IDs")
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while !(process_has_exited(*child) && process_has_exited(*grandchild))
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        let child_exited = process_has_exited(*child);
        let grandchild_exited = process_has_exited(*grandchild);
        if !grandchild_exited {
            // Clean up only the process group that this test starts.
            unsafe {
                if libc::getpgid(*grandchild as i32) == *child as i32 {
                    libc::kill(-(*child as i32), libc::SIGKILL);
                }
            }
        }
        assert!(child_exited, "The child survived the timeout");
        assert!(grandchild_exited, "The grandchild survived the timeout");
    }

    #[cfg(unix)]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stub_uploader_timeout_is_retried_and_reaped() {
        let mut calls = 0;
        let error = retry(
            2,
            Duration::ZERO,
            || {
                calls += 1;
                run_bounded(
                    Command::new("sh").args(["-c", "sleep 30"]),
                    Duration::from_millis(40),
                )
            },
            |_| {},
        )
        .unwrap_err();
        assert_eq!(calls, 2);
        assert!(error.to_string().contains("timed out"));
    }
}
