extern crate bityzba;

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use bzip2::Compression;
use bzip2::write::BzEncoder;
use serde::Deserialize;

#[invariant(!chrestomathy_chapter_id.is_empty())]
#[invariant(!ebnf_section_id.is_empty())]
#[invariant(!ebnf_symbols.is_empty())]
#[invariant(!edition_ancestry.is_empty())]
#[derive(Debug, Deserialize)]
struct CllImportMetadata {
    chrestomathy_chapter_id: String,
    ebnf_section_id: String,
    edition_ancestry: Vec<CllEditionAncestorMetadata>,
    ebnf_symbols: std::collections::BTreeMap<String, String>,
}

#[invariant(!title.is_empty())]
#[invariant(!version.is_empty())]
#[derive(Debug, Deserialize)]
struct CllEditionAncestorMetadata {
    title: String,
    version: String,
}

#[invariant(section.iter().all(|item| !item.id.is_empty()))]
#[derive(Debug, Deserialize)]
struct CllChrestomathyMetadata {
    section: Vec<CllChrestomathySectionMetadata>,
}

#[invariant(!id.is_empty())]
#[derive(Debug, Deserialize)]
struct CllChrestomathySectionMetadata {
    id: String,
}

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
    if let Err(error) = write_embedded_chapters() {
        panic!("failed to embed CLL chapters: {error}");
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_embedded_chapters() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or("crate is not under the workspace crates directory")?;
    let chapter_dir = workspace_dir.join("vendor/cll/chapters");
    let import_metadata_path = workspace_dir.join("vendor/cll-import-metadata.toml");
    let chrestomathy_metadata_path = workspace_dir.join("vendor/cll-chrestomathy.toml");
    let edition_env_path = workspace_dir.join("vendor/cll/.env");
    let vendored_from_path = workspace_dir.join("vendor/cll.VENDORED_FROM");
    println!("cargo:rerun-if-changed={}", chapter_dir.display());
    println!("cargo:rerun-if-changed={}", import_metadata_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        chrestomathy_metadata_path.display()
    );
    println!("cargo:rerun-if-changed={}", edition_env_path.display());
    println!("cargo:rerun-if-changed={}", vendored_from_path.display());
    let import_metadata = validate_import_metadata(&import_metadata_path)?;
    validate_chrestomathy_metadata(&chrestomathy_metadata_path)?;
    let edition = read_edition(&edition_env_path, &vendored_from_path, &import_metadata)?;

    let mut chapters = fs::read_dir(&chapter_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    chapters.retain(|path| path.extension().is_some_and(|extension| extension == "xml"));
    chapters.sort();
    let numbered_chapter_count = chapters
        .iter()
        .filter(|path| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|stem| !stem.is_empty() && stem.chars().all(|ch| ch.is_ascii_digit()))
        })
        .count();

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let mut generated = String::new();
    generated.push_str("pub const EMBEDDED_CLL_CHAPTERS: &[(&str, u16, &[u8])] = &[\n");
    for (chapter_index, path) in chapters.into_iter().enumerate() {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("chapter path has no UTF-8 file name")?;
        let chapter_number = chapter_number_for_file_name(file_name, numbered_chapter_count)?;
        if usize::from(chapter_number) != chapter_index + 1 {
            return Err(format!(
                "CLL chapter file {file_name} maps to chapter {chapter_number}, but sorted position is {}",
                chapter_index + 1
            )
            .into());
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let source = fs::read(&path)?;
        let compressed = compress_bzip2(&source)?;
        let compressed_file_name = format!("{file_name}.bz2");
        let compressed_path = out_dir.join(&compressed_file_name);
        fs::write(&compressed_path, compressed)?;
        generated.push_str("    (");
        generated.push_str(&format!("{file_name:?}"));
        generated.push_str(&format!(", {chapter_number}, "));
        generated.push_str("include_bytes!(concat!(env!(\"OUT_DIR\"), \"/\", ");
        generated.push_str(&format!("{compressed_file_name:?}"));
        generated.push_str("))");
        generated.push_str("),\n");
    }
    generated.push_str("];\n\n");
    generated.push_str(&edition);

    fs::write(out_dir.join("embedded_cll.rs"), generated)?;
    Ok(())
}

#[requires(path.file_name().is_some())]
#[ensures(ret.as_ref().is_ok_and(|metadata| !metadata.edition_ancestry.is_empty()) || ret.is_err())]
fn validate_import_metadata(path: &Path) -> Result<CllImportMetadata, Box<dyn std::error::Error>> {
    let metadata = fs::read_to_string(path)?;
    let metadata: CllImportMetadata = toml::from_str(&metadata)?;
    if metadata.chrestomathy_chapter_id.trim().is_empty() {
        return Err("vendor/cll-import-metadata.toml has empty chrestomathy_chapter_id".into());
    }
    if metadata.ebnf_section_id.trim().is_empty() {
        return Err("vendor/cll-import-metadata.toml has empty ebnf_section_id".into());
    }
    if metadata
        .ebnf_symbols
        .values()
        .any(|section_id| section_id.trim().is_empty())
    {
        return Err("vendor/cll-import-metadata.toml contains an empty EBNF target".into());
    }
    if metadata.edition_ancestry.is_empty() {
        return Err("vendor/cll-import-metadata.toml has no edition_ancestry entries".into());
    }
    if metadata
        .edition_ancestry
        .iter()
        .any(|ancestor| ancestor.title.trim().is_empty() || ancestor.version.trim().is_empty())
    {
        return Err(
            "vendor/cll-import-metadata.toml has an edition_ancestry entry with an empty field"
                .into(),
        );
    }
    Ok(metadata)
}

/// Emits the vendored edition's identity as generated constants.
///
/// Every value comes from a vendored file, so a submodule bump cannot leave the
/// reported edition behind: the book's own `.env` declares its title, version,
/// and publisher line, and `vendor/cll.VENDORED_FROM` records the pin we
/// vendored it at. The two are cross-checked against each other here, because a
/// version that can drift from the actual vendored text is worse than no
/// version at all.
#[requires(env_path.file_name().is_some())]
#[requires(vendored_from_path.file_name().is_some())]
#[ensures(ret.as_ref().is_ok_and(|generated| !generated.is_empty()) || ret.is_err())]
fn read_edition(
    env_path: &Path,
    vendored_from_path: &Path,
    import_metadata: &CllImportMetadata,
) -> Result<String, Box<dyn std::error::Error>> {
    let env_text = fs::read_to_string(env_path)?;
    let env = parse_key_value_file(&env_text, '=', "vendor/cll/.env")?;
    let vendored_from_text = fs::read_to_string(vendored_from_path)?;
    let vendored_from = parse_key_value_file(&vendored_from_text, ':', "vendor/cll.VENDORED_FROM")?;

    let title = required_field(&env, "TITLE", "vendor/cll/.env")?;
    let version = required_field(&env, "VERSION", "vendor/cll/.env")?;
    let publisher = required_field(&env, "PUBLISHER", "vendor/cll/.env")?;
    let upstream_url = required_field(&vendored_from, "upstream-url", "vendor/cll.VENDORED_FROM")?;
    let release_tag = required_field(&vendored_from, "release-tag", "vendor/cll.VENDORED_FROM")?;
    let commit = required_field(&vendored_from, "commit", "vendor/cll.VENDORED_FROM")?;

    // The book names itself `<edition>-<version>` while the repository tags the
    // same release `v<version>`; anything else means the submodule and the pin
    // record disagree about which edition is actually vendored.
    let tag_version = release_tag.strip_prefix('v').unwrap_or(release_tag);
    if version != tag_version && !version.ends_with(&format!("-{tag_version}")) {
        return Err(format!(
            "vendored CLL edition {version:?} does not match the pinned release tag {release_tag:?}"
        )
        .into());
    }

    let mut generated = String::from(
        "pub(crate) const EMBEDDED_CLL_EDITION: EmbeddedCllEdition = EmbeddedCllEdition {\n",
    );
    generated.push_str(&format!("    title: {title:?},\n"));
    generated.push_str(&format!("    version: {version:?},\n"));
    generated.push_str(&format!("    publisher: {publisher:?},\n"));
    generated.push_str("    ancestry: &[\n");
    for ancestor in &import_metadata.edition_ancestry {
        let ancestor_title = ancestor.title.trim();
        let ancestor_version = ancestor.version.trim();
        generated.push_str(&format!(
            "        EmbeddedCllEditionAncestor {{ title: {ancestor_title:?}, version: {ancestor_version:?} }},\n"
        ));
    }
    generated.push_str("    ],\n");
    generated.push_str(&format!("    upstream_url: {upstream_url:?},\n"));
    generated.push_str(&format!("    release_tag: {release_tag:?},\n"));
    generated.push_str(&format!("    commit: {commit:?},\n"));
    generated.push_str("};\n");
    Ok(generated)
}

/// Parses a flat `KEY<separator>VALUE` vendored metadata file, dropping blank
/// lines and `#` comments and stripping the double quotes the `.env` format
/// allows around a value.
#[requires(!source_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|fields| fields.iter().all(|(key, _)| !key.is_empty())) || ret.is_err())]
fn parse_key_value_file(
    text: &str,
    separator: char,
    source_name: &str,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let mut fields = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(separator)
            .ok_or_else(|| format!("{source_name} has a line without {separator:?}: {line:?}"))?;
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("{source_name} has a line with an empty key: {line:?}").into());
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value);
        fields.push((key.to_owned(), value.to_owned()));
    }
    Ok(fields)
}

#[requires(!key.is_empty())]
#[requires(!source_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn required_field<'a>(
    fields: &'a [(String, String)],
    key: &str,
    source_name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    fields
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{source_name} is missing a non-empty {key}").into())
}

#[requires(path.file_name().is_some())]
#[ensures(ret.as_ref().is_ok_and(|_| true) || ret.is_err())]
fn validate_chrestomathy_metadata(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::read_to_string(path)?;
    let metadata: CllChrestomathyMetadata = toml::from_str(&metadata)?;
    let mut section_ids = std::collections::BTreeSet::new();
    for section in &metadata.section {
        if !section_ids.insert(section.id.clone()) {
            return Err(format!("duplicate chrestomathy metadata section: {}", section.id).into());
        }
    }
    Ok(())
}

#[requires(!file_name.is_empty())]
#[requires(numbered_chapter_count > 0)]
#[ensures(ret.as_ref().is_ok_and(|number| *number > 0) || ret.is_err())]
fn chapter_number_for_file_name(
    file_name: &str,
    numbered_chapter_count: usize,
) -> Result<u16, Box<dyn std::error::Error>> {
    let stem = file_name
        .strip_suffix(".xml")
        .ok_or_else(|| format!("CLL chapter file does not end in .xml: {file_name}"))?;
    if !stem.is_empty() && stem.chars().all(|ch| ch.is_ascii_digit()) {
        let number = stem.parse::<u16>()?;
        if number == 0 {
            return Err(format!("CLL chapter file has zero chapter number: {file_name}").into());
        }
        return Ok(number);
    }
    if let Some(appendix_stem) = stem.strip_prefix('a')
        && !appendix_stem.is_empty()
        && appendix_stem.chars().all(|ch| ch.is_ascii_digit())
    {
        let appendix_number = appendix_stem.parse::<usize>()?;
        if appendix_number == 0 {
            return Err(format!("CLL appendix file has zero appendix number: {file_name}").into());
        }
        return u16::try_from(numbered_chapter_count + appendix_number)
            .map_err(|error| error.into());
    }
    Err(format!("CLL chapter file has unsupported numeric prefix: {file_name}").into())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| !bytes.is_empty()) || ret.is_err())]
fn compress_bzip2(source: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = BzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(source)?;
    encoder.finish()
}
