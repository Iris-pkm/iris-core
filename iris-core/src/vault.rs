//! Vault engine — create/open a vault directory, scan for `.md` files,
//! and read/write nodes.
//!
//! A vault is just a directory of `.md` files (SCHEMA_SPEC §2 — folder-agnostic,
//! folders are for human convenience only). Node identity and relationships live
//! in frontmatter, never in file paths, so the vault engine doesn't care about
//! layout — it only finds `.md` files and reads/writes them.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{IrisError, IrisResult};
use crate::parser::ParsedNode;

/// A directory on disk containing Iris nodes.
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Create a new vault at `path`, creating the directory if it doesn't exist.
    pub fn create(path: impl AsRef<Path>) -> IrisResult<Self> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Vault { root })
    }

    /// Open an existing vault directory.
    pub fn open(path: impl AsRef<Path>) -> IrisResult<Self> {
        let root = path.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(IrisError::Vault(format!(
                "not a directory: {}",
                root.display()
            )));
        }
        Ok(Vault { root })
    }

    /// The vault's root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Find every `.md` file in the vault, recursively. Skips dotfile directories
    /// (`.git`, `.iris`, etc.) since those hold machine state, not nodes.
    pub fn scan(&self) -> IrisResult<Vec<PathBuf>> {
        let mut found = Vec::new();
        scan_dir(&self.root, &mut found)?;
        Ok(found)
    }

    /// Read and parse the node at `path` (absolute, or relative to the vault root).
    pub fn read_node(&self, path: impl AsRef<Path>) -> IrisResult<ParsedNode> {
        let full_path = self.resolve(path.as_ref());
        let contents = fs::read_to_string(&full_path)?;
        ParsedNode::parse(&contents)
    }

    /// Write `contents` to `path` (absolute, or relative to the vault root).
    ///
    /// Writes atomically: the content lands in a temp file in the same directory,
    /// then an atomic rename replaces the target — so a crash or concurrent read
    /// never observes a partially-written node (ADR-021).
    pub fn write_node(&self, path: impl AsRef<Path>, contents: &str) -> IrisResult<()> {
        let full_path = self.resolve(path.as_ref());
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = full_path.with_extension("md.tmp");
        fs::write(&tmp_path, contents)?;
        fs::rename(&tmp_path, &full_path)?;
        Ok(())
    }

    /// Permanently remove a node file (absolute, or relative to the vault root).
    /// Hard deletion — no soft-delete/Trash semantics here, that's the engine's
    /// job (ADR-016). The file's history remains recoverable from git.
    pub fn remove_node(&self, path: impl AsRef<Path>) -> IrisResult<()> {
        let full_path = self.resolve(path.as_ref());
        fs::remove_file(&full_path)?;
        Ok(())
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }
}

fn scan_dir(dir: &Path, found: &mut Vec<PathBuf>) -> IrisResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            scan_dir(&path, found)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            found.push(path);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A self-cleaning temp directory for tests (avoids pulling in a `tempfile` dep).
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("iris-test-{label}-{nanos}"));
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

    const NOTE: &str = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
---

Hello vault.
";

    #[test]
    fn create_and_open() {
        let dir = TempDir::new("create-open");
        let vault = Vault::create(dir.path()).unwrap();
        assert_eq!(vault.root(), dir.path());

        let reopened = Vault::open(dir.path()).unwrap();
        assert_eq!(reopened.root(), dir.path());
    }

    #[test]
    fn open_missing_dir_fails() {
        let dir = TempDir::new("open-missing");
        let missing = dir.path().join("does-not-exist");
        assert!(Vault::open(missing).is_err());
    }

    #[test]
    fn write_then_read_node() {
        let dir = TempDir::new("write-read");
        let vault = Vault::create(dir.path()).unwrap();

        vault.write_node("notes/hello.md", NOTE).unwrap();
        let parsed = vault.read_node("notes/hello.md").unwrap();

        assert_eq!(parsed.node.id, "01JQZ8XYABCDEF0123456789AB");
        assert!(parsed.body.contains("Hello vault."));
    }

    #[test]
    fn scan_finds_md_files_and_skips_dotdirs() {
        let dir = TempDir::new("scan");
        let vault = Vault::create(dir.path()).unwrap();

        vault.write_node("notes/a.md", NOTE).unwrap();
        vault.write_node("tasks/b.md", NOTE).unwrap();
        vault.write_node(".iris/cache.md", NOTE).unwrap(); // should be skipped
        fs::write(dir.path().join("notes/readme.txt"), "not a node").unwrap();

        let mut found: Vec<_> = vault
            .scan()
            .unwrap()
            .into_iter()
            .map(|p| p.strip_prefix(dir.path()).unwrap().to_path_buf())
            .collect();
        found.sort();

        assert_eq!(
            found,
            vec![PathBuf::from("notes/a.md"), PathBuf::from("tasks/b.md")]
        );
    }

    #[test]
    fn remove_node_deletes_the_file() {
        let dir = TempDir::new("remove");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();

        vault.remove_node("notes/a.md").unwrap();

        assert!(!dir.path().join("notes/a.md").exists());
        assert!(vault.read_node("notes/a.md").is_err());
    }

    #[test]
    fn remove_node_missing_file_errors() {
        let dir = TempDir::new("remove-missing");
        let vault = Vault::create(dir.path()).unwrap();
        assert!(vault.remove_node("notes/missing.md").is_err());
    }

    #[test]
    fn write_is_atomic_no_leftover_tmp_file() {
        let dir = TempDir::new("atomic");
        let vault = Vault::create(dir.path()).unwrap();

        vault.write_node("notes/a.md", NOTE).unwrap();

        let entries: Vec<_> = fs::read_dir(dir.path().join("notes"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("a.md")]);
    }
}
