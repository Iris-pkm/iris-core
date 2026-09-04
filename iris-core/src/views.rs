//! Task views — filtered lenses over the cached task nodes (ARCHITECTURE.md §12,
//! PRD §7 Organize). Not separate storage: every view is a query over `cache.rs`'s
//! `nodes` table, scoped to `node_type = task` and excluding soft-deleted nodes.
//!
//! **Simplification, flagged:** these implement ARCHITECTURE.md §12's literal
//! filter formulas. PRD's richer Inbox description ("no project, no date, no
//! priority") and Logbook's "ordered by completion time" (no `completed_at`
//! field exists yet — this orders by id instead) aren't fully implemented.
//! Good enough to prove the view layer works; not the final query set.

use chrono::NaiveDate;

use crate::cache::{Cache, CachedNode};
use crate::error::IrisResult;

const TASK_BASE: &str = "node_type = 'task' AND deleted_at IS NULL AND is_template = 0";

/// Freshly captured tasks with no project (ARCHITECTURE.md §12: `project = null`).
pub fn inbox(cache: &Cache) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        &format!("SELECT * FROM nodes WHERE {TASK_BASE} AND has_project = 0 ORDER BY id"),
        [],
    )
}

/// Tasks scheduled for `today`, plus overdue tasks not yet done.
pub fn today(cache: &Cache, today: NaiveDate) -> IrisResult<Vec<CachedNode>> {
    let today = today.to_string();
    cache.query_nodes(
        &format!(
            "SELECT * FROM nodes WHERE {TASK_BASE}
             AND (scheduled_date = ?1 OR (due_date <= ?1 AND (status IS NULL OR status != 'done')))
             ORDER BY id"
        ),
        [today],
    )
}

/// Tasks scheduled within `[from, from + days]` inclusive.
pub fn upcoming(cache: &Cache, from: NaiveDate, days: u32) -> IrisResult<Vec<CachedNode>> {
    let end = from + chrono::Duration::days(days as i64);
    cache.query_nodes(
        &format!(
            "SELECT * FROM nodes WHERE {TASK_BASE}
             AND scheduled_date BETWEEN ?1 AND ?2 ORDER BY scheduled_date, id"
        ),
        [from.to_string(), end.to_string()],
    )
}

/// Tasks with no schedule and no due date — the GTD holding area.
pub fn someday_maybe(cache: &Cache) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        &format!(
            "SELECT * FROM nodes WHERE {TASK_BASE}
             AND scheduled_date IS NULL AND due_date IS NULL ORDER BY id"
        ),
        [],
    )
}

/// Completed tasks. Ordering is by id, not completion time — see module docs.
pub fn logbook(cache: &Cache) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        &format!("SELECT * FROM nodes WHERE {TASK_BASE} AND status = 'done' ORDER BY id"),
        [],
    )
}

/// Trash: every soft-deleted node, any type (ARCHITECTURE.md §5 Data Integrity
/// & Recovery), most-recently-deleted first. Unlike the task views above this
/// isn't scoped to `node_type = 'task'` — Trash holds anything the user deleted.
pub fn trash(cache: &Cache) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT * FROM nodes WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        [],
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
            let path = std::env::temp_dir().join(format!("iris-views-test-{label}-{nanos}"));
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

    fn task(id: &str, extra: &str) -> String {
        format!(
            "\
---
id: {id}
type: task
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
{extra}
---

Body.
"
        )
    }

    fn setup(dir: &Path) -> Cache {
        let vault = Vault::create(dir).unwrap();

        // Inbox: no project, no dates.
        vault
            .write_node("tasks/inbox.md", &task("01JQZ8INBOX0000000000000A", ""))
            .unwrap();

        // Has a project -> not Inbox.
        vault
            .write_node(
                "tasks/has-project.md",
                &task(
                    "01JQZ8PROJ00000000000000B",
                    "relations:\n  - type: parent_project\n    target: 01JQZ8PROJECTID0000000000AB",
                ),
            )
            .unwrap();

        // Scheduled today.
        vault
            .write_node(
                "tasks/today.md",
                &task("01JQZ8TODAY0000000000000C", "scheduled_date: 2026-01-15"),
            )
            .unwrap();

        // Overdue, not done.
        vault
            .write_node(
                "tasks/overdue.md",
                &task(
                    "01JQZ8OVERDUE000000000000D",
                    "due_date: 2026-01-10\nstatus: todo",
                ),
            )
            .unwrap();

        // Scheduled next week.
        vault
            .write_node(
                "tasks/next-week.md",
                &task("01JQZ8NEXTWEEK00000000000E", "scheduled_date: 2026-01-20"),
            )
            .unwrap();

        // Done.
        vault
            .write_node(
                "tasks/done.md",
                &task("01JQZ8DONE00000000000000F", "status: done"),
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();
        cache
    }

    fn jan15() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
    }

    #[test]
    fn inbox_excludes_tasks_with_a_project() {
        let dir = TempDir::new("inbox");
        let cache = setup(dir.path());
        let ids: Vec<_> = inbox(&cache).unwrap().into_iter().map(|n| n.id).collect();
        assert!(ids.contains(&"01JQZ8INBOX0000000000000A".to_string()));
        assert!(!ids.contains(&"01JQZ8PROJ00000000000000B".to_string()));
    }

    #[test]
    fn today_includes_scheduled_and_overdue() {
        let dir = TempDir::new("today");
        let cache = setup(dir.path());
        let ids: Vec<_> = today(&cache, jan15())
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(ids.contains(&"01JQZ8TODAY0000000000000C".to_string()));
        assert!(ids.contains(&"01JQZ8OVERDUE000000000000D".to_string()));
        assert!(!ids.contains(&"01JQZ8NEXTWEEK00000000000E".to_string()));
    }

    #[test]
    fn today_excludes_overdue_but_done() {
        let dir = TempDir::new("today-done");
        let vault = Vault::create(dir.path()).unwrap();
        vault
            .write_node(
                "tasks/overdue-done.md",
                &task(
                    "01JQZ8OVERDUEDONE0000000A",
                    "due_date: 2026-01-10\nstatus: done",
                ),
            )
            .unwrap();
        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = today(&cache, jan15())
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(!ids.contains(&"01JQZ8OVERDUEDONE0000000A".to_string()));
    }

    #[test]
    fn upcoming_is_bounded_by_days() {
        let dir = TempDir::new("upcoming");
        let cache = setup(dir.path());

        // next-week.md is scheduled 5 days out (2026-01-20) — a 3-day window
        // should include today's task but exclude it; a 7-day window should
        // include both.
        let narrow: Vec<_> = upcoming(&cache, jan15(), 3)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(narrow.contains(&"01JQZ8TODAY0000000000000C".to_string()));
        assert!(!narrow.contains(&"01JQZ8NEXTWEEK00000000000E".to_string()));

        let wide: Vec<_> = upcoming(&cache, jan15(), 7)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(wide.contains(&"01JQZ8NEXTWEEK00000000000E".to_string()));
    }

    #[test]
    fn someday_maybe_has_no_dates() {
        let dir = TempDir::new("someday");
        let cache = setup(dir.path());
        let ids: Vec<_> = someday_maybe(&cache)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert!(ids.contains(&"01JQZ8INBOX0000000000000A".to_string()));
        assert!(ids.contains(&"01JQZ8PROJ00000000000000B".to_string()));
        assert!(!ids.contains(&"01JQZ8TODAY0000000000000C".to_string()));
    }

    #[test]
    fn logbook_is_done_only() {
        let dir = TempDir::new("logbook");
        let cache = setup(dir.path());
        let ids: Vec<_> = logbook(&cache).unwrap().into_iter().map(|n| n.id).collect();
        assert_eq!(ids, vec!["01JQZ8DONE00000000000000F".to_string()]);
    }

    #[test]
    fn trash_holds_any_deleted_node_type_ordered_most_recent_first() {
        let dir = TempDir::new("trash");
        let vault = Vault::create(dir.path()).unwrap();

        vault
            .write_node(
                "notes/kept.md",
                "---\nid: 01JQZ8KEPT0000000000000A\ntype: note\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\n---\n\nStill here.\n",
            )
            .unwrap();
        vault
            .write_node(
                "notes/trashed-earlier.md",
                "---\nid: 01JQZ8TRASH1000000000000B\ntype: note\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\ndeleted_at: 2026-01-15T10:00:00Z\n---\n\nGone (earlier).\n",
            )
            .unwrap();
        vault
            .write_node(
                "tasks/trashed-later.md",
                "---\nid: 01JQZ8TRASH2000000000000C\ntype: task\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\ndeleted_at: 2026-01-15T12:00:00Z\n---\n\nGone (later).\n",
            )
            .unwrap();

        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();

        let ids: Vec<_> = trash(&cache).unwrap().into_iter().map(|n| n.id).collect();
        assert_eq!(
            ids,
            vec![
                "01JQZ8TRASH2000000000000C".to_string(),
                "01JQZ8TRASH1000000000000B".to_string(),
            ]
        );
    }
}
