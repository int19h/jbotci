extern crate bityzba;

use std::env;
use std::process::Command;

use bityzba::*;

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
    emit_build_tag();
}

/// Expose the short git commit as `JBOTCI_GIT_COMMIT_SHORT` so Discord results
/// can record which build produced them (an explicit `JBOTCI_GIT_COMMIT`, as
/// the container build sets, wins over the checkout's HEAD). The Discord state
/// codec only needs a stable short identifier; a build without git information
/// falls back to the crate version at runtime.
#[requires(true)]
#[ensures(true)]
fn emit_build_tag() {
    println!("cargo:rerun-if-env-changed=JBOTCI_GIT_COMMIT");
    let commit = match env::var("JBOTCI_GIT_COMMIT") {
        Ok(commit) if is_git_commit_hash(&commit) => Some(commit),
        Ok(commit) => panic!(
            "JBOTCI_GIT_COMMIT must be a 40-character hexadecimal Git commit hash, got `{commit}`"
        ),
        Err(_) => current_git_commit(),
    };
    if let Some(commit) = commit {
        let short = commit.chars().take(7).collect::<String>();
        println!("cargo:rustc-env=JBOTCI_GIT_COMMIT_SHORT={short}");
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|commit| is_git_commit_hash(commit)))]
fn current_git_commit() -> Option<String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok()?;
    emit_git_rerun_paths(&manifest_dir);
    let output = Command::new("git")
        .arg("-C")
        .arg(&manifest_dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    is_git_commit_hash(&commit).then_some(commit)
}

#[requires(true)]
#[ensures(ret == (value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())))]
fn is_git_commit_hash(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[requires(!manifest_dir.is_empty())]
#[ensures(true)]
fn emit_git_rerun_paths(manifest_dir: &str) {
    for name in ["HEAD", "packed-refs", "refs"] {
        let output = Command::new("git")
            .args(["-C", manifest_dir, "rev-parse", "--git-path", name])
            .output()
            .expect("failed to find Git build inputs");
        if output.status.success() {
            let path = String::from_utf8(output.stdout).expect("Git path must use UTF-8");
            let path = std::path::Path::new(manifest_dir).join(path.trim());
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
