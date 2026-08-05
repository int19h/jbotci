//! Offline mint/check entry point for the current smusni-v0 candidate bundle.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[path = "../codegen/smusni_v0_bundle.rs"]
mod smusni_v0_bundle;
#[path = "../codegen/smusni_v0_completeness.rs"]
mod smusni_v0_completeness;
#[path = "../codegen/smusni_v0_dispositions.rs"]
mod smusni_v0_dispositions;
#[path = "../codegen/smusni_v0_kernel.rs"]
mod smusni_v0_kernel;
#[path = "../codegen/smusni_v0_surface.rs"]
mod smusni_v0_surface;

use smusni_v0_bundle::{BundleMode, BundlePaths};

#[requires(true)]
#[ensures(true)]
fn main() -> Result<(), Box<dyn Error>> {
    let mode = match std::env::args().nth(1) {
        Some(value) if value == "--check" => BundleMode::Check,
        Some(value) if value == "--generate" => BundleMode::Generate,
        _ => return Err("usage: smusni_v0_bundle (--check|--generate)".into()),
    };
    if std::env::args().nth(2).is_some() {
        return Err("usage: smusni_v0_bundle (--check|--generate)".into());
    }
    let scratch = smusni_v0_bundle::scratch_dir("bundle-cli");
    fs::create_dir_all(&scratch)?;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths =
        BundlePaths::for_manifest_dir(&manifest_dir, scratch.join("lexical_scope_policies.rs"));
    let dispositions = smusni_v0_dispositions::projected_dispositions();
    smusni_v0_bundle::run(&paths, &dispositions, mode)?;
    Ok(())
}
