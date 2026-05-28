extern crate bityzba;

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use bzip2::Compression;
use bzip2::write::BzEncoder;

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
    println!("cargo:rerun-if-changed={}", chapter_dir.display());

    let mut chapters = fs::read_dir(&chapter_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    chapters.retain(|path| path.extension().is_some_and(|extension| extension == "xml"));
    chapters.sort();

    let mut generated = String::new();
    generated.push_str("pub const EMBEDDED_CLL_CHAPTERS: &[(&str, &[u8])] = &[\n");
    for path in chapters {
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("chapter path has no UTF-8 file name")?;
        println!("cargo:rerun-if-changed={}", path.display());
        let source = fs::read(&path)?;
        let compressed = compress_bzip2(&source)?;
        generated.push_str("    (");
        generated.push_str(&format!("{file_name:?}"));
        generated.push_str(", &");
        generated.push_str(&format!("{compressed:?}"));
        generated.push_str("),\n");
    }
    generated.push_str("];\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("embedded_cll.rs"), generated)?;
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|bytes| !bytes.is_empty()))]
fn compress_bzip2(source: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = BzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(source)?;
    encoder.finish()
}
