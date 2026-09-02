//! Git integration — the vault is a real git repository (ADR-001).
//!
//! Thin wrapper over `git2`: init/open a repo, commit all changes, and read
//! status/history. This is the durable-version-history half of the write path
//! (ADR-021) — the canonical file write happens first (`vault::write_node`),
//! git commit happens after and is retriable if it fails.

use std::path::Path;

use git2::{Oid, Repository, Signature, StatusOptions};

use crate::error::{IrisError, IrisResult};

pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    /// Initialize a new git repository at `path` (creating the directory if needed).
    pub fn init(path: impl AsRef<Path>) -> IrisResult<Self> {
        let repo = Repository::init(path).map_err(git_err)?;
        Ok(GitRepo { repo })
    }

    /// Open an existing git repository at `path`.
    pub fn open(path: impl AsRef<Path>) -> IrisResult<Self> {
        let repo = Repository::open(path).map_err(git_err)?;
        Ok(GitRepo { repo })
    }

    /// Stage every change in the working tree and commit it.
    ///
    /// Returns the new commit's id. If there's a previous commit, it becomes
    /// this commit's parent; otherwise this is the repository's first commit.
    pub fn commit_all(&self, message: &str) -> IrisResult<Oid> {
        let mut index = self.repo.index().map_err(git_err)?;
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .map_err(git_err)?;
        index.write().map_err(git_err)?;
        let tree_id = index.write_tree().map_err(git_err)?;
        let tree = self.repo.find_tree(tree_id).map_err(git_err)?;

        let sig = self.signature()?;
        let parent = self.head_commit();
        let parents: Vec<_> = parent.iter().collect();

        let commit_id = self
            .repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .map_err(git_err)?;
        Ok(commit_id)
    }

    /// Paths with uncommitted changes (new, modified, or deleted), relative to the repo root.
    pub fn status(&self) -> IrisResult<Vec<String>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        let statuses = self.repo.statuses(Some(&mut opts)).map_err(git_err)?;
        Ok(statuses
            .iter()
            .filter_map(|entry| entry.path().ok().map(str::to_string))
            .collect())
    }

    /// Commit ids from HEAD backwards, most recent first.
    pub fn history(&self) -> IrisResult<Vec<Oid>> {
        let Some(head) = self.head_commit() else {
            return Ok(Vec::new());
        };
        let mut revwalk = self.repo.revwalk().map_err(git_err)?;
        revwalk.push(head.id()).map_err(git_err)?;
        revwalk
            .collect::<Result<Vec<_>, _>>()
            .map_err(git_err)
    }

    fn head_commit(&self) -> Option<git2::Commit<'_>> {
        self.repo.head().ok()?.peel_to_commit().ok()
    }

    fn signature(&self) -> IrisResult<Signature<'static>> {
        self.repo
            .signature()
            .or_else(|_| Signature::now("Iris", "iris@localhost"))
            .map_err(git_err)
    }
}

fn git_err(e: git2::Error) -> IrisError {
    IrisError::Vault(format!("git error: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("iris-git-test-{label}-{nanos}"));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn init_creates_repo() {
        let dir = TempDir::new("init");
        GitRepo::init(dir.path()).unwrap();
        assert!(dir.path().join(".git").is_dir());
    }

    #[test]
    fn open_existing_repo() {
        let dir = TempDir::new("open");
        GitRepo::init(dir.path()).unwrap();
        GitRepo::open(dir.path()).unwrap();
    }

    #[test]
    fn commit_all_creates_commit_and_clears_status() {
        let dir = TempDir::new("commit");
        let repo = GitRepo::init(dir.path()).unwrap();
        fs::write(dir.path().join("note.md"), "hello").unwrap();

        assert_eq!(repo.status().unwrap(), vec!["note.md".to_string()]);

        repo.commit_all("first commit").unwrap();

        assert!(repo.status().unwrap().is_empty());
        assert_eq!(repo.history().unwrap().len(), 1);
    }

    #[test]
    fn second_commit_has_first_as_parent() {
        let dir = TempDir::new("history");
        let repo = GitRepo::init(dir.path()).unwrap();

        fs::write(dir.path().join("a.md"), "a").unwrap();
        repo.commit_all("first").unwrap();

        fs::write(dir.path().join("b.md"), "b").unwrap();
        repo.commit_all("second").unwrap();

        assert_eq!(repo.history().unwrap().len(), 2);
    }
}
