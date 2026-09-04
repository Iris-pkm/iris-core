//! Distillation queue — the "linked raw/undistilled notes" surfaced when a
//! project is active (ARCHITECTURE.md §11/§11.5, ADR-006, ADR-018).
//!
//! Not separate storage: queue membership is *derived*, purely from each
//! node's `parent_project` relation and its `distillation_level`
//! (ARCHITECTURE.md §11.5: "everything shown is derived from data already in
//! the graph... a query/presentation layer, not new stored state" —
//! guardrail 6.10). This resolves ARCHITECTURE.md §11's own open thread
//! ("is queue state derived, or does it need its own persisted table?") the
//! same way `views.rs` already resolved task views: a query over the cache,
//! no parallel system. Because membership is derived rather than snapshotted,
//! ADR-018 rule (3) — notes linked while a project stays active keep flowing
//! into the queue — falls out for free: there's nothing to re-snapshot.
//!
//! Project *activation* itself (ADR-018's state-machine transition into
//! `active`) lives in `engine.rs::set_project_status` — that's a mutation,
//! this module is query-only.
//!
//! **Simplification, flagged:** scoped to any node type carrying a
//! `parent_project` relation, not just `note`. ARCHITECTURE.md says "raw
//! notes" but doesn't restrict the mechanism to the `note` type, and nothing
//! else in the schema does either — a task or resource attached to a project
//! with a real `distillation_level` is just as much "raw material to
//! process." Full guided project activation (ADR-023's seven-part
//! environment — decisions, blocked tasks, resources, calendar, recommended
//! starting set) is a separate, larger piece and isn't built here; this is
//! only its first, already-named component.

use crate::cache::{Cache, CachedNode};
use crate::error::IrisResult;

/// Notes/tasks/etc. belonging to `project_id` that aren't yet fully
/// distilled — `distillation_level` unset (never processed) or anything
/// short of `summarized`. Ordered by id for determinism; a real ranking
/// (recency, distillation level, priority) is a UI-layer concern once one
/// exists.
pub fn queue(cache: &Cache, project_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT * FROM nodes
         WHERE deleted_at IS NULL AND is_template = 0
           AND parent_project = ?1
           AND (distillation_level IS NULL OR distillation_level != 'summarized')
         ORDER BY id",
        [project_id],
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
            let path = std::env::temp_dir().join(format!("iris-distillation-test-{label}-{nanos}"));
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

    const PROJECT_A: &str = "01JQZ8PROJECTAID00000000AB";
    const PROJECT_B: &str = "01JQZ8PROJECTBID00000000CD";

    fn note(id: &str, project: &str, extra: &str) -> String {
        format!(
            "\
---
id: {id}
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
relations:
  - type: parent_project
    target: {project}
{extra}
---

Body.
"
        )
    }

    fn setup(dir: &Path) -> Cache {
        let vault = Vault::create(dir).unwrap();

        // Never processed (no distillation_level at all) — in the queue.
        vault
            .write_node(
                "notes/never-processed.md",
                &note("01JQZ8NOTE1000000000000A", PROJECT_A, ""),
            )
            .unwrap();

        // Bolded but not summarized — still in the queue.
        vault
            .write_node(
                "notes/bolded.md",
                &note(
                    "01JQZ8NOTE2000000000000B",
                    PROJECT_A,
                    "distillation_level: bolded",
                ),
            )
            .unwrap();

        // Fully summarized — out of the queue.
        vault
            .write_node(
                "notes/summarized.md",
                &note(
                    "01JQZ8NOTE3000000000000C",
                    PROJECT_A,
                    "distillation_level: summarized",
                ),
            )
            .unwrap();

        // Belongs to a different project — not in Project A's queue.
        vault
            .write_node(
                "notes/other-project.md",
                &note("01JQZ8NOTE4000000000000D", PROJECT_B, ""),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();
        cache
    }

    #[test]
    fn queue_includes_unprocessed_and_partially_distilled_notes() {
        let dir = TempDir::new("queue-includes");
        let cache = setup(dir.path());
        let ids: Vec<_> = queue(&cache, PROJECT_A)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(ids.contains(&"01JQZ8NOTE1000000000000A".to_string()));
        assert!(ids.contains(&"01JQZ8NOTE2000000000000B".to_string()));
    }

    #[test]
    fn queue_excludes_summarized_and_other_projects() {
        let dir = TempDir::new("queue-excludes");
        let cache = setup(dir.path());
        let ids: Vec<_> = queue(&cache, PROJECT_A)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(!ids.contains(&"01JQZ8NOTE3000000000000C".to_string()));
        assert!(!ids.contains(&"01JQZ8NOTE4000000000000D".to_string()));
    }

    #[test]
    fn unknown_project_yields_empty_queue() {
        let dir = TempDir::new("queue-unknown");
        let cache = setup(dir.path());
        assert!(queue(&cache, "01JQZ8DOESNOTEXIST0000000A")
            .unwrap()
            .is_empty());
    }
}
