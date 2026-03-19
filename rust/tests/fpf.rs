//! Integration tests for FPF module

use s5d::fpf::doc_index::{chunk_markdown, DocIndex};

#[test]
fn test_chunk_markdown_basic() {
    let md = "# Title\nSome content\n## Section A\nBody A\n## Section B\nBody B\n";
    let chunks = chunk_markdown(md);
    assert!(chunks.len() >= 2, "Expected at least 2 chunks, got {}", chunks.len());
    assert_eq!(chunks[0].heading, "Title");
}

#[test]
fn test_chunk_markdown_nested() {
    let md = "# Root\n## Child\nContent\n### Grandchild\nDeep content\n";
    let chunks = chunk_markdown(md);
    assert!(chunks.len() >= 2);
    // Grandchild should have heading_path containing parent
    let gc = chunks.iter().find(|c| c.heading == "Grandchild");
    assert!(gc.is_some(), "Grandchild chunk not found");
    assert!(!gc.unwrap().heading_path.is_empty(), "Grandchild should have heading_path");
}

#[test]
fn test_chunk_markdown_empty() {
    let chunks = chunk_markdown("");
    assert!(chunks.is_empty());
}

#[test]
fn test_doc_index_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let idx = DocIndex::new(&db_path);
    idx.init().unwrap();

    let md = "# WLNK\nWeakest link bounds quality.\n# MONO\nMonotonicity of parts.\n";
    let n = idx.index(md, "test-v1").unwrap();
    assert_eq!(n, 2);
    assert_eq!(idx.chunk_count(), 2);

    // BM25 search
    let results = idx.search("weakest link", 5).unwrap();
    assert!(!results.is_empty(), "Search should find WLNK chunk");
    assert!(results[0].chunk.heading.contains("WLNK"));

    // Section lookup
    let section = idx.get_section("MONO").unwrap();
    assert!(section.is_some());
    assert!(section.unwrap().content.contains("Monotonicity"));
}

#[test]
fn test_doc_index_reindex() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let idx = DocIndex::new(&db_path);
    idx.init().unwrap();

    idx.index("# A\nFirst", "v1").unwrap();
    assert_eq!(idx.chunk_count(), 1);

    // Re-index replaces content
    idx.index("# B\nSecond\n# C\nThird", "v2").unwrap();
    assert_eq!(idx.chunk_count(), 2);

    let results = idx.search("Second", 5).unwrap();
    assert!(!results.is_empty());
}
