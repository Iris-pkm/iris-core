//! SQLite cache — a derived, rebuildable index of the vault (ADR-002).
//!
//! The cache is never the source of truth. It can be deleted and rebuilt from
//! the vault at any time with no data loss — `rebuild()` is the only way rows
//! get into it.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{IrisError, IrisResult};
use crate::vault::Vault;

/// A derived cache row for one node — enough to prove the cache reflects the
/// vault and to power basic task-view queries (ARCHITECTURE.md §12); richer
/// queries (search, full relation lookups) build on this later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedNode {
    pub id: String,
    pub node_type: String,
    pub path: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub scheduled_date: Option<String>,
    pub due_date: Option<String>,
    pub deleted_at: Option<String>,
    pub has_project: bool,
    pub domain: Option<String>,
    /// Comma-joined tags (SQLite has no array type). See `search.rs` for how
    /// tag filtering matches against this.
    pub tags: String,
    pub body: String,
}

pub struct Cache {
    conn: Connection,
}

impl Cache {
    /// Open (or create) a cache database file.
    pub fn open(path: impl AsRef<Path>) -> IrisResult<Self> {
        let conn = Connection::open(path).map_err(sqlite_err)?;
        let cache = Cache { conn };
        cache.create_schema()?;
        Ok(cache)
    }

    /// An in-memory cache, for tests.
    pub fn open_in_memory() -> IrisResult<Self> {
        let conn = Connection::open_in_memory().map_err(sqlite_err)?;
        let cache = Cache { conn };
        cache.create_schema()?;
        Ok(cache)
    }

    fn create_schema(&self) -> IrisResult<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS nodes (
                    id             TEXT PRIMARY KEY,
                    node_type      TEXT NOT NULL,
                    path           TEXT NOT NULL,
                    status         TEXT,
                    priority       TEXT,
                    scheduled_date TEXT,
                    due_date       TEXT,
                    deleted_at     TEXT,
                    has_project    INTEGER NOT NULL DEFAULT 0,
                    domain         TEXT,
                    tags           TEXT NOT NULL DEFAULT '',
                    body           TEXT NOT NULL DEFAULT ''
                );
                CREATE TABLE IF NOT EXISTS relations (
                    source_id TEXT NOT NULL,
                    rel_type  TEXT NOT NULL,
                    target_id TEXT NOT NULL
                );
                ",
            )
            .map_err(sqlite_err)
    }

    /// Wipe the cache and rebuild it from scratch by re-scanning and re-parsing
    /// the vault. Proves the cache is purely derived (ARCHITECTURE.md §16).
    pub fn rebuild(&mut self, vault: &Vault) -> IrisResult<()> {
        let tx = self.conn.transaction().map_err(sqlite_err)?;
        tx.execute("DELETE FROM nodes", []).map_err(sqlite_err)?;
        tx.execute("DELETE FROM relations", [])
            .map_err(sqlite_err)?;

        for path in vault.scan()? {
            // A malformed file is quarantined (skipped), never allowed to abort
            // the rebuild — one broken file degrades only itself.
            let Ok(parsed) = vault.read_node(&path) else {
                continue;
            };
            let rel_path = path
                .strip_prefix(vault.root())
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let node_type = node_type_str(&parsed.node.node_type);
            let priority = parsed.node.priority.as_ref().map(priority_str);
            let has_project = parsed
                .node
                .relations
                .iter()
                .any(|r| r.rel_type == "parent_project");
            let tags = parsed.node.tags.join(",");

            tx.execute(
                "INSERT INTO nodes
                    (id, node_type, path, status, priority, scheduled_date, due_date,
                     deleted_at, has_project, domain, tags, body)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    &parsed.node.id,
                    &node_type,
                    &rel_path,
                    &parsed.node.status,
                    &priority,
                    parsed.node.scheduled_date.map(|d| d.to_string()),
                    parsed.node.due_date.map(|d| d.to_string()),
                    parsed.node.deleted_at.map(|d| d.to_rfc3339()),
                    has_project as i64,
                    &parsed.node.domain,
                    &tags,
                    &parsed.body,
                ],
            )
            .map_err(sqlite_err)?;

            for rel in &parsed.node.relations {
                tx.execute(
                    "INSERT INTO relations (source_id, rel_type, target_id) VALUES (?1, ?2, ?3)",
                    (&parsed.node.id, &rel.rel_type, &rel.target),
                )
                .map_err(sqlite_err)?;
            }
        }

        tx.commit().map_err(sqlite_err)
    }

    /// All cached nodes, ordered by id (deterministic, for comparison/testing).
    pub fn list_nodes(&self) -> IrisResult<Vec<CachedNode>> {
        self.query_nodes("SELECT * FROM nodes ORDER BY id", [])
    }

    /// Run a `SELECT * FROM nodes WHERE ...` query and map every row to a `CachedNode`.
    /// Shared by the public views (`views.rs`) so query text and row-mapping live once.
    pub(crate) fn query_nodes<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> IrisResult<Vec<CachedNode>> {
        let mut stmt = self.conn.prepare(sql).map_err(sqlite_err)?;
        let rows = stmt
            .query_map(params, |row| {
                Ok(CachedNode {
                    id: row.get("id")?,
                    node_type: row.get("node_type")?,
                    path: row.get("path")?,
                    status: row.get("status")?,
                    priority: row.get("priority")?,
                    scheduled_date: row.get("scheduled_date")?,
                    due_date: row.get("due_date")?,
                    deleted_at: row.get("deleted_at")?,
                    has_project: row.get::<_, i64>("has_project")? != 0,
                    domain: row.get("domain")?,
                    tags: row.get("tags")?,
                    body: row.get("body")?,
                })
            })
            .map_err(sqlite_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_err)
    }
}

fn node_type_str(node_type: &crate::types::NodeType) -> String {
    serde_yaml::to_string(node_type)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn priority_str(priority: &crate::types::Priority) -> String {
    serde_yaml::to_string(priority)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn sqlite_err(e: rusqlite::Error) -> IrisError {
    IrisError::Vault(format!("cache error: {e}"))
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
            let path = std::env::temp_dir().join(format!("iris-cache-test-{label}-{nanos}"));
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

Hello.
";

    const TASK: &str = "\
---
id: 01JQZ8TASKID000000000000EF
type: task
created: 2026-01-15T10:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
relations:
  - type: parent_project
    target: 01JQZ8PROJECTID0000000000AB
---

Do the thing.
";

    const MALFORMED: &str = "\
---
id: [this is not valid yaml for a string
type: note
---

broken.
";

    #[test]
    fn rebuild_skips_malformed_file_without_aborting() {
        let dir = TempDir::new("malformed");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/good.md", NOTE).unwrap();
        vault.write_node("notes/bad.md", MALFORMED).unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let nodes = cache.list_nodes().unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, "01JQZ8XYABCDEF0123456789AB");
    }

    #[test]
    fn rebuild_populates_nodes_and_relations() {
        let dir = TempDir::new("rebuild");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();
        vault.write_node("tasks/b.md", TASK).unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let nodes = cache.list_nodes().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, "01JQZ8TASKID000000000000EF");
        assert_eq!(nodes[0].node_type, "task");
        assert_eq!(nodes[1].id, "01JQZ8XYABCDEF0123456789AB");
        assert_eq!(nodes[1].node_type, "note");

        let rel_count: i64 = cache
            .conn
            .query_row("SELECT COUNT(*) FROM relations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rel_count, 1);
    }

    #[test]
    fn full_cycle_rebuild_is_identical() {
        // Wipe cache, rebuild from vault, assert identical state to a fresh
        // rebuild — proves the cache is purely derived (ARCHITECTURE.md §16).
        let dir = TempDir::new("full-cycle");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();
        vault.write_node("tasks/b.md", TASK).unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();
        let first = cache.list_nodes().unwrap();

        // Simulate "wipe and rebuild": rebuild() already clears tables first,
        // so calling it again from the same vault state must reproduce the
        // exact same rows.
        cache.rebuild(&vault).unwrap();
        let second = cache.list_nodes().unwrap();

        assert_eq!(first, second, "rebuild must be deterministic");
    }

    #[test]
    fn open_creates_file_and_schema() {
        let dir = TempDir::new("open-file");
        let db_path = dir.path().join("cache.sqlite");
        let cache = Cache::open(&db_path).unwrap();
        assert!(db_path.exists());
        assert_eq!(cache.list_nodes().unwrap().len(), 0);
    }
}
