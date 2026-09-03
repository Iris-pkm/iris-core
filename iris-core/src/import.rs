//! Vault import — onboarding-critical, not a late utility (ADR-025). First
//! importers: plain-Markdown-folder and Obsidian, since they map most
//! directly onto Iris's own format (lowest mapping risk).
//!
//! **Scope, honestly stated:** each source `.md` file becomes one `note`
//! node. Preserved: body content, frontmatter `tags` (if the file already has
//! valid Iris/Obsidian-style frontmatter), file mtime as `created`/`modified`,
//! and — Obsidian only — `[[wikilinks]]` resolved to `related-to` relations.
//! **Not yet preserved:** attachments, folder hierarchy as anything beyond a
//! flat destination path, Obsidian's `#inline-tags`, and unresolved wikilinks
//! (a link to a note not found in the import set is silently dropped rather
//! than becoming a dangling relation — there's no target id to point at).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::engine::Engine;
use crate::error::IrisResult;
use crate::parser::ParsedNode;
use crate::types::{new_node_id, Node, NodeType, Relation, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub imported: usize,
    /// (source path, reason) for files that existed but couldn't be imported.
    pub skipped: Vec<(PathBuf, String)>,
}

/// Import every `.md` file under `source` as a plain note. No link resolution.
pub fn import_markdown_folder(engine: &mut Engine, source: &Path) -> IrisResult<ImportReport> {
    let files = collect_md_files(source)?;
    let mut report = ImportReport::default();

    for path in files {
        match import_one(engine, source, &path, &[]) {
            Ok(_) => report.imported += 1,
            Err(e) => report.skipped.push((path, e.to_string())),
        }
    }
    Ok(report)
}

/// Import an Obsidian vault: same as `import_markdown_folder`, plus resolving
/// `[[Note Name]]` / `[[Note Name|alias]]` wikilinks into `related-to` relations.
pub fn import_obsidian_vault(engine: &mut Engine, source: &Path) -> IrisResult<ImportReport> {
    let files = collect_md_files(source)?;

    // Pass 1: assign every file a node id, keyed by filename stem (how
    // Obsidian wikilinks reference other notes), so pass 2 can resolve links.
    let id_by_stem: Vec<(String, String)> = files
        .iter()
        .filter_map(|p| {
            p.file_stem()
                .map(|s| (s.to_string_lossy().to_lowercase(), new_node_id()))
        })
        .collect();

    let mut report = ImportReport::default();
    for path in &files {
        match import_one(engine, source, path, &id_by_stem) {
            Ok(_) => report.imported += 1,
            Err(e) => report.skipped.push((path.clone(), e.to_string())),
        }
    }
    Ok(report)
}

fn import_one(
    engine: &mut Engine,
    source_root: &Path,
    path: &Path,
    id_by_stem: &[(String, String)],
) -> IrisResult<()> {
    let raw = std::fs::read_to_string(path)?;

    // If the file already has valid frontmatter (some Obsidian vaults, or a
    // re-imported Iris file), reuse its tags and body. Otherwise the whole
    // file is the body and there are no tags.
    let (mut tags, body_text) = match ParsedNode::parse(&raw) {
        Ok(parsed) => (
            parsed.node.tags,
            parsed.body.trim_start_matches('\n').to_string(),
        ),
        Err(_) => (Vec::new(), raw.clone()),
    };
    tags.sort();
    tags.dedup();

    let id = id_by_stem
        .iter()
        .find(|(stem, _)| {
            Some(stem.as_str())
                == path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(str::to_lowercase)
                    .as_deref()
        })
        .map(|(_, id)| id.clone())
        .unwrap_or_else(new_node_id);

    let relations = resolve_wikilinks(&body_text, id_by_stem);
    let (created, modified) = file_times(path);

    let node = Node {
        id,
        node_type: NodeType::Note,
        created,
        modified,
        schema_version: CURRENT_SCHEMA_VERSION,
        lifecycle: None,
        archived_at: None,
        domain: None,
        tags,
        relations,
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
    };

    let dest_rel = Path::new("imported").join(path.strip_prefix(source_root).unwrap_or(path));
    let body = format!("\n\n{body_text}");
    engine.create_node(dest_rel, &node, &body)
}

/// Find `[[Target]]` / `[[Target|alias]]` / `[[Target#heading]]` wikilinks in
/// `body` and resolve each to a `related-to` relation, dropping any link whose
/// target isn't in `id_by_stem`.
fn resolve_wikilinks(body: &str, id_by_stem: &[(String, String)]) -> Vec<Relation> {
    let mut relations = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break };
        let inner = &rest[..end];
        rest = &rest[end + 2..];

        let target_name = inner
            .split(['|', '#'])
            .next()
            .unwrap_or(inner)
            .trim()
            .to_lowercase();
        if let Some((_, id)) = id_by_stem.iter().find(|(stem, _)| *stem == target_name) {
            relations.push(Relation {
                rel_type: "related-to".to_string(),
                target: id.clone(),
            });
        }
    }
    relations
}

fn collect_md_files(dir: &Path) -> IrisResult<Vec<PathBuf>> {
    let mut found = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                continue;
            }
            if path.is_dir() {
                found.extend(collect_md_files(&path)?);
            } else if path.extension().is_some_and(|e| e == "md") {
                found.push(path);
            }
        }
    }
    Ok(found)
}

fn file_times(path: &Path) -> (DateTime<Utc>, DateTime<Utc>) {
    let meta = std::fs::metadata(path).ok();
    let to_dt = |t: std::io::Result<std::time::SystemTime>| {
        t.ok().and_then(|t| DateTime::<Utc>::from(t).into())
    };
    let now = Utc::now();
    let created = meta
        .as_ref()
        .and_then(|m| to_dt(m.created()))
        .unwrap_or(now);
    let modified = meta
        .as_ref()
        .and_then(|m| to_dt(m.modified()))
        .unwrap_or(now);
    (created, modified)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("iris-import-test-{label}-{nanos}"));
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

    #[test]
    fn markdown_folder_imports_plain_files() {
        let src = TempDir::new("md-src");
        let dst = TempDir::new("md-dst");
        fs::write(src.path().join("a.md"), "# Hello\n\nJust a plain note.").unwrap();
        fs::write(src.path().join("b.md"), "Another note.").unwrap();

        let mut engine = Engine::init(dst.path()).unwrap();
        let report = import_markdown_folder(&mut engine, src.path()).unwrap();

        assert_eq!(report.imported, 2);
        assert!(report.skipped.is_empty());
        assert_eq!(engine.check_integrity().unwrap().malformed_files.len(), 0);
    }

    #[test]
    fn obsidian_import_resolves_wikilinks() {
        let src = TempDir::new("obsidian-src");
        let dst = TempDir::new("obsidian-dst");
        fs::write(
            src.path().join("Project Iris.md"),
            "This project links to [[Sync Design]] and [[Missing Note]].",
        )
        .unwrap();
        fs::write(
            src.path().join("Sync Design.md"),
            "Naive sync first, CRDT later.",
        )
        .unwrap();

        let mut engine = Engine::init(dst.path()).unwrap();
        let report = import_obsidian_vault(&mut engine, src.path()).unwrap();
        assert_eq!(report.imported, 2);

        let project = engine.read_node("imported/Project Iris.md").unwrap();
        // Resolved: one relation to Sync Design. Missing Note is dropped (no target id).
        assert_eq!(project.node.relations.len(), 1);
        assert_eq!(project.node.relations[0].rel_type, "related-to");

        let sync_design = engine.read_node("imported/Sync Design.md").unwrap();
        assert_eq!(project.node.relations[0].target, sync_design.node.id);
    }

    #[test]
    fn preserves_tags_from_existing_frontmatter() {
        let src = TempDir::new("tags-src");
        let dst = TempDir::new("tags-dst");
        fs::write(
            src.path().join("tagged.md"),
            "---\nid: whatever\ntype: note\ncreated: 2026-01-01T00:00:00Z\nmodified: 2026-01-01T00:00:00Z\nschema_version: 1\ntags: [imported, test]\n---\n\nBody here.",
        )
        .unwrap();

        let mut engine = Engine::init(dst.path()).unwrap();
        import_markdown_folder(&mut engine, src.path()).unwrap();

        let node = engine.read_node("imported/tagged.md").unwrap();
        assert_eq!(
            node.node.tags,
            vec!["imported".to_string(), "test".to_string()]
        );
        assert!(node.body.contains("Body here."));
    }
}
