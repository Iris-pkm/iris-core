//! Node CRUD engine — ties vault, cache, and git together behind the
//! canonical-write-wins write order (ADR-021):
//!
//! 1. validate the mutation
//! 2. atomically write the canonical file (`vault::write_node` — already atomic)
//! 3. rebuild the cache from the vault
//! 4. commit to git
//!
//! A canonical file write is never rolled back because a later step failed —
//! the cache is rebuildable and a commit is retriable, but the user's content
//! is never discarded for a secondary system's failure. Each step here simply
//! returns its own error without undoing the ones before it.

use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::cache::Cache;
use crate::error::{IrisError, IrisResult};
use crate::git::GitRepo;
use crate::integrity::{self, IntegrityReport};
use crate::parser::ParsedNode;
use crate::types::Node;
use crate::vault::Vault;

/// A captured node state, for undo/redo: either the node existed (with this
/// frontmatter and body) or the path was absent (nothing to restore but the
/// removal itself).
enum UndoState {
    Existing { node: Box<Node>, body: String },
    Absent,
}

/// One reversible step: "at `rel_path`, the state used to be `state`."
/// Applying it writes that state back (or removes the file, if it was absent).
struct UndoEntry {
    rel_path: PathBuf,
    state: UndoState,
}

/// Default Trash retention window before `purge_expired_trash_default` removes
/// a soft-deleted node from the working vault (ARCHITECTURE.md §5). Still
/// recoverable from git history afterward — this only affects the convenience
/// Trash view, not durability.
pub const DEFAULT_TRASH_RETENTION_DAYS: i64 = 30;

pub struct Engine {
    vault: Vault,
    cache: Cache,
    git: GitRepo,
    /// In-session undo/redo (ARCHITECTURE.md §5): in-memory, cleared on
    /// restart (there's no `Engine` state to reopen — a fresh `Engine::open`
    /// starts with empty stacks). Distinct from, and much shorter-lived than,
    /// git-history restore or Trash.
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
}

impl Engine {
    /// Create a brand-new vault at `path`: an empty directory, a git repo, and
    /// a fresh cache. `.iris/cache.sqlite` is gitignored — it's derived, never
    /// committed (SCHEMA_SPEC §2).
    pub fn init(path: impl AsRef<Path>) -> IrisResult<Self> {
        let vault = Vault::create(path.as_ref())?;
        let git = GitRepo::init(vault.root())?;
        std::fs::write(vault.root().join(".gitignore"), ".iris/cache.sqlite\n")?;
        std::fs::create_dir_all(vault.root().join(".iris"))?;
        let cache = Cache::open(vault.root().join(".iris/cache.sqlite"))?;

        let mut engine = Engine {
            vault,
            cache,
            git,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        engine.rebuild_cache()?;
        Ok(engine)
    }

    /// Open an existing vault (directory, git repo, and `.iris/cache.sqlite` must exist).
    pub fn open(path: impl AsRef<Path>) -> IrisResult<Self> {
        let vault = Vault::open(path.as_ref())?;
        let git = GitRepo::open(vault.root())?;
        let cache = Cache::open(vault.root().join(".iris/cache.sqlite"))?;
        Ok(Engine {
            vault,
            cache,
            git,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    /// Create a new node file and commit it.
    ///
    /// `body` is the exact content that follows the closing `---` (see
    /// `ParsedNode::body` for its precise semantics — typically starts with `\n`).
    pub fn create_node(
        &mut self,
        rel_path: impl AsRef<Path>,
        node: &Node,
        body: &str,
    ) -> IrisResult<()> {
        let rel_path = rel_path.as_ref();
        let before = self.capture_state(rel_path);
        self.write_node_raw(
            rel_path,
            node,
            body,
            &format!("Create {}", rel_path.display()),
        )?;
        self.push_undo(rel_path, before);
        Ok(())
    }

    /// Read a node.
    pub fn read_node(&self, rel_path: impl AsRef<Path>) -> IrisResult<ParsedNode> {
        self.vault.read_node(rel_path)
    }

    /// Tier 1 template instantiation (DECISION_LOG.md ADR-026): copy a node
    /// flagged `is_template: true` at `template_rel_path` into a brand-new,
    /// independent node at `new_rel_path`. The copy gets a fresh id and fresh
    /// `created`/`modified` timestamps, `is_template: false`, and is otherwise
    /// identical (body included). No link to the template is kept — later
    /// edits to the template never affect the copy.
    pub fn instantiate_template(
        &mut self,
        template_rel_path: impl AsRef<Path>,
        new_rel_path: impl AsRef<Path>,
    ) -> IrisResult<()> {
        let template = self.vault.read_node(&template_rel_path)?;
        if !template.node.is_template {
            return Err(IrisError::Validation(format!(
                "{} is not a template (is_template is false)",
                template_rel_path.as_ref().display()
            )));
        }
        let mut node = template.node.clone();
        node.id = crate::types::new_node_id();
        node.created = Utc::now();
        node.modified = node.created;
        node.is_template = false;
        self.create_node(new_rel_path, &node, &template.body)
    }

    /// Replace a node's frontmatter, preserving its body byte-for-byte.
    ///
    /// Note: this re-serializes the *frontmatter* from `node` — comments, key
    /// order, and unknown fields in the old frontmatter are not preserved.
    /// Full field-level lossless editing (ADR-019) needs a concrete-syntax-tree
    /// editor in `parser.rs` that doesn't exist yet; this is a known, honest gap.
    pub fn update_node(&mut self, rel_path: impl AsRef<Path>, node: &Node) -> IrisResult<()> {
        let rel_path = rel_path.as_ref();
        let existing = self.vault.read_node(rel_path)?;
        self.push_undo(
            rel_path,
            UndoState::Existing {
                node: Box::new(existing.node),
                body: existing.body.clone(),
            },
        );
        self.write_node_raw(
            rel_path,
            node,
            &existing.body,
            &format!("Update {}", rel_path.display()),
        )
    }

    /// Soft-delete a node: sets `deleted_at` (ADR-016). The file is not removed
    /// — it moves to the Trash view (`views::trash`) and stays recoverable via
    /// `restore_node` or `undo`, and later `purge_expired_trash` if unrestored.
    pub fn delete_node(&mut self, rel_path: impl AsRef<Path>) -> IrisResult<()> {
        let rel_path = rel_path.as_ref();
        let existing = self.vault.read_node(rel_path)?;
        self.push_undo(
            rel_path,
            UndoState::Existing {
                node: Box::new(existing.node.clone()),
                body: existing.body.clone(),
            },
        );
        let mut node = existing.node;
        node.deleted_at = Some(Utc::now());
        self.write_node_raw(
            rel_path,
            &node,
            &existing.body,
            &format!("Trash {}", rel_path.display()),
        )
    }

    /// Recover a node out of Trash: clears `deleted_at`.
    pub fn restore_node(&mut self, rel_path: impl AsRef<Path>) -> IrisResult<()> {
        let rel_path = rel_path.as_ref();
        let existing = self.vault.read_node(rel_path)?;
        self.push_undo(
            rel_path,
            UndoState::Existing {
                node: Box::new(existing.node.clone()),
                body: existing.body.clone(),
            },
        );
        let mut node = existing.node;
        node.deleted_at = None;
        self.write_node_raw(
            rel_path,
            &node,
            &existing.body,
            &format!("Restore {}", rel_path.display()),
        )
    }

    /// Permanently remove every Trash node whose `deleted_at` is older than
    /// `retention` (ARCHITECTURE.md §5: "after the window they're removed from
    /// the working vault... because every prior state was committed to git,
    /// they remain recoverable from history indefinitely"). Returns the count
    /// removed. Deliberately does **not** go through the undo stack — this is
    /// the git-history tier of recovery, not the in-session tier.
    pub fn purge_expired_trash(&mut self, retention: chrono::Duration) -> IrisResult<usize> {
        let cutoff = Utc::now() - retention;
        let trashed = self
            .cache
            .query_nodes("SELECT * FROM nodes WHERE deleted_at IS NOT NULL", [])?;

        let mut removed = 0;
        for row in trashed {
            let Some(deleted_at) = row.deleted_at.as_deref() else {
                continue;
            };
            let Ok(deleted_at) = chrono::DateTime::parse_from_rfc3339(deleted_at) else {
                continue;
            };
            if deleted_at.with_timezone(&Utc) < cutoff {
                self.vault.remove_node(&row.path)?;
                removed += 1;
            }
        }

        if removed > 0 {
            self.rebuild_cache()?;
            self.git
                .commit_all(&format!("Purge {removed} expired Trash item(s)"))?;
        }
        Ok(removed)
    }

    /// `purge_expired_trash` using the default ~30-day retention window
    /// (`DEFAULT_TRASH_RETENTION_DAYS`).
    pub fn purge_expired_trash_default(&mut self) -> IrisResult<usize> {
        self.purge_expired_trash(chrono::Duration::days(DEFAULT_TRASH_RETENTION_DAYS))
    }

    /// Reverse the most recent undoable operation (create/update/delete/restore).
    /// Returns `false` if there was nothing to undo. In-session only — cleared
    /// on restart, distinct from Trash and git-history restore
    /// (ARCHITECTURE.md §5).
    pub fn undo(&mut self) -> IrisResult<bool> {
        let Some(entry) = self.undo_stack.pop() else {
            return Ok(false);
        };
        let redo_state = self.capture_state(&entry.rel_path);
        self.apply_state(&entry.rel_path, &entry.state, "Undo")?;
        self.redo_stack.push(UndoEntry {
            rel_path: entry.rel_path,
            state: redo_state,
        });
        Ok(true)
    }

    /// Reapply the most recently undone operation. Returns `false` if there
    /// was nothing to redo.
    pub fn redo(&mut self) -> IrisResult<bool> {
        let Some(entry) = self.redo_stack.pop() else {
            return Ok(false);
        };
        let undo_state = self.capture_state(&entry.rel_path);
        self.apply_state(&entry.rel_path, &entry.state, "Redo")?;
        self.undo_stack.push(UndoEntry {
            rel_path: entry.rel_path,
            state: undo_state,
        });
        Ok(true)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Rebuild the cache from the vault (safe to call any time — ADR-002).
    pub fn rebuild_cache(&mut self) -> IrisResult<()> {
        self.cache.rebuild(&self.vault)
    }

    /// Run the vault integrity checker.
    pub fn check_integrity(&self) -> IrisResult<IntegrityReport> {
        integrity::check(&self.vault)
    }

    pub fn vault_root(&self) -> &Path {
        self.vault.root()
    }

    /// Write a node's canonical file, rebuild the cache, and commit — the
    /// shared core of every mutating operation (ADR-021's ordering), without
    /// touching the undo/redo stacks (callers push their own entries, since
    /// the "before" state they need depends on which operation this is).
    fn write_node_raw(
        &mut self,
        rel_path: &Path,
        node: &Node,
        body: &str,
        commit_msg: &str,
    ) -> IrisResult<()> {
        validate(node)?;
        let contents = render(node, body)?;
        self.vault.write_node(rel_path, &contents)?;
        self.rebuild_cache()?;
        self.git.commit_all(commit_msg)?;
        Ok(())
    }

    /// Permanently remove a node's file, rebuild the cache, and commit.
    fn remove_node_file(&mut self, rel_path: &Path, commit_msg: &str) -> IrisResult<()> {
        self.vault.remove_node(rel_path)?;
        self.rebuild_cache()?;
        self.git.commit_all(commit_msg)?;
        Ok(())
    }

    /// Snapshot the current state at `rel_path`, for the undo/redo stacks.
    fn capture_state(&self, rel_path: &Path) -> UndoState {
        match self.vault.read_node(rel_path) {
            Ok(parsed) => UndoState::Existing {
                node: Box::new(parsed.node),
                body: parsed.body,
            },
            Err(_) => UndoState::Absent,
        }
    }

    fn push_undo(&mut self, rel_path: &Path, before: UndoState) {
        self.undo_stack.push(UndoEntry {
            rel_path: rel_path.to_path_buf(),
            state: before,
        });
        self.redo_stack.clear();
    }

    /// Write `state` back at `rel_path`: recreate/overwrite it if it was
    /// `Existing`, or remove the file if it was `Absent`.
    fn apply_state(
        &mut self,
        rel_path: &Path,
        state: &UndoState,
        commit_msg: &str,
    ) -> IrisResult<()> {
        match state {
            UndoState::Existing { node, body } => {
                self.write_node_raw(rel_path, node, body, commit_msg)
            }
            UndoState::Absent => self.remove_node_file(rel_path, commit_msg),
        }
    }
}

fn validate(node: &Node) -> IrisResult<()> {
    if node.id.trim().is_empty() {
        return Err(IrisError::Validation("node id must not be empty".into()));
    }
    Ok(())
}

fn render(node: &Node, body: &str) -> IrisResult<String> {
    let frontmatter = serde_yaml::to_string(node)
        .map_err(|e| IrisError::Validation(format!("failed to serialize node: {e}")))?;
    let frontmatter = frontmatter.trim_end_matches('\n');
    Ok(format!("---\n{frontmatter}\n---{body}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{new_node_id, NodeType};
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
            let path = std::env::temp_dir().join(format!("iris-engine-test-{label}-{nanos}"));
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

    fn sample_node() -> Node {
        Node {
            id: new_node_id(),
            node_type: NodeType::Note,
            created: Utc::now(),
            modified: Utc::now(),
            schema_version: crate::types::CURRENT_SCHEMA_VERSION,
            lifecycle: None,
            archived_at: None,
            domain: None,
            tags: vec![],
            relations: vec![],
            deleted_at: None,
            is_template: false,
            distillation_level: None,
            status: None,
            priority: None,
            scheduled_date: None,
            due_date: None,
            estimated_pomodoros: None,
            actual_pomodoros: None,
            recurrence: None,
            checklist: vec![],
            start: None,
            end: None,
            external_id: None,
            project_status: None,
            start_date: None,
            target_date: None,
            source_url: None,
            read_status: None,
            reminder_text: None,
            fire_at: None,
            reminder_status: None,
            resolved: false,
            anchor: None,
            pinned: vec![],
            active_filter: None,
            default_view: None,
            theme: None,
            ink_attachment: None,
            date: None,
        }
    }

    #[test]
    fn init_creates_vault_git_and_cache() {
        let dir = TempDir::new("init");
        let engine = Engine::init(dir.path()).unwrap();
        assert!(dir.path().join(".git").is_dir());
        assert!(dir.path().join(".iris/cache.sqlite").exists());
        assert!(engine.check_integrity().unwrap().is_clean());
    }

    #[test]
    fn create_read_update_delete_round_trip() {
        let dir = TempDir::new("crud");
        let mut engine = Engine::init(dir.path()).unwrap();

        let node = sample_node();
        let id = node.id.clone();
        engine
            .create_node("notes/a.md", &node, "\n\nHello.\n")
            .unwrap();

        // read
        let parsed = engine.read_node("notes/a.md").unwrap();
        assert_eq!(parsed.node.id, id);
        assert!(parsed.body.contains("Hello."));

        // cache reflects it
        let cached = engine.cache.list_nodes().unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].id, id);

        // update: change domain, body must survive untouched
        let mut updated = parsed.node.clone();
        updated.domain = Some("iris-dev".to_string());
        engine.update_node("notes/a.md", &updated).unwrap();

        let after_update = engine.read_node("notes/a.md").unwrap();
        assert_eq!(after_update.node.domain.as_deref(), Some("iris-dev"));
        assert!(after_update.body.contains("Hello."));

        // delete: soft-delete, file still readable, deleted_at set
        engine.delete_node("notes/a.md").unwrap();
        let after_delete = engine.read_node("notes/a.md").unwrap();
        assert!(after_delete.node.deleted_at.is_some());

        // git history: create + update + delete = 3 commits
        assert_eq!(engine.git.history().unwrap().len(), 3);
        assert!(engine.git.status().unwrap().is_empty());
    }

    #[test]
    fn open_reopens_existing_vault() {
        let dir = TempDir::new("reopen");
        {
            let mut engine = Engine::init(dir.path()).unwrap();
            engine
                .create_node("notes/a.md", &sample_node(), "\n\nHi.\n")
                .unwrap();
        }

        let reopened = Engine::open(dir.path()).unwrap();
        let cached = reopened.cache.list_nodes().unwrap();
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn create_rejects_empty_id() {
        let dir = TempDir::new("validate");
        let mut engine = Engine::init(dir.path()).unwrap();
        let mut node = sample_node();
        node.id = "".to_string();
        assert!(engine.create_node("notes/a.md", &node, "\n").is_err());
    }

    #[test]
    fn instantiate_template_copies_with_new_id_and_clears_flag() {
        let dir = TempDir::new("template");
        let mut engine = Engine::init(dir.path()).unwrap();

        let mut template = sample_node();
        template.is_template = true;
        let template_id = template.id.clone();
        engine
            .create_node("templates/daily.md", &template, "\n\nTemplate body.\n")
            .unwrap();

        engine
            .instantiate_template("templates/daily.md", "notes/today.md")
            .unwrap();

        let copy = engine.read_node("notes/today.md").unwrap();
        assert_ne!(copy.node.id, template_id);
        assert!(!copy.node.is_template);
        assert!(copy.body.contains("Template body."));

        // Original template is untouched.
        let original = engine.read_node("templates/daily.md").unwrap();
        assert_eq!(original.node.id, template_id);
        assert!(original.node.is_template);
    }

    #[test]
    fn instantiate_template_rejects_non_template_source() {
        let dir = TempDir::new("template-reject");
        let mut engine = Engine::init(dir.path()).unwrap();
        let node = sample_node();
        engine.create_node("notes/a.md", &node, "\n").unwrap();

        assert!(engine
            .instantiate_template("notes/a.md", "notes/b.md")
            .is_err());
    }

    #[test]
    fn restore_node_clears_deleted_at() {
        let dir = TempDir::new("restore");
        let mut engine = Engine::init(dir.path()).unwrap();
        engine
            .create_node("notes/a.md", &sample_node(), "\n")
            .unwrap();
        engine.delete_node("notes/a.md").unwrap();
        assert!(engine
            .read_node("notes/a.md")
            .unwrap()
            .node
            .deleted_at
            .is_some());

        engine.restore_node("notes/a.md").unwrap();
        assert!(engine
            .read_node("notes/a.md")
            .unwrap()
            .node
            .deleted_at
            .is_none());
    }

    #[test]
    fn purge_expired_trash_removes_only_nodes_past_retention() {
        let dir = TempDir::new("purge");
        let mut engine = Engine::init(dir.path()).unwrap();

        // Deleted "now" — inside any reasonable retention window.
        engine
            .create_node("notes/fresh.md", &sample_node(), "\n")
            .unwrap();
        engine.delete_node("notes/fresh.md").unwrap();

        // Deleted far in the past — outside a 30-day window.
        engine
            .create_node("notes/stale.md", &sample_node(), "\n")
            .unwrap();
        engine.delete_node("notes/stale.md").unwrap();
        let mut backdated = engine.read_node("notes/stale.md").unwrap().node;
        backdated.deleted_at = Some(Utc::now() - chrono::Duration::days(45));
        // Bypass the undo-tracked update_node here — this is only test setup
        // to simulate an old deletion, not a real engine operation.
        let contents = render(&backdated, "\n").unwrap();
        engine
            .vault
            .write_node("notes/stale.md", &contents)
            .unwrap();
        engine.rebuild_cache().unwrap();

        let removed = engine.purge_expired_trash_default().unwrap();
        assert_eq!(removed, 1);
        assert!(engine.read_node("notes/fresh.md").is_ok());
        assert!(engine.read_node("notes/stale.md").is_err());
    }

    #[test]
    fn undo_reverses_create_update_and_delete() {
        let dir = TempDir::new("undo");
        let mut engine = Engine::init(dir.path()).unwrap();

        // Undo a create: the file should be gone afterward.
        engine
            .create_node("notes/a.md", &sample_node(), "\n\nHello.\n")
            .unwrap();
        assert!(engine.undo().unwrap());
        assert!(engine.read_node("notes/a.md").is_err());

        // Redo brings it back exactly as it was.
        assert!(engine.redo().unwrap());
        let after_redo = engine.read_node("notes/a.md").unwrap();
        assert!(after_redo.body.contains("Hello."));

        // Undo an update: domain reverts.
        let mut updated = after_redo.node.clone();
        updated.domain = Some("changed".to_string());
        engine.update_node("notes/a.md", &updated).unwrap();
        assert_eq!(
            engine
                .read_node("notes/a.md")
                .unwrap()
                .node
                .domain
                .as_deref(),
            Some("changed")
        );
        assert!(engine.undo().unwrap());
        assert_eq!(engine.read_node("notes/a.md").unwrap().node.domain, None);

        // Undo a delete: deleted_at clears.
        engine.delete_node("notes/a.md").unwrap();
        assert!(engine
            .read_node("notes/a.md")
            .unwrap()
            .node
            .deleted_at
            .is_some());
        assert!(engine.undo().unwrap());
        assert!(engine
            .read_node("notes/a.md")
            .unwrap()
            .node
            .deleted_at
            .is_none());
    }

    #[test]
    fn undo_with_empty_stack_is_a_harmless_no_op() {
        let dir = TempDir::new("undo-empty");
        let mut engine = Engine::init(dir.path()).unwrap();
        assert!(!engine.can_undo());
        assert!(!engine.undo().unwrap());
        assert!(!engine.can_redo());
        assert!(!engine.redo().unwrap());
    }

    #[test]
    fn new_mutation_clears_the_redo_stack() {
        let dir = TempDir::new("redo-clear");
        let mut engine = Engine::init(dir.path()).unwrap();
        engine
            .create_node("notes/a.md", &sample_node(), "\n")
            .unwrap();
        engine.undo().unwrap();
        assert!(engine.can_redo());

        // A fresh mutation invalidates the redo branch (standard editor behavior).
        engine
            .create_node("notes/b.md", &sample_node(), "\n")
            .unwrap();
        assert!(!engine.can_redo());
    }
}
