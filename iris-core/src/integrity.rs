//! Vault integrity checker (ARCHITECTURE.md §16).
//!
//! Walks every node in the vault and flags problems without ever aborting the
//! walk itself — a malformed file is quarantined (reported, skipped), never
//! allowed to crash the check or silently vanish.

use std::collections::HashSet;

use crate::error::IrisResult;
use crate::vault::Vault;

/// A file whose frontmatter failed to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedFile {
    pub path: String,
    pub error: String,
}

/// A relation whose target id doesn't exist anywhere in the vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanglingRelation {
    pub source_id: String,
    pub rel_type: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntegrityReport {
    pub malformed_files: Vec<MalformedFile>,
    pub dangling_relations: Vec<DanglingRelation>,
}

impl IntegrityReport {
    /// A known-good vault (the state the test-suite fixtures should always produce)
    /// yields zero issues.
    pub fn is_clean(&self) -> bool {
        self.malformed_files.is_empty() && self.dangling_relations.is_empty()
    }
}

/// Walk the vault and report structural problems: malformed frontmatter and
/// relations pointing at a node id that doesn't exist.
pub fn check(vault: &Vault) -> IrisResult<IntegrityReport> {
    let mut report = IntegrityReport::default();
    let mut parsed = Vec::new();

    for path in vault.scan()? {
        match vault.read_node(&path) {
            Ok(node) => parsed.push(node),
            Err(e) => report.malformed_files.push(MalformedFile {
                path: path.strip_prefix(vault.root()).unwrap_or(&path).to_string_lossy().into_owned(),
                error: e.to_string(),
            }),
        }
    }

    let known_ids: HashSet<&str> = parsed.iter().map(|p| p.node.id.as_str()).collect();

    for parsed_node in &parsed {
        for rel in &parsed_node.node.relations {
            if !known_ids.contains(rel.target.as_str()) {
                report.dangling_relations.push(DanglingRelation {
                    source_id: parsed_node.node.id.clone(),
                    rel_type: rel.rel_type.clone(),
                    target_id: rel.target.clone(),
                });
            }
        }
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
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
            let path = std::env::temp_dir().join(format!("iris-integrity-test-{label}-{nanos}"));
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

    const TASK_WITH_DANGLING_RELATION: &str = "\
---
id: 01JQZ8TASKID000000000000EF
type: task
created: 2026-01-15T10:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
relations:
  - type: parent_project
    target: 01JQZ8DOESNOTEXIST0000000A
---

Orphaned task.
";

    const MALFORMED: &str = "\
---
id: [this is not valid yaml for a string
type: note
---

broken.
";

    #[test]
    fn clean_vault_reports_no_issues() {
        let dir = TempDir::new("clean");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();

        let report = check(&vault).unwrap();
        assert!(report.is_clean());
    }

    #[test]
    fn flags_dangling_relation() {
        let dir = TempDir::new("dangling");
        let vault = Vault::create(dir.path()).unwrap();
        vault
            .write_node("tasks/a.md", TASK_WITH_DANGLING_RELATION)
            .unwrap();

        let report = check(&vault).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.dangling_relations.len(), 1);
        assert_eq!(
            report.dangling_relations[0].target_id,
            "01JQZ8DOESNOTEXIST0000000A"
        );
    }

    #[test]
    fn quarantines_malformed_file_without_aborting_the_walk() {
        let dir = TempDir::new("malformed");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/good.md", NOTE).unwrap();
        vault.write_node("notes/bad.md", MALFORMED).unwrap();

        let report = check(&vault).unwrap();
        assert_eq!(report.malformed_files.len(), 1);
        assert!(report.malformed_files[0].path.contains("bad.md"));
        // The good file was still processed despite the bad one existing.
        assert!(report.dangling_relations.is_empty());
    }
}
