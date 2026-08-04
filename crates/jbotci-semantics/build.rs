extern crate bityzba;

use std::error::Error;
use std::path::PathBuf;

#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[path = "codegen/lexical_scope_policy.rs"]
mod lexical_scope_policy;

#[requires(true)]
#[ensures(true)]
fn main() -> Result<(), Box<dyn Error>> {
    bityzba::require_contracts().unwrap();
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let dictionary_dir = manifest_dir.join("../jbotci-dictionary-data/data");
    let paths = lexical_scope_policy::GeneratorPaths {
        source: manifest_dir.join("data/sources/lojban-org/oblique_keywords.txt"),
        source_metadata: manifest_dir
            .join("data/sources/lojban-org/oblique_keywords.metadata.toml"),
        policies: manifest_dir.join("data/lexical-scope-policies.toml"),
        witnesses: manifest_dir.join("data/smusni-draft9-must-compact.txt"),
        dictionary: dictionary_dir.join("dictionary-en.json"),
        dictionary_metadata: dictionary_dir.join("dictionary-en.metadata.toml"),
        output: PathBuf::from(std::env::var("OUT_DIR")?).join("lexical_scope_policies.rs"),
    };
    lexical_scope_policy::generate_from_paths(&paths)?;
    for path in paths.inputs() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    Ok(())
}
