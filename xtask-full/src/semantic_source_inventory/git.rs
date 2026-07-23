use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use bityzba::{ensures, invariant, new, requires};

#[invariant(object_id.len() == 40)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeEntry {
    pub(crate) object_id: String,
}

#[invariant(repository_root.is_absolute() && commit.len() == 40 && tree.len() == 40 && !entries.is_empty())]
#[derive(Debug)]
pub(crate) struct GitTree {
    pub(crate) repository_root: PathBuf,
    pub(crate) commit: String,
    pub(crate) tree: String,
    pub(crate) entries: BTreeMap<String, TreeEntry>,
}

impl GitTree {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(crate) fn load(repository_root: &Path, revision: &str) -> Result<Self> {
        let repository_root = repository_root
            .canonicalize()
            .with_context(|| format!("canonicalizing `{}`", repository_root.display()))?;
        let commit = git_text(
            &repository_root,
            &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
        )?;
        validate_object_id(&commit, "commit")?;
        let tree = git_text(
            &repository_root,
            &["rev-parse", "--verify", &format!("{commit}^{{tree}}")],
        )?;
        validate_object_id(&tree, "tree")?;
        let output = Command::new("git")
            .current_dir(&repository_root)
            .args(["ls-tree", "-r", "-z", "--full-tree", &commit])
            .output()
            .context("running `git ls-tree` for the pinned inventory commit")?;
        if !output.status.success() {
            bail!(
                "`git ls-tree` failed for commit `{commit}`: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let mut entries = BTreeMap::new();
        for raw_entry in output.stdout.split(|byte| *byte == 0) {
            if raw_entry.is_empty() {
                continue;
            }
            let Some(tab) = raw_entry.iter().position(|byte| *byte == b'\t') else {
                bail!("`git ls-tree` emitted an entry without a path separator");
            };
            let metadata = std::str::from_utf8(&raw_entry[..tab])
                .context("`git ls-tree` emitted non-UTF-8 metadata")?;
            let path = std::str::from_utf8(&raw_entry[tab + 1..])
                .context("inventory paths must be valid UTF-8")?;
            let mut metadata = metadata.split_ascii_whitespace();
            let _mode = metadata
                .next()
                .context("`git ls-tree` entry omitted its mode")?;
            let kind = metadata
                .next()
                .context("`git ls-tree` entry omitted its object kind")?;
            let object_id = metadata
                .next()
                .context("`git ls-tree` entry omitted its object id")?;
            if metadata.next().is_some() {
                bail!("`git ls-tree` entry contained unexpected metadata fields");
            }
            if kind != "blob" {
                continue;
            }
            validate_object_id(object_id, "blob")?;
            let entry = new!(TreeEntry {
                object_id: object_id.to_owned(),
            });
            if entries.insert(path.to_owned(), entry).is_some() {
                bail!("pinned tree contains duplicate path `{path}`");
            }
        }
        Ok(new!(GitTree {
            repository_root,
            commit,
            tree,
            entries,
        }))
    }

    #[requires(paths.iter().all(|path| self.entries.contains_key(path)))]
    #[ensures(ret.as_ref().is_ok_and(|blobs| blobs.len() == paths.len()))]
    pub(crate) fn read_blobs(&self, paths: &BTreeSet<String>) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut child = Command::new("git")
            .current_dir(&self.repository_root)
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("starting `git cat-file --batch`")?;
        let mut stdin = child
            .stdin
            .take()
            .context("`git cat-file` stdin was not piped")?;
        let stdout = child
            .stdout
            .take()
            .context("`git cat-file` stdout was not piped")?;
        let mut stdout = BufReader::new(stdout);
        let mut blobs = BTreeMap::new();
        for path in paths {
            let entry = self
                .entries
                .get(path)
                .with_context(|| format!("pinned tree does not contain `{path}`"))?;
            writeln!(stdin, "{}", entry.object_id).context("querying `git cat-file --batch`")?;
            stdin.flush().context("flushing `git cat-file` query")?;
            let mut header = String::new();
            stdout
                .read_line(&mut header)
                .context("reading `git cat-file` response header")?;
            let mut fields = header.split_ascii_whitespace();
            let returned_object = fields
                .next()
                .context("`git cat-file` response omitted the object id")?;
            let kind = fields
                .next()
                .context("`git cat-file` response omitted the object kind")?;
            let size = fields
                .next()
                .context("`git cat-file` response omitted the object size")?
                .parse::<usize>()
                .context("`git cat-file` emitted a non-numeric object size")?;
            if fields.next().is_some() {
                bail!("`git cat-file` emitted unexpected response fields for `{path}`");
            }
            if returned_object != entry.object_id || kind != "blob" {
                bail!(
                    "`git cat-file` returned `{returned_object}` ({kind}) for expected blob `{}`",
                    entry.object_id
                );
            }
            let mut bytes = vec![0; size];
            stdout
                .read_exact(&mut bytes)
                .with_context(|| format!("reading pinned blob `{path}`"))?;
            let mut terminator = [0_u8; 1];
            stdout
                .read_exact(&mut terminator)
                .context("reading `git cat-file` response terminator")?;
            if terminator != [b'\n'] {
                bail!("`git cat-file` response for `{path}` lacked its newline terminator");
            }
            blobs.insert(path.clone(), bytes);
        }
        drop(stdin);
        let output = child
            .wait_with_output()
            .context("waiting for `git cat-file --batch`")?;
        if !output.status.success() {
            bail!(
                "`git cat-file --batch` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(blobs)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(crate) fn ensure_worktree_clean(&self) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repository_root)
            .args(["status", "--porcelain=v1", "--untracked-files=all"])
            .output()
            .context("checking inventory worktree cleanliness")?;
        if !output.status.success() {
            bail!(
                "`git status` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        if !output.stdout.is_empty() {
            bail!(
                "semantic source inventory check requires a clean worktree; `git status --porcelain` reported:\n{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }
}

#[requires(!value.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_object_id(value: &str, kind: &str) -> Result<()> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("resolved {kind} object id `{value}` is not a full SHA-1 id");
    }
    Ok(())
}

#[requires(!arguments.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
fn git_text(repository_root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repository_root)
        .args(arguments)
        .output()
        .with_context(|| format!("running `git {}`", arguments.join(" ")))?;
    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let text = std::str::from_utf8(&output.stdout)
        .context("git emitted non-UTF-8 object identity")?
        .trim();
    if text.is_empty() {
        bail!("`git {}` emitted an empty identity", arguments.join(" "));
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[requires(!arguments.is_empty())]
    #[ensures(true)]
    fn run_git(repository: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .current_dir(repository)
            .args(arguments)
            .status()
            .expect("git starts for inventory test");
        assert!(status.success(), "git command failed: {arguments:?}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pinned_tree_reads_objects_and_detects_worktree_drift() {
        let temporary = tempfile::tempdir().expect("temporary Git repository");
        run_git(temporary.path(), &["init", "--quiet", "--object-format=sha1"]);
        fs::write(temporary.path().join("sample.txt"), b"pinned bytes\n")
            .expect("write tracked sample");
        run_git(temporary.path(), &["add", "sample.txt"]);
        run_git(
            temporary.path(),
            &[
                "-c",
                "user.name=Inventory Test",
                "-c",
                "user.email=inventory@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "pinned test",
            ],
        );

        let tree = GitTree::load(temporary.path(), "HEAD").expect("load pinned tree");
        let paths = BTreeSet::from(["sample.txt".to_owned()]);
        let blobs = tree.read_blobs(&paths).expect("read pinned blob");
        assert_eq!(blobs["sample.txt"], b"pinned bytes\n");
        tree.ensure_worktree_clean().expect("committed tree is clean");

        fs::write(temporary.path().join("untracked.txt"), b"drift\n")
            .expect("write untracked drift witness");
        let error = tree
            .ensure_worktree_clean()
            .expect_err("untracked input drift must fail check mode");
        assert!(error.to_string().contains("requires a clean worktree"));
    }
}
