use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use bityzba::{ensures, invariant, new, requires};
use serde::Serialize;

use super::model::{Manifest, ManifestArtifact};
use super::source::sha256_hex;

#[invariant(!bytes.is_empty() && *record_count > 0)]
#[derive(Debug)]
pub(crate) struct ArtifactBytes {
    pub(crate) bytes: Vec<u8>,
    pub(crate) record_count: usize,
}

#[invariant(true)]
#[derive(Debug, Default)]
pub(crate) struct ArtifactSet {
    pub(crate) artifacts: BTreeMap<String, ArtifactBytes>,
}

impl ArtifactSet {
    #[requires(!name.is_empty())]
    #[ensures(self.artifacts.contains_key(name))]
    pub(crate) fn insert_jsonl<T: Serialize>(&mut self, name: &str, records: &[T]) -> Result<()> {
        let mut bytes = Vec::new();
        for record in records {
            serde_json::to_writer(&mut bytes, record)
                .with_context(|| format!("serializing record for `{name}`"))?;
            bytes.push(b'\n');
        }
        if records.is_empty() {
            bail!("required inventory artifact `{name}` would be empty");
        }
        if self
            .artifacts
            .insert(
                name.to_owned(),
                new!(ArtifactBytes {
                    bytes,
                    record_count: records.len(),
                }),
            )
            .is_some()
        {
            bail!("inventory artifact `{name}` was inserted twice");
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.len() == self.artifacts.len())]
    pub(crate) fn manifest_artifacts(&self) -> Vec<ManifestArtifact> {
        self.artifacts
            .iter()
            .map(|(name, artifact)| {
                new!(ManifestArtifact {
                    name: name.clone(),
                    sha256: sha256_hex(&artifact.bytes),
                    byte_length: artifact.bytes.len(),
                    record_count: artifact.record_count,
                })
            })
            .collect()
    }

    #[requires(output.is_absolute())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(crate) fn write(&self, output: &Path, manifest: &Manifest) -> Result<()> {
        fs::create_dir_all(output)
            .with_context(|| format!("creating inventory output `{}`", output.display()))?;
        for (name, artifact) in &self.artifacts {
            let path = output.join(name);
            fs::write(&path, &artifact.bytes)
                .with_context(|| format!("writing inventory artifact `{}`", path.display()))?;
        }
        let mut manifest_bytes = serde_json::to_vec_pretty(manifest)
            .context("serializing semantic source inventory manifest")?;
        manifest_bytes.push(b'\n');
        fs::write(output.join("manifest.json"), manifest_bytes)
            .context("writing semantic source inventory manifest")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jsonl_bytes_and_manifest_metadata_are_deterministic() {
        let records = vec![
            BTreeMap::from([("key", "alpha"), ("value", "one")]),
            BTreeMap::from([("key", "beta"), ("value", "two")]),
        ];
        let mut first = ArtifactSet::default();
        first
            .insert_jsonl("records.jsonl", &records)
            .expect("records serialize");
        let mut second = ArtifactSet::default();
        second
            .insert_jsonl("records.jsonl", &records)
            .expect("records serialize identically");
        assert_eq!(
            first.artifacts["records.jsonl"].bytes.as_slice(),
            second.artifacts["records.jsonl"].bytes.as_slice()
        );
        assert_eq!(first.manifest_artifacts(), second.manifest_artifacts());
    }
}
