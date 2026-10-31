//! Live metadata proofs for optional data-object reuse. No persisted receipt authorizes a skip.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use clap::Args;
use md5::{Digest, Md5};
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{R2_UPLOAD_PARALLELISM, R2UploadObject, normalize_r2_prefix, r2_publish::Publisher};

#[invariant(true)]
#[derive(Debug, Args, Clone, Copy, Default)]
pub(super) struct Options {
    /// Reuse data only when fresh R2 metadata proves that the uploaded bytes match.
    #[arg(long)]
    skip_unchanged: bool,
    /// Reproduce the skipped-object sample for the same sorted publication plan.
    #[arg(long, requires = "skip_unchanged")]
    skip_sample_seed: Option<u64>,
}

#[invariant(true)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(super) struct LiveMetadata {
    pub(super) key: String,
    size: Option<u64>,
    etag: Option<String>,
    http_metadata: Option<HttpMetadata>,
}

#[invariant(true)]
#[derive(Debug, Deserialize, Serialize, Clone)]
struct HttpMetadata {
    #[serde(rename = "contentType")]
    content_type: Option<String>,
    #[serde(rename = "cacheControl")]
    cache_control: Option<String>,
    #[serde(rename = "contentEncoding")]
    content_encoding: Option<String>,
}

#[invariant(true)]
#[derive(Debug, PartialEq, Eq)]
struct LocalProof {
    size: u64,
    md5: [u8; 16],
}

#[contract_trait]
pub(super) trait Client: Sync {
    #[requires(!bucket.is_empty() && !prefix.is_empty())]
    #[ensures(true)]
    fn metadata(&self, bucket: &str, prefix: &str) -> Result<BTreeMap<String, LiveMetadata>>;

    #[requires(!bucket.is_empty())]
    #[ensures(true)]
    fn put(&self, bucket: &str, object: &R2UploadObject) -> Result<()>;

    #[requires(!bucket.is_empty())]
    #[ensures(true)]
    fn verify(&self, bucket: &str, object: &R2UploadObject) -> Result<()>;
}

#[contract_trait]
impl Client for Publisher {
    fn metadata(&self, bucket: &str, prefix: &str) -> Result<BTreeMap<String, LiveMetadata>> {
        self.live_metadata(bucket, prefix)
    }

    fn put(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        Publisher::put(self, bucket, object)
    }

    fn verify(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        self.verify_existing(bucket, object)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Counts {
    uploaded: usize,
    skipped: usize,
}

#[invariant(true)]
#[derive(Clone, Copy)]
enum Outcome {
    Uploaded,
    Skipped,
}

#[invariant(!prefix.is_empty())]
#[invariant(!object.object_key.is_empty() && crate::r2_upload_boundary_object(&object.object_key))]
#[derive(Clone)]
struct Boundary {
    prefix: String,
    object: R2UploadObject,
}

#[invariant(true)]
pub(super) struct Session<C: Client = Publisher> {
    publisher: C,
    seed: Option<u64>,
    pending: Vec<Boundary>,
    totals: Mutex<BTreeMap<String, BTreeMap<String, Counts>>>,
}

impl Session<Publisher> {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|session| session.seed.is_some() == options.skip_unchanged) || ret.is_err())]
    pub(super) fn from_env(options: Options) -> Result<Self> {
        // Keep the existing Unix, account, and pinned-tool preflight before any build.
        let publisher = Publisher::from_env()?;
        let seed = options
            .skip_unchanged
            .then(|| options.skip_sample_seed.unwrap_or_else(rand::random));
        Ok(Self {
            publisher,
            seed,
            pending: Vec::new(),
            totals: Mutex::new(BTreeMap::new()),
        })
    }
}

impl<C: Client> Session<C> {
    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn batch(
        &mut self,
        bucket: &str,
        prefix: &str,
        objects: &[R2UploadObject],
    ) -> Result<()> {
        let prefix = normalize_r2_prefix(prefix)?;
        publish_data(
            &self.publisher,
            bucket.trim(),
            &prefix,
            objects,
            self.seed,
            |object, outcome| self.record(&prefix, object, outcome),
        )?;
        self.pending.extend(
            objects
                .iter()
                .filter(|object| crate::r2_upload_boundary_object(&object.object_key))
                .map(|object| {
                    new!(Boundary {
                        prefix: prefix.clone(),
                        object: object.clone()
                    })
                }),
        );
        Ok(())
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn put(&mut self, bucket: &str, object: &R2UploadObject) -> Result<()> {
        if !crate::r2_upload_boundary_object(&object.object_key) {
            bail!("Only a publication boundary can finish an R2 session");
        }
        let prefix = object
            .object_key
            .rsplit_once('/')
            .map_or("(root)", |(parent, _)| parent);
        self.pending.push(new!(Boundary {
            prefix: prefix.to_owned(),
            object: object.clone(),
        }));
        self.finish(bucket)
    }

    #[requires(!bucket.trim().is_empty())]
    #[ensures(true)]
    pub(super) fn finish(&mut self, bucket: &str) -> Result<()> {
        let pending = std::mem::take(&mut self.pending);
        publish_boundaries(
            &self.publisher,
            bucket,
            &pending,
            |prefix, object, outcome| {
                self.record(prefix, object, outcome);
            },
        )
    }

    #[requires(!prefix.is_empty())]
    #[ensures(true)]
    fn record(&self, prefix: &str, object: &R2UploadObject, outcome: Outcome) {
        let corpus = corpus_group(prefix, &object.object_key);
        let mut totals = self.totals.lock().expect("publication totals lock");
        let counts = totals
            .entry(prefix.to_owned())
            .or_default()
            .entry(corpus.to_owned())
            .or_default();
        match outcome {
            Outcome::Uploaded => counts.uploaded += 1,
            Outcome::Skipped => counts.skipped += 1,
        }
    }
}

impl<C: Client> Drop for Session<C> {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        // The session also prints partial totals when a failed comparison stops publication.
        let Ok(totals) = self.totals.lock() else {
            return;
        };
        for (prefix, corpora) in totals.iter() {
            let mut total = Counts::default();
            for (corpus, counts) in corpora {
                eprintln!(
                    "{}",
                    serde_json::json!({"r2_summary": "corpus", "prefix": prefix,
                    "corpus": corpus, "uploaded_verified": counts.uploaded, "skipped": counts.skipped})
                );
                total.uploaded += counts.uploaded;
                total.skipped += counts.skipped;
            }
            eprintln!(
                "{}",
                serde_json::json!({"r2_summary": "prefix", "prefix": prefix,
                "uploaded_verified": total.uploaded, "skipped": total.skipped})
            );
        }
    }
}

#[requires(!prefix.is_empty())]
#[ensures(!ret.is_empty())]
fn corpus_group<'a>(prefix: &str, key: &'a str) -> &'a str {
    let relative = key
        .strip_prefix(prefix)
        .unwrap_or(key)
        .trim_start_matches('/');
    // Match the pack layouts so a model name cannot become a corpus label.
    let parts = relative.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [
            "models",
            _,
            "spaces",
            _,
            "packs",
            _,
            "corpora",
            corpus,
            _,
            ..,
        ]
        | ["models", _, "packs", _, "corpora", corpus, _, ..]
        | ["corpora", corpus, _, ..]
            if !corpus.is_empty() =>
        {
            corpus
        }
        _ => "shared",
    }
}

#[requires(true)]
#[ensures(true)]
fn local_proof(path: &Path) -> Result<LocalProof> {
    let mut file = fs::File::open(path)?;
    let mut digest = Md5::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size = size
            .checked_add(count as u64)
            .context("upload file size overflow")?;
    }
    Ok(LocalProof {
        size,
        md5: digest.finalize().into(),
    })
}

#[requires(true)]
#[ensures(ret.is_some() == (etag.len() == 32 && etag.bytes().all(|byte| byte.is_ascii_hexdigit())))]
fn plain_md5(etag: &str) -> Option<[u8; 16]> {
    if etag.len() != 32 || !etag.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0; 16];
    for (position, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&etag[position * 2..position * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|proof| live.size == Some(proof.size)))]
fn metadata_proof(live: &LiveMetadata, object: &R2UploadObject) -> Option<LocalProof> {
    let http = live.http_metadata.as_ref()?;
    // Missing fields never prove equivalence, even when the intended encoding is absent.
    if live.key != object.object_key
        || http.content_type.as_deref()? != object.content_type
        || http.cache_control.as_deref()? != object.cache_control
        || Some(http.content_encoding.as_deref()?) != object.content_encoding
    {
        return None;
    }
    Some(LocalProof {
        size: live.size?,
        md5: plain_md5(live.etag.as_deref()?)?,
    })
}

#[requires(true)]
#[ensures(ret <= count)]
#[ensures(count == 0 || ret > 0)]
#[ensures(ret == count || (ret >= 20 && ret >= count.div_ceil(100)))]
fn sample_size(count: usize) -> usize {
    count.div_ceil(100).max(20).min(count)
}

#[requires(!bucket.is_empty() && !prefix.is_empty())]
#[ensures(true)]
fn publish_data(
    client: &impl Client,
    bucket: &str,
    prefix: &str,
    objects: &[R2UploadObject],
    seed: Option<u64>,
    record: impl Fn(&R2UploadObject, Outcome) + Sync,
) -> Result<()> {
    let scoped_prefix = format!("{prefix}/");
    if objects
        .iter()
        .any(|object| !object.object_key.starts_with(&scoped_prefix))
    {
        bail!("R2 publication plan contains an object outside its prefix");
    }
    let mut keys = BTreeSet::new();
    if objects
        .iter()
        .any(|object| !keys.insert(object.object_key.as_str()))
    {
        bail!("R2 publication plan contains duplicate object keys");
    }
    let live = if seed.is_some()
        && objects
            .iter()
            .any(|object| !crate::r2_upload_boundary_object(&object.object_key))
    {
        client.metadata(bucket, &scoped_prefix).unwrap_or_else(|_| {
            eprintln!("R2 live metadata is unavailable. Upload every data object in this prefix.");
            BTreeMap::new()
        })
    } else {
        BTreeMap::new()
    };
    let mut skipped = Vec::new();
    let mut uploads = Vec::new();
    for object in objects
        .iter()
        .filter(|object| !crate::r2_upload_boundary_object(&object.object_key))
    {
        if let Some(metadata) = seed.and_then(|_| live.get(&object.object_key))
            && metadata_proof(metadata, object).is_some_and(|expected| {
                local_proof(&object.local_path).is_ok_and(|actual| actual == expected)
            })
        {
            eprintln!(
                "{}",
                serde_json::json!({"r2_object": &object.object_key, "status": "skipped",
                "prefix": prefix, "corpus": corpus_group(prefix, &object.object_key), "matching_metadata": metadata})
            );
            record(object, Outcome::Skipped);
            skipped.push(object);
        } else {
            uploads.push(object);
        }
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(R2_UPLOAD_PARALLELISM)
        .build()?;
    pool.install(|| {
        uploads.par_iter().try_for_each(|object| {
            client.put(bucket, object)?;
            uploaded(prefix, object);
            record(object, Outcome::Uploaded);
            Ok::<_, anyhow::Error>(())
        })
    })?;
    if let Some(seed) = seed {
        // Sort before shuffling so filesystem and listing order cannot change a seeded sample.
        skipped.sort_by(|left, right| left.object_key.cmp(&right.object_key));
        let count = sample_size(skipped.len());
        skipped.shuffle(&mut StdRng::seed_from_u64(seed));
        eprintln!(
            "{}",
            serde_json::json!({"r2_sample_seed": seed, "prefix": prefix,
            "skipped": skipped.len(), "sample_size": count})
        );
        for object in skipped.iter().take(count) {
            client.verify(bucket, object).with_context(|| {
                format!(
                    "R2 skipped-object sample failed for `{}` with seed {seed}",
                    object.object_key
                )
            })?;
            eprintln!(
                "{}",
                serde_json::json!({"r2_object": &object.object_key, "status": "sample-verified", "seed": seed})
            );
        }
    }
    Ok(())
}

#[requires(!bucket.is_empty())]
#[ensures(true)]
fn publish_boundaries(
    client: &impl Client,
    bucket: &str,
    objects: &[Boundary],
    record: impl Fn(&str, &R2UploadObject, Outcome),
) -> Result<()> {
    // Defer every boundary until all prefix batches and samples pass, including later model prefixes.
    for catalog in [false, true] {
        for boundary in objects.iter().filter(|boundary| {
            (boundary.object.object_key.rsplit('/').next() == Some("catalog.json")) == catalog
        }) {
            client.put(bucket, &boundary.object)?;
            uploaded(&boundary.prefix, &boundary.object);
            record(&boundary.prefix, &boundary.object, Outcome::Uploaded);
        }
    }
    Ok(())
}

#[requires(!prefix.is_empty())]
#[ensures(true)]
fn uploaded(prefix: &str, object: &R2UploadObject) {
    eprintln!(
        "{}",
        serde_json::json!({"r2_object": &object.object_key, "status": "uploaded-verified",
        "prefix": prefix, "corpus": corpus_group(prefix, &object.object_key)})
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::R2UploadObjectData;
    use bityzba::data;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    static NEXT_TEST: AtomicU64 = AtomicU64::new(0);

    #[invariant(true)]
    struct Files {
        root: std::path::PathBuf,
    }
    impl Files {
        #[requires(true)]
        #[ensures(ret.root.is_dir())]
        fn new() -> Self {
            let target = std::path::PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap());
            let root = target.join(format!(
                "r2-skip-{}-{}",
                std::process::id(),
                NEXT_TEST.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        #[requires(true)]
        #[ensures(true)]
        fn object(&self, key: &str) -> R2UploadObject {
            let source = self.root.join("payload.br");
            fs::write(&source, b"\x83\x0b\x80validated original bytes\x03").unwrap();
            R2UploadObject::from_data(data!(R2UploadObject {
                local_path: source,
                object_key: key.to_owned(),
                content_type: "application/octet-stream",
                content_encoding: Some("br"),
                cache_control: "public, max-age=31536000, immutable",
            }))
        }
    }
    impl Drop for Files {
        #[requires(true)]
        #[ensures(true)]
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[invariant(true)]
    #[derive(Default)]
    struct Stub {
        metadata: BTreeMap<String, LiveMetadata>,
        metadata_error: bool,
        calls: Mutex<Vec<String>>,
        fail_sample: AtomicBool,
    }

    #[contract_trait]
    impl Client for Stub {
        fn metadata(&self, _bucket: &str, prefix: &str) -> Result<BTreeMap<String, LiveMetadata>> {
            self.calls.lock().unwrap().push(format!("list:{prefix}"));
            if self.metadata_error {
                bail!("stub metadata error");
            }
            Ok(self.metadata.clone())
        }
        fn put(&self, _bucket: &str, object: &R2UploadObject) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("put:{}", object.object_key));
            Ok(())
        }
        fn verify(&self, _bucket: &str, object: &R2UploadObject) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("sample:{}", object.object_key));
            if self.fail_sample.load(Ordering::Relaxed) {
                bail!("stub sampled body mismatch");
            }
            Ok(())
        }
    }

    #[contract_trait]
    impl Client for &Stub {
        fn metadata(&self, bucket: &str, prefix: &str) -> Result<BTreeMap<String, LiveMetadata>> {
            Client::metadata(*self, bucket, prefix)
        }
        fn put(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
            Client::put(*self, bucket, object)
        }
        fn verify(&self, bucket: &str, object: &R2UploadObject) -> Result<()> {
            Client::verify(*self, bucket, object)
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn metadata(object: &R2UploadObject) -> LiveMetadata {
        let local = local_proof(&object.local_path).unwrap();
        let digest = local
            .md5
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        LiveMetadata {
            key: object.object_key.clone(),
            size: Some(local.size),
            etag: Some(digest),
            http_metadata: Some(HttpMetadata {
                content_type: Some(object.content_type.to_owned()),
                cache_control: Some(object.cache_control.to_owned()),
                content_encoding: object.content_encoding.map(str::to_owned),
            }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn run(stub: &Stub, objects: &[R2UploadObject], seed: Option<u64>) -> Result<Counts> {
        let mut session = Session {
            publisher: stub,
            seed,
            pending: Vec::new(),
            totals: Mutex::new(BTreeMap::new()),
        };
        session.batch("stub-bucket", "packs", objects)?;
        session.finish("stub-bucket")?;
        let mut total = Counts::default();
        for corpora in session.totals.lock().unwrap().values() {
            for counts in corpora.values() {
                total.uploaded += counts.uploaded;
                total.skipped += counts.skipped;
            }
        }
        Ok(total)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn matching_data_is_actually_skipped_sampled_and_boundaries_are_uploaded_in_order() {
        let files = Files::new();
        let objects = [
            files.object("packs/catalog.json"),
            files.object("packs/manifest.json"),
            files.object("packs/corpora/vlacku-en/vectors.f16"),
        ];
        let stub = Stub {
            metadata: objects
                .iter()
                .map(|object| (object.object_key.clone(), metadata(object)))
                .collect(),
            ..Stub::default()
        };
        assert_eq!(
            run(&stub, &objects, Some(17)).unwrap(),
            Counts {
                uploaded: 2,
                skipped: 1
            }
        );
        assert_eq!(
            *stub.calls.lock().unwrap(),
            [
                "list:packs/",
                "sample:packs/corpora/vlacku-en/vectors.f16",
                "put:packs/manifest.json",
                "put:packs/catalog.json"
            ]
        );
        assert_eq!(corpus_group("packs", &objects[2].object_key), "vlacku-en");
        assert_eq!(corpus_group("packs", &objects[0].object_key), "shared");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_incomplete_or_different_proof_uploads_instead() {
        let files = Files::new();
        let object = files.object("packs/data.f16");
        let original = metadata(&object);
        let mut variants = Vec::new();
        let mut wrong = original.clone();
        wrong.size = Some(original.size.unwrap() + 1);
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.etag = Some("00000000000000000000000000000000".to_owned());
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.etag.as_mut().unwrap().push_str("-2");
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.etag = None;
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.size = None;
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata = None;
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata.as_mut().unwrap().content_type = None;
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata.as_mut().unwrap().cache_control = None;
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata.as_mut().unwrap().content_encoding = None;
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata.as_mut().unwrap().content_type = Some("text/plain".to_owned());
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata.as_mut().unwrap().cache_control = Some("no-cache".to_owned());
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.http_metadata.as_mut().unwrap().content_encoding = Some("gzip".to_owned());
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.etag = Some(format!("{:x}", Md5::digest(b"validated original bytes")));
        variants.push(wrong);
        let mut wrong = original.clone();
        wrong.etag = Some(format!("\"{}\"", original.etag.unwrap()));
        variants.push(wrong);
        for (case, wrong) in variants.into_iter().enumerate() {
            let stub = Stub {
                metadata: BTreeMap::from([(object.object_key.clone(), wrong)]),
                ..Stub::default()
            };
            assert_eq!(
                run(&stub, std::slice::from_ref(&object), Some(17)).unwrap(),
                Counts {
                    uploaded: 1,
                    skipped: 0
                },
                "case {case}"
            );
            assert_eq!(
                *stub.calls.lock().unwrap(),
                ["list:packs/", "put:packs/data.f16"],
                "case {case}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn default_mode_and_missing_or_failed_metadata_preserve_uploads() {
        let files = Files::new();
        let object = files.object("packs/data.f16");
        let stub = Stub {
            metadata: BTreeMap::from([(object.object_key.clone(), metadata(&object))]),
            ..Stub::default()
        };
        assert_eq!(
            run(&stub, std::slice::from_ref(&object), None).unwrap(),
            Counts {
                uploaded: 1,
                skipped: 0
            }
        );
        assert_eq!(*stub.calls.lock().unwrap(), ["put:packs/data.f16"]);
        for metadata_error in [false, true] {
            let stub = Stub {
                metadata_error,
                ..Stub::default()
            };
            assert_eq!(
                run(&stub, std::slice::from_ref(&object), Some(0)).unwrap(),
                Counts {
                    uploaded: 1,
                    skipped: 0
                }
            );
            assert_eq!(
                *stub.calls.lock().unwrap(),
                ["list:packs/", "put:packs/data.f16"]
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn all_prefix_data_precedes_every_manifest_and_catalog() {
        let files = Files::new();
        let first = [
            files.object("packs/data.f16"),
            files.object("packs/manifest.json"),
        ];
        let second = [
            files.object("models/data.f16"),
            files.object("models/manifest.json"),
        ];
        let stub = Stub {
            metadata: BTreeMap::from([(first[0].object_key.clone(), metadata(&first[0]))]),
            ..Stub::default()
        };
        let mut session = Session {
            publisher: &stub,
            seed: Some(17),
            pending: Vec::new(),
            totals: Mutex::new(BTreeMap::new()),
        };
        session.batch("stub-bucket", "packs", &first).unwrap();
        assert_eq!(
            *stub.calls.lock().unwrap(),
            ["list:packs/", "sample:packs/data.f16"]
        );
        session.batch("stub-bucket", "models", &second).unwrap();
        session
            .put("stub-bucket", &files.object("packs/catalog.json"))
            .unwrap();
        assert_eq!(
            *stub.calls.lock().unwrap(),
            [
                "list:packs/",
                "sample:packs/data.f16",
                "list:models/",
                "put:models/data.f16",
                "put:packs/manifest.json",
                "put:models/manifest.json",
                "put:packs/catalog.json"
            ]
        );
        let totals = session.totals.lock().unwrap();
        assert_eq!(
            totals.keys().map(String::as_str).collect::<Vec<_>>(),
            ["models", "packs"]
        );
        assert_eq!(
            totals["models"]["shared"],
            Counts {
                uploaded: 2,
                skipped: 0
            }
        );
        assert_eq!(
            totals["packs"]["shared"],
            Counts {
                uploaded: 2,
                skipped: 1
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_later_prefix_sample_failure_leaves_earlier_boundaries_unpublished() {
        let files = Files::new();
        let first = [
            files.object("packs/data.f16"),
            files.object("packs/manifest.json"),
        ];
        let second = [
            files.object("models/data.f16"),
            files.object("models/manifest.json"),
        ];
        let stub = Stub {
            metadata: [&first[0], &second[0]]
                .into_iter()
                .map(|object| (object.object_key.clone(), metadata(object)))
                .collect(),
            ..Stub::default()
        };
        {
            let mut session = Session {
                publisher: &stub,
                seed: Some(17),
                pending: Vec::new(),
                totals: Mutex::new(BTreeMap::new()),
            };
            session.batch("stub-bucket", "packs", &first).unwrap();
            stub.fail_sample.store(true, Ordering::Relaxed);
            assert!(session.batch("stub-bucket", "models", &second).is_err());
        }
        assert_eq!(
            *stub.calls.lock().unwrap(),
            [
                "list:packs/",
                "sample:packs/data.f16",
                "list:models/",
                "sample:models/data.f16"
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_or_duplicate_keys_stop_before_any_remote_operation() {
        let files = Files::new();
        let object = files.object("packs/data.f16");
        let stub = Stub::default();
        assert!(run(&stub, &[object.clone(), object], Some(23)).is_err());
        assert!(run(&stub, &[files.object("other-prefix/data.f16")], Some(23)).is_err());
        assert!(stub.calls.lock().unwrap().is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn absent_encoding_on_an_uncompressed_object_is_not_a_complete_proof() {
        let files = Files::new();
        let mut data = files.object("packs/data.f16").into_data();
        data.content_encoding = None;
        let object = R2UploadObject::from_data(data);
        let stub = Stub {
            metadata: BTreeMap::from([(object.object_key.clone(), metadata(&object))]),
            ..Stub::default()
        };
        assert_eq!(
            run(&stub, &[object], Some(23)).unwrap(),
            Counts {
                uploaded: 1,
                skipped: 0
            }
        );
        assert_eq!(
            *stub.calls.lock().unwrap(),
            ["list:packs/", "put:packs/data.f16"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sample_failure_stops_before_any_manifest_or_catalog() {
        let files = Files::new();
        let objects = [
            files.object("packs/data.f16"),
            files.object("packs/manifest.json"),
            files.object("packs/catalog.json"),
        ];
        let stub = Stub {
            metadata: BTreeMap::from([(objects[0].object_key.clone(), metadata(&objects[0]))]),
            fail_sample: AtomicBool::new(true),
            ..Stub::default()
        };
        let error = run(&stub, &objects, Some(19)).unwrap_err();
        assert!(format!("{error:#}").contains("sampled body mismatch"));
        assert_eq!(
            *stub.calls.lock().unwrap(),
            ["list:packs/", "sample:packs/data.f16"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sample_size_is_bounded_and_seed_is_reproducible_across_input_order() {
        assert_eq!(sample_size(0), 0);
        assert_eq!(sample_size(1), 1);
        assert_eq!(sample_size(19), 19);
        assert_eq!(sample_size(20), 20);
        assert_eq!(sample_size(2001), 21);
        let files = Files::new();
        let mut objects = (0..101)
            .map(|index| files.object(&format!("packs/{index:04}.f16")))
            .collect::<Vec<_>>();
        let live = objects
            .iter()
            .map(|object| (object.object_key.clone(), metadata(object)))
            .collect::<BTreeMap<_, _>>();
        let first = Stub {
            metadata: live.clone(),
            ..Stub::default()
        };
        assert_eq!(
            run(&first, &objects, Some(123)).unwrap(),
            Counts {
                uploaded: 0,
                skipped: 101
            }
        );
        objects.reverse();
        let second = Stub {
            metadata: live,
            ..Stub::default()
        };
        run(&second, &objects, Some(123)).unwrap();
        let first_calls = first.calls.lock().unwrap();
        let second_calls = second.calls.lock().unwrap();
        assert_eq!(*first_calls, *second_calls);
        assert_eq!(
            first_calls
                .iter()
                .filter(|call| call.starts_with("sample:"))
                .count(),
            20
        );
        assert!(!first_calls.iter().any(|call| call.starts_with("put:")));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn publication_flags_are_opt_in_and_seed_requires_the_mode() {
        use clap::Parser;
        for command in [
            "publish-web-embeddings-r2",
            "publish-f2llm-webgpu-r2",
            "publish-gguf-embeddings-r2",
        ] {
            assert!(crate::Cli::try_parse_from(["xtask-full", command]).is_ok());
            assert!(
                crate::Cli::try_parse_from([
                    "xtask-full",
                    command,
                    "--skip-unchanged",
                    "--skip-sample-seed",
                    "123"
                ])
                .is_ok()
            );
            assert!(
                crate::Cli::try_parse_from(["xtask-full", command, "--skip-sample-seed", "123"])
                    .is_err()
            );
        }
        assert!(!Options::default().skip_unchanged);
        assert_eq!(
            corpus_group(
                "packs",
                "packs/models/m/spaces/s/packs/p/corpora/cukta/items.json"
            ),
            "cukta"
        );
        assert_eq!(
            corpus_group(
                "packs",
                "packs/models/m/packs/p/corpora/vlacku-en/items.json"
            ),
            "vlacku-en"
        );
        assert_eq!(
            corpus_group("packs", "packs/models/corpora/tensors/shard.bin"),
            "shared"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_md5_requires_exactly_32_hex_digits_and_hashes_encoded_bytes() {
        assert!(plain_md5("d41d8cd98f00b204e9800998ecf8427e").is_some());
        assert!(plain_md5("D41D8CD98F00B204E9800998ECF8427E").is_some());
        for wrong in [
            "",
            "d41d8cd98f00b204e9800998ecf8427e-1",
            "W/\"d41d8cd98f00b204e9800998ecf8427e\"",
            "gggggggggggggggggggggggggggggggg",
        ] {
            assert!(plain_md5(wrong).is_none());
        }
        let files = Files::new();
        let object = files.object("packs/data.f16");
        let proof = local_proof(&object.local_path).unwrap();
        assert_eq!(proof.size, 28);
        assert_ne!(
            proof.md5,
            <[u8; 16]>::from(Md5::digest(b"validated original bytes"))
        );
    }
}
