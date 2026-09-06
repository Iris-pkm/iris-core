//! Task dependency queries (ARCHITECTURE.md §12, ADR-017).
//!
//! Only the canonical direction is ever stored: `blocks` and `depends-on`.
//! Their inverses (`blocked-by`, `depended-on-by`) are never written to a
//! file — they're derived here, on demand, from the `relations` table. This
//! is the inverse registry ADR-017 describes, applied to the one pair of
//! relation types that currently need it. A node's "blocked" state is
//! computed, not stored: "whether anything that `blocks` it is still
//! incomplete" (ARCHITECTURE.md §12), which is exactly what `is_blocked`
//! and `blocked_by` check.
//!
//! Project-scoped bulk queries (e.g. "every blocked task in this project")
//! live in `activation.rs`; these are the single-node, either-direction
//! primitives it and any future caller build on.

use crate::cache::{Cache, CachedNode};
use crate::error::IrisResult;

fn is_incomplete(node: &CachedNode) -> bool {
    node.status.as_deref() != Some("done")
}

/// Nodes this node directly depends on (the canonical, stored `depends-on`
/// relation) that aren't done yet — the concrete reason it's blocked.
pub fn blocked_by(cache: &Cache, node_id: &str) -> IrisResult<Vec<CachedNode>> {
    let targets = cache.query_nodes(
        "SELECT n.* FROM nodes n
         JOIN relations r ON r.target_id = n.id
         WHERE r.source_id = ?1 AND r.rel_type = 'depends-on' AND n.deleted_at IS NULL
         ORDER BY n.id",
        [node_id],
    )?;
    Ok(targets.into_iter().filter(is_incomplete).collect())
}

/// Nodes that block this one via the canonical, stored `blocks` relation
/// pointing at it, that aren't done yet.
pub fn blocked_by_incoming(cache: &Cache, node_id: &str) -> IrisResult<Vec<CachedNode>> {
    let sources = cache.query_nodes(
        "SELECT n.* FROM nodes n
         JOIN relations r ON r.source_id = n.id
         WHERE r.target_id = ?1 AND r.rel_type = 'blocks' AND n.deleted_at IS NULL
         ORDER BY n.id",
        [node_id],
    )?;
    Ok(sources.into_iter().filter(is_incomplete).collect())
}

/// Whether this node is currently blocked at all — an unfinished
/// `depends-on` target, or an unfinished `blocks` source pointing at it.
pub fn is_blocked(cache: &Cache, node_id: &str) -> IrisResult<bool> {
    Ok(!blocked_by(cache, node_id)?.is_empty() || !blocked_by_incoming(cache, node_id)?.is_empty())
}

/// Nodes this node blocks — the canonical, stored `blocks` relation,
/// straight lookup (no inverse needed, this direction is already stored).
pub fn blocks(cache: &Cache, node_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT n.* FROM nodes n
         JOIN relations r ON r.target_id = n.id
         WHERE r.source_id = ?1 AND r.rel_type = 'blocks' AND n.deleted_at IS NULL
         ORDER BY n.id",
        [node_id],
    )
}

/// Nodes that depend on this one — the derived inverse of the canonical,
/// stored `depends-on` relation (`depended-on-by`, per ADR-017's registry).
pub fn depended_on_by(cache: &Cache, node_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT n.* FROM nodes n
         JOIN relations r ON r.source_id = n.id
         WHERE r.target_id = ?1 AND r.rel_type = 'depends-on' AND n.deleted_at IS NULL
         ORDER BY n.id",
        [node_id],
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("iris-dependencies-test-{label}-{nanos}"));
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

    fn task(id: &str, status: &str, extra_relations: &str) -> String {
        format!(
            "\
---
id: {id}
type: task
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
{status}
relations:
{extra_relations}
---

Body.
"
        )
    }

    #[test]
    fn blocked_by_depends_on_excludes_done_targets() {
        let dir = TempDir::new("blocked-by-depends-on");
        let vault = Vault::create(dir.path()).unwrap();

        vault
            .write_node(
                "tasks/prereq-open.md",
                &task("01JQZ8PREREQOPEN000000AB", "status: todo", ""),
            )
            .unwrap();
        vault
            .write_node(
                "tasks/prereq-done.md",
                &task("01JQZ8PREREQDONE000000AB", "status: done", ""),
            )
            .unwrap();
        vault
            .write_node(
                "tasks/dependent.md",
                &task(
                    "01JQZ8DEPENDENTID000000AB",
                    "",
                    "  - type: depends-on\n    target: 01JQZ8PREREQOPEN000000AB\n  - type: depends-on\n    target: 01JQZ8PREREQDONE000000AB",
                ),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = blocked_by(&cache, "01JQZ8DEPENDENTID000000AB")
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["01JQZ8PREREQOPEN000000AB".to_string()]);
        assert!(is_blocked(&cache, "01JQZ8DEPENDENTID000000AB").unwrap());
        assert!(!is_blocked(&cache, "01JQZ8PREREQOPEN000000AB").unwrap());
    }

    #[test]
    fn blocked_by_incoming_blocks_relation() {
        let dir = TempDir::new("blocked-by-incoming");
        let vault = Vault::create(dir.path()).unwrap();

        vault
            .write_node(
                "tasks/blocker.md",
                &task(
                    "01JQZ8BLOCKERID0000000AB",
                    "status: todo",
                    "  - type: blocks\n    target: 01JQZ8BLOCKEDID0000000AB",
                ),
            )
            .unwrap();
        vault
            .write_node(
                "tasks/blocked.md",
                &task("01JQZ8BLOCKEDID0000000AB", "", ""),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = blocked_by_incoming(&cache, "01JQZ8BLOCKEDID0000000AB")
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["01JQZ8BLOCKERID0000000AB".to_string()]);

        let blocks_ids: Vec<_> = blocks(&cache, "01JQZ8BLOCKERID0000000AB")
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(blocks_ids, vec!["01JQZ8BLOCKEDID0000000AB".to_string()]);
    }

    #[test]
    fn depended_on_by_is_the_derived_inverse_of_depends_on() {
        let dir = TempDir::new("depended-on-by");
        let vault = Vault::create(dir.path()).unwrap();

        vault
            .write_node("tasks/prereq.md", &task("01JQZ8PREREQID0000000AB", "", ""))
            .unwrap();
        vault
            .write_node(
                "tasks/dependent.md",
                &task(
                    "01JQZ8DEPENDENTID000000AB",
                    "",
                    "  - type: depends-on\n    target: 01JQZ8PREREQID0000000AB",
                ),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = depended_on_by(&cache, "01JQZ8PREREQID0000000AB")
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["01JQZ8DEPENDENTID000000AB".to_string()]);
    }
}
