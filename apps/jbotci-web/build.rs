extern crate bityzba;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use bityzba::*;

#[requires(true)]
#[ensures(true)]
fn main() {
    bityzba::require_contracts().unwrap();
    emit_git_build_info();
}

#[requires(true)]
#[ensures(true)]
fn emit_git_build_info() {
    println!("cargo:rerun-if-env-changed=JBOTCI_GIT_COMMIT");
    emit_git_rerun_paths();

    if let Some(commit) = env::var("JBOTCI_GIT_COMMIT")
        .ok()
        .filter(|value| is_git_commit_hash(value))
        .or_else(current_git_commit)
    {
        let short = commit.chars().take(7).collect::<String>();
        println!("cargo:rustc-env=JBOTCI_GIT_COMMIT={commit}");
        println!("cargo:rustc-env=JBOTCI_GIT_COMMIT_SHORT={short}");
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|commit| commit.chars().all(|character| character.is_ascii_hexdigit())))]
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
#[ensures(true)]
fn emit_git_rerun_paths() {
    let Some(manifest_dir) = env::var("CARGO_MANIFEST_DIR").ok() else {
        return;
    };
    let Some(git_dir) = git_path(&manifest_dir, "--git-dir") else {
        return;
    };
    let git_common_dir =
        git_path(&manifest_dir, "--git-common-dir").unwrap_or_else(|| git_dir.clone());
    let head_path = git_dir.join("HEAD");
    if head_path.exists() {
        println!("cargo:rerun-if-changed={}", head_path.display());
    }
    if let Some(ref_path) = active_branch_ref(&head_path, &git_common_dir) {
        println!("cargo:rerun-if-changed={}", ref_path.display());
    }
    let packed_refs = git_common_dir.join("packed-refs");
    if packed_refs.exists() {
        println!("cargo:rerun-if-changed={}", packed_refs.display());
    }
}

#[requires(!manifest_dir.is_empty())]
#[requires(!flag.is_empty())]
#[ensures(true)]
fn git_path(manifest_dir: &str, flag: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(manifest_dir)
        .arg("rev-parse")
        .arg(flag)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    Some(if path.is_absolute() {
        path
    } else {
        Path::new(manifest_dir).join(path)
    })
}

#[requires(true)]
#[ensures(true)]
fn active_branch_ref(head_path: &Path, git_common_dir: &Path) -> Option<PathBuf> {
    let head = std::fs::read_to_string(head_path).ok()?;
    let reference = head.trim().strip_prefix("ref: ")?;
    let path = git_common_dir.join(reference);
    path.exists().then_some(path)
}

#[requires(true)]
#[ensures(true)]
fn is_git_commit_hash(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())
}
