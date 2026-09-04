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

    /// Clone a remote repository into `path` (creating it). The transport is
    /// whatever `url` names — a user-controlled git remote (self-hosted Gitea,
    /// private GitHub/GitLab, a local/NAS path over SSH) per ADR-030; there is
    /// no Iris-run relay. This is the git half of "restore from backup"
    /// (ARCHITECTURE.md §5) — the engine layer adds the cache rebuild and
    /// integrity check that make a restore trustworthy.
    pub fn clone(url: &str, path: impl AsRef<Path>) -> IrisResult<Self> {
        let repo = Repository::clone(url, path).map_err(git_err)?;
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
        revwalk.collect::<Result<Vec<_>, _>>().map_err(git_err)
    }

    /// Named checkpoints and branching (ARCHITECTURE.md §5): "save a named
    /// checkpoint here" and "try a different reorganization" map directly onto
    /// git tags and branches — real git the whole way down, so anything done
    /// here stays inspectable/recoverable with any standard git tool. The UI
    /// surfaces these in plain language; this layer just does the git.
    /// Tag the current HEAD as a checkpoint.
    pub fn create_checkpoint(&self, name: &str) -> IrisResult<Oid> {
        let head = self.require_head_commit()?;
        self.repo
            .tag_lightweight(name, head.as_object(), false)
            .map_err(git_err)
    }

    /// Checkpoint (tag) names, alphabetically.
    pub fn list_checkpoints(&self) -> IrisResult<Vec<String>> {
        let tags = self.repo.tag_names(None).map_err(git_err)?;
        let mut names = Vec::new();
        for entry in tags.iter() {
            if let Ok(Some(name)) = entry {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    /// Branch off the current HEAD, without switching to it.
    pub fn create_branch(&self, name: &str) -> IrisResult<()> {
        let head = self.require_head_commit()?;
        self.repo.branch(name, &head, false).map_err(git_err)?;
        Ok(())
    }

    /// Local branch names, alphabetically.
    pub fn list_branches(&self) -> IrisResult<Vec<String>> {
        let branches = self
            .repo
            .branches(Some(git2::BranchType::Local))
            .map_err(git_err)?;
        let mut names = Vec::new();
        for entry in branches {
            let (branch, _) = entry.map_err(git_err)?;
            if let Some(name) = branch.name().map_err(git_err)? {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    /// The current branch's name, or `None` if HEAD is detached (e.g. after
    /// checking out a checkpoint tag rather than a branch).
    pub fn current_branch(&self) -> IrisResult<Option<String>> {
        let head = self.repo.head().map_err(git_err)?;
        if !head.is_branch() {
            return Ok(None);
        }
        Ok(Some(head.shorthand().map_err(git_err)?.to_string()))
    }

    /// Switch the working tree to `name` — a branch or a checkpoint tag.
    /// Checking out a branch leaves HEAD attached to it (further commits move
    /// the branch, standard git behavior); checking out a tag (or any other
    /// commit-ish) leaves HEAD detached at that commit, exactly like `git
    /// checkout <tag>` on the command line — inspecting a checkpoint doesn't
    /// silently start a new branch under you.
    pub fn checkout(&self, name: &str) -> IrisResult<()> {
        let (object, reference) = self.repo.revparse_ext(name).map_err(git_err)?;
        self.repo
            .checkout_tree(&object, Some(git2::build::CheckoutBuilder::new().force()))
            .map_err(git_err)?;
        match reference {
            Some(r) if r.is_branch() => {
                let refname = r.name().map_err(git_err)?;
                self.repo.set_head(refname).map_err(git_err)?;
            }
            _ => {
                let commit = object.peel_to_commit().map_err(git_err)?;
                self.repo.set_head_detached(commit.id()).map_err(git_err)?;
            }
        }
        Ok(())
    }

    fn require_head_commit(&self) -> IrisResult<git2::Commit<'_>> {
        self.head_commit()
            .ok_or_else(|| IrisError::Vault("no commits yet".into()))
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
    fn checkpoint_tags_are_listed_and_sorted() {
        let dir = TempDir::new("checkpoint");
        let repo = GitRepo::init(dir.path()).unwrap();
        fs::write(dir.path().join("a.md"), "a").unwrap();
        repo.commit_all("first").unwrap();

        repo.create_checkpoint("v2").unwrap();
        repo.create_checkpoint("v1").unwrap();

        assert_eq!(
            repo.list_checkpoints().unwrap(),
            vec!["v1".to_string(), "v2".to_string()]
        );
    }

    #[test]
    fn branch_is_created_without_switching_to_it() {
        let dir = TempDir::new("branch-create");
        let repo = GitRepo::init(dir.path()).unwrap();
        fs::write(dir.path().join("a.md"), "a").unwrap();
        repo.commit_all("first").unwrap();

        repo.create_branch("try-reorg").unwrap();

        assert!(repo
            .list_branches()
            .unwrap()
            .contains(&"try-reorg".to_string()));
        // Still on the original branch (master/main) — creating a branch
        // doesn't switch to it.
        assert_ne!(
            repo.current_branch().unwrap(),
            Some("try-reorg".to_string())
        );
    }

    #[test]
    fn checkout_branch_switches_the_working_tree_and_stays_attached() {
        let dir = TempDir::new("checkout-branch");
        let repo = GitRepo::init(dir.path()).unwrap();
        fs::write(dir.path().join("a.md"), "original").unwrap();
        repo.commit_all("first").unwrap();
        let original_branch = repo.current_branch().unwrap().unwrap();
        repo.create_branch("try-reorg").unwrap();

        repo.checkout("try-reorg").unwrap();
        assert_eq!(
            repo.current_branch().unwrap(),
            Some("try-reorg".to_string())
        );

        // Diverge on the branch.
        fs::write(dir.path().join("a.md"), "reorganized").unwrap();
        repo.commit_all("reorg").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("a.md")).unwrap(),
            "reorganized"
        );

        // Switching back restores the original content.
        repo.checkout(&original_branch).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("a.md")).unwrap(),
            "original"
        );
    }

    #[test]
    fn checkout_checkpoint_tag_detaches_head() {
        let dir = TempDir::new("checkout-tag");
        let repo = GitRepo::init(dir.path()).unwrap();
        fs::write(dir.path().join("a.md"), "v1 content").unwrap();
        repo.commit_all("first").unwrap();
        repo.create_checkpoint("v1").unwrap();

        fs::write(dir.path().join("a.md"), "v2 content").unwrap();
        repo.commit_all("second").unwrap();

        repo.checkout("v1").unwrap();
        assert_eq!(repo.current_branch().unwrap(), None); // detached HEAD
        assert_eq!(
            fs::read_to_string(dir.path().join("a.md")).unwrap(),
            "v1 content"
        );
    }

    #[test]
    fn clone_checks_out_committed_files() {
        let source_dir = TempDir::new("clone-source");
        let source = GitRepo::init(source_dir.path()).unwrap();
        fs::write(source_dir.path().join("note.md"), "hello").unwrap();
        source.commit_all("first commit").unwrap();

        let dest_dir = TempDir::new("clone-dest");
        fs::remove_dir_all(dest_dir.path()).unwrap(); // clone must create the dir itself
        let cloned = GitRepo::clone(&source_dir.path().to_string_lossy(), dest_dir.path()).unwrap();

        assert!(dest_dir.path().join("note.md").exists());
        assert_eq!(cloned.history().unwrap().len(), 1);
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
