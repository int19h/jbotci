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
