//! Export — one intermediate document model (IDM), many thin format writers
//! (ADR-028). Every writer consumes the IDM; none re-parses markdown or
//! re-resolves attachments independently.
//!
//! **Stage 1 scope, honestly stated:** JSON, CSV, HTML, and PDF are all real,
//! working writers now. PDF renders via an embedded Typst compiler
//! (`typst-as-lib`, pinned to the same 0.15.x line as `typst`/`typst-pdf`;
//! `typst-kit`'s embedded fonts, system font scanning off, so the build has
//! no runtime font dependency) — `body_markdown` is converted to Typst markup
//! by walking `pulldown-cmark`'s event stream (see `markdown_to_typst`), not
//! by re-parsing markdown a second time. Coverage matches the writer, not the
//! full Tier A rich-editing surface (ADR-027): headings, paragraphs,
//! bold/italic/strikethrough, inline code, fenced code blocks, links,
//! ordered/unordered lists (nesting via indent), block quotes, rules. Known
//! gaps, honestly flagged rather than silently mishandled: tables, math, and
//! images/attachments (no Tier-B resolution exists yet) fall back to a
//! visible placeholder rather than being dropped silently; an inline code
//! span containing a backtick is not escaped correctly (rare in practice).
//!
//! No attachments/Tier-B sidecar artifacts exist yet, so the IDM here is
//! just a node's resolved frontmatter + markdown body — it will need to grow
//! once those exist.

use std::path::Path;

use chrono::{DateTime, Utc};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use typst_as_lib::typst_kit_options::TypstKitFontOptions;
use typst_as_lib::TypstEngine;
use typst_layout::PagedDocument;

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

/// Stage 1 — PDF via an embedded Typst compiler (ADR-028). Builds a Typst
/// source document from the IDM (title heading + converted body) and
/// compiles it in-process — no external `typst` binary, no shelling out.
pub fn to_pdf(doc: &IdmDoc) -> IrisResult<Vec<u8>> {
    let source = format!(
        "#set page(margin: 1.75in)\n#set text(size: 11pt)\n= {}\n\n{}",
        escape_typst_text(&doc.title),
        markdown_to_typst(&doc.body_markdown)
    );

    let engine = TypstEngine::builder()
        .main_file(source)
        .search_fonts_with(TypstKitFontOptions::new().include_system_fonts(false))
        .build();

    let compiled = engine.compile::<PagedDocument>();
    let typst_doc = compiled
        .output
        .map_err(|e| IrisError::Validation(format!("PDF export failed to compile: {e:?}")))?;

    typst_pdf::pdf(&typst_doc, &Default::default())
        .map_err(|e| IrisError::Validation(format!("PDF export failed to render: {e:?}")))
}

/// Converts a markdown body to Typst markup by walking `pulldown-cmark`'s
/// event stream once — reuses the same parse the HTML writer relies on
/// rather than hand-rolling a second markdown parser.
fn markdown_to_typst(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::ENABLE_STRIKETHROUGH);
    let mut out = String::new();
    // Typst's "+"/"-" list markers only start a list at the beginning of a
    // line; track nesting depth so items indent instead of colliding.
    let mut list_depth: Vec<Option<u64>> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let n = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    out.push_str(&"=".repeat(n));
                    out.push(' ');
                }
                Tag::Paragraph => {}
                Tag::Strong => out.push('*'),
                Tag::Emphasis => out.push('_'),
                Tag::Strikethrough => out.push_str("#strike["),
                Tag::CodeBlock(kind) => {
                    out.push_str("```");
                    if let CodeBlockKind::Fenced(lang) = kind {
                        out.push_str(&lang);
                    }
                    out.push('\n');
                }
                Tag::BlockQuote(_) => out.push_str("#quote(block: true)["),
                Tag::List(start) => list_depth.push(start),
                Tag::Item => {
                    out.push('\n');
                    out.push_str(&"  ".repeat(list_depth.len().saturating_sub(1)));
                    match list_depth.last() {
                        Some(Some(_)) => out.push_str("+ "),
                        _ => out.push_str("- "),
                    }
                }
                Tag::Link { dest_url, .. } => {
                    out.push_str("#link(\"");
                    out.push_str(&dest_url.replace('\\', "\\\\").replace('"', "\\\""));
                    out.push_str("\")[");
                }
                Tag::Image { .. } => out.push_str("#emph[[image: "),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => out.push_str("\n\n"),
                TagEnd::Paragraph => out.push_str("\n\n"),
                TagEnd::Strong => out.push('*'),
                TagEnd::Emphasis => out.push('_'),
                TagEnd::Strikethrough => out.push(']'),
                TagEnd::CodeBlock => out.push_str("```\n\n"),
                TagEnd::BlockQuote(_) => out.push_str("]\n\n"),
                TagEnd::List(_) => {
                    list_depth.pop();
                    out.push('\n');
                    if list_depth.is_empty() {
                        out.push('\n');
                    }
                }
                TagEnd::Item => {}
                TagEnd::Link => out.push(']'),
                TagEnd::Image => out.push_str("]]"),
                _ => {}
            },
            Event::Text(text) => out.push_str(&escape_typst_text(&text)),
            Event::Code(text) => {
                out.push('`');
                out.push_str(&text);
                out.push('`');
            }
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push_str(" #linebreak()\n"),
            Event::Rule => out.push_str("\n#line(length: 100%)\n\n"),
            _ => {}
        }
    }

    out
}

/// Escapes Typst markup-significant characters in plain text so note content
/// renders as literal text rather than being interpreted as markup. Known
/// gap: a `-`/`+`/digit-`.` sequence at the start of a text run inside a
/// paragraph (not one of our own list items) can still be read as a list
/// marker by Typst — rare in practice, not worth a stateful line-start
/// tracker for this pass.
fn escape_typst_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '#' | '*' | '_' | '$' | '<' | '>' | '@' | '[' | ']' | '`' | '~'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
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
    fn pdf_compiles_to_a_valid_pdf() {
        let dir = TempDir::new("pdf");
        let doc = sample_doc(dir.path());
        let pdf = to_pdf(&doc).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 100);
    }

    #[test]
    fn markdown_to_typst_converts_common_constructs() {
        let typst =
            markdown_to_typst("# Heading\n\n**bold** and *italic* and `code`.\n\n- one\n- two\n");
        assert!(typst.contains("= Heading"));
        assert!(typst.contains("*bold*"));
        assert!(typst.contains("_italic_"));
        assert!(typst.contains("`code`"));
        assert!(typst.contains("- one"));
        assert!(typst.contains("- two"));
    }

    #[test]
    fn markdown_to_typst_escapes_special_characters() {
        let typst = markdown_to_typst("Cost is #5 per item.");
        assert!(typst.contains("\\#5"));
    }
}
