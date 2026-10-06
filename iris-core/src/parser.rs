//! Lossless frontmatter parser — the first line of code in Iris.
//!
//! Every Iris node is a markdown file with YAML frontmatter between `---` markers.
//! This parser splits a file into its raw frontmatter and body, preserving both
//! byte-for-byte so that round-trip (parse → serialize) is identity for unmodified
//! nodes. This is the foundation of ADR-019 (lossless editing) and the golden-file
//! test suite.
//!
//! ## Format
//!
//! ```markdown
//! ---
//! id: 01JQZ8XYABCDEF0123456789AB
//! type: note
//! created: 2026-01-15T09:30:00Z
//! modified: 2026-01-15T09:30:00Z
//! schema_version: 1
//! ---
//!
//! Body content here.
//! ```
//!
//! The first line must be `---`. The second `---` closes the frontmatter block.
//! Everything after is the body (preserved byte-for-byte, including leading/trailing
//! whitespace).

use crate::error::{IrisError, IrisResult};
use crate::types::Node;

/// A parsed node file — the raw frontmatter and body preserved for lossless round-trip,
/// plus the typed Node deserialized from the frontmatter for convenient access.
#[derive(Debug, Clone)]
pub struct ParsedNode {
    /// The exact text between the `---` markers, including trailing newline (if any).
    pub raw_frontmatter: String,
    /// Everything after the closing `---` marker, preserved byte-for-byte.
    pub body: String,
    /// The frontmatter deserialized into a typed Node.
    pub node: Node,
}

impl ParsedNode {
    // ------------------------------------------------------------------
    // Parse
    // ------------------------------------------------------------------

    /// Parse a markdown file's contents into a `ParsedNode`.
    ///
    /// Returns an error if the file doesn't start with `---`, doesn't have a
    /// closing `---`, or the frontmatter YAML is invalid.
    pub fn parse(contents: &str) -> IrisResult<Self> {
        // The file must start with "---" (possibly followed by whitespace/newline).
        let Some(rest_after_open) = contents.strip_prefix("---") else {
            return Err(IrisError::Parse(
                "file does not start with '---' frontmatter delimiter".into(),
            ));
        };

        // The opening "---" must be immediately followed by a newline
        // (so "---foo" doesn't count as an opening delimiter).
        let rest_after_open = rest_after_open
            .strip_prefix('\n')
            .ok_or_else(|| IrisError::Parse("expected newline after opening '---'".into()))?;

        // Find the closing "---" on its own line.
        let (raw_frontmatter, body) = split_on_closing_delimiter(rest_after_open)?;

        // Parse the frontmatter YAML into a typed Node.
        let node: Node = serde_yaml::from_str(&raw_frontmatter)
            .map_err(|e| IrisError::Parse(format!("invalid frontmatter YAML: {e}")))?;

        Ok(ParsedNode {
            raw_frontmatter,
            body: body.to_string(),
            node,
        })
    }

    // ------------------------------------------------------------------
    // Serialize (lossless round-trip)
    // ------------------------------------------------------------------

    /// Serialize back to the exact on-disk representation.
    ///
    /// For an unmodified node, this produces byte-identical output to the
    /// original file (the golden-file guarantee).
    ///
    /// The format is: `---\n` + raw_frontmatter + `\n---` + body.
    /// Note: `body` already starts with the character (if any) that follows
    /// the closing `---` — typically `\n` for a normal file, or empty for
    /// a file that ends exactly at `---`.
    pub fn serialize(&self) -> String {
        let mut out = String::with_capacity(4 + self.raw_frontmatter.len() + 4 + self.body.len());
        out.push_str("---\n");
        out.push_str(&self.raw_frontmatter);
        out.push_str("\n---");
        out.push_str(&self.body);
        out
    }
}

/// Rewrite `old_raw` (a node's existing frontmatter) so it describes `node`,
/// touching only the top-level keys whose value actually changed (ADR-019
/// lossless *editing*): comments, blank lines, key order, formatting of
/// untouched keys, and fields `Node` doesn't model all survive byte-for-byte.
/// Changed keys are rewritten in place, new keys are appended, and a key
/// `Node` models but `node` no longer sets is removed.
///
/// Returns `None` when it can't do this safely (CRLF, quoted keys, a
/// continuation line stranded behind a comment, ...) — the result is always
/// re-parsed and checked equal to `node`, so `None` means "fall back to a full
/// re-serialize", never a wrong file.
// ponytail: line-based on top-level keys, no YAML CST; nested-value edits rewrite
// the whole top-level key (comments inside that key are lost). Upgrade: a real CST.
pub fn merge_frontmatter(old_raw: &str, node: &Node) -> Option<String> {
    use serde_yaml::{Mapping, Value};

    if old_raw.contains('\r') {
        return None;
    }
    let as_map = |n: &Node| match serde_yaml::to_value(n).ok()? {
        Value::Mapping(m) => Some(m),
        _ => None,
    };
    let old_typed = as_map(&serde_yaml::from_str::<Node>(old_raw).ok()?)?;
    let new_map = as_map(node)?;
    let emit = |k: &Value, v: &Value| {
        let mut m = Mapping::new();
        m.insert(k.clone(), v.clone());
        serde_yaml::to_string(&m)
            .ok()
            .map(|s| s.trim_end_matches('\n').to_string())
    };

    // Split into segments: a top-level `key:` line plus its indented / `- `
    // continuation lines, or a single verbatim comment/blank line.
    let mut segs: Vec<(Option<String>, Vec<&str>)> = Vec::new();
    for line in old_raw.split('\n') {
        let first = line.chars().next();
        if matches!(first, Some(' ' | '\t' | '-')) {
            match segs.last_mut() {
                Some((Some(_), lines)) => lines.push(line),
                _ => return None,
            }
        } else if first.is_none() || first == Some('#') {
            segs.push((None, vec![line]));
        } else {
            let key = line.split_once(':')?.0;
            if key.starts_with(['"', '\'']) {
                return None;
            }
            segs.push((Some(key.trim_end().to_string()), vec![line]));
        }
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (key, lines) in &segs {
        let Some(key) = key else {
            out.push(lines.join("\n"));
            continue;
        };
        let k = Value::String(key.clone());
        seen.insert(k.clone());
        match (new_map.get(&k), old_typed.get(&k)) {
            (Some(new), Some(old)) if new == old => out.push(lines.join("\n")),
            (Some(new), _) => out.push(emit(&k, new)?),
            (None, Some(_)) => {} // modelled field cleared -> drop it
            (None, None) => out.push(lines.join("\n")), // unknown field -> keep
        }
    }
    for (k, v) in &new_map {
        // Skip keys only present via serde defaults (e.g. `resolved: false`).
        if !seen.contains(k) && old_typed.get(k) != Some(v) {
            out.push(emit(k, v)?);
        }
    }

    let merged = out.join("\n");
    (serde_yaml::from_str::<Node>(&merged).ok()? == *node).then_some(merged)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Find the closing `---` delimiter and split the input into `(frontmatter, remainder)`.
///
/// The closing delimiter must be `---` at the start of a line (after a `\n`).
/// The `\n` before `---` belongs to the frontmatter (it ends the last frontmatter line).
/// The remainder is *everything* after `---` — it may start with `\n`, with content,
/// or be empty (file ends exactly at `---`). This is what makes the round-trip lossless:
/// we don't add or strip any implicit newlines around the delimiter.
fn split_on_closing_delimiter(input: &str) -> IrisResult<(String, &str)> {
    let mut search_start = 0;

    loop {
        let Some(delim_pos) = input[search_start..].find("\n---") else {
            return Err(IrisError::Parse(
                "missing closing '---' frontmatter delimiter".into(),
            ));
        };

        let abs_pos = search_start + delim_pos;

        // The closing delimiter is `---` at the start of a line (after `\n`).
        // It must be at end-of-input OR followed by `\n` or `\r\n` (on its own line).
        let rest = &input[abs_pos + 1..]; // skip the `\n`, now looking at `---...`
        debug_assert!(rest.starts_with("---"));
        let after_dashes = &rest[3..]; // skip `---`

        let is_valid_closing = after_dashes.is_empty()
            || after_dashes.starts_with('\n')
            || after_dashes.starts_with("\r\n");

        if is_valid_closing {
            // frontmatter = everything before the `\n---` delimiter
            // (the `\n` before `---` is part of the delimiter, not the content)
            let frontmatter = &input[..abs_pos];
            // remainder = everything after `---` (may be empty, `\n...`, etc.)
            return Ok((frontmatter.to_string(), after_dashes));
        }

        // "---foo" — false positive, keep searching past this `\n`
        search_start = abs_pos + 1;

        if search_start >= input.len() {
            return Err(IrisError::Parse(
                "missing closing '---' frontmatter delimiter".into(),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_note() {
        let contents = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
domain: trading
distillation_level: raw
---

Markets overreact to fear more than to greed.
";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        assert_eq!(parsed.node.id, "01JQZ8XYABCDEF0123456789AB");
        assert_eq!(parsed.node.domain.as_deref(), Some("trading"));
        assert!(parsed.body.starts_with('\n'));
        assert!(parsed.body.contains("Markets overreact"));
    }

    #[test]
    fn parse_closed_trading_journal_entry() {
        let contents = "\
---
id: 01JQZ8TRADEID000000000000
type: trading-journal-entry
created: 2026-09-02T09:30:00Z
modified: 2026-09-02T16:05:00Z
schema_version: 1
symbol: AAPL
entry: 187.2
exit: 191.4
pnl: 420.0
r_multiple: 1.4
---

Broke above 50dma with volume, momentum continuation play.
";
        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        assert_eq!(parsed.node.symbol.as_deref(), Some("AAPL"));
        assert_eq!(parsed.node.entry, Some(187.2));
        assert_eq!(parsed.node.exit, Some(191.4));
        assert_eq!(parsed.node.pnl, Some(420.0));
        assert_eq!(parsed.node.r_multiple, Some(1.4));
    }

    /// An open trade has no `exit` — open/closed is derived from this
    /// field's presence, not a separate stored status.
    #[test]
    fn parse_open_trading_journal_entry_has_no_exit() {
        let contents = "\
---
id: 01JQZ8TRADEID000000000001
type: trading-journal-entry
created: 2026-09-07T09:30:00Z
modified: 2026-09-07T09:30:00Z
schema_version: 1
symbol: MSFT
entry: 402.1
pnl: 180.0
---

Holding into next week's guidance — thesis still intact.
";
        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        assert_eq!(parsed.node.symbol.as_deref(), Some("MSFT"));
        assert_eq!(parsed.node.exit, None);
        assert_eq!(parsed.node.pnl, Some(180.0));
    }

    #[test]
    fn round_trip_identical() {
        let contents = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
domain: trading
distillation_level: raw
tags: [market-psychology, fear]
---

Markets overreact to fear far more than to greed. Worth watching for
capitulation signals rather than euphoria — the downside moves are faster.
";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        let serialized = parsed.serialize();
        assert_eq!(serialized, contents, "round-trip must be byte-identical");
    }

    #[test]
    fn round_trip_with_comments() {
        // Comments and unusual formatting in frontmatter must survive round-trip.
        let contents = "\
---
# This is a comment in the YAML frontmatter
id: 01JQZ8TASKID000000000000EF
type: task
created: 2026-01-15T10:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
domain: iris-dev
status: todo
priority: high

# Another comment above a field
scheduled_date: 2026-01-17
due_date: 2026-01-20
estimated_pomodoros: 3
relations:
  - type: parent_project
    target: 01JQZ8PROJECTID0000000000AB
checklist:
  - text: Re-read the sync section
    done: false
  - text: Note any open questions
    done: false
---

Review the architecture doc before the planning session.
";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        let serialized = parsed.serialize();
        assert_eq!(serialized, contents, "round-trip must preserve comments");
    }

    #[test]
    fn round_trip_project_node() {
        let contents = "\
---
id: 01JQZ8PROJECTID0000000000AB
type: project
created: 2026-01-10T08:00:00Z
modified: 2026-01-15T10:00:00Z
schema_version: 1
domain: iris-dev
status: active
target_date: 2026-06-30
---

Building Iris. Long-haul personal project.
";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        let serialized = parsed.serialize();
        assert_eq!(serialized, contents);
    }

    #[test]
    fn empty_body() {
        // Closing `---` with no trailing newline: body is empty.
        let contents = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
---";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        assert_eq!(parsed.body, "");
        let serialized = parsed.serialize();
        assert_eq!(serialized, contents);
    }

    #[test]
    fn body_with_blank_line_after_frontmatter() {
        // Closing `---\n` followed by a blank line: body starts with the blank line.
        let contents = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
---

Body starts after a blank line.
";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        // Body is everything after `---`: the `\n` that ends the `---` line,
        // plus the blank line, plus the content.
        assert_eq!(parsed.body, "\n\nBody starts after a blank line.\n");
        let serialized = parsed.serialize();
        assert_eq!(serialized, contents);
    }

    #[test]
    fn body_with_code_fence_containing_dashes() {
        // The body contains `---` inside a code fence — must not confuse the parser.
        let contents = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
---

Here's a code block:

```
---
not: frontmatter
---
```
";

        let parsed = ParsedNode::parse(contents).expect("parse should succeed");
        assert!(parsed.body.contains("```"));
        assert!(parsed.body.contains("not: frontmatter"));
        let serialized = parsed.serialize();
        assert_eq!(serialized, contents);
    }

    #[test]
    fn rejects_missing_opening_delimiter() {
        let result = ParsedNode::parse("just some markdown\nno frontmatter\n");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_closing_delimiter() {
        let contents = "\
---
id: 01JQZ8XYABCDEF0123456789AB
type: note
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1

No closing delimiter...
";
        let result = ParsedNode::parse(contents);
        assert!(result.is_err());
    }

    const COMMENTED: &str = "\
# my note
id: 01JQZ8XYABCDEF0123456789AB
type: note
mood: smug   # unknown to Iris
created: 2026-01-15T09:30:00Z
modified: 2026-01-15T09:30:00Z
schema_version: 1
tags:
- a
- b

# trailing comment
domain: work";

    #[test]
    fn merge_unchanged_node_is_byte_identical() {
        let node = ParsedNode::parse(&format!("---\n{COMMENTED}\n---\n"))
            .unwrap()
            .node;
        assert_eq!(merge_frontmatter(COMMENTED, &node).unwrap(), COMMENTED);
    }

    #[test]
    fn merge_edits_only_changed_keys() {
        let mut node = ParsedNode::parse(&format!("---\n{COMMENTED}\n---\n"))
            .unwrap()
            .node;
        node.tags.push("c".into());
        node.domain = None; // clearing a modelled field removes it
        node.priority = Some(crate::types::Priority::High); // new key appended
        let merged = merge_frontmatter(COMMENTED, &node).unwrap();
        assert!(merged.starts_with("# my note\nid: 01JQ"));
        assert!(merged.contains("mood: smug   # unknown to Iris"));
        assert!(merged.contains("# trailing comment"));
        assert!(merged.contains("tags:\n- a\n- b\n- c"));
        assert!(!merged.contains("domain"));
        assert!(merged.ends_with("priority: high"));
        assert_eq!(serde_yaml::from_str::<Node>(&merged).unwrap(), node);
    }

    #[test]
    fn merge_declines_crlf() {
        let node = ParsedNode::parse(&format!("---\n{COMMENTED}\n---\n"))
            .unwrap()
            .node;
        assert!(merge_frontmatter(&COMMENTED.replace('\n', "\r\n"), &node).is_none());
    }
}
