use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::R2UploadObject;
#[cfg(test)]
use crate::R2UploadObjectData;

const WRANGLER_VERSION: &str = "4.149.0";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(180);
const ATTEMPTS: usize = 3;
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[invariant(account.len() == 32 && account.bytes().all(|byte| byte.is_ascii_hexdigit()))]
pub(super) struct Publisher {
    script: PathBuf,
    account: String,
    scratch: PathBuf,
}

impl Publisher {
    #[requires(true)]
    #[ensures(ret.as_ref().map_or(true, |publisher| !publisher.account.is_empty()))]
    pub(super) fn from_env() -> Result<Self> {
        require_unix_host(cfg!(unix))?;
        let account = std::env::var("CLOUDFLARE_ACCOUNT_ID")
            .context("Set CLOUDFLARE_ACCOUNT_ID before publishing")?;
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
        Ok(Self::from_data(data!(Publisher {
            script: script,
            account: account,
            scratch: scratch,
        })))
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn put(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        self.put_with_readback(bucket, object, |downloaded, metadata| {
            self.read_existing(bucket, object, downloaded, metadata)
        })
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn verify_existing(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        self.verify_with_readback(object, |downloaded, metadata| {
            self.read_existing(bucket, object, downloaded, metadata)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn verify_with_readback(
        &self,
        object: &R2UploadObject,
        readback: impl FnOnce(&Path, &Path) -> Result<()>,
    ) -> Result<()> {
        let expected = fingerprint(&object.local_path)?;
        let temporary = Readback::new(&self.scratch)?;
        let downloaded = temporary.path.join("object");
        let metadata = temporary.path.join("metadata.json");
        readback(&downloaded, &metadata)?;
        verify_readback(
            object,
            &expected,
            &downloaded,
            &serde_json::from_slice(&fs::read(&metadata)?)?,
        )
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
            .arg(concat!(include_str!("r2_readback.cjs"), "\nmain(process.argv.slice(1)).catch(error => { console.error(error.message); process.exitCode = 1; });"))
            .arg(&self.script).arg(&self.account).arg(bucket.trim())
            .arg(&object.object_key).arg(object.content_encoding.unwrap_or("identity"))
            .arg(downloaded).arg(metadata);
        run_bounded(&mut get, COMMAND_TIMEOUT).context("R2 read-back failed")
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
        let temporary = Readback::new(&self.scratch)?;
        let downloaded = temporary.path.join("object");
        let metadata_path = temporary.path.join("metadata.json");
        retry(
            ATTEMPTS,
            Duration::from_secs(2),
            || {
                let mut put = self.command();
                put.args(["r2", "object", "put", &target, "--file"])
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
                if downloaded.exists() {
                    fs::remove_file(&downloaded)?;
                }
                if metadata_path.exists() {
                    fs::remove_file(&metadata_path)?;
                }
                readback(&downloaded, &metadata_path)?;
                let metadata: ReadbackMetadata =
                    serde_json::from_slice(&fs::read(&metadata_path)?)?;
                verify_readback(object, &expected, &downloaded, &metadata)
            },
            std::thread::sleep,
        )
        .with_context(|| format!("R2 object `{target}` failed after {ATTEMPTS} attempts"))
    }

    #[requires(true)]
    #[ensures(true)]
    fn command(&self) -> Command {
        let mut command = self.node_command();
        command.arg(&self.script);
        command
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

#[requires(true)]
#[ensures(ret.is_ok() == is_unix)]
fn require_unix_host(is_unix: bool) -> Result<()> {
    if !is_unix {
        bail!("R2 publication requires a Unix host. Use Linux or macOS.");
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
    stored_size: u64,
    stored_encoding: Option<String>,
    body_encoding: Option<String>,
}

#[requires(true)]
#[ensures(true)]
fn verify_readback(
    object: &R2UploadObject,
    expected: &Fingerprint,
    downloaded: &Path,
    metadata: &ReadbackMetadata,
) -> Result<()> {
    if metadata.stored_size != expected.size
        || metadata.stored_encoding.as_deref() != object.content_encoding
    {
        bail!("R2 stored size or Content-Encoding differs from the upload");
    }
    let actual = fingerprint(downloaded)?;
    match (object.content_encoding, metadata.body_encoding.as_deref()) {
        (None, None) | (Some("br"), Some("br")) => compare_fingerprint(expected, &actual),
        (Some("br"), None | Some("identity")) => {
            bail!("R2 returned a decoded Brotli body. Exact stored bytes cannot be verified")
        }
        _ => bail!("R2 returned an unexpected Content-Encoding"),
    }
}

#[invariant(true)]
#[derive(Debug, PartialEq, Eq)]
struct Fingerprint {
    size: u64,
    digest: [u8; 32],
}

#[requires(true)]
#[ensures(true)]
fn fingerprint(path: &Path) -> Result<Fingerprint> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut size = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size += count as u64;
    }
    Ok(Fingerprint {
        size,
        digest: digest.finalize().into(),
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
        };
        assert!(
            compare_fingerprint(
                &expected,
                &Fingerprint {
                    size: 2,
                    digest: [2; 32]
                }
            )
            .is_err()
        );
        assert!(
            compare_fingerprint(
                &expected,
                &Fingerprint {
                    size: 3,
                    digest: [1; 32]
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
        let publisher = Publisher::from_data(data!(Publisher {
            script: script,
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        publisher
            .put_with_readback("stub-bucket", &object, |downloaded, metadata| {
                fs::copy(temporary.path.join("remote"), downloaded)?;
                fs::write(
                    metadata,
                    r#"{"stored_size":18,"stored_encoding":null,"body_encoding":null}"#,
                )?;
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
        let publisher = Publisher::from_data(data!(Publisher {
            script: script,
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        let error = publisher
            .put_with_readback("stub-bucket", &object, |downloaded, metadata| {
                fs::write(downloaded, b"wrong bytes")?;
                fs::write(
                    metadata,
                    r#"{"stored_size":11,"stored_encoding":null,"body_encoding":null}"#,
                )?;
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
    fn brotli_success_requires_the_exact_encoded_stream() {
        const STREAM_A: &[u8] = b"\x83\x0b\x80validated original bytes\x03";
        const STREAM_B: &[u8] = b"\x85\x0b\x80validated original bytes\x03";
        assert_eq!(STREAM_A.len(), STREAM_B.len());
        assert_ne!(STREAM_A, STREAM_B);
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target"));
        fs::create_dir_all(&root).unwrap();
        let temporary = Readback::new(&root).unwrap();
        let original = temporary.path.join("vectors.f16");
        fs::write(&original, b"validated original bytes").unwrap();
        let sibling = crate::brotli_sidecar_path(&original).unwrap();
        for stream in [STREAM_A, STREAM_B] {
            fs::write(&sibling, stream).unwrap();
            jbotci_embeddings::validate_brotli_sibling(&original).unwrap();
        }
        fs::write(&sibling, STREAM_A).unwrap();
        let object =
            crate::r2_upload_object_for_key(&temporary.path, "stub-prefix", "vectors.f16").unwrap();
        let expected = fingerprint(&object.local_path).unwrap();
        let downloaded = temporary.path.join("download");
        let mut metadata = ReadbackMetadata {
            stored_size: expected.size,
            stored_encoding: Some("br".to_owned()),
            body_encoding: Some("br".to_owned()),
        };
        fs::write(&downloaded, STREAM_A).unwrap();
        verify_readback(&object, &expected, &downloaded, &metadata).unwrap();
        fs::write(&downloaded, STREAM_B).unwrap();
        assert!(
            verify_readback(&object, &expected, &downloaded, &metadata)
                .unwrap_err()
                .to_string()
                .contains("SHA-256")
        );
        fs::copy(&original, &downloaded).unwrap();
        metadata.body_encoding = None;
        assert!(
            verify_readback(&object, &expected, &downloaded, &metadata)
                .unwrap_err()
                .to_string()
                .contains("decoded Brotli")
        );
        fs::write(&downloaded, STREAM_A).unwrap();
        metadata.body_encoding = Some("br".to_owned());
        metadata.stored_encoding = None;
        assert!(verify_readback(&object, &expected, &downloaded, &metadata).is_err());
        metadata.stored_encoding = Some("br".to_owned());
        metadata.stored_size += 1;
        assert!(verify_readback(&object, &expected, &downloaded, &metadata).is_err());
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
        let publisher = Publisher::from_data(data!(Publisher {
            script: temporary.path.join("must-not-run.cjs"),
            account: "0123456789abcdef0123456789abcdef".to_owned(),
            scratch: temporary.path.clone(),
        }));
        for bytes in [b"right bytes", b"wrong bytes"] {
            let mut calls = 0;
            let result = publisher.verify_with_readback(&object, |downloaded, metadata| {
                calls += 1;
                fs::write(downloaded, bytes)?;
                fs::write(
                    metadata,
                    r#"{"stored_size":11,"stored_encoding":null,"body_encoding":null}"#,
                )?;
                Ok(())
            });
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
