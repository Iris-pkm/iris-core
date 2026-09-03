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

use std::path::Path;

use chrono::Utc;

use crate::cache::Cache;
use crate::error::{IrisError, IrisResult};
use crate::git::GitRepo;
use crate::integrity::{self, IntegrityReport};
use crate::parser::ParsedNode;
use crate::types::Node;
use crate::vault::Vault;

pub struct Engine {
    vault: Vault,
    cache: Cache,
    git: GitRepo,
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

        let mut engine = Engine { vault, cache, git };
        engine.rebuild_cache()?;
        Ok(engine)
    }

    /// Open an existing vault (directory, git repo, and `.iris/cache.sqlite` must exist).
    pub fn open(path: impl AsRef<Path>) -> IrisResult<Self> {
        let vault = Vault::open(path.as_ref())?;
        let git = GitRepo::open(vault.root())?;
        let cache = Cache::open(vault.root().join(".iris/cache.sqlite"))?;
        Ok(Engine { vault, cache, git })
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
        validate(node)?;
        let contents = render(node, body)?;
        self.vault.write_node(&rel_path, &contents)?;
        self.rebuild_cache()?;
        self.git
            .commit_all(&format!("Create {}", rel_path.as_ref().display()))?;
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
        validate(node)?;
        let existing = self.vault.read_node(&rel_path)?;
        let contents = render(node, &existing.body)?;
        self.vault.write_node(&rel_path, &contents)?;
        self.rebuild_cache()?;
        self.git
            .commit_all(&format!("Update {}", rel_path.as_ref().display()))?;
        Ok(())
    }

    /// Soft-delete a node: sets `deleted_at` (ADR-016). The file is not removed.
    pub fn delete_node(&mut self, rel_path: impl AsRef<Path>) -> IrisResult<()> {
        let existing = self.vault.read_node(&rel_path)?;
        let mut node = existing.node.clone();
        node.deleted_at = Some(Utc::now());
        self.update_node(rel_path, &node)
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
}
