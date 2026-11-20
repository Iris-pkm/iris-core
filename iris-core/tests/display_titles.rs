use iris_core::{
    engine::Engine,
    ffi::FfiNode,
    parser::{merge_frontmatter, ParsedNode},
    search::{search, SearchFilters},
};

const NOTE: &str = "---\nid: 01JQZ8XYABCDEF0123456789AB\ntype: note\ncreated: 2026-01-15T09:30:00Z\nmodified: 2026-01-15T09:30:00Z\nschema_version: 1\ntitle: '  Authored title  ' # keep formatting\n---\n\n# Heading title\n";

#[test]
fn authored_title_survives_round_trip_and_ffi_edit() {
    for source in [
        NOTE.to_string(),
        NOTE.replace("title: '  Authored title  ' # keep formatting\n", ""),
    ] {
        let parsed = ParsedNode::parse(&source).unwrap();
        assert_eq!(parsed.serialize(), source);
        let mut dto = FfiNode::from(&parsed.node);
        dto.domain = Some("changed".into());
        let edited = dto.try_into().unwrap();
        let merged = merge_frontmatter(&parsed.raw_frontmatter, &edited).unwrap();
        if parsed.node.title.is_some() {
            assert!(merged.contains("title: '  Authored title  ' # keep formatting"));
        }
        assert_eq!(
            ParsedNode::parse(&format!("---\n{merged}\n---{}", parsed.body))
                .unwrap()
                .node
                .title,
            parsed.node.title
        );
    }
}

#[test]
fn old_cache_rebuilds_on_open_and_titles_are_searchable() {
    let root = std::env::temp_dir().join(format!(
        "iris-title-{}-{}",
        std::process::id(),
        iris_core::types::new_node_id()
    ));
    let mut engine = Engine::init(&root).unwrap();
    let parsed = ParsedNode::parse(NOTE).unwrap();
    engine
        .create_node("notes/unrelated.md", &parsed.node, &parsed.body)
        .unwrap();
    assert_eq!(
        engine.cache().list_nodes().unwrap()[0].title,
        "Authored title"
    );
    assert_eq!(
        search(engine.cache(), "Authored", &SearchFilters::default())
            .unwrap()
            .len(),
        1
    );
    drop(engine);
    let cache_path = root.join(".iris/cache.sqlite");
    let connection = rusqlite::Connection::open(&cache_path).unwrap();
    connection
        .execute_batch(
            "DROP TABLE nodes; CREATE TABLE nodes (id TEXT PRIMARY KEY); PRAGMA user_version = 0;",
        )
        .unwrap();
    drop(connection);
    let mut engine = Engine::open(&root).unwrap();
    assert_eq!(
        engine.cache().list_nodes().unwrap()[0].title,
        "Authored title"
    );
    let mut node = engine.read_node("notes/unrelated.md").unwrap().node;
    node.title = None;
    engine.update_node("notes/unrelated.md", &node).unwrap();
    assert_eq!(
        engine.cache().list_nodes().unwrap()[0].title,
        "Heading title"
    );
    assert_eq!(
        search(engine.cache(), "Heading title", &SearchFilters::default())
            .unwrap()
            .len(),
        1
    );
    assert!(!engine
        .read_node("notes/unrelated.md")
        .unwrap()
        .raw_frontmatter
        .contains("title:"));
    drop(engine);
    std::fs::remove_dir_all(root).unwrap();
}
