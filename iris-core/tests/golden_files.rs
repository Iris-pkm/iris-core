//! Golden-file round-trip tests — the mechanical enforcement of "files are canonical."
//!
//! Every `.md` file in `tests/fixtures/` is parsed and re-serialized; the output
//! must be byte-identical to the original. This is the test that catches silent
//! parse→serialize drift: reordered fields, dropped comments, reformatted values.
//!
//! See `ARCHITECTURE.md` §16 and ADR-019.

use iris_core::parser::ParsedNode;
use std::fs;
use std::path::Path;

#[test]
fn golden_file_round_trip_simple_note() {
    assert_round_trip("tests/fixtures/simple-note.md");
}

#[test]
fn golden_file_round_trip_task() {
    assert_round_trip("tests/fixtures/task.md");
}

#[test]
fn golden_file_round_trip_project() {
    assert_round_trip("tests/fixtures/project.md");
}

#[test]
fn golden_file_round_trip_note_with_comment() {
    assert_round_trip("tests/fixtures/note-with-comment.md");
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Read a fixture file, parse it, serialize it, and assert byte-identical output.
fn assert_round_trip(path: &str) {
    let full_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let contents = fs::read_to_string(&full_path).expect("should read fixture file");

    let parsed = ParsedNode::parse(&contents).expect("should parse fixture file");
    let serialized = parsed.serialize();

    assert_eq!(
        serialized, contents,
        "golden-file round-trip failed for {}",
        full_path.display()
    );
}
