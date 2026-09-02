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
            .ok_or_else(|| {
                IrisError::Parse(
                    "expected newline after opening '---'".into(),
                )
            })?;

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
        let mut out = String::with_capacity(
            4 + self.raw_frontmatter.len() + 4 + self.body.len(),
        );
        out.push_str("---\n");
        out.push_str(&self.raw_frontmatter);
        out.push_str("\n---");
        out.push_str(&self.body);
        out
    }
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
}
