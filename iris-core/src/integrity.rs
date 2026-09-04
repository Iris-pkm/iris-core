//! Vault integrity checker (ARCHITECTURE.md §16).
//!
//! Walks every node in the vault and flags problems without ever aborting the
//! walk itself — a malformed file is quarantined (reported, skipped), never
//! allowed to crash the check or silently vanish.

use std::collections::HashSet;

use crate::error::IrisResult;
use crate::types::NodeType;
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

/// An `annotation` node whose anchor doesn't resolve — its `annotates`
/// relation is missing or dangling, or its text-fragment anchor no longer
/// appears in the target's body (ARCHITECTURE.md §4: "if neither [CRDT
/// position nor text fragment] resolves, the annotation goes to an
/// `orphaned` state"). A reply annotation (no text anchor, `annotates` a
/// parent annotation) is never flagged here — it has nothing to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrphanedAnnotation {
    pub annotation_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntegrityReport {
    pub malformed_files: Vec<MalformedFile>,
    pub dangling_relations: Vec<DanglingRelation>,
    pub orphaned_annotations: Vec<OrphanedAnnotation>,
}

impl IntegrityReport {
    /// A known-good vault (the state the test-suite fixtures should always produce)
    /// yields zero issues.
    pub fn is_clean(&self) -> bool {
        self.malformed_files.is_empty()
            && self.dangling_relations.is_empty()
            && self.orphaned_annotations.is_empty()
    }
}

/// Walk the vault and report structural problems: malformed frontmatter,
/// relations pointing at a node id that doesn't exist, and annotations whose
/// anchor no longer resolves.
pub fn check(vault: &Vault) -> IrisResult<IntegrityReport> {
    let mut report = IntegrityReport::default();
    let mut parsed = Vec::new();

    for path in vault.scan()? {
        match vault.read_node(&path) {
            Ok(node) => parsed.push(node),
            Err(e) => report.malformed_files.push(MalformedFile {
                path: path
                    .strip_prefix(vault.root())
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
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

    for parsed_node in &parsed {
        if parsed_node.node.node_type != NodeType::Annotation {
            continue;
        }

        let annotates = parsed_node
            .node
            .relations
            .iter()
            .find(|r| r.rel_type == "annotates");
        let Some(annotates) = annotates else {
            report.orphaned_annotations.push(OrphanedAnnotation {
                annotation_id: parsed_node.node.id.clone(),
                reason: "no `annotates` relation".to_string(),
            });
            continue;
        };

        let Some(target) = parsed.iter().find(|p| p.node.id == annotates.target) else {
            report.orphaned_annotations.push(OrphanedAnnotation {
                annotation_id: parsed_node.node.id.clone(),
                reason: format!("annotates target `{}` does not exist", annotates.target),
            });
            continue;
        };

        // No text anchor at all means nothing to resolve — a threaded reply,
        // not an orphan (see doc comment on `OrphanedAnnotation`).
        if let Some(fragment) = parsed_node
            .node
            .anchor
            .as_ref()
            .and_then(|a| a.text_fragment.as_deref())
        {
            if !target.body.contains(fragment) {
                report.orphaned_annotations.push(OrphanedAnnotation {
                    annotation_id: parsed_node.node.id.clone(),
                    reason: "anchor text fragment not found in target body".to_string(),
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
    fn resolved_annotation_is_clean() {
        let dir = TempDir::new("annotation-resolved");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();
        vault
            .write_node(
                "notes/comment.md",
                "---\n\
id: 01JQZ8ANNOTID00000000000AB\n\
type: annotation\n\
created: 2026-01-15T09:30:00Z\n\
modified: 2026-01-15T09:30:00Z\n\
schema_version: 1\n\
resolved: false\n\
anchor:\n  text_fragment: \"Hello.\"\n\
relations:\n  - type: annotates\n    target: 01JQZ8XYABCDEF0123456789AB\n\
---\n\nComment body.\n",
            )
            .unwrap();

        let report = check(&vault).unwrap();
        assert!(report.is_clean());
    }

    #[test]
    fn annotation_with_unmatched_fragment_is_orphaned() {
        let dir = TempDir::new("annotation-orphaned");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();
        vault
            .write_node(
                "notes/comment.md",
                "---\n\
id: 01JQZ8ANNOTID00000000000AB\n\
type: annotation\n\
created: 2026-01-15T09:30:00Z\n\
modified: 2026-01-15T09:30:00Z\n\
schema_version: 1\n\
resolved: false\n\
anchor:\n  text_fragment: \"text that was deleted\"\n\
relations:\n  - type: annotates\n    target: 01JQZ8XYABCDEF0123456789AB\n\
---\n\nComment body.\n",
            )
            .unwrap();

        let report = check(&vault).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.orphaned_annotations.len(), 1);
        assert_eq!(
            report.orphaned_annotations[0].annotation_id,
            "01JQZ8ANNOTID00000000000AB"
        );
    }

    #[test]
    fn threaded_reply_with_no_anchor_is_not_orphaned() {
        let dir = TempDir::new("annotation-reply");
        let vault = Vault::create(dir.path()).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();
        vault
            .write_node(
                "notes/comment.md",
                "---\n\
id: 01JQZ8ANNOTID00000000000AB\n\
type: annotation\n\
created: 2026-01-15T09:30:00Z\n\
modified: 2026-01-15T09:30:00Z\n\
schema_version: 1\n\
resolved: false\n\
anchor:\n  text_fragment: \"Hello.\"\n\
relations:\n  - type: annotates\n    target: 01JQZ8XYABCDEF0123456789AB\n\
---\n\nComment body.\n",
            )
            .unwrap();
        vault
            .write_node(
                "notes/reply.md",
                "---\n\
id: 01JQZ8REPLYID0000000000CD\n\
type: annotation\n\
created: 2026-01-15T09:31:00Z\n\
modified: 2026-01-15T09:31:00Z\n\
schema_version: 1\n\
resolved: false\n\
relations:\n  - type: annotates\n    target: 01JQZ8ANNOTID00000000000AB\n\
---\n\nReply body.\n",
            )
            .unwrap();

        let report = check(&vault).unwrap();
        assert!(report.is_clean());
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
