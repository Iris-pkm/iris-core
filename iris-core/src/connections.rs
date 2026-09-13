//! Node connections — the Node Editor's "Connections" right panel
//! (`design/navigation.md` §1): every relation touching a node, in either
//! direction. Outgoing relations are already stored on the node itself;
//! incoming ("backlinks") are derived by scanning the `relations` table for
//! edges that target this node — the same never-store-the-inverse principle
//! as ADR-017's `blocked-by`/`depended-on-by` (`dependencies.rs`),
//! generalized here to every relation type, not just `blocks`/`depends-on`.

use crate::cache::{Cache, CachedNode};
use crate::error::IrisResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ConnectionDirection {
    /// This node's own relation, pointing at `node`.
    Outgoing,
    /// `node`'s relation, pointing at this node.
    Incoming,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Connection {
    pub node: CachedNode,
    pub rel_type: String,
    pub direction: ConnectionDirection,
    pub label: String,
}

/// Every relation touching `node_id`, either direction, with a
/// human-readable per-item label. Excludes deleted nodes on either side.
pub fn connections(cache: &Cache, node_id: &str) -> IrisResult<Vec<Connection>> {
    let mut result = Vec::new();

    for (rel_type, source_id, target_id) in cache.relations_touching(node_id)? {
        let (other_id, direction) = if source_id == node_id {
            (target_id, ConnectionDirection::Outgoing)
        } else {
            (source_id, ConnectionDirection::Incoming)
        };

        let mut matches = cache.query_nodes(
            "SELECT * FROM nodes WHERE id = ?1 AND deleted_at IS NULL",
            [other_id],
        )?;
        let Some(other) = matches.pop() else {
            continue; // dangling relation — integrity.rs's job to flag, not ours
        };

        result.push(Connection {
            label: label_for(&rel_type, direction),
            node: other,
            rel_type,
            direction,
        });
    }

    result.sort_by(|a, b| a.node.id.cmp(&b.node.id));
    Ok(result)
}

/// Per-item display label for one relation, derived from SCHEMA_SPEC.md §5's
/// canonical/inverse relation-type registry. The registry itself names the
/// *collective* inverse (e.g. `parent`'s inverse is `children`); singularized
/// here for a per-item tag next to one specific connected node ("child of"
/// rather than "children"). Falls back to the raw `rel_type` for anything
/// not in the registry — an honest gap, not a guessed label.
fn label_for(rel_type: &str, direction: ConnectionDirection) -> String {
    use ConnectionDirection::*;
    match (rel_type, direction) {
        ("parent_project", Outgoing) => "in project",
        ("parent_project", Incoming) => "in this project",
        ("parent", Outgoing) => "parent",
        ("parent", Incoming) => "child of",
        ("blocks", Outgoing) => "blocks",
        ("blocks", Incoming) => "blocked by",
        ("depends-on", Outgoing) => "depends on",
        ("depends-on", Incoming) => "depended on by",
        ("related-to", _) => "related", // symmetric — its own inverse
        ("references", Outgoing) => "refs",
        ("references", Incoming) => "referenced by",
        ("annotates", Outgoing) => "on",
        ("annotates", Incoming) => "annotated by",
        ("graduated-from", Outgoing) => "graduated from",
        ("graduated-from", Incoming) => "graduated into",
        ("flow-next", Outgoing) => "next",
        ("flow-next", Incoming) => "previous",
        ("instance-of", Outgoing) => "instance of",
        ("instance-of", Incoming) => "has instance",
        (other, _) => return other.to_string(),
    }
    .to_string()
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
            let path = std::env::temp_dir().join(format!("iris-connections-test-{label}-{nanos}"));
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

    fn node(id: &str, node_type: &str, extra_relations: &str) -> String {
        format!(
            "\
---
id: {id}
type: {node_type}
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
relations:
{extra_relations}
---

Body.
"
        )
    }

    const PROJECT: &str = "01JQZ8PROJECTID0000000000AB";
    const CHILD_NOTE: &str = "01JQZ8CHILDID000000000000CD";
    const RESOURCE: &str = "01JQZ8RESOURCEID000000000EF";

    #[test]
    fn connections_include_both_outgoing_and_incoming() {
        let dir = TempDir::new("both-directions");
        let vault = Vault::create(dir.path()).unwrap();

        vault
            .write_node(
                "projects/p.md",
                &node(
                    PROJECT,
                    "project",
                    "  - type: references\n    target: 01JQZ8RESOURCEID000000000EF",
                ),
            )
            .unwrap();
        vault
            .write_node(
                "notes/child.md",
                &node(
                    CHILD_NOTE,
                    "note",
                    "  - type: parent\n    target: 01JQZ8PROJECTID0000000000AB",
                ),
            )
            .unwrap();
        vault
            .write_node(RESOURCE_PATH, &node(RESOURCE, "resource", ""))
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let conns = connections(&cache, PROJECT).unwrap();
        assert_eq!(conns.len(), 2);

        let child = conns.iter().find(|c| c.node.id == CHILD_NOTE).unwrap();
        assert_eq!(child.direction, ConnectionDirection::Incoming);
        assert_eq!(child.label, "child of");

        let resource = conns.iter().find(|c| c.node.id == RESOURCE).unwrap();
        assert_eq!(resource.direction, ConnectionDirection::Outgoing);
        assert_eq!(resource.label, "refs");
    }

    const RESOURCE_PATH: &str = "resources/r.md";

    #[test]
    fn dangling_relation_is_silently_excluded_not_erroring() {
        let dir = TempDir::new("dangling");
        let vault = Vault::create(dir.path()).unwrap();
        vault
            .write_node(
                "notes/a.md",
                &node(
                    "01JQZ8ANOTEID00000000000AB",
                    "note",
                    "  - type: references\n    target: 01JQZ8DOESNOTEXIST0000000A",
                ),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let conns = connections(&cache, "01JQZ8ANOTEID00000000000AB").unwrap();
        assert!(conns.is_empty());
    }

    #[test]
    fn unknown_relation_type_falls_back_to_raw_string() {
        let dir = TempDir::new("unknown-type");
        let vault = Vault::create(dir.path()).unwrap();
        vault
            .write_node(
                "notes/a.md",
                &node(
                    "01JQZ8ANOTEID00000000000AB",
                    "note",
                    "  - type: totally-custom\n    target: 01JQZ8BNOTEID00000000000CD",
                ),
            )
            .unwrap();
        vault
            .write_node(
                "notes/b.md",
                &node("01JQZ8BNOTEID00000000000CD", "note", ""),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let conns = connections(&cache, "01JQZ8ANOTEID00000000000AB").unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].label, "totally-custom");
    }
}
