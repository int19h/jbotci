extern crate bityzba;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use bityzba::*;

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
    generate_embedded_assets();
}

#[requires(true)]
#[ensures(true)]
fn generate_embedded_assets() {
    let output_path =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set")).join("embedded_assets.rs");
    let enabled = env::var_os("CARGO_FEATURE_EMBED_WEB_ASSETS").is_some();
    let Some(root) = env::var_os("JBOTCI_WEB_DIST").map(PathBuf::from) else {
        write_generated_assets(&output_path, &[]);
        if enabled {
            panic!("embed-web-assets requires JBOTCI_WEB_DIST to point at the Dioxus public dir");
        }
        return;
    };
    if !enabled {
        write_generated_assets(&output_path, &[]);
        return;
    }
    let root = root
        .canonicalize()
        .unwrap_or_else(|error| panic!("failed to canonicalize JBOTCI_WEB_DIST: {error}"));
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    files.sort();
    write_generated_assets(&output_path, &files);
    println!("cargo:rerun-if-env-changed=JBOTCI_WEB_DIST");
    println!("cargo:rerun-if-changed={}", root.display());
}

#[requires(root.is_dir())]
#[requires(current.is_dir())]
#[ensures(true)]
fn collect_files(root: &Path, current: &Path, files: &mut Vec<EmbeddedFile>) {
    for entry in fs::read_dir(current)
        .unwrap_or_else(|error| panic!("failed to read `{}`: {error}", current.display()))
    {
        let entry = entry.unwrap_or_else(|error| panic!("failed to read dir entry: {error}"));
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("collected path is under root")
                .to_string_lossy()
                .replace('\\', "/");
            let (request_path, encoding) = if let Some(stripped) = relative.strip_suffix(".br") {
                (format!("/{stripped}"), Some("br"))
            } else {
                (format!("/{relative}"), None)
            };
            files.push(EmbeddedFile {
                request_path,
                encoding,
                filesystem_path: path
                    .canonicalize()
                    .unwrap_or_else(|error| panic!("failed to canonicalize asset path: {error}")),
            });
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn write_generated_assets(output_path: &Path, files: &[EmbeddedFile]) {
    let mut output = String::from("pub static EMBEDDED_ASSETS: &[EmbeddedAsset] = &[\n");
    for file in files {
        let encoding = match file.encoding {
            Some(encoding) => format!("Some({encoding:?})"),
            None => "None".to_owned(),
        };
        output.push_str(&format!(
            "    EmbeddedAsset {{ request_path: {:?}, content_encoding: {}, bytes: include_bytes!({:?}) }},\n",
            file.request_path,
            encoding,
            file.filesystem_path.display().to_string(),
        ));
    }
    output.push_str("];\n");
    fs::write(output_path, output)
        .unwrap_or_else(|error| panic!("failed to write generated embedded assets: {error}"));
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
struct EmbeddedFile {
    request_path: String,
    encoding: Option<&'static str>,
    filesystem_path: PathBuf,
}
