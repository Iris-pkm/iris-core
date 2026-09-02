//! Basic search — lexical (title/body) plus type/domain/tag filtering over the
//! cache (PRD §7 "Basic search"). This is *not* the four-layer fusion search
//! (lexical + semantic + structural + temporal, RRF-ranked) — that's Phase 4.
//! No `title` field exists in the schema, so "title" search matches against
//! the node's file path as a stand-in; flagged as a simplification.

use crate::cache::{Cache, CachedNode};
use crate::error::IrisResult;

#[derive(Debug, Clone, Default)]
pub struct SearchFilters<'a> {
    pub node_type: Option<&'a str>,
    pub domain: Option<&'a str>,
    pub tag: Option<&'a str>,
}

/// Search by `query` (matched against path and body, case-insensitive substring),
/// narrowed by `filters`. An empty `query` returns everything matching the filters.
/// Excludes soft-deleted nodes.
pub fn search(cache: &Cache, query: &str, filters: &SearchFilters) -> IrisResult<Vec<CachedNode>> {
    cache.query_nodes(
        "SELECT * FROM nodes WHERE deleted_at IS NULL
           AND (?1 = '' OR path LIKE '%' || ?1 || '%' COLLATE NOCASE
                         OR body LIKE '%' || ?1 || '%' COLLATE NOCASE)
           AND (?2 IS NULL OR node_type = ?2)
           AND (?3 IS NULL OR domain = ?3)
           AND (?4 IS NULL OR (',' || tags || ',') LIKE '%,' || ?4 || ',%')
         ORDER BY id",
        rusqlite::params![query, filters.node_type, filters.domain, filters.tag],
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
            let path = std::env::temp_dir().join(format!("iris-search-test-{label}-{nanos}"));
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

    fn note(id: &str, domain: &str, tags: &str, body: &str) -> String {
        format!(
            "\
---
id: {id}
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
domain: {domain}
tags: [{tags}]
---

{body}
"
        )
    }

    fn setup(dir: &Path) -> Cache {
        let vault = Vault::create(dir).unwrap();
        vault
            .write_node(
                "notes/trading.md",
                &note(
                    "01JQZ8TRADING000000000000A",
                    "trading",
                    "fear, greed",
                    "Markets overreact to fear.",
                ),
            )
            .unwrap();
        vault
            .write_node(
                "notes/music.md",
                &note(
                    "01JQZ8MUSIC00000000000000B",
                    "music",
                    "riff",
                    "A new riff idea in D minor.",
                ),
            )
            .unwrap();
        let mut cache = Cache::open_in_memory().unwrap();
        cache.rebuild(&vault).unwrap();
        cache
    }

    #[test]
    fn empty_query_returns_everything() {
        let dir = TempDir::new("empty-query");
        let cache = setup(dir.path());
        let results = search(&cache, "", &SearchFilters::default()).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn body_match_is_case_insensitive() {
        let dir = TempDir::new("body-match");
        let cache = setup(dir.path());
        let results = search(&cache, "FEAR", &SearchFilters::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "01JQZ8TRADING000000000000A");
    }

    #[test]
    fn path_match_works_too() {
        let dir = TempDir::new("path-match");
        let cache = setup(dir.path());
        let results = search(&cache, "music", &SearchFilters::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "01JQZ8MUSIC00000000000000B");
    }

    #[test]
    fn domain_filter_narrows_results() {
        let dir = TempDir::new("domain-filter");
        let cache = setup(dir.path());
        let filters = SearchFilters {
            domain: Some("music"),
            ..Default::default()
        };
        let results = search(&cache, "", &filters).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "01JQZ8MUSIC00000000000000B");
    }

    #[test]
    fn tag_filter_matches_whole_tag_only() {
        let dir = TempDir::new("tag-filter");
        let cache = setup(dir.path());

        let matches_fear = search(
            &cache,
            "",
            &SearchFilters {
                tag: Some("fear"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(matches_fear.len(), 1);

        // "fea" must not match "fear" as a substring — whole-tag matching only.
        let matches_partial = search(
            &cache,
            "",
            &SearchFilters {
                tag: Some("fea"),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches_partial.is_empty());
    }
}
