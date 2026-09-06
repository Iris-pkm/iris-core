//! Guided project activation (ADR-023, ARCHITECTURE.md §11.5) — assembling
//! the focused working environment a project's activation surfaces.
//!
//! Every part of this is a query over the existing substrate (nodes,
//! relations, the distillation queue), never new stored state — ADR-023's
//! own guardrail 6.10 requirement, same pattern as `views.rs`/`search.rs`/
//! `distillation.rs`. Activation itself (the `someday/planned/paused ->
//! active` transition) lives in `engine.rs::set_project_status`; this module
//! only assembles what to show once that's true — it can be called any time
//! a project is active, not just at the moment of transition, since the
//! environment stays live for as long as the project does (ARCHITECTURE.md
//! §11.5: "kept live while the project is active").
//!
//! **Scope, matching ADR-023's seven parts:**
//! 1. `distillation::queue` — linked raw/undistilled notes (reused, not
//!    duplicated here).
//! 2. `unresolved_decisions` — open `annotation` comments (`resolved: false`)
//!    on the project or anything belonging to it.
//! 3. `blocked_tasks` — the project's tasks currently unable to proceed
//!    (an unmet `depends-on`, or blocked by an unfinished task via `blocks`).
//! 4. `related_resources` — `resource` nodes belonging to the project.
//! 5. `upcoming_events` — `event` nodes belonging to the project with a
//!    future `start`.
//! 6. `recently_added` — the project's most recently created nodes.
//! 7. `recommended_starting_set` — see its own doc comment; this is the one
//!    ADR-023 flags as AI-enrichable, and ARCHITECTURE.md §11.5 leaves its
//!    exact zero-AI heuristic as an open thread.
//!
//! **Not built:** AI-suggested ordering of the starting set (Phase 4,
//! ADR-023's own "AI enriches, never gates" boundary) and "why it's here"
//! rationale per item (ADR-024's transparent retrieval — shares infrastructure
//! with fused search, which doesn't exist yet either).

use crate::cache::{Cache, CachedNode};
use crate::distillation;
use crate::error::IrisResult;

/// The full assembled environment for one active project.
#[derive(Debug, Clone, Default)]
pub struct ActivationEnvironment {
    pub distillation_queue: Vec<CachedNode>,
    pub unresolved_decisions: Vec<CachedNode>,
    pub blocked_tasks: Vec<CachedNode>,
    pub related_resources: Vec<CachedNode>,
    pub upcoming_events: Vec<CachedNode>,
    pub recently_added: Vec<CachedNode>,
    pub recommended_starting_set: Vec<CachedNode>,
}

/// Assemble the guided-activation environment for `project_id`. Callable any
/// time the project is active — not only right after `set_project_status`
/// fires — since the environment is a live read-model, not a snapshot.
pub fn assemble(cache: &Cache, project_id: &str) -> IrisResult<ActivationEnvironment> {
    Ok(ActivationEnvironment {
        distillation_queue: distillation::queue(cache, project_id)?,
        unresolved_decisions: unresolved_decisions(cache, project_id)?,
        blocked_tasks: blocked_tasks(cache, project_id)?,
        related_resources: related_resources(cache, project_id)?,
        upcoming_events: upcoming_events(cache, project_id)?,
        recently_added: recently_added(cache, project_id, 10)?,
        recommended_starting_set: recommended_starting_set(cache, project_id)?,
    })
}

/// Open `annotation` comments (`resolved: false`) anchored either directly to
/// the project or to any node that belongs to it — Iris's stand-in for a
/// "decision" node type, of which there isn't one: `annotation.resolved`
/// already models exactly "has this open question been settled?"
pub fn unresolved_decisions(cache: &Cache, project_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT a.* FROM nodes a
         JOIN relations r ON r.source_id = a.id AND r.rel_type = 'annotates'
         WHERE a.node_type = 'annotation' AND a.resolved = 0 AND a.deleted_at IS NULL
           AND (
             r.target_id = ?1
             OR r.target_id IN (SELECT id FROM nodes WHERE parent_project = ?1)
           )
         ORDER BY a.id",
        [project_id],
    )
}

/// The project's tasks currently unable to proceed: an outstanding
/// `depends-on` target that isn't done, or blocked by a `blocks` source
/// that isn't done.
pub fn blocked_tasks(cache: &Cache, project_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT DISTINCT t.* FROM nodes t
         WHERE t.node_type = 'task' AND t.parent_project = ?1 AND t.deleted_at IS NULL
           AND (
             EXISTS (
               SELECT 1 FROM relations r JOIN nodes dep ON dep.id = r.target_id
               WHERE r.source_id = t.id AND r.rel_type = 'depends-on'
                 AND (dep.status IS NULL OR dep.status != 'done')
             )
             OR EXISTS (
               SELECT 1 FROM relations r JOIN nodes blocker ON blocker.id = r.source_id
               WHERE r.target_id = t.id AND r.rel_type = 'blocks'
                 AND (blocker.status IS NULL OR blocker.status != 'done')
             )
           )
         ORDER BY t.id",
        [project_id],
    )
}

/// `resource` nodes belonging to the project.
pub fn related_resources(cache: &Cache, project_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT * FROM nodes
         WHERE node_type = 'resource' AND parent_project = ?1 AND deleted_at IS NULL
         ORDER BY id",
        [project_id],
    )
}

/// `event` nodes belonging to the project with a future `start`.
pub fn upcoming_events(cache: &Cache, project_id: &str) -> IrisResult<Vec<CachedNode>> {
    let now = chrono::Utc::now().to_rfc3339();
    cache.query_nodes(
        "SELECT * FROM nodes
         WHERE node_type = 'event' AND parent_project = ?1 AND deleted_at IS NULL
           AND event_start IS NOT NULL AND event_start >= ?2
         ORDER BY event_start ASC",
        rusqlite::params![project_id, now],
    )
}

/// The project's `limit` most recently created nodes, of any type.
pub fn recently_added(cache: &Cache, project_id: &str, limit: u32) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT * FROM nodes
         WHERE parent_project = ?1 AND deleted_at IS NULL
         ORDER BY created DESC LIMIT ?2",
        rusqlite::params![project_id, limit],
    )
}

/// The project's tasks that are actually startable right now: not done, and
/// not blocked by an unmet `depends-on`. Ordered by priority.
///
/// **Zero-AI heuristic, flagged as a simplification of an open thread:**
/// ARCHITECTURE.md §11.5 leaves the exact non-AI ordering undefined beyond a
/// named example — "unblocked + highest-priority + on the critical path".
/// This implements the first two literally; "on the critical path" needs the
/// Phase 3 timeline/dependency-graph critical-path analysis, which doesn't
/// exist yet, so it's simply not part of the ordering here. AI-suggested
/// ordering (Phase 4) layers on top of this later — it enriches, never gates
/// (ADR-023).
pub fn recommended_starting_set(cache: &Cache, project_id: &str) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT * FROM nodes t
         WHERE t.node_type = 'task' AND t.parent_project = ?1 AND t.deleted_at IS NULL
           AND (t.status IS NULL OR t.status != 'done')
           AND NOT EXISTS (
             SELECT 1 FROM relations r JOIN nodes dep ON dep.id = r.target_id
             WHERE r.source_id = t.id AND r.rel_type = 'depends-on'
               AND (dep.status IS NULL OR dep.status != 'done')
           )
         ORDER BY
           CASE t.priority
             WHEN 'urgent' THEN 0
             WHEN 'high' THEN 1
             WHEN 'normal' THEN 2
             WHEN 'low' THEN 3
             ELSE 4
           END,
           t.id",
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
            let path = std::env::temp_dir().join(format!("iris-activation-test-{label}-{nanos}"));
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

    const PROJECT: &str = "01JQZ8PROJECTID0000000000AB";

    fn write(vault: &Vault, path: &str, contents: &str) {
        vault.write_node(path, contents).unwrap();
    }

    #[test]
    fn unresolved_decisions_finds_open_comments_on_project_and_members() {
        let dir = TempDir::new("decisions");
        let vault = Vault::create(dir.path()).unwrap();

        write(
            &vault,
            "notes/task.md",
            "---\nid: 01JQZ8TASKID000000000000EF\ntype: task\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nDo it.\n",
        );
        // Open comment on a project member.
        write(
            &vault,
            "notes/open-comment.md",
            "---\nid: 01JQZ8OPENID00000000000CD\ntype: annotation\ncreated: 2026-01-15T09:31:00Z\nmodified: 2026-01-15T09:31:00Z\nschema_version: 1\nresolved: false\nanchor:\n  text_fragment: \"Do it\"\nrelations:\n  - type: annotates\n    target: 01JQZ8TASKID000000000000EF\n---\n\nWhich approach?\n",
        );
        // Resolved comment — should not appear.
        write(
            &vault,
            "notes/resolved-comment.md",
            "---\nid: 01JQZ8RESOLVEDID000000CD\ntype: annotation\ncreated: 2026-01-15T09:32:00Z\nmodified: 2026-01-15T09:32:00Z\nschema_version: 1\nresolved: true\nanchor:\n  text_fragment: \"Do it\"\nrelations:\n  - type: annotates\n    target: 01JQZ8TASKID000000000000EF\n---\n\nSettled.\n",
        );

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = unresolved_decisions(&cache, PROJECT)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["01JQZ8OPENID00000000000CD".to_string()]);
    }

    #[test]
    fn blocked_tasks_finds_unmet_dependencies_both_directions() {
        let dir = TempDir::new("blocked");
        let vault = Vault::create(dir.path()).unwrap();

        // Prerequisite task, not done.
        write(
            &vault,
            "tasks/prereq.md",
            "---\nid: 01JQZ8PREREQID0000000000A\ntype: task\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\nstatus: todo\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nPrereq.\n",
        );
        // Depends on the unfinished prereq -> blocked.
        write(
            &vault,
            "tasks/dependent.md",
            "---\nid: 01JQZ8DEPENDENTID000000AB\ntype: task\ncreated: 2026-01-15T09:31:00Z\nmodified: 2026-01-15T09:31:00Z\nschema_version: 1\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n  - type: depends-on\n    target: 01JQZ8PREREQID0000000000A\n---\n\nDependent.\n",
        );
        // Freestanding task, not blocked.
        write(
            &vault,
            "tasks/free.md",
            "---\nid: 01JQZ8FREEID00000000000AB\ntype: task\ncreated: 2026-01-15T09:32:00Z\nmodified: 2026-01-15T09:32:00Z\nschema_version: 1\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nFree.\n",
        );

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = blocked_tasks(&cache, PROJECT)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec!["01JQZ8DEPENDENTID000000AB".to_string()]);
    }

    #[test]
    fn recommended_starting_set_excludes_blocked_and_done_orders_by_priority() {
        let dir = TempDir::new("starting-set");
        let vault = Vault::create(dir.path()).unwrap();

        write(
            &vault,
            "tasks/prereq.md",
            "---\nid: 01JQZ8PREREQID0000000000A\ntype: task\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\nstatus: todo\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nPrereq.\n",
        );
        write(
            &vault,
            "tasks/blocked.md",
            "---\nid: 01JQZ8BLOCKEDID0000000AB\ntype: task\ncreated: 2026-01-15T09:31:00Z\nmodified: 2026-01-15T09:31:00Z\nschema_version: 1\npriority: urgent\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n  - type: depends-on\n    target: 01JQZ8PREREQID0000000000A\n---\n\nBlocked despite urgency.\n",
        );
        write(
            &vault,
            "tasks/done.md",
            "---\nid: 01JQZ8DONEID0000000000AB\ntype: task\ncreated: 2026-01-15T09:32:00Z\nmodified: 2026-01-15T09:32:00Z\nschema_version: 1\nstatus: done\npriority: urgent\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nAlready done.\n",
        );
        write(
            &vault,
            "tasks/low.md",
            "---\nid: 01JQZ8LOWID00000000000AB\ntype: task\ncreated: 2026-01-15T09:33:00Z\nmodified: 2026-01-15T09:33:00Z\nschema_version: 1\npriority: low\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nLow priority, startable.\n",
        );
        write(
            &vault,
            "tasks/high.md",
            "---\nid: 01JQZ8HIGHID00000000000AB\ntype: task\ncreated: 2026-01-15T09:34:00Z\nmodified: 2026-01-15T09:34:00Z\nschema_version: 1\npriority: high\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nHigh priority, startable.\n",
        );

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = recommended_starting_set(&cache, PROJECT)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        // prereq (no priority set) sorts last among startables; high before low.
        assert_eq!(
            ids,
            vec![
                "01JQZ8HIGHID00000000000AB".to_string(),
                "01JQZ8LOWID00000000000AB".to_string(),
                "01JQZ8PREREQID0000000000A".to_string(),
            ]
        );
    }

    #[test]
    fn assemble_returns_all_seven_parts_without_error() {
        let dir = TempDir::new("assemble");
        let vault = Vault::create(dir.path()).unwrap();
        write(
            &vault,
            "tasks/a.md",
            "---\nid: 01JQZ8TASKID000000000000EF\ntype: task\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\nrelations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB\n---\n\nDo it.\n",
        );

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let env = assemble(&cache, PROJECT).unwrap();
        assert_eq!(env.recommended_starting_set.len(), 1);
        assert!(env.upcoming_events.is_empty());
    }
}
