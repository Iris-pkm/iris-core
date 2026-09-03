//! Export — one intermediate document model (IDM), many thin format writers
//! (ADR-028). Every writer consumes the IDM; none re-parses markdown or
//! re-resolves attachments independently.
//!
//! **Stage 1 scope, honestly stated:** JSON, CSV, and HTML are real, working
//! writers. PDF is a documented stub — ADR-028 specifies PDF via an embedded
//! Typst layout engine, which is a substantial subsystem of its own (fonts,
//! page layout, a `World` trait implementation) and genuinely out of scope
//! for this pass. `to_pdf` returns a clear "not implemented" error rather
//! than silently producing nothing or a half-correct PDF.
//!
//! No attachments/Tier-B sidecar artifacts exist yet, so the IDM here is
//! just a node's resolved frontmatter + markdown body — it will need to grow
//! once those exist.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::{IrisError, IrisResult};
use crate::parser::ParsedNode;

/// The intermediate document model: a node's resolved content, independent
/// of any output format.
#[derive(Debug, Clone, Serialize)]
pub struct IdmDoc {
    pub id: String,
    pub node_type: String,
    /// No `title` field exists in the schema (see `search.rs`'s same note) —
    /// this is the file path, used as a stand-in.
    pub title: String,
    pub domain: Option<String>,
    pub tags: Vec<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub body_markdown: String,
}

impl IdmDoc {
    pub fn from_parsed(parsed: &ParsedNode, path: &Path) -> Self {
        let node = &parsed.node;
        IdmDoc {
            id: node.id.clone(),
            node_type: node_type_str(&node.node_type),
            title: path.to_string_lossy().into_owned(),
            domain: node.domain.clone(),
            tags: node.tags.clone(),
            created: node.created,
            modified: node.modified,
            body_markdown: parsed.body.trim_start_matches('\n').to_string(),
        }
    }
}

/// Stage 1 — JSON: a direct, structured serialization of the IDM.
pub fn to_json(doc: &IdmDoc) -> IrisResult<String> {
    serde_json::to_string_pretty(doc)
        .map_err(|e| IrisError::Validation(format!("JSON export failed: {e}")))
}

/// Stage 1 — CSV: one row per document, for tabular views (task lists, etc).
pub fn to_csv(docs: &[IdmDoc]) -> IrisResult<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record([
            "id",
            "node_type",
            "title",
            "domain",
            "tags",
            "created",
            "modified",
            "body_markdown",
        ])
        .map_err(csv_err)?;
    for doc in docs {
        writer
            .write_record([
                &doc.id,
                &doc.node_type,
                &doc.title,
                doc.domain.as_deref().unwrap_or(""),
                &doc.tags.join(","),
                &doc.created.to_rfc3339(),
                &doc.modified.to_rfc3339(),
                &doc.body_markdown,
            ])
            .map_err(csv_err)?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| IrisError::Validation(format!("CSV export failed: {e}")))?;
    String::from_utf8(bytes)
        .map_err(|e| IrisError::Validation(format!("CSV export produced invalid UTF-8: {e}")))
}

/// Stage 1 — HTML: renders the markdown body via `pulldown-cmark`. Will later
/// share this rendering path with the live-preview editor pane (ADR-028), which
/// doesn't exist yet.
pub fn to_html(doc: &IdmDoc) -> String {
    let parser = pulldown_cmark::Parser::new(&doc.body_markdown);
    let mut body_html = String::new();
    pulldown_cmark::html::push_html(&mut body_html, parser);

    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>{}</title></head>\n<body>\n{}\n</body></html>\n",
        html_escape(&doc.title),
        body_html
    )
}

/// Stage 1 — PDF via embedded Typst. **Not implemented** — see module docs.
pub fn to_pdf(_doc: &IdmDoc) -> IrisResult<Vec<u8>> {
    Err(IrisError::Validation(
        "PDF export is not implemented yet — needs an embedded Typst integration (ADR-028)".into(),
    ))
}

fn node_type_str(node_type: &crate::types::NodeType) -> String {
    serde_yaml::to_string(node_type)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn csv_err(e: csv::Error) -> IrisError {
    IrisError::Validation(format!("CSV export failed: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;
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
            let path = std::env::temp_dir().join(format!("iris-export-test-{label}-{nanos}"));
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
domain: trading
tags: [fear, greed]
---

# Heading

Markets **overreact** to fear.
";

    fn sample_doc(dir: &Path) -> IdmDoc {
        let vault = Vault::create(dir).unwrap();
        vault.write_node("notes/a.md", NOTE).unwrap();
        let parsed = vault.read_node("notes/a.md").unwrap();
        IdmDoc::from_parsed(&parsed, Path::new("notes/a.md"))
    }

    #[test]
    fn json_round_trips_the_essentials() {
        let dir = TempDir::new("json");
        let doc = sample_doc(dir.path());
        let json = to_json(&doc).unwrap();
        assert!(json.contains("01JQZ8XYABCDEF0123456789AB"));
        assert!(json.contains("trading"));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], "01JQZ8XYABCDEF0123456789AB");
    }

    #[test]
    fn csv_has_header_and_one_row_per_doc() {
        let dir = TempDir::new("csv");
        let doc = sample_doc(dir.path());
        let csv = to_csv(&[doc.clone(), doc]).unwrap();

        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        let headers = reader.headers().unwrap().clone();
        assert_eq!(headers.get(0), Some("id"));
        assert_eq!(headers.get(1), Some("node_type"));

        let records: Vec<_> = reader.records().collect::<Result<_, _>>().unwrap();
        assert_eq!(records.len(), 2); // 2 data rows (header handled separately above)
    }

    #[test]
    fn html_renders_markdown() {
        let dir = TempDir::new("html");
        let doc = sample_doc(dir.path());
        let html = to_html(&doc);
        assert!(html.contains("<h1>Heading</h1>"));
        assert!(html.contains("<strong>overreact</strong>"));
    }

    #[test]
    fn pdf_is_a_clear_not_implemented_error() {
        let dir = TempDir::new("pdf");
        let doc = sample_doc(dir.path());
        let err = to_pdf(&doc).unwrap_err();
        assert!(err.to_string().contains("not implemented"));
    }
}
